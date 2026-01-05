use chrono::Utc;
use uuid::Uuid;

use crate::db::DbPool;
use crate::error::{Error, Result};
use crate::models::{
    AddCollectionItem, Collection, CollectionFilter, CollectionItem, CreateCollection,
    UpdateCollection,
};

pub struct CollectionService {
    pool: DbPool,
}

impl CollectionService {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, user_id: Uuid, input: CreateCollection) -> Result<Collection> {
        let id = Uuid::new_v4();
        let now = Utc::now();

        sqlx::query(
            r#"
            INSERT INTO collections (id, user_id, barcode, title, description, disc_type, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(id)
        .bind(user_id)
        .bind(&input.barcode)
        .bind(&input.title)
        .bind(&input.description)
        .bind(&input.disc_type)
        .bind(now.to_rfc3339())
        .bind(now.to_rfc3339())
        .execute(&self.pool)
        .await?;

        self.get_by_id(user_id, id).await
    }

    pub async fn get_by_id(&self, user_id: Uuid, id: Uuid) -> Result<Collection> {
        sqlx::query_as::<_, Collection>("SELECT * FROM collections WHERE id = ? AND user_id = ?")
            .bind(id)
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or(Error::NotFound)
    }

    pub async fn list(&self, user_id: Uuid, filter: CollectionFilter) -> Result<Vec<Collection>> {
        let limit = filter.limit.unwrap_or(50);
        let offset = filter.offset.unwrap_or(0);

        let collections = sqlx::query_as::<_, Collection>(
            "SELECT * FROM collections WHERE user_id = ? ORDER BY title LIMIT ? OFFSET ?",
        )
        .bind(user_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok(collections)
    }

    pub async fn update(
        &self,
        user_id: Uuid,
        id: Uuid,
        input: UpdateCollection,
    ) -> Result<Collection> {
        // Verify ownership
        let _ = self.get_by_id(user_id, id).await?;

        if let Some(ref title) = input.title {
            sqlx::query("UPDATE collections SET title = ? WHERE id = ? AND user_id = ?")
                .bind(title)
                .bind(id)
                .bind(user_id)
                .execute(&self.pool)
                .await?;
        }

        self.get_by_id(user_id, id).await
    }

    pub async fn delete(&self, user_id: Uuid, id: Uuid) -> Result<()> {
        let result = sqlx::query("DELETE FROM collections WHERE id = ? AND user_id = ?")
            .bind(id)
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(Error::NotFound);
        }

        Ok(())
    }

    pub async fn add_item(
        &self,
        user_id: Uuid,
        collection_id: Uuid,
        input: AddCollectionItem,
    ) -> Result<CollectionItem> {
        // Verify collection ownership
        let _ = self.get_by_id(user_id, collection_id).await?;

        let id = Uuid::new_v4();
        let now = Utc::now();
        let position = input.position.unwrap_or(0);

        let item_type = match input.item_type {
            crate::models::CollectionItemType::Movie => "movie",
            crate::models::CollectionItemType::Series => "series",
        };

        sqlx::query(
            r#"
            INSERT INTO collection_items (id, collection_id, item_type, movie_id, series_id, position, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(id)
        .bind(collection_id)
        .bind(item_type)
        .bind(input.movie_id)
        .bind(input.series_id)
        .bind(position)
        .bind(now.to_rfc3339())
        .execute(&self.pool)
        .await?;

        sqlx::query_as::<_, CollectionItem>("SELECT * FROM collection_items WHERE id = ?")
            .bind(id)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| e.into())
    }

    pub async fn get_items(
        &self,
        user_id: Uuid,
        collection_id: Uuid,
    ) -> Result<Vec<CollectionItem>> {
        // Verify collection ownership
        let _ = self.get_by_id(user_id, collection_id).await?;

        let items = sqlx::query_as::<_, CollectionItem>(
            "SELECT * FROM collection_items WHERE collection_id = ? ORDER BY position",
        )
        .bind(collection_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(items)
    }

    pub async fn remove_item(
        &self,
        user_id: Uuid,
        collection_id: Uuid,
        item_id: Uuid,
    ) -> Result<()> {
        // Verify collection ownership
        let _ = self.get_by_id(user_id, collection_id).await?;

        let result = sqlx::query("DELETE FROM collection_items WHERE id = ? AND collection_id = ?")
            .bind(item_id)
            .bind(collection_id)
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(Error::NotFound);
        }

        Ok(())
    }
}
