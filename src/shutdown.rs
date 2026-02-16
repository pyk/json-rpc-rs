//! Shutdown signal for graceful server shutdown.
//!
//! This module provides a `ShutdownSignal` that can be used to signal
//! a server to shut down gracefully.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::error::Error;

/// A shutdown signal that can be used to request graceful shutdown.
///
/// This signal is thread-safe and can be cloned and shared across threads.
/// It uses an atomic boolean for efficient shutdown signaling.
#[derive(Debug, Clone)]
pub struct ShutdownSignal {
    inner: Arc<AtomicBool>,
}

impl ShutdownSignal {
    /// Create a new shutdown signal that is not triggered.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Check if shutdown has been requested and return an error if so.
    ///
    /// This is a convenience method that returns an error immediately
    /// if shutdown has been requested, making it easy to propagate
    /// shutdown errors.
    pub fn check_shutdown(&self) -> Result<(), Error> {
        if self.is_shutdown_requested() {
            Err(Error::ProtocolError("Shutdown requested".to_string()))
        } else {
            Ok(())
        }
    }

    /// Check if shutdown has been requested.
    ///
    /// Returns `true` if shutdown has been requested, `false` otherwise.
    pub fn is_shutdown_requested(&self) -> bool {
        self.inner.load(Ordering::SeqCst)
    }

    /// Signal that shutdown should begin.
    ///
    /// This method signals that shutdown has been requested.
    /// All clones of this signal will report as shutdown requested.
    pub fn signal(&self) {
        self.inner.store(true, Ordering::SeqCst);
    }
}

impl Default for ShutdownSignal {
    fn default() -> Self {
        Self::new()
    }
}
