#[cfg(test)]
mod tests {
    use sqlx::PgPool;

    #[tokio::test]
    async fn test_db_connection() {
        let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL required");
        let pool = PgPool::connect(&db_url).await.unwrap();
        //let res = sqlx::query!("SELECT 1 as id").fetch_one(&pool).await.unwrap();
        //assert_eq!(res.id, Some(1));
    }
}