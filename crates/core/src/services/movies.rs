use chrono::Utc;
use uuid::Uuid;

use crate::db::DbPool;
use crate::error::{Error, Result};
use crate::models::{CreateMovie, Movie, MovieFilter, MovieRow, UpdateMovie};

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
                disc_type, production_year, poster_path, created_at, updated_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(id.to_string())
        .bind(user_id.to_string())
        .bind(&input.barcode)
        .bind(input.tmdb_id)
        .bind(&input.title)
        .bind(&input.original_title)
        .bind(&input.disc_type)
        .bind(input.production_year)
        .bind(&input.poster_path)
        .bind(now.to_rfc3339())
        .bind(now.to_rfc3339())
        .execute(&self.pool)
        .await?;

        self.get_by_id(user_id, id).await
    }

    pub async fn get_by_id(&self, user_id: Uuid, id: Uuid) -> Result<Movie> {
        let row =
            sqlx::query_as::<_, MovieRow>("SELECT * FROM movies WHERE id = ? AND user_id = ?")
                .bind(id.to_string())
                .bind(user_id.to_string())
                .fetch_optional(&self.pool)
                .await?
                .ok_or(Error::NotFound)?;
        Ok(row.into())
    }

    pub async fn count(&self, user_id: Uuid, filter: &MovieFilter) -> Result<i64> {
        let mut query = String::from("SELECT COUNT(*) as count FROM movies WHERE user_id = ?");
        let mut params: Vec<String> = vec![user_id.to_string()];

        if let Some(ref search) = filter.search {
            query.push_str(" AND (title LIKE ? OR original_title LIKE ? OR director LIKE ?)");
            let search_pattern = format!("%{}%", search);
            params.push(search_pattern.clone());
            params.push(search_pattern.clone());
            params.push(search_pattern);
        }

        if let Some(ref genre) = filter.genre {
            query.push_str(" AND genres LIKE ?");
            params.push(format!("%{}%", genre));
        }

        if let Some(ref disc_type) = filter.disc_type {
            query.push_str(" AND disc_type = ?");
            params.push(disc_type.clone());
        }

        if let Some(watched) = filter.watched {
            query.push_str(" AND watched = ?");
            params.push(if watched {
                "1".to_string()
            } else {
                "0".to_string()
            });
        }

        if let Some(year_from) = filter.year_from {
            query.push_str(" AND production_year >= ?");
            params.push(year_from.to_string());
        }

        if let Some(year_to) = filter.year_to {
            query.push_str(" AND production_year <= ?");
            params.push(year_to.to_string());
        }

        let mut q = sqlx::query_scalar::<_, i64>(&query);
        for param in params {
            q = q.bind(param);
        }

        let count = q.fetch_one(&self.pool).await?;
        Ok(count)
    }

    pub async fn list(&self, user_id: Uuid, filter: MovieFilter) -> Result<Vec<Movie>> {
        let limit = filter.limit.unwrap_or(50);
        let offset = filter.offset.unwrap_or(0);
        let sort_by = filter.sort_by.unwrap_or_else(|| "title".to_string());
        let sort_order = filter.sort_order.unwrap_or_else(|| "asc".to_string());

        // Build dynamic query
        let mut query = String::from("SELECT * FROM movies WHERE user_id = ?");
        let mut params: Vec<String> = vec![user_id.to_string()];

        if let Some(ref search) = filter.search {
            query.push_str(" AND (title LIKE ? OR original_title LIKE ? OR director LIKE ?)");
            let search_pattern = format!("%{}%", search);
            params.push(search_pattern.clone());
            params.push(search_pattern.clone());
            params.push(search_pattern);
        }

        if let Some(ref genre) = filter.genre {
            query.push_str(" AND genres LIKE ?");
            params.push(format!("%{}%", genre));
        }

        if let Some(ref disc_type) = filter.disc_type {
            query.push_str(" AND disc_type = ?");
            params.push(disc_type.clone());
        }

        if let Some(watched) = filter.watched {
            query.push_str(" AND watched = ?");
            params.push(if watched {
                "1".to_string()
            } else {
                "0".to_string()
            });
        }

        if let Some(year_from) = filter.year_from {
            query.push_str(" AND production_year >= ?");
            params.push(year_from.to_string());
        }

        if let Some(year_to) = filter.year_to {
            query.push_str(" AND production_year <= ?");
            params.push(year_to.to_string());
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
        query.push_str(&format!(
            " ORDER BY {} {} LIMIT ? OFFSET ?",
            sort_column, order
        ));

        // Execute with dynamic params
        let mut q = sqlx::query_as::<_, MovieRow>(&query);
        for param in params {
            q = q.bind(param);
        }
        q = q.bind(limit).bind(offset);

        let rows = q.fetch_all(&self.pool).await?;
        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    pub async fn update(&self, user_id: Uuid, id: Uuid, input: UpdateMovie) -> Result<Movie> {
        // Verify ownership first
        let _ = self.get_by_id(user_id, id).await?;

        // Update each field individually if provided
        if let Some(ref title) = input.title {
            sqlx::query("UPDATE movies SET title = ? WHERE id = ? AND user_id = ?")
                .bind(title)
                .bind(id.to_string())
                .bind(user_id.to_string())
                .execute(&self.pool)
                .await?;
        }

        if let Some(ref description) = input.description {
            sqlx::query("UPDATE movies SET description = ? WHERE id = ? AND user_id = ?")
                .bind(description)
                .bind(id.to_string())
                .bind(user_id.to_string())
                .execute(&self.pool)
                .await?;
        }

        if let Some(watched) = input.watched {
            sqlx::query("UPDATE movies SET watched = ? WHERE id = ? AND user_id = ?")
                .bind(watched)
                .bind(id.to_string())
                .bind(user_id.to_string())
                .execute(&self.pool)
                .await?;
        }

        if let Some(rating) = input.personal_rating {
            sqlx::query("UPDATE movies SET personal_rating = ? WHERE id = ? AND user_id = ?")
                .bind(rating)
                .bind(id.to_string())
                .bind(user_id.to_string())
                .execute(&self.pool)
                .await?;
        }

        if let Some(ref location) = input.location {
            sqlx::query("UPDATE movies SET location = ? WHERE id = ? AND user_id = ?")
                .bind(location)
                .bind(id.to_string())
                .bind(user_id.to_string())
                .execute(&self.pool)
                .await?;
        }

        if let Some(ref notes) = input.notes {
            sqlx::query("UPDATE movies SET notes = ? WHERE id = ? AND user_id = ?")
                .bind(notes)
                .bind(id.to_string())
                .bind(user_id.to_string())
                .execute(&self.pool)
                .await?;
        }

        if let Some(ref poster_path) = input.poster_path {
            sqlx::query("UPDATE movies SET poster_path = ? WHERE id = ? AND user_id = ?")
                .bind(poster_path)
                .bind(id.to_string())
                .bind(user_id.to_string())
                .execute(&self.pool)
                .await?;
        }

        if let Some(tmdb_id) = input.tmdb_id {
            sqlx::query("UPDATE movies SET tmdb_id = ? WHERE id = ? AND user_id = ?")
                .bind(tmdb_id)
                .bind(id.to_string())
                .bind(user_id.to_string())
                .execute(&self.pool)
                .await?;
        }

        if let Some(ref imdb_id) = input.imdb_id {
            sqlx::query("UPDATE movies SET imdb_id = ? WHERE id = ? AND user_id = ?")
                .bind(imdb_id)
                .bind(id.to_string())
                .bind(user_id.to_string())
                .execute(&self.pool)
                .await?;
        }

        if let Some(ref original_title) = input.original_title {
            sqlx::query("UPDATE movies SET original_title = ? WHERE id = ? AND user_id = ?")
                .bind(original_title)
                .bind(id.to_string())
                .bind(user_id.to_string())
                .execute(&self.pool)
                .await?;
        }

        if let Some(ref tagline) = input.tagline {
            sqlx::query("UPDATE movies SET tagline = ? WHERE id = ? AND user_id = ?")
                .bind(tagline)
                .bind(id.to_string())
                .bind(user_id.to_string())
                .execute(&self.pool)
                .await?;
        }

        if let Some(running_time) = input.running_time {
            sqlx::query("UPDATE movies SET running_time = ? WHERE id = ? AND user_id = ?")
                .bind(running_time)
                .bind(id.to_string())
                .bind(user_id.to_string())
                .execute(&self.pool)
                .await?;
        }

        if let Some(ref director) = input.director {
            sqlx::query("UPDATE movies SET director = ? WHERE id = ? AND user_id = ?")
                .bind(director)
                .bind(id.to_string())
                .bind(user_id.to_string())
                .execute(&self.pool)
                .await?;
        }

        if let Some(ref actors) = input.actors {
            sqlx::query("UPDATE movies SET actors = ? WHERE id = ? AND user_id = ?")
                .bind(actors)
                .bind(id.to_string())
                .bind(user_id.to_string())
                .execute(&self.pool)
                .await?;
        }

        if let Some(ref genres) = input.genres {
            sqlx::query("UPDATE movies SET genres = ? WHERE id = ? AND user_id = ?")
                .bind(genres)
                .bind(id.to_string())
                .bind(user_id.to_string())
                .execute(&self.pool)
                .await?;
        }

        if let Some(budget) = input.budget {
            sqlx::query("UPDATE movies SET budget = ? WHERE id = ? AND user_id = ?")
                .bind(budget)
                .bind(id.to_string())
                .bind(user_id.to_string())
                .execute(&self.pool)
                .await?;
        }

        if let Some(revenue) = input.revenue {
            sqlx::query("UPDATE movies SET revenue = ? WHERE id = ? AND user_id = ?")
                .bind(revenue)
                .bind(id.to_string())
                .bind(user_id.to_string())
                .execute(&self.pool)
                .await?;
        }

        // Update timestamp
        sqlx::query("UPDATE movies SET updated_at = ? WHERE id = ? AND user_id = ?")
            .bind(chrono::Utc::now().to_rfc3339())
            .bind(id.to_string())
            .bind(user_id.to_string())
            .execute(&self.pool)
            .await?;

        self.get_by_id(user_id, id).await
    }

    pub async fn delete(&self, user_id: Uuid, id: Uuid) -> Result<()> {
        let result = sqlx::query("DELETE FROM movies WHERE id = ? AND user_id = ?")
            .bind(id.to_string())
            .bind(user_id.to_string())
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(Error::NotFound);
        }

        Ok(())
    }

    pub async fn find_by_barcode(&self, user_id: Uuid, barcode: &str) -> Result<Option<Movie>> {
        let row =
            sqlx::query_as::<_, MovieRow>("SELECT * FROM movies WHERE barcode = ? AND user_id = ?")
                .bind(barcode)
                .bind(user_id.to_string())
                .fetch_optional(&self.pool)
                .await?;

        Ok(row.map(|r| r.into()))
    }

    pub async fn find_by_tmdb_id(&self, user_id: Uuid, tmdb_id: i64) -> Result<Option<Movie>> {
        let row =
            sqlx::query_as::<_, MovieRow>("SELECT * FROM movies WHERE tmdb_id = ? AND user_id = ?")
                .bind(tmdb_id)
                .bind(user_id.to_string())
                .fetch_optional(&self.pool)
                .await?;

        Ok(row.map(|r| r.into()))
    }

    pub async fn find_by_title(&self, user_id: Uuid, title: &str) -> Result<Vec<Movie>> {
        let rows = sqlx::query_as::<_, MovieRow>(
            "SELECT * FROM movies WHERE (title = ? OR original_title = ?) AND user_id = ?",
        )
        .bind(title)
        .bind(title)
        .bind(user_id.to_string())
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
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

        // Check by TMDB ID
        if let Some(id) = tmdb_id
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

            // Find duplicates by barcode
            if let Some(ref barcode) = movie.barcode {
                if !barcode.is_empty() {
                    for other in &movies {
                        if other.id != movie.id
                            && other.barcode.as_ref() == Some(barcode)
                            && !group.iter().any(|m| m.id == other.id)
                        {
                            group.push(other.clone());
                        }
                    }
                }
            }

            // Find duplicates by TMDB ID
            if let Some(tmdb_id) = movie.tmdb_id {
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
