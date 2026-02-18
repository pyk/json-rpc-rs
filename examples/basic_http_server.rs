//! A JSON-RPC 2.0 server for error handling testing using HTTP.
//!
//! This example demonstrates JSON-RPC error handling by providing methods that
//! return various types of errors as defined in the JSON-RPC 2.0 specification.
//!
//! ## Methods
//!
//! - `hello(text: String)` - Returns a greeting if text is "world", otherwise
//!   returns a server error (-32000)
//! - `internal_error()` - Simulates an internal server error (-32603)
//!
//! Usage:
//!
//! ```bash
//! cargo run --example basic_http_server
//! ```
//!
//! Then send requests:
//!
//! ```bash
//! # Successful request
//! curl -X POST http://localhost:3001/jsonrpc \
//!   -H "Content-Type: application/json" \
//!   -d '{"jsonrpc":"2.0","method":"hello","params":"world","id":1}'
//!
//! # Custom server error (-32000)
//! curl -X POST http://localhost:3001/jsonrpc \
//!   -H "Content-Type: application/json" \
//!   -d '{"jsonrpc":"2.0","method":"hello","params":"earth","id":2}'
//!
//! # Internal error (-32603)
//! curl -X POST http://localhost:3001/jsonrpc \
//!   -H "Content-Type: application/json" \
//!   -d '{"jsonrpc":"2.0","method":"internal_error","id":3}'
//! ```
//!
//! Expected responses:
//!
//! ```json
//! // Success
//! {"jsonrpc":"2.0","result":"Hello, world!","id":1}
//!
//! // Server error (-32000)
//! {"jsonrpc":"2.0","error":{"code":-32000,"message":"text must be 'world'"},"id":2}
//!
//! // Internal error (-32603)
//! {"jsonrpc":"2.0","error":{"code":-32603,"message":"Protocol error: Internal error occurred"},"id":3}
//! ```

use anyhow::Result;
use json_rpc::{Error, Http, Methods};
use tracing::info;

async fn hello(params: String) -> Result<String, Error> {
    if params != "world" {
        return Err(Error::rpc(-32000, "text must be 'world'"));
    }
    Ok(format!("Hello, {}!", params))
}

async fn internal_error(_params: ()) -> Result<(), Error> {
    Err(Error::protocol("Internal error occurred"))
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .with_writer(std::io::stderr)
        .init();

    info!("Initializing basic HTTP server for error handling tests");

    let methods = Methods::new()
        .add("hello", hello)
        .add("internal_error", internal_error);

    let addr: std::net::SocketAddr = "127.0.0.1:3001".parse()?;
    let transport = Http::new(addr);

    info!("Basic HTTP server started on http://localhost:3001");
    info!("Available methods:");
    info!(
        "  hello(text: String) - Returns greeting if text is 'world', otherwise returns server error"
    );
    info!("  internal_error() - Simulates internal server error");
    info!("Examples:");
    info!("  {{\"jsonrpc\":\"2.0\",\"method\":\"hello\",\"params\":\"world\",\"id\":1}}");
    info!("  {{\"jsonrpc\":\"2.0\",\"method\":\"hello\",\"params\":\"earth\",\"id\":2}}");
    info!("  {{\"jsonrpc\":\"2.0\",\"method\":\"internal_error\",\"id\":3}}");

    info!("Starting server run loop");
    json_rpc::serve(transport, methods).await?;
    info!("Server run loop completed");

    Ok(())
}
