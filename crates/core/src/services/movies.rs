use chrono::Utc;
use uuid::Uuid;

use crate::db::DbPool;
use crate::error::{Error, Result};
use crate::models::{CreateMovie, Movie, MovieFilter, UpdateMovie};

/// Check if a barcode is a placeholder/invalid value that shouldn't be used for duplicate detection
fn is_placeholder_barcode(barcode: &str) -> bool {
    // All zeros (any length) - e.g., "000000000000"
    if barcode.chars().all(|c| c == '0') {
        return true;
    }
    // All same digit (e.g., "111111111111", "999999999999")
    if let Some(first) = barcode.chars().next()
        && first.is_ascii_digit()
        && barcode.chars().all(|c| c == first)
    {
        return true;
    }
    false
}

pub struct MovieService {
    pool: DbPool,
}

impl MovieService {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, user_id: Uuid, input: CreateMovie) -> Result<Movie> {
        let id = Uuid::new_v4();
        let now = Utc::now();

        sqlx::query(
            r#"
            INSERT INTO movies (
                id, user_id, barcode, tmdb_id, title, original_title, 
                disc_type, production_year, created_at, updated_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(id)
        .bind(user_id)
        .bind(&input.barcode)
        .bind(input.tmdb_id)
        .bind(&input.title)
        .bind(&input.original_title)
        .bind(&input.disc_type)
        .bind(input.production_year)
        .bind(now.to_rfc3339())
        .bind(now.to_rfc3339())
        .execute(&self.pool)
        .await?;

        self.get_by_id(user_id, id).await
    }

    pub async fn get_by_id(&self, user_id: Uuid, id: Uuid) -> Result<Movie> {
        sqlx::query_as::<_, Movie>("SELECT * FROM movies WHERE id = ? AND user_id = ?")
            .bind(id)
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or(Error::NotFound)
    }

    pub async fn count(&self, user_id: Uuid, filter: &MovieFilter) -> Result<i64> {
        let mut query = String::from("SELECT COUNT(*) as count FROM movies WHERE user_id = ?");

        if filter.search.is_some() {
            query.push_str(" AND (title LIKE ? OR original_title LIKE ? OR director LIKE ?)");
        }

        if filter.genre.is_some() {
            query.push_str(" AND genres LIKE ?");
        }

        if filter.disc_type.is_some() {
            query.push_str(" AND disc_type = ?");
        }

        if filter.watched.is_some() {
            query.push_str(" AND watched = ?");
        }

        if filter.year_from.is_some() {
            query.push_str(" AND production_year >= ?");
        }

        if filter.year_to.is_some() {
            query.push_str(" AND production_year <= ?");
        }

        let mut q = sqlx::query_scalar::<_, i64>(&query).bind(user_id);

        if let Some(ref search) = filter.search {
            let search_pattern = format!("%{}%", search);
            q = q
                .bind(search_pattern.clone())
                .bind(search_pattern.clone())
                .bind(search_pattern);
        }

        if let Some(ref genre) = filter.genre {
            q = q.bind(format!("%{}%", genre));
        }

        if let Some(ref disc_type) = filter.disc_type {
            q = q.bind(disc_type);
        }

        if let Some(watched) = filter.watched {
            q = q.bind(watched);
        }

        if let Some(year_from) = filter.year_from {
            q = q.bind(year_from);
        }

        if let Some(year_to) = filter.year_to {
            q = q.bind(year_to);
        }

        let count = q.fetch_one(&self.pool).await?;
        Ok(count)
    }

    pub async fn list(&self, user_id: Uuid, filter: MovieFilter) -> Result<Vec<Movie>> {
        let limit = filter.limit; // None = no limit (return all)
        let offset = filter.offset.unwrap_or(0);
        let sort_by = filter.sort_by.unwrap_or_else(|| "title".to_string());
        let sort_order = filter.sort_order.unwrap_or_else(|| "asc".to_string());

        // Build dynamic query string first
        // Exclude poster_data from list queries for performance (it's large BLOB data)
        let mut query = String::from(
            "SELECT id, user_id, collection_number, barcode, tmdb_id, imdb_id, title, original_title, \
            sort_title, personal_title, personal_sort_title, description, tagline, production_year, \
            release_date, running_time, director, actors, production_companies, production_countries, \
            studios, rating, personal_rating, disc_type, media_type, discs, region_codes, video_standard, \
            aspect_ratio, audio_tracks, subtitles, is_3d, mastered_in_4k, genres, categories, tags, \
            movie_group, is_collection, parent_collection_id, watched, digital_copies, status, condition, slip_cover, cover_type, edition, \
            extra_features, purchase_date, price, currency, purchase_place, value_date, value_price, \
            value_currency, lent_to, lent_due, location, notes, budget, revenue, spoken_languages, \
            added_date, created_at, updated_at FROM movies WHERE user_id = ?",
        );

        if filter.search.is_some() {
            query.push_str(" AND (title LIKE ? OR original_title LIKE ? OR director LIKE ?)");
        }

        if filter.genre.is_some() {
            query.push_str(" AND genres LIKE ?");
        }

        if filter.disc_type.is_some() {
            query.push_str(" AND disc_type = ?");
        }

        if filter.watched.is_some() {
            query.push_str(" AND watched = ?");
        }

        if filter.is_collection.is_some() {
            query.push_str(" AND is_collection = ?");
        }

        if filter.exclude_collection_children == Some(true) {
            query.push_str(" AND parent_collection_id IS NULL");
        }

        if filter.year_from.is_some() {
            query.push_str(" AND production_year >= ?");
        }

        if filter.year_to.is_some() {
            query.push_str(" AND production_year <= ?");
        }

        // Whitelist allowed sort columns
        let allowed_sorts = [
            "title",
            "sort_title",
            "production_year",
            "created_at",
            "personal_rating",
        ];
        let sort_column = if allowed_sorts.contains(&sort_by.as_str()) {
            sort_by
        } else {
            "title".to_string()
        };

        let order = if sort_order.to_lowercase() == "desc" {
            "DESC"
        } else {
            "ASC"
        };

        // Use COALESCE to handle NULL sort_title (fall back to title)
        // Use COLLATE NOCASE for case-insensitive sorting
        let order_clause = if sort_column == "sort_title" {
            format!("COALESCE(sort_title, title) COLLATE NOCASE {}", order)
        } else {
            format!("{} COLLATE NOCASE {}", sort_column, order)
        };

        // Add ORDER BY, and optionally LIMIT/OFFSET
        if limit.is_some() {
            query.push_str(&format!(" ORDER BY {} LIMIT ? OFFSET ?", order_clause));
        } else {
            query.push_str(&format!(" ORDER BY {}", order_clause));
        }

        // Now bind all parameters in the correct order
        let mut q = sqlx::query_as::<_, Movie>(&query).bind(user_id);

        if let Some(ref search) = filter.search {
            let search_pattern = format!("%{}%", search);
            q = q
                .bind(search_pattern.clone())
                .bind(search_pattern.clone())
                .bind(search_pattern);
        }

        if let Some(ref genre) = filter.genre {
            q = q.bind(format!("%{}%", genre));
        }

        if let Some(ref disc_type) = filter.disc_type {
            q = q.bind(disc_type);
        }

        if let Some(watched) = filter.watched {
            q = q.bind(watched);
        }

        if let Some(is_collection) = filter.is_collection {
            q = q.bind(is_collection);
        }

        // Note: exclude_collection_children doesn't need a bind (IS NULL check)

        if let Some(year_from) = filter.year_from {
            q = q.bind(year_from);
        }

        if let Some(year_to) = filter.year_to {
            q = q.bind(year_to);
        }

        // Only bind limit/offset if limit is specified
        let rows = if let Some(lim) = limit {
            q.bind(lim).bind(offset).fetch_all(&self.pool).await?
        } else {
            q.fetch_all(&self.pool).await?
        };
        Ok(rows)
    }

    pub async fn update(&self, user_id: Uuid, id: Uuid, input: UpdateMovie) -> Result<Movie> {
        // Verify ownership first
        let _ = self.get_by_id(user_id, id).await?;

        // Update each field individually if provided
        if let Some(ref title) = input.title {
            sqlx::query("UPDATE movies SET title = ? WHERE id = ? AND user_id = ?")
                .bind(title)
                .bind(id)
                .bind(user_id)
                .execute(&self.pool)
                .await?;
        }

        if let Some(ref description) = input.description {
            sqlx::query("UPDATE movies SET description = ? WHERE id = ? AND user_id = ?")
                .bind(description)
                .bind(id)
                .bind(user_id)
                .execute(&self.pool)
                .await?;
        }

        if let Some(watched) = input.watched {
            sqlx::query("UPDATE movies SET watched = ? WHERE id = ? AND user_id = ?")
                .bind(watched)
                .bind(id)
                .bind(user_id)
                .execute(&self.pool)
                .await?;
        }

        if let Some(rating) = input.personal_rating {
            sqlx::query("UPDATE movies SET personal_rating = ? WHERE id = ? AND user_id = ?")
                .bind(rating)
                .bind(id)
                .bind(user_id)
                .execute(&self.pool)
                .await?;
        }

        if let Some(ref location) = input.location {
            sqlx::query("UPDATE movies SET location = ? WHERE id = ? AND user_id = ?")
                .bind(location)
                .bind(id)
                .bind(user_id)
                .execute(&self.pool)
                .await?;
        }

        if let Some(ref notes) = input.notes {
            sqlx::query("UPDATE movies SET notes = ? WHERE id = ? AND user_id = ?")
                .bind(notes)
                .bind(id)
                .bind(user_id)
                .execute(&self.pool)
                .await?;
        }

        if let Some(tmdb_id) = input.tmdb_id {
            sqlx::query("UPDATE movies SET tmdb_id = ? WHERE id = ? AND user_id = ?")
                .bind(tmdb_id)
                .bind(id)
                .bind(user_id)
                .execute(&self.pool)
                .await?;
        }

        if let Some(ref imdb_id) = input.imdb_id {
            sqlx::query("UPDATE movies SET imdb_id = ? WHERE id = ? AND user_id = ?")
                .bind(imdb_id)
                .bind(id)
                .bind(user_id)
                .execute(&self.pool)
                .await?;
        }

        if let Some(ref original_title) = input.original_title {
            sqlx::query("UPDATE movies SET original_title = ? WHERE id = ? AND user_id = ?")
                .bind(original_title)
                .bind(id)
                .bind(user_id)
                .execute(&self.pool)
                .await?;
        }

        if let Some(ref tagline) = input.tagline {
            sqlx::query("UPDATE movies SET tagline = ? WHERE id = ? AND user_id = ?")
                .bind(tagline)
                .bind(id)
                .bind(user_id)
                .execute(&self.pool)
                .await?;
        }

        if let Some(running_time) = input.running_time {
            sqlx::query("UPDATE movies SET running_time = ? WHERE id = ? AND user_id = ?")
                .bind(running_time)
                .bind(id)
                .bind(user_id)
                .execute(&self.pool)
                .await?;
        }

        if let Some(ref director) = input.director {
            sqlx::query("UPDATE movies SET director = ? WHERE id = ? AND user_id = ?")
                .bind(director)
                .bind(id)
                .bind(user_id)
                .execute(&self.pool)
                .await?;
        }

        if let Some(ref actors) = input.actors {
            sqlx::query("UPDATE movies SET actors = ? WHERE id = ? AND user_id = ?")
                .bind(actors)
                .bind(id)
                .bind(user_id)
                .execute(&self.pool)
                .await?;
        }

        if let Some(ref genres) = input.genres {
            sqlx::query("UPDATE movies SET genres = ? WHERE id = ? AND user_id = ?")
                .bind(genres)
                .bind(id)
                .bind(user_id)
                .execute(&self.pool)
                .await?;
        }

        if let Some(budget) = input.budget {
            sqlx::query("UPDATE movies SET budget = ? WHERE id = ? AND user_id = ?")
                .bind(budget)
                .bind(id)
                .bind(user_id)
                .execute(&self.pool)
                .await?;
        }

        if let Some(revenue) = input.revenue {
            sqlx::query("UPDATE movies SET revenue = ? WHERE id = ? AND user_id = ?")
                .bind(revenue)
                .bind(id)
                .bind(user_id)
                .execute(&self.pool)
                .await?;
        }

        if let Some(ref disc_type) = input.disc_type {
            sqlx::query("UPDATE movies SET disc_type = ? WHERE id = ? AND user_id = ?")
                .bind(disc_type)
                .bind(id)
                .bind(user_id)
                .execute(&self.pool)
                .await?;
        }

        if let Some(is_collection) = input.is_collection {
            sqlx::query("UPDATE movies SET is_collection = ? WHERE id = ? AND user_id = ?")
                .bind(is_collection)
                .bind(id)
                .bind(user_id)
                .execute(&self.pool)
                .await?;
        }

        if let Some(ref parent_collection_id) = input.parent_collection_id {
            sqlx::query("UPDATE movies SET parent_collection_id = ? WHERE id = ? AND user_id = ?")
                .bind(parent_collection_id)
                .bind(id)
                .bind(user_id)
                .execute(&self.pool)
                .await?;
        }

        if let Some(ref poster_data) = input.poster_data {
            sqlx::query("UPDATE movies SET poster_data = ? WHERE id = ? AND user_id = ?")
                .bind(poster_data)
                .bind(id)
                .bind(user_id)
                .execute(&self.pool)
                .await?;
        }

        // Update timestamp
        sqlx::query("UPDATE movies SET updated_at = ? WHERE id = ? AND user_id = ?")
            .bind(Utc::now().to_rfc3339())
            .bind(id)
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        self.get_by_id(user_id, id).await
    }

    pub async fn update_movie_poster_data(
        &self,
        user_id: Uuid,
        id: Uuid,
        poster_data: Option<Vec<u8>>,
    ) -> Result<Movie> {
        sqlx::query(
            "UPDATE movies SET poster_data = ?, updated_at = ? WHERE id = ? AND user_id = ?",
        )
        .bind(&poster_data)
        .bind(Utc::now().to_rfc3339())
        .bind(id)
        .bind(user_id)
        .execute(&self.pool)
        .await?;

        self.get_by_id(user_id, id).await
    }

    pub async fn get_movie_poster_data(&self, user_id: Uuid, id: Uuid) -> Result<Option<Vec<u8>>> {
        // Verify movie belongs to user
        let _movie = self.get_by_id(user_id, id).await?;

        let data = sqlx::query_scalar::<_, Option<Vec<u8>>>(
            "SELECT poster_data FROM movies WHERE id = ? AND user_id = ?",
        )
        .bind(id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(data.flatten())
    }

    pub async fn get_movie_poster_data_public(&self, id: Uuid) -> Result<Option<Vec<u8>>> {
        // Public method to get poster without user verification
        let data =
            sqlx::query_scalar::<_, Option<Vec<u8>>>("SELECT poster_data FROM movies WHERE id = ?")
                .bind(id)
                .fetch_optional(&self.pool)
                .await?;

        Ok(data.flatten())
    }

    pub async fn delete(&self, user_id: Uuid, id: Uuid) -> Result<()> {
        let result = sqlx::query("DELETE FROM movies WHERE id = ? AND user_id = ?")
            .bind(id)
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(Error::NotFound);
        }

        Ok(())
    }

    /// Delete all movies for a user
    pub async fn delete_all(&self, user_id: Uuid) -> Result<u64> {
        let result = sqlx::query("DELETE FROM movies WHERE user_id = ?")
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected())
    }

