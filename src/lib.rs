//! JSON-RPC 2.0 implementation with a builder pattern API.
//!
//! This module provides a simple, flexible JSON-RPC 2.0 implementation
//! that uses a builder pattern for server configuration and method registration.
//! It handles the wire format (transport), message parsing, and method routing.
//!
//! # Architecture
//!
//! The json-rpc crate is organized into five main components:
//!
//! - [`types`] - JSON-RPC 2.0 message types (Request, Response, Notification, Error)
//! - [`transports`] - I/O handling with multiple transport implementations (Stdio, InMemory)
//! - [`server`] - Server with builder pattern and thread pool for concurrent request handling
//! - [`shutdown`] - Shutdown signal for graceful server shutdown
//! - [`cancellation`] - Cancellation token for request cancellation
//! - [`error`] - Internal error types for the implementation
//!
//! # Quick Start
//!
//! To implement a JSON-RPC server using this module:
//!
//! ## 1. Create a Server and Register Methods
//!
//! ```no_run
//! use json_rpc::{Server, Error};
//!
//! // Create a new server
//! let mut server = Server::new();
//!
//! // Register methods with type-safe parameters
//! server.register("add", |params: (i32, i32)| {
//!     Ok(params.0 + params.1)
//! })?;
//!
//! server.register("echo", |params: String| {
//!     Ok(params)
//! })?;
//! # Ok::<(), Error>(())
//! ```
//!
//! ## 2. Configure the Server (Optional)
//!
//! ```no_run
//! use json_rpc::{Server, ShutdownSignal, Error};
//!
//! let shutdown = ShutdownSignal::new();
//!
//! let mut server = Server::new()
//!     .with_thread_pool_size(4)
//!     .with_shutdown_signal(shutdown);
//!
//! server.register("add", |params: (i32, i32)| {
//!     Ok(params.0 + params.1)
//! })?;
//! # Ok::<(), Error>(())
//! ```
//!
//! ## 3. Run the Server
//!
//! ```no_run
//! use json_rpc::{Server, Error};
//!
//! let mut server = Server::new();
//! server.register("echo", |params: String| Ok(params))?;
//!
//! // Run with default Stdio transport
//! server.run()?;
//! # Ok::<(), Error>(())
//! ```
//!
//! # Using Custom Transports
//!
//! You can run the server with any transport that implements the [`Transport`] trait:
//!
//! ```no_run
//! use json_rpc::{Server, InMemory, Error};
//!
//! let mut server = Server::new();
//! server.register("echo", |params: String| Ok(params))?;
//!
//! // Run with InMemory transport
//! let (transport, _sender) = InMemory::unconnected();
//! server.run_with_transport(transport)?;
//! # Ok::<(), Error>(())
//! ```
//!
//! # Struct Parameters
//!
//! Methods can use struct parameters for more complex APIs:
//!
//! ```no_run
//! use json_rpc::{Server, Error};
//! use serde::Deserialize;
//!
//! #[derive(Deserialize)]
//! struct InitializeParams {
//!     name: String,
//!     version: String,
//! }
//!
//! let mut server = Server::new();
//!
//! server.register("initialize", |params: InitializeParams| {
//!     Ok(format!("Server {} v{} initialized", params.name, params.version))
//! })?;
//! # Ok::<(), Error>(())
//! ```
//!
//! # Graceful Shutdown
//!
//! Use a shutdown signal to gracefully stop the server:
//!
//! ```no_run
//! use json_rpc::{Server, ShutdownSignal, Error};
//! use std::thread;
//! use std::time::Duration;
//!
//! let shutdown = ShutdownSignal::new();
//!
//! let mut server = Server::new()
//!     .with_shutdown_signal(shutdown.clone());
//!
//! server.register("shutdown", |_params: ()| {
//!     // Signal shutdown from within a handler
//!     // shutdown.signal(); // This would need access to shutdown
//!     Ok("Shutting down".to_string())
//! })?;
//!
//! // In a real application, you'd spawn the server in a thread
//! // and signal shutdown from another thread or signal handler
//! thread::spawn(move || {
//!     thread::sleep(Duration::from_secs(5));
//!     shutdown.signal();
//! });
//! # Ok::<(), Error>(())
//! ```
//!
//! # Layer Responsibilities
//!
//! | Component | Responsibility |
//! |-----------|----------------|
//! | `types` | JSON-RPC message structures and serialization |
//! | `transport` | Reading/writing bytes to the wire |
//! | `server` | Method registration, request handling, thread pool |
//! | `shutdown` | Graceful shutdown signaling |
//! | `cancellation` | Request cancellation |
//! | `error` | Error types and handling |
//!
//! # Protocol vs Transport
//!
//! This module handles the JSON-RPC protocol layer, not the transport layer.
//! The [`Transport`] trait defines the interface for all transport implementations.
//! Currently, the following transports are provided:
//!
//! - [`Stdio`] - stdio-based NDJSON (newline-delimited JSON) transport
//! - [`InMemory`] - in-memory transport for testing and in-process communication
//!
//! You can implement custom transports (TCP, WebSocket, etc.) by implementing
//! the [`Transport`] trait.
//!
//! # Thread Pool
//!
//! The server uses a fixed-size thread pool for concurrent request handling:
//!
//! - Default size: Number of CPU cores
//! - Configurable via `.with_thread_pool_size()`
//! - Each request is processed in a worker thread
//! - Responses are sent back to the main thread for transmission
//!
//! # Error Handling
//!
//! Methods return `Result<T, Error>`, where `Error` is an enum with these variants:
//!
//! - `ProtocolError` - Protocol-level errors (invalid method, etc.)
//! - `TransportError` - I/O errors from the transport layer
//! - `ParseError` - JSON parsing errors
//! - `Cancelled` - Operation was cancelled via CancellationToken

pub use cancellation::CancellationToken;
pub use error::Error;
pub use server::Server;
pub use shutdown::ShutdownSignal;
pub use transports::{InMemory, Stdio, Transport};
pub use types::{Message, Notification, Request, RequestId, Response};

pub mod cancellation;
pub mod error;
pub mod server;
pub mod shutdown;
pub mod transports;
pub mod types;
