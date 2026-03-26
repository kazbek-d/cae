use axum::{routing::get, Router, extract::State, Json};
use sqlx::PgPool;
use cae_types::AuditEntry;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();

    let db_url = std::env::var("DATABASE_URL")?;
    let pool = PgPool::connect(&db_url).await?;

    let app = Router::new()
        .route("/ledger", get(get_ledger))
        .with_state(pool);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    tracing::info!("📡 API started on :3000");
    axum::serve(listener, app).await?;
    Ok(())
}

async fn get_ledger(State(pool): State<PgPool>) -> Json<Vec<AuditEntry>> {
    // In reality, this will be a SELECT from ledger_entries
    Json(vec![]) 
}