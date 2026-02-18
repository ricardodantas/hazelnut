//! TUI Application module

mod events;
mod state;
mod ui;

pub use state::AppState;

use anyhow::Result;
use crossterm::{
    event::{self, Event},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::prelude::*;
use std::io::stdout;
use std::path::PathBuf;
use std::time::Duration;

use crate::config::Config;
use crate::theme::Theme;

use std::sync::mpsc;

/// Messages from background tasks
enum BackgroundMsg {
    UpdateAvailable(String),
}

/// Run the TUI application
pub async fn run(config_path: Option<PathBuf>) -> Result<()> {
    // Load config from specified path or default (~/.config/hazelnut/config.toml)
    let config = Config::load(config_path.as_deref())?;

    // Load theme from config or use default
    let theme = Theme::load(&config);

    // Initialize terminal
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    // Create app state
    let mut state = AppState::new(config.clone(), theme);

    // Start daemon on launch if configured (Unix only)
    #[cfg(unix)]
    if config.general.start_daemon_on_launch && !state.daemon_running {
        use std::process::{Command, Stdio};

        // Find hazelnutd binary
        let daemon_cmd = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|dir| dir.join("hazelnutd")))
            .filter(|p| p.exists())
            .unwrap_or_else(|| std::path::PathBuf::from("hazelnutd"));

        match Command::new(&daemon_cmd)
            .args(["start"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
        {
            Ok(status) if status.success() => {
                state.daemon_running = true;
                state.status_message = Some("Daemon started automatically".to_string());
            }
            Ok(_) | Err(_) => {
                // Silently fail - daemon might already be running
            }
        }
    }

    // Spawn background update check
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let check = crate::check_for_updates_crates_io_timeout(std::time::Duration::from_secs(5));
        if let crate::VersionCheck::UpdateAvailable { latest, .. } = check {
            let _ = tx.send(BackgroundMsg::UpdateAvailable(latest));
        }
    });

    // Start embedded watcher when daemon is not running
    let mut embedded_watcher = if !state.daemon_running {
        match create_embedded_watcher(&config) {
            Ok(w) => {
                state.daemon_running = true;
                state.status_message = Some("Watching files (embedded)".to_string());
                Some(w)
            }
            Err(e) => {
                tracing::error!("Failed to start embedded watcher: {}", e);
                None
            }
        }
    } else {
        None
    };

    // Main loop
    let result = run_app(&mut terminal, &mut state, rx, &mut embedded_watcher);

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    state: &mut AppState,
    bg_rx: mpsc::Receiver<BackgroundMsg>,
    embedded_watcher: &mut Option<crate::Watcher>,
) -> Result<()> {
    loop {
        // Check for background messages (non-blocking)
        if let Ok(msg) = bg_rx.try_recv() {
            match msg {
                BackgroundMsg::UpdateAvailable(version) => {
                    state.set_update_available(version);
                }
            }
        }

        // Draw UI
        terminal.draw(|frame| ui::render(frame, state))?;

        // Process pending update after draw (so "Updating..." is visible)
        if state.pending_update {
            events::process_pending_update(state);
            // Redraw immediately after update completes
            terminal.draw(|frame| ui::render(frame, state))?;
        }

        // Handle events
        if event::poll(Duration::from_millis(100))?
            && let Event::Key(key) = event::read()?
            && key.kind == crossterm::event::KeyEventKind::Press
        {
            events::handle_key(state, key);
        }

        // Process embedded watcher events
        if let Some(watcher) = embedded_watcher {
            match watcher.process_events() {
                Ok(count) if count > 0 => {
                    tracing::info!("Processed {} files", count);
                }
                Err(e) => {
                    tracing::error!("Watcher error: {}", e);
                }
                _ => {}
            }
        }

        // Tick for animations
        state.tick();

        if state.should_quit {
            break;
        }
    }

    Ok(())
}

/// Create an embedded file watcher for use when the daemon is not running.
/// This enables file watching on all platforms (including Windows).
fn create_embedded_watcher(config: &crate::Config) -> Result<crate::Watcher> {
    let engine = crate::RuleEngine::new(config.rules.clone());
    let mut watcher = crate::Watcher::new(
        engine,
        config.general.polling_interval_secs,
        config.general.debounce_seconds,
    )?;

    for watch in &config.watches {
        let expanded_path = crate::expand_path(&watch.path);
        if let Err(e) = watcher.watch(&expanded_path, watch.recursive) {
            tracing::error!("Failed to watch {}: {}", expanded_path.display(), e);
        }
    }

    Ok(watcher)
}
