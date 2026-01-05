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

use my_movies_core::models::{Claims, CreateMovie, MovieFilter, UpdateMovie};
use my_movies_core::services::TmdbService;

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

    let limit = filter.limit.unwrap_or(50);
    let offset = filter.offset.unwrap_or(0);

    match state.movie_service.list(claims.sub, filter).await {
        Ok(movies) => (
            StatusCode::OK,
            Json(json!({
                "items": movies,
                "total": total,
                "limit": limit,
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

#[derive(Debug, serde::Deserialize)]
pub struct RefreshTmdbQuery {
    #[serde(default)]
    pub force: bool, // If true, reload all data even if already present
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

    // Try to get TMDB details
    let tmdb_details = if let Some(tmdb_id) = movie.tmdb_id {
        // Use existing TMDB ID
        state
            .tmdb_service
            .get_movie_details(tmdb_id, language)
            .await
            .ok()
    } else {
        // Search by title
        let year = movie.production_year;
        match state
            .tmdb_service
            .search_movies(&movie.title, year, language, include_adult)
            .await
        {
            Ok(results) if !results.is_empty() => {
                let first = &results[0];
                state
                    .tmdb_service
                    .get_movie_details(first.id, language)
                    .await
                    .ok()
            }
            _ => None,
        }
    };

    let Some(details) = tmdb_details else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "No TMDB data found for this movie" })),
        )
            .into_response();
    };

    // Get credits for director and actors
    let credits = state
        .tmdb_service
        .get_movie_credits(details.id, language)
        .await
        .ok();

    // Build update
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

    // Download poster image if available
    // Download if:
    // - force=true (always download)
    // - OR poster_data is None (no poster stored in DB yet)
    let should_download_poster = params.force || movie.poster_data.is_none();

    let poster_data = if should_download_poster {
        if let Some(ref poster_path) = details.poster_path {
            download_poster_image(poster_path).await
        } else {
            None
        }
    } else {
        None // Skip download if poster already in DB and not forcing
    };

    // Build update - only include fields that are missing or if force=true
    let mut update = my_movies_core::models::UpdateMovie {
        tmdb_id: Some(details.id),
        ..Default::default()
    };

    // Only update fields if they're missing or force=true
    if params.force || movie.imdb_id.is_none() {
        update.imdb_id = details.imdb_id.clone();
    }
    if params.force || movie.original_title.is_none() {
        update.original_title = details.original_title.clone();
    }
    if params.force || movie.description.is_none() {
        update.description = details.overview.clone();
    }
    if params.force || movie.tagline.is_none() {
        update.tagline = details.tagline.clone();
    }
    if params.force || movie.running_time.is_none() {
        update.running_time = details.runtime;
    }
    if params.force || movie.director.is_none() {
        update.director = director;
    }
    if params.force || movie.actors.is_none() {
        update.actors = actors;
    }
    if params.force || movie.genres.is_none() {
        update.genres = genres;
    }
    if params.force || movie.budget.is_none() {
        update.budget = details.budget;
    }
    if params.force || movie.revenue.is_none() {
        update.revenue = details.revenue;
    }

    // First update the movie data
    let updated_movie = match state.movie_service.update(claims.sub, id, update).await {
        Ok(m) => m,
        Err(e) => {
            return (
                StatusCode::from_u16(e.status_code()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
                Json(json!({ "error": e.to_string() })),
            )
                .into_response();
        }
    };

    // Then update poster data if we downloaded it
    let final_movie = if let Some(data) = poster_data {
        match state
            .movie_service
            .update_movie_poster_data(claims.sub, id, Some(data))
            .await
        {
            Ok(m) => m,
            Err(e) => {
                tracing::warn!("Failed to save poster data: {}", e);
                updated_movie
            }
        }
    } else {
        updated_movie
    };

    // Broadcast to WebSocket clients
    let msg = json!({
        "type": "movie_updated",
        "payload": final_movie
    });
    let _ = state.ws_broadcast.send(msg.to_string());

    (StatusCode::OK, Json(json!(final_movie))).into_response()
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
