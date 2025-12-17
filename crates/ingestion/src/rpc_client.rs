//! Ethereum RPC client for block ingestion.

use anyhow::Result;
use reqwest::Client;
use serde_json::{json, Value};
use tokio::time::Instant;
use tracing::{debug, info};
use mev_africa_telemetry::Metrics;

/// Ethereum RPC client wrapper.
pub struct RpcClient {
    client: Client,
    rpc_url: String,
    metrics: Metrics,
}

impl RpcClient {
    /// Create a new RPC client.
    ///
    /// # Arguments
    /// * `rpc_url` - HTTP/HTTPS JSON-RPC endpoint URL (e.g., Chainstack endpoint)
    /// * `metrics` - Metrics collector
    pub fn new(rpc_url: &str, metrics: Metrics) -> Result<Self> {
        info!("Initialized RPC client for {}", rpc_url);

        Ok(Self {
            client: Client::new(),
            rpc_url: rpc_url.to_string(),
            metrics,
        })
    }

    async fn call_rpc(&self, method: &str, params: Value) -> Result<Value> {
        let payload = json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
            "id": 1
        });

        let response = self.client
            .post(&self.rpc_url)
            .json(&payload)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("RPC request failed with status: {}", response.status()));
        }

        let result: Value = response.json().await?;
        
        // Check for RPC error
        if let Some(error) = result.get("error") {
            return Err(anyhow::anyhow!("RPC error: {}", error));
        }

        Ok(result["result"].clone())
    }

    /// Get the latest block number.
    pub async fn get_latest_block_number(&self) -> Result<u64> {
        let start = Instant::now();
        let result = self.call_rpc("eth_blockNumber", json!([])).await?;
        let duration = start.elapsed().as_secs_f64();
        self.metrics.observe_rpc_latency("get_block_number", duration);

        let hex_str = result.as_str().ok_or_else(|| anyhow::anyhow!("Invalid response"))?;
        let block_num = u64::from_str_radix(hex_str.strip_prefix("0x").unwrap_or(hex_str), 16)?;
        debug!("Latest block number: {}", block_num);
        Ok(block_num)
    }

    /// Get a block by number with full transaction details.
    /// Returns the raw JSON block data for flexible parsing.
    pub async fn get_block(&self, block_number: u64) -> Result<Option<Value>> {
        let start = Instant::now();
        let hex_block = format!("0x{:x}", block_number);
        let result = self.call_rpc("eth_getBlockByNumber", json!([hex_block, true])).await?;
        let duration = start.elapsed().as_secs_f64();
        self.metrics.observe_rpc_latency("get_block", duration);

        if result.is_null() {
            return Ok(None);
        }

        debug!("Fetched block {}", block_number);
        Ok(Some(result))
    }
}