    /// Get IDs of movies that have poster data
    /// Used to filter which movies need enrichment without loading full poster blobs
    pub async fn get_movie_ids_with_poster(&self, user_id: Uuid) -> Result<Vec<Uuid>> {
        // Query returns (id,) tuples where id is a Uuid blob
        let rows: Vec<(Uuid,)> =
            sqlx::query_as("SELECT id FROM movies WHERE user_id = ? AND poster_data IS NOT NULL")
                .bind(user_id)
                .fetch_all(&self.pool)
                .await?;

        Ok(rows.into_iter().map(|(id,)| id).collect())
    }

    /// Get only the poster data for a movie (for export without loading full Movie struct)
    pub async fn get_poster_data(&self, user_id: Uuid, movie_id: Uuid) -> Result<Option<Vec<u8>>> {
        let row: Option<(Vec<u8>,)> =
            sqlx::query_as("SELECT poster_data FROM movies WHERE id = ? AND user_id = ? AND poster_data IS NOT NULL")
                .bind(movie_id)
                .bind(user_id)
                .fetch_optional(&self.pool)
                .await?;

        Ok(row.map(|(data,)| data))
    }

    pub async fn find_by_barcode(&self, user_id: Uuid, barcode: &str) -> Result<Option<Movie>> {
        sqlx::query_as::<_, Movie>("SELECT * FROM movies WHERE barcode = ? AND user_id = ?")
            .bind(barcode)
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(Into::into)
    }

