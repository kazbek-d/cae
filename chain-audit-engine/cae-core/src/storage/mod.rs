pub mod tests; // Include tests

use sqlx::PgPool;
use eyre::Result;

pub async fn mark_as_processed(pool: &PgPool, id: i32) -> Result<()> {
    //sqlx::query!("UPDATE transaction_logs SET processed = true WHERE id = $1", id)
    //    .execute(pool).await?;
    Ok(())
}