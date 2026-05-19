//! Terminal event types and event-to-message translation.
//!
//! Defines the `Event` enum and the `handle_event()` function
//! that translates events into `Message` values via the
//! keybinding system.

use crate::app::{App, InputMode};
use crate::config::keys::{KeyAction, KeyCombo};
use crate::update::Message;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseEvent};

/// Events the application can receive.
///
/// These are translated from raw crossterm events by the
/// event handler task running in [`crate::tui::Tui`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// Application initialization.
    Init,
    /// Application quit requested.
    Quit,
    /// An error occurred in the event handler.
    Error,
    /// Periodic tick for background work.
    Tick,
    /// Time to render a new frame.
    Render,
    /// A key was pressed.
    Key(KeyEvent),
    /// A mouse event occurred.
    Mouse(MouseEvent),
    /// The terminal was resized.
    Resize(u16, u16),
    /// The terminal gained focus.
    FocusGained,
    /// The terminal lost focus.
    FocusLost,
    /// Text was pasted from the clipboard.
    Paste(String),
}

/// Translate a raw terminal event into a `Message` via the keybinding system.
///
/// Returns `None` for events that don't produce messages (e.g., mouse
/// events, focus changes, or key events that don't map to any action).
pub fn handle_event(app: &App, event: &Event) -> Option<Message> {
    match *event {
        Event::Quit => Some(Message::Quit),
        Event::Tick => Some(Message::Tick),
        Event::Render => Some(Message::Render),
        Event::Resize(w, h) => Some(Message::Resize(w, h)),
        Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
            // Modal interception priority: Search > Help > Normal
            if app.input_mode == InputMode::Search {
                return handle_search_key(key_event);
            }
            let combo = KeyCombo::new(key_event.code, key_event.modifiers);
            // Modal interception: help overlay swallows all keys except ?/Esc
            if app.show_help {
                return match app.config.keybindings.action_for(&combo) {
                    Some(KeyAction::Help) => Some(Message::ToggleHelp),
                    Some(KeyAction::CloseThread) => Some(Message::Back),
                    _ => None,
                };
            }
            app.config
                .keybindings
                .action_for(&combo)
                .and_then(action_to_message)
        }
        Event::Init => Some(Message::Init),
        Event::Error => {
            tracing::error!("terminal event stream error");
            None
        }
        Event::Key(_)
        | Event::Mouse(_)
        | Event::FocusGained
        | Event::FocusLost
        | Event::Paste(_) => None,
    }
}

/// Handle a key event during search input mode.
///
/// Bypasses the keybinding system entirely — keys are interpreted
/// as text input, cursor movement, or search lifecycle actions.
fn handle_search_key(key_event: KeyEvent) -> Option<Message> {
    match key_event.code {
        KeyCode::Char(c)
            if !key_event
                .modifiers
                .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
        {
            Some(Message::SearchInput(c))
        }
        KeyCode::Backspace => Some(Message::SearchInput('\x08')),
        KeyCode::Enter => Some(Message::SearchSubmit),
        KeyCode::Esc => Some(Message::SearchCancel),
        _ => None,
    }
}

