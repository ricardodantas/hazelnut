//! Theme configuration and colors.
//!
//! Hazelnut supports popular terminal color schemes out of the box.
//! Each theme defines colors for UI elements like accents, backgrounds,
//! text, selections, and more.
//!
//! Themes are identical to Feedo for consistency.

use ratatui::style::{Color, Modifier, Style};
use serde::{Deserialize, Serialize};

/// Available themes based on popular terminal/editor color schemes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Theme {
    /// Dracula — dark purple aesthetic.
    #[default]
    Dracula,
    /// One Dark Pro — Atom's iconic dark theme.
    OneDarkPro,
    /// Nord — arctic, bluish color palette.
    Nord,
    /// Catppuccin Mocha — warm pastel dark theme.
    CatppuccinMocha,
    /// Catppuccin Latte — warm pastel light theme.
    CatppuccinLatte,
    /// Gruvbox Dark — retro groove colors.
    GruvboxDark,
    /// Gruvbox Light — retro groove, light variant.
    GruvboxLight,
    /// Tokyo Night — futuristic dark blue.
    TokyoNight,
    /// Solarized Dark — precision colors, dark.
    SolarizedDark,
    /// Solarized Light — precision colors, light.
    SolarizedLight,
    /// Monokai Pro — classic syntax highlighting colors.
    MonokaiPro,
    /// Rosé Pine — all natural pine, faux fur, and a bit of soho vibes.
    RosePine,
    /// Kanagawa — dark theme inspired by Katsushika Hokusai.
    Kanagawa,
    /// Everforest — comfortable green forest theme.
    Everforest,
    /// Cyberpunk — neon-soaked futuristic theme.
    Cyberpunk,
}

