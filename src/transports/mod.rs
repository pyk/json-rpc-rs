//! Transport implementations for JSON-RPC 2.0 communication.
//!
//! This module provides various transport implementations for JSON-RPC communication,
//! including stdio-based and in-memory transports. All transports implement the
//! common [`Transport`] trait, making them interchangeable.

pub mod in_memory;
pub mod stdio;
pub mod transport;

pub use transport::Transport;

// Re-export transport implementations for convenience
pub use in_memory::InMemory;
pub use stdio::Stdio;
