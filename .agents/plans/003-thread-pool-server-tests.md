---
title: "Thread Pool Server Refactoring and Testing"
seq: 003
slug: "threadpool-server-tests"
created: "2026-02-17T10:41:46Z"
status: archived
---

# Thread Pool Server Refactoring and Testing

This task adds support for cancellable functions to the Server API, updates
graceful shutdown documentation, creates a comprehensive threadpool server
example, and adds integration tests to validate concurrent request handling,
graceful shutdown, and request cancellation functionality.

## Current Problems

The current Server API only supports normal function handlers. Users cannot
register cancellable handlers that accept a CancellationToken. This prevents
testing and demonstrating request cancellation functionality. Additionally, the
graceful shutdown documentation lacks clarity on expected behavior.

Current register API:

```rust
pub fn register<F, P, R>(&mut self, method: &str, handler: F) -> Result<(), Error>
where
    F: Fn(P) -> Result<R, Error> + Send + Sync + 'static,
    P: serde::de::DeserializeOwned + Send + Sync + 'static,
    R: Serialize + Send + Sync + 'static,
```

No integration tests exist for thread pool concurrent behavior, graceful
shutdown, or request cancellation. Additionally, there is no mechanism for
handlers to receive cancellation tokens.

## Proposed Solution

1. Create a `Context` struct that contains a `CancellationToken`
2. Update `register()` method to pass `Context` to all handlers
3. Update `src/shutdown.rs` with comprehensive documentation explaining graceful
   shutdown behavior
4. Create `examples/threadpool_server.rs` demonstrating thread pool usage,
   context-based handlers, and graceful shutdown
5. Create `tests/threadpool_server.rs` with deterministic tests for concurrent
   requests, graceful shutdown, and cancellable requests

## Analysis Required

### Dependency Investigation

- [ ] Review `std::sync::mpsc` channel behavior during shutdown to ensure
      in-progress requests complete
- [ ] Verify thread pool drop behavior and worker thread joining
- [ ] Confirm Arc<Mutex<HashMap>> is safe for concurrent handler access

### Code Locations to Check

- `src/server.rs` - Handler registration and request processing logic
- `src/shutdown.rs` - Shutdown signal implementation and behavior
- `src/cancellation.rs` - CancellationToken integration
- `src/transports/in_memory.rs` - Transport for testing

## Implementation Checklist

### Code Changes

- [ ] Create `Context` struct in `src/server.rs` with `cancellation_token()`
      method
- [ ] Add `cancel_request()` method to `Context` struct
- [ ] Update `register()` method signature to accept handlers with `Context`
      parameter
- [ ] Update handler storage to accommodate context-based handlers
- [ ] Add request tracking infrastructure to `Server` (HashMap: request_id ->
      CancellationToken)
- [ ] Add `cancel_request()` method to `Server` struct
- [ ] Modify `process_request_with_batch()` to create and pass a new `Context`
      when calling handlers
- [ ] Update request processing to track pending requests by request_id
- [ ] Update `src/shutdown.rs` with comprehensive documentation
- [ ] Add graceful shutdown behavior documentation to `src/shutdown.rs`

### Documentation Updates

- [ ] Add doc comments for `Context` struct explaining its purpose and API
- [ ] Add doc comments for `Context::cancel_request()` method
- [ ] Add doc comments for `Server::cancel_request()` method
- [ ] Update doc comments for `register()` method to explain Context parameter
- [ ] Add examples showing how to use Context for cancellation
- [ ] Add section on "Built-in vs Application-level Cancellation" to docs
- [ ] Update `src/lib.rs` to ensure proper re-exports
- [ ] Add module-level documentation to `src/shutdown.rs`

### Test Updates

- [ ] Create `examples/threadpool_server.rs` with normal and cancellable methods
- [ ] Add example of using `Context::cancel_request()` to cancel other requests
- [ ] Create `tests/threadpool_server.rs` with three test cases
- [ ] Add test for `cancel_request()` functionality
- [ ] Add `common` module usage to `tests/threadpool_server.rs`
- [ ] Implement shared token setup pattern for cancellation testing

## Test Plan

### Verification Tests

- [ ] Test that `register()` works with Context parameter (regression for normal
      handlers that ignore context)
- [ ] Test that `register()` works for cancellable handlers that use
      `ctx.cancellation_token().check_cancelled()`
- [ ] Test concurrent requests use different worker threads (deterministic via
      thread ID tracking)
