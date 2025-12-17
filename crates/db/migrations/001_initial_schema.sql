-- Initial schema for MEV Africa data collection
-- Schema version: 1

-- Blocks table
CREATE TABLE IF NOT EXISTS blocks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    block_number INTEGER NOT NULL UNIQUE,
    block_hash TEXT NOT NULL UNIQUE,
    parent_hash TEXT NOT NULL,
    timestamp TEXT NOT NULL,
    fee_recipient TEXT NOT NULL,
    base_fee TEXT NOT NULL,
    gas_used INTEGER NOT NULL,
    total_priority_fees TEXT NOT NULL,
    is_africa_tagged BOOLEAN NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_blocks_number ON blocks(block_number);
CREATE INDEX IF NOT EXISTS idx_blocks_hash ON blocks(block_hash);
CREATE INDEX IF NOT EXISTS idx_blocks_fee_recipient ON blocks(fee_recipient);
CREATE INDEX IF NOT EXISTS idx_blocks_africa_tagged ON blocks(is_africa_tagged);
CREATE INDEX IF NOT EXISTS idx_blocks_timestamp ON blocks(timestamp);

-- Transactions table
CREATE TABLE IF NOT EXISTS transactions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    block_id INTEGER NOT NULL,
    tx_hash TEXT NOT NULL UNIQUE,
    position_index INTEGER NOT NULL,
    sender_address TEXT NOT NULL,
    max_priority_fee TEXT NOT NULL,
    calldata_summary TEXT,
    log_summary TEXT,
    is_mev_candidate BOOLEAN NOT NULL DEFAULT 0,
    mev_reason_codes TEXT, -- JSON array of reason codes
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (block_id) REFERENCES blocks(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_transactions_block_id ON transactions(block_id);
CREATE INDEX IF NOT EXISTS idx_transactions_hash ON transactions(tx_hash);
CREATE INDEX IF NOT EXISTS idx_transactions_sender ON transactions(sender_address);
CREATE INDEX IF NOT EXISTS idx_transactions_mev_candidate ON transactions(is_mev_candidate);

-- Builders table
CREATE TABLE IF NOT EXISTS builders (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    fee_recipient TEXT NOT NULL UNIQUE,
    builder_name TEXT,
    is_known BOOLEAN NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_builders_fee_recipient ON builders(fee_recipient);
CREATE INDEX IF NOT EXISTS idx_builders_known ON builders(is_known);

-- Validators table (from CSV import)
CREATE TABLE IF NOT EXISTS validators (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    fee_recipient TEXT NOT NULL,
    validator_pubkey TEXT NOT NULL,
    operator_name TEXT,
    country TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(fee_recipient, validator_pubkey)
);

CREATE INDEX IF NOT EXISTS idx_validators_fee_recipient ON validators(fee_recipient);
CREATE INDEX IF NOT EXISTS idx_validators_pubkey ON validators(validator_pubkey);
CREATE INDEX IF NOT EXISTS idx_validators_country ON validators(country);

-- Annotations table for custom tags and notes
CREATE TABLE IF NOT EXISTS annotations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    block_id INTEGER,
    transaction_id INTEGER,
    tag TEXT NOT NULL,
    note TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (block_id) REFERENCES blocks(id) ON DELETE CASCADE,
    FOREIGN KEY (transaction_id) REFERENCES transactions(id) ON DELETE CASCADE,
    CHECK ((block_id IS NOT NULL) OR (transaction_id IS NOT NULL))
);

CREATE INDEX IF NOT EXISTS idx_annotations_block_id ON annotations(block_id);
CREATE INDEX IF NOT EXISTS idx_annotations_transaction_id ON annotations(transaction_id);
CREATE INDEX IF NOT EXISTS idx_annotations_tag ON annotations(tag);

-- Schema version tracking
CREATE TABLE IF NOT EXISTS schema_version (
    version INTEGER PRIMARY KEY,
    applied_at TEXT NOT NULL DEFAULT (datetime('now'))
);

INSERT OR IGNORE INTO schema_version (version) VALUES (1);



