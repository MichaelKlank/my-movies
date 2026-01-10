use chrono::Utc;
use uuid::Uuid;

use crate::db::DbPool;
use crate::error::{Error, Result};
use crate::models::{CreateSeries, Series, SeriesFilter, UpdateSeries};

pub struct SeriesService {
    pool: DbPool,
}

impl SeriesService {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, user_id: Uuid, input: CreateSeries) -> Result<Series> {
        let id = Uuid::new_v4();
        let now = Utc::now();

        sqlx::query(
            r#"
            INSERT INTO series (id, user_id, barcode, tmdb_id, title, disc_type, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(id)
        .bind(user_id)
        .bind(&input.barcode)
        .bind(input.tmdb_id)
        .bind(&input.title)
        .bind(&input.disc_type)
        .bind(now.to_rfc3339())
        .bind(now.to_rfc3339())
        .execute(&self.pool)
        .await?;

        self.get_by_id(user_id, id).await
    }

    pub async fn get_by_id(&self, user_id: Uuid, id: Uuid) -> Result<Series> {
        sqlx::query_as::<_, Series>("SELECT * FROM series WHERE id = ? AND user_id = ?")
            .bind(id)
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or(Error::NotFound)
    }

    pub async fn list(&self, user_id: Uuid, filter: SeriesFilter) -> Result<Vec<Series>> {
        let limit = filter.limit; // None = no limit
        let offset = filter.offset.unwrap_or(0);

        let series = if let Some(lim) = limit {
            sqlx::query_as::<_, Series>(
                "SELECT * FROM series WHERE user_id = ? ORDER BY title LIMIT ? OFFSET ?",
            )
            .bind(user_id)
            .bind(lim)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as::<_, Series>("SELECT * FROM series WHERE user_id = ? ORDER BY title")
                .bind(user_id)
                .fetch_all(&self.pool)
                .await?
        };

        Ok(series)
    }

    pub async fn update(&self, user_id: Uuid, id: Uuid, input: UpdateSeries) -> Result<Series> {
        // Verify ownership
        let _ = self.get_by_id(user_id, id).await?;

        // Update fields (simplified - add more as needed)
        if let Some(ref title) = input.title {
            sqlx::query("UPDATE series SET title = ? WHERE id = ? AND user_id = ?")
                .bind(title)
                .bind(id)
                .bind(user_id)
                .execute(&self.pool)
                .await?;
        }

        self.get_by_id(user_id, id).await
    }

    pub async fn delete(&self, user_id: Uuid, id: Uuid) -> Result<()> {
        let result = sqlx::query("DELETE FROM series WHERE id = ? AND user_id = ?")
            .bind(id)
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(Error::NotFound);
        }

        Ok(())
    }

    pub async fn count(&self, user_id: Uuid) -> Result<i64> {
        let count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM series WHERE user_id = ?")
            .bind(user_id)
            .fetch_one(&self.pool)
            .await?;

        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::{create_test_db_with_users, fixtures};

    async fn setup() -> SeriesService {
        let pool = create_test_db_with_users().await;
        SeriesService::new(pool)
    }

    #[tokio::test]
    async fn test_create_series() {
        let service = setup().await;
        let user_id = fixtures::test_user_id();

        let series = service
            .create(
                user_id,
                CreateSeries {
                    barcode: Some("1234567890123".to_string()),
                    tmdb_id: Some(1396),
                    title: "Breaking Bad".to_string(),
                    disc_type: Some("Blu-ray".to_string()),
                },
            )
            .await
            .unwrap();

        assert_eq!(series.title, "Breaking Bad");
        assert_eq!(series.tmdb_id, Some(1396));
    }

    #[tokio::test]
    async fn test_get_series_by_id() {
        let service = setup().await;
        let user_id = fixtures::test_user_id();

        let created = service
            .create(
                user_id,
                CreateSeries {
                    barcode: None,
                    tmdb_id: Some(1396),
                    title: "Breaking Bad".to_string(),
                    disc_type: None,
                },
            )
            .await
            .unwrap();

        let retrieved = service.get_by_id(user_id, created.id).await.unwrap();
        assert_eq!(retrieved.id, created.id);
        assert_eq!(retrieved.title, "Breaking Bad");
    }

    #[tokio::test]
    async fn test_get_nonexistent_series_fails() {
        let service = setup().await;
        let user_id = fixtures::test_user_id();

        let result = service.get_by_id(user_id, Uuid::new_v4()).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            Error::NotFound => {}
            e => panic!("Expected NotFound error, got {:?}", e),
        }
    }

