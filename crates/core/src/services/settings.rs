use sqlx::SqlitePool;

use crate::models::{Setting, SettingKey, SettingUpdate};
use crate::{Error, Result};

pub struct SettingsService {
    pool: SqlitePool,
}

impl SettingsService {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Get a setting value, checking environment variable first, then database
    pub async fn get(&self, key: SettingKey) -> Result<Option<String>> {
        // First check environment variable (highest priority)
        if let Ok(value) = std::env::var(key.env_var())
            && !value.is_empty()
        {
            return Ok(Some(value));
        }

        // Then check database
        let setting = sqlx::query_as::<_, Setting>("SELECT * FROM settings WHERE key = ?")
            .bind(key.as_str())
            .fetch_optional(&self.pool)
            .await?;

        Ok(setting.and_then(|s| {
            if s.value.is_empty() {
                None
            } else {
                Some(s.value)
            }
        }))
    }

    /// Get a setting, returning error if not configured
    pub async fn get_required(&self, key: SettingKey) -> Result<String> {
        self.get(key).await?.ok_or_else(|| {
            Error::Configuration(format!(
                "{} is not configured. Set it via environment variable {} or in Settings.",
                key.as_str(),
                key.env_var()
            ))
        })
    }

    /// Get all settings (for admin UI)
    pub async fn list(&self) -> Result<Vec<Setting>> {
        let settings = sqlx::query_as::<_, Setting>("SELECT * FROM settings ORDER BY key")
            .fetch_all(&self.pool)
            .await?;

        Ok(settings)
    }

    /// Update a setting in the database
    pub async fn update(&self, key: SettingKey, update: SettingUpdate) -> Result<Setting> {
        sqlx::query(
            r#"
            INSERT INTO settings (key, value, description, updated_at)
            VALUES (?, ?, ?, CURRENT_TIMESTAMP)
            ON CONFLICT(key) DO UPDATE SET
                value = excluded.value,
                updated_at = CURRENT_TIMESTAMP
            "#,
        )
        .bind(key.as_str())
        .bind(&update.value)
        .bind(key.description())
        .execute(&self.pool)
        .await?;

        let setting = sqlx::query_as::<_, Setting>("SELECT * FROM settings WHERE key = ?")
            .bind(key.as_str())
            .fetch_one(&self.pool)
            .await?;

        Ok(setting)
    }

    /// Check if a setting is configured (either via env or database)
    pub async fn is_configured(&self, key: SettingKey) -> bool {
        self.get(key).await.ok().flatten().is_some()
    }

