use alloy_primitives::{Address, B256, Bloom, BloomInput, Bytes, U256};
use alloy_sol_types::SolEvent;
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;

use crate::chainlink::{AnswerUpdated, ParsedAnswerUpdated};
use crate::univ3::{ParsedSwap, Swap};

/// Log entry from receipt
#[derive(Debug, Deserialize, Clone)]
pub struct ReceiptLog {
    pub address: Address,
    pub data: Bytes,
    pub topics: Vec<B256>,
}

/// Inner receipt data (inside the transaction type wrapper)
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReceiptInner {
    pub logs: Vec<ReceiptLog>,
    pub logs_bloom: Option<Bloom>,
    pub status: Option<String>,
    pub cumulative_gas_used: Option<String>,
}

/// Receipt wrapped by transaction type (Eip1559, Legacy, etc.)
#[derive(Debug, Deserialize)]
pub enum FlashblockReceipt {
    Legacy(ReceiptInner),
    Eip2930(ReceiptInner),
    Eip1559(ReceiptInner),
    Eip4844(ReceiptInner),
    Eip7702(ReceiptInner),
    /// OP Stack deposit transaction
    Deposit(ReceiptInner),
}

impl FlashblockReceipt {
    /// Get the inner receipt data regardless of transaction type
    pub fn inner(&self) -> &ReceiptInner {
        match self {
            FlashblockReceipt::Legacy(inner)
            | FlashblockReceipt::Eip2930(inner)
            | FlashblockReceipt::Eip1559(inner)
            | FlashblockReceipt::Eip4844(inner)
            | FlashblockReceipt::Eip7702(inner)
            | FlashblockReceipt::Deposit(inner) => inner,
        }
    }

    /// Get the logs from this receipt
    pub fn logs(&self) -> &[ReceiptLog] {
        &self.inner().logs
    }

    /// Check if this receipt might contain a Swap event using its bloom filter
    pub fn may_have_swap(&self) -> bool {
        self.inner()
            .logs_bloom
            .as_ref()
            .map(|bloom| bloom.contains_input(BloomInput::Hash(Swap::SIGNATURE_HASH)))
            .unwrap_or(true) // If no bloom, assume it might have swaps
    }

    /// Check if this receipt might contain a Chainlink AnswerUpdated event using its bloom filter
    pub fn may_have_answer_updated(&self) -> bool {
        self.inner()
            .logs_bloom
            .as_ref()
            .map(|bloom| bloom.contains_input(BloomInput::Hash(AnswerUpdated::SIGNATURE_HASH)))
            .unwrap_or(true) // If no bloom, assume it might have updates
    }
}

/// Flashblock metadata containing receipts and balance changes
#[derive(Debug, Default, Deserialize)]
pub struct FlashblockMetadata {
    /// Transaction receipts keyed by tx hash
    pub receipts: HashMap<String, FlashblockReceipt>,
    /// New account balances after this flashblock
    pub new_account_balances: HashMap<String, String>,
    /// Block number
    pub block_number: u64,
}

impl FlashblockMetadata {
    /// Extract all Swap events from receipts, using bloom filters to skip irrelevant receipts
    pub fn extract_swaps(&self) -> Vec<ParsedSwap> {
        self.receipts
            .values()
            .filter(|receipt| receipt.may_have_swap())
            .flat_map(|receipt| ParsedSwap::extract_all(receipt.logs()))
            .collect()
    }

    /// Extract all Chainlink AnswerUpdated events from receipts, using bloom filters to skip irrelevant receipts
    pub fn extract_answer_updates(&self) -> Vec<ParsedAnswerUpdated> {
        self.receipts
            .values()
            .filter(|receipt| receipt.may_have_answer_updated())
            .flat_map(|receipt| ParsedAnswerUpdated::extract_all(receipt.logs()))
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

    /// Extract all Chainlink AnswerUpdated events from this flashblock's metadata
    pub fn extract_answer_updates(&self) -> Vec<ParsedAnswerUpdated> {
        self.metadata
            .as_ref()
            .map(|m| m.extract_answer_updates())
            .unwrap_or_default()
    }
}
