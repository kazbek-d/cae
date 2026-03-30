mod ingestion;
mod storage;

use alloy::providers::ProviderBuilder;
use tracing::level_filters;
use std::sync::Arc;
use tokio::spawn;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_max_level(level_filters::LevelFilter::INFO)
        .init();

    let db_url = std::env::var("DATABASE_URL")?;
    let pool = sqlx::PgPool::connect(&db_url).await?;

    // Define the networks you want to index
    let networks = vec![
        (1, "ETH_RPC_URL"),       // Ethereum Mainnet
        //(42161, "ARB_RPC_URL"),   // Arbitrum
        //(8453, "BASE_RPC_URL"),   // Base
    ];

    // 1. Spawn a single Worker to process everything in the DB
    let pool_worker = pool.clone();
    spawn(async move {
        if let Err(e) = ingestion::worker::run_worker(pool_worker).await {
            tracing::error!("Global Worker crashed: {:?}", e);
        }
    });

    // 2. Spawn a Fetcher for each network
    for (chain_id, env_var) in networks {
        let rpc_url = std::env::var(env_var)?.parse()?;
        let provider = Arc::new(ProviderBuilder::new().connect_http(rpc_url));
        let pool_fetcher = pool.clone();

        spawn(async move {
            if let Err(e) = ingestion::fetcher::run_fetcher(provider, pool_fetcher, chain_id).await {
                tracing::error!("Fetcher for Chain {} crashed: {:?}", chain_id, e);
            }
        });
    }

    tokio::signal::ctrl_c().await?;
    Ok(())
}