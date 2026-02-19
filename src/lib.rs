//! A framework-agnostic JSON-RPC 2.0 implementation with Bring Your Own Transport.
//!
//! This library handles the JSON-RPC protocol layer including message parsing,
//! method routing, and response generation. You register methods with the
//! `JsonRpc` handler, then process JSON-RPC messages from any transport.
//!
//! # Bring Your Own Transport
//!
//! The library does not include transport implementations. You read JSON strings
//! from your transport (stdio, HTTP, WebSocket, TCP, etc.), call `JsonRpc::call()`,
//! and write the response back. This gives you full control over your transport
//! layer.
//!
//! # Quick Start
//!
//! Create a handler and process a message:
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
//! // Read from your transport
//! let message = r#"{"jsonrpc":"2.0","method":"echo","params":"hello","id":1}"#;
//!
//! // Process the message
//! if let Some(response) = json_rpc.call(message).await {
//!     // Write to your transport
//!     println!("{}", response);
//! }
//! # });
//! ```
//!
//! # Stdio Example
//!
//! Read newline-delimited JSON from stdin and write responses to stdout:
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
//! Handlers can use struct parameters for complex APIs:
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
//! The axum feature provides a handler for HTTP integration. Enable the feature
//! in Cargo.toml:
//!
//! ```toml
//! [dependencies]
//! json-rpc-rs = { version = "0.2", features = ["axum"] }
//! ```
//!
//! ```no_run
//! # #[cfg(feature = "axum")]
//! # {
//! use json_rpc::{JsonRpc, axum::handler};
//! use axum::Router;
//! use std::sync::Arc;
//!
//! async fn echo(params: serde_json::Value) -> Result<serde_json::Value, json_rpc::Error> {
//!     Ok(params)
//! }
//!
//! let json_rpc = JsonRpc::new().add("echo", echo);
//! let app = Router::new()
//!     .route("/jsonrpc", handler)
//!     .with_state(Arc::new(json_rpc));
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
