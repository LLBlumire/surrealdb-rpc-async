use std::{collections::BTreeMap, ops::Deref, sync::Arc};

use crate::{
    error::{Error, Result},
    request::SurrealRequest,
    response::SurrealRawResponse,
};

use futures::{sink::Send as SinkSend, sink::SinkExt, stream::SplitSink, StreamExt};
use serde_json::Value;
use tokio::{
    net::TcpStream,
    sync::{mpsc, oneshot},
};
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};

macro_rules! handle_err {
    ($err_handler:expr, $try:expr) => {
        match $try {
            Ok(e) => e,
            Err(e) => match $err_handler.handle_error(e.into()) {
                crate::client::ClientAction::IgnoreError => continue,
                crate::client::ClientAction::Shutdown => return,
            },
        }
    };
}

/// A websocket based client connected to a surrealdb database server.
#[derive(Debug)]
pub struct Client {
    response_channel_sink: mpsc::UnboundedSender<(String, oneshot::Sender<SurrealRawResponse>)>,
    socket_sink: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
    _task: tokio::task::JoinHandle<()>,
}
impl Client {
    /// Create a new client
    pub fn builder(request: impl ToString) -> ClientBuilder {
        ClientBuilder::new(request)
    }
    /// Send a ping to the server
    pub fn ping(&mut self) -> Result<Sender<'_>> {
        self.send(SurrealRequest::ping())
    }
    /// Send a query to the server
    pub fn query(
        &mut self,
        query: impl ToString,
        params: BTreeMap<String, serde_json::Value>,
    ) -> Result<Sender<'_>> {
        self.send(SurrealRequest::query(query.to_string(), params))
    }
    /// Set the namespace and database
    pub fn use_ns_db(&mut self, ns: impl ToString, db: impl ToString) -> Result<Sender<'_>> {
        self.send(SurrealRequest::use_ns_db(ns.to_string(), db.to_string()))
    }
    /// Sign in to an account on the server
    pub fn sign_in(&mut self, username: impl ToString, password: impl ToString) -> Result<Sender<'_>> {
        self.send(SurrealRequest::sign_in(username, password))
    }

    fn send(&mut self, request: SurrealRequest) -> Result<Sender<'_>> {
        let (response_sink, response_stream) = oneshot::channel();
        self.response_channel_sink
            .send((request.id().to_string(), response_sink))
            .map_err(|_| Error::ClientShutdown)?;
        let message = serde_json::to_string(&request)?;
        let send = self.socket_sink.send(Message::Text(message));
        Ok(Sender {
            send,
            receiver: Receiver { response_stream },
        })
    }
}

/// A sender is a request which has been built, and is ready to send with `.send()`
pub struct Sender<'a> {
    send: SinkSend<'a, SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>, Message>,
    receiver: Receiver,
}
impl<'a> Sender<'a> {
    /// Sends the request when awaited
    pub async fn send(self) -> Result<Receiver> {
        self.send.await?;
        Ok(self.receiver)
    }
}

/// A receiver is the result of a sent request, and is ready to receive data with `.response()`
pub struct Receiver {
    response_stream: oneshot::Receiver<SurrealRawResponse>,
}
impl Receiver {
    /// Awaits a response from the server when awaited
    pub async fn response(self) -> Result<Value> {
        Ok(self
            .response_stream
            .await
            .map_err(|_| Error::ClientShutdown)?
            .transpose()?)
    }
}

/// Builder pattern constructor for [`Client`]
#[derive(Clone)]
pub struct ClientBuilder {
    request: String,
    err_handler: Arc<dyn ErrorHandler + Send + Sync>,
    pre_sign_in: Option<(String, String)>,
    pre_ns_db: Option<(String, String)>,
}

pub trait ErrorHandler {
    fn handle_error(&self, error: Error) -> ClientAction;
}

impl<T> ErrorHandler for T
where
    T: Fn(Error) -> ClientAction,
{
    fn handle_error(&self, error: Error) -> ClientAction {
        self(error)
    }
}
impl ErrorHandler for () {
    fn handle_error(&self, _error: Error) -> ClientAction {
        ClientAction::IgnoreError
    }
}
impl ErrorHandler for Arc<dyn ErrorHandler> {
    fn handle_error(&self, error: Error) -> ClientAction {
        self.deref().handle_error(error)
    }
}

