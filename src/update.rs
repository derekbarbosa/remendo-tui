//! Application state updater (TEA `update` function).
//!
//! The `Message` enum defines every action the app can take.
//! The `update()` function is a pure state transition — it takes
//! a mutable `App` reference and a `Message`, modifies state,
//! and returns a `Cmd` describing any async side-effects to perform.

use crate::app::{App, RunningState};
use crate::client::ApiError;
use crate::client::types::ListParams;
use crate::cmd::Cmd;
use crate::models::{MailingList, Paginated, Patchset, PatchsetDetail, ServerStats};

/// Every action the application can take.
///
/// Produced by [`crate::event::handle_event`] from raw terminal
/// events, or by background tasks delivering API responses.
#[derive(Debug)]
pub enum Message {
    /// Exit the application.
    Quit,
    /// Application should perform initial data load.
    Init,
    /// User requested a refresh of the current view.
    Refresh,
    /// Periodic tick for background work.
    Tick,
    /// Time to render a new frame.
    Render,
    /// Terminal was resized.
    Resize(u16, u16),
    /// Patchset list loaded from API.
    PatchsetsLoaded(Result<Paginated<Patchset>, ApiError>),
    /// Patchset detail loaded from API.
    PatchsetDetailLoaded(Box<Result<PatchsetDetail, ApiError>>),
    /// Server stats loaded from API.
    StatsLoaded(Result<ServerStats, ApiError>),
    /// Mailing lists loaded from API.
    ListsLoaded(Result<Vec<MailingList>, ApiError>),
}

/// Apply a message to the application state and return any
/// commands (async side-effects) to execute.
///
/// This is the core TEA update function — a pure state transition.
/// It never performs I/O directly; instead it returns `Cmd` values
/// that the runtime executes asynchronously.
///
/// # Examples
///
/// ```
/// use remendo_tui::app::App;
/// use remendo_tui::config::Config;
/// use remendo_tui::update::{update, Message};
/// use remendo_tui::cmd::Cmd;
/// use remendo_tui::app::RunningState;
///
/// let mut app = App::new(Config::default());
/// let cmd = update(&mut app, Message::Quit);
/// assert_eq!(app.running_state, RunningState::Done);
/// assert!(matches!(cmd, Cmd::None));
/// ```
pub fn update(app: &mut App, msg: Message) -> Cmd {
    match msg {
        Message::Quit => {
            app.running_state = RunningState::Done;
            Cmd::None
        }
        Message::Init => Cmd::Batch(vec![
            Cmd::FetchLists,
            Cmd::FetchPatchsets(ListParams::default()),
            Cmd::FetchStats,
        ]),
        Message::Refresh => Cmd::Batch(vec![
            Cmd::FetchPatchsets(ListParams::default()),
            Cmd::FetchLists,
        ]),
        Message::Tick | Message::Render => Cmd::None,
        Message::Resize(_w, _h) => Cmd::None,
        Message::PatchsetsLoaded(result) => {
            match result {
                Ok(paginated) => {
                    app.patchsets = paginated;
                    app.error_state = None;
                }
                Err(e) => app.error_state = Some(e.to_string()),
            }
            Cmd::None
        }
        Message::PatchsetDetailLoaded(result) => {
            match *result {
                Ok(_detail) => { /* future: store in detail view state */ }
                Err(e) => app.error_state = Some(e.to_string()),
            }
            Cmd::None
        }
        Message::StatsLoaded(result) => {
            match result {
                Ok(_stats) => { /* future: store stats */ }
                Err(e) => app.error_state = Some(e.to_string()),
            }
            Cmd::None
        }
        Message::ListsLoaded(result) => {
            match result {
                Ok(lists) => {
                    app.mailing_lists = lists;
                    app.error_state = None;
                }
                Err(e) => app.error_state = Some(e.to_string()),
            }
            Cmd::None
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::config::Config;

    #[test]
    fn quit_returns_none() {
        let mut app = App::new(Config::default());
        let cmd = update(&mut app, Message::Quit);
        assert_eq!(app.running_state, RunningState::Done);
        assert!(matches!(cmd, Cmd::None));
    }

    #[test]
    fn init_returns_batch_with_three_fetches() {
        let mut app = App::new(Config::default());
        let cmd = update(&mut app, Message::Init);
        match cmd {
            Cmd::Batch(cmds) => {
                assert_eq!(cmds.len(), 3);
                assert!(matches!(cmds[0], Cmd::FetchLists));
                assert!(matches!(cmds[1], Cmd::FetchPatchsets(_)));
                assert!(matches!(cmds[2], Cmd::FetchStats));
            }
            other => panic!("expected Cmd::Batch, got {other:?}"),
        }
    }

    #[test]
    fn refresh_returns_batch_with_two_fetches() {
        let mut app = App::new(Config::default());
        let cmd = update(&mut app, Message::Refresh);
        match cmd {
            Cmd::Batch(cmds) => {
                assert_eq!(cmds.len(), 2);
                assert!(matches!(cmds[0], Cmd::FetchPatchsets(_)));
                assert!(matches!(cmds[1], Cmd::FetchLists));
            }
            other => panic!("expected Cmd::Batch, got {other:?}"),
        }
    }

    #[test]
    fn tick_and_render_return_none() {
        let mut app = App::new(Config::default());
        assert!(matches!(update(&mut app, Message::Tick), Cmd::None));
        assert!(matches!(update(&mut app, Message::Render), Cmd::None));
    }

    #[test]
    fn resize_returns_none() {
        let mut app = App::new(Config::default());
        assert!(matches!(
            update(&mut app, Message::Resize(120, 40)),
            Cmd::None
        ));
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
        let cmd = update(&mut app, Message::PatchsetsLoaded(Ok(paginated)));
        assert_eq!(app.patchsets.items.len(), 1);
        assert!(app.error_state.is_none());
        assert!(matches!(cmd, Cmd::None));
    }

    #[test]
    fn patchsets_loaded_error() {
        let mut app = App::new(Config::default());
        let err = ApiError::Network {
            source: "connection refused".into(),
            remote: "test".to_string(),
        };
        let cmd = update(&mut app, Message::PatchsetsLoaded(Err(err)));
        assert!(app.error_state.is_some());
        assert!(matches!(cmd, Cmd::None));
    }

    #[test]
    fn lists_loaded_success() {
        let mut app = App::new(Config::default());
        let lists = vec![MailingList {
            name: "LKML".to_string(),
            group: Some("org.kernel.vger.linux-kernel".to_string()),
        }];
        let cmd = update(&mut app, Message::ListsLoaded(Ok(lists)));
        assert_eq!(app.mailing_lists.len(), 1);
        assert!(matches!(cmd, Cmd::None));
    }

    #[test]
    fn error_state_set_on_api_failure() {
        let mut app = App::new(Config::default());
        let err = ApiError::Timeout {
            endpoint: "/api/stats".to_string(),
            duration: std::time::Duration::from_secs(5),
        };
        let cmd = update(&mut app, Message::StatsLoaded(Err(err)));
        assert!(app.error_state.is_some());
        assert!(matches!(cmd, Cmd::None));
    }
}
