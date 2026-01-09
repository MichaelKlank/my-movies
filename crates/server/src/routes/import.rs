use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use axum::extract::{Multipart, Query};
use axum::{Extension, Json, extract::State, http::StatusCode, response::IntoResponse};
use serde_json::json;
use tokio::time::{Duration, sleep};

use my_movies_core::models::{Claims, MovieFilter, UpdateMovie};
use my_movies_core::services::TmdbService;

use crate::AppState;

/// Global state for TMDB enrichment
static ENRICH_CANCELLED: AtomicBool = AtomicBool::new(false);
static ENRICH_RUNNING: AtomicBool = AtomicBool::new(false);
static ENRICH_TOTAL: AtomicU32 = AtomicU32::new(0);
static ENRICH_CURRENT: AtomicU32 = AtomicU32::new(0);
static ENRICH_UPDATED: AtomicU32 = AtomicU32::new(0);
static ENRICH_ERRORS: AtomicU32 = AtomicU32::new(0);

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

pub async fn import_csv(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    mut multipart: Multipart,
) -> impl IntoResponse {
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

            let cursor = std::io::Cursor::new(data);

            match state.import_service.import_csv(claims.sub, cursor).await {
                Ok(result) => {
                    // Broadcast refresh to WebSocket clients
                    let msg = json!({ "type": "collection_imported" });
                    let _ = state.ws_broadcast.send(msg.to_string());

                    return (
                        StatusCode::OK,
                        Json(json!({
                            "movies_imported": result.movies_imported,
                            "series_imported": result.series_imported,
                            "collections_imported": result.collections_imported,
                            "errors": result.errors
                        })),
                    )
                        .into_response();
                }
                Err(e) => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({ "error": e.to_string() })),
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

#[derive(Debug, serde::Deserialize)]
pub struct EnrichTmdbQuery {
    #[serde(default)]
    pub force: bool, // If true, reload all movies even if they already have data
}

/// Enrich all movies with TMDB data
/// This runs asynchronously and sends progress via WebSocket
pub async fn enrich_movies_tmdb(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Query(params): Query<EnrichTmdbQuery>,
) -> impl IntoResponse {
    // Get ALL movies (no limit)
    let filter = MovieFilter {
        limit: Some(10000), // High limit to get all
        ..Default::default()
    };

    let movies = match state.movie_service.list(claims.sub, filter).await {
        Ok(m) => m,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": e.to_string() })),
            )
                .into_response();
        }
    };

    // Filter movies based on force parameter
    let movies_to_enrich: Vec<_> = if params.force {
        // If force=true, process all movies
        movies
    } else {
        // Otherwise, process movies without tmdb_id OR without poster_data
        // (meaning they need TMDB data or poster image)
        movies
            .into_iter()
            .filter(|m| m.tmdb_id.is_none() || m.poster_data.is_none())
            .collect()
    };

    let total = movies_to_enrich.len();

    if total == 0 {
        return (
            StatusCode::OK,
            Json(json!({
                "message": "All movies already have TMDB data",
                "total": 0
            })),
        )
            .into_response();
    }

    // Check if already running
    if ENRICH_RUNNING.load(Ordering::SeqCst) {
        return (
            StatusCode::CONFLICT,
            Json(json!({ "error": "TMDB enrichment is already running" })),
        )
            .into_response();
    }

    // Reset cancellation flag and mark as running, init progress
    ENRICH_CANCELLED.store(false, Ordering::SeqCst);
    ENRICH_RUNNING.store(true, Ordering::SeqCst);
    ENRICH_TOTAL.store(total as u32, Ordering::SeqCst);
    ENRICH_CURRENT.store(0, Ordering::SeqCst);
    ENRICH_UPDATED.store(0, Ordering::SeqCst);
    ENRICH_ERRORS.store(0, Ordering::SeqCst);

    // Send initial status
    let msg = json!({
        "type": "tmdb_enrich_started",
        "payload": { "total": total }
    });
    let _ = state.ws_broadcast.send(msg.to_string());

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
    let language = user.language.clone();
    let include_adult = user.include_adult;

    // Spawn background task for enrichment
    let state_clone = state.clone();
    let user_id = claims.sub;
    let language_clone = language.clone();
    let force_clone = params.force;

    tokio::spawn(async move {
        let mut enriched = 0;
        let mut errors: Vec<String> = Vec::new();
        let lang = language_clone.as_deref();
        let mut cancelled = false;

        for (index, movie) in movies_to_enrich.iter().enumerate() {
            // Check for cancellation
            if ENRICH_CANCELLED.load(Ordering::SeqCst) {
                cancelled = true;
                let msg = json!({
                    "type": "tmdb_enrich_cancelled",
                    "payload": {
                        "current": index,
                        "total": total,
                        "enriched": enriched
                    }
                });
                let _ = state_clone.ws_broadcast.send(msg.to_string());
                break;
            }
            // Try to get TMDB details - priority: tmdb_id > imdb_id > title search
            let tmdb_details = if let Some(tmdb_id) = movie.tmdb_id {
                // Use existing TMDB ID
                state_clone
                    .tmdb_service
                    .get_movie_details(tmdb_id, lang)
                    .await
                    .ok()
            } else if let Some(imdb_id) = &movie.imdb_id {
                // Try to find by IMDB ID
                match state_clone.tmdb_service.find_by_imdb_id(imdb_id).await {
                    Ok(Some(found)) => state_clone
                        .tmdb_service
                        .get_movie_details(found.id, lang)
                        .await
                        .ok(),
                    _ => {
                        // Fallback to title search
                        let year = movie.production_year;
                        match state_clone
                            .tmdb_service
                            .search_movies(&movie.title, year, lang, include_adult)
                            .await
                        {
                            Ok(results) if !results.is_empty() => state_clone
                                .tmdb_service
                                .get_movie_details(results[0].id, lang)
                                .await
                                .ok(),
                            _ => None,
                        }
                    }
                }
            } else {
                // Search by title
                let year = movie.production_year;
                match state_clone
                    .tmdb_service
                    .search_movies(&movie.title, year, lang, include_adult)
                    .await
                {
                    Ok(results) if !results.is_empty() => {
                        let first = &results[0];
                        state_clone
                            .tmdb_service
                            .get_movie_details(first.id, lang)
                            .await
                            .ok()
                    }
                    _ => None,
                }
            };

            if let Some(details) = tmdb_details {
                // Get credits
                let credits = state_clone
                    .tmdb_service
                    .get_movie_credits(details.id, lang)
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

                // Download poster image if available
                // Download if:
                // - force=true (always download)
                // - OR poster_data is None (no poster stored in DB yet)
                let should_download_poster = force_clone || movie.poster_data.is_none();

                let poster_data = if should_download_poster {
                    if let Some(ref poster_path) = details.poster_path {
                        download_poster_image(poster_path).await
                    } else {
                        None
                    }
                } else {
                    None // Skip download if poster already in DB and not forcing
                };

                let update = UpdateMovie {
                    tmdb_id: Some(details.id),
                    imdb_id: details.imdb_id.clone(),
                    original_title: details.original_title.clone(),
                    description: details.overview.clone(),
                    tagline: details.tagline.clone(),
                    running_time: details.runtime,
                    director,
                    actors,
                    genres,
                    budget: details.budget,
                    revenue: details.revenue,
                    ..Default::default()
                };

                // First update the movie data
                if state_clone
                    .movie_service
                    .update(user_id, movie.id, update)
                    .await
                    .is_ok()
                {
                    // Then update poster data if we downloaded it
                    #[allow(clippy::collapsible_if)]
                    if let Some(data) = poster_data {
                        if state_clone
                            .movie_service
                            .update_movie_poster_data(user_id, movie.id, Some(data))
                            .await
                            .is_err()
                        {
                            tracing::warn!("Failed to save poster data for: {}", movie.title);
                        }
                    }
                    enriched += 1;
                } else {
                    errors.push(format!("Failed to update: {}", movie.title));
                }
            } else {
                errors.push(format!("No TMDB data found: {}", movie.title));
            }

            // Update global progress
            ENRICH_CURRENT.store((index + 1) as u32, Ordering::SeqCst);
            ENRICH_UPDATED.store(enriched as u32, Ordering::SeqCst);
            ENRICH_ERRORS.store(errors.len() as u32, Ordering::SeqCst);

            // Send progress every 10 movies or at the end
            if (index + 1) % 10 == 0 || index == total - 1 {
                let msg = json!({
                    "type": "tmdb_enrich_progress",
                    "payload": {
                        "current": index + 1,
                        "total": total,
                        "enriched": enriched,
                        "errors_count": errors.len()
                    }
                });
                let _ = state_clone.ws_broadcast.send(msg.to_string());
            }

            // Rate limiting - don't hammer TMDB API
            sleep(Duration::from_millis(250)).await;
        }

        // Mark as not running and reset progress
        ENRICH_RUNNING.store(false, Ordering::SeqCst);

        // Send completion (only if not cancelled - cancellation sends its own message)
        if !cancelled {
            let msg = json!({
                "type": "tmdb_enrich_complete",
                "payload": {
                    "total": total,
                    "enriched": enriched,
                    "errors": errors
                }
            });
            let _ = state_clone.ws_broadcast.send(msg.to_string());
        }
    });

    // Return immediately
    (
        StatusCode::ACCEPTED,
        Json(json!({
            "message": "TMDB enrichment started",
            "total": total
        })),
    )
        .into_response()
}

