use alloy_primitives::{Address, B256, Bloom, BloomInput, Bytes, I256, U160, U256};
use alloy_rpc_types::Log;
use alloy_sol_types::{SolEvent, sol};
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;

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

/// A decoded Uniswap V3 Swap event with pool address
#[derive(Debug, Clone)]
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
    pub fn from_log(log: &Log) -> Option<Self> {
        // Check if this log matches the Swap event signature
        let topics = log.topics();
        if topics.first() != Some(&Swap::SIGNATURE_HASH) {
            return None;
        }

        // Decode the event (convert RPC Log to primitives Log)
        let decoded = Swap::decode_log(&log.inner, true).ok()?;

        Some(Self {
            pool: log.address(),
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
    pub fn extract_all(logs: &[Log]) -> Vec<Self> {
        logs.iter().filter_map(Self::from_log).collect()
    }
}

/// Receipt data from flashblock metadata
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FlashblockReceipt {
    #[serde(default)]
    pub logs: Vec<Log>,
    #[serde(default)]
    pub logs_bloom: Option<Bloom>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub gas_used: Option<U256>,
}

impl FlashblockReceipt {
    /// Check if this receipt might contain a Swap event using its bloom filter
    pub fn may_have_swap(&self) -> bool {
        self.logs_bloom
            .as_ref()
            .map(|bloom| bloom.contains_input(BloomInput::Hash(Swap::SIGNATURE_HASH)))
            .unwrap_or(true) // If no bloom, assume it might have swaps
    }
}

/// Flashblock metadata containing receipts and balance changes
#[derive(Debug, Deserialize)]
pub struct FlashblockMetadata {
    /// Transaction receipts keyed by tx hash
    #[serde(default)]
    pub receipts: HashMap<String, FlashblockReceipt>,
    /// New account balances after this flashblock
    #[serde(default)]
    pub new_account_balances: HashMap<String, String>,
    /// Block number
    #[serde(default)]
    pub block_number: u64,
}

impl FlashblockMetadata {
    /// Extract all Swap events from receipts, using bloom filters to skip irrelevant receipts
    pub fn extract_swaps(&self) -> Vec<ParsedSwap> {
        self.receipts
            .values()
            .filter(|receipt| receipt.may_have_swap())
            .flat_map(|receipt| ParsedSwap::extract_all(&receipt.logs))
            .collect()
    }
}

/// Execution payload diff containing the changes in this flashblock.
#[derive(Debug, Deserialize)]
pub struct ExecutionPayloadDiff {
    /// Blob gas used in this block
    pub blob_gas_used: U256,
    /// Block hash
    pub block_hash: B256,
    /// Total gas used
    pub gas_used: U256,
    /// Bloom filter for logs
    pub logs_bloom: Bloom,
    /// Receipts trie root
    pub receipts_root: B256,
    /// State trie root
    pub state_root: B256,
    /// RLP-encoded transactions
    pub transactions: Vec<Bytes>,
    /// Withdrawals in this block
    #[serde(default)]
    pub withdrawals: Vec<Value>,
    /// Withdrawals trie root
    pub withdrawals_root: B256,
}

impl std::fmt::Display for ExecutionPayloadDiff {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ExecutionPayloadDiff {{ blob_gas_used: {}, block_hash: {:?}, gas_used: {}, logs_bloom: {:?}, receipts_root: {:?}, state_root: {:?}, transactions: [{} txs], withdrawals: [{} withdrawals], withdrawals_root: {:?} }}",
            self.blob_gas_used,
            self.block_hash,
            self.gas_used,
            self.logs_bloom,
            self.receipts_root,
            self.state_root,
            self.transactions.len(),
            self.withdrawals.len(),
            self.withdrawals_root
        )
    }
}

/// A minimal view of the Flashblock payload so we can print something structured.
/// Most fields are left as `Value` so you can extend this as needed.
#[derive(Debug, Deserialize)]
pub struct Flashblock {
    pub payload_id: String,
    pub index: u64,
    #[serde(default)]
    pub metadata: Option<FlashblockMetadata>,
    #[serde(default)]
    pub base: Value,
    #[serde(default)]
    pub diff: Option<ExecutionPayloadDiff>,
}

impl Flashblock {
    /// Extract all Swap events from this flashblock's metadata
    pub fn extract_swaps(&self) -> Vec<ParsedSwap> {
        self.metadata
            .as_ref()
            .map(|m| m.extract_swaps())
            .unwrap_or_default()
    }
}