impl Theme {
    /// Get all available theme names.
    #[must_use]
    pub const fn all() -> &'static [Self] {
        &[
            Self::Dracula,
            Self::OneDarkPro,
            Self::Nord,
            Self::CatppuccinMocha,
            Self::CatppuccinLatte,
            Self::GruvboxDark,
            Self::GruvboxLight,
            Self::TokyoNight,
            Self::SolarizedDark,
            Self::SolarizedLight,
            Self::MonokaiPro,
            Self::RosePine,
            Self::Kanagawa,
            Self::Everforest,
            Self::Cyberpunk,
        ]
    }

    /// Get the next theme in rotation
    pub fn next(&self) -> Theme {
        let themes = Self::all();
        let current = themes.iter().position(|t| t == self).unwrap_or(0);
        themes[(current + 1) % themes.len()]
    }

    /// Get the display name for the theme.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Dracula => "Dracula",
            Self::OneDarkPro => "One Dark Pro",
            Self::Nord => "Nord",
            Self::CatppuccinMocha => "Catppuccin Mocha",
            Self::CatppuccinLatte => "Catppuccin Latte",
            Self::GruvboxDark => "Gruvbox Dark",
            Self::GruvboxLight => "Gruvbox Light",
            Self::TokyoNight => "Tokyo Night",
            Self::SolarizedDark => "Solarized Dark",
            Self::SolarizedLight => "Solarized Light",
            Self::MonokaiPro => "Monokai Pro",
            Self::RosePine => "Rosé Pine",
            Self::Kanagawa => "Kanagawa",
            Self::Everforest => "Everforest",
            Self::Cyberpunk => "Cyberpunk",
        }
    }

    /// Load theme from config or use default
    pub fn load() -> anyhow::Result<Theme> {
        // TODO: Load from config file
        Ok(Theme::default())
    }

    /// Get the color palette for this theme
    pub fn colors(&self) -> ThemeColors {
        ThemeColors::from_palette(self.palette())
    }

    /// Get the color palette for this theme.
    #[must_use]
    #[allow(clippy::too_many_lines)]
    pub const fn palette(self) -> ThemePalette {
        match self {
            // Dracula: https://draculatheme.com/contribute
            Self::Dracula => ThemePalette {
                accent: Color::Rgb(189, 147, 249),    // Purple
                secondary: Color::Rgb(255, 121, 198), // Pink
                bg: Color::Rgb(40, 42, 54),           // Background
                fg: Color::Rgb(248, 248, 242),        // Foreground
                muted: Color::Rgb(98, 114, 164),      // Comment
                selection: Color::Rgb(68, 71, 90),    // Selection
                error: Color::Rgb(255, 85, 85),       // Red
                warning: Color::Rgb(255, 184, 108),   // Orange
                success: Color::Rgb(80, 250, 123),    // Green
                info: Color::Rgb(139, 233, 253),      // Cyan
            },

            // One Dark Pro: https://github.com/Binaryify/OneDark-Pro
            Self::OneDarkPro => ThemePalette {
                accent: Color::Rgb(97, 175, 239),     // Blue
                secondary: Color::Rgb(198, 120, 221), // Magenta
                bg: Color::Rgb(40, 44, 52),           // Background
                fg: Color::Rgb(171, 178, 191),        // Foreground
                muted: Color::Rgb(92, 99, 112),       // Comment
                selection: Color::Rgb(62, 68, 81),    // Selection
                error: Color::Rgb(224, 108, 117),     // Red
                warning: Color::Rgb(229, 192, 123),   // Yellow
                success: Color::Rgb(152, 195, 121),   // Green
                info: Color::Rgb(86, 182, 194),       // Cyan
            },

            // Nord: https://www.nordtheme.com
            Self::Nord => ThemePalette {
                accent: Color::Rgb(136, 192, 208),    // Frost blue
                secondary: Color::Rgb(129, 161, 193), // Frost darker
                bg: Color::Rgb(46, 52, 64),           // Polar Night
                fg: Color::Rgb(236, 239, 244),        // Snow Storm
                muted: Color::Rgb(76, 86, 106),       // Polar Night lighter
                selection: Color::Rgb(67, 76, 94),    // Selection
                error: Color::Rgb(191, 97, 106),      // Aurora red
                warning: Color::Rgb(235, 203, 139),   // Aurora yellow
                success: Color::Rgb(163, 190, 140),   // Aurora green
                info: Color::Rgb(94, 129, 172),       // Frost
            },

            // Catppuccin Mocha: https://catppuccin.com
            Self::CatppuccinMocha => ThemePalette {
                accent: Color::Rgb(137, 180, 250),    // Blue
                secondary: Color::Rgb(245, 194, 231), // Pink
                bg: Color::Rgb(30, 30, 46),           // Base
                fg: Color::Rgb(205, 214, 244),        // Text
                muted: Color::Rgb(108, 112, 134),     // Overlay0
                selection: Color::Rgb(49, 50, 68),    // Surface0
                error: Color::Rgb(243, 139, 168),     // Red
                warning: Color::Rgb(249, 226, 175),   // Yellow
                success: Color::Rgb(166, 227, 161),   // Green
                info: Color::Rgb(148, 226, 213),      // Teal
            },

            // Catppuccin Latte (light theme)
            Self::CatppuccinLatte => ThemePalette {
                accent: Color::Rgb(30, 102, 245),     // Blue
                secondary: Color::Rgb(234, 118, 203), // Pink
                bg: Color::Rgb(239, 241, 245),        // Base
                fg: Color::Rgb(76, 79, 105),          // Text
                muted: Color::Rgb(140, 143, 161),     // Overlay0
                selection: Color::Rgb(204, 208, 218), // Surface0
                error: Color::Rgb(210, 15, 57),       // Red
                warning: Color::Rgb(223, 142, 29),    // Yellow
                success: Color::Rgb(64, 160, 43),     // Green
                info: Color::Rgb(23, 146, 153),       // Teal
            },

            // Gruvbox Dark: https://github.com/morhetz/gruvbox
            Self::GruvboxDark => ThemePalette {
                accent: Color::Rgb(250, 189, 47),     // Yellow
                secondary: Color::Rgb(211, 134, 155), // Purple
                bg: Color::Rgb(40, 40, 40),           // bg0
                fg: Color::Rgb(235, 219, 178),        // fg
                muted: Color::Rgb(146, 131, 116),     // gray
                selection: Color::Rgb(80, 73, 69),    // bg2
                error: Color::Rgb(251, 73, 52),       // red
                warning: Color::Rgb(254, 128, 25),    // orange
                success: Color::Rgb(184, 187, 38),    // green
                info: Color::Rgb(131, 165, 152),      // aqua
            },

            // Gruvbox Light
            Self::GruvboxLight => ThemePalette {
                accent: Color::Rgb(181, 118, 20),     // Yellow
                secondary: Color::Rgb(143, 63, 113),  // Purple
                bg: Color::Rgb(251, 241, 199),        // bg0
                fg: Color::Rgb(60, 56, 54),           // fg
                muted: Color::Rgb(146, 131, 116),     // gray
                selection: Color::Rgb(213, 196, 161), // bg2
                error: Color::Rgb(157, 0, 6),         // red
                warning: Color::Rgb(175, 58, 3),      // orange
                success: Color::Rgb(121, 116, 14),    // green
                info: Color::Rgb(66, 123, 88),        // aqua
            },

            // Tokyo Night: https://github.com/enkia/tokyo-night-vscode-theme
            Self::TokyoNight => ThemePalette {
                accent: Color::Rgb(122, 162, 247),    // Blue
                secondary: Color::Rgb(187, 154, 247), // Magenta
                bg: Color::Rgb(26, 27, 38),           // Background
                fg: Color::Rgb(192, 202, 245),        // Foreground
                muted: Color::Rgb(86, 95, 137),       // Comment
                selection: Color::Rgb(41, 46, 66),    // Selection
                error: Color::Rgb(247, 118, 142),     // Red
                warning: Color::Rgb(224, 175, 104),   // Yellow
                success: Color::Rgb(158, 206, 106),   // Green
                info: Color::Rgb(125, 207, 255),      // Cyan
            },

            // Solarized Dark: https://ethanschoonover.com/solarized/
            Self::SolarizedDark => ThemePalette {
                accent: Color::Rgb(38, 139, 210),     // Blue
                secondary: Color::Rgb(108, 113, 196), // Violet
                bg: Color::Rgb(0, 43, 54),            // base03
                fg: Color::Rgb(131, 148, 150),        // base0
                muted: Color::Rgb(88, 110, 117),      // base01
                selection: Color::Rgb(7, 54, 66),     // base02
                error: Color::Rgb(220, 50, 47),       // red
                warning: Color::Rgb(181, 137, 0),     // yellow
                success: Color::Rgb(133, 153, 0),     // green
                info: Color::Rgb(42, 161, 152),       // cyan
            },

            // Solarized Light
            Self::SolarizedLight => ThemePalette {
                accent: Color::Rgb(38, 139, 210),     // Blue
                secondary: Color::Rgb(108, 113, 196), // Violet
                bg: Color::Rgb(253, 246, 227),        // base3
                fg: Color::Rgb(101, 123, 131),        // base00
                muted: Color::Rgb(147, 161, 161),     // base1
                selection: Color::Rgb(238, 232, 213), // base2
                error: Color::Rgb(220, 50, 47),       // red
                warning: Color::Rgb(181, 137, 0),     // yellow
                success: Color::Rgb(133, 153, 0),     // green
                info: Color::Rgb(42, 161, 152),       // cyan
            },

            // Monokai Pro: https://monokai.pro
            Self::MonokaiPro => ThemePalette {
                accent: Color::Rgb(255, 216, 102),    // Yellow
                secondary: Color::Rgb(171, 157, 242), // Purple
                bg: Color::Rgb(45, 42, 46),           // Background
                fg: Color::Rgb(252, 252, 250),        // Foreground
                muted: Color::Rgb(114, 113, 105),     // Comment
                selection: Color::Rgb(81, 80, 79),    // Selection
                error: Color::Rgb(255, 97, 136),      // Red
                warning: Color::Rgb(252, 152, 103),   // Orange
                success: Color::Rgb(169, 220, 118),   // Green
                info: Color::Rgb(120, 220, 232),      // Cyan
            },

            // Rosé Pine: https://rosepinetheme.com
            Self::RosePine => ThemePalette {
                accent: Color::Rgb(235, 188, 186),    // Rose
                secondary: Color::Rgb(196, 167, 231), // Iris
                bg: Color::Rgb(25, 23, 36),           // Base
                fg: Color::Rgb(224, 222, 244),        // Text
                muted: Color::Rgb(110, 106, 134),     // Muted
                selection: Color::Rgb(38, 35, 58),    // Overlay
                error: Color::Rgb(235, 111, 146),     // Love
                warning: Color::Rgb(246, 193, 119),   // Gold
                success: Color::Rgb(156, 207, 216),   // Foam
                info: Color::Rgb(49, 116, 143),       // Pine
            },

            // Kanagawa: https://github.com/rebelot/kanagawa.nvim
            Self::Kanagawa => ThemePalette {
                accent: Color::Rgb(127, 180, 202),    // Crystal blue
                secondary: Color::Rgb(149, 127, 184), // Oniviolet
                bg: Color::Rgb(31, 31, 40),           // Sumi ink
                fg: Color::Rgb(220, 215, 186),        // Fuji white
                muted: Color::Rgb(84, 84, 109),       // Katana gray
                selection: Color::Rgb(54, 54, 70),    // Wave blue
                error: Color::Rgb(195, 64, 67),       // Samurai red
                warning: Color::Rgb(255, 169, 107),   // Ronin yellow
                success: Color::Rgb(118, 148, 106),   // Spring green
                info: Color::Rgb(126, 156, 216),      // Spring blue
            },

            // Everforest: https://github.com/sainnhe/everforest
            Self::Everforest => ThemePalette {
                accent: Color::Rgb(131, 193, 120),    // Green
                secondary: Color::Rgb(214, 153, 182), // Purple
                bg: Color::Rgb(47, 53, 55),           // bg0
                fg: Color::Rgb(211, 198, 170),        // fg
                muted: Color::Rgb(133, 146, 137),     // gray
                selection: Color::Rgb(68, 78, 79),    // bg2
                error: Color::Rgb(230, 126, 128),     // red
                warning: Color::Rgb(219, 188, 127),   // yellow
                success: Color::Rgb(167, 192, 128),   // green
                info: Color::Rgb(124, 195, 191),      // aqua
            },

            // Cyberpunk: custom neon theme
            Self::Cyberpunk => ThemePalette {
                accent: Color::Rgb(0, 255, 255),    // Neon cyan
                secondary: Color::Rgb(255, 0, 255), // Neon magenta
                bg: Color::Rgb(13, 2, 33),          // Dark purple
                fg: Color::Rgb(240, 240, 240),      // Bright white
                muted: Color::Rgb(100, 100, 140),   // Muted purple
                selection: Color::Rgb(40, 20, 80),  // Purple selection
                error: Color::Rgb(255, 0, 60),      // Neon red
                warning: Color::Rgb(255, 230, 0),   // Neon yellow
                success: Color::Rgb(0, 255, 100),   // Neon green
                info: Color::Rgb(0, 180, 255),      // Neon blue
            },
        }
    }
}

