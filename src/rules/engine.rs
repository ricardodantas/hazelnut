//! Rule engine - evaluates and executes rules

use anyhow::Result;
use std::path::{Path, PathBuf};
use tracing::{debug, info, trace};

use super::{Action, Rule};

/// What happened when rules were applied to one file
#[derive(Debug, Default)]
pub struct Applied {
    /// Whether any rule matched
    pub matched: bool,
    /// Names of the rules whose actions ran successfully, in order
    pub rules_run: Vec<String>,
    /// Paths those actions created (move, copy, rename and archive destinations)
    pub created: Vec<PathBuf>,
    /// The error that stopped processing, if any. `rules_run` and `created` still
    /// describe what happened before it.
    pub error: Option<anyhow::Error>,
}

/// Engine for evaluating rules against files
pub struct RuleEngine {
    rules: Vec<Rule>,
}

impl RuleEngine {
    /// Create a new rule engine with the given rules
    pub fn new(rules: Vec<Rule>) -> Self {
        Self { rules }
    }

    /// Evaluate rules for a file and return the first matching action
    pub fn evaluate_first(&self, path: &Path) -> Result<Option<Action>> {
        debug!("Evaluating first matching rule for: {}", path.display());

        for rule in &self.rules {
            if !rule.enabled {
                trace!("Skipping disabled rule: {}", rule.name);
                continue;
            }

            if rule.condition.matches(path)? {
                info!("Rule '{}' matched: {}", rule.name, path.display());
                return Ok(Some(rule.action.clone()));
            }
        }

        debug!("No rules matched for: {}", path.display());
        Ok(None)
    }

    /// Evaluate all matching rules and return all actions (respecting stop_processing)
    pub fn evaluate_all(&self, path: &Path) -> Result<Vec<Action>> {
        debug!("Evaluating all rules for: {}", path.display());

        let mut actions = Vec::new();

        for rule in &self.rules {
            if !rule.enabled {
                continue;
            }

            if rule.condition.matches(path)? {
                info!("Rule '{}' matched: {}", rule.name, path.display());
                actions.push(rule.action.clone());
                if rule.stop_processing {
                    break;
                }
            }
        }

        Ok(actions)
    }

    /// Evaluate only rules whose names are in the allowed list (or all if None)
    pub fn evaluate_filtered(
        &self,
        path: &Path,
        allowed_rules: Option<&[String]>,
    ) -> Result<Vec<Action>> {
        Ok(self
            .matching_rules(path, allowed_rules, &[])?
            .into_iter()
            .map(|rule| rule.action.clone())
            .collect())
    }

    /// Enabled rules matching `path`, limited to `allowed_rules` (all if None or
    /// empty) and leaving out `skip_rules`, up to the first `stop_processing` rule
    fn matching_rules(
        &self,
        path: &Path,
        allowed_rules: Option<&[String]>,
        skip_rules: &[String],
    ) -> Result<Vec<&Rule>> {
        let allowed = allowed_rules.filter(|names| !names.is_empty());
        debug!("Evaluating rules for: {}", path.display());

        let mut matching = Vec::new();
        for rule in &self.rules {
            if !rule.enabled {
                continue;
            }
            if allowed.is_some_and(|names| !names.contains(&rule.name)) {
                trace!("Skipping rule '{}' (not in filter)", rule.name);
                continue;
            }
            if skip_rules.contains(&rule.name) {
                trace!("Skipping rule '{}' (already applied)", rule.name);
                continue;
            }
            if rule.condition.matches(path)? {
                info!("Rule '{}' matched: {}", rule.name, path.display());
                matching.push(rule);
                if rule.stop_processing {
                    break;
                }
            }
        }
        Ok(matching)
    }

    /// Evaluate filtered rules and execute all matching actions
    pub fn process_filtered(&self, path: &Path, allowed_rules: Option<&[String]>) -> Result<bool> {
        let applied = self.apply_filtered(path, allowed_rules, &[]);
        match applied.error {
            Some(e) => Err(e),
            None => Ok(applied.matched),
        }
    }

