CREATE TABLE IF NOT EXISTS transaction_logs (
    id SERIAL PRIMARY KEY,
    chain_id BIGINT NOT NULL,
    tx_hash BYTEA NOT NULL,
    log_index INT NOT NULL,
    address BYTEA NOT NULL,
    data BYTEA NOT NULL,
    topics JSONB NOT NULL,
    processed BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(chain_id, tx_hash, log_index)
);

CREATE TABLE IF NOT EXISTS ledger_entries (
    id SERIAL PRIMARY KEY,
    chain_id BIGINT NOT NULL,
    tx_hash BYTEA NOT NULL,
    event_name TEXT NOT NULL,
    token_address BYTEA NOT NULL,
    amount_delta NUMERIC(38, 0),
    block_number BIGINT NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_logs_unprocessed ON transaction_logs (processed) WHERE processed = FALSE;