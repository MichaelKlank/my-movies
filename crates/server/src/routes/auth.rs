use std::sync::Arc;

use axum::body::Body;
use axum::{
    Extension, Json,
    extract::{Multipart, Path, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use serde_json::json;

use my_movies_core::models::{
    Claims, CreateUser, ForgotPasswordRequest, LoginRequest, ResetPasswordRequest,
};
use uuid::Uuid;

use crate::AppState;

pub async fn register(
    State(state): State<Arc<AppState>>,
    Json(input): Json<CreateUser>,
) -> impl IntoResponse {
    match state.auth_service.register(input).await {
        Ok(auth_response) => {
            // Broadcast new user to WebSocket clients (for admin user list)
            let msg = json!({
                "type": "user_created",
                "payload": &auth_response.user
            });
            let _ = state.ws_broadcast.send(msg.to_string());

            (StatusCode::CREATED, Json(auth_response)).into_response()
        }
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
        Ok(auth_response) => (StatusCode::OK, Json(auth_response)).into_response(),
        Err(e) => (
            StatusCode::from_u16(e.status_code()).unwrap_or(StatusCode::UNAUTHORIZED),
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
        Ok(user) => (StatusCode::OK, Json(user)).into_response(),
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
        Ok(_) => (
            StatusCode::OK,
            Json(json!({ "message": "Password reset email sent" })),
        )
            .into_response(),
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
        Ok(_) => (
            StatusCode::OK,
            Json(json!({ "message": "Password reset successfully" })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::from_u16(e.status_code()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

#[derive(serde::Deserialize)]
pub struct UpdateLanguageRequest {
    pub language: Option<String>,
}

pub async fn update_language(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<UpdateLanguageRequest>,
) -> impl IntoResponse {
    match state
        .auth_service
        .update_user_language(claims.sub, body.language)
        .await
    {
        Ok(user) => {
            // Broadcast update to WebSocket clients
            let msg = json!({
                "type": "user_updated",
                "payload": user
            });
            let _ = state.ws_broadcast.send(msg.to_string());

            (StatusCode::OK, Json(user)).into_response()
        }
        Err(e) => (
            StatusCode::from_u16(e.status_code()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

#[derive(serde::Deserialize)]
pub struct UpdateIncludeAdultRequest {
    pub include_adult: bool,
}

pub async fn update_include_adult(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<UpdateIncludeAdultRequest>,
) -> impl IntoResponse {
    match state
        .auth_service
        .update_user_include_adult(claims.sub, body.include_adult)
        .await
    {
        Ok(user) => {
            // Broadcast update to WebSocket clients
            let msg = json!({
                "type": "user_updated",
                "payload": user
            });
            let _ = state.ws_broadcast.send(msg.to_string());

            (StatusCode::OK, Json(user)).into_response()
        }
        Err(e) => (
            StatusCode::from_u16(e.status_code()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

/// Upload avatar image for current user
pub async fn upload_avatar(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    mut multipart: Multipart,
) -> impl IntoResponse {
    // Process multipart upload
    loop {
        let field_result = multipart.next_field().await;
        let field = match field_result {
            Ok(Some(field)) => field,
            Ok(None) => break, // No more fields
            Err(e) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(json!({ "error": format!("Failed to parse multipart form: {}", e) })),
                )
                    .into_response();
            }
        };

        let name = field.name().unwrap_or("").to_string();

        if name == "file" {
            // Get content type
            let content_type = field.content_type().unwrap_or("image/jpeg").to_string();

            let _extension = match content_type.as_str() {
                "image/png" => "png",
                "image/gif" => "gif",
                "image/webp" => "webp",
                "image/jpeg" | "image/jpg" => "jpg",
                _ => {
                    return (
                        StatusCode::BAD_REQUEST,
                        Json(json!({
                            "error": format!("Unsupported content type: {}. Supported types: image/png, image/jpeg, image/gif, image/webp", content_type)
                        })),
                    )
                        .into_response();
                }
            };

            // Read file data using bytes() - same approach as upload_poster
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

            // Validate file size (max 5MB)
            const MAX_FILE_SIZE: usize = 5 * 1024 * 1024; // 5MB
            if data.len() > MAX_FILE_SIZE {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(json!({ "error": format!("File too large. Maximum size is 5MB, got {} bytes", data.len()) })),
                )
                    .into_response();
            }

            // Validate it's actually an image (basic check)
            if data.len() < 8 {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(json!({ "error": "File too small to be a valid image" })),
                )
                    .into_response();
            }

            // Store image data directly in database
            match state
                .auth_service
                .update_user_avatar_data(claims.sub, Some(data))
                .await
            {
                Ok(user) => {
                    // Broadcast update to WebSocket clients
                    let msg = json!({
                        "type": "user_updated",
                        "payload": user
                    });
                    let _ = state.ws_broadcast.send(msg.to_string());

                    return (
                        StatusCode::OK,
                        Json(json!({
                            "message": "Avatar uploaded successfully",
                            "user": user
                        })),
                    )
                        .into_response();
                }
                Err(e) => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({ "error": format!("Failed to update user: {}", e) })),
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

/// Delete avatar for current user
pub async fn delete_avatar(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
) -> impl IntoResponse {
    // Remove avatar data from database
    match state
        .auth_service
        .update_user_avatar_data(claims.sub, None)
        .await
    {
        Ok(user) => {
            // Broadcast update to WebSocket clients
            let msg = json!({
                "type": "user_updated",
                "payload": user
            });
            let _ = state.ws_broadcast.send(msg.to_string());

            (
                StatusCode::OK,
                Json(json!({
                    "message": "Avatar deleted successfully",
                    "user": user
                })),
            )
                .into_response()
        }
        Err(e) => (
            StatusCode::from_u16(e.status_code()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

/// Get avatar image for a user
pub async fn get_avatar(
    State(state): State<Arc<AppState>>,
    Path(user_id): Path<Uuid>,
) -> impl IntoResponse {
    match state.auth_service.get_user_avatar_data(user_id).await {
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
            Json(json!({ "error": "Avatar not found" })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::from_u16(e.status_code()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}