    pub async fn find_by_tmdb_id(&self, user_id: Uuid, tmdb_id: i64) -> Result<Option<Movie>> {
        sqlx::query_as::<_, Movie>("SELECT * FROM movies WHERE tmdb_id = ? AND user_id = ?")
            .bind(tmdb_id)
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(Into::into)
    }

    pub async fn find_by_title(&self, user_id: Uuid, title: &str) -> Result<Vec<Movie>> {
        sqlx::query_as::<_, Movie>(
            "SELECT * FROM movies WHERE (title = ? OR original_title = ?) AND user_id = ?",
        )
        .bind(title)
        .bind(title)
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(Into::into)
    }

    /// Check for potential duplicates before adding
    pub async fn find_duplicates(
        &self,
        user_id: Uuid,
        title: &str,
        barcode: Option<&str>,
        tmdb_id: Option<i64>,
    ) -> Result<Vec<Movie>> {
        let mut duplicates = Vec::new();

        // Check by barcode first (most reliable)
        if let Some(bc) = barcode
            && !bc.is_empty()
            && let Some(movie) = self.find_by_barcode(user_id, bc).await?
        {
            duplicates.push(movie);
            return Ok(duplicates); // Barcode match is definitive
        }

        // Check by TMDB ID (exclude 0 as it's a placeholder)
        if let Some(id) = tmdb_id
            && id > 0
            && let Some(movie) = self.find_by_tmdb_id(user_id, id).await?
        {
            duplicates.push(movie);
            return Ok(duplicates); // TMDB ID match is definitive
        }

        // Check by title (less reliable, might have false positives)
        let title_matches = self.find_by_title(user_id, title).await?;
        duplicates.extend(title_matches);

        Ok(duplicates)
    }

