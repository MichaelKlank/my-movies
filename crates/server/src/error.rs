use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

/// Unified API error type for consistent error responses
pub struct ApiError {
    pub status: StatusCode,
    pub message: String,
}

impl ApiError {
    pub fn new(status: StatusCode, message: impl Into<String>) -> Self {
        Self {
            status,
            message: message.into(),
        }
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, message)
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(StatusCode::NOT_FOUND, message)
    }

    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, message)
    }

    pub fn unauthorized(message: impl Into<String>) -> Self {
        Self::new(StatusCode::UNAUTHORIZED, message)
    }

    pub fn conflict(message: impl Into<String>) -> Self {
        Self::new(StatusCode::CONFLICT, message)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status, Json(json!({ "error": self.message }))).into_response()
    }
}

/// Convert from core service errors to API errors
impl From<my_movies_core::error::Error> for ApiError {
    fn from(err: my_movies_core::error::Error) -> Self {
        let status = StatusCode::from_u16(err.status_code())
            .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        Self::new(status, err.to_string())
    }
}

/// Result type alias for API handlers
pub type ApiResult<T> = Result<T, ApiError>;
