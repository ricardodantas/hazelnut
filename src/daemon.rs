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
        dirs::state_dir()
            .unwrap_or_else(|| {
                dirs::home_dir()
                    .map(|h| h.join(".local").join("state"))
                    .unwrap_or_else(|| PathBuf::from("/tmp"))
            })
            .join("hazelnut")
            .join("hazelnutd.pid")
    }

    /// Get the log file path
    /// Uses dirs::state_dir() with fallback for portability
    fn log_file_path() -> PathBuf {
        dirs::state_dir()
            .unwrap_or_else(|| {
                dirs::home_dir()
                    .map(|h| h.join(".local").join("state"))
                    .unwrap_or_else(|| PathBuf::from("/tmp"))
            })
            .join("hazelnut")
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

    /// Safe wrapper around `libc::kill`. Returns Ok(()) if the signal was
    /// delivered, or an `io::Error` on failure.
    fn send_signal_safe(pid: i32, signal: i32) -> bool {
        // SAFETY: kill(2) is safe to call with any pid/signal combination;
        // the kernel validates arguments and returns an error code.
        let ret = unsafe { libc::kill(pid, signal) };
        ret == 0
    }

    /// Check if a process is running (signal 0 probes without sending)
    fn is_process_running(pid: i32) -> bool {
        send_signal_safe(pid, 0)
    }

    /// Send a signal to the daemon
    fn send_signal(pid: i32, signal: i32) -> bool {
        send_signal_safe(pid, signal)
    }

    /// Safe wrapper around `libc::setsid`.
    fn setsid_safe() -> std::io::Result<()> {
        // SAFETY: setsid(2) creates a new session; no memory-safety concerns.
        let ret = unsafe { libc::setsid() };
        if ret == -1 {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(())
        }
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
            // SAFETY: pre_exec runs between fork and exec; setsid_safe is
            // async-signal-safe and doesn't allocate.
            unsafe {
                cmd.pre_exec(setsid_safe);
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

            #[cfg(target_os = "linux")]
            if let Some(uptime) = hazelnut::read_process_uptime(pid as u32) {
                println!("   Uptime: {}", uptime);
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
        use std::collections::VecDeque;
        use std::sync::{Arc, Mutex};
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
        use tokio::signal::unix::{SignalKind, signal};
        use tokio::time::{Duration, interval};
        use tracing::info;

        /// Maximum number of log entries kept in the ring buffer.
        const MAX_LOG_ENTRIES: usize = 500;

        // In-memory ring buffer for log entries returned by GetLog.
        let log_buffer: Arc<Mutex<VecDeque<String>>> =
            Arc::new(Mutex::new(VecDeque::with_capacity(MAX_LOG_ENTRIES)));

        /// Push a message into the ring buffer, evicting the oldest if full.
        fn push_log(buf: &Mutex<VecDeque<String>>, msg: String, max: usize) {
            if let Ok(mut ring) = buf.lock() {
                if ring.len() >= max {
                    ring.pop_front();
                }
                ring.push_back(msg);
            }
        }

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
        push_log(
            &log_buffer,
            format!(
                "[{}] Daemon started (PID: {})",
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                std::process::id()
            ),
            MAX_LOG_ENTRIES,
        );

        use std::sync::atomic::AtomicBool;

        // Flag to signal stop from spawned IPC tasks
        let stop_flag = Arc::new(AtomicBool::new(false));

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
                    // Check if stop was requested by an IPC task
                    if stop_flag.load(std::sync::atomic::Ordering::Relaxed) {
                        info!("Stop flag set, shutting down...");
                        break;
                    }
                    match watcher.process_events() {
                        Ok(count) if count > 0 => {
                            let msg = format!("[{}] Processed {} file(s)", chrono::Local::now().format("%Y-%m-%d %H:%M:%S"), count);
                            info!("Processed {} files", count);
                            push_log(&log_buffer, msg, MAX_LOG_ENTRIES);
                        }
                        Err(e) => {
                            let msg = format!("[{}] Error: {}", chrono::Local::now().format("%Y-%m-%d %H:%M:%S"), e);
                            tracing::error!("Error processing events: {}", e);
                            push_log(&log_buffer, msg, MAX_LOG_ENTRIES);
                        }
                        _ => {}
                    }
                }
                result = ipc_listener.accept() => {
                    if let Ok((stream, _)) = result {
                        let log_buf = Arc::clone(&log_buffer);
                        let uptime_start = start_time;
                        // Capture stats at command-handling time (not accept time)
                        // so they reflect current state after potential SIGHUP reloads.
                        let num_watches = config.watches.len();
                        let num_rules = config.rules.len();
                        let files_count = watcher.files_processed();
                        let stop = Arc::clone(&stop_flag);

                        // Handle IPC synchronously to avoid race between stop flag
                        // and accepting new connections.
                        let reader = BufReader::new(stream);
                        let mut lines = reader.lines();
                        // Apply a per-connection read timeout so a slow/malicious client
                        // cannot block the daemon event loop indefinitely.
                        let read_result = tokio::time::timeout(
                            Duration::from_secs(5),
                            lines.next_line(),
                        ).await;
                        if let Ok(Ok(Some(line))) = read_result {
                            let response = match serde_json::from_str::<hazelnut::ipc::DaemonCommand>(&line) {
                                Ok(cmd) => match cmd {
                                    hazelnut::ipc::DaemonCommand::Status => {
                                        hazelnut::ipc::DaemonResponse::Status {
                                            running: true,
                                            uptime_seconds: uptime_start.elapsed().as_secs(),
                                            watches: num_watches,
                                            rules: num_rules,
                                            files_processed: files_count,
                                        }
                                    }
                                    hazelnut::ipc::DaemonCommand::Stop => {
                                        info!("Stop requested via IPC");
                                        let resp = serde_json::to_string(&hazelnut::ipc::DaemonResponse::Ok).unwrap_or_default();
                                        let stream = lines.into_inner().into_inner();
                                        let mut w = stream;
                                        let _ = w.write_all(format!("{resp}\n").as_bytes()).await;
                                        let _ = w.flush().await;
                                        stop.store(true, std::sync::atomic::Ordering::SeqCst);
                                        // Break immediately — no more connections accepted
                                        break;
                                    }
                                    hazelnut::ipc::DaemonCommand::Reload => {
                                        match i32::try_from(std::process::id()) {
                                            Ok(pid) => {
                                                send_signal_safe(pid, libc::SIGHUP);
                                                hazelnut::ipc::DaemonResponse::Ok
                                            }
                                            Err(_) => hazelnut::ipc::DaemonResponse::Error {
                                                message: "PID too large for signal delivery".to_string(),
                                            },
                                        }
                                    }
                                    hazelnut::ipc::DaemonCommand::GetLog { limit } => {
                                        let entries = if let Ok(ring) = log_buf.lock() {
                                            let skip = ring.len().saturating_sub(limit);
                                            ring.iter().skip(skip).cloned().collect()
                                        } else {
                                            vec![]
                                        };
                                        hazelnut::ipc::DaemonResponse::Log { entries }
                                    }
                                    hazelnut::ipc::DaemonCommand::GetStats => {
                                        hazelnut::ipc::DaemonResponse::Status {
                                            running: true,
                                            uptime_seconds: uptime_start.elapsed().as_secs(),
                                            watches: num_watches,
                                            rules: num_rules,
                                            files_processed: files_count,
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

                        // Check stop flag after every IPC command
                        if stop_flag.load(std::sync::atomic::Ordering::SeqCst) {
                            break;
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
