//! Theme configuration and colors.
//!
//! Hazelnut supports popular terminal color schemes out of the box.
//! Theme palettes are provided by the `ratatui-themes` crate,
//! with extended UI styling through `ThemeColors`.

use ratatui::style::{Color, Modifier, Style};
use ratatui_themes::{ThemeName, ThemePalette};
use serde::{Deserialize, Serialize};

/// Theme wrapper around `ThemeName` from ratatui-themes.
///
/// This provides Hazelnut-specific functionality like loading from config
/// and creating extended UI color palettes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Theme(pub ThemeName);

impl Theme {
    /// Get all available theme names.
    #[must_use]
    pub fn all() -> &'static [ThemeName] {
        ThemeName::all()
    }

    /// Get the next theme in rotation
    #[must_use]
    pub fn next(&self) -> Theme {
        Theme(self.0.next())
    }

    /// Get the display name for the theme.
    #[must_use]
    pub fn name(&self) -> &'static str {
        self.0.display_name()
    }

    /// Load theme from config or use default
    pub fn load(config: &crate::config::Config) -> Theme {
        config
            .general
            .theme
            .as_ref()
            .and_then(|name| name.parse::<ThemeName>().ok())
            .map(Theme::from)
            .unwrap_or_default()
    }

    /// Get the color palette for this theme
    #[must_use]
    pub fn colors(&self) -> ThemeColors {
        ThemeColors::from_palette(self.0.palette())
    }

    /// Get the raw color palette for this theme.
    #[must_use]
    pub fn palette(&self) -> ThemePalette {
        self.0.palette()
    }

    /// Get the inner ThemeName
    #[must_use]
    pub fn inner(&self) -> ThemeName {
        self.0
    }

    /// Get the kebab-case slug for config files
    #[must_use]
    pub fn slug(&self) -> &'static str {
        self.0.slug()
    }
}

impl From<ThemeName> for Theme {
    fn from(name: ThemeName) -> Self {
        Theme(name)
    }
}

impl std::fmt::Display for Theme {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// Extended color palette for UI elements.
///
/// This provides pre-built styles and derived colors for Hazelnut's UI,
/// based on a theme's base palette.
#[derive(Debug, Clone)]
pub struct ThemeColors {
    // Base colors (from palette)
    pub bg: Color,
    pub bg_secondary: Color,
    pub bg_highlight: Color,
    pub fg: Color,
    pub fg_dim: Color,
    pub fg_muted: Color,

    // Accent colors
    pub primary: Color,
    pub secondary: Color,
    pub accent: Color,

    // Semantic colors
    pub success: Color,
    pub warning: Color,
    pub error: Color,
    pub info: Color,

    // UI elements
    pub border: Color,
    pub border_focus: Color,
    pub selection: Color,

    // Special
    pub logo_primary: Color,
    pub logo_secondary: Color,
}

impl ThemeColors {
    /// Create ThemeColors from a ThemePalette
    #[must_use]
    pub fn from_palette(p: ThemePalette) -> Self {
        // Derive secondary backgrounds by lightening/darkening
        let bg_secondary = Self::adjust_brightness(p.bg, 10);
        let bg_highlight = Self::adjust_brightness(p.bg, 20);

        Self {
            bg: p.bg,
            bg_secondary,
            bg_highlight,
            fg: p.fg,
            fg_dim: p.muted,
            fg_muted: p.muted,

            primary: p.accent,
            secondary: p.secondary,
            accent: p.secondary,

            success: p.success,
            warning: p.warning,
            error: p.error,
            info: p.info,

            border: p.muted,
            border_focus: p.accent,
            selection: p.selection,

            logo_primary: p.accent,
            logo_secondary: p.secondary,
        }
    }

    /// Adjust color brightness
    fn adjust_brightness(color: Color, amount: i16) -> Color {
        if let Color::Rgb(r, g, b) = color {
            let adjust = |c: u8| -> u8 {
                if amount > 0 {
                    c.saturating_add(amount.min(255) as u8)
                } else {
                    // Clamp to avoid panic: (-i16::MIN) overflows, so clamp first
                    let neg = (-(amount.max(-255))) as u8;
                    c.saturating_sub(neg)
                }
            };
            Color::Rgb(adjust(r), adjust(g), adjust(b))
        } else {
            color
        }
    }

    // Style helpers

    /// Default text style
    #[must_use]
    pub fn text(&self) -> Style {
        Style::default().fg(self.fg)
    }

