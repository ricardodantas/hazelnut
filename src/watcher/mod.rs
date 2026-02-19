//! File system watcher

mod handler;

pub use handler::EventHandler;

use anyhow::Result;
use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher as NotifyWatcher};
use std::path::Path;
use std::sync::mpsc;
use std::time::Duration;
use tracing::{debug, error, info};

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::rules::{Rule, RuleEngine};

/// File system watcher that monitors directories and applies rules
pub struct Watcher {
    watcher: RecommendedWatcher,
    engine: RuleEngine,
    rx: mpsc::Receiver<Result<notify::Event, notify::Error>>,
    event_handler: EventHandler,
    files_processed: Arc<AtomicU64>,
    /// Mapping of watched directory path → allowed rule names (empty = all rules)
    watch_rules: std::collections::HashMap<std::path::PathBuf, Vec<String>>,
    /// Cache of canonical paths for watched directories
    canonical_cache: std::collections::HashMap<std::path::PathBuf, std::path::PathBuf>,
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
            files_processed: Arc::new(AtomicU64::new(0)),
            watch_rules: std::collections::HashMap::new(),
            canonical_cache: std::collections::HashMap::new(),
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
        self.watch_rules.insert(canonical.clone(), rules);
        self.canonical_cache
            .insert(canonical.clone(), canonical.clone());
        info!("Watching: {} (recursive: {})", path.display(), recursive);

        // Initial scan — run in a background thread so TUI startup isn't blocked.
        let scan_path = path.to_path_buf();
        let scan_rules: Arc<Vec<Rule>> = Arc::new(self.engine.rules().to_vec());
        let allowed_rules: Option<Vec<String>> = self
            .watch_rules
            .get(&canonical)
            .filter(|r| !r.is_empty())
            .cloned();
        let counter = Arc::clone(&self.files_processed);
        std::thread::spawn(move || {
            scan_existing_background(&scan_path, recursive, &scan_rules, allowed_rules, counter);
        });

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

    /// Process already-polled events and apply rules (with debouncing)
    pub fn process_polled_events(&mut self, events: Vec<notify::Event>) -> Result<usize> {
        let mut processed = 0;

        for event in events {
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

        self.files_processed
            .fetch_add(processed as u64, Ordering::Relaxed);
        Ok(processed)
    }

    /// Get total number of files processed
    pub fn files_processed(&self) -> u64 {
        self.files_processed.load(Ordering::Relaxed)
    }

    /// Process events and apply rules (polls + processes, convenience method)
    pub fn process_events(&mut self) -> Result<usize> {
        let events = self.poll()?;
        self.process_polled_events(events)
    }

    /// Carry over files_processed count from a previous watcher (e.g. on config reload)
    pub fn carry_over_files_processed(&mut self, old: &Watcher) {
        self.files_processed
            .store(old.files_processed(), Ordering::Relaxed);
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
        // Try matching with the raw event path first to avoid a syscall per event.
        // Watch paths are already canonicalized at registration time.
        let mut best_match: Option<(&std::path::PathBuf, &Vec<String>)> = None;
        for (watch_path, rules) in &self.watch_rules {
            let watch_canonical = self.canonical_cache.get(watch_path).unwrap_or(watch_path);
            if file_path.starts_with(watch_canonical)
                && best_match.is_none_or(|(prev, _)| {
                    watch_canonical.as_os_str().len()
                        > self
                            .canonical_cache
                            .get(prev)
                            .unwrap_or(prev)
                            .as_os_str()
                            .len()
                })
            {
                best_match = Some((watch_path, rules));
            }
        }

        // Fallback: canonicalize the event path only if raw comparison found nothing
        // (handles symlinked event paths).
        if best_match.is_none()
            && let Ok(canonical) = std::fs::canonicalize(file_path)
        {
            for (watch_path, rules) in &self.watch_rules {
                let watch_canonical = self.canonical_cache.get(watch_path).unwrap_or(watch_path);
                if canonical.starts_with(watch_canonical)
                    && best_match.is_none_or(|(prev, _)| {
                        watch_canonical.as_os_str().len()
                            > self
                                .canonical_cache
                                .get(prev)
                                .unwrap_or(prev)
                                .as_os_str()
                                .len()
                    })
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
}

/// Run the initial scan in a background thread so TUI startup isn't blocked.
fn scan_existing_background(
    path: &Path,
    recursive: bool,
    rules: &[Rule],
    allowed_rules: Option<Vec<String>>,
    counter: Arc<AtomicU64>,
) {
    let engine = RuleEngine::new(rules.to_vec());
    let allowed = allowed_rules.as_deref();

    let entries: Box<dyn Iterator<Item = std::fs::DirEntry>> = if recursive {
        match walkdir(path) {
            Ok(entries) => entries,
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
            match engine.process_filtered(&file_path, allowed) {
                Ok(true) => {
                    matched += 1;
                }
                Ok(false) => {}
                Err(e) => {
                    if e.downcast_ref::<std::io::Error>()
                        .is_some_and(|io_err| io_err.kind() == std::io::ErrorKind::NotFound)
                    {
                        debug!(
                            "File disappeared before processing: {}",
                            file_path.display()
                        );
                        continue;
                    }
                    error!("Rule processing failed for {}: {}", file_path.display(), e);
                }
            }
        }
    }

    if scanned > 0 {
        info!(
            "Background scan of {}: {} files scanned, {} matched rules",
            path.display(),
            scanned,
            matched
        );
        counter.fetch_add(matched, Ordering::Relaxed);
    }
}

/// Recursively iterate all file entries from a directory tree.
/// Returns a boxed iterator to avoid collecting into a Vec.
fn walkdir(path: &Path) -> Result<Box<dyn Iterator<Item = std::fs::DirEntry>>> {
    let mut stack = vec![path.to_path_buf()];
    let mut entries = Vec::new();

    // NOTE: We still collect into a Vec internally because returning a true
    // lazy iterator over a recursive directory walk requires either unsafe
    // self-referential types or a crate like `walkdir`. The allocation is
    // bounded by the number of files on disk, which is unavoidable for a
    // full-tree scan. The Box<dyn Iterator> signature keeps the public API
    // ready for a zero-alloc implementation in the future.
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir)? {
            let entry = entry?;
            let ft = entry.file_type()?;
            if ft.is_symlink() {
                continue;
            }
            if ft.is_dir() {
                stack.push(entry.path());
            } else {
                entries.push(entry);
            }
        }
    }
    Ok(Box::new(entries.into_iter()))
}
