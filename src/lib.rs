//! A thread pool-based JSON-RPC 2.0 implementation using blocking I/O.
//!
//! This library provides a simple, user-friendly API for creating local JSON-RPC
//! servers. It handles the JSON-RPC protocol layer including message parsing,
//! method routing, and response generation. The server uses a thread pool for
//! concurrent request handling and supports graceful shutdown, request
//! cancellation, and multiple transport implementations.
//!
//! # Design Goals
//!
//! The library prioritizes simplicity and usability for local JSON-RPC servers.
//! It uses blocking I/O and a builder pattern for server configuration. Methods
//! register as closures with automatic parameter deserialization, making it
//! easy to define handlers that accept typed parameters.
//!
//! # Architecture
//!
//! The library is organized into several modules:
//!
//! [`types`] contains JSON-RPC 2.0 message types including Request, Response,
//! Notification, and Error. These structures handle serialization and
//! deserialization of JSON-RPC messages.
//!
//! [`transports`] defines the Transport trait and provides implementations. The
//! Stdio transport uses NDJSON (newline-delimited JSON) over stdin/stdout,
//! while InMemory is useful for testing. Custom transports can be implemented
//! by extending the Transport trait.
//!
//! [`server`] contains the Server type with a builder pattern API. It manages
//! method registration, request processing, and the thread pool for concurrent
//! request handling.
//!
//! [`shutdown`] provides the ShutdownSignal for graceful server shutdown.
//! Signal the shutdown from any thread to stop the server cleanly.
//!
//! [`cancellation`] provides the CancellationToken for cancelling in-flight
//! requests. This is useful when you need to abort long-running operations.
//!
//! [`error`] defines internal error types for implementation-level errors,
//! separate from JSON-RPC protocol errors sent over the wire.
//!
//! # Quick Start
//!
//! Create a server and register a method:
//!
//! ```no_run
//! use json_rpc::Server;
//!
//! let mut server = Server::new();
//!
//! server.register("add", |params: (i32, i32)| {
//!     Ok(params.0 + params.1)
//! })?;
//!
//! # Ok::<(), json_rpc::Error>(())
//! ```
//!
//! Run the server with the default Stdio transport:
//!
//! ```no_run
//! use json_rpc::Server;
//!
//! let mut server = Server::new();
//! server.register("echo", |params: String| Ok(params))?;
//! server.run()?;
//! # Ok::<(), json_rpc::Error>(())
//! ```
//!
//! # Struct Parameters
//!
//! Methods can use struct parameters for more complex APIs:
//!
//! ```no_run
//! use json_rpc::Server;
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
//! # Ok::<(), json_rpc::Error>(())
//! ```
//!
//! # Error Handling
//!
//! Methods return `Result<T, Error>`. Create JSON-RPC protocol errors with
//! specific codes:
//!
//! ```no_run
//! use json_rpc::{Server, Error};
//!
//! let mut server = Server::new();
//!
//! server.register("divide", |params: (i32, i32)| {
//!     if params.1 == 0 {
//!         return Err(Error::rpc(-32000, "Division by zero"));
//!     }
//!     Ok(params.0 / params.1)
//! })?;
//! # Ok::<(), json_rpc::Error>(())
//! ```
//!
//! # Graceful Shutdown
//!
//! Use a shutdown signal to stop the server cleanly:
//!
//! ```no_run
//! use json_rpc::{Server, ShutdownSignal};
//! use std::thread;
//! use std::time::Duration;
//!
//! let shutdown = ShutdownSignal::new();
//! let mut server = Server::new()
//!     .with_shutdown_signal(shutdown.clone());
//!
//! thread::spawn(move || {
//!     thread::sleep(Duration::from_secs(5));
//!     shutdown.signal();
//! });
//!
//! # Ok::<(), json_rpc::Error>(())
//! ```
//!
//! # Batch Requests
//!
//! The library automatically handles batch requests. Since the server uses NDJSON
//! (newline-delimited JSON), batch request arrays must be on a single line.
//! Send multiple requests in a single array:
//!
//! ```json
//! [{"jsonrpc":"2.0","method":"add","params":[1,2],"id":"1"},{"jsonrpc":"2.0","method":"add","params":[3,4],"id":"2"}]
//! ```
//!
//! The server processes each request concurrently and returns an array of
//! responses.
//!
//! # Thread Pool
//!
//! The server uses a fixed-size thread pool for concurrent request handling.
//! The default size equals the number of CPU cores. Configure it with
//! `.with_thread_pool_size()`. Each request processes in a worker thread,
//! and responses return to the main thread for transmission.
//!
//! # Transports
//!
//! The library separates protocol handling from transport. The default Stdio
//! transport reads newline-delimited JSON from stdin and writes responses to
//! stdout. The InMemory transport provides an in-memory channel for testing.
//! Implement custom transports by implementing the Transport trait.

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
