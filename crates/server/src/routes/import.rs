use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use axum::extract::{Multipart, Query};
use axum::{Extension, Json, extract::State, http::StatusCode, response::IntoResponse};
use serde_json::json;
use tokio::time::{Duration, sleep};

use my_movies_core::models::{Claims, MovieFilter};

use crate::{ApiError, AppState};
use crate::routes::movies::{TmdbRefreshResult, refresh_movie_tmdb_internal};

/// Global state for TMDB enrichment
static ENRICH_CANCELLED: AtomicBool = AtomicBool::new(false);
static ENRICH_RUNNING: AtomicBool = AtomicBool::new(false);
static ENRICH_TOTAL: AtomicU32 = AtomicU32::new(0);
static ENRICH_CURRENT: AtomicU32 = AtomicU32::new(0);
static ENRICH_UPDATED: AtomicU32 = AtomicU32::new(0);
static ENRICH_ERRORS: AtomicU32 = AtomicU32::new(0);

pub async fn import_csv(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, ApiError> {
    while let Some(field) = multipart.next_field().await.unwrap_or(None) {
        let name = field.name().unwrap_or("").to_string();

        if name == "file" {
            let data = field.bytes().await
                .map_err(|e| ApiError::bad_request(format!("Failed to read file: {}", e)))?;

            let cursor = std::io::Cursor::new(data);
            let result = state.import_service.import_csv(claims.sub, cursor).await?;

            let msg = json!({ "type": "collection_imported" });
            let _ = state.ws_broadcast.send(msg.to_string());

            return Ok((
                StatusCode::OK,
                Json(json!({
                    "movies_imported": result.movies_imported,
                    "series_imported": result.series_imported,
                    "collections_imported": result.collections_imported,
                    "errors": result.errors
                })),
            ));
        }
    }

    Err(ApiError::bad_request("No file provided"))
}

#[derive(Debug, serde::Deserialize)]
pub struct EnrichTmdbQuery {
    #[serde(default)]
    pub force: bool,
}

/// Enrich all movies with TMDB data
pub async fn enrich_movies_tmdb(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Query(params): Query<EnrichTmdbQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let filter = MovieFilter {
        limit: Some(10000),
        ..Default::default()
    };

    let movies = state.movie_service.list(claims.sub, filter).await?;

    let movies_to_enrich: Vec<_> = if params.force {
        tracing::info!("Force mode: processing all {} movies", movies.len());
        movies
    } else {
        let movies_with_poster: std::collections::HashSet<_> = state
            .movie_service
            .get_movie_ids_with_poster(claims.sub)
            .await
            .unwrap_or_default()
            .into_iter()
            .collect();

        tracing::info!(
            "Enrichment filter: {} total movies, {} have TMDB ID, {} have poster",
            movies.len(),
            movies.iter().filter(|m| m.tmdb_id.is_some()).count(),
            movies_with_poster.len()
        );

        let filtered: Vec<_> = movies
            .into_iter()
            .filter(|m| m.tmdb_id.is_none() || !movies_with_poster.contains(&m.id))
            .collect();

        tracing::info!(
            "After filter: {} movies need enrichment",
            filtered.len()
        );

        filtered
    };

    let total = movies_to_enrich.len();

    if total == 0 {
        return Ok((
            StatusCode::OK,
            Json(json!({
                "message": "All movies already have TMDB data",
                "total": 0
            })),
        ));
    }

    if ENRICH_RUNNING.load(Ordering::SeqCst) {
        return Err(ApiError::conflict("TMDB enrichment is already running"));
    }

    // Reset state
    ENRICH_CANCELLED.store(false, Ordering::SeqCst);
    ENRICH_RUNNING.store(true, Ordering::SeqCst);
    ENRICH_TOTAL.store(total as u32, Ordering::SeqCst);
    ENRICH_CURRENT.store(0, Ordering::SeqCst);
    ENRICH_UPDATED.store(0, Ordering::SeqCst);
    ENRICH_ERRORS.store(0, Ordering::SeqCst);

    let msg = json!({ "type": "tmdb_enrich_started", "payload": { "total": total } });
    let _ = state.ws_broadcast.send(msg.to_string());

    let user = state.auth_service.get_user(claims.sub).await?;
    let language = user.language.clone();
    let include_adult = user.include_adult;

    let state_clone = state.clone();
    let user_id = claims.sub;

    tokio::spawn(async move {
        run_enrichment(state_clone, user_id, movies_to_enrich, language, include_adult, params.force).await;
    });

    Ok((
        StatusCode::ACCEPTED,
        Json(json!({ "message": "TMDB enrichment started", "total": total })),
    ))
}

async fn run_enrichment(
    state: Arc<AppState>,
    user_id: uuid::Uuid,
    movies: Vec<my_movies_core::models::Movie>,
    language: Option<String>,
    include_adult: bool,
    force: bool,
) {
    let total = movies.len();
    let mut enriched = 0;
    let mut errors: Vec<String> = Vec::new();
    let lang = language.as_deref();
    let mut cancelled = false;

    for (index, movie) in movies.iter().enumerate() {
        if ENRICH_CANCELLED.load(Ordering::SeqCst) {
            cancelled = true;
            let msg = json!({
                "type": "tmdb_enrich_cancelled",
                "payload": { "current": index, "total": total, "enriched": enriched }
            });
            let _ = state.ws_broadcast.send(msg.to_string());
            break;
        }

        match refresh_movie_tmdb_internal(&state, user_id, movie, lang, include_adult, force).await {
            TmdbRefreshResult::Success(_) => enriched += 1,
            TmdbRefreshResult::NotFound(msg) | TmdbRefreshResult::Error(msg) => errors.push(msg),
        }

        ENRICH_CURRENT.store((index + 1) as u32, Ordering::SeqCst);
        ENRICH_UPDATED.store(enriched as u32, Ordering::SeqCst);
        ENRICH_ERRORS.store(errors.len() as u32, Ordering::SeqCst);

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
            let _ = state.ws_broadcast.send(msg.to_string());
        }

        sleep(Duration::from_millis(250)).await;
    }

    ENRICH_RUNNING.store(false, Ordering::SeqCst);

    if !cancelled {
        let msg = json!({
            "type": "tmdb_enrich_complete",
            "payload": { "total": total, "enriched": enriched, "errors": errors }
        });
        let _ = state.ws_broadcast.send(msg.to_string());
    }
}

