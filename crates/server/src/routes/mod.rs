pub mod auth;
pub mod collections;
pub mod import;
pub mod movies;
pub mod scan;
pub mod series;
pub mod settings;
pub mod users;
pub mod ws;

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};

/// Application error type for routes
pub struct AppError(pub my_movies_core::Error);

impl From<my_movies_core::Error> for AppError {
    fn from(err: my_movies_core::Error) -> Self {
        AppError(err)
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = StatusCode::from_u16(self.0.status_code()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        let body = Json(serde_json::json!({ "error": self.0.to_string() }));
        (status, body).into_response()
    }
}
