use axum::{extract::State, routing::get, Json, Router};
use sqlx::PgPool;
use tower_http::cors::CorsLayer;

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    let pool = PgPool::connect(&std::env::var("DATABASE_URL").unwrap())
        .await
        .unwrap();

    let app = Router::new()
        .route("/bundle/balance", get(get_bundle_balance))
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
