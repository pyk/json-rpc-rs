# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.0] - 2026-02-19

### Breaking Changes

#### Major Architecture Change: Bring Your Own Transport

The library has been refactored from a full server implementation with built-in
transports to a "Bring Your Own Transport" handler pattern. This simplifies the
library and makes it more flexible and framework-agnostic.

**API Changes:**

- `Methods` renamed to `JsonRpc` and moved to `src/jsonrpc.rs`
- `serve()` function removed - users now call `JsonRpc::call()` directly
- All built-in transports removed from the library:
    - `Stdio` transport removed
    - `Http` transport removed
    - `InMemory` transport removed
    - `Transport` trait removed
- `Error::TransportError` variant removed - transport I/O errors should be
  handled by user's transport code
- `Error::Cancelled` variant removed - cancellation handled by transport layer
- `Error::transport()` constructor method removed
- `Default` impl for `JsonRpc` removed - use `JsonRpc::new()` instead
- axum integration is now optional (enable with `features = ["axum"]`)

**Migration Guide:**

Old server-based API:

```rust
use json_rpc::{Methods, Stdio};

let methods = Methods::new().add("echo", echo);
let transport = Stdio::new();
json_rpc::serve(transport, methods).await?;
```

New handler-based API:

```rust
use json_rpc::JsonRpc;

let json_rpc = JsonRpc::new().add("echo", echo);
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

For HTTP integration with axum:

```rust
use axum::{Router, routing::post};
use json_rpc::{JsonRpc, axum::handler};
use std::sync::Arc;

let json_rpc = JsonRpc::new().add("echo", echo);
let app = Router::new()
    .route("/jsonrpc", post(handler))
    .with_state(Arc::new(json_rpc));
