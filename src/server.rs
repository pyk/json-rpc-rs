//! JSON-RPC server with builder pattern.
//!
//! This module provides a `Server` that uses a builder pattern for
//! method registration and includes a thread pool for concurrent
//! request handling.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;

use serde::Serialize;
use tracing::debug;

use crate::error::Error;
use crate::shutdown::ShutdownSignal;
use crate::transports::{Stdio, Transport};
use crate::types::{Message, Notification, Request, RequestId, Response};

trait HandlerFn: Send + Sync {
    fn call(&self, params: serde_json::Value) -> Result<serde_json::Value, Error>;
}

struct HandlerWrapper<F, P, R>
where
    F: Fn(P) -> Result<R, Error> + Send + Sync + 'static,
    P: serde::de::DeserializeOwned + Send + Sync + 'static,
    R: Serialize + Send + Sync + 'static,
{
    f: Arc<F>,
    _phantom: std::marker::PhantomData<(P, R)>,
}

impl<F, P, R> HandlerFn for HandlerWrapper<F, P, R>
where
    F: Fn(P) -> Result<R, Error> + Send + Sync + 'static,
    P: serde::de::DeserializeOwned + Send + Sync + 'static,
    R: Serialize + Send + Sync + 'static,
{
    fn call(&self, params: serde_json::Value) -> Result<serde_json::Value, Error> {
        let parsed: P = serde_json::from_value(params)?;
        let result = (self.f)(parsed)?;
        Ok(serde_json::to_value(result)?)
    }
}

type Job = Box<dyn FnOnce() + Send + 'static>;

struct Worker {
    _handle: thread::JoinHandle<()>,
}

impl Worker {
    fn spawn(_id: usize, receiver: Arc<Mutex<std::sync::mpsc::Receiver<Job>>>) -> Self {
        let handle = thread::spawn(move || {
            loop {
                let job = {
                    let rx = match receiver.lock() {
                        Ok(guard) => guard,
                        Err(_) => break,
                    };
                    rx.recv()
                };

                match job {
                    Ok(job) => job(),
                    Err(_) => break,
                }
            }
        });

        Self { _handle: handle }
    }
}

struct ThreadPool {
    workers: Vec<Worker>,
    sender: Option<std::sync::mpsc::Sender<Job>>,
}

impl ThreadPool {
    fn new(size: usize) -> Self {
        assert!(size > 0, "Thread pool size must be greater than 0");

        let (sender, receiver) = std::sync::mpsc::channel();
        let receiver = Arc::new(Mutex::new(receiver));

        let mut workers = Vec::with_capacity(size);

        for id in 0..size {
            workers.push(Worker::spawn(id, Arc::clone(&receiver)));
        }

        Self {
            workers,
            sender: Some(sender),
        }
    }

    fn execute<F>(&self, job: F) -> Result<(), Error>
    where
        F: FnOnce() + Send + 'static,
    {
        let job = Box::new(job);
        let sender = self.sender.as_ref().ok_or_else(|| {
            Error::TransportError(std::io::Error::new(
                std::io::ErrorKind::NotConnected,
                "Thread pool is not available",
            ))
        })?;

        sender.send(job).map_err(|_| {
            Error::TransportError(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "Failed to send job to thread pool",
            ))
        })
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        drop(self.sender.take());
        for _worker in &mut self.workers {}
    }
}

struct ResponseData {
    response: Response,
    batch_id: Option<usize>,
    batch_index: Option<usize>,
}

struct BatchContext {
    responses: Vec<Option<Response>>,
    expected_count: usize,
}

pub struct Server {
    handlers: HashMap<String, Box<dyn HandlerFn>>,
    thread_pool_size: usize,
    shutdown_signal: Option<ShutdownSignal>,
    transport: Option<Box<dyn Transport>>,
}

