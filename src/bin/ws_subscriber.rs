use std::env;

use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{error, info, warn};

const DEFAULT_WS_URL: &str = "ws://127.0.0.1:8080";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    // Get WebSocket URL from args or use default
    let ws_url = env::args().nth(1).unwrap_or_else(|| {
        info!("No URL provided, using default: {}", DEFAULT_WS_URL);
        DEFAULT_WS_URL.to_string()
    });

    info!("Connecting to WebSocket server at: {}", ws_url);

    loop {
        match connect_and_subscribe(&ws_url).await {
            Ok(()) => {
                info!("Connection closed normally");
                break;
            }
            Err(e) => {
                error!("Connection error: {}", e);
                info!("Reconnecting in 5 seconds...");
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            }
        }
    }

    Ok(())
}

async fn connect_and_subscribe(url: &str) -> Result<(), Box<dyn std::error::Error>> {
    let (ws_stream, response) = connect_async(url).await?;

    info!(
        "WebSocket connected! Response status: {}",
        response.status()
    );

    let (mut write, mut read) = ws_stream.split();

    // Spawn a task to handle periodic ping to keep connection alive
    let ping_handle = tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));
        loop {
            interval.tick().await;
            if let Err(e) = write.send(Message::Ping(vec![])).await {
                warn!("Failed to send ping: {}", e);
                break;
            }
        }
    });

    // Handle incoming messages
    while let Some(msg) = read.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                // Pretty print JSON if possible
                match serde_json::from_str::<serde_json::Value>(&text) {
                    Ok(json) => {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&json).unwrap_or(text.clone())
                        );
                    }
                    Err(_) => {
                        println!("{}", text);
                    }
                }
            }
            Ok(Message::Binary(data)) => {
                info!("Received binary message: {} bytes", data.len());
            }
            Ok(Message::Ping(data)) => {
                info!("Received ping, pong will be sent automatically");
                // tungstenite handles pong automatically
                drop(data);
            }
            Ok(Message::Pong(_)) => {
                // Expected response to our ping
            }
            Ok(Message::Close(frame)) => {
                info!("Server closed connection: {:?}", frame);
                break;
            }
            Ok(Message::Frame(_)) => {
                // Raw frame, ignore
            }
            Err(e) => {
                error!("Error receiving message: {}", e);
                break;
            }
        }
    }

    ping_handle.abort();
    Ok(())
}
