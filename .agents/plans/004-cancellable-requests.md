---
title: "Cancellable Requests"
seq: 004
slug: "cancellable-requests"
created: "2026-02-19T00:00:00Z"
status: not-started
---

# Cancellable Requests

Implement comprehensive support for cancellable requests in the json-rpc-rs
library. This task provides built-in primitives for checking if a request is
cancelled and for cancelling other pending requests. The implementation includes
an example demonstrating cancellation patterns and integration tests verifying
the functionality.

## Current Problems

The json-rpc-rs library currently lacks support for request cancellation. This
limitation prevents:

1. **Per-request cancellation**: Handlers cannot check if they should stop
   processing due to external cancellation signals
2. **Cross-request cancellation**: Handlers cannot cancel other pending requests
3. **Graceful termination**: Long-running operations cannot be interrupted
   cleanly
4. **Resource management**: Applications cannot release resources held by
   cancelled requests
5. **User experience**: Clients cannot cancel long-running operations, leading
   to poor UX

The JSON-RPC 2.0 specification does not define a cancellation mechanism, but the
library requires cancellation support as one of its expected requirements:

- Support for Request Cancellation
- Support for Graceful Shutdown
- Support for Concurrent Request via Thread Pool

Without cancellation support, the library cannot effectively handle:

- Long-running computations that need to be interrupted
- Batch operations where individual requests should be cancellable
- Server shutdown scenarios where pending requests need cleanup
- Resource-intensive operations that should be abortable

## Proposed Solution

1. **Create `Context` struct** with simple cancellation primitives:
    - `is_cancelled(id: &str) -> bool` - check if the given id is cancelled
    - `cancel(id: &str) -> bool` - mark the given id as cancelled, returns true
      if it wasn't already cancelled
    - These methods operate on arbitrary string identifiers chosen by the
      application
    - No request tracking by the library - applications manage identifiers

2. **Update handler registration**:
    - Modify `register()` to pass `Context` to all handlers
    - Handlers receive `(Context, Params) -> Result<R, Error>` signature
    - Handlers can choose to use Context or ignore it

3. **Create `examples/cancellable_request.rs`**:
    - Demonstrate simple cancellation with session_id example
    - Demonstrate checking and cancelling arbitrary identifiers
    - Show how multiple handlers can share identifiers
    - Demonstrate non-cancellable method (simple addition)
    - Show client usage example demonstrating the flow

