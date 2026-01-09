use axum::{Extension, Json, extract::State};
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

use my_movies_core::models::{Claims, UserPublic, UserRole};

use crate::{ApiError, AppState};

/// List all users (admin only)
pub async fn list_users(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<Vec<UserPublic>>, ApiError> {
    if claims.role != UserRole::Admin {
        return Err(ApiError::from(my_movies_core::Error::Forbidden));
    }

    let users = state.auth_service.list_all_users().await?;
    Ok(Json(users))
}

#[derive(serde::Deserialize)]
pub struct UpdateRoleRequest {
    pub role: String,
}

/// Update a user's role (admin only)
pub async fn update_user_role(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    axum::extract::Path(user_id): axum::extract::Path<String>,
    Json(body): Json<UpdateRoleRequest>,
) -> Result<Json<UserPublic>, ApiError> {
    if claims.role != UserRole::Admin {
        return Err(ApiError::from(my_movies_core::Error::Forbidden));
    }

    let user_id = Uuid::parse_str(&user_id)
        .map_err(|_| ApiError::bad_request("Invalid user ID"))?;

    if user_id == claims.sub && body.role != "admin" {
        return Err(ApiError::bad_request("Du kannst deine eigene Admin-Rolle nicht entfernen"));
    }

    let new_role = match body.role.as_str() {
        "admin" => UserRole::Admin,
        "user" => UserRole::User,
        _ => return Err(ApiError::bad_request("Invalid role")),
    };

    let user = state.auth_service.update_user_role(user_id, new_role).await?;

    let msg = json!({ "type": "user_updated", "payload": user });
    let _ = state.ws_broadcast.send(msg.to_string());

    Ok(Json(user))
}

/// Delete a user (admin only)
pub async fn delete_user(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    axum::extract::Path(user_id): axum::extract::Path<String>,
) -> Result<Json<DeleteResponse>, ApiError> {
    if claims.role != UserRole::Admin {
        return Err(ApiError::from(my_movies_core::Error::Forbidden));
    }

    let user_id = Uuid::parse_str(&user_id)
        .map_err(|_| ApiError::bad_request("Invalid user ID"))?;

    if user_id == claims.sub {
        return Err(ApiError::bad_request("Du kannst dich selbst nicht löschen"));
    }

    state.auth_service.delete_user(user_id).await?;
    Ok(Json(DeleteResponse {
        message: "User deleted successfully".to_string(),
    }))
}

#[derive(serde::Serialize)]
pub struct DeleteResponse {
    pub message: String,
}

#[derive(serde::Deserialize)]
pub struct SetPasswordRequest {
    pub password: String,
}

/// Admin set password for a user
pub async fn admin_set_password(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    axum::extract::Path(user_id): axum::extract::Path<String>,
    Json(body): Json<SetPasswordRequest>,
) -> Result<Json<PasswordResetResponse>, ApiError> {
    if claims.role != UserRole::Admin {
        return Err(ApiError::from(my_movies_core::Error::Forbidden));
    }

    let user_id = Uuid::parse_str(&user_id)
        .map_err(|_| ApiError::bad_request("Invalid user ID"))?;

    if body.password.len() < 4 {
        return Err(ApiError::bad_request("Passwort muss mindestens 4 Zeichen lang sein"));
    }

    state.auth_service.admin_set_password(user_id, &body.password).await?;
    Ok(Json(PasswordResetResponse {
        message: "Password updated successfully".to_string(),
    }))
}

#[derive(serde::Serialize)]
pub struct PasswordResetResponse {
    pub message: String,
}

#[derive(serde::Deserialize)]
pub struct AdminCreateUserRequest {
    pub username: String,
    pub email: String,
    pub password: Option<String>,
}

#[derive(serde::Serialize)]
pub struct AdminCreateUserResponse {
    pub user: UserPublic,
    pub reset_token: Option<String>,
}

/// Admin create a new user
pub async fn admin_create_user(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<AdminCreateUserRequest>,
) -> Result<Json<AdminCreateUserResponse>, ApiError> {
    if claims.role != UserRole::Admin {
        return Err(ApiError::from(my_movies_core::Error::Forbidden));
    }

    if body.username.len() < 2 {
        return Err(ApiError::bad_request("Username muss mindestens 2 Zeichen lang sein"));
    }

    if !body.email.contains('@') {
        return Err(ApiError::bad_request("Ungültige E-Mail-Adresse"));
    }

    if let Some(ref pwd) = body.password && pwd.len() < 4 {
        return Err(ApiError::bad_request("Passwort muss mindestens 4 Zeichen lang sein"));
    }

    let (user, reset_token) = state
        .auth_service
        .admin_create_user(body.username, body.email, body.password)
        .await?;

    let msg = json!({ "type": "user_created", "payload": user });
    let _ = state.ws_broadcast.send(msg.to_string());

    Ok(Json(AdminCreateUserResponse { user, reset_token }))
}
