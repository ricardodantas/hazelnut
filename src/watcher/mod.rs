//! File system watcher

mod handler;

pub use handler::EventHandler;

use anyhow::Result;
use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher as NotifyWatcher};
use std::path::Path;
use std::sync::mpsc;
use std::time::Duration;
use tracing::{debug, error, info};

use crate::rules::RuleEngine;

/// File system watcher that monitors directories and applies rules
pub struct Watcher {
    watcher: RecommendedWatcher,
    engine: RuleEngine,
    rx: mpsc::Receiver<Result<notify::Event, notify::Error>>,
}

impl Watcher {
    /// Create a new watcher with the given rule engine and polling interval
    pub fn new(engine: RuleEngine, polling_interval_secs: u64) -> Result<Self> {
        let (tx, rx) = mpsc::channel();

        let watcher = RecommendedWatcher::new(
            move |res| {
                if let Err(e) = tx.send(res) {
                    error!("Failed to send watch event: {}", e);
                }
            },
            Config::default().with_poll_interval(Duration::from_secs(polling_interval_secs)),
        )?;

        Ok(Self {
            watcher,
            engine,
            rx,
        })
    }

    /// Start watching a directory
    pub fn watch(&mut self, path: &Path, recursive: bool) -> Result<()> {
        let mode = if recursive {
            RecursiveMode::Recursive
        } else {
            RecursiveMode::NonRecursive
        };

        self.watcher.watch(path, mode)?;
        info!("Watching: {} (recursive: {})", path.display(), recursive);

        Ok(())
    }

    /// Stop watching a directory
    pub fn unwatch(&mut self, path: &Path) -> Result<()> {
        self.watcher.unwatch(path)?;
        info!("Stopped watching: {}", path.display());
        Ok(())
    }

    /// Process pending events (non-blocking)
    pub fn poll(&self) -> Result<Vec<notify::Event>> {
        let mut events = Vec::new();

        while let Ok(result) = self.rx.try_recv() {
            match result {
                Ok(event) => events.push(event),
                Err(e) => error!("Watch error: {}", e),
            }
        }

        Ok(events)
    }

    /// Process events and apply rules
    pub fn process_events(&self) -> Result<usize> {
        let mut processed = 0;

        for event in self.poll()? {
            debug!("Event: {:?}", event.kind);

            // Only process create and modify events
            match event.kind {
                notify::EventKind::Create(_)
                | notify::EventKind::Modify(_)
                | notify::EventKind::Access(_) => {
                    for path in &event.paths {
                        if path.is_file() && path.exists() {
                            info!("File event detected: {}", path.display());
                            if self.engine.process(path)? {
                                processed += 1;
                            }
                        }
                    }
                }
                _ => {
                    debug!("Ignoring event kind: {:?}", event.kind);
                }
            }
        }

        Ok(processed)
    }

    /// Get the rule engine
    pub fn engine(&self) -> &RuleEngine {
        &self.engine
    }
}
