use std::sync::Arc;

use axum::{
    Extension, Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde_json::json;
use uuid::Uuid;

use my_movies_core::models::{
    AddCollectionItem, Claims, CollectionFilter, CreateCollection, UpdateCollection,
};

use crate::{ApiError, AppState};

pub async fn list(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Query(filter): Query<CollectionFilter>,
) -> Result<impl IntoResponse, ApiError> {
    let collections = state.collection_service.list(claims.sub, filter).await?;
    Ok((StatusCode::OK, Json(json!(collections))))
}

pub async fn get(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    let collection = state.collection_service.get_by_id(claims.sub, id).await?;
    Ok((StatusCode::OK, Json(json!(collection))))
}

pub async fn create(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Json(input): Json<CreateCollection>,
) -> Result<impl IntoResponse, ApiError> {
    let collection = state.collection_service.create(claims.sub, input).await?;
    Ok((StatusCode::CREATED, Json(json!(collection))))
}

pub async fn update(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<Uuid>,
    Json(input): Json<UpdateCollection>,
) -> Result<impl IntoResponse, ApiError> {
    let collection = state
        .collection_service
        .update(claims.sub, id, input)
        .await?;
    Ok((StatusCode::OK, Json(json!(collection))))
}

pub async fn delete(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    state.collection_service.delete(claims.sub, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn get_items(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    let items = state.collection_service.get_items(claims.sub, id).await?;
    Ok((StatusCode::OK, Json(json!(items))))
}

pub async fn add_item(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<Uuid>,
    Json(input): Json<AddCollectionItem>,
) -> Result<impl IntoResponse, ApiError> {
    let item = state
        .collection_service
        .add_item(claims.sub, id, input)
        .await?;
    Ok((StatusCode::CREATED, Json(json!(item))))
}

pub async fn remove_item(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Path((id, item_id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, ApiError> {
    state
        .collection_service
        .remove_item(claims.sub, id, item_id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
