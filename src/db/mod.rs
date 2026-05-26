use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
pub mod users;
pub mod patients;

pub async fn create_db_pool() -> Result<PgPool, sqlx::Error> {
    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set in .env file");
    
    PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
}