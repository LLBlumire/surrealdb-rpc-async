//! # SurrealDB Websocket Sync Client
//!
//! This crate provides a simple websocket based client for connecting to a SurrealDB database.
//!
//! It has intentionally been designed around simplicity, and so does not implement all possible
//! surreal operations, for this one will need to wait for the official client to be released.
//!
//! What it does support is queries, and their responses, as well as providing a simple connection
//! pool (feature `pool`).

mod client;
pub mod error;
mod request;
pub(crate) mod response;

#[cfg(feature = "pool")]
pub mod pool;

pub use client::Client;
pub use client::ClientAction;
pub use client::ClientBuilder;
