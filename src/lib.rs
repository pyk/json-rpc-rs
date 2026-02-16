//! JSON-RPC 2.0 implementation for building protocol handlers.
//!
//! This module provides a generic, protocol-agnostic JSON-RPC 2.0 implementation
//! that can be used to build any JSON-RPC-based protocol. It handles the
//! wire format (transport), message parsing, and method routing.
//!
//! # Architecture
//!
//! The jsonrpc module is organized into four main components:
//!
//! - [`types`] - JSON-RPC 2.0 message types (Request, Response, Notification, Error)
//! - [`transports`] - I/O handling with multiple transport implementations (Stdio, InMemory)
//! - [`handler`] - Message handling and the main I/O loop
//! - [`router`] - Protocol-agnostic method routing
//!
//! # Quick Start
//!
//! To implement a protocol using this module:
//!
//! ## 1. Define Your Protocol Methods
//!
//! Create an enum representing all methods in your protocol:
//!
//! ```no_run
//! use acp::jsonrpc::RequestId;
//!
//! enum MyProtocolMethod {
//!     Initialize(RequestId),
//!     DoSomething(RequestId),
//!     Unknown(RequestId, String),
//! }
//! ```
//!
//! ## 2. Implement the Router Trait
//!
//! Implement the [`Router`] trait to map JSON-RPC method names to your protocol methods:
//!
//! ```no_run
//! use acp::jsonrpc::{Router, Request, RequestId};
//! use acp::jsonrpc::types::Response;
//! use acp::Error;
//!
//! enum MyProtocolMethod {
//!     Initialize(RequestId),
//!     DoSomething(RequestId),
//!     Unknown(RequestId, String),
//! }
//!
//! struct MyRouter;
//!
//! impl Router for MyRouter {
//!     type Method = MyProtocolMethod;
//!
//!     fn route(&self, request: Request) -> Self::Method {
//!         match request.method.as_str() {
//!             "initialize" => MyProtocolMethod::Initialize(request.id),
//!             "doSomething" => MyProtocolMethod::DoSomething(request.id),
//!             _ => MyProtocolMethod::Unknown(request.id, request.method),
//!         }
//!     }
//!
//!     fn handle<F>(&self, method: Self::Method, handler: F) -> Result<Option<serde_json::Value>, Error>
//!     where
//!         F: FnOnce() -> Result<serde_json::Value, Error>,
//!     {
//!         match method {
//!             MyProtocolMethod::Initialize(id) => {
//!                 // Your business logic here
//!                 let result = handler()?;
//!                 Ok(Some(result))
//!             }
//!             MyProtocolMethod::DoSomething(id) => {
//!                 // Your business logic here
//!                 let result = handler()?;
//!                 Ok(Some(result))
//!             }
//!             MyProtocolMethod::Unknown(_, _) => {
//!                 Err(Error::protocol("Unknown method"))
//!             }
//!         }
//!     }
//!
//!     fn unknown_method_response(&self, id: RequestId, method: &str) -> Response {
//!         Response::error(
//!             id,
//!             acp::jsonrpc::types::Error::method_not_found(
//!                 format!("Unknown method: {}", method)
//!             ),
//!         )
//!     }
//! }
//! ```
//!
//! ## 3. Create and Run the Handler
//!
//! ```no_run
//! use acp::jsonrpc::{Handler, Router};
//!
//! // Assuming MyRouter is defined as in step 2 above
//! # struct MyRouter;
//! # impl Router for MyRouter { type Method = (); fn route(&self, _: acp::jsonrpc::Request) -> Self::Method { () } fn handle<F>(&self, _: Self::Method, _: F) -> Result<Option<serde_json::Value>, acp::Error> where F: FnOnce() -> Result<serde_json::Value>, acp::Error> { Ok(None) } fn unknown_method_response(&self, _: acp::jsonrpc::RequestId, _: &str) -> acp::jsonrpc::Response { todo!() } }
//!
//! let router = MyRouter;
//! let mut handler = Handler::new(router);
//! handler.run()?;  // Blocks and processes messages
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # Layer Responsibilities
//!
//! | Component | Responsibility |
//! |-----------|----------------|
//! | `types` | JSON-RPC message structures and serialization |
//! | `transport` | Reading/writing bytes to the wire |
//! | `handler` | Main I/O loop, message dispatch |
//! | `router` | Maps method names to protocol-specific handlers |
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
//! # Example: ACP Protocol
//!
//! The Agent Client Protocol (ACP) uses this module by:
//!
//! 1. Defining ACP methods in an enum (`Initialize`, `SessionNew`, etc.)
//! 2. Implementing `Router` to map `"initialize"` -> `ACPMethod::Initialize`
//! 3. Using `Handler` to run the main I/O loop
//!
//! See [`crate::agent`] for the ACP implementation.

pub use handler::Handler;
pub use router::{ErrorExt as _, JsonRpcErrorExt as _, Router};
pub use transports::{InMemory, Stdio, Transport};
pub use types::{Error, Message, Notification, Request, RequestId, Response};

pub mod handler;
pub mod router;
pub mod transports;
pub mod types;
