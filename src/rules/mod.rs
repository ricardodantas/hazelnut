//! Rule engine - conditions and actions for file organization

mod action;
mod condition;
mod engine;

pub use action::Action;
pub use condition::Condition;
pub use engine::{Applied, RuleEngine};

use serde::{Deserialize, Serialize};

/// A rule that matches files and performs actions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    /// Human-readable name
    pub name: String,

    /// Whether the rule is active
    #[serde(default = "default_enabled")]
    pub enabled: bool,

    /// Conditions to match (all must match)
    #[serde(default)]
    pub condition: Condition,

    /// Action to perform on matched files
    pub action: Action,

    /// Stop processing further rules if this matches
    #[serde(default)]
    pub stop_processing: bool,
}

fn default_enabled() -> bool {
    true
}

impl Rule {
    /// Create a new rule
    pub fn new(name: impl Into<String>, condition: Condition, action: Action) -> Self {
        Self {
            name: name.into(),
            enabled: true,
            condition,
            action,
            stop_processing: false,
        }
    }
}