/// Cancel the running TMDB enrichment
pub async fn cancel_enrich_tmdb(
    Extension(claims): Extension<Claims>,
) -> Result<impl IntoResponse, ApiError> {
    if claims.role != my_movies_core::models::UserRole::Admin {
        return Err(ApiError::from(my_movies_core::Error::Forbidden));
    }

    if !ENRICH_RUNNING.load(Ordering::SeqCst) {
        return Ok((StatusCode::OK, Json(json!({ "message": "No enrichment running" }))));
    }

    ENRICH_CANCELLED.store(true, Ordering::SeqCst);
    Ok((StatusCode::OK, Json(json!({ "message": "Cancellation requested" }))))
}

/// Get current enrichment status
pub async fn get_enrich_status() -> impl IntoResponse {
    let is_running = ENRICH_RUNNING.load(Ordering::SeqCst);

    if is_running {
        (StatusCode::OK, Json(json!({
            "is_running": true,
            "total": ENRICH_TOTAL.load(Ordering::SeqCst),
            "current": ENRICH_CURRENT.load(Ordering::SeqCst),
            "updated": ENRICH_UPDATED.load(Ordering::SeqCst),
            "errors_count": ENRICH_ERRORS.load(Ordering::SeqCst)
        })))
    } else {
        (StatusCode::OK, Json(json!({ "is_running": false })))
    }
}