- [ ] Test graceful shutdown completes all pending requests
- [ ] Test graceful shutdown rejects new requests after shutdown signal
- [ ] Test cancellable requests return `Error::Cancelled` when shared token is
      cancelled (using Arc<CancellationToken> passed through handler closure)
- [ ] Test example server compiles and runs without errors

### Regression Tests

- [ ] Run `cargo test --all` to ensure no regressions in existing tests
- [ ] Verify existing examples (`basic_server`, `echo_server`) still work
- [ ] Confirm `cargo clippy -- -D warnings` passes

## Structure After Changes

### File Structure

```
json-rpc-rs/
├── examples/
│   ├── basic_server.rs
│   ├── echo_server.rs
│   └── threadpool_server.rs  (new)
├── src/
│   ├── cancellation.rs
│   ├── error.rs
│   ├── lib.rs
│   ├── server.rs  (modified)
│   ├── shutdown.rs  (modified)
│   └── types.rs
└── tests/
    ├── basic_server.rs
    ├── common.rs
    ├── echo_server.rs
    └── threadpool_server.rs  (new)
```

### Server API After Changes

```rust
pub struct Context {
    cancellation_token: CancellationToken,
}

impl Context {
    /// Returns the cancellation token for the current request.
    ///
    /// This is a **built-in library primitive** that allows handlers to check
    /// if the current request has been cancelled.
    ///
    /// See "How to Add Cancellable Methods" section for usage examples.
    pub fn cancellation_token(&self) -> &CancellationToken {
        &self.cancellation_token
    }

    /// Cancels a pending request by its request ID.
    ///
    /// This is a **built-in library primitive** that allows handlers to cancel
    /// other pending requests. Delegates to the underlying `Server::cancel_request()`.
    ///
    /// Returns `Ok(true)` if the request was found and cancelled,
    /// `Ok(false)` if the request ID was not found.
    ///
    /// See "How to Cancel Specific Pending Requests" for usage examples.
    pub fn cancel_request(&self, request_id: &str) -> Result<bool, Error>;
}

impl Server {
    /// Registers a JSON-RPC method handler.
    ///
    /// All handlers receive a `Context` parameter that provides:
    /// - Per-request cancellation checking via `cancellation_token()`
    /// - Cross-request cancellation capabilities (via server reference, see below)
    ///
    /// See "Built-in vs Application-level Cancellation" for details on what
    /// the library provides vs what applications implement.
    pub fn register<F, P, R>(&mut self, method: &str, handler: F) -> Result<(), Error>
    where
        F: Fn(Context, P) -> Result<R, Error> + Send + Sync + 'static,
        P: serde::de::DeserializeOwned + Send + Sync + 'static,
        R: Serialize + Send + Sync + 'static,
    }

    /// Cancels a pending request by its request ID.
    ///
    /// This is a **built-in library primitive** that allows handlers to cancel
    /// other pending requests. Returns `Ok(true)` if the request was found and
    /// cancelled, `Ok(false)` if the request ID was not found.
    ///
    /// See "How to Cancel Specific Pending Requests" for usage examples.
    pub fn cancel_request(&self, request_id: &str) -> Result<bool, Error>;
}

// Usage - normal handler ignores context:
server.register("add", |_ctx, params: (i32, i32)| {
    Ok(params.0 + params.1)
})?;

// Usage - cancellable handler checks if itself is cancelled:
server.register("long_task", |ctx, params: u64| {
    for _ in 0..params {
        ctx.cancellation_token().check_cancelled()?;
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    Ok("completed".to_string())
})?;

// Usage - handler cancels other pending requests:
server.register("cancel_other", |ctx, request_id: String| {
    // Cancel the request with the given ID
    let cancelled = ctx.cancel_request(&request_id)?;
    Ok(format!("Cancelled request: {}", cancelled))
})?;
```

## How to Add Cancellable Methods

All handlers in the server receive a `Context` parameter that contains a
`CancellationToken`. To make a method cancellable, the handler simply checks the
token during its execution.

### Basic Cancellable Method

```rust
use json_rpc::{Server, Error};
use std::thread;
use std::time::Duration;

let mut server = Server::new();

// Add a cancellable sleep method
server.register("sleep", |ctx, params: u64| {
    let duration = params;

    // Check for cancellation before starting
    ctx.cancellation_token().check_cancelled()?;

    // Sleep with periodic cancellation checks
    let mut elapsed = 0;
    while elapsed < duration {
        let sleep_time = std::cmp::min(100, duration - elapsed);
        thread::sleep(Duration::from_millis(sleep_time));
        elapsed += sleep_time;

        // Check for cancellation after each sleep
        ctx.cancellation_token().check_cancelled()?;
    }

    Ok(format!("Slept for {}ms", duration))
})?;
```

