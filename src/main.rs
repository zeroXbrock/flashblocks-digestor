mod args;
mod protocols;
mod utils;

use std::sync::{Arc, atomic::AtomicBool};

use clap::Parser;
use flashblocks_types::flashblocks::Flashblock;
use futures_util::StreamExt;
use protocols::process_all_protocols;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::protocol::Message;
use tracing::{debug, error, info, warn};
use utils::decompress_brotli;

use crate::args::{Args, StreamType};
use flashblocks_indexer_streams::StreamOutput;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse CLI arguments
    let args = Args::parse();

    // Initialize tracing with timestamps (fixed-width format for aligned logs)
    tracing_subscriber::fmt()
        .with_target(false)
        .with_timer(tracing_subscriber::fmt::time::LocalTime::new(
            time::macros::format_description!(
                "[year]-[month]-[day]T[hour]:[minute]:[second].[subsecond digits:3]"
            ),
        ))
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    // Create stream output based on CLI argument
    let stream_output = match args.stream {
        StreamType::Websocket => {
            info!("Using WebSocket stream output");
            StreamOutput::websocket()
        }
        StreamType::Sse => {
            info!("Using SSE stream output");
            StreamOutput::sse()
        }
        StreamType::Print => {
            info!("Using Print stream output");
            StreamOutput::print()
        }
    };

    // Start the stream output (starts WebSocket/SSE server if applicable)
    stream_output.start(&args.addr).await?;

    let url = &args.url;

    info!("Connecting to Flashblocks WebSocket: {url}");
    let (ws_stream, _) = connect_async(url).await?;
    info!("Connected. Streaming Flashblocks… (Ctrl-C to exit)");

    let (_, mut read) = ws_stream.split();
    let done = Arc::new(AtomicBool::new(false));
    let done_clone = done.clone();

    // listen for CTRL-C in the background to shutdown gracefully
    tokio::task::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to listen for Ctrl-C");
        warn!("Ctrl-C received, shutting down.");
        done_clone.store(true, std::sync::atomic::Ordering::SeqCst);
    });

    info!("parsing flashblocks...");
    while let Some(msg_result) = read.next().await {
        if done.load(std::sync::atomic::Ordering::SeqCst) {
            info!("Shutting down stream reader.");
            break;
        }
        match msg_result {
            Ok(Message::Text(text)) => {
                handle_message(&text, &stream_output);
            }
            Ok(Message::Binary(bin)) => {
                // Binary frames are Brotli-compressed JSON
                match decompress_brotli(&bin) {
                    Ok(text) => handle_message(&text, &stream_output),
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
    Ok(())
}

fn handle_message(text: &str, stream: &StreamOutput) {
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

                    // Process all protocols in parallel
                    process_all_protocols(&fb, num, stream);
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
