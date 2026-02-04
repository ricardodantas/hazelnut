//! Rule engine - evaluates and executes rules

use anyhow::Result;
use std::path::Path;
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
    pub fn evaluate(&self, path: &Path) -> Result<Option<Action>> {
        debug!("Evaluating rules for: {}", path.display());

        for rule in &self.rules {
            if !rule.enabled {
                trace!("Skipping disabled rule: {}", rule.name);
                continue;
            }

            if rule.condition.matches(path)? {
                info!("Rule '{}' matched: {}", rule.name, path.display());
                return Ok(Some(rule.action.clone()));
            } else {
                debug!("Rule '{}' did not match: {}", rule.name, path.display());
            }
        }

        debug!("No rules matched for: {}", path.display());
        Ok(None)
    }

    /// Evaluate rules and execute the matching action
    pub fn process(&self, path: &Path) -> Result<bool> {
        if let Some(action) = self.evaluate(path)? {
            action.execute(path)?;
            return Ok(true);
        }
        Ok(false)
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

        let result = engine.evaluate(Path::new("/tmp/test.pdf")).unwrap();
        assert!(result.is_some());

        let result = engine.evaluate(Path::new("/tmp/test.txt")).unwrap();
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

        let result = engine.evaluate(Path::new("/tmp/test.pdf")).unwrap();
        assert!(result.is_none());
    }
}