impl Server {
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
            thread_pool_size: num_cpus::get(),
            shutdown_signal: None,
            transport: None,
        }
    }

    pub fn with_thread_pool_size(mut self, size: usize) -> Self {
        assert!(size > 0, "Thread pool size must be greater than 0");
        self.thread_pool_size = size;
        self
    }

    pub fn with_shutdown_signal(mut self, signal: ShutdownSignal) -> Self {
        self.shutdown_signal = Some(signal);
        self
    }

    pub fn with_transport<T>(mut self, transport: T) -> Self
    where
        T: Transport + 'static,
    {
        self.transport = Some(Box::new(transport));
        self
    }

    pub fn register<F, P, R>(&mut self, method: &str, handler: F) -> Result<(), Error>
    where
        F: Fn(P) -> Result<R, Error> + Send + Sync + 'static,
        P: serde::de::DeserializeOwned + Send + Sync + 'static,
        R: Serialize + Send + Sync + 'static,
    {
        let wrapper = HandlerWrapper {
            f: Arc::new(handler),
            _phantom: std::marker::PhantomData,
        };
        self.handlers.insert(method.to_string(), Box::new(wrapper));
        Ok(())
    }

    pub async fn run(&mut self) -> Result<(), Error> {
        let mut transport = self
            .transport
            .take()
            .unwrap_or_else(|| Box::new(Stdio::default()) as Box<dyn Transport>);
        let thread_pool = ThreadPool::new(self.thread_pool_size);
        let handlers = Arc::new(std::sync::Mutex::new(std::mem::take(&mut self.handlers)));
        let shutdown_signal = self.shutdown_signal.clone();
        let (response_sender, response_receiver) = std::sync::mpsc::channel::<ResponseData>();
        let mut batches: HashMap<usize, BatchContext> = HashMap::new();
        let mut next_batch_id: usize = 0;

        loop {
            if let Some(ref signal) = shutdown_signal
                && signal.is_shutdown_requested()
            {
                break;
            }

            let json_str = match transport.receive_message() {
                Ok(msg) => {
                    debug!("Received message from transport: {}", msg);
                    msg
                }
                Err(Error::TransportError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                    debug!("EOF received, breaking loop");
                    break;
                }
                Err(e) => {
                    debug!("Transport error: {}", e);
                    let error = crate::types::Error::internal_error("Internal error");
                    let response = Response::error(RequestId::Null, error);
                    let json = match serde_json::to_string(&response) {
                        Ok(json) => json,
                        Err(e) => {
                            eprintln!("Failed to serialize internal error response: {}", e);
                            continue;
                        }
                    };
                    debug!("Sending internal error response: {}", json);
                    let _ = transport.send_message(&json);
                    continue;
                }
            };

            let value: serde_json::Value = match serde_json::from_str(&json_str) {
                Ok(v) => {
                    debug!("JSON parsed successfully");
                    v
                }
                Err(_e) => {
                    debug!("Failed to parse JSON string: {}", json_str);
                    let error = crate::types::Error::parse_error("Parse error");
                    let response = Response::error(RequestId::Null, error);
                    let json = match serde_json::to_string(&response) {
                        Ok(json) => json,
                        Err(e) => {
                            eprintln!("Failed to serialize parse error response: {}", e);
                            continue;
                        }
                    };
                    debug!("Sending parse error response: {}", json);
                    let _ = transport.send_message(&json);
                    continue;
                }
            };

            let request_id = value.get("id").and_then(|id_value| match id_value {
                serde_json::Value::Null => Some(RequestId::Null),
                serde_json::Value::Number(n) => n.as_u64().map(RequestId::Number),
                serde_json::Value::String(s) => Some(RequestId::String(s.clone())),
                _ => None,
            });
            debug!("Extracted request_id: {:?}", request_id);

            let message = match Message::from_json(value) {
                Ok(msg) => {
                    debug!("Message parsed successfully");
                    msg
                }
                Err(Error::InvalidRequest(e)) => {
                    debug!("Invalid Request error caught: {}", e);
                    let error = crate::types::Error::invalid_request("Invalid Request");
                    let id_to_use = request_id.unwrap_or(RequestId::Null);
                    debug!("Using request_id in error response: {:?}", id_to_use);
                    let response = Response::error(id_to_use, error);
                    let json = match serde_json::to_string(&response) {
                        Ok(json) => json,
                        Err(e) => {
                            eprintln!("Failed to serialize invalid request error response: {}", e);
                            continue;
                        }
                    };
                    debug!("Sending Invalid Request error response: {}", json);
                    let _ = transport.send_message(&json);
                    continue;
                }
                Err(e) => {
                    debug!("Error parsing message: {}", e);
                    eprintln!("Error parsing message: {}", e);
                    let error = crate::types::Error::internal_error("Internal error");
                    let response = Response::error(request_id.unwrap_or(RequestId::Null), error);
                    let json = match serde_json::to_string(&response) {
                        Ok(json) => json,
                        Err(e) => {
                            eprintln!("Failed to serialize internal error response: {}", e);
                            continue;
                        }
                    };
                    debug!("Sending internal error response: {}", json);
                    let _ = transport.send_message(&json);
                    continue;
                }
            };

            let handlers_clone = Arc::clone(&handlers);

            match message {
                Message::Request(request) => {
                    let sender_clone = response_sender.clone();
                    thread_pool.execute(move || {
                        if let Err(e) = Self::process_request(handlers_clone, sender_clone, request)
                        {
                            eprintln!("Error processing request: {}", e);
                        }
                    })?;
                }
                Message::Notification(notification) => {
                    if let Err(e) = Self::process_notification(handlers_clone, notification) {
                        eprintln!("Error processing notification: {}", e);
                    }
                }
                Message::Batch(messages) => {
                    let batch_id = next_batch_id;
                    next_batch_id = next_batch_id.wrapping_add(1);

                    let request_count = messages
                        .iter()
                        .filter(|m| matches!(m, Message::Request(_) | Message::Response(_)))
                        .count();

                    if request_count > 0 {
                        batches.insert(
                            batch_id,
                            BatchContext {
                                responses: vec![None; request_count],
                                expected_count: request_count,
                            },
                        );

                        if let Err(e) = Self::process_batch(
                            &thread_pool,
                            handlers_clone,
                            response_sender.clone(),
                            batch_id,
                            messages,
                        ) {
                            eprintln!("Error processing batch: {}", e);
                            batches.remove(&batch_id);
                        }
                    } else {
                        eprintln!("Batch contains only notifications - no response sent");
                    }
                }
                Message::Response(_response) => {}
            }

            while let Ok(response_data) =
                response_receiver.recv_timeout(std::time::Duration::from_millis(100))
            {
                if let Some(batch_id) = response_data.batch_id
                    && let Some(batch_index) = response_data.batch_index
                    && let Some(batch) = batches.get_mut(&batch_id)
                    && batch_index < batch.responses.len()
                {
                    batch.responses[batch_index] = Some(response_data.response);

                    let completed = batch.responses.iter().filter(|r| r.is_some()).count();
                    if completed == batch.expected_count {
                        let responses: Vec<Response> =
                            batch.responses.drain(..).flatten().collect();

                        if !responses.is_empty() {
                            let batch_json = serde_json::to_string(&responses)?;
                            transport.send_message(&batch_json)?;
                        }

                        batches.remove(&batch_id);
                    }
                } else {
                    let json = serde_json::to_string(&response_data.response)?;
                    transport.send_message(&json)?;
                }
            }
        }

        while let Ok(response_data) =
            response_receiver.recv_timeout(std::time::Duration::from_millis(100))
        {
            let json = serde_json::to_string(&response_data.response)?;
            transport.send_message(&json)?;
        }

        Ok(())
    }

    fn process_request(
        handlers: Arc<std::sync::Mutex<HashMap<String, Box<dyn HandlerFn>>>>,
        sender: std::sync::mpsc::Sender<ResponseData>,
        request: Request,
    ) -> Result<(), Error> {
        Self::process_request_with_batch(handlers, sender, request, None, None)
    }

    fn process_request_with_batch(
        handlers: Arc<std::sync::Mutex<HashMap<String, Box<dyn HandlerFn>>>>,
        sender: std::sync::mpsc::Sender<ResponseData>,
        request: Request,
        batch_id: Option<usize>,
        batch_index: Option<usize>,
    ) -> Result<(), Error> {
        let id = request.id.clone();
        let method_name = request.method.clone();
        let params = request.params.unwrap_or(serde_json::Value::Null);

        let response = match handlers.lock() {
            Ok(handlers_lock) => match handlers_lock.get(&method_name) {
                Some(handler) => match handler.call(params) {
                    Ok(result) => Response::success(id, result),
                    Err(Error::RpcError { code, message }) => {
                        let error = crate::types::Error::new(code, message, None);
                        Response::error(id, error)
                    }
                    Err(e) => {
                        let error = crate::types::Error::new(-32603, e.to_string(), None);
                        Response::error(id, error)
                    }
                },
                None => {
                    let error = crate::types::Error::method_not_found(format!(
                        "Unknown method: {}",
                        method_name
                    ));
                    Response::error(id, error)
                }
            },
            Err(_) => {
                let error = crate::types::Error::internal_error("Internal server error");
                Response::error(id, error)
            }
        };

        sender
            .send(ResponseData {
                response,
                batch_id,
                batch_index,
            })
            .map_err(|e| {
                Error::TransportError(std::io::Error::new(std::io::ErrorKind::BrokenPipe, e))
            })?;

        Ok(())
    }

    fn process_notification(
        handlers: Arc<std::sync::Mutex<HashMap<String, Box<dyn HandlerFn>>>>,
        notification: Notification,
    ) -> Result<(), Error> {
        eprintln!("Processing notification: {}", notification.method);
        let method_name = notification.method.clone();
        let params = notification.params.unwrap_or(serde_json::Value::Null);

        match handlers.lock() {
            Ok(handlers_lock) => match handlers_lock.get(&method_name) {
                Some(handler) => {
                    let _ = handler.call(params);
                    Ok(())
                }
                None => Ok(()),
            },
            Err(_) => Ok(()),
        }
    }

    fn process_batch(
        thread_pool: &ThreadPool,
        handlers: Arc<std::sync::Mutex<HashMap<String, Box<dyn HandlerFn>>>>,
        sender: std::sync::mpsc::Sender<ResponseData>,
        batch_id: usize,
        messages: Vec<Message>,
    ) -> Result<(), Error> {
        let mut request_index = 0;

        for message in messages {
            match message {
                Message::Request(request) => {
                    let handlers_clone = Arc::clone(&handlers);
                    let sender_clone = sender.clone();
                    let index = request_index;
                    request_index += 1;

                    thread_pool.execute(move || {
                        if let Err(e) = Self::process_request_with_batch(
                            handlers_clone,
                            sender_clone,
                            request,
                            Some(batch_id),
                            Some(index),
                        ) {
                            eprintln!("Error processing request in batch: {}", e);
                        }
                    })?;
                }
                Message::Notification(notification) => {
                    if let Err(e) = Self::process_notification(handlers.clone(), notification) {
                        eprintln!("Error processing notification in batch: {}", e);
                    }
                }
                Message::Response(response) => {
                    let sender_clone = sender.clone();
                    let index = request_index;
                    request_index += 1;

                    sender_clone
                        .send(ResponseData {
                            response,
                            batch_id: Some(batch_id),
                            batch_index: Some(index),
                        })
                        .map_err(|e| {
                            Error::TransportError(std::io::Error::new(
                                std::io::ErrorKind::BrokenPipe,
                                e,
                            ))
                        })?;
                }
                _ => {
                    debug!("Unexpected message type in batch: {:?}", message);
                }
            }
        }

        Ok(())
    }
}

impl Default for Server {
    fn default() -> Self {
        Self::new()
    }
}
