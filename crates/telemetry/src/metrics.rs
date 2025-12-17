//! Prometheus metrics for MEV Africa data collection.

use prometheus::{
    register_histogram_vec, register_int_counter, HistogramVec, IntCounter,
    Encoder, TextEncoder,
};

/// Metrics collector for the MEV Africa service.
#[derive(Clone)]
pub struct Metrics {
    blocks_processed: IntCounter,
    transactions_processed: IntCounter,
    mev_candidate_count: IntCounter,
    africa_tagged_blocks: IntCounter,
    rpc_errors: IntCounter,
    rpc_latency: HistogramVec,
}

impl Metrics {
    /// Create a new metrics instance.
    pub fn new() -> anyhow::Result<Self> {
        let blocks_processed = register_int_counter!(
            "mev_africa_blocks_processed_total",
            "Total number of blocks processed"
        )?;

        let transactions_processed = register_int_counter!(
            "mev_africa_transactions_processed_total",
            "Total number of transactions processed"
        )?;

        let mev_candidate_count = register_int_counter!(
            "mev_africa_mev_candidates_total",
            "Total number of MEV candidate transactions detected"
        )?;

        let africa_tagged_blocks = register_int_counter!(
            "mev_africa_africa_tagged_blocks_total",
            "Total number of blocks tagged as Africa-related"
        )?;

        let rpc_errors = register_int_counter!(
            "mev_africa_rpc_errors_total",
            "Total number of RPC errors"
        )?;

        let rpc_latency = register_histogram_vec!(
            "mev_africa_rpc_latency_seconds",
            "RPC call latency in seconds",
            &["operation"]
        )?;

        Ok(Self {
            blocks_processed,
            transactions_processed,
            mev_candidate_count,
            africa_tagged_blocks,
            rpc_errors,
            rpc_latency,
        })
    }

    /// Increment the blocks processed counter.
    pub fn inc_blocks_processed(&self) {
        self.blocks_processed.inc();
    }

    /// Increment the transactions processed counter.
    pub fn inc_transactions_processed(&self, count: u64) {
        self.transactions_processed.inc_by(count);
    }

    /// Increment the MEV candidate counter.
    pub fn inc_mev_candidates(&self, count: u64) {
        self.mev_candidate_count.inc_by(count);
    }

    /// Increment the Africa tagged blocks counter.
    pub fn inc_africa_tagged_blocks(&self) {
        self.africa_tagged_blocks.inc();
    }

    /// Increment the RPC errors counter.
    pub fn inc_rpc_errors(&self) {
        self.rpc_errors.inc();
    }

    /// Record RPC latency.
    pub fn observe_rpc_latency(&self, operation: &str, duration_secs: f64) {
        self.rpc_latency.with_label_values(&[operation]).observe(duration_secs);
    }

    /// Get Prometheus metrics as a string.
    pub fn gather(&self) -> anyhow::Result<String> {
        let encoder = TextEncoder::new();
        let metric_families = prometheus::gather();
        let mut buffer = Vec::new();
        encoder.encode(&metric_families, &mut buffer)?;
        Ok(String::from_utf8(buffer)?)
    }
}

impl Default for Metrics {
    fn default() -> Self {
        Self::new().expect("Failed to create metrics")
    }
}

