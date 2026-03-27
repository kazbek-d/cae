mod ingestion;
mod storage;

use dotenvy::dotenv;
use sqlx::postgres::PgPoolOptions;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    dotenv().ok();
    //tracing_subscriber::fmt::init();
    tracing_subscriber::fmt()
    .with_max_level(tracing::Level::INFO)
    .init();

    let db_url = std::env::var("DATABASE_URL")?;
    let pool = PgPoolOptions::new().max_connections(10).connect(&db_url).await?;

    tracing::info!("🚀 CAE-Core: System started");

    // Start the worker in a separate thread
    let pool_worker = pool.clone();
    tokio::spawn(async move {
        if let Err(e) = ingestion::worker::run_worker(pool_worker).await {
            tracing::error!("Worker error: {:?}", e);
        }
    });

    tokio::signal::ctrl_c().await?;
    Ok(())
}