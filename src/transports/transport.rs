//! Transport trait for JSON-RPC 2.0 communication.
//!
//! This module defines the common interface that all transport implementations
//! must support for JSON-RPC communication.

use crate::error::Error;
use crate::methods::Methods;

/// Trait defining the interface for JSON-RPC transports.
///
/// A transport is responsible for serving JSON-RPC messages using its
/// specific communication mechanism. Different transport implementations
/// can support different patterns:
///
/// - **Stdio**: Continuous stream of newline-delimited JSON over stdin/stdout
/// - **HTTP**: Request/response pattern with HTTP POST
/// - **InMemory**: In-memory channel for testing
///
/// The `serve` method contains all the logic for receiving messages,
/// processing them through the method registry, and sending responses.
/// This allows each transport to implement the pattern that best suits
/// its communication mechanism.
///
/// # Example
///
/// ```no_run
/// use json_rpc::{Methods, Stdio, Transport};
/// use serde_json::Value;
///
/// async fn echo(params: Value) -> Result<Value, json_rpc::Error> {
///     Ok(params)
/// }
///
/// # tokio::runtime::Runtime::new().unwrap().block_on(async {
/// let methods = Methods::new().add("echo", echo);
/// let transport = Stdio::new();
/// transport.serve(methods).await.unwrap();
/// # });
/// ```
pub trait Transport {
    /// Serve the JSON-RPC server with the given methods.
    ///
    /// This method starts the server and runs until shutdown or an error occurs.
    /// The transport is responsible for:
    ///
    /// 1. Receiving incoming messages according to its communication pattern
    /// 2. Parsing and validating JSON-RPC messages
    /// 3. Routing requests to the appropriate method handlers
    /// 4. Sending responses back through the same communication channel
    ///
    /// # Arguments
    ///
    /// * `methods` - The method registry containing all registered JSON-RPC methods
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` when the server shuts down gracefully, or an error if
    /// the server encounters a fatal error.
    ///
    /// # Note
    ///
    /// This method takes `self` by value, which means the transport is consumed
    /// when serving starts. This allows the transport to manage its resources
    /// (like file handles, sockets, etc.) as needed.
    fn serve(self, methods: Methods)
    -> impl std::future::Future<Output = Result<(), Error>> + Send;
}
