//! Application state management

use crate::config::Config;
use crate::rules::{Action, Condition, Rule};
use crate::theme::Theme;
use std::path::PathBuf;

/// Check if the daemon is currently running by checking the PID file
#[cfg(unix)]
fn is_daemon_running() -> bool {
    let pid_file = dirs::runtime_dir()
        .or_else(dirs::state_dir)
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("hazelnutd.pid");

    if let Ok(pid_str) = std::fs::read_to_string(&pid_file) {
        if let Ok(pid) = pid_str.trim().parse::<i32>() {
            // Check if process is running using kill -0
            return unsafe { libc::kill(pid, 0) == 0 };
        }
    }
    false
}

#[cfg(not(unix))]
fn is_daemon_running() -> bool {
    false
}

/// Input mode for the application
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mode {
    /// Normal navigation mode
    #[default]
    Normal,
    /// Theme picker dialog
    ThemePicker,
    /// Help dialog
    Help,
    /// Settings dialog
    Settings,
    /// Editing an existing rule
    EditRule,
    /// Adding a new rule
    AddRule,
    /// Editing an existing watch
    EditWatch,
    /// Adding a new watch
    AddWatch,
    /// About dialog
    About,
}

/// Settings menu items
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsItem {
    DaemonControl,
    ThemeSelection,
    ConfigLocation,
    PollingInterval,
    LogRetention,
    StartupBehavior,
    Notifications,
}

impl SettingsItem {
    pub fn all() -> &'static [SettingsItem] {
        &[
            SettingsItem::DaemonControl,
            SettingsItem::ThemeSelection,
            SettingsItem::ConfigLocation,
            SettingsItem::PollingInterval,
            SettingsItem::LogRetention,
            SettingsItem::StartupBehavior,
            SettingsItem::Notifications,
        ]
    }

    pub fn label(&self) -> &'static str {
        match self {
            SettingsItem::DaemonControl => "Start/Stop Daemon",
            SettingsItem::ThemeSelection => "Theme",
            SettingsItem::ConfigLocation => "Config File",
            SettingsItem::PollingInterval => "Polling Interval",
            SettingsItem::LogRetention => "Log Retention",
            SettingsItem::StartupBehavior => "Start Daemon on Launch",
            SettingsItem::Notifications => "Notifications",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            SettingsItem::DaemonControl => "🔌",
            SettingsItem::ThemeSelection => "🎨",
            SettingsItem::ConfigLocation => "📁",
            SettingsItem::PollingInterval => "⏱",
            SettingsItem::LogRetention => "📋",
            SettingsItem::StartupBehavior => "🚀",
            SettingsItem::Notifications => "🔔",
        }
    }
}

/// Main application state
#[derive(Debug)]
pub struct AppState {
    /// Current view/tab
    pub view: View,

    /// Current input mode
    pub mode: Mode,

    /// Loaded configuration
    pub config: Config,

    /// Path to config file (if specified)
    pub config_path: Option<PathBuf>,

    /// Current theme
    pub theme: Theme,

    /// Index of selected rule (if in rules view)
    pub selected_rule: Option<usize>,

    /// Index of selected watch folder (if in watches view)
    pub selected_watch: Option<usize>,

    /// Activity log entries
    pub log_entries: Vec<LogEntry>,

    /// Whether the app should quit
    pub should_quit: bool,

    /// Status message to display
    pub status_message: Option<String>,

    /// Scroll offset for log view
    pub log_scroll: usize,

    /// Show help popup (deprecated, use mode instead)
    pub show_help: bool,

    /// Animation frame counter
    pub frame: u64,

    /// Theme picker index
    pub theme_picker_index: usize,

    /// Settings dialog selected item index
    pub settings_index: usize,

    /// Whether daemon is currently running
    pub daemon_running: bool,

    /// Rule editor state
    pub rule_editor: Option<RuleEditorState>,

    /// Watch editor state
    pub watch_editor: Option<WatchEditorState>,
}

