//! Configuration management

mod schema;

pub use schema::{Config, WatchConfig};

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

impl Config {
    /// Load configuration from a file or default location
    pub fn load(path: Option<&Path>) -> Result<Self> {
        let config_path = path
            .map(PathBuf::from)
            .or_else(Self::default_path)
            .context("Could not determine config path")?;

        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)
                .with_context(|| format!("Failed to read config from {}", config_path.display()))?;

            let config: Config = toml::from_str(&content).with_context(|| {
                format!("Failed to parse config from {}", config_path.display())
            })?;

            Ok(config)
        } else {
            Ok(Self::default())
        }
    }

    /// Save configuration to a file
    pub fn save(&self, path: Option<&Path>) -> Result<()> {
        let config_path = path
            .map(PathBuf::from)
            .or_else(Self::default_path)
            .context("Could not determine config path")?;

        // Ensure parent directory exists
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent).with_context(|| {
                format!("Failed to create config directory: {}", parent.display())
            })?;
        }

        let content = toml::to_string_pretty(self).context("Failed to serialize config")?;

        std::fs::write(&config_path, content)
            .with_context(|| format!("Failed to write config to {}", config_path.display()))?;

        Ok(())
    }

    /// Get the default config file path
    pub fn default_path() -> Option<PathBuf> {
        dirs::config_dir().map(|d| d.join("hazelnut").join("config.toml"))
    }

    /// Get the default data directory
    pub fn data_dir() -> Option<PathBuf> {
        dirs::data_dir().map(|d| d.join("hazelnut"))
    }
}
