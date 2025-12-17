//! Structured logging setup.

use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

/// Initialize structured logging with environment-based filtering.
///
/// # Arguments
/// * `log_level` - Optional log level override (e.g., "info", "debug", "error")
pub fn init_logging(log_level: Option<&str>) -> anyhow::Result<()> {
    let filter = if let Some(level) = log_level {
        EnvFilter::new(level)
    } else {
        EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new("info"))
    };

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().json())
        .init();

    Ok(())
}



