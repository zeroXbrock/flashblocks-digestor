use clap::{Parser, ValueEnum};

#[derive(Debug, Clone, Copy, ValueEnum, Default)]
pub enum StreamType {
    /// Output to WebSocket server (default)
    #[default]
    Websocket,
    /// Output to Server-Sent Events (SSE) server
    Sse,
    /// Output to stdout
    Print,
}

#[derive(Parser, Debug)]
#[command(name = "flashblocks-digestor")]
#[command(about = "Digest and stream Flashblocks data")]
pub struct Args {
    /// Flashblocks WebSocket URL to connect to
    #[arg(short, long, default_value = "wss://sepolia.flashblocks.base.org/ws")]
    pub url: String,

    /// Stream output type
    #[arg(short, long, value_enum, default_value_t = StreamType::default())]
    pub stream: StreamType,

    /// Server address (used with websocket and sse stream types)
    #[arg(long, default_value = "localhost:9001")]
    pub addr: String,
}
