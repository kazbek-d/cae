use crate::ingestion::transformers::{erc20::Erc20Transformer, lp::LpTransformer};
use crate::storage;
use alloy::primitives::{Address, Bytes, LogData, B256};
use alloy::providers::Provider;
use alloy::rpc::types::Log;
use sqlx::{PgPool, Row};
use std::sync::Arc;

pub async fn run_worker<P: Provider + 'static>(
    pool: PgPool,
    provider: Arc<P>,
    chain_id: u64,
) -> eyre::Result<()> {
    loop {
        let watchlist = storage::get_watchlist(&pool).await?;
        let rows = storage::get_unprocessed_logs_by_chain(&pool, chain_id).await?;

        for row in rows {
            let log_id: i32 = row.get("id");
            let topics_raw: Vec<Vec<u8>> = row.get("topics");
            let topics: Vec<B256> = topics_raw
                .into_iter()
                .map(|t| B256::from_slice(&t))
                .collect();

            let log = Log {
                inner: alloy::primitives::Log {
                    address: Address::from_slice(row.get("address")),
                    data: LogData::new_unchecked(
                        topics,
                        Bytes::from(row.get::<Vec<u8>, _>("data")),
                    ),
                },
                block_hash: None,
                block_number: None,
                block_timestamp: None,
                transaction_index: None,
                removed: false,
                transaction_hash: Some(row.get::<String, _>("tx_hash").parse().unwrap()),
                log_index: Some(row.get::<i32, _>("log_index") as u64),
            };

            let entry = LpTransformer::transform(&log, chain_id, &watchlist)
                .or_else(|| Erc20Transformer::transform(&log, chain_id, &watchlist));

            if let Some(mut e) = entry {
                let (sym, _) = storage::get_or_discover_token(
                    &pool,
                    provider.clone(),
                    chain_id,
                    e.token_address,
                )
                .await?;
                e.description = format!("{} [{}]", e.description, sym);
                storage::save_audit_entry(&pool, e).await?;
            }
            storage::mark_processed(&pool, log_id).await?;
        }
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    }
}
