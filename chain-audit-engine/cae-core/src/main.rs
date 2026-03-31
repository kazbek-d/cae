mod ingestion;
mod storage;

use alloy::providers::{ProviderBuilder, WsConnect, Provider};
use std::sync::Arc;
use std::env;
use tracing::{info, error};

#[tokio::main]
async fn main() -> eyre::Result<()> {
    tracing_subscriber::fmt::init();
    dotenv::dotenv().ok();

    let pool = sqlx::PgPool::connect(&env::var("DATABASE_URL")?).await?;

    let networks = vec![
        (1, "ETH_RPC_URL"),
        (42161, "ARB_RPC_URL"),
        (8453, "BASE_RPC_URL"),
    ];

    for (chain_id, env_var) in networks {
        let pool = pool.clone();
        let rpc_url = match env::var(env_var) {
            Ok(url) => url,
            Err(_) => continue,
        };

        tokio::spawn(async move {
            info!("Chain {}: Connecting...", chain_id);
            let ws = WsConnect::new(rpc_url);
            let provider = Arc::new(ProviderBuilder::new().on_ws(ws).await.unwrap());
            let watchlist = storage::get_watchlist(&pool).await.unwrap_or_default();

            // 1. Backfill (Last 5000 blocks)
            let current = provider.get_block_number().await.unwrap();
            let backfiller = ingestion::fetcher::Backfiller::new(provider.clone(), pool.clone());
            let _ = backfiller.scan_history(chain_id, &watchlist, current - 5000).await;

            // 2. Worker
            let p_worker = pool.clone();
            let pr_worker = provider.clone();
            tokio::spawn(async move {
                ingestion::worker::run_worker(p_worker, pr_worker, chain_id).await.unwrap();
            });

            // 3. Real-time Listener
            ingestion::fetcher::run_realtime_listener(provider, pool, chain_id, watchlist).await.unwrap();
        });
    }

    tokio::signal::ctrl_c().await?;
    Ok(())
}