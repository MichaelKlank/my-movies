use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};
use chrono::{Duration, Utc};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use uuid::Uuid;

use crate::db::DbPool;
use crate::error::{Error, Result};
use crate::models::{
    AuthResponse, Claims, CreateUser, ForgotPasswordRequest, LoginRequest, ResetPasswordRequest,
    User, UserPublic, UserRole, UserRow,
};

pub struct AuthService {
    pool: DbPool,
    jwt_secret: String,
}

impl AuthService {
    pub fn new(pool: DbPool, jwt_secret: String) -> Self {
        Self { pool, jwt_secret }
    }

    pub async fn register(&self, input: CreateUser) -> Result<AuthResponse> {
        // Check if username or email already exists
        let existing = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM users WHERE username = ? OR email = ?",
        )
        .bind(&input.username)
        .bind(&input.email)
        .fetch_one(&self.pool)
        .await?;

        if existing > 0 {
            return Err(Error::Duplicate("Username or email already exists".into()));
        }

        // Hash password
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let password_hash = argon2
            .hash_password(input.password.as_bytes(), &salt)
            .map_err(|e| Error::Internal(e.to_string()))?
            .to_string();

        let id = Uuid::new_v4();
        let now = Utc::now();

        // Check if this is the first user (make them admin)
        let user_count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM users")
            .fetch_one(&self.pool)
            .await?;

        let role = if user_count == 0 {
            UserRole::Admin
        } else {
            UserRole::User
        };

        let role_str = match role {
            UserRole::Admin => "admin",
            UserRole::User => "user",
        };

