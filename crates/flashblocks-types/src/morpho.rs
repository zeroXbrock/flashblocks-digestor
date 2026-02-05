use alloy_primitives::{Address, B256, Bloom, BloomInput, U256};
use alloy_sol_types::{SolEvent, sol};
use serde::Serialize;

use crate::flashblocks::ReceiptLog;

// Morpho Blue events
sol! {
    /// Emitted when a user supplies assets to a market
    event Supply(
        bytes32 indexed id,
        address indexed caller,
        address indexed onBehalf,
        uint256 assets,
        uint256 shares
    );

    /// Emitted when a user withdraws assets from a market
    event Withdraw(
        bytes32 indexed id,
        address caller,
        address indexed onBehalf,
        address indexed receiver,
        uint256 assets,
        uint256 shares
    );

    /// Emitted when a user borrows assets from a market
    event Borrow(
        bytes32 indexed id,
        address caller,
        address indexed onBehalf,
        address indexed receiver,
        uint256 assets,
        uint256 shares
    );

    /// Emitted when a user repays borrowed assets
    event Repay(
        bytes32 indexed id,
        address indexed caller,
        address indexed onBehalf,
        uint256 assets,
        uint256 shares
    );

    /// Emitted when a user supplies collateral to a market
    event SupplyCollateral(
        bytes32 indexed id,
        address indexed caller,
        address indexed onBehalf,
        uint256 assets
    );

    /// Emitted when a user withdraws collateral from a market
    event WithdrawCollateral(
        bytes32 indexed id,
        address caller,
        address indexed onBehalf,
        address indexed receiver,
        uint256 assets
    );

    /// Emitted when a liquidation occurs
    event Liquidate(
        bytes32 indexed id,
        address indexed caller,
        address indexed borrower,
        uint256 repaidAssets,
        uint256 repaidShares,
        uint256 seizedAssets,
        uint256 badDebtAssets,
        uint256 badDebtShares
    );

    /// Emitted when a new market is created
    event CreateMarket(
        bytes32 indexed id,
        address loanToken,
        address collateralToken,
        address oracle,
        address irm,
        uint256 lltv
    );
}

/// Detected Morpho events based on bloom filter
#[derive(Debug, Default)]
pub struct MorphoEvents {
    pub may_have_supply: bool,
    pub may_have_withdraw: bool,
    pub may_have_borrow: bool,
    pub may_have_repay: bool,
    pub may_have_supply_collateral: bool,
    pub may_have_withdraw_collateral: bool,
    pub may_have_liquidation: bool,
    pub may_have_create_market: bool,
}

impl MorphoEvents {
    /// Check the bloom filter for potential Morpho events.
    /// Note: Bloom filters can have false positives but no false negatives.
    pub fn from_bloom(bloom: &Bloom) -> Self {
        Self {
            may_have_supply: bloom.contains_input(BloomInput::Hash(Supply::SIGNATURE_HASH)),
            may_have_withdraw: bloom.contains_input(BloomInput::Hash(Withdraw::SIGNATURE_HASH)),
            may_have_borrow: bloom.contains_input(BloomInput::Hash(Borrow::SIGNATURE_HASH)),
            may_have_repay: bloom.contains_input(BloomInput::Hash(Repay::SIGNATURE_HASH)),
            may_have_supply_collateral: bloom
                .contains_input(BloomInput::Hash(SupplyCollateral::SIGNATURE_HASH)),
            may_have_withdraw_collateral: bloom
                .contains_input(BloomInput::Hash(WithdrawCollateral::SIGNATURE_HASH)),
            may_have_liquidation: bloom.contains_input(BloomInput::Hash(Liquidate::SIGNATURE_HASH)),
            may_have_create_market: bloom
                .contains_input(BloomInput::Hash(CreateMarket::SIGNATURE_HASH)),
        }
    }

    /// Returns true if any Morpho event might be present
    pub fn any(&self) -> bool {
        self.may_have_supply
            || self.may_have_withdraw
            || self.may_have_borrow
            || self.may_have_repay
            || self.may_have_supply_collateral
            || self.may_have_withdraw_collateral
            || self.may_have_liquidation
            || self.may_have_create_market
    }
}

