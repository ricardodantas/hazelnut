//! Which watched folder a path belongs to, and which rules apply there

use std::path::{Path, PathBuf};

use crate::config::WatchConfig;

/// A watched folder and the rule names allowed in it
#[derive(Debug, Clone)]
pub struct WatchScope {
    /// Canonical path of the watched folder
    root: PathBuf,
    /// Whether subfolders are watched too
    recursive: bool,
    /// Allowed rule names (empty = all rules)
    rules: Vec<String>,
}

impl WatchScope {
    /// Canonical path of the watched folder
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Whether subfolders of the watched folder are watched too
    pub fn is_recursive(&self) -> bool {
        self.recursive
    }

    /// Rule names allowed in this watch, or `None` when all rules apply
    pub fn allowed_rules(&self) -> Option<&[String]> {
        Some(self.rules.as_slice()).filter(|rules| !rules.is_empty())
    }
}

/// Maps watched folders to the rules allowed in them.
///
/// A path belongs to the most specific (deepest) watched folder that contains it,
/// so a nested watch overrides the rule list of its parent watch.
#[derive(Debug, Clone, Default)]
pub struct WatchScopes {
    scopes: Vec<WatchScope>,
}

impl WatchScopes {
    /// Create an empty set of watch scopes
    pub fn new() -> Self {
        Self::default()
    }

    /// Build the scopes for every watch in the config (paths are `~`/env-expanded)
    pub fn from_config(watches: &[WatchConfig]) -> Self {
        let mut scopes = Self::new();
        for watch in watches {
            scopes.insert(
                &crate::expand_path(&watch.path),
                watch.recursive,
                watch.rules.clone(),
            );
        }
        scopes
    }

    /// Register a watched folder; registering the same folder again replaces it
    pub fn insert(&mut self, path: &Path, recursive: bool, rules: Vec<String>) {
        let root = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
        let scope = WatchScope {
            root,
            recursive,
            rules,
        };
        match self.scopes.iter_mut().find(|s| s.root == scope.root) {
            Some(existing) => *existing = scope,
            None => self.scopes.push(scope),
        }
    }

    /// Rule names allowed for `path`, or `None` when all rules apply (the path is not
    /// inside any watched folder, or its watch has an empty rule list)
    pub fn allowed_rules_for(&self, path: &Path) -> Option<&[String]> {
        self.find(path).and_then(WatchScope::allowed_rules)
    }

    /// Most specific (deepest) watch containing `path`, which may be the watched
    /// folder itself
    pub fn find(&self, path: &Path) -> Option<&WatchScope> {
        // Watch roots are canonical. Try the path as given first to avoid a syscall,
        // then fall back to resolving symlinks in its parent folder (for example
        // /tmp -> /private/tmp on macOS, or a symlinked watch folder on Linux).
        // Only the parent is resolved so a symlinked file still belongs to the folder
        // it sits in, not to wherever its target lives. Last, resolve the whole path,
        // for a symlink that points at a watched folder.
        self.find_lexical(path)
            .or_else(|| {
                let parent = std::fs::canonicalize(path.parent()?).ok()?;
                self.find_lexical(&parent.join(path.file_name()?))
            })
            .or_else(|| self.find_lexical(&std::fs::canonicalize(path).ok()?))
    }

