use crate::storage;
use alloy::primitives::Address;
use alloy::providers::Provider;
use alloy::rpc::types::Filter;
use cae_types::TransactionIntent;
use serde_json::json;
use std::sync::Arc;
use tracing::{debug, error, info, warn};

pub struct Backfiller<P> {
    provider: Arc<P>,
    pool: sqlx::PgPool,
}

impl<P: Provider + 'static> Backfiller<P> {
    pub fn new(provider: Arc<P>, pool: sqlx::PgPool) -> Self {
        Self { provider, pool }
    }

    /// Scans the full history of a wallet using Alchemy's Asset Transfer API.
    /// This method bypasses block-by-block syncing for historical data to fill the ledger quickly.
    pub async fn scan_wallet_history(&self, wallet: &str, chain_id: u64) -> eyre::Result<()> {
        info!("Starting Alchemy history scan for wallet: {}", wallet);

        // 1. Scan OUTBOUND transfers (fromAddress)
        // This identifies tokens sent away from the wallet.
        let params_out = json!({
            "fromBlock": "0x0",
            "toBlock": "latest",
            "fromAddress": wallet,
            "category": ["external", "erc20"], // Captures both Native ETH and ERC20 tokens
            "withMetadata": true,
            "excludeZeroValue": true,
        });

        let res_out: serde_json::Value = self
            .provider
            .client()
            .request("alchemy_getAssetTransfers", (params_out,))
            .await?;

        self.ingest_alchemy_transfers(res_out, chain_id, TransactionIntent::Outbound)
            .await?;

        // 2. Scan INBOUND transfers (toAddress)
        // This identifies tokens received by the wallet.
        let params_in = json!({
            "fromBlock": "0x0",
            "toBlock": "latest",
            "toAddress": wallet,
            "category": ["external", "erc20"],
            "withMetadata": true,
            "excludeZeroValue": true,
        });

        let res_in: serde_json::Value = self
            .provider
            .client()
            .request("alchemy_getAssetTransfers", (params_in,))
            .await?;

        self.ingest_alchemy_transfers(res_in, chain_id, TransactionIntent::Inbound)
            .await?;

        info!("Alchemy history scan completed for {}", wallet);
        Ok(())
    }

    /// Internal helper to parse Alchemy's JSON response and save entries to the database.
    async fn ingest_alchemy_transfers(
        &self,
        response: serde_json::Value,
        chain_id: u64,
        intent: TransactionIntent,
    ) -> eyre::Result<()> {
        if let Some(transfers) = response["transfers"].as_array() {
            for tx in transfers {
                let tx_hash = tx["hash"].as_str().unwrap_or_default().to_string();

                // Alchemy returns "null" or "0x0" for Native ETH transfers in the address field.
                let raw_addr = tx["rawContract"]["address"]
                    .as_str()
                    .unwrap_or("0x0000000000000000000000000000000000000000");
                let token_addr: Address = raw_addr.parse().unwrap_or(Address::ZERO);

                // Extract the transfer amount as a string.
                let amount = tx["value"].as_f64().unwrap_or(0.0).to_string();

                let entry = cae_types::AuditEntry {
                    chain_id,
                    tx_hash,
                    event_name: "AlchemyHistorySync".into(),
                    token_address: token_addr,
                    amount_delta: amount,
                    intent: intent.clone(),
                    description: format!(
                        "Historical {} transfer synced via Alchemy",
                        intent.to_string()
                    ),
                };

                // Attempt to save to the ledger.
                // duplicates are ignored via SQL unique constraints (chain_id, tx_hash, log_index).
                if let Err(e) = storage::save_audit_entry(&self.pool, entry).await {
                    debug!("Skipping duplicate or invalid historical entry: {}", e);
                }
            }
        }
        Ok(())
    }
}

