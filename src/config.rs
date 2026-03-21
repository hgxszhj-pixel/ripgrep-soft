//! Configuration management for TurboSearch

use crate::gui::state::{AppTheme, Favorites};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// Current theme
    pub theme: AppTheme,
    /// Font size
    pub font_size: f32,
    /// Maximum files to index
    pub max_index_files: usize,
    /// Maximum filename search results
    pub max_filename_results: usize,
    /// Maximum content search results
    pub max_content_results: usize,
    /// Show welcome dialog on startup
    pub show_welcome: bool,
    /// Last search path
    pub last_search_path: Option<String>,
    /// Selected media player
    pub media_player: Option<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            theme: AppTheme::Blue,
            font_size: 14.0,
            max_index_files: 100_000,
            max_filename_results: 500,
            max_content_results: 5_000,
            show_welcome: true,
            last_search_path: None,
            media_player: None,
        }
    }
}

impl AppConfig {
    /// Get the configuration directory
    pub fn config_dir() -> Option<PathBuf> {
        dirs::data_local_dir().map(|d| d.join("turbo-search"))
    }

    /// Get the settings file path
    pub fn settings_path() -> Option<PathBuf> {
        Self::config_dir().map(|d| d.join("settings.json"))
    }

    /// Get the favorites file path
    pub fn favorites_path() -> Option<PathBuf> {
        Self::config_dir().map(|d| d.join("favorites.json"))
    }

    /// Get the last search path file
    pub fn last_path_file() -> Option<PathBuf> {
        Self::config_dir().map(|d| d.join("last_path.txt"))
    }

    /// Load configuration from file
    pub fn load() -> Self {
        if let Some(path) = Self::settings_path() {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(config) = serde_json::from_str(&content) {
                    return config;
                }
            }
        }
        Self::default()
    }

    /// Save configuration to file
    pub fn save(&self) -> Result<(), ConfigError> {
        let config_dir = Self::config_dir()
            .ok_or(ConfigError::NoConfigDir)?;

        std::fs::create_dir_all(&config_dir)
            .map_err(|e| ConfigError::Io(e.to_string()))?;

        let path = Self::settings_path()
            .ok_or(ConfigError::NoConfigDir)?;

        let content = serde_json::to_string_pretty(self)
            .map_err(|e| ConfigError::Serialize(e.to_string()))?;

        std::fs::write(&path, content)
            .map_err(|e| ConfigError::Io(e.to_string()))?;

        Ok(())
    }

    /// Load favorites
    pub fn load_favorites() -> Favorites {
        if let Some(path) = Self::favorites_path() {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(favorites) = serde_json::from_str(&content) {
                    return favorites;
                }
            }
        }
        Favorites::new()
    }

    /// Save favorites
    pub fn save_favorites(favorites: &Favorites) -> Result<(), ConfigError> {
        let config_dir = Self::config_dir()
            .ok_or(ConfigError::NoConfigDir)?;

        std::fs::create_dir_all(&config_dir)
            .map_err(|e| ConfigError::Io(e.to_string()))?;

        let path = Self::favorites_path()
            .ok_or(ConfigError::NoConfigDir)?;

        let content = serde_json::to_string_pretty(favorites)
            .map_err(|e| ConfigError::Serialize(e.to_string()))?;

        std::fs::write(&path, content)
            .map_err(|e| ConfigError::Io(e.to_string()))?;

        Ok(())
    }

    /// Load last search path
    pub fn load_last_path() -> Option<PathBuf> {
        if let Some(path) = Self::last_path_file() {
            if let Ok(content) = std::fs::read_to_string(&path) {
                let path = PathBuf::from(content.trim());
                if path.exists() {
                    return Some(path);
                }
            }
        }
        None
    }

    /// Save last search path
    pub fn save_last_path(path: &std::path::Path) -> Result<(), ConfigError> {
        let config_dir = Self::config_dir()
            .ok_or(ConfigError::NoConfigDir)?;

        std::fs::create_dir_all(&config_dir)
            .map_err(|e| ConfigError::Io(e.to_string()))?;

        let path_file = Self::last_path_file()
            .ok_or(ConfigError::NoConfigDir)?;

        std::fs::write(&path_file, path.to_string_lossy().as_bytes())
            .map_err(|e| ConfigError::Io(e.to_string()))?;

        Ok(())
    }
}

/// Configuration errors
#[derive(Debug)]
pub enum ConfigError {
    NoConfigDir,
    Io(String),
    Serialize(String),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::NoConfigDir => write!(f, "No configuration directory found"),
            ConfigError::Io(e) => write!(f, "IO error: {e}"),
            ConfigError::Serialize(e) => write!(f, "Serialization error: {e}"),
        }
    }
}

impl std::error::Error for ConfigError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AppConfig::default();
        assert_eq!(config.theme, AppTheme::Blue);
        assert_eq!(config.font_size, 14.0);
        assert_eq!(config.max_index_files, 100_000);
        assert_eq!(config.max_filename_results, 500);
        assert_eq!(config.max_content_results, 5_000);
        assert!(config.show_welcome);
        assert!(config.last_search_path.is_none());
        assert!(config.media_player.is_none());
    }

    #[test]
    fn test_config_dir() {
        let dir = AppConfig::config_dir();
        assert!(dir.is_some());
        let path = dir.unwrap();
        assert!(path.to_string_lossy().contains("turbo-search"));
    }

    #[test]
    fn test_serialize_roundtrip() {
        let config = AppConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let loaded: AppConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config.theme, loaded.theme);
        assert_eq!(config.font_size, loaded.font_size);
        assert_eq!(config.max_index_files, loaded.max_index_files);
    }

    #[test]
    fn test_config_error_display() {
        let err = ConfigError::NoConfigDir;
        assert!(err.to_string().contains("configuration directory"));

        let err = ConfigError::Io("test".to_string());
        assert!(err.to_string().contains("IO error"));

        let err = ConfigError::Serialize("test".to_string());
        assert!(err.to_string().contains("Serialization error"));
    }
}
