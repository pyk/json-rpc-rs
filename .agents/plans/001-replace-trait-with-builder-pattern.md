---
title: "Replace Trait-Based with Builder Pattern"
seq: 001
slug: "replace-trait-with-builder-pattern"
created: "2026-02-16T17:09:29Z"
status: completed
---

# Replace Trait-Based with Builder Pattern

Replace the trait-based method registration API with a builder pattern that
eliminates boilerplate and simplifies method definition. This change removes the
Router trait and Handler struct, replacing them with a Server struct that uses
closures for method registration.

## Current Problems

The current implementation requires users to implement the Router trait and
define an enum for protocol methods. This creates boilerplate and complexity.

```rust
// Current approach - requires trait implementation and enum
enum MyProtocolMethod {
    Initialize(RequestId),
    DoSomething(RequestId),
}

struct MyRouter;

impl Router for MyRouter {
    type Method = MyProtocolMethod;

    fn route(&self, request: Request) -> Self::Method {
        match request.method.as_str() {
            "initialize" => MyProtocolMethod::Initialize(request.id),
            "doSomething" => MyProtocolMethod::DoSomething(request.id),
            _ => MyProtocolMethod::Unknown(request.id, request.method),
        }
    }

    fn handle<F>(&self, method: Self::Method, handler: F) -> Result<Option<serde_json::Value>, Error>
    where
        F: FnOnce() -> Result<serde_json::Value, Error>,
    {
        match method {
            MyProtocolMethod::Initialize(id) => handler().map(Some),
            MyProtocolMethod::DoSomething(id) => handler().map(Some),
            MyProtocolMethod::Unknown(_, _) => Err(Error::protocol("Unknown method")),
        }
    }

    fn unknown_method_response(&self, id: RequestId, method: &str) -> Response {
        Response::error(id, json_rpc::types::Error::method_not_found(method))
    }
}

let router = MyRouter;
let mut handler: Handler<MyRouter, Stdio> = Handler::new(router);
handler.run()?;
```

## Proposed Solution

Implement a builder pattern API that allows direct method registration with
type-safe parameters.

```rust
// New approach - simple and direct
let shutdown = ShutdownSignal::new();

let mut server = Server::new()
    .with_thread_pool_size(4)
    .with_shutdown_signal(shutdown);

server.register("initialize", |params: InitParams| {
    Ok(serde_json::json!({ "status": "initialized" }))
})?;

server.register("doSomething", |params: DoSomethingParams| {
    Ok(params.value + 1)
})?;

// Single run() method - shutdown signal is optional
server.run()?;
```

Key changes:

1. Remove Router trait and Handler struct
2. Create Server struct with builder methods
3. Create ShutdownSignal for graceful shutdown (configured via builder)
4. Create CancellationToken for request cancellation
5. Create ThreadPool for concurrent request handling
6. Rewrite error.rs to include Cancelled error
7. Modify transport layer for alignment
8. Restructure modules
9. Single `run()` method - shutdown signal is optional via
   `.with_shutdown_signal()`

## Analysis Required

### Dependency Investigation

- [ ] Review existing thiserror usage in error.rs
- [ ] Review current Transport trait methods needed by Server
- [ ] Review ThreadPool implementation details from builder-pattern.md

### Code Locations to Check

- `json-rpc-rs/src/error.rs` - Current error definitions
- `json-rpc-rs/src/router.rs` - Router trait to remove
- `json-rpc-rs/src/handler.rs` - Handler implementation to replace
- `json-rpc-rs/src/transports/transport.rs` - Transport trait alignment
- `json-rpc-rs/src/lib.rs` - Module exports to update

## Implementation Checklist

### Code Changes

#### Error Handling

- [x] Rewrite `src/error.rs` with thiserror
- [x] Add `Cancelled` error variant for CancellationToken
- [x] Keep `ProtocolError`, `TransportError`, `ParseError` variants
- [x] Ensure all errors implement `std::error::Error` and `Display`

#### Shutdown Signal