/// Parsed Morpho Supply event
#[derive(Debug, Clone, Serialize)]
pub struct ParsedMorphoSupply {
    /// Address of the Morpho contract
    pub morpho: Address,
    /// Market identifier
    pub market_id: B256,
    /// The caller initiating the supply
    pub caller: Address,
    /// The beneficiary of the supply
    pub on_behalf_of: Address,
    /// Amount of assets supplied
    pub assets: U256,
    /// Amount of shares minted
    pub shares: U256,
}

impl ParsedMorphoSupply {
    /// Try to parse a Supply event from a log entry
    pub fn try_from_log(log: &ReceiptLog) -> Option<Self> {
        if log.topics.len() != 4 {
            return None;
        }

        if log.topics[0] != Supply::SIGNATURE_HASH {
            return None;
        }

        let decoded = Supply::decode_raw_log(log.topics.iter().copied(), &log.data, true).ok()?;

        Some(Self {
            morpho: log.address,
            market_id: B256::from(decoded.id),
            caller: decoded.caller,
            on_behalf_of: decoded.onBehalf,
            assets: decoded.assets,
            shares: decoded.shares,
        })
    }

    /// Extract all Supply events from a slice of logs
    pub fn extract_all(logs: &[ReceiptLog]) -> Vec<Self> {
        logs.iter().filter_map(Self::try_from_log).collect()
    }
}

/// Parsed Morpho Withdraw event
#[derive(Debug, Clone, Serialize)]
pub struct ParsedMorphoWithdraw {
    /// Address of the Morpho contract
    pub morpho: Address,
    /// Market identifier
    pub market_id: B256,
    /// The caller initiating the withdrawal
    pub caller: Address,
    /// The owner of the position
    pub on_behalf_of: Address,
    /// The receiver of the assets
    pub receiver: Address,
    /// Amount of assets withdrawn
    pub assets: U256,
    /// Amount of shares burned
    pub shares: U256,
}

impl ParsedMorphoWithdraw {
    /// Try to parse a Withdraw event from a log entry
    pub fn try_from_log(log: &ReceiptLog) -> Option<Self> {
        if log.topics.len() != 4 {
            return None;
        }

        if log.topics[0] != Withdraw::SIGNATURE_HASH {
            return None;
        }

        let decoded = Withdraw::decode_raw_log(log.topics.iter().copied(), &log.data, true).ok()?;

        Some(Self {
            morpho: log.address,
            market_id: B256::from(decoded.id),
            caller: decoded.caller,
            on_behalf_of: decoded.onBehalf,
            receiver: decoded.receiver,
            assets: decoded.assets,
            shares: decoded.shares,
        })
    }

    /// Extract all Withdraw events from a slice of logs
    pub fn extract_all(logs: &[ReceiptLog]) -> Vec<Self> {
        logs.iter().filter_map(Self::try_from_log).collect()
    }
}

/// Parsed Morpho Borrow event
#[derive(Debug, Clone, Serialize)]
pub struct ParsedMorphoBorrow {
    /// Address of the Morpho contract
    pub morpho: Address,
    /// Market identifier
    pub market_id: B256,
    /// The caller initiating the borrow
    pub caller: Address,
    /// The owner of the position
    pub on_behalf_of: Address,
    /// The receiver of the borrowed assets
    pub receiver: Address,
    /// Amount of assets borrowed
    pub assets: U256,
    /// Amount of shares minted (debt)
    pub shares: U256,
}

impl ParsedMorphoBorrow {
    /// Try to parse a Borrow event from a log entry
    pub fn try_from_log(log: &ReceiptLog) -> Option<Self> {
        if log.topics.len() != 4 {
            return None;
        }

        if log.topics[0] != Borrow::SIGNATURE_HASH {
            return None;
        }

        let decoded = Borrow::decode_raw_log(log.topics.iter().copied(), &log.data, true).ok()?;

        Some(Self {
            morpho: log.address,
            market_id: B256::from(decoded.id),
            caller: decoded.caller,
            on_behalf_of: decoded.onBehalf,
            receiver: decoded.receiver,
            assets: decoded.assets,
            shares: decoded.shares,
        })
    }

