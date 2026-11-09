//! Rule engine - evaluates and executes rules

use anyhow::Result;
use std::path::{Path, PathBuf};
use tracing::{debug, info, trace};

use super::{Action, Rule};

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
        match allowed_rules {
            Some(names) if !names.is_empty() => {
                debug!(
                    "Evaluating filtered rules ({} allowed) for: {}",
                    names.len(),
                    path.display()
                );
                let mut actions = Vec::new();
                for rule in &self.rules {
                    if !rule.enabled {
                        continue;
                    }
                    if !names.iter().any(|n| n == &rule.name) {
                        trace!("Skipping rule '{}' (not in filter)", rule.name);
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
            _ => self.evaluate_all(path),
        }
    }

    /// Evaluate filtered rules and execute all matching actions
    pub fn process_filtered(&self, path: &Path, allowed_rules: Option<&[String]>) -> Result<bool> {
        Ok(self.apply_filtered(path, allowed_rules)?.is_some())
    }

    /// Evaluate filtered rules and execute all matching actions.
    ///
    /// Returns `None` when no rule matched, otherwise the paths the executed actions
    /// created (move, copy, rename and archive destinations).
    pub fn apply_filtered(
        &self,
        path: &Path,
        allowed_rules: Option<&[String]>,
    ) -> Result<Option<Vec<PathBuf>>> {
        let actions = self.evaluate_filtered(path, allowed_rules)?;
        if actions.is_empty() {
            return Ok(None);
        }
        let mut created = Vec::new();
        for action in &actions {
            created.extend(action.execute(path)?);
            // After a destructive action, the file is gone — stop processing
            if matches!(
                action,
                Action::Move { .. } | Action::Rename { .. } | Action::Trash | Action::Delete
            ) {
                break;
            }
        }
        Ok(Some(created))
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

    #[test]
    fn apply_filtered_should_return_the_paths_the_actions_created() {
        let temp = tempfile::tempdir().unwrap();
        let file = temp.path().join("doc.pdf");
        std::fs::write(&file, "x").unwrap();
        let engine = move_pdfs_to(temp.path().join("pdfs"));

        let created = engine.apply_filtered(&file, None).unwrap();

        assert_eq!(
            created,
            Some(vec![temp.path().join("pdfs").join("doc.pdf")])
        );
    }

    #[test]
    fn apply_filtered_should_return_none_when_no_rule_matches() {
        let temp = tempfile::tempdir().unwrap();
        let file = temp.path().join("notes.txt");
        std::fs::write(&file, "x").unwrap();
        let engine = move_pdfs_to(temp.path().join("pdfs"));

        assert_eq!(engine.apply_filtered(&file, None).unwrap(), None);
    }
}
