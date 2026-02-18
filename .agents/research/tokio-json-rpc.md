# Tokio JSON-RPC Research

## Sources

- [Tokio README](https://raw.githubusercontent.com/tokio-rs/tokio/refs/heads/master/README.md)
- [Tokio Graceful Shutdown](https://tokio.rs/tokio/topics/shutdown)
- [Tokio Glossary](https://tokio.rs/tokio/glossary)
- [Tokio API Documentation](https://docs.rs/tokio/latest/tokio/)
- [Cargo Features Reference](https://doc.rust-lang.org/cargo/reference/features.html)

---

## Answered Questions

1. What tokio feature flags should be enabled?
2. How to allow user to dynamically enable feature based on the transport used?

---

## 1. Minimal Tokio Features for JSON-RPC Requirements

**Answering**: Question 1

For a json-rpc-rs library supporting stdio transport with graceful shutdown,
request cancellation, and batch requests, enable these Tokio feature flags:

### Base Runtime Features

**`rt`**: Required for `tokio::spawn` and task scheduling. The `tokio::task`
module is present only when the "rt" feature flag is enabled.

**`sync`**: Required for synchronization primitives including
`CancellationToken` for graceful shutdown and request cancellation. The
`tokio::sync` module is present only when the "sync" feature flag is enabled.

**`macros`**: Required for `#[tokio::main]` and `#[tokio::test]` attributes. The
`macros` feature flag enables these macros.

### Transport-Specific Features

**`io-std`**: Required for stdio transport. This enables `Stdout`, `Stdin`, and
`Stderr` types.

**`net`**: Required for HTTP/SSE or other network transports. This enables
`tokio::net` types such as `TcpStream`, `UnixStream`, and `UdpSocket`.

### Optional Features

**`time`**: Useful for request timeouts. Enables `tokio::time` types and allows
the schedulers to enable the built-in timer.

**`rt-multi-thread`**: Enables the heavier, multi-threaded, work-stealing
scheduler. The default `rt` provides the current-thread single-threaded
scheduler.

### Recommended Minimal Configuration

For stdio transport only:

```toml
[dependencies]
tokio = { version = "1", features = ["rt", "sync", "macros", "io-std"] }
```

For applications (not libraries), the Tokio documentation recommends using the
`full` feature flag:

```toml
[dependencies]
tokio = { version = "1", features = ["full"] }
```

The `full` feature enables all features except `test-util` and `tracing`.

For libraries, enable only the features you need to provide the lightest weight
crate.

---

## 2. Dynamic Feature Enabling for Transports

**Answering**: Question 2

Use Cargo's feature system to allow users to enable transport-specific features
dynamically.

### Feature Definition Structure

Define features in the `[features]` section of `Cargo.toml`. Features can
specify other features or optional dependencies that they enable.

```toml
[dependencies]
tokio = { version = "1", default-features = false }

[features]
default = ["transport-stdio"]

# Transport features
transport-stdio = ["tokio/io-std"]
transport-http-sse = ["tokio/net", "tokio/time"]
```

### Default Feature Configuration

Use the `default` feature to specify which transport is enabled by default. By
default, all features are disabled unless explicitly enabled.

```toml
[features]
default = ["transport-stdio"]
```

Users can disable default features with the `--no-default-features` command-line
flag or `default-features = false` in the dependency declaration.

### Transport Feature Composition

Transport features can enable multiple Tokio features to provide complete
functionality for that transport.

```toml
[dependencies]
tokio = { version = "1", default-features = false }

[features]
# Stdio transport enables base runtime features plus stdio I/O
transport-stdio = ["tokio/rt", "tokio/sync", "tokio/macros", "tokio/io-std"]

# HTTP/SSE transport enables network and timing features
transport-http-sse = ["tokio/rt", "tokio/sync", "tokio/macros", "tokio/net", "tokio/time"]

# Users can enable multiple transports
transport-all = ["transport-stdio", "transport-http-sse"]
```

### Usage Examples

Users can enable specific transports when adding the dependency:

```toml
# Default (stdio only)
json-rpc-rs = "0.1"

# HTTP/SSE transport only
json-rpc-rs = { version = "0.1", features = ["transport-http-sse"], default-features = false }

# Multiple transports
json-rpc-rs = { version = "0.1", features = ["transport-stdio", "transport-http-sse"] }
```

### Optional Dependencies Approach

Alternatively, mark optional dependencies and create implicit features:

```toml
[dependencies]
tokio = { version = "1", optional = true, default-features = false }

[features]
default = ["tokio/rt", "tokio/sync", "tokio/macros", "tokio/io-std"]

transport-stdio = ["tokio/rt", "tokio/sync", "tokio/macros", "tokio/io-std"]
transport-http-sse = ["tokio/rt", "tokio/sync", "tokio/macros", "tokio/net", "tokio/time"]
```

### Conditional Compilation

Use `#[cfg(feature = "...")]` attributes to conditionally include code based on
enabled features:

```rust
#[cfg(feature = "transport-stdio")]
pub mod stdio;

#[cfg(feature = "transport-http-sse")]
pub mod http_sse;
```

### Feature Guidelines

- Features should be additive. Enabling a feature should not disable
  functionality.
- Features should usually be safe to enable in any combination.
- For libraries, provide the lightest weight crate by enabling only necessary
  features.
- Feature names may include letters, numbers, `-`, `+`, and `.` (after the first
  character).
