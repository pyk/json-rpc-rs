# Cargo Dev Dependency Features

## Sources

- [Cargo Features Reference](https://doc.rust-lang.org/cargo/reference/features.html)
- [Cargo Specifying Dependencies](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html)
- [Cargo Dependency Resolution](https://doc.rust-lang.org/cargo/reference/resolver.html#feature-resolver-version-2)

---

## Answered Questions

1. Is it possible to enable Cargo features only for dev dependencies?

---

## 1. Enabling Features Only for Dev Dependencies

**Answering**: Question 1

Yes, you can enable features only for dev dependencies. Use resolver version 2
and specify the same dependency in both `[dependencies]` and
`[dev-dependencies]` sections with different feature sets.

### Resolver Version Requirement

Set `resolver = "2"` in `[package]` or `[workspace]` section of `Cargo.toml`:

```toml
[package]
name = "my-package"
version = "0.1.0"
edition = "2024"
resolver = "2"
```

The edition "2024" defaults to resolver version 2. The edition "2021" also
defaults to resolver version 2.

### Configuration Pattern

Specify the dependency without dev-specific features in `[dependencies]` and
with dev-specific features in `[dev-dependencies]`:

```toml
[dependencies]
tokio = { version = "1", features = ["rt", "sync", "io-std"] }

[dev-dependencies]
tokio = { version = "1", features = ["rt", "sync", "macros", "io-std"] }
```

### How It Works

With resolver version 2, features enabled on dev-dependencies will not be
unified when those same dependencies are used as a normal dependency, unless
those dev-dependencies are currently being built. For example:

- Normal builds: `tokio` is compiled with `rt`, `sync`, and `io-std` features
- Test builds: `tokio` is compiled with `rt`, `sync`, `macros`, and `io-std`
  features

The `macros` feature is only enabled when building tests, examples, or
benchmarks.

### When Dev Features Are Enabled

Dev-dependencies are used only when compiling tests, examples, and benchmarks.
This includes:

- `cargo test`
- `cargo test --all-targets`
- `cargo build --examples`
- `cargo build --benches`

During normal builds like `cargo build` or `cargo check`, the dev-dependencies
features are not enabled.

### Feature Unification Behavior

Without resolver version 2 (version 1), Cargo would unify features across all
uses of a dependency. If `macros` were enabled in dev-dependencies, it would
also be enabled for normal dependencies.

Resolver version 2 changes this behavior. Features enabled on dev-dependencies
remain isolated unless building dev targets. This allows different feature sets
for different contexts.

### Workspace Configuration

For workspaces, set the resolver at the workspace level:

```toml
[workspace]
members = ["crates/*"]
resolver = "2"

[workspace.dependencies]
tokio = { version = "1" }

[dependencies]
tokio = { workspace = true, features = ["rt", "sync", "io-std"] }

[dev-dependencies]
tokio = { workspace = true, features = ["rt", "sync", "macros", "io-std"] }
```
