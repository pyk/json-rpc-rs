//! A simple JSON-RPC 2.0 echo handler using stdio.
//!
//! This example demonstrates how to create a JSON-RPC handler using the
//! json-rpc-rs library. The handler provides a single "echo" method that
//! returns any JSON parameters sent to it.
//!
//! Usage:
//!
//! ```bash
//! echo '{"jsonrpc":"2.0","method":"echo","params":{"message":"hello"},"id":1}' | cargo run --example echo_stdio
//! ```
//!
//! Expected response:
//!
//! ```json
//! {"jsonrpc":"2.0","result":{"message":"hello"},"id":1}
//! ```

use anyhow::Result;
use json_rpc::JsonRpc;
use serde_json::Value;
use tokio::io::AsyncBufReadExt;
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

    info!("Echo handler started. Send JSON-RPC messages via stdin.");
    info!("Example: {{\"jsonrpc\":\"2.0\",\"method\":\"echo\",\"params\":\"hello\",\"id\":1}}");

    let stdin = tokio::io::stdin();
    let mut reader = tokio::io::BufReader::new(stdin);
    let mut line = String::new();

    info!("Starting message processing loop");

    while reader.read_line(&mut line).await? > 0 {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            line.clear();
            continue;
        }

        info!("Processing message: {}", trimmed);

        match json_rpc.call(trimmed).await {
            Some(response) => {
                info!("Sending response: {}", response);
                println!("{}", response);
            }
            None => {
                info!("Notification processed - no response needed");
            }
        }

        line.clear();
    }

    info!("Message processing loop completed");
    Ok(())
}
