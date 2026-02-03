use alloy_primitives::{Address, Bloom, BloomInput, I256, LogData, U160};
use alloy_sol_types::{SolEvent, sol};
use serde::Serialize;

use crate::flashblocks::ReceiptLog;

// Uniswap V3 Pool events
sol! {
    /// Emitted when liquidity is minted for a given position
    event Mint(
        address sender,
        address indexed owner,
        int24 indexed tickLower,
        int24 indexed tickUpper,
        uint128 amount,
        uint256 amount0,
        uint256 amount1
    );

    /// Emitted when fees are collected by the owner of a position
    event Collect(
        address indexed owner,
        address recipient,
        int24 indexed tickLower,
        int24 indexed tickUpper,
        uint128 amount0,
        uint128 amount1
    );

    /// Emitted when a position's liquidity is removed
    event Burn(
        address indexed owner,
        int24 indexed tickLower,
        int24 indexed tickUpper,
        uint128 amount,
        uint256 amount0,
        uint256 amount1
    );

    /// Emitted by the pool for any swaps between token0 and token1
    event Swap(
        address indexed sender,
        address indexed recipient,
        int256 amount0,
        int256 amount1,
        uint160 sqrtPriceX96,
        uint128 liquidity,
        int24 tick
    );

    /// Emitted by the pool for any flash loans of token0/token1
    event Flash(
        address indexed sender,
        address indexed recipient,
        uint256 amount0,
        uint256 amount1,
        uint256 paid0,
        uint256 paid1
    );
}

/// Detected UniswapV3 events based on bloom filter
#[derive(Debug, Default)]
pub struct UniV3Events {
    pub may_have_mint: bool,
    pub may_have_collect: bool,
    pub may_have_burn: bool,
    pub may_have_swap: bool,
    pub may_have_flash: bool,
}

impl UniV3Events {
    /// Check the bloom filter for potential UniV3 events.
    /// Note: Bloom filters can have false positives but no false negatives.
    pub fn from_bloom(bloom: &Bloom) -> Self {
        Self {
            may_have_mint: bloom.contains_input(BloomInput::Hash(Mint::SIGNATURE_HASH)),
            may_have_collect: bloom.contains_input(BloomInput::Hash(Collect::SIGNATURE_HASH)),
            may_have_burn: bloom.contains_input(BloomInput::Hash(Burn::SIGNATURE_HASH)),
            may_have_swap: bloom.contains_input(BloomInput::Hash(Swap::SIGNATURE_HASH)),
            may_have_flash: bloom.contains_input(BloomInput::Hash(Flash::SIGNATURE_HASH)),
        }
    }

    /// Returns true if any UniV3 event might be present
    pub fn any(&self) -> bool {
        self.may_have_mint
            || self.may_have_collect
            || self.may_have_burn
            || self.may_have_swap
            || self.may_have_flash
    }
}

/// Partial slot0 data that can be derived from swap events.
///
/// The full on-chain slot0 contains:
/// - sqrtPriceX96: uint160 ✓ (from Swap event)
/// - tick: int24 ✓ (from Swap event)
/// - observationIndex: uint16 ✗ (oracle state, needs RPC)
/// - observationCardinality: uint16 ✗ (oracle state, needs RPC)
/// - observationCardinalityNext: uint16 ✗ (oracle state, needs RPC)
/// - feeProtocol: uint8 ✗ (protocol fee, needs RPC)
/// - unlocked: bool ✗ (reentrancy lock, needs RPC)
///
/// Note: `liquidity` is NOT part of slot0 - it's a separate storage variable,
/// but we include it here since it's emitted in the Swap event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Slot0Partial {
    /// The current sqrt(price) as a Q64.96 value
    pub sqrt_price_x96: U160,
    /// The current tick
    pub tick: i32,
}

/// Pool state derived from a Swap event (slot0 + liquidity)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PoolState {
    /// The current sqrt(price) as a Q64.96 value
    pub sqrt_price_x96: U160,
    /// The current tick
    pub tick: i32,
    /// The current in-range liquidity (separate from slot0 on-chain)
    pub liquidity: u128,
}

impl PoolState {
    /// Calculate the price of token0 in terms of token1.
    /// price = (sqrtPriceX96 / 2^96)^2
    pub fn price_0_in_1(&self) -> f64 {
        let sqrt_price = self.sqrt_price_x96.to::<u128>() as f64 / (2_u128.pow(96) as f64);
        sqrt_price * sqrt_price
    }

    /// Calculate the price of token1 in terms of token0.
    pub fn price_1_in_0(&self) -> f64 {
        1.0 / self.price_0_in_1()
    }

    /// Get just the slot0 portion (sqrtPriceX96 and tick)
    pub fn slot0(&self) -> Slot0Partial {
        Slot0Partial {
            sqrt_price_x96: self.sqrt_price_x96,
            tick: self.tick,
        }
    }
}

/// A decoded Uniswap V3 Swap event with pool address
#[derive(Debug, Clone, Serialize)]
pub struct ParsedSwap {
    /// The pool contract that emitted the event
    pub pool: Address,
    /// The address that initiated the swap
    pub sender: Address,
    /// The address that received the output
    pub recipient: Address,
    /// Amount of token0 (positive = pool received, negative = pool sent)
    pub amount0: I256,
    /// Amount of token1 (positive = pool received, negative = pool sent)
    pub amount1: I256,
    /// The sqrt(price) after the swap as a Q64.96
    pub sqrt_price_x96: U160,
    /// The liquidity after the swap
    pub liquidity: u128,
    /// The tick after the swap
    pub tick: i32,
}

impl ParsedSwap {
    /// Try to decode a Swap event from a log
    pub fn from_log(log: &ReceiptLog) -> Option<Self> {
        // Check if this log matches the Swap event signature
        if log.topics.first() != Some(&Swap::SIGNATURE_HASH) {
            return None;
        }

        // Create a LogData for decoding
        let log_data = LogData::new(log.topics.clone(), log.data.clone())?;

        // Decode the event
        let decoded = Swap::decode_log_data(&log_data, true).ok()?;

        Some(Self {
            pool: log.address,
            sender: decoded.sender,
            recipient: decoded.recipient,
            amount0: decoded.amount0,
            amount1: decoded.amount1,
            sqrt_price_x96: decoded.sqrtPriceX96,
            liquidity: decoded.liquidity,
            tick: decoded.tick.as_i32(),
        })
    }

    /// Extract all Swap events from a list of logs
    pub fn extract_all(logs: &[ReceiptLog]) -> Vec<Self> {
        logs.iter().filter_map(Self::from_log).collect()
    }

    /// Get the pool state after this swap (sqrtPriceX96, tick, and liquidity)
    pub fn pool_state(&self) -> PoolState {
        PoolState {
            sqrt_price_x96: self.sqrt_price_x96,
            tick: self.tick,
            liquidity: self.liquidity,
        }
    }
}
