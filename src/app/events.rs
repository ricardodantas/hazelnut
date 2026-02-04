//! Event handling for the TUI

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::state::{
    AppState, Mode, RuleEditorField, RuleEditorState, SettingsItem, View, WatchEditorField,
    WatchEditorState,
};
use crate::theme::Theme;

/// Handle a key event and update state
pub fn handle_key(state: &mut AppState, key: KeyEvent) {
    // Handle mode-specific input first
    match state.mode {
        Mode::ThemePicker => {
            handle_theme_picker_key(state, key);
            return;
        }
        Mode::Help => {
            if matches!(key.code, KeyCode::Esc | KeyCode::Char('?') | KeyCode::Enter) {
                state.mode = Mode::Normal;
                state.show_help = false;
            }
            return;
        }
        Mode::Settings => {
            handle_settings_key(state, key);
            return;
        }
        Mode::EditRule | Mode::AddRule => {
            handle_rule_editor_key(state, key);
            return;
        }
        Mode::EditWatch | Mode::AddWatch => {
            handle_watch_editor_key(state, key);
            return;
        }
        Mode::About => {
            handle_about_key(state, key);
            return;
        }
        Mode::UpdateConfirm => {
            handle_update_confirm_key(state, key);
            return;
        }
        Mode::Updating => {
            // No input during update
            return;
        }
        Mode::Normal => {}
    }

    // Legacy help popup support
    if state.show_help {
        if matches!(key.code, KeyCode::Esc | KeyCode::Char('?') | KeyCode::Enter) {
            state.show_help = false;
        }
        return;
    }

    // Global keybindings
    match (key.modifiers, key.code) {
        (KeyModifiers::CONTROL, KeyCode::Char('c'))
        | (KeyModifiers::CONTROL, KeyCode::Char('q')) => {
            state.should_quit = true;
            return;
        }
        (_, KeyCode::Char('q')) => {
            state.should_quit = true;
            return;
        }
        (_, KeyCode::Char('?')) | (_, KeyCode::F(1)) => {
            state.mode = Mode::Help;
            state.show_help = true;
            return;
        }
        (_, KeyCode::Tab) => {
            state.next_view();
            return;
        }
        (KeyModifiers::SHIFT, KeyCode::BackTab) => {
            state.prev_view();
            return;
        }
        // Number keys for quick navigation
        (_, KeyCode::Char('1')) => {
            state.view = View::Dashboard;
            return;
        }
        (_, KeyCode::Char('2')) => {
            state.view = View::Rules;
            return;
        }
        (_, KeyCode::Char('3')) => {
            state.view = View::Watches;
            return;
        }
        (_, KeyCode::Char('4')) => {
            state.view = View::Log;
            return;
        }
        // Theme picker (just 't', like Feedo)
        (_, KeyCode::Char('t')) => {
            // Set picker index to current theme
            state.theme_picker_index = Theme::all()
                .iter()
                .position(|t| *t == state.theme)
                .unwrap_or(0);
            state.mode = Mode::ThemePicker;
            return;
        }
        // Settings dialog
        (_, KeyCode::Char('s')) => {
            state.settings_index = 0;
            state.mode = Mode::Settings;
            return;
        }
        // About dialog
        (_, KeyCode::Char('A')) => {
            state.mode = Mode::About;
            return;
        }
        _ => {}
    }

    // View-specific keybindings
    match state.view {
        View::Dashboard => handle_dashboard_key(state, key),
        View::Rules => handle_rules_key(state, key),
        View::Watches => handle_watches_key(state, key),
        View::Log => handle_log_key(state, key),
    }
}

fn handle_theme_picker_key(state: &mut AppState, key: KeyEvent) {
    let themes = Theme::all();
    let len = themes.len();

    match key.code {
        KeyCode::Esc => {
            // Cancel - restore original theme
            state.mode = Mode::Normal;
        }
        KeyCode::Enter => {
            // Apply selected theme
            let selected_theme = themes[state.theme_picker_index];
            state.theme = selected_theme;
            state.mode = Mode::Normal;
            state.set_status(format!("Theme set to {}", selected_theme.name()));
            // TODO: Save to config file
        }
        KeyCode::Down | KeyCode::Char('j') => {
            state.theme_picker_index = (state.theme_picker_index + 1) % len;
            // Preview theme
            state.theme = themes[state.theme_picker_index];
        }
        KeyCode::Up | KeyCode::Char('k') => {
            state.theme_picker_index = state.theme_picker_index.checked_sub(1).unwrap_or(len - 1);
            // Preview theme
            state.theme = themes[state.theme_picker_index];
        }
        KeyCode::Home | KeyCode::Char('g') => {
            state.theme_picker_index = 0;
            state.theme = themes[state.theme_picker_index];
        }
        KeyCode::End | KeyCode::Char('G') => {
            state.theme_picker_index = len - 1;
            state.theme = themes[state.theme_picker_index];
        }
        _ => {}
    }
}

