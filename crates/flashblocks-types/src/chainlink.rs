use alloy_primitives::{Address, Bloom, BloomInput, I256, U256};
use alloy_sol_types::{SolEvent, sol};
use serde::Serialize;

use crate::flashblocks::ReceiptLog;

// Chainlink Aggregator events
sol! {
    /// Emitted when a new answer is submitted by an oracle
    event AnswerUpdated(
        int256 indexed current,
        uint256 indexed roundId,
        uint256 updatedAt
    );

    /// Emitted when a new round is started
    event NewRound(
        uint256 indexed roundId,
        address indexed startedBy,
        uint256 startedAt
    );
}

/// Detected Chainlink oracle events based on bloom filter
#[derive(Debug, Default)]
pub struct ChainlinkEvents {
    pub may_have_answer_updated: bool,
    pub may_have_new_round: bool,
}

impl ChainlinkEvents {
    /// Check the bloom filter for potential Chainlink events.
    /// Note: Bloom filters can have false positives but no false negatives.
    pub fn from_bloom(bloom: &Bloom) -> Self {
        Self {
            may_have_answer_updated: bloom
                .contains_input(BloomInput::Hash(AnswerUpdated::SIGNATURE_HASH)),
            may_have_new_round: bloom.contains_input(BloomInput::Hash(NewRound::SIGNATURE_HASH)),
        }
    }

    /// Returns true if any Chainlink event might be present
    pub fn any(&self) -> bool {
        self.may_have_answer_updated || self.may_have_new_round
    }
}

/// Parsed Chainlink AnswerUpdated event with pool address
#[derive(Debug, Clone, Serialize)]
pub struct ParsedAnswerUpdated {
    /// Address of the Chainlink aggregator/feed
    pub feed: Address,
    /// The new answer value (price with decimals)
    pub answer: I256,
    /// The round ID for this update
    pub round_id: U256,
    /// Timestamp when the answer was updated
    pub updated_at: U256,
}

impl ParsedAnswerUpdated {
    /// Try to parse an AnswerUpdated event from a log entry
    pub fn try_from_log(log: &ReceiptLog) -> Option<Self> {
        // AnswerUpdated has 2 indexed parameters (current, roundId) + 1 non-indexed (updatedAt)
        if log.topics.len() != 3 {
            return None;
        }

        // Check event signature
        if log.topics[0] != AnswerUpdated::SIGNATURE_HASH {
            return None;
        }

        // Decode the event
        let decoded =
            AnswerUpdated::decode_raw_log(log.topics.iter().copied(), &log.data, true).ok()?;

        Some(Self {
            feed: log.address,
            answer: decoded.current,
            round_id: decoded.roundId,
            updated_at: decoded.updatedAt,
        })
    }

    /// Extract all AnswerUpdated events from a slice of logs
    pub fn extract_all(logs: &[ReceiptLog]) -> Vec<Self> {
        logs.iter().filter_map(Self::try_from_log).collect()
    }
}

/// Parsed Chainlink NewRound event
#[derive(Debug, Clone, Serialize)]
pub struct ParsedNewRound {
    /// Address of the Chainlink aggregator/feed
    pub feed: Address,
    /// The round ID that was started
    pub round_id: U256,
    /// Address that started the round
    pub started_by: Address,
    /// Timestamp when the round was started
    pub started_at: U256,
}

impl ParsedNewRound {
    /// Try to parse a NewRound event from a log entry
    pub fn try_from_log(log: &ReceiptLog) -> Option<Self> {
        // NewRound has 2 indexed parameters (roundId, startedBy) + 1 non-indexed (startedAt)
        if log.topics.len() != 3 {
            return None;
        }

        // Check event signature
        if log.topics[0] != NewRound::SIGNATURE_HASH {
            return None;
        }

        // Decode the event
        let decoded = NewRound::decode_raw_log(log.topics.iter().copied(), &log.data, true).ok()?;

        Some(Self {
            feed: log.address,
            round_id: decoded.roundId,
            started_by: decoded.startedBy,
            started_at: decoded.startedAt,
        })
    }

    /// Extract all NewRound events from a slice of logs
    pub fn extract_all(logs: &[ReceiptLog]) -> Vec<Self> {
        logs.iter().filter_map(Self::try_from_log).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chainlink_events_default() {
        let events = ChainlinkEvents::default();
        assert!(!events.any());
    }

    #[test]
    fn test_answer_updated_signature() {
        // Verify the event signature hash is computed correctly
        // AnswerUpdated(int256,uint256,uint256)
        let expected_sig = alloy_primitives::keccak256(b"AnswerUpdated(int256,uint256,uint256)");
        assert_eq!(AnswerUpdated::SIGNATURE_HASH, expected_sig);
    }

    #[test]
    fn test_new_round_signature() {
        // Verify the event signature hash is computed correctly
        // NewRound(uint256,address,uint256)
        let expected_sig = alloy_primitives::keccak256(b"NewRound(uint256,address,uint256)");
        assert_eq!(NewRound::SIGNATURE_HASH, expected_sig);
    }
}
