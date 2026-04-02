mod ingestion;
mod storage;

use alloy::providers::ProviderBuilder;
use std::sync::Arc;
use std::env;
use tracing::info;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();
    dotenv::dotenv().ok();

    let db_url = env::var("DATABASE_URL")?;
    let pool = sqlx::PgPool::connect(&db_url).await?;

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
            info!("Chain {}: Initializing Polling Mode", chain_id);
            let provider = ProviderBuilder::new().on_http(rpc_url.parse().unwrap()).boxed();
            let provider = Arc::new(provider);
            let watchlist = storage::get_watchlist(&pool).await.unwrap_or_default();

            // 1. Backfill & Alchemy History
            let bf = ingestion::fetcher::Backfiller::new(provider.clone(), pool.clone());
            for addr in &watchlist {
                let _ = bf.scan_wallet_history(&format!("{:?}", addr), chain_id).await;
            }

            // 2. Worker
            let p_w = pool.clone();
            let pr_w = provider.clone();
            tokio::spawn(async move {
                ingestion::worker::run_worker(p_w, pr_w, chain_id).await.unwrap();
            });

            // 3. Polling Fetcher
            ingestion::fetcher::run_polling_fetcher(provider, pool, chain_id, watchlist).await.unwrap();
        });
    }

    tokio::signal::ctrl_c().await?;
    Ok(())
}