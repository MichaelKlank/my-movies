use std::sync::Arc;

use axum::extract::Multipart;
use axum::{Extension, Json, extract::State, http::StatusCode, response::IntoResponse};
use serde_json::json;
use tokio::time::{Duration, sleep};

use my_movies_core::models::{Claims, MovieFilter, UpdateMovie};

use crate::AppState;

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

/// Enrich all movies that don't have a poster_path with TMDB data
/// This runs asynchronously and sends progress via WebSocket
pub async fn enrich_movies_tmdb(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
) -> impl IntoResponse {
    // Get ALL movies without poster_path (no limit)
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

    let movies_without_poster: Vec<_> = movies
        .into_iter()
        .filter(|m| m.poster_path.is_none())
        .collect();

    let total = movies_without_poster.len();

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

    // Send initial status
    let msg = json!({
        "type": "tmdb_enrich_started",
        "payload": { "total": total }
    });
    let _ = state.ws_broadcast.send(msg.to_string());

    // Spawn background task for enrichment
    let state_clone = state.clone();
    let user_id = claims.sub;

    tokio::spawn(async move {
        let mut enriched = 0;
        let mut errors: Vec<String> = Vec::new();

        for (index, movie) in movies_without_poster.iter().enumerate() {
            // Try to get TMDB details - priority: tmdb_id > imdb_id > title search
            let tmdb_details = if let Some(tmdb_id) = movie.tmdb_id {
                // Use existing TMDB ID
                state_clone
                    .tmdb_service
                    .get_movie_details(tmdb_id)
                    .await
                    .ok()
            } else if let Some(ref imdb_id) = movie.imdb_id {
                // Try to find by IMDB ID
                match state_clone.tmdb_service.find_by_imdb_id(imdb_id).await {
                    Ok(Some(found)) => state_clone
                        .tmdb_service
                        .get_movie_details(found.id)
                        .await
                        .ok(),
                    _ => {
                        // Fallback to title search
                        let year = movie.production_year.map(|y| y as i32);
                        match state_clone
                            .tmdb_service
                            .search_movies(&movie.title, year)
                            .await
                        {
                            Ok(results) if !results.is_empty() => state_clone
                                .tmdb_service
                                .get_movie_details(results[0].id)
                                .await
                                .ok(),
                            _ => None,
                        }
                    }
                }
            } else {
                // Search by title
                let year = movie.production_year.map(|y| y as i32);
                match state_clone
                    .tmdb_service
                    .search_movies(&movie.title, year)
                    .await
                {
                    Ok(results) if !results.is_empty() => {
                        let first = &results[0];
                        state_clone
                            .tmdb_service
                            .get_movie_details(first.id)
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
                    .get_movie_credits(details.id)
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

                let update = UpdateMovie {
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

                if state_clone
                    .movie_service
                    .update(user_id, movie.id, update)
                    .await
                    .is_ok()
                {
                    enriched += 1;
                } else {
                    errors.push(format!("Failed to update: {}", movie.title));
                }
            } else {
                errors.push(format!("No TMDB data found: {}", movie.title));
            }

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

        // Send completion
        let msg = json!({
            "type": "tmdb_enrich_complete",
            "payload": {
                "total": total,
                "enriched": enriched,
                "errors": errors
            }
        });
        let _ = state_clone.ws_broadcast.send(msg.to_string());
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
