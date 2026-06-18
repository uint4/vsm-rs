use serde::{Deserialize, Serialize};
use thiserror::Error;

pub type VsmResult<T> = Result<T, VsmError>;

#[derive(Debug, Clone, Error, Serialize, Deserialize)]
pub enum VsmError {
    #[error("not found: {0}")]
    NotFound(String),

    #[error("already exists: {0}")]
    AlreadyExists(String),

    #[error("invalid input: {0}")]
    InvalidInput(String),

    #[error("invalid payload: {0}")]
    InvalidPayload(String),

    #[error("validation failed: {0}")]
    Validation(String),

    #[error("actor unavailable: {0}")]
    ActorUnavailable(String),

    #[error("actor not found: {0}")]
    ActorNotFound(String),

    #[error("unit already registered: {0}")]
    UnitAlreadyRegistered(String),

    #[error("supervisor error: {0}")]
    Supervisor(String),

    #[error("ractor error: {0}")]
    Ractor(String),

    #[error("channel error: {0}")]
    Channel(String),

    #[error("serialization error: {0}")]
    Serialization(String),

    #[error("runtime error: {0}")]
    Runtime(String),
}

impl From<serde_json::Error> for VsmError {
    fn from(value: serde_json::Error) -> Self {
        Self::Serialization(value.to_string())
    }
}
