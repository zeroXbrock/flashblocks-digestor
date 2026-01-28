use alloy_primitives::{B256, Bloom, Bytes, U256};
use serde::Deserialize;
use serde_json::Value;

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
    pub metadata: Value,
    #[serde(default)]
    pub base: Value,
    #[serde(default)]
    pub diff: Option<ExecutionPayloadDiff>,
}
