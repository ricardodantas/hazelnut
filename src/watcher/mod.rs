//! File system watcher

mod handler;
mod scopes;

pub use handler::EventHandler;
pub use scopes::{WatchScope, WatchScopes};

use anyhow::Result;
use handler::OwnOutputs;
use notify::event::{EventKind, ModifyKind};
use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher as NotifyWatcher};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::thread::JoinHandle;
use std::time::Duration;
use tracing::{debug, error, info, warn};

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::rules::RuleEngine;

/// File system watcher that monitors directories and applies rules
pub struct Watcher {
    watcher: RecommendedWatcher,
    engine: RuleEngine,
    rx: mpsc::Receiver<Result<notify::Event, notify::Error>>,
    event_handler: EventHandler,
    files_processed: Arc<AtomicU64>,
    /// Watched directories and the rule names allowed in each
    scopes: WatchScopes,
    /// Files our own actions just created, so rules don't run on them again
    own_outputs: OwnOutputs,
    /// Initial scans still running in the background
    initial_scans: Vec<JoinHandle<()>>,
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
            scopes: WatchScopes::new(),
            own_outputs: OwnOutputs::default(),
            initial_scans: Vec::new(),
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
        let allowed_rules: Option<Vec<String>> = Some(rules.clone()).filter(|r| !r.is_empty());
        self.scopes.insert(path, recursive, rules);
        let watch_root = self.scopes.find(path).map(|s| s.root().to_path_buf());
        info!("Watching: {} (recursive: {})", path.display(), recursive);

        // Initial scan — run in a background thread so TUI startup isn't blocked.
        let scan = InitialScan {
            path: path.to_path_buf(),
            recursive,
            allowed_rules,
            watch_root,
        };
        let engine = RuleEngine::new(self.engine.rules().to_vec());
        let own_outputs = self.own_outputs.clone();
        let counter = Arc::clone(&self.files_processed);
        self.initial_scans.retain(|scan| !scan.is_finished());
        self.initial_scans.push(std::thread::spawn(move || {
            scan.run(&engine, &own_outputs, &counter);
        }));

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

            // Only process create and modify events. Creates and renames mean the
            // path just appeared in the watched tree.
            let appeared = match event.kind {
                EventKind::Create(_) | EventKind::Modify(ModifyKind::Name(_)) => true,
                EventKind::Modify(_) => false,
                _ => {
                    debug!("Ignoring event kind: {:?}", event.kind);
                    continue;
                }
            };

