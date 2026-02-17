//! JSON-RPC 2.0 message types.
//!
//! This module defines JSON-RPC 2.0 message types as specified in:
//! https://www.jsonrpc.org/specification

use std::fmt;

use crate::error::Error as InternalError;
use serde::{Deserialize, Serialize};
use tracing::debug;

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
/// Represents any JSON-RPC message: request, response, notification, or batch.
#[derive(Debug, Clone)]
pub enum Message {
    /// Request message (expects a response).
    Request(Request),
    /// Response message (reply to a request).
    Response(Response),
    /// Notification message (no response expected).
    Notification(Notification),
    /// Batch of messages (multiple requests/notifications).
    Batch(Vec<Message>),
}

impl Message {
    /// Try to parse JSON into a JSON-RPC message.
    ///
    /// Attempts to deserialize JSON into Request, Response, Notification, or Batch.
    /// The method first checks if the message is an array to detect batch requests.
    /// For single messages, it checks if the message has an `id` field to distinguish
    /// between requests and notifications. Then it checks if there's an `error`
    /// field to distinguish between requests and responses.
    ///
    /// # Errors
    ///
    /// Returns `Error::InvalidRequest` if the JSON is not a valid
    /// JSON-RPC message structure (e.g., wrong field types, missing required fields).
    /// This is distinct from parse errors (-32700) which occur for invalid JSON syntax.
    pub fn from_json(value: serde_json::Value) -> Result<Self, InternalError> {
        debug!("Parsing JSON value: {:?}", value);
        let value_ref = &value;

        // Handle batch requests (array of messages)
        if let Some(arr) = value_ref.as_array() {
            debug!("Detected batch request with {} items", arr.len());
            // Empty array is Invalid Request
            if arr.is_empty() {
                debug!("Empty array detected - returning Invalid Request error");
                return Err(InternalError::invalid_request("Invalid Request"));
            }

            // Parse each message in the batch
            let mut messages = Vec::new();
            for (index, item) in arr.iter().enumerate() {
                debug!("Processing batch item {}: {:?}", index, item);
                match Self::from_json_internal(item.clone()) {
                    Ok(msg) => {
                        debug!("Batch item {} parsed successfully", index);
                        messages.push(msg);
                    }
                    Err(e) => {
                        debug!("Batch item {} failed to parse: {:?}", index, e);
                        // Invalid individual request - will be handled at request processing time
                        // Try to create an error response with the id if present
                        if let Some(id_value) = item.get("id") {
                            if let Ok(id) = serde_json::from_value::<RequestId>(id_value.clone()) {
                                // Create an error response for this invalid request
                                let error_response =
                                    Response::error(id, Error::invalid_request("Invalid Request"));
                                messages.push(Message::Response(error_response));
                            } else {
                                // Invalid id type, create error response with null id
                                let error_response = Response::error(
                                    RequestId::Null,
                                    Error::invalid_request("Invalid Request"),
                                );
                                messages.push(Message::Response(error_response));
                            }
                        } else {
                            // No id - check if this is a notification
                            if item.get("method").is_some() {
                                // This is a notification, skip it (notifications don't get responses)
                                debug!(
                                    "Batch item {} is a notification (has method but no id), skipping",
                                    index
                                );
                            } else {
                                // Invalid request without id - create error response with null id
                                debug!(
                                    "Batch item {} is invalid (no id or method), creating error response",
                                    index
                                );
                                let error_response = Response::error(
                                    RequestId::Null,
                                    Error::invalid_request("Invalid Request"),
                                );
                                messages.push(Message::Response(error_response));
                            }
                        }
                    }
                }
            }
            debug!("Batch parsing complete, {} messages", messages.len());
            return Ok(Message::Batch(messages));
        }

        // Check if this is a request/notification or response
        if value_ref.get("id").is_some() {
            debug!("Message has 'id' field, checking for error/method");
            if value_ref.get("error").is_some() {
                debug!("Message has 'error' field, parsing as Response");
                serde_json::from_value(value)
                    .map(Message::Response)
                    .map_err(|e| {
                        debug!("Failed to parse as Response: {}", e);
                        InternalError::invalid_request("Invalid Request")
                    })
            } else if value_ref.get("method").is_some() {
                // This is a request
                debug!("Message has 'method' field, parsing as Request");
                let req: Request = serde_json::from_value(value).map_err(|e| {
                    debug!("Failed to deserialize as Request: {}", e);
                    InternalError::invalid_request("Invalid Request")
                })?;

                // Validate jsonrpc field
                if req.jsonrpc != "2.0" {
                    debug!("Invalid jsonrpc value: '{}', expected '2.0'", req.jsonrpc);
                    return Err(InternalError::invalid_request("Invalid Request"));
                }

                debug!("Request parsed successfully: {}", req.method);
                Ok(Message::Request(req))
            } else {
                // Has id but no method or error - this is invalid
                debug!("Message has 'id' but no 'method' or 'error' - Invalid Request");
                Err(InternalError::invalid_request("Invalid Request"))
            }
        } else {
            // No id - this is a notification
            debug!("Message has no 'id' field, parsing as Notification");
            let notif: Notification = serde_json::from_value(value).map_err(|e| {
                debug!("Failed to deserialize as Notification: {}", e);
                InternalError::invalid_request("Invalid Request")
            })?;

            // Validate jsonrpc field for notification
            if notif.jsonrpc != "2.0" {
                debug!("Invalid jsonrpc value: '{}', expected '2.0'", notif.jsonrpc);
                return Err(InternalError::invalid_request("Invalid Request"));
            }

            debug!("Notification parsed successfully: {}", notif.method);
            Ok(Message::Notification(notif))
        }
    }

