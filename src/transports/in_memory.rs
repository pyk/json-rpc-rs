//! In-memory transport for JSON-RPC 2.0.
//!
//! This module implements an in-memory transport for JSON-RPC 2.0 communication
//! within the same process. It uses async channels for message passing and is primarily
//! useful for testing and in-process communication scenarios.

use tokio::sync::mpsc;

use crate::Methods;
use crate::error::Error;
use crate::transports::Transport;

/// In-memory transport for JSON-RPC messages.
///
/// This transport uses async channels for message passing, allowing JSON-RPC communication
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
/// use json_rpc::Methods;
/// use serde_json::Value;
///
/// async fn echo(params: Value) -> Result<Value, json_rpc::Error> {
///     Ok(params)
/// }
///
/// let methods = Methods::new().add("echo", echo);
///
/// // Create a pair of connected transports
/// let (transport_a, transport_b) = InMemory::pair();
///
/// // Start the server on one transport
/// tokio::spawn(async move {
///     transport_a.serve(methods).await.unwrap();
/// });
///
/// // Send requests from the other transport
/// let request = r#"{"jsonrpc":"2.0","method":"echo","params":"hello","id":1}"#;
/// let response = transport_b.send_and_receive(request).await.unwrap();
/// println!("{}", response);
/// ```
pub struct InMemory {
    receiver: mpsc::Receiver<String>,
    sender: mpsc::Sender<String>,
}

impl InMemory {
    /// Create a new in-memory transport with the given sender and receiver.
    ///
    /// This is typically used by the `pair()` method, but can be used directly
    /// if you need to connect to existing channels.
    pub fn new(receiver: mpsc::Receiver<String>, sender: mpsc::Sender<String>) -> Self {
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
        let (sender_a, receiver_a) = mpsc::channel(128);
        let (sender_b, receiver_b) = mpsc::channel(128);

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
    /// Note that if you try to receive from this transport, it will wait indefinitely
    /// until a message is sent via the returned sender.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use json_rpc::transports::in_memory::InMemory;
    /// use json_rpc::Methods;
    /// use serde_json::Value;
    ///
    /// async fn echo(params: Value) -> Result<Value, json_rpc::Error> {
    ///     Ok(params)
    /// }
    ///
    /// let methods = Methods::new().add("echo", echo);
    /// let (transport, sender) = InMemory::unconnected();
    ///
    /// // Start the server
    /// tokio::spawn(async move {
    ///     transport.serve(methods).await.unwrap();
    /// });
    ///
    /// // Send requests
    /// sender.send(r#"{"jsonrpc":"2.0","method":"echo","params":"hello","id":1}"#.to_string()).await.unwrap();
    /// ```
    pub fn unconnected() -> (Self, mpsc::Sender<String>) {
        let (sender, receiver) = mpsc::channel(128);
        let transport = Self::new(receiver, sender.clone());
        (transport, sender)
    }

    /// Get a reference to the sender channel.
    ///
    /// This can be used to clone the sender if you need multiple endpoints
    /// to send messages to this transport.
    pub fn sender(&self) -> &mpsc::Sender<String> {
        &self.sender
    }

    /// Get a reference to the receiver channel.
    ///
    /// This can be used to clone the receiver if you need multiple endpoints
    /// to receive messages from this transport.
    pub fn receiver(&self) -> &mpsc::Receiver<String> {
        &self.receiver
    }

    /// Send a JSON-RPC request and wait for a response.
    ///
    /// This helper method is useful for testing when you want to send a request
    /// and wait for the response in a single call.
    ///
    /// # Arguments
    ///
    /// * `request` - The JSON-RPC request as a string
    ///
    /// # Returns
    ///
    /// Returns the JSON-RPC response as a string, or an error if the channel is closed.
    pub async fn send_and_receive(&mut self, request: &str) -> Result<String, Error> {
        self.sender.send(request.to_string()).await.map_err(|_| {
            Error::TransportError(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "Receiver disconnected",
            ))
        })?;

        self.receiver.recv().await.ok_or_else(|| {
            Error::TransportError(std::io::Error::new(
                std::io::ErrorKind::ConnectionReset,
                "Sender disconnected",
            ))
        })
    }
}

impl Transport for InMemory {
    /// Serve the JSON-RPC server using in-memory transport.
    ///
    /// This method runs in a loop, receiving messages from the in-memory channel,
    /// processing each message through the method registry, and sending
    /// responses back through the response channel.
    ///
    /// The server runs until the sender is disconnected.
    ///
    /// # Arguments
    ///
    /// * `methods` - The method registry containing all registered JSON-RPC methods
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` when the sender is disconnected, or an error if a fatal error occurs.
    async fn serve(mut self, methods: Methods) -> Result<(), Error> {
        loop {
            // Receive a message from the channel
            let request = match self.receiver.recv().await {
                Some(msg) => msg,
                None => {
                    // Sender disconnected
                    break;
                }
            };

            // Process the message through the method registry
            if let Some(response) = methods.process_message(&request).await {
                // Send the response back through the channel
                if let Err(_) = self.sender.send(response).await {
                    // Receiver disconnected
                    break;
                }
            }
        }

        Ok(())
    }
}
