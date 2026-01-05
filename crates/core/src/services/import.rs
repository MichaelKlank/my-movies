use chrono::Utc;
use csv::ReaderBuilder;
use std::io::Read;
use uuid::Uuid;

use crate::db::DbPool;
use crate::error::{Error, Result};

pub struct ImportService {
    pool: DbPool,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct CsvMovieRecord {
    #[serde(rename = "Collection Number")]
    pub collection_number: Option<String>,
    #[serde(rename = "Collection Item Type")]
    pub collection_item_type: Option<String>,
    pub title: Option<String>,
    #[serde(rename = "Original Title")]
    pub original_title: Option<String>,
    #[serde(rename = "Sort Title")]
    pub sort_title: Option<String>,
    pub barcode: Option<String>,
    #[serde(rename = "Disc Type")]
    pub disc_type: Option<String>,
    #[serde(rename = "Production Year")]
    pub production_year: Option<String>,
    #[serde(rename = "IMDB Id")]
    pub imdb_id: Option<String>,
    #[serde(rename = "Running Time")]
    pub running_time: Option<String>,
    pub rating: Option<String>,
    pub description: Option<String>,
    pub director: Option<String>,
    pub actors: Option<String>,
    #[serde(rename = "Audio Tracks")]
    pub audio_tracks: Option<String>,
    pub subtitles: Option<String>,
    pub categories: Option<String>,
    #[serde(rename = "Digital Copies")]
    pub digital_copies: Option<String>,
    #[serde(rename = "Region Codes")]
    pub region_codes: Option<String>,
    pub discs: Option<String>,
    pub genres: Option<String>,
    pub watched: Option<String>,
    pub tagline: Option<String>,
    pub budget: Option<String>,
    pub revenue: Option<String>,
    pub network: Option<String>,
    #[serde(rename = "3D")]
    pub is_3d: Option<String>,
    pub status: Option<String>,
    #[serde(rename = "Production Companies")]
    pub production_companies: Option<String>,
    #[serde(rename = "Production Countries")]
    pub production_countries: Option<String>,
    #[serde(rename = "Spoken Languages")]
    pub spoken_languages: Option<String>,
    pub studios: Option<String>,
    #[serde(rename = "First Aired")]
    pub first_aired: Option<String>,
    #[serde(rename = "Mastered in 4K")]
    pub mastered_in_4k: Option<String>,
    #[serde(rename = "Media Type")]
    pub media_type: Option<String>,
    #[serde(rename = "Slip Cover")]
    pub slip_cover: Option<String>,
    #[serde(rename = "Aspect Ratio")]
    pub aspect_ratio: Option<String>,
    #[serde(rename = "Video Standard")]
    pub video_standard: Option<String>,
    #[serde(rename = "Cover Type")]
    pub cover_type: Option<String>,
    #[serde(rename = "Release Date")]
    pub release_date: Option<String>,
    #[serde(rename = "Par. Rating")]
    pub parental_rating: Option<String>,
    #[serde(rename = "Extra Features")]
    pub extra_features: Option<String>,
    pub edition: Option<String>,
    #[serde(rename = "Air Time")]
    pub air_time: Option<String>,
    pub group: Option<String>,
    #[serde(rename = "Personal Title")]
    pub personal_title: Option<String>,
    #[serde(rename = "Personal Sort Title")]
    pub personal_sort_title: Option<String>,
    pub notes: Option<String>,
    pub tags: Option<String>,
    #[serde(rename = "Personal Rating")]
    pub personal_rating: Option<String>,
    #[serde(rename = "Type")]
    pub item_type: Option<String>,
    pub condition: Option<String>,
    #[serde(rename = "Added Date")]
    pub added_date: Option<String>,
    #[serde(rename = "Lent To")]
    pub lent_to: Option<String>,
    #[serde(rename = "Lent Due")]
    pub lent_due: Option<String>,
    pub location: Option<String>,
    #[serde(rename = "Purchase Date")]
    pub purchase_date: Option<String>,
    pub price: Option<String>,
    pub currency: Option<String>,
    #[serde(rename = "Purchase Place")]
    pub purchase_place: Option<String>,
    #[serde(rename = "Value Date")]
    pub value_date: Option<String>,
    #[serde(rename = "Value Price")]
    pub value_price: Option<String>,
    #[serde(rename = "Value Currency")]
    pub value_currency: Option<String>,
    #[serde(rename = "Episodes Count")]
    pub episodes_count: Option<String>,
}

#[derive(Debug)]
pub struct ImportResult {
    pub movies_imported: u32,
    pub series_imported: u32,
    pub collections_imported: u32,
    pub errors: Vec<String>,
}

impl ImportService {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    pub async fn import_csv<R: Read>(&self, user_id: Uuid, reader: R) -> Result<ImportResult> {
        let mut csv_reader = ReaderBuilder::new()
            .has_headers(true)
            .flexible(true)
            .from_reader(reader);

