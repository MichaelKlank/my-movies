use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::AppState;

#[derive(Debug, Deserialize)]
pub struct BarcodeRequest {
    pub barcode: String,
}

#[derive(Debug, Serialize)]
pub struct BarcodeResponse {
    pub barcode: String,
    pub title: Option<String>,
    pub vendor: Option<String>,
    pub tmdb_results: Vec<TmdbSearchResult>,
}

#[derive(Debug, Serialize)]
pub struct TmdbSearchResult {
    pub id: i64,
    pub title: String,
    pub year: Option<String>,
    pub poster_url: Option<String>,
    pub poster_path: Option<String>,
}

pub async fn lookup_barcode(
    State(state): State<Arc<AppState>>,
    Json(input): Json<BarcodeRequest>,
) -> impl IntoResponse {
    // First, lookup the barcode in EAN database
    let ean_result = state.ean_service.lookup(&input.barcode).await;

    let title = match &ean_result {
        Ok(Some(result)) => Some(result.title.clone()),
        _ => None,
    };

    // If we found a title, search TMDB
    let tmdb_results = if let Some(ref t) = title {
        match state.tmdb_service.search_movies(t, None).await {
            Ok(results) => results
                .into_iter()
                .take(5)
                .map(|m| TmdbSearchResult {
                    id: m.id,
                    title: m.title,
                    year: m
                        .release_date
                        .and_then(|d| d.get(..4).map(|s| s.to_string())),
                    poster_url: m
                        .poster_path
                        .as_ref()
                        .map(|p| my_movies_core::services::TmdbService::poster_url(p, "w200")),
                    poster_path: m.poster_path,
                })
                .collect(),
            Err(_) => Vec::new(),
        }
    } else {
        Vec::new()
    };

    let response = BarcodeResponse {
        barcode: input.barcode,
        title,
        vendor: ean_result.ok().flatten().and_then(|r| r.vendor),
        tmdb_results,
    };

    (StatusCode::OK, Json(json!(response))).into_response()
}

#[derive(Debug, Deserialize)]
pub struct TmdbSearchQuery {
    pub query: String,
    pub year: Option<i32>,
}

pub async fn search_tmdb_movies(
    State(state): State<Arc<AppState>>,
    Query(params): Query<TmdbSearchQuery>,
) -> impl IntoResponse {
    match state
        .tmdb_service
        .search_movies(&params.query, params.year)
        .await
    {
        Ok(results) => {
            let results: Vec<TmdbSearchResult> = results
                .into_iter()
                .take(20)
                .map(|m| TmdbSearchResult {
                    id: m.id,
                    title: m.title,
                    year: m
                        .release_date
                        .and_then(|d| d.get(..4).map(|s| s.to_string())),
                    poster_url: m
                        .poster_path
                        .as_ref()
                        .map(|p| my_movies_core::services::TmdbService::poster_url(p, "w200")),
                    poster_path: m.poster_path,
                })
                .collect();
            (StatusCode::OK, Json(json!(results))).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn search_tmdb_tv(
    State(state): State<Arc<AppState>>,
    Query(params): Query<TmdbSearchQuery>,
) -> impl IntoResponse {
    match state.tmdb_service.search_tv(&params.query).await {
        Ok(results) => {
            let results: Vec<TmdbSearchResult> = results
                .into_iter()
                .take(20)
                .map(|m| TmdbSearchResult {
                    id: m.id,
                    title: m.name,
                    year: m
                        .first_air_date
                        .and_then(|d| d.get(..4).map(|s| s.to_string())),
                    poster_url: m
                        .poster_path
                        .as_ref()
                        .map(|p| my_movies_core::services::TmdbService::poster_url(p, "w200")),
                    poster_path: m.poster_path,
                })
                .collect();
            (StatusCode::OK, Json(json!(results))).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn get_tmdb_movie(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    match state.tmdb_service.get_movie_details(id).await {
        Ok(details) => (StatusCode::OK, Json(json!(details))).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn get_tmdb_tv(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    match state.tmdb_service.get_tv_details(id).await {
        Ok(details) => (StatusCode::OK, Json(json!(details))).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}
