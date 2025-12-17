//! Audit logging for sample payloads.

use serde::Serialize;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use tracing::info;

/// Write a sample JSON payload to the audit file.
///
/// # Arguments
/// * `path` - Path to the audit file
/// * `payload` - Serializable payload to write
pub fn write_audit_sample<P: AsRef<Path>, T: Serialize>(
    path: Option<P>,
    payload: &T,
) -> anyhow::Result<()> {
    if let Some(audit_path) = path {
        let json = serde_json::to_string_pretty(payload)?;
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&audit_path)?;
        writeln!(file, "{}", json)?;
        info!("Wrote audit sample to {:?}", audit_path.as_ref());
    }
    Ok(())
}

