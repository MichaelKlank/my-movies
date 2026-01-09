use axum::{Extension, Json, extract::State};
use std::sync::Arc;

use my_movies_core::{
    models::{Claims, SettingKey, SettingUpdate, UserRole},
    services::SettingStatus,
};

use crate::{ApiError, AppState};

/// Get all settings status (for admin UI)
pub async fn get_settings(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<Vec<SettingStatus>>, ApiError> {
    if claims.role != UserRole::Admin {
        return Err(ApiError::from(my_movies_core::Error::Forbidden));
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
) -> Result<Json<SettingStatus>, ApiError> {
    if claims.role != UserRole::Admin {
        return Err(ApiError::from(my_movies_core::Error::Forbidden));
    }

    let setting_key = match key.as_str() {
        "tmdb_api_key" => SettingKey::TmdbApiKey,
        _ => return Err(ApiError::not_found("Setting not found")),
    };

    // Update runtime services directly (no restart needed!)
    match setting_key {
        SettingKey::TmdbApiKey => {
            state.tmdb_service.set_api_key(update.value.clone());
        }
    }

    state.settings_service.update(setting_key, update).await?;

    let statuses = state.settings_service.get_status().await?;
    let status = statuses
        .into_iter()
        .find(|s| s.key == key)
        .ok_or_else(|| ApiError::not_found("Setting not found"))?;

    Ok(Json(status))
}

/// Test TMDB API key
pub async fn test_tmdb(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<TmdbTestResult>, ApiError> {
    if claims.role != UserRole::Admin {
        return Err(ApiError::from(my_movies_core::Error::Forbidden));
    }

    match state.tmdb_service.search_movies("test", None, None, false).await {
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
