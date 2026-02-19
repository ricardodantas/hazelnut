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
    event_handler: EventHandler,
    files_processed: u64,
    /// Mapping of watched directory path → allowed rule names (empty = all rules)
    watch_rules: std::collections::HashMap<std::path::PathBuf, Vec<String>>,
}

impl Watcher {
    /// Create a new watcher with the given rule engine, polling interval, and debounce duration
    pub fn new(
        engine: RuleEngine,
        polling_interval_secs: u64,
        debounce_seconds: u64,
    ) -> Result<Self> {
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
            event_handler: EventHandler::new(debounce_seconds),
            files_processed: 0,
            watch_rules: std::collections::HashMap::new(),
        })
    }

    /// Start watching a directory
    pub fn watch(&mut self, path: &Path, recursive: bool) -> Result<()> {
        self.watch_with_rules(path, recursive, Vec::new())
    }

    /// Start watching a directory with a specific set of allowed rule names
    pub fn watch_with_rules(
        &mut self,
        path: &Path,
        recursive: bool,
        rules: Vec<String>,
    ) -> Result<()> {
        let mode = if recursive {
            RecursiveMode::Recursive
        } else {
            RecursiveMode::NonRecursive
        };

        self.watcher.watch(path, mode)?;
        let canonical = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
        self.watch_rules.insert(canonical, rules);
        info!("Watching: {} (recursive: {})", path.display(), recursive);

        // Initial scan
        self.scan_existing(path, recursive);

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

    /// Process events and apply rules (with debouncing)
    pub fn process_events(&mut self) -> Result<usize> {
        let mut processed = 0;

        for event in self.poll()? {
            debug!("Event: {:?}", event.kind);

            // Only process create and modify events
            match event.kind {
                notify::EventKind::Create(_) | notify::EventKind::Modify(_) => {
                    // Use event handler to debounce
                    let paths_to_process = self.event_handler.should_process(&event);

                    for path in paths_to_process {
                        info!("File event detected: {}", path.display());
                        let allowed = self.allowed_rules_for(&path);
                        match self.engine.process_filtered(&path, allowed) {
                            Ok(true) => processed += 1,
                            Ok(false) => {} // No matching rule
                            Err(e) => {
                                // Skip NotFound errors (file gone between event and processing)
                                if e.downcast_ref::<std::io::Error>().is_some_and(|io_err| {
                                    io_err.kind() == std::io::ErrorKind::NotFound
                                }) {
                                    debug!(
                                        "File disappeared before processing: {}",
                                        path.display()
                                    );
                                    continue;
                                }
                                error!("Rule processing failed for {}: {}", path.display(), e);
                                let rule_name = self.find_matching_rule_name(&path);
                                crate::notifications::notify_rule_error(&rule_name, &e.to_string());
                            }
                        }
                    }
                }
                _ => {
                    debug!("Ignoring event kind: {:?}", event.kind);
                }
            }
        }

        // Periodically clean up old entries
        self.event_handler.cleanup();

        self.files_processed += processed as u64;
        Ok(processed)
    }

    /// Get total number of files processed
    pub fn files_processed(&self) -> u64 {
        self.files_processed
    }

    /// Find the name of the first matching rule for a path
    fn find_matching_rule_name(&self, path: &std::path::Path) -> String {
        for rule in self.engine.rules() {
            if rule.enabled && rule.condition.matches(path).unwrap_or(false) {
                return rule.name.clone();
            }
        }
        "unknown".to_string()
    }

    /// Get the rule engine
    pub fn engine(&self) -> &RuleEngine {
        &self.engine
    }

    /// Find the allowed rules filter for a file path based on which watch directory it belongs to
    fn allowed_rules_for(&self, file_path: &Path) -> Option<&[String]> {
        let canonical =
            std::fs::canonicalize(file_path).unwrap_or_else(|_| file_path.to_path_buf());
        // Find the watch directory that is a prefix of this file path
        let mut best_match: Option<(&std::path::PathBuf, &Vec<String>)> = None;
        for (watch_path, rules) in &self.watch_rules {
            if canonical.starts_with(watch_path) {
                // Pick the longest (most specific) match
                if best_match
                    .is_none_or(|(prev, _)| watch_path.as_os_str().len() > prev.as_os_str().len())
                {
                    best_match = Some((watch_path, rules));
                }
            }
        }
        match best_match {
            Some((_, rules)) if !rules.is_empty() => Some(rules.as_slice()),
            _ => None,
        }
    }

    /// Scan existing files in a watched directory and apply matching rules.
    /// This ensures age-based rules (e.g. "delete after 7 days") catch
    /// files that were already present before the watcher started.
    fn scan_existing(&mut self, path: &Path, recursive: bool) {
        // Get allowed rules for this watch path
        let canonical = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
        let allowed_rules: Option<Vec<String>> = self
            .watch_rules
            .get(&canonical)
            .filter(|r| !r.is_empty())
            .cloned();
        let allowed = allowed_rules.as_deref();

        let entries: Box<dyn Iterator<Item = std::fs::DirEntry>> = if recursive {
            match walkdir(path) {
                Ok(entries) => Box::new(entries.into_iter()),
                Err(e) => {
                    error!("Failed to scan directory {}: {}", path.display(), e);
                    return;
                }
            }
        } else {
            match std::fs::read_dir(path) {
                Ok(rd) => Box::new(rd.filter_map(|e| e.ok())),
                Err(e) => {
                    error!("Failed to scan directory {}: {}", path.display(), e);
                    return;
                }
            }
        };

        let mut scanned = 0u64;
        let mut matched = 0u64;

        for entry in entries {
            let file_path = entry.path();
            if file_path.is_file() {
                scanned += 1;
                match self.engine.process_filtered(&file_path, allowed) {
                    Ok(true) => {
                        matched += 1;
                        self.files_processed += 1;
                    }
                    Ok(false) => {}
                    Err(e) => {
                        error!("Rule processing failed for {}: {}", file_path.display(), e);
                        let rule_name = self.find_matching_rule_name(&file_path);
                        crate::notifications::notify_rule_error(&rule_name, &e.to_string());
                    }
                }
            }
        }

        if scanned > 0 {
            info!(
                "Initial scan of {}: {} files scanned, {} matched rules",
                path.display(),
                scanned,
                matched
            );
        }
    }
}

/// Recursively collect all file entries from a directory tree.
fn walkdir(path: &Path) -> Result<Vec<std::fs::DirEntry>> {
    let mut result = Vec::new();
    walk_recursive(path, &mut result)?;
    Ok(result)
}

fn walk_recursive(path: &Path, result: &mut Vec<std::fs::DirEntry>) -> Result<()> {
    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        let ft = entry.file_type()?;
        if ft.is_symlink() {
            // Skip symlinks to avoid potential loops
            continue;
        }
        if ft.is_dir() {
            walk_recursive(&entry.path(), result)?;
        } else {
            result.push(entry);
        }
    }
    Ok(())
}
