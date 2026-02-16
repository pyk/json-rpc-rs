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

fn main() -> Result<()> {
    let mut server = Server::new();

    // Register the echo method that returns the parameters unchanged
    server.register("echo", |params: serde_json::Value| Ok(params))?;

    eprintln!("Echo server started. Send JSON-RPC messages via stdin.");
    eprintln!("Example: {{\"jsonrpc\":\"2.0\",\"method\":\"echo\",\"params\":\"hello\",\"id\":1}}");
    eprintln!();

    server.run()?;

    Ok(())
}