pub async fn run_polling_fetcher_old<P: Provider + 'static>(
    provider: Arc<P>,
    pool: sqlx::PgPool,
    chain_id: u64,
    watchlist: Vec<Address>,
) -> eyre::Result<()> {
    let mut last_processed = provider.get_block_number().await?;
    loop {
        if let Ok(current) = provider.get_block_number().await {
            for block_num in (last_processed + 1)..=current {
                let block = provider
                    .get_block_by_number(block_num.into(), true)
                    .await?
                    .unwrap();
                for tx in block.transactions.as_transactions().unwrap() {
                    let from_w = watchlist.contains(&tx.from);
                    let to_w = tx.to.map_or(false, |t| watchlist.contains(&t));
                    if (from_w || to_w) && tx.value > alloy::primitives::U256::ZERO {
                        //if tx.value > alloy::primitives::U256::ZERO {
                        let intent = if from_w && to_w {
                            TransactionIntent::InternalTransfer
                        } else if to_w {
                            TransactionIntent::Inbound
                        } else {
                            TransactionIntent::Outbound
                        };
                        storage::save_native_transfer(
                            &pool,
                            chain_id,
                            tx.hash,
                            tx.value,
                            intent,
                            "Native ETH".into(),
                        )
                        .await?;
                    }
                }
                let filter = Filter::new()
                    .from_block(block_num)
                    .to_block(block_num)
                    .address(watchlist.clone());
                let logs = provider.get_logs(&filter).await?;
                for log in logs {
                    storage::save_raw_log(&pool, chain_id, &log).await?;
                }
                last_processed = block_num;
            }
        }
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    }
}

pub async fn run_polling_fetcher<P: Provider + 'static>(
    provider: Arc<P>,
    pool: sqlx::PgPool,
    chain_id: u64,
    watchlist: Vec<Address>,
) -> eyre::Result<()> {
    // Initial sync: get the current block height to start polling from
    let mut last_processed = provider.get_block_number().await?;

    info!(
        chain_id,
        start_block = last_processed,
        watchlist_len = watchlist.len(),
        "Fetcher initialized and starting real-time poll"
    );

    loop {
        // Fetch current block height from the node
        if let Ok(current) = provider.get_block_number().await {
            // Iterate through every new block found since the last loop
            for block_num in (last_processed + 1)..=current {
                info!(chain_id, block_num, "Scanning block for activity");

                // Get block details including full transaction objects
                let block = match provider.get_block_by_number(block_num.into(), true).await? {
                    Some(b) => b,
                    None => {
                        warn!(block_num, "Block not returned by provider, skipping");
                        continue;
                    }
                };

                // --- Part 1: Native ETH Transfer Detection ---
                let transactions = block.transactions.as_transactions().unwrap();
                let mut native_count = 0;

                for tx in transactions {
                    let from_w = watchlist.contains(&tx.from);
                    let to_w = tx.to.map_or(false, |t| watchlist.contains(&t));

                    // Check if the transaction involves a watched address and has a non-zero value
                    if (from_w || to_w) && tx.value > alloy::primitives::U256::ZERO {
                        let intent = if from_w && to_w {
                            TransactionIntent::InternalTransfer
                        } else if to_w {
                            TransactionIntent::Inbound
                        } else {
                            TransactionIntent::Outbound
                        };

                        debug!(tx_hash = %tx.hash, "Relevant native ETH transfer found");

                        storage::save_native_transfer(
                            &pool,
                            chain_id,
                            tx.hash,
                            tx.value,
                            intent,
                            "Native ETH".into(),
                        )
                        .await?;

                        native_count += 1;
                    }
                }

                // --- Part 2: Smart Contract Log Detection (ERC20, LP, etc.) ---
                // Filter logs emitted by any address in the watchlist within this block
                let filter = Filter::new()
                    .from_block(block_num)
                    .to_block(block_num)
                    .address(watchlist.clone());

                let logs = provider.get_logs(&filter).await?;
                let log_count = logs.len();

                for log in logs {
                    storage::save_raw_log(&pool, chain_id, &log).await?;
                }

                // Summary log for the processed block if activity was found
                if native_count > 0 || log_count > 0 {
                    info!(
                        block = block_num,
                        eth_transfers = native_count,
                        token_logs = log_count,
                        "Activity detected and saved"
                    );
                }

                // Update the tracker so we don't process this block again
                last_processed = block_num;
            }
        } else {
            error!(
                chain_id,
                "Failed to fetch latest block number from provider"
            );
        }

        // Wait before polling for new blocks again
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    }
}
