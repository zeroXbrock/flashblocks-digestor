use flashblocks_indexer_streams::{DataStream, StreamOutput};
use flashblocks_types::flashblocks::Flashblock;
use tracing::{debug, error, info};

use super::ProtocolHandler;

/// Handler for Morpho Blue lending protocol events.
pub struct MorphoHandler;

impl ProtocolHandler for MorphoHandler {
    fn process(&self, fb: &Flashblock, block_number: u64, stream: &StreamOutput) {
        let updates = fb.extract_morpho_updates();

        if updates.is_empty() {
            return;
        }

        info!(
            block_number = block_number,
            supplies = updates.supplies.len(),
            withdraws = updates.withdraws.len(),
            borrows = updates.borrows.len(),
            repays = updates.repays.len(),
            supply_collaterals = updates.supply_collaterals.len(),
            withdraw_collaterals = updates.withdraw_collaterals.len(),
            liquidations = updates.liquidations.len(),
            create_markets = updates.create_markets.len(),
            total = updates.total_count(),
            "Morpho events detected"
        );

        // Stream supply events
        for supply in &updates.supplies {
            debug!(
                morpho = %supply.morpho,
                market_id = %supply.market_id,
                caller = %supply.caller,
                on_behalf_of = %supply.on_behalf_of,
                assets = %supply.assets,
                shares = %supply.shares,
                "Morpho Supply"
            );
            stream.send("Morpho_supply", supply).unwrap_or_else(|e| {
                error!("Failed to send Morpho supply to stream: {}", e);
            });
        }

        // Stream withdraw events
        for withdraw in &updates.withdraws {
            debug!(
                morpho = %withdraw.morpho,
                market_id = %withdraw.market_id,
                caller = %withdraw.caller,
                on_behalf_of = %withdraw.on_behalf_of,
                receiver = %withdraw.receiver,
                assets = %withdraw.assets,
                shares = %withdraw.shares,
                "Morpho Withdraw"
            );
            stream.send("Morpho_withdraw", withdraw).unwrap_or_else(|e| {
                error!("Failed to send Morpho withdraw to stream: {}", e);
            });
        }

        // Stream borrow events
        for borrow in &updates.borrows {
            debug!(
                morpho = %borrow.morpho,
                market_id = %borrow.market_id,
                caller = %borrow.caller,
                on_behalf_of = %borrow.on_behalf_of,
                receiver = %borrow.receiver,
                assets = %borrow.assets,
                shares = %borrow.shares,
                "Morpho Borrow"
            );
            stream.send("Morpho_borrow", borrow).unwrap_or_else(|e| {
                error!("Failed to send Morpho borrow to stream: {}", e);
            });
        }

        // Stream repay events
        for repay in &updates.repays {
            debug!(
                morpho = %repay.morpho,
                market_id = %repay.market_id,
                caller = %repay.caller,
                on_behalf_of = %repay.on_behalf_of,
                assets = %repay.assets,
                shares = %repay.shares,
                "Morpho Repay"
            );
            stream.send("Morpho_repay", repay).unwrap_or_else(|e| {
                error!("Failed to send Morpho repay to stream: {}", e);
            });
        }

        // Stream supply collateral events
        for supply_collateral in &updates.supply_collaterals {
            debug!(
                morpho = %supply_collateral.morpho,
                market_id = %supply_collateral.market_id,
                caller = %supply_collateral.caller,
                on_behalf_of = %supply_collateral.on_behalf_of,
                assets = %supply_collateral.assets,
                "Morpho SupplyCollateral"
            );
            stream
                .send("Morpho_supply_collateral", supply_collateral)
                .unwrap_or_else(|e| {
                    error!("Failed to send Morpho supply collateral to stream: {}", e);
                });
        }

        // Stream withdraw collateral events
        for withdraw_collateral in &updates.withdraw_collaterals {
            debug!(
                morpho = %withdraw_collateral.morpho,
                market_id = %withdraw_collateral.market_id,
                caller = %withdraw_collateral.caller,
                on_behalf_of = %withdraw_collateral.on_behalf_of,
                receiver = %withdraw_collateral.receiver,
                assets = %withdraw_collateral.assets,
                "Morpho WithdrawCollateral"
            );
            stream
                .send("Morpho_withdraw_collateral", withdraw_collateral)
                .unwrap_or_else(|e| {
                    error!("Failed to send Morpho withdraw collateral to stream: {}", e);
                });
        }

        // Stream liquidation events
        for liquidation in &updates.liquidations {
            debug!(
                morpho = %liquidation.morpho,
                market_id = %liquidation.market_id,
                caller = %liquidation.caller,
                borrower = %liquidation.borrower,
                repaid_assets = %liquidation.repaid_assets,
                repaid_shares = %liquidation.repaid_shares,
                seized_assets = %liquidation.seized_assets,
                bad_debt_assets = %liquidation.bad_debt_assets,
                bad_debt_shares = %liquidation.bad_debt_shares,
                "Morpho Liquidation"
            );
            stream
                .send("Morpho_liquidation", liquidation)
                .unwrap_or_else(|e| {
                    error!("Failed to send Morpho liquidation to stream: {}", e);
                });
        }

        // Stream create market events
        for create_market in &updates.create_markets {
            debug!(
                morpho = %create_market.morpho,
                market_id = %create_market.market_id,
                loan_token = %create_market.loan_token,
                collateral_token = %create_market.collateral_token,
                oracle = %create_market.oracle,
                irm = %create_market.irm,
                lltv = %create_market.lltv,
                "Morpho CreateMarket"
            );
            stream
                .send("Morpho_create_market", create_market)
                .unwrap_or_else(|e| {
                    error!("Failed to send Morpho create market to stream: {}", e);
                });
        }
    }
}
