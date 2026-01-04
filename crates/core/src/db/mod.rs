use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};
use std::time::Duration;

pub type DbPool = SqlitePool;

pub async fn create_pool(database_url: &str) -> Result<DbPool, sqlx::Error> {
    // Ensure the data directory exists
    if let Some(path) = database_url.strip_prefix("sqlite:") {
        if let Some(parent) = std::path::Path::new(path).parent() {
            std::fs::create_dir_all(parent).ok();
        }
    }

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(3))
        .connect(database_url)
        .await?;

    // Run migrations
    sqlx::migrate!("src/db/migrations")
        .run(&pool)
        .await?;

    Ok(pool)
}
