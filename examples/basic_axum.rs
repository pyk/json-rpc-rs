//! A basic JSON-RPC 2.0 server using axum with multiple methods.
//!
//! This example demonstrates how to create a JSON-RPC server with multiple
//! methods using the json-rpc-rs library and axum. It includes methods that
//! demonstrate various JSON-RPC features including success responses,
//! errors, and notifications.
//!
//! Usage:
//!
//! ```bash
//! cargo run --example basic_axum --features axum
//! ```
//!
//! Then send requests:
//!
//! ```bash
//! curl -X POST http://localhost:3000/jsonrpc \
//!   -H "Content-Type: application/json" \
//!   -d '{"jsonrpc":"2.0","method":"hello","params":"world","id":1}'
//! ```
//!
//! Expected response:
//!
//! ```json
//! {"jsonrpc":"2.0","result":"Hello, world!","id":1}
//! ```
//!
//! This example requires the "axum" feature to be enabled.

use anyhow::Result;
use axum::Router;
use axum::routing::post;
use json_rpc::axum::handler;
use json_rpc::{Error, JsonRpc};
use serde_json::Value;
use std::sync::Arc;
use tracing::info;

/// Hello method that returns a greeting.
async fn hello(params: String) -> Result<String, Error> {
    info!("Hello called with params: {}", params);

    if params == "world" {
        Ok(format!("Hello, {}!", params))
    } else {
        Err(Error::rpc(-32000, "text must be 'world'"))
    }
}

/// Internal error method that returns an internal server error.
async fn internal_error(_params: Value) -> Result<String, Error> {
    Err(Error::protocol("Internal error occurred"))
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .with_writer(std::io::stderr)
        .init();

    info!("Initializing basic JSON-RPC server");

    let json_rpc = JsonRpc::new()
        .add("hello", hello)
        .add("internal_error", internal_error);

    let app = Router::new()
        .route("/jsonrpc", post(handler))
        .with_state(Arc::new(json_rpc));

    let addr: std::net::SocketAddr = "127.0.0.1:3001".parse()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;
    let local_addr = listener.local_addr()?;

    info!("Server started on http://{}", local_addr);
    info!("JSON-RPC endpoint: http://{}/jsonrpc", local_addr);
    info!("Available methods:");
    info!("  - hello: Returns greeting (params: \"world\")");
    info!("  - internal_error: Returns internal error for testing");
    info!("");
    info!("Example request:");
    info!(
        "  curl -X POST http://{}/jsonrpc -H \"Content-Type: application/json\" -d '{{\"jsonrpc\":\"2.0\",\"method\":\"hello\",\"params\":\"world\",\"id\":1}}'",
        local_addr
    );

    axum::serve(listener, app).await?;

    Ok(())
}
