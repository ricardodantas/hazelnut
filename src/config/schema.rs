//! Configuration schema

use crate::rules::Rule;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Main configuration structure
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    /// General settings
    #[serde(default)]
    pub general: GeneralConfig,

    /// Watched folders
    #[serde(default, rename = "watch")]
    pub watches: Vec<WatchConfig>,

    /// Organization rules
    #[serde(default, rename = "rule")]
    pub rules: Vec<Rule>,
}

/// General application settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    /// Log level (trace, debug, info, warn, error)
    #[serde(default = "default_log_level")]
    pub log_level: String,

    /// Path to log file
    #[serde(default)]
    pub log_file: Option<PathBuf>,

    /// Seconds to wait before processing a file (debounce)
    #[serde(default = "default_debounce")]
    pub debounce_seconds: u64,

    /// Polling interval in seconds for watching file changes
    #[serde(default = "default_polling_interval")]
    pub polling_interval_secs: u64,

    /// Maximum number of log entries to retain
    #[serde(default = "default_log_retention")]
    pub log_retention: usize,

    /// Start daemon automatically when TUI launches
    #[serde(default)]
    pub start_daemon_on_launch: bool,

    /// Enable desktop notifications (future feature)
    #[serde(default)]
    pub notifications_enabled: bool,

    /// Theme name
    #[serde(default)]
    pub theme: Option<String>,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            log_level: default_log_level(),
            log_file: None,
            debounce_seconds: default_debounce(),
            polling_interval_secs: default_polling_interval(),
            log_retention: default_log_retention(),
            start_daemon_on_launch: false,
            notifications_enabled: false,
            theme: None,
        }
    }
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_debounce() -> u64 {
    2
}

fn default_polling_interval() -> u64 {
    5
}

fn default_log_retention() -> usize {
    1000
}

/// Configuration for a watched folder
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchConfig {
    /// Path to watch
    pub path: PathBuf,

    /// Watch subdirectories recursively
    #[serde(default)]
    pub recursive: bool,

    /// Only apply rules with these names (empty = all rules)
    #[serde(default)]
    pub rules: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_minimal_config() {
        let toml = r#"
            [[watch]]
            path = "~/Downloads"
        "#;

        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(config.watches.len(), 1);
        assert_eq!(config.watches[0].path.to_string_lossy(), "~/Downloads");
        assert!(!config.watches[0].recursive);
    }

    #[test]
    fn test_parse_full_config() {
        let toml = r#"
            [general]
            log_level = "debug"
            debounce_seconds = 5

            [[watch]]
            path = "~/Downloads"
            recursive = true
            rules = ["pdfs", "images"]

            [[rule]]
            name = "pdfs"
            enabled = true

            [rule.condition]
            extension = "pdf"

            [rule.action]
            type = "move"
            destination = "~/Documents/PDFs"
        "#;

        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(config.general.log_level, "debug");
        assert_eq!(config.general.debounce_seconds, 5);
        assert_eq!(config.watches.len(), 1);
        assert!(config.watches[0].recursive);
        assert_eq!(config.rules.len(), 1);
        assert_eq!(config.rules[0].name, "pdfs");
    }
}