fn handle_dashboard_key(state: &mut AppState, key: KeyEvent) {
    match key.code {
        KeyCode::Char('r') => state.view = View::Rules,
        KeyCode::Char('w') => state.view = View::Watches,
        KeyCode::Char('l') => state.view = View::Log,
        KeyCode::Char('u') | KeyCode::Char('U') => {
            if state.update_available.is_some() {
                state.mode = Mode::UpdateConfirm;
            }
        }
        _ => {}
    }
}

fn handle_rules_key(state: &mut AppState, key: KeyEvent) {
    let len = state.config.rules.len();

    // Allow adding new rules even if list is empty
    if key.code == KeyCode::Char('n') {
        state.rule_editor = Some(RuleEditorState::new_rule());
        state.mode = Mode::AddRule;
        return;
    }

    if len == 0 {
        return;
    }

    match key.code {
        KeyCode::Up | KeyCode::Char('k') => {
            state.selected_rule = Some(
                state
                    .selected_rule
                    .map(|i| i.saturating_sub(1))
                    .unwrap_or(0),
            );
        }
        KeyCode::Down | KeyCode::Char('j') => {
            state.selected_rule = Some(
                state
                    .selected_rule
                    .map(|i| (i + 1).min(len - 1))
                    .unwrap_or(0),
            );
        }
        KeyCode::Home | KeyCode::Char('g') => {
            state.selected_rule = Some(0);
        }
        KeyCode::End | KeyCode::Char('G') => {
            state.selected_rule = Some(len.saturating_sub(1));
        }
        KeyCode::Enter | KeyCode::Char(' ') => {
            // Toggle rule enabled status
            if let Some(rule) = state.current_rule_mut() {
                rule.enabled = !rule.enabled;
                let name = rule.name.clone();
                let status = if rule.enabled { "enabled" } else { "disabled" };
                state.set_status(format!("Rule '{}' {}", name, status));
                // Save config
                save_config(state);
            }
        }
        KeyCode::Char('e') => {
            // Edit selected rule
            if let Some(idx) = state.selected_rule {
                if let Some(rule) = state.config.rules.get(idx) {
                    state.rule_editor = Some(RuleEditorState::from_rule(idx, rule));
                    state.mode = Mode::EditRule;
                }
            } else {
                state.set_status("Select a rule first");
            }
        }
        KeyCode::Char('d') | KeyCode::Delete => {
            // Delete selected rule
            if let Some(idx) = state.selected_rule {
                if idx < state.config.rules.len() {
                    let rule_name = state.config.rules[idx].name.clone();
                    state.config.rules.remove(idx);

                    // Update selection
                    if state.config.rules.is_empty() {
                        state.selected_rule = None;
                    } else if idx >= state.config.rules.len() {
                        state.selected_rule = Some(state.config.rules.len() - 1);
                    }

                    // Save config
                    save_config(state);
                    state.set_status(format!("Deleted rule '{}'", rule_name));
                }
            } else {
                state.set_status("Select a rule first");
            }
        }
        _ => {}
    }
}

