use alloy::sol;
use alloy::rpc::types::Log;
use alloy::sol_types::SolEvent;
use cae_types::{AuditEntry, Transformer};

sol! {
    #[sol(abi)]
    interface IUniswapV3 {
        event Swap(address indexed sender, address indexed recipient, int256 amount0, int256 amount1, uint160 sqrtPriceX96, uint128 liquidity, int24 tick);
    }
}

pub struct UniswapTransformer;

impl Transformer for UniswapTransformer {
    fn name(&self) -> &'static str { "Uniswap_V3" }
    fn transform(&self, log: &Log, chain_id: u64) -> Option<AuditEntry> {
        let decoded = IUniswapV3::Swap::decode_log(&log.inner).ok()?;
        Some(AuditEntry {
            chain_id,
            tx_hash: log.transaction_hash.unwrap_or_default(),
            event_name: "Swap".to_string(),
            token_address: log.address(),
            amount_delta: decoded.amount0.to_string(),
            block_number: log.block_number.unwrap_or_default(),
        })
    }
}