/// Available views in the TUI
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum View {
    #[default]
    Dashboard,
    Rules,
    Watches,
    Log,
}

/// A log entry for activity tracking
#[derive(Debug, Clone)]
pub struct LogEntry {
    pub timestamp: chrono::DateTime<chrono::Local>,
    pub level: LogLevel,
    pub message: String,
    pub file: Option<PathBuf>,
    pub rule: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Info,
    Success,
    Warning,
    Error,
}

impl AppState {
    /// Create a new application state from config
    pub fn new(config: Config, theme: Theme) -> Self {
        Self::with_config_path(config, theme, None)
    }

    /// Create a new application state from config with a specific config path
    pub fn with_config_path(config: Config, theme: Theme, config_path: Option<PathBuf>) -> Self {
        // Find current theme index
        let theme_picker_index = Theme::all().iter().position(|t| *t == theme).unwrap_or(0);

        let mut state = Self {
            view: View::default(),
            mode: Mode::default(),
            config,
            config_path,
            theme,
            selected_rule: None,
            selected_watch: None,
            log_entries: Vec::new(),
            should_quit: false,
            status_message: None,
            log_scroll: 0,
            show_help: false,
            frame: 0,
            theme_picker_index,
            settings_index: 0,
            daemon_running: is_daemon_running(),
            rule_editor: None,
            watch_editor: None,
        };

        // Add welcome log entries
        state.log(LogLevel::Info, "🧹 Hazelnut started");
        state.log(
            LogLevel::Info,
            format!(
                "Loaded {} rules, {} watch folders",
                state.config.rules.len(),
                state.config.watches.len()
            ),
        );

        state
    }

    /// Get the currently selected rule, if any
    pub fn current_rule(&self) -> Option<&Rule> {
        self.selected_rule.and_then(|i| self.config.rules.get(i))
    }

    /// Get the currently selected rule mutably, if any
    pub fn current_rule_mut(&mut self) -> Option<&mut Rule> {
        self.selected_rule
            .and_then(|i| self.config.rules.get_mut(i))
    }

    /// Add a log entry
    pub fn log(&mut self, level: LogLevel, message: impl Into<String>) {
        self.log_entries.push(LogEntry {
            timestamp: chrono::Local::now(),
            level,
            message: message.into(),
            file: None,
            rule: None,
        });

        // Keep log bounded
        if self.log_entries.len() > 1000 {
            self.log_entries.remove(0);
        }
    }

    /// Set a temporary status message
    pub fn set_status(&mut self, message: impl Into<String>) {
        self.status_message = Some(message.into());
    }

    /// Clear the status message
    pub fn clear_status(&mut self) {
        self.status_message = None;
    }

    /// Navigate to the next view
    pub fn next_view(&mut self) {
        self.view = match self.view {
            View::Dashboard => View::Rules,
            View::Rules => View::Watches,
            View::Watches => View::Log,
            View::Log => View::Dashboard,
        };
    }

    /// Navigate to the previous view
    pub fn prev_view(&mut self) {
        self.view = match self.view {
            View::Dashboard => View::Log,
            View::Rules => View::Dashboard,
            View::Watches => View::Rules,
            View::Log => View::Watches,
        };
    }

    /// Increment frame counter (for animations)
    pub fn tick(&mut self) {
        self.frame = self.frame.wrapping_add(1);
    }
}

/// Fields in the rule editor
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RuleEditorField {
    #[default]
    Name,
    Enabled,
    // Conditions
    Extension,
    NameGlob,
    NameRegex,
    SizeGreater,
    SizeLess,
    AgeGreater,
    AgeLess,
    IsDirectory,
    IsHidden,
    // Action
    ActionType,
    ActionDestination,
    ActionPattern,
    ActionCommand,
}

