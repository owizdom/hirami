//! Transaction analyzer for MEV detection.

use alloy::rpc::types::Transaction;
use mev_africa_db::models::MevReasonCode;
use crate::detectors::detect_mev_patterns;

/// Transaction analysis result.
#[derive(Debug, Clone)]
pub struct TransactionAnalysis {
    /// Whether this transaction is a MEV candidate.
    pub is_mev_candidate: bool,
    /// Reason codes for MEV detection.
    pub reason_codes: Vec<MevReasonCode>,
    /// Summary of calldata (first 100 bytes as hex).
    pub calldata_summary: Option<String>,
    /// Summary of logs (count and topics).
    pub log_summary: Option<String>,
}

/// Analyzer for detecting MEV patterns in transactions.
pub struct TransactionAnalyzer;

impl TransactionAnalyzer {
    /// Analyze a transaction for MEV patterns.
    ///
    /// # Arguments
    /// * `tx` - The transaction to analyze
    /// * `block_txs` - All transactions in the block (for context)
    /// * `tx_index` - Index of this transaction in the block
    ///
    /// # Returns
    /// Analysis result with MEV detection flags and reason codes
    pub fn analyze(
        tx: &Transaction,
        block_txs: &[&Transaction],
        tx_index: usize,
    ) -> TransactionAnalysis {
        let reason_codes = detect_mev_patterns(tx, block_txs, tx_index);
        let is_mev_candidate = !reason_codes.is_empty();

        let calldata_summary = if !tx.input.is_empty() {
            let hex_str = hex::encode(tx.input.as_ref());
            Some(if hex_str.len() > 200 {
                format!("{}...", &hex_str[..200])
            } else {
                hex_str
            })
        } else {
            None
        };

        // Note: Logs are not available in Transaction type from RPC
        // They would need to be fetched separately via eth_getTransactionReceipt
        let log_summary = None;

        TransactionAnalysis {
            is_mev_candidate,
            reason_codes,
            calldata_summary,
            log_summary,
        }
    }
}

