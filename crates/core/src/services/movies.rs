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
        let limit = filter.limit.unwrap_or(50);
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

        query.push_str(&format!(" ORDER BY {} LIMIT ? OFFSET ?", order_clause));

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

        let rows = q.bind(limit).bind(offset).fetch_all(&self.pool).await?;
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
            if let Some(tmdb_id) = movie.tmdb_id
                && tmdb_id > 0
            {
                for other in &movies {
                    if other.id != movie.id
                        && other.tmdb_id == Some(tmdb_id)
                        && !group.iter().any(|m| m.id == other.id)
                    {
                        group.push(other.clone());
                    }
                }
            }

            // Find duplicates by exact title match
            for other in &movies {
                if other.id != movie.id
                    && other.title.to_lowercase() == movie.title.to_lowercase()
                    && !group.iter().any(|m| m.id == other.id)
                {
                    group.push(other.clone());
                }
            }

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
