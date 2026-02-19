//! A basic JSON-RPC 2.0 handler demonstrating various method types.
//!
//! This example demonstrates how to create a JSON-RPC handler using the
//! json-rpc-rs library. It provides several methods:
//!
//! - `hello`: Greets the user with their name
//! - `subtract`: Subtracts two numbers
//! - `internal_error`: Demonstrates internal errors
//! - `sum`: Sums an array of numbers
//!
//! Usage:
//!
//! ```bash
//! echo '{"jsonrpc":"2.0","method":"hello","params":"world","id":1}' | cargo run --example basic_stdio
//! ```
//!
//! Expected response:
//!
//! ```json
//! {"jsonrpc":"2.0","result":"Hello, world!","id":1}
//! ```

use anyhow::Result;
use json_rpc::{Error, JsonRpc};
use serde_json::Value;
use tokio::io::AsyncBufReadExt;
use tracing::info;

/// Greet the user with their name.
async fn hello(params: String) -> Result<String, Error> {
    if params == "world" {
        Ok(format!("Hello, {}!", params))
    } else {
        Err(Error::rpc(-32000, "text must be 'world'"))
    }
}

/// Subtract two numbers.
async fn subtract(params: (i32, i32)) -> Result<i32, Error> {
    Ok(params.0 - params.1)
}

/// Sum an array of numbers.
async fn sum(params: Vec<i32>) -> Result<i32, Error> {
    Ok(params.into_iter().sum())
}

/// Demonstrates internal errors.
async fn internal_error(_params: Value) -> Result<String, Error> {
    Err(Error::protocol("Internal error occurred"))
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .with_writer(std::io::stderr)
        .init();

    info!("Initializing basic JSON-RPC handler");

    let json_rpc = JsonRpc::new()
        .add("hello", hello)
        .add("subtract", subtract)
        .add("sum", sum)
        .add("internal_error", internal_error);

    info!("Basic handler started. Send JSON-RPC messages via stdin.");
    info!("Available methods:");
    info!("  - hello(name: string): Returns greeting");
    info!("  - subtract(a: number, b: number): Returns a - b");
    info!("  -sum(numbers: array): Returns sum of numbers");
    info!("  - internal_error(): Demonstrates internal error");
    info!("");
    info!("Examples:");
    info!("  {{\"jsonrpc\":\"2.0\",\"method\":\"hello\",\"params\":\"world\",\"id\":1}}");
    info!("  {{\"jsonrpc\":\"2.0\",\"method\":\"subtract\",\"params\":[10,5],\"id\":2}}");
    info!("  {{\"jsonrpc\":\"2.0\",\"method\":\"sum\",\"params\":[1,2,3,4,5],\"id\":3}}");

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
