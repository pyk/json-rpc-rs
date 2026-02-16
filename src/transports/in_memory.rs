//! In-memory transport for JSON-RPC 2.0.
//!
//! This module implements an in-memory transport for JSON-RPC 2.0 communication
//! within the same process. It uses channels for message passing and is primarily
//! useful for testing and in-process communication scenarios.

use std::sync::mpsc::{self, Receiver, Sender};

use crate::error::Error;
use crate::jsonrpc::transports::Transport;
use crate::jsonrpc::types::{Message, Notification, Request, Response};

/// In-memory transport for JSON-RPC messages.
///
/// This transport uses channels for message passing, allowing JSON-RPC communication
/// between different parts of the same process. It is primarily useful for:
///
/// - Testing JSON-RPC handlers without I/O
/// - In-process communication between components
/// - Mock implementations for development
///
/// # Example
///
/// ```no_run
/// use acp::jsonrpc::transports::in_memory::InMemory;
///
/// // Create a pair of connected transports
/// let (transport_a, transport_b) = InMemory::pair();
///
/// // transport_a and transport_b can now communicate with each other
/// ```
pub struct InMemory {
    receiver: Receiver<String>,
    sender: Sender<String>,
}

impl InMemory {
    /// Create a new in-memory transport with the given sender and receiver.
    ///
    /// This is typically used by the `pair()` method, but can be used directly
    /// if you need to connect to existing channels.
    ///
    /// # Arguments
    ///
    /// * `receiver` - Channel receiver for incoming messages
    /// * `sender` - Channel sender for outgoing messages
    pub fn new(receiver: Receiver<String>, sender: Sender<String>) -> Self {
        Self { receiver, sender }
    }

    /// Create a pair of connected in-memory transports.
    ///
    /// Returns two transport instances that are connected to each other.
    /// Messages sent on transport_a will be received by transport_b, and vice versa.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use acp::jsonrpc::transports::in_memory::InMemory;
    ///
    /// let (transport_a, transport_b) = InMemory::pair();
    ///
    /// // Messages sent from transport_a are received by transport_b
    /// // and messages sent from transport_b are received by transport_a
    /// ```
    pub fn pair() -> (Self, Self) {
        let (sender_a, receiver_a) = mpsc::channel();
        let (sender_b, receiver_b) = mpsc::channel();

        let transport_a = Self::new(receiver_b, sender_a);
        let transport_b = Self::new(receiver_a, sender_b);

        (transport_a, transport_b)
    }

    /// Create a new unconnected in-memory transport.
    ///
    /// This creates a transport with its own channel. Returns the transport and a
    /// sender that can be used to send messages to it. This is primarily useful
    /// for scenarios where you want to manually control message sending to the transport.
    ///
    /// # Arguments
    ///
    /// * Returns a tuple of (transport, sender) where:
    ///   - transport: The in-memory transport that will receive messages
    ///   - sender: A cloned sender that can be used to send messages to the transport
    ///
    /// Note that if you try to receive from this transport, it will block indefinitely
    /// until a message is sent via the returned sender.
    pub fn unconnected() -> (Self, Sender<String>) {
        let (sender, receiver) = mpsc::channel();
        let transport = Self::new(receiver, sender.clone());
        (transport, sender)
    }

    /// Get a reference to the sender channel.
    ///
    /// This can be used to clone the sender if you need multiple endpoints
    /// to send messages to this transport.
    pub fn sender(&self) -> &Sender<String> {
        &self.sender
    }

    /// Get a reference to the receiver channel.
    ///
    /// This can be used to clone the receiver if you need multiple endpoints
    /// to receive messages from this transport.
    pub fn receiver(&self) -> &Receiver<String> {
        &self.receiver
    }
}

impl Transport for InMemory {
    /// Receive a JSON-RPC message from the in-memory channel.
    ///
    /// This method blocks until a message is available on the receiver channel.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The sender has been disconnected and no messages remain
    /// - The message is malformed JSON
    fn receive_message(&mut self) -> Result<Message, Error> {
        let json_str = self.receiver.recv().map_err(|_| {
            Error::TransportError(std::io::Error::new(
                std::io::ErrorKind::ConnectionReset,
                "Channel sender disconnected",
            ))
        })?;

        let value: serde_json::Value = serde_json::from_str(&json_str).map_err(|e| {
            Error::TransportError(std::io::Error::new(std::io::ErrorKind::InvalidData, e))
        })?;

        Message::from_json(value).map_err(Error::from)
    }

    /// Send a JSON-RPC request through the in-memory channel.
    ///
    /// Serializes the request and sends it through the sender channel.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Serialization fails
    /// - The receiver has been disconnected
    fn send_request(&mut self, request: &Request) -> Result<(), Error> {
        let json = serde_json::to_string(request).map_err(|e| {
            Error::TransportError(std::io::Error::new(std::io::ErrorKind::InvalidData, e))
        })?;

        self.sender.send(json).map_err(|_| {
            Error::TransportError(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "Channel receiver disconnected",
            ))
        })
    }

    /// Send a JSON-RPC response through the in-memory channel.
    ///
    /// Serializes the response and sends it through the sender channel.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Serialization fails
    /// - The receiver has been disconnected
    fn send_response(&mut self, response: &Response) -> Result<(), Error> {
        let json = serde_json::to_string(response).map_err(|e| {
            Error::TransportError(std::io::Error::new(std::io::ErrorKind::InvalidData, e))
        })?;

        self.sender.send(json).map_err(|_| {
            Error::TransportError(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "Channel receiver disconnected",
            ))
        })
    }

    /// Send a JSON-RPC notification through the in-memory channel.
    ///
    /// Serializes the notification and sends it through the sender channel.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Serialization fails
    /// - The receiver has been disconnected
    fn send_notification(&mut self, notification: &Notification) -> Result<(), Error> {
        let json = serde_json::to_string(notification).map_err(|e| {
            Error::TransportError(std::io::Error::new(std::io::ErrorKind::InvalidData, e))
        })?;

        self.sender.send(json).map_err(|_| {
            Error::TransportError(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "Channel receiver disconnected",
            ))
        })
    }
}