    /// Evaluate filtered rules (leaving out `skip_rules`) and execute the matching
    /// actions, stopping after the first one that moves, renames or deletes the file
    /// or that fails.
    ///
    /// The result lists what ran before any failure, so callers can still account
    /// for the paths those actions created.
    pub fn apply_filtered(
        &self,
        path: &Path,
        allowed_rules: Option<&[String]>,
        skip_rules: &[String],
    ) -> Applied {
        let mut applied = Applied::default();
        let rules = match self.matching_rules(path, allowed_rules, skip_rules) {
            Ok(rules) => rules,
            Err(e) => {
                applied.error = Some(e);
                return applied;
            }
        };
        applied.matched = !rules.is_empty();

        for rule in rules {
            match rule.action.execute(path) {
                Ok(created) => {
                    applied.rules_run.push(rule.name.clone());
                    applied.created.extend(created);
                }
                Err(e) => {
                    applied.error = Some(e);
                    break;
                }
            }
            // After a destructive action, the file is gone — stop processing
            if matches!(
                rule.action,
                Action::Move { .. } | Action::Rename { .. } | Action::Trash | Action::Delete
            ) {
                break;
            }
        }
        applied
    }

    /// Evaluate rules and execute all matching actions
    pub fn process(&self, path: &Path) -> Result<bool> {
        let actions = self.evaluate_all(path)?;
        if actions.is_empty() {
            return Ok(false);
        }
        for action in &actions {
            action.execute(path)?;
            // After a destructive action, the file is gone — stop processing
            if matches!(
                action,
                Action::Move { .. } | Action::Rename { .. } | Action::Trash | Action::Delete
            ) {
                break;
            }
        }
        Ok(true)
    }

    /// Get all rules
    pub fn rules(&self) -> &[Rule] {
        &self.rules
    }

    /// Get enabled rules only
    pub fn enabled_rules(&self) -> impl Iterator<Item = &Rule> {
        self.rules.iter().filter(|r| r.enabled)
    }

    /// Add a rule
    pub fn add_rule(&mut self, rule: Rule) {
        self.rules.push(rule);
    }

