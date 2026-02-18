//! A simple JSON-RPC 2.0 echo server.
//!
//! This example demonstrates how to create a JSON-RPC server using the
//! json-rpc-rs library. The server provides a single "echo" method that
//! returns any JSON parameters sent to it.
//!
//! Usage:
//!
//! ```bash
//! echo '{"jsonrpc":"2.0","method":"echo","params":{"message":"hello"},"id":1}' | cargo run --example echo_server
//! ```
//!
//! Expected response:
//!
//! ```json
//! {"jsonrpc":"2.0","result":{"message":"hello"},"id":1}
//! ```

use anyhow::Result;
use json_rpc::{Methods, Stdio};
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

    info!("Initializing echo server");

    // Build our application with methods
    let methods = Methods::new().add("echo", echo);

    // Create stdio transport
    let transport = Stdio::new();

    info!("Echo server started. Send JSON-RPC messages via stdin.");
    info!("Example: {{\"jsonrpc\":\"2.0\",\"method\":\"echo\",\"params\":\"hello\",\"id\":1}}");

    info!("Starting server run loop");
    json_rpc::serve(transport, methods).await?;
    info!("Server run loop completed");

    Ok(())
}