    /// Convert the message to JSON.
    pub fn to_json(&self) -> Result<serde_json::Value, serde_json::Error> {
        match self {
            Message::Request(req) => serde_json::to_value(req),
            Message::Response(res) => serde_json::to_value(res),
            Message::Notification(notif) => serde_json::to_value(notif),
            Message::Batch(messages) => {
                let json_array: Result<Vec<_>, _> = messages.iter().map(|m| m.to_json()).collect();
                Ok(serde_json::Value::Array(json_array?))
            }
        }
    }

    /// Get the request ID if this is a request or response.
    pub fn id(&self) -> Option<&RequestId> {
        match self {
            Message::Request(req) => Some(&req.id),
            Message::Response(res) => Some(&res.id),
            Message::Notification(_) => None,
            Message::Batch(_) => None,
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

    /// Check if this message is a batch.
    pub fn is_batch(&self) -> bool {
        matches!(self, Message::Batch(_))
    }

    /// Internal method to parse JSON without strict jsonrpc validation for batch processing.
    ///
    /// This method is identical to from_json except it doesn't handle batch requests.
    /// It's used to parse individual items in a batch.
    fn from_json_internal(value: serde_json::Value) -> Result<Self, InternalError> {
        let value_ref = &value;

        // Check if this is a request/notification or response
        if value_ref.get("id").is_some() {
            if value_ref.get("error").is_some() {
                serde_json::from_value(value)
                    .map(Message::Response)
                    .map_err(|_| InternalError::invalid_request("Invalid Request"))
            } else if value_ref.get("method").is_some() {
                // Try to deserialize as Request, catching all errors
                serde_json::from_value::<Request>(value)
                    .map(|req| {
                        // Check if jsonrpc is valid (must be "2.0")
                        if req.jsonrpc != "2.0" {
                            return Err(InternalError::invalid_request("Invalid Request"));
                        }
                        Ok(Message::Request(req))
                    })
                    .map_err(|_| InternalError::invalid_request("Invalid Request"))?
            } else {
                // Has id but no method or error - this is invalid
                Err(InternalError::invalid_request("Invalid Request"))
            }
        } else {
            // No id - this is a notification
            // Try to deserialize as Notification, catching all errors
            serde_json::from_value::<Notification>(value)
                .map(|notif| {
                    // Check if jsonrpc is valid (must be "2.0")
                    if notif.jsonrpc != "2.0" {
                        return Err(InternalError::invalid_request("Invalid Request"));
                    }
                    Ok(Message::Notification(notif))
                })
                .map_err(|_| InternalError::invalid_request("Invalid Request"))?
        }
    }
}