impl RuleEditorField {
    /// Get the next field in tab order
    pub fn next(self) -> Self {
        match self {
            Self::Name => Self::Enabled,
            Self::Enabled => Self::Extension,
            Self::Extension => Self::NameGlob,
            Self::NameGlob => Self::NameRegex,
            Self::NameRegex => Self::SizeGreater,
            Self::SizeGreater => Self::SizeLess,
            Self::SizeLess => Self::AgeGreater,
            Self::AgeGreater => Self::AgeLess,
            Self::AgeLess => Self::IsDirectory,
            Self::IsDirectory => Self::IsHidden,
            Self::IsHidden => Self::ActionType,
            Self::ActionType => Self::ActionDestination,
            Self::ActionDestination => Self::ActionPattern,
            Self::ActionPattern => Self::ActionCommand,
            Self::ActionCommand => Self::Name,
        }
    }

    /// Get the previous field in tab order
    pub fn prev(self) -> Self {
        match self {
            Self::Name => Self::ActionCommand,
            Self::Enabled => Self::Name,
            Self::Extension => Self::Enabled,
            Self::NameGlob => Self::Extension,
            Self::NameRegex => Self::NameGlob,
            Self::SizeGreater => Self::NameRegex,
            Self::SizeLess => Self::SizeGreater,
            Self::AgeGreater => Self::SizeLess,
            Self::AgeLess => Self::AgeGreater,
            Self::IsDirectory => Self::AgeLess,
            Self::IsHidden => Self::IsDirectory,
            Self::ActionType => Self::IsHidden,
            Self::ActionDestination => Self::ActionType,
            Self::ActionPattern => Self::ActionDestination,
            Self::ActionCommand => Self::ActionPattern,
        }
    }
}

/// Available action types for the editor
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ActionTypeSelection {
    #[default]
    Move,
    Copy,
    Rename,
    Trash,
    Delete,
    Run,
    Archive,
    Nothing,
}

impl ActionTypeSelection {
    pub fn all() -> &'static [Self] {
        &[
            Self::Move,
            Self::Copy,
            Self::Rename,
            Self::Trash,
            Self::Delete,
            Self::Run,
            Self::Archive,
            Self::Nothing,
        ]
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Move => "Move",
            Self::Copy => "Copy",
            Self::Rename => "Rename",
            Self::Trash => "Trash",
            Self::Delete => "Delete",
            Self::Run => "Run Command",
            Self::Archive => "Archive",
            Self::Nothing => "Nothing",
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::Move => Self::Copy,
            Self::Copy => Self::Rename,
            Self::Rename => Self::Trash,
            Self::Trash => Self::Delete,
            Self::Delete => Self::Run,
            Self::Run => Self::Archive,
            Self::Archive => Self::Nothing,
            Self::Nothing => Self::Move,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Self::Move => Self::Nothing,
            Self::Copy => Self::Move,
            Self::Rename => Self::Copy,
            Self::Trash => Self::Rename,
            Self::Delete => Self::Trash,
            Self::Run => Self::Delete,
            Self::Archive => Self::Run,
            Self::Nothing => Self::Archive,
        }
    }
}

/// Fields in the watch editor dialog
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WatchEditorField {
    #[default]
    Path,
    Recursive,
    Rules,
}

impl WatchEditorField {
    pub fn next(self) -> Self {
        match self {
            Self::Path => Self::Recursive,
            Self::Recursive => Self::Rules,
            Self::Rules => Self::Path,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Self::Path => Self::Rules,
            Self::Recursive => Self::Path,
            Self::Rules => Self::Recursive,
        }
    }
}

/// State for the watch editor dialog
#[derive(Debug, Clone, Default)]
pub struct WatchEditorState {
    /// Currently focused field
    pub field: WatchEditorField,

    /// Watch index being edited (None if adding new)
    pub editing_index: Option<usize>,

    /// Path to watch
    pub path: String,

    /// Watch recursively
    pub recursive: bool,

    /// Selected rule names (empty = all rules apply)
    pub rules_filter: Vec<String>,

