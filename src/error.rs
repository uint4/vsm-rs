//! Error and result types shared by the public API.
//!
//! `VsmError` maps actor lookup failures, channel failures, validation errors,
//! serialization errors, and supervisor/runtime failures into one serializable
//! enum. Some domain outcomes are not errors; for example System 1 transaction
//! failures such as `NoSuitableUnit` are represented by `TransactionResult`.

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

/// Framework-owned failures in the trait-driven runtime surface.
///
/// These errors describe runtime mechanics such as admission, protocol
/// validation, cancellation, persistence, and shutdown. Application/domain
/// failures are preserved separately through [`ApplicationFailure`].
#[derive(Debug, Clone, PartialEq, Eq, Error, Serialize, Deserialize)]
pub enum FrameworkError {
    #[error("target unavailable: {target}")]
    Unavailable { target: String },

    #[error("operation timed out: {operation}")]
    Timeout { operation: String },

    #[error("invalid protocol: {reason}")]
    InvalidProtocol { reason: String },

    #[error("admission rejected by backpressure: {reason}")]
    Backpressured { reason: String },

    #[error("persistence failure: {reason}")]
    Persistence { reason: String },

    #[error("runtime is shutting down")]
    Shutdown,

    #[error("operation was cancelled")]
    Cancelled,

    #[error("snapshot is incompatible for {key}: {reason}")]
    SnapshotIncompatible { key: String, reason: String },

    #[error("snapshot was rejected for {key}: {reason}")]
    SnapshotRejected { key: String, reason: String },

    #[error("runtime failure: {reason}")]
    Runtime { reason: String },
}

impl From<FrameworkError> for VsmError {
    fn from(value: FrameworkError) -> Self {
        match value {
            FrameworkError::Unavailable { target } => Self::ActorUnavailable(target),
            FrameworkError::Timeout { operation } => {
                Self::Runtime(format!("operation timed out: {operation}"))
            }
            FrameworkError::InvalidProtocol { reason } => Self::Validation(reason),
            FrameworkError::Backpressured { reason } => Self::Runtime(reason),
            FrameworkError::Persistence { reason } => Self::Runtime(reason),
            FrameworkError::Shutdown => Self::Runtime("runtime is shutting down".to_string()),
            FrameworkError::Cancelled => Self::Runtime("operation was cancelled".to_string()),
            FrameworkError::SnapshotIncompatible { key, reason } => {
                Self::Runtime(format!("snapshot is incompatible for {key}: {reason}"))
            }
            FrameworkError::SnapshotRejected { key, reason } => {
                Self::Runtime(format!("snapshot was rejected for {key}: {reason}"))
            }
            FrameworkError::Runtime { reason } => Self::Runtime(reason),
        }
    }
}

/// Application-owned failure classification preserved by framework results.
#[derive(Debug, Error)]
pub enum ApplicationFailure<E>
where
    E: std::error::Error + Send + Sync + 'static,
{
    #[error("application rejected work: {0}")]
    Rejected(E),

    #[error("application failed work: {0}")]
    Failed(E),
}

/// Work execution failure with framework and application causes kept separate.
#[derive(Debug, Error)]
pub enum WorkError<E>
where
    E: std::error::Error + Send + Sync + 'static,
{
    #[error(transparent)]
    Application(#[from] ApplicationFailure<E>),

    #[error(transparent)]
    Framework(#[from] FrameworkError),
}
