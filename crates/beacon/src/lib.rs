//! Beacon chain integration interface for MEV Africa data collection.
//!
//! This module provides a trait-based interface for beacon chain adapters
//! that can map slots to proposers and proposers to validator pubkeys.
//! The core ingestion does not depend on this at compile time.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Represents a validator public key (BLS12-381 public key as hex string).
pub type ValidatorPubkey = String;

/// Represents a slot number in the beacon chain.
pub type Slot = u64;

/// Represents a proposer index in the beacon chain.
pub type ProposerIndex = u64;

/// Error type for beacon chain operations.
#[derive(Debug, thiserror::Error)]
pub enum BeaconError {
    #[error("Beacon adapter not available")]
    NotAvailable,
    #[error("Slot not found: {0}")]
    SlotNotFound(Slot),
    #[error("Proposer not found: {0}")]
    ProposerNotFound(ProposerIndex),
    #[error("Network error: {0}")]
    Network(#[from] anyhow::Error),
}

/// Result type for beacon chain operations.
pub type BeaconResult<T> = Result<T, BeaconError>;

/// Information about a block proposer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposerInfo {
    /// The proposer index.
    pub index: ProposerIndex,
    /// The validator public key.
    pub pubkey: ValidatorPubkey,
}

/// Trait for beacon chain adapters.
///
/// This trait allows different implementations of beacon chain clients
/// to be swapped in without modifying the core ingestion code.
#[async_trait]
pub trait BeaconAdapter: Send + Sync {
    /// Get the proposer information for a given slot.
    ///
    /// # Arguments
    /// * `slot` - The beacon chain slot number
    ///
    /// # Returns
    /// The proposer information, or an error if the slot is not found
    /// or the adapter is not available.
    async fn get_proposer_for_slot(&self, slot: Slot) -> BeaconResult<ProposerInfo>;

    /// Get the validator public key for a given proposer index.
    ///
    /// # Arguments
    /// * `proposer_index` - The proposer index
    ///
    /// # Returns
    /// The validator public key, or an error if the proposer is not found.
    async fn get_validator_pubkey(&self, proposer_index: ProposerIndex) -> BeaconResult<ValidatorPubkey>;
}

/// Placeholder beacon adapter that always returns `NotAvailable`.
///
/// This adapter can be used when a beacon node is not available.
/// To use a real beacon node, implement the `BeaconAdapter` trait
/// for a type that connects to a beacon node REST API (e.g., Lighthouse,
/// Prysm, or Teku).
pub struct PlaceholderBeaconAdapter;

#[async_trait]
impl BeaconAdapter for PlaceholderBeaconAdapter {
    async fn get_proposer_for_slot(&self, _slot: Slot) -> BeaconResult<ProposerInfo> {
        Err(BeaconError::NotAvailable)
    }

    async fn get_validator_pubkey(&self, _proposer_index: ProposerIndex) -> BeaconResult<ValidatorPubkey> {
        Err(BeaconError::NotAvailable)
    }
}

