//! Test helpers for creating in-memory test databases and fixtures

use crate::db::DbPool;
use chrono::Utc;
use sqlx::sqlite::SqlitePoolOptions;

/// Creates an in-memory SQLite database with all migrations applied
pub async fn create_test_db() -> DbPool {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("Failed to create test database");

    // Run migrations
    sqlx::migrate!("./src/db/migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    pool
}

/// Creates a test database with pre-seeded test users
pub async fn create_test_db_with_users() -> DbPool {
    let pool = create_test_db().await;

    let now = Utc::now().to_rfc3339();

    // Create test user
    sqlx::query(
        r#"INSERT INTO users (id, username, email, password_hash, role, created_at, updated_at)
           VALUES (?, 'testuser', 'test@test.com', 'hash', 'user', ?, ?)"#,
    )
    .bind(fixtures::test_user_id())
    .bind(&now)
    .bind(&now)
    .execute(&pool)
    .await
    .expect("Failed to create test user");

    // Create test admin
    sqlx::query(
        r#"INSERT INTO users (id, username, email, password_hash, role, created_at, updated_at)
           VALUES (?, 'testadmin', 'admin@test.com', 'hash', 'admin', ?, ?)"#,
    )
    .bind(fixtures::test_admin_id())
    .bind(&now)
    .bind(&now)
    .execute(&pool)
    .await
    .expect("Failed to create test admin");

    pool
}

/// Test fixtures for common test data
pub mod fixtures {
    use uuid::Uuid;

    pub fn test_user_id() -> Uuid {
        Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap()
    }

    pub fn test_admin_id() -> Uuid {
        Uuid::parse_str("00000000-0000-0000-0000-000000000002").unwrap()
    }

    pub fn test_movie_id() -> Uuid {
        Uuid::parse_str("00000000-0000-0000-0000-000000000010").unwrap()
    }

    pub fn test_collection_id() -> Uuid {
        Uuid::parse_str("00000000-0000-0000-0000-000000000020").unwrap()
    }
}
