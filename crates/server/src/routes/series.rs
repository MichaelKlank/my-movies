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

use crate::{ApiError, AppState};

pub async fn list(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Query(filter): Query<SeriesFilter>,
) -> Result<impl IntoResponse, ApiError> {
    let series = state.series_service.list(claims.sub, filter).await?;
    Ok((StatusCode::OK, Json(json!(series))))
}

pub async fn get(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    let series = state.series_service.get_by_id(claims.sub, id).await?;
    Ok((StatusCode::OK, Json(json!(series))))
}

pub async fn create(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Json(input): Json<CreateSeries>,
) -> Result<impl IntoResponse, ApiError> {
    let series = state.series_service.create(claims.sub, input).await?;
    let msg = json!({ "type": "series_added", "payload": series });
    let _ = state.ws_broadcast.send(msg.to_string());
    Ok((StatusCode::CREATED, Json(json!(series))))
}

pub async fn update(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<Uuid>,
    Json(input): Json<UpdateSeries>,
) -> Result<impl IntoResponse, ApiError> {
    let series = state.series_service.update(claims.sub, id, input).await?;
    let msg = json!({ "type": "series_updated", "payload": series });
    let _ = state.ws_broadcast.send(msg.to_string());
    Ok((StatusCode::OK, Json(json!(series))))
}

pub async fn delete(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    state.series_service.delete(claims.sub, id).await?;
    let msg = json!({ "type": "series_deleted", "payload": { "id": id } });
    let _ = state.ws_broadcast.send(msg.to_string());
    Ok(StatusCode::NO_CONTENT)
}
