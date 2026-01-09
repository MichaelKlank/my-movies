use std::sync::Arc;

use axum::body::Body;
use axum::{
    Extension, Json,
    extract::{Multipart, Path, Query, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use serde_json::json;
use uuid::Uuid;

use my_movies_core::models::{Claims, CreateMovie, Movie, MovieFilter, UpdateMovie};
use my_movies_core::services::{TmdbCollectionOverview, TmdbMovie, TmdbService};
use serde::{Deserialize, Serialize};

use crate::AppState;

/// Download poster image from TMDB URL and return as bytes
async fn download_poster_image(poster_path: &str) -> Option<Vec<u8>> {
    // Build full TMDB image URL (use w500 for good quality)
    let image_url = TmdbService::poster_url(poster_path, "w500");

    // Download the image
    match reqwest::get(&image_url).await {
        Ok(response) => {
            if response.status().is_success() {
                match response.bytes().await {
                    Ok(bytes) => {
                        let data = bytes.to_vec();
                        // Validate it's actually an image (basic check)
                        if data.len() >= 8 {
                            Some(data)
                        } else {
                            tracing::warn!("Downloaded poster too small: {} bytes", data.len());
                            None
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Failed to read poster bytes: {}", e);
                        None
                    }
                }
            } else {
                tracing::warn!("Failed to download poster: HTTP {}", response.status());
                None
            }
        }
        Err(e) => {
            tracing::warn!("Failed to download poster from {}: {}", image_url, e);
            None
        }
    }
}

pub async fn list(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Query(filter): Query<MovieFilter>,
) -> impl IntoResponse {
    // Get total count first
    let total = match state.movie_service.count(claims.sub, &filter).await {
        Ok(count) => count,
        Err(e) => {
            return (
                StatusCode::from_u16(e.status_code()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
                Json(json!({ "error": e.to_string() })),
            )
                .into_response();
        }
    };

    let limit = filter.limit; // None = no limit
    let offset = filter.offset.unwrap_or(0);

    match state.movie_service.list(claims.sub, filter).await {
        Ok(movies) => (
            StatusCode::OK,
            Json(json!({
                "items": movies,
                "total": total,
                "limit": limit.unwrap_or(total as i64), // Report actual count if no limit
                "offset": offset
            })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::from_u16(e.status_code()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn get(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match state.movie_service.get_by_id(claims.sub, id).await {
        Ok(movie) => (StatusCode::OK, Json(json!(movie))).into_response(),
        Err(e) => (
            StatusCode::from_u16(e.status_code()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn create(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Json(input): Json<CreateMovie>,
) -> impl IntoResponse {
    match state.movie_service.create(claims.sub, input).await {
        Ok(movie) => {
            // Broadcast to WebSocket clients
            let msg = json!({
                "type": "movie_added",
                "payload": movie
            });
            let _ = state.ws_broadcast.send(msg.to_string());

            (StatusCode::CREATED, Json(json!(movie))).into_response()
        }
        Err(e) => (
            StatusCode::from_u16(e.status_code()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn update(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<Uuid>,
    Json(input): Json<UpdateMovie>,
) -> impl IntoResponse {
    match state.movie_service.update(claims.sub, id, input).await {
        Ok(movie) => {
            // Broadcast to WebSocket clients
            let msg = json!({
                "type": "movie_updated",
                "payload": movie
            });
            let _ = state.ws_broadcast.send(msg.to_string());

            (StatusCode::OK, Json(json!(movie))).into_response()
        }
        Err(e) => (
            StatusCode::from_u16(e.status_code()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn delete(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match state.movie_service.delete(claims.sub, id).await {
        Ok(_) => {
            // Broadcast to WebSocket clients
            let msg = json!({
                "type": "movie_deleted",
                "payload": { "id": id }
            });
            let _ = state.ws_broadcast.send(msg.to_string());

            StatusCode::NO_CONTENT.into_response()
        }
        Err(e) => (
            StatusCode::from_u16(e.status_code()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

/// Delete all movies for the current user
pub async fn delete_all(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
) -> impl IntoResponse {
    match state.movie_service.delete_all(claims.sub).await {
        Ok(count) => {
            // Broadcast to WebSocket clients
            let msg = json!({
                "type": "all_movies_deleted",
                "payload": { "count": count }
            });
            let _ = state.ws_broadcast.send(msg.to_string());

            (
                StatusCode::OK,
                Json(json!({ "deleted": count, "message": format!("{} movies deleted", count) })),
            )
                .into_response()
        }
        Err(e) => (
            StatusCode::from_u16(e.status_code()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

/// Export movie for JSON (without binary poster data)
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ExportMovie {
    pub id: String,
    pub barcode: Option<String>,
    pub tmdb_id: Option<i64>,
    pub title: String,
    pub original_title: Option<String>,
    pub sort_title: Option<String>,
    pub description: Option<String>,
    pub production_year: Option<i32>,
    pub disc_type: Option<String>,
    pub running_time: Option<i32>,
    pub genres: Option<String>,
    pub director: Option<String>,
    pub actors: Option<String>,
    pub watched: bool,
    pub location: Option<String>,
    pub rating: Option<String>,
    pub personal_rating: Option<f64>,
    pub notes: Option<String>,
    pub is_collection: bool,
    pub parent_collection_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ExportData {
    pub version: String,
    pub exported_at: String,
    pub total_movies: usize,
    pub movies: Vec<ExportMovie>,
}

/// Import result
#[derive(Debug, serde::Serialize)]
pub struct JsonImportResult {
    pub imported: usize,
    pub skipped: usize,
    pub errors: Vec<String>,
}

/// Export all movies as ZIP with JSON metadata and poster images
pub async fn export(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
) -> impl IntoResponse {
    use std::io::{Cursor, Write};
    use zip::ZipWriter;
    use zip::write::SimpleFileOptions;

    // Get ALL movies - no limit!
    let filter = MovieFilter {
        exclude_collection_children: Some(false),
        ..Default::default()
    };

    let movies = match state.movie_service.list(claims.sub, filter).await {
        Ok(m) => m,
        Err(e) => {
            return (
                StatusCode::from_u16(e.status_code()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
                Json(json!({ "error": e.to_string() })),
            )
                .into_response();
        }
    };

    // Get list of movie IDs that have poster data
    // (list() doesn't include poster_data for performance, so we need to check separately)
    let poster_ids_result = state
        .movie_service
        .get_movie_ids_with_poster(claims.sub)
        .await;

    let movies_with_poster: std::collections::HashSet<uuid::Uuid> = match &poster_ids_result {
        Ok(ids) => {
            tracing::info!("get_movie_ids_with_poster returned {} IDs", ids.len());
            ids.clone().into_iter().collect()
        }
        Err(e) => {
            tracing::error!("get_movie_ids_with_poster failed: {:?}", e);
            std::collections::HashSet::new()
        }
    };

    tracing::info!(
        "Export: {} movies total, {} have posters (HashSet size: {})",
        movies.len(),
        poster_ids_result.as_ref().map(|v| v.len()).unwrap_or(0),
        movies_with_poster.len()
    );

    // Create ZIP in memory
    let mut zip_buffer = Cursor::new(Vec::new());
    {
        let mut zip = ZipWriter::new(&mut zip_buffer);
        let options = SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated)
            .compression_level(Some(6));

        let mut export_movies: Vec<ExportMovie> = Vec::new();
        let mut posters_included = 0;

        let mut checked_count = 0;
        let mut fetch_errors = 0;

        for movie in &movies {
            let movie_id = movie.id.to_string();
            let has_poster = movies_with_poster.contains(&movie.id);

            // Add poster to ZIP if exists - use get_poster_data() to fetch only poster bytes
            if has_poster {
                checked_count += 1;
                match state
                    .movie_service
                    .get_poster_data(claims.sub, movie.id)
                    .await
                {
                    Ok(Some(poster_data)) => {
                        let poster_filename = format!("posters/{}.jpg", movie_id);
                        match zip.start_file(&poster_filename, options) {
                            Ok(_) => {
                                if zip.write_all(&poster_data).is_ok() {
                                    posters_included += 1;
                                }
                            }
                            Err(e) => {
                                tracing::warn!("Failed to start file in ZIP: {}", e);
                            }
                        }
                    }
                    Ok(None) => {
                        // Poster was expected but not found
                        fetch_errors += 1;
                        if fetch_errors <= 5 {
                            tracing::warn!("Poster not found for movie {}", movie_id);
                        }
                    }
                    Err(e) => {
                        fetch_errors += 1;
                        if fetch_errors <= 5 {
                            tracing::warn!(
                                "Failed to fetch poster for movie {}: {:?}",
                                movie_id,
                                e
                            );
                        }
                    }
                }
            }

            export_movies.push(ExportMovie {
                id: movie_id.clone(),
                barcode: movie.barcode.clone(),
                tmdb_id: movie.tmdb_id,
                title: movie.title.clone(),
                original_title: movie.original_title.clone(),
                sort_title: movie.sort_title.clone(),
                description: movie.description.clone(),
                production_year: movie.production_year,
                disc_type: movie.disc_type.clone(),
                running_time: movie.running_time,
                genres: movie.genres.clone(),
                director: movie.director.clone(),
                actors: movie.actors.clone(),
                watched: movie.watched,
                location: movie.location.clone(),
                rating: movie.rating.clone(),
                personal_rating: movie.personal_rating,
                notes: movie.notes.clone(),
                is_collection: movie.is_collection,
                parent_collection_id: movie.parent_collection_id.map(|id| id.to_string()),
                created_at: movie.created_at.to_rfc3339(),
                updated_at: movie.updated_at.to_rfc3339(),
            });
        }

        let export_data = ExportData {
            version: "1.0".to_string(),
            exported_at: chrono::Utc::now().to_rfc3339(),
            total_movies: export_movies.len(),
            movies: export_movies,
        };

        // Add movies.json to ZIP
        let json_content = serde_json::to_string_pretty(&export_data).unwrap_or_default();
        if zip.start_file("movies.json", options).is_ok() {
            let _ = zip.write_all(json_content.as_bytes());
        }

        tracing::info!(
            "Export created: {} movies, checked {} with posters, {} fetch errors, {} posters written to ZIP",
            export_data.total_movies,
            checked_count,
            fetch_errors,
            posters_included
        );

        let _ = zip.finish();
    }

    let zip_data = zip_buffer.into_inner();
    let filename = format!(
        "my-movies-backup-{}.zip",
        chrono::Utc::now().format("%Y%m%d-%H%M%S")
    );

    (
        StatusCode::OK,
        [
            (
                axum::http::header::CONTENT_TYPE,
                "application/zip".to_string(),
            ),
            (
                axum::http::header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{}\"", filename),
            ),
        ],
        zip_data,
    )
        .into_response()
}

/// Import movies from JSON export
pub async fn import_json(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Json(import_data): Json<ExportData>,
) -> impl IntoResponse {
    use std::collections::HashMap;

    let mut imported = 0;
    let mut skipped = 0;
    let mut errors: Vec<String> = Vec::new();

    // Map old IDs to new IDs (for collection relationships)
    let mut id_map: HashMap<String, Uuid> = HashMap::new();

    // First pass: Import all movies (collections first to establish parent relationships)
    // Sort so collections come first
    let mut movies_to_import = import_data.movies;
    movies_to_import.sort_by(|a, b| b.is_collection.cmp(&a.is_collection));

    for export_movie in movies_to_import {
        // Check if movie with same barcode or tmdb_id already exists
        let existing = if let Some(ref barcode) = export_movie.barcode {
            if !barcode.is_empty() && !barcode.chars().all(|c| c == '0') {
                state
                    .movie_service
                    .find_by_barcode(claims.sub, barcode)
                    .await
                    .ok()
                    .flatten()
            } else {
                None
            }
        } else {
            None
        };

        // Also check by tmdb_id
        let existing = existing.or({
            if let Some(tmdb_id) = export_movie.tmdb_id {
                if tmdb_id > 0 {
                    // We'd need an async block here, skip for now
                    None
                } else {
                    None
                }
            } else {
                None
            }
        });

        if existing.is_some() {
            skipped += 1;
            // Still map the old ID to the existing movie's ID
            if let Some(ref existing_movie) = existing {
                id_map.insert(export_movie.id.clone(), existing_movie.id);
            }
            continue;
        }

        // Resolve parent_collection_id from old ID to new ID
        let parent_collection_id = export_movie
            .parent_collection_id
            .as_ref()
            .and_then(|old_id| id_map.get(old_id).copied());

        // Create the movie
        let create_movie = CreateMovie {
            barcode: export_movie.barcode.clone(),
            tmdb_id: export_movie.tmdb_id,
            title: export_movie.title.clone(),
            original_title: export_movie.original_title.clone(),
            production_year: export_movie.production_year,
            disc_type: export_movie.disc_type.clone(),
        };

        match state.movie_service.create(claims.sub, create_movie).await {
            Ok(new_movie) => {
                // Map old ID to new ID
                id_map.insert(export_movie.id.clone(), new_movie.id);

                // Update additional fields that aren't in CreateMovie
                let update = UpdateMovie {
                    title: None,
                    original_title: None,
                    sort_title: export_movie.sort_title.clone(),
                    description: export_movie.description.clone(),
                    production_year: None,
                    disc_type: None,
                    running_time: export_movie.running_time,
                    genres: export_movie.genres.clone(),
                    director: export_movie.director.clone(),
                    actors: export_movie.actors.clone(),
                    watched: Some(export_movie.watched),
                    location: export_movie.location.clone(),
                    rating: export_movie.rating.clone(),
                    personal_rating: export_movie.personal_rating,
                    notes: export_movie.notes.clone(),
                    is_collection: Some(export_movie.is_collection),
                    parent_collection_id,
                    ..Default::default()
                };

                if let Err(e) = state
                    .movie_service
                    .update(claims.sub, new_movie.id, update)
                    .await
                {
                    errors.push(format!("Error updating '{}': {}", export_movie.title, e));
                }

                imported += 1;
            }
            Err(e) => {
                errors.push(format!("Error importing '{}': {}", export_movie.title, e));
            }
        }
    }

    // Broadcast to WebSocket clients
    let msg = json!({
        "type": "collection_imported",
        "payload": { "count": imported }
    });
    let _ = state.ws_broadcast.send(msg.to_string());

    (
        StatusCode::OK,
        Json(JsonImportResult {
            imported,
            skipped,
            errors,
        }),
    )
}

/// Import movies from ZIP backup (with poster images)
pub async fn import_zip(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    mut multipart: Multipart,
) -> impl IntoResponse {
    use std::collections::HashMap;
    use std::io::{Cursor, Read};
    use zip::ZipArchive;

    // Get the file from multipart
    while let Some(field) = multipart.next_field().await.unwrap_or(None) {
        let name = field.name().unwrap_or("").to_string();

        if name == "file" {
            let data = match field.bytes().await {
                Ok(bytes) => bytes,
                Err(e) => {
                    return (
                        StatusCode::BAD_REQUEST,
                        Json(json!({ "error": format!("Failed to read file: {}", e) })),
                    )
                        .into_response();
                }
            };

            // Open ZIP archive
            let cursor = Cursor::new(data.to_vec());
            let mut archive = match ZipArchive::new(cursor) {
                Ok(a) => a,
                Err(e) => {
                    return (
                        StatusCode::BAD_REQUEST,
                        Json(json!({ "error": format!("Invalid ZIP file: {}", e) })),
                    )
                        .into_response();
                }
            };

            // Read movies.json from ZIP
            let export_data: ExportData = {
                let mut json_file = match archive.by_name("movies.json") {
                    Ok(f) => f,
                    Err(_) => {
                        return (
                            StatusCode::BAD_REQUEST,
                            Json(json!({ "error": "ZIP file does not contain movies.json" })),
                        )
                            .into_response();
                    }
                };

                let mut json_content = String::new();
                if let Err(e) = json_file.read_to_string(&mut json_content) {
                    return (
                        StatusCode::BAD_REQUEST,
                        Json(json!({ "error": format!("Failed to read movies.json: {}", e) })),
                    )
                        .into_response();
                }

                match serde_json::from_str(&json_content) {
                    Ok(d) => d,
                    Err(e) => {
                        return (
                            StatusCode::BAD_REQUEST,
                            Json(json!({ "error": format!("Invalid movies.json: {}", e) })),
                        )
                            .into_response();
                    }
                }
            };

            // Extract poster images from ZIP into a map
            let mut posters: HashMap<String, Vec<u8>> = HashMap::new();
            for i in 0..archive.len() {
                if let Ok(mut file) = archive.by_index(i) {
                    let name = file.name().to_string();
                    if name.starts_with("posters/") && name.ends_with(".jpg") {
                        // Extract movie ID from filename (posters/uuid.jpg)
                        if let Some(id_str) = name
                            .strip_prefix("posters/")
                            .and_then(|s| s.strip_suffix(".jpg"))
                        {
                            let mut poster_data = Vec::new();
                            if file.read_to_end(&mut poster_data).is_ok() {
                                posters.insert(id_str.to_string(), poster_data);
                            }
                        }
                    }
                }
            }

            tracing::info!(
                "ZIP import: {} movies, {} posters found",
                export_data.total_movies,
                posters.len()
            );

            // Import movies
            let mut imported = 0;
            let mut skipped = 0;
            let mut errors: Vec<String> = Vec::new();
            let mut id_map: HashMap<String, Uuid> = HashMap::new();

            // Sort so collections come first
            let mut movies_to_import = export_data.movies;
            movies_to_import.sort_by(|a, b| b.is_collection.cmp(&a.is_collection));

            for export_movie in movies_to_import {
                // Check if movie with same barcode already exists
                let existing = if let Some(ref barcode) = export_movie.barcode {
                    if !barcode.is_empty() && !barcode.chars().all(|c| c == '0') {
                        state
                            .movie_service
                            .find_by_barcode(claims.sub, barcode)
                            .await
                            .ok()
                            .flatten()
                    } else {
                        None
                    }
                } else {
                    None
                };

                if existing.is_some() {
                    skipped += 1;
                    if let Some(ref existing_movie) = existing {
                        id_map.insert(export_movie.id.clone(), existing_movie.id);
                    }
                    continue;
                }

                // Resolve parent_collection_id from old ID to new ID
                let parent_collection_id = export_movie
                    .parent_collection_id
                    .as_ref()
                    .and_then(|old_id| id_map.get(old_id).copied());

                // Create the movie
                let create_movie = CreateMovie {
                    barcode: export_movie.barcode.clone(),
                    tmdb_id: export_movie.tmdb_id,
                    title: export_movie.title.clone(),
                    original_title: export_movie.original_title.clone(),
                    production_year: export_movie.production_year,
                    disc_type: export_movie.disc_type.clone(),
                };

                match state.movie_service.create(claims.sub, create_movie).await {
                    Ok(new_movie) => {
                        id_map.insert(export_movie.id.clone(), new_movie.id);

                        // Get poster data if available
                        let poster_data = posters.get(&export_movie.id).cloned();

                        // Update additional fields and poster
                        let update = UpdateMovie {
                            title: None,
                            original_title: None,
                            sort_title: export_movie.sort_title.clone(),
                            description: export_movie.description.clone(),
                            production_year: None,
                            disc_type: None,
                            running_time: export_movie.running_time,
                            genres: export_movie.genres.clone(),
                            director: export_movie.director.clone(),
                            actors: export_movie.actors.clone(),
                            watched: Some(export_movie.watched),
                            location: export_movie.location.clone(),
                            rating: export_movie.rating.clone(),
                            personal_rating: export_movie.personal_rating,
                            notes: export_movie.notes.clone(),
                            is_collection: Some(export_movie.is_collection),
                            parent_collection_id,
                            poster_data,
                            ..Default::default()
                        };

                        if let Err(e) = state
                            .movie_service
                            .update(claims.sub, new_movie.id, update)
                            .await
                        {
                            errors.push(format!("Error updating '{}': {}", export_movie.title, e));
                        }

                        imported += 1;
                    }
                    Err(e) => {
                        errors.push(format!("Error importing '{}': {}", export_movie.title, e));
                    }
                }
            }

            // Broadcast to WebSocket clients
            let msg = json!({
                "type": "collection_imported",
                "payload": { "count": imported }
            });
            let _ = state.ws_broadcast.send(msg.to_string());

            return (
                StatusCode::OK,
                Json(json!({
                    "imported": imported,
                    "skipped": skipped,
                    "posters_restored": posters.len(),
                    "errors": errors
                })),
            )
                .into_response();
        }
    }

    (
        StatusCode::BAD_REQUEST,
        Json(json!({ "error": "No file provided" })),
    )
        .into_response()
}

#[derive(Debug, serde::Deserialize)]
pub struct RefreshTmdbQuery {
    #[serde(default)]
    pub force: bool, // If true, reload all data even if already present
}

/// Result of TMDB refresh operation
pub enum TmdbRefreshResult {
    Success(Box<Movie>),
    NotFound(String),
    Error(String),
}

/// Internal function to refresh TMDB data for a movie
/// Used by both the refresh_tmdb route handler and the batch enrichment process
pub async fn refresh_movie_tmdb_internal(
    state: &Arc<AppState>,
    user_id: Uuid,
    movie: &Movie,
    language: Option<&str>,
    include_adult: bool,
    force: bool,
) -> TmdbRefreshResult {
    // Special handling for collections
    if movie.is_collection {
        return match handle_collection_refresh_internal(state, user_id, movie.id, movie, language)
            .await
        {
            Ok(m) => TmdbRefreshResult::Success(Box::new(m)),
            Err(e) => TmdbRefreshResult::Error(e),
        };
    }

    // Try to get TMDB details - first as movie, then as TV series
    let mut tmdb_details = None;
    let mut tv_details = None;
    let mut is_tv_series = false;

    if let Some(tmdb_id) = movie.tmdb_id {
        // Use existing TMDB ID - try movie first
        tmdb_details = state
            .tmdb_service
            .get_movie_details(tmdb_id, language)
            .await
            .ok();

        // If movie lookup failed, maybe it's a TV series ID
        if tmdb_details.is_none() {
            tv_details = state
                .tmdb_service
                .get_tv_details(tmdb_id, language)
                .await
                .ok();
            if tv_details.is_some() {
                is_tv_series = true;
            }
        }
    } else {
        // Clean the title by removing trademark/copyright symbols
        let clean_search_title = clean_title_for_search(&movie.title);

        // Search by title - first as movie
        let year = movie.production_year;
        match state
            .tmdb_service
            .search_movies(&clean_search_title, year, language, include_adult)
            .await
        {
            Ok(results) if !results.is_empty() => {
                tmdb_details = state
                    .tmdb_service
                    .get_movie_details(results[0].id, language)
                    .await
                    .ok();
            }
            _ => {}
        }

        // If no movie found, try as TV series
        if tmdb_details.is_none() {
            let series_name = clean_title_for_search(&extract_tv_series_name(&movie.title));

            match state.tmdb_service.search_tv(&series_name, language).await {
                Ok(results) if !results.is_empty() => {
                    tv_details = state
                        .tmdb_service
                        .get_tv_details(results[0].id, language)
                        .await
                        .ok();
                    if tv_details.is_some() {
                        is_tv_series = true;
                    }
                }
                _ => {}
            }
        }

        // If still nothing found, try with cleaned title
        if tmdb_details.is_none() && tv_details.is_none() {
            let mut search_titles: Vec<String> = Vec::new();

            // Strategy 1: Use extract_base_title_from_collection
            let clean_title = extract_base_title_from_collection(&movie.title);
            if !clean_title.is_empty() && clean_title.to_lowercase() != movie.title.to_lowercase() {
                search_titles.push(clean_title);
            }

            // Strategy 2: Extract title after possessive
            let title_clone = movie.title.clone();
            for apostrophe in &["'s ", "' ", "'s ", "' ", "ʼs ", "ʼ "] {
                if let Some(pos) = title_clone.find(apostrophe) {
                    let after = &title_clone[pos + apostrophe.len()..];
                    let cleaned = if let Some(paren_pos) = after.find(" (") {
                        after[..paren_pos].trim()
                    } else {
                        after.trim()
                    };
                    if !cleaned.is_empty() && cleaned.len() > 2 {
                        search_titles.push(cleaned.to_string());
                    }
                    break;
                }
            }

            // Strategy 3: Remove parenthetical suffix only
            if let Some(paren_pos) = movie.title.rfind(" (") {
                let without_parens = movie.title[..paren_pos].trim();
                if !without_parens.is_empty() && without_parens != movie.title {
                    search_titles.push(without_parens.to_string());
                }
            }

            for search_title in search_titles {
                // Try movie search
                if tmdb_details.is_none()
                    && let Ok(results) = state
                        .tmdb_service
                        .search_movies(&search_title, year, language, include_adult)
                        .await
                    && !results.is_empty()
                {
                    tmdb_details = state
                        .tmdb_service
                        .get_movie_details(results[0].id, language)
                        .await
                        .ok();
                    if tmdb_details.is_some() {
                        break;
                    }
                }

                // Try TV search
                if tmdb_details.is_none()
                    && tv_details.is_none()
                    && let Ok(results) = state.tmdb_service.search_tv(&search_title, language).await
                    && !results.is_empty()
                {
                    tv_details = state
                        .tmdb_service
                        .get_tv_details(results[0].id, language)
                        .await
                        .ok();
                    if tv_details.is_some() {
                        is_tv_series = true;
                        break;
                    }
                }
            }
        }
    }

    // Check if we found anything
    if tmdb_details.is_none() && tv_details.is_none() {
        return TmdbRefreshResult::NotFound(format!("No TMDB data found: {}", movie.title));
    }

    // Build update based on whether we found a movie or TV series
    let (
        tmdb_id,
        original_title,
        description,
        tagline,
        poster_path,
        genres,
        runtime,
        director,
        actors,
        imdb_id,
        budget,
        revenue,
    ) = if let Some(ref details) = tmdb_details {
        let credits = state
            .tmdb_service
            .get_movie_credits(details.id, language)
            .await
            .ok();

        let director = credits.as_ref().and_then(|c| {
            c.crew
                .iter()
                .find(|p| p.job == "Director")
                .map(|p| p.name.clone())
        });

        let actors = credits.as_ref().map(|c| {
            c.cast
                .iter()
                .take(10)
                .map(|p| p.name.clone())
                .collect::<Vec<_>>()
                .join(", ")
        });

        let genres = details.genres.as_ref().map(|g| {
            g.iter()
                .map(|genre| genre.name.clone())
                .collect::<Vec<_>>()
                .join(", ")
        });

        (
            details.id,
            details.original_title.clone(),
            details.overview.clone(),
            details.tagline.clone(),
            details.poster_path.clone(),
            genres,
            details.runtime,
            director,
            actors,
            details.imdb_id.clone(),
            details.budget,
            details.revenue,
        )
    } else if let Some(ref details) = tv_details {
        let credits = state
            .tmdb_service
            .get_tv_credits(details.id, language)
            .await
            .ok();

        let creators = details.created_by.as_ref().map(|c| {
            c.iter()
                .map(|p| p.name.clone())
                .collect::<Vec<_>>()
                .join(", ")
        });

        let actors = credits.as_ref().map(|c| {
            c.cast
                .iter()
                .take(10)
                .map(|p| p.name.clone())
                .collect::<Vec<_>>()
                .join(", ")
        });

        let genres = details.genres.as_ref().map(|g| {
            g.iter()
                .map(|genre| genre.name.clone())
                .collect::<Vec<_>>()
                .join(", ")
        });

        let runtime = details
            .episode_run_time
            .as_ref()
            .and_then(|runtimes| runtimes.first().copied());

        (
            details.id,
            details.original_name.clone(),
            details.overview.clone(),
            details.tagline.clone(),
            details.poster_path.clone(),
            genres,
            runtime,
            creators,
            actors,
            None,
            None,
            None,
        )
    } else {
        return TmdbRefreshResult::NotFound(format!("No TMDB data found: {}", movie.title));
    };

    // Download poster image if available
    let should_download_poster = force || movie.poster_data.is_none();

    let poster_data = if should_download_poster {
        if let Some(ref path) = poster_path {
            download_poster_image(path).await
        } else {
            None
        }
    } else {
        None
    };

    // Build update - only include fields that are missing or if force=true
    let mut update = my_movies_core::models::UpdateMovie {
        tmdb_id: Some(tmdb_id),
        ..Default::default()
    };

    if force || movie.imdb_id.is_none() {
        update.imdb_id = imdb_id;
    }
    if force || movie.original_title.is_none() {
        update.original_title = original_title;
    }
    if force || movie.description.is_none() {
        update.description = description;
    }
    if force || movie.tagline.is_none() {
        update.tagline = tagline;
    }
    if force || movie.running_time.is_none() {
        update.running_time = runtime;
    }
    if force || movie.director.is_none() {
        update.director = director;
    }
    if force || movie.actors.is_none() {
        update.actors = actors;
    }
    if force || movie.genres.is_none() {
        update.genres = genres;
    }
    if !is_tv_series {
        if force || movie.budget.is_none() {
            update.budget = budget;
        }
        if force || movie.revenue.is_none() {
            update.revenue = revenue;
        }
    }

    // Update the movie data
    let updated_movie = match state.movie_service.update(user_id, movie.id, update).await {
        Ok(m) => m,
        Err(e) => {
            return TmdbRefreshResult::Error(format!("Failed to update {}: {}", movie.title, e));
        }
    };

    // Update poster data if we downloaded it
    let final_movie = if let Some(data) = poster_data {
        match state
            .movie_service
            .update_movie_poster_data(user_id, movie.id, Some(data))
            .await
        {
            Ok(m) => m,
            Err(_) => updated_movie,
        }
    } else {
        updated_movie
    };

    TmdbRefreshResult::Success(Box::new(final_movie))
}

/// Internal version of handle_collection_refresh that returns a Result
/// Contains all strategies for finding collection posters
async fn handle_collection_refresh_internal(
    state: &Arc<AppState>,
    user_id: Uuid,
    collection_id: Uuid,
    collection: &Movie,
    language: Option<&str>,
) -> Result<Movie, String> {
    let lang = language.unwrap_or("de-DE");

    // Strategy 1: Try to find a TMDB collection and use its poster
    let collection_search_term = extract_collection_name(&collection.title);
    if let Ok(collections) = state
        .tmdb_service
        .search_collections(&collection_search_term, Some(lang))
        .await
        && let Some(tmdb_collection) = collections.into_iter().next()
        && let Some(ref poster_path) = tmdb_collection.poster_path
        && let Some(poster_data) = download_poster_image(poster_path).await
    {
        state
            .movie_service
            .update_movie_poster_data(user_id, collection_id, Some(poster_data))
            .await
            .map_err(|e| e.to_string())?;
        return state
            .movie_service
            .get_by_id(user_id, collection_id)
            .await
            .map_err(|e| e.to_string());
    }

    // Strategy 2: Get poster from first child movie
    let filter = MovieFilter {
        exclude_collection_children: Some(false),
        ..Default::default()
    };

    if let Ok(all_movies) = state.movie_service.list(user_id, filter).await {
        let child_movies: Vec<_> = all_movies
            .into_iter()
            .filter(|m| m.parent_collection_id == Some(collection_id))
            .collect();

        if let Some(first_child) = child_movies.first() {
            // Strategy 2a: If first child has a TMDB ID, get poster from TMDB
            if let Some(tmdb_id) = first_child.tmdb_id
                && let Ok(details) = state
                    .tmdb_service
                    .get_movie_details(tmdb_id, language)
                    .await
                && let Some(ref poster_path) = details.poster_path
                && let Some(poster_data) = download_poster_image(poster_path).await
            {
                state
                    .movie_service
                    .update_movie_poster_data(user_id, collection_id, Some(poster_data))
                    .await
                    .map_err(|e| e.to_string())?;
                return state
                    .movie_service
                    .get_by_id(user_id, collection_id)
                    .await
                    .map_err(|e| e.to_string());
            }

            // Strategy 2b: Copy existing poster from first child
            if let Ok(child_with_poster) =
                state.movie_service.get_by_id(user_id, first_child.id).await
                && let Some(poster_data) = child_with_poster.poster_data
            {
                state
                    .movie_service
                    .update_movie_poster_data(user_id, collection_id, Some(poster_data))
                    .await
                    .map_err(|e| e.to_string())?;
                return state
                    .movie_service
                    .get_by_id(user_id, collection_id)
                    .await
                    .map_err(|e| e.to_string());
            }

            // Strategy 2c: Search TMDB by first child's title
            if first_child.tmdb_id.is_none() {
                let clean_title = clean_title_for_search(&first_child.title);
                if let Ok(results) = state
                    .tmdb_service
                    .search_movies(&clean_title, first_child.production_year, language, false)
                    .await
                    && let Some(first_result) = results.into_iter().next()
                    && let Some(ref poster_path) = first_result.poster_path
                    && let Some(poster_data) = download_poster_image(poster_path).await
                {
                    state
                        .movie_service
                        .update_movie_poster_data(user_id, collection_id, Some(poster_data))
                        .await
                        .map_err(|e| e.to_string())?;
                    return state
                        .movie_service
                        .get_by_id(user_id, collection_id)
                        .await
                        .map_err(|e| e.to_string());
                }
            }
        }
    }

    // Strategy 3: Extract movie titles from the collection title itself
    let extracted_titles = extract_titles_from_collection_title(&collection.title);
    if let Some(first_title) = extracted_titles.first() {
        let clean_title = clean_title_for_search(first_title);
        if let Ok(results) = state
            .tmdb_service
            .search_movies(&clean_title, collection.production_year, language, false)
            .await
            && let Some(first_result) = results.into_iter().next()
            && let Some(ref poster_path) = first_result.poster_path
            && let Some(poster_data) = download_poster_image(poster_path).await
        {
            state
                .movie_service
                .update_movie_poster_data(user_id, collection_id, Some(poster_data))
                .await
                .map_err(|e| e.to_string())?;
            return state
                .movie_service
                .get_by_id(user_id, collection_id)
                .await
                .map_err(|e| e.to_string());
        }
    }

    // Strategy 4: Search TMDB with the base collection title
    let base_title = extract_base_title_from_collection(&collection.title);
    let clean_base = clean_title_for_search(&base_title);

    // Try movie search
    if let Ok(results) = state
        .tmdb_service
        .search_movies(&clean_base, None, language, false)
        .await
        && let Some(first_result) = results.into_iter().next()
        && let Some(ref poster_path) = first_result.poster_path
        && let Some(poster_data) = download_poster_image(poster_path).await
    {
        state
            .movie_service
            .update_movie_poster_data(user_id, collection_id, Some(poster_data))
            .await
            .map_err(|e| e.to_string())?;
        return state
            .movie_service
            .get_by_id(user_id, collection_id)
            .await
            .map_err(|e| e.to_string());
    }

    // Try TV search
    if let Ok(results) = state.tmdb_service.search_tv(&clean_base, language).await
        && !results.is_empty()
        && let Some(ref poster_path) = results[0].poster_path
        && let Some(poster_data) = download_poster_image(poster_path).await
    {
        state
            .movie_service
            .update_movie_poster_data(user_id, collection_id, Some(poster_data))
            .await
            .map_err(|e| e.to_string())?;
        return state
            .movie_service
            .get_by_id(user_id, collection_id)
            .await
            .map_err(|e| e.to_string());
    }

    Err(format!(
        "Could not find poster for collection: {}",
        collection.title
    ))
}

pub async fn refresh_tmdb(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<Uuid>,
    Query(params): Query<RefreshTmdbQuery>,
) -> impl IntoResponse {
    // Get the movie first
    let movie = match state.movie_service.get_by_id(claims.sub, id).await {
        Ok(m) => m,
        Err(e) => {
            return (
                StatusCode::from_u16(e.status_code()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
                Json(json!({ "error": e.to_string() })),
            )
                .into_response();
        }
    };

    // Get user's preferences
    let user = match state.auth_service.get_user(claims.sub).await {
        Ok(u) => u,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "Failed to get user" })),
            )
                .into_response();
        }
    };
    let language = user.language.as_deref();
    let include_adult = user.include_adult;

    // Use the internal function for the actual refresh
    match refresh_movie_tmdb_internal(
        &state,
        claims.sub,
        &movie,
        language,
        include_adult,
        params.force,
    )
    .await
    {
        TmdbRefreshResult::Success(final_movie) => {
            // Broadcast to WebSocket clients
            let msg = json!({
                "type": "movie_updated",
                "payload": final_movie
            });
            let _ = state.ws_broadcast.send(msg.to_string());
            (StatusCode::OK, Json(json!(final_movie))).into_response()
        }
        TmdbRefreshResult::NotFound(msg) => {
            (StatusCode::NOT_FOUND, Json(json!({ "error": msg }))).into_response()
        }
        TmdbRefreshResult::Error(msg) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": msg })),
        )
            .into_response(),
    }
}

#[derive(Debug, serde::Deserialize)]
pub struct CheckDuplicateQuery {
    pub title: String,
    pub barcode: Option<String>,
    pub tmdb_id: Option<i64>,
}

/// Check for duplicates before adding a movie
pub async fn check_duplicates(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Query(query): Query<CheckDuplicateQuery>,
) -> impl IntoResponse {
    match state
        .movie_service
        .find_duplicates(
            claims.sub,
            &query.title,
            query.barcode.as_deref(),
            query.tmdb_id,
        )
        .await
    {
        Ok(duplicates) => (
            StatusCode::OK,
            Json(json!({
                "has_duplicates": !duplicates.is_empty(),
                "duplicates": duplicates
            })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::from_u16(e.status_code()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

/// Find all duplicate groups in the collection
pub async fn find_all_duplicates(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
) -> impl IntoResponse {
    match state.movie_service.find_all_duplicates(claims.sub).await {
        Ok(groups) => (
            StatusCode::OK,
            Json(json!({
                "duplicate_groups": groups,
                "total_groups": groups.len()
            })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::from_u16(e.status_code()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

/// Upload a poster image for a movie
pub async fn upload_poster(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<Uuid>,
    mut multipart: Multipart,
) -> impl IntoResponse {
    // Verify movie exists and belongs to user
    if let Err(e) = state.movie_service.get_by_id(claims.sub, id).await {
        return (
            StatusCode::from_u16(e.status_code()).unwrap_or(StatusCode::NOT_FOUND),
            Json(json!({ "error": e.to_string() })),
        )
            .into_response();
    }

    // Process multipart upload
    while let Some(field) = multipart.next_field().await.unwrap_or(None) {
        let name = field.name().unwrap_or("").to_string();

        if name == "file" {
            // Get content type (extension not needed anymore since we store in DB)
            let _content_type = field.content_type().unwrap_or("image/jpeg").to_string();

            let data = match field.bytes().await {
                Ok(bytes) => bytes.to_vec(),
                Err(e) => {
                    return (
                        StatusCode::BAD_REQUEST,
                        Json(json!({ "error": format!("Failed to read file: {}", e) })),
                    )
                        .into_response();
                }
            };

            // Validate it's actually an image (basic check)
            if data.len() < 8 {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(json!({ "error": "File too small to be a valid image" })),
                )
                    .into_response();
            }

            // Validate file size (max 5MB)
            const MAX_FILE_SIZE: usize = 5 * 1024 * 1024; // 5MB
            if data.len() > MAX_FILE_SIZE {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(json!({ "error": format!("File too large. Maximum size is 5MB, got {} bytes", data.len()) })),
                )
                    .into_response();
            }

            // Store image data directly in database
            match state
                .movie_service
                .update_movie_poster_data(claims.sub, id, Some(data))
                .await
            {
                Ok(movie) => {
                    // Broadcast to WebSocket clients
                    let msg = json!({
                        "type": "movie_updated",
                        "payload": movie
                    });
                    let _ = state.ws_broadcast.send(msg.to_string());

                    return (
                        StatusCode::OK,
                        Json(json!({
                            "message": "Poster uploaded successfully",
                            "movie": movie
                        })),
                    )
                        .into_response();
                }
                Err(e) => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({ "error": format!("Failed to update movie: {}", e) })),
                    )
                        .into_response();
                }
            }
        }
    }

    (
        StatusCode::BAD_REQUEST,
        Json(json!({ "error": "No file provided" })),
    )
        .into_response()
}

/// Set poster from URL - downloads the image and stores it in the database
#[derive(Debug, serde::Deserialize)]
pub struct SetPosterUrlRequest {
    pub url: String,
}

pub async fn set_poster_from_url(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<Uuid>,
    Json(input): Json<SetPosterUrlRequest>,
) -> impl IntoResponse {
    // Verify movie exists and belongs to user
    if let Err(e) = state.movie_service.get_by_id(claims.sub, id).await {
        return (
            StatusCode::from_u16(e.status_code()).unwrap_or(StatusCode::NOT_FOUND),
            Json(json!({ "error": e.to_string() })),
        )
            .into_response();
    }

    // Download the image from URL
    let image_data = match reqwest::get(&input.url).await {
        Ok(response) => {
            if !response.status().is_success() {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(json!({ "error": format!("Failed to download image: HTTP {}", response.status()) })),
                )
                    .into_response();
            }
            match response.bytes().await {
                Ok(bytes) => bytes.to_vec(),
                Err(e) => {
                    return (
                        StatusCode::BAD_REQUEST,
                        Json(json!({ "error": format!("Failed to read image data: {}", e) })),
                    )
                        .into_response();
                }
            }
        }
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": format!("Failed to download image: {}", e) })),
            )
                .into_response();
        }
    };

    // Validate it's actually an image (basic check)
    if image_data.len() < 8 {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "Downloaded file too small to be a valid image" })),
        )
            .into_response();
    }

    // Validate file size (max 5MB)
    let max_file_size: usize = 5 * 1024 * 1024; // 5MB
    if image_data.len() > max_file_size {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": format!("Image too large. Maximum size is 5MB, got {} bytes", image_data.len()) })),
        )
            .into_response();
    }

    // Store image data directly in database
    match state
        .movie_service
        .update_movie_poster_data(claims.sub, id, Some(image_data))
        .await
    {
        Ok(movie) => {
            // Broadcast to WebSocket clients
            let msg = json!({
                "type": "movie_updated",
                "payload": movie
            });
            let _ = state.ws_broadcast.send(msg.to_string());

            (
                StatusCode::OK,
                Json(json!({
                    "message": "Poster set successfully",
                    "movie": movie
                })),
            )
                .into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": format!("Failed to update movie: {}", e) })),
        )
            .into_response(),
    }
}

/// Get poster image for a movie
pub async fn get_poster(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    // Verify movie belongs to user
    if state.movie_service.get_by_id(claims.sub, id).await.is_err() {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "Movie not found" })),
        )
            .into_response();
    }
    match state
        .movie_service
        .get_movie_poster_data(claims.sub, id)
        .await
    {
        Ok(Some(data)) => {
            // Determine content type from first few bytes (magic numbers)
            let content_type = if data.len() >= 8 {
                if data[0..8] == [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A] {
                    "image/png"
                } else if data.len() >= 3 && data[0..3] == [0xFF, 0xD8, 0xFF] {
                    "image/jpeg"
                } else if data.len() >= 6
                    && (data[0..6] == [0x47, 0x49, 0x46, 0x38, 0x39, 0x61]
                        || data[0..6] == [0x47, 0x49, 0x46, 0x38, 0x37, 0x61])
                {
                    "image/gif"
                } else if data.len() >= 12 && data[8..12] == [0x57, 0x45, 0x42, 0x50] {
                    "image/webp"
                } else {
                    "image/jpeg" // Default fallback
                }
            } else {
                "image/jpeg"
            };

            match Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, content_type)
                .body(Body::from(data))
            {
                Ok(response) => response.into_response(),
                Err(e) => {
                    tracing::error!("Failed to build response: {}", e);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({ "error": "Failed to build response" })),
                    )
                        .into_response()
                }
            }
        }
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "Poster not found" })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::from_u16(e.status_code()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

// ============ Collection Analysis Endpoints ============

#[derive(Debug, Serialize)]
pub struct CollectionAnalysisResult {
    pub is_collection: bool,
    pub confidence: f32, // 0.0 - 1.0
    pub tmdb_collection: Option<TmdbCollectionOverview>,
    pub extracted_titles: Vec<ExtractedTitle>,
    pub total_movies: usize,
}

#[derive(Debug, Serialize, Clone)]
pub struct ExtractedTitle {
    pub title: String,
    pub tmdb_match: Option<TmdbMovie>,
    pub tmdb_tv_match: Option<TmdbTvMatch>,
    pub description_excerpt: Option<String>,
    pub is_tv_series: bool,
}

#[derive(Debug, Serialize, Clone)]
pub struct TmdbTvMatch {
    pub id: i64,
    pub name: String,
    pub original_name: Option<String>,
    pub overview: Option<String>,
    pub poster_path: Option<String>,
    pub first_air_date: Option<String>,
    pub vote_average: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub struct SplitCollectionRequest {
    pub selected_movies: Vec<SelectedMovie>,
    #[allow(dead_code)]
    pub keep_original: bool,
    /// TMDB collection poster path (if available from analyze_collection)
    pub collection_poster_path: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SelectedMovie {
    pub title: String,
    pub tmdb_id: Option<i64>,
}

/// Analyze a movie to detect if it's a collection and extract individual films
pub async fn analyze_collection(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Path(movie_id): Path<Uuid>,
) -> impl IntoResponse {
    // Get the movie
    let movie = match state.movie_service.get_by_id(claims.sub, movie_id).await {
        Ok(m) => m,
        Err(e) => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({ "error": e.to_string() })),
            )
                .into_response();
        }
    };

    // Get user's language preference
    let user = state.auth_service.get_user(claims.sub).await.ok();
    let language = user
        .and_then(|u| u.language)
        .unwrap_or_else(|| "de-DE".to_string());

    let mut result = CollectionAnalysisResult {
        is_collection: false,
        confidence: 0.0,
        tmdb_collection: None,
        extracted_titles: Vec::new(),
        total_movies: 0,
    };

    // Step 1: Check if title suggests a collection and extract expected count
    let title_lower = movie.title.to_lowercase();
    let collection_keywords = [
        "collection",
        "box",
        "sammlung",
        "set",
        "filme",
        "movies",
        "anthology",
        "trilogy",
        "trilogie",
        "quadrilogy",
        "pentalogy",
        "hexalogy",
        "complete",
        "komplett",
        "edition",
        "reihe",
        "filmreihe",
        "saga",
    ];

    // Check for collection keywords OR number range patterns like "1-6", "1-5"
    let has_number_range = regex::Regex::new(r"\d+[-–]\d+")
        .map(|re| re.is_match(&title_lower))
        .unwrap_or(false);

    let has_collection_keyword = collection_keywords
        .iter()
        .any(|kw| title_lower.contains(kw))
        || has_number_range;

    // Try to extract expected movie count from title (e.g., "6-Film Collection" -> 6)
    let expected_count = extract_movie_count_from_title(&movie.title);
    tracing::debug!("Expected movie count from title: {:?}", expected_count);

    // Step 1.5: Check if original_title contains a clear movie list (semicolon-separated)
    // This has highest priority as it's an explicit list of movies
    let mut description_titles: Vec<ExtractedTitle> = Vec::new();

    if let Some(ref original_title) = movie.original_title {
        // Check if original_title contains a semicolon-separated list (likely a movie list)
        if original_title.contains("; ") || original_title.matches(';').count() >= 2 {
            let parsed_from_original = parse_titles_from_movie_title(original_title);
            tracing::debug!(
                "Parsed {} titles from original_title '{}'",
                parsed_from_original.len(),
                original_title
            );

            if parsed_from_original.len() >= 2 {
                for title in parsed_from_original {
                    let is_tv = is_likely_tv_series(&title);
                    let mut extracted = ExtractedTitle {
                        title: title.clone(),
                        tmdb_match: None,
                        tmdb_tv_match: None,
                        description_excerpt: None,
                        is_tv_series: is_tv,
                    };

                    // Search TMDB
                    if is_tv {
                        if let Ok(tv_results) =
                            state.tmdb_service.search_tv(&title, Some(&language)).await
                            && let Some(tv) = tv_results.into_iter().next()
                        {
                            extracted.tmdb_tv_match = Some(TmdbTvMatch {
                                id: tv.id,
                                name: tv.name,
                                original_name: tv.original_name,
                                overview: tv.overview,
                                poster_path: tv.poster_path,
                                first_air_date: tv.first_air_date,
                                vote_average: tv.vote_average,
                            });
                        }
                    } else if let Ok(search_results) = state
                        .tmdb_service
                        .search_movies(&title, None, Some(&language), false)
                        .await
                    {
                        extracted.tmdb_match = search_results.into_iter().next();
                    }

                    description_titles.push(extracted);
                }
            }
        }
    }

    // Step 2: If original_title didn't have a movie list, try to parse description
    // This is more reliable for box sets because TMDB collections often don't match physical box sets exactly
    if description_titles.len() < 2
        && let Some(ref description) = movie.description
    {
        let parsed_titles = parse_collection_description(description);
        tracing::debug!("Parsed {} titles from description", parsed_titles.len());

        if parsed_titles.len() >= 2 {
            // Try to find TMDB matches for each parsed title
            for parsed in parsed_titles {
                let is_tv = is_likely_tv_series(&parsed.title);
                let mut extracted = ExtractedTitle {
                    title: parsed.title.clone(),
                    tmdb_match: None,
                    tmdb_tv_match: None,
                    description_excerpt: parsed.excerpt,
                    is_tv_series: is_tv,
                };

                // Search TMDB - try TV first if it looks like a series, otherwise movie
                if is_tv {
                    if let Ok(tv_results) = state
                        .tmdb_service
                        .search_tv(&parsed.title, Some(&language))
                        .await
                        && let Some(tv) = tv_results.into_iter().next()
                    {
                        extracted.tmdb_tv_match = Some(TmdbTvMatch {
                            id: tv.id,
                            name: tv.name,
                            original_name: tv.original_name,
                            overview: tv.overview,
                            poster_path: tv.poster_path,
                            first_air_date: tv.first_air_date,
                            vote_average: tv.vote_average,
                        });
                    }
                } else if let Ok(search_results) = state
                    .tmdb_service
                    .search_movies(&parsed.title, None, Some(&language), false)
                    .await
                {
                    extracted.tmdb_match = search_results.into_iter().next();
                }

                description_titles.push(extracted);
            }
        }
    }

    // Step 2b: If description parsing didn't work, try parsing the title itself
    // Handles patterns like "Triple Feature: Divergent, Insurgent, Allegiant"
    if description_titles.len() < 2 {
        let mut title_parsed_titles: Vec<ExtractedTitle> = Vec::new();

        // Try both title and original_title
        for title_to_parse in [Some(&movie.title), movie.original_title.as_ref()]
            .into_iter()
            .flatten()
        {
            let parsed_from_title = parse_titles_from_movie_title(title_to_parse);
            tracing::debug!(
                "Parsed {} titles from movie title '{}'",
                parsed_from_title.len(),
                title_to_parse
            );

            if parsed_from_title.len() >= 2 && parsed_from_title.len() > title_parsed_titles.len() {
                title_parsed_titles.clear();
                for title in parsed_from_title {
                    let is_tv = is_likely_tv_series(&title);
                    let mut extracted = ExtractedTitle {
                        title: title.clone(),
                        tmdb_match: None,
                        tmdb_tv_match: None,
                        description_excerpt: None,
                        is_tv_series: is_tv,
                    };

                    // Search TMDB - try TV first if it looks like a series
                    if is_tv {
                        if let Ok(tv_results) =
                            state.tmdb_service.search_tv(&title, Some(&language)).await
                            && let Some(tv) = tv_results.into_iter().next()
                        {
                            extracted.tmdb_tv_match = Some(TmdbTvMatch {
                                id: tv.id,
                                name: tv.name,
                                original_name: tv.original_name,
                                overview: tv.overview,
                                poster_path: tv.poster_path,
                                first_air_date: tv.first_air_date,
                                vote_average: tv.vote_average,
                            });
                        }
                    } else if let Ok(search_results) = state
                        .tmdb_service
                        .search_movies(&title, None, Some(&language), false)
                        .await
                    {
                        extracted.tmdb_match = search_results.into_iter().next();
                    }

                    title_parsed_titles.push(extracted);
                }
            }
        }

        if title_parsed_titles.len() >= 2 {
            description_titles = title_parsed_titles;
        }
    }

    // Step 3: Try TMDB collection API
    // For large franchises like James Bond, TMDB may have multiple collections
    // (e.g., classic Bond + Daniel Craig Bond), so we combine all matching collections
    let mut tmdb_titles: Vec<ExtractedTitle> = Vec::new();
    let mut tmdb_collection_info: Option<TmdbCollectionOverview> = None;
    let mut seen_tmdb_ids: std::collections::HashSet<i64> = std::collections::HashSet::new();

    let collection_search_term = extract_collection_name(&movie.title);
    tracing::debug!(
        "Step 3: Searching TMDB collections with term: '{}'",
        collection_search_term
    );

    if let Ok(collections) = state
        .tmdb_service
        .search_collections(&collection_search_term, Some(&language))
        .await
    {
        tracing::debug!(
            "Found {} TMDB collections for '{}'",
            collections.len(),
            collection_search_term
        );

        // Process all matching collections (up to 5 to avoid too many API calls)
        for (idx, collection) in collections.into_iter().take(5).enumerate() {
            tracing::debug!(
                "Processing TMDB collection {}: '{}' (id: {})",
                idx + 1,
                collection.name,
                collection.id
            );

            if let Ok(details) = state
                .tmdb_service
                .get_collection_details(collection.id, Some(&language))
                .await
            {
                tracing::debug!(
                    "TMDB collection '{}' has {} movies",
                    collection.name,
                    details.parts.len()
                );

                // Use the first collection as the main info
                if tmdb_collection_info.is_none() {
                    tmdb_collection_info = Some(collection);
                }

                // Add movies from this collection, avoiding duplicates
                for part in details.parts {
                    if !seen_tmdb_ids.contains(&part.id) {
                        seen_tmdb_ids.insert(part.id);
                        tmdb_titles.push(ExtractedTitle {
                            title: part.title.clone(),
                            tmdb_match: Some(part),
                            tmdb_tv_match: None,
                            description_excerpt: None,
                            is_tv_series: false,
                        });
                    }
                }

                // If we've reached or exceeded expected count, stop fetching more collections
                if let Some(expected) = expected_count
                    && tmdb_titles.len() >= expected
                {
                    tracing::debug!(
                        "Reached expected count {} with {} movies, stopping collection fetch",
                        expected,
                        tmdb_titles.len()
                    );
                    break;
                }
            }
        }

        tracing::debug!(
            "Total movies from all TMDB collections: {}",
            tmdb_titles.len()
        );
    }

    // Step 4: Decide which source to use
    // Prefer description parsing if:
    // - It found more movies than TMDB
    // - Or it matches the expected count better
    // - Or TMDB found nothing
    tracing::debug!(
        "Step 4: description_titles={}, tmdb_titles={}, expected_count={:?}",
        description_titles.len(),
        tmdb_titles.len(),
        expected_count
    );

    let use_description = if description_titles.is_empty() {
        false
    } else if tmdb_titles.is_empty() {
        true
    } else if let Some(expected) = expected_count {
        // If we know expected count, prefer the source that's closer
        let desc_diff = (description_titles.len() as i32 - expected as i32).abs();
        let tmdb_diff = (tmdb_titles.len() as i32 - expected as i32).abs();
        desc_diff <= tmdb_diff
    } else {
        // Default: prefer more results
        description_titles.len() >= tmdb_titles.len()
    };

    tracing::debug!("Step 4 decision: use_description={}", use_description);

    if use_description && !description_titles.is_empty() {
        result.is_collection = true;
        result.confidence = if has_collection_keyword { 0.85 } else { 0.7 };
        result.extracted_titles = description_titles;
        result.total_movies = result.extracted_titles.len();
        // Still include TMDB collection info if available
        result.tmdb_collection = tmdb_collection_info;
    } else if !tmdb_titles.is_empty() {
        result.is_collection = true;
        result.confidence = 0.9;
        result.tmdb_collection = tmdb_collection_info;
        result.extracted_titles = tmdb_titles;
        result.total_movies = result.extracted_titles.len();
    }

    // Step 5: Supplement with TMDB movie search
    // Run this if:
    // - We found no movies at all, OR
    // - We have an expected count and haven't reached it yet
    let needs_movie_search = result.extracted_titles.is_empty()
        || (expected_count.is_some() && result.extracted_titles.len() < expected_count.unwrap());

    tracing::debug!(
        "Step 5: extracted_titles={}, expected_count={:?}, needs_movie_search={}",
        result.extracted_titles.len(),
        expected_count,
        needs_movie_search
    );

    if needs_movie_search && has_collection_keyword {
        let base_title = extract_base_title_from_collection(&movie.title);
        tracing::debug!(
            "Step 5: Searching TMDB movies with base title: '{}'",
            base_title
        );

        if !base_title.is_empty() {
            // Collect existing TMDB IDs to avoid duplicates
            let existing_ids: std::collections::HashSet<i64> = result
                .extracted_titles
                .iter()
                .filter_map(|t| t.tmdb_match.as_ref().map(|m| m.id))
                .collect();

            // Search TMDB for movies with this base title (fetch up to 2 pages = 40 results)
            if let Ok(search_results) = state
                .tmdb_service
                .search_movies_paginated(&base_title, None, Some(&language), false, 2)
                .await
            {
                // Filter to movies that likely belong to the same franchise
                let base_lower = base_title.to_lowercase();
                let potential_matches: Vec<_> = search_results
                    .into_iter()
                    .filter(|m| {
                        // Skip if already in results
                        if existing_ids.contains(&m.id) {
                            return false;
                        }
                        let title_lower = m.title.to_lowercase();
                        title_lower.contains(&base_lower)
                            || base_lower
                                .contains(title_lower.split_whitespace().next().unwrap_or(""))
                    })
                    .take(30)
                    .collect();

                tracing::debug!(
                    "Step 5: Found {} additional movies matching '{}'",
                    potential_matches.len(),
                    base_title
                );

                if !potential_matches.is_empty() {
                    result.is_collection = true;
                    // Lower confidence if this is the only source
                    if result.extracted_titles.is_empty() {
                        result.confidence = 0.5;
                    }

                    for movie_match in potential_matches {
                        result.extracted_titles.push(ExtractedTitle {
                            title: movie_match.title.clone(),
                            tmdb_match: Some(movie_match),
                            tmdb_tv_match: None,
                            description_excerpt: None,
                            is_tv_series: false,
                        });
                    }
                    result.total_movies = result.extracted_titles.len();
                }
            }
        }
    }

    // If we found collection keywords but no movies, still mark as potential collection
    if has_collection_keyword && result.extracted_titles.is_empty() {
        result.is_collection = true;
        result.confidence = 0.3;
    }

    // Special case: If title looks like a TV series and we haven't found collection parts,
    // try to find it as a single TV series on TMDB
    if result.extracted_titles.is_empty() && is_likely_tv_series(&movie.title) {
        // Extract the series name (remove "Season X" etc.)
        let series_name = extract_tv_series_name(&movie.title);
        tracing::debug!("Searching TMDB TV for: {}", series_name);

        if let Ok(tv_results) = state
            .tmdb_service
            .search_tv(&series_name, Some(&language))
            .await
            && let Some(tv) = tv_results.into_iter().next()
        {
            result.is_collection = false; // It's a single TV series, not a collection
            result.confidence = 0.8;
            result.total_movies = 1;
            result.extracted_titles.push(ExtractedTitle {
                title: tv.name.clone(),
                tmdb_match: None,
                tmdb_tv_match: Some(TmdbTvMatch {
                    id: tv.id,
                    name: tv.name,
                    original_name: tv.original_name,
                    overview: tv.overview,
                    poster_path: tv.poster_path,
                    first_air_date: tv.first_air_date,
                    vote_average: tv.vote_average,
                }),
                description_excerpt: None,
                is_tv_series: true,
            });
        }
    }

    // Final fallback: If we still haven't found anything, try extracting a clean title
    // and searching TMDB. This handles cases like "Sarah Waters' Fingersmith (Doppel-DVD)"
    // where the actual title is hidden behind author names and format indicators.
    if result.extracted_titles.is_empty() {
        // Try multiple title extraction strategies
        let mut search_titles: Vec<String> = Vec::new();

        // Strategy 1: Use extract_base_title_from_collection
        let clean_title = extract_base_title_from_collection(&movie.title);
        if !clean_title.is_empty() && clean_title.to_lowercase() != movie.title.to_lowercase() {
            search_titles.push(clean_title);
        }

        // Strategy 2: Extract title after possessive (any apostrophe-like character)
        // "Sarah Waters' Fingersmith (Doppel-DVD)" -> "Fingersmith"
        let title_clone = movie.title.clone();
        for apostrophe in &["'s ", "' ", "'s ", "' ", "ʼs ", "ʼ "] {
            if let Some(pos) = title_clone.find(apostrophe) {
                let after = &title_clone[pos + apostrophe.len()..];
                // Remove parenthetical suffix
                let cleaned = if let Some(paren_pos) = after.find(" (") {
                    after[..paren_pos].trim()
                } else {
                    after.trim()
                };
                if !cleaned.is_empty() && cleaned.len() > 2 {
                    search_titles.push(cleaned.to_string());
                }
                break;
            }
        }

        // Strategy 3: Remove parenthetical suffix only
        // "Movie Title (Doppel-DVD)" -> "Movie Title"
        if let Some(paren_pos) = movie.title.rfind(" (") {
            let without_parens = movie.title[..paren_pos].trim();
            if !without_parens.is_empty() && without_parens != movie.title {
                search_titles.push(without_parens.to_string());
            }
        }

        tracing::debug!(
            "Final fallback: trying search titles {:?} from '{}'",
            search_titles,
            movie.title
        );

        // Try each extracted title until we find something
        for search_title in search_titles {
            if result.extracted_titles.is_empty() {
                // Try TV series search first (for miniseries like Fingersmith)
                if let Ok(tv_results) = state
                    .tmdb_service
                    .search_tv(&search_title, Some(&language))
                    .await
                    && let Some(tv) = tv_results.into_iter().next()
                {
                    result.is_collection = false;
                    result.confidence = 0.7;
                    result.total_movies = 1;
                    result.extracted_titles.push(ExtractedTitle {
                        title: tv.name.clone(),
                        tmdb_match: None,
                        tmdb_tv_match: Some(TmdbTvMatch {
                            id: tv.id,
                            name: tv.name,
                            original_name: tv.original_name,
                            overview: tv.overview,
                            poster_path: tv.poster_path,
                            first_air_date: tv.first_air_date,
                            vote_average: tv.vote_average,
                        }),
                        description_excerpt: None,
                        is_tv_series: true,
                    });
                    break;
                }
            }

            // If no TV series found, try movie search
            if result.extracted_titles.is_empty()
                && let Ok(movie_results) = state
                    .tmdb_service
                    .search_movies(&search_title, None, Some(&language), false)
                    .await
                && let Some(movie_match) = movie_results.into_iter().next()
            {
                result.is_collection = false;
                result.confidence = 0.7;
                result.total_movies = 1;
                result.extracted_titles.push(ExtractedTitle {
                    title: movie_match.title.clone(),
                    tmdb_match: Some(movie_match),
                    tmdb_tv_match: None,
                    description_excerpt: None,
                    is_tv_series: false,
                });
                break;
            }
        }
    }

    (StatusCode::OK, Json(result)).into_response()
}

/// Split a collection into individual movie entries
pub async fn split_collection(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Path(movie_id): Path<Uuid>,
    Json(request): Json<SplitCollectionRequest>,
) -> impl IntoResponse {
    // Get the original movie
    let original = match state.movie_service.get_by_id(claims.sub, movie_id).await {
        Ok(m) => m,
        Err(e) => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({ "error": e.to_string() })),
            )
                .into_response();
        }
    };

    // Get user's language preference for TMDB
    let user = state.auth_service.get_user(claims.sub).await.ok();
    let language = user
        .as_ref()
        .and_then(|u| u.language.clone())
        .unwrap_or_else(|| "de-DE".to_string());
    let include_adult = user.map(|u| u.include_adult).unwrap_or(false);

    // Mark original as collection
    if let Err(e) = state
        .movie_service
        .update(
            claims.sub,
            movie_id,
            UpdateMovie {
                is_collection: Some(true),
                ..Default::default()
            },
        )
        .await
    {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": format!("Failed to mark as collection: {}", e) })),
        )
            .into_response();
    }

    // If collection doesn't have a poster, try to get one
    let collection_needs_poster = original.poster_data.is_none();
    let mut first_movie_poster_path: Option<String> = None;

    // If we have a TMDB collection poster, download it for the collection
    if collection_needs_poster && let Some(ref poster_path) = request.collection_poster_path {
        tracing::debug!("Downloading collection poster from TMDB: {}", poster_path);
        if let Some(poster_data) = download_poster_image(poster_path).await {
            let _ = state
                .movie_service
                .update_movie_poster_data(claims.sub, movie_id, Some(poster_data))
                .await;
        }
    }

    let mut created_movies = Vec::new();
    let mut errors = Vec::new();

    // Create individual movie entries
    for selected in request.selected_movies {
        let selected_title = selected.title.clone(); // Clone for error message

        // Try to get TMDB details if we have an ID
        let tmdb_details = if let Some(tmdb_id) = selected.tmdb_id {
            state
                .tmdb_service
                .get_movie_details(tmdb_id, Some(&language))
                .await
                .ok()
        } else {
            // Try to search for the movie
            if let Ok(results) = state
                .tmdb_service
                .search_movies(&selected.title, None, Some(&language), include_adult)
                .await
            {
                if let Some(first) = results.into_iter().next() {
                    state
                        .tmdb_service
                        .get_movie_details(first.id, Some(&language))
                        .await
                        .ok()
                } else {
                    None
                }
            } else {
                None
            }
        };

        // Create the movie entry
        let create_input = CreateMovie {
            barcode: None, // Don't copy barcode to individual movies
            tmdb_id: tmdb_details.as_ref().map(|d| d.id),
            title: tmdb_details
                .as_ref()
                .map(|d| d.title.clone())
                .unwrap_or(selected.title),
            original_title: tmdb_details.as_ref().and_then(|d| d.original_title.clone()),
            disc_type: original.disc_type.clone(),
            production_year: tmdb_details.as_ref().and_then(|d| {
                d.release_date
                    .as_ref()
                    .and_then(|rd| rd.split('-').next().and_then(|y| y.parse().ok()))
            }),
        };

        match state.movie_service.create(claims.sub, create_input).await {
            Ok(new_movie) => {
                // Update with more details and link to parent collection
                let mut update = UpdateMovie {
                    parent_collection_id: Some(movie_id),
                    location: original.location.clone(),
                    ..Default::default()
                };

                if let Some(ref details) = tmdb_details {
                    update.description = details.overview.clone();
                    update.tagline = details.tagline.clone();
                    update.imdb_id = details.imdb_id.clone();
                    update.running_time = details.runtime;
                    update.budget = details.budget;
                    update.revenue = details.revenue;

                    if let Some(ref genres) = details.genres {
                        update.genres = Some(
                            genres
                                .iter()
                                .map(|g| g.name.clone())
                                .collect::<Vec<_>>()
                                .join(", "),
                        );
                    }

                    // Download poster
                    if let Some(ref poster_path) = details.poster_path {
                        // Store first movie's poster path as fallback for collection
                        if first_movie_poster_path.is_none() {
                            first_movie_poster_path = Some(poster_path.clone());
                        }

                        if let Some(poster_data) = download_poster_image(poster_path).await {
                            let _ = state
                                .movie_service
                                .update_movie_poster_data(
                                    claims.sub,
                                    new_movie.id,
                                    Some(poster_data),
                                )
                                .await;
                        }
                    }
                }

                // Get credits for director/actors
                if let Some(tmdb_id) = tmdb_details.as_ref().map(|d| d.id)
                    && let Ok(credits) = state
                        .tmdb_service
                        .get_movie_credits(tmdb_id, Some(&language))
                        .await
                {
                    // Get director(s)
                    let directors: Vec<_> = credits
                        .crew
                        .iter()
                        .filter(|c| c.job == "Director")
                        .map(|c| c.name.clone())
                        .collect();
                    if !directors.is_empty() {
                        update.director = Some(directors.join(", "));
                    }

                    // Get top actors
                    let actors: Vec<_> = credits
                        .cast
                        .iter()
                        .take(10)
                        .map(|c| c.name.clone())
                        .collect();
                    if !actors.is_empty() {
                        update.actors = Some(actors.join(", "));
                    }
                }

                let _ = state
                    .movie_service
                    .update(claims.sub, new_movie.id, update)
                    .await;
                created_movies.push(new_movie.id.to_string());
            }
            Err(e) => {
                errors.push(format!("Failed to create '{}': {}", selected_title, e));
            }
        }
    }

    // Fallback: If collection still has no poster and we have a first movie poster, use it
    if collection_needs_poster
        && request.collection_poster_path.is_none()
        && let Some(ref poster_path) = first_movie_poster_path
    {
        tracing::debug!("Using first movie poster for collection: {}", poster_path);
        if let Some(poster_data) = download_poster_image(poster_path).await {
            let _ = state
                .movie_service
                .update_movie_poster_data(claims.sub, movie_id, Some(poster_data))
                .await;
        }
    }

    // Broadcast update
    let _ = state.ws_broadcast.send(
        serde_json::to_string(&json!({
            "type": "collection_split",
            "payload": {
                "collection_id": movie_id.to_string(),
                "created_count": created_movies.len(),
            }
        }))
        .unwrap_or_default(),
    );

    (
        StatusCode::OK,
        Json(json!({
            "message": "Collection split successfully",
            "created_movies": created_movies,
            "errors": errors,
        })),
    )
        .into_response()
}

/// Get all movies that belong to a collection
pub async fn get_collection_movies(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Path(collection_id): Path<Uuid>,
) -> impl IntoResponse {
    let filter = MovieFilter {
        exclude_collection_children: Some(false), // Include children
        ..Default::default()
    };

    match state.movie_service.list(claims.sub, filter).await {
        Ok(movies) => {
            let collection_movies: Vec<_> = movies
                .into_iter()
                .filter(|m| m.parent_collection_id == Some(collection_id))
                .collect();

            (StatusCode::OK, Json(collection_movies)).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

// ============ Helper Functions ============

/// Extract individual movie titles from a collection title
/// e.g., "Angel Has Fallen / London Has Fallen / Olympus Has Fallen" -> ["Angel Has Fallen", "London Has Fallen", "Olympus Has Fallen"]
/// e.g., "Die Bourne Identität + Die Bourne Verschwörung" -> ["Die Bourne Identität", "Die Bourne Verschwörung"]
/// e.g., "Die Bestimmung - Triple Feature: Divergent, Insurgent, Allegiant" -> ["Divergent", "Insurgent", "Allegiant"]
fn extract_titles_from_collection_title(title: &str) -> Vec<String> {
    // Pattern 1: "Collection Name: Movie1, Movie2, Movie3" (comma-separated after colon)
    if let Some(colon_pos) = title.rfind(':') {
        let after_colon = title[colon_pos + 1..].trim();
        // Check if it looks like a comma-separated list (at least one comma)
        if after_colon.contains(',') {
            let parts: Vec<String> = after_colon
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty() && s.len() > 2) // Filter out very short strings
                .collect();

            if parts.len() >= 2 {
                return parts;
            }
        }
    }

    // Pattern 2: Try different separators on the full title
    let separators = [" / ", " + ", " & ", "; "];

    for sep in separators {
        if title.contains(sep) {
            let parts: Vec<String> = title
                .split(sep)
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();

            if parts.len() >= 2 {
                return parts;
            }
        }
    }

    // No separator found, return empty
    vec![]
}

/// Clean a title for TMDB search by removing trademark/copyright symbols
/// e.g., "The Dark Knight Rises™" -> "The Dark Knight Rises"
/// e.g., "Disney® Frozen" -> "Disney Frozen"
fn clean_title_for_search(title: &str) -> String {
    title
        .replace(['™', '®', '©'], "")
        .replace("(TM)", "")
        .replace("(R)", "")
        .replace("(C)", "")
        .trim()
        .to_string()
}

/// Try to extract expected movie count from title
/// e.g., "Alien 6-Film Collection" -> Some(6)
/// e.g., "Star Wars Complete Saga (9 Filme)" -> Some(9)
/// e.g., "Resident Evil 1-6" -> Some(6)
fn extract_movie_count_from_title(title: &str) -> Option<usize> {
    let title_lower = title.to_lowercase();

    // First, check for number range pattern: "1-6" means 6 movies
    if let Ok(re) = regex::Regex::new(r"(\d+)[-–](\d+)")
        && let Some(cap) = re.captures(&title_lower)
        && let Some(m) = cap.get(2)
        && let Ok(count) = m.as_str().parse::<usize>()
    {
        return Some(count);
    }

    // Pattern: "N-Film", "N Filme", "N Movies", "N-Movie", "(N Filme)"
    let patterns = [
        r"(\d+)\s*-?\s*film",
        r"(\d+)\s*filme",
        r"(\d+)\s*movies",
        r"\((\d+)\s*filme?\)",
    ];

    for pattern in patterns {
        if let Ok(re) = regex::Regex::new(pattern)
            && let Some(cap) = re.captures(&title_lower)
            && let Some(m) = cap.get(1)
            && let Ok(count) = m.as_str().parse::<usize>()
        {
            return Some(count);
        }
    }
    None
}

/// Extract a potential collection name from a movie title
fn extract_collection_name(title: &str) -> String {
    let mut name = title.to_string();

    // Remove format indicators in parentheses: (Doppel-DVD), (2-DVD), (Blu-ray), etc.
    if let Ok(re) = regex::Regex::new(r"(?i)\s*\([^)]*(?:dvd|blu-?ray|disc|disk|cd)[^)]*\)\s*$") {
        name = re.replace(&name, "").to_string();
    }

    // Handle possessive author prefixes: "Author's Title" or "Author' Title" -> "Title"
    // Supports: ' (U+0027), ' (U+2019 right single quote), ʼ (U+02BC), ′ (U+2032)
    if let Ok(re) = regex::Regex::new(
        r"^[A-ZÄÖÜ][a-zäöüß]+(?:\s+[A-ZÄÖÜ][a-zäöüß]+)*['\u{2019}\u{02BC}\u{2032}]s?\s+",
    ) {
        name = re.replace(&name, "").to_string();
    }

    // First, handle colon-separated titles (e.g., "Fast & Furious: 8-Movie-Collection")
    if let Some(colon_pos) = name.find(':') {
        let before_colon = name[..colon_pos].trim();
        let after_colon = name[colon_pos + 1..].trim().to_lowercase();

        // If what's after the colon contains collection indicators, use what's before
        let collection_indicators = [
            "movie",
            "film",
            "collection",
            "box",
            "set",
            "edition",
            "saga",
            "complete",
            "trilogy",
            "trilogie",
        ];
        if collection_indicators
            .iter()
            .any(|ind| after_colon.contains(ind))
        {
            return before_colon.to_string();
        }
    }

    // Remove common suffixes ONLY at the END of the string
    // Order matters - check longer patterns first
    let suffixes = [
        " - complete collection",
        " complete collection",
        " - complete trilogy",
        " complete trilogy",
        " - complete saga",
        " complete saga",
        " - complete",
        " complete",
        " collection",
        " anthology",
        " box set",
        " box",
        " sammlung",
        " set",
        " komplett",
        " trilogy",
        " trilogie",
        " saga",
    ];

    name = name.to_lowercase();

    // Keep removing suffixes until none match (handles "complete trilogy" -> removes both)
    let mut changed = true;
    while changed {
        changed = false;
        for suffix in &suffixes {
            if name.ends_with(suffix) {
                name = name[..name.len() - suffix.len()].to_string();
                changed = true;
                break; // Start over with the new shorter string
            }
        }
    }

    // Also try to extract franchise name (e.g., "Alien 6-Film Collection" -> "Alien")
    // Look for patterns like "N-Film", "N Filme", "N Movies", or number ranges like "1-6"
    let patterns = [
        r"\s+\d+[-–]\d+\s*$", // Number range at end: "1-6", "1-5" (with en-dash too)
        r"\d+[-\s]*movie[-\s]*",
        r"\d+[-\s]*film[-\s]*",
        r"\d+[-\s]*filme[-\s]*",
        r"\d+[-\s]*movies[-\s]*",
    ];

    for pattern in patterns {
        if let Ok(re) = regex::Regex::new(pattern) {
            name = re.replace_all(&name, "").to_string();
        }
    }

    name.trim().to_string()
}

/// Extract the base franchise title from a collection name
/// e.g., "The Expendables Trilogy" -> "Expendables"
/// e.g., "Alien 6-Film Collection" -> "Alien"
/// e.g., "Fast & Furious: 8-Movie-Collection" -> "Fast & Furious"
/// e.g., "The Complete Matrix Trilogy" -> "Matrix"
/// e.g., "Sarah Waters' Fingersmith (Doppel-DVD)" -> "Fingersmith"
pub fn extract_base_title_from_collection(title: &str) -> String {
    let mut name = title.to_string();

    // Remove format indicators in parentheses: (Doppel-DVD), (2-DVD), (Blu-ray), etc.
    // (?i) makes it case-insensitive to match DVD, Dvd, dvd etc.
    if let Ok(re) = regex::Regex::new(r"(?i)\s*\([^)]*(?:dvd|blu-?ray|disc|disk|cd)[^)]*\)\s*$") {
        name = re.replace(&name, "").to_string();
    }

    // Handle possessive author prefixes: "Author's Title" or "Author' Title" -> "Title"
    // Pattern: Word(s) followed by 's or ' (various apostrophe types) and then the actual title
    // Supports: ' (U+0027), ' (U+2019 right single quote), ʼ (U+02BC), ′ (U+2032)
    if let Ok(re) = regex::Regex::new(
        r"^[A-ZÄÖÜ][a-zäöüß]+(?:\s+[A-ZÄÖÜ][a-zäöüß]+)*['\u{2019}\u{02BC}\u{2032}]s?\s+",
    ) {
        name = re.replace(&name, "").to_string();
    }

    // First, handle colon-separated titles (e.g., "Fast & Furious: 8-Movie-Collection")
    // Take the part before the colon if it looks like a franchise name
    if let Some(colon_pos) = name.find(':') {
        let before_colon = name[..colon_pos].trim();
        let after_colon = name[colon_pos + 1..].trim().to_lowercase();

        // If what's after the colon contains collection indicators, use what's before
        let collection_indicators = [
            "movie",
            "film",
            "collection",
            "box",
            "set",
            "edition",
            "saga",
            "complete",
            "trilogy",
            "trilogie",
        ];
        if collection_indicators
            .iter()
            .any(|ind| after_colon.contains(ind))
        {
            name = before_colon.to_string();
        }
    }

    // Regex patterns for numbered collections - remove these patterns
    let number_patterns = [
        r"\d+[-–]\d+",                     // Number range: "1-6", "1-5" (with en-dash too)
        r"\d+[-\s]*movie[-\s]*collection", // "8-Movie-Collection", "8 Movie Collection"
        r"\d+[-\s]*film[-\s]*collection",  // "6-Film-Collection"
        r"\d+[-\s]*movie[-\s]*set",
        r"\d+[-\s]*film[-\s]*set",
        r"\d+[-\s]*movie", // "8-Movie"
        r"\d+[-\s]*film",  // "6-Film"
        r"\d+[-\s]*filme", // German: "6-Filme"
        r"\d+[-\s]*movies",
    ];

    for pattern in number_patterns {
        if let Ok(re) = regex::Regex::new(&format!(r"(?i)[\s:]*{}[\s]*$", pattern)) {
            name = re.replace_all(&name, " ").to_string();
        }
    }

    // Suffixes to remove ONLY from the END of the string
    // Order matters - check longer patterns first
    let suffixes_to_remove = [
        // Multi-word (longer patterns first)
        " dvd collection box no.",
        " dvd collection box",
        " blu-ray collection box",
        " collection box no.",
        " collection box",
        " - complete collection",
        " complete collection",
        " - complete trilogy",
        " complete trilogy",
        " - complete saga",
        " complete saga",
        " - complete",
        " - triple pack",
        " triple pack",
        " - double pack",
        " double pack",
        " - twin pack",
        " twin pack",
        " box set",
        " box-set",
        " box no.",
        " pack",
        // Single words
        " dvd collection",
        " blu-ray collection",
        " collection",
        " trilogy",
        " trilogie",
        " quadrilogy",
        " pentalogy",
        " saga",
        " hexalogy",
        " anthology",
        " complete",
        " komplett",
        " komplette",
        " ultimate",
        " definitive",
        " essential",
        " sammlung",
        " edition",
        " box",
        " no.",
    ];

    name = name.to_lowercase();

    // FIRST: Remove trailing numbers before suffixes (so "box no. 4" becomes "box no.")
    // Remove trailing number patterns: " 1", " 1,2,3 & 4", " 1-6", " 1+2", etc.
    if let Ok(re) = regex::Regex::new(r"\s+[\d,\s&+und\-–]+\s*$") {
        name = re.replace(&name, "").to_string();
    }
    // Remove trailing Roman numeral ranges: " I-III", " I-IV", " I-VI", etc.
    if let Ok(re) = regex::Regex::new(r"(?i)\s+[ivxlc]+[-–][ivxlc]+\s*$") {
        name = re.replace(&name, "").to_string();
    }
    // Remove trailing Roman numerals: " I", " II", " III", " IV", etc.
    if let Ok(re) = regex::Regex::new(r"(?i)\s+[ivxlc]+\s*$") {
        name = re.replace(&name, "").to_string();
    }
    // Fallback: simple trailing number
    if let Ok(re) = regex::Regex::new(r"\s+\d+\s*$") {
        name = re.replace(&name, "").to_string();
    }

    // THEN: Remove suffixes (now "box no." can be matched)
    let mut changed = true;
    while changed {
        changed = false;
        for suffix in &suffixes_to_remove {
            if name.ends_with(suffix) {
                name = name[..name.len() - suffix.len()].to_string();
                changed = true;
                break;
            }
        }
    }

    // Clean up - remove common prefixes
    let mut result = name
        .trim()
        .trim_end_matches(['-', ':', ' '])
        .trim_start_matches("the ")
        .trim_start_matches("die ")
        .trim()
        .to_string();

    // Remove "complete" from the beginning (after "the" was removed)
    let prefixes_to_remove = ["complete ", "ultimate ", "essential ", "definitive "];
    for prefix in prefixes_to_remove {
        if result.starts_with(prefix) {
            result = result[prefix.len()..].to_string();
            break;
        }
    }

    // Handle German compound words ending with collection indicators
    // e.g., "edelsteintrilogie" -> "edelstein", "marvelsammlung" -> "marvel"
    let compound_suffixes = [
        "trilogie",
        "trilogy",
        "sammlung",
        "collection",
        "anthologie",
        "anthology",
    ];
    for suffix in compound_suffixes {
        if result.ends_with(suffix) && result.len() > suffix.len() {
            // Check if it's a compound word (no space before suffix)
            let before_suffix = &result[..result.len() - suffix.len()];
            if !before_suffix.ends_with(' ') && before_suffix.len() >= 3 {
                result = before_suffix.to_string();
                break;
            }
        }
    }

    result.trim().to_string()
}

/// Check if a title is likely a TV series based on keywords
fn is_likely_tv_series(title: &str) -> bool {
    let title_lower = title.to_lowercase();
    let tv_keywords = [
        "season",
        "staffel",
        "serie",
        "series",
        "episode",
        "folge",
        "staffeln",
        "seasons",
        "serien",
        "episodes",
        "folgen",
        "complete series",
        "komplette serie",
        "gesamtbox",
        "tv serie",
        "tv-serie",
        "tv series",
        "tv-series",
    ];

    tv_keywords.iter().any(|kw| title_lower.contains(kw))
}

/// Extract the TV series name from a title like "Beverly Hills, 90210 - Die erste Season"
fn extract_tv_series_name(title: &str) -> String {
    // Ordinal words in German and English
    let german_ordinals =
        "erste|zweite|dritte|vierte|fünfte|sechste|siebte|achte|neunte|zehnte|elfte|zwölfte";
    let english_ordinals =
        "first|second|third|fourth|fifth|sixth|seventh|eighth|ninth|tenth|eleventh|twelfth";

    // Patterns to remove from the title
    let patterns_to_remove: Vec<String> = vec![
        // Season patterns - German with "komplette X staffel" (e.g., "Die komplette erste Staffel")
        format!(
            r"[-–:]\s*(die\s+)?(komplette\s+)?({})\s+staffel\s*\d*",
            german_ordinals
        ),
        format!(
            r"[-–:]\s*(die\s+)?(komplette\s+)?({}|komplette|complete|ganze)?\s*staffel\s*\d*",
            german_ordinals
        ),
        format!(
            r"[-–:]\s*(die\s+)?(komplette\s+)?({}|komplette|complete|ganze)?\s*season\s*\d*",
            german_ordinals
        ),
        // Season patterns - English with "complete X season"
        format!(
            r"[-–:]\s*(the\s+)?(complete\s+)?({})\s+season\s*\d*",
            english_ordinals
        ),
        format!(
            r"[-–:]\s*(the\s+)?(complete\s+)?({}|complete|entire|full)?\s*season\s*\d*",
            english_ordinals
        ),
        // Generic season/staffel with numbers
        r"\s*[-–]\s*staffel\s*\d+".to_string(),
        r"\s*[-–]\s*season\s*\d+".to_string(),
        r"\s*staffel\s*\d+".to_string(),
        r"\s*season\s*\d+".to_string(),
        r"\s*[-–]\s*s\d+".to_string(), // "- S1", "- S01"
        // Box/Collection patterns - with "Die/The" article
        r"[-–:]\s*(die\s+)?(komplette\s+)?serie".to_string(),
        r"[-–:]\s*(the\s+)?(complete\s+)?series".to_string(),
        r"\s*[-–]\s*gesamtbox".to_string(),
        r"\s*[-–]\s*box\s*set".to_string(),
    ];

    let mut result = title.to_string();

    for pattern in &patterns_to_remove {
        if let Ok(re) = regex::Regex::new(&format!("(?i){}", pattern)) {
            result = re.replace_all(&result, "").to_string();
        }
    }

    // Clean up
    result = result
        .trim()
        .trim_end_matches('-')
        .trim_end_matches('–')
        .trim()
        .to_string();

    // If we removed everything, return original
    if result.is_empty() {
        title.to_string()
    } else {
        result
    }
}

/// Parse movie titles directly from a collection's title
/// Handles patterns like "Triple Feature: Divergent, Insurgent, Allegiant"
/// or "The Divergent Series: Divergent, Insurgent, Allegiant"
fn parse_titles_from_movie_title(title: &str) -> Vec<String> {
    let mut titles = Vec::new();

    // Pattern 0a: Look for "Title N1, N2, ... & Nlast" pattern with any number of entries
    // e.g., "Deadpool 1 & 2" → ["Deadpool", "Deadpool 2"]
    // e.g., "Iron Man 1, 2 & 3" → ["Iron Man", "Iron Man 2", "Iron Man 3"]
    // e.g., "Gregs Tagebuch 1,2,3 & 4" → ["Gregs Tagebuch", "Gregs Tagebuch 2", "Gregs Tagebuch 3", "Gregs Tagebuch 4"]
    if let Ok(re) = regex::Regex::new(r"^(.+?)\s+([\d,\s]+)[&+]\s*(\d+)$")
        && let Some(caps) = re.captures(title.trim())
    {
        let base_title = caps.get(1).map(|m| m.as_str().trim()).unwrap_or("");
        let numbers_part = caps.get(2).map(|m| m.as_str()).unwrap_or("");
        let last_num = caps.get(3).map(|m| m.as_str());

        if !base_title.is_empty() {
            // Parse all numbers from the comma-separated part
            let number_re = regex::Regex::new(r"\d+").unwrap();
            for num_match in number_re.find_iter(numbers_part) {
                let n = num_match.as_str();
                if n == "1" {
                    titles.push(base_title.to_string());
                } else {
                    titles.push(format!("{} {}", base_title, n));
                }
            }

            // Add last number
            if let Some(n) = last_num {
                titles.push(format!("{} {}", base_title, n));
            }

            if titles.len() >= 2 {
                return titles;
            }
        }
        titles.clear();
    }

    // Pattern 0b: Look for "+" separator (e.g., "Die Bourne Identität + Die Bourne Verschwörung")
    // This is a common pattern for 2-film collections
    if title.contains(" + ") {
        let parts: Vec<&str> = title.split(" + ").collect();

        if parts.len() >= 2 {
            for part in parts {
                let cleaned = part.trim().to_string();
                if !cleaned.is_empty() && cleaned.len() < 80 && !titles.contains(&cleaned) {
                    titles.push(cleaned);
                }
            }
        }

        if titles.len() >= 2 {
            return titles;
        }
        titles.clear(); // Reset if we didn't get enough titles
    }

    // Pattern 0c: Look for semicolon-separated titles with optional years
    // e.g., "Rubinrot (2013); Saphirblau (2014); Smaragdgrün (2016)"
    // e.g., "Film A; Film B; Film C"
    if title.contains("; ") {
        let parts: Vec<&str> = title.split(';').collect();

        if parts.len() >= 2 {
            // Year pattern in parentheses
            let year_re = regex::Regex::new(r"\s*\(\d{4}\)\s*$").ok();

            for part in parts {
                let trimmed = part.trim();
                // Remove year in parentheses if present
                let cleaned = if let Some(ref re) = year_re {
                    re.replace(trimmed, "").trim().to_string()
                } else {
                    trimmed.to_string()
                };

                if !cleaned.is_empty()
                    && cleaned.len() >= 3
                    && cleaned.len() < 80
                    && !titles.contains(&cleaned)
                {
                    titles.push(cleaned);
                }
            }
        }

        if titles.len() >= 2 {
            return titles;
        }
        titles.clear();
    }

    // Pattern 1: Look for a colon followed by comma-separated list
    // e.g., "Triple Feature: Divergent, Insurgent, Allegiant"
    // e.g., "The Divergent Series: Divergent, Insurgent, Allegiant"
    if let Some(colon_pos) = title.find(':') {
        let after_colon = &title[colon_pos + 1..].trim();

        // Check if what's after the colon looks like a comma-separated list
        if after_colon.contains(',') {
            let parts: Vec<&str> = after_colon.split(',').collect();

            // If we have at least 2 parts and they look like movie titles (not too long, not empty)
            if parts.len() >= 2 {
                let valid_parts: Vec<&str> = parts
                    .iter()
                    .map(|p| p.trim())
                    .filter(|p| !p.is_empty() && p.len() < 80)
                    .collect();

                if valid_parts.len() >= 2 {
                    for part in valid_parts {
                        // Clean up the title (remove year in parentheses, etc.)
                        let cleaned = part
                            .trim()
                            .trim_matches(|c| c == '(' || c == ')')
                            .to_string();

                        if !cleaned.is_empty() && !titles.contains(&cleaned) {
                            titles.push(cleaned);
                        }
                    }
                }
            }
        }
    }

    // Pattern 2: Look for "X, Y & Z" or "X, Y und Z" pattern anywhere in title
    if titles.is_empty() {
        // Check for comma-separated list with optional "and/&/und" before last item
        let patterns = [
            r"[:,]\s*([^,&]+),\s*([^,&]+)\s*(?:&|und|and)\s*([^,&]+)", // "Title: A, B & C"
            r"[:,]\s*([^,]+),\s*([^,]+),\s*([^,]+)",                   // "Title: A, B, C"
        ];

        for pattern in patterns {
            if let Ok(re) = regex::Regex::new(pattern)
                && let Some(caps) = re.captures(title)
            {
                for i in 1..=caps.len() - 1 {
                    if let Some(m) = caps.get(i) {
                        let part = m.as_str().trim();
                        if !part.is_empty()
                            && part.len() < 80
                            && !titles.contains(&part.to_string())
                        {
                            titles.push(part.to_string());
                        }
                    }
                }
            }
            if titles.len() >= 2 {
                break;
            }
        }
    }

    titles
}

struct ParsedTitle {
    title: String,
    excerpt: Option<String>,
}

/// Parse a collection description to extract individual film titles
fn parse_collection_description(description: &str) -> Vec<ParsedTitle> {
    let mut titles = Vec::new();

    // Pattern 1: "; ; TITLE" pattern - double semicolon followed by ALL CAPS title
    // Split by "; ;" and check each segment for ALL CAPS titles
    let segments: Vec<&str> = description.split("; ;").collect();

    // Determine if first segment is intro text or a movie title
    // If first segment starts with ALL CAPS text followed by ; it's likely a movie title
    let skip_first = if let Some(first) = segments.first() {
        let first = first.trim();
        if let Some(end_pos) = find_title_end(first) {
            let potential_title = &first[..end_pos];
            // If it doesn't look like a title (not mostly uppercase), skip it
            !is_mostly_uppercase(potential_title.trim())
        } else {
            // No clear title end found, might be intro text
            !is_mostly_uppercase(&first.chars().take(50).collect::<String>())
        }
    } else {
        false
    };

    let segments_iter: Box<dyn Iterator<Item = &&str>> = if skip_first {
        Box::new(segments.iter().skip(1))
    } else {
        Box::new(segments.iter())
    };

    for segment in segments_iter {
        // Find the title part - it's the ALL CAPS text before the description
        // The title ends at :; or just ; followed by lowercase text
        let segment = segment.trim();

        // Try to find where the title ends (either at :; or ; followed by description)
        let title_end = find_title_end(segment);

        if let Some(end_pos) = title_end {
            let title_part = &segment[..end_pos];
            let title = title_part.trim().trim_end_matches(':').trim();

            if title.len() > 3 && is_mostly_uppercase(title) && !is_common_phrase(title) {
                let cleaned = clean_extracted_title(title);
                if !cleaned.is_empty()
                    && cleaned.len() > 3
                    && !titles.iter().any(|t: &ParsedTitle| t.title == cleaned)
                {
                    let excerpt = if end_pos < segment.len() {
                        let rest = segment[end_pos..].trim_start_matches([':', ';', ' ']);
                        if !rest.is_empty() && rest.len() > 10 {
                            Some(rest.chars().take(200).collect::<String>())
                        } else {
                            None
                        }
                    } else {
                        None
                    };

                    titles.push(ParsedTitle {
                        title: cleaned,
                        excerpt,
                    });
                }
            }
        }
    }

    // Pattern 2: Fallback - ALL CAPS lines ending with :; or ;
    if titles.len() < 2 {
        let caps_pattern = regex::Regex::new(
            r"(?m)^[\s;]*([A-ZÄÖÜÀÁÂÃÈÉÊËÌÍÎÏÒÓÔÕÙÚÛÝŸ][A-ZÄÖÜÀÁÂÃÈÉÊËÌÍÎÏÒÓÔÕÙÚÛÝŸ0-9³²¹\s\-',\.!?:]+?):?;"
        ).ok();

        if let Some(re) = caps_pattern {
            for cap in re.captures_iter(description) {
                if let Some(m) = cap.get(1) {
                    let title = m.as_str().trim().trim_end_matches(':');
                    if title.len() > 3 && !is_common_phrase(title) {
                        let cleaned = clean_extracted_title(title);
                        if !cleaned.is_empty()
                            && cleaned.len() > 3
                            && !titles.iter().any(|t| t.title == cleaned)
                        {
                            titles.push(ParsedTitle {
                                title: cleaned,
                                excerpt: None,
                            });
                        }
                    }
                }
            }
        }
    }

    // Pattern 2: Look for "; ; TITLE" pattern (double semicolon separator)
    // This is common in some collection descriptions
    if titles.len() < 2 {
        let double_semi_pattern = regex::Regex::new(r";\s*;\s*([^;:]+?)(?::|;|$)").ok();
        if let Some(re) = double_semi_pattern {
            for cap in re.captures_iter(description) {
                if let Some(m) = cap.get(1) {
                    let title = m.as_str().trim();
                    // Check if it looks like a title (starts with uppercase, mostly uppercase)
                    let uppercase_ratio = title.chars().filter(|c| c.is_uppercase()).count() as f32
                        / title.chars().filter(|c| c.is_alphabetic()).count().max(1) as f32;

                    if title.len() > 3
                        && title.len() < 80
                        && uppercase_ratio > 0.5
                        && !is_common_phrase(title)
                    {
                        let cleaned = clean_extracted_title(title);
                        if !cleaned.is_empty() && !titles.iter().any(|t| t.title == cleaned) {
                            titles.push(ParsedTitle {
                                title: cleaned,
                                excerpt: None,
                            });
                        }
                    }
                }
            }
        }
    }

    // Pattern 3: Numbered list (1. Film Title, 2. Film Title)
    if titles.len() < 2 {
        let numbered_pattern = regex::Regex::new(r"(?m)^\s*\d+[\.\)]\s*(.+?)(?:\n|$)").ok();
        if let Some(re) = numbered_pattern {
            for cap in re.captures_iter(description) {
                if let Some(m) = cap.get(1) {
                    let title = m.as_str().trim();
                    if !title.is_empty() && !is_common_phrase(title) {
                        let cleaned = clean_extracted_title(title);
                        if !titles.iter().any(|t| t.title == cleaned) {
                            titles.push(ParsedTitle {
                                title: cleaned,
                                excerpt: None,
                            });
                        }
                    }
                }
            }
        }
    }

    // Pattern 3b: Bullet point list (• Title; • Title; or * Title; or - Title;)
    // e.g., "• Alles über Eva; • Niagara; • Blondinen bevorzugt;"
    if titles.len() < 2 {
        // Match bullet points: •, *, -, followed by title, ending with ; or end of string
        let bullet_pattern = regex::Regex::new(r"[•\*\-]\s*([^;•\*]+?)(?:;|$)").ok();
        if let Some(re) = bullet_pattern {
            for cap in re.captures_iter(description) {
                if let Some(m) = cap.get(1) {
                    let title = m.as_str().trim();
                    // Skip if it's intro/promo text (usually longer)
                    if !title.is_empty()
                        && title.len() >= 3
                        && title.len() < 80
                        && !is_common_phrase(title)
                        && !title.to_lowercase().starts_with("diese")
                        && !title.to_lowercase().starts_with("this")
                    {
                        let cleaned = clean_extracted_title(title);
                        if !cleaned.is_empty()
                            && cleaned.len() >= 3
                            && !titles.iter().any(|t| t.title == cleaned)
                        {
                            titles.push(ParsedTitle {
                                title: cleaned,
                                excerpt: None,
                            });
                        }
                    }
                }
            }
        }
    }

    // Pattern 4: Double semicolon groups with mixed case titles
    // Format: "Title1; Bonus-...; ; Title2; Bonus-...; ; Title3"
    // Each group is separated by "; ; " and the first non-Bonus item in each group is the title
    if titles.len() < 2 && description.contains("; ;") {
        let groups: Vec<&str> = description.split("; ;").collect();

        for group in groups {
            // Split group by single semicolon
            let items: Vec<&str> = group.split(';').collect();

            // Find first item that's not a bonus/extra
            for item in items {
                let trimmed = item.trim();
                // Skip bonus content
                if trimmed.to_lowercase().starts_with("bonus")
                    || trimmed.to_lowercase().contains("kurzfilm")
                    || trimmed.to_lowercase().contains("trickfilm")
                    || trimmed.to_lowercase().contains("dokumentation")
                    || trimmed.is_empty()
                    || trimmed.len() < 3
                    || trimmed.len() > 80
                {
                    continue;
                }

                // Check if this looks like a title (starts with uppercase)
                if let Some(first_char) = trimmed.chars().next()
                    && first_char.is_uppercase()
                    && !is_common_phrase(trimmed)
                {
                    let cleaned = clean_extracted_title(trimmed);
                    if !cleaned.is_empty()
                        && cleaned.len() > 3
                        && !titles.iter().any(|t| t.title == cleaned)
                    {
                        titles.push(ParsedTitle {
                            title: cleaned,
                            excerpt: None,
                        });
                        break; // Only take first valid title per group
                    }
                }
            }
        }
    }

    // Pattern 5: Semicolon-separated with ALL CAPS detection
    // Look for segments between semicolons that are mostly uppercase
    if titles.len() < 2 && description.contains(';') {
        let parts: Vec<&str> = description.split(';').collect();
        for part in parts {
            let trimmed = part.trim();
            if trimmed.len() > 3 && trimmed.len() < 80 {
                // Check uppercase ratio
                let uppercase_count = trimmed.chars().filter(|c| c.is_uppercase()).count();
                let alpha_count = trimmed.chars().filter(|c| c.is_alphabetic()).count().max(1);
                let uppercase_ratio = uppercase_count as f32 / alpha_count as f32;

                if uppercase_ratio > 0.7 && !is_common_phrase(trimmed) {
                    let cleaned = clean_extracted_title(trimmed);
                    if !cleaned.is_empty()
                        && cleaned.len() > 3
                        && !titles.iter().any(|t| t.title == cleaned)
                    {
                        titles.push(ParsedTitle {
                            title: cleaned,
                            excerpt: None,
                        });
                    }
                }
            }
        }
    }

    // Pattern 6: Title Case titles followed by :; and description
    // Format: "Title:; Description text...; ; Next Title:; More description..."
    // e.g., "Matrix:; Der Hacker Neo wird...; ; Matrix Reloaded:; Und wenn..."
    if titles.len() < 2 {
        // Match: Title (starting with uppercase, short) followed by :; and then text
        let title_colon_semi_pattern = regex::Regex::new(
            r"(?:^|;\s*;\s*|;\s*)([A-ZÄÖÜ][A-Za-zÄÖÜäöüß0-9\s\-':]+?):;\s+[A-ZÄÖÜ]",
        )
        .ok();

        if let Some(re) = title_colon_semi_pattern {
            for cap in re.captures_iter(description) {
                if let Some(m) = cap.get(1) {
                    let title = m.as_str().trim();
                    // Check it's a reasonable title length (not too long)
                    if title.len() >= 3 && title.len() <= 60 && !is_common_phrase(title) {
                        let cleaned = clean_extracted_title(title);
                        if !cleaned.is_empty()
                            && cleaned.len() >= 3
                            && !titles.iter().any(|t| t.title == cleaned)
                        {
                            titles.push(ParsedTitle {
                                title: cleaned,
                                excerpt: None,
                            });
                        }
                    }
                }
            }
        }
    }

    titles
}

/// Find where the title ends in a segment
/// Title ends at :; or when we see description text (not ALL CAPS) after ; or :
/// Returns byte index (not char index) for safe string slicing
fn find_title_end(segment: &str) -> Option<usize> {
    // Use char_indices to get byte positions
    let char_indices: Vec<(usize, char)> = segment.char_indices().collect();

    for (idx, &(byte_pos, c)) in char_indices.iter().enumerate() {
        // If we hit a semicolon, check what follows
        if c == ';' {
            // Check if next non-space char is lowercase (description start)
            let rest = &segment[byte_pos + 1..];
            let rest_trimmed = rest.trim_start();
            if let Some(next_char) = rest_trimmed.chars().next()
                && (next_char.is_lowercase()
                    || (next_char.is_uppercase() && rest_trimmed.len() > 20))
            {
                return Some(byte_pos);
            }
            return Some(byte_pos);
        }

        // If we hit a colon followed by semicolon, that's the end
        if c == ':' && idx + 1 < char_indices.len() && char_indices[idx + 1].1 == ';' {
            return Some(byte_pos);
        }

        // If we hit a colon followed by text, check if it's description (not ALL CAPS)
        if c == ':' && byte_pos + 1 < segment.len() {
            let rest = &segment[byte_pos + 1..];
            let rest_trimmed = rest.trim_start();

            // Check the first ~50 chars to see if it's mixed case (description) vs ALL CAPS (still title)
            let sample: String = rest_trimmed.chars().take(50).collect();
            if !sample.is_empty() {
                // If the sample is NOT mostly uppercase, it's description text
                if !is_mostly_uppercase(&sample) {
                    return Some(byte_pos);
                }
            }
        }
    }

    // If no clear end found, return the whole segment if it looks like a title
    if is_mostly_uppercase(segment) && segment.len() < 80 {
        Some(segment.len())
    } else {
        None
    }
}

/// Check if text is mostly uppercase (title-like)
fn is_mostly_uppercase(text: &str) -> bool {
    let alpha_chars: Vec<char> = text.chars().filter(|c| c.is_alphabetic()).collect();
    if alpha_chars.is_empty() {
        return false;
    }
    let uppercase_count = alpha_chars.iter().filter(|c| c.is_uppercase()).count();
    (uppercase_count as f32 / alpha_chars.len() as f32) > 0.7
}

/// Check if text is a common non-title phrase
fn is_common_phrase(text: &str) -> bool {
    let phrases = [
        "ERLEBEN SIE",
        "DIESE",
        "JETZT",
        "ERSTMALS",
        "ZUSAMMEN",
        "BOX",
        "COLLECTION",
        "SET",
        "ENTHÄLT",
        "BEINHALTET",
        "HINWEIS",
        "NOTE",
        "FEATURES",
        "SPECIAL",
    ];

    let upper = text.to_uppercase();
    phrases.iter().any(|p| upper.starts_with(p))
}

/// Clean up an extracted title
fn clean_extracted_title(title: &str) -> String {
    let mut cleaned = title.to_string();

    // Remove trailing punctuation
    cleaned = cleaned.trim_end_matches([':', ';', '-', '.']).to_string();

    // Convert from ALL CAPS to Title Case (simple version)
    if cleaned.chars().all(|c| !c.is_lowercase()) {
        cleaned = cleaned
            .split_whitespace()
            .map(|word| {
                let mut chars: Vec<char> = word.chars().collect();
                if !chars.is_empty() {
                    chars[0] = chars[0].to_uppercase().next().unwrap_or(chars[0]);
                    for c in chars.iter_mut().skip(1) {
                        *c = c.to_lowercase().next().unwrap_or(*c);
                    }
                }
                chars.into_iter().collect::<String>()
            })
            .collect::<Vec<_>>()
            .join(" ");
    }

    cleaned.trim().to_string()
}
