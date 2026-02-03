use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

use futures_util::{SinkExt, StreamExt};
use serde::Serialize;
use tokio::{
    net::{TcpListener, TcpStream},
    sync::{RwLock, broadcast},
};
use tokio_tungstenite::{accept_async, tungstenite::Message};
use tracing::{error, info, warn};

use crate::{envelope::StreamEnvelope, error::StreamError, r#trait::DataStream};

type ClientId = u64;

/// A WebSocket server that broadcasts messages to all connected clients
pub struct WebSocketServer {
    /// Broadcast channel sender for distributing messages to all clients
    broadcast_tx: broadcast::Sender<String>,
    /// Connected clients map (client_id -> address)
    clients: Arc<RwLock<HashMap<ClientId, SocketAddr>>>,
    /// Counter for generating unique client IDs
    next_client_id: Arc<AtomicU64>,
}

impl WebSocketServer {
    /// Creates a new WebSocket server with the specified broadcast channel capacity
    pub fn new(channel_capacity: usize) -> Self {
        let (broadcast_tx, _) = broadcast::channel(channel_capacity);
        Self {
            broadcast_tx,
            clients: Arc::new(RwLock::new(HashMap::new())),
            next_client_id: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Creates a new WebSocket server with default channel capacity (100)
    pub fn with_default_capacity() -> Self {
        Self::new(100)
    }

    /// Starts the WebSocket server on the specified address in the background.
    /// Returns immediately after binding to the address.
    /// The server runs in a spawned task until the process exits.
    pub async fn start(&self, addr: &str) -> Result<(), StreamError> {
        let listener = TcpListener::bind(addr).await.map_err(|e| {
            StreamError::SendError(format!("Failed to bind to address {}: {}", addr, e))
        })?;

        info!("WebSocket server listening on: {}", addr);

        let broadcast_tx = self.broadcast_tx.clone();
        let clients = self.clients.clone();
        let next_client_id = self.next_client_id.clone();

        // Spawn the accept loop in the background
        tokio::spawn(async move {
            loop {
                match listener.accept().await {
                    Ok((stream, addr)) => {
                        let client_id = next_client_id.fetch_add(1, Ordering::SeqCst);
                        let broadcast_rx = broadcast_tx.subscribe();
                        let clients_clone = clients.clone();

                        // Add client to the map
                        {
                            let mut clients_guard = clients.write().await;
                            clients_guard.insert(client_id, addr);
                        }

                        info!(
                            "New WebSocket connection from: {} (client_id: {})",
                            addr, client_id
                        );

                        tokio::spawn(async move {
                            if let Err(e) = handle_connection(
                                stream,
                                addr,
                                client_id,
                                broadcast_rx,
                                clients_clone,
                            )
                            .await
                            {
                                error!("Error handling connection from {}: {}", addr, e);
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

impl DataStream for WebSocketServer {
    fn send<T: Serialize>(&self, data_type: &str, data: &T) -> Result<(), StreamError> {
        let envelope = StreamEnvelope::new(data_type, data);
        self.send_envelope(&envelope)
    }

    fn send_envelope<T: Serialize>(&self, envelope: &StreamEnvelope<T>) -> Result<(), StreamError> {
        let json = serde_json::to_string(envelope)
            .map_err(|e| StreamError::SendError(format!("Failed to serialize data: {}", e)))?;

        // Send to all connected clients via broadcast channel
        // Note: send() returns an error if there are no receivers, which is fine
        match self.broadcast_tx.send(json) {
            Ok(receiver_count) => {
                if receiver_count == 0 {
                    // No clients connected, but this is not an error
                    tracing::debug!("No clients connected to receive message");
                }
                Ok(())
            }
            Err(e) => {
                // This happens when there are no receivers
                tracing::debug!("No active receivers: {}", e);
                Ok(())
            }
        }
    }
}

/// Handles an individual WebSocket connection
async fn handle_connection(
    stream: TcpStream,
    addr: SocketAddr,
    client_id: ClientId,
    mut broadcast_rx: broadcast::Receiver<String>,
    clients: Arc<RwLock<HashMap<ClientId, SocketAddr>>>,
) -> Result<(), StreamError> {
    let ws_stream = accept_async(stream)
        .await
        .map_err(|e| StreamError::SendError(format!("WebSocket handshake failed: {}", e)))?;

    let (mut write, mut read) = ws_stream.split();

    loop {
        tokio::select! {
            // Handle incoming messages from the client
            msg = read.next() => {
                match msg {
                    Some(Ok(Message::Close(_))) | None => {
                        info!("Client {} disconnected", addr);
                        break;
                    }
                    Some(Ok(Message::Ping(data))) => {
                        if let Err(e) = write.send(Message::Pong(data)).await {
                            warn!("Failed to send pong to {}: {}", addr, e);
                            break;
                        }
                    }
                    Some(Ok(Message::Text(text))) => {
                        // Log received messages (could be used for subscriptions in the future)
                        tracing::debug!("Received from {}: {}", addr, text);
                    }
                    Some(Ok(_)) => {
                        // Ignore other message types (Binary, Pong, Frame)
                    }
                    Some(Err(e)) => {
                        warn!("Error receiving from {}: {}", addr, e);
                        break;
                    }
                }
            }
            // Handle broadcast messages to send to this client
            result = broadcast_rx.recv() => {
                match result {
                    Ok(msg) => {
                        if let Err(e) = write.send(Message::Text(msg)).await {
                            warn!("Failed to send message to {}: {}", addr, e);
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(count)) => {
                        warn!("Client {} lagged behind by {} messages", addr, count);
                        // Continue receiving, client will miss some messages
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        info!("Broadcast channel closed, disconnecting {}", addr);
                        break;
                    }
                }
            }
        }
    }

    // Remove client from the map
    {
        let mut clients_guard = clients.write().await;
        clients_guard.remove(&client_id);
    }

    info!(
        "Client {} (id: {}) removed from active clients",
        addr, client_id
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_websocket_server_creation() {
        let server = WebSocketServer::new(50);
        assert_eq!(server.next_client_id.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn test_send_message_no_clients() {
        let server = WebSocketServer::with_default_capacity();

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
