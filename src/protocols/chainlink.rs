use flashblocks_indexer_streams::{DataStream, StreamOutput};
use flashblocks_types::flashblocks::Flashblock;
use tracing::{debug, error, info};

use super::ProtocolHandler;

/// Handler for Chainlink oracle price updates.
pub struct ChainlinkHandler;

impl ProtocolHandler for ChainlinkHandler {
    fn process(&self, fb: &Flashblock, block_number: u64, stream: &StreamOutput) {
        let oracle_updates = fb.extract_answer_updates();

        if oracle_updates.is_empty() {
            return;
        }

        info!(
            block_number = block_number,
            count = oracle_updates.len(),
            "Chainlink AnswerUpdated events detected"
        );

        for update in &oracle_updates {
            debug!(
                feed = %update.feed,
                answer = %update.answer,
                round_id = %update.round_id,
                updated_at = %update.updated_at,
                "AnswerUpdated"
            );

            stream
                .send("Chainlink_answer_updated", update)
                .unwrap_or_else(|e| {
                    error!("Failed to send oracle update to stream: {}", e);
                });
        }
    }
}
