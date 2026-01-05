use std::sync::Arc;

use axum::{
    Extension, Json,
    extract::{Multipart, Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde_json::json;
use tokio::fs;
use uuid::Uuid;

use my_movies_core::models::{Claims, CreateMovie, MovieFilter, UpdateMovie};

use crate::AppState;

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

pub async fn refresh_tmdb(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<Uuid>,
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

    let update = my_movies_core::models::UpdateMovie {
        tmdb_id: Some(details.id),
        imdb_id: details.imdb_id.clone(),
        original_title: details.original_title.clone(),
        description: details.overview.clone(),
        tagline: details.tagline.clone(),
        running_time: details.runtime,
        poster_path: details.poster_path.clone(),
        director,
        actors,
        genres,
        budget: details.budget,
        revenue: details.revenue,
        ..Default::default()
    };

    match state.movie_service.update(claims.sub, id, update).await {
        Ok(updated_movie) => {
            // Broadcast to WebSocket clients
            let msg = json!({
                "type": "movie_updated",
                "payload": updated_movie
            });
            let _ = state.ws_broadcast.send(msg.to_string());

            (StatusCode::OK, Json(json!(updated_movie))).into_response()
        }
        Err(e) => (
            StatusCode::from_u16(e.status_code()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
            Json(json!({ "error": e.to_string() })),
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

    // Create uploads directory if it doesn't exist
    let uploads_dir = std::path::PathBuf::from("uploads/posters");
    if let Err(e) = fs::create_dir_all(&uploads_dir).await {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": format!("Failed to create uploads directory: {}", e) })),
        )
            .into_response();
    }

    // Process multipart upload
    while let Some(field) = multipart.next_field().await.unwrap_or(None) {
        let name = field.name().unwrap_or("").to_string();

        if name == "file" {
            // Get content type to determine extension
            let content_type = field.content_type().unwrap_or("image/jpeg").to_string();
            let extension = match content_type.as_str() {
                "image/png" => "png",
                "image/gif" => "gif",
                "image/webp" => "webp",
                _ => "jpg", // Default to jpg for jpeg and unknown types
            };

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

            // Validate it's actually an image (basic check)
            if data.len() < 8 {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(json!({ "error": "File too small to be a valid image" })),
                )
                    .into_response();
            }

            // Generate unique filename
            let filename = format!("{}.{}", id, extension);
            let file_path = uploads_dir.join(&filename);

            // Save file
            if let Err(e) = fs::write(&file_path, &data).await {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({ "error": format!("Failed to save file: {}", e) })),
                )
                    .into_response();
            }

            // Update movie with local poster path (use special prefix to indicate local file)
            let poster_url = format!("/uploads/posters/{}", filename);
            let update = UpdateMovie {
                poster_path: Some(poster_url.clone()),
                ..Default::default()
            };

            if let Err(e) = state.movie_service.update(claims.sub, id, update).await {
                // Try to clean up the uploaded file
                let _ = fs::remove_file(&file_path).await;
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({ "error": format!("Failed to update movie: {}", e) })),
                )
                    .into_response();
            }

            return (
                StatusCode::OK,
                Json(json!({
                    "message": "Poster uploaded successfully",
                    "poster_path": poster_url
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
