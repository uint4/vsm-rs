//! Crate-owned cooperative cancellation primitive.
//!
//! The trait-driven runtime passes cancellation state through role contexts
//! without exposing an external cancellation-token dependency in public APIs.

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use crate::error::{FrameworkError, WorkError};
use crate::roles::ViableSystem;

/// A cheap, cloneable cooperative cancellation flag.
#[derive(Clone, Debug, Default)]
pub struct CancellationToken {
    cancelled: Arc<AtomicBool>,
}

impl CancellationToken {
    /// Creates a token in the active state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Requests cancellation for all clones of this token.
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    /// Returns `true` after cancellation has been requested.
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }

    /// Converts the current cancellation state into a framework error.
    pub fn check(&self) -> Result<(), FrameworkError> {
        if self.is_cancelled() {
            Err(FrameworkError::Cancelled)
        } else {
            Ok(())
        }
    }

    /// Converts the current cancellation state into a typed work error.
    pub fn check_work<V>(&self) -> Result<(), WorkError<V::AppError>>
    where
        V: ViableSystem,
    {
        self.check().map_err(WorkError::from)
    }
}
