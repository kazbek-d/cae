use sqlx::PgPool;
use tokio::time::{sleep, Duration};
use alloy::rpc::types::Log;
use alloy::primitives::{Address, B256, Bytes, LogData};
use crate::ingestion::transformers::UniswapTransformer;
use crate::storage;
use cae_types::Transformer;

/// The main worker loop that processes raw blockchain logs from the database
pub async fn run_worker(pool: PgPool) -> eyre::Result<()> {
    // Register all active protocol transformers
    // You can easily add more transformers here (e.g., Aave, Curve, Lido)
    let transformers: Vec<Box<dyn Transformer>> = vec![
        Box::new(UniswapTransformer),
    ];

    tracing::info!(target: "cae_worker", "Worker started. Polling for unprocessed logs...");

    loop {
        // 1. Fetch a batch of unprocessed logs from the Ingestion Layer
        // We use a limit to prevent memory spikes and ensure smooth processing
        let rows = sqlx::query_as::<_, (i64, i32, Vec<u8>, Vec<u8>, Vec<u8>, sqlx::types::JsonValue)>(
            r#"
            SELECT id, chain_id, tx_hash, address, data, topics 
            FROM transaction_logs 
            WHERE processed = false 
            ORDER BY created_at ASC
            LIMIT 50
            "#
        )
        .fetch_all(&pool).await?;

        if rows.is_empty() {
            // No work to do, back off for a few seconds
            sleep(Duration::from_secs(5)).await;
            continue;
        }

        let rows_len = rows.len();
        for (id, chain_id, tx_hash_bytes, address_bytes, data_bytes, topics_json_value) in rows {
            // 2. Hydrate database bytes back into Alloy types
            // We convert database binary data (BYTEA) into EVM primitives
            let contract_address = Address::from_slice(&address_bytes);
            let tx_hash = B256::from_slice(&tx_hash_bytes);
            
            // Reconstruct topics from JSONB column
            let topics_json: Vec<String> = serde_json::from_value(topics_json_value)?;
            let topics: Vec<B256> = topics_json
                .into_iter()
                .map(|s| s.parse().unwrap_or_default())
                .collect();

            // Construct the standardized Alloy Log object for transformers
            let alloy_log = Log {
                inner: alloy::primitives::Log {
                    address: contract_address,
                    data: LogData::new_unchecked(topics, Bytes::from(data_bytes)),
                },
                block_hash: None,
                block_number: None,
                block_timestamp: None,
                transaction_hash: Some(tx_hash),
                transaction_index: None,
                log_index: None,
                removed: false,
            };

            // 3. Run the hydrated log through the transformer pipeline
            let mut matched = false;
            for tf in &transformers {
                if let Some(entry) = tf.transform(&alloy_log, chain_id as u64) {
                    tracing::debug!(
                        target: "cae_worker", 
                        "Transformer [{}] identified event [{}] in tx {:?}", 
                        tf.name(), entry.event_name, tx_hash
                    );
                    
                    // Persist the identified financial entry to the Ledger Layer
                    if let Err(e) = storage::save_audit_entry(&pool, entry).await {
                        tracing::error!("Failed to save audit entry: {:?}", e);
                        // We do not mark as processed here so we can retry on next loop
                        continue;
                    }
                    matched = true;
                }
            }

            if !matched {
                tracing::trace!(target: "cae_worker", "No transformer matched log ID: {}", id);
            }

            // 4. Update the processing state
            // Even if no transformer matched, we mark it as processed because 
            // it has been inspected by the current engine logic.
            if let Err(e) = storage::mark_as_processed(&pool, id).await {
                tracing::error!("Failed to mark log {} as processed: {:?}", id, e);
            }
        }
        
        tracing::info!(target: "cae_worker", "Processed batch of {} logs", rows_len);
    }
}