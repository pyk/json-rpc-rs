# Tokio Integration Research for json-rpc-rs

## Sources

- [tokio README](https://raw.githubusercontent.com/tokio-rs/tokio/refs/heads/master/README.md)
- [Graceful Shutdown in tokio](https://tokio.rs/tokio/topics/shutdown)
- [tokio Glossary](https://tokio.rs/tokio/glossary)
- [tokio docs homepage](https://docs.rs/tokio/latest/tokio/)
- [Cargo Features Reference](https://doc.rust-lang.org/cargo/reference/features.html)

---

## 1. Tokio Feature Flags for Stdio Transport

Since `json-rpc-rs` only needs to support stdio transport, the following tokio
feature flags should be enabled:

```toml
[dependencies]
tokio = { version = "1.49.0", features = ["rt-multi-thread", "sync", "macros", "time", "io-std", "io-util"] }
```

### Feature Flags Explanation

- **`rt-multi-thread`**: Enables the multi-threaded, work-stealing scheduler for
  efficient async task execution on multiple CPU cores.

- **`sync`**: Enables synchronization primitives in `tokio::sync` including
  channels (`oneshot`, `mpsc`, `watch`, `broadcast`), `Mutex`, `Semaphore`, and
  `RwLock` for managing concurrent request handling and internal state.

- **`macros`**: Enables `#[tokio::main]` and `#[tokio::test]` macros for easier
  testing.

- **`time`**: Enables `tokio::time` utilities including `sleep()`, `timeout()`,
  and `interval()` for request timeouts and timing.

- **`io-std`**: Enables `Stdout`, `Stdin`, and `Stderr` types for asynchronous
  stdio transport.

- **`io-util`**: Enables I/O based `Ext` traits (`AsyncReadExt`,
  `AsyncWriteExt`, `AsyncBufReadExt`) and utilities for reading and writing
  JSON-RPC messages over stdin/stdout.

**Note**: The `net` feature flag is not needed since only stdio transport is
supported.

---

## 2. Dynamic Transport Feature Configuration

Based on the Cargo features documentation, here's how to allow users to
dynamically enable features based on the transport used:

### Cargo.toml Configuration

```toml
[dependencies]
tokio = { version = "1.49.0", features = ["rt-multi-thread", "sync", "macros", "time", "io-util"], optional = true }
tokio-util = { version = "0.7", optional = true }

[features]
default = ["tokio/rt-multi-thread", "tokio/sync", "tokio/macros", "tokio/time", "tokio/io-util", "transport-stdio"]

# Transport features
transport-stdio = ["tokio/io-std"]
transport-http-sse = ["tokio/net", "tokio-util/compat"]
transport-tcp = ["tokio/net"]
transport-uds = ["tokio/net"]
```

### Explanation

1. **Base tokio features** (`rt-multi-thread`, `sync`, `macros`, `time`,
   `io-util`) are always required regardless of transport type.

2. **Transport-specific features** enable additional tokio features needed for
   that transport:
    - `transport-stdio` enables `tokio/io-std` for stdin/stdout
    - `transport-http-sse` enables `tokio/net` for HTTP and `tokio-util/compat`
      for SSE compatibility
    - `transport-tcp` enables `tokio/net` for TCP sockets
    - `transport-uds` enables `tokio/net` for Unix domain sockets

3. **Using the syntax `"tokio/io-std"`** enables a specific feature of the
   `tokio` dependency when the `transport-stdio` feature is enabled.

### Conditional Compilation

```rust
#[cfg(feature = "transport-stdio")]
pub mod stdio_transport;

#[cfg(feature = "transport-http-sse")]
pub mod http_sse_transport;
```

### Usage Examples

Enable only stdio transport:

```toml
# In user's Cargo.toml
json-rpc-rs = { version = "1.0", features = ["transport-stdio"] }
```

Enable multiple transports:

```toml
json-rpc-rs = { version = "1.0", features = ["transport-stdio", "transport-http-sse"] }
```

**Note**: According to the Cargo features documentation, features should be
additive. Enabling a feature should not disable functionality, and it should be
safe to enable any combination of features.
