//! Hazelnut TUI Application
//!
//! Terminal user interface for managing file organization rules.

use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Parser, Debug)]
#[command(name = "hazelnut")]
#[command(author, version, about = "Terminal-based automated file organizer")]
struct Cli {
    /// Path to config file
    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(clap::Subcommand, Debug)]
enum Commands {
    /// Start the TUI (default)
    Ui,

    /// List all rules
    List,

    /// Validate config file
    Check {
        /// Path to config file to validate
        #[arg(short, long)]
        config: Option<PathBuf>,
    },

    /// Run rules once without watching (dry-run by default)
    Run {
        /// Actually perform actions (not just dry-run)
        #[arg(long)]
        apply: bool,

        /// Target directory to process
        #[arg(short, long)]
        dir: Option<PathBuf>,
    },

    /// Show daemon status
    Status,

    /// Check for updates and install if available
    Update,
}

/// Show daemon status
#[cfg(unix)]
fn show_daemon_status() {
    let state_dir = dirs::home_dir()
        .map(|h| h.join(".local").join("state").join("hazelnut"))
        .unwrap_or_else(|| PathBuf::from("/tmp"));

    let pid_file = state_dir.join("hazelnutd.pid");
    let log_file = state_dir.join("hazelnutd.log");

    let (running, pid) = if let Ok(pid_str) = std::fs::read_to_string(&pid_file) {
        if let Ok(pid) = pid_str.trim().parse::<i32>() {
            if unsafe { libc::kill(pid, 0) == 0 } {
                (true, Some(pid as u32))
            } else {
                (false, None)
            }
        } else {
            (false, None)
        }
    } else {
        (false, None)
    };

    if running {
        let pid = pid.unwrap();
        println!("🌰 Hazelnut daemon is running");
        println!("   PID: {}", pid);
        println!("   PID file: {}", pid_file.display());
        println!("   Log file: {}", log_file.display());

        // Try to get process uptime on Linux
        #[cfg(target_os = "linux")]
        {
            if let Ok(stat) = std::fs::read_to_string(format!("/proc/{}/stat", pid)) {
                let parts: Vec<&str> = stat.split_whitespace().collect();
                if parts.len() > 21
                    && let Ok(start_ticks) = parts[21].parse::<u64>()
                    && let Ok(uptime_str) = std::fs::read_to_string("/proc/uptime")
                    && let Some(uptime_secs) = uptime_str.split_whitespace().next()
                    && let Ok(uptime) = uptime_secs.parse::<f64>()
                {
                    let clock_ticks: u64 = unsafe { libc::sysconf(libc::_SC_CLK_TCK) as u64 };
                    let start_secs = start_ticks / clock_ticks;
                    let running_secs = uptime as u64 - start_secs;

                    let hours = running_secs / 3600;
                    let mins = (running_secs % 3600) / 60;
                    let secs = running_secs % 60;

                    if hours > 0 {
                        println!("   Uptime: {}h {}m {}s", hours, mins, secs);
                    } else if mins > 0 {
                        println!("   Uptime: {}m {}s", mins, secs);
                    } else {
                        println!("   Uptime: {}s", secs);
                    }
                }
            }
        }
    } else {
        println!("🌰 Hazelnut daemon is not running");
    }
}

#[cfg(not(unix))]
fn show_daemon_status() {
    println!("🌰 Daemon status is only available on Unix systems");
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    let log_level = if cli.verbose { "debug" } else { "info" };
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("HAZELNUT_LOG").unwrap_or_else(|_| log_level.to_string()),
        ))
        .with(tracing_subscriber::fmt::layer().with_target(false))
        .init();

    match cli.command {
        None | Some(Commands::Ui) => {
            hazelnut::app::run(cli.config).await?;
        }
        Some(Commands::List) => {
            let config = hazelnut::Config::load(cli.config.as_deref())?;
            println!("Rules:");
            for (i, rule) in config.rules.iter().enumerate() {
                let status = if rule.enabled { "✓" } else { "✗" };
                println!("  {} [{}] {}", status, i + 1, rule.name);
            }
        }
        Some(Commands::Check {
            config: config_path,
        }) => {
            let path = config_path.or(cli.config);
            match hazelnut::Config::load(path.as_deref()) {
                Ok(config) => {
                    println!("✓ Config is valid");
                    println!("  {} watch paths", config.watches.len());
                    println!("  {} rules", config.rules.len());
                }
                Err(e) => {
                    eprintln!("✗ Config error: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Some(Commands::Run { apply, dir }) => {
            let config = hazelnut::Config::load(cli.config.as_deref())?;
            let engine = hazelnut::RuleEngine::new(config.rules);

            let dirs: Vec<_> = if let Some(d) = dir {
                vec![d]
            } else {
                config
                    .watches
                    .iter()
                    .map(|w| hazelnut::expand_path(&w.path))
                    .collect()
            };

            for dir in dirs {
                println!("Processing: {}", dir.display());
                let entries = std::fs::read_dir(&dir)?;
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() {
                        let actions = engine.evaluate_all(&path)?;
                        for action in actions {
                            if apply {
                                println!("  Applying: {} -> {:?}", path.display(), action);
                                action.execute(&path)?;
                            } else {
                                println!("  [dry-run] {} -> {:?}", path.display(), action);
                            }
                        }
                    }
                }
            }
        }
        Some(Commands::Status) => {
            show_daemon_status();
        }
        Some(Commands::Update) => {
            run_update_command();
        }
    }

    Ok(())
}

/// Run the update command
fn run_update_command() {
    use hazelnut::{
        VERSION, VersionCheck, check_for_updates_crates_io, detect_package_manager, run_update,
    };

    println!("🌰 Checking for updates...\n");

    let pm = detect_package_manager();
    println!("  Installed via: {}", pm.name());
    println!("  Current version: {}", VERSION);

    // Use crates.io API (no rate limits, more reliable)
    let check = check_for_updates_crates_io();

    match check {
        VersionCheck::UpdateAvailable { latest, .. } => {
            println!("  Latest version: {}", latest);
            println!("\n⬆ Update available! Installing...\n");

            match run_update(&pm) {
                Ok(()) => {
                    println!("✓ Successfully updated to {}!", latest);
                    println!("\nRestart hazelnut to use the new version.");
                }
                Err(e) => {
                    println!("✗ Update failed: {}", e);
                    println!("\nYou can manually update with:");
                    println!("  {}", pm.update_command());
                    std::process::exit(1);
                }
            }
        }
        VersionCheck::UpToDate => {
            println!("\n✓ Already on the latest version!");
        }
        VersionCheck::CheckFailed(msg) => {
            println!("\n⚠ Could not check for updates: {}", msg);
            std::process::exit(1);
        }
    }
}
