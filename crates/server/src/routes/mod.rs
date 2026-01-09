pub mod auth;
pub mod collections;
pub mod import;
pub mod movies;
pub mod scan;
pub mod series;
pub mod settings;
pub mod users;
pub mod ws;

// Re-export ApiError as AppError for backward compatibility
pub use crate::error::ApiError as AppError;
