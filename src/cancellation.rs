//! Cancellation token for request cancellation.
//!
//! This module provides a `CancellationToken` that can be used to cancel
//! long-running operations in a thread-safe manner.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::error::Error;

/// A cancellation token that can be used to signal cancellation.
///
/// This token is thread-safe and can be cloned and shared across threads.
/// It uses an atomic boolean for efficient cancellation signaling.
#[derive(Debug, Clone)]
pub struct CancellationToken {
    inner: Arc<AtomicBool>,
}

impl CancellationToken {
    /// Create a new cancellation token that is not cancelled.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Check if the token is cancelled and return an error if so.
    ///
    /// This is a convenience method that returns an error immediately
    /// if cancellation has been requested, making it easy to propagate
    /// cancellation errors.
    pub fn check_cancelled(&self) -> Result<(), Error> {
        if self.is_cancelled() {
            Err(Error::Cancelled)
        } else {
            Ok(())
        }
    }

    /// Check if the token is cancelled.
    ///
    /// Returns `true` if cancellation has been requested, `false` otherwise.
    pub fn is_cancelled(&self) -> bool {
        self.inner.load(Ordering::SeqCst)
    }

    /// Cancel the token.
    ///
    /// This method signals that cancellation has been requested.
    /// All clones of this token will report as cancelled.
    pub fn cancel(&self) {
        self.inner.store(true, Ordering::SeqCst);
    }
}

impl Default for CancellationToken {
    fn default() -> Self {
        Self::new()
    }
}
