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
}

impl Default for Methods {
    fn default() -> Self {
        Self::new()
    }
}