fn handle_watches_key(state: &mut AppState, key: KeyEvent) {
    use super::state::WatchEditorState;

    let len = state.config.watches.len();

    // Get available rule names for the editor
    let available_rules: Vec<String> = state.config.rules.iter().map(|r| r.name.clone()).collect();

    // Allow adding new watches even if list is empty
    if key.code == KeyCode::Char('a') || key.code == KeyCode::Char('n') {
        state.watch_editor = Some(WatchEditorState::new_watch(available_rules));
        state.mode = Mode::AddWatch;
        return;
    }

    if len == 0 {
        return;
    }

    match key.code {
        KeyCode::Up | KeyCode::Char('k') => {
            state.selected_watch = Some(
                state
                    .selected_watch
                    .map(|i| i.saturating_sub(1))
                    .unwrap_or(0),
            );
        }
        KeyCode::Down | KeyCode::Char('j') => {
            state.selected_watch = Some(
                state
                    .selected_watch
                    .map(|i| (i + 1).min(len - 1))
                    .unwrap_or(0),
            );
        }
        KeyCode::Home | KeyCode::Char('g') => {
            state.selected_watch = Some(0);
        }
        KeyCode::End | KeyCode::Char('G') => {
            state.selected_watch = Some(len.saturating_sub(1));
        }
        KeyCode::Char('e') => {
            // Edit selected watch
            if let Some(idx) = state.selected_watch {
                if let Some(watch) = state.config.watches.get(idx) {
                    state.watch_editor =
                        Some(WatchEditorState::from_watch(idx, watch, available_rules));
                    state.mode = Mode::EditWatch;
                }
            } else {
                state.set_status("Select a watch first");
            }
        }
        KeyCode::Char('d') | KeyCode::Delete => {
            // Delete selected watch
            if let Some(idx) = state.selected_watch {
                if idx < state.config.watches.len() {
                    let watch_path = state.config.watches[idx].path.display().to_string();
                    state.config.watches.remove(idx);

                    // Update selection
                    if state.config.watches.is_empty() {
                        state.selected_watch = None;
                    } else if idx >= state.config.watches.len() {
                        state.selected_watch = Some(state.config.watches.len() - 1);
                    }

                    // Save config
                    save_config(state);
                    state.set_status(format!("Deleted watch '{}'", watch_path));
                }
            } else {
                state.set_status("Select a watch first");
            }
        }
        KeyCode::Char('o') | KeyCode::Enter => {
            // Open folder in file manager
            if let Some(i) = state.selected_watch
                && let Some(watch) = state.config.watches.get(i)
            {
                let path = watch.path.display().to_string();
                state.set_status(format!("Would open: {}", path));
            }
        }
        _ => {}
    }
}

fn handle_log_key(state: &mut AppState, key: KeyEvent) {
    let len = state.log_entries.len();

    match key.code {
        KeyCode::Char('c') => {
            state.log_entries.clear();
            state.log_scroll = 0;
            state.set_status("Log cleared");
        }
        KeyCode::Up | KeyCode::Char('k') => {
            state.log_scroll = state.log_scroll.saturating_sub(1);
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if state.log_scroll < len.saturating_sub(1) {
                state.log_scroll += 1;
            }
        }
        KeyCode::PageUp => {
            state.log_scroll = state.log_scroll.saturating_sub(10);
        }
        KeyCode::PageDown => {
            state.log_scroll = (state.log_scroll + 10).min(len.saturating_sub(1));
        }
        KeyCode::Home | KeyCode::Char('g') => {
            state.log_scroll = 0;
        }
        KeyCode::End | KeyCode::Char('G') => {
            state.log_scroll = len.saturating_sub(1);
        }
        _ => {}
    }
}

fn handle_about_key(state: &mut AppState, key: KeyEvent) {
    match key.code {
        KeyCode::Esc | KeyCode::Enter | KeyCode::Char('q') => {
            state.mode = Mode::Normal;
        }
        KeyCode::Char('g') | KeyCode::Char('G') => {
            // Open GitHub repository
            let _ = open::that("https://github.com/ricardodantas/hazelnut");
        }
        _ => {}
    }
}

fn handle_update_confirm_key(state: &mut AppState, key: KeyEvent) {
    match key.code {
        KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('N') => {
            state.mode = Mode::Normal;
        }
        KeyCode::Enter | KeyCode::Char('y') | KeyCode::Char('Y') => {
            // Start the update
            state.mode = Mode::Updating;
            state.update_status = Some("Updating...".to_string());

            // Run update (this blocks, but it's intentional for now)
            match crate::run_update(&state.package_manager) {
                Ok(()) => {
                    state.update_status =
                        Some("Update complete! Please restart hazelnut.".to_string());
                    state.update_available = None;
                }
                Err(e) => {
                    state.update_status = Some(format!("Update failed: {}", e));
                }
            }
            state.mode = Mode::Normal;
        }
        _ => {}
    }
}

