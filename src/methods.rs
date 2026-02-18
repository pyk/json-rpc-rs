//! JSON-RPC method registry with builder pattern.
//!
//! This module provides a `Methods` type for registering JSON-RPC methods
//! using a builder pattern. The registry can be passed to the `serve` function
//! to start a JSON-RPC server.

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

use serde::Serialize;

use crate::error::Error;
use std::sync::Arc;

/// Type alias for async handler functions.
type BoxedHandler = Box<
    dyn Fn(
            serde_json::Value,
        ) -> Pin<Box<dyn Future<Output = Result<serde_json::Value, Error>> + Send>>
        + Send
        + Sync,
>;

/// Registry of JSON-RPC methods with a builder pattern.
///
/// `Methods` allows you to register JSON-RPC method handlers using a fluent
/// builder API. The registered methods can then be passed to the `serve` function
/// to start a JSON-RPC server.
///
/// # Example
///
/// ```no_run
/// use json_rpc::Methods;
///
/// async fn echo(params: serde_json::Value) -> Result<serde_json::Value, json_rpc::Error> {
///     Ok(params)
/// }
///
/// let methods = Methods::new()
///     .add("echo", echo);
/// # json_rpc::serve(methods).await.unwrap();
/// ```
pub struct Methods {
    handlers: HashMap<String, BoxedHandler>,
}

impl Methods {
    /// Create a new empty method registry.
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
    /// # Type Parameters
    ///
    /// - `F`: The handler function type
    /// - `P`: The parameter type (must implement `DeserializeOwned`)
    /// - `R`: The return type (must implement `Serialize`)
    /// - `Fut`: The future type returned by the handler
    ///
    /// # Example
    ///
    /// ```no_run
    /// use json_rpc::Methods;
    /// use serde_json::Value;
    ///
    /// async fn add(params: (i32, i32)) -> Result<i32, json_rpc::Error> {
    ///     Ok(params.0 + params.1)
    /// }
    ///
    /// let methods = Methods::new()
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
    /// This helper method is used by transport implementations to process
    /// incoming JSON-RPC messages. It handles:
    ///
    /// - JSON parsing and validation
    /// - Message type detection (request, notification, batch, response)
    /// - Method routing and execution
    /// - Error handling and response generation
    ///
    /// # Arguments
    ///
    /// * `json_str` - The raw JSON string from the transport
    ///
    /// # Returns
    ///
    /// Returns `Some(response_json)` if a response should be sent (for requests),
    /// or `None` if no response is needed (for notifications).
    pub async fn process_message(&self, json_str: &str) -> Option<String> {
        use crate::types::{Message, RequestId, Response};

        let value: serde_json::Value = match serde_json::from_str(json_str) {
            Ok(v) => v,
            Err(_) => {
                let error = crate::types::Error::parse_error("Parse error");
                let response = Response::error(RequestId::Null, error);
                return serde_json::to_string(&response).ok();
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
                return serde_json::to_string(&response).ok();
            }
            Err(_) => {
                let error = crate::types::Error::internal_error("Internal error");
                let response = Response::error(request_id.unwrap_or(RequestId::Null), error);
                return serde_json::to_string(&response).ok();
            }
        };

        match message {
            Message::Request(request) => {
                let method_name = &request.method;
                let params = request.params.unwrap_or(serde_json::Value::Null);
                let response = if let Some(handler) = self.get_handler(method_name) {
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
                serde_json::to_string(&response).ok()
            }
            Message::Notification(notification) => {
                if let Some(handler) = self.get_handler(&notification.method) {
                    let params = notification.params.unwrap_or(serde_json::Value::Null);
                    let _ = handler(params).await;
                }
                None
            }
            Message::Batch(_messages) => {
                let error = crate::types::Error::internal_error("Batch requests not yet supported");
                let response = Response::error(request_id.unwrap_or(RequestId::Null), error);
                serde_json::to_string(&response).ok()
            }
            Message::Response(_response) => None,
        }
    }
}

impl Default for Methods {
    fn default() -> Self {
        Self::new()
    }
}