/// Color palette for a theme.
#[derive(Debug, Clone, Copy)]
pub struct ThemePalette {
    /// Primary accent color (highlights, active elements).
    pub accent: Color,
    /// Secondary accent color.
    pub secondary: Color,
    /// Background color.
    pub bg: Color,
    /// Foreground/text color.
    pub fg: Color,
    /// Muted/dimmed text color.
    pub muted: Color,
    /// Selection/highlight background.
    pub selection: Color,
    /// Error/red color.
    pub error: Color,
    /// Warning/yellow color.
    pub warning: Color,
    /// Success/green color.
    pub success: Color,
    /// Info/blue color.
    pub info: Color,
}

/// Extended color palette for UI elements
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
                    c.saturating_add(amount as u8)
                } else {
                    c.saturating_sub((-amount) as u8)
                }
            };
            Color::Rgb(adjust(r), adjust(g), adjust(b))
        } else {
            color
        }
    }

    // Style helpers

    /// Default text style
    pub fn text(&self) -> Style {
        Style::default().fg(self.fg)
    }

    /// Dimmed text style
    pub fn text_dim(&self) -> Style {
        Style::default().fg(self.fg_dim)
    }

    /// Muted text style
    pub fn text_muted(&self) -> Style {
        Style::default().fg(self.fg_muted)
    }

    /// Primary accent style
    pub fn text_primary(&self) -> Style {
        Style::default().fg(self.primary)
    }

    /// Secondary accent style
    pub fn text_secondary(&self) -> Style {
        Style::default().fg(self.secondary)
    }

    /// Success style
    pub fn text_success(&self) -> Style {
        Style::default().fg(self.success)
    }

    /// Warning style
    pub fn text_warning(&self) -> Style {
        Style::default().fg(self.warning)
    }

    /// Error style
    pub fn text_error(&self) -> Style {
        Style::default().fg(self.error)
    }

    /// Info style
    pub fn text_info(&self) -> Style {
        Style::default().fg(self.info)
    }

    /// Block border style
    pub fn block(&self) -> Style {
        Style::default().fg(self.border)
    }

    /// Focused block border style
    pub fn block_focus(&self) -> Style {
        Style::default().fg(self.border_focus)
    }

    /// Selected item style
    pub fn selected(&self) -> Style {
        Style::default()
            .bg(self.selection)
            .fg(self.fg)
            .add_modifier(Modifier::BOLD)
    }

    /// Tab style
    pub fn tab(&self) -> Style {
        Style::default().fg(self.fg_muted)
    }

    /// Active tab style
    pub fn tab_active(&self) -> Style {
        Style::default()
            .fg(self.primary)
            .add_modifier(Modifier::BOLD)
    }

    /// Key hint style (for shortcuts)
    pub fn key_hint(&self) -> Style {
        Style::default()
            .fg(self.accent)
            .add_modifier(Modifier::BOLD)
    }

    /// Logo primary style
    pub fn logo_style_primary(&self) -> Style {
        Style::default()
            .fg(self.logo_primary)
            .add_modifier(Modifier::BOLD)
    }

    /// Logo secondary style
    pub fn logo_style_secondary(&self) -> Style {
        Style::default()
            .fg(self.logo_secondary)
            .add_modifier(Modifier::BOLD)
    }
}