    /// Extract all Borrow events from a slice of logs
    pub fn extract_all(logs: &[ReceiptLog]) -> Vec<Self> {
        logs.iter().filter_map(Self::try_from_log).collect()
    }
}

/// Parsed Morpho Repay event
#[derive(Debug, Clone, Serialize)]
pub struct ParsedMorphoRepay {
    /// Address of the Morpho contract
    pub morpho: Address,
    /// Market identifier
    pub market_id: B256,
    /// The caller initiating the repayment
    pub caller: Address,
    /// The owner of the position being repaid
    pub on_behalf_of: Address,
    /// Amount of assets repaid
    pub assets: U256,
    /// Amount of shares burned (debt reduced)
    pub shares: U256,
}

impl ParsedMorphoRepay {
    /// Try to parse a Repay event from a log entry
    pub fn try_from_log(log: &ReceiptLog) -> Option<Self> {
        if log.topics.len() != 4 {
            return None;
        }

        if log.topics[0] != Repay::SIGNATURE_HASH {
            return None;
        }

        let decoded = Repay::decode_raw_log(log.topics.iter().copied(), &log.data, true).ok()?;

        Some(Self {
            morpho: log.address,
            market_id: B256::from(decoded.id),
            caller: decoded.caller,
            on_behalf_of: decoded.onBehalf,
            assets: decoded.assets,
            shares: decoded.shares,
        })
    }

    /// Extract all Repay events from a slice of logs
    pub fn extract_all(logs: &[ReceiptLog]) -> Vec<Self> {
        logs.iter().filter_map(Self::try_from_log).collect()
    }
}

/// Parsed Morpho SupplyCollateral event
#[derive(Debug, Clone, Serialize)]
pub struct ParsedMorphoSupplyCollateral {
    /// Address of the Morpho contract
    pub morpho: Address,
    /// Market identifier
    pub market_id: B256,
    /// The caller initiating the collateral supply
    pub caller: Address,
    /// The beneficiary of the collateral
    pub on_behalf_of: Address,
    /// Amount of collateral assets supplied
    pub assets: U256,
}

impl ParsedMorphoSupplyCollateral {
    /// Try to parse a SupplyCollateral event from a log entry
    pub fn try_from_log(log: &ReceiptLog) -> Option<Self> {
        if log.topics.len() != 4 {
            return None;
        }

        if log.topics[0] != SupplyCollateral::SIGNATURE_HASH {
            return None;
        }

        let decoded =
            SupplyCollateral::decode_raw_log(log.topics.iter().copied(), &log.data, true).ok()?;

        Some(Self {
            morpho: log.address,
            market_id: B256::from(decoded.id),
            caller: decoded.caller,
            on_behalf_of: decoded.onBehalf,
            assets: decoded.assets,
        })
    }

    /// Extract all SupplyCollateral events from a slice of logs
    pub fn extract_all(logs: &[ReceiptLog]) -> Vec<Self> {
        logs.iter().filter_map(Self::try_from_log).collect()
    }
}

/// Parsed Morpho WithdrawCollateral event
#[derive(Debug, Clone, Serialize)]
pub struct ParsedMorphoWithdrawCollateral {
    /// Address of the Morpho contract
    pub morpho: Address,
    /// Market identifier
    pub market_id: B256,
    /// The caller initiating the collateral withdrawal
    pub caller: Address,
    /// The owner of the collateral
    pub on_behalf_of: Address,
    /// The receiver of the collateral
    pub receiver: Address,
    /// Amount of collateral assets withdrawn
    pub assets: U256,
}

impl ParsedMorphoWithdrawCollateral {
    /// Try to parse a WithdrawCollateral event from a log entry
    pub fn try_from_log(log: &ReceiptLog) -> Option<Self> {
        if log.topics.len() != 4 {
            return None;
        }

        if log.topics[0] != WithdrawCollateral::SIGNATURE_HASH {
            return None;
        }

        let decoded =
            WithdrawCollateral::decode_raw_log(log.topics.iter().copied(), &log.data, true).ok()?;

        Some(Self {
            morpho: log.address,
            market_id: B256::from(decoded.id),
            caller: decoded.caller,
            on_behalf_of: decoded.onBehalf,
            receiver: decoded.receiver,
            assets: decoded.assets,
        })
    }

