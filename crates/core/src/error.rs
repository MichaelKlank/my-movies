use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Authentication failed: {0}")]
    Auth(String),

    #[error("Invalid credentials")]
    InvalidCredentials,

    #[error("Token expired")]
    TokenExpired,

    #[error("Invalid or expired reset token")]
    InvalidResetToken,

    #[error("User not found")]
    UserNotFound,

    #[error("Item not found")]
    NotFound,

    #[error("Permission denied")]
    Forbidden,

    #[error("Duplicate entry: {0}")]
    Duplicate(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("External API error: {0}")]
    ExternalApi(String),

    #[error("CSV import error: {0}")]
    CsvImport(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl Error {
    pub fn status_code(&self) -> u16 {
        match self {
            Error::InvalidCredentials | Error::TokenExpired => 401,
            Error::Forbidden => 403,
            Error::NotFound | Error::UserNotFound => 404,
            Error::Duplicate(_) | Error::Validation(_) | Error::InvalidResetToken => 400,
            _ => 500,
        }
    }
}
