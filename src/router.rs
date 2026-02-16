//! JSON-RPC method router.
//!
//! This module provides a generic `Router` trait for implementing protocol-agnostic
//! JSON-RPC method routing.
//!

use crate::error::Error;
use crate::types::{Request, RequestId, Response};

/// Router trait for handling method routing.
///
/// Implement this trait to define how JSON-RPC method names are mapped
/// to your protocol-specific methods. The router is protocol-agnostic -
/// you decide what methods your protocol supports and how to handle them.
pub trait Router {
    /// The method type for your protocol.
    type Method;

    /// Route a JSON-RPC request to a protocol method.
    ///
    /// This is called for each incoming request to determine which
    /// protocol method should handle it.
    fn route(&self, request: Request) -> Self::Method;

    /// Handle a routed method.
    ///
    /// The `handler` closure contains the actual business logic for this method.
    /// The router should match on the method and call the handler, returning
    /// the result or an error.
    fn handle<F>(
        &self,
        method: Self::Method,
        handler: F,
    ) -> Result<Option<serde_json::Value>, Error>
    where
        F: FnOnce() -> Result<serde_json::Value, Error>;

    /// Create an error response for an unknown method.
    fn unknown_method_response(&self, id: RequestId, method: &str) -> Response;
}

/// Helper extensions for Error.
pub trait JsonRpcErrorExt {
    fn method_not_found(message: impl Into<String>) -> Self;
    fn invalid_params(message: impl Into<String>) -> Self;
    fn internal_error(message: impl Into<String>) -> Self;
    fn into_response(self, id: RequestId) -> Response;
}

impl JsonRpcErrorExt for crate::types::Error {
    fn method_not_found(message: impl Into<String>) -> Self {
        Self::new(-32601, message, None)
    }

    fn invalid_params(message: impl Into<String>) -> Self {
        Self::new(-32602, message, None)
    }

    fn internal_error(message: impl Into<String>) -> Self {
        Self::new(-32603, message, None)
    }

    fn into_response(self, id: RequestId) -> Response {
        Response::error(id, self)
    }
}

/// Helper extensions for Error.
pub trait ErrorExt {
    fn protocol(message: impl Into<String>) -> Self;
}

impl ErrorExt for Error {
    fn protocol(message: impl Into<String>) -> Self {
        Error::ProtocolError(message.into())
    }
}
