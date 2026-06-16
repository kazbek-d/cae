CREATE TABLE IF NOT EXISTS transaction_logs (
    id SERIAL PRIMARY KEY,
    chain_id BIGINT NOT NULL,
    tx_hash TEXT NOT NULL,
    log_index INT NOT NULL,
    address BYTEA NOT NULL,
    data BYTEA NOT NULL,
    topics BYTEA[] NOT NULL,
    is_processed BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(chain_id, tx_hash, log_index)
);

CREATE TABLE IF NOT EXISTS ledger_entries (
    id SERIAL PRIMARY KEY,
    chain_id BIGINT NOT NULL,
    tx_hash TEXT NOT NULL,
    event_name TEXT NOT NULL,
    token_address BYTEA NOT NULL,
    amount_delta TEXT NOT NULL,
    intent TEXT NOT NULL, 
    description TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS token_metadata (
    chain_id BIGINT NOT NULL,
    address BYTEA NOT NULL,
    symbol TEXT NOT NULL,
    decimals INT DEFAULT 18,
    is_lp_token BOOLEAN DEFAULT FALSE,
    PRIMARY KEY (chain_id, address)
);

CREATE TABLE IF NOT EXISTS watch_list (
    address BYTEA PRIMARY KEY,
    label TEXT
);

INSERT INTO watch_list (address, label) VALUES 
(DECODE('3DdfA8eC3052539b6C9549F12beA232742307E56', 'hex'), 'Justin Sun (DeFi Whale)'),
(DECODE('0000006daea1723962647b73127d6cff30108cb0', 'hex'), 'Wintermute (Market Maker)'),
(DECODE('28C6c06290CC3F951793962b0a3D65b7A0a3D0A3', 'hex'), 'Binance 14 (Hot Wallet)');