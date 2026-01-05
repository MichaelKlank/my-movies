use std::sync::Arc;

use axum::{
    Extension, Json,
    extract::{Multipart, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde_json::json;
use tokio::fs;

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

#[derive(Debug, serde::Deserialize)]
pub struct UpdateLanguageRequest {
    pub language: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
pub struct UpdateIncludeAdultRequest {
    pub include_adult: bool,
}

pub async fn update_language(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Json(input): Json<UpdateLanguageRequest>,
) -> impl IntoResponse {
    match state
        .auth_service
        .update_user_language(claims.sub, input.language)
        .await
    {
        Ok(user) => {
            // Broadcast update to WebSocket clients
            let msg = json!({
                "type": "user_updated",
                "payload": user
            });
            let _ = state.ws_broadcast.send(msg.to_string());
            (StatusCode::OK, Json(json!(user))).into_response()
        }
        Err(e) => (
            StatusCode::from_u16(e.status_code()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn update_include_adult(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Json(input): Json<UpdateIncludeAdultRequest>,
) -> impl IntoResponse {
    match state
        .auth_service
        .update_user_include_adult(claims.sub, input.include_adult)
        .await
    {
        Ok(user) => {
            // Broadcast update to WebSocket clients
            let msg = json!({
                "type": "user_updated",
                "payload": user
            });
            let _ = state.ws_broadcast.send(msg.to_string());
            (StatusCode::OK, Json(json!(user))).into_response()
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
    // Create uploads directory if it doesn't exist
    let uploads_dir = std::path::PathBuf::from("uploads/avatars");
    if let Err(e) = fs::create_dir_all(&uploads_dir).await {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": format!("Failed to create uploads directory: {}", e) })),
        )
            .into_response();
    }

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
            // Get content type to determine extension
            let content_type = field.content_type().unwrap_or("image/jpeg").to_string();

            let extension = match content_type.as_str() {
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

            // Generate unique filename using user ID
            let filename = format!("{}.{}", claims.sub, extension);
            let file_path = uploads_dir.join(&filename);

            // Save file
            if let Err(e) = fs::write(&file_path, &data).await {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({ "error": format!("Failed to save file: {}", e) })),
                )
                    .into_response();
            }

            // Update user with avatar path
            let avatar_url = format!("/uploads/avatars/{}", filename);
            match state
                .auth_service
                .update_user_avatar(claims.sub, Some(avatar_url.clone()))
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
                    // Try to clean up the uploaded file
                    let _ = fs::remove_file(&file_path).await;
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
    // Get current user to find avatar path
    let user = match state.auth_service.get_user(claims.sub).await {
        Ok(u) => u,
        Err(e) => {
            return (
                StatusCode::from_u16(e.status_code()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
                Json(json!({ "error": e.to_string() })),
            )
                .into_response();
        }
    };

    // Delete old avatar file if it exists
    if let Some(ref avatar_path) = user.avatar_path
        && avatar_path.starts_with("/uploads/avatars/")
    {
        let file_path =
            std::path::PathBuf::from(avatar_path.strip_prefix('/').unwrap_or(avatar_path));
        let _ = fs::remove_file(&file_path).await;
    }

    // Update user to remove avatar path
    match state
        .auth_service
        .update_user_avatar(claims.sub, None)
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
