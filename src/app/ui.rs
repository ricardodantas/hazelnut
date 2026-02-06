//! UI rendering for the TUI

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, Paragraph, Tabs, Wrap},
};

use super::state::{
    AppState, LogLevel, Mode, RuleEditorField, SettingsItem, View, WatchEditorField,
};
use crate::config::Config;
use crate::theme::Theme;

/// ASCII art logo for Hazelnut
const LOGO: &str = r#"
██╗  ██╗ █████╗ ███████╗███████╗██╗     ███╗   ██╗██╗   ██╗████████╗
██║  ██║██╔══██╗╚══███╔╝██╔════╝██║     ████╗  ██║██║   ██║╚══██╔══╝
███████║███████║  ███╔╝ █████╗  ██║     ██╔██╗ ██║██║   ██║   ██║   
██╔══██║██╔══██║ ███╔╝  ██╔══╝  ██║     ██║╚██╗██║██║   ██║   ██║   
██║  ██║██║  ██║███████╗███████╗███████╗██║ ╚████║╚██████╔╝   ██║   
╚═╝  ╚═╝╚═╝  ╚═╝╚══════╝╚══════╝╚══════╝╚═╝  ╚═══╝ ╚═════╝    ╚═╝   
"#;

/// Hazelnut icon
const ICON: &str = "🌰";

/// Render the entire UI
pub fn render(frame: &mut Frame, state: &AppState) {
    let colors = state.theme.colors();

    // Set background
    let area = frame.area();
    let bg_block = Block::default().style(Style::default().bg(colors.bg));
    frame.render_widget(bg_block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Tabs
            Constraint::Min(0),    // Main content
            Constraint::Length(1), // Status bar
        ])
        .split(area);

    render_tabs(frame, state, chunks[0]);
    render_main(frame, state, chunks[1]);
    render_status_bar(frame, state, chunks[2]);

    // Render help popup if active
    if state.show_help || state.mode == Mode::Help {
        render_help_popup(frame, state);
    }

    // Render theme picker if active
    if state.mode == Mode::ThemePicker {
        render_theme_picker(frame, state);
    }

    // Render settings dialog if active
    if state.mode == Mode::Settings {
        render_settings_dialog(frame, state);
    }

    // Render rule editor if active
    if matches!(state.mode, Mode::EditRule | Mode::AddRule) {
        render_rule_editor(frame, state);
    }

    // Render watch editor if active
    if matches!(state.mode, Mode::EditWatch | Mode::AddWatch) {
        render_watch_editor(frame, state);
    }

    // Render about dialog if active
    if state.mode == Mode::About {
        render_about_dialog(frame, state);
    }

    // Render update confirmation dialog
    if state.mode == Mode::UpdateConfirm {
        render_update_confirm_dialog(frame, state);
    }

    // Render updating overlay (while update is in progress)
    if state.mode == Mode::Updating {
        render_updating_overlay(frame, state);
    }

    // Render update status message (after completion)
    if state.mode != Mode::Updating
        && let Some(ref status) = state.update_status
    {
        render_update_status(frame, state, status);
    }
}

fn render_tabs(frame: &mut Frame, state: &AppState, area: Rect) {
    let colors = state.theme.colors();

    let titles: Vec<Line> = vec![
        format!(
            "{}  Dashboard",
            if state.view == View::Dashboard {
                "●"
            } else {
                "○"
            }
        ),
        format!(
            "{}  Rules",
            if state.view == View::Rules {
                "●"
            } else {
                "○"
            }
        ),
        format!(
            "{}  Watches",
            if state.view == View::Watches {
                "●"
            } else {
                "○"
            }
        ),
        format!(
            "{}  Log",
            if state.view == View::Log {
                "●"
            } else {
                "○"
            }
        ),
    ]
    .into_iter()
    .map(Line::from)
    .collect();

    let selected = match state.view {
        View::Dashboard => 0,
        View::Rules => 1,
        View::Watches => 2,
        View::Log => 3,
    };

    let tabs = Tabs::new(titles)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(colors.block())
                .title(format!(" {} Hazelnut ", ICON))
                .title_style(colors.logo_style_primary()),
        )
        .select(selected)
        .style(colors.tab())
        .highlight_style(colors.tab_active())
        .divider(Span::styled(" │ ", colors.text_muted()));

    frame.render_widget(tabs, area);
}

fn render_main(frame: &mut Frame, state: &AppState, area: Rect) {
    match state.view {
        View::Dashboard => render_dashboard(frame, state, area),
        View::Rules => render_rules(frame, state, area),
        View::Watches => render_watches(frame, state, area),
        View::Log => render_log(frame, state, area),
    }
}

