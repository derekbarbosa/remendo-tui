//! Application state and lifecycle.
//!
//! Defines the top-level `App` struct (all application state) and
//! the `RunningState` enum controlling the main event loop.

use crate::config::Config;
use crate::models::{MailingList, Paginated, Patchset, PatchsetDetail, ServerStats};

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
    /// Index of the currently selected patchset in the list.
    pub selected_index: usize,
    /// Cached terminal height for half-page scroll calculation.
    pub terminal_height: u16,
    /// Index of the currently active remote in `config.remotes`.
    pub active_remote_index: usize,
    /// Which panel currently has keyboard focus.
    pub focus: FocusPanel,
    /// Current view mode (list vs. detail).
    pub view_mode: ViewMode,
    /// Loaded patchset detail for the detail view.
    pub selected_detail: Option<PatchsetDetail>,
    /// Cached server stats from the last successful `/api/stats` response.
    pub stats: Option<ServerStats>,
    /// Whether the help overlay is currently visible.
    pub show_help: bool,
    /// Vertical scroll offset for the detail view content.
    pub detail_scroll_offset: usize,
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
            selected_index: 0,
            terminal_height: 0,
            active_remote_index: 0,
            focus: FocusPanel::default(),
            view_mode: ViewMode::default(),
            selected_detail: None,
            stats: None,
            show_help: false,
            detail_scroll_offset: 0,
        }
    }

    /// Whether the application should continue running.
    #[must_use]
    pub fn is_running(&self) -> bool {
        self.running_state == RunningState::Running
    }
}

/// Which panel currently has keyboard focus.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum FocusPanel {
    /// The patchset list table (main pane).
    #[default]
    PatchsetList,
    /// The remote/mailbox sidebar.
    Sidebar,
}

/// Which view the main pane is displaying.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    /// Showing the patchset list table.
    #[default]
    List,
    /// Showing the detail view for a selected patchset.
    Detail,
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
        config.remotes.push(crate::config::RemoteConfig::fixture("upstream"));
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
