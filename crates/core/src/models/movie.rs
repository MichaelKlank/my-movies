use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Movie struct with proper Uuid types
/// UUIDs are stored as BLOB (16 bytes) in SQLite
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Movie {
    pub id: Uuid,
    pub user_id: Uuid,

    // Identifiers
    pub collection_number: Option<String>,
    pub barcode: Option<String>,
    pub tmdb_id: Option<i64>,
    pub imdb_id: Option<String>,

    // Titles
    pub title: String,
    pub original_title: Option<String>,
    pub sort_title: Option<String>,
    pub personal_title: Option<String>,
    pub personal_sort_title: Option<String>,

    // Description
    pub description: Option<String>,
    pub tagline: Option<String>,

    // Production Info
    pub production_year: Option<i32>,
    pub release_date: Option<NaiveDate>,
    pub running_time: Option<i32>,
    pub director: Option<String>,
    pub actors: Option<String>,
    pub production_companies: Option<String>,
    pub production_countries: Option<String>,
    pub studios: Option<String>,

    // Ratings
    pub rating: Option<String>, // MPAA/FSK
    pub personal_rating: Option<f64>,

    // Media Info
    pub disc_type: Option<String>,
    pub media_type: Option<String>,
    pub discs: Option<i32>,
    pub region_codes: Option<String>,
    pub video_standard: Option<String>,
    pub aspect_ratio: Option<String>,
    pub audio_tracks: Option<String>,
    pub subtitles: Option<String>,
    pub is_3d: bool,
    pub mastered_in_4k: bool,

    // Categorization
    pub genres: Option<String>,
    pub categories: Option<String>,
    pub tags: Option<String>,
    #[sqlx(rename = "movie_group")]
    pub group: Option<String>,

    // User Status
    pub watched: bool,
    pub digital_copies: Option<String>,
    pub status: Option<String>,

    // Physical Info
    pub condition: Option<String>,
    pub slip_cover: bool,
    pub cover_type: Option<String>,
    pub edition: Option<String>,
    pub extra_features: Option<String>,

    // Financial
    pub purchase_date: Option<NaiveDate>,
    pub price: Option<f64>,
    pub currency: Option<String>,
    pub purchase_place: Option<String>,
    pub value_date: Option<NaiveDate>,
    pub value_price: Option<f64>,
    pub value_currency: Option<String>,

    // Lending
    pub lent_to: Option<String>,
    pub lent_due: Option<NaiveDate>,

    // Location
    pub location: Option<String>,

    // Notes
    pub notes: Option<String>,

    // Financial (budget/revenue from TMDB)
    pub budget: Option<i64>,
    pub revenue: Option<i64>,
    pub spoken_languages: Option<String>,

    // Poster image stored as BLOB in database
    // Skip serialization to avoid sending large BLOBs in JSON responses
    #[serde(skip)]
    #[sqlx(skip)]
    pub poster_data: Option<Vec<u8>>,

    // Timestamps
    pub added_date: Option<NaiveDate>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateMovie {
    pub barcode: Option<String>,
    pub tmdb_id: Option<i64>,
    pub title: String,
    pub original_title: Option<String>,
    pub disc_type: Option<String>,
    pub production_year: Option<i32>,
    // ... other optional fields can be added via update
}

#[derive(Debug, Deserialize, Default)]
pub struct UpdateMovie {
    pub collection_number: Option<String>,
    pub barcode: Option<String>,
    pub tmdb_id: Option<i64>,
    pub imdb_id: Option<String>,
    pub title: Option<String>,
    pub original_title: Option<String>,
    pub sort_title: Option<String>,
    pub personal_title: Option<String>,
    pub personal_sort_title: Option<String>,
    pub description: Option<String>,
    pub tagline: Option<String>,
    pub production_year: Option<i32>,
    pub release_date: Option<NaiveDate>,
    pub running_time: Option<i32>,
    pub director: Option<String>,
    pub actors: Option<String>,
    pub production_companies: Option<String>,
    pub production_countries: Option<String>,
    pub studios: Option<String>,
    pub rating: Option<String>,
    pub personal_rating: Option<f64>,
    pub disc_type: Option<String>,
    pub media_type: Option<String>,
    pub discs: Option<i32>,
    pub region_codes: Option<String>,
    pub video_standard: Option<String>,
    pub aspect_ratio: Option<String>,
    pub audio_tracks: Option<String>,
    pub subtitles: Option<String>,
    pub is_3d: Option<bool>,
    pub mastered_in_4k: Option<bool>,
    pub genres: Option<String>,
    pub categories: Option<String>,
    pub tags: Option<String>,
    pub group: Option<String>,
    pub watched: Option<bool>,
    pub digital_copies: Option<String>,
    pub status: Option<String>,
    pub condition: Option<String>,
    pub slip_cover: Option<bool>,
    pub cover_type: Option<String>,
    pub edition: Option<String>,
    pub extra_features: Option<String>,
    pub purchase_date: Option<NaiveDate>,
    pub price: Option<f64>,
    pub currency: Option<String>,
    pub purchase_place: Option<String>,
    pub value_date: Option<NaiveDate>,
    pub value_price: Option<f64>,
    pub value_currency: Option<String>,
    pub lent_to: Option<String>,
    pub lent_due: Option<NaiveDate>,
    pub location: Option<String>,
    pub notes: Option<String>,
    pub budget: Option<i64>,
    pub revenue: Option<i64>,
    pub spoken_languages: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
pub struct MovieFilter {
    pub search: Option<String>,
    pub genre: Option<String>,
    pub disc_type: Option<String>,
    pub watched: Option<bool>,
    pub year_from: Option<i32>,
    pub year_to: Option<i32>,
    pub sort_by: Option<String>,
    pub sort_order: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}