        sqlx::query(
            r#"
            INSERT INTO users (id, username, email, password_hash, role, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(id.to_string())
        .bind(&input.username)
        .bind(&input.email)
        .bind(&password_hash)
        .bind(role_str)
        .bind(now.to_rfc3339())
        .bind(now.to_rfc3339())
        .execute(&self.pool)
        .await?;

        let user = User {
            id,
            username: input.username,
            email: input.email,
            password_hash,
            role,
            created_at: now,
            updated_at: now,
            reset_token: None,
            reset_token_expires: None,
        };

        let token = self.create_token(&user)?;

        Ok(AuthResponse {
            token,
            user: user.into(),
        })
    }

    pub async fn login(&self, input: LoginRequest) -> Result<AuthResponse> {
        let row = sqlx::query_as::<_, UserRow>("SELECT * FROM users WHERE username = ?")
            .bind(&input.username)
            .fetch_optional(&self.pool)
            .await?
            .ok_or(Error::InvalidCredentials)?;

        let user: User =
            row.try_into()
                .map_err(|e: Box<dyn std::error::Error + Send + Sync>| {
                    Error::Internal(e.to_string())
                })?;

        // Verify password
        let parsed_hash =
            PasswordHash::new(&user.password_hash).map_err(|e| Error::Internal(e.to_string()))?;

        Argon2::default()
            .verify_password(input.password.as_bytes(), &parsed_hash)
            .map_err(|_| Error::InvalidCredentials)?;

        let token = self.create_token(&user)?;

        Ok(AuthResponse {
            token,
            user: user.into(),
        })
    }

    pub fn create_token(&self, user: &User) -> Result<String> {
        let now = Utc::now();
        let exp = now + Duration::days(7);

        let claims = Claims {
            sub: user.id,
            username: user.username.clone(),
            role: user.role.clone(),
            iat: now.timestamp(),
            exp: exp.timestamp(),
        };

        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.jwt_secret.as_bytes()),
        )
        .map_err(|e| Error::Internal(e.to_string()))
    }

    pub fn verify_token(&self, token: &str) -> Result<Claims> {
        decode::<Claims>(
            token,
            &DecodingKey::from_secret(self.jwt_secret.as_bytes()),
            &Validation::default(),
        )
        .map(|data| data.claims)
        .map_err(|e| match e.kind() {
            jsonwebtoken::errors::ErrorKind::ExpiredSignature => Error::TokenExpired,
            _ => Error::Auth(e.to_string()),
        })
    }

    pub async fn get_user(&self, user_id: Uuid) -> Result<UserPublic> {
        let row = sqlx::query_as::<_, UserRow>("SELECT * FROM users WHERE id = ?")
            .bind(user_id.to_string())
            .fetch_optional(&self.pool)
            .await?
            .ok_or(Error::UserNotFound)?;

        let user: User =
            row.try_into()
                .map_err(|e: Box<dyn std::error::Error + Send + Sync>| {
                    Error::Internal(e.to_string())
                })?;
        Ok(user.into())
    }

    pub async fn get_user_by_username(&self, username: &str) -> Result<User> {
        let row = sqlx::query_as::<_, UserRow>("SELECT * FROM users WHERE username = ?")
            .bind(username)
            .fetch_optional(&self.pool)
            .await?
            .ok_or(Error::UserNotFound)?;

        row.try_into()
            .map_err(|e: Box<dyn std::error::Error + Send + Sync>| Error::Internal(e.to_string()))
    }

    pub async fn request_password_reset(&self, input: ForgotPasswordRequest) -> Result<String> {
        // Find user by email
        let row = sqlx::query_as::<_, UserRow>("SELECT * FROM users WHERE email = ?")
            .bind(&input.email)
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| Error::Validation("E-Mail-Adresse nicht gefunden".to_string()))?;

        let user: User =
            row.try_into()
                .map_err(|e: Box<dyn std::error::Error + Send + Sync>| {
                    Error::Internal(e.to_string())
                })?;

        // Generate reset token
        let reset_token = Uuid::new_v4().to_string();
        let expires = Utc::now() + Duration::hours(1);

        // Hash the token before storing (for security)
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let token_hash = argon2
            .hash_password(reset_token.as_bytes(), &salt)
            .map_err(|e| Error::Internal(e.to_string()))?
            .to_string();

        // Store hashed token
        sqlx::query(
            "UPDATE users SET reset_token = ?, reset_token_expires = ?, updated_at = ? WHERE id = ?",
        )
        .bind(&token_hash)
        .bind(expires.to_rfc3339())
        .bind(Utc::now().to_rfc3339())
        .bind(user.id.to_string())
        .execute(&self.pool)
        .await?;

        // Log the reset link (in production, this would be sent via email)
        let reset_link = format!("/reset-password?token={}", reset_token);
        tracing::info!("=== PASSWORD RESET LINK ===");
        tracing::info!("User: {} ({})", user.username, user.email);
        tracing::info!("Reset link: {}", reset_link);
        tracing::info!("Token expires: {}", expires);
        tracing::info!("===========================");

        Ok("If the email exists, a reset link has been sent.".to_string())
    }

    pub async fn reset_password(&self, input: ResetPasswordRequest) -> Result<()> {
        // Find users with non-expired reset tokens
        let rows = sqlx::query_as::<_, UserRow>(
            "SELECT * FROM users WHERE reset_token IS NOT NULL AND reset_token_expires > ?",
        )
        .bind(Utc::now().to_rfc3339())
        .fetch_all(&self.pool)
        .await?;

        // Find the user whose token matches
        let mut found_user: Option<User> = None;
        for row in rows {
            let user: User =
                row.try_into()
                    .map_err(|e: Box<dyn std::error::Error + Send + Sync>| {
                        Error::Internal(e.to_string())
                    })?;
            if let Some(ref token_hash) = user.reset_token {
                let parsed_hash =
                    PasswordHash::new(token_hash).map_err(|e| Error::Internal(e.to_string()))?;

                if Argon2::default()
                    .verify_password(input.token.as_bytes(), &parsed_hash)
                    .is_ok()
                {
                    found_user = Some(user);
                    break;
                }
            }
        }

        let user = found_user.ok_or(Error::InvalidResetToken)?;

        // Hash new password
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let password_hash = argon2
            .hash_password(input.password.as_bytes(), &salt)
            .map_err(|e| Error::Internal(e.to_string()))?
            .to_string();

        // Update password and clear reset token
        sqlx::query(
            "UPDATE users SET password_hash = ?, reset_token = NULL, reset_token_expires = NULL, updated_at = ? WHERE id = ?",
        )
        .bind(&password_hash)
        .bind(Utc::now().to_rfc3339())
        .bind(user.id.to_string())
        .execute(&self.pool)
        .await?;

        tracing::info!("Password reset successful for user: {}", user.username);

        Ok(())
    }

    // ============ Admin User Management ============

    /// List all users (admin only)
    pub async fn list_all_users(&self) -> Result<Vec<UserPublic>> {
        let rows = sqlx::query_as::<_, UserRow>(
            "SELECT * FROM users ORDER BY created_at DESC"
        )
        .fetch_all(&self.pool)
        .await?;

        let mut users = Vec::new();
        for row in rows {
            let user: User = row.try_into()
                .map_err(|e: Box<dyn std::error::Error + Send + Sync>| {
                    Error::Internal(e.to_string())
                })?;
            users.push(user.into());
        }
        Ok(users)
    }

    /// Update a user's role (admin only)
    pub async fn update_user_role(&self, user_id: Uuid, new_role: UserRole) -> Result<UserPublic> {
        let role_str = match new_role {
            UserRole::Admin => "admin",
            UserRole::User => "user",
        };

        let result = sqlx::query(
            "UPDATE users SET role = ?, updated_at = ? WHERE id = ?"
        )
        .bind(role_str)
        .bind(Utc::now().to_rfc3339())
        .bind(user_id.to_string())
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(Error::UserNotFound);
        }

        self.get_user(user_id).await
    }

    /// Delete a user and all their data (admin only)
    pub async fn delete_user(&self, user_id: Uuid) -> Result<()> {
        // Delete user's movies
        sqlx::query("DELETE FROM movies WHERE user_id = ?")
            .bind(user_id.to_string())
            .execute(&self.pool)
            .await?;

        // Delete user's series
        sqlx::query("DELETE FROM series WHERE user_id = ?")
            .bind(user_id.to_string())
            .execute(&self.pool)
            .await?;

        // Delete user's collections
        sqlx::query("DELETE FROM collection_items WHERE collection_id IN (SELECT id FROM collections WHERE user_id = ?)")
            .bind(user_id.to_string())
            .execute(&self.pool)
            .await?;

        sqlx::query("DELETE FROM collections WHERE user_id = ?")
            .bind(user_id.to_string())
            .execute(&self.pool)
            .await?;

        // Delete the user
        let result = sqlx::query("DELETE FROM users WHERE id = ?")
            .bind(user_id.to_string())
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(Error::UserNotFound);
        }

        Ok(())
    }

    /// Admin can set a new password for a user
    pub async fn admin_set_password(&self, user_id: Uuid, new_password: &str) -> Result<()> {
        // Hash new password
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let password_hash = argon2
            .hash_password(new_password.as_bytes(), &salt)
            .map_err(|e| Error::Internal(e.to_string()))?
            .to_string();

        let result = sqlx::query(
            "UPDATE users SET password_hash = ?, reset_token = NULL, reset_token_expires = NULL, updated_at = ? WHERE id = ?"
        )
        .bind(&password_hash)
        .bind(Utc::now().to_rfc3339())
        .bind(user_id.to_string())
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(Error::UserNotFound);
        }

        Ok(())
    }
}
