# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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

[0.1.0]: https://github.com/pyk/json-rpc-rs/releases/tag/v0.1.0
