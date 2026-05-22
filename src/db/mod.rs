use sqlx::SqlitePool;
use sqlx::sqlite::SqlitePoolOptions;

pub async fn create_db_pool() -> Result<SqlitePool, sqlx::Error> {
    SqlitePoolOptions::new()
        .max_connections(5)
        .connect("patients.db")
        .await
}
