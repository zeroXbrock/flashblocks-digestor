use alloy_primitives::{Address, Bloom, BloomInput, U256};
use alloy_sol_types::{SolEvent, sol};
use serde::Serialize;

use crate::flashblocks::ReceiptLog;

// AAVE V3 Pool events for user actions
sol! {
    /// Emitted when a user supplies assets to the pool
    event Supply(
        address indexed reserve,
        address user,
        address indexed onBehalfOf,
        uint256 amount,
        uint16 indexed referralCode
    );

    /// Emitted when a user withdraws assets from the pool
    event Withdraw(
        address indexed reserve,
        address indexed user,
        address indexed to,
        uint256 amount
    );

    /// Emitted when a user borrows assets from the pool
    event Borrow(
        address indexed reserve,
        address user,
        address indexed onBehalfOf,
        uint256 amount,
        uint8 interestRateMode,
        uint256 borrowRate,
        uint16 indexed referralCode
    );

    /// Emitted when a user repays borrowed assets
    event Repay(
        address indexed reserve,
        address indexed user,
        address indexed repayer,
        uint256 amount,
        bool useATokens
    );

    /// Emitted when a liquidation occurs
    event LiquidationCall(
        address indexed collateralAsset,
        address indexed debtAsset,
        address indexed user,
        uint256 debtToCover,
        uint256 liquidatedCollateralAmount,
        address liquidator,
        bool receiveAToken
    );
}

/// Detected AAVE events based on bloom filter
#[derive(Debug, Default)]
pub struct AaveEvents {
    pub may_have_supply: bool,
    pub may_have_withdraw: bool,
    pub may_have_borrow: bool,
    pub may_have_repay: bool,
    pub may_have_liquidation: bool,
}

impl AaveEvents {
    /// Check the bloom filter for potential AAVE events.
    /// Note: Bloom filters can have false positives but no false negatives.
    pub fn from_bloom(bloom: &Bloom) -> Self {
        Self {
            may_have_supply: bloom.contains_input(BloomInput::Hash(Supply::SIGNATURE_HASH)),
            may_have_withdraw: bloom.contains_input(BloomInput::Hash(Withdraw::SIGNATURE_HASH)),
            may_have_borrow: bloom.contains_input(BloomInput::Hash(Borrow::SIGNATURE_HASH)),
            may_have_repay: bloom.contains_input(BloomInput::Hash(Repay::SIGNATURE_HASH)),
            may_have_liquidation: bloom
                .contains_input(BloomInput::Hash(LiquidationCall::SIGNATURE_HASH)),
        }
    }

    /// Returns true if any AAVE user event might be present
    pub fn any(&self) -> bool {
        self.may_have_supply
            || self.may_have_withdraw
            || self.may_have_borrow
            || self.may_have_repay
            || self.may_have_liquidation
    }
}

/// Parsed AAVE Supply event
#[derive(Debug, Clone, Serialize)]
pub struct ParsedSupply {
    /// Address of the AAVE pool contract
    pub pool: Address,
    /// The reserve (token) being supplied
    pub reserve: Address,
    /// The user initiating the supply
    pub user: Address,
    /// The beneficiary of the supply (receives aTokens)
    pub on_behalf_of: Address,
    /// Amount supplied
    pub amount: U256,
    /// Referral code
    pub referral_code: u16,
}

impl ParsedSupply {
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
            pool: log.address,
            reserve: decoded.reserve,
            user: decoded.user,
            on_behalf_of: decoded.onBehalfOf,
            amount: decoded.amount,
            referral_code: decoded.referralCode,
        })
    }

    /// Extract all Supply events from a slice of logs
    pub fn extract_all(logs: &[ReceiptLog]) -> Vec<Self> {
        logs.iter().filter_map(Self::try_from_log).collect()
    }
}

/// Parsed AAVE Withdraw event
#[derive(Debug, Clone, Serialize)]
pub struct ParsedWithdraw {
    /// Address of the AAVE pool contract
    pub pool: Address,
    /// The reserve (token) being withdrawn
    pub reserve: Address,
    /// The user initiating the withdrawal
    pub user: Address,
    /// The recipient of the withdrawn assets
    pub to: Address,
    /// Amount withdrawn
    pub amount: U256,
}

impl ParsedWithdraw {
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
            pool: log.address,
            reserve: decoded.reserve,
            user: decoded.user,
            to: decoded.to,
            amount: decoded.amount,
        })
    }

    /// Extract all Withdraw events from a slice of logs
    pub fn extract_all(logs: &[ReceiptLog]) -> Vec<Self> {
        logs.iter().filter_map(Self::try_from_log).collect()
    }
}

/// Parsed AAVE Borrow event
#[derive(Debug, Clone, Serialize)]
pub struct ParsedBorrow {
    /// Address of the AAVE pool contract
    pub pool: Address,
    /// The reserve (token) being borrowed
    pub reserve: Address,
    /// The user initiating the borrow
    pub user: Address,
    /// The beneficiary of the borrow (receives the tokens)
    pub on_behalf_of: Address,
    /// Amount borrowed
    pub amount: U256,
    /// Interest rate mode (1 = stable, 2 = variable)
    pub interest_rate_mode: u8,
    /// The borrow rate
    pub borrow_rate: U256,
    /// Referral code
    pub referral_code: u16,
}

impl ParsedBorrow {
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
            pool: log.address,
            reserve: decoded.reserve,
            user: decoded.user,
            on_behalf_of: decoded.onBehalfOf,
            amount: decoded.amount,
            interest_rate_mode: decoded.interestRateMode,
            borrow_rate: decoded.borrowRate,
            referral_code: decoded.referralCode,
        })
    }

    /// Extract all Borrow events from a slice of logs
    pub fn extract_all(logs: &[ReceiptLog]) -> Vec<Self> {
        logs.iter().filter_map(Self::try_from_log).collect()
    }
}

