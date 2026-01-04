use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A collection represents a box set or bundle of movies/series
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Collection {
    pub id: Uuid,
    pub user_id: Uuid,
    
    // Identifiers
    pub collection_number: Option<String>,
    pub barcode: Option<String>,
    
    // Titles
    pub title: String,
    pub sort_title: Option<String>,
    pub personal_title: Option<String>,
    
    // Description
    pub description: Option<String>,
    
    // Media Info
    pub disc_type: Option<String>,
    pub discs: Option<i32>,
    pub region_codes: Option<String>,
    
    // Categorization
    pub genres: Option<String>,
    pub categories: Option<String>,
    pub tags: Option<String>,
    
    // Physical Info
    pub condition: Option<String>,
    pub slip_cover: bool,
    pub cover_type: Option<String>,
    pub edition: Option<String>,
    
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
    
    // Timestamps
    pub added_date: Option<NaiveDate>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Links movies/series to a collection
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct CollectionItem {
    pub id: Uuid,
    pub collection_id: Uuid,
    pub item_type: CollectionItemType,
    pub movie_id: Option<Uuid>,
    pub series_id: Option<Uuid>,
    pub position: i32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "TEXT", rename_all = "lowercase")]
pub enum CollectionItemType {
    Movie,
    Series,
}

#[derive(Debug, Deserialize)]
pub struct CreateCollection {
    pub barcode: Option<String>,
    pub title: String,
    pub description: Option<String>,
    pub disc_type: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AddCollectionItem {
    pub item_type: CollectionItemType,
    pub movie_id: Option<Uuid>,
    pub series_id: Option<Uuid>,
    pub position: Option<i32>,
}

#[derive(Debug, Deserialize, Default)]
pub struct UpdateCollection {
    pub title: Option<String>,
    pub sort_title: Option<String>,
    pub description: Option<String>,
    pub location: Option<String>,
    pub notes: Option<String>,
    // ... add other fields as needed
}

#[derive(Debug, Deserialize, Default)]
pub struct CollectionFilter {
    pub search: Option<String>,
    pub sort_by: Option<String>,
    pub sort_order: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}
