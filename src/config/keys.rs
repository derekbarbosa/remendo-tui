//! Keybinding configuration: `KeyAction`, `KeyCombo`, and `KeybindingsConfig`.
//!
//! Maps physical key combinations to semantic actions, decoupling
//! the user interface from hardcoded key assignments per the TRD.

use crossterm::event::{KeyCode, KeyModifiers};
use serde::Deserialize;
use std::collections::HashMap;
use std::fmt;

/// Every semantic action the TUI can perform.
///
/// This is the decoupling layer the TRD requires: operations are
/// agnostic of the keys that trigger them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyAction {
    /// Exit the application.
    Quit,
    /// Scroll down one line.
    ScrollDown,
    /// Scroll up one line.
    ScrollUp,
    /// Scroll down half a page.
    ScrollHalfPageDown,
    /// Scroll up half a page.
    ScrollHalfPageUp,
    /// Switch to the next mailbox/remote.
    NextMailbox,
    /// Switch to the previous mailbox/remote.
    PrevMailbox,
    /// Open the selected thread/patchset.
    OpenThread,
    /// Close the current detail view.
    CloseThread,
    /// Refresh the current view.
    Refresh,
    /// Activate the search prompt.
    Search,
    /// Toggle bookmark on the selected item.
    BookmarkToggle,
    /// View the raw review log.
    ViewRawLog,
    /// Show the help overlay.
    Help,
    /// Focus the sidebar panel.
    FocusSidebar,
    /// Jump to the next comment/finding.
    NextComment,
    /// Jump to the previous comment/finding.
    PrevComment,
}

/// A physical key combination (key code + modifiers).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct KeyCombo {
    /// The key code.
    pub code: KeyCode,
    /// Active modifier keys (Ctrl, Shift, Alt).
    pub modifiers: KeyModifiers,
}

impl KeyCombo {
    /// Create a new key combo.
    #[must_use]
    pub fn new(code: KeyCode, modifiers: KeyModifiers) -> Self {
        Self { code, modifiers }
    }

    /// Parse a key combo from a string like `"C-r"`, `"S-Tab"`, `"q"`.
    ///
    /// Modifier prefixes: `C-` (Ctrl), `S-` (Shift), `A-` (Alt).
    /// Multiple modifiers can be chained: `"C-S-x"`.
    fn parse(s: &str) -> Option<Self> {
        let mut modifiers = KeyModifiers::NONE;
        let mut remaining = s;

        // Parse modifier prefixes
        loop {
            if let Some(rest) = remaining.strip_prefix("C-") {
                modifiers |= KeyModifiers::CONTROL;
                remaining = rest;
            } else if let Some(rest) = remaining.strip_prefix("S-") {
                modifiers |= KeyModifiers::SHIFT;
                remaining = rest;
            } else if let Some(rest) = remaining.strip_prefix("A-") {
                modifiers |= KeyModifiers::ALT;
                remaining = rest;
            } else {
                break;
            }
        }

        let code = parse_key_code(remaining)?;
        Some(Self { code, modifiers })
    }
}

impl fmt::Display for KeyCombo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.modifiers.contains(KeyModifiers::CONTROL) {
            write!(f, "C-")?;
        }
        if self.modifiers.contains(KeyModifiers::SHIFT) {
            write!(f, "S-")?;
        }
        if self.modifiers.contains(KeyModifiers::ALT) {
            write!(f, "A-")?;
        }
        match self.code {
            KeyCode::Char(c) => write!(f, "{c}"),
            KeyCode::Enter => write!(f, "Enter"),
            KeyCode::Tab => write!(f, "Tab"),
            KeyCode::BackTab => write!(f, "S-Tab"),
            KeyCode::Esc => write!(f, "Esc"),
            KeyCode::Backspace => write!(f, "Backspace"),
            KeyCode::Delete => write!(f, "Delete"),
            KeyCode::Up => write!(f, "Up"),
            KeyCode::Down => write!(f, "Down"),
            KeyCode::Left => write!(f, "Left"),
            KeyCode::Right => write!(f, "Right"),
            KeyCode::Home => write!(f, "Home"),
            KeyCode::End => write!(f, "End"),
            KeyCode::PageUp => write!(f, "PageUp"),
            KeyCode::PageDown => write!(f, "PageDown"),
            KeyCode::F(n) => write!(f, "F{n}"),
            _ => write!(f, "?"),
        }
    }
}

