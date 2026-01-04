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

#[derive(Debug, Deserialize, Serialize)]
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

    /// Get current API key
    fn get_api_key(&self) -> String {
        self.api_key.read().map(|k| k.clone()).unwrap_or_default()
    }

    pub async fn search_movies(&self, query: &str, year: Option<i32>) -> Result<Vec<TmdbMovie>> {
        let mut url = format!(
            "{}/search/movie?api_key={}&query={}&language=de-DE",
            TMDB_BASE_URL,
            self.get_api_key(),
            urlencoding::encode(query)
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

        Ok(result.results)
    }

    /// Find a movie by external ID (e.g., IMDB ID)
    pub async fn find_by_imdb_id(&self, imdb_id: &str) -> Result<Option<TmdbMovie>> {
        let url = format!(
            "{}/find/{}?api_key={}&external_source=imdb_id",
            TMDB_BASE_URL, imdb_id, self.get_api_key()
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

    pub async fn get_movie_details(&self, tmdb_id: i64) -> Result<TmdbMovieDetails> {
        let url = format!(
            "{}/movie/{}?api_key={}&language=de-DE",
            TMDB_BASE_URL, tmdb_id, self.get_api_key()
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

    pub async fn get_movie_credits(&self, tmdb_id: i64) -> Result<TmdbCredits> {
        let url = format!(
            "{}/movie/{}/credits?api_key={}&language=de-DE",
            TMDB_BASE_URL, tmdb_id, self.get_api_key()
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

    pub async fn search_tv(&self, query: &str) -> Result<Vec<TmdbTvShow>> {
        let url = format!(
            "{}/search/tv?api_key={}&query={}&language=de-DE",
            TMDB_BASE_URL,
            self.get_api_key(),
            urlencoding::encode(query)
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

    pub async fn get_tv_details(&self, tmdb_id: i64) -> Result<TmdbTvDetails> {
        let url = format!(
            "{}/tv/{}?api_key={}&language=de-DE",
            TMDB_BASE_URL, tmdb_id, self.get_api_key()
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

    /// Get full poster URL
    pub fn poster_url(path: &str, size: &str) -> String {
        format!("https://image.tmdb.org/t/p/{}{}", size, path)
    }
}
