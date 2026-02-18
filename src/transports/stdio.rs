//! Stdio-based transport for JSON-RPC 2.0.
//!
//! This module implements stdio-based NDJSON (newline-delimited JSON) transport
//! for JSON-RPC 2.0 communication over stdin/stdout using tokio async I/O.

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};

use crate::Methods;
use crate::error::Error;
use crate::transports::Transport;

/// Stdio-based transport for JSON-RPC messages.
///
/// This transport reads newline-delimited JSON from stdin and writes
/// newline-terminated JSON to stdout. It uses buffered async I/O for efficiency.
pub struct Stdio;

impl Stdio {
    /// Create a new stdio transport.
    ///
    /// Uses stdin for reading and stdout for writing.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use json_rpc::Stdio;
    ///
    /// let transport = Stdio::new();
    /// ```
    pub fn new() -> Self {
        Self
    }
}

impl Default for Stdio {
    fn default() -> Self {
        Self::new()
    }
}

impl Transport for Stdio {
    /// Serve the JSON-RPC server using stdio transport.
    ///
    /// This method runs in a loop, reading newline-delimited JSON from stdin,
    /// processing each message through the method registry, and writing
    /// newline-terminated JSON responses to stdout.
    ///
    /// The server runs until stdin is closed or an error occurs.
    async fn serve(self, methods: Methods) -> Result<(), Error> {
        let mut reader = BufReader::new(tokio::io::stdin());
        let mut writer = BufWriter::new(tokio::io::stdout());

        loop {
            let mut line = String::new();
            let bytes_read = reader.read_line(&mut line).await?;
            if bytes_read == 0 {
                break;
            }

            if line.ends_with('\n') {
                line.pop();
                if line.ends_with('\r') {
                    line.pop();
                }
            }

            if let Some(response) = methods.process_message(&line).await {
                writer.write_all(response.as_bytes()).await?;
                writer.write_all(b"\n").await?;
                writer.flush().await?;
            }
        }

        Ok(())
    }
}