### Cancellable Long-Running Computation

```rust
server.register("fibonacci", |params: u64, ctx| {
    let n = params;

    fn fib(ctx: &json_rpc::Context, n: u64) -> Result<u64, Error> {
        ctx.cancellation_token().check_cancelled()?;
        match n {
            0 => Ok(0),
            1 => Ok(1),
            _ => Ok(fib(n-1, ctx)? + fib(n-2, ctx)?),
        }
    }

    fib(&ctx, n)
})?;
```

### Non-Cancellable Method (Ignores Context)

```rust
server.register("add", |_ctx, params: (i32, i32)| {
    Ok(params.0 + params.1)
})?;
```

## How to Cancel Specific Pending Requests

The `json-rpc-rs` library provides **built-in primitives** for cancelling
pending requests. These primitives can be used directly by handlers to cancel
other pending requests.

### Built-in Primitives

The library provides two main primitives for cancelling requests:

1. **`Context::cancel_request()`** - Cancels a pending request by request ID
   (accessible from handlers)
2. **`Server::cancel_request()`** - Cancels a pending request by request ID
   (accessible outside handlers)

Both methods return `Ok(true)` if the request was found and cancelled,
`Ok(false)` if the request ID was not found, or `Err` if there was an error.

#### Example: Cancelling a Specific Request from Another Handler

```rust
// Register a long-running task that accepts a request_id
server.register("long_task", |ctx, params: (String, u64)| {
    let (request_id, iterations) = params;

    for i in 0..iterations {
        // Check if this request itself is cancelled
        ctx.cancellation_token().check_cancelled()?;
        thread::sleep(Duration::from_millis(10));
    }

    Ok(format!("Task {} completed", request_id))
})?;

// Register a cancel method that cancels other requests
server.register("cancel_request", |ctx, request_id: String| {
    // Built-in primitive: cancel the pending request by ID
    match ctx.cancel_request(&request_id) {
        Ok(true) => Ok(format!("Cancelled request: {}", request_id)),
        Ok(false) => Ok(format!("Request not found: {}", request_id)),
        Err(e) => Err(e),
    }
})?;
```

#### Example: Cancelling a Request from Outside the Server

```rust
// Start the server in a thread
let server_handle = thread::spawn(move || {
    server.run();
});

// From another thread, cancel a specific request
let request_id = "task-123";
if let Ok(true) = server.cancel_request(&request_id) {
    println!("Successfully cancelled request: {}", request_id);
}
```

### Application-Level Patterns

While the library provides built-in primitives for cancellation, applications
may need to implement additional patterns on top of these primitives:

#### Pattern 1: Request ID Generation and Passing

The library doesn't generate request IDs - applications must decide how to
identify and pass request IDs:

```rust
// Application generates unique IDs
use std::sync::atomic::{AtomicU64, Ordering};

static REQUEST_COUNTER: AtomicU64 = AtomicU64::new(0);

// Clients include request IDs in their method calls
client.call::<(String, u64), String>("long_task", ("req-1", 1000))?;
client.call::<String, String>("cancel_request", "req-1".to_string())?;
```

#### Pattern 2: Group-Based Cancellation

For cancelling multiple related requests, applications can track groups:

```rust
// Application maintains request group tracking
use std::collections::HashSet;
use std::sync::{Arc, Mutex};

let group_tracker: Arc<Mutex<HashSet<String>>> = Arc::new(Mutex::new(HashSet::new()));

// When starting a request, add it to a group
let group_tracker_clone = group_tracker.clone();
server.register("start_task", move |ctx, (group_name, request_id): (String, String)| {
    let mut tracker = group_tracker_clone.lock().unwrap();
    tracker.insert(request_id.clone());
    // ... perform task ...
    Ok(())
});

// Cancel all tasks in a group
let group_tracker_clone = group_tracker.clone();
server.register("cancel_group", move |ctx, group_name: String| {
    let tracker = group_tracker_clone.lock().unwrap();
    let mut cancelled = 0;
    for request_id in tracker.iter() {
        if ctx.cancel_request(request_id)? {
            cancelled += 1;
        }
    }
    Ok(format!("Cancelled {} requests", cancelled))
})?;
```

