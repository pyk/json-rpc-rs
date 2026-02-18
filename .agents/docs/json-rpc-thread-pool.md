# Thread Pool Usage Analysis for json-rpc-rs

## Executive Summary

This document analyzes whether using thread pools is a good architectural
decision for implementing the `json-rpc-rs` library, based on the facts gathered
from thread-pool-notes.md.

**Bottom Line**: Thread pools should **not** be used as the primary execution
mechanism due to critical limitations with request cancellation. Instead, use
async I/O as the foundation with an optional thread pool for CPU-bound
operations.

---

## 🚨 Critical Issue: Request Cancellation

### The Conflict

**Requirement**: Support for Request Cancellation - Users can cancel pending
requests from other methods.

**Thread Pool Limitation** (from thread-pool-notes.md):

```rust
// Graceful Shutdown section (wadingpool):
// "Cannot interrupt running tasks - shutdown may take time"
```

```markdown
// Potential Improvements section (wadingpool): // - Task cancellation support
[listed as missing feature]
```

### The Problem

Thread pools **cannot** easily interrupt tasks that are already executing. To
support request cancellation with thread pools, you would need:

1. **Cooperative cancellation**: Each task must explicitly check for
   cancellation signals
2. **Complex design**: Every method handler needs to be cancellation-aware
3. **Error-prone**: It's easy to forget to check cancellation, leading to zombie
   tasks
4. **No guarantees**: Long-running operations without cancellation points cannot
   be interrupted

This fundamentally conflicts with the requirement for reliable request
cancellation.

---

## ✅ Where Thread Pools Could Help

Based on thread-pool-notes.md, thread pools would be useful in specific
scenarios:

### 1. Batch Requests (CPU-bound methods)

```rust
// If batch methods involve heavy computation:
let batch_results: Vec<Value> = batch_requests
    .par_iter()  // Using Rayon for parallel processing
    .map(|req| handle_request(req))
    .collect();
```

### 2. Heavy JSON Processing

- Large payload validation
- Complex schema validation
- Deep object manipulation

### 3. Background Work

- Long-running computations triggered by methods
- Data processing tasks
- Analytics or aggregation

### 4. CPU-Intensive Methods

From the notes on thread pool design for I/O workloads:

> "Thread pools should be used for CPU-bound work (e.g., DB calls, heavy
> computation), not I/O waiting."

---

## ❌ Where Thread Pools Are Not Ideal

### I/O Workloads (Network, stdin/stdout, Files, etc.)

From the I/O workloads section of thread-pool-notes.md:

> "The whole point of asynchronous I/O is that many things can be done on one
> thread, because most of your operations are wait operations."

This applies to ALL I/O workloads, including:

- ✅ Network sockets (TCP/UDP)
- ✅ stdin/stdout (file descriptors)
- ✅ File I/O
- ✅ Pipes
- ✅ Any blocking I/O operation

**Why stdin/stdout are the same as network I/O:**

| Aspect                     | stdin/stdout | Network Sockets | Same? |
| -------------------------- | ------------ | --------------- | ----- |
| Wait for data availability | ✅ Yes       | ✅ Yes          | ✅    |
| Wait for write buffer      | ✅ Yes       | ✅ Yes          | ✅    |
| Can be non-blocking        | ✅ Yes       | ✅ Yes          | ✅    |
| Event loop compatible      | ✅ Yes       | ✅ Yes          | ✅    |
| Thread pool wastes threads | ✅ Yes       | ✅ Yes          | ✅    |
| Async I/O efficient        | ✅ Yes       | ✅ Yes          | ✅    |

**Why async I/O is better for JSON-RPC:**

| Operation                 | Thread Pool                 | Async I/O                         |
| ------------------------- | --------------------------- | --------------------------------- |
| Waiting for connections   | Wastes threads              | Efficient single-threaded         |
| Reading/Writing data      | Wastes threads              | Efficient event loop              |
| stdin/stdout I/O          | Wastes threads              | Efficient event loop              |
| Handling multiple clients | Expensive context switching | Scalable thousands of connections |
| I/O-bound methods         | Inefficient                 | Native support                    |

**JSON-RPC over stdin/stdout example:**

```rust
// ❌ Bad: Using thread pool for stdin/stdout
let pool = ThreadPool::new(4);
pool.spawn(move || {
    loop {
        // This thread is BLOCKED waiting for stdin
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).unwrap();

        // Process JSON-RPC request
        let response = handle_request(&input);

        // Thread is BLOCKED waiting for stdout write
        println!("{}", response);
    }
});

// ✅ Good: Using async I/O for stdin/stdout
async fn run_jsonrpc_server() {
    let mut stdin = tokio::io::stdin();
    let mut stdout = tokio::io::stdout();

    loop {
        // Efficiently waits for stdin data
        let mut input = String::new();
        stdin.read_line(&mut input).await?;

        // Process request (can spawn CPU-bound work to thread pool if needed)
        let response = handle_request(input).await?;

        // Efficiently writes to stdout
        stdout.write_all(response.as_bytes()).await?;
    }
}
```