    /// Find all duplicate movies in the collection
    pub async fn find_all_duplicates(&self, user_id: Uuid) -> Result<Vec<Vec<Movie>>> {
        // Get all movies
        let movies = self
            .list(
                user_id,
                MovieFilter {
                    limit: Some(10000),
                    ..Default::default()
                },
            )
            .await?;

        let mut duplicate_groups: Vec<Vec<Movie>> = Vec::new();
        let mut processed_ids: std::collections::HashSet<String> = std::collections::HashSet::new();

        for movie in &movies {
            if processed_ids.contains(&movie.id.to_string()) {
                continue;
            }

            let mut group = vec![movie.clone()];

            // Find duplicates by barcode (exclude empty and placeholder barcodes)
            // Note: Same barcode = definitely same physical item, so disc_type check not needed
            if let Some(ref barcode) = movie.barcode
                && !barcode.is_empty()
                && !is_placeholder_barcode(barcode)
            {
                for other in &movies {
                    if other.id != movie.id
                        && other.barcode.as_ref() == Some(barcode)
                        && !group.iter().any(|m| m.id == other.id)
                    {
                        group.push(other.clone());
                    }
                }
            }

            // Find duplicates by TMDB ID (exclude 0 as it's a placeholder)
            // IMPORTANT: Only consider duplicates if they have the SAME disc_type
            // (DVD vs Blu-Ray of the same movie are NOT duplicates)
            if let Some(tmdb_id) = movie.tmdb_id
                && tmdb_id > 0
            {
                for other in &movies {
                    if other.id != movie.id
                        && other.tmdb_id == Some(tmdb_id)
                        && same_disc_type(&movie.disc_type, &other.disc_type)
                        && !group.iter().any(|m| m.id == other.id)
                    {
                        group.push(other.clone());
                    }
                }
            }

            // Find duplicates by exact title match
            // IMPORTANT: Must have same disc_type AND for TV series, must be same season
            for other in &movies {
                if other.id != movie.id
                    && other.title.to_lowercase() == movie.title.to_lowercase()
                    && same_disc_type(&movie.disc_type, &other.disc_type)
                    && !group.iter().any(|m| m.id == other.id)
                {
                    // Exact title match = definitely same, add to group
                    group.push(other.clone());
                }
            }

            // Also check for similar titles that might be different seasons of same show
            // e.g., "Die Nanny - Staffel 1" vs "Die Nanny - Staffel 2" should NOT be duplicates
            // But "Die Nanny Season One" and "Die Nanny: Staffel 1" SHOULD be duplicates
            // Only exact title matches are considered duplicates (already handled above)

            if group.len() > 1 {
                for m in &group {
                    processed_ids.insert(m.id.to_string());
                }
                duplicate_groups.push(group);
            }
        }

        Ok(duplicate_groups)
    }
}

