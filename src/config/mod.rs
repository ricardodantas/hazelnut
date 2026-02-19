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

    /// Save configuration to a file (with advisory file locking)
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

        // Use a lockfile to prevent concurrent writes
        let lock_path = config_path.with_extension("toml.lock");
        let lock_file = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&lock_path)
            .with_context(|| format!("Failed to create lock file: {}", lock_path.display()))?;

        use fs2::FileExt;
        lock_file
            .lock_exclusive()
            .with_context(|| "Failed to acquire config file lock")?;

        let result = std::fs::write(&config_path, content)
            .with_context(|| format!("Failed to write config to {}", config_path.display()));

        let _ = lock_file.unlock();
        // Clean up the lock file after successful write
        let _ = std::fs::remove_file(&lock_path);

        result
    }

    /// Get the default config file path
    /// Uses the platform config directory (via dirs::config_dir), falling back to ~/.config
    pub fn default_path() -> Option<PathBuf> {
        let config_base =
            dirs::config_dir().or_else(|| dirs::home_dir().map(|d| d.join(".config")))?;
        Some(config_base.join("hazelnut").join("config.toml"))
    }

    /// Get the default data directory
    pub fn data_dir() -> Option<PathBuf> {
        dirs::data_dir().map(|d| d.join("hazelnut"))
    }
}
