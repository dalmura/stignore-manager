use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use stignore_lib::SonarrConfig;

use super::matcher::matches_series;

#[derive(Debug)]
pub enum SonarrError {
    RequestFailed(reqwest::Error),
    InvalidResponse(String),
    NotFound(String),
}

impl std::fmt::Display for SonarrError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SonarrError::RequestFailed(e) => write!(f, "Sonarr request failed: {}", e),
            SonarrError::InvalidResponse(msg) => write!(f, "Sonarr invalid response: {}", msg),
            SonarrError::NotFound(msg) => write!(f, "Sonarr series not found: {}", msg),
        }
    }
}

impl std::error::Error for SonarrError {}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SonarrSeries {
    pub id: i64,
    pub title: String,
    #[serde(default)]
    pub clean_title: Option<String>,
    #[serde(default)]
    pub year: Option<i32>,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub monitored: bool,
    #[serde(default)]
    pub seasons: Vec<SonarrSeason>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SonarrSeason {
    pub season_number: i32,
    pub monitored: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SonarrEpisodeFile {
    pub id: i64,
    pub series_id: i64,
    pub season_number: i32,
    #[serde(default)]
    pub relative_path: Option<String>,
    #[serde(default)]
    pub path: Option<String>,
}

#[derive(Clone)]
pub struct SonarrClient {
    client: Client,
    pub config: SonarrConfig,
}

impl SonarrClient {
    pub fn new(config: SonarrConfig, timeout_seconds: u64) -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(timeout_seconds))
                .build()
                .expect("Failed to build Sonarr HTTP client"),
            config,
        }
    }

    fn format_url(&self, path: &str) -> String {
        let base = self.config.url.trim_end_matches('/');
        let path = path.trim_start_matches('/');
        format!("{}/{}", base, path)
    }

    /// Fetch all series from Sonarr
    pub async fn get_series(&self) -> Result<Vec<SonarrSeries>, SonarrError> {
        let url = self.format_url("api/v3/series");
        tracing::debug!("Fetching series from Sonarr: {}", url);

        let response = self
            .client
            .get(&url)
            .header("X-Api-Key", &self.config.api_key)
            .send()
            .await
            .map_err(SonarrError::RequestFailed)?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(SonarrError::InvalidResponse(format!(
                "HTTP {}: {}",
                status, body
            )));
        }

        response
            .json::<Vec<SonarrSeries>>()
            .await
            .map_err(|e| SonarrError::InvalidResponse(e.to_string()))
    }

    /// Fetch a single series from Sonarr by ID
    pub async fn get_series_by_id(&self, series_id: i64) -> Result<SonarrSeries, SonarrError> {
        let url = self.format_url(&format!("api/v3/series/{}", series_id));

        let response = self
            .client
            .get(&url)
            .header("X-Api-Key", &self.config.api_key)
            .send()
            .await
            .map_err(SonarrError::RequestFailed)?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(SonarrError::InvalidResponse(format!(
                "HTTP {}: {}",
                status, body
            )));
        }

        response
            .json::<SonarrSeries>()
            .await
            .map_err(|e| SonarrError::InvalidResponse(e.to_string()))
    }

    /// Finds a series matching the given item name (folder or series name)
    pub async fn find_series(&self, item_name: &str) -> Result<Option<SonarrSeries>, SonarrError> {
        let series_list = self.get_series().await?;
        for series in series_list {
            if matches_series(
                item_name,
                &series.title,
                series.clean_title.as_deref(),
                series.year,
                series.path.as_deref(),
            ) {
                return Ok(Some(series));
            }
        }
        Ok(None)
    }

    /// Deletes a whole series from Sonarr by ID
    pub async fn delete_series(
        &self,
        series_id: i64,
        add_import_list_exclusion: bool,
    ) -> Result<(), SonarrError> {
        let url = format!(
            "{}?deleteFiles=false&addImportListExclusion={}",
            self.format_url(&format!("api/v3/series/{}", series_id)),
            add_import_list_exclusion
        );

        tracing::info!(
            "Deleting series ID {} from Sonarr (addImportListExclusion={})",
            series_id,
            add_import_list_exclusion
        );

        let response = self
            .client
            .delete(&url)
            .header("X-Api-Key", &self.config.api_key)
            .send()
            .await
            .map_err(SonarrError::RequestFailed)?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(SonarrError::InvalidResponse(format!(
                "HTTP {}: {}",
                status, body
            )));
        }

        Ok(())
    }

    /// Fetch episode files for a series
    pub async fn get_episode_files(
        &self,
        series_id: i64,
    ) -> Result<Vec<SonarrEpisodeFile>, SonarrError> {
        let url = format!(
            "{}?seriesId={}",
            self.format_url("api/v3/episodefile"),
            series_id
        );

        let response = self
            .client
            .get(&url)
            .header("X-Api-Key", &self.config.api_key)
            .send()
            .await
            .map_err(SonarrError::RequestFailed)?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(SonarrError::InvalidResponse(format!(
                "HTTP {}: {}",
                status, body
            )));
        }

        response
            .json::<Vec<SonarrEpisodeFile>>()
            .await
            .map_err(|e| SonarrError::InvalidResponse(e.to_string()))
    }

    /// Deletes a single episode file by ID
    pub async fn delete_episode_file(&self, episode_file_id: i64) -> Result<(), SonarrError> {
        let url = self.format_url(&format!("api/v3/episodefile/{}", episode_file_id));

        let response = self
            .client
            .delete(&url)
            .header("X-Api-Key", &self.config.api_key)
            .send()
            .await
            .map_err(SonarrError::RequestFailed)?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(SonarrError::InvalidResponse(format!(
                "HTTP {}: {}",
                status, body
            )));
        }

        Ok(())
    }

    /// Updates series season monitoring status to unmonitor a deleted season
    pub async fn unmonitor_season(
        &self,
        series_id: i64,
        season_number: i32,
    ) -> Result<(), SonarrError> {
        let mut series = self.get_series_by_id(series_id).await?;
        let mut modified = false;

        for season in &mut series.seasons {
            if season.season_number == season_number && season.monitored {
                season.monitored = false;
                modified = true;
            }
        }

        if !modified {
            return Ok(());
        }

        let url = self.format_url(&format!("api/v3/series/{}", series_id));
        let response = self
            .client
            .put(&url)
            .header("X-Api-Key", &self.config.api_key)
            .json(&series)
            .send()
            .await
            .map_err(SonarrError::RequestFailed)?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(SonarrError::InvalidResponse(format!(
                "HTTP {}: {}",
                status, body
            )));
        }

        Ok(())
    }

    /// Deletes all episode files for a season and unmonitors the season
    pub async fn delete_season(
        &self,
        series_id: i64,
        season_number: i32,
    ) -> Result<usize, SonarrError> {
        let episode_files = self.get_episode_files(series_id).await?;
        let season_files: Vec<_> = episode_files
            .into_iter()
            .filter(|f| f.season_number == season_number)
            .collect();

        let count = season_files.len();
        for file in season_files {
            if let Err(e) = self.delete_episode_file(file.id).await {
                tracing::warn!(
                    "Failed to delete Sonarr episode file ID {} for season {}: {}",
                    file.id,
                    season_number,
                    e
                );
            }
        }

        // Unmonitor season so Sonarr doesn't re-download it
        if let Err(e) = self.unmonitor_season(series_id, season_number).await {
            tracing::warn!(
                "Failed to unmonitor Sonarr season {} for series {}: {}",
                season_number,
                series_id,
                e
            );
        }

        Ok(count)
    }
}
