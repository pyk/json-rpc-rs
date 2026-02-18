//! Transport implementations for JSON-RPC 2.0 communication.
//!
//! This module provides various transport implementations for JSON-RPC communication,
//! including stdio-based and in-memory transports. All transports implement the
//! common [`Transport`] trait, making them interchangeable.

pub use http::Http;
pub use in_memory::InMemory;
pub use stdio::Stdio;
pub use transport::Transport;

pub mod http;
pub mod in_memory;
pub mod stdio;
pub mod transport;
