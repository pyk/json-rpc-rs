//! JSON-RPC handler for message processing.
//!
//! This module provides the `JsonRpc` handler for registering methods and
//! processing JSON-RPC messages. Call `JsonRpc::call()` with a JSON string to
//! process a request and get a response string.

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use serde::Serialize;

use crate::error::Error;
use crate::types::{Message, RequestId, Response};

/// Type alias for async handler functions.
type BoxedHandler = Box<
    dyn Fn(
            serde_json::Value,
        ) -> Pin<Box<dyn Future<Output = Result<serde_json::Value, Error>> + Send>>
        + Send
        + Sync,
>;

/// JSON-RPC handler for message processing.
///
/// `JsonRpc` registers method handlers and processes JSON-RPC messages via the
/// `call()` method. Use the builder pattern to add methods with automatic
/// parameter deserialization.
///
/// # Example
///
/// ```no_run
/// use json_rpc::JsonRpc;
///
/// async fn echo(params: serde_json::Value) -> Result<serde_json::Value, json_rpc::Error> {
///     Ok(params)
/// }
///
/// let json_rpc = JsonRpc::new()
///     .add("echo", echo);
///
/// # tokio::runtime::Runtime::new().unwrap().block_on(async {
/// let response = json_rpc.call(r#"{"jsonrpc":"2.0","method":"echo","params":"hello","id":1}"#).await;
/// # });
/// ```
pub struct JsonRpc {
    handlers: HashMap<String, BoxedHandler>,
}

