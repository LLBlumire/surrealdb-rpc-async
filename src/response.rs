use serde::Deserialize;
use serde_json::Value;

use crate::error::{Error, Result};

/// A response received from the websocket server
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct SurrealRawResponse {
    pub id: String,
    #[serde(flatten)]
    pub result: SurrealRawResult,
}
impl SurrealRawResponse {
    pub fn transpose(self) -> Result<Value> {
        match self {
            SurrealRawResponse {
                result: SurrealRawResult::Result(value),
                ..
            } => Ok(value),
            SurrealRawResponse {
                result: SurrealRawResult::Error { code, message },
                ..
            } => Err(Error::SurrealError { code, message }),
        }
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SurrealRawResult {
    Result(Value),
    Error { code: i64, message: String },
}