/// Cancel the running TMDB enrichment
pub async fn cancel_enrich_tmdb(Extension(claims): Extension<Claims>) -> impl IntoResponse {
    // Only admins can cancel
    if claims.role != my_movies_core::models::UserRole::Admin {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({ "error": "Only admins can cancel enrichment" })),
        )
            .into_response();
    }

    if !ENRICH_RUNNING.load(Ordering::SeqCst) {
        return (
            StatusCode::OK,
            Json(json!({ "message": "No enrichment running" })),
        )
            .into_response();
    }

    ENRICH_CANCELLED.store(true, Ordering::SeqCst);

    (
        StatusCode::OK,
        Json(json!({ "message": "Cancellation requested" })),
    )
        .into_response()
}

/// Get current enrichment status (for checking on page load)
pub async fn get_enrich_status() -> impl IntoResponse {
    let is_running = ENRICH_RUNNING.load(Ordering::SeqCst);

    if is_running {
        let total = ENRICH_TOTAL.load(Ordering::SeqCst);
        let current = ENRICH_CURRENT.load(Ordering::SeqCst);
        let updated = ENRICH_UPDATED.load(Ordering::SeqCst);
        let errors = ENRICH_ERRORS.load(Ordering::SeqCst);

        (
            StatusCode::OK,
            Json(json!({
                "is_running": true,
                "total": total,
                "current": current,
                "updated": updated,
                "errors_count": errors
            })),
        )
            .into_response()
    } else {
        (
            StatusCode::OK,
            Json(json!({
                "is_running": false
            })),
        )
            .into_response()
    }
}