        let mut result = ImportResult {
            movies_imported: 0,
            series_imported: 0,
            collections_imported: 0,
            errors: Vec::new(),
        };

        for (index, record_result) in csv_reader.deserialize::<CsvMovieRecord>().enumerate() {
            let row_num = index + 2; // +2 because of 0-indexing and header row

            match record_result {
                Ok(record) => {
                    if let Err(e) = self.import_record(user_id, &record).await {
                        result.errors.push(format!("Row {}: {}", row_num, e));
                    } else {
                        // Determine type and increment counter
                        match record.item_type.as_deref() {
                            Some("Series") => result.series_imported += 1,
                            Some("Collection") => result.collections_imported += 1,
                            _ => result.movies_imported += 1,
                        }
                    }
                }
                Err(e) => {
                    result
                        .errors
                        .push(format!("Row {}: Parse error - {}", row_num, e));
                }
            }
        }

        Ok(result)
    }

    async fn import_record(&self, user_id: Uuid, record: &CsvMovieRecord) -> Result<()> {
        let title = record
            .title
            .as_ref()
            .ok_or_else(|| Error::CsvImport("Missing title".into()))?;

        let id = Uuid::new_v4();
        let now = Utc::now();

        // Determine if this is a movie, series, or collection based on item_type
        match record.item_type.as_deref() {
            Some("Series") => self.import_series(user_id, id, &now, record, title).await,
            Some("Collection") => {
                self.import_collection(user_id, id, &now, record, title)
                    .await
            }
            _ => self.import_movie(user_id, id, &now, record, title).await,
        }
    }

