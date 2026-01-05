use axum::{Extension, Json, extract::State};
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

use my_movies_core::models::{Claims, UserPublic, UserRole};

use crate::AppState;

/// List all users (admin only)
pub async fn list_users(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<Vec<UserPublic>>, crate::routes::AppError> {
    // Only admins can list users
    if claims.role != UserRole::Admin {
        return Err(my_movies_core::Error::Forbidden.into());
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
) -> Result<Json<UserPublic>, crate::routes::AppError> {
    // Only admins can update roles
    if claims.role != UserRole::Admin {
        return Err(my_movies_core::Error::Forbidden.into());
    }

    let user_id = Uuid::parse_str(&user_id)
        .map_err(|_| my_movies_core::Error::Validation("Invalid user ID".into()))?;

    // Prevent admin from demoting themselves
    if user_id == claims.sub && body.role != "admin" {
        return Err(my_movies_core::Error::Validation(
            "Du kannst deine eigene Admin-Rolle nicht entfernen".into(),
        )
        .into());
    }

    let new_role = match body.role.as_str() {
        "admin" => UserRole::Admin,
        "user" => UserRole::User,
        _ => return Err(my_movies_core::Error::Validation("Invalid role".into()).into()),
    };

    let user = state
        .auth_service
        .update_user_role(user_id, new_role)
        .await?;

    // Broadcast update to WebSocket clients
    let msg = json!({
        "type": "user_updated",
        "payload": user
    });
    let _ = state.ws_broadcast.send(msg.to_string());

    Ok(Json(user))
}

/// Delete a user (admin only)
pub async fn delete_user(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    axum::extract::Path(user_id): axum::extract::Path<String>,
) -> Result<Json<DeleteResponse>, crate::routes::AppError> {
    // Only admins can delete users
    if claims.role != UserRole::Admin {
        return Err(my_movies_core::Error::Forbidden.into());
    }

    let user_id = Uuid::parse_str(&user_id)
        .map_err(|_| my_movies_core::Error::Validation("Invalid user ID".into()))?;

    // Prevent admin from deleting themselves
    if user_id == claims.sub {
        return Err(my_movies_core::Error::Validation(
            "Du kannst dich selbst nicht löschen".into(),
        )
        .into());
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
) -> Result<Json<PasswordResetResponse>, crate::routes::AppError> {
    // Only admins can set passwords
    if claims.role != UserRole::Admin {
        return Err(my_movies_core::Error::Forbidden.into());
    }

    let user_id = Uuid::parse_str(&user_id)
        .map_err(|_| my_movies_core::Error::Validation("Invalid user ID".into()))?;

    if body.password.len() < 4 {
        return Err(my_movies_core::Error::Validation(
            "Passwort muss mindestens 4 Zeichen lang sein".into(),
        )
        .into());
    }

    state
        .auth_service
        .admin_set_password(user_id, &body.password)
        .await?;
    Ok(Json(PasswordResetResponse {
        message: "Password updated successfully".to_string(),
    }))
}

#[derive(serde::Serialize)]
pub struct PasswordResetResponse {
    pub message: String,
}
