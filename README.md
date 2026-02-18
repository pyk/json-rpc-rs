# json-rpc-rs

An async Rust implementation of JSON-RPC 2.0. This library provides a simple,
user-friendly API for creating JSON-RPC servers with async/await support,
multiple transport options, and full JSON-RPC 2.0 compliance.

## Features

- **Builder Pattern**: Configure methods with a fluent, intuitive API
- **Type-Safe Methods**: Register methods using closures with automatic
  parameter deserialization
- **Async/Await**: Built on tokio for efficient async request handling
- **Multiple Transports**: Use Stdio, HTTP, or InMemory transports, or implement
  custom ones
- **JSON-RPC 2.0 Compliant**: Full support for requests, responses,
  notifications, and errors
- **Batch Requests**: Full JSON-RPC 2.0 batch request support
- **Error Handling**: Simple error creation with JSON-RPC compliant error codes

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
json-rpc-rs = "0.2"
tokio = { version = "1", features = ["full"] }
```

## Quick Start

Create a simple JSON-RPC server with one method and run it using stdin/stdout:

```rust
use json_rpc::{Methods, Stdio};
use serde_json::Value;

async fn add(params: (i32, i32)) -> Result<i32, json_rpc::Error> {
    Ok(params.0 + params.1)
}

#[tokio::main]
async fn main() -> Result<(), json_rpc::Error> {
    let methods = Methods::new()
        .add("add", add);

    let transport = Stdio::new();
    json_rpc::serve(transport, methods).await?;
    Ok(())
}
```

Send a request:

```bash
echo '{"jsonrpc":"2.0","method":"add","params":[5,3],"id":1}' | cargo run
```

Response:

```json
{ "jsonrpc": "2.0", "result": 8, "id": 1 }
```

## Usage

### Creating a Server

Create a server with default configuration:

```rust
use json_rpc::{Methods, Stdio};

let methods = Methods::new();
let transport = Stdio::new();
json_rpc::serve(transport, methods).await?;
```

### Registering Methods

Register methods with tuple parameters:

```rust
methods.add("subtract", |params: (i32, i32)| async move {
    Ok(params.0 - params.1)
});
```

Register methods with a single parameter:

```rust
methods.add("echo", |params: String| async move {
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

methods.add("distance", |params: Point| async move {
    Ok((params.x.pow(2) + params.y.pow(2)) as f64).sqrt()
});
```

### Running the Server

Run with the default Stdio transport:

```rust
let transport = Stdio::new();
json_rpc::serve(transport, methods).await?;
```

Run with HTTP transport:

```rust
use json_rpc::Http;
use std::net::SocketAddr;

let addr: SocketAddr = "127.0.0.1:3000".parse().unwrap();
let transport = Http::new(addr);
json_rpc::serve(transport, methods).await?;
```

### Error Handling

Return protocol errors:

```rust
use json_rpc::Error;

methods.add("divide", |params: (i32, i32)| async move {
    if params.1 == 0 {
        return Err(Error::rpc(-32000, "Division by zero"));
    }
    Ok(params.0 / params.1)
});
```

Return JSON-RPC standard errors:

```rust
methods.add("strict_method", |_params: ()| async move {
    Err(Error::rpc(-32601, "Method not found"))
});
```

### Batch Requests

The library handles batch requests automatically. Since the stdio transport uses
NDJSON (newline-delimited JSON), batch request arrays must be on a single line:

```bash
echo '[{"jsonrpc":"2.0","method":"add","params":[1,2],"id":"1"},{"jsonrpc":"2.0","method":"add","params":[3,4],"id":"2"}]' | cargo run
```

Response:

```json
[
    { "jsonrpc": "2.0", "result": 3, "id": "1" },
    { "jsonrpc": "2.0", "result": 7, "id": "2" }
]
```

## Transports

The library provides three built-in transports:

- **Stdio**: NDJSON (newline-delimited JSON) over stdin/stdout. This is the
  default and ideal for local servers.
- **HTTP**: HTTP POST requests using axum web framework. Perfect for web-based
  APIs.
- **InMemory**: In-memory transport for testing and in-process communication.

Implement custom transports by implementing the
[`Transport`](src/transports/transport.rs) trait. See the
[transport module](src/transports) for examples.

### Stdio Transport

The Stdio transport reads newline-delimited JSON from stdin and writes
newline-terminated JSON to stdout:

```rust
use json_rpc::Stdio;

let transport = Stdio::new();
json_rpc::serve(transport, methods).await?;
```

### HTTP Transport

The HTTP transport accepts POST requests at `/jsonrpc`:

```rust
use json_rpc::Http;
use std::net::SocketAddr;

let addr: SocketAddr = "127.0.0.1:3000".parse().unwrap();
let transport = Http::new(addr);
json_rpc::serve(transport, methods).await?;
```

Send requests using curl:

```bash
curl -X POST http://localhost:3000/jsonrpc \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"add","params":[5,3],"id":1}'
```

### InMemory Transport

The InMemory transport is useful for testing:

```rust
use json_rpc::InMemory;

let (transport, sender) = InMemory::unconnected();
tokio::spawn(async move {
    transport.serve(methods).await.unwrap();
});

sender.send(r#"{"jsonrpc":"2.0","method":"echo","params":"hello","id":1}"#.to_string()).await?;
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

- `echo_stdio_server.rs`: Simple echo server using stdio
- `echo_http_server.rs`: Simple echo server using HTTP
- `basic_stdio_server.rs`: Demonstrates error handling with stdio
- `basic_http_server.rs`: Demonstrates error handling with HTTP

Run an example:

```bash
# Stdio examples (pipe JSON to stdin)
echo '{"jsonrpc":"2.0","method":"echo","params":"hello","id":1}' | cargo run --example echo_stdio_server

# HTTP examples (start server, then use curl)
cargo run --example echo_http_server
curl -X POST http://localhost:3000/jsonrpc \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"echo","params":"hello","id":1}'
```

## Documentation

Generate and view API documentation:

```bash
cargo doc --open
```

## License

MIT