    /// Remove a rule by index
    pub fn remove_rule(&mut self, index: usize) -> Option<Rule> {
        if index < self.rules.len() {
            Some(self.rules.remove(index))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::Condition;
    use std::path::PathBuf;

    #[test]
    fn test_evaluate_matching_rule() {
        let rules = vec![Rule::new(
            "PDFs",
            Condition {
                extension: Some("pdf".to_string()),
                ..Default::default()
            },
            Action::Move {
                destination: PathBuf::from("/tmp/pdfs"),
                create_destination: true,
                overwrite: false,
            },
        )];

        let engine = RuleEngine::new(rules);

        let result = engine.evaluate_first(Path::new("/tmp/test.pdf")).unwrap();
        assert!(result.is_some());

        let result = engine.evaluate_first(Path::new("/tmp/test.txt")).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_disabled_rules_skipped() {
        let rules = vec![Rule {
            name: "Disabled".to_string(),
            enabled: false,
            condition: Condition {
                extension: Some("pdf".to_string()),
                ..Default::default()
            },
            action: Action::Delete,
            stop_processing: false,
        }];

        let engine = RuleEngine::new(rules);

        let result = engine.evaluate_first(Path::new("/tmp/test.pdf")).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_evaluate_filtered_only_allowed_rules() {
        let rules = vec![
            Rule::new(
                "PDFs",
                Condition {
                    extension: Some("pdf".to_string()),
                    ..Default::default()
                },
                Action::Move {
                    destination: PathBuf::from("/tmp/pdfs"),
                    create_destination: true,
                    overwrite: false,
                },
            ),
            Rule::new(
                "Images",
                Condition {
                    extension: Some("png".to_string()),
                    ..Default::default()
                },
                Action::Move {
                    destination: PathBuf::from("/tmp/images"),
                    create_destination: true,
                    overwrite: false,
                },
            ),
        ];

        let engine = RuleEngine::new(rules);

        // With filter allowing only "Images", PDFs should not match
        let filter = vec!["Images".to_string()];
        let result = engine
            .evaluate_filtered(Path::new("/tmp/test.pdf"), Some(&filter))
            .unwrap();
        assert!(result.is_empty());

        // But PNGs should match
        let result = engine
            .evaluate_filtered(Path::new("/tmp/test.png"), Some(&filter))
            .unwrap();
        assert!(!result.is_empty());

        // With None filter, all rules apply
        let result = engine
            .evaluate_filtered(Path::new("/tmp/test.pdf"), None)
            .unwrap();
        assert!(!result.is_empty());

        // With empty filter, all rules apply
        let empty: Vec<String> = vec![];
        let result = engine
            .evaluate_filtered(Path::new("/tmp/test.pdf"), Some(&empty))
            .unwrap();
        assert!(!result.is_empty());
    }

    fn move_pdfs_to(destination: PathBuf) -> RuleEngine {
        RuleEngine::new(vec![Rule::new(
            "PDFs",
            Condition {
                extension: Some("pdf".to_string()),
                ..Default::default()
            },
            Action::Move {
                destination,
                create_destination: true,
                overwrite: false,
            },
        )])
    }

    fn summary(applied: &Applied) -> (bool, Vec<String>, Vec<PathBuf>, bool) {
        (
            applied.matched,
            applied.rules_run.clone(),
            applied.created.clone(),
            applied.error.is_some(),
        )
    }

    #[test]
    fn apply_filtered_should_return_the_rules_run_and_the_paths_they_created() {
        let temp = tempfile::tempdir().unwrap();
        let file = temp.path().join("doc.pdf");
        std::fs::write(&file, "x").unwrap();
        let engine = move_pdfs_to(temp.path().join("pdfs"));

        let applied = engine.apply_filtered(&file, None, &[]);

        assert_eq!(
            summary(&applied),
            (
                true,
                vec!["PDFs".to_string()],
                vec![temp.path().join("pdfs").join("doc.pdf")],
                false
            )
        );
    }

    #[test]
    fn apply_filtered_should_report_no_match_when_no_rule_matches() {
        let temp = tempfile::tempdir().unwrap();
        let file = temp.path().join("notes.txt");
        std::fs::write(&file, "x").unwrap();
        let engine = move_pdfs_to(temp.path().join("pdfs"));

        let applied = engine.apply_filtered(&file, None, &[]);

        assert_eq!(summary(&applied), (false, vec![], vec![], false));
    }

    #[test]
    fn apply_filtered_should_not_run_skipped_rules() {
        let temp = tempfile::tempdir().unwrap();
        let file = temp.path().join("doc.pdf");
        std::fs::write(&file, "x").unwrap();
        let engine = move_pdfs_to(temp.path().join("pdfs"));

        let applied = engine.apply_filtered(&file, None, &["PDFs".to_string()]);

        assert_eq!(summary(&applied), (false, vec![], vec![], false));
    }

    #[test]
    fn apply_filtered_should_keep_the_outputs_of_actions_before_a_failing_one() {
        let temp = tempfile::tempdir().unwrap();
        let file = temp.path().join("doc.pdf");
        let taken = temp.path().join("taken");
        std::fs::write(&file, "x").unwrap();
        std::fs::create_dir(&taken).unwrap();
        std::fs::write(taken.join("doc.pdf"), "already here").unwrap();
        let pdfs = Condition {
            extension: Some("pdf".to_string()),
            ..Default::default()
        };
        let engine = RuleEngine::new(vec![
            Rule::new(
                "Backup",
                pdfs.clone(),
                Action::Copy {
                    destination: temp.path().join("backup"),
                    create_destination: true,
                    overwrite: false,
                },
            ),
            Rule::new(
                "File",
                pdfs,
                Action::Move {
                    destination: taken,
                    create_destination: true,
                    overwrite: false,
                },
            ),
        ]);

        let applied = engine.apply_filtered(&file, None, &[]);

        assert_eq!(
            summary(&applied),
            (
                true,
                vec!["Backup".to_string()],
                vec![temp.path().join("backup").join("doc.pdf")],
                true
            )
        );
    }
}
