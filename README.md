# json-rpc-rs

An async Rust implementation of JSON-RPC 2.0. This library provides a simple,
user-friendly API for creating JSON-RPC handlers with async/await support and
full JSON-RPC 2.0 compliance.

## Features

- **Builder Pattern**: Configure handlers with a fluent, intuitive API
- **Type-Safe Methods**: Register methods using closures with automatic
  parameter deserialization
- **Async/Await**: Built on tokio for efficient async request handling
- **Multiple Integrations**: Use stdio, HTTP (via axum), or custom integrations
- **JSON-RPC 2.0 Compliant**: Full support for requests, responses,
  notifications, and errors
- **Batch Requests**: Full JSON-RPC 2.0 batch request support
- **Error Handling**: Simple error creation with JSON-RPC compliant error codes

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
json-rpc-rs = "0.2"
tokio = { version = "1", features = ["rt", "io-util"] }
```

For axum integration, enable the feature:

```toml
[dependencies]
json-rpc-rs = { version = "0.2", features = ["axum"] }
```

## Quick Start

### Stdio

Create a simple JSON-RPC handler with one method and run it using stdio:

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

    Ok(())
}
```

Send a request:

```bash
echo '{"jsonrpc":"2.0","method":"echo","params":"hello","id":1}' | cargo run
```

Response:

```json
{ "jsonrpc": "2.0", "result": "hello", "id": 1 }
```

### Axum (HTTP)

Create a simple JSON-RPC server with one method using axum:

```rust
use axum::{Router, routing::post};
use json_rpc::{JsonRpc, axum};
use serde_json::Value;
use std::sync::Arc;

async fn echo(params: Value) -> Result<Value, json_rpc::Error> {
    Ok(params)
}

#[tokio::main]
async fn main() -> Result<(), json_rpc::Error> {
    let json_rpc = JsonRpc::new()
        .add("echo", echo);

    let app = Router::new()
        .route("/jsonrpc", post(axum::handler))
        .with_state(Arc::new(json_rpc));

    let addr: std::net::SocketAddr = "127.0.0.1:3000".parse()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
```

Send a request:

```bash
curl -X POST http://localhost:3000/jsonrpc \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"echo","params":"hello","id":1}'
```

Response:

```json
{ "jsonrpc": "2.0", "result": "hello", "id": 1 }
```

## Usage

### Creating a Handler

Create a handler with default configuration:

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

Process JSON-RPC messages directly:

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

### Stdio Integration

The stdio integration reads newline-delimited JSON from stdin and processes each
line:

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

This is ideal for command-line tools, LSP (Language Server Protocol)
implementations, and process communication.

### Axum Integration

The axum integration (enabled with the `axum` feature) provides a simple way to
serve JSON-RPC over HTTP:

```rust
use axum::{Router, routing::post};
use json_rpc::{JsonRpc, axum::handler};
use std::sync::Arc;

let json_rpc = JsonRpc::new().add("echo", echo);
let app = Router::new()
    .route("/jsonrpc", post(handler))
    .with_state(Arc::new(json_rpc));
```

The `handler` function extracts the request body, calls `json_rpc.call()`, and
returns the appropriate HTTP response. This handles:

- Request body parsing
- JSON-RPC message processing
- Response serialization
- Error handling

This is ideal for web services and HTTP-based APIs.

### Custom Integrations

Since json-rpc-rs focuses on message processing rather than being a full server,
you can easily integrate it with any transport or framework:

```rust
use json_rpc::JsonRpc;

let json_rpc = JsonRpc::new()
    .add("my_method", my_handler);

// Use with your custom transport
// Example: WebSocket, TCP, etc.
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

The library handles batch requests automatically. The `call()` method accepts
JSON arrays representing batch requests and returns an array of responses.

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

This library implements the JSON-RPC 2.0 specification, including:

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

- `echo_stdio.rs`: Simple echo handler using stdio
- `echo_axum.rs`: Simple echo handler using axum (requires `axum` feature)
- `basic_stdio.rs`: Advanced handler with multiple methods and error handling
- `basic_axum.rs`: Advanced handler with multiple methods and error handling
  (requires `axum` feature)
- `graceful_shutdown_http.rs`: Demonstrates graceful shutdown with axum
  (requires `axum` feature)

Run an example:

```bash
# Stdio examples (pipe JSON to stdin)
echo '{"jsonrpc":"2.0","method":"echo","params":"hello","id":1}' | cargo run --example echo_stdio

# Axum examples (requires axum feature)
cargo run --example echo_axum --features axum
```

## Documentation

Generate and view API documentation:

```bash
cargo doc --open
```

## License

MIT
