use std::sync::Arc;

use axum::{
    Extension, Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde_json::json;
use uuid::Uuid;

use my_movies_core::models::{Claims, CreateSeries, SeriesFilter, UpdateSeries};

use crate::AppState;

pub async fn list(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Query(filter): Query<SeriesFilter>,
) -> impl IntoResponse {
    match state.series_service.list(claims.sub, filter).await {
        Ok(series) => (StatusCode::OK, Json(json!(series))).into_response(),
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
    match state.series_service.get_by_id(claims.sub, id).await {
        Ok(series) => (StatusCode::OK, Json(json!(series))).into_response(),
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
    Json(input): Json<CreateSeries>,
) -> impl IntoResponse {
    match state.series_service.create(claims.sub, input).await {
        Ok(series) => {
            let msg = json!({ "type": "series_added", "payload": series });
            let _ = state.ws_broadcast.send(msg.to_string());
            (StatusCode::CREATED, Json(json!(series))).into_response()
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
    Json(input): Json<UpdateSeries>,
) -> impl IntoResponse {
    match state.series_service.update(claims.sub, id, input).await {
        Ok(series) => {
            let msg = json!({ "type": "series_updated", "payload": series });
            let _ = state.ws_broadcast.send(msg.to_string());
            (StatusCode::OK, Json(json!(series))).into_response()
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
    match state.series_service.delete(claims.sub, id).await {
        Ok(_) => {
            let msg = json!({ "type": "series_deleted", "payload": { "id": id } });
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
