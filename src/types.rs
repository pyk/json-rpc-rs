//! JSON-RPC 2.0 message types.
//!
//! This module defines JSON-RPC 2.0 message types as specified in:
//! https://www.jsonrpc.org/specification

use std::fmt;

use crate::error::Error as InternalError;
use serde::{Deserialize, Serialize};

/// JSON-RPC 2.0 request message.
///
/// A request is a call from client to server to execute a method.
/// See: https://www.jsonrpc.org/specification#request_object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Request {
    /// JSON-RPC version. Always "2.0".
    pub jsonrpc: String,
    /// Request identifier used to match responses.
    pub id: RequestId,
    /// Method name to invoke.
    pub method: String,
    /// Parameters for the method.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
}

impl Request {
    /// Create a new JSON-RPC request.
    pub fn new(
        id: RequestId,
        method: impl Into<String>,
        params: Option<serde_json::Value>,
    ) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            method: method.into(),
            params,
        }
    }
}

/// JSON-RPC 2.0 response message.
///
/// A response is a message sent from server to client in reply to a request.
/// See: https://www.jsonrpc.org/specification#response_object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    /// JSON-RPC version. Always "2.0".
    pub jsonrpc: String,
    /// Result of the method invocation (if successful).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    /// Error information (if the method invocation failed).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<Error>,
    /// Request identifier matching the original request.
    pub id: RequestId,
}

impl Response {
    /// Create a successful response.
    pub fn success(id: RequestId, result: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }

    /// Create an error response.
    pub fn error(id: RequestId, error: Error) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(error),
        }
    }

    /// Validate that result and error are mutually exclusive.
    ///
    /// Returns an error if both result and error are present, or if neither is present.
    pub fn validate(&self) -> Result<(), String> {
        match (&self.result, &self.error) {
            (Some(_), Some(_)) => Err("Response cannot have both result and error".to_string()),
            (None, None) => Err("Response must have either result or error".to_string()),
            _ => Ok(()),
        }
    }
}

/// JSON-RPC 2.0 notification message.
///
/// A notification is a request object without an id member.
/// A notification does not expect a response.
/// See: https://www.jsonrpc.org/specification#notification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    /// JSON-RPC version. Always "2.0".
    pub jsonrpc: String,
    /// Method name to invoke.
    pub method: String,
    /// Parameters for the method.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
}

impl Notification {
    /// Create a new JSON-RPC notification.
    pub fn new(method: impl Into<String>, params: Option<serde_json::Value>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            method: method.into(),
            params,
        }
    }
}

/// JSON-RPC 2.0 error object.
///
/// Contains error information for failed method invocations.
/// See: https://www.jsonrpc.org/specification#error_object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Error {
    /// Error code indicating the error type.
    pub code: i32,
    /// Short description of the error.
    pub message: String,
    /// Additional error data (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl Error {
    /// Create a new JSON-RPC error.
    pub fn new(code: i32, message: impl Into<String>, data: Option<serde_json::Value>) -> Self {
        Self {
            code,
            message: message.into(),
            data,
        }
    }

    /// Create a parse error (-32700).
    pub fn parse_error(message: impl Into<String>) -> Self {
        Self::new(-32700, message, None)
    }

    /// Create an invalid request error (-32600).
    pub fn invalid_request(message: impl Into<String>) -> Self {
        Self::new(-32600, message, None)
    }

    /// Create a method not found error (-32601).
    pub fn method_not_found(message: impl Into<String>) -> Self {
        Self::new(-32601, message, None)
    }

    /// Create an invalid params error (-32602).
    pub fn invalid_params(message: impl Into<String>) -> Self {
        Self::new(-32602, message, None)
    }

    /// Create an internal error (-32603).
    pub fn internal_error(message: impl Into<String>) -> Self {
        Self::new(-32603, message, None)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "JSON-RPC error {}: {}", self.code, self.message)
    }
}

/// JSON-RPC 2.0 request identifier.
///
/// An identifier established by the client that must contain a String, Number, or NULL value.
/// See: https://www.jsonrpc.org/specification#request_object
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum RequestId {
    /// Null identifier.
    Null,
    /// Number identifier.
    Number(u64),
    /// String identifier.
    String(String),
}

impl fmt::Display for RequestId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RequestId::Null => write!(f, "null"),
            RequestId::Number(n) => write!(f, "{}", n),
            RequestId::String(s) => write!(f, "{}", s),
        }
    }
}

/// JSON-RPC 2.0 message type.
///
/// Represents any JSON-RPC message: request, response, or notification.
#[derive(Debug, Clone)]
pub enum Message {
    /// Request message (expects a response).
    Request(Request),
    /// Response message (reply to a request).
    Response(Response),
    /// Notification message (no response expected).
    Notification(Notification),
}

impl Message {
    /// Try to parse JSON into a JSON-RPC message.
    ///
    /// Attempts to deserialize JSON into Request, Response, or Notification.
    /// The method first checks if the message has an `id` field to distinguish
    /// between requests and notifications. Then it checks if there's an `error`
    /// field to distinguish between requests and responses.
    ///
    /// # Errors
    ///
    /// Returns `Error::InvalidRequest` if the JSON is not a valid
    /// JSON-RPC message structure (e.g., wrong field types, missing required fields).
    /// This is distinct from parse errors (-32700) which occur for invalid JSON syntax.
    pub fn from_json(value: serde_json::Value) -> Result<Self, InternalError> {
        let value_ref = &value;
        if value_ref.get("id").is_some() {
            if value_ref.get("error").is_some() {
                serde_json::from_value(value)
                    .map(Message::Response)
                    .map_err(|e| InternalError::invalid_request(e.to_string()))
            } else if value_ref.get("method").is_some() {
                serde_json::from_value(value)
                    .map(Message::Request)
                    .map_err(|e| InternalError::invalid_request(e.to_string()))
            } else {
                serde_json::from_value(value)
                    .map(Message::Response)
                    .map_err(|e| InternalError::invalid_request(e.to_string()))
            }
        } else {
            serde_json::from_value(value)
                .map(Message::Notification)
                .map_err(|e| InternalError::invalid_request(e.to_string()))
        }
    }

    /// Convert the message to JSON.
    pub fn to_json(&self) -> Result<serde_json::Value, serde_json::Error> {
        match self {
            Message::Request(req) => serde_json::to_value(req),
            Message::Response(res) => serde_json::to_value(res),
            Message::Notification(notif) => serde_json::to_value(notif),
        }
    }

    /// Get the request ID if this is a request or response.
    pub fn id(&self) -> Option<&RequestId> {
        match self {
            Message::Request(req) => Some(&req.id),
            Message::Response(res) => Some(&res.id),
            Message::Notification(_) => None,
        }
    }

    /// Check if this message is a request.
    pub fn is_request(&self) -> bool {
        matches!(self, Message::Request(_))
    }

    /// Check if this message is a response.
    pub fn is_response(&self) -> bool {
        matches!(self, Message::Response(_))
    }

    /// Check if this message is a notification.
    pub fn is_notification(&self) -> bool {
        matches!(self, Message::Notification(_))
    }
}
