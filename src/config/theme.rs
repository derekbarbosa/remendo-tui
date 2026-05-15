//! Theme and colorscheme configuration.
//!
//! Defines semantic color roles for the TUI, with support for
//! hex (`#rrggbb`) and named color strings.

use ratatui::style::Color;
use serde::{Deserialize, Deserializer};
use std::fmt;

/// A color value that deserializes from hex or named color strings.
///
/// Converts to [`ratatui::style::Color`] for rendering.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColorValue(pub Color);

impl ColorValue {
    /// Get the underlying ratatui color.
    #[must_use]
    pub fn color(&self) -> Color {
        self.0
    }
}

impl Default for ColorValue {
    fn default() -> Self {
        Self(Color::Reset)
    }
}

impl fmt::Display for ColorValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

/// Parse a color string into a [`Color`].
///
/// Supports hex (`#rrggbb`) and named colors (`red`, `blue`, etc.).
fn parse_color(s: &str) -> Color {
    let s = s.trim();

    // Hex color: #rrggbb
    if let Some(hex) = s.strip_prefix('#') {
        if hex.len() == 6
            && let (Ok(r), Ok(g), Ok(b)) = (
                u8::from_str_radix(&hex[0..2], 16),
                u8::from_str_radix(&hex[2..4], 16),
                u8::from_str_radix(&hex[4..6], 16),
            )
        {
            return Color::Rgb(r, g, b);
        }
        tracing::warn!(color = s, "invalid hex color, using Reset");
        return Color::Reset;
    }

    // Named colors
    match s.to_lowercase().as_str() {
        "black" => Color::Black,
        "red" => Color::Red,
        "green" => Color::Green,
        "yellow" => Color::Yellow,
        "blue" => Color::Blue,
        "magenta" => Color::Magenta,
        "cyan" => Color::Cyan,
        "white" => Color::White,
        "gray" | "grey" => Color::Gray,
        "dark_gray" | "dark_grey" | "darkgray" | "darkgrey" => Color::DarkGray,
        "light_red" | "lightred" => Color::LightRed,
        "light_green" | "lightgreen" => Color::LightGreen,
        "light_yellow" | "lightyellow" => Color::LightYellow,
        "light_blue" | "lightblue" => Color::LightBlue,
        "light_magenta" | "lightmagenta" => Color::LightMagenta,
        "light_cyan" | "lightcyan" => Color::LightCyan,
        "reset" => Color::Reset,
        _ => {
            tracing::warn!(color = s, "unknown color name, using Reset");
            Color::Reset
        }
    }
}

impl<'de> Deserialize<'de> for ColorValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(Self(parse_color(&s)))
    }
}

/// Semantic color palette for the TUI.
///
/// Each field represents a semantic role, not a raw ANSI code.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct ColorPalette {
    /// Default text color.
    pub foreground: ColorValue,
    /// Default background color.
    pub background: ColorValue,
    /// Accent color for highlights and active elements.
    pub accent: ColorValue,
    /// Color for error messages and critical findings.
    pub error: ColorValue,
    /// Color for warnings and medium-severity findings.
    pub warning: ColorValue,
    /// Color for success indicators.
    pub success: ColorValue,
    /// Color for informational text.
    pub info: ColorValue,
    /// Background color for selected items.
    pub selected_bg: ColorValue,
    /// Text color for selected items.
    pub selected_fg: ColorValue,
    /// Border and separator color.
    pub border: ColorValue,
    /// Muted/dimmed text color.
    pub muted: ColorValue,
}

impl Default for ColorPalette {
    fn default() -> Self {
        Self {
            foreground: ColorValue(Color::Rgb(0xd8, 0xd8, 0xd8)),
            background: ColorValue(Color::Rgb(0x18, 0x18, 0x18)),
            accent: ColorValue(Color::Rgb(0x6a, 0x9f, 0xb5)),
            error: ColorValue(Color::Rgb(0xac, 0x42, 0x42)),
            warning: ColorValue(Color::Rgb(0xf4, 0xbf, 0x75)),
            success: ColorValue(Color::Rgb(0x90, 0xa9, 0x59)),
            info: ColorValue(Color::Rgb(0x75, 0xb5, 0xaa)),
            selected_bg: ColorValue(Color::Rgb(0x38, 0x38, 0x38)),
            selected_fg: ColorValue(Color::Rgb(0xf8, 0xf8, 0xf8)),
            border: ColorValue(Color::Rgb(0x58, 0x58, 0x58)),
            muted: ColorValue(Color::Rgb(0x6b, 0x6b, 0x6b)),
        }
    }
}

/// Color theme configuration for the TUI.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct ThemeConfig {
    /// The semantic color palette.
    pub colors: ColorPalette,
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn parse_hex_color() {
        let color = parse_color("#6a9fb5");
        assert_eq!(color, Color::Rgb(106, 159, 181));
    }

    #[test]
    fn parse_named_color() {
        assert_eq!(parse_color("red"), Color::Red);
        assert_eq!(parse_color("blue"), Color::Blue);
        assert_eq!(parse_color("green"), Color::Green);
        assert_eq!(parse_color("dark_gray"), Color::DarkGray);
    }

    #[test]
    fn parse_invalid_color_falls_back() {
        assert_eq!(parse_color("not_a_color"), Color::Reset);
        assert_eq!(parse_color("#xyz"), Color::Reset);
    }

    #[test]
    fn default_palette_has_dark_theme() {
        let palette = ColorPalette::default();
        assert_eq!(palette.background, ColorValue(Color::Rgb(0x18, 0x18, 0x18)));
        assert_eq!(palette.accent, ColorValue(Color::Rgb(0x6a, 0x9f, 0xb5)));
    }

    #[test]
    fn deserialize_theme_from_toml() {
        let toml_str = r##"
            [colors]
            accent = "#ff0000"
            error = "red"
        "##;
        let theme: ThemeConfig =
            toml::from_str(toml_str).expect("parse theme");
        assert_eq!(theme.colors.accent, ColorValue(Color::Rgb(255, 0, 0)));
        assert_eq!(theme.colors.error, ColorValue(Color::Red));
        // Non-overridden fields keep defaults
        assert_eq!(
            theme.colors.background,
            ColorValue(Color::Rgb(0x18, 0x18, 0x18))
        );
    }

    #[test]
    fn color_value_display() {
        let cv = ColorValue(Color::Red);
        let display = cv.to_string();
        assert!(!display.is_empty());
    }
}