- [x] Create `src/shutdown.rs` module
- [x] Implement `ShutdownSignal` struct with `Arc<AtomicBool>`
- [x] Implement `new()` method
- [x] Implement `check_shutdown()` that returns `Err(Error)` if shutdown
      requested
- [x] Implement `is_shutdown_requested()` that returns `bool`
- [x] Implement `signal()` method for thread-safe shutdown signaling

#### Cancellation Token

- [x] Create `src/cancellation.rs` module
- [x] Implement `CancellationToken` struct with `Arc<AtomicBool>`
- [x] Implement `new()` method
- [x] Implement `check_cancelled()` that returns `Err(Error::Cancelled)`
- [x] Implement `is_cancelled()` that returns `bool`
- [x] Implement `cancel()` method

#### Thread Pool

- [x] Implement `ThreadPool` struct in `src/server.rs`
- [x] Implement `Worker` struct for thread pool workers
- [x] Implement job queue with `mpsc::channel`
- [x] Implement `execute()` method for submitting jobs
- [x] Implement `Drop` trait for graceful shutdown
- [x] Ensure workers finish current jobs before exit

#### Server Implementation

- [x] Create `src/server.rs` module
- [x] Implement `Server` struct with
      `handlers: HashMap<String, Box<dyn HandlerFn>>`
- [x] Implement `thread_pool_size: usize` field
- [x] Implement internal `HandlerFn` trait for type erasure
- [x] Implement `new()` method with default thread pool size (CPU cores)
- [x] Implement `with_thread_pool_size()` builder method
- [x] Implement `register<F, P, R>()` method with type-safe parameters
    - `F: Fn(P) -> Result<R, Error> + Send + Sync + 'static`
    - `P: DeserializeOwned + Send + Sync + 'static`
    - `R: Serialize + Send + Sync + 'static`
- [x] Implement `with_shutdown_signal()` builder method (optional)
- [x] Implement `run()` method using Stdio transport
    - Check if shutdown signal is configured
    - If configured: wait for shutdown signal
    - If not configured: wait for EOF
- [x] Implement `run_with_transport<T>()` generic method
    - Same shutdown logic as `run()`
- [x] Implement message parsing and handler dispatch
- [x] Implement concurrent request handling with thread pool
- [x] Implement graceful shutdown logic with configured shutdown signal

#### Transport Layer

- [x] Verify `src/transports/transport.rs` compatibility with Server
- [x] Ensure `receive_message()`, `send_response()`, `send_notification()` work
      with Server
- [x] Keep existing implementations (Stdio, InMemory) compatible

#### Library Exports

- [x] Update `src/lib.rs` module declarations
- [x] Remove `pub use handler::Handler;`
- [x] Remove `pub use router::{ErrorExt, JsonRpcErrorExt, Router};`
- [x] Add `pub use server::Server;`
- [x] Add `pub use shutdown::ShutdownSignal;`
- [x] Add `pub use cancellation::CancellationToken;`
- [x] Keep `pub use transports::{InMemory, Stdio, Transport};`
- [x] Keep
      `pub use types::{Message, Notification, Request, RequestId, Response};`
- [x] Update lib.rs documentation with builder pattern examples

#### Cleanup

- [x] Delete `src/router.rs` file
- [x] Delete `src/handler.rs` file

### Documentation Updates

- [x] Update `src/lib.rs` documentation with builder pattern examples
- [x] Add ShutdownSignal documentation
- [x] Add CancellationToken documentation
- [x] Add Server API documentation
- [x] Update examples/echo_server.rs with new API examples

### Test Updates

- [x] No tests to write (per user request)
- [x] Ensure code compiles without tests

### Test Plan

#### Verification Tests

- [x] Create a simple echo server and verify it compiles
- [x] Test `register()` method with different parameter types
- [x] Test `with_thread_pool_size()` builder method
- [x] Test `with_shutdown_signal()` builder method
- [x] Test `run()` method without shutdown signal (uses EOF)
- [x] Test `run()` method with shutdown signal configured
- [x] Test `run_with_transport()` with different transports
- [x] Test fluent API:
      `Server::new().with_thread_pool_size(4).with_shutdown_signal(shutdown).run()?`
