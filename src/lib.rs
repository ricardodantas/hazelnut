//! Hazelnut - Terminal-based automated file organizer
//!
//! A Hazel-like file organization tool with a TUI interface.

pub mod app;
#[cfg(unix)]
pub mod autostart;
pub mod config;
pub mod ipc;
pub mod notifications;
pub mod rules;
pub mod theme;
pub mod watcher;

pub use config::Config;
pub use rules::{Action, Condition, Rule, RuleEngine};
pub use theme::Theme;
pub use watcher::Watcher;

/// Safe wrapper around `libc::kill`. Returns `true` if the signal was delivered.
#[cfg(unix)]
pub fn process_is_running(pid: i32) -> bool {
    // SAFETY: kill(2) with signal 0 merely probes whether the process exists.
    unsafe { libc::kill(pid, 0) == 0 }
}

/// Safe wrapper around `libc::sysconf`.
#[cfg(unix)]
pub fn clock_ticks_per_sec() -> u64 {
    // SAFETY: sysconf(_SC_CLK_TCK) has no memory-safety concerns.
    unsafe { libc::sysconf(libc::_SC_CLK_TCK) as u64 }
}

/// Safe wrapper around `libc::getuid`.
#[cfg(unix)]
pub fn current_uid() -> u32 {
    // SAFETY: getuid(2) is always safe to call.
    unsafe { libc::getuid() }
}

/// Format a duration in seconds as a human-readable uptime string.
pub fn format_uptime(running_secs: u64) -> String {
    let hours = running_secs / 3600;
    let mins = (running_secs % 3600) / 60;
    let secs = running_secs % 60;
    if hours > 0 {
        format!("{}h {}m {}s", hours, mins, secs)
    } else if mins > 0 {
        format!("{}m {}s", mins, secs)
    } else {
        format!("{}s", secs)
    }
}

/// Read process uptime on Linux by parsing /proc.
/// Returns a formatted uptime string or `None` if unavailable.
#[cfg(target_os = "linux")]
pub fn read_process_uptime(pid: u32) -> Option<String> {
    let stat = std::fs::read_to_string(format!("/proc/{}/stat", pid)).ok()?;
    let parts: Vec<&str> = stat.split_whitespace().collect();
    if parts.len() <= 21 {
        return None;
    }
    let start_ticks: u64 = parts[21].parse().ok()?;
    let uptime_str = std::fs::read_to_string("/proc/uptime").ok()?;
    let uptime: f64 = uptime_str.split_whitespace().next()?.parse().ok()?;
    let clock_ticks = clock_ticks_per_sec();
    let start_secs = start_ticks / clock_ticks;
    let running_secs = uptime as u64 - start_secs;
    Some(format_uptime(running_secs))
}

/// Current version from Cargo.toml
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

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

/// Compare semver versions, returns true if `latest` is newer than `current`.
/// Pre-release suffixes (everything after `-`) are stripped before comparing.
fn version_is_newer(latest: &str, current: &str) -> bool {
    let parse = |v: &str| -> Vec<u32> {
        let base = v.split('-').next().unwrap_or(v);
        base.split('.').filter_map(|s| s.parse().ok()).collect()
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

/// Expand ~ and environment variables ($VAR, ${VAR}) in a path
pub fn expand_path(path: &std::path::Path) -> std::path::PathBuf {
    let path_str = path.to_string_lossy();

    // First expand ~ prefix
    let expanded = if let Some(stripped) = path_str.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            home.join(stripped).to_string_lossy().to_string()
        } else {
            path_str.to_string()
        }
    } else if path_str == "~" {
        if let Some(home) = dirs::home_dir() {
            home.to_string_lossy().to_string()
        } else {
            path_str.to_string()
        }
    } else {
        path_str.to_string()
    };

    // Then expand $VAR and ${VAR} patterns
    use std::sync::LazyLock;
    static ENV_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
        regex::Regex::new(r"\$\{([^}]+)\}|\$([A-Za-z_][A-Za-z0-9_]*)").expect("invalid env regex")
    });

    let result = ENV_RE.replace_all(&expanded, |caps: &regex::Captures| {
        let var_name = caps
            .get(1)
            .or_else(|| caps.get(2))
            .map(|m| m.as_str())
            .unwrap_or("");
        std::env::var(var_name).unwrap_or_else(|_| caps[0].to_string())
    });

    std::path::PathBuf::from(result.as_ref())
}

/// Detected package manager for installation
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackageManager {
    Cargo,
    Homebrew { formula: String },
}

