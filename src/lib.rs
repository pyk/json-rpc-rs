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
//! Create a method registry and serve:
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
//! let methods = Methods::new()
//!     .add("echo", echo);
//!
//! let transport = Stdio::new();
//! json_rpc::serve(transport, methods).await.unwrap();
//! # Ok::<(), json_rpc::Error>(())
//! ```
//!
//! # Struct Parameters
//!
//! Methods can use struct parameters for more complex APIs:
//!
//! ```no_run
//! use json_rpc::Methods;
//! use serde::Deserialize;
//!
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
//! let methods = Methods::new()
//!     .add("initialize", initialize);
//! # json_rpc::serve(Stdio::new(), methods).await.unwrap();
//! ```
//!
//! # Error Handling
//!
//! Methods return `Result<T, Error>`. Create JSON-RPC protocol errors with
//! specific codes:
//!
//! ```no_run
//! use json_rpc::{Methods, Error};
//! use serde_json::Value;
//!
//! async fn divide(params: (i32, i32)) -> Result<i32, Error> {
//!     if params.1 == 0 {
//!         return Err(Error::rpc(-32000, "Division by zero"));
//!     }
//!     Ok(params.0 / params.1)
//! }
//!
//! let methods = Methods::new()
//!     .add("divide", divide);
//! # json_rpc::serve(Stdio::new(), methods).await.unwrap();
//! ```
//!
//! # Transports
//!
//! The library separates protocol handling from transport. The Stdio transport
//! reads newline-delimited JSON from stdin and writes responses to stdout.
//! The InMemory transport provides an in-memory channel for testing.
//! Implement custom transports by implementing the Transport trait.
//!
//! # Limitations
//!
//! Batch requests are not yet supported. Sending a batch request will return
//! an internal error (-32603) with the message "Batch requests not yet supported".
//! Batch support will be added in a future version.

pub use error::Error;
pub use methods::Methods;
pub use transports::{InMemory, Stdio, Transport};
pub use types::{Message, Notification, Request, RequestId, Response};

/// Serve a JSON-RPC server with the given transport and methods.
///
/// This function creates a server with the provided methods and runs it
/// using the specified transport. The transport determines how JSON-RPC
/// messages are sent and received (e.g., stdio, TCP, in-memory).
///
/// JSON-RPC is transport-agnostic - the protocol works with any transport
/// that can send and receive raw JSON strings.
///
/// # Limitations
///
/// Batch requests are not yet supported. Sending a batch request will return
/// an internal error (-32603) with the message "Batch requests not yet supported".
/// Batch support will be added in a future version.
///
/// # Arguments
///
/// * `transport` - The transport to use for communication (stdio, TCP, etc.)
/// * `methods` - The method registry containing all registered JSON-RPC methods
///
/// # Returns
///
/// Returns `Ok(())` when the server shuts down gracefully, or an error if
/// the server encounters a fatal error.
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
/// let methods = Methods::new()
///     .add("echo", echo);
///
/// let transport = Stdio::new();
/// json_rpc::serve(transport, methods).await.unwrap();
/// ```
pub async fn serve<T>(transport: T, methods: Methods) -> Result<(), Error>
where
    T: Transport + 'static,
{
    let mut transport = transport;
    let methods = std::sync::Arc::new(methods);

    loop {
        let json_str = match transport.receive_message().await {
            Ok(msg) => msg,
            Err(Error::TransportError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                break;
            }
            Err(_e) => {
                let error = crate::types::Error::internal_error("Internal error");
                let response = Response::error(RequestId::Null, error);
                let json = serde_json::to_string(&response).map_err(Error::from)?;
                let _ = transport.send_message(&json).await;
                continue;
            }
        };

        let value: serde_json::Value = match serde_json::from_str(&json_str) {
            Ok(v) => v,
            Err(_e) => {
                let error = crate::types::Error::parse_error("Parse error");
                let response = Response::error(RequestId::Null, error);
                let json = serde_json::to_string(&response).map_err(Error::from)?;
                let _ = transport.send_message(&json).await;
                continue;
            }
        };

        let request_id = value.get("id").and_then(|id_value| match id_value {
            serde_json::Value::Null => Some(RequestId::Null),
            serde_json::Value::Number(n) => n.as_u64().map(RequestId::Number),
            serde_json::Value::String(s) => Some(RequestId::String(s.clone())),
            _ => None,
        });

        let message = match Message::from_json(value) {
            Ok(msg) => msg,
            Err(Error::InvalidRequest(_e)) => {
                let error = crate::types::Error::invalid_request("Invalid Request");
                let id_to_use = request_id.unwrap_or(RequestId::Null);
                let response = Response::error(id_to_use, error);
                let json = serde_json::to_string(&response).map_err(Error::from)?;
                let _ = transport.send_message(&json).await;
                continue;
            }
            Err(_e) => {
                let error = crate::types::Error::internal_error("Internal error");
                let response = Response::error(request_id.unwrap_or(RequestId::Null), error);
                let json = serde_json::to_string(&response).map_err(Error::from)?;
                let _ = transport.send_message(&json).await;
                continue;
            }
        };

        match message {
            Message::Request(request) => {
                let method_name = &request.method;
                let params = request.params.unwrap_or(serde_json::Value::Null);
                let response = if let Some(handler) = methods.get_handler(method_name) {
                    let result = handler(params).await;
                    match result {
                        Ok(result_value) => Response::success(request.id.clone(), result_value),
                        Err(e) => {
                            let error = match e {
                                crate::error::Error::RpcError { code, message } => {
                                    crate::types::Error::new(code, message, None)
                                }
                                _ => crate::types::Error::new(-32603, e.to_string(), None),
                            };
                            Response::error(request.id.clone(), error)
                        }
                    }
                } else {
                    let error = crate::types::Error::method_not_found(format!(
                        "Unknown method: {}",
                        method_name
                    ));
                    Response::error(request.id.clone(), error)
                };
                let json = serde_json::to_string(&response).map_err(Error::from)?;
                let _ = transport.send_message(&json).await;
            }
            Message::Notification(notification) => {
                if let Some(handler) = methods.get_handler(&notification.method) {
                    let params = notification.params.unwrap_or(serde_json::Value::Null);
                    let _ = handler(params).await;
                }
            }
            Message::Batch(_messages) => {
                let error = crate::types::Error::internal_error("Batch requests not yet supported");
                let response = Response::error(request_id.unwrap_or(RequestId::Null), error);
                let json = serde_json::to_string(&response).map_err(Error::from)?;
                let _ = transport.send_message(&json).await;
            }
            Message::Response(_response) => {}
        }
    }

    Ok(())
}

pub mod error;
pub mod methods;
pub mod transports;
pub mod types;