### Platform Considerations

From thread-pool-notes.md:

> "Don't test on macOS with MIO if targeting Linux. Linux uses epoll, macOS uses
> kqueue."

Async I/O runtimes (tokio, async-std) handle these platform differences
transparently.

### Graceful Shutdown Complexity

Thread pools require explicit shutdown logic:

```rust
impl Drop for ThreadPool {
    fn drop(&mut self) {
        let mut sender = self.sender.take();
        drop(sender);  // Signal workers to stop

        while let Some(thread) = self.threads.pop() {
            thread.join().unwrap();  // Wait for completion
        }
    }
}
```

Async runtimes have native graceful shutdown support.

---

## 📋 Recommended Architecture

Based on the thread-pool-notes.md recommendations for I/O workloads:

```
┌──────────────────────────────────────────────────────┐
│          Async I/O Runtime (Tokio/async-std)         │
├──────────────────────────────────────────────────────┤
│                                                      │
│  ┌──────────────────────────────────────────────┐    │
│  │  Transport Layer (Multiple Transports)       │    │
│  │  - TCP Socket                                │    │
│  │  - WebSocket                                 │    │
│  │  - HTTP/HTTPS                                │    │
│  │                                              │    │
│  │  Listener Thread:                            │    │
│  │    - Accept new connections                  │    │
│  │    - Pass to event loop                      │    │
│  └───────────────┬──────────────────────────────┘    │
│                  │                                   │
│  ┌───────────────▼───────────────────────────────┐   │
│  │  Event Loop Thread                            │   │
│  │    - Wait for I/O events                      │   │
│  │    - Read/Write data                          │   │
│  │    - Handle disconnects                       │   │
│  │    - Re-arm sockets                           │   │
│  └───────────────┬───────────────────────────────┘   │
│                  │                                   │
│  ┌───────────────▼───────────────────────────────┐   │
│  │  JSON-RPC Protocol Layer                      │   │
│  │    - Parse requests                           │   │
│  │    - Validate format                          │   │
│  │    - Handle batch requests                    │   │
│  └───────────────┬───────────────────────────────┘   │
│                  │                                   │
│  ┌───────────────▼───────────────────────────────┐   │
│  │  Method Registry & Router                     │   │
│  │    - Route to handler                         │   │
│  │    - Collect results                          │   │
│  │    - Format responses                         │   │
│  └───────────────┬───────────────────────────────┘   │
│                  │                                   │
│  ┌───────────────▼───────────────────────────────┐   │
│  │  Method Execution                             │   │
│  │                                               │   │
│  │  ┌─────────────────────────────────────────┐  │   │
│  │  │  Fast Methods (async/await)             │  │   │
│  │  │    - I/O-bound operations               │  │   │
│  │  │    - Quick computations                 │  │   │
│  │  │    - Default execution path             │  │   │
│  │  └─────────────────────────────────────────┘  │   │
│  │                                               │   │
│  │  ┌─────────────────────────────────────────┐  │   │
│  │  │  Optional Thread Pool                   │  │   │
│  │  │    - CPU-bound methods only             │  │   │
│  │  │    - #[cpu_intensive] decorator         │  │   │
│  │  │    - Batch request processing           │  │   │
│  │  │    - User-opt-in                        │  │   │
│  │  └─────────────────────────────────────────┘  │   │
│  │                                               │   │
│  └───────────────────────────────────────────────┘   │
│                                                      │
└──────────────────────────────────────────────────────┘
```

### Default Execution Path

```rust
// Most methods use async/await
async fn handle_request(req: Request) -> Result<Response, Error> {
    // Fast, efficient, supports cancellation
    let result = method_handler(req).await?;
    Ok(Response::success(result))
}
```

### Thread Pool Execution Path (Optional)

```rust
#[cpu_intensive]  // Opt-in decorator
async fn heavy_computation(req: Request) -> Result<Response, Error> {
    // Runs on dedicated thread pool
    let result = thread_pool.spawn(move || {
        // CPU-bound work that doesn't support cancellation mid-execution
        perform_heavy_computation(req)
    }).await?;
    Ok(Response::success(result))
}
```

---

## 🎯 Specific Recommendations

### 1. Use Async I/O as the Foundation

**Rationale**:

- Native cancellation support (CancellationToken, abort on drop)
- Efficient for network workloads
- Built-in graceful shutdown
- User-friendly API with async/await

**Implementation**: Use Tokio (industry standard) or async-std

### 2. Make Thread Pool Optional and Opt-In

**Rationale**:

- Avoids complexity for simple use cases
- Provides control when needed
- Matches the thread-pool-notes.md recommendation:
    > "Thread pools should be used for CPU-bound work... not I/O waiting"

**Implementation Options**:

- Decorator: `#[cpu_intensive]`
- Trait method: `fn execution_strategy(&self) -> ExecutionStrategy`
- Configuration: Register method with execution mode

### 3. Implement Request Cancellation with Async

