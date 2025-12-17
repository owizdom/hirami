//! Integration tests for MEV Africa ingestion service.

#[cfg(test)]
mod tests {
    use mev_africa_db::DbPool;
    use mev_africa_heuristics::detectors::detect_mev_patterns;
    use alloy::rpc::types::Transaction;
    use alloy::primitives::{Address, U256};

    fn create_test_tx(sender: Address, priority_fee: Option<u64>) -> Transaction {
        Transaction {
            hash: Default::default(),
            nonce: 0,
            block_hash: None,
            block_number: None,
            transaction_index: None,
            from: Some(sender),
            to: None,
            value: U256::ZERO,
            gas_price: None,
            max_fee_per_gas: None,
            max_priority_fee_per_gas: priority_fee.map(U256::from),
            gas: U256::ZERO,
            input: None,
            chain_id: None,
            v: 0,
            r: Default::default(),
            s: Default::default(),
            access_list: None,
            transaction_type: None,
        }
    }

    #[tokio::test]
    async fn test_database_creation() {
        let db = DbPool::new(":memory:").await.unwrap();
        db.migrate().await.unwrap();
        // If we get here, migration succeeded
    }

    #[test]
    fn test_mev_detection_high_priority_fee() {
        let mut block_txs = vec![];
        for i in 0..10 {
            block_txs.push(create_test_tx(
                Address::ZERO,
                Some(1_000_000_000 + i * 100_000_000),
            ));
        }

        let outlier = create_test_tx(Address::ZERO, Some(10_000_000_000));
        let reasons = detect_mev_patterns(&outlier, &block_txs, 10);
        assert!(!reasons.is_empty());
    }

    #[test]
    fn test_mev_detection_repeated_sender() {
        let sender = Address::from([1; 20]);
        let mut block_txs = vec![];
        for _ in 0..3 {
            block_txs.push(create_test_tx(sender, Some(1_000_000_000)));
        }

        let reasons = detect_mev_patterns(&block_txs[0], &block_txs, 0);
        assert!(!reasons.is_empty());
    }
}



