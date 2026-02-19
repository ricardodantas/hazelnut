//! Hazelnut Daemon (hazelnutd)
//!
//! Background service that watches directories and applies rules.
//!
//! This daemon is Unix-only as it uses Unix signals and process management.

// On non-Unix platforms, provide a stub that exits with a helpful message
#[cfg(not(unix))]
fn main() {
    eprintln!("Error: hazelnutd is only available on Unix systems (Linux, macOS).");
    eprintln!("The daemon requires Unix signals and process management features.");
    eprintln!();
    eprintln!("On Windows, you can still use the hazelnut TUI application.");
    std::process::exit(1);
}

// All Unix-specific code is in this module
#[cfg(unix)]
mod unix_daemon {
    use anyhow::{Context, Result};
    use clap::Parser;
    use std::fs;
    use std::path::PathBuf;
    use std::process::{Command, Stdio};
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

    #[derive(Parser, Debug)]
    #[command(name = "hazelnutd")]
    #[command(author, version, about = "Hazelnut background daemon")]
    pub struct Cli {
        /// Path to config file
        #[arg(short, long, value_name = "FILE")]
        pub config: Option<std::path::PathBuf>,

        #[command(subcommand)]
        pub command: Commands,
    }

    #[derive(clap::Subcommand, Debug)]
    pub enum Commands {
        /// Start the daemon in background
        Start,

        /// Stop the running daemon
        Stop,

        /// Restart the daemon
        Restart,

        /// Show daemon status
        Status,

        /// Reload configuration (HUP signal)
        Reload,

        /// Run in foreground (for debugging)
        Run,
    }

    /// Get the PID file path
    /// Uses ~/.local/state/hazelnut/ on all platforms for consistency
    fn pid_file_path() -> PathBuf {
        dirs::home_dir()
            .map(|h| h.join(".local").join("state").join("hazelnut"))
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join("hazelnutd.pid")
    }

    /// Get the log file path
    /// Uses ~/.local/state/hazelnut/ on all platforms for consistency
    fn log_file_path() -> PathBuf {
        dirs::home_dir()
            .map(|h| h.join(".local").join("state").join("hazelnut"))
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join("hazelnutd.log")
    }

    /// Read PID from file
    fn read_pid() -> Option<i32> {
        let pid_file = pid_file_path();
        if pid_file.exists() {
            fs::read_to_string(&pid_file)
                .ok()
                .and_then(|s| s.trim().parse().ok())
        } else {
            None
        }
    }

