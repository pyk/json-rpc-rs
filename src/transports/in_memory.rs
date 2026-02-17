//! In-memory transport for JSON-RPC 2.0.
//!
//! This module implements an in-memory transport for JSON-RPC 2.0 communication
//! within the same process. It uses channels for message passing and is primarily
//! useful for testing and in-process communication scenarios.

use std::sync::mpsc::{self, Receiver, Sender};

use crate::error::Error;
use crate::transports::Transport;

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
/// use json_rpc::transports::in_memory::InMemory;
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
    /// use json_rpc::transports::in_memory::InMemory;
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
    /// Receive a raw JSON string from the in-memory channel.
    ///
    /// This method blocks until a message is available on the receiver channel.
    /// No parsing or validation is performed - that's the responsibility
    /// of the caller (typically the server layer).
    fn receive_message(&mut self) -> Result<String, Error> {
        self.receiver.recv().map_err(|_| {
            Error::TransportError(std::io::Error::new(
                std::io::ErrorKind::ConnectionReset,
                "Channel sender disconnected",
            ))
        })
    }

    /// Send a raw JSON string through the in-memory channel.
    ///
    /// Sends the JSON string as-is without additional serialization.
    /// The caller is responsible for serializing JSON-RPC messages
    /// to JSON strings before calling this method.
    fn send_message(&mut self, json: &str) -> Result<(), Error> {
        self.sender.send(json.to_string()).map_err(|_| {
            Error::TransportError(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "Channel receiver disconnected",
            ))
        })
    }
}