    fn find_lexical(&self, path: &Path) -> Option<&WatchScope> {
        self.scopes
            .iter()
            .filter(|scope| path.starts_with(&scope.root))
            .max_by_key(|scope| scope.root.as_os_str().len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(names: &[&str]) -> Vec<String> {
        names.iter().map(|n| n.to_string()).collect()
    }

    #[test]
    fn allowed_rules_for_should_return_rules_of_the_containing_watch() {
        let temp = tempfile::tempdir().unwrap();
        let mut scopes = WatchScopes::new();
        scopes.insert(temp.path(), false, names(&["pdfs"]));

        let allowed = scopes.allowed_rules_for(&temp.path().join("doc.pdf"));

        assert_eq!(allowed, Some(names(&["pdfs"]).as_slice()));
    }

    #[test]
    fn allowed_rules_for_should_prefer_the_most_specific_watch() {
        let temp = tempfile::tempdir().unwrap();
        let nested = temp.path().join("nested");
        std::fs::create_dir(&nested).unwrap();
        let mut scopes = WatchScopes::new();
        scopes.insert(&nested, false, names(&["inner"]));
        scopes.insert(temp.path(), false, names(&["outer"]));

        let allowed = scopes.allowed_rules_for(&nested.join("doc.pdf"));

        assert_eq!(allowed, Some(names(&["inner"]).as_slice()));
    }

    #[test]
    fn allowed_rules_for_should_allow_all_rules_when_the_watch_lists_none() {
        let temp = tempfile::tempdir().unwrap();
        let mut scopes = WatchScopes::new();
        scopes.insert(temp.path(), false, Vec::new());

        assert_eq!(scopes.allowed_rules_for(&temp.path().join("doc.pdf")), None);
    }

    #[test]
    fn allowed_rules_for_should_allow_all_rules_outside_every_watch() {
        let temp = tempfile::tempdir().unwrap();
        let watched = temp.path().join("watched");
        std::fs::create_dir(&watched).unwrap();
        let mut scopes = WatchScopes::new();
        scopes.insert(&watched, false, names(&["pdfs"]));

        assert_eq!(scopes.allowed_rules_for(&temp.path().join("doc.pdf")), None);
    }

    #[test]
    fn allowed_rules_for_should_not_match_a_sibling_with_a_common_prefix() {
        let temp = tempfile::tempdir().unwrap();
        let watched = temp.path().join("a");
        let sibling = temp.path().join("ab");
        std::fs::create_dir(&watched).unwrap();
        std::fs::create_dir(&sibling).unwrap();
        let mut scopes = WatchScopes::new();
        scopes.insert(&watched, false, names(&["pdfs"]));

        assert_eq!(scopes.allowed_rules_for(&sibling.join("doc.pdf")), None);
    }

    #[test]
    fn insert_should_replace_the_rules_of_an_already_watched_folder() {
        let temp = tempfile::tempdir().unwrap();
        let mut scopes = WatchScopes::new();
        scopes.insert(temp.path(), false, names(&["old"]));
        scopes.insert(temp.path(), false, names(&["new"]));

        let allowed = scopes.allowed_rules_for(&temp.path().join("doc.pdf"));

        assert_eq!(allowed, Some(names(&["new"]).as_slice()));
    }

    #[cfg(unix)]
    #[test]
    fn allowed_rules_for_should_resolve_a_symlinked_watch_folder() {
        let temp = tempfile::tempdir().unwrap();
        let real = temp.path().join("real");
        let link = temp.path().join("link");
        std::fs::create_dir(&real).unwrap();
        std::os::unix::fs::symlink(&real, &link).unwrap();
        std::fs::write(real.join("doc.pdf"), "x").unwrap();
        let mut scopes = WatchScopes::new();
        scopes.insert(&link, false, names(&["pdfs"]));

        let allowed = scopes.allowed_rules_for(&link.join("doc.pdf"));

        assert_eq!(allowed, Some(names(&["pdfs"]).as_slice()));
    }

    #[cfg(unix)]
    #[test]
    fn allowed_rules_for_should_keep_a_symlinked_file_in_the_folder_it_sits_in() {
        let temp = tempfile::tempdir().unwrap();
        let real = temp.path().join("real");
        let link = temp.path().join("link");
        let elsewhere = temp.path().join("elsewhere");
        std::fs::create_dir(&real).unwrap();
        std::fs::create_dir(&elsewhere).unwrap();
        std::os::unix::fs::symlink(&real, &link).unwrap();
        std::fs::write(elsewhere.join("target.pdf"), "x").unwrap();
        std::os::unix::fs::symlink(elsewhere.join("target.pdf"), real.join("doc.pdf")).unwrap();
        let mut scopes = WatchScopes::new();
        scopes.insert(&link, false, names(&["pdfs"]));

        let allowed = scopes.allowed_rules_for(&link.join("doc.pdf"));

        assert_eq!(allowed, Some(names(&["pdfs"]).as_slice()));
    }

    #[test]
    fn find_should_report_whether_the_containing_watch_is_recursive() {
        let temp = tempfile::tempdir().unwrap();
        let sub = temp.path().join("sub");
        std::fs::create_dir(&sub).unwrap();
        let mut scopes = WatchScopes::new();
        scopes.insert(temp.path(), true, Vec::new());

        let scope = scopes.find(&sub).unwrap();

        assert!(scope.is_recursive());
    }

    #[cfg(unix)]
    #[test]
    fn find_should_match_a_symlink_to_a_watched_folder() {
        let temp = tempfile::tempdir().unwrap();
        let real = temp.path().join("real");
        let link = temp.path().join("link");
        std::fs::create_dir(&real).unwrap();
        std::os::unix::fs::symlink(&real, &link).unwrap();
        let mut scopes = WatchScopes::new();
        scopes.insert(&real, true, Vec::new());

        let root = scopes.find(&link).map(WatchScope::root);

        assert_eq!(root, Some(std::fs::canonicalize(&real).unwrap().as_path()));
    }
}
