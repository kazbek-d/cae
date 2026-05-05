use axum::{extract::{State, Path}, routing::{get, post, delete}, Json, Router};
use sqlx::PgPool;
use tower_http::cors::CorsLayer;
use serde::{Deserialize, Serialize};

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    let pool = PgPool::connect(&std::env::var("DATABASE_URL").unwrap())
        .await
        .unwrap();

    // Auto-migration
    let _ = sqlx::query("ALTER TABLE ledger_entries ADD COLUMN IF NOT EXISTS wallet_address BYTEA")
        .execute(&pool)
        .await;

    let app = Router::new()
        .route("/bundle/balance", get(get_bundle_balance))
        .route("/api/transactions", get(get_transactions))
        .route("/api/wallets", get(get_wallets).post(add_wallet))
        .route("/api/wallets/{address}", delete(delete_wallet))
        .layer(CorsLayer::permissive())
        .with_state(pool);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("CAE API ONLINE: http://localhost:3000");
    axum::serve(listener, app).await.unwrap();
}

async fn get_bundle_balance(State(pool): State<PgPool>) -> Json<serde_json::Value> {
    let rows = sqlx::query!(
        r#"
        SELECT 
            l.chain_id,
            COALESCE(m.symbol, 'UNK') as asset,
            SUM(CASE WHEN l.intent IN ('Inbound', 'BridgeIn') THEN l.amount_delta::numeric 
                  WHEN l.intent IN ('Outbound', 'BridgeOut') THEN -l.amount_delta::numeric 
                  ELSE 0 END) / POW(10, COALESCE(m.decimals, 18)) as balance
        FROM ledger_entries l
        LEFT JOIN token_metadata m ON l.chain_id = m.chain_id AND l.token_address = m.address
        GROUP BY 1, 2, m.decimals
        "#
    )
    .fetch_all(&pool)
    .await
    .unwrap();

    let data: Vec<_> = rows.into_iter().map(|r| {
        serde_json::json!({ "chain": r.chain_id, "asset": r.asset, "balance": r.balance.unwrap_or_default().to_string() })
    }).collect();

    Json(serde_json::json!(data))
}

#[derive(Serialize)]
struct Transaction {
    chain_id: i64,
    tx_hash: String,
    intent: String,
    amount: String,
    asset: String,
    wallet: Option<String>,
    created_at: String,
}

#[derive(Deserialize)]
struct TransactionQuery {
    wallet: Option<String>,
}

async fn get_transactions(
    State(pool): State<PgPool>,
    axum::extract::Query(query): axum::extract::Query<TransactionQuery>
) -> Json<Vec<Transaction>> {
    let wallet_hex = query.wallet.map(|w| w.trim_start_matches("0x").to_string());
    
    let rows = sqlx::query(
        r#"
        SELECT 
            l.chain_id,
            l.tx_hash,
            l.intent,
            (l.amount_delta::numeric / POW(10, COALESCE(m.decimals, 18)))::text as formatted_amount,
            COALESCE(m.symbol, 'UNK') as asset,
            TO_CHAR(l.created_at, 'YYYY-MM-DD HH24:MI:SS') as created_at,
            l.wallet_address as wallet
        FROM ledger_entries l
        LEFT JOIN token_metadata m ON l.chain_id = m.chain_id AND l.token_address = m.address
        WHERE $1::text IS NULL OR l.wallet_address = DECODE($1, 'hex')
        ORDER BY l.created_at DESC
        LIMIT 50
        "#,
    )
    .bind(wallet_hex)
    .fetch_all(&pool)
    .await
    .unwrap();

    let txs = rows.into_iter().map(|r| {
        use sqlx::Row;
        Transaction {
            chain_id: r.get("chain_id"),
            tx_hash: r.get("tx_hash"),
            intent: r.get("intent"),
            amount: r.get::<Option<String>, _>("formatted_amount").unwrap_or_default(),
            asset: r.get::<Option<String>, _>("asset").unwrap_or_else(|| "UNK".to_string()),
            wallet: r.get::<Option<Vec<u8>>, _>("wallet").map(|w| format!("0x{}", hex::encode(w))),
            created_at: r.get::<Option<String>, _>("created_at").unwrap_or_default(),
        }
    }).collect();

    Json(txs)
}

#[derive(Serialize)]
struct Wallet {
    address: String,
    label: String,
}

#[derive(Deserialize)]
struct AddWalletRequest {
    address: String,
    label: String,
}

async fn get_wallets(State(pool): State<PgPool>) -> Json<Vec<Wallet>> {
    let rows = sqlx::query!("SELECT ENCODE(address, 'hex') as hex_addr, label FROM watch_list")
        .fetch_all(&pool)
        .await
        .unwrap();

    let wallets = rows.into_iter().map(|r| Wallet {
        address: format!("0x{}", r.hex_addr.unwrap()),
        label: r.label.unwrap_or_default(),
    }).collect();

    Json(wallets)
}

async fn add_wallet(State(pool): State<PgPool>, Json(payload): Json<AddWalletRequest>) -> Json<serde_json::Value> {
    let clean_addr = payload.address.trim_start_matches("0x");
    
    if clean_addr.len() != 40 {
        return Json(serde_json::json!({ "error": "Invalid address length" }));
    }

    let _ = sqlx::query!(
        "INSERT INTO watch_list (address, label) VALUES (DECODE($1, 'hex'), $2) ON CONFLICT (address) DO UPDATE SET label = EXCLUDED.label",
        clean_addr,
        payload.label
    )
    .execute(&pool)
    .await;

    Json(serde_json::json!({ "status": "ok" }))
}

async fn delete_wallet(State(pool): State<PgPool>, Path(address): Path<String>) -> Json<serde_json::Value> {
    let clean_addr = address.trim_start_matches("0x");
    
    let _ = sqlx::query!(
        "DELETE FROM watch_list WHERE address = DECODE($1, 'hex')",
        clean_addr
    )
    .execute(&pool)
    .await;

    Json(serde_json::json!({ "status": "ok" }))
}
