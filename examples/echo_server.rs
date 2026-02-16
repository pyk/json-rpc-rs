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
use json_rpc::Server;
use tracing::{debug, info};

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .with_writer(std::io::stderr)
        .init();

    info!("Initializing echo server");
    let mut server = Server::new();

    debug!("Registering 'echo' method");
    server.register("echo", |params: serde_json::Value| {
        debug!("Echo handler called with params: {:?}", params);
        let result = Ok(params);
        debug!("Echo handler returning result: {:?}", result);
        result
    })?;

    eprintln!("Echo server started. Send JSON-RPC messages via stdin.");
    eprintln!("Example: {{\"jsonrpc\":\"2.0\",\"method\":\"echo\",\"params\":\"hello\",\"id\":1}}");
    eprintln!();

    info!("Starting server run loop");
    server.run()?;
    info!("Server run loop completed");

    Ok(())
}
