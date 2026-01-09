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
                .bind(id.to_string())
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