impl JsonRpc {
    /// Create a new empty JSON-RPC handler.
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
        }
    }

    /// Register a JSON-RPC method handler.
    ///
    /// The handler must be an async function that takes deserialized parameters
    /// and returns a `Result` with either the return value or an `Error`.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use json_rpc::JsonRpc;
    /// use serde_json::Value;
    ///
    /// async fn add(params: (i32, i32)) -> Result<i32, json_rpc::Error> {
    ///     Ok(params.0 + params.1)
    /// }
    ///
    /// let json_rpc = JsonRpc::new()
    ///     .add("add", add);
    /// ```
    pub fn add<F, P, R, Fut>(mut self, method: &str, handler: F) -> Self
    where
        F: Fn(P) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<R, Error>> + Send + Sync + 'static,
        P: serde::de::DeserializeOwned + Send + Sync + 'static,
        R: Serialize + Send + Sync + 'static,
    {
        let handler = Arc::new(handler);
        let boxed: BoxedHandler = Box::new(move |params: serde_json::Value| {
            let handler = Arc::clone(&handler);
            Box::pin(async move {
                let parsed: P = serde_json::from_value(params)?;
                let result = handler(parsed).await?;
                Ok(serde_json::to_value(result)?)
            })
        });

        self.handlers.insert(method.to_string(), boxed);
        self
    }

    /// Get the handler for a method name, if it exists.
    pub(crate) fn get_handler(&self, method: &str) -> Option<&BoxedHandler> {
        self.handlers.get(method)
    }

    /// Process a JSON-RPC message and return the response JSON string (if any).
    ///
    /// This method processes a JSON-RPC message string and returns the response.
    /// It handles:
    ///
    /// - JSON parsing and validation
    /// - Message type detection (request, notification, batch, response)
    /// - Method routing and execution
    /// - Error handling and response generation
    ///
    /// Returns `None` for notifications (which don't require a response).
    pub async fn call(&self, json_str: &str) -> Option<String> {
        let value: serde_json::Value = match serde_json::from_str(json_str) {
            Ok(v) => v,
            Err(_) => {
                let error = crate::types::Error::parse_error("Parse error");
                let response = Response::error(RequestId::Null, error);
                match serde_json::to_string(&response) {
                    Ok(s) => return Some(s),
                    Err(e) => {
                        tracing::error!("Failed to serialize parse error response: {}", e);
                        return None;
                    }
                }
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
            Err(Error::InvalidRequest(_)) => {
                let error = crate::types::Error::invalid_request("Invalid Request");
                let id_to_use = request_id.unwrap_or(RequestId::Null);
                let response = Response::error(id_to_use, error);
                match serde_json::to_string(&response) {
                    Ok(s) => return Some(s),
                    Err(e) => {
                        tracing::error!("Failed to serialize invalid request response: {}", e);
                        return None;
                    }
                }
            }
            Err(_) => {
                let error = crate::types::Error::internal_error("Internal error");
                let response = Response::error(request_id.unwrap_or(RequestId::Null), error);
                match serde_json::to_string(&response) {
                    Ok(s) => return Some(s),
                    Err(e) => {
                        tracing::error!("Failed to serialize internal error response: {}", e);
                        return None;
                    }
                }
            }
        };

        match message {
            Message::Request(request) => {
                let method_name = &request.method;
                let params = request.params.unwrap_or(serde_json::Value::Null);
                let request_id = request.id.clone();
                let response = if let Some(handler) = self.get_handler(method_name) {
                    let result = handler(params).await;
                    match result {
                        Ok(result_value) => Response::success(request_id, result_value),
                        Err(e) => {
                            let error = match e {
                                crate::error::Error::RpcError { code, message } => {
                                    crate::types::Error::new(code, message, None)
                                }
                                _ => crate::types::Error::new(-32603, e.to_string(), None),
                            };
                            Response::error(request_id, error)
                        }
                    }
                } else {
                    let error = crate::types::Error::method_not_found(format!(
                        "Unknown method: {}",
                        method_name
                    ));
                    Response::error(request_id, error)
                };
                match serde_json::to_string(&response) {
                    Ok(s) => Some(s),
                    Err(e) => {
                        tracing::error!("Failed to serialize response: {}", e);
                        None
                    }
                }
            }
            Message::Notification(notification) => {
                if let Some(handler) = self.get_handler(&notification.method) {
                    let params = notification.params.unwrap_or(serde_json::Value::Null);
                    let _ = handler(params).await;
                }
                None
            }
            Message::Batch(messages) => {
                let mut responses = Vec::new();

                for message in messages {
                    match message {
                        Message::Request(request) => {
                            let method_name = &request.method;
                            let params = request.params.unwrap_or(serde_json::Value::Null);
                            let id = request.id;
                            let response = if let Some(handler) = self.get_handler(method_name) {
                                let result = handler(params).await;
                                match result {
                                    Ok(result_value) => Response::success(id, result_value),
                                    Err(e) => {
                                        let error = match e {
                                            crate::error::Error::RpcError { code, message } => {
                                                crate::types::Error::new(code, message, None)
                                            }
                                            _ => crate::types::Error::new(
                                                -32603,
                                                e.to_string(),
                                                None,
                                            ),
                                        };
                                        Response::error(id, error)
                                    }
                                }
                            } else {
                                let error = crate::types::Error::method_not_found(format!(
                                    "Unknown method: {}",
                                    method_name
                                ));
                                Response::error(id, error)
                            };
                            responses.push(response);
                        }
                        Message::Notification(notification) => {
                            if let Some(handler) = self.get_handler(&notification.method) {
                                let params = notification.params.unwrap_or(serde_json::Value::Null);
                                let _ = handler(params).await;
                            }
                        }
                        Message::Response(response) => {
                            responses.push(response);
                        }
                        Message::Batch(_) => {
                            let error_response = Response::error(
                                crate::types::RequestId::Null,
                                crate::types::Error::invalid_request("Invalid Request"),
                            );
                            responses.push(error_response);
                        }
                    }
                }

                match serde_json::to_string(&responses) {
                    Ok(s) => Some(s),
                    Err(e) => {
                        tracing::error!("Failed to serialize batch responses: {}", e);
                        None
                    }
                }
            }
            Message::Response(_response) => None,
        }
    }
}

impl Default for JsonRpc {
    fn default() -> Self {
        Self::new()
    }
}
