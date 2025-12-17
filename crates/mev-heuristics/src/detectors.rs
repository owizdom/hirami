//! MEV detection heuristics.

use alloy::rpc::types::Transaction;
use mev_africa_db::models::MevReasonCode as DbMevReasonCode;
use rust_decimal::Decimal;

/// Analyze a transaction for MEV patterns.
///
/// # Arguments
/// * `tx` - The transaction to analyze
/// * `block_txs` - All transactions in the block (for context)
/// * `tx_index` - Index of this transaction in the block
///
/// # Returns
/// Vector of MEV reason codes if MEV is detected
pub fn detect_mev_patterns(
    tx: &Transaction,
    block_txs: &[&Transaction],
    tx_index: usize,
) -> Vec<DbMevReasonCode> {
    let mut reasons = Vec::new();

    // High priority fee outlier detection
    if is_high_priority_fee_outlier(tx, block_txs) {
        reasons.push(DbMevReasonCode::HighPriorityFee);
    }

    // Repeated sender detection
    if is_repeated_sender(tx, block_txs, tx_index) {
        reasons.push(DbMevReasonCode::RepeatedSender);
    }

    // Atomic multiswap detection
    if is_atomic_multiswap(tx) {
        reasons.push(DbMevReasonCode::AtomicMultiswap);
    }

    // Sandwich pattern detection
    if is_sandwich_pattern(tx, block_txs, tx_index) {
        reasons.push(DbMevReasonCode::SandwichPattern);
    }

    reasons
}

/// Check if transaction has unusually high priority fee relative to block median.
fn is_high_priority_fee_outlier(tx: &Transaction, block_txs: &[&Transaction]) -> bool {
    let tx_priority_fee = match tx.max_priority_fee_per_gas {
        Some(fee) => Decimal::from(fee as u64),
        None => return false,
    };

    // Calculate median priority fee for the block
    let mut fees: Vec<Decimal> = block_txs
        .iter()
        .filter_map(|t| t.max_priority_fee_per_gas.map(|f| Decimal::from(f as u64)))
        .collect();

    if fees.is_empty() {
        return false;
    }

    fees.sort();
    let median = if fees.len() % 2 == 0 {
        (fees[fees.len() / 2 - 1] + fees[fees.len() / 2]) / Decimal::from(2)
    } else {
        fees[fees.len() / 2]
    };

    // Flag if priority fee is more than 3x the median
    tx_priority_fee > median * Decimal::from(3)
}

/// Check if sender appears multiple times in the block (potential bot activity).
fn is_repeated_sender(tx: &Transaction, block_txs: &[&Transaction], _tx_index: usize) -> bool {
    let sender = tx.from.to_string();

    let count = block_txs
        .iter()
        .filter(|t| t.from.to_string() == sender)
        .count();

    // Flag if sender appears 3+ times in the same block
    count >= 3
}

/// Check if transaction contains atomic multiswap patterns.
///
/// This is a simplified heuristic that looks for:
/// - Multiple internal calls (via calldata analysis)
/// - Common DEX router patterns
fn is_atomic_multiswap(tx: &Transaction) -> bool {
    // Check if calldata suggests multiple swaps
    // This is a simplified check - in production, you'd decode the calldata
    if !tx.input.is_empty() {
        let input_str = hex::encode(tx.input.as_ref());
        // Look for common swap function selectors
        // Uniswap V2: 0x7ff36ab5 (swapExactETHForTokens)
        // Uniswap V3: 0x414bf389 (exactInputSingle)
        // 0x5c11d795 (multicall)
        let swap_patterns = [
            "7ff36ab5", // swapExactETHForTokens
            "414bf389", // exactInputSingle
            "5c11d795", // multicall
        ];

        let pattern_count = swap_patterns
            .iter()
            .filter(|pattern| input_str.contains(*pattern))
            .count();

        // Flag if multiple swap patterns detected
        pattern_count >= 2
    } else {
        false
    }
}

/// Check if transaction is part of a sandwich pattern.
///
/// A sandwich pattern typically involves:
/// 1. A transaction before the target (front-run)
/// 2. The target transaction (victim)
/// 3. A transaction after the target (back-run)
///
/// All from the same sender or coordinated senders.
fn is_sandwich_pattern(tx: &Transaction, block_txs: &[&Transaction], tx_index: usize) -> bool {
    let sender = tx.from.to_string();

    // Check if same sender has transactions before and after this one
    let has_before = block_txs[..tx_index]
        .iter()
        .any(|t| t.from.to_string() == sender);

    let has_after = block_txs[tx_index + 1..]
        .iter()
        .any(|t| t.from.to_string() == sender);

    has_before && has_after
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy::primitives::{Address, U256};

    fn create_test_tx(sender: Address, priority_fee: Option<u64>) -> Transaction {
        use alloy::primitives::Bytes;
        Transaction {
            hash: Default::default(),
            nonce: 0,
            block_hash: None,
            block_number: None,
            transaction_index: None,
            from: sender,
            to: None,
            value: U256::ZERO,
            gas_price: None,
            max_fee_per_gas: None,
            max_priority_fee_per_gas: priority_fee.map(U256::from),
            gas: U256::ZERO,
            input: Bytes::new(),
            chain_id: None,
            v: 0,
            r: Default::default(),
            s: Default::default(),
            access_list: None,
            transaction_type: None,
        }
    }

    #[test]
    fn test_high_priority_fee_outlier() {
        let mut block_txs = vec![];
        for i in 0..10 {
            block_txs.push(create_test_tx(
                Address::ZERO,
                Some(1_000_000_000 + i * 100_000_000), // 1-2 gwei range
            ));
        }

        // Add outlier with 10 gwei
        let outlier = create_test_tx(Address::ZERO, Some(10_000_000_000));
        let reasons = detect_mev_patterns(&outlier, &block_txs, 10);
        assert!(reasons.contains(&DbMevReasonCode::HighPriorityFee));
    }

    #[test]
    fn test_repeated_sender() {
        let sender = Address::from([1; 20]);
        let mut block_txs = vec![];
        for _ in 0..3 {
            block_txs.push(create_test_tx(sender, Some(1_000_000_000)));
        }

        let reasons = detect_mev_patterns(&block_txs[0], &block_txs, 0);
        assert!(reasons.contains(&DbMevReasonCode::RepeatedSender));
    }
}