let listener = tokio::net::TcpListener::bind(addr).await?;
axum::serve(listener, app).await?;
```

### Changed

- Simplify `from_json` parsing logic with modular helper functions
- Add axum handler in `src/axum.rs` for HTTP integration
- Make axum and http dependencies optional with new feature flags
- Update Cargo.toml metadata and description
- Update documentation throughout codebase to reflect handler pattern

### Fixed

- Fix version number in README.md

## [0.2.0] - 2026-02-18 - "Async JSON-RPC"

### Breaking Changes

#### Major Architecture Change: Thread Pool to Async

The library has been completely refactored from a thread pool-based
implementation to an async/tokio-based architecture. This is a major breaking
change.

**API Changes:**

- `Server::new()` replaced with `Methods::new()`
- `server.register()` replaced with `methods.add()`
- `server.run()` replaced with `json_rpc::serve(transport, methods)`
- Method handlers must now be async functions instead of sync closures
- Example:

    ```rust
    // Old API
    server.register("add", |params: (i32, i32)| {
        Ok(params.0 + params.1)
    })?;
    server.run()?;

    // New API
    methods.add("add", |params: (i32, i32)| async move {
        Ok(params.0 + params.1)
    });
    json_rpc::serve(transport, methods).await?;
    ```

**Removed Features:**

- Thread pool configuration (concurrency now handled by tokio's task scheduler)
- `ShutdownSignal` for graceful server shutdown
- `CancellationToken` for request cancellation
- `with_thread_pool_size()` method
- `with_shutdown_signal()` method
- `with_transport()` method (transport now passed to `serve()`)
- `Server` type and all its methods

**New Features:**

- Full async/await support built on tokio
- HTTP transport using axum web framework
- Improved transport abstraction - each transport implements its own serving
  logic
- Better separation of concerns between protocol handling and transport

**Module Changes:**

- `src/server.rs` removed (functionality moved to lib.rs and transports)
- `src/shutdown.rs` removed
- `src/cancellation.rs` removed
- `src/methods.rs` added (method registry with builder pattern)
- `src/transports/http.rs` added (new HTTP transport)

**Dependency Changes:**

- Added: `tokio` (1.0) - async runtime
- Added: `axum` (0.7) - HTTP web server framework
- Removed: `num_cpus` - no longer needed
- Removed: `assert_cmd` - testing infrastructure changed

**Migration Guide:**

To migrate from the old thread pool-based API:

1. Add `tokio` to your `Cargo.toml` with `features = ["full"]`
2. Change all method handlers to async functions
3. Replace `Server::new()` with `Methods::new()`
4. Replace `server.register()` calls with `methods.add()`
5. Replace `server.run()` with `json_rpc::serve(transport, methods).await`
6. Remove any shutdown signal or cancellation token usage
7. Update to use one of the three transports: `Stdio`, `Http`, or `InMemory`

## [0.1.0] - 2026-02-17 - "Thread Pool Builder"

### Added

#### Core Features

- **Thread Pool-based JSON-RPC 2.0 Server**: A complete implementation of the
  JSON-RPC 2.0 specification with concurrent request handling
- **Builder Pattern API**: Fluent, intuitive API for server configuration and
  method registration
- **Type-Safe Method Registration**: Register methods using closures with
  automatic parameter deserialization via serde
- **Concurrent Request Handling**: Fixed-size thread pool (configurable)
  processes requests in parallel
- **Multiple Transports**:
    - **Stdio transport**: NDJSON (newline-delimited JSON) over stdin/stdout
    - **InMemory transport**: For testing and in-process communication
    - **Custom transports**: Implement the `Transport` trait for your own
      transport layer
- **Graceful Shutdown**: Clean server shutdown with signal support via
  `ShutdownSignal`
- **Request Cancellation**: Cancel in-flight requests using `CancellationToken`
- **Batch Request Support**: Full JSON-RPC 2.0 batch request handling

#### Error Handling

- **Comprehensive Error Types**: Support for all JSON-RPC 2.0 error codes:
    - Parse errors (-32700)
    - Invalid request errors (-32600)
    - Method not found (-32601)
    - Invalid params (-32602)
    - Internal errors (-32603)
    - Server errors (-32000 to -32099) for custom application errors
- **Error Response Generation**: Automatic generation of JSON-RPC compliant
  error responses
- **Request ID Preservation**: Preserves request IDs in error responses per
  specification

#### Modules

- `server`: Core server implementation with thread pool and builder pattern
- `transports`: Transport implementations (Stdio, InMemory) and Transport trait
- `types`: JSON-RPC 2.0 message types (Request, Response, Notification, Error)
- `shutdown`: Shutdown signal for graceful server shutdown
- `cancellation`: Cancellation token for request cancellation
- `error`: Internal error types and JSON-RPC error definitions

#### Examples

- **echo_server**: Simple echo server demonstrating basic functionality
- **basic_server**: Error handling example with multiple error scenarios

#### Documentation

- Comprehensive README with:
    - Feature overview
    - Installation instructions
    - Quick start guide
    - Detailed usage examples
    - Architecture description
    - Design goals and limitations
    - JSON-RPC 2.0 compliance notes
- Full API documentation with examples
- Inline code documentation for all public APIs

#### Testing

- **Integration Tests**:
    - 11 echo_server tests covering various parameter types
    - 24 basic_server tests for comprehensive error handling
- **Doc Tests**: 12 doctests embedded in documentation
- **Test Coverage**: All major features tested including:
    - String, object, array, null, boolean, number parameters
    - Nested JSON structures
    - Unicode characters
    - Empty strings and values
    - Large JSON structures
    - All JSON-RPC 2.0 error codes
    - Batch requests
    - Notifications (no-response behavior)
    - Graceful shutdown

#### Dependencies

- `serde` (1.0): Serialization/deserialization
- `serde_json` (1.0): JSON processing
- `thiserror` (latest): Error derivation
- `num_cpus` (latest): CPU core detection for default thread pool size
- `anyhow` (dev-dependency): Error handling for examples
- `assert_cmd` (dev-dependency): Integration test support
- `tracing` (dev-dependency): Logging for debugging

#### Cargo Metadata

- Project description
- Keywords: json-rpc, rpc, server, json
- Categories: network-programming
- License: MIT
- Repository: https://github.com/pyk/json-rpc-rs
- Documentation: https://docs.rs/json-rpc-rs

### Architecture

The library is organized into six main components:

| Component      | Responsibility                                     |
| -------------- | -------------------------------------------------- |
| `types`        | JSON-RPC 2.0 message structures and serialization  |
| `transports`   | Reading/writing messages to/from transports        |
| `server`       | Method registration, request handling, thread pool |
| `shutdown`     | Graceful shutdown signaling                        |
| `cancellation` | Request cancellation                               |
| `error`        | Error types and handling                           |

### Key Design Decisions

1. **Blocking I/O**: Uses blocking I/O for simplicity. Not designed for async
   runtimes like tokio.
2. **Builder Pattern**: Eliminates boilerplate with fluent API for server
   configuration
3. **Type Erasure**: Uses `HashMap<String, Box<dyn HandlerFn>>` for flexible
   method registration
4. **Fixed Thread Pool**: Deterministic resource usage with configurable pool
   size
5. **NDJSON Transport**: Default transport uses newline-delimited JSON for clean
   message boundaries

### Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
json-rpc-rs = "0.1"
```

### Quick Start

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

### Breaking Changes

This is the initial release (v0.1.0). There are no breaking changes from
previous versions.

### Known Limitations

- Blocking I/O only (not suitable for async runtimes)
- Stdio transport requires NDJSON (newline-delimited JSON)
- Designed for local JSON-RPC servers, not distributed systems
- Batch request arrays must be on a single line due to NDJSON format
- Custom transports require implementing the `Transport` trait

### JSON-RPC 2.0 Compliance

This library implements the JSON-RPC 2.0 specification including:

- Request objects with `jsonrpc`, `method`, `params`, and `id` fields
- Notification objects (requests without `id`)
- Response objects with `result` or `error` fields
- Error objects with `code`, `message`, and optional `data` fields
- Batch requests (arrays of requests)
- Standard error codes (-32700 to -32099)

[0.3.0]: https://github.com/pyk/json-rpc-rs/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/pyk/json-rpc-rs/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/pyk/json-rpc-rs/releases/tag/v0.1.0
