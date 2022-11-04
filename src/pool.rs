use std::{
    collections::VecDeque,
    ops::{Deref, DerefMut},
    sync::Arc,
};

use tokio::sync::Mutex;

use crate::{error::Result, Client, ClientBuilder};

#[derive(Clone)]
pub struct Pool {
    connections: Arc<Mutex<VecDeque<Client>>>,
    builder: ClientBuilder,
}

impl Pool {
    pub(crate) fn new(builder: ClientBuilder) -> Pool {
        Pool {
            connections: Default::default(),
            builder,
        }
    }

    pub async fn get(&self) -> Result<PooledClient> {
        let pool = self.connections.clone();
        if let Some(client) = self.connections.lock().await.pop_front() {
            Ok(PooledClient {
                client: Some(client),
                pool,
            })
        } else {
            let client = self.builder.clone().build().await?;
            Ok(PooledClient {
                client: Some(client),
                pool,
            })
        }
    }
}

pub struct PooledClient {
    client: Option<Client>,
    pool: Arc<Mutex<VecDeque<Client>>>,
}
impl Drop for PooledClient {
    fn drop(&mut self) {
        let pool = self.pool.clone();
        // SAFETY: the pool is only None during dropping, and drop cannot happen twice.
        let client = unsafe { self.client.take().unwrap_unchecked() };

        tokio::spawn(async move { pool.lock().await.push_back(client) });
    }
}
impl Deref for PooledClient {
    type Target = Client;

    fn deref(&self) -> &Self::Target {
        // SAFETY: the pool is only None during dropping, and deref is not called during drop.
        unsafe { self.client.as_ref().unwrap_unchecked() }
    }
}
impl DerefMut for PooledClient {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // SAFETY: the pool is only None during dropping, and deref is not called during drop.
        unsafe { self.client.as_mut().unwrap_unchecked() }
    }
}
