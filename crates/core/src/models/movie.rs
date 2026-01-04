use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Database row representation with String IDs for SQLite TEXT columns
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct MovieRow {
    pub id: String,
    pub user_id: String,

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
    pub rating: Option<String>,
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
    pub poster_path: Option<String>,

    // Timestamps
    pub added_date: Option<NaiveDate>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Application-level Movie struct with proper Uuid types
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    pub poster_path: Option<String>,

    // Timestamps
    pub added_date: Option<NaiveDate>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<MovieRow> for Movie {
    fn from(row: MovieRow) -> Self {
        Self {
            id: Uuid::parse_str(&row.id).unwrap_or_else(|_| Uuid::nil()),
            user_id: Uuid::parse_str(&row.user_id).unwrap_or_else(|_| Uuid::nil()),
            collection_number: row.collection_number,
            barcode: row.barcode,
            tmdb_id: row.tmdb_id,
            imdb_id: row.imdb_id,
            title: row.title,
            original_title: row.original_title,
            sort_title: row.sort_title,
            personal_title: row.personal_title,
            personal_sort_title: row.personal_sort_title,
            description: row.description,
            tagline: row.tagline,
            production_year: row.production_year,
            release_date: row.release_date,
            running_time: row.running_time,
            director: row.director,
            actors: row.actors,
            production_companies: row.production_companies,
            production_countries: row.production_countries,
            studios: row.studios,
            rating: row.rating,
            personal_rating: row.personal_rating,
            disc_type: row.disc_type,
            media_type: row.media_type,
            discs: row.discs,
            region_codes: row.region_codes,
            video_standard: row.video_standard,
            aspect_ratio: row.aspect_ratio,
            audio_tracks: row.audio_tracks,
            subtitles: row.subtitles,
            is_3d: row.is_3d,
            mastered_in_4k: row.mastered_in_4k,
            genres: row.genres,
            categories: row.categories,
            tags: row.tags,
            group: row.group,
            watched: row.watched,
            digital_copies: row.digital_copies,
            status: row.status,
            condition: row.condition,
            slip_cover: row.slip_cover,
            cover_type: row.cover_type,
            edition: row.edition,
            extra_features: row.extra_features,
            purchase_date: row.purchase_date,
            price: row.price,
            currency: row.currency,
            purchase_place: row.purchase_place,
            value_date: row.value_date,
            value_price: row.value_price,
            value_currency: row.value_currency,
            lent_to: row.lent_to,
            lent_due: row.lent_due,
            location: row.location,
            notes: row.notes,
            budget: row.budget,
            revenue: row.revenue,
            spoken_languages: row.spoken_languages,
            poster_path: row.poster_path,
            added_date: row.added_date,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateMovie {
    pub barcode: Option<String>,
    pub tmdb_id: Option<i64>,
    pub title: String,
    pub original_title: Option<String>,
    pub disc_type: Option<String>,
    pub production_year: Option<i32>,
    pub poster_path: Option<String>,
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
    pub poster_path: Option<String>,
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
