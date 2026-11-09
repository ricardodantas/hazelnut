//! Event handler for file system events

use indexmap::IndexMap;
use notify::Event;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

/// Maximum number of entries in the debounce map before forcing a cleanup
const MAX_DEBOUNCE_ENTRIES: usize = 10_000;

/// Debounces file system events to avoid processing the same file multiple times
pub struct EventHandler {
    /// Recent events by path (IndexMap preserves insertion order for fair cleanup)
    recent: IndexMap<PathBuf, Instant>,

    /// Debounce duration
    debounce: Duration,
}

impl EventHandler {
    /// Create a new event handler with the given debounce duration
    pub fn new(debounce_seconds: u64) -> Self {
        Self {
            recent: IndexMap::new(),
            debounce: Duration::from_secs(debounce_seconds),
        }
    }

    /// Check if an event should be processed (returns true if not recently seen)
    pub fn should_process(&mut self, event: &Event) -> Vec<PathBuf> {
        let now = Instant::now();
        let mut paths_to_process = Vec::new();

        for path in &event.paths {
            let should_process = self
                .recent
                .get(path)
                .map(|&last| now.duration_since(last) > self.debounce)
                .unwrap_or(true);

            if should_process {
                self.recent.insert(path.clone(), now);
                paths_to_process.push(path.clone());
            }
        }

        // If the map has grown too large, force a cleanup
        if self.recent.len() > MAX_DEBOUNCE_ENTRIES {
            self.cleanup();
        }

        paths_to_process
    }

    /// Treat `path` as just processed, so events for it within the debounce window
    /// are dropped
    pub fn mark_seen(&mut self, path: &Path) {
        self.recent.insert(path.to_path_buf(), Instant::now());
    }

    /// Clean up old entries (call periodically)
    pub fn cleanup(&mut self) {
        let now = Instant::now();
        let threshold = self.debounce * 10; // Keep entries for 10x debounce period

        self.recent
            .retain(|_, &mut last| now.duration_since(last) < threshold);
    }
}

/// How long a path hazelnut created stays known, counted from the first event batch
/// that could contain its events
const OWN_OUTPUT_TTL: Duration = Duration::from_secs(10);

/// A path created by one of hazelnut's own actions
#[derive(Debug, Clone)]
struct OwnOutput {
    /// Root of the watch the original file was in
    watch_root: Option<PathBuf>,
    /// Rules that already ran on the original file (and the files it came from)
    rules_applied: Vec<String>,
    /// Start of the first event batch after the path was created. Until then the
    /// entry can't expire, however long the event loop is busy.
    armed_at: Option<Instant>,
}

/// Paths recently created by hazelnut's own actions (move, copy, rename and archive
/// destinations), with the watch the original file was in and the rules that
/// already ran on it.
///
/// When a rule moves a file to a folder inside the same recursive watch, the OS
/// reports the new file like any other. The rules that already ran must not run on
/// it again: with destinations such as `{dir}/sorted` or `{datetime}` every pass
/// picks a new folder and the file would keep moving. Other rules still run, so a
/// rename followed by a move keeps working, and each rule runs at most once per
/// file, which rules out loops. A file moved into a *different* watch gets all of
/// that watch's rules, so watches can be chained.
///
/// Shared (cheaply cloned) between the event loop and the initial-scan threads.
#[derive(Debug, Clone)]
pub(crate) struct OwnOutputs {
    created: Arc<Mutex<HashMap<PathBuf, OwnOutput>>>,
    ttl: Duration,
}

impl Default for OwnOutputs {
    fn default() -> Self {
        Self::with_ttl(OWN_OUTPUT_TTL)
    }
}

impl OwnOutputs {
    fn with_ttl(ttl: Duration) -> Self {
        Self {
            created: Arc::default(),
            ttl,
        }
    }

    /// Remember that `rules_applied` ran on a file from the watch rooted at
    /// `watch_root` and created `path`
    pub(crate) fn record(&self, path: &Path, watch_root: Option<&Path>, rules_applied: &[String]) {
        let entry = OwnOutput {
            watch_root: watch_root.map(Path::to_path_buf),
            rules_applied: rules_applied.to_vec(),
            armed_at: None,
        };
        let mut created = self.lock();
        // Event paths may spell the location differently (symlinked folders)
        if let Ok(canonical) = std::fs::canonicalize(path) {
            created.insert(canonical, entry.clone());
        }
        created.insert(path.to_path_buf(), entry);
    }