impl ClientBuilder {
    /// Create a new client which will connect to a surrealdb websocket
    ///
    /// ```ignore
    /// let client = ClientBuilder::new("ws://0.0.0.0:8000/rpc").build().await.unwrap()
    /// ```
    pub fn new(request: impl ToString) -> ClientBuilder {
        ClientBuilder {
            request: request.to_string(),
            err_handler: Arc::new(()),
            pre_sign_in: None,
            pre_ns_db: None,
        }
    }

    /// Set an error handler which will be called if the client receives an error message from the
    /// websocket for which it cannot identify which query is being responded to.
    pub fn with_err_handler<F: ErrorHandler + Send + Sync + 'static>(
        self,
        err_handler: F,
    ) -> ClientBuilder {
        ClientBuilder {
            request: self.request,
            err_handler: Arc::new(err_handler),
            pre_sign_in: None,
            pre_ns_db: None,
        }
    }
    
    /// Set the sign in configuration
    pub fn sign_in(mut self, username: impl ToString, password: impl ToString) -> Self {
        self.pre_sign_in = Some((username.to_string(), password.to_string()));
        self
    }
    
    /// Set the namespace configuration
    pub fn ns_db(mut self, ns: impl ToString, db: impl ToString) -> Self {
        self.pre_ns_db = Some((ns.to_string(), db.to_string()));
        self
    }

    #[cfg(feature = "pool")]
    /// Create a client pool that will initialise new clients with the builder
    pub fn build_pool(self) -> crate::pool::Pool {
        crate::pool::Pool::new(self)
    }

    /// Create the client
    pub async fn build(self) -> Result<Client> {
        let mut client = make_client(self.request, self.err_handler).await?;
        if let Some((username, password)) = self.pre_sign_in {
            client.sign_in(username, password)?.send().await?.response().await?;
        }
        if let Some((ns, db)) = self.pre_ns_db {
            client.use_ns_db(ns, db)?.send().await?.response().await?;
        }
        Ok(client)
    }
}

async fn make_client(
    request: impl ToString,
    err_handler: Arc<dyn ErrorHandler + Send + Sync>,
) -> Result<Client> {
    // Connect to the websocket
    let (socket, _) = connect_async(request.to_string()).await?;

    // Split the websocket into a sink, which messages will be sent into, and a stream, which
    // responses will be returned from.
    let (socket_sink, mut socket_stream) = socket.split();

    // Create a channel which will send response channels to our task
    let (response_channel_sink, mut response_channel_stream) =
        mpsc::unbounded_channel::<(String, oneshot::Sender<SurrealRawResponse>)>();

    let task = tokio::spawn(async move {
        let mut response_sinks = BTreeMap::new();
        let mut pending_sending = BTreeMap::new();
        loop {
            tokio::select! {
                response_channel = response_channel_stream.recv() => {
                    if let Some((id, response_sink)) = response_channel {
                        if let Some(response) = pending_sending.remove(&id) {
                            // if we already had data waiting for this sink, send it
                            let _ = response_sink.send(response);
                        } else {
                            // otherwise, add the sink to the list of sinks we are waiting for a
                            // response for
                            response_sinks.insert(id, response_sink);
                        }
                    } else {
                        return;
                    }
                }
                response = socket_stream.next() => {
                    match response {
                        Some(response) => {
                            if let Message::Text(response) = handle_err!(err_handler, response) {
                                let response = handle_err!(
                                    err_handler,
                                    serde_json::from_str::<SurrealRawResponse>(&response)
                                );
                                let response_sink = match response_sinks.remove(&response.id) {
                                    Some(response_sink) => response_sink,
                                    None => {
                                        // if there's no response sink, then pend the data
                                        pending_sending.insert(response.id.clone(), response);
                                        continue;
                                    },
                                };
                                // if this fails it is because the receiver dropped, which a user is
                                // free to do if they wish
                                let _ = response_sink.send(response);
                            }
                        },
                        None => {
                            return;
                        }
                    }
                }
            }
        }
    });

    Ok(Client {
        response_channel_sink,
        socket_sink,
        _task: task,
    })
}

/// Actions that can be taken on the client in response to an error.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq)]
pub enum ClientAction {
    /// Perform no client action, silently ignoring the error.
    #[default]
    IgnoreError,
    /// Shut down the client, causing all outstanding and future queries to receive a websocket
    /// error.
    Shutdown,
}