- [x] Test CancellationToken usage within handlers

#### Regression Tests

- [x] Verify Stdio transport works with new Server
- [x] Verify InMemory transport works with new Server
- [x] Verify error handling works correctly
- [x] Verify JSON-RPC 2.0 compliance

## Structure After Changes

### File Structure

```
json-rpc-rs/src/
├── cancellation.rs      # NEW: CancellationToken for cancellation
├── error.rs              # MODIFIED: Updated with Cancelled variant
├── lib.rs                # MODIFIED: Updated exports and documentation
├── server.rs             # NEW: Server with builder pattern and ThreadPool
├── shutdown.rs           # NEW: ShutdownSignal for graceful shutdown
├── types.rs              # UNCHANGED: JSON-RPC message types
└── transports/
    ├── in_memory.rs      # UNCHANGED: InMemory transport
    ├── mod.rs            # UNCHANGED: Transport module
    ├── stdio.rs          # UNCHANGED: Stdio transport
    └── transport.rs      # MODIFIED: If needed for Server compatibility
```

### Module Exports (lib.rs)

```rust
pub use error::Error;
pub use server::Server;
pub use shutdown::ShutdownSignal;
pub use cancellation::CancellationToken;
pub use transports::{InMemory, Stdio, Transport};
pub use types::{Message, Notification, Request, RequestId, Response};

pub mod cancellation;
pub mod error;
pub mod server;
pub mod shutdown;
pub mod transports;
pub mod types;
```

## Design Considerations

1. **Thread Pool Architecture**: Fixed-size thread pool created on server start
    - **Alternative**: Lazy thread pool creation - rejected for simplicity
    - **Rationale**: Deterministic resource usage, easier testing

2. **Handler Storage**: Use `HashMap<String, Box<dyn HandlerFn>>` for type
   erasure
    - **Alternative**: Use enum with variants - rejected for inflexibility
    - **Rationale**: Allows any handler signature, supports dynamic registration

3. **Shutdown Signaling**: Use `Arc<AtomicBool>` for thread-safe shutdown
    - **Alternative**: Use channels - rejected for complexity
    - **Rationale**: Simpler API, efficient for boolean signaling

4. **Error Handling**: Extend existing Error enum with Cancelled variant
    - **Alternative**: Create separate error type - rejected for fragmentation
    - **Rationale**: Single error type simplifies user code

5. **Module Structure**: Create separate modules for Server, Shutdown,
   CancellationToken
    - **Alternative**: Put all in one file - rejected for maintainability
    - **Rationale**: Clear separation of concerns, easier to navigate

## Success Criteria

- Server struct with builder pattern compiles and runs
- Method registration with type-safe parameters works correctly
- ShutdownSignal enables graceful shutdown
- CancellationToken supports request cancellation
- ThreadPool processes requests concurrently
- All transports (Stdio, InMemory) work with new Server
- Code follows Rust best practices and style guidelines
- Documentation includes clear examples
- **Base Criteria:**
    - `rust-lint` passes
    - `cargo clippy -- -D warnings` passes
    - `cargo build` succeeds
    - `cargo test` passes

## Implementation Notes

- ThreadPool uses `mpsc::channel` for job queue
- Worker threads loop: `receiver.recv()` -> execute job -> repeat
- Graceful shutdown: Drop sender to close queue, then join all threads
- HandlerFn trait used internally for type erasure
- Server validates thread_pool_size > 0 in `with_thread_pool_size()`
- ShutdownSignal is optional - if not set, `run()` uses EOF to stop
- `run()` checks shutdown signal in message receive loop
- ShutdownSignal::check_shutdown() returns Err(Error) for easy propagation
- CancellationToken::check_cancelled() returns Err(Error::Cancelled)
- Both ShutdownSignal and CancellationToken are Clone + Send + Sync
- Fluent API: All builder methods return `mut self` for chaining
