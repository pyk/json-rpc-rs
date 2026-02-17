# json-rpc-rs

A thread pool-based Rust implementation of JSON-RPC 2.0 using blocking I/O. This
library provides a simple, user-friendly API for creating local JSON-RPC servers
with concurrent request handling, graceful shutdown, and support for multiple
transports.

## Features

- **Builder Pattern**: Configure servers with a fluent, intuitive API
- **Type-Safe Methods**: Register methods using closures with automatic
  parameter deserialization
- **Thread Pool**: Handle concurrent requests with a configurable worker pool
- **Multiple Transports**: Use Stdio or InMemory transports, or implement custom
  ones
- **Graceful Shutdown**: Clean server shutdown with signal support
- **Request Cancellation**: Cancel in-flight requests when needed
- **Batch Requests**: Full JSON-RPC 2.0 batch request support
- **Error Handling**: Simple error creation with JSON-RPC compliant error codes

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
json-rpc-rs = "0.1"
```

## Quick Start

Create a simple JSON-RPC server with one method and run it using stdin/stdout:

```rust
use json_rpc::Server;

fn main() -> Result<(), json_rpc::Error> {
    let mut server = Server::new();

    server.register("add", |params: (i32, i32)| {
        Ok(params.0 + params.1)
    })?;

    server.run()?;
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
use json_rpc::Server;

let mut server = Server::new();
```

Configure thread pool size and shutdown signal:

```rust
use json_rpc::{Server, ShutdownSignal};

let shutdown = ShutdownSignal::new();
let mut server = Server::new()
    .with_thread_pool_size(4)
    .with_shutdown_signal(shutdown);
```

### Registering Methods

Register methods with tuple parameters:

```rust
server.register("subtract", |params: (i32, i32)| {
    Ok(params.0 - params.1)
})?;
```

Register methods with a single parameter:

```rust
server.register("echo", |params: String| {
    Ok(params)
})?;
```

Register methods with struct parameters:

```rust
use serde::Deserialize;

#[derive(Deserialize)]
struct Point {
    x: i32,
    y: i32,
}

server.register("distance", |params: Point| {
    Ok((params.x.pow(2) + params.y.pow(2)) as f64).sqrt()
})?;
```

### Running the Server

Run with the default Stdio transport:

```rust
server.run()?;
```

Run with a custom transport:

```rust
use json_rpc::{Server, InMemory};

let (transport, sender) = InMemory::unconnected();
let mut server = Server::new().with_transport(transport);
server.register("echo", |params: String| Ok(params))?;
server.run()?;
```

### Error Handling

Return protocol errors:

```rust
use json_rpc::Error;

server.register("divide", |params: (i32, i32)| {
    if params.1 == 0 {
        return Err(Error::rpc(-32000, "Division by zero"));
    }
    Ok(params.0 / params.1)
})?;
```

Return JSON-RPC standard errors:

```rust
server.register("strict_method", |_params: ()| {
    Err(Error::method_not_found("Method temporarily unavailable"))
})?;
```

### Graceful Shutdown

Use a shutdown signal to stop the server:

```rust
use json_rpc::{Server, ShutdownSignal};
use std::thread;
use std::time::Duration;

let shutdown = ShutdownSignal::new();
let mut server = Server::new().with_shutdown_signal(shutdown.clone());

// In a separate thread, signal shutdown after 5 seconds
thread::spawn(move || {
    thread::sleep(Duration::from_secs(5));
    shutdown.signal();
});

server.run()?;
```

### Batch Requests

The library handles batch requests automatically. Since the server uses NDJSON
(newline-delimited JSON), batch request arrays must be on a single line:

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

### Request Cancellation

Cancel in-flight requests using a cancellation token:

```rust
use json_rpc::{Server, CancellationToken};
use std::sync::Arc;

let token = Arc::new(CancellationToken::new());
let mut server = Server::new();

server.register("long_task", |params: ()| {
    // Check for cancellation
    if token.is_cancelled() {
        return Err(json_rpc::Error::Cancelled);
    }
    // Perform long-running task
    Ok("Task completed".to_string())
})?;
```

## Transports

The library provides two built-in transports:

- **Stdio**: NDJSON (newline-delimited JSON) over stdin/stdout. This is the
  default and ideal for local servers.
- **InMemory**: In-memory transport for testing and in-process communication.

Implement custom transports by implementing the
[`Transport`](src/transports/transport.rs) trait. See the
[transport module](src/transports) for examples.

## Thread Pool

The server uses a fixed-size thread pool for concurrent request handling:

- Default size: Number of CPU cores
- Configure with `.with_thread_pool_size(n)`
- Each request processes in a worker thread
- Responses return to the main thread for transmission

## Limitations

- This library uses blocking I/O. It is not designed for async runtimes like
  tokio.
- The default Stdio transport works with NDJSON (newline-delimited JSON) only.
- This library is designed for local JSON-RPC servers, not distributed systems.
- Custom transports require implementing the
  [`Transport`](src/transports/transport.rs) trait.

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

- `basic_server.rs`: Demonstrates error handling
- `echo_server.rs`: Simple echo server

Run an example:

```bash
cargo run --example basic_server
```

## Documentation

Generate and view API documentation:

```bash
cargo doc --open
```

## License

MIT
