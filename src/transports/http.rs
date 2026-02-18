//! HTTP-based transport for JSON-RPC 2.0.
//!
//! This module implements HTTP-based transport for JSON-RPC 2.0 communication
//! using axum for the web server. It supports the standard JSON-RPC POST pattern
//! where requests are sent via HTTP POST and responses are returned in the HTTP response.

use axum::{
    Router,
    extract::{Request as AxumRequest, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::post,
};
use std::sync::Arc;

use crate::error::Error;
use crate::methods::Methods;
use crate::transports::Transport;

/// Default port for the HTTP server.
const DEFAULT_PORT: u16 = 3000;

/// Default path for JSON-RPC endpoints.
const DEFAULT_PATH: &str = "/jsonrpc";

/// Shared state for the HTTP server.
#[derive(Clone)]
struct HttpState {
    /// The method registry for processing JSON-RPC requests.
    methods: Arc<Methods>,
}

/// HTTP-based transport for JSON-RPC messages.
///
/// This transport uses axum to handle HTTP POST requests with JSON-RPC messages.
/// Each HTTP POST request is treated as a JSON-RPC request, and the response
/// is returned in the HTTP response body.
///
/// # Architecture
///
/// When a request arrives via HTTP POST:
/// 1. The handler reads the request body as JSON
/// 2. The JSON is processed through `methods.process_message()`
/// 3. If a response is generated, it's returned with Content-Type: application/json
/// 4. If no response is needed (notification), an empty 200 OK is returned
///
/// This design is much simpler than channel-based approaches because HTTP is
/// inherently request/response - we don't need to manage pending responses
/// or correlate requests with responses.
///
/// # Example
///
/// ```no_run
/// use json_rpc::{Http, Methods};
/// use serde_json::Value;
///
/// async fn echo(params: Value) -> Result<Value, json_rpc::Error> {
///     Ok(params)
/// }
///
/// let methods = Methods::new().add("echo", echo);
/// let transport = Http::new();
/// json_rpc::serve(transport, methods).await.unwrap();
/// ```
pub struct Http {
    /// The address to bind the HTTP server to.
    address: std::net::SocketAddr,
}

impl Http {
    /// Create a new HTTP transport with default settings.
    ///
    /// The server will bind to `127.0.0.1:3000` and accept POST requests at `/jsonrpc`.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use json_rpc::Http;
    ///
    /// let transport = Http::new();
    /// ```
    pub fn new() -> Self {
        Self::with_address((std::net::Ipv4Addr::LOCALHOST, DEFAULT_PORT))
    }

    /// Create a new HTTP transport with the specified bind address.
    ///
    /// The server will accept POST requests at `/jsonrpc` on the specified address.
    ///
    /// # Arguments
    ///
    /// * `addr` - The address to bind the HTTP server to
    ///
    /// # Example
    ///
    /// ```no_run
    /// use json_rpc::Http;
    ///
    /// let transport = Http::with_address(([127, 0, 0, 1], 8080));
    /// ```
    pub fn with_address(addr: impl std::net::ToSocketAddrs) -> Self {
        // Resolve the address to a SocketAddr
        let mut addrs_iter = addr.to_socket_addrs().unwrap();
        let address = addrs_iter.next().expect("No address found");

        Self { address }
    }
}

impl Default for Http {
    fn default() -> Self {
        Self::new()
    }
}

impl Transport for Http {
    /// Serve the JSON-RPC server using HTTP transport.
    ///
    /// This method starts an axum HTTP server that accepts POST requests
    /// at `/jsonrpc`. Each request is processed as a JSON-RPC message and
    /// the response is returned in the HTTP response.
    ///
    /// The server runs until it is shut down (e.g., by Ctrl+C).
    ///
    /// # Arguments
    ///
    /// * `methods` - The method registry containing all registered JSON-RPC methods
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` when the server shuts down gracefully, or an error if
    /// the server fails to start.
    async fn serve(self, methods: Methods) -> Result<(), Error> {
        // Create shared state with the methods registry
        let state = HttpState {
            methods: Arc::new(methods),
        };

        // Build the axum router
        let app = Router::new()
            .route(DEFAULT_PATH, post(handle_jsonrpc))
            .with_state(state);

        // Start the HTTP server
        let listener = tokio::net::TcpListener::bind(self.address)
            .await
            .map_err(|e| {
                Error::TransportError(std::io::Error::new(
                    std::io::ErrorKind::AddrInUse,
                    format!("Failed to bind to port {}: {}", DEFAULT_PORT, e),
                ))
            })?;

        let local_addr = listener
            .local_addr()
            .map_err(|e| Error::TransportError(e))?;

        eprintln!("HTTP transport listening on http://{}", local_addr);
        eprintln!("JSON-RPC endpoint: http://{}{}", local_addr, DEFAULT_PATH);

        // Run the server
        axum::serve(listener, app).await.map_err(|e| {
            Error::TransportError(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("HTTP server error: {}", e),
            ))
        })?;

        Ok(())
    }
}

/// Handle HTTP POST requests for JSON-RPC messages.
///
/// This Axum handler extracts the JSON from the request body, processes it
/// through the method registry, and returns the JSON-RPC response.
async fn handle_jsonrpc(State(state): State<HttpState>, request: AxumRequest) -> Response {
    // Read the request body
    let bytes = match axum::body::to_bytes(request.into_body(), 10 * 1024 * 1024).await {
        Ok(b) => b,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                format!("Failed to read body: {}", e),
            )
                .into_response();
        }
    };

    let json_str = match String::from_utf8(bytes.to_vec()) {
        Ok(s) => s,
        Err(_) => {
            return (StatusCode::BAD_REQUEST, "Invalid UTF-8 in request body").into_response();
        }
    };

    // Process the JSON-RPC message through the method registry
    let response_json = state.methods.process_message(&json_str).await;

    match response_json {
        Some(json) => (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "application/json")],
            json,
        )
            .into_response(),
        None => {
            // This was a notification (no response expected)
            StatusCode::OK.into_response()
        }
    }
}
