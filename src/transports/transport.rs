//! Transport trait for JSON-RPC 2.0 communication.
//!
//! This module defines the common interface that all transport implementations
//! must support for JSON-RPC communication.

use crate::error::Error;
use crate::types::{Message, Notification, Request, Response};

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
    fn receive_message(&mut self) -> Result<Message, Error>;

    /// Send a JSON-RPC request through the transport.
    ///
    /// Serializes and sends the request according to the transport's
    /// wire format (e.g., newline-delimited JSON for stdio).
    fn send_request(&mut self, request: &Request) -> Result<(), Error>;

    /// Send a JSON-RPC response through the transport.
    ///
    /// Serializes and sends the response according to the transport's
    /// wire format.
    fn send_response(&mut self, response: &Response) -> Result<(), Error>;

    /// Send a JSON-RPC notification through the transport.
    ///
    /// Serializes and sends the notification according to the transport's
    /// wire format.
    fn send_notification(&mut self, notification: &Notification) -> Result<(), Error>;
}
