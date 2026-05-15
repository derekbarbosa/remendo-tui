//! Application state and lifecycle.
//!
//! Defines the top-level `App` struct (all application state) and
//! the `RunningState` enum controlling the main event loop.

use crate::config::Config;
use crate::models::{MailingList, Paginated, Patchset};

/// Top-level application state.
///
/// All mutable state the TUI needs lives here. The `update()` function
/// in [`crate::update`] is the sole mutator — the TEA pattern keeps
/// state transitions pure and testable.
pub struct App {
    /// Whether the app is running or exiting.
    pub running_state: RunningState,
    /// Loaded configuration.
    pub config: Config,
    /// Currently displayed patchset list.
    pub patchsets: Paginated<Patchset>,
    /// Name of the active remote/mailbox.
    pub active_remote: String,
    /// Current error message to display, if any.
    pub error_state: Option<String>,
    /// Available mailing lists from the active remote.
    pub mailing_lists: Vec<MailingList>,
}

impl App {
    /// Create a new application with the given configuration.
    #[must_use]
    pub fn new(config: Config) -> Self {
        let active_remote = config
            .remotes
            .first()
            .map(|r| r.name.clone())
            .unwrap_or_default();

        Self {
            running_state: RunningState::Running,
            config,
            patchsets: Paginated {
                items: vec![],
                total: 0,
                page: 1,
                per_page: 50,
            },
            active_remote,
            error_state: None,
            mailing_lists: vec![],
        }
    }

    /// Whether the application should continue running.
    #[must_use]
    pub fn is_running(&self) -> bool {
        self.running_state == RunningState::Running
    }
}

/// Controls the main event loop lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunningState {
    /// The application is actively running.
    Running,
    /// The application should exit after the current frame.
    Done,
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn default_app_is_running() {
        let app = App::new(Config::default());
        assert_eq!(app.running_state, RunningState::Running);
        assert!(app.is_running());
    }

    #[test]
    fn app_with_remotes_sets_active() {
        let mut config = Config::default();
        config.remotes.push(crate::config::RemoteConfig {
            name: "upstream".to_string(),
            url: "https://sashiko.dev".to_string(),
            auth_env: None,
            timeout_seconds: 15,
            max_retries: 3,
        });
        let app = App::new(config);
        assert_eq!(app.active_remote, "upstream");
    }

    #[test]
    fn app_without_remotes_has_empty_active() {
        let app = App::new(Config::default());
        assert!(app.active_remote.is_empty());
    }

    #[test]
    fn running_state_variants() {
        assert_ne!(RunningState::Running, RunningState::Done);
    }
}
