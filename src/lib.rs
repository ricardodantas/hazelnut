//! Hazelnut - Terminal-based automated file organizer
//!
//! A Hazel-like file organization tool with a TUI interface.

pub mod app;
pub mod config;
pub mod ipc;
pub mod rules;
pub mod theme;
pub mod watcher;

pub use config::Config;
pub use rules::{Action, Condition, Rule, RuleEngine};
pub use theme::Theme;
pub use watcher::Watcher;

/// Current version from Cargo.toml
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// GitHub repository for release checks
const GITHUB_REPO: &str = "ricardodantas/hazelnut";

/// Result of a version check
#[derive(Debug, Clone)]
pub enum VersionCheck {
    /// Running the latest version
    UpToDate,
    /// A newer version is available
    UpdateAvailable { latest: String, current: String },
    /// Could not check (network error, etc.)
    CheckFailed(String),
}

/// Check if a newer version is available on GitHub
pub fn check_for_updates() -> VersionCheck {
    check_for_updates_timeout(std::time::Duration::from_secs(3))
}

/// Check if a newer version is available on GitHub with custom timeout
pub fn check_for_updates_timeout(timeout: std::time::Duration) -> VersionCheck {
    let url = format!("https://api.github.com/repos/{}/releases/latest", GITHUB_REPO);
    
    let agent = ureq::AgentBuilder::new()
        .timeout(timeout)
        .build();
    
    let result = agent.get(&url)
        .set("User-Agent", &format!("hazelnut/{}", VERSION))
        .call();
    
    match result {
        Ok(response) => {
            match response.into_json::<serde_json::Value>() {
                Ok(json) => {
                    if let Some(tag) = json.get("tag_name").and_then(|v| v.as_str()) {
                        let latest = tag.trim_start_matches('v').to_string();
                        let current = VERSION.to_string();
                        
                        if version_is_newer(&latest, &current) {
                            VersionCheck::UpdateAvailable { latest, current }
                        } else {
                            VersionCheck::UpToDate
                        }
                    } else {
                        VersionCheck::CheckFailed("Could not parse release info".to_string())
                    }
                }
                Err(e) => VersionCheck::CheckFailed(format!("Failed to parse response: {}", e)),
            }
        }
        Err(e) => VersionCheck::CheckFailed(format!("Network error: {}", e)),
    }
}

/// Compare semver versions, returns true if `latest` is newer than `current`
fn version_is_newer(latest: &str, current: &str) -> bool {
    let parse = |v: &str| -> Vec<u32> {
        v.split('.')
            .filter_map(|s| s.parse().ok())
            .collect()
    };
    
    let latest_parts = parse(latest);
    let current_parts = parse(current);
    
    for i in 0..3 {
        let l = latest_parts.get(i).copied().unwrap_or(0);
        let c = current_parts.get(i).copied().unwrap_or(0);
        if l > c {
            return true;
        }
        if l < c {
            return false;
        }
    }
    false
}

/// Expand ~ in a path to the user's home directory
pub fn expand_path(path: &std::path::Path) -> std::path::PathBuf {
    let path_str = path.to_string_lossy();

    if let Some(stripped) = path_str.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(stripped);
        }
    } else if path_str == "~"
        && let Some(home) = dirs::home_dir()
    {
        return home;
    }

    path.to_path_buf()
}
