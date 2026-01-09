use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::sync::RwLock;
use urlencoding;

const TMDB_BASE_URL: &str = "https://api.themoviedb.org/3";

pub struct TmdbService {
    client: reqwest::Client,
    api_key: RwLock<String>,
}

#[derive(Debug, Deserialize)]
pub struct TmdbSearchResult {
    pub results: Vec<TmdbMovie>,
    pub total_results: i32,
    pub total_pages: i32,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TmdbMovie {
    pub id: i64,
    pub title: String,
    pub original_title: Option<String>,
    pub overview: Option<String>,
    pub poster_path: Option<String>,
    pub backdrop_path: Option<String>,
    pub release_date: Option<String>,
    pub vote_average: Option<f64>,
    pub vote_count: Option<i32>,
    pub popularity: Option<f64>,
    pub genre_ids: Option<Vec<i32>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TmdbMovieDetails {
    pub id: i64,
    pub title: String,
    pub original_title: Option<String>,
    pub tagline: Option<String>,
    pub overview: Option<String>,
    pub poster_path: Option<String>,
    pub backdrop_path: Option<String>,
    pub release_date: Option<String>,
    pub runtime: Option<i32>,
    pub vote_average: Option<f64>,
    pub budget: Option<i64>,
    pub revenue: Option<i64>,
    pub imdb_id: Option<String>,
    pub genres: Option<Vec<TmdbGenre>>,
    pub production_companies: Option<Vec<TmdbCompany>>,
    pub production_countries: Option<Vec<TmdbCountry>>,
    pub spoken_languages: Option<Vec<TmdbLanguage>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TmdbGenre {
    pub id: i32,
    pub name: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TmdbCompany {
    pub id: i32,
    pub name: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TmdbCountry {
    pub iso_3166_1: String,
    pub name: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TmdbLanguage {
    pub iso_639_1: String,
    pub name: String,
    pub english_name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TmdbCredits {
    pub cast: Vec<TmdbCast>,
    pub crew: Vec<TmdbCrew>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TmdbCast {
    pub id: i32,
    pub name: String,
    pub character: Option<String>,
    pub order: Option<i32>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TmdbCrew {
    pub id: i32,
    pub name: String,
    pub job: String,
    pub department: String,
}

// Collection types
#[derive(Debug, Deserialize, Serialize)]
pub struct TmdbCollectionSearchResult {
    pub results: Vec<TmdbCollectionOverview>,
    pub total_results: i32,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TmdbCollectionOverview {
    pub id: i64,
    pub name: String,
    pub poster_path: Option<String>,
    pub backdrop_path: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TmdbCollection {
    pub id: i64,
    pub name: String,
    pub overview: Option<String>,
    pub poster_path: Option<String>,
    pub backdrop_path: Option<String>,
    pub parts: Vec<TmdbMovie>,
}

// TV Series types
#[derive(Debug, Deserialize)]
pub struct TmdbTvSearchResult {
    pub results: Vec<TmdbTvShow>,
    pub total_results: i32,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TmdbTvShow {
    pub id: i64,
    pub name: String,
    pub original_name: Option<String>,
    pub overview: Option<String>,
    pub poster_path: Option<String>,
    pub first_air_date: Option<String>,
    pub vote_average: Option<f64>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TmdbTvDetails {
    pub id: i64,
    pub name: String,
    pub original_name: Option<String>,
    pub tagline: Option<String>,
    pub overview: Option<String>,
    pub poster_path: Option<String>,
    pub first_air_date: Option<String>,
    pub last_air_date: Option<String>,
    pub number_of_episodes: Option<i32>,
    pub number_of_seasons: Option<i32>,
    pub episode_run_time: Option<Vec<i32>>,
    pub status: Option<String>,
    pub networks: Option<Vec<TmdbNetwork>>,
    pub genres: Option<Vec<TmdbGenre>>,
    pub created_by: Option<Vec<TmdbCreator>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TmdbCreator {
    pub id: i64,
    pub name: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TmdbNetwork {
    pub id: i32,
    pub name: String,
}

impl TmdbService {
    pub fn new(api_key: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key: RwLock::new(api_key),
        }
    }

    /// Update the API key at runtime (e.g., when settings are changed)
    pub fn set_api_key(&self, api_key: String) {
        if let Ok(mut key) = self.api_key.write() {
            *key = api_key;
        }
    }

    /// Get current API key, returns error if not configured
    fn get_api_key(&self) -> Result<String> {
        let key = self.api_key.read().map(|k| k.clone()).unwrap_or_default();
        if key.is_empty() {
            return Err(Error::ExternalApi(
                "TMDB API key not configured. Please set it in Settings.".to_string(),
            ));
        }
        Ok(key)
    }

    pub async fn search_movies(
        &self,
        query: &str,
        year: Option<i32>,
        language: Option<&str>,
        include_adult: bool,
    ) -> Result<Vec<TmdbMovie>> {
        self.search_movies_paginated(query, year, language, include_adult, 1)
            .await
    }

    /// Search for movies with pagination support
    /// max_pages limits how many pages to fetch (each page has ~20 results)
    pub async fn search_movies_paginated(
        &self,
        query: &str,
        year: Option<i32>,
        language: Option<&str>,
        include_adult: bool,
        max_pages: u32,
    ) -> Result<Vec<TmdbMovie>> {
        let lang = language.unwrap_or("de-DE");
        let api_key = self.get_api_key()?;
        let mut all_results = Vec::new();

        for page in 1..=max_pages {
            let mut url = format!(
                "{}/search/movie?api_key={}&query={}&language={}&include_adult={}&page={}",
                TMDB_BASE_URL,
                api_key,
                urlencoding::encode(query),
                lang,
                include_adult,
                page
            );

            if let Some(y) = year {
                url.push_str(&format!("&year={}", y));
            }

            let response = self
                .client
                .get(&url)
                .send()
                .await
                .map_err(|e| Error::ExternalApi(e.to_string()))?;

            if !response.status().is_success() {
                return Err(Error::ExternalApi(format!(
                    "TMDB API error: {}",
                    response.status()
                )));
            }

            let result: TmdbSearchResult = response
                .json()
                .await
                .map_err(|e| Error::ExternalApi(e.to_string()))?;

            let results_count = result.results.len();
            all_results.extend(result.results);

            // Stop if we've reached the last page (less than 20 results)
            if results_count < 20 {
                break;
            }
        }

        Ok(all_results)
    }

    /// Find a movie by external ID (e.g., IMDB ID)
    pub async fn find_by_imdb_id(&self, imdb_id: &str) -> Result<Option<TmdbMovie>> {
        let api_key = self.get_api_key()?;
        let url = format!(
            "{}/find/{}?api_key={}&external_source=imdb_id",
            TMDB_BASE_URL, imdb_id, api_key
        );

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| Error::ExternalApi(e.to_string()))?;

        if !response.status().is_success() {
            return Err(Error::ExternalApi(format!(
                "TMDB API error: {}",
                response.status()
            )));
        }

        #[derive(Debug, Deserialize)]
        struct FindResult {
            movie_results: Vec<TmdbMovie>,
        }

        let result: FindResult = response
            .json()
            .await
            .map_err(|e| Error::ExternalApi(e.to_string()))?;

        Ok(result.movie_results.into_iter().next())
    }

    pub async fn get_movie_details(
        &self,
        tmdb_id: i64,
        language: Option<&str>,
    ) -> Result<TmdbMovieDetails> {
        let lang = language.unwrap_or("de-DE");
        let api_key = self.get_api_key()?;
        let url = format!(
            "{}/movie/{}?api_key={}&language={}",
            TMDB_BASE_URL, tmdb_id, api_key, lang
        );

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| Error::ExternalApi(e.to_string()))?;

        if !response.status().is_success() {
            return Err(Error::ExternalApi(format!(
                "TMDB API error: {}",
                response.status()
            )));
        }

        response
            .json()
            .await
            .map_err(|e| Error::ExternalApi(e.to_string()))
    }

    pub async fn get_movie_credits(
        &self,
        tmdb_id: i64,
        language: Option<&str>,
    ) -> Result<TmdbCredits> {
        let lang = language.unwrap_or("de-DE");
        let api_key = self.get_api_key()?;
        let url = format!(
            "{}/movie/{}/credits?api_key={}&language={}",
            TMDB_BASE_URL, tmdb_id, api_key, lang
        );

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| Error::ExternalApi(e.to_string()))?;

        response
            .json()
            .await
            .map_err(|e| Error::ExternalApi(e.to_string()))
    }

    pub async fn search_tv(&self, query: &str, language: Option<&str>) -> Result<Vec<TmdbTvShow>> {
        let lang = language.unwrap_or("de-DE");
        let api_key = self.get_api_key()?;
        let url = format!(
            "{}/search/tv?api_key={}&query={}&language={}",
            TMDB_BASE_URL,
            api_key,
            urlencoding::encode(query),
            lang
        );

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| Error::ExternalApi(e.to_string()))?;

        let result: TmdbTvSearchResult = response
            .json()
            .await
            .map_err(|e| Error::ExternalApi(e.to_string()))?;

        Ok(result.results)
    }

    pub async fn get_tv_details(
        &self,
        tmdb_id: i64,
        language: Option<&str>,
    ) -> Result<TmdbTvDetails> {
        let lang = language.unwrap_or("de-DE");
        let api_key = self.get_api_key()?;
        let url = format!(
            "{}/tv/{}?api_key={}&language={}",
            TMDB_BASE_URL, tmdb_id, api_key, lang
        );

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| Error::ExternalApi(e.to_string()))?;

        response
            .json()
            .await
            .map_err(|e| Error::ExternalApi(e.to_string()))
    }

    /// Get TV series credits (cast and crew)
    pub async fn get_tv_credits(
        &self,
        tmdb_id: i64,
        language: Option<&str>,
    ) -> Result<TmdbCredits> {
        let lang = language.unwrap_or("de-DE");
        let api_key = self.get_api_key()?;
        let url = format!(
            "{}/tv/{}/credits?api_key={}&language={}",
            TMDB_BASE_URL, tmdb_id, api_key, lang
        );

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| Error::ExternalApi(e.to_string()))?;

        response
            .json()
            .await
            .map_err(|e| Error::ExternalApi(e.to_string()))
    }

    /// Search for collections (e.g., "Alien Collection")
    pub async fn search_collections(
        &self,
        query: &str,
        language: Option<&str>,
    ) -> Result<Vec<TmdbCollectionOverview>> {
        let lang = language.unwrap_or("de-DE");
        let api_key = self.get_api_key()?;
        let url = format!(
            "{}/search/collection?api_key={}&query={}&language={}",
            TMDB_BASE_URL,
            api_key,
            urlencoding::encode(query),
            lang
        );

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| Error::ExternalApi(e.to_string()))?;

        if !response.status().is_success() {
            return Err(Error::ExternalApi(format!(
                "TMDB API error: {}",
                response.status()
            )));
        }

        let result: TmdbCollectionSearchResult = response
            .json()
            .await
            .map_err(|e| Error::ExternalApi(e.to_string()))?;

        Ok(result.results)
    }

    /// Get collection details with all movies in the collection
    pub async fn get_collection_details(
        &self,
        collection_id: i64,
        language: Option<&str>,
    ) -> Result<TmdbCollection> {
        let lang = language.unwrap_or("de-DE");
        let api_key = self.get_api_key()?;
        let url = format!(
            "{}/collection/{}?api_key={}&language={}",
            TMDB_BASE_URL, collection_id, api_key, lang
        );

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| Error::ExternalApi(e.to_string()))?;

        if !response.status().is_success() {
            return Err(Error::ExternalApi(format!(
                "TMDB API error: {}",
                response.status()
            )));
        }

        response
            .json()
            .await
            .map_err(|e| Error::ExternalApi(e.to_string()))
    }

    /// Get full poster URL
    pub fn poster_url(path: &str, size: &str) -> String {
        format!("https://image.tmdb.org/t/p/{}{}", size, path)
    }
}
