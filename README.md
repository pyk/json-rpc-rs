# json-rpc-rs

A framework-agnostic async Rust implementation of JSON-RPC 2.0 with Bring Your
Own Transport. Process JSON-RPC messages with async/await support and full
specification compliance.

## Features

- Configure handlers with the builder pattern
- Register methods with closures and automatic parameter deserialization
- Process messages asynchronously with tokio
- Bring Your Own Transport: integrate with stdio, HTTP, WebSocket, TCP, or any
  custom transport
- Support requests, responses, notifications, and errors
- Handle batch requests
- Create errors with JSON-RPC compliant codes

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
json-rpc-rs = "0.3"
tokio = { version = "1", features = ["rt", "io-util"] }
```

For axum integration, enable the feature:

```toml
[dependencies]
json-rpc-rs = { version = "0.3", features = ["axum"] }
```

## Quick Start

Create a JSON-RPC handler and process messages. Since this library uses Bring
Your Own Transport, you read JSON strings from your transport, call
`json_rpc.call()`, and write the response back.

```rust
use json_rpc::JsonRpc;
use serde_json::Value;

async fn echo(params: Value) -> Result<Value, json_rpc::Error> {
    Ok(params)
}

#[tokio::main]
async fn main() -> Result<(), json_rpc::Error> {
    let json_rpc = JsonRpc::new()
        .add("echo", echo);

    // Read from your transport (stdin, HTTP, WebSocket, TCP, etc.)
    let message = r#"{"jsonrpc":"2.0","method":"echo","params":"hello","id":1}"#;
    if let Some(response) = json_rpc.call(message).await {
        // Write the response back to your transport
        println!("{}", response);
    }

    Ok(())
}
```

## Usage

### Creating a Handler

```rust
use json_rpc::JsonRpc;

let json_rpc = JsonRpc::new();
```

### Registering Methods

Register methods with tuple parameters:

```rust
json_rpc.add("subtract", |params: (i32, i32)| async move {
    Ok(params.0 - params.1)
});
```

Register methods with a single parameter:

```rust
json_rpc.add("echo", |params: String| async move {
    Ok(params)
});
```

Register methods with struct parameters:

```rust
use serde::Deserialize;

#[derive(Deserialize)]
struct Point {
    x: i32,
    y: i32,
}

json_rpc.add("distance", |params: Point| async move {
    Ok((params.x.pow(2) + params.y.pow(2)) as f64).sqrt())
});
```

### Processing Messages

The `call()` method processes JSON-RPC messages. Pass a JSON string from your
transport, get back a JSON response string. Returns `None` for notifications
(requests without `id`). This is the Bring Your Own Transport pattern in action.

```rust
match json_rpc.call(json_message).await {
    Some(response) => {
        // Handle response
        println!("{}", response);
    }
    None => {
        // Notification - no response needed
    }
}
```

### Example: Stdio

Read newline-delimited JSON from stdin and write responses to stdout:

```rust
use json_rpc::JsonRpc;
use tokio::io::AsyncBufReadExt;

let stdin = tokio::io::stdin();
let mut reader = tokio::io::BufReader::new(stdin);
let mut line = String::new();

while reader.read_line(&mut line).await? > 0 {
    let trimmed = line.trim();
    if !trimmed.is_empty() {
        if let Some(response) = json_rpc.call(trimmed).await {
            println!("{}", response);
        }
    }
    line.clear();
}
```

Send a request:

```bash
echo '{"jsonrpc":"2.0","method":"echo","params":"hello","id":1}' | cargo run
```

### Example: Axum

The axum feature provides a handler for HTTP integration:

```rust
use axum::{Router, routing::post};
use json_rpc::{JsonRpc, axum::handler};
use std::sync::Arc;

let json_rpc = JsonRpc::new().add("echo", echo);
let app = Router::new()
    .route("/jsonrpc", post(handler))
    .with_state(Arc::new(json_rpc));
```

Send a request:

```bash
curl -X POST http://localhost:3000/jsonrpc \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"echo","params":"hello","id":1}'
```

### Custom Transports

The Bring Your Own Transport pattern lets you integrate with any transport or
framework. Read a JSON string, call `json_rpc.call()`, write the response.

```rust
use json_rpc::JsonRpc;

let json_rpc = JsonRpc::new()
    .add("my_method", my_handler);

// Read from WebSocket, TCP, custom protocol, etc.
let message = receive_message();
if let Some(response) = json_rpc.call(&message).await {
    send_response(response);
}
```

### Error Handling

Return protocol errors:

```rust
use json_rpc::Error;

json_rpc.add("divide", |params: (i32, i32)| async move {
    if params.1 == 0 {
        return Err(Error::rpc(-32000, "Division by zero"));
    }
    Ok(params.0 / params.1)
});
```

Return JSON-RPC standard errors:

```rust
json_rpc.add("strict_method", |_params: ()| async move {
    Err(Error::rpc(-32601, "Method not found"))
});
```

### Batch Requests

The `call()` method accepts JSON arrays representing batch requests and returns
an array of responses.

```bash
echo '[
  {"jsonrpc":"2.0","method":"add","params":[1,2],"id":"1"},
  {"jsonrpc":"2.0","method":"add","params":[3,4],"id":"2"}
]' | cargo run
```

Response:

```json
[
    { "jsonrpc": "2.0", "result": 3, "id": "1" },
    { "jsonrpc": "2.0", "result": 7, "id": "2" }
]
```

## JSON-RPC 2.0 Compliance

Implements the JSON-RPC 2.0 specification:

- Request objects with `jsonrpc`, `method`, `params`, and `id` fields
- Notification objects (requests without `id`)
- Response objects with `result` or `error` fields
- Error objects with `code`, `message`, and optional `data` fields
- Batch requests (arrays of requests)
- Standard error codes (-32700 to -32099)

See [JSON-RPC 2.0 Specification](https://www.jsonrpc.org/specification) for
details.

## Examples

The [examples](examples) directory contains working examples:

- `echo_stdio.rs`: Echo handler using stdio
- `echo_axum.rs`: Echo handler using axum (requires `axum` feature)
- `basic_stdio.rs`: Handler with multiple methods and error handling
- `basic_axum.rs`: Handler with multiple methods and error handling (requires
  `axum` feature)
- `graceful_shutdown_http.rs`: Graceful shutdown with axum (requires `axum`
  feature)

Run an example:

```bash
# Stdio example
echo '{"jsonrpc":"2.0","method":"echo","params":"hello","id":1}' | cargo run --example echo_stdio

# Axum example
cargo run --example echo_axum --features axum
```

## Documentation

Generate and view API documentation:

```bash
cargo doc --open
```

## License

MIT
