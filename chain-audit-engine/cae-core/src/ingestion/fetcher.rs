use alloy::providers::Provider;
use alloy::rpc::types::Filter;
use std::sync::Arc;
use crate::storage;
use alloy::primitives::Address;
use cae_types::TransactionIntent;
use serde_json::json;
use tracing::info;

pub struct Backfiller<P> { provider: Arc<P>, pool: sqlx::PgPool }

impl<P: Provider + 'static> Backfiller<P> {
    pub fn new(provider: Arc<P>, pool: sqlx::PgPool) -> Self { Self { provider, pool } }

    pub async fn scan_wallet_history(&self, wallet: &str, _chain_id: u64) -> eyre::Result<()> {
        info!("Alchemy History Scan: {}", wallet);
        let params = json!({
            "fromBlock": "0x0", "toBlock": "latest", "fromAddress": wallet,
            "category": ["external", "erc20"], "withMetadata": true, "excludeZeroValue": true,
        });
        let res: serde_json::Value = self.provider.client().request("alchemy_getAssetTransfers", (params,)).await?;
        if let Some(txs) = res["transfers"].as_array() {
            info!("Found {} historical transfers", txs.len());
        }
        Ok(())
    }
}

pub async fn run_polling_fetcher<P: Provider + 'static>(
    provider: Arc<P>, pool: sqlx::PgPool, chain_id: u64, watchlist: Vec<Address>
) -> eyre::Result<()> {
    let mut last_processed = provider.get_block_number().await?;
    loop {
        if let Ok(current) = provider.get_block_number().await {
            for block_num in (last_processed + 1)..=current {
                let block = provider.get_block_by_number(block_num.into(), true).await?.unwrap();
                for tx in block.transactions.as_transactions().unwrap() {
                    let from_w = watchlist.contains(&tx.from);
                    let to_w = tx.to.map_or(false, |t| watchlist.contains(&t));
                    if (from_w || to_w) && tx.value > alloy::primitives::U256::ZERO {
                        let intent = if from_w && to_w { TransactionIntent::InternalTransfer } else if to_w { TransactionIntent::Inbound } else { TransactionIntent::Outbound };
                        storage::save_native_transfer(&pool, chain_id, tx.hash, tx.value, intent, "Native ETH".into()).await?;
                    }
                }
                let filter = Filter::new().from_block(block_num).to_block(block_num).address(watchlist.clone());
                let logs = provider.get_logs(&filter).await?;
                for log in logs { storage::save_raw_log(&pool, chain_id, &log).await?; }
                last_processed = block_num;
            }
        }
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    }
}