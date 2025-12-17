# Hirami

A Rust-based MEV (Maximal Extractable Value) data collection service focused on Ethereum validators in Africa. Hirami is designed for long-term research and emphasizes observability and data integrity.

## Features

- **Execution Layer Block Ingestion**: Continuously polls Ethereum execution nodes and stores comprehensive block data
- **MEV Signal Extraction**: Implements heuristic detectors for identifying potential MEV transactions
- **Builder Attribution**: Tracks fee recipients and maintains a builders table
- **Africa Validator Tagging**: Tags blocks associated with Africa-based validators via CSV import
- **Beacon Chain Interface**: Trait-based interface for future beacon chain integration
- **Observability**: Structured logging with tracing and Prometheus metrics
- **Storage**: SQLite database with schema versioning

## Architecture

Hirami is organized as a Cargo workspace with the following crates:

- `mev-africa-beacon`: Beacon chain integration trait interface
- `mev-africa-db`: Database layer with SQLite schema and migrations
- `mev-africa-heuristics`: MEV detection heuristics
- `mev-africa-ingestion`: Core block ingestion service
- `mev-africa-telemetry`: Observability (metrics, logging, audit)
- `mev-africa-cli`: CLI application and configuration

## Prerequisites

- Rust 1.70+ (with cargo)
- Access to an Ethereum execution node (HTTP JSON-RPC endpoint)
- SQLite 3.x

## Installation

1. Clone the repository:
```bash
git clone <repository-url>
cd mevAfrica
```

2. Build the project:
```bash
cargo build --release
```

## Configuration

All configuration is done via environment variables or CLI arguments.

### Required Settings

- `EXECUTION_RPC_URL`: Ethereum execution node RPC URL (e.g., `https://ethereum-mainnet.core.chainstack.com/390f7fa4351543e290dc3e4bf9d9058f`)
- `DATABASE_PATH`: Path to SQLite database file (default: `mev_africa.db`)
- `AFRICA_VALIDATORS_CSV`: Path to CSV file with Africa validator mappings
- `POLL_INTERVAL_SECONDS`: Block polling interval in seconds (default: 12)

### Optional Settings

- `METRICS_BIND_ADDRESS`: Prometheus metrics endpoint address (default: `0.0.0.0:9090`)
- `LOG_LEVEL`: Logging level (info, debug, error) - defaults to info
- `SAMPLE_OUTPUT_PATH`: Path for audit sample JSON output
- `START_FROM_LATEST`: Start from latest block instead of catching up from database (default: `false`)

## Usage

### Import Africa Validators

First, import the Africa validators CSV:

```bash
cargo run --bin mev-africa -- import-validators \
  --database-path mev_africa.db \
  --africa-validators-csv validators.csv
```

The CSV should have the following columns:
- `fee_recipient`: Fee recipient address
- `validator_pubkey`: Validator public key
- `operator_name`: Optional operator name
- `country`: Optional country code

See `examples/africa_validators_example.csv` for an example format.

### Start Ingestion Service

Start from the latest block (recommended for real-time monitoring):

```bash
cargo run --bin mev-africa -- ingest \
  --execution-rpc-url https://ethereum-mainnet.core.chainstack.com/390f7fa4351543e290dc3e4bf9d9058f \
  --database-path mev_africa.db \
  --africa-validators-csv examples/africa_validators_example.csv \
  --poll-interval-seconds 12 \
  --metrics-bind-address 0.0.0.0:9090 \
  --start-from-latest
```

Or start from the last processed block in the database (for catch-up):

```bash
cargo run --bin mev-africa -- ingest \
  --execution-rpc-url https://ethereum-mainnet.core.chainstack.com/390f7fa4351543e290dc3e4bf9d9058f \
  --database-path mev_africa.db \
  --africa-validators-csv examples/africa_validators_example.csv
```

Using environment variables:

```bash
export EXECUTION_RPC_URL=https://ethereum-mainnet.core.chainstack.com/390f7fa4351543e290dc3e4bf9d9058f
export DATABASE_PATH=mev_africa.db
export AFRICA_VALIDATORS_CSV=examples/africa_validators_example.csv
export POLL_INTERVAL_SECONDS=12
export START_FROM_LATEST=true

cargo run --bin mev-africa -- ingest
```

## Database Schema

The service creates the following tables:

- `blocks`: Block data (number, hash, fee recipient, etc.)
- `transactions`: Transaction data with MEV flags
- `builders`: Builder fee recipient mappings
- `validators`: Africa validator mappings from CSV
- `annotations`: Custom tags and notes

See `crates/db/migrations/001_initial_schema.sql` for the full schema.

## MEV Detection Heuristics

Hirami implements the following MEV detection heuristics:

1. **High Priority Fee Outlier**: Flags transactions with priority fees >3x the block median
2. **Repeated Sender Sequence**: Flags senders appearing 3+ times in the same block (potential bot activity)
3. **Atomic Multiswap**: Detects multiple swap patterns in transaction calldata (common in MEV strategies)
4. **Sandwich Pattern**: Detects front-run and back-run patterns from the same sender

