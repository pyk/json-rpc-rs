//! Stdio-based transport for JSON-RPC 2.0.
//!
//! This module implements stdio-based NDJSON (newline-delimited JSON) transport
//! for JSON-RPC 2.0 communication over stdin/stdout.

use std::io::{BufRead, BufReader, BufWriter, Write};

use crate::error::Error;
use crate::transports::Transport;
use crate::types::{Message, Notification, Request, Response};

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
    /// Receive a JSON-RPC message from stdin.
    ///
    /// Reads a newline-delimited JSON message and attempts to parse it
    /// as a JSON-RPC request, response, or notification.
    ///
    fn receive_message(&mut self) -> Result<Message, Error> {
        let json_str = self.read_message()?;
        let value: serde_json::Value = serde_json::from_str(&json_str)?;
        Message::from_json(value).map_err(Error::from)
    }

    /// Send a JSON-RPC request.
    ///
    /// Serializes the request and writes it to stdout.
    ///
    fn send_request(&mut self, request: &Request) -> Result<(), Error> {
        let json = serde_json::to_string(request)?;
        self.write_message(&json)
    }

    /// Send a JSON-RPC response.
    ///
    /// Serializes the response and writes it to stdout.
    ///
    fn send_response(&mut self, response: &Response) -> Result<(), Error> {
        let json = serde_json::to_string(response)?;
        self.write_message(&json)
    }

    /// Send a JSON-RPC notification.
    ///
    /// Serializes the notification and writes it to stdout.
    ///
    fn send_notification(&mut self, notification: &Notification) -> Result<(), Error> {
        let json = serde_json::to_string(notification)?;
        self.write_message(&json)
    }
}

impl Default for Stdio {
    fn default() -> Self {
        Self::new()
    }
}