    async fn import_movie(
        &self,
        user_id: Uuid,
        id: Uuid,
        now: &chrono::DateTime<Utc>,
        record: &CsvMovieRecord,
        title: &str,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO movies (
                id, user_id, collection_number, barcode, title, original_title, sort_title,
                personal_title, personal_sort_title, description, tagline,
                production_year, release_date, running_time, director, actors,
                production_companies, production_countries, studios,
                rating, personal_rating, disc_type, media_type, discs, region_codes,
                video_standard, aspect_ratio, audio_tracks, subtitles,
                is_3d, mastered_in_4k, genres, categories, tags, movie_group,
                watched, digital_copies, status, condition, slip_cover, cover_type,
                edition, extra_features, purchase_date, price, currency, purchase_place,
                value_date, value_price, value_currency, lent_to, lent_due, location,
                notes, budget, revenue, spoken_languages, imdb_id, added_date,
                created_at, updated_at
            )
            VALUES (
                ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?
            )
            "#,
        )
        .bind(id)
        .bind(user_id)
        .bind(&record.collection_number)
        .bind(&record.barcode)
        .bind(title)
        .bind(&record.original_title)
        .bind(&record.sort_title)
        .bind(&record.personal_title)
        .bind(&record.personal_sort_title)
        .bind(&record.description)
        .bind(&record.tagline)
        .bind(Self::parse_int(&record.production_year))
        .bind(&record.release_date)
        .bind(Self::parse_int(&record.running_time))
        .bind(&record.director)
        .bind(&record.actors)
        .bind(&record.production_companies)
        .bind(&record.production_countries)
        .bind(&record.studios)
        .bind(&record.rating)
        .bind(Self::parse_float(&record.personal_rating))
        .bind(&record.disc_type)
        .bind(&record.media_type)
        .bind(Self::parse_int(&record.discs))
        .bind(&record.region_codes)
        .bind(&record.video_standard)
        .bind(&record.aspect_ratio)
        .bind(&record.audio_tracks)
        .bind(&record.subtitles)
        .bind(Self::parse_bool(&record.is_3d))
        .bind(Self::parse_bool(&record.mastered_in_4k))
        .bind(&record.genres)
        .bind(&record.categories)
        .bind(&record.tags)
        .bind(&record.group)
        .bind(Self::parse_bool(&record.watched))
        .bind(&record.digital_copies)
        .bind(&record.status)
        .bind(&record.condition)
        .bind(Self::parse_bool(&record.slip_cover))
        .bind(&record.cover_type)
        .bind(&record.edition)
        .bind(&record.extra_features)
        .bind(&record.purchase_date)
        .bind(Self::parse_float(&record.price))
        .bind(&record.currency)
        .bind(&record.purchase_place)
        .bind(&record.value_date)
        .bind(Self::parse_float(&record.value_price))
        .bind(&record.value_currency)
        .bind(&record.lent_to)
        .bind(&record.lent_due)
        .bind(&record.location)
        .bind(&record.notes)
        .bind(Self::parse_int(&record.budget))
        .bind(Self::parse_int(&record.revenue))
        .bind(&record.spoken_languages)
        .bind(&record.imdb_id)
        .bind(&record.added_date)
        .bind(now.to_rfc3339())
        .bind(now.to_rfc3339())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn import_series(
        &self,
        user_id: Uuid,
        id: Uuid,
        now: &chrono::DateTime<Utc>,
        record: &CsvMovieRecord,
        title: &str,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO series (
                id, user_id, collection_number, barcode, title, original_title, sort_title,
                description, tagline, production_year, first_aired, network, status,
                episodes_count, running_time, actors, rating, personal_rating,
                disc_type, media_type, discs, region_codes, video_standard, aspect_ratio,
                audio_tracks, subtitles, is_3d, mastered_in_4k, genres, categories,
                tags, series_group, watched, digital_copies, condition, slip_cover,
                cover_type, edition, purchase_date, price, currency, purchase_place,
                value_date, value_price, value_currency, lent_to, lent_due, location,
                notes, spoken_languages, imdb_id, added_date, created_at, updated_at
            )
            VALUES (
                ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?
            )
            "#,
        )
        .bind(id)
        .bind(user_id)
        .bind(&record.collection_number)
        .bind(&record.barcode)
        .bind(title)
        .bind(&record.original_title)
        .bind(&record.sort_title)
        .bind(&record.description)
        .bind(&record.tagline)
        .bind(Self::parse_int(&record.production_year))
        .bind(&record.first_aired)
        .bind(&record.network)
        .bind(&record.status)
        .bind(Self::parse_int(&record.episodes_count))
        .bind(Self::parse_int(&record.running_time))
        .bind(&record.actors)
        .bind(&record.rating)
        .bind(Self::parse_float(&record.personal_rating))
        .bind(&record.disc_type)
        .bind(&record.media_type)
        .bind(Self::parse_int(&record.discs))
        .bind(&record.region_codes)
        .bind(&record.video_standard)
        .bind(&record.aspect_ratio)
        .bind(&record.audio_tracks)
        .bind(&record.subtitles)
        .bind(Self::parse_bool(&record.is_3d))
        .bind(Self::parse_bool(&record.mastered_in_4k))
        .bind(&record.genres)
        .bind(&record.categories)
        .bind(&record.tags)
        .bind(&record.group)
        .bind(Self::parse_bool(&record.watched))
        .bind(&record.digital_copies)
        .bind(&record.condition)
        .bind(Self::parse_bool(&record.slip_cover))
        .bind(&record.cover_type)
        .bind(&record.edition)
        .bind(&record.purchase_date)
        .bind(Self::parse_float(&record.price))
        .bind(&record.currency)
        .bind(&record.purchase_place)
        .bind(&record.value_date)
        .bind(Self::parse_float(&record.value_price))
        .bind(&record.value_currency)
        .bind(&record.lent_to)
        .bind(&record.lent_due)
        .bind(&record.location)
        .bind(&record.notes)
        .bind(&record.spoken_languages)
        .bind(&record.imdb_id)
        .bind(&record.added_date)
        .bind(now.to_rfc3339())
        .bind(now.to_rfc3339())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn import_collection(
        &self,
        user_id: Uuid,
        id: Uuid,
        now: &chrono::DateTime<Utc>,
        record: &CsvMovieRecord,
        title: &str,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO collections (
                id, user_id, collection_number, barcode, title, sort_title,
                description, disc_type, discs, region_codes, genres, categories,
                tags, condition, slip_cover, cover_type, edition,
                purchase_date, price, currency, purchase_place,
                value_date, value_price, value_currency, lent_to, lent_due,
                location, notes, added_date, created_at, updated_at
            )
            VALUES (
                ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?
            )
            "#,
        )
        .bind(id)
        .bind(user_id)
        .bind(&record.collection_number)
        .bind(&record.barcode)
        .bind(title)
        .bind(&record.sort_title)
        .bind(&record.description)
        .bind(&record.disc_type)
        .bind(Self::parse_int(&record.discs))
        .bind(&record.region_codes)
        .bind(&record.genres)
        .bind(&record.categories)
        .bind(&record.tags)
        .bind(&record.condition)
        .bind(Self::parse_bool(&record.slip_cover))
        .bind(&record.cover_type)
        .bind(&record.edition)
        .bind(&record.purchase_date)
        .bind(Self::parse_float(&record.price))
        .bind(&record.currency)
        .bind(&record.purchase_place)
        .bind(&record.value_date)
        .bind(Self::parse_float(&record.value_price))
        .bind(&record.value_currency)
        .bind(&record.lent_to)
        .bind(&record.lent_due)
        .bind(&record.location)
        .bind(&record.notes)
        .bind(&record.added_date)
        .bind(now.to_rfc3339())
        .bind(now.to_rfc3339())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    fn parse_int(s: &Option<String>) -> Option<i64> {
        s.as_ref().and_then(|v| v.parse().ok())
    }

    fn parse_float(s: &Option<String>) -> Option<f64> {
        s.as_ref().and_then(|v| v.replace(',', ".").parse().ok())
    }

    fn parse_bool(s: &Option<String>) -> bool {
        s.as_ref()
            .map(|v| matches!(v.to_lowercase().as_str(), "true" | "yes" | "1" | "ja"))
            .unwrap_or(false)
    }
}
