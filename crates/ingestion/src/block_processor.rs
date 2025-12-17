//! Block processing and storage logic.

use chrono::DateTime;
use mev_africa_db::DbPool;
use mev_africa_telemetry::{Metrics, audit};
use rust_decimal::Decimal;
use serde::Serialize;
use serde_json::Value;
use sqlx::Row;
use tracing::{error, info, warn};
use crate::validator_tagger::ValidatorTagger;

/// Block processor for ingesting and storing blocks.
pub struct BlockProcessor {
    db: DbPool,
    metrics: Metrics,
    validator_tagger: ValidatorTagger,
    sample_output_path: Option<String>,
}

#[derive(Debug, Serialize)]
struct AuditBlock {
    block_number: u64,
    block_hash: String,
    fee_recipient: String,
    is_africa_tagged: bool,
    transaction_count: usize,
    mev_candidate_count: usize,
}

impl BlockProcessor {
    /// Create a new block processor.
    ///
    /// # Arguments
    /// * `db` - Database pool
    /// * `metrics` - Metrics collector
    /// * `validator_tagger` - Validator tagger
    /// * `sample_output_path` - Optional path for audit samples
    pub fn new(
        db: DbPool,
        metrics: Metrics,
        validator_tagger: ValidatorTagger,
        sample_output_path: Option<String>,
    ) -> Self {
        Self {
            db,
            metrics,
            validator_tagger,
            sample_output_path,
        }
    }