fn render_dashboard(frame: &mut Frame, state: &AppState, area: Rect) {
    let colors = state.theme.colors();

    // Check if we need to show update banner
    let has_update = state.update_available.is_some();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(if has_update {
            vec![
                Constraint::Length(3), // Update banner
                Constraint::Length(9), // Logo
                Constraint::Min(0),    // Content
            ]
        } else {
            vec![
                Constraint::Length(9), // Logo
                Constraint::Min(0),    // Content
            ]
        })
        .split(area);

    let (logo_area, content_area) = if has_update {
        // Render update banner
        if let Some(ref latest) = state.update_available {
            let pm = &state.package_manager;
            let banner = Paragraph::new(Line::from(vec![
                Span::styled("  ⬆️  ", Style::default().fg(colors.warning)),
                Span::styled("Update available: ", colors.text()),
                Span::styled(
                    format!("v{}", latest),
                    Style::default()
                        .fg(colors.warning)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(" (current: ", colors.text_muted()),
                Span::styled(format!("v{})", crate::VERSION), colors.text_muted()),
                Span::styled(" — Press ", colors.text_muted()),
                Span::styled(
                    "[U]",
                    Style::default()
                        .fg(colors.primary)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(format!(" to update via {}", pm.name()), colors.text_muted()),
            ]))
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(colors.warning)),
            );
            frame.render_widget(banner, chunks[0]);
        }
        (chunks[1], chunks[2])
    } else {
        (chunks[0], chunks[1])
    };

    // Logo
    let logo_lines: Vec<Line> = LOGO
        .lines()
        .enumerate()
        .map(|(i, line)| {
            // Gradient effect
            let style = if i % 2 == 0 {
                colors.logo_style_primary()
            } else {
                colors.logo_style_secondary()
            };
            Line::styled(line, style)
        })
        .collect();

    let logo = Paragraph::new(logo_lines)
        .alignment(Alignment::Center)
        .block(Block::default());
    frame.render_widget(logo, logo_area);

    // Content area
    let content_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(content_area);

    // Left: Stats
    let enabled_rules = state.config.rules.iter().filter(|r| r.enabled).count();
    let total_rules = state.config.rules.len();
    let watch_count = state.config.watches.len();

    let stats_content = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  📁 Watch Folders:  ", colors.text_dim()),
            Span::styled(watch_count.to_string(), colors.text_primary()),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  📋 Total Rules:    ", colors.text_dim()),
            Span::styled(total_rules.to_string(), colors.text_primary()),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  ✓  Active Rules:   ", colors.text_dim()),
            Span::styled(enabled_rules.to_string(), colors.text_success()),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  ✗  Disabled:       ", colors.text_dim()),
            Span::styled(
                (total_rules - enabled_rules).to_string(),
                colors.text_muted(),
            ),
        ]),
        Line::from(""),
        Line::from(""),
        Line::from(vec![
            Span::styled("  🔌 Daemon:         ", colors.text_dim()),
            if state.daemon_running {
                Span::styled("Running", colors.text_success())
            } else {
                Span::styled("Not connected", colors.text_error())
            },
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  🎨 Theme:          ", colors.text_dim()),
            Span::styled(state.theme.name(), colors.text_secondary()),
        ]),
    ];

    let stats = Paragraph::new(stats_content).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(colors.block())
            .title(" Statistics ")
            .title_style(colors.text_primary()),
    );
    frame.render_widget(stats, content_chunks[0]);

    // Right: Quick Actions
    let actions_content = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled("[r]", colors.key_hint()),
            Span::styled(" View & manage rules", colors.text()),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled("[w]", colors.key_hint()),
            Span::styled(" Watch folders", colors.text()),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled("[l]", colors.key_hint()),
            Span::styled(" Activity log", colors.text()),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled("[s]", colors.key_hint()),
            Span::styled(" Settings", colors.text()),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled("[?]", colors.key_hint()),
            Span::styled(" Help & keybindings", colors.text()),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled("[t]", colors.key_hint()),
            Span::styled(" Change theme", colors.text()),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled("[q]", colors.key_hint()),
            Span::styled(" Quit", colors.text()),
        ]),
    ];

    let actions = Paragraph::new(actions_content).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(colors.block())
            .title(" Quick Actions ")
            .title_style(colors.text_primary()),
    );
    frame.render_widget(actions, content_chunks[1]);
}

fn render_rules(frame: &mut Frame, state: &AppState, area: Rect) {
    let colors = state.theme.colors();

    if state.config.rules.is_empty() {
        let empty = Paragraph::new(vec![
            Line::from(""),
            Line::from(""),
            Line::styled("  No rules configured", colors.text_muted()),
            Line::from(""),
            Line::from(vec![
                Span::styled("  Edit ", colors.text_dim()),
                Span::styled("~/.config/hazelnut/config.toml", colors.text_primary()),
                Span::styled(" to add rules", colors.text_dim()),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("  Or press ", colors.text_dim()),
                Span::styled("[n]", colors.key_hint()),
                Span::styled(" to create a new rule", colors.text_dim()),
            ]),
        ])
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(colors.block())
                .title(" Rules ")
                .title_style(colors.text_primary()),
        );
        frame.render_widget(empty, area);
        return;
    }

    let items: Vec<ListItem> = state
        .config
        .rules
        .iter()
        .enumerate()
        .map(|(i, rule)| {
            let (status_icon, status_style) = if rule.enabled {
                ("✓", colors.text_success())
            } else {
                ("✗", colors.text_error())
            };

            let is_selected = state.selected_rule == Some(i);
            let base_style = if is_selected {
                colors.selected()
            } else {
                colors.text()
            };

            // Build the rule line
            let action_preview = match &rule.action {
                crate::rules::Action::Move { destination, .. } => {
                    format!("→ {}", destination.display())
                }
                crate::rules::Action::Copy { destination, .. } => {
                    format!("⇒ {}", destination.display())
                }
                crate::rules::Action::Rename { pattern } => format!("✎ {}", pattern),
                crate::rules::Action::Trash => "🗑 Trash".to_string(),
                crate::rules::Action::Delete => "⚠ Delete".to_string(),
                crate::rules::Action::Run { command, .. } => format!("$ {}", command),
                crate::rules::Action::Archive { .. } => "📦 Archive".to_string(),
                crate::rules::Action::Nothing => "∅ Nothing".to_string(),
            };

            ListItem::new(Line::from(vec![
                Span::styled(format!(" {} ", status_icon), status_style),
                Span::styled(&rule.name, base_style.add_modifier(Modifier::BOLD)),
                Span::styled(format!("  {}", action_preview), colors.text_dim()),
            ]))
            .style(base_style)
        })
        .collect();

    let rules_list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(if state.view == View::Rules {
                    colors.block_focus()
                } else {
                    colors.block()
                })
                .title(format!(" Rules ({}) ", state.config.rules.len()))
                .title_style(colors.text_primary()),
        )
        .highlight_style(colors.selected());

    frame.render_widget(rules_list, area);
}

