//! Error types for the JSON-RPC implementation.
//!
//! This module defines internal errors that can occur during JSON-RPC processing,
//! distinct from the JSON-RPC wire format errors defined in the `types` module.

use std::io;

/// Internal errors that can occur during JSON-RPC processing.
///
/// These are implementation-level errors, separate from the JSON-RPC protocol
/// error objects that are sent over the wire (defined in `types::Error`).
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Protocol-level error, such as invalid method or parameters.
    #[error("Protocol error: {0}")]
    ProtocolError(String),

    /// Transport I/O error.
    #[error("Transport error: {0}")]
    TransportError(#[from] io::Error),

    /// JSON parsing error.
    #[error("JSON parse error: {0}")]
    ParseError(#[from] serde_json::Error),

    /// Operation was cancelled.
    #[error("Operation was cancelled")]
    Cancelled,
}

impl Error {
    /// Create a new protocol error.
    pub fn protocol(message: impl Into<String>) -> Self {
        Self::ProtocolError(message.into())
    }

    /// Create a new transport error.
    pub fn transport(error: impl Into<io::Error>) -> Self {
        Self::TransportError(error.into())
    }
}
