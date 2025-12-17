//! Core ingestion service for MEV Africa data collection.

pub mod rpc_client;
pub mod block_processor;
pub mod validator_tagger;

pub use block_processor::BlockProcessor;
pub use rpc_client::RpcClient;
pub use validator_tagger::ValidatorTagger;
