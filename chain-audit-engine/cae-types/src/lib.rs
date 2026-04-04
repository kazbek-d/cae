use alloy::primitives::Address;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransactionIntent {
    Swap,
    BridgeOut,
    BridgeIn,
    LiquidityProvision,
    Staking,
    SimpleTransfer,
    InternalTransfer,
    Inbound,
    Outbound,
    Unknown,
}

impl ToString for TransactionIntent {
    fn to_string(&self) -> String {
        format!("{:?}", self)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub chain_id: u64,
    pub tx_hash: String,
    pub event_name: String,
    pub token_address: Address,
    pub amount_delta: String,
    pub intent: TransactionIntent,
    pub description: String,
}