    /// Extract all WithdrawCollateral events from a slice of logs
    pub fn extract_all(logs: &[ReceiptLog]) -> Vec<Self> {
        logs.iter().filter_map(Self::try_from_log).collect()
    }
}

/// Parsed Morpho Liquidate event
#[derive(Debug, Clone, Serialize)]
pub struct ParsedMorphoLiquidation {
    /// Address of the Morpho contract
    pub morpho: Address,
    /// Market identifier
    pub market_id: B256,
    /// The liquidator
    pub caller: Address,
    /// The borrower being liquidated
    pub borrower: Address,
    /// Amount of assets repaid by liquidator
    pub repaid_assets: U256,
    /// Amount of debt shares burned
    pub repaid_shares: U256,
    /// Amount of collateral seized
    pub seized_assets: U256,
    /// Bad debt assets (if any)
    pub bad_debt_assets: U256,
    /// Bad debt shares (if any)
    pub bad_debt_shares: U256,
}

impl ParsedMorphoLiquidation {
    /// Try to parse a Liquidate event from a log entry
    pub fn try_from_log(log: &ReceiptLog) -> Option<Self> {
        if log.topics.len() != 4 {
            return None;
        }

        if log.topics[0] != Liquidate::SIGNATURE_HASH {
            return None;
        }

        let decoded =
            Liquidate::decode_raw_log(log.topics.iter().copied(), &log.data, true).ok()?;

        Some(Self {
            morpho: log.address,
            market_id: B256::from(decoded.id),
            caller: decoded.caller,
            borrower: decoded.borrower,
            repaid_assets: decoded.repaidAssets,
            repaid_shares: decoded.repaidShares,
            seized_assets: decoded.seizedAssets,
            bad_debt_assets: decoded.badDebtAssets,
            bad_debt_shares: decoded.badDebtShares,
        })
    }

    /// Extract all Liquidate events from a slice of logs
    pub fn extract_all(logs: &[ReceiptLog]) -> Vec<Self> {
        logs.iter().filter_map(Self::try_from_log).collect()
    }
}

/// Parsed Morpho CreateMarket event
#[derive(Debug, Clone, Serialize)]
pub struct ParsedMorphoCreateMarket {
    /// Address of the Morpho contract
    pub morpho: Address,
    /// Market identifier
    pub market_id: B256,
    /// The loan token for this market
    pub loan_token: Address,
    /// The collateral token for this market
    pub collateral_token: Address,
    /// The oracle used for this market
    pub oracle: Address,
    /// The interest rate model
    pub irm: Address,
    /// Liquidation loan-to-value ratio
    pub lltv: U256,
}

impl ParsedMorphoCreateMarket {
    /// Try to parse a CreateMarket event from a log entry
    pub fn try_from_log(log: &ReceiptLog) -> Option<Self> {
        if log.topics.len() != 2 {
            return None;
        }

        if log.topics[0] != CreateMarket::SIGNATURE_HASH {
            return None;
        }

        let decoded =
            CreateMarket::decode_raw_log(log.topics.iter().copied(), &log.data, true).ok()?;

        Some(Self {
            morpho: log.address,
            market_id: B256::from(decoded.id),
            loan_token: decoded.loanToken,
            collateral_token: decoded.collateralToken,
            oracle: decoded.oracle,
            irm: decoded.irm,
            lltv: decoded.lltv,
        })
    }

    /// Extract all CreateMarket events from a slice of logs
    pub fn extract_all(logs: &[ReceiptLog]) -> Vec<Self> {
        logs.iter().filter_map(Self::try_from_log).collect()
    }
}

/// All Morpho events extracted from logs
#[derive(Debug, Clone, Default, Serialize)]
pub struct MorphoUpdates {
    pub supplies: Vec<ParsedMorphoSupply>,
    pub withdraws: Vec<ParsedMorphoWithdraw>,
    pub borrows: Vec<ParsedMorphoBorrow>,
    pub repays: Vec<ParsedMorphoRepay>,
    pub supply_collaterals: Vec<ParsedMorphoSupplyCollateral>,
    pub withdraw_collaterals: Vec<ParsedMorphoWithdrawCollateral>,
    pub liquidations: Vec<ParsedMorphoLiquidation>,
    pub create_markets: Vec<ParsedMorphoCreateMarket>,
}

