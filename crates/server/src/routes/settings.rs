use axum::{Extension, Json, extract::State};
use std::sync::Arc;

use my_movies_core::{
    models::{Claims, SettingKey, SettingUpdate, UserRole},
    services::SettingStatus,
};

use crate::AppState;

/// Get all settings status (for admin UI)
pub async fn get_settings(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<Vec<SettingStatus>>, crate::routes::AppError> {
    // Only admins can view settings
    if claims.role != UserRole::Admin {
        return Err(my_movies_core::Error::Forbidden.into());
    }

    let statuses = state.settings_service.get_status().await?;
    Ok(Json(statuses))
}

/// Update a setting
pub async fn update_setting(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    axum::extract::Path(key): axum::extract::Path<String>,
    Json(update): Json<SettingUpdate>,
) -> Result<Json<SettingStatus>, crate::routes::AppError> {
    // Only admins can update settings
    if claims.role != UserRole::Admin {
        return Err(my_movies_core::Error::Forbidden.into());
    }

    // Parse the key
    let setting_key = match key.as_str() {
        "tmdb_api_key" => SettingKey::TmdbApiKey,
        _ => return Err(my_movies_core::Error::NotFound.into()),
    };

    // Update the setting
    state.settings_service.update(setting_key, update).await?;

    // Return updated status
    let statuses = state.settings_service.get_status().await?;
    let status = statuses
        .into_iter()
        .find(|s| s.key == key)
        .ok_or(my_movies_core::Error::NotFound)?;

    Ok(Json(status))
}

/// Test TMDB API key
pub async fn test_tmdb(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<TmdbTestResult>, crate::routes::AppError> {
    // Only admins can test settings
    if claims.role != UserRole::Admin {
        return Err(my_movies_core::Error::Forbidden.into());
    }

    // Try to make a simple TMDB API call
    match state.tmdb_service.search_movies("test", None).await {
        Ok(_) => Ok(Json(TmdbTestResult {
            success: true,
            message: "TMDB API key is valid".to_string(),
        })),
        Err(e) => Ok(Json(TmdbTestResult {
            success: false,
            message: format!("TMDB API error: {}", e),
        })),
    }
}

#[derive(serde::Serialize)]
pub struct TmdbTestResult {
    pub success: bool,
    pub message: String,
}
