use alloy::providers::Provider;
use alloy::rpc::types::{BlockTransactionsKind, Filter};
use std::sync::Arc;
use crate::storage;
use alloy::primitives::{Address, B256};
use cae_types::TransactionIntent;

pub struct Backfiller<P> { provider: Arc<P>, pool: sqlx::PgPool }

impl<P: Provider + 'static> Backfiller<P> {
    pub fn new(provider: Arc<P>, pool: sqlx::PgPool) -> Self { Self { provider, pool } }
    pub async fn scan_history(&self, chain_id: u64, watchlist: &[Address], start_block: u64) -> eyre::Result<()> {
        let current_block = self.provider.get_block_number().await?;
        let mut from = start_block;
        while from < current_block {
            let to = std::cmp::min(from + 2000, current_block);
            let filter = Filter::new().from_block(from).to_block(to).address(watchlist.to_vec());
            let logs = self.provider.get_logs(&filter).await?;
            for log in logs { storage::save_raw_log(&self.pool, chain_id, &log).await?; }
            from = to + 1;
        }
        Ok(())
    }
}

pub async fn run_realtime_listener<P: Provider + 'static>(
    provider: Arc<P>,
    pool: sqlx::PgPool,
    chain_id: u64,
    watchlist: Vec<Address>,
) -> eyre::Result<()> {
    let mut block_stream = provider.subscribe_blocks().await?.into_stream();
    while let Some(block_header) = block_stream.next().await {
        let block_number = block_header.number.unwrap();
        let block = provider.get_block_by_number(block_number.into(), BlockTransactionsKind::Full).await?.unwrap();

        for tx in block.transactions.as_transactions().unwrap() {
            let from_watch = watchlist.contains(&tx.from);
            let to_watch = tx.to.map_or(false, |to| watchlist.contains(&to));
            if (from_watch || to_watch) && tx.value > alloy::primitives::U256::ZERO {
                let intent = if from_watch && to_watch { TransactionIntent::InternalTransfer }
                             else if to_watch { TransactionIntent::Inbound }
                             else { TransactionIntent::Outbound };
                storage::save_native_transfer(&pool, chain_id, tx.hash, tx.value, intent, "Native ETH transfer".into()).await?;
            }
        }
        let filter = Filter::new().from_block(block_number).to_block(block_number);
        let logs = provider.get_logs(&filter).await?;
        for log in logs { storage::save_raw_log(&pool, chain_id, &log).await?; }
    }
    Ok(())
}