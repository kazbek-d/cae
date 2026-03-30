use alloy::providers::Provider;
use alloy::rpc::types::Filter;
use sqlx::PgPool;
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use crate::storage;

/// The Fetcher is responsible for pulling raw logs from a specific blockchain 
/// and persisting them into the transaction_logs table.
pub async fn run_fetcher<P>(
    provider: Arc<P>,
    pool: PgPool,
    chain_id: u64,
) -> eyre::Result<()> 
where 
    P: Provider + 'static 
{
    tracing::info!(target: "cae_fetcher", "Starting fetcher for Chain ID: {}", chain_id);

    // 1. Determine polling interval based on the chain
    let poll_interval = match chain_id {
        1 => Duration::from_secs(12),      // Ethereum Mainnet
        42161 | 8453 => Duration::from_secs(2), // Arbitrum/Base (Fast blocks)
        _ => Duration::from_secs(5),       // Default
    };

    // 2. Initialize the starting block
    // In a real app, you would fetch the 'last_synced_block' from the DB
    let mut last_processed_block = provider.get_block_number().await?;

    loop {
        // Get the current tip of the chain
        let current_block = match provider.get_block_number().await {
            Ok(n) => n,
            Err(e) => {
                tracing::error!(target: "cae_fetcher", "Chain {}: Failed to get latest block: {:?}", chain_id, e);
                sleep(Duration::from_secs(10)).await;
                continue;
            }
        };

        // If we are caught up, wait for new blocks
        if current_block <= last_processed_block {
            sleep(poll_interval).await;
            continue;
        }

        // 3. Define the fetch range
        // We limit the range to 1000 blocks per request to avoid RPC timeouts
        let target_block = std::cmp::min(last_processed_block + 1000, current_block);
        
        tracing::debug!(
            target: "cae_fetcher", 
            "Chain {}: Syncing range {} -> {}", 
            chain_id, last_processed_block + 1, target_block
        );

        // 4. Build the filter
        let filter = Filter::new()
            .from_block(last_processed_block + 1)
            .to_block(target_block);

        // 5. Fetch logs via RPC
        match provider.get_logs(&filter).await {
            Ok(logs) => {
                for log in logs {
                    // Persist raw log to the Ingestion Layer
                    if let Err(e) = storage::save_raw_log(&pool, chain_id, &log).await {
                        tracing::error!(target: "cae_fetcher", "Chain {}: DB Error: {:?}", chain_id, e);
                    }
                }
                
                // Update progress
                last_processed_block = target_block;
            }
            Err(e) => {
                tracing::error!(target: "cae_fetcher", "Chain {}: RPC error during get_logs: {:?}", chain_id, e);
                sleep(Duration::from_secs(5)).await;
            }
        }

        // Brief throttle to avoid hitting RPC rate limits
        sleep(Duration::from_millis(100)).await;
    }
}