impl PackageManager {
    /// Get display name
    pub fn name(&self) -> &'static str {
        match self {
            PackageManager::Cargo => "cargo",
            PackageManager::Homebrew { .. } => "brew",
        }
    }

    /// Get the update command
    pub fn update_command(&self) -> String {
        match self {
            PackageManager::Cargo => "cargo install hazelnut".to_string(),
            PackageManager::Homebrew { formula } => format!("brew upgrade {}", formula),
        }
    }
}

/// Detect how hazelnut was installed
pub fn detect_package_manager() -> PackageManager {
    // Check if the current executable is in Homebrew's Cellar
    if let Ok(exe_path) = std::env::current_exe() {
        let exe_str = exe_path.to_string_lossy();

        // Path looks like: /opt/homebrew/Cellar/hazelnut/0.2.16/bin/hazelnut
        // or for taps: /opt/homebrew/Cellar/hazelnut/0.2.16/bin/hazelnut (same location)
        if exe_str.contains("/Cellar/") || exe_str.contains("/homebrew/") {
            // Try to get the full formula name from brew
            if let Ok(output) = std::process::Command::new("brew")
                .args(["info", "--json=v2", "hazelnut"])
                .output()
                && output.status.success()
                && let Ok(json) = serde_json::from_slice::<serde_json::Value>(&output.stdout)
                && let Some(formulae) = json.get("formulae").and_then(|f| f.as_array())
                && let Some(formula) = formulae.first()
                && let Some(full_name) = formula.get("full_name").and_then(|n| n.as_str())
            {
                return PackageManager::Homebrew {
                    formula: full_name.to_string(),
                };
            }
            // Fallback to just "hazelnut" if we can't determine the tap
            return PackageManager::Homebrew {
                formula: "hazelnut".to_string(),
            };
        }
    }

    // Default to cargo
    PackageManager::Cargo
}

/// Run the update command and return the result.
///
/// NOTE: This intentionally uses blocking `Command::status()` calls since it's
/// only invoked from the CLI `update` subcommand where blocking is expected.
pub fn run_update(pm: &PackageManager) -> Result<(), String> {
    use std::process::Stdio;

    match pm {
        PackageManager::Cargo => {
            match std::process::Command::new("cargo")
                .args(["install", "hazelnut"])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
            {
                Ok(status) if status.success() => Ok(()),
                Ok(status) => Err(format!("Update failed with status: {}", status)),
                Err(e) => Err(format!("Failed to run cargo: {}", e)),
            }
        }
        PackageManager::Homebrew { formula } => {
            // First update the tap to get latest formula
            let _ = std::process::Command::new("brew")
                .args(["update"])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();

            // Then upgrade the formula
            match std::process::Command::new("brew")
                .args(["upgrade", formula])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
            {
                Ok(status) if status.success() => Ok(()),
                Ok(_) => {
                    // upgrade returns non-zero if already up to date, try reinstall
                    match std::process::Command::new("brew")
                        .args(["reinstall", formula])
                        .stdout(Stdio::null())
                        .stderr(Stdio::null())
                        .status()
                    {
                        Ok(status) if status.success() => Ok(()),
                        Ok(status) => Err(format!("Update failed with status: {}", status)),
                        Err(e) => Err(format!("Failed to run brew: {}", e)),
                    }
                }
                Err(e) => Err(format!("Failed to run brew: {}", e)),
            }
        }
    }
}

/// Check for updates using crates.io API (no rate limits).
pub fn check_for_updates_crates_io() -> VersionCheck {
    check_for_updates_crates_io_timeout(std::time::Duration::from_secs(5))
}

/// Check for updates using crates.io API with custom timeout.
pub fn check_for_updates_crates_io_timeout(timeout: std::time::Duration) -> VersionCheck {
    let url = "https://crates.io/api/v1/crates/hazelnut";

    let agent = ureq::AgentBuilder::new().timeout(timeout).build();

    let result = agent
        .get(url)
        .set("User-Agent", &format!("hazelnut/{}", VERSION))
        .call();

    match result {
        Ok(response) => match response.into_json::<serde_json::Value>() {
            Ok(json) => {
                // crates.io returns: {"crate": {"max_version": "1.2.3", ...}}
                if let Some(latest_str) = json
                    .get("crate")
                    .and_then(|c| c.get("max_version"))
                    .and_then(|v| v.as_str())
                {
                    let latest = latest_str.to_string();
                    let current = VERSION.to_string();

                    if version_is_newer(&latest, &current) {
                        VersionCheck::UpdateAvailable { latest, current }
                    } else {
                        VersionCheck::UpToDate
                    }
                } else {
                    VersionCheck::CheckFailed("Could not parse crates.io response".to_string())
                }
            }
            Err(e) => VersionCheck::CheckFailed(format!("Failed to parse response: {}", e)),
        },
        Err(e) => VersionCheck::CheckFailed(format!("Request failed: {}", e)),
    }
}
