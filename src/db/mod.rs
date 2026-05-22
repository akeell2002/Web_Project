use sqlx::sqlite::SqlitePoolOptions;
use sqlx::SqlitePool;

pub async fn create_db_pool() -> Result<SqlitePool, sqlx::Error> {
    SqlitePoolOptions::new()
        .max_connections(5)
        .connect("patients.db")
        .await
}