    #[tokio::test]
    async fn test_list_series() {
        let service = setup().await;
        let user_id = fixtures::test_user_id();

        for i in 1..=3 {
            service
                .create(
                    user_id,
                    CreateSeries {
                        barcode: None,
                        tmdb_id: Some(i),
                        title: format!("Series {}", i),
                        disc_type: None,
                    },
                )
                .await
                .unwrap();
        }

        let series = service
            .list(user_id, SeriesFilter::default())
            .await
            .unwrap();
        assert_eq!(series.len(), 3);
    }

    #[tokio::test]
    async fn test_list_series_with_limit() {
        let service = setup().await;
        let user_id = fixtures::test_user_id();

        for i in 1..=10 {
            service
                .create(
                    user_id,
                    CreateSeries {
                        barcode: None,
                        tmdb_id: Some(i),
                        title: format!("Series {}", i),
                        disc_type: None,
                    },
                )
                .await
                .unwrap();
        }

        let series = service
            .list(
                user_id,
                SeriesFilter {
                    search: None,
                    genre: None,
                    network: None,
                    watched: None,
                    sort_by: None,
                    sort_order: None,
                    limit: Some(5),
                    offset: None,
                },
            )
            .await
            .unwrap();
        assert_eq!(series.len(), 5);
    }

    #[tokio::test]
    async fn test_update_series() {
        let service = setup().await;
        let user_id = fixtures::test_user_id();

        let series = service
            .create(
                user_id,
                CreateSeries {
                    barcode: None,
                    tmdb_id: Some(1396),
                    title: "Breaking Bad".to_string(),
                    disc_type: None,
                },
            )
            .await
            .unwrap();

        let updated = service
            .update(
                user_id,
                series.id,
                UpdateSeries {
                    title: Some("Breaking Bad (Complete Series)".to_string()),
                    original_title: None,
                    description: None,
                    network: None,
                    episodes_count: None,
                    watched: None,
                    personal_rating: None,
                    location: None,
                    notes: None,
                },
            )
            .await
            .unwrap();

        assert_eq!(updated.title, "Breaking Bad (Complete Series)");
    }

    #[tokio::test]
    async fn test_delete_series() {
        let service = setup().await;
        let user_id = fixtures::test_user_id();

        let series = service
            .create(
                user_id,
                CreateSeries {
                    barcode: None,
                    tmdb_id: Some(1396),
                    title: "Breaking Bad".to_string(),
                    disc_type: None,
                },
            )
            .await
            .unwrap();

        service.delete(user_id, series.id).await.unwrap();

        let result = service.get_by_id(user_id, series.id).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_delete_nonexistent_series_fails() {
        let service = setup().await;
        let user_id = fixtures::test_user_id();

        let result = service.delete(user_id, Uuid::new_v4()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_count_series() {
        let service = setup().await;
        let user_id = fixtures::test_user_id();

        for i in 1..=5 {
            service
                .create(
                    user_id,
                    CreateSeries {
                        barcode: None,
                        tmdb_id: Some(i),
                        title: format!("Series {}", i),
                        disc_type: None,
                    },
                )
                .await
                .unwrap();
        }

        let count = service.count(user_id).await.unwrap();
        assert_eq!(count, 5);
    }
}
