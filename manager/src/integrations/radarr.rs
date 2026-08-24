use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use stignore_lib::RadarrConfig;

use super::matcher::matches_movie;

#[derive(Debug)]
pub enum RadarrError {
    RequestFailed(reqwest::Error),
    InvalidResponse(String),
    NotFound(String),
}

impl std::fmt::Display for RadarrError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RadarrError::RequestFailed(e) => write!(f, "Radarr request failed: {}", e),
            RadarrError::InvalidResponse(msg) => write!(f, "Radarr invalid response: {}", msg),
            RadarrError::NotFound(msg) => write!(f, "Radarr movie not found: {}", msg),
        }
    }
}

impl std::error::Error for RadarrError {}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RadarrMovie {
    pub id: i64,
    pub title: String,
    #[serde(default)]
    pub clean_title: Option<String>,
    #[serde(default)]
    pub year: i32,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub folder_name: Option<String>,
    #[serde(default)]
    pub monitored: bool,
    #[serde(default)]
    pub has_file: bool,
    #[serde(default)]
    pub movie_file: Option<RadarrMovieFile>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RadarrMovieFile {
    pub id: i64,
    #[serde(default)]
    pub relative_path: Option<String>,
    #[serde(default)]
    pub path: Option<String>,
}

#[derive(Clone)]
pub struct RadarrClient {
    client: Client,
    pub config: RadarrConfig,
}

impl RadarrClient {
    pub fn new(config: RadarrConfig, timeout_seconds: u64) -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(timeout_seconds))
                .build()
                .expect("Failed to build Radarr HTTP client"),
            config,
        }
    }

    fn format_url(&self, path: &str) -> String {
        let base = self.config.url.trim_end_matches('/');
        let path = path.trim_start_matches('/');
        format!("{}/{}", base, path)
    }

    /// Fetch all movies from Radarr
    pub async fn get_movies(&self) -> Result<Vec<RadarrMovie>, RadarrError> {
        let url = self.format_url("api/v3/movie");
        tracing::debug!("Fetching movies from Radarr: {}", url);

        let response = self
            .client
            .get(&url)
            .header("X-Api-Key", &self.config.api_key)
            .send()
            .await
            .map_err(RadarrError::RequestFailed)?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(RadarrError::InvalidResponse(format!(
                "HTTP {}: {}",
                status, body
            )));
        }

        response
            .json::<Vec<RadarrMovie>>()
            .await
            .map_err(|e| RadarrError::InvalidResponse(e.to_string()))
    }

    /// Finds a movie matching the given item name (folder or file name)
    pub async fn find_movie(&self, item_name: &str) -> Result<Option<RadarrMovie>, RadarrError> {
        let movies = self.get_movies().await?;
        for movie in movies {
            if matches_movie(
                item_name,
                &movie.title,
                movie.clean_title.as_deref(),
                movie.year,
                movie.path.as_deref(),
            ) {
                return Ok(Some(movie));
            }
        }
        Ok(None)
    }

    /// Deletes a movie from Radarr by ID
    pub async fn delete_movie(
        &self,
        movie_id: i64,
        add_import_exclusion: bool,
    ) -> Result<(), RadarrError> {
        let url = format!(
            "{}?deleteFiles=false&addImportExclusion={}",
            self.format_url(&format!("api/v3/movie/{}", movie_id)),
            add_import_exclusion
        );

        tracing::info!(
            "Deleting movie ID {} from Radarr (addImportExclusion={})",
            movie_id,
            add_import_exclusion
        );

        let response = self
            .client
            .delete(&url)
            .header("X-Api-Key", &self.config.api_key)
            .send()
            .await
            .map_err(RadarrError::RequestFailed)?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(RadarrError::InvalidResponse(format!(
                "HTTP {}: {}",
                status, body
            )));
        }

        Ok(())
    }
}
