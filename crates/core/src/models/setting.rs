use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Setting {
    pub key: String,
    pub value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingUpdate {
    pub value: String,
}

/// Settings that can be configured via the UI
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingKey {
    TmdbApiKey,
}

impl SettingKey {
    pub fn as_str(&self) -> &'static str {
        match self {
            SettingKey::TmdbApiKey => "tmdb_api_key",
        }
    }

    pub fn env_var(&self) -> &'static str {
        match self {
            SettingKey::TmdbApiKey => "TMDB_API_KEY",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            SettingKey::TmdbApiKey => "API key for The Movie Database (themoviedb.org)",
        }
    }
}
