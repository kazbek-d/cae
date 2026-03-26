use serde::{Serialize, Deserialize};
use alloy::primitives::{Address, B256};
use alloy::rpc::types::Log;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub chain_id: u64,
    pub tx_hash: B256,
    pub event_name: String,
    pub token_address: Address,
    pub amount_delta: String,
    pub block_number: u64,
}

pub trait Transformer: Send + Sync {
    fn name(&self) -> &'static str;
    fn transform(&self, log: &Log, chain_id: u64) -> Option<AuditEntry>;
}