fn render_watches(frame: &mut Frame, state: &AppState, area: Rect) {
    let colors = state.theme.colors();

    if state.config.watches.is_empty() {
        let empty = Paragraph::new(vec![
            Line::from(""),
            Line::from(""),
            Line::styled("  No watch folders configured", colors.text_muted()),
            Line::from(""),
            Line::from(vec![
                Span::styled("  Edit ", colors.text_dim()),
                Span::styled("~/.config/hazelnut/config.toml", colors.text_primary()),
                Span::styled(" to add folders", colors.text_dim()),
            ]),
        ])
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(colors.block())
                .title(" Watch Folders ")
                .title_style(colors.text_primary()),
        );
        frame.render_widget(empty, area);
        return;
    }

    let items: Vec<ListItem> = state
        .config
        .watches
        .iter()
        .enumerate()
        .map(|(i, watch)| {
            let is_selected = state.selected_watch == Some(i);
            let base_style = if is_selected {
                colors.selected()
            } else {
                colors.text()
            };

            let recursive_indicator = if watch.recursive { " (recursive)" } else { "" };
            let path_str = watch.path.display().to_string();

            // Check if path exists
            let (icon, path_style) = if watch.path.exists() {
                ("📁", colors.text())
            } else {
                ("⚠", colors.text_warning())
            };

            ListItem::new(Line::from(vec![
                Span::styled(format!(" {} ", icon), base_style),
                Span::styled(path_str, path_style),
                Span::styled(recursive_indicator, colors.text_muted()),
            ]))
            .style(base_style)
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(if state.view == View::Watches {
                colors.block_focus()
            } else {
                colors.block()
            })
            .title(format!(" Watch Folders ({}) ", state.config.watches.len()))
            .title_style(colors.text_primary()),
    );

    frame.render_widget(list, area);
}

fn render_log(frame: &mut Frame, state: &AppState, area: Rect) {
    let colors = state.theme.colors();

    if state.log_entries.is_empty() {
        let empty = Paragraph::new(vec![
            Line::from(""),
            Line::from(""),
            Line::styled("  No activity yet", colors.text_muted()),
            Line::from(""),
            Line::styled("  Waiting for file events...", colors.text_dim()),
        ])
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(colors.block())
                .title(" Activity Log ")
                .title_style(colors.text_primary()),
        );
        frame.render_widget(empty, area);
        return;
    }

    let items: Vec<ListItem> = state
        .log_entries
        .iter()
        .rev()
        .map(|entry| {
            let (icon, level_style) = match entry.level {
                LogLevel::Info => ("ℹ", colors.text_info()),
                LogLevel::Success => ("✓", colors.text_success()),
                LogLevel::Warning => ("⚠", colors.text_warning()),
                LogLevel::Error => ("✗", colors.text_error()),
            };

            let time = entry.timestamp.format("%H:%M:%S").to_string();

            ListItem::new(Line::from(vec![
                Span::styled(format!(" {} ", icon), level_style),
                Span::styled(format!("[{}] ", time), colors.text_muted()),
                Span::styled(&entry.message, colors.text()),
            ]))
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(if state.view == View::Log {
                colors.block_focus()
            } else {
                colors.block()
            })
            .title(format!(
                " Activity Log ({}) [c: clear] ",
                state.log_entries.len()
            ))
            .title_style(colors.text_primary()),
    );

    frame.render_widget(list, area);
}

fn render_status_bar(frame: &mut Frame, state: &AppState, area: Rect) {
    let colors = state.theme.colors();

    let content = if let Some(ref msg) = state.status_message {
        vec![
            Span::styled(" ", Style::default()),
            Span::styled(msg, colors.text_secondary()),
        ]
    } else {
        vec![
            Span::styled(" ", Style::default()),
            Span::styled("Tab", colors.key_hint()),
            Span::styled(": views  ", colors.text_muted()),
            Span::styled("?", colors.key_hint()),
            Span::styled(": help  ", colors.text_muted()),
            Span::styled("s", colors.key_hint()),
            Span::styled(": settings  ", colors.text_muted()),
            Span::styled("t", colors.key_hint()),
            Span::styled(": theme  ", colors.text_muted()),
            Span::styled("A", colors.key_hint()),
            Span::styled(": about  ", colors.text_muted()),
            Span::styled("q", colors.key_hint()),
            Span::styled(": quit", colors.text_muted()),
        ]
    };

    let status =
        Paragraph::new(Line::from(content)).style(Style::default().bg(colors.bg_secondary));
    frame.render_widget(status, area);
}

