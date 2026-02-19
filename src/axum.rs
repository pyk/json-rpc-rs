//! Axum integration for JSON-RPC handlers.
//!
//! This module provides integration between the `JsonRpc` handler and the axum
//! web framework. It enables you to easily serve JSON-RPC requests over HTTP
//! using axum.
//!
//! # Example
//!
//! ```no_run
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
//! ```

use std::sync::Arc;

use axum::{
    extract::{Request, State},
    http::{StatusCode, header},
    response::IntoResponse,
};

use crate::JsonRpc;

/// Axum handler for processing JSON-RPC requests.
///
/// This handler extracts the request body, calls the JSON-RPC handler,
/// and returns the appropriate HTTP response.
///
/// # Example
///
/// ```no_run
/// use json_rpc::{JsonRpc, axum::handler};
/// use axum::Router;
/// use std::sync::Arc;
///
/// let json_rpc = JsonRpc::new().add("echo", echo);
/// let app = Router::new()
///     .route("/jsonrpc", handler)
///     .with_state(Arc::new(json_rpc));
/// ```
pub async fn handler(State(json_rpc): State<Arc<JsonRpc>>, request: Request) -> impl IntoResponse {
    // Extract request body with a size limit of 10MB
    let bytes = match axum::body::to_bytes(request.into_body(), 10 * 1024 * 1024).await {
        Ok(b) => b,
        Err(e) => {
            tracing::error!("Failed to read request body: {}", e);
            return error_response(
                StatusCode::BAD_REQUEST,
                r#"{"jsonrpc":"2.0","error":{"code":-32700,"message":"Parse error"},"id":null}"#,
            );
        }
    };

    // Convert bytes to string
    let json_str = match String::from_utf8(bytes.to_vec()) {
        Ok(s) => s,
        Err(_) => {
            tracing::error!("Invalid UTF-8 in request body");
            return error_response(
                StatusCode::BAD_REQUEST,
                r#"{"jsonrpc":"2.0","error":{"code":-32700,"message":"Parse error"},"id":null}"#,
            );
        }
    };

    tracing::debug!("Processing JSON-RPC request: {}", json_str);

    // Process the JSON-RPC message
    match json_rpc.call(&json_str).await {
        Some(response_json) => {
            tracing::debug!("Sending JSON-RPC response: {}", response_json);
            success_response(&response_json)
        }
        None => {
            // Notification - no response needed
            tracing::debug!("Notification processed - no response needed");
            StatusCode::OK.into_response()
        }
    }
}

/// Create a successful JSON-RPC response.
fn success_response(json: &str) -> axum::response::Response {
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/json")],
        json.to_string(),
    )
        .into_response()
}

/// Create an error JSON-RPC response.
fn error_response(status: StatusCode, json: &str) -> axum::response::Response {
    (
        status,
        [(header::CONTENT_TYPE, "application/json")],
        json.to_string(),
    )
        .into_response()
}