### Built-in vs Application-Level Cancellation

To avoid confusion, here's what the library provides vs what applications
implement:

| Feature                                | Provided by Library | Implemented by Application |
| -------------------------------------- | ------------------- | -------------------------- |
| Per-request cancellation token (CHECK) | ✓                   | -                          |
| Cancel pending requests by ID          | ✓                   | -                          |
| Request ID generation                  | -                   | ✓                          |
| Request ID passing between handlers    | -                   | ✓                          |
| Group-based cancellation patterns      | -                   | ✓                          |
| Request tracking for UI display        | -                   | ✓                          |
| Cancellation history/logging           | -                   | ✓                          |

**Key Principle**: The library provides the **primitives** (Context and Server
methods) to enable cancellation. Applications build **patterns** and
**workflows** on top of these primitives based on their specific needs.

### Request Cancellation Best Practices

1. **Check Frequently**: Call `check_cancelled()` at regular intervals during
   long-running operations
2. **Handle Gracefully**: When cancelled, return `Error::Cancelled` or a custom
   error to inform the client
3. **Clean Up Resources**: Ensure cancellation doesn't leave resources in an
   inconsistent state
4. **Use Request IDs Consistently**: Establish a clear convention for how
   request IDs are generated, passed, and used
5. **Handle Not-Found Cases**: Always handle the case where `cancel_request`
   returns `Ok(false)` - the request may have already completed
6. **Consider Race Conditions**: A request may complete between checking if it
   exists and attempting to cancel it

## Design Considerations

1. **Context-Based Handler Signature**: All handlers receive a `Context`
   parameter containing a `CancellationToken`. Handlers can choose to use it for
   cancellation or ignore it. This provides a simple, uniform API.
    - Benefit: No need for separate register methods or trait-based dispatch
    - Benefit: Backward compatibility can be maintained with adapter closures
    - Alternative: Separate `register()` and `register_cancellable()` methods.
      This is less ergonomic and requires users to choose the correct method.

2. **CancellationToken Management**: The server creates a new
   `CancellationToken` for each request and passes it in the `Context`. For
   testing and demonstration, use a shared `Arc<CancellationToken>` that can be
   cancelled from outside the handler.
    - For production use, applications may implement their own token management
      strategy (e.g., per-request tokens via application-specific mechanisms)
    - Alternative: Pass tokens via JSON-RPC request metadata. This is complex
      and non-standard.

3. **Handler Storage Strategy**: Store handlers in a single HashMap. All
   handlers have the same signature `(P, Context) -> Result<R, Error>`, so no
   special wrapping is needed.

4. **Shutdown Verification**: The thread pool's `Drop` implementation
   automatically joins all worker threads when dropped. The server's run loop
   checks the shutdown signal before receiving new messages. When shutdown is
   signaled, the loop exits, ThreadPool drops, and workers join.
    - Alternative: Add explicit pending request tracking. This is unnecessary
      since the thread pool handles this.

5. **Test Determinism**: Use thread ID tracking via
   `std::thread::current().id()` to verify concurrent execution. This is more
   reliable than timing-based assertions.
    - Alternative: Use timing with sleep methods and response sequence. This can
      be flaky due to system load.

6. **In-Memory Transport for Testing**: Use `InMemory::unconnected()` for
   integration tests. This provides better control than Stdio for testing
   concurrent requests and shutdown behavior.

7. **Cross-Request Cancellation as Built-in Feature**: The library provides
   built-in primitives for cancelling specific pending requests via
   `Server::cancel_request()` and `Context::cancel_request()`. This eliminates
   the need for applications to implement their own request tracking mechanisms.
    - Benefit: Consistent, well-documented API for cross-request cancellation
    - Benefit: Reduces boilerplate code in applications
    - Benefit: Thread-safe implementation handled by the library
    - Application-level patterns (request ID generation, group management) are
      still needed but use the built-in primitives
    - Alternative: Require applications to implement all cancellation tracking.
      This is more flexible but increases complexity and potential for errors.

## Success Criteria

- `Context` struct compiles and provides access to `CancellationToken`
- `Context::cancel_request()` compiles and can cancel pending requests by ID
- `Server::cancel_request()` compiles and can cancel pending requests by ID
- Updated `register()` method compiles and accepts handlers with `Context`
  parameter