fn render_help_popup(frame: &mut Frame, state: &AppState) {
    let colors = state.theme.colors();
    let area = frame.area();

    // Calculate popup size
    let popup_width = 60u16.min(area.width.saturating_sub(4));
    let popup_height = 32u16.min(area.height.saturating_sub(4));

    let popup_area = Rect {
        x: (area.width - popup_width) / 2,
        y: (area.height - popup_height) / 2,
        width: popup_width,
        height: popup_height,
    };

    // Clear the area
    frame.render_widget(Clear, popup_area);

    let help_content = vec![
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Navigation",
            colors.text_primary().add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![
            Span::styled("  Tab / Shift+Tab    ", colors.key_hint()),
            Span::styled("Switch between views", colors.text()),
        ]),
        Line::from(vec![
            Span::styled("  1-4                ", colors.key_hint()),
            Span::styled("Jump to view directly", colors.text()),
        ]),
        Line::from(vec![
            Span::styled("  j/k or ↑/↓         ", colors.key_hint()),
            Span::styled("Navigate lists", colors.text()),
        ]),
        Line::from(vec![
            Span::styled("  g/G                ", colors.key_hint()),
            Span::styled("Go to first/last item", colors.text()),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Rules View",
            colors.text_primary().add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![
            Span::styled("  Enter/Space        ", colors.key_hint()),
            Span::styled("Toggle rule on/off", colors.text()),
        ]),
        Line::from(vec![
            Span::styled("  e                  ", colors.key_hint()),
            Span::styled("Edit selected rule", colors.text()),
        ]),
        Line::from(vec![
            Span::styled("  n                  ", colors.key_hint()),
            Span::styled("Create new rule", colors.text()),
        ]),
        Line::from(vec![
            Span::styled("  d                  ", colors.key_hint()),
            Span::styled("Delete selected rule", colors.text()),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Watches View",
            colors.text_primary().add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![
            Span::styled("  e                  ", colors.key_hint()),
            Span::styled("Edit selected watch", colors.text()),
        ]),
        Line::from(vec![
            Span::styled("  a/n                ", colors.key_hint()),
            Span::styled("Add new watch", colors.text()),
        ]),
        Line::from(vec![
            Span::styled("  d                  ", colors.key_hint()),
            Span::styled("Delete selected watch", colors.text()),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Dashboard",
            colors.text_primary().add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![
            Span::styled("  D                  ", colors.key_hint()),
            Span::styled("Toggle daemon on/off", colors.text()),
        ]),
        Line::from(vec![
            Span::styled("  U                  ", colors.key_hint()),
            Span::styled("Update hazelnut (if available)", colors.text()),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  General",
            colors.text_primary().add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![
            Span::styled("  s                  ", colors.key_hint()),
            Span::styled("Open settings", colors.text()),
        ]),
        Line::from(vec![
            Span::styled("  t                  ", colors.key_hint()),
            Span::styled("Open theme selector", colors.text()),
        ]),
        Line::from(vec![
            Span::styled("  A                  ", colors.key_hint()),
            Span::styled("About Hazelnut", colors.text()),
        ]),
        Line::from(vec![
            Span::styled("  ?                  ", colors.key_hint()),
            Span::styled("Toggle this help", colors.text()),
        ]),
        Line::from(vec![
            Span::styled("  q / Ctrl+c         ", colors.key_hint()),
            Span::styled("Quit application", colors.text()),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Press ", colors.text_muted()),
            Span::styled("Esc", colors.key_hint()),
            Span::styled(" or ", colors.text_muted()),
            Span::styled("?", colors.key_hint()),
            Span::styled(" to close", colors.text_muted()),
        ]),
    ];

    let help = Paragraph::new(help_content)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(colors.block_focus())
                .style(Style::default().bg(colors.bg_secondary))
                .title(" ⌨ Keyboard Shortcuts ")
                .title_style(colors.text_primary()),
        )
        .wrap(Wrap { trim: false });

    frame.render_widget(help, popup_area);
}

/// Helper to create a centered rect
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_width = r.width * percent_x / 100;
    let popup_height = r.height * percent_y / 100;
    Rect {
        x: r.x + (r.width - popup_width) / 2,
        y: r.y + (r.height - popup_height) / 2,
        width: popup_width,
        height: popup_height,
    }
}

fn render_theme_picker(frame: &mut Frame, state: &AppState) {
    let colors = state.theme.colors();
    let area = frame.area();

    let popup_area = centered_rect(50, 70, area);
    frame.render_widget(Clear, popup_area);

    let themes = Theme::all();
    let items: Vec<ListItem> = themes
        .iter()
        .enumerate()
        .map(|(i, theme_name)| {
            let palette = theme_name.palette();
            let selected = i == state.theme_picker_index;

            // Create color preview squares
            let preview = format!(
                "  {} {} ",
                if selected { "▸" } else { " " },
                theme_name.display_name()
            );

            let style = if selected {
                Style::default()
                    .fg(palette.accent)
                    .bg(palette.selection)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(palette.fg)
            };

            ListItem::new(Line::from(vec![
                Span::styled(preview, style),
                Span::styled("█", Style::default().fg(palette.accent)),
                Span::styled("█", Style::default().fg(palette.secondary)),
                Span::styled("█", Style::default().fg(palette.success)),
                Span::styled("█", Style::default().fg(palette.warning)),
            ]))
        })
        .collect();

    let theme_list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(colors.primary))
            .border_type(BorderType::Rounded)
            .style(Style::default().bg(colors.bg))
            .title(format!(
                " 🎨 Select Theme ({}/{}) ",
                state.theme_picker_index + 1,
                themes.len()
            ))
            .title_bottom(Line::from(" ↑↓ navigate │ ↵ apply │ Esc cancel ").centered()),
    );

    frame.render_widget(theme_list, popup_area);
}

fn render_settings_dialog(frame: &mut Frame, state: &AppState) {
    let colors = state.theme.colors();
    let area = frame.area();

    // Calculate popup size - a bit wider for settings
    let popup_width = 60u16.min(area.width.saturating_sub(4));
    let popup_height = 18u16.min(area.height.saturating_sub(4));

    let popup_area = Rect {
        x: (area.width - popup_width) / 2,
        y: (area.height - popup_height) / 2,
        width: popup_width,
        height: popup_height,
    };

    // Clear the area
    frame.render_widget(Clear, popup_area);

    let items = SettingsItem::all();
    let list_items: Vec<ListItem> = items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let selected = i == state.settings_index;
            let cursor = if selected { "▸" } else { " " };

            let value_str = get_settings_value_display(state, *item);

            let style = if selected {
                colors.selected().add_modifier(Modifier::BOLD)
            } else {
                colors.text()
            };

            let value_style = if selected {
                colors.text_secondary()
            } else {
                colors.text_muted()
            };

            ListItem::new(Line::from(vec![
                Span::styled(format!(" {} ", cursor), style),
                Span::styled(format!("{} ", item.icon()), style),
                Span::styled(format!("{:<24}", item.label()), style),
                Span::styled(value_str, value_style),
            ]))
        })
        .collect();

    let settings_list = List::new(list_items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(colors.primary))
            .border_type(BorderType::Rounded)
            .style(Style::default().bg(colors.bg))
            .title(format!(
                " ⚙ Settings ({}/{}) ",
                state.settings_index + 1,
                items.len()
            ))
            .title_style(colors.text_primary())
            .title_bottom(
                Line::from(" ↑↓ navigate │ ↵/␣ toggle │ ←→ adjust │ Esc close ").centered(),
            ),
    );

    frame.render_widget(settings_list, popup_area);
}

