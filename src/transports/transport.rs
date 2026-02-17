//! Transport trait for JSON-RPC 2.0 communication.
//!
//! This module defines the common interface that all transport implementations
//! must support for JSON-RPC communication.

use crate::error::Error;

/// Trait defining the interface for JSON-RPC transports.
///
/// A transport is responsible for sending and receiving raw JSON strings.
/// Different transport implementations can support different communication
/// mechanisms (stdio, TCP, WebSocket, in-memory, etc.).
///
/// The transport layer does NOT handle JSON-RPC message parsing or validation.
/// It only handles I/O operations - reading and writing raw JSON strings.
pub trait Transport {
    /// Receive a raw JSON string from the transport.
    ///
    /// This method should block until a complete message is received,
    /// or return an error if the transport is closed or encounters
    /// an error.
    ///
    /// The returned string is a raw JSON string that needs to be
    /// parsed and validated by the caller (typically the server layer).
    fn receive_message(&mut self) -> Result<String, Error>;

    /// Send a raw JSON string through the transport.
    ///
    /// Sends the JSON string as-is according to the transport's
    /// wire format (e.g., newline-delimited JSON for stdio).
    ///
    /// The caller (typically the server layer) is responsible for
    /// serializing JSON-RPC messages to JSON strings before calling this method.
    fn send_message(&mut self, json: &str) -> Result<(), Error>;
}