    /// All available rule names (for display in selector)
    pub available_rules: Vec<String>,

    /// Currently highlighted rule in the rules list
    pub rules_cursor: usize,
}

impl WatchEditorState {
    /// Create a new watch editor for adding
    pub fn new_watch(available_rules: Vec<String>) -> Self {
        Self {
            field: WatchEditorField::Path,
            editing_index: None,
            path: String::new(),
            recursive: false,
            rules_filter: Vec::new(),
            available_rules,
            rules_cursor: 0,
        }
    }

    /// Create editor state from an existing watch
    pub fn from_watch(
        index: usize,
        watch: &crate::config::WatchConfig,
        available_rules: Vec<String>,
    ) -> Self {
        Self {
            field: WatchEditorField::Path,
            editing_index: Some(index),
            path: watch.path.display().to_string(),
            recursive: watch.recursive,
            rules_filter: watch.rules.clone(),
            available_rules,
            rules_cursor: 0,
        }
    }

    /// Check if a rule is selected
    pub fn is_rule_selected(&self, rule_name: &str) -> bool {
        self.rules_filter.contains(&rule_name.to_string())
    }

    /// Toggle a rule selection
    pub fn toggle_rule(&mut self, rule_name: &str) {
        if let Some(pos) = self.rules_filter.iter().position(|r| r == rule_name) {
            self.rules_filter.remove(pos);
        } else {
            self.rules_filter.push(rule_name.to_string());
        }
    }

    /// Build a WatchConfig from the editor state
    pub fn to_watch(&self) -> crate::config::WatchConfig {
        crate::config::WatchConfig {
            path: std::path::PathBuf::from(&self.path),
            recursive: self.recursive,
            rules: self.rules_filter.clone(),
        }
    }
}

/// State for the rule editor dialog
#[derive(Debug, Clone, Default)]
pub struct RuleEditorState {
    /// Currently focused field
    pub field: RuleEditorField,

    /// Rule index being edited (None if adding new)
    pub editing_index: Option<usize>,

    // Basic fields
    pub name: String,
    pub enabled: bool,
    pub stop_processing: bool,

    // Condition fields
    pub extension: String,
    pub name_glob: String,
    pub name_regex: String,
    pub size_greater: String,
    pub size_less: String,
    pub age_greater: String,
    pub age_less: String,
    pub is_directory: Option<bool>,
    pub is_hidden: Option<bool>,

    // Action fields
    pub action_type: ActionTypeSelection,
    pub action_destination: String,
    pub action_pattern: String,
    pub action_command: String,
    pub action_args: String,
    pub action_overwrite: bool,
    pub action_delete_original: bool,
}

impl RuleEditorState {
    /// Create a new empty editor state for adding a rule
    pub fn new_rule() -> Self {
        Self {
            enabled: true,
            ..Default::default()
        }
    }