```rust
use tokio_util::sync::CancellationToken;

struct RequestContext {
    cancel_token: CancellationToken,
}

async fn handle_with_cancellation(
    ctx: RequestContext,
    request: Request
) -> Result<Response, Error> {
    tokio::select! {
        result = process_request(request) => result,
        _ = ctx.cancel_token.cancelled() => {
            Err(Error::Cancelled)
        }
    }
}
```

### 4. Graceful Shutdown Strategy

```rust
struct JsonRpcServer {
    shutdown_signal: ShutdownSignal,
    // ... other fields
}

impl JsonRpcServer {
    async fn shutdown(self) {
        // 1. Stop accepting new connections
        // 2. Cancel pending requests
        // 3. Wait for active handlers to complete
        // 4. Close transports
    }
}
```

### 5. Batch Request Handling

```rust
async fn handle_batch(
    requests: Vec<Request>
) -> Vec<Response> {
    // Parallel execution using async, not threads
    let mut handles = Vec::new();
    for req in requests {
        let handle = tokio::spawn(handle_request(req));
        handles.push(handle);
    }

    // Wait for all and collect results
    let results: Vec<_> = futures::future::join_all(handles)
        .await
        .into_iter()
        .filter_map(|r| r.ok())
        .collect();

    results
}
```

For CPU-intensive batch operations:

```rust
#[cpu_intensive]
async fn handle_batch_heavy(
    requests: Vec<Request>
) -> Vec<Response> {
    thread_pool.spawn(move || {
        requests
            .into_par_iter()  // Rayon parallel iterator
            .map(|req| handle_heavy_request_sync(req))
            .collect()
    }).await.unwrap()
}
```

---

## 📊 Requirements vs Implementation Comparison

| Requirement              | Thread Pool                                          | Async I/O                             | Recommended Approach      |
| ------------------------ | ---------------------------------------------------- | ------------------------------------- | ------------------------- |
| **User Friendly**        | ❌ Adds complexity, difficult API                    | ✅ Simple async/await                 | **Async I/O**             |
| **Easy Methods/Errors**  | ✅ Sync methods work                                 | ✅ Async methods work                 | **Both work**             |
| **Graceful Shutdown**    | ⚠️ Possible, complex to implement                    | ✅ Native support                     | **Async I/O**             |
| **Multiple Transports**  | ❌ Poor fit for network I/O                          | ✅ Excellent fit, event loop model    | **Async I/O**             |
| **Request Cancellation** | ❌ Very difficult, requires cooperative cancellation | ✅ Native support (CancellationToken) | **Async I/O**             |
| **Batch Requests**       | ✅ Good for CPU-bound batches                        | ⚠️ Depends on method type             | **Async + optional pool** |

---

## 🎬 Conclusion

### Summary

**Do not use thread pools as the primary execution mechanism for json-rpc-rs.**

The request cancellation requirement is a fundamental conflict with thread pool
architecture. Basic thread pools cannot interrupt running tasks, making reliable
cancellation nearly impossible without complex cooperative cancellation patterns
throughout the entire codebase.

### Recommended Approach

1. **Primary**: Build on async I/O (Tokio recommended)
    - Native cancellation support
    - Efficient for network workloads
    - User-friendly async/await API
    - Graceful shutdown built-in

2. **Secondary**: Provide optional thread pool for CPU-intensive methods
    - Opt-in via decorator or registration
    - Use Rayon or similar for parallel batch processing
    - Keep it simple: users who need it will use it

3. **Architecture**: Match the I/O workloads recommendation from
   thread-pool-notes.md
    > "Default setup should have 2 threads: one listener, one for I/O" "Thread
    > pool for CPU-bound work, not I/O waiting"

### Benefits of This Approach

✅ **Meets all requirements**:

- User friendly: Simple async/await API
- Easy methods: Intuitive error handling with `Result`
- Graceful shutdown: Native support
- Multiple transports: Event loop handles all equally well
- Request cancellation: Built-in cancellation tokens
- Batch requests: Parallel execution when beneficial

✅ **Performance**:

- Efficient for I/O workloads (most JSON-RPC usage)
- Optional parallelism for CPU-bound work
- Low overhead for simple use cases

✅ **Flexibility**:

- Users can opt into thread pool only when needed
- No forced complexity for simple implementations
- Future-proof for both async and sync method handlers

### Final Recommendation

```rust
// User-friendly API by default
#[rpc_method]
async fn get_data(id: String) -> Result<Data, Error> {
    // Fast, async, cancellable
    fetch_data(id).await
}

// Optional CPU-intensive decorator
#[rpc_method(cpu_intensive)]
async fn process_batch(ids: Vec<String>) -> Result<Vec<Data>, Error> {
    // Runs on thread pool, not cancellable mid-execution
    heavy_computation(ids)
}
```

This gives you the best of both worlds: simplicity by default, power when
needed.

---

## References

- thread-pool-notes.md: Comprehensive thread pool implementation notes
    - Basic thread pool implementation (wadingpool)
    - Multiple thread pools with Rayon
    - Rayon ThreadPool API reference
    - Thread pool design for I/O workloads
    - Rust User Forum discussion on ThreadPool performance
