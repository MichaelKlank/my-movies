use std::sync::Arc;

use axum::{Extension, Json, extract::State, http::StatusCode, response::IntoResponse};
use serde_json::json;

use my_movies_core::models::{
    Claims, CreateUser, ForgotPasswordRequest, LoginRequest, ResetPasswordRequest,
};

use crate::AppState;

pub async fn register(
    State(state): State<Arc<AppState>>,
    Json(input): Json<CreateUser>,
) -> impl IntoResponse {
    match state.auth_service.register(input).await {
        Ok(response) => (StatusCode::CREATED, Json(json!(response))).into_response(),
        Err(e) => (
            StatusCode::from_u16(e.status_code()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn login(
    State(state): State<Arc<AppState>>,
    Json(input): Json<LoginRequest>,
) -> impl IntoResponse {
    match state.auth_service.login(input).await {
        Ok(response) => (StatusCode::OK, Json(json!(response))).into_response(),
        Err(e) => (
            StatusCode::from_u16(e.status_code()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn me(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
) -> impl IntoResponse {
    match state.auth_service.get_user(claims.sub).await {
        Ok(user) => (StatusCode::OK, Json(json!(user))).into_response(),
        Err(e) => (
            StatusCode::from_u16(e.status_code()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn forgot_password(
    State(state): State<Arc<AppState>>,
    Json(input): Json<ForgotPasswordRequest>,
) -> impl IntoResponse {
    match state.auth_service.request_password_reset(input).await {
        Ok(message) => (StatusCode::OK, Json(json!({ "message": message }))).into_response(),
        Err(e) => (
            StatusCode::from_u16(e.status_code()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn reset_password(
    State(state): State<Arc<AppState>>,
    Json(input): Json<ResetPasswordRequest>,
) -> impl IntoResponse {
    match state.auth_service.reset_password(input).await {
        Ok(()) => (
            StatusCode::OK,
            Json(json!({ "message": "Password reset successful" })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::from_u16(e.status_code()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}
