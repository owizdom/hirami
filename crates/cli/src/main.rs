//! CLI application for MEV Africa data collection service.

use clap::{Parser, Subcommand};
use mev_africa_db::DbPool;
use mev_africa_ingestion::{BlockProcessor, RpcClient};
use mev_africa_ingestion::validator_tagger::ValidatorTagger;
use mev_africa_telemetry::{init_logging, Metrics};
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, error, info, warn};

#[derive(Parser)]
#[command(name = "mev-africa")]
#[command(about = "MEV data collection service for Ethereum validators in Africa")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the ingestion service
    Ingest {
        /// Ethereum execution RPC URL
        #[arg(long, default_value = "https://ethereum-mainnet.core.chainstack.com/390f7fa4351543e290dc3e4bf9d9058f")]
        execution_rpc_url: String,

        /// Database path
        #[arg(long, default_value = "mev_africa.db")]
        database_path: String,

        /// Africa validators CSV path
        #[arg(long, default_value = "examples/africa_validators_example.csv")]
        africa_validators_csv: String,

        /// Poll interval in seconds
        #[arg(long, default_value = "12")]
        poll_interval_seconds: u64,

        /// Metrics bind address
        #[arg(long, default_value = "0.0.0.0:9090")]
        metrics_bind_address: String,

        /// Log level
        #[arg(long)]
        log_level: Option<String>,

        /// Sample output path for audit logs
        #[arg(long)]
        sample_output_path: Option<String>,

        /// Start from latest block instead of catching up from database
        #[arg(long, default_value = "false")]
        start_from_latest: bool,
    },
    /// Import or refresh Africa validators CSV
    ImportValidators {
        /// Database path
        #[arg(long, default_value = "mev_africa.db")]
        database_path: String,

        /// Africa validators CSV path
        #[arg(long, default_value = "examples/africa_validators_example.csv")]
        africa_validators_csv: String,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Ingest {
            execution_rpc_url,
            database_path,
            africa_validators_csv,
            poll_interval_seconds,
            metrics_bind_address,
            log_level,
            sample_output_path,
            start_from_latest,
        } => {
            init_logging(log_level.as_deref())?;
            run_ingestion(
                &execution_rpc_url,
                &database_path,
                &africa_validators_csv,
                poll_interval_seconds,
                &metrics_bind_address,
                sample_output_path,
                start_from_latest,
            )
            .await?;
        }
        Commands::ImportValidators {
            database_path,
            africa_validators_csv,
        } => {
            init_logging(None)?;
            import_validators(&database_path, &africa_validators_csv).await?;
        }
    }

    Ok(())
}

async fn run_ingestion(
    rpc_url: &str,
    db_path: &str,
    validators_csv: &str,
    poll_interval: u64,
    metrics_addr: &str,
    sample_output_path: Option<String>,
    start_from_latest: bool,
) -> anyhow::Result<()> {
    info!("Starting MEV Africa ingestion service");

    // Initialize database
    let db = DbPool::new(db_path).await?;
    db.migrate().await?;

    // Import validators if CSV exists
    if std::path::Path::new(validators_csv).exists() {
        import_validators(db_path, validators_csv).await?;
    } else {
        warn!("Validators CSV not found at {}, continuing without Africa tagging", validators_csv);
    }

    // Initialize components
    let metrics = Metrics::new()?;
    let rpc_client = RpcClient::new(rpc_url, metrics.clone())?;
    let validator_tagger = ValidatorTagger::new(&db).await?;
    let processor = BlockProcessor::new(db.clone(), metrics.clone(), validator_tagger, sample_output_path);

    // Start metrics server
    start_metrics_server(metrics_addr, metrics.clone()).await?;

    // Main ingestion loop
    let mut last_block = if start_from_latest {
        // Start from current latest block
        let latest = rpc_client.get_latest_block_number().await?;
        info!("Starting from latest block: {}", latest);
        latest
    } else {
        // Start from last processed block in database
        get_last_processed_block(&db).await?
    };
    let poll_duration = Duration::from_secs(poll_interval);

    loop {
        match rpc_client.get_latest_block_number().await {
            Ok(latest_block) => {
                if latest_block > last_block {
                    info!("Processing blocks from {} to {}", last_block + 1, latest_block);
                    for block_num in (last_block + 1)..=latest_block {
                        match rpc_client.get_block(block_num).await {
                            Ok(Some(block_json)) => {
                                if let Err(e) = processor.process_block(&block_json).await {
                                    error!("Failed to process block {}: {}", block_num, e);
                                } else {
                                    last_block = block_num;
                                }
                            }
                            Ok(None) => {
                                warn!("Block {} not found", block_num);
                            }
                            Err(e) => {
                                error!("Failed to fetch block {}: {}", block_num, e);
                            }
                        }
                    }
                } else {
                    debug!("No new blocks, latest: {}", latest_block);
                }
            }
            Err(e) => {
                error!("Failed to get latest block number: {}", e);
            }
        }

        sleep(poll_duration).await;
    }
}

async fn get_last_processed_block(db: &DbPool) -> anyhow::Result<u64> {
    let result: Option<i64> = sqlx::query_scalar(
        "SELECT MAX(block_number) FROM blocks"
    )
    .fetch_optional(db.pool())
    .await?;

    Ok(result.unwrap_or(0) as u64)
}

async fn import_validators(db_path: &str, csv_path: &str) -> anyhow::Result<()> {
    info!("Importing validators from {}", csv_path);

    let db = DbPool::new(db_path).await?;
    let mut reader = csv::Reader::from_path(csv_path)?;

    let mut count = 0;
    for result in reader.deserialize() {
        let record: ValidatorRecord = result?;
        
        sqlx::query(
            r#"
            INSERT OR REPLACE INTO validators (
                fee_recipient, validator_pubkey, operator_name, country, updated_at
            ) VALUES (?, ?, ?, ?, datetime('now'))
            "#,
        )
        .bind(record.fee_recipient.to_lowercase())
        .bind(record.validator_pubkey)
        .bind(record.operator_name)
        .bind(record.country)
        .execute(db.pool())
        .await?;

        count += 1;
    }

    info!("Imported {} validators", count);
    Ok(())
}

#[derive(serde::Deserialize)]
struct ValidatorRecord {
    fee_recipient: String,
    validator_pubkey: String,
    operator_name: Option<String>,
    country: Option<String>,
}

async fn start_metrics_server(addr: &str, metrics: Metrics) -> anyhow::Result<()> {
    use axum::{
        extract::State,
        http::StatusCode,
        response::IntoResponse,
        routing::get,
        Router,
    };
    use std::sync::Arc;
    
    let metrics = Arc::new(metrics);
    
    async fn metrics_handler(
        State(metrics): State<Arc<Metrics>>,
    ) -> Result<impl IntoResponse, StatusCode> {
        match metrics.gather() {
            Ok(body) => Ok((StatusCode::OK, body)),
            Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
        }
    }
    
    let app = Router::new()
        .route("/metrics", get(metrics_handler))
        .with_state(metrics);
    
    let listener = tokio::net::TcpListener::bind(addr).await?;
    info!("Metrics server listening on http://{}", addr);
    
    tokio::spawn(async move {
        if let Err(e) = axum::serve(listener, app).await {
            error!("Metrics server error: {}", e);
        }
    });

    Ok(())
}


