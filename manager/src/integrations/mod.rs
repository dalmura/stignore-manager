pub mod matcher;
pub mod radarr;
pub mod sonarr;

use serde::{Deserialize, Serialize};
use stignore_lib::IntegrationsConfig;

use self::matcher::parse_season_number;
use self::radarr::RadarrClient;
use self::sonarr::SonarrClient;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MediaMatch {
    pub service: String,
    pub title: String,
    pub details: String,
    pub entity_type: String,
    pub default_exclusion: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MediaDeleteResult {
    pub service: String,
    pub success: bool,
    pub message: String,
}

#[derive(Clone, Default)]
pub struct IntegrationsManager {
    pub radarr: Option<RadarrClient>,
    pub sonarr: Option<SonarrClient>,
}

impl IntegrationsManager {
    pub fn new(config: &IntegrationsConfig, timeout_seconds: u64) -> Self {
        let radarr = config
            .radarr
            .as_ref()
            .filter(|r| r.enabled && !r.url.trim().is_empty())
            .map(|r| RadarrClient::new(r.clone(), timeout_seconds));

        let sonarr = config
            .sonarr
            .as_ref()
            .filter(|s| s.enabled && !s.url.trim().is_empty())
            .map(|s| SonarrClient::new(s.clone(), timeout_seconds));

        Self { radarr, sonarr }
    }

    /// Inspect whether a deletion on (category_id, folder_path) matches any media in Radarr or Sonarr
    pub async fn inspect_deletion(
        &self,
        category_id: &str,
        folder_path: &[String],
    ) -> Option<MediaMatch> {
        if folder_path.is_empty() {
            return None;
        }

        // 1. Check Radarr
        if let Some(radarr) = &self.radarr
            && radarr.config.category_id == category_id
        {
            let target_name = &folder_path[0];
            match radarr.find_movie(target_name).await {
                Ok(Some(movie)) => {
                    return Some(MediaMatch {
                        service: "Radarr".to_string(),
                        title: movie.title.clone(),
                        details: format!("Movie (ID: {}, Year: {})", movie.id, movie.year),
                        entity_type: "movie".to_string(),
                        default_exclusion: radarr.config.add_import_exclusion,
                    });
                }
                Ok(None) => {}
                Err(e) => {
                    tracing::warn!("Error inspecting Radarr for '{}': {}", target_name, e);
                }
            }
        }

        // 2. Check Sonarr
        if let Some(sonarr) = &self.sonarr
            && sonarr.config.category_id == category_id
        {
            let series_name = &folder_path[0];
            match sonarr.find_series(series_name).await {
                Ok(Some(series)) => {
                    if folder_path.len() == 1 {
                        // Entire series match
                        return Some(MediaMatch {
                            service: "Sonarr".to_string(),
                            title: series.title.clone(),
                            details: format!("TV Series (ID: {})", series.id),
                            entity_type: "series".to_string(),
                            default_exclusion: sonarr.config.add_import_list_exclusion,
                        });
                    } else if folder_path.len() >= 2 {
                        // Check if level 2 is a season
                        if let Some(season_num) = parse_season_number(&folder_path[1]) {
                            return Some(MediaMatch {
                                service: "Sonarr".to_string(),
                                title: format!("{} - Season {}", series.title, season_num),
                                details: format!(
                                    "Season {} of Series (ID: {})",
                                    season_num, series.id
                                ),
                                entity_type: "season".to_string(),
                                default_exclusion: sonarr.config.add_import_list_exclusion,
                            });
                        } else {
                            // Subfolder or file under series
                            return Some(MediaMatch {
                                service: "Sonarr".to_string(),
                                title: series.title.clone(),
                                details: format!("Media file under TV Series (ID: {})", series.id),
                                entity_type: "series".to_string(),
                                default_exclusion: sonarr.config.add_import_list_exclusion,
                            });
                        }
                    }
                }
                Ok(None) => {}
                Err(e) => {
                    tracing::warn!("Error inspecting Sonarr for '{}': {}", series_name, e);
                }
            }
        }

        None
    }

    /// Executes media deletion in Radarr or Sonarr when an item has 0 remaining copies across agents
    pub async fn execute_media_deletion(
        &self,
        category_id: &str,
        folder_path: &[String],
        add_exclusion_override: Option<bool>,
    ) -> Option<MediaDeleteResult> {
        if folder_path.is_empty() {
            return None;
        }

        // 1. Handle Radarr
        if let Some(radarr) = &self.radarr
            && radarr.config.category_id == category_id
        {
            let target_name = &folder_path[0];
            match radarr.find_movie(target_name).await {
                Ok(Some(movie)) => {
                    let add_exclusion =
                        add_exclusion_override.unwrap_or(radarr.config.add_import_exclusion);
                    match radarr.delete_movie(movie.id, add_exclusion).await {
                        Ok(()) => {
                            return Some(MediaDeleteResult {
                                service: "Radarr".to_string(),
                                success: true,
                                message: format!("Removed movie '{}' from Radarr", movie.title),
                            });
                        }
                        Err(e) => {
                            return Some(MediaDeleteResult {
                                service: "Radarr".to_string(),
                                success: false,
                                message: format!(
                                    "Failed to remove '{}' from Radarr: {}",
                                    movie.title, e
                                ),
                            });
                        }
                    }
                }
                Ok(None) => {}
                Err(e) => {
                    return Some(MediaDeleteResult {
                        service: "Radarr".to_string(),
                        success: false,
                        message: format!("Radarr lookup error: {}", e),
                    });
                }
            }
        }

        // 2. Handle Sonarr
        if let Some(sonarr) = &self.sonarr
            && sonarr.config.category_id == category_id
        {
            let series_name = &folder_path[0];
            match sonarr.find_series(series_name).await {
                Ok(Some(series)) => {
                    if folder_path.len() == 1 {
                        // Entire series deletion
                        let add_exclusion = add_exclusion_override
                            .unwrap_or(sonarr.config.add_import_list_exclusion);
                        match sonarr.delete_series(series.id, add_exclusion).await {
                            Ok(()) => {
                                return Some(MediaDeleteResult {
                                    service: "Sonarr".to_string(),
                                    success: true,
                                    message: format!(
                                        "Removed series '{}' from Sonarr",
                                        series.title
                                    ),
                                });
                            }
                            Err(e) => {
                                return Some(MediaDeleteResult {
                                    service: "Sonarr".to_string(),
                                    success: false,
                                    message: format!(
                                        "Failed to remove series '{}' from Sonarr: {}",
                                        series.title, e
                                    ),
                                });
                            }
                        }
                    } else if folder_path.len() >= 2
                        && let Some(season_num) = parse_season_number(&folder_path[1])
                    {
                        match sonarr.delete_season(series.id, season_num).await {
                            Ok(file_count) => {
                                return Some(MediaDeleteResult {
                                    service: "Sonarr".to_string(),
                                    success: true,
                                    message: format!(
                                        "Removed Season {} of '{}' from Sonarr (deleted {} file(s), unmonitored season)",
                                        season_num, series.title, file_count
                                    ),
                                });
                            }
                            Err(e) => {
                                return Some(MediaDeleteResult {
                                    service: "Sonarr".to_string(),
                                    success: false,
                                    message: format!(
                                        "Failed to delete Season {} of '{}' in Sonarr: {}",
                                        season_num, series.title, e
                                    ),
                                });
                            }
                        }
                    }
                }
                Ok(None) => {}
                Err(e) => {
                    return Some(MediaDeleteResult {
                        service: "Sonarr".to_string(),
                        success: false,
                        message: format!("Sonarr lookup error: {}", e),
                    });
                }
            }
        }

        None
    }
}
