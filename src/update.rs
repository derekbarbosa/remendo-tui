//! Application state updater (TEA `update` function).
//!
//! The `Message` enum defines every action the app can take.
//! The `update()` function is a pure state transition — it takes
//! a mutable `App` reference and a `Message`, and modifies state
//! accordingly. No I/O, no side effects.

use crate::app::{App, RunningState};
use crate::models::{MailingList, Paginated, Patchset, PatchsetDetail, ServerStats};

/// Placeholder error type for API responses.
///
/// Will be replaced by `ApiError` from `04-api-client` when that
/// feature lands. Using a boxed error keeps the `Message` shape
/// stable across the dependency boundary.
pub type ApiResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

/// Every action the application can take.
///
/// Produced by [`crate::event::handle_event`] from raw terminal
/// events, or by background tasks delivering API responses.
#[derive(Debug)]
pub enum Message {
    /// Exit the application.
    Quit,
    /// Periodic tick for background work.
    Tick,
    /// Time to render a new frame.
    Render,
    /// Terminal was resized.
    Resize(u16, u16),
    /// Patchset list loaded from API.
    PatchsetsLoaded(ApiResult<Paginated<Patchset>>),
    /// Patchset detail loaded from API.
    PatchsetDetailLoaded(Box<ApiResult<PatchsetDetail>>),
    /// Server stats loaded from API.
    StatsLoaded(ApiResult<ServerStats>),
    /// Mailing lists loaded from API.
    ListsLoaded(ApiResult<Vec<MailingList>>),
}

/// Apply a message to the application state.
///
/// This is the core TEA update function — a pure state transition
/// with no I/O or side effects.
///
/// # Examples
///
/// ```
/// use remendo_tui::app::App;
/// use remendo_tui::config::Config;
/// use remendo_tui::update::{update, Message};
/// use remendo_tui::app::RunningState;
///
/// let mut app = App::new(Config::default());
/// update(&mut app, Message::Quit);
/// assert_eq!(app.running_state, RunningState::Done);
/// ```
pub fn update(app: &mut App, msg: Message) {
    match msg {
        Message::Quit => app.running_state = RunningState::Done,
        Message::Tick | Message::Render => { /* periodic/render — no-op for now */ }
        Message::Resize(_w, _h) => { /* future: store for layout calculations */ }
        Message::PatchsetsLoaded(result) => match result {
            Ok(paginated) => {
                app.patchsets = paginated;
                app.error_state = None;
            }
            Err(e) => app.error_state = Some(e.to_string()),
        },
        Message::PatchsetDetailLoaded(result) => match *result {
            Ok(_detail) => { /* future: store in detail view state */ }
            Err(e) => app.error_state = Some(e.to_string()),
        },
        Message::StatsLoaded(result) => match result {
            Ok(_stats) => { /* future: store stats */ }
            Err(e) => app.error_state = Some(e.to_string()),
        },
        Message::ListsLoaded(result) => match result {
            Ok(lists) => {
                app.mailing_lists = lists;
                app.error_state = None;
            }
            Err(e) => app.error_state = Some(e.to_string()),
        },
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::config::Config;

    #[test]
    fn quit_sets_done() {
        let mut app = App::new(Config::default());
        update(&mut app, Message::Quit);
        assert_eq!(app.running_state, RunningState::Done);
        assert!(!app.is_running());
    }

    #[test]
    fn tick_and_render_are_noop() {
        let mut app = App::new(Config::default());
        update(&mut app, Message::Tick);
        assert!(app.is_running());
        update(&mut app, Message::Render);
        assert!(app.is_running());
    }

    #[test]
    fn resize_is_noop() {
        let mut app = App::new(Config::default());
        update(&mut app, Message::Resize(120, 40));
        assert!(app.is_running());
    }

    #[test]
    fn patchsets_loaded_success() {
        let mut app = App::new(Config::default());
        let paginated = Paginated {
            items: vec![Patchset::fixture()],
            total: 1,
            page: 1,
            per_page: 50,
        };
        update(&mut app, Message::PatchsetsLoaded(Ok(paginated)));
        assert_eq!(app.patchsets.items.len(), 1);
        assert!(app.error_state.is_none());
    }

    #[test]
    fn patchsets_loaded_error() {
        let mut app = App::new(Config::default());
        let err: Box<dyn std::error::Error + Send + Sync> = "network error".into();
        update(&mut app, Message::PatchsetsLoaded(Err(err)));
        assert!(app.error_state.is_some());
        assert!(
            app.error_state
                .as_deref()
                .is_some_and(|s| s.contains("network error"))
        );
    }

    #[test]
    fn lists_loaded_success() {
        let mut app = App::new(Config::default());
        let lists = vec![MailingList {
            name: "LKML".to_string(),
            group: Some("org.kernel.vger.linux-kernel".to_string()),
        }];
        update(&mut app, Message::ListsLoaded(Ok(lists)));
        assert_eq!(app.mailing_lists.len(), 1);
        assert!(app.error_state.is_none());
    }

    #[test]
    fn error_state_set_on_api_failure() {
        let mut app = App::new(Config::default());
        let err: Box<dyn std::error::Error + Send + Sync> = "timeout".into();
        update(&mut app, Message::StatsLoaded(Err(err)));
        assert_eq!(app.error_state.as_deref(), Some("timeout"));
    }
}