fn handle_settings_key(state: &mut AppState, key: KeyEvent) {
    let items = SettingsItem::all();
    let len = items.len();

    match key.code {
        KeyCode::Esc => {
            state.mode = Mode::Normal;
        }
        KeyCode::Up | KeyCode::Char('k') => {
            state.settings_index = state.settings_index.checked_sub(1).unwrap_or(len - 1);
        }
        KeyCode::Down | KeyCode::Char('j') => {
            state.settings_index = (state.settings_index + 1) % len;
        }
        KeyCode::Home | KeyCode::Char('g') => {
            state.settings_index = 0;
        }
        KeyCode::End | KeyCode::Char('G') => {
            state.settings_index = len - 1;
        }
        KeyCode::Enter | KeyCode::Char(' ') => {
            handle_settings_action(state);
        }
        // Quick adjustments with +/- for numeric values
        KeyCode::Char('+') | KeyCode::Char('=') | KeyCode::Right | KeyCode::Char('l') => {
            handle_settings_increment(state, true);
        }
        KeyCode::Char('-') | KeyCode::Left | KeyCode::Char('h') => {
            handle_settings_increment(state, false);
        }
        _ => {}
    }
}

fn handle_settings_action(state: &mut AppState) {
    let items = SettingsItem::all();
    let selected = items[state.settings_index];

    match selected {
        SettingsItem::DaemonControl => {
            toggle_daemon(state);
        }
        SettingsItem::ThemeSelection => {
            // Switch to theme picker
            state.theme_picker_index = Theme::all()
                .iter()
                .position(|t| *t == state.theme)
                .unwrap_or(0);
            state.mode = Mode::ThemePicker;
        }
        SettingsItem::ConfigLocation => {
            // Show config path (read-only, just display it)
            let path = state
                .config_path
                .as_ref()
                .map(|p| p.display().to_string())
                .or_else(|| crate::config::Config::default_path().map(|p| p.display().to_string()))
                .unwrap_or_else(|| "Not set".to_string());
            state.set_status(format!("Config: {}", path));
        }
        SettingsItem::PollingInterval => {
            // Cycle through common values: 1, 2, 5, 10, 30, 60
            let current = state.config.general.polling_interval_secs;
            let intervals = [1, 2, 5, 10, 30, 60];
            let next_idx = intervals.iter().position(|&x| x > current).unwrap_or(0);
            state.config.general.polling_interval_secs = intervals[next_idx];
            state.set_status(format!(
                "Polling interval: {}s",
                state.config.general.polling_interval_secs
            ));
            save_config(state);
        }
        SettingsItem::LogRetention => {
            // Cycle through common values: 100, 500, 1000, 5000, 10000
            let current = state.config.general.log_retention;
            let values = [100, 500, 1000, 5000, 10000];
            let next_idx = values.iter().position(|&x| x > current).unwrap_or(0);
            state.config.general.log_retention = values[next_idx];
            state.set_status(format!(
                "Log retention: {} entries",
                state.config.general.log_retention
            ));
            save_config(state);
        }
        SettingsItem::StartupBehavior => {
            state.config.general.start_daemon_on_launch =
                !state.config.general.start_daemon_on_launch;
            let status = if state.config.general.start_daemon_on_launch {
                "enabled"
            } else {
                "disabled"
            };
            state.set_status(format!("Start daemon on launch: {}", status));
            save_config(state);
        }
        SettingsItem::Notifications => {
            state.config.general.notifications_enabled =
                !state.config.general.notifications_enabled;
            let status = if state.config.general.notifications_enabled {
                "enabled"
            } else {
                "disabled"
            };
            state.set_status(format!("Notifications: {} (coming soon)", status));
            save_config(state);
        }
    }
}

fn handle_settings_increment(state: &mut AppState, increase: bool) {
    let items = SettingsItem::all();
    let selected = items[state.settings_index];

    match selected {
        SettingsItem::PollingInterval => {
            let current = state.config.general.polling_interval_secs;
            let intervals = [1, 2, 5, 10, 30, 60];
            let cur_idx = intervals
                .iter()
                .position(|&x| x >= current)
                .unwrap_or(intervals.len() - 1);
            let new_idx = if increase {
                (cur_idx + 1).min(intervals.len() - 1)
            } else {
                cur_idx.saturating_sub(1)
            };
            state.config.general.polling_interval_secs = intervals[new_idx];
            state.set_status(format!(
                "Polling interval: {}s",
                state.config.general.polling_interval_secs
            ));
            save_config(state);
        }
        SettingsItem::LogRetention => {
            let current = state.config.general.log_retention;
            let values = [100, 500, 1000, 5000, 10000];
            let cur_idx = values
                .iter()
                .position(|&x| x >= current)
                .unwrap_or(values.len() - 1);
            let new_idx = if increase {
                (cur_idx + 1).min(values.len() - 1)
            } else {
                cur_idx.saturating_sub(1)
            };
            state.config.general.log_retention = values[new_idx];
            state.set_status(format!(
                "Log retention: {} entries",
                state.config.general.log_retention
            ));
            save_config(state);
        }
        _ => {
            // For toggle items, just call the action
            handle_settings_action(state);
        }
    }
}

