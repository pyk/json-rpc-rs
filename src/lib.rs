//! A JSON-RPC 2.0 implementation with a simple builder pattern.
//!
//! This library provides a simple, user-friendly API for creating JSON-RPC
//! handlers. It handles the JSON-RPC protocol layer including message parsing,
//! method routing, and response generation. Methods are registered using a
//! builder pattern with automatic parameter deserialization, making it easy to
//! define handlers that accept typed parameters.
//!
//! # Design Goals
//!
//! The library prioritizes simplicity and usability for JSON-RPC handlers.
//! It uses a builder pattern (`JsonRpc::new().add()`) for method registration
//! and supports multiple integration options (stdio, HTTP via axum, custom).
//!
//! # Architecture
//!
//! The library is organized into several modules:
//!
//! [`types`] contains JSON-RPC 2.0 message types including Request, Response,
//! Notification, and Error. These structures handle serialization and
//! deserialization of JSON-RPC messages.
//!
//! [`jsonrpc`] contains the `JsonRpc` type with a builder pattern API for
//! registering JSON-RPC method handlers and processing messages.
//!
//! [`error`] defines internal error types for implementation-level errors,
//! separate from JSON-RPC protocol errors sent over the wire.
//!
//! [`axum`] (feature-gated) provides integration with the axum web framework,
//! allowing you to serve JSON-RPC over HTTP with minimal boilerplate.
//!
//! # Quick Start
//!
//! Create a handler and process messages:
//!
//! ```no_run
//! use json_rpc::JsonRpc;
//! use serde_json::Value;
//!
//! async fn echo(params: Value) -> Result<Value, json_rpc::Error> {
//!     Ok(params)
//! }
//!
//! # tokio::runtime::Runtime::new().unwrap().block_on(async {
//! let json_rpc = JsonRpc::new()
//!     .add("echo", echo);
//!
//! let response = json_rpc.call(r#"{"jsonrpc":"2.0","method":"echo","params":"hello","id":1}"#).await;
//! # });
//! ```
//!
//! # Stdio Integration
//!
//! Use stdio for command-line tools and LSP implementations:
//!
//! ```no_run
//! use json_rpc::JsonRpc;
//! use serde_json::Value;
//! use tokio::io::AsyncBufReadExt;
//!
//! async fn echo(params: Value) -> Result<Value, json_rpc::Error> {
//!     Ok(params)
//! }
//!
//! # tokio::runtime::Runtime::new().unwrap().block_on(async {
//! let json_rpc = JsonRpc::new().add("echo", echo);
//!
//! let stdin = tokio::io::stdin();
//! let mut reader = tokio::io::BufReader::new(stdin);
//! let mut line = String::new();
//!
//! while reader.read_line(&mut line).await.unwrap() > 0 {
//!     let trimmed = line.trim();
//!     if !trimmed.is_empty() {
//!         if let Some(response) = json_rpc.call(trimmed).await {
//!             println!("{}", response);
//!         }
//!     }
//!     line.clear();
//! }
//! # });
//! ```
//!
//! # Struct Parameters
//!
//! Handlers can use struct parameters for more complex APIs:
//!
//! ```no_run
//! use json_rpc::JsonRpc;
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
//! let json_rpc = JsonRpc::new()
//!     .add("initialize", initialize);
//! # });
//! ```
//!
//! # Error Handling
//!
//! Methods return `Result<T, Error>`. Create JSON-RPC protocol errors with
//! specific codes:
//!
//! ```no_run
//! use json_rpc::{JsonRpc, Error};
//!
//! async fn divide(params: (i32, i32)) -> Result<i32, Error> {
//!     if params.1 == 0 {
//!         return Err(Error::rpc(-32000, "Division by zero"));
//!     }
//!     Ok(params.0 / params.1)
//! }
//!
//! let json_rpc = JsonRpc::new().add("divide", divide);
//! ```
//!
//! # Axum Integration
//!
//! The axum integration (enabled with the `axum` feature) provides a simple
//! way to serve JSON-RPC over HTTP:
//!
//! ```no_run
//! # #[cfg(feature = "axum")]
//! # {
//! use json_rpc::{JsonRpc, axum::IntoAxumHandler};
//! use axum::Router;
//!
//! async fn echo(params: serde_json::Value) -> Result<serde_json::Value, json_rpc::Error> {
//!     Ok(params)
//! }
//!
//! let json_rpc = JsonRpc::new().add("echo", echo);
//! let app = Router::new().route("/jsonrpc", json_rpc.into_axum_handler());
//! # }
//! ```

pub use error::Error;
pub use jsonrpc::JsonRpc;
pub use types::{Message, Notification, Request, RequestId, Response};

pub mod error;
pub mod jsonrpc;
pub mod types;

#[cfg(feature = "axum")]
pub mod axum;
