//! A JSON-RPC 2.0 implementation with a simple builder pattern.
//!
//! This library provides a simple, user-friendly API for creating JSON-RPC
//! servers. It handles the JSON-RPC protocol layer including message parsing,
//! method routing, and response generation. Methods are registered using a
//! builder pattern with automatic parameter deserialization, making it easy to
//! define handlers that accept typed parameters.
//!
//! # Design Goals
//!
//! The library prioritizes simplicity and usability for JSON-RPC servers.
//! It uses a builder pattern (`Methods::new().add()`) for method registration
//! and supports multiple transport implementations (stdio, in-memory, custom).
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
//! [`methods`] contains the Methods type with a builder pattern API for
//! registering JSON-RPC method handlers.
//!
//! [`error`] defines internal error types for implementation-level errors,
//! separate from JSON-RPC protocol errors sent over the wire.
//!
//! # Quick Start
//!
//! Define methods and serve:
//!
//! ```no_run
//! use json_rpc::Methods;
//! use serde_json::Value;
//!
//! async fn echo(params: Value) -> Result<Value, json_rpc::Error> {
//!     Ok(params)
//! }
//!
//! let methods = Methods::new()
//!     .add("echo", echo);
//!
//! # Ok::<(), json_rpc::Error>(())
//! ```
//!
//! Run the server with the default Stdio transport:
//!
//! ```no_run
//! use json_rpc::{Methods, Stdio};
//! use serde_json::Value;
//!
//! async fn echo(params: Value) -> Result<Value, json_rpc::Error> {
//!     Ok(params)
//! }
//!
//! # tokio::runtime::Runtime::new().unwrap().block_on(async {
//! let methods = Methods::new()
//!     .add("echo", echo);
//!
//! let transport = Stdio::new();
//! json_rpc::serve(transport, methods).await.unwrap();
//! # Ok::<(), json_rpc::Error>(())
//! # });
//! ```
//!
//! # Struct Parameters
//!
//! Methods can use struct parameters for more complex APIs:
//!
//! ```no_run
//! use json_rpc::{Methods, Stdio};
//! use serde::Deserialize;
//!
//! # tokio::runtime::Runtime::new().unwrap().block_on(async {
//! #[derive(Deserialize)]
//! struct InitializeParams {
//!     name: String,
//!     version: String,
//! }
//!
//! async fn initialize(params: InitializeParams) -> Result<String, json_rpc::Error> {
//!     Ok(format!("Server {} v{} initialized", params.name, params.version))
//! }
//!
//! # tokio::runtime::Runtime::new().unwrap().block_on(async {
//! let methods = Methods::new()
//!     .add("initialize", initialize);
//! # json_rpc::serve(Stdio::new(), methods).await.unwrap();
//! # });
//! # });
//! ```
//!
//! # Error Handling
//!
//! Methods return `Result<T, Error>`. Create JSON-RPC protocol errors with
//! specific codes:
//!
//! ```no_run
//! use json_rpc::{Methods, Error, Stdio};
//! use serde_json::Value;
//!
//! async fn divide(params: (i32, i32)) -> Result<i32, Error> {
//!     if params.1 == 0 {
//!         return Err(Error::rpc(-32000, "Division by zero"));
//!     }
//!     Ok(params.0 / params.1)
//! }
//!
//! # tokio::runtime::Runtime::new().unwrap().block_on(async {
//! let methods = Methods::new()
//!     .add("divide", divide);
//! # json_rpc::serve(Stdio::new(), methods).await.unwrap();
//! # });
//! ```
//!
//! # Transports
//!
//! The library separates protocol handling from transport. All transports implement the
//! [`Transport`] trait, making them interchangeable. Choose the appropriate transport
//! based on your use case.
//!
//! ## Stdio Transport
//!
//! The Stdio transport reads newline-delimited JSON from stdin and writes responses to stdout.
//! This is useful for LSP (Language Server Protocol) implementations and command-line tools.
//!
//! ```no_run
//! use json_rpc::{Methods, Stdio};
//! use serde_json::Value;
//!
//! async fn echo(params: Value) -> Result<Value, json_rpc::Error> {
//!     Ok(params)
//! }
//!
//! # tokio::runtime::Runtime::new().unwrap().block_on(async {
//! let methods = Methods::new()
//!     .add("echo", echo);
//!
//! let transport = Stdio::new();
//! json_rpc::serve(transport, methods).await.unwrap();
//! # });
//! ```
//!
//! ## HTTP Transport
//!
//! The HTTP transport serves JSON-RPC over HTTP POST using axum. It accepts requests at
//! `/jsonrpc` and returns responses as JSON in the HTTP response body. This is ideal for
//! web services and APIs.
//!
//! ```no_run
//! use json_rpc::{Http, Methods};
//! use serde_json::Value;
//! use std::net::SocketAddr;
//!
//! async fn echo(params: Value) -> Result<Value, json_rpc::Error> {
//!     Ok(params)
//! }
//!
//! # tokio::runtime::Runtime::new().unwrap().block_on(async {
//! let methods = Methods::new().add("echo", echo);
//! let addr: SocketAddr = "127.0.0.1:3000".parse().unwrap();
//! let transport = Http::new(addr);
//! json_rpc::serve(transport, methods).await.unwrap();
//! # });
//! ```
//!
//! ## InMemory Transport
//!
//! The InMemory transport provides an in-memory channel for testing and development.
//! It's useful for unit tests where you don't need actual I/O.
//!
//! ## Custom Transports
//!
//! Implement custom transports by implementing the [`Transport`] trait to support
//! additional communication channels such as WebSockets, TCP sockets, or other protocols.

pub use error::Error;
pub use methods::Methods;
pub use transports::{Http, InMemory, Stdio, Transport};
pub use types::{Message, Notification, Request, RequestId, Response};

/// Serve a JSON-RPC server with the given transport and methods.
///
/// This function creates a server with the provided methods and runs it
/// using the specified transport. The transport determines how JSON-RPC
/// messages are sent and received (e.g., stdio, TCP, in-memory).
///
/// Each transport implementation handles its own serving logic, allowing
/// for different communication patterns (continuous stream, request/response, etc.).
///
/// # Example
///
/// ```no_run
/// use json_rpc::{Methods, Stdio};
/// use serde_json::Value;
///
/// async fn echo(params: Value) -> Result<Value, json_rpc::Error> {
///     Ok(params)
/// }
///
/// # tokio::runtime::Runtime::new().unwrap().block_on(async {
/// let methods = Methods::new()
///     .add("echo", echo);
///
/// let transport = Stdio::new();
/// json_rpc::serve(transport, methods).await.unwrap();
/// # });
/// ```
pub async fn serve<T>(transport: T, methods: Methods) -> Result<(), Error>
where
    T: Transport,
{
    transport.serve(methods).await
}

pub mod error;
pub mod methods;
pub mod transports;
pub mod types;
