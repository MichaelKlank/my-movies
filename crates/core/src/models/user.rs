use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "TEXT", rename_all = "lowercase")]
#[derive(Default)]
pub enum UserRole {
    Admin,
    #[default]
    User,
}

/// Database row representation
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct UserRow {
    pub id: String,
    pub username: String,
    pub email: String,
    pub password_hash: String,
    pub role: UserRole,
    pub created_at: String,
    pub updated_at: String,
    pub reset_token: Option<String>,
    pub reset_token_expires: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub role: UserRole,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(skip_serializing)]
    pub reset_token: Option<String>,
    #[serde(skip_serializing)]
    pub reset_token_expires: Option<DateTime<Utc>>,
}

impl TryFrom<UserRow> for User {
    type Error = Box<dyn std::error::Error + Send + Sync>;

    fn try_from(row: UserRow) -> Result<Self, Self::Error> {
        let reset_token_expires = row
            .reset_token_expires
            .as_ref()
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc));

        Ok(User {
            id: Uuid::parse_str(&row.id)?,
            username: row.username,
            email: row.email,
            password_hash: row.password_hash,
            role: row.role,
            created_at: DateTime::parse_from_rfc3339(&row.created_at)?.with_timezone(&Utc),
            updated_at: DateTime::parse_from_rfc3339(&row.updated_at)?.with_timezone(&Utc),
            reset_token: row.reset_token,
            reset_token_expires,
        })
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateUser {
    pub username: String,
    pub email: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub token: String,
    pub user: UserPublic,
}

#[derive(Debug, Clone, Serialize)]
pub struct UserPublic {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub role: UserRole,
    pub created_at: DateTime<Utc>,
}

impl From<User> for UserPublic {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            username: user.username,
            email: user.email,
            role: user.role,
            created_at: user.created_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: Uuid,
    pub username: String,
    pub role: UserRole,
    pub exp: i64,
    pub iat: i64,
}

#[derive(Debug, Deserialize)]
pub struct ForgotPasswordRequest {
    pub email: String,
}

#[derive(Debug, Deserialize)]
pub struct ResetPasswordRequest {
    pub token: String,
    pub password: String,
}
