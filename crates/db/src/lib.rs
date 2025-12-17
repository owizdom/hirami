//! Database layer for MEV Africa data collection.
//!
//! Provides SQLite storage with schema versioning and migrations.

pub mod migrations;
pub mod models;
pub mod pool;

pub use pool::DbPool;



