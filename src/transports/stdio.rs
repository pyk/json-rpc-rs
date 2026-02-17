//! Stdio-based transport for JSON-RPC 2.0.
//!
//! This module implements stdio-based NDJSON (newline-delimited JSON) transport
//! for JSON-RPC 2.0 communication over stdin/stdout.

use std::io::{BufRead, BufReader, BufWriter, Write};

use crate::error::Error;
use crate::transports::Transport;

/// Stdio-based transport for JSON-RPC messages.
///
/// This transport reads newline-delimited JSON from stdin and writes
/// newline-terminated JSON to stdout. It uses buffered I/O for efficiency.
pub struct Stdio {
    reader: BufReader<std::io::Stdin>,
    writer: BufWriter<std::io::Stdout>,
}

impl Stdio {
    /// Create a new stdio transport.
    ///
    /// Uses stdin for reading and stdout for writing.
    pub fn new() -> Self {
        Self {
            reader: BufReader::new(std::io::stdin()),
            writer: BufWriter::new(std::io::stdout()),
        }
    }

    /// Read a single newline-delimited JSON message from stdin.
    ///
    /// This method blocks until a complete line is received. It handles
    /// partial reads by using buffered I/O.
    ///
    pub fn read_message(&mut self) -> Result<String, Error> {
        let mut line = String::new();
        let bytes_read = self.reader.read_line(&mut line)?;
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

    /// Write a JSON message to stdout with newline termination.
    ///
    /// This method writes the message followed by a newline character.
    /// The output is buffered for efficiency.
    ///
    pub fn write_message(&mut self, message: &str) -> Result<(), Error> {
        writeln!(self.writer, "{}", message)?;
        self.writer.flush()?;
        Ok(())
    }
}

impl Transport for Stdio {
    /// Receive a raw JSON string from stdin.
    ///
    /// Reads a newline-delimited JSON message and returns it as a string.
    /// No parsing or validation is performed - that's the responsibility
    /// of the caller (typically the server layer).
    fn receive_message(&mut self) -> Result<String, Error> {
        self.read_message()
    }

    /// Send a raw JSON string to stdout.
    ///
    /// Writes the JSON string as-is to stdout with a newline.
    /// The caller is responsible for serializing JSON-RPC messages
    /// to JSON strings before calling this method.
    fn send_message(&mut self, json: &str) -> Result<(), Error> {
        self.write_message(json)
    }
}

impl Default for Stdio {
    fn default() -> Self {
        Self::new()
    }
}
