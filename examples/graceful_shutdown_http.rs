//! A JSON-RPC 2.0 server demonstrating graceful shutdown with HTTP.
//!
//! This example shows how to implement graceful shutdown in a JSON-RPC server
//! using the json-rpc-rs library with HTTP transport. When the user presses CTRL+C,
//! the server will:
//!
//! 1. Stop accepting new connections
//! 2. Wait for in-flight requests to complete
//! 3. Exit cleanly
//!
//! The example includes a `long_running_operation` method that simulates work,
//! allowing you to see how pending requests are handled during shutdown.
//!
//! Usage:
//!
//! ```bash
//! cargo run --example graceful_shutdown_http
//! ```
//!
//! Then, in another terminal:
//!
//! ```bash
//! # Start a long-running request
//! curl -X POST http://localhost:3000/jsonrpc \
//!   -H "Content-Type: application/json" \
//!   -d '{"jsonrpc":"2.0","method":"long_running_operation","params":{"duration_ms":10000},"id":1}'
//!
//! # While that's running, press CTRL+C in the server terminal
//! ```
//!
//! Expected behavior:
//! - The server will log that shutdown was initiated
//! - The long-running request will complete
//! - The server will exit gracefully after all pending requests finish

use std::sync::Arc;

use anyhow::Context;
use anyhow::Result;
use axum::{Router, routing::post};
use json_rpc::JsonRpc;
use json_rpc::axum::handler;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::{error, info};

/// Parameters for the long running operation.
#[derive(Debug, Deserialize)]
struct LongRunningParams {
    /// Duration in milliseconds to simulate work
    duration_ms: u64,
}

/// Result from the long running operation.
#[derive(Debug, Serialize)]
struct LongRunningResult {
    /// Whether the operation completed
    completed: bool,
    /// Duration actually executed
    duration_ms: u64,
}

/// Simulates a long-running operation.
///
/// This method sleeps for the specified duration, checking for cancellation
/// periodically. If the server is shutting down during the operation, it will
/// be interrupted gracefully.
async fn long_running_operation(
    params: LongRunningParams,
) -> Result<LongRunningResult, json_rpc::Error> {
    info!(
        "Starting long-running operation with duration {}ms",
        params.duration_ms
    );

    let start = std::time::Instant::now();

    let mut elapsed = 0;
    while elapsed < params.duration_ms {
        let remaining = params.duration_ms - elapsed;
        let sleep_duration = std::time::Duration::from_millis(remaining.min(100));

        tokio::select! {
            _ = tokio::time::sleep(sleep_duration) => {
                elapsed += sleep_duration.as_millis() as u64;
            }
            _ = tokio::signal::ctrl_c() => {
                info!("Long-running operation interrupted by shutdown signal");
                return Ok(LongRunningResult {
                    completed: false,
                    duration_ms: elapsed,
                });
            }
        }
    }

    let actual_duration = start.elapsed().as_millis() as u64;
    info!("Long-running operation completed in {}ms", actual_duration);

    Ok(LongRunningResult {
        completed: true,
        duration_ms: actual_duration,
    })
}

/// Echo method that returns the input parameters unchanged.
async fn echo(params: Value) -> Result<Value, json_rpc::Error> {
    info!("Echo called with params: {:?}", params);
    Ok(params)
}

/// Health check method.
async fn health(_params: Value) -> Result<String, json_rpc::Error> {
    info!("Health check called");
    Ok("healthy".to_string())
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .with_writer(std::io::stderr)
        .init();

    info!("Initializing graceful shutdown HTTP server");

    let json_rpc = JsonRpc::new()
        .add("long_running_operation", long_running_operation)
        .add("echo", echo)
        .add("health", health);

    let app = Router::new()
        .route("/jsonrpc", post(handler))
        .with_state(Arc::new(json_rpc));

    let addr: std::net::SocketAddr = "127.0.0.1:3000".parse()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;
    let local_addr = listener.local_addr()?;

    info!("Server started on http://{}", local_addr);
    info!("JSON-RPC endpoint: http://{}/jsonrpc", local_addr);
    info!("Available methods:");
    info!("  - long_running_operation: Simulates long work (params: {{duration_ms: number}})");
    info!("  - echo: Echoes back any JSON params");
    info!("  - health: Returns 'healthy'");
    info!("");
    info!("Press CTRL+C to initiate graceful shutdown");
    info!("The server will wait for in-flight requests to complete before exiting");

    let shutdown_signal = async {
        match tokio::signal::ctrl_c().await {
            Ok(()) => {
                info!("CTRL+C received - initiating graceful shutdown");
            }
            Err(err) => {
                error!("Failed to listen for shutdown signal: {}", err);
            }
        }
        info!("Shutdown signal received");
    };

    info!("Starting server with graceful shutdown support");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal)
        .await
        .context("HTTP server error")?;

    info!("Server has shut down gracefully");
    Ok(())
}