/// Parsed AAVE Repay event
#[derive(Debug, Clone, Serialize)]
pub struct ParsedRepay {
    /// Address of the AAVE pool contract
    pub pool: Address,
    /// The reserve (token) being repaid
    pub reserve: Address,
    /// The user whose debt is being repaid
    pub user: Address,
    /// The address making the repayment
    pub repayer: Address,
    /// Amount repaid
    pub amount: U256,
    /// Whether aTokens were used for repayment
    pub use_a_tokens: bool,
}

impl ParsedRepay {
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
            pool: log.address,
            reserve: decoded.reserve,
            user: decoded.user,
            repayer: decoded.repayer,
            amount: decoded.amount,
            use_a_tokens: decoded.useATokens,
        })
    }

    /// Extract all Repay events from a slice of logs
    pub fn extract_all(logs: &[ReceiptLog]) -> Vec<Self> {
        logs.iter().filter_map(Self::try_from_log).collect()
    }
}

/// Parsed AAVE LiquidationCall event
#[derive(Debug, Clone, Serialize)]
pub struct ParsedLiquidation {
    /// Address of the AAVE pool contract
    pub pool: Address,
    /// The collateral asset being liquidated
    pub collateral_asset: Address,
    /// The debt asset being repaid
    pub debt_asset: Address,
    /// The user being liquidated
    pub user: Address,
    /// Amount of debt covered
    pub debt_to_cover: U256,
    /// Amount of collateral liquidated
    pub liquidated_collateral_amount: U256,
    /// The liquidator address
    pub liquidator: Address,
    /// Whether liquidator receives aTokens
    pub receive_a_token: bool,
}

impl ParsedLiquidation {
    /// Try to parse a LiquidationCall event from a log entry
    pub fn try_from_log(log: &ReceiptLog) -> Option<Self> {
        if log.topics.len() != 4 {
            return None;
        }

        if log.topics[0] != LiquidationCall::SIGNATURE_HASH {
            return None;
        }

        let decoded =
            LiquidationCall::decode_raw_log(log.topics.iter().copied(), &log.data, true).ok()?;

        Some(Self {
            pool: log.address,
            collateral_asset: decoded.collateralAsset,
            debt_asset: decoded.debtAsset,
            user: decoded.user,
            debt_to_cover: decoded.debtToCover,
            liquidated_collateral_amount: decoded.liquidatedCollateralAmount,
            liquidator: decoded.liquidator,
            receive_a_token: decoded.receiveAToken,
        })
    }

    /// Extract all LiquidationCall events from a slice of logs
    pub fn extract_all(logs: &[ReceiptLog]) -> Vec<Self> {
        logs.iter().filter_map(Self::try_from_log).collect()
    }
}

/// All AAVE user action events extracted from logs
#[derive(Debug, Clone, Default, Serialize)]
pub struct AaveUserUpdates {
    pub supplies: Vec<ParsedSupply>,
    pub withdraws: Vec<ParsedWithdraw>,
    pub borrows: Vec<ParsedBorrow>,
    pub repays: Vec<ParsedRepay>,
    pub liquidations: Vec<ParsedLiquidation>,
}

impl AaveUserUpdates {
    /// Extract all AAVE user events from a slice of logs
    pub fn extract_all(logs: &[ReceiptLog]) -> Self {
        Self {
            supplies: ParsedSupply::extract_all(logs),
            withdraws: ParsedWithdraw::extract_all(logs),
            borrows: ParsedBorrow::extract_all(logs),
            repays: ParsedRepay::extract_all(logs),
            liquidations: ParsedLiquidation::extract_all(logs),
        }
    }

    /// Returns true if any AAVE user events were found
    pub fn is_empty(&self) -> bool {
        self.supplies.is_empty()
            && self.withdraws.is_empty()
            && self.borrows.is_empty()
            && self.repays.is_empty()
            && self.liquidations.is_empty()
    }

    /// Total count of all events
    pub fn total_count(&self) -> usize {
        self.supplies.len()
            + self.withdraws.len()
            + self.borrows.len()
            + self.repays.len()
            + self.liquidations.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aave_events_default() {
        let events = AaveEvents::default();
        assert!(!events.any());
    }

    #[test]
    fn test_supply_signature() {
        let expected_sig =
            alloy_primitives::keccak256(b"Supply(address,address,address,uint256,uint16)");
        assert_eq!(Supply::SIGNATURE_HASH, expected_sig);
    }

    #[test]
    fn test_withdraw_signature() {
        let expected_sig =
            alloy_primitives::keccak256(b"Withdraw(address,address,address,uint256)");
        assert_eq!(Withdraw::SIGNATURE_HASH, expected_sig);
    }

    #[test]
    fn test_borrow_signature() {
        let expected_sig = alloy_primitives::keccak256(
            b"Borrow(address,address,address,uint256,uint8,uint256,uint16)",
        );
        assert_eq!(Borrow::SIGNATURE_HASH, expected_sig);
    }

    #[test]
    fn test_repay_signature() {
        let expected_sig =
            alloy_primitives::keccak256(b"Repay(address,address,address,uint256,bool)");
        assert_eq!(Repay::SIGNATURE_HASH, expected_sig);
    }

    #[test]
    fn test_liquidation_signature() {
        let expected_sig = alloy_primitives::keccak256(
            b"LiquidationCall(address,address,address,uint256,uint256,address,bool)",
        );
        assert_eq!(LiquidationCall::SIGNATURE_HASH, expected_sig);
    }
}
