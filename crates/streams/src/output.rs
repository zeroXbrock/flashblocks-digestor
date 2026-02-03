use serde::Serialize;

use crate::{error::StreamError, print::PrintStream, r#trait::DataStream, websocket::WebSocketServer};

/// Enum wrapper for different stream output types
/// Allows runtime selection of stream implementation
pub enum StreamOutput {
    /// Print to stdout
    Print(PrintStream),
    /// WebSocket server broadcasting to connected clients
    WebSocket(WebSocketServer),
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

    /// Start the underlying stream if needed (e.g., WebSocket server)
    /// For PrintStream, this is a no-op
    pub async fn start(&self, addr: &str) -> Result<(), StreamError> {
        match self {
            Self::Print(_) => Ok(()),
            Self::WebSocket(ws) => ws.start(addr).await,
        }
    }
}

impl<T: Serialize> DataStream<T> for StreamOutput {
    fn send_message(&self, data: &T) -> Result<(), StreamError> {
        match self {
            Self::Print(stream) => stream.send_message(data),
            Self::WebSocket(stream) => stream.send_message(data),
        }
    }
}