    /// Create editor state from an existing rule
    pub fn from_rule(index: usize, rule: &Rule) -> Self {
        let (
            action_type,
            action_destination,
            action_pattern,
            action_command,
            action_args,
            action_overwrite,
            action_delete_original,
        ) = match &rule.action {
            Action::Move {
                destination,
                overwrite,
                ..
            } => (
                ActionTypeSelection::Move,
                destination.display().to_string(),
                String::new(),
                String::new(),
                String::new(),
                *overwrite,
                false,
            ),
            Action::Copy {
                destination,
                overwrite,
                ..
            } => (
                ActionTypeSelection::Copy,
                destination.display().to_string(),
                String::new(),
                String::new(),
                String::new(),
                *overwrite,
                false,
            ),
            Action::Rename { pattern } => (
                ActionTypeSelection::Rename,
                String::new(),
                pattern.clone(),
                String::new(),
                String::new(),
                false,
                false,
            ),
            Action::Trash => (
                ActionTypeSelection::Trash,
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                false,
                false,
            ),
            Action::Delete => (
                ActionTypeSelection::Delete,
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                false,
                false,
            ),
            Action::Run { command, args } => (
                ActionTypeSelection::Run,
                String::new(),
                String::new(),
                command.clone(),
                args.join(" "),
                false,
                false,
            ),
            Action::Archive {
                destination,
                delete_original,
            } => (
                ActionTypeSelection::Archive,
                destination
                    .as_ref()
                    .map(|p| p.display().to_string())
                    .unwrap_or_default(),
                String::new(),
                String::new(),
                String::new(),
                false,
                *delete_original,
            ),
            Action::Nothing => (
                ActionTypeSelection::Nothing,
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                false,
                false,
            ),
        };

        Self {
            field: RuleEditorField::Name,
            editing_index: Some(index),
            name: rule.name.clone(),
            enabled: rule.enabled,
            stop_processing: rule.stop_processing,
            extension: rule.condition.extension.clone().unwrap_or_default(),
            name_glob: rule.condition.name_matches.clone().unwrap_or_default(),
            name_regex: rule.condition.name_regex.clone().unwrap_or_default(),
            size_greater: rule
                .condition
                .size_greater_than
                .map(|v| v.to_string())
                .unwrap_or_default(),
            size_less: rule
                .condition
                .size_less_than
                .map(|v| v.to_string())
                .unwrap_or_default(),
            age_greater: rule
                .condition
                .age_days_greater_than
                .map(|v| v.to_string())
                .unwrap_or_default(),
            age_less: rule
                .condition
                .age_days_less_than
                .map(|v| v.to_string())
                .unwrap_or_default(),
            is_directory: rule.condition.is_directory,
            is_hidden: rule.condition.is_hidden,
            action_type,
            action_destination,
            action_pattern,
            action_command,
            action_args,
            action_overwrite,
            action_delete_original,
        }
    }

    /// Build a Rule from the editor state
    pub fn to_rule(&self) -> Rule {
        let condition = Condition {
            extension: if self.extension.is_empty() {
                None
            } else {
                Some(self.extension.clone())
            },
            extensions: Vec::new(),
            name_matches: if self.name_glob.is_empty() {
                None
            } else {
                Some(self.name_glob.clone())
            },
            name_regex: if self.name_regex.is_empty() {
                None
            } else {
                Some(self.name_regex.clone())
            },
            size_greater_than: self.size_greater.parse().ok(),
            size_less_than: self.size_less.parse().ok(),
            age_days_greater_than: self.age_greater.parse().ok(),
            age_days_less_than: self.age_less.parse().ok(),
            is_directory: self.is_directory,
            is_hidden: self.is_hidden,
        };

        let action = match self.action_type {
            ActionTypeSelection::Move => Action::Move {
                destination: PathBuf::from(&self.action_destination),
                create_destination: true,
                overwrite: self.action_overwrite,
            },
            ActionTypeSelection::Copy => Action::Copy {
                destination: PathBuf::from(&self.action_destination),
                create_destination: true,
                overwrite: self.action_overwrite,
            },
            ActionTypeSelection::Rename => Action::Rename {
                pattern: self.action_pattern.clone(),
            },
            ActionTypeSelection::Trash => Action::Trash,
            ActionTypeSelection::Delete => Action::Delete,
            ActionTypeSelection::Run => Action::Run {
                command: self.action_command.clone(),
                args: self
                    .action_args
                    .split_whitespace()
                    .map(String::from)
                    .collect(),
            },
            ActionTypeSelection::Archive => Action::Archive {
                destination: if self.action_destination.is_empty() {
                    None
                } else {
                    Some(PathBuf::from(&self.action_destination))
                },
                delete_original: self.action_delete_original,
            },
            ActionTypeSelection::Nothing => Action::Nothing,
        };

        Rule {
            name: self.name.clone(),
            enabled: self.enabled,
            condition,
            action,
            stop_processing: self.stop_processing,
        }
    }
}