    /// Dimmed text style
    #[must_use]
    pub fn text_dim(&self) -> Style {
        Style::default().fg(self.fg_dim)
    }

    /// Muted text style
    #[must_use]
    pub fn text_muted(&self) -> Style {
        Style::default().fg(self.fg_muted)
    }

    /// Primary accent style
    #[must_use]
    pub fn text_primary(&self) -> Style {
        Style::default().fg(self.primary)
    }

    /// Secondary accent style
    #[must_use]
    pub fn text_secondary(&self) -> Style {
        Style::default().fg(self.secondary)
    }

    /// Success style
    #[must_use]
    pub fn text_success(&self) -> Style {
        Style::default().fg(self.success)
    }

    /// Warning style
    #[must_use]
    pub fn text_warning(&self) -> Style {
        Style::default().fg(self.warning)
    }

    /// Error style
    #[must_use]
    pub fn text_error(&self) -> Style {
        Style::default().fg(self.error)
    }

    /// Info style
    #[must_use]
    pub fn text_info(&self) -> Style {
        Style::default().fg(self.info)
    }

    /// Block border style
    #[must_use]
    pub fn block(&self) -> Style {
        Style::default().fg(self.border)
    }

    /// Focused block border style
    #[must_use]
    pub fn block_focus(&self) -> Style {
        Style::default().fg(self.border_focus)
    }

    /// Selected item style
    #[must_use]
    pub fn selected(&self) -> Style {
        Style::default()
            .bg(self.selection)
            .fg(self.fg)
            .add_modifier(Modifier::BOLD)
    }

    /// Tab style
    #[must_use]
    pub fn tab(&self) -> Style {
        Style::default().fg(self.fg_muted)
    }

    /// Active tab style
    #[must_use]
    pub fn tab_active(&self) -> Style {
        Style::default()
            .fg(self.primary)
            .add_modifier(Modifier::BOLD)
    }

    /// Key hint style (for shortcuts)
    #[must_use]
    pub fn key_hint(&self) -> Style {
        Style::default()
            .fg(self.accent)
            .add_modifier(Modifier::BOLD)
    }

    /// Logo primary style
    #[must_use]
    pub fn logo_style_primary(&self) -> Style {
        Style::default()
            .fg(self.logo_primary)
            .add_modifier(Modifier::BOLD)
    }

    /// Logo secondary style
    #[must_use]
    pub fn logo_style_secondary(&self) -> Style {
        Style::default()
            .fg(self.logo_secondary)
            .add_modifier(Modifier::BOLD)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    /// Resolve the theme a config file with `theme = "<slug>"` would use.
    fn theme_from_config(slug: &str) -> ThemeName {
        let toml = format!("[general]\ntheme = \"{slug}\"\n");
        let config: Config = toml::from_str(&toml).expect("test config is valid TOML");
        Theme::load(&config).inner()
    }

    #[test]
    fn load_should_select_msx_from_config() {
        assert_eq!(theme_from_config("msx"), ThemeName::Msx);
    }

    #[test]
    fn load_should_select_midnight_commander_from_config() {
        assert_eq!(
            theme_from_config("midnight-commander"),
            ThemeName::MidnightCommander
        );
    }

    #[test]
    fn load_should_keep_theme_slugs_saved_by_earlier_releases() {
        // Slugs hazelnut wrote to config files before ratatui-themes 0.4.
        let earlier = [
            ("dracula", ThemeName::Dracula),
            ("one-dark-pro", ThemeName::OneDarkPro),
            ("nord", ThemeName::Nord),
            ("catppuccin-mocha", ThemeName::CatppuccinMocha),
            ("catppuccin-latte", ThemeName::CatppuccinLatte),
            ("gruvbox-dark", ThemeName::GruvboxDark),
            ("gruvbox-light", ThemeName::GruvboxLight),
            ("tokyo-night", ThemeName::TokyoNight),
            ("solarized-dark", ThemeName::SolarizedDark),
            ("solarized-light", ThemeName::SolarizedLight),
            ("monokai-pro", ThemeName::MonokaiPro),
            ("rose-pine", ThemeName::RosePine),
            ("kanagawa", ThemeName::Kanagawa),
            ("everforest", ThemeName::Everforest),
            ("cyberpunk", ThemeName::Cyberpunk),
        ];

        let resolved: Vec<_> = earlier
            .iter()
            .map(|&(slug, _)| (slug, theme_from_config(slug)))
            .collect();

        let expected: Vec<_> = earlier.to_vec();
        assert_eq!(resolved, expected);
    }
}