impl MorphoUpdates {
    /// Extract all Morpho events from a slice of logs
    pub fn extract_all(logs: &[ReceiptLog]) -> Self {
        Self {
            supplies: ParsedMorphoSupply::extract_all(logs),
            withdraws: ParsedMorphoWithdraw::extract_all(logs),
            borrows: ParsedMorphoBorrow::extract_all(logs),
            repays: ParsedMorphoRepay::extract_all(logs),
            supply_collaterals: ParsedMorphoSupplyCollateral::extract_all(logs),
            withdraw_collaterals: ParsedMorphoWithdrawCollateral::extract_all(logs),
            liquidations: ParsedMorphoLiquidation::extract_all(logs),
            create_markets: ParsedMorphoCreateMarket::extract_all(logs),
        }
    }

    /// Returns true if no Morpho events were found
    pub fn is_empty(&self) -> bool {
        self.supplies.is_empty()
            && self.withdraws.is_empty()
            && self.borrows.is_empty()
            && self.repays.is_empty()
            && self.supply_collaterals.is_empty()
            && self.withdraw_collaterals.is_empty()
            && self.liquidations.is_empty()
            && self.create_markets.is_empty()
    }

    /// Total count of all events
    pub fn total_count(&self) -> usize {
        self.supplies.len()
            + self.withdraws.len()
            + self.borrows.len()
            + self.repays.len()
            + self.supply_collaterals.len()
            + self.withdraw_collaterals.len()
            + self.liquidations.len()
            + self.create_markets.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_morpho_events_default() {
        let events = MorphoEvents::default();
        assert!(!events.any());
    }

    #[test]
    fn test_supply_signature() {
        let expected_sig =
            alloy_primitives::keccak256(b"Supply(bytes32,address,address,uint256,uint256)");
        assert_eq!(Supply::SIGNATURE_HASH, expected_sig);
    }

    #[test]
    fn test_withdraw_signature() {
        let expected_sig = alloy_primitives::keccak256(
            b"Withdraw(bytes32,address,address,address,uint256,uint256)",
        );
        assert_eq!(Withdraw::SIGNATURE_HASH, expected_sig);
    }

    #[test]
    fn test_borrow_signature() {
        let expected_sig =
            alloy_primitives::keccak256(b"Borrow(bytes32,address,address,address,uint256,uint256)");
        assert_eq!(Borrow::SIGNATURE_HASH, expected_sig);
    }

    #[test]
    fn test_repay_signature() {
        let expected_sig =
            alloy_primitives::keccak256(b"Repay(bytes32,address,address,uint256,uint256)");
        assert_eq!(Repay::SIGNATURE_HASH, expected_sig);
    }

    #[test]
    fn test_supply_collateral_signature() {
        let expected_sig =
            alloy_primitives::keccak256(b"SupplyCollateral(bytes32,address,address,uint256)");
        assert_eq!(SupplyCollateral::SIGNATURE_HASH, expected_sig);
    }

    #[test]
    fn test_withdraw_collateral_signature() {
        let expected_sig = alloy_primitives::keccak256(
            b"WithdrawCollateral(bytes32,address,address,address,uint256)",
        );
        assert_eq!(WithdrawCollateral::SIGNATURE_HASH, expected_sig);
    }

    #[test]
    fn test_liquidate_signature() {
        let expected_sig = alloy_primitives::keccak256(
            b"Liquidate(bytes32,address,address,uint256,uint256,uint256,uint256,uint256)",
        );
        assert_eq!(Liquidate::SIGNATURE_HASH, expected_sig);
    }

    #[test]
    fn test_create_market_signature() {
        let expected_sig = alloy_primitives::keccak256(
            b"CreateMarket(bytes32,address,address,address,address,uint256)",
        );
        assert_eq!(CreateMarket::SIGNATURE_HASH, expected_sig);
    }

    #[test]
    fn test_morpho_updates_empty() {
        let updates = MorphoUpdates::default();
        assert!(updates.is_empty());
        assert_eq!(updates.total_count(), 0);
    }
}
