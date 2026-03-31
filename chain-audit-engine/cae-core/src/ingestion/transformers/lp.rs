use alloy::primitives::Address;
use alloy::rpc::types::Log;
use alloy::sol;
use cae_types::{AuditEntry, TransactionIntent};

sol! { 
    interface IUniswapV2 { 
        event Mint(address indexed sender, uint amount0, uint amount1); 
        event Swap(address indexed sender, uint amount0In, uint amount1In, uint amount0Out, uint amount1Out, address indexed to);
    } 
}

pub struct LpTransformer;
impl LpTransformer {
    pub fn transform(log: &Log, chain_id: u64, watchlist: &[Address]) -> Option<AuditEntry> {
        if let Ok(mint) = log.log_decode::<IUniswapV2::Mint>() {
            if watchlist.contains(&mint.inner.sender) {
                return Some(AuditEntry {
                    chain_id, tx_hash: log.transaction_hash.unwrap().to_string(),
                    event_name: "LpMint".into(), token_address: log.address(),
                    amount_delta: "0".into(), intent: TransactionIntent::LiquidityProvision,
                    description: "Liquidity Provided".into(),
                });
            }
        }
        if let Ok(swap) = log.log_decode::<IUniswapV2::Swap>() {
            if watchlist.contains(&swap.inner.sender) || watchlist.contains(&swap.inner.to) {
                return Some(AuditEntry {
                    chain_id, tx_hash: log.transaction_hash.unwrap().to_string(),
                    event_name: "Swap".into(), token_address: log.address(),
                    amount_delta: "0".into(), intent: TransactionIntent::Swap,
                    description: "DEX Swap Detected".into(),
                });
            }
        }
        None
    }
}