4. **Create `tests/cancellable_request.rs`**:
    - Test basic cancellation (cancel id, then check it)
    - Test cross-request cancellation (one handler cancels another's id)
    - Test cancellation of non-existent ids (returns false for cancel, false for
      is_cancelled)
    - Test multiple concurrent ids
    - Verify cancellation is thread-safe

## Analysis Required

### Dependency Investigation

- [ ] Check if `Arc` and `HashMap` are available in `std` or need external
      crates
- [ ] Verify thread-safety requirements for cancellation storage

### Code Locations to Check

- `src/server.rs` - Understand current handler registration and request
  processing
- `src/lib.rs` - Check what's currently exported
- `examples/threadpool_server.rs` - Reference for implementing
  cancellable_request.rs

### Code Locations to Modify

- `src/server.rs` - Add `Context` struct with is_cancelled/cancel methods
- `src/lib.rs` - Export `Context`

## Implementation Checklist

### Code Changes

- [ ] Create `Context` struct in `src/server.rs`:
    - Add `cancelled_ids: Arc<Mutex<HashSet<String>>>` field for tracking
      cancelled identifiers
    - Implement `new()` constructor
    - Implement `is_cancelled(id: &str) -> bool` method
    - Implement `cancel(id: &str) -> bool` method
    - Implement `remove_cancel(id: &str) -> bool` method

- [ ] Update `Server::register()` method signature:
    - Change handler signature to `Fn(Context, P) -> Result<R, Error>`
    - Pass `Context` when calling handlers in request processing

### Documentation Updates

- [ ] Add comprehensive doc comments for `Context` struct:
    - Explain purpose: provides cancellation primitives to handlers
    - Mark methods as **built-in library primitives**
    - Include examples for per-request and cross-request cancellation

- [ ] Add doc comments for `Context::cancel()`:
    - Mark as **built-in library primitive**
    - Explain that it takes an arbitrary string identifier
    - Document return values (true if id was not previously cancelled)
    - Include usage examples

- [ ] Add doc comments for `Context::is_cancelled()`:
    - Mark as **built-in library primitive**
    - Explain that it takes an arbitrary string identifier
    - Document return values (true if id is cancelled)
    - Include usage examples

- [ ] Add doc comments for `Context::remove_cancel()`:
    - Mark as **built-in library primitive**
    - Explain that it takes an arbitrary string identifier
    - Document return values (true if id was previously cancelled)
    - Include usage examples for session-based workflows

- [ ] Update doc comments for `Server::register()`:
    - Explain all handlers now receive `Context` parameter
    - Note that handlers can ignore Context if not needed
    - Reference "How to Use Cancellable Requests" section

- [ ] Update `src/lib.rs` re-exports:
    - Export `Context` struct

### Example Code

- [ ] Create `examples/cancellable_request.rs`:
    - Import necessary types (`Server`, `Context`, `Error`, `Stdio`)
    - Create `Server` instance
    - Implement
      `session_prompt(_ctx: Context, session_id: String, prompt: String)`
      method:
        - Loop processing prompt
        - Check `ctx.is_cancelled(&session_id)` periodically
        - Return "prompt completed" if not cancelled
    - Implement `session_load(ctx: Context, session_id: String)` method:
        - Call `ctx.remove_cancel(&session_id)` to reset cancelled state
        - Return success message
    - Implement `session_cancel(ctx: Context, session_id: String)` method:
        - Call `ctx.cancel(&session_id)`
        - Return success/failure message
    - Implement `add(_ctx: Context, a: i32, b: i32)` method:
        - Simple addition, ignores Context (non-cancellable example)
    - Show client usage example demonstrating the flow:
        ```json
        // Start session
        {"jsonrpc":"2.0","method":"session_load","params":["session-123"],"id":1}
        // Start prompt for session
        {"jsonrpc":"2.0","method":"session_prompt","params":["session-123","explain Rust"],"id":2}
        // Cancel by session_id
        {"jsonrpc":"2.0","method":"session_cancel","params":["session-123"],"id":3}
        // Reload session (removes cancelled state)
        {"jsonrpc":"2.0","method":"session_load","params":["session-123"],"id":4}
        ```
    - Run server with `Stdio::new()` and add comments explaining the simple API

### Test Updates

- [ ] Create `tests/cancellable_request.rs`:
    - Use `InMemory::unconnected()` transport for test control
    - Create `common` module for test helpers

- [ ] Add test: `test_basic_cancellation`:
    - Start server with methods that check and cancel
    - Call `ctx.cancel("test-id")` from one handler
    - Verify `ctx.is_cancelled("test-id")` returns true
    - Verify `ctx.is_cancelled("other-id")` returns false

- [ ] Add test: `test_cross_request_cancellation`:
    - Start server with session_prompt and session_cancel methods
    - Spawn session_prompt request with session_id = "test-session"
    - Call session_cancel method with session_id = "test-session"
    - Verify session_prompt returns `Error::Cancelled`
    - Verify session_cancel reports success

- [ ] Add test: `test_cancel_nonexistent_id`:
    - Start server with cancel method
    - Call cancel method with non-existent id
    - Verify `ctx.cancel("nonexistent")` returns false
    - Verify `ctx.is_cancelled("nonexistent")` returns false

- [ ] Add test: `test_concurrent_cancellation`:
    - Start server with methods that check multiple ids
    - Spawn multiple requests checking different ids
    - Cancel one of the ids
    - Verify:
        - Requests checking cancelled id see cancellation
        - Other requests complete normally
        - Cancellation is isolated per id

- [ ] Add test: `test_non_cancellable_method`:
    - Start server with add method (ignores Context)
    - Call add with valid parameters
    - Verify request completes successfully

## Test Plan

### Verification Tests

- [ ] Verify `Context::cancel()` marks an id as cancelled and returns true
- [ ] Verify `Context::cancel()` returns false for already-cancelled ids
- [ ] Verify `Context::is_cancelled()` returns true for cancelled ids
- [ ] Verify `Context::is_cancelled()` returns false for non-cancelled ids
- [ ] Verify handler receives `Context` parameter
- [ ] Verify handler can ignore `Context` parameter
- [ ] Verify cancellation returns `Error::Cancelled` when detected
- [ ] Verify cross-request cancellation works from handler
- [ ] Verify concurrent requests are isolated (cancelling one doesn't affect
      others)
- [ ] Verify thread-safety of cancellation state under concurrent access
- [ ] Verify cancellation works with different transports (InMemory, Stdio)

### Regression Tests

- [ ] Run all existing tests to ensure no regressions
- [ ] Verify `examples/threadpool_server.rs` still runs
- [ ] Verify existing handlers work with Context parameter
- [ ] Verify backward compatibility with non-cancellable handlers

## Structure After Changes

### File Structure

```
json-rpc-rs/
├── examples/
│   ├── echo_server.rs
│   ├── basic_server.rs
│   └── cancellable_request.rs (new)
├── tests/
│   ├── echo_server.rs
│   ├── basic_server.rs
│   └── cancellable_request.rs (new)
├── src/
│   ├── cancellation.rs (new or existing)
│   ├── error.rs
│   ├── lib.rs
│   ├── server.rs (modified - Context, cancel_request)
│   ├── shutdown.rs
│   ├── types.rs
│   └── transports/
└── .agents/
    ├── docs/
    └── plans/
        ├── 001-replace-trait-with-builder-pattern.md
        ├── 002-error-handling-tests.md
        ├── 003-threadpool-server-tests.md
        └── 004-cancellable-requests.md (new)
```

### Server API After Changes

````rust
// Context struct - provides simple cancellation primitives
pub struct Context {
    cancelled_ids: Arc<Mutex<HashSet<String>>>,
}

impl Context {
    /// Check if the given id is cancelled.
    ///
    /// This allows handlers to check if a specific identifier has been marked
    /// as cancelled. Applications choose what ids to use (e.g., session_id,
    /// task_id, job_id, etc.). These are arbitrary strings chosen by the
    /// application and are not related to JSON-RPC request IDs.
    ///
    /// # Examples
    ///
    /// ```rust
    /// server.register("process", |ctx, task_id: String| {
    ///     for i in 0..100 {
    ///         if ctx.is_cancelled(&task_id) {
    ///             return Err(Error::Cancelled);
    ///         }
    ///         // ... do work ...
    ///     }
    ///     Ok("completed")
    /// })?;
    /// ```
    pub fn is_cancelled(&self, id: &str) -> bool;

    /// Mark the given id as cancelled.
    ///
    /// This allows handlers to cancel operations identified by the given id.
    /// The id is an arbitrary string chosen by the application. Returns true
    /// if the id was not already cancelled (i.e., this call changed its state),
    /// false if it was already cancelled.
    ///
    /// # Examples
    ///
    /// ```rust
    /// server.register("cancel_task", |ctx, task_id: String| {
    ///     if ctx.cancel(&task_id) {
    ///         Ok(format!("Task {} cancelled", task_id))
    ///     } else {
    ///         Ok(format!("Task {} was already cancelled", task_id))
    ///     }
    /// })?;
    /// ```
    pub fn cancel(&self, id: &str) -> bool;

    /// Remove the cancelled state for the given id.
    ///
    /// This removes the id from the cancelled set, allowing operations with
    /// this id to continue. Returns true if the id was previously cancelled
    /// (i.e., this call changed its state), false if it was not cancelled.
    ///
    /// This is particularly useful for session-based workflows where:
    /// - A session is loaded with an existing session_id
    /// - The session may have been cancelled previously
    /// - Loading a session should start fresh (not cancelled)
    ///
    /// # Examples
    ///
    /// ```rust
    /// server.register("session/load", |ctx, session_id: String| {
    ///     // Remove cancelled state when loading a session
    ///     ctx.remove_cancel(&session_id);
    ///     Ok(format!("Session {} loaded", session_id))
    /// })?;
    ///
    /// server.register("session/cancel", |ctx, session_id: String| {
    ///     if ctx.cancel(&session_id) {
    ///         Ok(format!("Session {} cancelled", session_id))
    ///     } else {
    ///         Ok(format!("Session {} was already cancelled", session_id))
    ///     }
    /// })?;
    /// ```
    pub fn remove_cancel(&self, id: &str) -> bool;
}

impl Server {
    /// Registers a JSON-RPC method handler.
    ///
    /// All handlers receive a `Context` parameter that provides:
    /// - `is_cancelled(id)` - check if an id is cancelled
    /// - `cancel(id)` - mark an id as cancelled
    /// - `remove_cancel(id)` - remove cancelled state for an id
    ///
    /// These methods operate on arbitrary string identifiers chosen by the
    /// application, not JSON-RPC request IDs.
    ///
    /// See `examples/cancellable_request.rs` for usage examples.
    pub fn register<F, P, R>(&mut self, method: &str, handler: F) -> Result<(), Error>
    where
        F: Fn(Context, P) -> Result<R, Error> + Send + Sync + 'static,
        P: serde::de::DeserializeOwned + Send + Sync + 'static,
        R: Serialize + Send + Sync + 'static,
    {
        // Handler storage logic
    }
}
````

## How to Use Cancellable Requests

The library provides three simple methods on `Context`:

1. `is_cancelled(id: &str) -> bool` - Check if an identifier is cancelled
2. `cancel(id: &str) -> bool` - Mark an identifier as cancelled
3. `remove_cancel(id: &str) -> bool` - Remove the cancelled state for an
   identifier

These operate on **arbitrary string identifiers** chosen by your application.
They are NOT related to JSON-RPC request IDs. This gives you full flexibility to
design your own cancellation patterns.

### Choosing Identifiers

Your application chooses what identifiers to use. Common patterns:

| Pattern         | Example Identifiers       | Use Case                   |
| --------------- | ------------------------- | -------------------------- |
| Session-based   | `"session-123"`, `"abc"`  | User sessions, chat, etc.  |
| Task-based      | `"task-456"`, `"job-789"` | Background jobs, workflows |
| Operation-based | `"upload-file"`, `"sync"` | Named operations           |
| Composite       | `"user:123:task:456"`     | Multiple contexts combined |

The key is that **both** the operation and its cancellation use the **same**
identifier.

### Built-in vs Application-Level Cancellation

| Feature                         | Provided by Library | Implemented by Application |
| ------------------------------- | ------------------- | -------------------------- |
| Check if id is cancelled        | ✓ (`is_cancelled`)  | -                          |
| Mark id as cancelled            | ✓ (`cancel`)        | -                          |
| Thread-safe cancellation state  | ✓                   | -                          |
| Identifier naming scheme        | -                   | ✓                          |
| Cancellation patterns/workflows | -                   | ✓                          |
| Group cancellation              | -                   | ✓                          |
| Request metadata tracking       | -                   | ✓                          |

**Key Principle**: The library provides simple primitives. Your application
builds whatever cancellation patterns you need on top of them.

## Design Considerations

1. **Simple API Surface**: The cancellation mechanism is extremely simple with
   just two methods: `is_cancelled(id)` and `cancel(id)`. Both take arbitrary
   string identifiers.
    - Benefit: Easy to understand and use
    - Benefit: No complexity about JSON-RPC request IDs
    - Benefit: Applications have complete flexibility
    - Alternative: Track requests by JSON-RPC id with complex mapping. This adds
      unnecessary complexity.

2. **Identifier Choice**: Applications choose what identifiers to use for
   cancellation. These are arbitrary strings and have no relationship to
   JSON-RPC request IDs.
    - Benefit: Full flexibility for application patterns
    - Benefit: No confusion with JSON-RPC spec
    - Benefit: Works with any naming scheme (session_id, task_id, etc.)
    - Alternative: Force use of JSON-RPC request IDs. This limits flexibility
      and doesn't match real-world use cases.

3. **Thread-Safe State Storage**: The `Context` maintains a thread-safe
   `HashSet<String>` of cancelled identifiers using `Arc<Mutex<HashSet<...>>>`.
    - Benefit: Simple and efficient for cancellation checks
    - Benefit: Thread-safe for concurrent requests
    - Benefit: No need for complex request tracking
    - Alternative: Use per-request tokens. This requires more infrastructure and
      adds complexity.

4. **Cross-Request Cancellation**: Any handler can cancel operations in other
   handlers by using the same identifier. No special infrastructure needed.
    - Benefit: Simple pattern for common use cases
    - Benefit: Applications design their own cancellation patterns
    - Benefit: No need for request tracking or mappings
    - Alternative: Track all pending requests. This is unnecessary complexity
      when identifiers are shared naturally.

5. **Graceful Degradation**: Calling `cancel(id)` on an already-cancelled id
   returns `false` (no-op). Calling `is_cancelled(id)` on a non-existent id
   returns `false`.
    - Benefit: Handles race conditions gracefully
    - Benefit: No errors for normal operations
    - Benefit: Simple boolean semantics
    - Alternative: Return errors for edge cases. This complicates error
      handling.

6. **Context Sharing**: All handlers receive the same `Context` with access to
   the same cancellation state.
    - Benefit: Cross-request cancellation just works
    - Benefit: No need for passing state between handlers
    - Benefit: Natural sharing of cancellation identifiers
    - Alternative: Per-request contexts with shared state. More complex.

7. **No Request Tracking**: The library does not track requests. Applications
   manage their own identifiers and patterns.
    - Benefit: Simpler implementation
    - Benefit: No confusion about what "request_id" means
    - Benefit: Applications have full control
    - Alternative: Track by JSON-RPC request id. Adds complexity without value.

8. **Backward Compatibility**: Handlers can ignore `Context` if they don't need
   cancellation.
    - Benefit: Existing code continues to work
    - Benefit: Gradual adoption possible
    - Benefit: Simple parameter name prefix convention (`_ctx`)
    - Alternative: Require all handlers to use Context. This breaks existing
      code.

## Success Criteria

- `Context` struct compiles with `is_cancelled()` and `cancel()` methods
- `is_cancelled(id)` returns correct boolean for cancelled/non-cancelled ids
- `cancel(id)` returns true on first call, false on subsequent calls
- Updated `register()` method compiles and accepts handlers with `Context`
  parameter
- Handlers can check cancellation status via `ctx.is_cancelled()`
- Handlers can cancel operations via `ctx.cancel()`
- Cancellation state is shared across all handlers
- Integration test for basic cancellation passes (cancel, then check)
- Integration test for cross-request cancellation passes (one handler cancels
  another's id)
- Integration test for cancelling non-existent id passes (is_cancelled returns
  false)
- Integration test for concurrent cancellation passes (isolated per id)
- Integration test for non-cancellable methods passes (ignores Context)
- Integration test for thread safety passes (no data races)
- Example `examples/cancellable_request.rs` demonstrates all features and runs
  without errors
- All existing tests continue to pass
- **Base Criteria:**
    - `rust-lint` passes
    - `cargo clippy -- -D warnings` passes
    - `cargo build` succeeds
    - `cargo test` passes

## Implementation Notes

### Implementation Overview

The `Context` struct is implemented with a single field
`cancelled_ids: Arc<Mutex<HashSet<String>>>`. This simple design provides:

- Thread-safe access to cancellation state across all concurrent handlers
- Arbitrary string identifiers chosen by applications (not related to JSON-RPC
  request IDs)
- Simple boolean semantics (true/false) for all operations
- No request tracking infrastructure needed

For complete API documentation with usage examples, see the "Server API After
Changes" section above.

### Handler Signature Transition

Old handler signature:

```rust
Fn(P) -> Result<R, Error>
```

New handler signature:

```rust
Fn(Context, P) -> Result<R, Error>
```

Backward compatibility can be maintained by wrapping old handlers:

```rust
let old_handler = |params: (i32, i32)| Ok(params.0 + params.1);
server.register("add", |_ctx, params| old_handler(params))?;
```

The `Context` parameter is always passed, but handlers can ignore it if they
don't need cancellation (using `_ctx` prefix).

### Thread Safety Considerations

The cancellation state must be thread-safe since multiple requests can be
processed concurrently:

- `cancelled_ids: Arc<Mutex<HashSet<String>>>`
- Benefit: Safe concurrent access without data races
- Benefit: Simple implementation with standard library types
- Alternative: Use RwLock instead of Mutex for better read performance. Not
  necessary given:
    - `is_cancelled()` reads are infrequent (typically called in loops)
    - `cancel()` writes are infrequent (only when cancelling)
    - Mutex is simpler and sufficient

**No lock ordering issues**: Since there's only one Mutex, there are no deadlock
risks from lock ordering.

### Error Handling

The API uses simple boolean returns instead of Results:

- `is_cancelled(id) -> bool`:
    - `true`: The id is cancelled
    - `false`: The id is not cancelled
    - No errors possible

- `cancel(id) -> bool`:
    - `true`: The id was not previously cancelled (this call changed state)
    - `false`: The id was already cancelled (no-op)
    - No errors possible (mutex poisoning is a panic, not an error)

**Why simple booleans instead of Results:**

- Simpler API surface
- No need for error handling in handlers
- Natural semantics (true/false is intuitive)
- Mutex poisoning is a serious bug, not a recoverable error

### Race Conditions

Potential race conditions and how they're handled:

1. **Cancel after handler completes**: If a handler completes (success or error)
   before another handler calls `cancel(id)`, the completed handler will never
   check `is_cancelled(id)` again.
    - **Application handling**: This is not a problem - the operation is already
      done, so cancellation has no effect. This is the expected behavior.

2. **Concurrent cancel calls**: Multiple calls to `cancel(id)` on the same id
   are safe due to Mutex protection. Only the first call returns `true`,
   subsequent calls return `false`.
    - **Application handling**: Check the return value to know if this was the
      first cancellation request.

3. **Check after cancel**: A handler calling `is_cancelled(id)` immediately
   after another handler calls `cancel(id)` will see the cancelled state.
    - **Application handling**: Handlers check periodically and stop processing
      when cancelled.

4. **Concurrent checks**: Multiple handlers can call `is_cancelled(id)` on the
   same id simultaneously. This is safe due to Mutex protection.
    - **Application handling**: No special handling needed - just check the
      result.

5. **Id isolation**: Different ids are completely isolated. Cancelling one id
   does not affect others.
    - **Application handling**: Use unique ids for different operations to avoid
      unintended cancellation.

### Memory Management

**No reference cycles**: Context doesn't hold references to Server, so there are
no circular references.

**Shared state**: All handlers receive the same Context with the same
`cancelled_ids` HashSet. This is intentional and enables cross-request
cancellation.

**Optional cleanup**: Cancelled ids can be removed from the HashSet by calling
`Context::remove_cancel(id)`. This allows applications to remove the cancelled
state for specific identifiers when needed (e.g., when reloading a session).

Applications are responsible for calling `remove_cancel()` at appropriate times:

- When loading a session that was previously cancelled
- When restarting an operation with the same identifier
- When clearing stale cancelled entries

If `remove_cancel()` is not called, cancelled ids remain in the HashSet
indefinitely. This is acceptable for many use cases because:

- The number of unique ids is typically bounded by application logic
- HashSet lookup is O(1) regardless of size
- Applications can reset the entire state by creating a new Server

### Testing Strategy

Test coverage should include:

1. **Basic operations**:
    - `cancel(id)` returns `true` on first call
    - `cancel(id)` returns `false` on second call
    - `is_cancelled(id)` returns `false` initially
    - `is_cancelled(id)` returns `true` after `cancel(id)`

2. **Cross-request cancellation**:
    - Handler A checks `is_cancelled("test-id")`
    - Handler B calls `cancel("test-id")`
    - Handler A sees cancellation and stops

3. **Concurrent access**:
    - Multiple threads calling `cancel()` on same id
    - Multiple threads calling `is_cancelled()` on same id
    - No data races or panics

4. **Id isolation**:
    - Cancel "id-1" doesn't affect "id-2"
    - Handlers checking different ids see correct state

5. **Non-cancellable methods**:
    - Handler ignoring Context works correctly
    - No impact on cancellation state

6. **Thread safety**:
    - Use `std::thread` or similar to spawn concurrent requests
    - Verify no data races with tools like `loom` if available

Use `InMemory::unconnected()` transport for integration tests:

- Better control than Stdio for testing concurrent behavior
- Can simulate multiple clients
- Easier to verify responses

Create a `common` module in tests for shared helpers:

- Request sending helper
- Response parsing helper
- Server setup helper

## Known Limitations

1. **Application-Managed Cleanup**: Cancelled identifiers remain in the HashSet
   until explicitly removed via `Context::remove_cancel(id)`.
    - Applications must call `remove_cancel()` when reloading identifiers (e.g.,
      sessions)
    - Applications can reset the entire state by creating a new Server if needed
    - The library does not automatically expire cancelled entries
    - This design maximizes flexibility while keeping the API simple

2. **Application-Managed Identifiers**: Applications are responsible for
   choosing and managing cancellation identifiers. The library doesn't enforce
   any naming scheme or structure.
    - This is intentional to maintain maximum flexibility
    - Applications can use UUIDs, timestamps, composite keys (e.g.,
      "user:123:task:456"), or any scheme they need
    - Applications must ensure ids are unique for different operations to avoid
      unintended cancellation
    - See "How to Use Cancellable Requests" section for examples

3. **Cancellation Latency**: There may be a delay between calling `cancel(id)`
   and the handler actually stopping, depending on how frequently the handler
   checks `is_cancelled(id)`.
    - Handlers must call `is_cancelled()` frequently in loops or long-running
      operations
    - Long-running operations without periodic checks won't stop immediately
    - Applications should determine appropriate check frequency based on their
      requirements (responsiveness vs. performance)

4. **Resource Cleanup**: The library provides the cancellation mechanism but
   doesn't enforce or assist with proper cleanup of resources (file handles,
   network connections, database transactions, etc.).
    - Handlers are responsible for cleaning up resources when cancelled
    - Use `Drop` trait or explicit cleanup patterns as needed
    - Consider using RAII patterns for automatic cleanup

5. **No Group Cancellation**: There's no built-in "cancel all operations with
   this prefix" or "cancel entire group" operation.
    - Applications can implement group patterns by using related ids (e.g.,
      "session-123:operation-A", "session-123:operation-B") and tracking them
    - Applications can maintain their own group management if needed
    - This keeps the library API simple while enabling complex patterns

6. **No Dependency Tracking**: Cancelling one identifier doesn't automatically
   cancel dependent operations or child operations.
    - Applications must implement their own dependency tracking if needed
    - Example: If operation A spawns operations B and C, cancelling A doesn't
      cancel B and C automatically
    - Applications can implement this by using shared identifiers or maintaining
      their own dependency graph

7. **No Cancellation History**: The library doesn't track when ids were
   cancelled or provide any history or logging of cancellation events.
    - Applications must implement their own logging if needed
    - No audit trail of cancellation operations
    - This keeps the library simple and focused on the core cancellation
      mechanism
