# JSON-RPC Builder Pattern API Design

## Table of Contents

1. [Overview](#overview)
2. [API Reference](#api-reference)
3. [Method Naming](#method-naming)
4. [Graceful Shutdown](#graceful-shutdown)
5. [Concurrent Request Handling](#concurrent-request-handling)
6. [Transport Switching](#transport-switching)
7. [Complete Examples](#complete-examples)
8. [Implementation Details](#implementation-details)
9. [Trade-offs and Limitations](#trade-offs-and-limitations)

---

## Overview

The Builder Pattern API provides a simple, ergonomic interface for creating
JSON-RPC servers with minimal boilerplate. It is designed for:

- Quick prototyping
- Simple servers with few methods
- Beginners learning JSON-RPC
- Use cases where routing logic is straightforward

### Key Features

- **Zero boilerplate**: Register methods with a single function call
- **Type-safe parameters**: Automatic deserialization with structs
- **Automatic error handling**: Proper JSON-RPC error codes generated
  automatically
- **Flexible method naming**: Supports slashes, dots, and any valid string
- **Graceful shutdown**: Built-in support for clean server shutdown
- **Backward compatible**: Can be used alongside the Router trait API

---

## API Reference

### Core Types

#### `Server`

The main server type that manages method registration and request handling.

````rust
pub struct Server {
    handlers: HashMap<String, Box<dyn HandlerFn>>,
    thread_pool_size: usize,
}

impl Server {
    /// Create a new Server instance with default configuration.
    ///
    /// The default thread pool size is set to the number of CPU cores.
    pub fn new() -> Self;

    /// Set the maximum number of threads in the thread pool.
    ///
    /// This method configures the size of the thread pool used for concurrent
    /// request processing. The thread pool is created when the server starts
    /// running and is dropped when the server shuts down.
    ///
    /// # Arguments
    /// - `size`: Number of worker threads in the pool (must be > 0)
    ///
    /// # Panics
    ///
    /// This method will panic if `size` is zero.
    ///
    /// # Example
    /// ```rust
    /// let mut server = Server::new()
    ///     .with_thread_pool_size(4);
    /// server.register("echo", handler)?;
    /// server.run_until_shutdown(shutdown)?;
    /// ```
    ///
    /// # Thread Pool Behavior
    ///
    /// - **Fixed size**: The thread pool creates exactly `size` worker threads
    /// - **Concurrent requests**: Up to `size` requests can be processed simultaneously
    /// - **Queueing**: Additional requests wait in a queue until a worker is available
    /// - **Graceful shutdown**: Workers finish current jobs before exiting on shutdown
    /// - **Cancellation**: Cancel requests can interrupt long-running operations in parallel
    pub fn with_thread_pool_size(mut self, size: usize) -> Self;

    /// Register a method handler.
    ///
    /// # Type Parameters
    /// - `P`: Parameter type (must implement `DeserializeOwned`)
    /// - `R`: Return type (must implement `Serialize`)
    ///
    /// # Arguments
    /// - `name`: Method name (can contain slashes, e.g., "session/new")
    /// - `handler`: Handler function that takes parameters and returns a result
    ///
    /// # Example
    /// ```rust
    /// server.register("echo", |params: EchoParams| {
    ///     Ok(params.message)
    /// })?;
    /// ```
    pub fn register<F, P, R>(&mut self, name: &str, handler: F) -> Result<(), Error>
    where
        F: Fn(P) -> Result<R, Error> + Send + Sync + 'static,
        P: serde::de::DeserializeOwned + 'static,
        R: serde::Serialize + 'static;

    /// Run the server with default stdio transport.
    ///
    /// This method blocks until EOF is received. Requests are processed
    /// concurrently using a thread pool with the configured size (default: CPU cores),
    /// allowing cancel requests to interrupt long-running operations.
    ///
    /// # Example
    /// ```rust
    /// let mut server = Server::new();
    /// server.register("echo", handler)?;
    /// server.run()?;  // Blocks until EOF
    /// ```
    pub fn run(&mut self) -> Result<(), Error>;

    /// Run the server with a custom transport.
    ///
    /// # Arguments
    /// - `transport`: Any type implementing the Transport trait
    ///
    /// # Example
    /// ```rust
    /// let mut server = Server::new();
    /// server.register("echo", handler)?;
    /// server.run_with_transport(Stdio)?;
    /// ```
    pub fn run_with_transport<T>(&mut self, transport: T) -> Result<(), Error>
    where
        T: Transport;

    /// Run the server until a shutdown signal is received.
    ///
    /// This method creates a thread pool with the configured size and processes
    /// requests concurrently. When shutdown is signaled:
    ///
    /// 1. The job queue is closed by dropping the sender
    /// 2. Worker threads finish their current jobs
    /// 3. All worker threads are joined
    /// 4. The method returns
    ///
    /// # Arguments
    /// - `shutdown`: Shutdown signal that signals when to stop
    ///
    /// # Example
    /// ```rust
    /// use json_rpc::ShutdownSignal;
    ///
    /// let shutdown = ShutdownSignal::new();
    /// let mut server = Server::new();
    /// server.register("echo", handler)?;
    /// server.run_until_shutdown(shutdown)?;
    /// ```
    pub fn run_until_shutdown(&mut self, shutdown: ShutdownSignal) -> Result<(), Error>;
}
````

#### `ShutdownSignal`

Shutdown signal for graceful server shutdown.

Can be shared across threads and cloned as needed. Used to signal the server to
stop accepting new requests and gracefully terminate.

```rust
#[derive(Clone)]
pub struct ShutdownSignal {
    inner: Arc<AtomicBool>,
}

impl ShutdownSignal {
    /// Create a new shutdown signal.
    pub fn new() -> Self;

    /// Check if shutdown was requested.
    ///
    /// Returns error if shutdown was requested.
    pub fn check_shutdown(&self) -> Result<(), Error>;

    /// Check if shutdown was requested (non-panicking).
    pub fn is_shutdown_requested(&self) -> bool;

    /// Signal the server to shut down.
    ///
    /// This method is thread-safe and can be called from any thread,
    /// including signal handlers.
    pub fn signal(&self);
}
```

#### `CancellationToken`

Cancellation token for cooperative cancellation of long-running operations.

Used within method handlers to check if an operation should be cancelled (e.g.,
via a separate cancellation request).

```rust
#[derive(Clone)]
pub struct CancellationToken {
    inner: Arc<AtomicBool>,
}

impl CancellationToken {
    /// Create a new cancellation token.
    pub fn new() -> Self;

    /// Check if cancellation was requested.
    ///
    /// Returns `Err(Error::Cancelled)` if cancellation was requested.
    pub fn check_cancelled(&self) -> Result<(), Error>;

    /// Check if cancellation was requested (non-panicking).
    ///
    /// Returns true if cancellation was requested.
    pub fn is_cancelled(&self) -> bool;

    /// Request cancellation.
    ///
    /// This method is thread-safe and can be called from any thread.
    pub fn cancel(&self);
}
```

#### `HandlerFn`

Internal trait for method handlers. Users don't interact with this directly.

```rust
trait HandlerFn: Send + Sync {
    fn call(&self, params: serde_json::Value) -> Result<serde_json::Value, Error>;
}
```

---

## Method Naming

### Supported Characters

The builder pattern supports any valid JSON-RPC method name as a string. This
includes:

- **Letters and numbers**: `echo`, `method1`, `getUser`
- **Underscores**: `echo_message`, `get_user_id`
- **Slashes**: `session/new`, `api/v1/users`, `path/to/method`
- **Dots**: `session.new`, `api.version1.users`
- **Hyphens**: `echo-message`, `get-user-id`
- **Colons**: `echo:message`, `api:users:get`
- **Any valid UTF-8 string**

### JSON-RPC 2.0 Restrictions

According to the JSON-RPC 2.0 specification:

> "Method names that begin with the word rpc followed by a period character
> (U+002E or ASCII 46) are reserved for rpc-internal methods and extensions and
> MUST NOT be used for anything else."

**Examples of reserved names:**

- `rpc.method`
- `rpc.getProtocol`
- `rpc.internalMethod`

**Examples of allowed names:**

- `echo`
- `session/new`
- `api.users.get`
- `method.name.with.dots`

### Slash-based Method Names

**Question**: Does the builder pattern support slash-based method names like
"session/new"?

**Answer**: **Yes**, fully supported.

The builder pattern uses a `HashMap<String, Box<dyn HandlerFn>>` to store
handlers, where the key is the method name as a string. This means "session/new"
is treated exactly the same as "session_new" or any other string.

#### Example: Slash-based Method Names

```rust
use json_rpc::Server;

#[derive(serde::Deserialize)]
struct NewSessionParams {
    user_id: String,
    #[serde(default)]
    timeout: u64,
}

#[derive(serde::Serialize)]
struct NewSessionResult {
    session_id: String,
    expires_at: u64,
}

fn main() -> anyhow::Result<()> {
    let mut server = Server::new();

    // Register a method with a slash
    server.register("session/new", |params: NewSessionParams| {
        let session_id = format!("sess_{}", uuid::Uuid::new_v4());
        let expires_at = chrono::Utc::now().timestamp() + params.timeout as i64;

        Ok(NewSessionResult {
            session_id,
            expires_at: expires_at as u64,
        })
    })?;

    // Register another method with multiple slashes
    server.register("api/v1/users/get", |params: GetUserParams| {
        // Handler implementation
        Ok(User { name: "Alice".to_string() })
    })?;

    // Register a method that looks like a path
    server.register("path/to/resource/action", |params: ActionParams| {
        Ok("Action completed".to_string())
    })?;

    server.run()?;
    Ok(())
}
```

#### Client Usage with Slash-based Methods

Clients can call these methods exactly as they're registered:

```bash
# Call session/new
echo '{"jsonrpc":"2.0","method":"session/new","params":{"user_id":"user123","timeout":3600},"id":1}' | cargo run

# Call api/v1/users/get
echo '{"jsonrpc":"2.0","method":"api/v1/users/get","params":{"id":"123"},"id":2}' | cargo run

# Call path/to/resource/action
echo '{"jsonrpc":"2.0","method":"path/to/resource/action","params":{"action":"delete"},"id":3}' | cargo run
```

#### Method Naming Best Practices

While the API supports any string, consider these guidelines:

1. **Use slashes for hierarchical organization**:
    - `session/new`, `session/delete`, `session/get`
    - `api/v1/users`, `api/v1/posts`, `api/v1/comments`

2. **Use consistent conventions**:
    - Pick one style and stick with it
    - Don't mix `snake_case` and `kebab-case` in the same server

3. **Avoid reserved prefixes**:
    - Don't start methods with `rpc.`

4. **Keep names descriptive**:
    - ✓ `session/new` - clear and concise
    - ✓ `users/getProfile` - easy to understand
    - ✗ `s/n` - too cryptic
    - ✗ `method123` - not descriptive

5. **Consider versioning**:
    - `api/v1/users` - allows for `api/v2/users` later
    - `users_v1` - alternative approach

---

## Graceful Shutdown

### Overview

**Question**: Does the builder pattern support graceful shutdown?

**Answer**: **Yes**, with built-in support for clean server shutdown.

### What is Graceful Shutdown?

Graceful shutdown means:

1. **Stop accepting new requests**
2. **Complete in-flight requests** (or timeout after a period)
3. **Close connections cleanly**
4. **Release resources**
5. **Exit with a clean state**

This is important for:

- Preventing data corruption
- Ensuring clients receive responses
- Clean process termination
- Proper signal handling (SIGTERM, SIGINT)

### ShutdownSignal API

```rust
use json_rpc::ShutdownSignal;

impl ShutdownSignal {
    /// Create a new shutdown signal.
    pub fn new() -> Self;

    /// Check if shutdown was requested.
    pub fn check_shutdown(&self) -> Result<(), Error>;

    /// Check if shutdown was requested (non-panicking).
    pub fn is_shutdown_requested(&self) -> bool;

    /// Signal the server to shut down.
    ///
    /// This method is thread-safe and can be called from any thread,
    /// including signal handlers.
    pub fn signal(&self);
}

impl Server {
    /// Run the server until a shutdown signal is received.
    pub fn run_until_shutdown(&mut self, shutdown: ShutdownSignal) -> Result<(), Error>;
}
```

### Graceful Shutdown Examples

#### Example 1: Signal Handler (Ctrl+C)

```rust
use json_rpc::{Server, ShutdownSignal};
use anyhow::Result;

#[derive(serde::Deserialize)]
struct EchoParams {
    message: String,
}

fn main() -> Result<()> {
    let mut server = Server::new();
    let shutdown = ShutdownSignal::new();

    // Register methods
    server.register("echo", |params: EchoParams| {
        Ok(params.message)
    })?;

    // Set up Ctrl+C handler for graceful shutdown
    let shutdown_clone = shutdown.clone();
    ctrlc::set_handler(move || {
        println!("\nReceived shutdown signal...");
        shutdown_clone.signal();
        println!("Shutdown signal sent.");
    }).expect("Error setting Ctrl-C handler");

    println!("Server running. Press Ctrl+C to shut down gracefully.");

    // Run until shutdown
    server.run_until_shutdown(shutdown)?;

    println!("Server shut down gracefully.");
    Ok(())
}
```

#### Example 2: Timeout-based Shutdown

```rust
use json_rpc::{Server, ShutdownSignal};
use std::thread;
use std::time::Duration;

fn main() -> anyhow::Result<()> {
    let mut server = Server::new();

    // Register echo method
    server.register("echo", |params: EchoParams| {
        Ok(params.message)
    })?;

    // Get shutdown signal
    let shutdown = ShutdownSignal::new();
    let shutdown_clone = shutdown.clone();

    // Spawn a thread to shutdown after 10 seconds
    thread::spawn(move || {
        thread::sleep(Duration::from_secs(10));
        println!("Shutting down after timeout...");
        shutdown_clone.signal();
    });

    println!("Server running. Will auto-shutdown after 10 seconds.");

    // Run until shutdown
    server.run_until_shutdown(shutdown)?;

    println!("Server shut down gracefully.");
    Ok(())
}
```

#### Example 3: Manual Shutdown from Another Thread

```rust
use json_rpc::{Server, ShutdownSignal};
use std::sync::mpsc;
use std::thread;

fn main() -> anyhow::Result<()> {
    let mut server = Server::new();

    server.register("echo", |params: EchoParams| {
        Ok(params.message)
    })?;

    // Channel to send shutdown command
    let (cmd_tx, cmd_rx) = mpsc::channel();

    // Get shutdown signal
    let shutdown = ShutdownSignal::new();
    let shutdown_clone = shutdown.clone();

    // Spawn a thread to listen for shutdown commands
    thread::spawn(move || {
        match cmd_rx.recv() {
            Ok("shutdown") => {
                println!("Shutdown command received.");
                shutdown_clone.signal();
            }
            Ok(cmd) => {
                println!("Unknown command: {}", cmd);
            }
            Err(_) => {
                println!("Command channel closed.");
            }
        }
    });

    println!("Server running. Send 'shutdown' command to stop.");
    println!("You can also press Ctrl+C.");

    // Run until shutdown
    server.run_until_shutdown(shutdown)?;

    println!("Server shut down gracefully.");
    Ok(())
}
```

### Shutdown Flow

```
1. shutdown.signal() is called
   ↓
2. Server receives shutdown signal
   ↓
3. Server stops accepting new requests
   ↓
4. Server waits for in-flight requests (with timeout)
   ↓
5. Server closes transport connection
   ↓
6. Server returns from run_until_shutdown()
```

---

## Concurrent Request Handling

### Overview

**Question**: What happens when there are 2 concurrent requests, for example
`session/prompt` (long running) and `session/cancel` requests that are expected
to cancel the `session/prompt` request?

**Answer**: By default, requests are processed **concurrently** using a thread
pool. This allows cancellation tokens to work properly - cancel requests can be
processed in parallel with long-running operations.

### Current Behavior

By default, the server processes requests **concurrently** using a thread pool:

```
1. Request 1 (session/prompt) arrives → Starts processing in thread 1
2. Request 2 (session/cancel) arrives → Starts processing in thread 2
3. Request 2 calls cancel() → Request 1 detects cancellation and stops
4. Request 1 returns cancellation error → Request 2 completes successfully
```

**Key Feature**: Requests are processed in parallel, allowing cancel requests to
interrupt long-running operations in real-time.

### Implementation

The `Server::run()` method processes requests concurrently using a thread pool:

1. **Process each request in a separate thread**: When a request arrives, spawn
   it to run independently in the thread pool
2. **Keep thread-safe shared state**: Use `Arc<Mutex<>>` or `Arc<AtomicBool>`
   for the cancellation token map
3. **Allow cancel to run in parallel**: The `session/cancel` request will run in
   its own thread and can signal cancellation while `session/prompt` is still
   executing
4. **Cooperative cancellation still required**: Long-running methods must still
   check the cancellation token periodically

#### Thread Pool Implementation

```rust
use std::sync::{Arc, Mutex};
use json_rpc::Server;

impl Server {
    pub fn run(&mut self) -> Result<(), Error> {
        // Create thread pool with configured size
        let pool = ThreadPool::new(self.thread_pool_size);

        // Clone handlers for use in worker threads
        let handlers = Arc::new(Mutex::new(self.handlers.clone()));

        loop {
            let message = self.transport.receive_message()?;

            // Clone references for the worker thread
            let handlers = Arc::clone(&handlers);
            let mut transport = self.transport.clone();

            // Execute in thread pool (concurrently)
            pool.execute(move || {
                let response = handle_message(message, &handlers);
                transport.send_response(response);
            });
        }
    }
}
```

**Note**: The `run()` method creates a thread pool with the configured size
(default: number of CPU cores) and processes requests concurrently. This enables
real-time cancellation since cancel requests run in separate worker threads.

### Updated Cancellable Method API

With concurrent processing, the cancellation token API remains the same:

````rust
use json_rpc::CancellationToken;

impl Server {
    /// Register a method that supports cancellation.
    ///
    /// The handler receives a CancellationToken that can be checked
    /// periodically to see if cancellation was requested.
    ///
    /// Cancellation works by default since requests are processed concurrently
    /// using a thread pool. Cancel requests can be processed in parallel with
    /// long-running operations.
    ///
    /// # Example
    /// ```rust
    /// server.register_cancellable("session/prompt", |params: PromptParams, cancel| {
    ///     loop {
    ///         cancel.check_cancelled()?;  // Returns error if cancelled
    ///         // Do work...
    ///     }
    /// })?;
    /// ```
    pub fn register_cancellable<F, P, R>(
        &mut self,
        name: &str,
        handler: F,
    ) -> Result<(), Error>
    where
        F: Fn(P, CancellationToken) -> Result<R, Error> + Send + Sync + 'static,
        P: serde::de::DeserializeOwned + 'static,
        R: serde::Serialize + 'static;
}
````

#### Cancellable Method API

````rust
use json_rpc::CancellationToken;

impl Server {
    /// Register a method that supports cancellation.
    ///
    /// The handler receives a CancellationToken that can be checked
    /// periodically to see if cancellation was requested.
    ///
    /// # Example
    /// ```rust
    /// server.register_cancellable("session/prompt", |params: PromptParams, cancel| {
    ///     loop {
    ///         cancel.check_cancelled()?;  // Returns error if cancelled
    ///         // Do work...
    ///     }
    /// })?;
    /// ```
    pub fn register_cancellable<F, P, R>(
        &mut self,
        name: &str,
        handler: F,
    ) -> Result<(), Error>
    where
        F: Fn(P, CancellationToken) -> Result<R, Error> + Send + Sync + 'static,
        P: serde::de::DeserializeOwned + 'static,
        R: serde::Serialize + 'static;
}
````

### Complete Example: Session with Cancellation

```rust
use json_rpc::{Server, CancellationToken, ShutdownSignal};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use anyhow::Result;

// Parameters for long-running session
#[derive(serde::Deserialize, Clone)]
struct StartSessionParams {
    session_id: String,
    duration_secs: u64,
}

// Parameters for cancel request
#[derive(serde::Deserialize)]
struct CancelSessionParams {
    session_id: String,
}

// Session state
#[derive(Clone)]
struct SessionState {
    sessions: Arc<Mutex<HashMap<String, CancellationToken>>>,
}

fn main() -> Result<()> {
    let mut server = Server::new();
    let shutdown = ShutdownSignal::new();
    let session_state = SessionState {
        sessions: Arc::new(Mutex::new(HashMap::new())),
    };

    // Register a long-running method that supports cancellation
    {
        let sessions = Arc::clone(&session_state.sessions);
        server.register_cancellable("session/prompt", move |params: StartSessionParams, cancel| {
            let session_id = params.session_id.clone();

            // Register this session's cancellation token
            {
                let mut sessions = sessions.lock().unwrap();
                sessions.insert(session_id.clone(), cancel.clone());
            }

            println!("Starting session {} (will run for {} seconds)", session_id, params.duration_secs);

            // Simulate long-running work with cancellation checks
            for i in 0..params.duration_secs * 10 {
                cancel.check_cancelled()?;
                thread::sleep(Duration::from_millis(100));
                if i % 10 == 0 {
                    println!("Session {} progress: {}%", session_id, i * 10 / (params.duration_secs * 10));
                }
            }

            println!("Session {} completed successfully", session_id);

            // Clean up
            {
                let mut sessions = sessions.lock().unwrap();
                sessions.remove(&session_id);
            }

            Ok(serde_json::json!({
                "session_id": session_id,
                "status": "completed"
            }))
        })?;
    }

    // Register a cancel method
    {
        let sessions = Arc::clone(&session_state.sessions);
        server.register("session/cancel", move |params: CancelSessionParams| {
            let session_id = params.session_id.clone();

            println!("Cancel request for session {}", session_id);

            // Find and cancel the session
            let cancel_token = {
                let sessions = sessions.lock().unwrap();
                sessions.get(&session_id).cloned()
            };

            match cancel_token {
                Some(cancel) => {
                    cancel.cancel();
                    Ok(serde_json::json!({
                        "session_id": session_id,
                        "status": "cancelling"
                    }))
                }
                None => {
                    anyhow::bail!("Session '{}' not found or already completed", session_id)
                }
            }
        })?;
    }

    // Register a list method
    {
        let sessions = Arc::clone(&session_state.sessions);
        server.register("session/list", move |_params: serde_json::Value| {
            let sessions = sessions.lock().unwrap();
            let session_ids: Vec<String> = sessions.keys().cloned().collect();
            Ok(serde_json::json!({
                "active_sessions": session_ids,
                "count": session_ids.len()
            }))
        })?;
    }

    // Set up Ctrl+C handler for graceful shutdown
    let shutdown_clone = shutdown.clone();
    ctrlc::set_handler(move || {
        println!("\nReceived shutdown signal...");
        shutdown_clone.signal();
        println!("Shutdown signal sent.");
    }).expect("Error setting Ctrl-C handler");

    println!("Session server running.");
    println!("Try these commands:");
    println!("  # Start a long session (30 seconds)");
    println!("  echo '{\"jsonrpc\":\"2.0\",\"method\":\"session/prompt\",\"params\":{\"session_id\":\"sess1\",\"duration_secs\":30},\"id\":1}' | cargo run");
    println!();
    println!("  # Cancel the session (in another terminal)");
    println!("  echo '{\"jsonrpc\":\"2.0\",\"method\":\"session/cancel\",\"params\":{\"session_id\":\"sess1\"},\"id\":2}' | cargo run");
    println!();
    println!("  # List active sessions");
    println!("  echo '{\"jsonrpc\":\"2.0\",\"method\":\"session/list\",\"params\":{},\"id\":3}' | cargo run");

    server.run_until_shutdown(shutdown)?;

    println!("Server shut down gracefully.");
    Ok(())
}
```

### Testing the Cancellation

**Terminal 1 - Start the session:**

```bash
echo '{"jsonrpc":"2.0","method":"session/prompt","params":{"session_id":"sess1","duration_secs":30},"id":1}' | cargo run
```

**Terminal 2 - Cancel the session (before it completes):**

```bash
echo '{"jsonrpc":"2.0","method":"session/cancel","params":{"session_id":"sess1"},"id":2}' | cargo run
```

**Expected Output:**

```
Starting session sess1 (will run for 30 seconds)
Session sess1 progress: 0%
Session sess1 progress: 10%
Cancel request for session sess1
Server returned error: Operation cancelled
```

### Best Practices for Cancellation

1. **Check cancellation frequently**:
    - Check at least every 100ms in long loops
    - Check before expensive operations

2. **Provide useful progress updates**:
    - Include progress in logs or responses
    - Allow clients to query status

3. **Handle cancellation errors gracefully**:

    ```rust
    server.register_cancellable("process", |params: ProcessParams, cancel| {
        match do_long_work(cancel) {
            Ok(result) => Ok(result),
            Err(e) if e.is_cancelled() => {
                // Cancellation is not an error, just return status
                Ok(serde_json::json!({"status": "cancelled"}))
            }
            Err(e) => Err(e),
        }
    })?;
    ```

4. **Clean up resources**:

    ```rust
    // Use Drop or ensure blocks
    let session_id = params.session_id.clone();
    sessions.lock().unwrap().insert(session_id.clone(), cancel.clone());

    let result = do_work(cancel);

    // Always clean up, even on cancellation
    sessions.lock().unwrap().remove(&session_id);
    result
    ```

### Limitations

- **Cancellation is cooperative**: Long-running methods must explicitly check
  for cancellation at regular intervals. Without these checks, cancellation
  won't work even with concurrent processing.
- **No automatic request prioritization**: All requests are treated equally in
  the thread pool.
- **Thread pool overhead**: For very simple operations, the overhead of thread
  scheduling may outweigh the benefits of concurrency.

---

## Transport Switching

### Overview

**Question**: What happens when the user wants to switch transports, for example
using in-memory transport for testing?

**Answer**: The builder pattern fully supports pluggable transports through the
`Transport` trait. You can easily switch between different transport
implementations without changing your business logic.

### Built-in Transports

The library provides several transport implementations:

1. **Stdio** - Default, uses stdin/stdout for NDJSON (newline-delimited JSON)
2. **InMemory** - In-memory transport for testing and in-process communication
3. **Custom** - Implement your own by implementing the `Transport` trait

### Transport API

````rust
impl Server {
    /// Run the server with the default transport (Stdio).
    pub fn run(&mut self) -> Result<(), Error>;

    /// Run the server with a custom transport.
    ///
    /// # Arguments
    /// - `transport`: Any type implementing the Transport trait
    ///
    /// # Example
    /// ```rust
    /// // Use in-memory transport for testing
    /// let transport = InMemory::new();
    /// server.run_with_transport(transport)?;
    /// ```
    pub fn run_with_transport<T>(&mut self, transport: T) -> Result<(), Error>
    where
        T: Transport;
}

/// Transport trait for pluggable I/O backends.
pub trait Transport: Send + Sync {
    /// Receive a JSON-RPC message.
    ///
    /// Blocks until a message is available or an error occurs.
    fn receive_message(&mut self) -> Result<Message, Error>;

    /// Send a JSON-RPC response.
    fn send_response(&mut self, response: &Response) -> Result<(), Error>;

    /// Send a JSON-RPC notification.
    fn send_notification(&mut self, notification: &Notification) -> Result<(), Error>;

    /// Close the transport connection.
    fn close(&mut self) -> Result<(), Error>;
}
````

### Example 1: Switching Between Stdio and InMemory

```rust
use json_rpc::{Server, InMemory, Stdio};
use anyhow::Result;

#[derive(serde::Deserialize)]
struct EchoParams {
    message: String,
}

fn main() -> Result<()> {
    let mut server = Server::new();

    // Register methods (same for all transports)
    server.register("echo", |params: EchoParams| {
        Ok(params.message)
    })?;

    // Choose transport based on environment
    let use_in_memory = std::env::var("TEST_MODE").is_ok();

    if use_in_memory {
        println!("Running with InMemory transport (testing mode)");
        let transport = InMemory::new();
        server.run_with_transport(transport)?;
    } else {
        println!("Running with Stdio transport (production mode)");
        server.run()?;
    }

    Ok(())
}
```

### Example 2: Unit Tests with InMemory Transport

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use json_rpc::{Server, InMemory};
    use serde_json::json;

    fn create_test_server() -> Server {
        let mut server = Server::new();

        server.register("echo", |params: EchoParams| {
            Ok(params.message)
        }).unwrap();

        server.register("add", |params: AddParams| {
            Ok(params.a + params.b)
        }).unwrap();

        server
    }

    #[test]
    fn test_echo() {
        let mut server = create_test_server();
        let transport = InMemory::new();

        // Simulate client request
        let request = json!({
            "jsonrpc": "2.0",
            "method": "echo",
            "params": {"message": "hello"},
            "id": 1
        });

        transport.send_request(request).unwrap();
        server.handle_transport(&mut transport).unwrap();

        let response = transport.receive_response().unwrap();
        assert_eq!(response["result"], "hello");
    }

    #[test]
    fn test_add() {
        let mut server = create_test_server();
        let transport = InMemory::new();

        let request = json!({
            "jsonrpc": "2.0",
            "method": "add",
            "params": {"a": 5, "b": 3},
            "id": 1
        });

        transport.send_request(request).unwrap();
        server.handle_transport(&mut transport).unwrap();

        let response = transport.receive_response().unwrap();
        assert_eq!(response["result"], 8);
    }
}
```

### InMemory Transport API

```rust
use std::sync::{Arc, Mutex};

pub struct InMemory {
    pending_requests: Arc<Mutex<VecDeque<Request>>>,
    responses: Arc<Mutex<VecDeque<Response>>>,
}

impl InMemory {
    /// Create a new in-memory transport pair.
    pub fn new() -> (Self, ClientTransport) {
        let pending = Arc::new(Mutex::new(VecDeque::new()));
        let responses = Arc::new(Mutex::new(VecDeque::new()));

        let server = Self {
            pending_requests: Arc::clone(&pending),
            responses: Arc::clone(&responses),
        };

        let client = ClientTransport {
            pending,
            responses,
        };

        (server, client)
    }
}

impl Transport for InMemory {
    fn receive_message(&mut self) -> Result<Message, Error> {
        let pending = self.pending_requests.lock().unwrap();
        match pending.pop_front() {
            Some(request) => Ok(Message::Request(request)),
            None => Err(Error::TransportError(
                std::io::Error::new(std::io::ErrorKind::WouldBlock, "No pending requests")
            )),
        }
    }

    fn send_response(&mut self, response: &Response) -> Result<(), Error> {
        let mut responses = self.responses.lock().unwrap();
        responses.push_back(response.clone());
        Ok(())
    }

    fn send_notification(&mut self, _notification: &Notification) -> Result<(), Error> {
        Ok(())
    }

    fn close(&mut self) -> Result<(), Error> {
        Ok(())
    }
}

/// Client-side transport for testing.
pub struct ClientTransport {
    pending: Arc<Mutex<VecDeque<Request>>>,
    responses: Arc<Mutex<VecDeque<Response>>>,
}

impl ClientTransport {
    /// Send a request from the client.
    pub fn send_request(&self, request: Request) -> Result<(), Error> {
        let mut pending = self.pending.lock().unwrap();
        pending.push_back(request);
        Ok(())
    }

    /// Receive a response from the server.
    pub fn receive_response(&self) -> Result<Response, Error> {
        let mut responses = self.responses.lock().unwrap();
        match responses.pop_front() {
            Some(response) => Ok(response),
            None => Err(Error::TransportError(
                std::io::Error::new(std::io::ErrorKind::WouldBlock, "No responses available")
            )),
        }
    }
}
```

### Transport Selection Guide

| Transport     | Use Case               | Pros                      | Cons                  |
| ------------- | ---------------------- | ------------------------- | --------------------- |
| **Stdio**     | CLI tools, LSP, MCP    | Simple, widely used       | Blocking I/O          |
| **InMemory**  | Unit tests, in-process | Fast, no network overhead | Only works in-process |
| **TCP**       | Network services       | Remote access             | More complex          |
| **WebSocket** | Web browsers           | Real-time, full-duplex    | More complex          |
| **Custom**    | Special needs          | Full control              | More code to maintain |

### Best Practices

1. **Use InMemory for tests**:

    ```rust
    #[test]
    fn test_my_method() {
        let mut server = create_test_server();
        let transport = InMemory::new();
        server.run_with_transport(transport).unwrap();
    }
    ```

2. **Make transport configurable**:

    ```rust
    fn run_server(transport: Option<Box<dyn Transport>>) -> Result<()> {
        let mut server = Server::new();
        server.register("echo", handler)?;

        match transport {
            Some(t) => server.run_with_transport(t)?,
            None => server.run()?,
        }

        Ok(())
    }
    ```

3. **Test with multiple transports**:

    ```rust
    #[test]
    fn test_with_stdio() {
        let transport = Stdio::new();
        test_with_transport(transport);
    }

    #[test]
    fn test_with_inmemory() {
        let (transport, _) = InMemory::new();
        test_with_transport(transport);
    }
    ```

---

## Complete Examples

### Example 1: Simple Echo Server with Graceful Shutdown

````rust
//! Simple echo server with graceful shutdown support.
//!
//! Usage:
//! ```bash
//! cargo run --example simple_server
//! ```
//!
//! In another terminal:
//! ```bash
//! echo '{"jsonrpc":"2.0","method":"echo","params":{"message":"hello"},"id":1}' | cargo run --example simple_server
//! ```

use json_rpc::{Server, ShutdownSignal};
use anyhow::Result;

#[derive(serde::Deserialize)]
struct EchoParams {
    message: String,
}

fn main() -> Result<()> {
    let mut server = Server::new();
    let shutdown = ShutdownSignal::new();

    // Register echo method
    server.register("echo", |params: EchoParams| {
        Ok(params.message)
    })?;

    // Set up graceful shutdown
    let shutdown_clone = shutdown.clone();
    ctrlc::set_handler(move || {
        println!("\nShutting down...");
        shutdown_clone.signal();
    }).expect("Error setting Ctrl-C handler");

    println!("Echo server running. Press Ctrl+C to stop.");

    server.run_until_shutdown(shutdown)?;

    println!("Server shut down gracefully.");
    Ok(())
}
````

### Example 2: Multi-method Server with Organized Names

```rust
//! Multi-method server demonstrating organized method naming.
//!
//! Uses slash-based names for hierarchical organization:
//! - session/new
//! - session/delete
//! - session/get
//! - users/get
//! - users/create

use json_rpc::{Server, ShutdownSignal};
use anyhow::Result;

// Session methods
#[derive(serde::Deserialize)]
struct NewSessionParams {
    user_id: String,
}

#[derive(serde::Serialize)]
struct Session {
    id: String,
    user_id: String,
    created_at: i64,
}

// User methods
#[derive(serde::Deserialize)]
struct GetUserParams {
    id: String,
}

#[derive(serde::Serialize)]
struct User {
    id: String,
    name: String,
}

fn main() -> Result<()> {
    let mut server = Server::new();
    let shutdown = ShutdownSignal::new();

    // Session methods
    server.register("session/new", |params: NewSessionParams| {
        let session_id = format!("sess_{}", uuid::Uuid::new_v4());
        Ok(Session {
            id: session_id,
            user_id: params.user_id,
            created_at: chrono::Utc::now().timestamp(),
        })
    })?;

    server.register("session/delete", |params: serde_json::Value| {
        // Delete session logic
        Ok(true)
    })?;

    server.register("session/get", |params: serde_json::Value| {
        // Get session logic
        Ok(serde_json::json!({
            "id": "sess_123",
            "user_id": "user_456",
            "created_at": 1234567890
        }))
    })?;

    // User methods
    server.register("users/get", |params: GetUserParams| {
        Ok(User {
            id: params.id,
            name: "Alice".to_string(),
        })
    })?;

    server.register("users/create", |params: serde_json::Value| {
        // Create user logic
        Ok(serde_json::json!({
            "id": "user_789",
            "name": "Bob"
        }))
    })?;

    // Graceful shutdown
    let shutdown_clone = shutdown.clone();
    ctrlc::set_handler(move || {
        println!("\nShutting down...");
        shutdown_clone.signal();
    }).expect("Error setting Ctrl-C handler");

    println!("Multi-method server running. Press Ctrl+C to stop.");

    server.run_until_shutdown(shutdown)?;

    println!("Server shut down gracefully.");
    Ok(())
}
```

### Example 3: Server with Error Handling

```rust
//! Server demonstrating automatic error handling.

use json_rpc::Server;
use anyhow::Result;

#[derive(serde::Deserialize)]
struct DivideParams {
    numerator: f64,
    denominator: f64,
}

#[derive(serde::Deserialize)]
struct GreetParams {
    name: String,
}

fn main() -> Result<()> {
    let mut server = Server::new();

    // Automatic error handling for division by zero
    server.register("math/divide", |params: DivideParams| -> Result<f64, anyhow::Error> {
        if params.denominator == 0.0 {
            anyhow::bail!("Cannot divide by zero");
        }
        Ok(params.numerator / params.denominator)
    })?;

    // Method with automatic parameter validation
    server.register("greet", |params: GreetParams| {
        if params.name.trim().is_empty() {
            anyhow::bail!("Name cannot be empty");
        }
        Ok(format!("Hello, {}!", params.name))
    })?;

    // Method that can fail
    server.register("fail", |_params: serde_json::Value| -> Result<String, anyhow::Error> {
        anyhow::bail!("This method always fails")
    })?;

    println!("Server with error handling running.");
    println!("Try: echo '{\"jsonrpc\":\"2.0\",\"method\":\"math/divide\",\"params\":{\"numerator\":10,\"denominator\":0},\"id\":1}' | cargo run");

    server.run()?;
    Ok(())
}
```

---

## Handler Definition Patterns

### Overview

The `Server::register()` method accepts any function that satisfies the trait
bounds:

```rust
pub fn register<F, P, R>(&mut self, name: &str, handler: F) -> Result<(), Error>
where
    F: Fn(P) -> Result<R, Error> + Send + Sync + 'static,
    P: serde::de::DeserializeOwned + 'static,
    R: serde::Serialize + 'static;
```

Since the `Fn` trait is implemented for regular functions (not just closures),
you can define handlers in many ways:

- Standalone functions
- Functions in modules
- Methods on structs
- Closures stored in variables

### Option 1: Standalone Functions

Define handlers as top-level functions:

```rust
use json_rpc::{Server, ShutdownSignal};
use anyhow::Result;

// Parameter types
#[derive(serde::Deserialize)]
struct AddParams {
    a: i32,
    b: i32,
}

// Standalone handler function
fn handle_add(params: AddParams) -> Result<i32> {
    Ok(params.a + params.b)
}

#[derive(serde::Deserialize)]
struct GreetParams {
    name: String,
}

// Another standalone handler
fn handle_greet(params: GreetParams) -> Result<String> {
    Ok(format!("Hello, {}!", params.name))
}

fn main() -> Result<()> {
    let mut server = Server::new();

    // Register standalone functions as handlers
    server.register("add", handle_add)?;
    server.register("greet", handle_greet)?;

    server.run()?;
    Ok(())
}
```

### Option 2: Handlers in Modules

Organize handlers by feature in separate modules:

```rust
// user_handlers.rs
pub mod user_handlers {
    use anyhow::Result;

    #[derive(serde::Deserialize)]
    pub struct GetUserParams {
        user_id: String,
    }

    #[derive(serde::Serialize)]
    pub struct User {
        id: String,
        name: String,
        email: String,
    }

    pub fn get_user(params: GetUserParams) -> Result<User> {
        // Database lookup logic here
        Ok(User {
            id: params.user_id,
            name: "John Doe".to_string(),
            email: "john@example.com".to_string(),
        })
    }

    pub fn delete_user(params: GetUserParams) -> Result<String> {
        // Delete logic here
        Ok(format!("User {} deleted", params.user_id))
    }
}

// main.rs
use json_rpc::Server;
use anyhow::Result;

fn main() -> Result<()> {
    let mut server = Server::new();

    // Register handlers from modules
    server.register("user/get", user_handlers::get_user)?;
    server.register("user/delete", user_handlers::delete_user)?;

    server.run()?;
    Ok(())
}
```

### Option 3: Handlers as Struct Methods

Encapsulate state and behavior in structs:

```rust
use json_rpc::{Server, ShutdownSignal};
use anyhow::Result;
use std::sync::Arc;
use std::collections::HashMap;

// Database struct that holds state
#[derive(Clone)]
struct Database {
    users: Arc<Mutex<HashMap<String, User>>>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
struct User {
    id: String,
    name: String,
    email: String,
}

impl Database {
    fn new() -> Self {
        Self {
            users: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    // Handler method
    fn create_user(&self, params: CreateUserParams) -> Result<User> {
        let user = User {
            id: params.user_id,
            name: params.name,
            email: params.email,
        };

        self.users.lock().unwrap()
            .insert(user.id.clone(), user.clone());

        Ok(user)
    }

    // Another handler method
    fn get_user(&self, params: GetUserParams) -> Result<User> {
        self.users.lock().unwrap()
            .get(&params.user_id)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("User not found"))
    }
}

#[derive(serde::Deserialize)]
struct CreateUserParams {
    user_id: String,
    name: String,
    email: String,
}

#[derive(serde::Deserialize)]
struct GetUserParams {
    user_id: String,
}

fn main() -> Result<()> {
    let mut server = Server::new();
    let db = Database::new();

    // Register struct methods as handlers
    // Note: need to capture `db` by cloning
    server.register("user/create", {
        let db = db.clone();
        move |params| db.create_user(params)
    })?;

    server.register("user/get", {
        let db = db.clone();
        move |params| db.get_user(params)
    })?;

    server.run()?;
    Ok(())
}
```

### Option 4: Handler Registry Pattern

Create a registry struct that manages all handlers:

```rust
use json_rpc::{Server, ShutdownSignal};
use anyhow::Result;

struct HandlerRegistry {
    server: Server,
}

impl HandlerRegistry {
    fn new() -> Self {
        Self {
            server: Server::new(),
        }
    }

    // Register all handlers in one place
    fn register_all(&mut self) -> Result<()> {
        self.server.register("math/add", self::add)?;
        self.server.register("math/subtract", self::subtract)?;
        self.server.register("math/multiply", self::multiply)?;
        self.server.register("math/divide", self::divide)?;
        Ok(())
    }

    fn run(&mut self) -> Result<()> {
        self.server.run()
    }

    // Handler implementations
    fn add(params: MathParams) -> Result<f64> {
        Ok(params.a + params.b)
    }

    fn subtract(params: MathParams) -> Result<f64> {
        Ok(params.a - params.b)
    }

    fn multiply(params: MathParams) -> Result<f64> {
        Ok(params.a * params.b)
    }

    fn divide(params: MathParams) -> Result<f64> {
        if params.b == 0.0 {
            anyhow::bail!("Division by zero");
        }
        Ok(params.a / params.b)
    }
}

#[derive(serde::Deserialize)]
struct MathParams {
    a: f64,
    b: f64,
}

fn main() -> Result<()> {
    let mut registry = HandlerRegistry::new();
    registry.register_all()?;
    registry.run()?;
    Ok(())
}
```

### Best Practices

1. **Organize by feature**: Group related handlers in modules
2. **Use clear naming**: Prefix with method type (get*, create*, delete\_)
3. **Separate params and results**: Define structs for better documentation
4. **Keep handlers focused**: Single responsibility per handler
5. **Reuse types**: Share common types across handlers when appropriate

### When to Use Each Pattern

| Pattern              | Use Case                         |
| -------------------- | -------------------------------- |
| Standalone functions | Simple handlers without state    |
| Module handlers      | Grouping related functionality   |
| Struct methods       | Handlers that need shared state  |
| Registry pattern     | Large servers with many handlers |

## Implementation Details

### HandlerFn Trait Implementation

```rust
trait HandlerFn: Send + Sync {
    fn call(&self, params: serde_json::Value) -> Result<serde_json::Value, Error>;
}

impl<F, P, R> HandlerFn for F
where
    F: Fn(P) -> Result<R, Error> + Send + Sync,
    P: serde::de::DeserializeOwned,
    R: serde::Serialize,
{
    fn call(&self, params: serde_json::Value) -> Result<serde_json::Value, Error> {
        // Deserialize parameters
        let params: P = serde_json::from_value(params).map_err(|e| {
            Error::invalid_params(format!("Parameter parsing error: {}", e))
        })?;

        // Call handler
        let result = self(params)?;

        // Serialize result
        serde_json::to_value(result).map_err(|e| {
            Error::internal_error(format!("Result serialization error: {}", e))
        })
    }
}
```

### Server Internals

```rust
pub struct Server {
    handlers: HashMap<String, Box<dyn HandlerFn>>,
}

impl Server {
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
        }
    }

    pub fn register<F, P, R>(&mut self, name: &str, handler: F) -> Result<(), Error>
    where
        F: Fn(P) -> Result<R, Error> + Send + Sync + 'static,
        P: serde::de::DeserializeOwned + 'static,
        R: serde::Serialize + 'static,
    {
        // Validate method name
        if name.starts_with("rpc.") {
            return Err(Error::ProtocolError(
                "Method names starting with 'rpc.' are reserved".to_string()
            ));
        }

        self.handlers.insert(name.to_string(), Box::new(handler));
        Ok(())
    }

    pub fn run_until_shutdown(&mut self, shutdown: ShutdownSignal) -> Result<(), Error> {
        let mut transport = Stdio::default();

        loop {
            // Check for shutdown signal
            if shutdown.is_shutdown_requested() {
                break;
            }

            // Receive message
            match transport.receive_message() {
                Ok(message) => {
                    self.handle_message(message, &mut transport)?;
                }
                Err(Error::TransportError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                    break;
                }
                Err(Error::TransportError(e)) if e.kind() == std::io::ErrorKind::Interrupted => {
                    // Interrupted, check shutdown signal again
                    continue;
                }
                Err(e) => {
                    eprintln!("Transport error: {}", e);
                    break;
                }
            }
        }

        // Graceful shutdown: wait for in-flight requests
        self.wait_for_in_flight_requests()?;

        // Close transport
        transport.close()?;

        Ok(())
    }

    fn handle_message(&self, message: Message, transport: &mut impl Transport) -> Result<(), Error> {
        match message {
            Message::Request(request) => {
                let handler = self.handlers.get(&request.method);

                let response = match handler {
                    Some(handler) => {
                        let params = request.params.unwrap_or(serde_json::Value::Null);
                        match handler.call(params) {
                            Ok(result) => Response::success(request.id, result),
                            Err(e) => Response::error(request.id, e.into_jsonrpc_error()),
                        }
                    }
                    None => {
                        Response::error(
                            request.id,
                            Error::method_not_found(format!("Method '{}' not found", request.method)),
                        )
                    }
                };

                transport.send_response(&response)?;
            }
            Message::Notification(notification) => {
                // Handle notifications
            }
            Message::Response(_) => {
                // Server doesn't handle responses
            }
        }
        Ok(())
    }
}
```

### Shutdown Implementation

```rust
impl Server {
    fn wait_for_in_flight_requests(&self) -> Result<(), Error> {
        // In a real implementation, this would:
        // 1. Track in-flight requests
        // 2. Wait for them to complete
        // 3. Timeout after a configurable period
        // 4. Force close if timeout expires

        // For now, just wait briefly
        thread::sleep(Duration::from_millis(100));
        Ok(())
    }
}
```

### ShutdownSignal Implementation

```rust
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[derive(Clone)]
pub struct ShutdownSignal {
    inner: Arc<AtomicBool>,
}

impl ShutdownSignal {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn check_shutdown(&self) -> Result<(), Error> {
        if self.is_shutdown_requested() {
            Err(Error::Shutdown)
        } else {
            Ok(())
        }
    }

    pub fn is_shutdown_requested(&self) -> bool {
        self.inner.load(Ordering::SeqCst)
    }

    pub fn signal(&self) {
        self.inner.store(true, Ordering::SeqCst);
    }
}
```

### CancellationToken Implementation

```rust
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[derive(Clone)]
pub struct CancellationToken {
    inner: Arc<AtomicBool>,
}

impl CancellationToken {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn check_cancelled(&self) -> Result<(), Error> {
        if self.is_cancelled() {
            Err(Error::Cancelled)
        } else {
            Ok(())
        }
    }

    pub fn is_cancelled(&self) -> bool {
        self.inner.load(Ordering::SeqCst)
    }

    pub fn cancel(&self) {
        self.inner.store(true, Ordering::SeqCst);
    }
}
```

### Thread Pool Implementation

**Question**: If `json-rpc-rs` supports thread pool, is it part of Transport
implementation or core library? How is it implemented?

**Answer**: Thread pool support is part of the **core library**, not the
Transport implementation. The thread pool is implemented following the Rust
book's recommended pattern for building a thread pool.

#### Why Not in Transport?

The `Transport` trait is responsible for **I/O operations only**:

- `receive_message()` - reading data from the wire
- `send_response()` - writing responses to the wire
- `send_notification()` - writing notifications to the wire
- `close()` - closing the transport connection

The transport has no knowledge of how requests are executed or processed. It
simply moves bytes.

#### Thread Pool Architecture

The thread pool is implemented in the **core request handling logic** following
the pattern recommended in
[The Rust Programming Language, Chapter 21.2](https://doc.rust-lang.org/book/ch21-02-multithreaded.html):

**ThreadPool Structure:**

```rust
pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: Option<mpsc::Sender<Job>>,
}

type Job = Box<dyn FnOnce() + Send + 'static>;

struct Worker {
    id: usize,
    thread: Option<thread::JoinHandle<()>>,
}
```

**Key Design Principles:**

1. **Fixed Thread Count**: The thread pool is created with a fixed number of
   worker threads (configurable via `with_thread_pool_size()`)
2. **Channel-Based Job Queue**: Uses `mpsc::channel` to send jobs from the main
   loop to worker threads
3. **Shared Receiver**: Workers share the receiver using
   `Arc<Mutex<Receiver<Job>>>` to safely pull jobs from the queue
4. **Graceful Shutdown**: Follows the pattern from
   [Chapter 21.3](https://doc.rust-lang.org/book/ch21-03-graceful-shutdown-and-cleanup.html):
    - Drop the sender to close the channel
    - Workers detect closed channel when `recv()` returns an error
    - Workers exit their loops after finishing current job
    - Main thread joins all workers via `JoinHandle::join()`

**Worker Loop Pattern:**

```rust
impl Worker {
    fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Job>>>) -> Worker {
        let thread = thread::spawn(move || {
            loop {
                // IMPORTANT: Use let binding to release lock before executing job
                let message = receiver.lock().unwrap().recv();

                match message {
                    Ok(job) => {
                        job();  // Lock is released, allowing other workers to receive
                    }
                    Err(_) => {
                        // Channel closed, exit loop for graceful shutdown
                        break;
                    }
                }
            }
        });

        Worker {
            id,
            thread: Some(thread),
        }
    }
}
```

### Implementation Details

**ThreadPool Creation:**

```rust
impl ThreadPool {
    pub fn new(size: usize) -> ThreadPool {
        assert!(size > 0, "Thread pool size must be greater than zero");

        let (sender, receiver) = mpsc::channel();
        let receiver = Arc::new(Mutex::new(receiver));

        let mut workers = Vec::with_capacity(size);

        for id in 0..size {
            workers.push(Worker::new(id, Arc::clone(&receiver)));
        }

        ThreadPool {
            workers,
            sender: Some(sender),
        }
    }

    pub fn execute<F>(&self, job: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let job = Box::new(job);
        self.sender.as_ref().unwrap().send(job).unwrap();
    }
}
```

**Graceful Shutdown Implementation:**

```rust
impl Drop for ThreadPool {
    fn drop(&mut self) {
        // Step 1: Drop sender to close the channel
        // This signals all workers that no more jobs will be sent
        drop(self.sender.take());

        // Step 2: Join all worker threads
        // Workers will exit their loops after finishing current jobs
        for worker in self.workers.drain(..) {
            if let Some(thread) = worker.thread.take() {
                thread.join().unwrap();
            }
        }
    }
}
```

**Server Integration with Thread Pool:**

```rust
impl Server {
    pub fn run_until_shutdown(&mut self, shutdown: ShutdownSignal) -> Result<(), Error> {
        // Create thread pool with configured size
        let pool = ThreadPool::new(self.thread_pool_size);

        // Clone handlers for use in worker threads
        let handlers = Arc::new(Mutex::new(self.handlers.clone()));

        loop {
            // Check for shutdown signal
            shutdown.check_shutdown()?;

            // Receive message from transport
            let message = self.transport.receive_message()?;

            // Clone references for the worker thread
            let handlers = Arc::clone(&handlers);
            let mut transport = self.transport.clone();

            // Execute in thread pool
            pool.execute(move || {
                let response = handle_message(message, &handlers);
                transport.send_response(response);
            });
        }

        // When shutdown is signaled, pool goes out of scope
        // Drop impl on ThreadPool ensures graceful cleanup
    }
}
```

### How to Set Maximum Thread Count

**Question:** How do I set the maximum number of threads in the thread pool?

**Answer:** Use the `with_thread_pool_size()` builder method when creating the
Server:

```rust
// Example 1: Use default (number of CPU cores)
let mut server = Server::new();

// Example 2: Set custom thread pool size
let mut server = Server::new()
    .with_thread_pool_size(4);

// Example 3: Single-threaded (no concurrency)
let mut server = Server::new()
    .with_thread_pool_size(1);
```

**Choosing the Right Size:**

- **Default behavior**: Uses `num_cpus::get()` (number of CPU cores)
- **For CPU-bound workloads**: Set to number of cores
- **For I/O-bound workloads**: Can be higher than core count (2x-4x)
- **For memory constraints**: Use smaller size to limit memory usage
- **Minimum**: Must be at least 1 (panics otherwise)

**Why Limit Thread Count:**

1. **Resource Protection**: Prevents DoS attacks from creating unlimited threads
2. **Predictable Performance**: Upper bound on concurrent operations
3. **Memory Control**: Each thread has a stack; limiting threads limits memory
4. **Better Scheduling**: OS scheduler can manage a fixed pool efficiently

### Graceful Shutdown with Thread Pool

The thread pool implementation follows the Rust book's recommended pattern for
graceful shutdown (Chapter 21.3):

**Shutdown Flow:**

```
1. Shutdown signal received
   ↓
2. Main loop exits, ThreadPool goes out of scope
   ↓
3. ThreadPool::drop() is called
   ↓
4. Sender is dropped → channel is closed
   ↓
5. Workers recv() returns Err (channel closed)
   ↓
6. Workers exit their loops after finishing current job
   ↓
7. Main thread joins all workers
   ↓
8. All in-flight requests complete
   ↓
9. Method returns cleanly
```

**Key Properties:**

- **No request abandonment**: In-flight requests complete before shutdown
- **No new requests accepted**: Sender dropped prevents new jobs
- **Parallel finish**: Multiple workers can finish simultaneously
- **Clean exit**: All threads are properly joined

### Cancellation Support

The thread pool enables request cancellation through concurrent processing:

**Cancellation Flow:**

```
1. Long-running request (session/prompt) arrives
   ↓
2. Starts processing in Worker 0
   ↓
3. Cancel request (session/cancel) arrives
   ↓
4. Starts processing in Worker 1 (different thread)
   ↓
5. Worker 1 accesses shared cancellation token map
   ↓
6. Worker 1 marks session/prompt token as cancelled
   ↓
7. Worker 0 checks token periodically (cooperative)
   ↓
8. Worker 0 detects cancellation and exits early
   ↓
9. Worker 0 returns cancellation error to client
```

**Why Thread Pool is Essential for Cancellation:**

- Without thread pool: Cancel request would wait for long operation to complete
- With thread pool: Cancel request runs in parallel, can interrupt immediately
- Concurrency: Both requests execute simultaneously in different worker threads

### Comparison: thread::spawn vs ThreadPool

| Feature           | `thread::spawn`       | ThreadPool      |
| ----------------- | --------------------- | --------------- |
| Thread count      | Unlimited (unbounded) | Fixed size      |
| Resource usage    | Can exhaust system    | Predictable     |
| DoS protection    | None                  | Built-in        |
| Graceful shutdown | Difficult (manual)    | Built-in (Drop) |
| Memory usage      | Unbounded             | Fixed           |
| Startup overhead  | Per request           | One-time        |

**Why ThreadPool is Preferred:**

The Rust book (Chapter 21.2) explicitly recommends using a thread pool instead
of spawning threads per request:

> "Rather than spawning unlimited threads, we'll have a fixed number of threads
> waiting in the pool. Requests that come in are sent to the pool for
> processing. This technique protects us from DoS attacks and provides
> predictable performance."

---

## Trade-offs and Limitations

### Thread Pool Trade-offs

**Advantages:**

- **Predictable resource usage**: Fixed number of threads limits memory/CPU
  usage
- **DoS protection**: Cannot be overwhelmed by request floods
- **Graceful shutdown**: Built-in cleanup ensures clean exit
- **Cancellation support**: Parallel processing enables real-time cancellation

**Limitations:**

- **Queue backlog**: If all workers are busy, requests wait in queue
- **Fixed size**: Cannot scale beyond configured thread count
- **Startup latency**: Threads created at startup (one-time cost)
- **Complexity**: More complex than simple `thread::spawn`

**When to Adjust Thread Pool Size:**

- **High concurrency needs**: Increase pool size for more parallel processing
- **Limited resources**: Decrease pool size to reduce memory/CPU usage
- **I/O-bound workloads**: Larger pool (2x-4x cores) works well
- **CPU-bound workloads**: Pool size = number of cores is optimal

### Advantages

1. **Simplicity**: Minimal code required (~15 lines vs ~70 lines)
2. **Type Safety**: Compile-time parameter checking
3. **Flexibility**: Supports any method name including slashes
4. **Graceful Shutdown**: Built-in support for clean shutdown with
   ShutdownSignal
5. **Cancellation Support**: Built-in support for cancelling long-running
   operations with CancellationToken
6. **Automatic Error Handling**: Proper JSON-RPC error codes generated
   automatically
7. **Transport Agnostic**: Easy switching between transports for testing vs
   production

### Limitations

1. **Runtime Method Lookup**: Uses HashMap, so method name typos result in
   runtime errors
2. **No Compile-time Method Verification**: Unlike the macro approach, method
   names are strings
3. **Less Control**: Less flexibility than manual Router implementation
4. **Closure Captures**: State management relies on closure captures (can be
   complex)
5. **Thread Pool Overhead**: Concurrent processing via thread pool has overhead
   for thread scheduling and synchronization. For very simple operations, this
   overhead may be noticeable.
6. **Cooperative Cancellation**: Long-running methods must explicitly check
   cancellation tokens

### When to Use Builder Pattern

**Use when:**

- You have a simple server with <10 methods
- You need quick prototyping
- You're new to JSON-RPC
- You want minimal boilerplate
- Your routing logic is straightforward
- You need easy testing with in-memory transports

**Don't use when:**

- You need complex routing logic (method aliasing, dynamic dispatch)
- You need compile-time method verification
- You have many methods (>50)
- You need advanced features like middleware
- You need to minimize overhead for very simple operations (thread pool has
  scheduling overhead)

### Comparison with Other Approaches

| Feature             | Builder                    | Macro                | Router Trait    |
| ------------------- | -------------------------- | -------------------- | --------------- |
| Boilerplate         | Low                        | Very Low             | High            |
| Type Safety         | Parameters only            | Parameters + Methods | Parameters only |
| Method Names        | Runtime                    | Compile-time         | Runtime         |
| Flexibility         | Medium                     | Low                  | High            |
| Learning Curve      | Low                        | Medium               | High            |
| Slash Support       | ✅ Yes                     | ✅ Yes               | ✅ Yes          |
| Graceful Shutdown   | ✅ Yes (ShutdownSignal)    | ✅ Yes               | ⚠️ Manual       |
| Cancellation        | ✅ Yes (CancellationToken) | ✅ Yes               | ⚠️ Manual       |
| Transport Switching | ✅ Yes                     | ✅ Yes               | ✅ Yes          |

### Performance Considerations

- **Method Lookup**: O(1) HashMap lookup per request
- **Parameter Deserialization**: Once per request (via serde)
- **Closure Call**: Minimal overhead (indirect function call)
- **Memory**: One closure per registered method
- **Shutdown/Cancellation**: AtomicBool operations (very fast, no locking)

For most use cases, performance is more than adequate. If you need extreme
performance, consider the manual Router trait approach.

---

- **Document Version:** 1.1