/// Helper function to check if two disc types are the same
/// Treats None and empty string as the same
fn same_disc_type(a: &Option<String>, b: &Option<String>) -> bool {
    let a_normalized = a
        .as_ref()
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty());
    let b_normalized = b
        .as_ref()
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty());
    a_normalized == b_normalized
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::{create_test_db_with_users, fixtures};

    async fn setup() -> MovieService {
        let pool = create_test_db_with_users().await;
        MovieService::new(pool)
    }

    #[tokio::test]
    async fn test_create_movie() {
        let service = setup().await;
        let user_id = fixtures::test_user_id();

        let movie = service
            .create(
                user_id,
                CreateMovie {
                    barcode: Some("1234567890123".to_string()),
                    tmdb_id: Some(550),
                    title: "Fight Club".to_string(),
                    original_title: Some("Fight Club".to_string()),
                    disc_type: Some("Blu-ray".to_string()),
                    production_year: Some(1999),
                },
            )
            .await
            .unwrap();

        assert_eq!(movie.title, "Fight Club");
        assert_eq!(movie.tmdb_id, Some(550));
        assert_eq!(movie.barcode, Some("1234567890123".to_string()));
        assert_eq!(movie.production_year, Some(1999));
    }

    #[tokio::test]
    async fn test_get_movie_by_id() {
        let service = setup().await;
        let user_id = fixtures::test_user_id();

        let created = service
            .create(
                user_id,
                CreateMovie {
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

        let retrieved = service.get_by_id(user_id, created.id).await.unwrap();
        assert_eq!(retrieved.id, created.id);
        assert_eq!(retrieved.title, "Fight Club");
    }

    #[tokio::test]
    async fn test_get_nonexistent_movie_fails() {
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
    async fn test_list_movies() {
        let service = setup().await;
        let user_id = fixtures::test_user_id();

        // Create multiple movies
        for i in 1..=5i32 {
            service
                .create(
                    user_id,
                    CreateMovie {
                        barcode: None,
                        tmdb_id: Some(i as i64),
                        title: format!("Movie {}", i),
                        original_title: None,
                        disc_type: None,
                        production_year: Some(2000 + i),
                    },
                )
                .await
                .unwrap();
        }

        let movies = service.list(user_id, MovieFilter::default()).await.unwrap();
        assert_eq!(movies.len(), 5);
    }

    #[tokio::test]
    async fn test_list_movies_with_limit() {
        let service = setup().await;
        let user_id = fixtures::test_user_id();

        for i in 1..=10i64 {
            service
                .create(
                    user_id,
                    CreateMovie {
                        barcode: None,
                        tmdb_id: Some(i),
                        title: format!("Movie {}", i),
                        original_title: None,
                        disc_type: None,
                        production_year: None,
                    },
                )
                .await
                .unwrap();
        }

        let movies = service
            .list(
                user_id,
                MovieFilter {
                    limit: Some(5),
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        assert_eq!(movies.len(), 5);
    }

    #[tokio::test]
    async fn test_list_movies_with_search() {
        let service = setup().await;
        let user_id = fixtures::test_user_id();

        service
            .create(
                user_id,
                CreateMovie {
                    barcode: None,
                    tmdb_id: Some(1),
                    title: "The Matrix".to_string(),
                    original_title: None,
                    disc_type: None,
                    production_year: None,
                },
            )
            .await
            .unwrap();

        service
            .create(
                user_id,
                CreateMovie {
                    barcode: None,
                    tmdb_id: Some(2),
                    title: "Inception".to_string(),
                    original_title: None,
                    disc_type: None,
                    production_year: None,
                },
            )
            .await
            .unwrap();

        let movies = service
            .list(
                user_id,
                MovieFilter {
                    search: Some("Matrix".to_string()),
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        assert_eq!(movies.len(), 1);
        assert_eq!(movies[0].title, "The Matrix");
    }

    #[tokio::test]
    async fn test_list_movies_with_disc_type_filter() {
        let service = setup().await;
        let user_id = fixtures::test_user_id();

        service
            .create(
                user_id,
                CreateMovie {
                    barcode: None,
                    tmdb_id: Some(1),
                    title: "DVD Movie".to_string(),
                    original_title: None,
                    disc_type: Some("DVD".to_string()),
                    production_year: None,
                },
            )
            .await
            .unwrap();

        service
            .create(
                user_id,
                CreateMovie {
                    barcode: None,
                    tmdb_id: Some(2),
                    title: "Blu-ray Movie".to_string(),
                    original_title: None,
                    disc_type: Some("Blu-ray".to_string()),
                    production_year: None,
                },
            )
            .await
            .unwrap();

        let movies = service
            .list(
                user_id,
                MovieFilter {
                    disc_type: Some("DVD".to_string()),
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        assert_eq!(movies.len(), 1);
        assert_eq!(movies[0].title, "DVD Movie");
    }

    #[tokio::test]
    async fn test_update_movie() {
        let service = setup().await;
        let user_id = fixtures::test_user_id();

        let movie = service
            .create(
                user_id,
                CreateMovie {
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

        let updated = service
            .update(
                user_id,
                movie.id,
                UpdateMovie {
                    title: Some("Fight Club (Updated)".to_string()),
                    personal_rating: Some(9.0),
                    watched: Some(true),
                    ..Default::default()
                },
            )
            .await
            .unwrap();

        assert_eq!(updated.title, "Fight Club (Updated)");
        assert_eq!(updated.personal_rating, Some(9.0));
        assert!(updated.watched);
    }

    #[tokio::test]
    async fn test_delete_movie() {
        let service = setup().await;
        let user_id = fixtures::test_user_id();

        let movie = service
            .create(
                user_id,
                CreateMovie {
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

        service.delete(user_id, movie.id).await.unwrap();

        let result = service.get_by_id(user_id, movie.id).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_delete_all_movies() {
        let service = setup().await;
        let user_id = fixtures::test_user_id();

        for i in 1..=5i64 {
            service
                .create(
                    user_id,
                    CreateMovie {
                        barcode: None,
                        tmdb_id: Some(i),
                        title: format!("Movie {}", i),
                        original_title: None,
                        disc_type: None,
                        production_year: None,
                    },
                )
                .await
                .unwrap();
        }

        let deleted = service.delete_all(user_id).await.unwrap();
        assert_eq!(deleted, 5);

        let movies = service.list(user_id, MovieFilter::default()).await.unwrap();
        assert!(movies.is_empty());
    }

    #[tokio::test]
    async fn test_count_movies() {
        let service = setup().await;
        let user_id = fixtures::test_user_id();

        for i in 1..=10i64 {
            service
                .create(
                    user_id,
                    CreateMovie {
                        barcode: None,
                        tmdb_id: Some(i),
                        title: format!("Movie {}", i),
                        original_title: None,
                        disc_type: None,
                        production_year: None,
                    },
                )
                .await
                .unwrap();
        }

        let count = service
            .count(user_id, &MovieFilter::default())
            .await
            .unwrap();
        assert_eq!(count, 10);
    }

    #[tokio::test]
    async fn test_find_by_barcode() {
        let service = setup().await;
        let user_id = fixtures::test_user_id();

        service
            .create(
                user_id,
                CreateMovie {
                    barcode: Some("1234567890123".to_string()),
                    tmdb_id: Some(550),
                    title: "Fight Club".to_string(),
                    original_title: None,
                    disc_type: None,
                    production_year: None,
                },
            )
            .await
            .unwrap();

        let movie = service
            .find_by_barcode(user_id, "1234567890123")
            .await
            .unwrap();
        assert!(movie.is_some());
        assert_eq!(movie.unwrap().title, "Fight Club");

        let movie = service
            .find_by_barcode(user_id, "0000000000000")
            .await
            .unwrap();
        assert!(movie.is_none());
    }

    #[tokio::test]
    async fn test_update_movie_poster_data() {
        let service = setup().await;
        let user_id = fixtures::test_user_id();

        let movie = service
            .create(
                user_id,
                CreateMovie {
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

        let poster_data = vec![1, 2, 3, 4, 5]; // Fake image data
        service
            .update_movie_poster_data(user_id, movie.id, Some(poster_data.clone()))
            .await
            .unwrap();

        let retrieved_poster = service.get_poster_data(user_id, movie.id).await.unwrap();
        assert!(retrieved_poster.is_some());
        assert_eq!(retrieved_poster.unwrap(), poster_data);
    }

    #[test]
    fn test_is_placeholder_barcode() {
        assert!(is_placeholder_barcode("000000000000"));
        assert!(is_placeholder_barcode("0000000000000"));
        assert!(is_placeholder_barcode("111111111111"));
        assert!(is_placeholder_barcode("999999999999"));
        assert!(!is_placeholder_barcode("1234567890123"));
        assert!(!is_placeholder_barcode("5050582721478"));
    }

    #[test]
    fn test_same_disc_type() {
        // Same types
        assert!(same_disc_type(
            &Some("DVD".to_string()),
            &Some("DVD".to_string())
        ));
        assert!(same_disc_type(
            &Some("dvd".to_string()),
            &Some("DVD".to_string())
        ));
        assert!(same_disc_type(
            &Some("Blu-ray".to_string()),
            &Some("blu-ray".to_string())
        ));

        // Different types
        assert!(!same_disc_type(
            &Some("DVD".to_string()),
            &Some("Blu-ray".to_string())
        ));

        // None and empty
        assert!(same_disc_type(&None, &None));
        assert!(same_disc_type(&None, &Some("".to_string())));
        assert!(same_disc_type(&Some("".to_string()), &None));
        assert!(same_disc_type(&Some("  ".to_string()), &None));
    }

    #[tokio::test]
    async fn test_find_by_tmdb_id() {
        let service = setup().await;
        let user_id = fixtures::test_user_id();

        service
            .create(
                user_id,
                CreateMovie {
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

        let movie = service.find_by_tmdb_id(user_id, 550).await.unwrap();
        assert!(movie.is_some());
        assert_eq!(movie.unwrap().title, "Fight Club");

        let movie = service.find_by_tmdb_id(user_id, 999999).await.unwrap();
        assert!(movie.is_none());
    }

    #[tokio::test]
    async fn test_find_by_title() {
        let service = setup().await;
        let user_id = fixtures::test_user_id();

        service
            .create(
                user_id,
                CreateMovie {
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

        service
            .create(
                user_id,
                CreateMovie {
                    barcode: None,
                    tmdb_id: Some(551),
                    title: "Fight Club 2".to_string(),
                    original_title: None,
                    disc_type: None,
                    production_year: None,
                },
            )
            .await
            .unwrap();

        let movies = service.find_by_title(user_id, "Fight Club").await.unwrap();
        assert_eq!(movies.len(), 1);
        assert_eq!(movies[0].title, "Fight Club");

        let movies = service.find_by_title(user_id, "NonExistent").await.unwrap();
        assert!(movies.is_empty());
    }

    #[tokio::test]
    async fn test_delete_all() {
        let service = setup().await;
        let user_id = fixtures::test_user_id();

        for i in 1..=5i64 {
            service
                .create(
                    user_id,
                    CreateMovie {
                        barcode: None,
                        tmdb_id: Some(i),
                        title: format!("Movie {}", i),
                        original_title: None,
                        disc_type: None,
                        production_year: None,
                    },
                )
                .await
                .unwrap();
        }

        let count = service
            .count(user_id, &MovieFilter::default())
            .await
            .unwrap();
        assert_eq!(count, 5);

        let deleted = service.delete_all(user_id).await.unwrap();
        assert_eq!(deleted, 5);

        let count = service
            .count(user_id, &MovieFilter::default())
            .await
            .unwrap();
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn test_get_movie_ids_with_poster() {
        let service = setup().await;
        let user_id = fixtures::test_user_id();

        // Create movies without posters
        let movie1 = service
            .create(
                user_id,
                CreateMovie {
                    barcode: None,
                    tmdb_id: Some(1),
                    title: "Movie 1".to_string(),
                    original_title: None,
                    disc_type: None,
                    production_year: None,
                },
            )
            .await
            .unwrap();

        let _movie2 = service
            .create(
                user_id,
                CreateMovie {
                    barcode: None,
                    tmdb_id: Some(2),
                    title: "Movie 2".to_string(),
                    original_title: None,
                    disc_type: None,
                    production_year: None,
                },
            )
            .await
            .unwrap();

        // Add poster to movie1 only
        service
            .update_movie_poster_data(user_id, movie1.id, Some(vec![1, 2, 3]))
            .await
            .unwrap();

        let ids_with_poster = service.get_movie_ids_with_poster(user_id).await.unwrap();
        assert_eq!(ids_with_poster.len(), 1);
        assert_eq!(ids_with_poster[0], movie1.id);
    }

    #[tokio::test]
    async fn test_find_duplicates() {
        let service = setup().await;
        let user_id = fixtures::test_user_id();

        // Create one movie
        service
            .create(
                user_id,
                CreateMovie {
                    barcode: Some("1234567890123".to_string()),
                    tmdb_id: Some(550),
                    title: "Fight Club".to_string(),
                    original_title: None,
                    disc_type: Some("DVD".to_string()),
                    production_year: None,
                },
            )
            .await
            .unwrap();

        // Check for duplicates - should find the existing movie by barcode
        let duplicates = service
            .find_duplicates(user_id, "Fight Club", Some("1234567890123"), Some(550))
            .await
            .unwrap();
        assert_eq!(duplicates.len(), 1);
        assert_eq!(duplicates[0].title, "Fight Club");

        // Check with unknown barcode - should find by title
        let duplicates = service
            .find_duplicates(user_id, "Fight Club", None, None)
            .await
            .unwrap();
        assert_eq!(duplicates.len(), 1);

        // Check with non-matching data - should find nothing
        let duplicates = service
            .find_duplicates(user_id, "NonExistent", None, None)
            .await
            .unwrap();
        assert!(duplicates.is_empty());
    }

    #[tokio::test]
    async fn test_find_all_duplicates() {
        let service = setup().await;
        let user_id = fixtures::test_user_id();

        // Create duplicates for barcode A
        for _ in 0..2 {
            service
                .create(
                    user_id,
                    CreateMovie {
                        barcode: Some("1111111111111".to_string()),
                        tmdb_id: Some(1),
                        title: "Movie A".to_string(),
                        original_title: None,
                        disc_type: Some("DVD".to_string()),
                        production_year: None,
                    },
                )
                .await
                .unwrap();
        }

        // Create duplicates for barcode B
        for _ in 0..3 {
            service
                .create(
                    user_id,
                    CreateMovie {
                        barcode: Some("2222222222222".to_string()),
                        tmdb_id: Some(2),
                        title: "Movie B".to_string(),
                        original_title: None,
                        disc_type: Some("DVD".to_string()),
                        production_year: None,
                    },
                )
                .await
                .unwrap();
        }

        // Create unique movie (no duplicate)
        service
            .create(
                user_id,
                CreateMovie {
                    barcode: Some("3333333333333".to_string()),
                    tmdb_id: Some(3),
                    title: "Movie C".to_string(),
                    original_title: None,
                    disc_type: None,
                    production_year: None,
                },
            )
            .await
            .unwrap();

        let duplicate_groups = service.find_all_duplicates(user_id).await.unwrap();
        assert_eq!(duplicate_groups.len(), 2); // Two groups of duplicates
    }

    #[tokio::test]
    async fn test_list_with_filters() {
        let service = setup().await;
        let user_id = fixtures::test_user_id();

        // Create movies with different attributes
        service
            .create(
                user_id,
                CreateMovie {
                    barcode: None,
                    tmdb_id: Some(1),
                    title: "Action Movie".to_string(),
                    original_title: None,
                    disc_type: Some("DVD".to_string()),
                    production_year: Some(2020),
                },
            )
            .await
            .unwrap();

        service
            .create(
                user_id,
                CreateMovie {
                    barcode: None,
                    tmdb_id: Some(2),
                    title: "Comedy Movie".to_string(),
                    original_title: None,
                    disc_type: Some("Blu-ray".to_string()),
                    production_year: Some(2021),
                },
            )
            .await
            .unwrap();

        // Test search filter
        let movies = service
            .list(
                user_id,
                MovieFilter {
                    search: Some("Action".to_string()),
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        assert_eq!(movies.len(), 1);
        assert_eq!(movies[0].title, "Action Movie");

        // Test disc_type filter
        let movies = service
            .list(
                user_id,
                MovieFilter {
                    disc_type: Some("DVD".to_string()),
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        assert_eq!(movies.len(), 1);
        assert_eq!(movies[0].disc_type, Some("DVD".to_string()));
    }

    #[tokio::test]
    async fn test_update_movie_various_fields() {
        let service = setup().await;
        let user_id = fixtures::test_user_id();

        let movie = service
            .create(
                user_id,
                CreateMovie {
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

        // Update multiple fields
        let updated = service
            .update(
                user_id,
                movie.id,
                UpdateMovie {
                    title: Some("Fight Club (Special Edition)".to_string()),
                    watched: Some(true),
                    personal_rating: Some(9.5),
                    location: Some("Shelf A".to_string()),
                    notes: Some("Great movie!".to_string()),
                    ..Default::default()
                },
            )
            .await
            .unwrap();

        assert_eq!(updated.title, "Fight Club (Special Edition)");
        assert!(updated.watched);
        assert_eq!(updated.personal_rating, Some(9.5));
        assert_eq!(updated.location, Some("Shelf A".to_string()));
        assert_eq!(updated.notes, Some("Great movie!".to_string()));
    }
}
