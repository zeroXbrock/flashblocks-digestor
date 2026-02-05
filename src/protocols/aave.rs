use flashblocks_indexer_streams::{DataStream, StreamOutput};
use flashblocks_types::flashblocks::Flashblock;
use tracing::{debug, error, info};

use super::ProtocolHandler;

/// Handler for AAVE user action events.
pub struct AaveHandler;

impl ProtocolHandler for AaveHandler {
    fn process(&self, fb: &Flashblock, block_number: u64, stream: &StreamOutput) {
        let updates = fb.extract_aave_updates();

        if updates.is_empty() {
            return;
        }

        info!(
            block_number = block_number,
            supplies = updates.supplies.len(),
            withdraws = updates.withdraws.len(),
            borrows = updates.borrows.len(),
            repays = updates.repays.len(),
            liquidations = updates.liquidations.len(),
            total = updates.total_count(),
            "AAVE user events detected"
        );

        // Stream supply events
        for supply in &updates.supplies {
            debug!(
                pool = %supply.pool,
                reserve = %supply.reserve,
                user = %supply.user,
                on_behalf_of = %supply.on_behalf_of,
                amount = %supply.amount,
                "AAVE Supply"
            );
            stream.send("Aave_supply", supply).unwrap_or_else(|e| {
                error!("Failed to send AAVE supply to stream: {}", e);
            });
        }

        // Stream withdraw events
        for withdraw in &updates.withdraws {
            debug!(
                pool = %withdraw.pool,
                reserve = %withdraw.reserve,
                user = %withdraw.user,
                to = %withdraw.to,
                amount = %withdraw.amount,
                "AAVE Withdraw"
            );
            stream.send("Aave_withdraw", withdraw).unwrap_or_else(|e| {
                error!("Failed to send AAVE withdraw to stream: {}", e);
            });
        }

        // Stream borrow events
        for borrow in &updates.borrows {
            debug!(
                pool = %borrow.pool,
                reserve = %borrow.reserve,
                user = %borrow.user,
                on_behalf_of = %borrow.on_behalf_of,
                amount = %borrow.amount,
                interest_rate_mode = borrow.interest_rate_mode,
                "AAVE Borrow"
            );
            stream.send("Aave_borrow", borrow).unwrap_or_else(|e| {
                error!("Failed to send AAVE borrow to stream: {}", e);
            });
        }

        // Stream repay events
        for repay in &updates.repays {
            debug!(
                pool = %repay.pool,
                reserve = %repay.reserve,
                user = %repay.user,
                repayer = %repay.repayer,
                amount = %repay.amount,
                "AAVE Repay"
            );
            stream.send("Aave_repay", repay).unwrap_or_else(|e| {
                error!("Failed to send AAVE repay to stream: {}", e);
            });
        }

        // Stream liquidation events
        for liquidation in &updates.liquidations {
            debug!(
                pool = %liquidation.pool,
                collateral_asset = %liquidation.collateral_asset,
                debt_asset = %liquidation.debt_asset,
                user = %liquidation.user,
                liquidator = %liquidation.liquidator,
                debt_to_cover = %liquidation.debt_to_cover,
                liquidated_collateral = %liquidation.liquidated_collateral_amount,
                "AAVE Liquidation"
            );
            stream
                .send("Aave_liquidation", liquidation)
                .unwrap_or_else(|e| {
                    error!("Failed to send AAVE liquidation to stream: {}", e);
                });
        }
    }
}
