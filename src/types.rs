//! JSON-RPC 2.0 message types.
//!
//! This module defines JSON-RPC 2.0 message types as specified in:
//! https://www.jsonrpc.org/specification

use std::fmt;

use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::error::Error as InternalError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Request {
    pub jsonrpc: String,
    pub id: RequestId,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
}

impl Request {
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<Error>,
    pub id: RequestId,
}

impl Response {
    pub fn success(id: RequestId, result: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }

    pub fn error(id: RequestId, error: Error) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(error),
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        match (&self.result, &self.error) {
            (Some(_), Some(_)) => Err("Response cannot have both result and error".to_string()),
            (None, None) => Err("Response must have either result or error".to_string()),
            _ => Ok(()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    pub jsonrpc: String,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
}

impl Notification {
    pub fn new(method: impl Into<String>, params: Option<serde_json::Value>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            method: method.into(),
            params,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Error {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl Error {
    pub fn new(code: i32, message: impl Into<String>, data: Option<serde_json::Value>) -> Self {
        Self {
            code,
            message: message.into(),
            data,
        }
    }

    pub fn parse_error(message: impl Into<String>) -> Self {
        Self::new(-32700, message, None)
    }

    pub fn invalid_request(message: impl Into<String>) -> Self {
        Self::new(-32600, message, None)
    }

    pub fn method_not_found(message: impl Into<String>) -> Self {
        Self::new(-32601, message, None)
    }

    pub fn invalid_params(message: impl Into<String>) -> Self {
        Self::new(-32602, message, None)
    }

    pub fn internal_error(message: impl Into<String>) -> Self {
        Self::new(-32603, message, None)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "JSON-RPC error {}: {}", self.code, self.message)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum RequestId {
    Null,
    Number(u64),
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

#[derive(Debug, Clone)]
pub enum Message {
    Request(Request),
    Response(Response),
    Notification(Notification),
    Batch(Vec<Message>),
}

impl Message {
    pub fn from_json(value: serde_json::Value) -> Result<Self, InternalError> {
        debug!("Parsing JSON value: {:?}", value);
        if let Some(arr) = value.as_array() {
            return Self::parse_batch(arr);
        }

        Self::parse_single(value)
    }

    fn parse_batch(arr: &[serde_json::Value]) -> Result<Self, InternalError> {
        if arr.is_empty() {
            return Err(InternalError::invalid_request("Invalid Request"));
        }

        let messages: Result<Vec<Message>, InternalError> =
            arr.iter().map(Self::parse_batch_item).collect();

        Ok(Message::Batch(messages?))
    }

    fn parse_batch_item(item: &serde_json::Value) -> Result<Self, InternalError> {
        match Self::parse_single(item.clone()) {
            Ok(msg) => Ok(msg),
            Err(_) => Ok(Message::Response(Response::error(
                RequestId::Null,
                Error::invalid_request("Invalid Request"),
            ))),
        }
    }

    fn parse_single(value: serde_json::Value) -> Result<Self, InternalError> {
        let has_id = value.get("id").is_some();

        if has_id {
            if value.get("error").is_some() {
                let resp: Response = serde_json::from_value(value)
                    .map_err(|_| InternalError::invalid_request("Invalid Request"))?;
                resp.validate()
                    .map_err(|_| InternalError::invalid_request("Invalid Request"))?;
                Ok(Message::Response(resp))
            } else if value.get("method").is_some() {
                let req: Request = serde_json::from_value(value)
                    .map_err(|_| InternalError::invalid_request("Invalid Request"))?;
                if req.jsonrpc != "2.0" {
                    return Err(InternalError::invalid_request("Invalid Request"));
                }

                Ok(Message::Request(req))
            } else {
                Err(InternalError::invalid_request("Invalid Request"))
            }
        } else {
            let notif: Notification = serde_json::from_value(value)
                .map_err(|_| InternalError::invalid_request("Invalid Request"))?;
            if notif.jsonrpc != "2.0" {
                return Err(InternalError::invalid_request("Invalid Request"));
            }

            Ok(Message::Notification(notif))
        }
    }

    pub fn to_json(&self) -> Result<serde_json::Value, serde_json::Error> {
        match self {
            Message::Request(req) => serde_json::to_value(req),
            Message::Response(res) => serde_json::to_value(res),
            Message::Notification(notif) => serde_json::to_value(notif),
            Message::Batch(messages) => {
                let json_array: Result<Vec<serde_json::Value>, serde_json::Error> =
                    messages.iter().map(|m| m.to_json()).collect();
                Ok(serde_json::Value::Array(json_array?))
            }
        }
    }

    pub fn id(&self) -> Option<&RequestId> {
        match self {
            Message::Request(req) => Some(&req.id),
            Message::Response(res) => Some(&res.id),
            Message::Notification(_) => None,
            Message::Batch(_) => None,
        }
    }

    pub fn is_request(&self) -> bool {
        matches!(self, Message::Request(_))
    }

    pub fn is_response(&self) -> bool {
        matches!(self, Message::Response(_))
    }

    pub fn is_notification(&self) -> bool {
        matches!(self, Message::Notification(_))
    }

    pub fn is_batch(&self) -> bool {
        matches!(self, Message::Batch(_))
    }
}
