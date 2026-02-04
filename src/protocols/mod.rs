//! Protocol handlers for extracting and streaming events from flashblocks.
//!
//! To add a new protocol:
//! 1. Create a new module (e.g., `my_protocol.rs`)
//! 2. Implement the `ProtocolHandler` trait
//! 3. Add the handler to the `ALL_HANDLERS` array in this file

mod chainlink;
mod univ3;

use flashblocks_indexer_streams::StreamOutput;
use flashblocks_types::flashblocks::Flashblock;

pub use chainlink::ChainlinkHandler;
pub use univ3::UniV3Handler;

/// Trait for protocol-specific event extraction and streaming.
///
/// Each protocol handler is responsible for:
/// - Extracting relevant events from a flashblock
/// - Logging the events
/// - Sending them to the data stream
pub trait ProtocolHandler: Send + Sync {
    /// Process a flashblock and send any extracted events to the stream.
    ///
    /// This method should:
    /// 1. Extract protocol-specific events from the flashblock
    /// 2. Log relevant information
    /// 3. Send each event to the stream
    fn process(&self, fb: &Flashblock, block_number: u64, stream: &StreamOutput);
}

/// All registered protocol handlers.
/// Add new handlers here to include them in parallel processing.
pub static ALL_HANDLERS: &[&dyn ProtocolHandler] = &[&UniV3Handler, &ChainlinkHandler];

/// Process a flashblock through all protocol handlers in parallel.
pub fn process_all_protocols(fb: &Flashblock, block_number: u64, stream: &StreamOutput) {
    rayon::scope(|s| {
        for handler in ALL_HANDLERS {
            s.spawn(|_| {
                handler.process(fb, block_number, stream);
            });
        }
    });
}
