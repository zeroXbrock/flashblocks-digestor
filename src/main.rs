mod utils;

use std::sync::{Arc, atomic::AtomicBool};

use flashblocks_types::flashblocks::Flashblock;
use futures_util::StreamExt;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::protocol::Message;
use tracing::{debug, error, info, warn};
use utils::decompress_brotli;

#[tokio::main]
async fn main() {
    // Load .env file if present (ignore errors if not found)
    dotenvy::dotenv().ok();

    // Initialize tracing with timestamps
    tracing_subscriber::fmt()
        .with_target(false)
        .with_timer(tracing_subscriber::fmt::time::LocalTime::rfc_3339())
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    // Choose which network to connect to:
    // - default: mainnet
    // - override with `FLASHBLOCKS_WS_URL`
    let url = std::env::var("FLASHBLOCKS_WS_URL").expect("missing env var: FLASHBLOCKS_WS_URL");

    info!("Connecting to Flashblocks WebSocket: {url}");

    let (ws_stream, _) = match connect_async(&url).await {
        Ok(res) => res,
        Err(e) => {
            error!("WebSocket connection error: {e}");
            return;
        }
    };

    info!("Connected. Streaming Flashblocks… (Ctrl-C to exit)");

    let (_, mut read) = ws_stream.split();
    let done = Arc::new(AtomicBool::new(false));
    let done_clone = done.clone();

    tokio::task::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to listen for Ctrl-C");
        warn!("Ctrl-C received, shutting down.");
        done_clone.store(true, std::sync::atomic::Ordering::SeqCst);
    });

    while let Some(msg_result) = read.next().await {
        if done.load(std::sync::atomic::Ordering::SeqCst) {
            info!("Shutting down stream reader.");
            break;
        }
        match msg_result {
            Ok(Message::Text(text)) => {
                handle_message(&text);
            }
            Ok(Message::Binary(bin)) => {
                // Binary frames are Brotli-compressed JSON
                match decompress_brotli(&bin) {
                    Ok(text) => handle_message(&text),
                    Err(e) => {
                        warn!("Failed to decompress binary frame: {e}");
                        info!("Binary frame ({} bytes)", bin.len());
                    }
                }
            }
            Ok(Message::Ping(_)) | Ok(Message::Pong(_)) | Ok(Message::Frame(_)) => {
                // Ignore; tungstenite handles pings/pongs at the protocol level.
                info!("Received control frame");
            }
            Ok(Message::Close(frame)) => {
                info!("WebSocket closed: {frame:?}");
                break;
            }
            Err(e) => {
                error!("WebSocket read error: {e}");
                break;
            }
        }
    }

    info!("Stream ended");
}

fn handle_message(text: &str) {
    // First try to parse into our minimal Flashblock struct.
    match serde_json::from_str::<Flashblock>(text) {
        Ok(fb) => {
            let block_number = fb.metadata.as_ref().map(|m| m.block_number);

            match block_number {
                Some(num) => {
                    // Debug: log flashblock metadata
                    if let Some(ref meta) = fb.metadata {
                        let total_logs: usize =
                            meta.receipts.values().map(|r| r.logs().len()).sum();
                        debug!(
                            payload_id = %fb.payload_id,
                            index = fb.index,
                            block_number = num,
                            receipts = meta.receipts.len(),
                            total_logs = total_logs,
                            "Flashblock"
                        );
                    }

                    // Extract and log swap events
                    let swaps = fb.extract_swaps();
                    if !swaps.is_empty() {
                        info!(
                            block_number = num,
                            count = swaps.len(),
                            "UniV3 Swaps detected"
                        );
                        for swap in &swaps {
                            debug!(
                                pool = %swap.pool,
                                sender = %swap.sender,
                                recipient = %swap.recipient,
                                amount0 = %swap.amount0,
                                amount1 = %swap.amount1,
                                sqrt_price_x96 = %swap.sqrt_price_x96,
                                liquidity = swap.liquidity,
                                tick = swap.tick,
                                "Swap"
                            );
                            let state = swap.pool_state();
                            info!(
                                pool = %swap.pool,
                                tick = state.tick,
                                price_0_in_1 = state.price_0_in_1(),
                                liquidity = state.liquidity,
                                "Pool state after swap"
                            );
                        }
                    }
                }
                None => {
                    debug!(
                        payload_id = %fb.payload_id,
                        index = fb.index,
                        "Flashblock (no block_number)"
                    );
                }
            }
        }
        Err(e) => {
            // If the schema changes or we get some other message type, dump the raw JSON.
            warn!(error = %e, "Failed to parse Flashblock JSON");
            debug!("Raw message: {text}");
        }
    }
}
