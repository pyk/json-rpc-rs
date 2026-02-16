//! JSON-RPC message handler.
//!
//! This module provides the `Handler` which handles the I/O loop
//! for JSON-RPC communication. It is protocol-agnostic - you provide
//! a router to handle method dispatch.

use crate::error::Error;
use crate::router::Router;
use crate::transports::{Stdio, Transport};
use crate::types::{Message, Request, Response};

/// JSON-RPC handler for processing messages.
///
/// This handler owns the transport and runs the main I/O loop,
/// handling message parsing, routing, and response sending.
///
/// # Type Parameters
///
/// - `R`: The router implementation
/// - `T`: The transport implementation (defaults to `Stdio`)
pub struct Handler<R, T = Stdio>
where
    R: Router,
    T: Transport,
{
    transport: T,
    router: R,
}

impl<R, T> Handler<R, T>
where
    R: Router,
    T: Transport,
{
    /// Create a new handler with the given router and transport.
    pub fn new_with_transport(router: R, transport: T) -> Self {
        Self { transport, router }
    }

    /// Create a new handler with the given router and default transport.
    ///
    /// Uses `Stdio` as the default transport.
    pub fn new(router: R) -> Self
    where
        T: Default,
    {
        Self {
            transport: T::default(),
            router,
        }
    }

    /// Run the main I/O loop.
    ///
    /// This method blocks and continuously reads messages from the transport,
    /// processes them via the router, and sends responses.
    pub fn run(&mut self) -> Result<(), Error> {
        loop {
            match self.transport.receive_message() {
                Ok(message) => {
                    if let Err(e) = self.handle_message(message) {
                        eprintln!("Error handling message: {}", e);
                    }
                }
                Err(Error::TransportError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                    break;
                }
                Err(e) => {
                    eprintln!("Transport error: {}", e);
                    break;
                }
            }
        }
        Ok(())
    }

    /// Handle a single JSON-RPC message.
    pub fn handle_message(&mut self, message: Message) -> Result<(), Error> {
        match message {
            Message::Request(request) => {
                self.handle_request(request)?;
            }
            Message::Notification(notification) => {
                self.handle_notification(notification)?;
            }
            Message::Response(_response) => {}
        }
        Ok(())
    }

    /// Handle a JSON-RPC request by routing it through the router.
    fn handle_request(&mut self, request: Request) -> Result<(), Error> {
        let id = request.id.clone();
        let method = self.router.route(request);

        let result = self
            .router
            .handle(method, || Err(Error::protocol("Handler not configured")));

        let response = match result {
            Ok(Some(value)) => Response::success(id.clone(), value),
            Ok(None) => Response::success(id.clone(), serde_json::Value::Null),
            Err(e) => {
                let error = crate::types::Error::new(-32000, e.to_string(), None);
                Response::error(id, error)
            }
        };

        self.send_response(response)
    }

    /// Handle a JSON-RPC notification.
    fn handle_notification(
        &mut self,
        _notification: crate::types::Notification,
    ) -> Result<(), Error> {
        Ok(())
    }

    /// Send a JSON-RPC response.
    pub fn send_response(&mut self, response: Response) -> Result<(), Error> {
        self.transport.send_response(&response)
    }

    /// Send a JSON-RPC notification.
    pub fn send_notification(
        &mut self,
        notification: crate::types::Notification,
    ) -> Result<(), Error> {
        self.transport.send_notification(&notification)
    }

    /// Take the transport for custom handling.
    pub fn take_transport(&mut self) -> T
    where
        T: Default,
    {
        std::mem::take(&mut self.transport)
    }

    /// Get a reference to the transport.
    pub fn transport(&self) -> &T {
        &self.transport
    }

    /// Get a reference to the router.
    pub fn router(&self) -> &R {
        &self.router
    }
}

impl<R, T> Default for Handler<R, T>
where
    R: Router + Default,
    T: Transport + Default,
{
    fn default() -> Self {
        Self::new(R::default())
    }
}
