//! A simple JSON-RPC 2.0 echo server using HTTP.
//!
//! This example demonstrates how to create a JSON-RPC server using the
//! json-rpc-rs library with HTTP transport. The server provides a single "echo"
//! method that returns any JSON parameters sent to it.
//!
//! Usage:
//!
//! ```bash
//! cargo run --example echo_http_server
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

use anyhow::Result;
use json_rpc::{Http, Methods};
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

    info!("Initializing HTTP echo server");

    let methods = Methods::new().add("echo", echo);

    let transport = Http::new();

    info!("Echo server started on http://localhost:3000");
    info!("Example: POST /jsonrpc with JSON-RPC message");

    info!("Starting server run loop");
    json_rpc::serve(transport, methods).await?;
    info!("Server run loop completed");

    Ok(())
}
