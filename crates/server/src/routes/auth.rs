use std::sync::Arc;

use axum::body::Body;
use axum::{
    Extension, Json,
    extract::{Multipart, Path, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use serde_json::json;
use uuid::Uuid;

use my_movies_core::models::{
    Claims, CreateUser, ForgotPasswordRequest, LoginRequest, ResetPasswordRequest,
};

use crate::{ApiError, AppState};

pub async fn register(
    State(state): State<Arc<AppState>>,
    Json(input): Json<CreateUser>,
) -> Result<impl IntoResponse, ApiError> {
    let auth_response = state.auth_service.register(input).await?;
    
    let msg = json!({ "type": "user_created", "payload": &auth_response.user });
    let _ = state.ws_broadcast.send(msg.to_string());

    Ok((StatusCode::CREATED, Json(auth_response)))
}

pub async fn login(
    State(state): State<Arc<AppState>>,
    Json(input): Json<LoginRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let auth_response = state.auth_service.login(input).await?;
    Ok((StatusCode::OK, Json(auth_response)))
}

pub async fn me(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
) -> Result<impl IntoResponse, ApiError> {
    let user = state.auth_service.get_user(claims.sub).await?;
    Ok((StatusCode::OK, Json(user)))
}

pub async fn forgot_password(
    State(state): State<Arc<AppState>>,
    Json(input): Json<ForgotPasswordRequest>,
) -> Result<impl IntoResponse, ApiError> {
    state.auth_service.request_password_reset(input).await?;
    Ok((StatusCode::OK, Json(json!({ "message": "Password reset email sent" }))))
}

pub async fn reset_password(
    State(state): State<Arc<AppState>>,
    Json(input): Json<ResetPasswordRequest>,
) -> Result<impl IntoResponse, ApiError> {
    state.auth_service.reset_password(input).await?;
    Ok((StatusCode::OK, Json(json!({ "message": "Password reset successfully" }))))
}

#[derive(serde::Deserialize)]
pub struct UpdateLanguageRequest {
    pub language: Option<String>,
}

pub async fn update_language(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<UpdateLanguageRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let user = state.auth_service.update_user_language(claims.sub, body.language).await?;
    
    let msg = json!({ "type": "user_updated", "payload": user });
    let _ = state.ws_broadcast.send(msg.to_string());

    Ok((StatusCode::OK, Json(user)))
}

#[derive(serde::Deserialize)]
pub struct UpdateIncludeAdultRequest {
    pub include_adult: bool,
}

pub async fn update_include_adult(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<UpdateIncludeAdultRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let user = state.auth_service.update_user_include_adult(claims.sub, body.include_adult).await?;
    
    let msg = json!({ "type": "user_updated", "payload": user });
    let _ = state.ws_broadcast.send(msg.to_string());

    Ok((StatusCode::OK, Json(user)))
}

#[derive(serde::Deserialize)]
pub struct UpdateThemeRequest {
    pub theme: Option<String>,
}

pub async fn update_theme(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<UpdateThemeRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let user = state.auth_service.update_user_theme(claims.sub, body.theme).await?;
    
    let msg = json!({ "type": "user_updated", "payload": user });
    let _ = state.ws_broadcast.send(msg.to_string());

    Ok((StatusCode::OK, Json(user)))
}

#[derive(serde::Deserialize)]
pub struct UpdateCardSizeRequest {
    pub card_size: Option<String>,
}

pub async fn update_card_size(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<UpdateCardSizeRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let user = state.auth_service.update_user_card_size(claims.sub, body.card_size).await?;
    
    let msg = json!({ "type": "user_updated", "payload": user });
    let _ = state.ws_broadcast.send(msg.to_string());

    Ok((StatusCode::OK, Json(user)))
}

/// Upload avatar image for current user
pub async fn upload_avatar(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, ApiError> {
    loop {
        let field = match multipart.next_field().await {
            Ok(Some(field)) => field,
            Ok(None) => break,
            Err(e) => return Err(ApiError::bad_request(format!("Failed to parse multipart form: {}", e))),
        };

        let name = field.name().unwrap_or("").to_string();

        if name == "file" {
            let content_type = field.content_type().unwrap_or("image/jpeg").to_string();

            let _extension = match content_type.as_str() {
                "image/png" => "png",
                "image/gif" => "gif",
                "image/webp" => "webp",
                "image/jpeg" | "image/jpg" => "jpg",
                _ => return Err(ApiError::bad_request(format!(
                    "Unsupported content type: {}. Supported: image/png, image/jpeg, image/gif, image/webp",
                    content_type
                ))),
            };

            let data = field.bytes().await
                .map_err(|e| ApiError::bad_request(format!("Failed to read file: {}", e)))?
                .to_vec();

            const MAX_FILE_SIZE: usize = 5 * 1024 * 1024;
            if data.len() > MAX_FILE_SIZE {
                return Err(ApiError::bad_request(format!(
                    "File too large. Maximum size is 5MB, got {} bytes",
                    data.len()
                )));
            }

            if data.len() < 8 {
                return Err(ApiError::bad_request("File too small to be a valid image"));
            }

            let user = state.auth_service.update_user_avatar_data(claims.sub, Some(data)).await?;

            let msg = json!({ "type": "user_updated", "payload": user });
            let _ = state.ws_broadcast.send(msg.to_string());

            return Ok((
                StatusCode::OK,
                Json(json!({ "message": "Avatar uploaded successfully", "user": user })),
            ));
        }
    }

    Err(ApiError::bad_request("No file provided"))
}

/// Delete avatar for current user
pub async fn delete_avatar(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
) -> Result<impl IntoResponse, ApiError> {
    let user = state.auth_service.update_user_avatar_data(claims.sub, None).await?;

    let msg = json!({ "type": "user_updated", "payload": user });
    let _ = state.ws_broadcast.send(msg.to_string());

    Ok((
        StatusCode::OK,
        Json(json!({ "message": "Avatar deleted successfully", "user": user })),
    ))
}

/// Get avatar image for a user
pub async fn get_avatar(
    State(state): State<Arc<AppState>>,
    Path(user_id): Path<Uuid>,
) -> Result<Response, ApiError> {
    let data = state.auth_service.get_user_avatar_data(user_id).await?
        .ok_or_else(|| ApiError::not_found("Avatar not found"))?;

    let content_type = detect_image_type(&data);

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, content_type)
        .body(Body::from(data))
        .map_err(|e| ApiError::internal(format!("Failed to build response: {}", e)))
}

fn detect_image_type(data: &[u8]) -> &'static str {
    if data.len() >= 8 {
        if data[0..8] == [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A] {
            return "image/png";
        }
        if data.len() >= 3 && data[0..3] == [0xFF, 0xD8, 0xFF] {
            return "image/jpeg";
        }
        if data.len() >= 6 && (data[0..6] == [0x47, 0x49, 0x46, 0x38, 0x39, 0x61]
            || data[0..6] == [0x47, 0x49, 0x46, 0x38, 0x37, 0x61])
        {
            return "image/gif";
        }
        if data.len() >= 12 && data[8..12] == [0x57, 0x45, 0x42, 0x50] {
            return "image/webp";
        }
    }
    "image/jpeg"
}
