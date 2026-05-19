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
    /// Navigate to the next page of results.
    NextPage,
    /// Navigate to the previous page of results.
    PrevPage,
    /// View the baseline application log in `$EDITOR`.
    ViewBaselineLog,
    /// Toggle between patchset and message list views.
    ToggleListContent,
    /// Toggle bookmark-only filter in the list view.
    BookmarkFilter,
    /// Cycle the sort column in the patchset list.
    CycleSort,
    /// Reverse the sort direction in the patchset list.
    ReverseSort,
}

impl KeyAction {
    /// Human-readable label for display in the help overlay.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Quit => "Quit",
            Self::ScrollDown => "Scroll down",
            Self::ScrollUp => "Scroll up",
            Self::ScrollHalfPageDown => "Half page down",
            Self::ScrollHalfPageUp => "Half page up",
            Self::NextMailbox => "Next mailbox",
            Self::PrevMailbox => "Prev mailbox",
            Self::OpenThread => "Open thread",
            Self::CloseThread => "Close / Back",
            Self::Refresh => "Refresh",
            Self::Search => "Search",
            Self::BookmarkToggle => "Toggle bookmark",
            Self::ViewRawLog => "View raw log",
            Self::Help => "Help",
            Self::FocusSidebar => "Focus sidebar",
            Self::NextComment => "Next comment",
            Self::PrevComment => "Prev comment",
            Self::NextPage => "Next page",
            Self::PrevPage => "Prev page",
            Self::ViewBaselineLog => "View baseline log",
            Self::ToggleListContent => "Toggle messages",
            Self::BookmarkFilter => "Bookmark filter",
            Self::CycleSort => "Cycle sort column",
            Self::ReverseSort => "Reverse sort",
        }
    }
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

        // Uppercase letters implicitly carry SHIFT — crossterm delivers
        // Shift+n as KeyCode::Char('N') with KeyModifiers::SHIFT.
        if let KeyCode::Char(c) = code
            && c.is_ascii_uppercase()
        {
            modifiers |= KeyModifiers::SHIFT;
        }

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
        "next_page" => KeyAction::NextPage,
        "prev_page" => KeyAction::PrevPage,
        "view_baseline_log" => KeyAction::ViewBaselineLog,
        "toggle_list_content" => KeyAction::ToggleListContent,
        "bookmark_filter" => KeyAction::BookmarkFilter,
        "cycle_sort" => KeyAction::CycleSort,
        "reverse_sort" => KeyAction::ReverseSort,
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
            ("next_page", "]"),
            ("prev_page", "["),
            ("view_baseline_log", "L"),
            ("toggle_list_content", "m"),
            ("bookmark_filter", "B"),
            ("cycle_sort", "s"),
            ("reverse_sort", "S"),
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
        assert_eq!(config.action_for(&tab_combo), Some(KeyAction::NextMailbox));
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
        let config: KeybindingsConfig = toml::from_str(toml_str).expect("parse keybindings");

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

    #[test]
    fn key_combo_display_all_named_keys() {
        // Cover remaining KeyCode arms in KeyCombo::fmt
        assert_eq!(
            KeyCombo::new(KeyCode::Tab, KeyModifiers::NONE).to_string(),
            "Tab"
        );
        assert_eq!(
            KeyCombo::new(KeyCode::BackTab, KeyModifiers::NONE).to_string(),
            "S-Tab"
        );
        assert_eq!(
            KeyCombo::new(KeyCode::Esc, KeyModifiers::NONE).to_string(),
            "Esc"
        );
        assert_eq!(
            KeyCombo::new(KeyCode::Backspace, KeyModifiers::NONE).to_string(),
            "Backspace"
        );
        assert_eq!(
            KeyCombo::new(KeyCode::Delete, KeyModifiers::NONE).to_string(),
            "Delete"
        );
        assert_eq!(
            KeyCombo::new(KeyCode::Up, KeyModifiers::NONE).to_string(),
            "Up"
        );
        assert_eq!(
            KeyCombo::new(KeyCode::Down, KeyModifiers::NONE).to_string(),
            "Down"
        );
        assert_eq!(
            KeyCombo::new(KeyCode::Left, KeyModifiers::NONE).to_string(),
            "Left"
        );
        assert_eq!(
            KeyCombo::new(KeyCode::Right, KeyModifiers::NONE).to_string(),
            "Right"
        );
        assert_eq!(
            KeyCombo::new(KeyCode::Home, KeyModifiers::NONE).to_string(),
            "Home"
        );
        assert_eq!(
            KeyCombo::new(KeyCode::End, KeyModifiers::NONE).to_string(),
            "End"
        );
        assert_eq!(
            KeyCombo::new(KeyCode::PageUp, KeyModifiers::NONE).to_string(),
            "PageUp"
        );
        assert_eq!(
            KeyCombo::new(KeyCode::PageDown, KeyModifiers::NONE).to_string(),
            "PageDown"
        );
        // F-key
        assert_eq!(
            KeyCombo::new(KeyCode::F(1), KeyModifiers::NONE).to_string(),
            "F1"
        );
        // Fallback arm
        assert_eq!(
            KeyCombo::new(KeyCode::Insert, KeyModifiers::NONE).to_string(),
            "?"
        );
    }

    #[test]
    fn key_combo_display_modifier_combos() {
        // Shift modifier
        assert_eq!(
            KeyCombo::new(KeyCode::Char('a'), KeyModifiers::SHIFT).to_string(),
            "S-a"
        );
        // Alt modifier
        assert_eq!(
            KeyCombo::new(KeyCode::Char('x'), KeyModifiers::ALT).to_string(),
            "A-x"
        );
        // Ctrl+Shift
        let cs = KeyCombo::new(
            KeyCode::Char('z'),
            KeyModifiers::CONTROL | KeyModifiers::SHIFT,
        );
        assert_eq!(cs.to_string(), "C-S-z");
        // Ctrl+Alt
        let ca = KeyCombo::new(
            KeyCode::Char('m'),
            KeyModifiers::CONTROL | KeyModifiers::ALT,
        );
        assert_eq!(ca.to_string(), "C-A-m");
    }

    #[test]
    fn key_action_label_covers_all_variants() {
        // Exercises every arm of KeyAction::label (CC=25)
        let labels = [
            (KeyAction::Quit, "Quit"),
            (KeyAction::ScrollDown, "Scroll down"),
            (KeyAction::ScrollUp, "Scroll up"),
            (KeyAction::ScrollHalfPageDown, "Half page down"),
            (KeyAction::ScrollHalfPageUp, "Half page up"),
            (KeyAction::NextMailbox, "Next mailbox"),
            (KeyAction::PrevMailbox, "Prev mailbox"),
            (KeyAction::OpenThread, "Open thread"),
            (KeyAction::CloseThread, "Close / Back"),
            (KeyAction::Refresh, "Refresh"),
            (KeyAction::Search, "Search"),
            (KeyAction::BookmarkToggle, "Toggle bookmark"),
            (KeyAction::ViewRawLog, "View raw log"),
            (KeyAction::Help, "Help"),
            (KeyAction::FocusSidebar, "Focus sidebar"),
            (KeyAction::NextComment, "Next comment"),
            (KeyAction::PrevComment, "Prev comment"),
            (KeyAction::NextPage, "Next page"),
            (KeyAction::PrevPage, "Prev page"),
            (KeyAction::ViewBaselineLog, "View baseline log"),
            (KeyAction::ToggleListContent, "Toggle messages"),
            (KeyAction::BookmarkFilter, "Bookmark filter"),
            (KeyAction::CycleSort, "Cycle sort column"),
            (KeyAction::ReverseSort, "Reverse sort"),
        ];
        for (action, expected) in labels {
            assert_eq!(action.label(), expected, "label mismatch for {action:?}");
        }
    }
}