/// Parse a key code string into a [`KeyCode`].
fn parse_key_code(s: &str) -> Option<KeyCode> {
    // Single character
    if s.len() == 1 {
        return Some(KeyCode::Char(s.chars().next()?));
    }

    // Named keys (case-insensitive)
    Some(match s.to_lowercase().as_str() {
        "enter" | "return" | "cr" => KeyCode::Enter,
        "tab" => KeyCode::Tab,
        "backtab" => KeyCode::BackTab,
        "esc" | "escape" => KeyCode::Esc,
        "space" => KeyCode::Char(' '),
        "backspace" | "bs" => KeyCode::Backspace,
        "delete" | "del" => KeyCode::Delete,
        "up" => KeyCode::Up,
        "down" => KeyCode::Down,
        "left" => KeyCode::Left,
        "right" => KeyCode::Right,
        "home" => KeyCode::Home,
        "end" => KeyCode::End,
        "pageup" | "pgup" => KeyCode::PageUp,
        "pagedown" | "pgdn" => KeyCode::PageDown,
        "f1" => KeyCode::F(1),
        "f2" => KeyCode::F(2),
        "f3" => KeyCode::F(3),
        "f4" => KeyCode::F(4),
        "f5" => KeyCode::F(5),
        "f6" => KeyCode::F(6),
        "f7" => KeyCode::F(7),
        "f8" => KeyCode::F(8),
        "f9" => KeyCode::F(9),
        "f10" => KeyCode::F(10),
        "f11" => KeyCode::F(11),
        "f12" => KeyCode::F(12),
        _ => return None,
    })
}

/// Parse an action name string into a [`KeyAction`].
fn parse_action(s: &str) -> Option<KeyAction> {
    Some(match s {
        "quit" => KeyAction::Quit,
        "scroll_down" => KeyAction::ScrollDown,
        "scroll_up" => KeyAction::ScrollUp,
        "scroll_half_page_down" => KeyAction::ScrollHalfPageDown,
        "scroll_half_page_up" => KeyAction::ScrollHalfPageUp,
        "next_mailbox" => KeyAction::NextMailbox,
        "prev_mailbox" => KeyAction::PrevMailbox,
        "open_thread" => KeyAction::OpenThread,
        "close_thread" => KeyAction::CloseThread,
        "refresh" => KeyAction::Refresh,
        "search" => KeyAction::Search,
        "bookmark_toggle" => KeyAction::BookmarkToggle,
        "view_raw_log" => KeyAction::ViewRawLog,
        "help" => KeyAction::Help,
        "focus_sidebar" => KeyAction::FocusSidebar,
        "next_comment" => KeyAction::NextComment,
        "prev_comment" => KeyAction::PrevComment,
        _ => return None,
    })
}

/// Keybinding configuration mapping physical keys to semantic actions.
#[derive(Debug, Clone)]
pub struct KeybindingsConfig {
    /// The resolved key-to-action map.
    pub bindings: HashMap<KeyCombo, KeyAction>,
}

impl KeybindingsConfig {
    /// Look up the action bound to a key combination.
    #[must_use]
    pub fn action_for(&self, combo: &KeyCombo) -> Option<KeyAction> {
        self.bindings.get(combo).copied()
    }
}

impl Default for KeybindingsConfig {
    fn default() -> Self {
        let defaults: &[(&str, &str)] = &[
            ("quit", "q"),
            ("scroll_down", "j"),
            ("scroll_up", "k"),
            ("scroll_half_page_down", "C-d"),
            ("scroll_half_page_up", "C-u"),
            ("next_mailbox", "Tab"),
            ("prev_mailbox", "S-Tab"),
            ("open_thread", "Enter"),
            ("close_thread", "Esc"),
            ("refresh", "C-r"),
            ("search", "/"),
            ("bookmark_toggle", "b"),
            ("view_raw_log", "r"),
            ("help", "?"),
            ("focus_sidebar", "C-s"),
            ("next_comment", "n"),
            ("prev_comment", "N"),
        ];

        let mut bindings = HashMap::new();
        for &(action_str, key_str) in defaults {
            if let (Some(action), Some(combo)) =
                (parse_action(action_str), KeyCombo::parse(key_str))
            {
                bindings.insert(combo, action);
            }
        }

        Self { bindings }
    }
}