    /// Write PID to file
    fn write_pid(pid: u32) -> Result<()> {
        let pid_file = pid_file_path();
        if let Some(parent) = pid_file.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&pid_file, pid.to_string())?;
        Ok(())
    }

    /// Remove PID file
    fn remove_pid_file() {
        let _ = fs::remove_file(pid_file_path());
    }

    /// Check if a process is running
    fn is_process_running(pid: i32) -> bool {
        // Use kill -0 to check if process exists
        unsafe { libc::kill(pid, 0) == 0 }
    }

    /// Send a signal to the daemon
    fn send_signal(pid: i32, signal: i32) -> bool {
        unsafe { libc::kill(pid, signal) == 0 }
    }

    /// Get daemon status
    fn get_status() -> (bool, Option<i32>) {
        if let Some(pid) = read_pid() {
            if is_process_running(pid) {
                return (true, Some(pid));
            }
            // Stale PID file
            remove_pid_file();
        }
        (false, None)
    }

    pub async fn run(cli: Cli) -> Result<()> {
        match cli.command {
            Commands::Start => {
                start_daemon(cli.config)?;
            }
            Commands::Stop => {
                stop_daemon()?;
            }
            Commands::Restart => {
                let _ = stop_daemon();
                std::thread::sleep(std::time::Duration::from_millis(500));
                start_daemon(cli.config)?;
            }
            Commands::Status => {
                show_status();
            }
            Commands::Reload => {
                reload_config()?;
            }
            Commands::Run => {
                // Initialize logging for foreground mode
                tracing_subscriber::registry()
                    .with(tracing_subscriber::EnvFilter::new(
                        std::env::var("HAZELNUT_LOG").unwrap_or_else(|_| "info".to_string()),
                    ))
                    .with(tracing_subscriber::fmt::layer().with_target(false))
                    .init();

                run_daemon(cli.config).await?;
            }
        }

        Ok(())
    }

    fn start_daemon(config_path: Option<PathBuf>) -> Result<()> {
        let (running, pid) = get_status();
        if running {
            println!("🌰 Daemon is already running (PID: {})", pid.unwrap());
            return Ok(());
        }

        println!("🌰 Starting hazelnut daemon...");

        // Get the path to the current executable
        let exe = std::env::current_exe().context("Failed to get executable path")?;

        // Build command
        let mut cmd = Command::new(&exe);
        cmd.arg("run");

        if let Some(ref config) = config_path {
            cmd.arg("--config").arg(config);
        }

        // Set up log file
        let log_path = log_file_path();
        if let Some(parent) = log_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let log_file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
            .context("Failed to open log file")?;

        let log_file_err = log_file.try_clone()?;

        // Start the daemon process
        cmd.stdin(Stdio::null())
            .stdout(log_file)
            .stderr(log_file_err);

        // On Unix, use setsid to detach from terminal
        {
            use std::os::unix::process::CommandExt;
            unsafe {
                cmd.pre_exec(|| {
                    if libc::setsid() == -1 {
                        return Err(std::io::Error::last_os_error());
                    }
                    Ok(())
                });
            }
        }

        let child = cmd.spawn().context("Failed to start daemon")?;
        let pid = child.id();

        write_pid(pid)?;

        println!("✓ Daemon started (PID: {})", pid);
        println!("  Log file: {}", log_path.display());

        Ok(())
    }

    fn stop_daemon() -> Result<()> {
        let (running, pid) = get_status();

        if !running {
            println!("🌰 Daemon is not running");
            return Ok(());
        }

        let pid = pid.unwrap();
        println!("🌰 Stopping daemon (PID: {})...", pid);

        // Send SIGTERM
        if send_signal(pid, libc::SIGTERM) {
            // Wait for process to exit (up to 5 seconds)
            for _ in 0..50 {
                if !is_process_running(pid) {
                    remove_pid_file();
                    println!("✓ Daemon stopped");
                    return Ok(());
                }
                std::thread::sleep(std::time::Duration::from_millis(100));
            }

            // Force kill if still running
            println!("  Sending SIGKILL...");
            send_signal(pid, libc::SIGKILL);
            std::thread::sleep(std::time::Duration::from_millis(100));
            remove_pid_file();
            println!("✓ Daemon killed");
        } else {
            remove_pid_file();
            println!("✗ Failed to stop daemon (process may have already exited)");
        }

        Ok(())
    }

    fn show_status() {
        let (running, pid) = get_status();

        if running {
            let pid = pid.unwrap();
            println!("🌰 Hazelnut daemon is running");
            println!("   PID: {}", pid);
            println!("   PID file: {}", pid_file_path().display());
            println!("   Log file: {}", log_file_path().display());

            // Try to get process uptime on Linux
            #[cfg(target_os = "linux")]
            {
                if let Ok(stat) = fs::read_to_string(format!("/proc/{}/stat", pid)) {
                    let parts: Vec<&str> = stat.split_whitespace().collect();
                    if parts.len() > 21 {
                        // Field 22 is starttime in clock ticks
                        if let Ok(start_ticks) = parts[21].parse::<u64>() {
                            // Get system uptime
                            if let Ok(uptime_str) = fs::read_to_string("/proc/uptime")
                                && let Some(uptime_secs) = uptime_str.split_whitespace().next()
                                && let Ok(uptime) = uptime_secs.parse::<f64>()
                            {
                                let clock_ticks: u64 =
                                    unsafe { libc::sysconf(libc::_SC_CLK_TCK) as u64 };
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
                }
            }
        } else {
            println!("🌰 Hazelnut daemon is not running");
        }
    }

    fn reload_config() -> Result<()> {
        let (running, pid) = get_status();

        if !running {
            println!("🌰 Daemon is not running");
            return Ok(());
        }

        let pid = pid.unwrap();
        println!("🌰 Reloading configuration (PID: {})...", pid);

        if send_signal(pid, libc::SIGHUP) {
            println!("✓ Reload signal sent");
        } else {
            println!("✗ Failed to send reload signal");
        }

        Ok(())
    }

    async fn run_daemon(config_path: Option<std::path::PathBuf>) -> Result<()> {
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
        use tokio::signal::unix::{SignalKind, signal};
        use tokio::time::{Duration, interval};
        use tracing::info;

        // Write PID file for foreground mode too
        write_pid(std::process::id())?;

        let start_time = std::time::Instant::now();

        // Set up IPC listener
        let sock_path = hazelnut::ipc::socket_path();
        // Clean up stale socket
        let _ = std::fs::remove_file(&sock_path);
        let ipc_listener = tokio::net::UnixListener::bind(&sock_path)
            .with_context(|| format!("Failed to bind IPC socket at {}", sock_path.display()))?;
        info!("IPC listening on {}", sock_path.display());

        // Set up signal handlers
        let mut sigterm = signal(SignalKind::terminate())?;
        let mut sigint = signal(SignalKind::interrupt())?;
        let mut sighup = signal(SignalKind::hangup())?;

        let config_path_clone = config_path.clone();
        let mut config = hazelnut::Config::load(config_path.as_deref())?;

        // Initialize notifications
        hazelnut::notifications::init(config.general.notifications_enabled);

        info!(
            "Loaded config with {} watch paths and {} rules",
            config.watches.len(),
            config.rules.len()
        );

        let engine = hazelnut::RuleEngine::new(config.rules.clone());
        let mut watcher = hazelnut::Watcher::new(
            engine,
            config.general.polling_interval_secs,
            config.general.debounce_seconds,
        )?;

        for watch in &config.watches {
            let expanded_path = hazelnut::expand_path(&watch.path);
            info!("Watching: {}", expanded_path.display());
            if let Err(e) =
                watcher.watch_with_rules(&expanded_path, watch.recursive, watch.rules.clone())
            {
                tracing::error!("Failed to watch {}: {}", expanded_path.display(), e);
                hazelnut::notifications::notify_watch_error(
                    &expanded_path.display().to_string(),
                    &e.to_string(),
                );
            }
        }

        info!("Daemon running (PID: {})", std::process::id());

        // Poll for events periodically
        let mut poll_interval = interval(Duration::from_millis(500));

        loop {
            tokio::select! {
                _ = sigterm.recv() => {
                    info!("Received SIGTERM, shutting down...");
                    break;
                }
                _ = sigint.recv() => {
                    info!("Received SIGINT, shutting down...");
                    break;
                }
                _ = sighup.recv() => {
                    info!("Received SIGHUP, reloading configuration...");
                    match hazelnut::Config::load(config_path_clone.as_deref()) {
                        Ok(new_config) => {
                            config = new_config;
                            // Update notification settings
                            hazelnut::notifications::init(config.general.notifications_enabled);
                            // Recreate watcher with new rules, polling interval, and debounce
                            let engine = hazelnut::RuleEngine::new(config.rules.clone());
                            match hazelnut::Watcher::new(
                                engine,
                                config.general.polling_interval_secs,
                                config.general.debounce_seconds,
                            ) {
                                Ok(mut new_watcher) => {
                                    for watch in &config.watches {
                                        let expanded_path = hazelnut::expand_path(&watch.path);
                                        if let Err(e) = new_watcher.watch_with_rules(&expanded_path, watch.recursive, watch.rules.clone()) {
                                            tracing::error!("Failed to watch {}: {}", expanded_path.display(), e);
                                            hazelnut::notifications::notify_watch_error(
                                                &expanded_path.display().to_string(),
                                                &e.to_string(),
                                            );
                                        }
                                    }
                                    new_watcher.carry_over_files_processed(&watcher);
                                    watcher = new_watcher;
                                    info!("Configuration reloaded: {} watches, {} rules",
                                        config.watches.len(), config.rules.len());
                                }
                                Err(e) => {
                                    tracing::error!("Failed to create new watcher: {}", e);
                                }
                            }
                        }
                        Err(e) => {
                            tracing::error!("Failed to reload config: {}", e);
                        }
                    }
                }
                _ = poll_interval.tick() => {
                    match watcher.process_events() {
                        Ok(count) if count > 0 => {
                            info!("Processed {} files", count);
                        }
                        Err(e) => {
                            tracing::error!("Error processing events: {}", e);
                        }
                        _ => {}
                    }
                }
                result = ipc_listener.accept() => {
                    if let Ok((stream, _)) = result {
                        let reader = BufReader::new(stream);
                        let mut lines = reader.lines();
                        if let Ok(Some(line)) = lines.next_line().await {
                            let response = match serde_json::from_str::<hazelnut::ipc::DaemonCommand>(&line) {
                                Ok(cmd) => match cmd {
                                    hazelnut::ipc::DaemonCommand::Status => {
                                        hazelnut::ipc::DaemonResponse::Status {
                                            running: true,
                                            uptime_seconds: start_time.elapsed().as_secs(),
                                            watches: config.watches.len(),
                                            rules: config.rules.len(),
                                            files_processed: watcher.files_processed(),
                                        }
                                    }
                                    hazelnut::ipc::DaemonCommand::Stop => {
                                        info!("Stop requested via IPC");
                                        // Send response before breaking
                                        let resp = serde_json::to_string(&hazelnut::ipc::DaemonResponse::Ok).unwrap_or_default();
                                        let stream = lines.into_inner().into_inner();
                                        let mut w = stream;
                                        let _ = w.write_all(format!("{resp}\n").as_bytes()).await;
                                        let _ = w.flush().await;
                                        break;
                                    }
                                    hazelnut::ipc::DaemonCommand::Reload => {
                                        // Trigger reload via SIGHUP to self
                                        unsafe { libc::kill(std::process::id() as i32, libc::SIGHUP); }
                                        hazelnut::ipc::DaemonResponse::Ok
                                    }
                                    hazelnut::ipc::DaemonCommand::GetLog { limit: _ } => {
                                        // Log retrieval not yet tracked in-memory
                                        hazelnut::ipc::DaemonResponse::Log { entries: vec![] }
                                    }
                                    hazelnut::ipc::DaemonCommand::GetStats => {
                                        hazelnut::ipc::DaemonResponse::Status {
                                            running: true,
                                            uptime_seconds: start_time.elapsed().as_secs(),
                                            watches: config.watches.len(),
                                            rules: config.rules.len(),
                                            files_processed: watcher.files_processed(),
                                        }
                                    }
                                },
                                Err(e) => hazelnut::ipc::DaemonResponse::Error {
                                    message: format!("Invalid command: {e}"),
                                },
                            };
                            let resp_json = serde_json::to_string(&response).unwrap_or_default();
                            let stream = lines.into_inner().into_inner();
                            let mut w = stream;
                            let _ = w.write_all(format!("{resp_json}\n").as_bytes()).await;
                            let _ = w.flush().await;
                        }
                    }
                }
            }
        }

        remove_pid_file();
        let _ = std::fs::remove_file(&sock_path);
        info!("Daemon stopped");
        Ok(())
    }
}

#[cfg(unix)]
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    use clap::Parser;
    let cli = unix_daemon::Cli::parse();
    unix_daemon::run(cli).await
}