fn get_settings_value_display(state: &AppState, item: SettingsItem) -> String {
    match item {
        SettingsItem::DaemonControl => {
            if state.daemon_running {
                "● Running".to_string()
            } else {
                "○ Stopped".to_string()
            }
        }
        SettingsItem::ThemeSelection => state.theme.name().to_string(),
        SettingsItem::ConfigLocation => {
            state
                .config_path
                .as_ref()
                .map(|p| {
                    // Shorten path for display
                    let s = p.display().to_string();
                    if s.len() > 25 {
                        format!("...{}", &s[s.len() - 22..])
                    } else {
                        s
                    }
                })
                .or_else(|| {
                    Config::default_path().map(|p| {
                        let s = p.display().to_string();
                        if s.len() > 25 {
                            format!("...{}", &s[s.len() - 22..])
                        } else {
                            s
                        }
                    })
                })
                .unwrap_or_else(|| "Not set".to_string())
        }
        SettingsItem::PollingInterval => {
            format!("{}s", state.config.general.polling_interval_secs)
        }
        SettingsItem::LogRetention => {
            format!("{} entries", state.config.general.log_retention)
        }
        SettingsItem::StartupBehavior => {
            if state.config.general.start_daemon_on_launch {
                "✓ Enabled".to_string()
            } else {
                "✗ Disabled".to_string()
            }
        }
        SettingsItem::Notifications => {
            let status = if state.config.general.notifications_enabled {
                "✓ Enabled"
            } else {
                "✗ Disabled"
            };
            format!("{} (coming soon)", status)
        }
    }
}

