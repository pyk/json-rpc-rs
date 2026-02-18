//! Stdio-based transport for JSON-RPC 2.0.
//!
//! This module implements stdio-based NDJSON (newline-delimited JSON) transport
//! for JSON-RPC 2.0 communication over stdin/stdout using tokio async I/O.

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};

use crate::error::Error;
use crate::transports::Transport;

/// Stdio-based transport for JSON-RPC messages.
///
/// This transport reads newline-delimited JSON from stdin and writes
/// newline-terminated JSON to stdout. It uses buffered async I/O for efficiency.
pub struct Stdio {
    reader: BufReader<tokio::io::Stdin>,
    writer: BufWriter<tokio::io::Stdout>,
}

impl Stdio {
    /// Create a new stdio transport.
    ///
    /// Uses stdin for reading and stdout for writing.
    pub fn new() -> Self {
        Self {
            reader: BufReader::new(tokio::io::stdin()),
            writer: BufWriter::new(tokio::io::stdout()),
        }
    }
}

impl Transport for Stdio {
    /// Receive a raw JSON string from stdin.
    ///
    /// Reads a newline-delimited JSON message and returns it as a string.
    /// No parsing or validation is performed - that's the responsibility
    /// of the caller (typically the server layer).
    async fn receive_message(&mut self) -> Result<String, Error> {
        let mut line = String::new();
        let bytes_read = self.reader.read_line(&mut line).await?;
        if bytes_read == 0 {
            return Err(Error::TransportError(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "End of input",
            )));
        }

        if line.ends_with('\n') {
            line.pop();
            if line.ends_with('\r') {
                line.pop();
            }
        }
        Ok(line)
    }

    /// Send a raw JSON string to stdout.
    ///
    /// Writes the JSON string as-is to stdout with a newline.
    /// The caller is responsible for serializing JSON-RPC messages
    /// to JSON strings before calling this method.
    async fn send_message(&mut self, json: &str) -> Result<(), Error> {
        self.writer.write_all(json.as_bytes()).await?;
        self.writer.write_all(b"\n").await?;
        self.writer.flush().await?;
        Ok(())
    }
}
