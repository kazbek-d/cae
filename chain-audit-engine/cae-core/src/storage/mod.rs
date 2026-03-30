pub mod tests; // Include tests
pub use cae_types::AuditEntry;
use sqlx::PgPool;

pub async fn save_audit_entry(pool: &PgPool, entry: AuditEntry) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO ledger_entries (chain_id, tx_hash, event_name, token_address, amount_delta, block_number)
        VALUES ($1, $2, $3, $4, $5, $6)
        "#
    )
    .bind(entry.chain_id as i64)
    .bind(entry.tx_hash.as_slice())
    .bind(entry.event_name)
    .bind(entry.token_address.as_slice())
    .bind(entry.amount_delta.parse::<f64>().unwrap_or(0.0) as i64)
    .bind(entry.block_number as i64)
    .execute(pool).await?;
    Ok(())
}

pub async fn mark_as_processed(pool: &PgPool, id: i64) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE transaction_logs SET processed = true WHERE id = $1")
        .bind(id)
        .execute(pool).await?;
    Ok(())
}

pub async fn save_raw_log(
    pool: &sqlx::PgPool,
    chain_id: u64,
    log: &alloy::rpc::types::Log,
) -> eyre::Result<()> {
    // Extract transaction hash and log index for the unique constraint
    let tx_hash = log.transaction_hash.unwrap_or_default();
    let log_index = log.log_index.unwrap_or_default() as i32;
    
    // Serialize topics to JSON for storage
    let topics = serde_json::to_value(&log.inner.data.topics())?;

    sqlx::query(
        r#"
        INSERT INTO transaction_logs (chain_id, tx_hash, log_index, address, data, topics)
        VALUES ($1, $2, $3, $4, $5, $6)
        ON CONFLICT (chain_id, tx_hash, log_index) DO NOTHING
        "#
    )
    .bind(chain_id as i64)
    .bind(tx_hash.as_slice())
    .bind(log_index)
    .bind(log.address().as_slice())
    .bind(log.inner.data.data.as_ref())
    .bind(topics)
    .execute(pool)
    .await?;

    Ok(())
}