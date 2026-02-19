//! A simple JSON-RPC 2.0 echo handler using axum.
//!
//! This example demonstrates how to create a JSON-RPC handler using the
//! json-rpc-rs library and integrate it with axum. The handler provides a
//! single "echo" method that returns any JSON parameters sent to it.
//!
//! This example requires the "axum" feature to be enabled.
//!
//! Usage:
//!
//! ```bash
//! cargo run --example echo_axum
//! ```
//!
//! Then send requests:
//!
//! ```bash
//! curl -X POST http://localhost:3000/jsonrpc \
//!   -H "Content-Type: application/json" \
//!   -d '{"jsonrpc":"2.0","method":"echo","params":{"message":"hello"},"id":1}'
//! ```
//!
//! Expected response:
//!
//! ```json
//! {"jsonrpc":"2.0","result":{"message":"hello"},"id":1}
//! ```

use std::sync::Arc;

use anyhow::Result;

use axum::Router;
use axum::routing::post;
use json_rpc::JsonRpc;
use json_rpc::axum::handler;
use serde_json::Value;
use tracing::info;

async fn echo(params: Value) -> Result<Value, json_rpc::Error> {
    Ok(params)
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .with_writer(std::io::stderr)
        .init();

    info!("Initializing JSON-RPC handler");

    let json_rpc = JsonRpc::new().add("echo", echo);

    let app = Router::new()
        .route("/jsonrpc", post(handler))
        .with_state(Arc::new(json_rpc));

    let addr: std::net::SocketAddr = "127.0.0.1:3000".parse()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;
    let local_addr = listener.local_addr()?;

    info!("Server started on http://{}", local_addr);
    info!("JSON-RPC endpoint: http://{}/jsonrpc", local_addr);
    info!("Available methods:");
    info!("  - echo: Echoes back any JSON params");
    info!("");
    info!("Example request:");
    info!(
        "  curl -X POST http://{}/jsonrpc -H \"Content-Type: application/json\" -d '{{\"jsonrpc\":\"2.0\",\"method\":\"echo\",\"params\":\"hello\",\"id\":1}}'",
        local_addr
    );

    axum::serve(listener, app).await?;

    Ok(())
}