Each detected MEV transaction is stored with reason codes (JSON array) explaining why it was flagged. These are heuristic detectors and may produce false positives. They are designed for research purposes, not perfect classification.

**MEV Detection Status**: ✅ Active and working - transactions are being analyzed and flagged in real-time.

## Metrics

Prometheus metrics are exposed at `/metrics` on the configured bind address (default: `http://localhost:9090/metrics`). Available metrics:

- `mev_africa_blocks_processed_total`: Total blocks processed
- `mev_africa_transactions_processed_total`: Total transactions processed
- `mev_africa_mev_candidates_total`: Total MEV candidate transactions detected
- `mev_africa_africa_tagged_blocks_total`: Total Africa-tagged blocks (requires African validator addresses in CSV)
- `mev_africa_rpc_errors_total`: Total RPC errors
- `mev_africa_rpc_latency_seconds`: RPC call latency histogram (by operation)

**Example**: View metrics with `curl http://localhost:9090/metrics`

## Testing

Run unit tests:

```bash
cargo test
```

Run integration tests:

```bash
cargo test --test integration_test
```

## Docker

### Using Docker Compose (Development with Anvil)

For local development with a mock execution node:

```bash
docker-compose up
```

This will start:
- Hirami ingestion service
- A mock execution node (Anvil) for testing
- Prometheus metrics endpoint

### Using Docker Compose (Production with Chainstack)

For production use with the Chainstack Ethereum mainnet endpoint:

```bash
docker-compose -f docker-compose.prod.yml up
```

This will start:
- Hirami ingestion service connected to Chainstack
- Prometheus metrics endpoint

### Building Docker Image

```bash
docker build -t hirami:latest .
```

## Current Status

✅ **Working Features:**
- Block ingestion from Ethereum mainnet
- MEV detection with 4 heuristic patterns
- Real-time metrics and logging
- SQLite database storage
- CSV export of MEV candidates
- Start from latest block option (`--start-from-latest`)

⚠️ **Africa Validator Tagging:**
- Infrastructure is ready and working
- Requires real African validator fee recipient addresses in CSV
- Currently using example/placeholder addresses
- Blocks will be tagged automatically when real addresses are provided

## Extending the Service

### Adding a Beacon Chain Adapter

Hirami includes a trait-based interface for beacon chain integration. To add a real beacon adapter:

1. Implement the `BeaconAdapter` trait from `mev-africa-beacon`
2. Connect to a beacon node REST API (Lighthouse, Prysm, or Teku)
3. Swap in your implementation in the ingestion service

This can help identify validators by pubkey and map them to fee recipients.

### Adding MEV Boost Header Ingestion

The codebase is designed to support MEV Boost header ingestion for deeper MEV analysis:
- Ingest MEV Boost headers
- Record builder public keys
- Track relay metadata
- Implement header-to-payload unblinding

## Data Export

Export MEV candidates to CSV:

```bash
sqlite3 -header -csv mev_africa.db > mev_candidates.csv <<EOF
SELECT 
    t.tx_hash,
    t.sender_address,
    CAST(t.max_priority_fee AS INTEGER) as priority_fee_wei,
    ROUND(CAST(t.max_priority_fee AS INTEGER) / 1000000000.0, 4) as priority_fee_gwei,
    t.mev_reason_codes,
    b.block_number,
    b.fee_recipient,
    b.timestamp
FROM transactions t
JOIN blocks b ON t.block_id = b.id
WHERE t.is_mev_candidate = 1
ORDER BY CAST(t.max_priority_fee AS INTEGER) DESC;
EOF
```

## Privacy and Ethics

Hirami is designed for research purposes. Please observe the following:

- **Do not publish or deanonymize private validators without consent**
- **Respect privacy**: Raw IP-level data is excluded from telemetry and logs
- **Data collection**: Only collects publicly available on-chain data

## License

MIT OR Apache-2.0

## Getting African Validator Addresses

To tag blocks as Africa-related, you need real African validator fee recipient addresses. Since there's no public list, consider:

1. **Research**: Identify known African staking operators and their fee recipients
2. **Community Outreach**: Contact African validator operators directly
3. **Beacon Chain API**: Use beacon chain data to identify validators by operator name
4. **Collect First, Tag Later**: Collect all MEV data now, tag blocks retroactively when addresses are found

Once you have addresses, add them to a CSV and import:
```bash
cargo run --bin mev-africa -- import-validators \
  --database-path mev_africa.db \
  --africa-validators-csv your_africa_validators.csv
```

## Contributing

Hirami is a research tool. Contributions should focus on:
- Data integrity and correctness
- Observability improvements
- Extensibility for new data sources
- MEV detection heuristics improvements
- Documentation

## Support

For issues or questions, please open an issue in the repository.