    /// Call at the start of each event batch, with the time its events were polled:
    /// starts the clock on paths created since the last batch and forgets expired ones
    pub(crate) fn start_batch(&self, now: Instant) {
        let ttl = self.ttl;
        self.lock().retain(|_, entry| match entry.armed_at {
            None => {
                entry.armed_at = Some(now);
                true
            }
            Some(armed_at) => now.saturating_duration_since(armed_at) < ttl,
        });
    }

    /// The rules that already ran, if a rule run on a file from the same watch just
    /// created `path`. `now` is the start of the current event batch.
    pub(crate) fn rules_applied(
        &self,
        path: &Path,
        watch_root: Option<&Path>,
        now: Instant,
    ) -> Option<Vec<String>> {
        let created = self.lock();
        if created.is_empty() {
            return None;
        }
        let lookup = |p: &Path| {
            created
                .get(p)
                .filter(|entry| {
                    entry.watch_root.as_deref() == watch_root
                        && entry
                            .armed_at
                            .is_none_or(|at| now.saturating_duration_since(at) < self.ttl)
                })
                .map(|entry| entry.rules_applied.clone())
        };
        lookup(path).or_else(|| {
            let parent = std::fs::canonicalize(path.parent()?).ok()?;
            lookup(&parent.join(path.file_name()?))
        })
    }

    fn lock(&self) -> MutexGuard<'_, HashMap<PathBuf, OwnOutput>> {
        // The map stays consistent even if a holder panicked
        self.created.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use notify::EventKind;

    #[test]
    fn test_debounce() {
        let mut handler = EventHandler::new(1);

        let event = Event {
            kind: EventKind::Create(notify::event::CreateKind::File),
            paths: vec![PathBuf::from("/tmp/test.txt")],
            attrs: Default::default(),
        };

        // First event should be processed
        let paths = handler.should_process(&event);
        assert_eq!(paths.len(), 1);

        // Immediate second event should be debounced
        let paths = handler.should_process(&event);
        assert_eq!(paths.len(), 0);
    }

    const TTL: Duration = Duration::from_secs(10);

    fn recorded(path: &Path, root: &Path) -> OwnOutputs {
        let outputs = OwnOutputs::with_ttl(TTL);
        outputs.record(path, Some(root), &["Sort".to_string()]);
        outputs
    }

    #[test]
    fn own_output_should_report_the_rules_already_applied() {
        let outputs = recorded(Path::new("/w/sorted/a.txt"), Path::new("/w"));

        let applied = outputs.rules_applied(
            Path::new("/w/sorted/a.txt"),
            Some(Path::new("/w")),
            Instant::now(),
        );

        assert_eq!(applied, Some(vec!["Sort".to_string()]));
    }

    #[test]
    fn own_output_should_not_apply_to_another_watch() {
        let outputs = recorded(Path::new("/w/inbox/a.txt"), Path::new("/w"));

        let applied = outputs.rules_applied(
            Path::new("/w/inbox/a.txt"),
            Some(Path::new("/w/inbox")),
            Instant::now(),
        );

        assert_eq!(applied, None);
    }

    #[test]
    fn own_output_should_not_expire_before_an_event_batch_has_started() {
        let outputs = recorded(Path::new("/w/sorted/a.txt"), Path::new("/w"));
        let much_later = Instant::now() + TTL * 100;

        let applied = outputs.rules_applied(
            Path::new("/w/sorted/a.txt"),
            Some(Path::new("/w")),
            much_later,
        );

        assert_eq!(applied, Some(vec!["Sort".to_string()]));
    }

    #[test]
    fn own_output_should_expire_once_its_batch_is_older_than_the_ttl() {
        let outputs = recorded(Path::new("/w/sorted/a.txt"), Path::new("/w"));
        let batch = Instant::now();
        outputs.start_batch(batch);

        let applied = outputs.rules_applied(
            Path::new("/w/sorted/a.txt"),
            Some(Path::new("/w")),
            batch + TTL + Duration::from_millis(1),
        );

        assert_eq!(applied, None);
    }

    #[test]
    fn start_batch_should_forget_expired_outputs() {
        let outputs = recorded(Path::new("/w/sorted/a.txt"), Path::new("/w"));
        let batch = Instant::now();
        outputs.start_batch(batch);

        outputs.start_batch(batch + TTL * 2);

        assert!(outputs.lock().is_empty());
    }
}