fn render_rule_editor(frame: &mut Frame, state: &AppState) {
    let colors = state.theme.colors();
    let area = frame.area();

    let Some(ref editor) = state.rule_editor else {
        return;
    };

    // Calculate popup size - wider for the editor
    let popup_width = 70u16.min(area.width.saturating_sub(4));
    let popup_height = 28u16.min(area.height.saturating_sub(4));

    let popup_area = Rect {
        x: (area.width - popup_width) / 2,
        y: (area.height - popup_height) / 2,
        width: popup_width,
        height: popup_height,
    };

    // Clear the area
    frame.render_widget(Clear, popup_area);

    // Helper to render a field
    let field_style = |f: RuleEditorField| {
        if editor.field == f {
            colors.selected().add_modifier(Modifier::BOLD)
        } else {
            colors.text()
        }
    };

    let label_style = |f: RuleEditorField| {
        if editor.field == f {
            colors.text_primary()
        } else {
            colors.text_dim()
        }
    };

    let cursor = |f: RuleEditorField| if editor.field == f { "▸" } else { " " };

    let tri_state_display = |v: Option<bool>| match v {
        None => "Any",
        Some(true) => "Yes",
        Some(false) => "No",
    };

    let title = if state.mode == Mode::EditRule {
        format!(" ✏ Edit Rule: {} ", editor.name)
    } else {
        " ✚ New Rule ".to_string()
    };

    let content = vec![
        Line::from(""),
        // Basic Info Section
        Line::from(vec![Span::styled(
            "  Basic Info",
            colors.text_primary().add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![
            Span::styled(
                format!(" {} ", cursor(RuleEditorField::Name)),
                field_style(RuleEditorField::Name),
            ),
            Span::styled("Name:        ", label_style(RuleEditorField::Name)),
            Span::styled(&editor.name, field_style(RuleEditorField::Name)),
            Span::styled(
                if editor.field == RuleEditorField::Name {
                    "▏"
                } else {
                    ""
                },
                colors.text_primary(),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                format!(" {} ", cursor(RuleEditorField::Enabled)),
                field_style(RuleEditorField::Enabled),
            ),
            Span::styled("Enabled:     ", label_style(RuleEditorField::Enabled)),
            Span::styled(
                if editor.enabled { "✓ Yes" } else { "✗ No" },
                field_style(RuleEditorField::Enabled),
            ),
        ]),
        Line::from(""),
        // Conditions Section
        Line::from(vec![Span::styled(
            "  Conditions",
            colors.text_primary().add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![
            Span::styled(
                format!(" {} ", cursor(RuleEditorField::Extension)),
                field_style(RuleEditorField::Extension),
            ),
            Span::styled("Extension:   ", label_style(RuleEditorField::Extension)),
            Span::styled(
                if editor.extension.is_empty() {
                    "(any)"
                } else {
                    &editor.extension
                },
                field_style(RuleEditorField::Extension),
            ),
            Span::styled(
                if editor.field == RuleEditorField::Extension {
                    "▏"
                } else {
                    ""
                },
                colors.text_primary(),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                format!(" {} ", cursor(RuleEditorField::NameGlob)),
                field_style(RuleEditorField::NameGlob),
            ),
            Span::styled("Name Glob:   ", label_style(RuleEditorField::NameGlob)),
            Span::styled(
                if editor.name_glob.is_empty() {
                    "(any)"
                } else {
                    &editor.name_glob
                },
                field_style(RuleEditorField::NameGlob),
            ),
            Span::styled(
                if editor.field == RuleEditorField::NameGlob {
                    "▏"
                } else {
                    ""
                },
                colors.text_primary(),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                format!(" {} ", cursor(RuleEditorField::NameRegex)),
                field_style(RuleEditorField::NameRegex),
            ),
            Span::styled("Name Regex:  ", label_style(RuleEditorField::NameRegex)),
            Span::styled(
                if editor.name_regex.is_empty() {
                    "(any)"
                } else {
                    &editor.name_regex
                },
                field_style(RuleEditorField::NameRegex),
            ),
            Span::styled(
                if editor.field == RuleEditorField::NameRegex {
                    "▏"
                } else {
                    ""
                },
                colors.text_primary(),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                format!(" {} ", cursor(RuleEditorField::SizeGreater)),
                field_style(RuleEditorField::SizeGreater),
            ),
            Span::styled("Size >:      ", label_style(RuleEditorField::SizeGreater)),
            Span::styled(
                if editor.size_greater.is_empty() {
                    "(any)"
                } else {
                    &editor.size_greater
                },
                field_style(RuleEditorField::SizeGreater),
            ),
            Span::styled(
                if editor.field == RuleEditorField::SizeGreater {
                    "▏"
                } else {
                    ""
                },
                colors.text_primary(),
            ),
            Span::styled(" bytes", colors.text_dim()),
        ]),
        Line::from(vec![
            Span::styled(
                format!(" {} ", cursor(RuleEditorField::SizeLess)),
                field_style(RuleEditorField::SizeLess),
            ),
            Span::styled("Size <:      ", label_style(RuleEditorField::SizeLess)),
            Span::styled(
                if editor.size_less.is_empty() {
                    "(any)"
                } else {
                    &editor.size_less
                },
                field_style(RuleEditorField::SizeLess),
            ),
            Span::styled(
                if editor.field == RuleEditorField::SizeLess {
                    "▏"
                } else {
                    ""
                },
                colors.text_primary(),
            ),
            Span::styled(" bytes", colors.text_dim()),
        ]),
        Line::from(vec![
            Span::styled(
                format!(" {} ", cursor(RuleEditorField::AgeGreater)),
                field_style(RuleEditorField::AgeGreater),
            ),
            Span::styled("Age > days:  ", label_style(RuleEditorField::AgeGreater)),
            Span::styled(
                if editor.age_greater.is_empty() {
                    "(any)"
                } else {
                    &editor.age_greater
                },
                field_style(RuleEditorField::AgeGreater),
            ),
            Span::styled(
                if editor.field == RuleEditorField::AgeGreater {
                    "▏"
                } else {
                    ""
                },
                colors.text_primary(),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                format!(" {} ", cursor(RuleEditorField::AgeLess)),
                field_style(RuleEditorField::AgeLess),
            ),
            Span::styled("Age < days:  ", label_style(RuleEditorField::AgeLess)),
            Span::styled(
                if editor.age_less.is_empty() {
                    "(any)"
                } else {
                    &editor.age_less
                },
                field_style(RuleEditorField::AgeLess),
            ),
            Span::styled(
                if editor.field == RuleEditorField::AgeLess {
                    "▏"
                } else {
                    ""
                },
                colors.text_primary(),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                format!(" {} ", cursor(RuleEditorField::IsDirectory)),
                field_style(RuleEditorField::IsDirectory),
            ),
            Span::styled("Is Dir:      ", label_style(RuleEditorField::IsDirectory)),
            Span::styled(
                tri_state_display(editor.is_directory),
                field_style(RuleEditorField::IsDirectory),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                format!(" {} ", cursor(RuleEditorField::IsHidden)),
                field_style(RuleEditorField::IsHidden),
            ),
            Span::styled("Is Hidden:   ", label_style(RuleEditorField::IsHidden)),
            Span::styled(
                tri_state_display(editor.is_hidden),
                field_style(RuleEditorField::IsHidden),
            ),
        ]),
        Line::from(""),
        // Action Section
        Line::from(vec![Span::styled(
            "  Action",
            colors.text_primary().add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![
            Span::styled(
                format!(" {} ", cursor(RuleEditorField::ActionType)),
                field_style(RuleEditorField::ActionType),
            ),
            Span::styled("Type:        ", label_style(RuleEditorField::ActionType)),
            Span::styled(
                format!("< {} >", editor.action_type.name()),
                field_style(RuleEditorField::ActionType),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                format!(" {} ", cursor(RuleEditorField::ActionDestination)),
                field_style(RuleEditorField::ActionDestination),
            ),
            Span::styled(
                "Destination: ",
                label_style(RuleEditorField::ActionDestination),
            ),
            Span::styled(
                if editor.action_destination.is_empty() {
                    "(none)"
                } else {
                    &editor.action_destination
                },
                field_style(RuleEditorField::ActionDestination),
            ),
            Span::styled(
                if editor.field == RuleEditorField::ActionDestination {
                    "▏"
                } else {
                    ""
                },
                colors.text_primary(),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                format!(" {} ", cursor(RuleEditorField::ActionPattern)),
                field_style(RuleEditorField::ActionPattern),
            ),
            Span::styled("Pattern:     ", label_style(RuleEditorField::ActionPattern)),
            Span::styled(
                if editor.action_pattern.is_empty() {
                    "(none)"
                } else {
                    &editor.action_pattern
                },
                field_style(RuleEditorField::ActionPattern),
            ),
            Span::styled(
                if editor.field == RuleEditorField::ActionPattern {
                    "▏"
                } else {
                    ""
                },
                colors.text_primary(),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                format!(" {} ", cursor(RuleEditorField::ActionCommand)),
                field_style(RuleEditorField::ActionCommand),
            ),
            Span::styled("Command:     ", label_style(RuleEditorField::ActionCommand)),
            Span::styled(
                if editor.action_command.is_empty() {
                    "(none)"
                } else {
                    &editor.action_command
                },
                field_style(RuleEditorField::ActionCommand),
            ),
            Span::styled(
                if editor.field == RuleEditorField::ActionCommand {
                    "▏"
                } else {
                    ""
                },
                colors.text_primary(),
            ),
        ]),
        Line::from(""),
        // Contextual help line
        Line::from(vec![
            Span::styled("  💡 ", colors.text_dim()),
            Span::styled(
                field_help(editor.field),
                colors.text_muted().add_modifier(Modifier::ITALIC),
            ),
        ]),
    ];

    let editor_widget = Paragraph::new(content)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(colors.primary))
                .border_type(BorderType::Rounded)
                .style(Style::default().bg(colors.bg))
                .title(title)
                .title_style(colors.text_primary())
                .title_bottom(
                    Line::from(" Tab: next field │ Enter: save │ Esc: cancel ").centered(),
                ),
        )
        .wrap(Wrap { trim: false });

    frame.render_widget(editor_widget, popup_area);
}

/// Returns contextual help text for each rule editor field
fn field_help(field: RuleEditorField) -> &'static str {
    use RuleEditorField::*;
    match field {
        Name => "Type a descriptive name for this rule",
        Enabled => "Space/←→ to toggle on/off",
        Extension => "e.g. 'pdf', 'jpg' — leave empty for any",
        NameGlob => "Glob pattern, e.g. 'Screenshot*.png' or '*.tmp'",
        NameRegex => "Regex pattern, e.g. '^invoice_\\d+\\.pdf$'",
        SizeGreater => "Type bytes (e.g. 1048576 = 1MB) — files larger than this",
        SizeLess => "Type bytes — files smaller than this",
        AgeGreater => "Type days — files older than this many days",
        AgeLess => "Type days — files newer than this many days",
        IsDirectory => "Space/←→ to cycle: Any → Yes → No",
        IsHidden => "Space/←→ to cycle: Any → Yes → No",
        ActionType => "←→ or Space to change action type",
        ActionDestination => "Target folder path, e.g. ~/Documents/PDFs",
        ActionPattern => "Rename pattern, e.g. '{name}_{date}.{ext}'",
        ActionCommand => "Command to run, e.g. 'convert' or '/usr/bin/script.sh'",
    }
}

fn render_watch_editor(frame: &mut Frame, state: &AppState) {
    let colors = state.theme.colors();
    let area = frame.area();

    let Some(ref editor) = state.watch_editor else {
        return;
    };

    // Calculate popup size - needs more height for rules list
    let rules_count = editor.available_rules.len();
    let base_height = 10u16; // Path, Recursive, help, margins
    let rules_height = (rules_count as u16).clamp(2, 6); // Show up to 6 rules
    let popup_height = (base_height + rules_height).min(area.height.saturating_sub(4));
    let popup_width = 65u16.min(area.width.saturating_sub(4));

    let popup_area = Rect {
        x: (area.width - popup_width) / 2,
        y: (area.height - popup_height) / 2,
        width: popup_width,
        height: popup_height,
    };

    // Clear the area
    frame.render_widget(Clear, popup_area);

    // Helper to render a field
    let field_style = |f: WatchEditorField| {
        if editor.field == f {
            colors.selected().add_modifier(Modifier::BOLD)
        } else {
            colors.text()
        }
    };

    let label_style = |f: WatchEditorField| {
        if editor.field == f {
            colors.text_primary()
        } else {
            colors.text_dim()
        }
    };

    let cursor = |f: WatchEditorField| if editor.field == f { "▸" } else { " " };

    let title = if state.mode == Mode::EditWatch {
        " ✏ Edit Watch ".to_string()
    } else {
        " ✚ New Watch ".to_string()
    };

    let mut content = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(
                format!(" {} ", cursor(WatchEditorField::Path)),
                field_style(WatchEditorField::Path),
            ),
            Span::styled("Path:      ", label_style(WatchEditorField::Path)),
            Span::styled(
                if editor.path.is_empty() {
                    "(enter path)"
                } else {
                    &editor.path
                },
                field_style(WatchEditorField::Path),
            ),
            Span::styled(
                if editor.field == WatchEditorField::Path {
                    "▏"
                } else {
                    ""
                },
                colors.text_primary(),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                format!(" {} ", cursor(WatchEditorField::Recursive)),
                field_style(WatchEditorField::Recursive),
            ),
            Span::styled("Recursive: ", label_style(WatchEditorField::Recursive)),
            Span::styled(
                if editor.recursive {
                    "✓ Yes"
                } else {
                    "✗ No"
                },
                field_style(WatchEditorField::Recursive),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                format!(" {} ", cursor(WatchEditorField::Rules)),
                field_style(WatchEditorField::Rules),
            ),
            Span::styled("Rules:     ", label_style(WatchEditorField::Rules)),
            Span::styled(
                if editor.rules_filter.is_empty() {
                    "(all rules)".to_string()
                } else {
                    format!("{} selected", editor.rules_filter.len())
                },
                field_style(WatchEditorField::Rules),
            ),
        ]),
    ];

    // Add rules list when Rules field is active
    if editor.field == WatchEditorField::Rules && !editor.available_rules.is_empty() {
        for (i, rule_name) in editor.available_rules.iter().enumerate() {
            let is_selected = editor.is_rule_selected(rule_name);
            let is_cursor = i == editor.rules_cursor;

            let checkbox = if is_selected { "☑" } else { "☐" };
            let prefix = if is_cursor { " ▸ " } else { "   " };

            let rule_style = if is_cursor {
                colors.selected().add_modifier(Modifier::BOLD)
            } else if is_selected {
                colors.text_primary()
            } else {
                colors.text_dim()
            };

            content.push(Line::from(vec![
                Span::styled(format!("     {}{} ", prefix, checkbox), rule_style),
                Span::styled(rule_name, rule_style),
            ]));
        }
    }

    content.push(Line::from(""));
    // Contextual help line
    content.push(Line::from(vec![
        Span::styled("  💡 ", colors.text_dim()),
        Span::styled(
            watch_field_help(editor.field),
            colors.text_muted().add_modifier(Modifier::ITALIC),
        ),
    ]));

    let editor_widget = Paragraph::new(content)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(colors.primary))
                .border_type(BorderType::Rounded)
                .style(Style::default().bg(colors.bg))
                .title(title)
                .title_style(colors.text_primary())
                .title_bottom(
                    Line::from(" Tab: next field │ Enter: save │ Esc: cancel ").centered(),
                ),
        )
        .wrap(Wrap { trim: false });

    frame.render_widget(editor_widget, popup_area);
}

