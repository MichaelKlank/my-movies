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
        let limit = filter.limit; // None = no limit
        let offset = filter.offset.unwrap_or(0);

        let collections = if let Some(lim) = limit {
            sqlx::query_as::<_, Collection>(
                "SELECT * FROM collections WHERE user_id = ? ORDER BY title LIMIT ? OFFSET ?",
            )
            .bind(user_id)
            .bind(lim)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as::<_, Collection>(
                "SELECT * FROM collections WHERE user_id = ? ORDER BY title",
            )
            .bind(user_id)
            .fetch_all(&self.pool)
            .await?
        };

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::{create_test_db_with_users, fixtures};

    async fn setup() -> CollectionService {
        let pool = create_test_db_with_users().await;
        CollectionService::new(pool)
    }

    #[tokio::test]
    async fn test_create_collection() {
        let service = setup().await;
        let user_id = fixtures::test_user_id();

        let collection = service
            .create(
                user_id,
                CreateCollection {
                    barcode: Some("1234567890123".to_string()),
                    title: "My Movie Collection".to_string(),
                    description: Some("A collection of great movies".to_string()),
                    disc_type: Some("Blu-ray".to_string()),
                },
            )
            .await
            .unwrap();

        assert_eq!(collection.title, "My Movie Collection");
        assert_eq!(collection.barcode, Some("1234567890123".to_string()));
    }

    #[tokio::test]
    async fn test_get_collection_by_id() {
        let service = setup().await;
        let user_id = fixtures::test_user_id();

        let created = service
            .create(
                user_id,
                CreateCollection {
                    barcode: None,
                    title: "Test Collection".to_string(),
                    description: None,
                    disc_type: None,
                },
            )
            .await
            .unwrap();

        let retrieved = service.get_by_id(user_id, created.id).await.unwrap();
        assert_eq!(retrieved.id, created.id);
        assert_eq!(retrieved.title, "Test Collection");
    }

    #[tokio::test]
    async fn test_get_nonexistent_collection_fails() {
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
    async fn test_list_collections() {
        let service = setup().await;
        let user_id = fixtures::test_user_id();

        for i in 1..=3 {
            service
                .create(
                    user_id,
                    CreateCollection {
                        barcode: None,
                        title: format!("Collection {}", i),
                        description: None,
                        disc_type: None,
                    },
                )
                .await
                .unwrap();
        }

        let collections = service
            .list(user_id, CollectionFilter::default())
            .await
            .unwrap();
        assert_eq!(collections.len(), 3);
    }

    #[tokio::test]
    async fn test_list_collections_with_limit() {
        let service = setup().await;
        let user_id = fixtures::test_user_id();

        for i in 1..=10 {
            service
                .create(
                    user_id,
                    CreateCollection {
                        barcode: None,
                        title: format!("Collection {}", i),
                        description: None,
                        disc_type: None,
                    },
                )
                .await
                .unwrap();
        }

        let collections = service
            .list(
                user_id,
                CollectionFilter {
                    limit: Some(5),
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        assert_eq!(collections.len(), 5);
    }

    #[tokio::test]
    async fn test_update_collection() {
        let service = setup().await;
        let user_id = fixtures::test_user_id();

        let collection = service
            .create(
                user_id,
                CreateCollection {
                    barcode: None,
                    title: "Original Title".to_string(),
                    description: None,
                    disc_type: None,
                },
            )
            .await
            .unwrap();

        let updated = service
            .update(
                user_id,
                collection.id,
                UpdateCollection {
                    title: Some("Updated Title".to_string()),
                    ..Default::default()
                },
            )
            .await
            .unwrap();

        assert_eq!(updated.title, "Updated Title");
    }

    #[tokio::test]
    async fn test_delete_collection() {
        let service = setup().await;
        let user_id = fixtures::test_user_id();

        let collection = service
            .create(
                user_id,
                CreateCollection {
                    barcode: None,
                    title: "To Delete".to_string(),
                    description: None,
                    disc_type: None,
                },
            )
            .await
            .unwrap();

        service.delete(user_id, collection.id).await.unwrap();

        let result = service.get_by_id(user_id, collection.id).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_delete_nonexistent_collection_fails() {
        let service = setup().await;
        let user_id = fixtures::test_user_id();

        let result = service.delete(user_id, Uuid::new_v4()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_collection_isolation_between_users() {
        let service = setup().await;
        let user1 = fixtures::test_user_id();
        let user2 = fixtures::test_admin_id();

        // User 1 creates a collection
        let collection = service
            .create(
                user1,
                CreateCollection {
                    barcode: None,
                    title: "User 1 Collection".to_string(),
                    description: None,
                    disc_type: None,
                },
            )
            .await
            .unwrap();

        // User 2 should not be able to access it
        let result = service.get_by_id(user2, collection.id).await;
        assert!(result.is_err());

        // User 1's list should have the collection
        let user1_collections = service
            .list(user1, CollectionFilter::default())
            .await
            .unwrap();
        assert_eq!(user1_collections.len(), 1);

        // User 2's list should be empty
        let user2_collections = service
            .list(user2, CollectionFilter::default())
            .await
            .unwrap();
        assert!(user2_collections.is_empty());
    }

    #[tokio::test]
    async fn test_add_item_to_collection() {
        let pool = create_test_db_with_users().await;
        let collection_service = CollectionService::new(pool.clone());
        let movie_service = crate::services::movies::MovieService::new(pool);
        let user_id = fixtures::test_user_id();

        // Create a collection
        let collection = collection_service
            .create(
                user_id,
                CreateCollection {
                    barcode: None,
                    title: "My Collection".to_string(),
                    description: None,
                    disc_type: None,
                },
            )
            .await
            .unwrap();

        // Create a movie
        let movie = movie_service
            .create(
                user_id,
                crate::models::movie::CreateMovie {
                    barcode: None,
                    tmdb_id: Some(550),
                    title: "Fight Club".to_string(),
                    original_title: None,
                    disc_type: None,
                    production_year: None,
                },
            )
            .await
            .unwrap();

        // Add movie to collection
        collection_service
            .add_item(
                user_id,
                collection.id,
                AddCollectionItem {
                    item_type: crate::models::collection::CollectionItemType::Movie,
                    movie_id: Some(movie.id),
                    series_id: None,
                    position: None,
                },
            )
            .await
            .unwrap();

        // Get items
        let items = collection_service
            .get_items(user_id, collection.id)
            .await
            .unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].movie_id, Some(movie.id));
    }

    #[tokio::test]
    async fn test_remove_item_from_collection() {
        let pool = create_test_db_with_users().await;
        let collection_service = CollectionService::new(pool.clone());
        let movie_service = crate::services::movies::MovieService::new(pool);
        let user_id = fixtures::test_user_id();

        // Create collection and movie
        let collection = collection_service
            .create(
                user_id,
                CreateCollection {
                    barcode: None,
                    title: "My Collection".to_string(),
                    description: None,
                    disc_type: None,
                },
            )
            .await
            .unwrap();

        let movie = movie_service
            .create(
                user_id,
                crate::models::movie::CreateMovie {
                    barcode: None,
                    tmdb_id: Some(550),
                    title: "Fight Club".to_string(),
                    original_title: None,
                    disc_type: None,
                    production_year: None,
                },
            )
            .await
            .unwrap();

        // Add and then remove
        let item = collection_service
            .add_item(
                user_id,
                collection.id,
                AddCollectionItem {
                    item_type: crate::models::collection::CollectionItemType::Movie,
                    movie_id: Some(movie.id),
                    series_id: None,
                    position: None,
                },
            )
            .await
            .unwrap();

        collection_service
            .remove_item(user_id, collection.id, item.id)
            .await
            .unwrap();

        // Items should be empty
        let items = collection_service
            .get_items(user_id, collection.id)
            .await
            .unwrap();
        assert!(items.is_empty());
    }

    #[tokio::test]
    async fn test_list_collections_with_offset() {
        let service = setup().await;
        let user_id = fixtures::test_user_id();

        for i in 1..=5 {
            service
                .create(
                    user_id,
                    CreateCollection {
                        barcode: None,
                        title: format!("Collection {}", i),
                        description: None,
                        disc_type: None,
                    },
                )
                .await
                .unwrap();
        }

        // Get with offset
        let collections = service
            .list(
                user_id,
                CollectionFilter {
                    limit: Some(2),
                    offset: Some(2),
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        assert_eq!(collections.len(), 2);
    }
}
