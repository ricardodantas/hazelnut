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

/// How long events for a path that hazelnut itself just created are ignored
const OWN_OUTPUT_TTL: Duration = Duration::from_secs(10);

/// Created path -> (root of the watch the original file was in, when it was created)
type CreatedPaths = HashMap<PathBuf, (Option<PathBuf>, Instant)>;

/// Paths recently created by hazelnut's own actions (move, copy, rename and archive
/// destinations), each with the watch the original file was in.
///
/// When a rule moves a file to a folder inside the same recursive watch, the OS
/// reports the new file like any other. Rules must not run on it again: with
/// destinations such as `{dir}/sorted` or `{datetime}` every pass picks a new folder
/// and the file would keep moving. A file moved into a *different* watch is still
/// processed with that watch's rules, so watches can be chained.
///
/// Shared (cheaply cloned) between the event loop and the initial-scan threads.
#[derive(Debug, Clone, Default)]
pub(crate) struct OwnOutputs {
    created: Arc<Mutex<CreatedPaths>>,
}

impl OwnOutputs {
    /// Remember that a rule run on a file from the watch rooted at `watch_root`
    /// created `path`
    pub(crate) fn record(&self, path: &Path, watch_root: Option<&Path>) {
        let now = Instant::now();
        let entry = (watch_root.map(Path::to_path_buf), now);
        let mut created = self.lock();
        created.retain(|_, (_, at)| now.duration_since(*at) < OWN_OUTPUT_TTL);
        // Event paths may spell the location differently (symlinked folders)
        if let Ok(canonical) = std::fs::canonicalize(path) {
            created.insert(canonical, entry.clone());
        }
        created.insert(path.to_path_buf(), entry);
    }

    /// Whether `path` was just created by a rule run on a file from the same watch
    pub(crate) fn is_own(&self, path: &Path, watch_root: Option<&Path>) -> bool {
        let created = self.lock();
        if created.is_empty() {
            return false;
        }
        let recorded = |p: &Path| {
            created.get(p).is_some_and(|(root, at)| {
                at.elapsed() < OWN_OUTPUT_TTL && root.as_deref() == watch_root
            })
        };
        recorded(path)
            || path
                .parent()
                .and_then(|parent| std::fs::canonicalize(parent).ok())
                .zip(path.file_name())
                .is_some_and(|(parent, name)| recorded(&parent.join(name)))
    }

    fn lock(&self) -> MutexGuard<'_, CreatedPaths> {
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
}
