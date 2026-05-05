use sqlx::PgPool;
use std::env;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    dotenv::dotenv().ok();
    let db_url = env::var("DATABASE_URL")?;
    let pool = PgPool::connect(&db_url).await?;
    
    sqlx::query("ALTER TABLE ledger_entries ADD COLUMN IF NOT EXISTS wallet_address BYTEA;")
        .execute(&pool)
        .await?;
        
    println!("Migration successful: Added wallet_address to ledger_entries");
    Ok(())
}
