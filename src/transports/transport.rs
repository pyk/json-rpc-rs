//! Transport trait for JSON-RPC 2.0 communication.
//!
//! This module defines the common interface that all transport implementations
//! must support for JSON-RPC communication.

use crate::error::Error;
use crate::jsonrpc::types::{Message, Notification, Request, Response};

/// Trait defining the interface for JSON-RPC transports.
///
/// A transport is responsible for sending and receiving JSON-RPC messages.
/// Different transport implementations can support different communication
/// mechanisms (stdio, TCP, WebSocket, in-memory, etc.).
pub trait Transport {
    /// Receive a JSON-RPC message from the transport.
    ///
    /// This method should block until a complete message is received,
    /// or return an error if the transport is closed or encounters
    /// an error.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The transport is closed
    /// - An I/O error occurs
    /// - The message is malformed
    fn receive_message(&mut self) -> Result<Message, Error>;

    /// Send a JSON-RPC request through the transport.
    ///
    /// Serializes and sends the request according to the transport's
    /// wire format (e.g., newline-delimited JSON for stdio).
    ///
    /// # Errors
    ///
    /// Returns an error if serialization or sending fails.
    fn send_request(&mut self, request: &Request) -> Result<(), Error>;

    /// Send a JSON-RPC response through the transport.
    ///
    /// Serializes and sends the response according to the transport's
    /// wire format.
    ///
    /// # Errors
    ///
    /// Returns an error if serialization or sending fails.
    fn send_response(&mut self, response: &Response) -> Result<(), Error>;

    /// Send a JSON-RPC notification through the transport.
    ///
    /// Serializes and sends the notification according to the transport's
    /// wire format.
    ///
    /// # Errors
    ///
    /// Returns an error if serialization or sending fails.
    fn send_notification(&mut self, notification: &Notification) -> Result<(), Error>;
}