/// Map a semantic key action to a `Message`.
///
/// Returns `Some` for all currently implemented actions. The `Option`
/// return type is retained for the `.and_then(action_to_message)` call
/// pattern and for forward compatibility when new `KeyAction` variants
/// are added before their `Message` counterparts.
#[allow(clippy::unnecessary_wraps)]
fn action_to_message(action: KeyAction) -> Option<Message> {
    match action {
        KeyAction::Quit => Some(Message::Quit),
        KeyAction::Refresh => Some(Message::Refresh),
        KeyAction::ScrollDown => Some(Message::ScrollDown),
        KeyAction::ScrollUp => Some(Message::ScrollUp),
        KeyAction::ScrollHalfPageDown => Some(Message::HalfPageDown),
        KeyAction::ScrollHalfPageUp => Some(Message::HalfPageUp),
        KeyAction::OpenThread => Some(Message::Select),
        KeyAction::NextMailbox => Some(Message::NextMailbox),
        KeyAction::PrevMailbox => Some(Message::PrevMailbox),
        KeyAction::FocusSidebar => Some(Message::ToggleFocus),
        KeyAction::CloseThread => Some(Message::Back),
        KeyAction::Help => Some(Message::ToggleHelp),
        KeyAction::NextPage => Some(Message::NextPage),
        KeyAction::PrevPage => Some(Message::PrevPage),
        KeyAction::Search => Some(Message::SearchStart),
        KeyAction::ViewRawLog => Some(Message::ViewRawLog),
        KeyAction::BookmarkToggle => Some(Message::BookmarkToggle),
        KeyAction::NextComment => Some(Message::NextComment),
        KeyAction::PrevComment => Some(Message::PrevComment),
        KeyAction::ViewBaselineLog => Some(Message::ViewBaselineLog),
        KeyAction::ToggleListContent => Some(Message::ToggleListContent),
        KeyAction::BookmarkFilter => Some(Message::ToggleBookmarkFilter),
        KeyAction::CycleSort => Some(Message::CycleSort),
        KeyAction::ReverseSort => Some(Message::ReverseSort),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crossterm::event::{KeyCode, KeyModifiers};

    fn make_key_event(code: KeyCode, modifiers: KeyModifiers) -> Event {
        Event::Key(KeyEvent::new_with_kind(
            code,
            modifiers,
            KeyEventKind::Press,
        ))
    }

    #[test]
    fn event_key_variant() {
        let event = make_key_event(KeyCode::Char('q'), KeyModifiers::NONE);
        assert!(matches!(event, Event::Key(_)));
    }

    #[test]
    fn event_resize_variant() {
        let event = Event::Resize(80, 24);
        assert!(matches!(event, Event::Resize(80, 24)));
    }

    #[test]
    fn event_equality() {
        assert_eq!(Event::Tick, Event::Tick);
        assert_ne!(Event::Tick, Event::Render);
    }

    #[test]
    fn handle_event_quit_via_keybinding() {
        let app = App::new(Config::default());
        let event = make_key_event(KeyCode::Char('q'), KeyModifiers::NONE);
        let msg = handle_event(&app, &event);
        assert!(matches!(msg, Some(Message::Quit)));
    }

    #[test]
    fn handle_event_tick() {
        let app = App::new(Config::default());
        let msg = handle_event(&app, &Event::Tick);
        assert!(matches!(msg, Some(Message::Tick)));
    }

    #[test]
    fn handle_event_resize() {
        let app = App::new(Config::default());
        let msg = handle_event(&app, &Event::Resize(120, 40));
        assert!(matches!(msg, Some(Message::Resize(120, 40))));
    }

    #[test]
    fn handle_event_unbound_key_returns_none() {
        let app = App::new(Config::default());
        let event = make_key_event(KeyCode::F(12), KeyModifiers::NONE);
        assert!(handle_event(&app, &event).is_none());
    }

    #[test]
    fn handle_event_mouse_returns_none() {
        let app = App::new(Config::default());
        let event = Event::Mouse(MouseEvent {
            kind: crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left),
            column: 0,
            row: 0,
            modifiers: KeyModifiers::NONE,
        });
        assert!(handle_event(&app, &event).is_none());
    }

    #[test]
    fn handle_event_focus_returns_none() {
        let app = App::new(Config::default());
        assert!(handle_event(&app, &Event::FocusGained).is_none());
        assert!(handle_event(&app, &Event::FocusLost).is_none());
    }

    #[test]
    fn handle_event_init_produces_message_init() {
        let app = App::new(Config::default());
        let msg = handle_event(&app, &Event::Init);
        assert!(matches!(msg, Some(Message::Init)));
    }

    #[test]
    fn handle_event_refresh_via_keybinding() {
        let app = App::new(Config::default());
        // Ctrl-r is the default Refresh keybinding
        let event = make_key_event(KeyCode::Char('r'), KeyModifiers::CONTROL);
        let msg = handle_event(&app, &event);
        assert!(matches!(msg, Some(Message::Refresh)));
    }

    #[test]
    fn handle_event_scroll_down_via_j() {
        let app = App::new(Config::default());
        let event = make_key_event(KeyCode::Char('j'), KeyModifiers::NONE);
        let msg = handle_event(&app, &event);
        assert!(matches!(msg, Some(Message::ScrollDown)));
    }

    #[test]
    fn handle_event_scroll_up_via_k() {
        let app = App::new(Config::default());
        let event = make_key_event(KeyCode::Char('k'), KeyModifiers::NONE);
        let msg = handle_event(&app, &event);
        assert!(matches!(msg, Some(Message::ScrollUp)));
    }

    #[test]
    fn handle_event_half_page_down_via_ctrl_d() {
        let app = App::new(Config::default());
        let event = make_key_event(KeyCode::Char('d'), KeyModifiers::CONTROL);
        let msg = handle_event(&app, &event);
        assert!(matches!(msg, Some(Message::HalfPageDown)));
    }

    #[test]
    fn handle_event_half_page_up_via_ctrl_u() {
        let app = App::new(Config::default());
        let event = make_key_event(KeyCode::Char('u'), KeyModifiers::CONTROL);
        let msg = handle_event(&app, &event);
        assert!(matches!(msg, Some(Message::HalfPageUp)));
    }

    #[test]
    fn handle_event_select_via_enter() {
        let app = App::new(Config::default());
        let event = make_key_event(KeyCode::Enter, KeyModifiers::NONE);
        let msg = handle_event(&app, &event);
        assert!(matches!(msg, Some(Message::Select)));
    }

    #[test]
    fn handle_event_next_mailbox_via_tab() {
        let app = App::new(Config::default());
        let event = make_key_event(KeyCode::Tab, KeyModifiers::NONE);
        let msg = handle_event(&app, &event);
        assert!(matches!(msg, Some(Message::NextMailbox)));
    }

    #[test]
    fn handle_event_prev_mailbox_via_shift_tab() {
        let app = App::new(Config::default());
        let event = make_key_event(KeyCode::Tab, KeyModifiers::SHIFT);
        let msg = handle_event(&app, &event);
        assert!(matches!(msg, Some(Message::PrevMailbox)));
    }

    #[test]
    fn handle_event_toggle_focus_via_ctrl_s() {
        let app = App::new(Config::default());
        let event = make_key_event(KeyCode::Char('s'), KeyModifiers::CONTROL);
        let msg = handle_event(&app, &event);
        assert!(matches!(msg, Some(Message::ToggleFocus)));
    }

    #[test]
    fn handle_event_init_via_init_event() {
        let app = App::new(Config::default());
        let event = Event::Init;
        let msg = handle_event(&app, &event);
        assert!(matches!(msg, Some(Message::Init)));
    }

    #[test]
    fn handle_search_key_char_input() {
        let key = KeyEvent::new_with_kind(
            KeyCode::Char('a'),
            KeyModifiers::NONE,
            KeyEventKind::Press,
        );
        assert!(matches!(
            handle_search_key(key),
            Some(Message::SearchInput('a'))
        ));
    }

    #[test]
    fn handle_search_key_backspace() {
        let key = KeyEvent::new_with_kind(
            KeyCode::Backspace,
            KeyModifiers::NONE,
            KeyEventKind::Press,
        );
        assert!(matches!(
            handle_search_key(key),
            Some(Message::SearchInput('\x08'))
        ));
    }

    #[test]
    fn handle_search_key_enter_submits() {
        let key = KeyEvent::new_with_kind(
            KeyCode::Enter,
            KeyModifiers::NONE,
            KeyEventKind::Press,
        );
        assert!(matches!(handle_search_key(key), Some(Message::SearchSubmit)));
    }

    #[test]
    fn handle_search_key_esc_cancels() {
        let key = KeyEvent::new_with_kind(
            KeyCode::Esc,
            KeyModifiers::NONE,
            KeyEventKind::Press,
        );
        assert!(matches!(handle_search_key(key), Some(Message::SearchCancel)));
    }

    #[test]
    fn handle_search_key_ctrl_char_ignored() {
        let key = KeyEvent::new_with_kind(
            KeyCode::Char('a'),
            KeyModifiers::CONTROL,
            KeyEventKind::Press,
        );
        assert!(handle_search_key(key).is_none());
    }

    #[test]
    fn handle_search_key_unknown_returns_none() {
        let key = KeyEvent::new_with_kind(
            KeyCode::F(1),
            KeyModifiers::NONE,
            KeyEventKind::Press,
        );
        assert!(handle_search_key(key).is_none());
    }

    #[test]
    fn action_to_message_covers_all_variants() {
        use crate::config::keys::KeyAction;
        let pairs: Vec<(KeyAction, Message)> = vec![
            (KeyAction::Quit, Message::Quit),
            (KeyAction::Refresh, Message::Refresh),
            (KeyAction::ScrollDown, Message::ScrollDown),
            (KeyAction::ScrollUp, Message::ScrollUp),
            (KeyAction::ScrollHalfPageDown, Message::HalfPageDown),
            (KeyAction::ScrollHalfPageUp, Message::HalfPageUp),
            (KeyAction::OpenThread, Message::Select),
            (KeyAction::NextMailbox, Message::NextMailbox),
            (KeyAction::PrevMailbox, Message::PrevMailbox),
            (KeyAction::FocusSidebar, Message::ToggleFocus),
            (KeyAction::CloseThread, Message::Back),
            (KeyAction::Help, Message::ToggleHelp),
            (KeyAction::NextPage, Message::NextPage),
            (KeyAction::PrevPage, Message::PrevPage),
            (KeyAction::Search, Message::SearchStart),
            (KeyAction::ViewRawLog, Message::ViewRawLog),
            (KeyAction::BookmarkToggle, Message::BookmarkToggle),
            (KeyAction::NextComment, Message::NextComment),
            (KeyAction::PrevComment, Message::PrevComment),
            (KeyAction::ViewBaselineLog, Message::ViewBaselineLog),
            (KeyAction::ToggleListContent, Message::ToggleListContent),
            (KeyAction::BookmarkFilter, Message::ToggleBookmarkFilter),
            (KeyAction::CycleSort, Message::CycleSort),
            (KeyAction::ReverseSort, Message::ReverseSort),
        ];
        for (action, expected) in pairs {
            let result = action_to_message(action);
            assert!(result.is_some(), "action_to_message({action:?}) returned None");
            assert_eq!(
                std::mem::discriminant(&result.unwrap()),
                std::mem::discriminant(&expected),
                "mismatch for {action:?}"
            );
        }
    }

    #[test]
    fn handle_event_search_mode_routes_to_search_key() {
        let mut app = App::new(Config::default());
        app.input_mode = crate::app::InputMode::Search;
        let event = make_key_event(KeyCode::Char('x'), KeyModifiers::NONE);
        let msg = handle_event(&app, &event);
        assert!(matches!(msg, Some(Message::SearchInput('x'))));
    }
}
