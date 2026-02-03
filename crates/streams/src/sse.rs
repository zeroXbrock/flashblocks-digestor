use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

use http_body_util::{BodyExt, StreamBody};
use hyper::{
    Request, Response,
    body::{Bytes, Frame},
    server::conn::http1,
    service::service_fn,
};
use hyper_util::rt::TokioIo;
use serde::Serialize;
use tokio::{
    net::TcpListener,
    sync::{RwLock, broadcast},
};
use tokio_stream::StreamExt;
use tokio_stream::wrappers::BroadcastStream;
use tracing::{error, info, warn};

use crate::{envelope::StreamEnvelope, error::StreamError, r#trait::DataStream};

type ClientId = u64;
type BoxBody = http_body_util::combinators::BoxBody<Bytes, std::io::Error>;

/// A Server-Sent Events (SSE) server that broadcasts messages to all connected clients
pub struct SseServer {
    /// Broadcast channel sender for distributing messages to all clients
    broadcast_tx: broadcast::Sender<String>,
    /// Connected clients map (client_id -> address)
    clients: Arc<RwLock<HashMap<ClientId, SocketAddr>>>,
    /// Counter for generating unique client IDs
    next_client_id: Arc<AtomicU64>,
}

impl SseServer {
    /// Creates a new SSE server with the specified broadcast channel capacity
    pub fn new(channel_capacity: usize) -> Self {
        let (broadcast_tx, _) = broadcast::channel(channel_capacity);
        Self {
            broadcast_tx,
            clients: Arc::new(RwLock::new(HashMap::new())),
            next_client_id: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Creates a new SSE server with default channel capacity (100)
    pub fn with_default_capacity() -> Self {
        Self::new(100)
    }

    /// Starts the SSE server on the specified address in the background.
    /// Returns immediately after binding to the address.
    /// The server runs in a spawned task until the process exits.
    pub async fn start(&self, addr: &str) -> Result<(), StreamError> {
        let listener = TcpListener::bind(addr).await.map_err(|e| {
            StreamError::SendError(format!("Failed to bind to address {}: {}", addr, e))
        })?;

        info!("SSE server listening on: http://{}", addr);

        let broadcast_tx = self.broadcast_tx.clone();
        let clients = self.clients.clone();
        let next_client_id = self.next_client_id.clone();

        // Spawn the accept loop in the background
        tokio::spawn(async move {
            loop {
                match listener.accept().await {
                    Ok((stream, addr)) => {
                        let client_id = next_client_id.fetch_add(1, Ordering::SeqCst);
                        let broadcast_tx = broadcast_tx.clone();
                        let clients = clients.clone();

                        tokio::spawn(async move {
                            let io = TokioIo::new(stream);

                            let service = service_fn(move |req| {
                                let broadcast_tx = broadcast_tx.clone();
                                let clients = clients.clone();
                                handle_request(req, addr, client_id, broadcast_tx, clients)
                            });

                            if let Err(e) =
                                http1::Builder::new().serve_connection(io, service).await
                            {
                                if !e.is_incomplete_message() {
                                    error!("Error serving connection from {}: {}", addr, e);
                                }
                            }
                        });
                    }
                    Err(e) => {
                        error!("Failed to accept connection: {}", e);
                    }
                }
            }
        });

        Ok(())
    }

    /// Returns the number of currently connected clients
    pub async fn client_count(&self) -> usize {
        self.clients.read().await.len()
    }

    /// Returns a clone of the broadcast sender for external use
    pub fn get_broadcast_sender(&self) -> broadcast::Sender<String> {
        self.broadcast_tx.clone()
    }
}

impl DataStream for SseServer {
    fn send<T: Serialize>(&self, data_type: &str, data: &T) -> Result<(), StreamError> {
        let envelope = StreamEnvelope::new(data_type, data);
        self.send_envelope(&envelope)
    }

    fn send_envelope<T: Serialize>(&self, envelope: &StreamEnvelope<T>) -> Result<(), StreamError> {
        let json = serde_json::to_string(envelope)
            .map_err(|e| StreamError::SendError(format!("Failed to serialize data: {}", e)))?;

        // Send to all connected clients via broadcast channel
        match self.broadcast_tx.send(json) {
            Ok(receiver_count) => {
                if receiver_count == 0 {
                    tracing::debug!("No clients connected to receive message");
                }
                Ok(())
            }
            Err(e) => {
                tracing::debug!("No active receivers: {}", e);
                Ok(())
            }
        }
    }
}

async fn handle_request(
    req: Request<hyper::body::Incoming>,
    addr: SocketAddr,
    client_id: ClientId,
    broadcast_tx: broadcast::Sender<String>,
    clients: Arc<RwLock<HashMap<ClientId, SocketAddr>>>,
) -> Result<Response<BoxBody>, std::io::Error> {
    // Only handle GET requests to /events or /
    let path = req.uri().path();
    if path != "/events" && path != "/" {
        let response = Response::builder().status(404).body(empty_body()).unwrap();
        return Ok(response);
    }

    // Add client to the map
    {
        let mut clients_guard = clients.write().await;
        clients_guard.insert(client_id, addr);
    }

    info!(
        "New SSE connection from: {} (client_id: {})",
        addr, client_id
    );

    let broadcast_rx = broadcast_tx.subscribe();

    // Convert broadcast receiver to a stream of SSE events
    let stream = BroadcastStream::new(broadcast_rx).filter_map(move |result| {
        match result {
            Ok(data) => {
                // Format as SSE event: "data: <json>\n\n"
                let event = format!("data: {}\n\n", data);
                Some(Ok(Frame::data(Bytes::from(event))))
            }
            Err(tokio_stream::wrappers::errors::BroadcastStreamRecvError::Lagged(count)) => {
                warn!("Client {} lagged behind by {} messages", client_id, count);
                None
            }
        }
    });

    // Create response with SSE headers
    let body = StreamBody::new(stream);
    let boxed_body = BoxBody::new(body);

    let response = Response::builder()
        .status(200)
        .header("Content-Type", "text/event-stream")
        .header("Cache-Control", "no-cache")
        .header("Connection", "keep-alive")
        .header("Access-Control-Allow-Origin", "*")
        .body(boxed_body)
        .unwrap();

    // Note: Client removal happens when the connection is dropped
    // We spawn a task to clean up when the broadcast receiver is dropped
    let clients_cleanup = clients.clone();
    tokio::spawn(async move {
        // This task will complete when the main connection handling is done
        // Wait a bit for the connection to fully close
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        let mut clients_guard = clients_cleanup.write().await;
        if clients_guard.remove(&client_id).is_some() {
            info!(
                "Client {} (id: {}) removed from active clients",
                addr, client_id
            );
        }
    });

    Ok(response)
}

fn empty_body() -> BoxBody {
    use http_body_util::Empty;
    BoxBody::new(Empty::new().map_err(|_| std::io::Error::other("empty body error")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sse_server_creation() {
        let server = SseServer::new(50);
        assert_eq!(server.next_client_id.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn test_send_message_no_clients() {
        let server = SseServer::with_default_capacity();

        #[derive(Serialize)]
        struct TestData {
            message: String,
        }

        let data = TestData {
            message: "Hello".to_string(),
        };

        // Should not error even with no clients
        let result = server.send_auto(&data);
        assert!(result.is_ok());
    }
}
