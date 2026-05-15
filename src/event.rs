//! Terminal event types and event-to-message translation.
//!
//! Defines the `Event` enum and the `handle_event()` function
//! that translates events into `Message` values via the
//! keybinding system.

use crate::app::App;
use crate::config::keys::{KeyAction, KeyCombo};
use crate::update::Message;
use crossterm::event::{KeyEvent, KeyEventKind, MouseEvent};

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
            let combo = KeyCombo::new(key_event.code, key_event.modifiers);
            app.config
                .keybindings
                .action_for(&combo)
                .and_then(action_to_message)
        }
        Event::Init
        | Event::Error
        | Event::Key(_)
        | Event::Mouse(_)
        | Event::FocusGained
        | Event::FocusLost
        | Event::Paste(_) => None,
    }
}

/// Map a semantic key action to a `Message`.
///
/// Actions that don't yet have corresponding UI features return `None`.
fn action_to_message(action: KeyAction) -> Option<Message> {
    match action {
        KeyAction::Quit => Some(Message::Quit),
        // Other actions will produce messages when their UI slugs land
        KeyAction::ScrollDown
        | KeyAction::ScrollUp
        | KeyAction::ScrollHalfPageDown
        | KeyAction::ScrollHalfPageUp
        | KeyAction::NextMailbox
        | KeyAction::PrevMailbox
        | KeyAction::OpenThread
        | KeyAction::CloseThread
        | KeyAction::Refresh
        | KeyAction::Search
        | KeyAction::BookmarkToggle
        | KeyAction::ViewRawLog
        | KeyAction::Help
        | KeyAction::FocusSidebar
        | KeyAction::NextComment
        | KeyAction::PrevComment => None,
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
}
