use thiserror::Error;
use tokio_tungstenite::tungstenite::Message;

use crate::response::SurrealRawResponse;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    #[error(transparent)]
    Websocket(#[from] tokio_tungstenite::tungstenite::Error),
    #[error(transparent)]
    Json(#[from] serde_json::error::Error),
    #[error("Client was shut down and can no longer send messages")]
    ClientShutdown,
    #[error("The server behaved in an unexpected fashion")]
    UnexpectedServerBehaviour(Message),
    #[error("The server sent a response, but not for an ID a request was sent for")]
    ResponseForUnknownID(SurrealRawResponse),
    #[error("{message}")]
    SurrealError { code: i64, message: String },
}