            // Use event handler to debounce
            for path in self.event_handler.should_process(&event) {
                info!("File event detected: {}", path.display());
                processed += self.process_path(&path);

                if appeared && self.is_new_subfolder_of_recursive_watch(&path) {
                    processed += self.process_folder_contents(&path);
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

    /// Get the rule engine
    pub fn engine(&self) -> &RuleEngine {
        &self.engine
    }

    /// Apply the rules of the path's watch to one path; returns 1 if a rule matched
    fn process_path(&self, path: &Path) -> usize {
        let scope = self.scopes.find(path);
        let watch_root = scope.map(WatchScope::root);

        if self.own_outputs.is_own(path, watch_root) {
            debug!(
                "Skipping {}: just created by a rule in the same watch",
                path.display()
            );
            return 0;
        }
        // Events can arrive late (or be replayed) for files a rule already moved
        if std::fs::symlink_metadata(path).is_err() {
            debug!("File disappeared before processing: {}", path.display());
            return 0;
        }

        let allowed = scope.and_then(WatchScope::allowed_rules);
        usize::from(apply_rules(
            &self.engine,
            &self.own_outputs,
            path,
            allowed,
            watch_root,
            true,
        ))
    }

    /// Whether `path` is a folder below the root of a recursive watch, and not one
    /// our own actions just created
    fn is_new_subfolder_of_recursive_watch(&self, path: &Path) -> bool {
        if !path.is_dir() {
            return false;
        }
        let Some(scope) = self.scopes.find(path) else {
            return false;
        };
        scope.is_recursive()
            && std::fs::canonicalize(path).is_ok_and(|canonical| canonical != scope.root())
            && !self.own_outputs.is_own(path, Some(scope.root()))
    }

    /// Process everything already inside a folder that appeared in a recursive watch.
    ///
    /// A folder moved into the watched tree, or (on Linux) copied or extracted into it
    /// faster than the OS starts watching the new folder, produces an event for the
    /// folder only, never for the files already inside it.
    fn process_folder_contents(&mut self, folder: &Path) -> usize {
        let entries = match list_entries(folder, true) {
            Ok(entries) => entries,
            Err(e) => {
                error!("Failed to scan folder {}: {}", folder.display(), e);
                return 0;
            }
        };
        debug!(
            "Processing {} entries already inside {}",
            entries.len(),
            folder.display()
        );

        let mut processed = 0;
        for entry in entries {
            // If the OS reports these paths after all, don't process them twice
            self.event_handler.mark_seen(&entry);
            processed += self.process_path(&entry);
        }
        processed
    }

    /// Wait until every initial scan has finished
    #[cfg(test)]
    fn wait_for_initial_scans(&mut self) {
        for scan in self.initial_scans.drain(..) {
            scan.join().expect("initial scan thread panicked");
        }
    }
}

/// Apply the allowed rules to one path, and remember the paths the actions created.
/// Returns whether a rule matched.
fn apply_rules(
    engine: &RuleEngine,
    own_outputs: &OwnOutputs,
    path: &Path,
    allowed: Option<&[String]>,
    watch_root: Option<&Path>,
    notify_on_error: bool,
) -> bool {
    match engine.apply_filtered(path, allowed) {
        Ok(Some(created)) => {
            for created_path in &created {
                own_outputs.record(created_path, watch_root);
            }
            true
        }
        Ok(None) => false,
        Err(e) => {
            // Skip NotFound errors (file gone between event and processing)
            if e.downcast_ref::<std::io::Error>()
                .is_some_and(|io_err| io_err.kind() == std::io::ErrorKind::NotFound)
            {
                debug!("File disappeared before processing: {}", path.display());
                return false;
            }
            error!("Rule processing failed for {}: {}", path.display(), e);
            if notify_on_error {
                let rule_name = find_matching_rule_name(engine, path);
                crate::notifications::notify_rule_error(&rule_name, &e.to_string());
            }
            false
        }
    }
}

/// Find the name of the first matching rule for a path
fn find_matching_rule_name(engine: &RuleEngine, path: &Path) -> String {
    for rule in engine.rules() {
        if rule.enabled && rule.condition.matches(path).unwrap_or(false) {
            return rule.name.clone();
        }
    }
    "unknown".to_string()
}

/// The scan of what a watched folder already contains when watching starts
struct InitialScan {
    path: PathBuf,
    recursive: bool,
    allowed_rules: Option<Vec<String>>,
    watch_root: Option<PathBuf>,
}

impl InitialScan {
    /// Run the scan (in a background thread so TUI startup isn't blocked)
    fn run(&self, engine: &RuleEngine, own_outputs: &OwnOutputs, counter: &AtomicU64) {
        let entries = match list_entries(&self.path, self.recursive) {
            Ok(entries) => entries,
            Err(e) => {
                error!("Failed to scan directory {}: {}", self.path.display(), e);
                return;
            }
        };

        let mut scanned = 0u64;
        let mut matched = 0u64;
        for entry in &entries {
            scanned += 1;
            if apply_rules(
                engine,
                own_outputs,
                entry,
                self.allowed_rules.as_deref(),
                self.watch_root.as_deref(),
                false,
            ) {
                matched += 1;
            }
        }

        if scanned > 0 {
            info!(
                "Background scan of {}: {} files scanned, {} matched rules",
                self.path.display(),
                scanned,
                matched
            );
            counter.fetch_add(matched, Ordering::Relaxed);
        }
    }
}

/// List what is inside `dir`: files and folders, including those in subfolders when
/// `recursive` is set.
///
/// The listing is complete before it is returned, so files that rules move or rename
/// while it is being processed are not visited again. When descending, symlinks are
/// skipped (never followed), and a subfolder that can't be read is skipped with a
/// warning rather than aborting the whole walk.
///
/// # Errors
///
/// Returns an error if `dir` itself can't be read.
pub fn list_entries(dir: &Path, recursive: bool) -> std::io::Result<Vec<PathBuf>> {
    if !recursive {
        return std::fs::read_dir(dir)?
            .map(|entry| entry.map(|e| e.path()))
            .collect();
    }

    let mut entries = Vec::new();
    let mut pending = vec![dir.to_path_buf()];
    while let Some(current) = pending.pop() {
        let read_dir = match std::fs::read_dir(&current) {
            Ok(read_dir) => read_dir,
            Err(e) if current != dir => {
                warn!("Skipping unreadable folder {}: {}", current.display(), e);
                continue;
            }
            Err(e) => return Err(e),
        };
        for entry in read_dir {
            let entry = entry?;
            let file_type = entry.file_type()?;
            if file_type.is_symlink() {
                continue;
            }
            if file_type.is_dir() {
                pending.push(entry.path());
            }
            entries.push(entry.path());
        }
    }
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::{Action, Condition, Rule};
    use notify::event::{CreateKind, RenameMode};

    fn move_rule(name: &str, extension: &str, destination: PathBuf) -> Rule {
        Rule::new(
            name,
            Condition {
                extension: Some(extension.to_string()),
                ..Default::default()
            },
            Action::Move {
                destination,
                create_destination: true,
                overwrite: false,
            },
        )
    }

    fn event(kind: EventKind, path: &Path) -> notify::Event {
        notify::Event::new(kind).add_path(path.to_path_buf())
    }

    fn renamed_to(path: &Path) -> notify::Event {
        event(EventKind::Modify(ModifyKind::Name(RenameMode::To)), path)
    }

    /// A watcher whose initial scans have finished, so tests only see the events
    /// they feed it
    fn watcher_for(rules: Vec<Rule>, watches: &[(&Path, bool, &[&str])]) -> Watcher {
        let mut watcher = Watcher::new(RuleEngine::new(rules), 1, 1).unwrap();
        for (path, recursive, allowed) in watches {
            let allowed = allowed.iter().map(|r| r.to_string()).collect();
            watcher.watch_with_rules(path, *recursive, allowed).unwrap();
        }
        watcher.wait_for_initial_scans();
        watcher
    }

    #[test]
    fn folder_appearing_in_a_recursive_watch_should_have_its_files_processed() {
        let temp = tempfile::tempdir().unwrap();
        let watched = temp.path().join("watched");
        let out = temp.path().join("out");
        std::fs::create_dir(&watched).unwrap();
        let mut watcher = watcher_for(
            vec![move_rule("txt", "txt", out.clone())],
            &[(&watched, true, &[])],
        );
        // A folder moved (or, on Linux, quickly copied) into the watch: only the
        // folder itself produces an event, not the files already inside it.
        let folder = watched.join("folder");
        std::fs::create_dir_all(folder.join("deeper")).unwrap();
        std::fs::write(folder.join("deeper").join("note.txt"), "x").unwrap();

        watcher
            .process_polled_events(vec![renamed_to(&folder)])
            .unwrap();

        assert!(out.join("note.txt").exists());
    }

    #[test]
    fn folder_appearing_in_a_non_recursive_watch_should_keep_its_files() {
        let temp = tempfile::tempdir().unwrap();
        let watched = temp.path().join("watched");
        let out = temp.path().join("out");
        std::fs::create_dir(&watched).unwrap();
        let mut watcher = watcher_for(
            vec![move_rule("txt", "txt", out.clone())],
            &[(&watched, false, &[])],
        );
        let folder = watched.join("folder");
        std::fs::create_dir(&folder).unwrap();
        std::fs::write(folder.join("note.txt"), "x").unwrap();

        watcher
            .process_polled_events(vec![event(EventKind::Create(CreateKind::Folder), &folder)])
            .unwrap();

        assert!(folder.join("note.txt").exists());
    }

    #[test]
    fn file_moved_by_a_rule_inside_the_same_recursive_watch_should_not_be_processed_again() {
        let temp = tempfile::tempdir().unwrap();
        let watched = temp.path().join("watched");
        std::fs::create_dir(&watched).unwrap();
        // `{dir}/sorted` resolves to a new folder on every pass, so re-processing the
        // moved file would nest it one level deeper each time.
        let mut watcher = watcher_for(
            vec![move_rule("txt", "txt", PathBuf::from("{dir}/sorted"))],
            &[(&watched, true, &[])],
        );
        let file = watched.join("note.txt");
        std::fs::write(&file, "x").unwrap();
        watcher
            .process_polled_events(vec![event(EventKind::Create(CreateKind::File), &file)])
            .unwrap();
        let sorted = watched.join("sorted");

        // The events the move itself causes
        watcher
            .process_polled_events(vec![
                event(EventKind::Create(CreateKind::Folder), &sorted),
                renamed_to(&sorted.join("note.txt")),
            ])
            .unwrap();

        assert!(sorted.join("note.txt").exists());
    }

    #[test]
    fn file_moved_by_a_rule_into_another_watch_should_get_that_watch_rules() {
        let temp = tempfile::tempdir().unwrap();
        let watched = temp.path().join("watched");
        let inbox = watched.join("inbox");
        let out = temp.path().join("out");
        std::fs::create_dir_all(&inbox).unwrap();
        let mut watcher = watcher_for(
            vec![
                move_rule("to inbox", "pdf", inbox.clone()),
                move_rule("from inbox", "pdf", out.clone()),
            ],
            &[
                (&watched, true, &["to inbox"]),
                (&inbox, false, &["from inbox"]),
            ],
        );
        let file = watched.join("doc.pdf");
        std::fs::write(&file, "x").unwrap();
        watcher
            .process_polled_events(vec![event(EventKind::Create(CreateKind::File), &file)])
            .unwrap();

        watcher
            .process_polled_events(vec![renamed_to(&inbox.join("doc.pdf"))])
            .unwrap();

        assert!(out.join("doc.pdf").exists());
    }

    #[test]
    fn event_for_a_file_that_is_already_gone_should_be_skipped() {
        let temp = tempfile::tempdir().unwrap();
        let watched = temp.path().join("watched");
        std::fs::create_dir(&watched).unwrap();
        let mut watcher = watcher_for(
            vec![move_rule("txt", "txt", temp.path().join("out"))],
            &[(&watched, true, &[])],
        );

        let processed = watcher
            .process_polled_events(vec![event(
                EventKind::Create(CreateKind::File),
                &watched.join("gone.txt"),
            )])
            .unwrap();

        assert_eq!(processed, 0);
    }

    #[test]
    fn list_entries_should_descend_into_subfolders_only_when_recursive() {
        let temp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(temp.path().join("sub")).unwrap();
        std::fs::write(temp.path().join("top.txt"), "x").unwrap();
        std::fs::write(temp.path().join("sub").join("nested.txt"), "x").unwrap();

        let mut flat = list_entries(temp.path(), false).unwrap();
        let mut deep = list_entries(temp.path(), true).unwrap();
        flat.sort();
        deep.sort();

        assert_eq!(
            (flat, deep),
            (
                vec![temp.path().join("sub"), temp.path().join("top.txt")],
                vec![
                    temp.path().join("sub"),
                    temp.path().join("sub").join("nested.txt"),
                    temp.path().join("top.txt"),
                ]
            )
        );
    }
}
