mod ingestion;
mod storage;

use alloy::providers::ProviderBuilder;
use sqlx::PgPool;
use std::env;
use std::sync::Arc;
use tracing::{error, info};

#[tokio::main]
async fn main() -> eyre::Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();
    dotenv::dotenv().ok();

    let db_url = env::var("DATABASE_URL")?;
    let pool = PgPool::connect(&db_url).await?;

    let networks: Vec<(u64, &str)> = vec![
        (1, "ETH_RPC_URL"),
        (42161, "ARB_RPC_URL"),
        (8453, "BASE_RPC_URL"),
    ];

    for (chain_id, env_var) in networks {
        let pool = pool.clone();
        if let Ok(rpc_url) = env::var(env_var) {
            tokio::spawn(async move {
                if let Err(e) = initialize_network(chain_id, rpc_url, pool).await {
                    error!("Chain {chain_id} initialization failed: {e}");
                }
            });
        }
    }

    tokio::signal::ctrl_c().await?;
    Ok(())
}

async fn initialize_network(chain_id: u64, rpc_url: String, pool: PgPool) -> eyre::Result<()> {
    info!("Chain {chain_id}: Initializing Polling Mode");

    let provider = ProviderBuilder::new().on_http(rpc_url.parse()?).boxed();
    let provider = Arc::new(provider);
    let watchlist = storage::get_watchlist(&pool).await.unwrap_or_default();

    // 1. Backfill & Alchemy History
    let backfiller = ingestion::fetcher::Backfiller::new(provider.clone(), pool.clone());
    for addr in &watchlist {
        let _ = backfiller
            .scan_wallet_history(&format!("{:?}", addr), chain_id)
            .await;
    }

    // 2. Worker
    let worker_pool = pool.clone();
    let worker_provider = provider.clone();
    tokio::spawn(async move {
        if let Err(e) = ingestion::worker::run_worker(worker_pool, worker_provider, chain_id).await
        {
            error!("Chain {chain_id} worker failed: {e}");
        }
    });

    // 3. Polling Fetcher
    ingestion::fetcher::run_polling_fetcher(provider, pool, chain_id, watchlist).await?;

    Ok(())
}