fn toggle_daemon(state: &mut AppState) {
    use std::process::{Command, Stdio};

    // Find hazelnutd binary - check same directory as current executable first
    let daemon_cmd = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|dir| dir.join("hazelnutd")))
        .filter(|p| p.exists())
        .unwrap_or_else(|| std::path::PathBuf::from("hazelnutd"));

    if state.daemon_running {
        // Stop daemon
        match Command::new(&daemon_cmd)
            .args(["stop"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
        {
            Ok(status) if status.success() => {
                state.daemon_running = false;
                state.set_status("Daemon stopped");
            }
            Ok(_) => {
                state.set_status("Failed to stop daemon");
            }
            Err(e) => {
                state.set_status(format!("Error stopping daemon: {}", e));
            }
        }
    } else {
        // Start daemon
        match Command::new(&daemon_cmd)
            .args(["start"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
        {
            Ok(status) if status.success() => {
                state.daemon_running = true;
                state.set_status("Daemon started");
            }
            Ok(_) => {
                state.set_status("Failed to start daemon");
            }
            Err(e) => {
                state.set_status(format!("Error starting daemon: {}", e));
            }
        }
    }
}

fn save_config(state: &mut AppState) {
    if let Err(e) = state.config.save(state.config_path.as_deref()) {
        state.set_status(format!("Failed to save config: {}", e));
    }
}

fn handle_rule_editor_key(state: &mut AppState, key: KeyEvent) {
    let Some(ref mut editor) = state.rule_editor else {
        state.mode = Mode::Normal;
        return;
    };

    match key.code {
        KeyCode::Esc => {
            // Cancel editing
            state.rule_editor = None;
            state.mode = Mode::Normal;
            state.set_status("Cancelled");
        }
        KeyCode::Tab => {
            // Move to next field
            editor.field = editor.field.next();
        }
        KeyCode::BackTab => {
            // Move to previous field
            editor.field = editor.field.prev();
        }
        KeyCode::Enter => {
            // Save the rule
            if editor.name.trim().is_empty() {
                state.set_status("Rule name is required");
                return;
            }

            let rule = editor.to_rule();
            let rule_name = rule.name.clone();

            if let Some(idx) = editor.editing_index {
                // Update existing rule
                if idx < state.config.rules.len() {
                    state.config.rules[idx] = rule;
                    state.set_status(format!("Updated rule '{}'", rule_name));
                }
            } else {
                // Add new rule
                state.config.rules.push(rule);
                state.selected_rule = Some(state.config.rules.len() - 1);
                state.set_status(format!("Created rule '{}'", rule_name));
            }

            // Save to config file
            save_config(state);

            state.rule_editor = None;
            state.mode = Mode::Normal;
        }
        // Handle field-specific input
        _ => {
            handle_rule_editor_field_input(editor, key);
        }
    }
}

fn handle_rule_editor_field_input(editor: &mut RuleEditorState, key: KeyEvent) {
    match editor.field {
        RuleEditorField::Name => handle_text_input(&mut editor.name, key),
        RuleEditorField::Enabled => {
            if matches!(
                key.code,
                KeyCode::Char(' ') | KeyCode::Left | KeyCode::Right
            ) {
                editor.enabled = !editor.enabled;
            }
        }
        RuleEditorField::Extension => handle_text_input(&mut editor.extension, key),
        RuleEditorField::NameGlob => handle_text_input(&mut editor.name_glob, key),
        RuleEditorField::NameRegex => handle_text_input(&mut editor.name_regex, key),
        RuleEditorField::SizeGreater => handle_numeric_input(&mut editor.size_greater, key),
        RuleEditorField::SizeLess => handle_numeric_input(&mut editor.size_less, key),
        RuleEditorField::AgeGreater => handle_numeric_input(&mut editor.age_greater, key),
        RuleEditorField::AgeLess => handle_numeric_input(&mut editor.age_less, key),
        RuleEditorField::IsDirectory => {
            if matches!(
                key.code,
                KeyCode::Char(' ') | KeyCode::Left | KeyCode::Right
            ) {
                editor.is_directory = match editor.is_directory {
                    None => Some(true),
                    Some(true) => Some(false),
                    Some(false) => None,
                };
            }
        }
        RuleEditorField::IsHidden => {
            if matches!(
                key.code,
                KeyCode::Char(' ') | KeyCode::Left | KeyCode::Right
            ) {
                editor.is_hidden = match editor.is_hidden {
                    None => Some(true),
                    Some(true) => Some(false),
                    Some(false) => None,
                };
            }
        }
        RuleEditorField::ActionType => match key.code {
            KeyCode::Char(' ') | KeyCode::Right | KeyCode::Char('l') => {
                editor.action_type = editor.action_type.next();
            }
            KeyCode::Left | KeyCode::Char('h') => {
                editor.action_type = editor.action_type.prev();
            }
            _ => {}
        },
        RuleEditorField::ActionDestination => {
            handle_text_input(&mut editor.action_destination, key)
        }
        RuleEditorField::ActionPattern => handle_text_input(&mut editor.action_pattern, key),
        RuleEditorField::ActionCommand => handle_text_input(&mut editor.action_command, key),
    }
}

fn handle_text_input(text: &mut String, key: KeyEvent) {
    match key.code {
        KeyCode::Char(c) => {
            text.push(c);
        }
        KeyCode::Backspace => {
            text.pop();
        }
        KeyCode::Delete => {
            text.clear();
        }
        _ => {}
    }
}

fn handle_numeric_input(text: &mut String, key: KeyEvent) {
    match key.code {
        KeyCode::Char(c) if c.is_ascii_digit() => {
            text.push(c);
        }
        KeyCode::Backspace => {
            text.pop();
        }
        KeyCode::Delete => {
            text.clear();
        }
        _ => {}
    }
}

fn handle_watch_editor_key(state: &mut AppState, key: KeyEvent) {
    let Some(ref mut editor) = state.watch_editor else {
        state.mode = Mode::Normal;
        return;
    };

    match key.code {
        KeyCode::Esc => {
            // Cancel editing
            state.watch_editor = None;
            state.mode = Mode::Normal;
            state.set_status("Cancelled");
        }
        KeyCode::Tab => {
            // Move to next field
            editor.field = editor.field.next();
        }
        KeyCode::BackTab => {
            // Move to previous field
            editor.field = editor.field.prev();
        }
        KeyCode::Enter => {
            // Save the watch
            if editor.path.trim().is_empty() {
                state.set_status("Path is required");
                return;
            }

            let watch = editor.to_watch();
            let watch_path = watch.path.display().to_string();

            if let Some(idx) = editor.editing_index {
                // Update existing watch
                if idx < state.config.watches.len() {
                    state.config.watches[idx] = watch;
                    state.set_status(format!("Updated watch '{}'", watch_path));
                }
            } else {
                // Add new watch
                state.config.watches.push(watch);
                state.selected_watch = Some(state.config.watches.len() - 1);
                state.set_status(format!("Added watch '{}'", watch_path));
            }

            // Save to config file
            save_config(state);

            state.watch_editor = None;
            state.mode = Mode::Normal;
        }
        // Handle field-specific input
        _ => {
            handle_watch_editor_field_input(editor, key);
        }
    }
}

fn handle_watch_editor_field_input(editor: &mut WatchEditorState, key: KeyEvent) {
    match editor.field {
        WatchEditorField::Path => handle_text_input(&mut editor.path, key),
        WatchEditorField::Recursive => {
            if matches!(
                key.code,
                KeyCode::Char(' ') | KeyCode::Left | KeyCode::Right
            ) {
                editor.recursive = !editor.recursive;
            }
        }
        WatchEditorField::Rules => {
            let rule_count = editor.available_rules.len();
            if rule_count == 0 {
                return;
            }
            match key.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    editor.rules_cursor = editor.rules_cursor.saturating_sub(1);
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if editor.rules_cursor < rule_count.saturating_sub(1) {
                        editor.rules_cursor += 1;
                    }
                }
                KeyCode::Char(' ') => {
                    // Toggle selection for current rule
                    if let Some(rule_name) =
                        editor.available_rules.get(editor.rules_cursor).cloned()
                    {
                        editor.toggle_rule(&rule_name);
                    }
                }
                KeyCode::Char('a') => {
                    // Select all rules
                    editor.rules_filter = editor.available_rules.clone();
                }
                KeyCode::Char('c') => {
                    // Clear all selections (means all rules apply)
                    editor.rules_filter.clear();
                }
                _ => {}
            }
        }
    }
}
