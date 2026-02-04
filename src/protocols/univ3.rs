use flashblocks_indexer_streams::{DataStream, StreamOutput};
use flashblocks_types::flashblocks::Flashblock;
use tracing::{debug, error, info};

use super::ProtocolHandler;

/// Handler for Uniswap V3 swap events.
pub struct UniV3Handler;

impl ProtocolHandler for UniV3Handler {
    fn process(&self, fb: &Flashblock, block_number: u64, stream: &StreamOutput) {
        let swaps = fb.extract_swaps();

        if swaps.is_empty() {
            return;
        }

        info!(
            block_number = block_number,
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
                price_0_in_1 = %format!("{:.32}", state.price_0_in_1()),
                liquidity = state.liquidity,
                "Pool state after swap"
            );

            stream.send("UniV3_swap", swap).unwrap_or_else(|e| {
                error!("Failed to send swap to stream: {}", e);
            });
        }
    }
}
