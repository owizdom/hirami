//! Database models and types.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Block data stored in the database.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Block {
    pub id: i64,
    pub block_number: i64,
    pub block_hash: String,
    pub parent_hash: String,
    pub timestamp: DateTime<Utc>,
    pub fee_recipient: String,
    pub base_fee: String, // Stored as string to preserve precision
    pub gas_used: i64,
    pub total_priority_fees: String, // Stored as string to preserve precision
    pub is_africa_tagged: bool,
    pub created_at: DateTime<Utc>,
}

/// Transaction data stored in the database.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Transaction {
    pub id: i64,
    pub block_id: i64,
    pub tx_hash: String,
    pub position_index: i64,
    pub sender_address: String,
    pub max_priority_fee: String,
    pub calldata_summary: Option<String>,
    pub log_summary: Option<String>,
    pub is_mev_candidate: bool,
    pub mev_reason_codes: Option<String>, // JSON array of reason codes
    pub created_at: DateTime<Utc>,
}

/// Builder information.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Builder {
    pub id: i64,
    pub fee_recipient: String,
    pub builder_name: Option<String>,
    pub is_known: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Validator information from CSV import.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Validator {
    pub id: i64,
    pub fee_recipient: String,
    pub validator_pubkey: String,
    pub operator_name: Option<String>,
    pub country: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Custom annotation for blocks or transactions.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Annotation {
    pub id: i64,
    pub block_id: Option<i64>,
    pub transaction_id: Option<i64>,
    pub tag: String,
    pub note: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// MEV reason codes for transaction classification.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MevReasonCode {
    HighPriorityFee,
    RepeatedSender,
    AtomicMultiswap,
    SandwichPattern,
}

impl MevReasonCode {
    pub fn as_str(&self) -> &'static str {
        match self {
            MevReasonCode::HighPriorityFee => "high_priority_fee",
            MevReasonCode::RepeatedSender => "repeated_sender",
            MevReasonCode::AtomicMultiswap => "atomic_multiswap",
            MevReasonCode::SandwichPattern => "sandwich_pattern",
        }
    }
}