/// Returns contextual help text for each watch editor field
fn watch_field_help(field: WatchEditorField) -> &'static str {
    match field {
        WatchEditorField::Path => "Full path to watch, e.g. ~/Downloads or /Users/you/Downloads",
        WatchEditorField::Recursive => "Space/←→ to toggle — watch subdirectories too",
        WatchEditorField::Rules => "Space: toggle │ a: select all │ c: clear (all rules apply)",
    }
}

fn render_about_dialog(frame: &mut Frame, state: &AppState) {
    let colors = state.theme.colors();
    let area = frame.area();

    let popup_area = centered_rect(80, 60, area);
    frame.render_widget(Clear, popup_area);

    let version = env!("CARGO_PKG_VERSION");
    let repo = "https://github.com/ricardodantas/hazelnut";

    let logo = [
        "██╗  ██╗ █████╗ ███████╗███████╗██╗     ███╗   ██╗██╗   ██╗████████╗",
        "██║  ██║██╔══██╗╚══███╔╝██╔════╝██║     ████╗  ██║██║   ██║╚══██╔══╝",
        "███████║███████║  ███╔╝ █████╗  ██║     ██╔██╗ ██║██║   ██║   ██║   ",
        "██╔══██║██╔══██║ ███╔╝  ██╔══╝  ██║     ██║╚██╗██║██║   ██║   ██║   ",
        "██║  ██║██║  ██║███████╗███████╗███████╗██║ ╚████║╚██████╔╝   ██║   ",
        "╚═╝  ╚═╝╚═╝  ╚═╝╚══════╝╚══════╝╚══════╝╚═╝  ╚═══╝ ╚═════╝    ╚═╝   ",
    ];

    let mut lines: Vec<Line> = logo
        .iter()
        .map(|line| Line::from(Span::styled(*line, Style::default().fg(colors.primary))))
        .collect();

    lines.extend([
        Line::from(""),
        Line::from(Span::styled(
            "🌰 Terminal file organizer inspired by Hazel",
            Style::default()
                .fg(colors.fg)
                .add_modifier(Modifier::ITALIC),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("Version: ", colors.text_muted()),
            Span::styled(
                version,
                Style::default()
                    .fg(colors.primary)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Author: ", colors.text_muted()),
            Span::styled("Ricardo Dantas", colors.text()),
        ]),
        Line::from(vec![
            Span::styled("License: ", colors.text_muted()),
            Span::styled("MIT", colors.text()),
        ]),
        Line::from(vec![
            Span::styled("Repo: ", colors.text_muted()),
            Span::styled(repo, Style::default().fg(colors.primary)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "Built with Rust 🦀 + Ratatui",
            colors.text_muted().add_modifier(Modifier::ITALIC),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                " [G] ",
                Style::default()
                    .fg(colors.primary)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("Open GitHub"),
            Span::raw("    "),
            Span::styled(" [Esc] ", colors.text_muted()),
            Span::raw("Close"),
        ]),
    ]);

    let paragraph = Paragraph::new(lines).alignment(Alignment::Center).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(colors.primary))
            .style(Style::default().bg(colors.bg))
            .title(" 🌰 About Hazelnut ")
            .title_style(
                Style::default()
                    .fg(colors.primary)
                    .add_modifier(Modifier::BOLD),
            ),
    );

    frame.render_widget(paragraph, popup_area);
}

