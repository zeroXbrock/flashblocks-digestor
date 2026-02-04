use serde::Serialize;

use crate::{
    envelope::StreamEnvelope, error::StreamError, print::PrintStream, sse::SseServer,
    r#trait::DataStream, websocket::WebSocketServer,
};

/// Enum wrapper for different stream output types
/// Allows runtime selection of stream implementation
pub enum StreamOutput {
    /// Print to stdout
    Print(PrintStream),
    /// WebSocket server broadcasting to connected clients
    WebSocket(WebSocketServer),
    /// Server-Sent Events (SSE) server broadcasting to connected clients
    Sse(SseServer),
}

impl StreamOutput {
    /// Create a new print stream output
    pub fn print() -> Self {
        Self::Print(PrintStream)
    }

    /// Create a new websocket stream output with default capacity
    pub fn websocket() -> Self {
        Self::WebSocket(WebSocketServer::with_default_capacity())
    }

    /// Create a new websocket stream output with specified capacity
    pub fn websocket_with_capacity(capacity: usize) -> Self {
        Self::WebSocket(WebSocketServer::new(capacity))
    }

    /// Create a new SSE stream output with default capacity
    pub fn sse() -> Self {
        Self::Sse(SseServer::with_default_capacity())
    }

    /// Create a new SSE stream output with specified capacity
    pub fn sse_with_capacity(capacity: usize) -> Self {
        Self::Sse(SseServer::new(capacity))
    }

    /// Start the underlying stream if needed (e.g., WebSocket server)
    /// For PrintStream, this is a no-op
    pub async fn start(&self, addr: &str) -> Result<(), StreamError> {
        match self {
            Self::Print(_) => Ok(()),
            Self::WebSocket(ws) => ws.start(addr).await,
            Self::Sse(sse) => sse.start(addr).await,
        }
    }
}

impl DataStream for StreamOutput {
    fn send<T: Serialize>(&self, data_type: &str, data: &T) -> Result<(), StreamError> {
        match self {
            Self::Print(stream) => stream.send(data_type, data),
            Self::WebSocket(stream) => stream.send(data_type, data),
            Self::Sse(stream) => stream.send(data_type, data),
        }
    }

    fn send_envelope<T: Serialize>(&self, envelope: &StreamEnvelope<T>) -> Result<(), StreamError> {
        match self {
            Self::Print(stream) => stream.send_envelope(envelope),
            Self::WebSocket(stream) => stream.send_envelope(envelope),
            Self::Sse(stream) => stream.send_envelope(envelope),
        }
    }
}
