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
use json_rpc::types::Error;
use json_rpc::{Handler, Request, RequestId, Response, Router};

/// Protocol methods for the echo server.
enum EchoMethod {
    /// Echo method that returns the parameters.
    Echo(RequestId, serde_json::Value),
    /// Unknown method.
    Unknown(RequestId, String),
}

/// Router for the echo server.
struct EchoRouter;

impl Router for EchoRouter {
    type Method = EchoMethod;

    /// Route a JSON-RPC request to an EchoMethod.
    fn route(&self, request: Request) -> Self::Method {
        match request.method.as_str() {
            "echo" => {
                let params = request.params.unwrap_or(serde_json::Value::Null);
                EchoMethod::Echo(request.id, params)
            }
            _ => EchoMethod::Unknown(request.id, request.method),
        }
    }

    /// Handle the routed method.
    fn handle<F>(
        &self,
        method: Self::Method,
        _handler: F,
    ) -> Result<Option<serde_json::Value>, json_rpc::Error>
    where
        F: FnOnce() -> Result<serde_json::Value, json_rpc::Error>,
    {
        match method {
            EchoMethod::Echo(_id, params) => Ok(Some(params)),
            EchoMethod::Unknown(_id, _method) => {
                Err(json_rpc::Error::ProtocolError("Unknown method".to_string()))
            }
        }
    }

    /// Create an error response for unknown methods.
    fn unknown_method_response(&self, id: RequestId, method: &str) -> Response {
        Response::error(
            id,
            Error::method_not_found(format!("Method '{}' not found", method)),
        )
    }
}

fn main() -> Result<()> {
    let router = EchoRouter;

    let mut handler: Handler<EchoRouter> = Handler::new(router);

    println!("Echo server started. Send JSON-RPC messages via stdin.");
    println!("Example: {{\"jsonrpc\":\"2.0\",\"method\":\"echo\",\"params\":\"hello\",\"id\":1}}");
    println!();

    handler.run()?;

    Ok(())
}
