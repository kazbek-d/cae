use sqlx::PgPool;
use tokio::time::{sleep, Duration};
use crate::ingestion::transformers::UniswapTransformer;
use crate::storage;
use cae_types::Transformer;

pub async fn run_worker(pool: PgPool) -> eyre::Result<()> {
    let transformers: Vec<Box<dyn Transformer>> = vec![Box::new(UniswapTransformer)];

    loop {
        // Select 50 unprocessed records
        //let rows = sqlx::query!(
        //    "SELECT id, chain_id, tx_hash, address, data, topics FROM transaction_logs WHERE processed = false LIMIT 50"
        //)
        //.fetch_all(&pool).await?;

        //for row in rows {
        //    // In a real application, here goes the logic for row -> Log conversion
        //    // After successful transformation, save to ledger_entries
        //    storage::mark_as_processed(&pool, row.id).await?;
        //}
        
        sleep(Duration::from_secs(2)).await;
    }
}