fn render_update_confirm_dialog(frame: &mut Frame, state: &AppState) {
    let colors = state.theme.colors();
    let area = frame.area();

    // Center popup
    let popup_width = 50;
    let popup_height = 9;
    let popup_area = Rect {
        x: area.width.saturating_sub(popup_width) / 2,
        y: area.height.saturating_sub(popup_height) / 2,
        width: popup_width.min(area.width),
        height: popup_height.min(area.height),
    };

    // Clear area behind popup
    frame.render_widget(ratatui::widgets::Clear, popup_area);

    let latest = state.update_available.as_deref().unwrap_or("unknown");
    let pm = &state.package_manager;

    let lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("Update to ", colors.text()),
            Span::styled(
                format!("v{}", latest),
                Style::default()
                    .fg(colors.warning)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("?", colors.text()),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Command: ", colors.text_muted()),
            Span::styled(pm.update_command(), Style::default().fg(colors.primary)),
        ]),
        Line::from(""),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                " [Y] ",
                Style::default()
                    .fg(colors.success)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("Yes, update"),
            Span::raw("    "),
            Span::styled(" [N/Esc] ", colors.text_muted()),
            Span::raw("Cancel"),
        ]),
    ];

    let paragraph = Paragraph::new(lines).alignment(Alignment::Center).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(colors.warning))
            .style(Style::default().bg(colors.bg))
            .title(" ⬆️ Update Hazelnut ")
            .title_style(
                Style::default()
                    .fg(colors.warning)
                    .add_modifier(Modifier::BOLD),
            ),
    );

    frame.render_widget(paragraph, popup_area);
}

fn render_update_status(frame: &mut Frame, state: &AppState, status: &str) {
    let colors = state.theme.colors();
    let area = frame.area();

    // Bottom banner
    let banner_height = 3;
    let banner_area = Rect {
        x: 0,
        y: area.height.saturating_sub(banner_height),
        width: area.width,
        height: banner_height,
    };

    let is_success = status.contains("complete");
    let border_color = if is_success {
        colors.success
    } else {
        colors.warning
    };

    let paragraph = Paragraph::new(Line::from(status))
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(border_color))
                .style(Style::default().bg(colors.bg)),
        );

    frame.render_widget(paragraph, banner_area);
}

fn render_updating_overlay(frame: &mut Frame, state: &AppState) {
    let colors = state.theme.colors();
    let area = frame.area();

    // Dim the background
    let overlay = Block::default().style(Style::default().bg(Color::Black));
    frame.render_widget(overlay, area);

    // Centered modal
    let popup_width = 40;
    let popup_height = 5;
    let popup_area = Rect {
        x: area.width.saturating_sub(popup_width) / 2,
        y: area.height.saturating_sub(popup_height) / 2,
        width: popup_width.min(area.width),
        height: popup_height.min(area.height),
    };

    let text = vec![
        Line::from(""),
        Line::from(Span::styled(
            "⏳ Updating... please wait",
            Style::default()
                .fg(colors.warning)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];

    let paragraph = Paragraph::new(text).alignment(Alignment::Center).block(
        Block::default()
            .title(" Update in Progress ")
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(colors.warning))
            .style(Style::default().bg(colors.bg)),
    );

    frame.render_widget(paragraph, popup_area);
}
