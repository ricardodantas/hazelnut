//! Auto-start functionality for hazelnutd daemon
//!
//! Supports:
//! - macOS: LaunchAgent plist
//! - Linux: systemd user service

use std::fs;
use std::io;
use std::path::PathBuf;

/// Check if auto-start is currently enabled
pub fn is_enabled() -> bool {
    get_autostart_path().map(|p| p.exists()).unwrap_or(false)
}

/// Enable auto-start for the daemon
pub fn enable() -> io::Result<()> {
    let path = get_autostart_path().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::Unsupported,
            "Auto-start not supported on this platform",
        )
    })?;

    // Create parent directory if needed
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let content = get_autostart_content()?;
    fs::write(&path, content)?;

    // On Linux with systemd, reload the daemon
    #[cfg(target_os = "linux")]
    {
        let _ = std::process::Command::new("systemctl")
            .args(["--user", "daemon-reload"])
            .output();
    }

    Ok(())
}

/// Disable auto-start for the daemon
pub fn disable() -> io::Result<()> {
    let path = get_autostart_path().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::Unsupported,
            "Auto-start not supported on this platform",
        )
    })?;

    if path.exists() {
        fs::remove_file(&path)?;
    }

    // On Linux with systemd, reload the daemon
    #[cfg(target_os = "linux")]
    {
        let _ = std::process::Command::new("systemctl")
            .args(["--user", "daemon-reload"])
            .output();
    }

    Ok(())
}

/// Toggle auto-start (enable if disabled, disable if enabled)
pub fn toggle() -> io::Result<bool> {
    if is_enabled() {
        disable()?;
        Ok(false)
    } else {
        enable()?;
        Ok(true)
    }
}

/// Get the path to the autostart file for the current platform
fn get_autostart_path() -> Option<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        dirs::home_dir().map(|h| {
            h.join("Library")
                .join("LaunchAgents")
                .join("me.ricardodantas.hazelnutd.plist")
        })
    }

    #[cfg(target_os = "linux")]
    {
        // Prefer systemd if available, fallback to XDG autostart
        if is_systemd_available() {
            dirs::home_dir().map(|h| {
                h.join(".config")
                    .join("systemd")
                    .join("user")
                    .join("hazelnutd.service")
            })
        } else {
            dirs::config_dir().map(|c| c.join("autostart").join("hazelnutd.desktop"))
        }
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        None
    }
}

/// Get the content for the autostart file
fn get_autostart_content() -> io::Result<String> {
    let binary_path = get_daemon_binary_path()?;

    #[cfg(target_os = "macos")]
    {
        Ok(format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>me.ricardodantas.hazelnutd</string>
    <key>ProgramArguments</key>
    <array>
        <string>{}</string>
        <string>run</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <false/>
    <key>StandardOutPath</key>
    <string>/tmp/hazelnutd.stdout.log</string>
    <key>StandardErrorPath</key>
    <string>/tmp/hazelnutd.stderr.log</string>
</dict>
</plist>
"#,
            binary_path.display()
        ))
    }

    #[cfg(target_os = "linux")]
    {
        if is_systemd_available() {
            Ok(format!(
                r#"[Unit]
Description=Hazelnut File Organizer Daemon
After=default.target

[Service]
Type=simple
ExecStart={} run
Restart=on-failure
RestartSec=5

[Install]
WantedBy=default.target
"#,
                binary_path.display()
            ))
        } else {
            Ok(format!(
                r#"[Desktop Entry]
Type=Application
Name=Hazelnut Daemon
Exec={} run
Hidden=false
NoDisplay=true
X-GNOME-Autostart-enabled=true
"#,
                binary_path.display()
            ))
        }
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "Auto-start not supported on this platform",
        ))
    }
}

/// Find the daemon binary path
fn get_daemon_binary_path() -> io::Result<PathBuf> {
    // First try to find hazelnutd in PATH
    if let Ok(output) = std::process::Command::new("which")
        .arg("hazelnutd")
        .output()
        && output.status.success()
    {
        let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !path.is_empty() {
            return Ok(PathBuf::from(path));
        }
    }

    // Fallback: check common locations
    let common_paths = [
        "/usr/local/bin/hazelnutd",
        "/opt/homebrew/bin/hazelnutd",
        "/usr/bin/hazelnutd",
    ];

    for path in common_paths {
        let p = PathBuf::from(path);
        if p.exists() {
            return Ok(p);
        }
    }

    // Last resort: check if cargo installed it
    if let Some(home) = dirs::home_dir() {
        let cargo_bin = home.join(".cargo").join("bin").join("hazelnutd");
        if cargo_bin.exists() {
            return Ok(cargo_bin);
        }
    }

    Err(io::Error::new(
        io::ErrorKind::NotFound,
        "Could not find hazelnutd binary. Make sure it's installed and in PATH.",
    ))
}

/// Check if systemd is available on Linux
#[cfg(target_os = "linux")]
fn is_systemd_available() -> bool {
    std::process::Command::new("systemctl")
        .arg("--user")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}