impl<'de> Deserialize<'de> for KeybindingsConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw: HashMap<String, String> = HashMap::deserialize(deserializer)?;
        let mut config = Self::default();

        for (action_str, key_str) in &raw {
            let Some(action) = parse_action(action_str) else {
                tracing::warn!(action = %action_str, "unknown keybinding action, skipping");
                continue;
            };
            let Some(combo) = KeyCombo::parse(key_str) else {
                tracing::warn!(
                    key = %key_str,
                    action = %action_str,
                    "unparseable key combo, skipping"
                );
                continue;
            };
            config.bindings.insert(combo, action);
        }

        Ok(config)
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn parse_single_char_key() {
        let combo = KeyCombo::parse("q").expect("parse 'q'");
        assert_eq!(combo.code, KeyCode::Char('q'));
        assert_eq!(combo.modifiers, KeyModifiers::NONE);
    }

    #[test]
    fn parse_ctrl_modifier() {
        let combo = KeyCombo::parse("C-r").expect("parse 'C-r'");
        assert_eq!(combo.code, KeyCode::Char('r'));
        assert!(combo.modifiers.contains(KeyModifiers::CONTROL));
    }

    #[test]
    fn parse_shift_modifier() {
        let combo = KeyCombo::parse("S-Tab").expect("parse 'S-Tab'");
        assert_eq!(combo.code, KeyCode::Tab);
        assert!(combo.modifiers.contains(KeyModifiers::SHIFT));
    }

    #[test]
    fn parse_alt_modifier() {
        let combo = KeyCombo::parse("A-x").expect("parse 'A-x'");
        assert_eq!(combo.code, KeyCode::Char('x'));
        assert!(combo.modifiers.contains(KeyModifiers::ALT));
    }

    #[test]
    fn parse_named_keys() {
        assert_eq!(
            KeyCombo::parse("Enter").expect("Enter").code,
            KeyCode::Enter
        );
        assert_eq!(KeyCombo::parse("Esc").expect("Esc").code, KeyCode::Esc);
        assert_eq!(
            KeyCombo::parse("Space").expect("Space").code,
            KeyCode::Char(' ')
        );
        assert_eq!(KeyCombo::parse("F5").expect("F5").code, KeyCode::F(5));
    }

    #[test]
    fn parse_unknown_key_returns_none() {
        assert!(KeyCombo::parse("UnknownKey").is_none());
    }

    #[test]
    fn default_keybindings_has_expected_mappings() {
        let config = KeybindingsConfig::default();
        let quit_combo = KeyCombo::new(KeyCode::Char('q'), KeyModifiers::NONE);
        assert_eq!(config.action_for(&quit_combo), Some(KeyAction::Quit));

        let refresh_combo = KeyCombo::new(KeyCode::Char('r'), KeyModifiers::CONTROL);
        assert_eq!(config.action_for(&refresh_combo), Some(KeyAction::Refresh));

        let tab_combo = KeyCombo::new(KeyCode::Tab, KeyModifiers::NONE);
        assert_eq!(
            config.action_for(&tab_combo),
            Some(KeyAction::NextMailbox)
        );
    }

    #[test]
    fn action_for_unknown_combo_returns_none() {
        let config = KeybindingsConfig::default();
        let unknown = KeyCombo::new(KeyCode::F(12), KeyModifiers::NONE);
        assert!(config.action_for(&unknown).is_none());
    }

    #[test]
    fn deserialize_custom_keybindings() {
        let toml_str = r#"
            quit = "C-q"
            refresh = "F5"
        "#;
        let config: KeybindingsConfig =
            toml::from_str(toml_str).expect("parse keybindings");

        let ctrl_q = KeyCombo::new(KeyCode::Char('q'), KeyModifiers::CONTROL);
        assert_eq!(config.action_for(&ctrl_q), Some(KeyAction::Quit));

        let f5 = KeyCombo::new(KeyCode::F(5), KeyModifiers::NONE);
        assert_eq!(config.action_for(&f5), Some(KeyAction::Refresh));
    }

    #[test]
    fn deserialize_unknown_action_is_skipped() {
        let toml_str = r#"
            quit = "q"
            unknown_future_action = "x"
        "#;
        let config: KeybindingsConfig =
            toml::from_str(toml_str).expect("parse with unknown action");
        // "q" should still work
        let q = KeyCombo::new(KeyCode::Char('q'), KeyModifiers::NONE);
        assert_eq!(config.action_for(&q), Some(KeyAction::Quit));
    }

    #[test]
    fn key_combo_display() {
        let combo = KeyCombo::new(KeyCode::Char('r'), KeyModifiers::CONTROL);
        assert_eq!(combo.to_string(), "C-r");

        let plain = KeyCombo::new(KeyCode::Char('q'), KeyModifiers::NONE);
        assert_eq!(plain.to_string(), "q");

        let enter = KeyCombo::new(KeyCode::Enter, KeyModifiers::NONE);
        assert_eq!(enter.to_string(), "Enter");
    }
}
