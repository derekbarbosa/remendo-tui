//! Application state and lifecycle.
//!
//! Defines the top-level `App` struct (all application state) and
//! the `RunningState` enum controlling the main event loop.

use crate::bookmarks::BookmarkStore;
use crate::client::types::ListParams;
use crate::config::Config;
use crate::models::{EmailMessage, MailingList, Paginated, Patchset, PatchsetDetail, ServerStats};
use std::path::PathBuf;

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
    /// Context for the loading screen (patchset being fetched).
    pub loading_context: Option<LoadingContext>,
    /// Cached server stats from the last successful `/api/stats` response.
    pub stats: Option<ServerStats>,
    /// Whether the help overlay is currently visible.
    pub show_help: bool,
    /// Vertical scroll offset for the detail view content.
    pub detail_scroll_offset: usize,
    /// Current query parameters for the patchset list.
    /// Shared by pagination, search, and mailing list filter.
    pub list_params: ListParams,
    /// Current keyboard input mode.
    pub input_mode: InputMode,
    /// Contents of the search input buffer (while typing).
    pub search_buffer: String,
    /// Cursor position within `search_buffer` (char index).
    pub search_cursor: usize,
    /// Which section of the sidebar has focus.
    pub sidebar_section: SidebarSection,
    /// Scroll index within the mailing list section (0 = "All").
    pub sidebar_list_index: usize,
    /// Set of bookmarked patchsets.
    pub bookmarks: BookmarkStore,
    /// Path to the bookmarks persistence file.
    pub bookmarks_path: PathBuf,
    /// Cached line positions of comment boundaries in the detail view.
    pub comment_positions: Vec<usize>,
    /// Index into `comment_positions` for the current comment.
    pub current_comment_index: Option<usize>,
    /// What content the list view is displaying (patchsets or messages).
    pub list_content: ListContent,
    /// Currently displayed message list (when `list_content == Messages`).
    pub messages: Paginated<EmailMessage>,
    /// Loaded message detail for the message detail view.
    pub selected_message: Option<EmailMessage>,
    /// Which column the patchset list is sorted by.
    pub sort_column: SortColumn,
    /// Current sort direction.
    pub sort_direction: SortDirection,
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
            loading_context: None,
            stats: None,
            show_help: false,
            detail_scroll_offset: 0,
            list_params: ListParams::default(),
            input_mode: InputMode::default(),
            search_buffer: String::new(),
            search_cursor: 0,
            sidebar_section: SidebarSection::default(),
            sidebar_list_index: 0,
            bookmarks: BookmarkStore::new(),
            bookmarks_path: PathBuf::new(),
            comment_positions: vec![],
            current_comment_index: None,
            list_content: ListContent::default(),
            messages: Paginated {
                items: vec![],
                total: 0,
                page: 1,
                per_page: 50,
            },
            selected_message: None,
            sort_column: SortColumn::Default,
            sort_direction: SortDirection::Ascending,
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
    /// Loading a patchset detail (shows confirmation dialog).
    Loading,
    /// Showing the detail view for a selected patchset.
    Detail,
}

/// Context for the loading screen — summary of the patchset being fetched.
#[derive(Debug, Clone, Default)]
pub struct LoadingContext {
    /// Database ID of the patchset being loaded.
    pub patchset_id: i64,
    /// Subject line for visual confirmation.
    pub subject: String,
    /// Status text.
    pub status: String,
}

/// Which section of the sidebar has focus when the sidebar is active.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum SidebarSection {
    /// The remotes list.
    #[default]
    Remotes,
    /// The mailing lists.
    MailingLists,
}

/// Which column the patchset list is sorted by.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum SortColumn {
    /// API-returned order, no comparator applied.
    #[default]
    Default,
    /// Sort by patchset status (lifecycle priority).
    Status,
    /// Sort by date.
    Date,
    /// Sort by total finding count.
    Findings,
    /// Sort by author name (lexicographic).
    Author,
}

/// Sort direction for the patchset list.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum SortDirection {
    /// Ascending order (A→Z, oldest→newest, lowest→highest).
    #[default]
    Ascending,
    /// Descending order (Z→A, newest→oldest, highest→lowest).
    Descending,
}

/// What content the list view is displaying.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum ListContent {
    /// Showing patchsets (default).
    #[default]
    Patchsets,
    /// Showing mailing list messages.
    Messages,
}

/// What mode the keyboard is operating in.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    /// Normal keybinding mode.
    #[default]
    Normal,
    /// Text input mode for the search bar.
    Search,
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