    /// Get settings status for the UI (shows which are configured)
    pub async fn get_status(&self) -> Result<Vec<SettingStatus>> {
        let mut statuses = Vec::new();

        for key in [SettingKey::TmdbApiKey] {
            let env_value = std::env::var(key.env_var()).ok();
            let db_setting = sqlx::query_as::<_, Setting>("SELECT * FROM settings WHERE key = ?")
                .bind(key.as_str())
                .fetch_optional(&self.pool)
                .await?;

            let (source, is_configured) = if env_value.as_ref().is_some_and(|v| !v.is_empty()) {
                (SettingSource::Environment, true)
            } else if db_setting.as_ref().is_some_and(|s| !s.value.is_empty()) {
                (SettingSource::Database, true)
            } else {
                (SettingSource::None, false)
            };

            statuses.push(SettingStatus {
                key: key.as_str().to_string(),
                env_var: key.env_var().to_string(),
                description: key.description().to_string(),
                is_configured,
                source,
                // Don't expose the actual value for security
                value_preview: if is_configured {
                    Some("••••••••".to_string())
                } else {
                    None
                },
            });
        }

        Ok(statuses)
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SettingStatus {
    pub key: String,
    pub env_var: String,
    pub description: String,
    pub is_configured: bool,
    pub source: SettingSource,
    pub value_preview: Option<String>,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SettingSource {
    Environment,
    Database,
    None,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::create_test_db;

    async fn setup() -> SettingsService {
        let pool = create_test_db().await;
        SettingsService::new(pool)
    }

    #[tokio::test]
    async fn test_update_and_get_setting() {
        let service = setup().await;

        // Update a setting
        let setting = service
            .update(
                SettingKey::TmdbApiKey,
                SettingUpdate {
                    value: "test-api-key".to_string(),
                },
            )
            .await
            .unwrap();

        assert_eq!(setting.value, "test-api-key");

        // Get the setting
        let value = service.get(SettingKey::TmdbApiKey).await.unwrap();
        assert_eq!(value, Some("test-api-key".to_string()));
    }

    #[tokio::test]
    async fn test_get_nonexistent_setting() {
        let service = setup().await;

        // Ensure no env var is set for this test
        // SAFETY: We're in a single-threaded test context
        unsafe { std::env::remove_var("TMDB_API_KEY") };

        let value = service.get(SettingKey::TmdbApiKey).await.unwrap();
        assert!(value.is_none());
    }

    #[tokio::test]
    async fn test_get_required_fails_when_not_configured() {
        let service = setup().await;

        // Ensure no env var is set
        // SAFETY: We're in a single-threaded test context
        unsafe { std::env::remove_var("TMDB_API_KEY") };

        let result = service.get_required(SettingKey::TmdbApiKey).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            Error::Configuration(_) => {}
            e => panic!("Expected Configuration error, got {:?}", e),
        }
    }

    #[tokio::test]
    async fn test_get_required_succeeds_when_configured() {
        let service = setup().await;

        service
            .update(
                SettingKey::TmdbApiKey,
                SettingUpdate {
                    value: "test-key".to_string(),
                },
            )
            .await
            .unwrap();

        let value = service.get_required(SettingKey::TmdbApiKey).await.unwrap();
        assert_eq!(value, "test-key");
    }

    #[tokio::test]
    async fn test_list_settings() {
        let service = setup().await;

        // Add a setting
        service
            .update(
                SettingKey::TmdbApiKey,
                SettingUpdate {
                    value: "test-key".to_string(),
                },
            )
            .await
            .unwrap();

        let settings = service.list().await.unwrap();
        assert!(!settings.is_empty());
    }

    #[tokio::test]
    async fn test_is_configured_false_when_not_set() {
        let service = setup().await;
        // SAFETY: We're in a single-threaded test context
        unsafe { std::env::remove_var("TMDB_API_KEY") };

        let is_configured = service.is_configured(SettingKey::TmdbApiKey).await;
        assert!(!is_configured);
    }

    #[tokio::test]
    async fn test_is_configured_true_when_set_in_db() {
        let service = setup().await;
        // SAFETY: We're in a single-threaded test context
        unsafe { std::env::remove_var("TMDB_API_KEY") };

        service
            .update(
                SettingKey::TmdbApiKey,
                SettingUpdate {
                    value: "test-key".to_string(),
                },
            )
            .await
            .unwrap();

        let is_configured = service.is_configured(SettingKey::TmdbApiKey).await;
        assert!(is_configured);
    }

    #[tokio::test]
    async fn test_get_status() {
        let service = setup().await;
        // SAFETY: We're in a single-threaded test context
        unsafe { std::env::remove_var("TMDB_API_KEY") };

        let statuses = service.get_status().await.unwrap();
        assert!(!statuses.is_empty());

        let tmdb_status = statuses.iter().find(|s| s.key == "tmdb_api_key").unwrap();
        assert!(!tmdb_status.is_configured);
        assert_eq!(tmdb_status.source, SettingSource::None);
    }

    #[tokio::test]
    async fn test_update_overwrites_existing() {
        let service = setup().await;

        service
            .update(
                SettingKey::TmdbApiKey,
                SettingUpdate {
                    value: "first-key".to_string(),
                },
            )
            .await
            .unwrap();

        service
            .update(
                SettingKey::TmdbApiKey,
                SettingUpdate {
                    value: "second-key".to_string(),
                },
            )
            .await
            .unwrap();

        let value = service.get(SettingKey::TmdbApiKey).await.unwrap();
        assert_eq!(value, Some("second-key".to_string()));
    }
}
