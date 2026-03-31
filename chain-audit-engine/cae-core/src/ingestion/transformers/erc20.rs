use alloy::primitives::Address;
use alloy::rpc::types::Log;
use alloy::sol;
use cae_types::{AuditEntry, TransactionIntent};

sol!(event Transfer(address indexed from, address indexed to, uint256 value););

pub struct Erc20Transformer;
impl Erc20Transformer {
    pub fn transform(log: &Log, chain_id: u64, watchlist: &[Address]) -> Option<AuditEntry> {
        let transfer = log.log_decode::<Transfer>().ok()?.inner.data;
        let is_from = watchlist.contains(&transfer.from);
        let is_to = watchlist.contains(&transfer.to);
        if !is_from && !is_to { return None; }

        let intent = if is_from && is_to { TransactionIntent::InternalTransfer }
                     else if is_to { TransactionIntent::Inbound }
                     else { TransactionIntent::Outbound };

        Some(AuditEntry {
            chain_id, tx_hash: log.transaction_hash.unwrap().to_string(),
            event_name: "Transfer".into(), token_address: log.address(),
            amount_delta: transfer.value.to_string(), intent,
            description: "ERC20 Movement".into(),
        })
    }
}