- Handlers can access cancellation token via `ctx.cancellation_token()`
- Handlers can cancel other pending requests via `ctx.cancel_request()`
- Integration test for concurrent requests passes (verifies different threads
  process requests)
- Integration test for graceful shutdown passes (completes pending requests)
- Integration test for cancellable requests passes (returns `Error::Cancelled`
  when shared token is cancelled)
- Integration test for cross-request cancellation passes (one request cancels
  another pending request)
- Example `threadpool_server.rs` demonstrates all features and runs without
  errors
- All existing tests continue to pass
- **Base Criteria:**
    - `rust-lint` passes
    - `cargo clippy -- -D warnings` passes
    - `cargo build` succeeds
    - `cargo test` passes

## Implementation Notes

The implementation introduces a `Context` struct containing a
`CancellationToken` and updates the `register()` method to pass this context to
all handlers. This provides a simple, uniform API where handlers can choose to
use the cancellation token or ignore it.

The context-based approach works as follows:

1. Create a `Context` struct that wraps a `CancellationToken`
2. Update `register()` method signature to accept handlers with a
   `(Context, P) -> Result<R, Error>` signature
3. When calling handlers in `process_request_with_batch()`, create a new
   `CancellationToken` for each request and wrap it in a `Context`
4. Handlers access the token via `ctx.cancellation_token()` and can call
   `check_cancelled()` to detect cancellation

**Note**: The Context's per-request token is useful for application-level
cancellation logic. However, since the server creates a new token for each
request, external code cannot directly cancel it. For testing purposes, use a
shared `Arc<CancellationToken>` captured in the handler's closure environment.

**Cancellation Testing**: For integration tests, create a shared
`Arc<CancellationToken>` before starting the server, and store it in a location
accessible to the handler (e.g., using a static `OnceLock` or passing it through
the handler's closure environment). To cancel, call `shared_token.cancel()` from
the test thread.

**Cross-Request Cancellation**: The implementation also adds support for
cancelling specific pending requests:

1. Add a `HashMap<String, Arc<CancellationToken>>` to `Server` to track pending
   requests by their request ID
2. When processing a request, if a request_id is provided, store the
   cancellation token in the tracking map
3. Implement `Server::cancel_request(request_id)` that:
    - Looks up the request in the tracking map
    - Calls `cancel()` on the token if found
    - Removes the entry from the map
    - Returns `Ok(true)` if cancelled, `Ok(false)` if not found
4. Implement `Context::cancel_request(request_id)` that delegates to the server
   (requires Context to hold a weak reference to Server or similar mechanism)

This provides built-in primitives for cross-request cancellation, eliminating
the need for applications to implement their own request tracking mechanisms.

**Request ID Handling**: The library tracks requests by request_id but doesn't
generate them. Applications are responsible for:

- Generating unique request IDs
- Passing request IDs in method parameters
- Maintaining any application-level request metadata

The library simply provides the infrastructure to cancel requests when given
their ID.

**Backward Compatibility**: To maintain backward compatibility, existing
handlers can be wrapped with a closure that accepts `Context` but ignores it:

```rust
// Old style:
server.register("old_method", |params: (i32, i32)| {
    Ok(params.0 + params.1)
})?;

// Can be updated to:
server.register("old_method", |_ctx, params: (i32, i32)| {
    Ok(params.0 + params.1)
})?;
```

For testing, use InMemory transport with the `unconnected()` method to get a
transport and sender. This allows controlling when messages are sent and when
the transport is closed. Track thread IDs in a shared
`Arc<Mutex<HashSet<ThreadId>>>` to verify concurrent execution.

**Cancellation Testing**: For integration tests, create a shared
`Arc<CancellationToken>` before registering the handler and capture it in the
handler's closure. The handler checks this shared token (not the one from
Context) to demonstrate cancellation functionality. To cancel, call
`shared_token.cancel()` from the test thread.

This pattern demonstrates that cancellation works when triggered. The Context
parameter is ignored in this example, but handlers can use
`ctx.cancellation_token()` if they need per-request cancellation in addition to
the shared token.

Graceful shutdown works because:

1. The run loop checks `shutdown_signal.is_shutdown_requested()` before
   receiving new messages
2. When shutdown is signaled, the loop exits
3. The `ThreadPool` drops, triggering its `Drop` implementation
4. `ThreadPool::drop()` drops the sender, workers receive `None` and exit
5. All worker threads are joined before the server returns

This ensures all in-progress requests complete before the server exits.