    /// Process and store a block.
    ///
    /// # Arguments
    /// * `block_json` - The block JSON data from RPC
    pub async fn process_block(&self, block_json: &Value) -> anyhow::Result<()> {
        // Extract block fields from JSON
        let block_number_hex = block_json["number"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Block missing number"))?;
        let block_number = u64::from_str_radix(
            block_number_hex.strip_prefix("0x").unwrap_or(block_number_hex),
            16,
        )?;

        info!("Processing block {}", block_number);

        let block_hash = block_json["hash"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Block missing hash"))?
            .to_string();
        let parent_hash = block_json["parentHash"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Block missing parentHash"))?
            .to_string();
        
        let timestamp_hex = block_json["timestamp"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Block missing timestamp"))?;
        let timestamp = DateTime::from_timestamp(
            i64::from_str_radix(
                timestamp_hex.strip_prefix("0x").unwrap_or(timestamp_hex),
                16,
            )?,
            0,
        ).ok_or_else(|| anyhow::anyhow!("Invalid timestamp"))?;

        let fee_recipient = block_json["miner"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Block missing miner"))?
            .to_string();
        let is_africa_tagged = self.validator_tagger.is_africa_tagged(&fee_recipient);

        let base_fee = block_json["baseFeePerGas"]
            .as_str()
            .map(|s| {
                u64::from_str_radix(s.strip_prefix("0x").unwrap_or(s), 16)
                    .unwrap_or(0)
                    .to_string()
            })
            .unwrap_or_else(|| "0".to_string());

        let gas_used_hex = block_json["gasUsed"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Block missing gasUsed"))?;
        let gas_used = i64::from_str_radix(
            gas_used_hex.strip_prefix("0x").unwrap_or(gas_used_hex),
            16,
        )?;

        // Extract transactions
        let transactions_json = block_json["transactions"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("Block missing transactions array"))?;
        
        // For now, we'll process transactions as JSON and extract what we need
        // Calculate total priority fees from transactions
        let mut total_priority_fees = Decimal::ZERO;
        let mut transactions_data = Vec::new();
        
        for tx_json in transactions_json {
            if let Some(priority_fee_hex) = tx_json["maxPriorityFeePerGas"].as_str() {
                if let Ok(priority_fee) = u64::from_str_radix(
                    priority_fee_hex.strip_prefix("0x").unwrap_or(priority_fee_hex),
                    16,
                ) {
                    total_priority_fees += Decimal::from(priority_fee);
                }
            }
            transactions_data.push(tx_json.clone());
        }

        // Store block
        let block_id = sqlx::query(
            r#"
            INSERT INTO blocks (
                block_number, block_hash, parent_hash, timestamp,
                fee_recipient, base_fee, gas_used, total_priority_fees,
                is_africa_tagged
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            RETURNING id
            "#,
        )
        .bind(block_number as i64)
        .bind(&block_hash)
        .bind(&parent_hash)
        .bind(timestamp.to_rfc3339())
        .bind(&fee_recipient)
        .bind(&base_fee)
        .bind(gas_used)
        .bind(total_priority_fees.to_string())
        .bind(is_africa_tagged)
        .fetch_one(self.db.pool())
        .await?
        .get::<i64, _>(0);

        // Process transactions
        let mut mev_candidate_count = 0;
        for (index, tx_json) in transactions_data.iter().enumerate() {
            match self.process_transaction_json(block_id, tx_json, &transactions_data, index).await {
                Ok(is_mev) => {
                    if is_mev {
                        mev_candidate_count += 1;
                    }
                }
                Err(e) => {
                    error!("Failed to process transaction {} in block {}: {}", index, block_number, e);
                }
            }
        }

        // Update builder table
        self.update_builder(&fee_recipient).await?;

        // Update metrics
        self.metrics.inc_blocks_processed();
        self.metrics.inc_transactions_processed(transactions_data.len() as u64);
        self.metrics.inc_mev_candidates(mev_candidate_count);
        if is_africa_tagged {
            self.metrics.inc_africa_tagged_blocks();
        }

        // Write audit sample
        let audit_block = AuditBlock {
            block_number,
            block_hash,
            fee_recipient,
            is_africa_tagged,
            transaction_count: transactions_data.len(),
            mev_candidate_count: mev_candidate_count as usize,
        };

        if let Some(ref path) = self.sample_output_path {
            if let Err(e) = audit::write_audit_sample(Some(path), &audit_block) {
                warn!("Failed to write audit sample: {}", e);
            }
        }

        info!(
            "Processed block {}: {} transactions, {} MEV candidates, Africa tagged: {}",
            block_number,
            transactions_data.len(),
            mev_candidate_count,
            is_africa_tagged
        );

        Ok(())
    }

    async fn process_transaction_json(
        &self,
        block_id: i64,
        tx_json: &Value,
        block_txs: &[Value],
        tx_index: usize,
    ) -> anyhow::Result<bool> {
        // Extract transaction data from JSON
        let tx_hash = tx_json["hash"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Transaction missing hash"))?
            .to_string();
        let position_index = tx_index as i64;
        let sender_address = tx_json["from"]
            .as_str()
            .unwrap_or("unknown")
            .to_string();

        let max_priority_fee = tx_json["maxPriorityFeePerGas"]
            .as_str()
            .map(|s| {
                u64::from_str_radix(s.strip_prefix("0x").unwrap_or(s), 16)
                    .unwrap_or(0)
                    .to_string()
            })
            .unwrap_or_else(|| "0".to_string());

        // MEV detection heuristics
        let mut mev_reasons = Vec::new();
        
        // 1. High priority fee outlier (check if >3x block median)
        let priority_fee_value = max_priority_fee.parse::<u64>().unwrap_or(0);
        if priority_fee_value > 0 {
            // Calculate median priority fee for the block
            let mut fees: Vec<u64> = block_txs
                .iter()
                .filter_map(|tx| {
                    tx["maxPriorityFeePerGas"]
                        .as_str()
                        .and_then(|s| u64::from_str_radix(s.strip_prefix("0x").unwrap_or(s), 16).ok())
                })
                .collect();
            
            if !fees.is_empty() {
                fees.sort();
                let median = if fees.len() % 2 == 0 {
                    (fees[fees.len() / 2 - 1] + fees[fees.len() / 2]) / 2
                } else {
                    fees[fees.len() / 2]
                };
                
                if priority_fee_value > median * 3 && median > 0 {
                    mev_reasons.push("high_priority_fee_outlier");
                }
            }
        }
        
        // 2. Repeated sender (check if sender appears 3+ times in block)
        let sender_count = block_txs
            .iter()
            .filter(|tx| tx["from"].as_str() == tx_json["from"].as_str())
            .count();
        if sender_count >= 3 {
            mev_reasons.push("repeated_sender_sequence");
        }
        
        // 3. Atomic multiswap (check for multiple swap patterns in calldata)
        if let Some(input) = tx_json["input"].as_str() {
            let swap_patterns = ["022c0d9f", "472b43f3", "5c11d795", "7ff36ab5", "414bf389"];
            let pattern_count = swap_patterns
                .iter()
                .filter(|pattern| input.contains(*pattern))
                .count();
            if pattern_count >= 2 {
                mev_reasons.push("atomic_multiswap");
            }
        }
        
        // 4. Sandwich pattern (same sender before and after this tx)
        let tx_sender = tx_json["from"].as_str();
        if let Some(sender) = tx_sender {
            let has_before = block_txs[..tx_index]
                .iter()
                .any(|tx| tx["from"].as_str() == Some(sender));
            let has_after = block_txs[tx_index + 1..]
                .iter()
                .any(|tx| tx["from"].as_str() == Some(sender));
            if has_before && has_after {
                mev_reasons.push("sandwich_pattern");
            }
        }
        
        let is_mev_candidate = !mev_reasons.is_empty();
        let mev_reason_codes = if is_mev_candidate {
            Some(serde_json::to_string(&mev_reasons)?)
        } else {
            None
        };
        
        // Extract calldata summary
        let calldata_summary = tx_json["input"]
            .as_str()
            .map(|input| {
                if input.len() > 200 {
                    format!("{}...", &input[..200])
                } else {
                    input.to_string()
                }
            });

        // Store transaction
        sqlx::query(
            r#"
            INSERT INTO transactions (
                block_id, tx_hash, position_index, sender_address,
                max_priority_fee, calldata_summary, log_summary,
                is_mev_candidate, mev_reason_codes
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(block_id)
        .bind(&tx_hash)
        .bind(position_index)
        .bind(&sender_address)
        .bind(&max_priority_fee)
        .bind(calldata_summary.as_ref())
        .bind(None::<String>)
        .bind(is_mev_candidate)
        .bind(mev_reason_codes.as_ref())
        .execute(self.db.pool())
        .await?;

        Ok(is_mev_candidate)
    }

    async fn update_builder(&self, fee_recipient: &str) -> anyhow::Result<()> {
        // Check if builder exists
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM builders WHERE fee_recipient = ?)"
        )
        .bind(fee_recipient)
        .fetch_one(self.db.pool())
        .await?;

        if !exists {
            // Insert as unknown builder
            sqlx::query(
                "INSERT OR IGNORE INTO builders (fee_recipient, is_known) VALUES (?, 0)"
            )
            .bind(fee_recipient)
            .execute(self.db.pool())
            .await?;
        }

        Ok(())
    }
}

