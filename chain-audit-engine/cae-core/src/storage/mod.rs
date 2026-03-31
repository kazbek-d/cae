use sqlx::{PgPool, Row};
use alloy::primitives::{Address, B256, Bytes, U256};
use alloy::rpc::types::Log;
use alloy::providers::Provider;
use crate::ingestion::transformers::IERC20Metadata;
use cae_types::{AuditEntry, TransactionIntent};
use std::sync::Arc;

pub async fn get_watchlist(pool: &PgPool) -> eyre::Result<Vec<Address>> {
    let rows = sqlx::query!("SELECT address FROM watch_list").fetch_all(pool).await?;
    Ok(rows.into_iter().map(|r| Address::from_slice(&r.address)).collect())
}

pub async fn save_raw_log(pool: &PgPool, chain_id: u64, log: &Log) -> eyre::Result<()> {
    let topics: Vec<Vec<u8>> = log.topics().iter().map(|t| t.as_slice().to_vec()).collect();
    sqlx::query!(
        "INSERT INTO transaction_logs (chain_id, tx_hash, log_index, address, data, topics) VALUES ($1, $2, $3, $4, $5, $6) ON CONFLICT DO NOTHING",
        chain_id as i64, log.transaction_hash.unwrap().to_string(), log.log_index.unwrap() as i32, 
        log.address().as_slice(), log.data().data.as_ref(), &topics
    ).execute(pool).await?;
    Ok(())
}

pub async fn save_native_transfer(pool: &PgPool, chain_id: u64, tx_hash: B256, amount: U256, intent: TransactionIntent, desc: String) -> eyre::Result<()> {
    sqlx::query!(
        "INSERT INTO ledger_entries (chain_id, tx_hash, event_name, token_address, amount_delta, intent, description) VALUES ($1, $2, $3, $4, $5, $6, $7)",
        chain_id as i64, tx_hash.to_string(), "NativeTransfer", Address::ZERO.as_slice(), amount.to_string(), intent.to_string(), desc
    ).execute(pool).await?;
    Ok(())
}

pub async fn get_unprocessed_logs_by_chain(pool: &PgPool, chain_id: u64) -> eyre::Result<Vec<sqlx::postgres::PgRow>> {
    sqlx::query("SELECT * FROM transaction_logs WHERE is_processed = FALSE AND chain_id = $1 ORDER BY created_at ASC")
        .bind(chain_id as i64).fetch_all(pool).await.map_err(Into::into)
}

pub async fn get_or_discover_token<P: Provider>(pool: &PgPool, provider: Arc<P>, chain_id: u64, addr: Address) -> eyre::Result<(String, i32)> {
    if addr == Address::ZERO { return Ok(("ETH".into(), 18)); }
    if let Some(row) = sqlx::query!("SELECT symbol, decimals FROM token_metadata WHERE address = $1", addr.as_slice()).fetch_optional(pool).await? {
        return Ok((row.symbol, row.decimals.unwrap_or(18)));
    }
    let contract = IERC20Metadata::new(addr, provider);
    let symbol = contract.symbol().call().await.map(|s| s._0).unwrap_or("?".into());
    let decimals = contract.decimals().call().await.map(|d| d._0 as i32).unwrap_or(18);
    sqlx::query!("INSERT INTO token_metadata (chain_id, address, symbol, decimals) VALUES ($1, $2, $3, $4) ON CONFLICT DO NOTHING", chain_id as i64, addr.as_slice(), symbol, decimals).execute(pool).await?;
    Ok((symbol, decimals))
}

pub async fn save_audit_entry(pool: &PgPool, entry: AuditEntry) -> eyre::Result<()> {
    sqlx::query!(
        "INSERT INTO ledger_entries (chain_id, tx_hash, event_name, token_address, amount_delta, intent, description) VALUES ($1, $2, $3, $4, $5, $6, $7)",
        entry.chain_id as i64, entry.tx_hash, entry.event_name, entry.token_address.as_slice(), entry.amount_delta, entry.intent.to_string(), entry.description
    ).execute(pool).await?;
    Ok(())
}

pub async fn mark_processed(pool: &PgPool, id: i32) -> eyre::Result<()> {
    sqlx::query!("UPDATE transaction_logs SET is_processed = TRUE WHERE id = $1", id).execute(pool).await?;
    Ok(())
}