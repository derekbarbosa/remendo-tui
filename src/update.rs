//! Application state updater (TEA `update` function).
//!
//! The `Message` enum defines every action the app can take.
//! The `update()` function is a pure state transition — it takes
//! a mutable `App` reference and a `Message`, modifies state,
//! and returns a `Cmd` describing any async side-effects to perform.

use crate::app::{App, FocusPanel, InputMode, RunningState, SidebarSection, ViewMode};
use crate::client::ApiError;
use crate::client::types::ListParams;
use crate::cmd::Cmd;
use crate::models::{MailingList, PatchId, Paginated, Patchset, PatchsetDetail, ServerStats};

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
    /// Move selection down one row.
    ScrollDown,
    /// Move selection up one row.
    ScrollUp,
    /// Move selection down half a page.
    HalfPageDown,
    /// Move selection up half a page.
    HalfPageUp,
    /// Select the currently highlighted patchset.
    Select,
    /// Switch to the next remote/mailbox.
    NextMailbox,
    /// Switch to the previous remote/mailbox.
    PrevMailbox,
    /// Toggle focus between sidebar and main pane.
    ToggleFocus,
    /// Navigate back: dismiss overlay, or return from detail to list.
    Back,
    /// Toggle the help overlay visibility.
    ToggleHelp,
    /// Navigate to the next page of patchsets.
    NextPage,
    /// Navigate to the previous page of patchsets.
    PrevPage,
    /// Enter search input mode.
    SearchStart,
    /// Insert a character into the search buffer.
    SearchInput(char),
    /// Submit the search query (Enter).
    SearchSubmit,
    /// Cancel search mode (Esc).
    SearchCancel,
    /// Open the raw review log in the configured editor.
    ViewRawLog,
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
            Cmd::FetchPatchsets(app.list_params.clone()),
            Cmd::FetchStats,
        ]),
        Message::Refresh => Cmd::Batch(vec![
            Cmd::FetchPatchsets(app.list_params.clone()),
            Cmd::FetchLists,
            Cmd::FetchStats,
        ]),
        Message::Tick | Message::Render => Cmd::None,
        Message::Resize(_w, h) => {
            app.terminal_height = h;
            Cmd::None
        }
        Message::PatchsetsLoaded(result) => handle_patchsets_loaded(app, result),
        Message::PatchsetDetailLoaded(result) => handle_detail_loaded(app, *result),
        Message::StatsLoaded(result) => handle_stats_loaded(app, result),
        Message::ListsLoaded(result) => handle_lists_loaded(app, result),
        Message::ScrollDown
        | Message::ScrollUp
        | Message::HalfPageDown
        | Message::HalfPageUp => handle_scroll(app, &msg),
        Message::Select => handle_select(app),
        Message::NextMailbox => {
            if app.config.remotes.is_empty() {
                return Cmd::None;
            }
            let new_idx = (app.active_remote_index + 1) % app.config.remotes.len();
            switch_remote(app, new_idx)
        }
        Message::PrevMailbox => {
            if app.config.remotes.is_empty() {
                return Cmd::None;
            }
            let len = app.config.remotes.len();
            let new_idx = app.active_remote_index.checked_sub(1).unwrap_or(len - 1);
            switch_remote(app, new_idx)
        }
        Message::ToggleFocus => {
            app.focus = match app.focus {
                FocusPanel::PatchsetList => FocusPanel::Sidebar,
                FocusPanel::Sidebar => FocusPanel::PatchsetList,
            };
            Cmd::None
        }
        Message::Back => {
            if app.show_help {
                app.show_help = false;
                return Cmd::None;
            }
            if app.view_mode == ViewMode::Detail {
                app.view_mode = ViewMode::List;
                app.selected_detail = None;
                app.detail_scroll_offset = 0;
            }
            Cmd::None
        }
        Message::ToggleHelp => {
            app.show_help = !app.show_help;
            Cmd::None
        }
        Message::NextPage => {
            let total_pages = app.patchsets.total_pages();
            if total_pages > 0 && app.list_params.page < total_pages {
                app.list_params.page += 1;
                app.selected_index = 0;
                Cmd::FetchPatchsets(app.list_params.clone())
            } else {
                Cmd::None
            }
        }
        Message::PrevPage => {
            if app.list_params.page > 1 {
                app.list_params.page -= 1;
                app.selected_index = 0;
                Cmd::FetchPatchsets(app.list_params.clone())
            } else {
                Cmd::None
            }
        }
        Message::SearchStart
        | Message::SearchInput(_)
        | Message::SearchSubmit
        | Message::SearchCancel => handle_search(app, &msg),
        Message::ViewRawLog => handle_view_raw_log(app),
    }
}

/// Handle `Message::ViewRawLog` — collect review content and open in editor.
fn handle_view_raw_log(app: &App) -> Cmd {
    use std::fmt::Write;

    let Some(ref detail) = app.selected_detail else {
        return Cmd::None;
    };

    let mut log = String::new();
    for review in &detail.reviews {
        let _ = writeln!(
            log,
            "=== Review {} (patch {}) ===",
            review.id, review.patch_id
        );
        let _ = writeln!(
            log,
            "Status: {} | Model: {}",
            review.status,
            review.model.as_deref().unwrap_or("?")
        );
        if let Some(ref summary) = review.summary {
            let _ = write!(log, "\n--- Summary ---\n{summary}\n");
        }
        if let Some(ref inline) = review.inline_review {
            let _ = write!(log, "\n--- Inline Review ---\n{inline}\n");
        }
        log.push_str("\n\n");
    }

    if log.is_empty() {
        return Cmd::None;
    }

    let editor = app.config.resolved_editor();
    Cmd::OpenEditor {
        content: log,
        editor,
    }
}

/// Handle search lifecycle messages.
fn handle_search(app: &mut App, msg: &Message) -> Cmd {
    match *msg {
        Message::SearchStart => {
            app.input_mode = InputMode::Search;
            app.search_buffer = app.list_params.search.clone().unwrap_or_default();
            app.search_cursor = app.search_buffer.len();
            Cmd::None
        }
        Message::SearchInput(c) => {
            if c == '\x08' {
                // Backspace sentinel
                if app.search_cursor > 0 {
                    app.search_cursor -= 1;
                    app.search_buffer.remove(app.search_cursor);
                }
            } else {
                app.search_buffer.insert(app.search_cursor, c);
                app.search_cursor += c.len_utf8();
            }
            Cmd::None
        }
        Message::SearchSubmit => {
            app.input_mode = InputMode::Normal;
            app.list_params.search = if app.search_buffer.is_empty() {
                None
            } else {
                Some(app.search_buffer.clone())
            };
            app.list_params.page = 1;
            app.selected_index = 0;
            app.search_buffer.clear();
            app.search_cursor = 0;
            Cmd::FetchPatchsets(app.list_params.clone())
        }
        Message::SearchCancel => {
            app.input_mode = InputMode::Normal;
            app.search_buffer.clear();
            app.search_cursor = 0;
            Cmd::None
        }
        _ => Cmd::None,
    }
}

/// Handle API response for patchset list.
fn handle_patchsets_loaded(app: &mut App, result: Result<Paginated<Patchset>, ApiError>) -> Cmd {
    match result {
        Ok(paginated) => {
            app.patchsets = paginated;
            app.error_state = None;
        }
        Err(e) => app.error_state = Some(e.to_string()),
    }
    Cmd::None
}

/// Handle `Message::Select` — dispatch based on focus panel.
fn handle_select(app: &mut App) -> Cmd {
    match app.focus {
        FocusPanel::PatchsetList => {
            let Some(patchset) = app.patchsets.items.get(app.selected_index) else {
                return Cmd::None;
            };
            app.detail_scroll_offset = 0;
            Cmd::FetchPatchsetDetail(PatchId::Numeric(patchset.id))
        }
        FocusPanel::Sidebar => handle_sidebar_select(app),
    }
}

/// Handle API response for patchset detail.
fn handle_detail_loaded(app: &mut App, result: Result<PatchsetDetail, ApiError>) -> Cmd {
    match result {
        Ok(detail) => {
            app.selected_detail = Some(detail);
            app.view_mode = ViewMode::Detail;
            app.error_state = None;
        }
        Err(e) => app.error_state = Some(e.to_string()),
    }
    Cmd::None
}

/// Handle API response for server stats.
fn handle_stats_loaded(app: &mut App, result: Result<ServerStats, ApiError>) -> Cmd {
    match result {
        Ok(stats) => {
            app.stats = Some(stats);
            app.error_state = None;
        }
        Err(e) => app.error_state = Some(e.to_string()),
    }
    Cmd::None
}

/// Handle API response for mailing lists.
fn handle_lists_loaded(app: &mut App, result: Result<Vec<MailingList>, ApiError>) -> Cmd {
    match result {
        Ok(lists) => {
            app.mailing_lists = lists;
            app.error_state = None;
        }
        Err(e) => app.error_state = Some(e.to_string()),
    }
    Cmd::None
}

/// Handle scroll/selection messages, routed by focus panel.
fn handle_scroll(app: &mut App, msg: &Message) -> Cmd {
    match app.focus {
        FocusPanel::PatchsetList => {}
        FocusPanel::Sidebar => return handle_sidebar_scroll(app, msg),
    }
    // Detail view: scroll the detail content
    if app.view_mode == ViewMode::Detail {
        match *msg {
            Message::ScrollDown => app.detail_scroll_offset += 1,
            Message::ScrollUp => {
                app.detail_scroll_offset = app.detail_scroll_offset.saturating_sub(1);
            }
            Message::HalfPageDown => {
                app.detail_scroll_offset +=
                    usize::from(app.terminal_height / 2).max(1);
            }
            Message::HalfPageUp => {
                let half = usize::from(app.terminal_height / 2).max(1);
                app.detail_scroll_offset = app.detail_scroll_offset.saturating_sub(half);
            }
            _ => {}
        }
        return Cmd::None;
    }
    // List view: scroll the selected index
    match *msg {
        Message::ScrollDown => {
            let len = app.patchsets.items.len();
            if len > 0 {
                app.selected_index = (app.selected_index + 1).min(len - 1);
            }
        }
        Message::ScrollUp => {
            app.selected_index = app.selected_index.saturating_sub(1);
        }
        Message::HalfPageDown => {
            let len = app.patchsets.items.len();
            let half = usize::from(app.terminal_height / 2).max(1);
            if len > 0 {
                app.selected_index = (app.selected_index + half).min(len - 1);
            }
        }
        Message::HalfPageUp => {
            let half = usize::from(app.terminal_height / 2).max(1);
            app.selected_index = app.selected_index.saturating_sub(half);
        }
        _ => {}
    }
    Cmd::None
}

/// Handle scroll within the sidebar — navigate between remotes and mailing lists.
fn handle_sidebar_scroll(app: &mut App, msg: &Message) -> Cmd {
    let is_down = matches!(*msg, Message::ScrollDown | Message::HalfPageDown);
    let is_up = matches!(*msg, Message::ScrollUp | Message::HalfPageUp);

    match app.sidebar_section {
        SidebarSection::Remotes => {
            if is_down {
                if app.active_remote_index + 1 < app.config.remotes.len() {
                    app.active_remote_index += 1;
                } else if !app.mailing_lists.is_empty() {
                    // Transition to mailing lists
                    app.sidebar_section = SidebarSection::MailingLists;
                    app.sidebar_list_index = 0;
                }
            } else if is_up {
                app.active_remote_index = app.active_remote_index.saturating_sub(1);
            }
        }
        SidebarSection::MailingLists => {
            // +1 for the "All" entry at index 0
            let max_index = app.mailing_lists.len(); // 0=All, 1..len=lists
            if is_down {
                if app.sidebar_list_index < max_index {
                    app.sidebar_list_index += 1;
                }
            } else if is_up {
                if app.sidebar_list_index > 0 {
                    app.sidebar_list_index -= 1;
                } else {
                    // Transition back to remotes
                    app.sidebar_section = SidebarSection::Remotes;
                    app.active_remote_index =
                        app.config.remotes.len().saturating_sub(1);
                }
            }
        }
    }
    Cmd::None
}

/// Handle Enter in the sidebar — switch remote or apply mailing list filter.
fn handle_sidebar_select(app: &mut App) -> Cmd {
    match app.sidebar_section {
        SidebarSection::Remotes => switch_remote(app, app.active_remote_index),
        SidebarSection::MailingLists => {
            if app.sidebar_list_index == 0 {
                // "All" — clear filter
                app.list_params.mailing_list = None;
            } else {
                let idx = app.sidebar_list_index - 1;
                if let Some(list) = app.mailing_lists.get(idx) {
                    app.list_params.mailing_list =
                        Some(list.group.clone().unwrap_or_else(|| list.name.clone()));
                }
            }
            app.list_params.page = 1;
            app.selected_index = 0;
            Cmd::FetchPatchsets(app.list_params.clone())
        }
    }
}

/// Switch the active remote to the given index, clear patchset data,
/// and return a fetch command batch.
fn switch_remote(app: &mut App, new_index: usize) -> Cmd {
    let Some(remote) = app.config.remotes.get(new_index) else {
        return Cmd::None;
    };
    app.active_remote_index = new_index;
    app.active_remote = remote.name.clone();
    app.patchsets.items.clear();
    app.patchsets.total = 0;
    app.selected_index = 0;
    app.error_state = None;
    app.stats = None;
    app.view_mode = ViewMode::List;
    app.selected_detail = None;
    app.detail_scroll_offset = 0;
    app.list_params = ListParams::default();
    app.input_mode = InputMode::Normal;
    app.search_buffer.clear();
    app.search_cursor = 0;
    app.sidebar_section = SidebarSection::Remotes;
    app.sidebar_list_index = 0;
    Cmd::Batch(vec![
        Cmd::FetchPatchsets(app.list_params.clone()),
        Cmd::FetchLists,
        Cmd::FetchStats,
    ])
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
    fn refresh_returns_batch_with_three_fetches() {
        let mut app = App::new(Config::default());
        let cmd = update(&mut app, Message::Refresh);
        match cmd {
            Cmd::Batch(cmds) => {
                assert_eq!(cmds.len(), 3);
                assert!(matches!(cmds[0], Cmd::FetchPatchsets(_)));
                assert!(matches!(cmds[1], Cmd::FetchLists));
                assert!(matches!(cmds[2], Cmd::FetchStats));
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

    fn app_with_patchsets(count: usize) -> App {
        let mut app = App::new(Config::default());
        app.patchsets = Paginated {
            items: (0..count).map(|i| {
                let mut ps = Patchset::fixture();
                ps.id = i64::try_from(i).expect("test count fits i64");
                ps
            }).collect(),
            total: u32::try_from(count).expect("test count fits u32"),
            page: 1,
            per_page: 50,
        };
        app.terminal_height = 24;
        app
    }

    #[test]
    fn scroll_down_increments_index() {
        let mut app = app_with_patchsets(10);
        assert_eq!(app.selected_index, 0);
        let cmd = update(&mut app, Message::ScrollDown);
        assert_eq!(app.selected_index, 1);
        assert!(matches!(cmd, Cmd::None));
    }

    #[test]
    fn scroll_down_clamps_at_last_item() {
        let mut app = app_with_patchsets(5);
        app.selected_index = 4; // last item
        let cmd = update(&mut app, Message::ScrollDown);
        assert_eq!(app.selected_index, 4); // stays clamped
        assert!(matches!(cmd, Cmd::None));
    }

    #[test]
    fn scroll_down_empty_list_is_noop() {
        let mut app = app_with_patchsets(0);
        let cmd = update(&mut app, Message::ScrollDown);
        assert_eq!(app.selected_index, 0);
        assert!(matches!(cmd, Cmd::None));
    }

    #[test]
    fn scroll_up_decrements_index() {
        let mut app = app_with_patchsets(10);
        app.selected_index = 5;
        let cmd = update(&mut app, Message::ScrollUp);
        assert_eq!(app.selected_index, 4);
        assert!(matches!(cmd, Cmd::None));
    }

    #[test]
    fn scroll_up_clamps_at_zero() {
        let mut app = app_with_patchsets(10);
        app.selected_index = 0;
        let cmd = update(&mut app, Message::ScrollUp);
        assert_eq!(app.selected_index, 0);
        assert!(matches!(cmd, Cmd::None));
    }

    #[test]
    fn half_page_down_advances_by_half_height() {
        let mut app = app_with_patchsets(50);
        app.terminal_height = 24;
        app.selected_index = 0;
        let cmd = update(&mut app, Message::HalfPageDown);
        assert_eq!(app.selected_index, 12); // 24/2 = 12
        assert!(matches!(cmd, Cmd::None));
    }

    #[test]
    fn half_page_down_clamps_at_last_item() {
        let mut app = app_with_patchsets(10);
        app.terminal_height = 24;
        app.selected_index = 5;
        let cmd = update(&mut app, Message::HalfPageDown);
        assert_eq!(app.selected_index, 9); // clamped to last
        assert!(matches!(cmd, Cmd::None));
    }

    #[test]
    fn half_page_up_subtracts_half_height() {
        let mut app = app_with_patchsets(50);
        app.terminal_height = 24;
        app.selected_index = 20;
        let cmd = update(&mut app, Message::HalfPageUp);
        assert_eq!(app.selected_index, 8); // 20 - 12 = 8
        assert!(matches!(cmd, Cmd::None));
    }

    #[test]
    fn half_page_up_clamps_at_zero() {
        let mut app = app_with_patchsets(50);
        app.terminal_height = 24;
        app.selected_index = 3;
        let cmd = update(&mut app, Message::HalfPageUp);
        assert_eq!(app.selected_index, 0); // clamped to 0
        assert!(matches!(cmd, Cmd::None));
    }

    #[test]
    fn select_returns_fetch_detail() {
        let mut app = app_with_patchsets(5);
        app.selected_index = 2;
        let cmd = update(&mut app, Message::Select);
        assert_eq!(app.selected_index, 2); // unchanged
        // Should return FetchPatchsetDetail for the selected patchset
        assert!(matches!(cmd, Cmd::FetchPatchsetDetail(_)));
    }

    #[test]
    fn select_empty_list_returns_none() {
        let mut app = app_with_patchsets(0);
        let cmd = update(&mut app, Message::Select);
        assert!(matches!(cmd, Cmd::None));
    }

    #[test]
    fn detail_loaded_stores_detail_and_switches_view() {
        let mut app = App::new(Config::default());
        let detail = PatchsetDetail::fixture();
        let cmd = update(
            &mut app,
            Message::PatchsetDetailLoaded(Box::new(Ok(detail))),
        );
        assert!(matches!(cmd, Cmd::None));
        assert_eq!(app.view_mode, ViewMode::Detail);
        assert!(app.selected_detail.is_some());
        assert!(app.error_state.is_none());
    }

    #[test]
    fn detail_loaded_error_sets_error_state() {
        let mut app = App::new(Config::default());
        let err = ApiError::Network {
            source: "timeout".into(),
            remote: "test".to_string(),
        };
        let cmd = update(
            &mut app,
            Message::PatchsetDetailLoaded(Box::new(Err(err))),
        );
        assert!(matches!(cmd, Cmd::None));
        assert!(app.error_state.is_some());
        assert!(app.selected_detail.is_none());
    }

    #[test]
    fn resize_stores_terminal_height() {
        let mut app = App::new(Config::default());
        assert_eq!(app.terminal_height, 0);
        let cmd = update(&mut app, Message::Resize(120, 40));
        assert_eq!(app.terminal_height, 40);
        assert!(matches!(cmd, Cmd::None));
    }

    fn app_with_remotes(names: &[&str]) -> App {
        let mut config = Config::default();
        for name in names {
            config.remotes.push(crate::config::RemoteConfig::fixture(name));
        }
        App::new(config)
    }

    #[test]
    fn next_mailbox_cycles_forward() {
        let mut app = app_with_remotes(&["upstream", "staging", "local"]);
        assert_eq!(app.active_remote_index, 0);
        assert_eq!(app.active_remote, "upstream");

        let cmd = update(&mut app, Message::NextMailbox);
        assert_eq!(app.active_remote_index, 1);
        assert_eq!(app.active_remote, "staging");
        assert!(matches!(cmd, Cmd::Batch(_)));
    }

    #[test]
    fn next_mailbox_wraps_to_first() {
        let mut app = app_with_remotes(&["upstream", "staging"]);
        app.active_remote_index = 1;
        app.active_remote = "staging".to_string();

        let cmd = update(&mut app, Message::NextMailbox);
        assert_eq!(app.active_remote_index, 0);
        assert_eq!(app.active_remote, "upstream");
        assert!(matches!(cmd, Cmd::Batch(_)));
    }

    #[test]
    fn prev_mailbox_cycles_backward() {
        let mut app = app_with_remotes(&["upstream", "staging", "local"]);
        app.active_remote_index = 2;
        app.active_remote = "local".to_string();

        let cmd = update(&mut app, Message::PrevMailbox);
        assert_eq!(app.active_remote_index, 1);
        assert_eq!(app.active_remote, "staging");
        assert!(matches!(cmd, Cmd::Batch(_)));
    }

    #[test]
    fn prev_mailbox_wraps_to_last() {
        let mut app = app_with_remotes(&["upstream", "staging", "local"]);
        assert_eq!(app.active_remote_index, 0);

        let cmd = update(&mut app, Message::PrevMailbox);
        assert_eq!(app.active_remote_index, 2);
        assert_eq!(app.active_remote, "local");
        assert!(matches!(cmd, Cmd::Batch(_)));
    }

    #[test]
    fn next_mailbox_no_remotes_is_noop() {
        let mut app = App::new(Config::default());
        let cmd = update(&mut app, Message::NextMailbox);
        assert_eq!(app.active_remote_index, 0);
        assert!(matches!(cmd, Cmd::None));
    }

    #[test]
    fn prev_mailbox_no_remotes_is_noop() {
        let mut app = App::new(Config::default());
        let cmd = update(&mut app, Message::PrevMailbox);
        assert_eq!(app.active_remote_index, 0);
        assert!(matches!(cmd, Cmd::None));
    }

    #[test]
    fn mailbox_switch_clears_patchsets_and_resets_selection() {
        let mut app = app_with_remotes(&["upstream", "staging"]);
        // Simulate loaded patchsets
        app.patchsets = Paginated {
            items: vec![Patchset::fixture()],
            total: 1,
            page: 1,
            per_page: 50,
        };
        app.selected_index = 5;
        app.error_state = Some("old error".to_string());

        update(&mut app, Message::NextMailbox);
        assert!(app.patchsets.items.is_empty());
        assert_eq!(app.patchsets.total, 0);
        assert_eq!(app.selected_index, 0);
        assert!(app.error_state.is_none());
    }

    #[test]
    fn mailbox_switch_returns_fetch_batch() {
        let mut app = app_with_remotes(&["upstream", "staging"]);
        let cmd = update(&mut app, Message::NextMailbox);
        match cmd {
            Cmd::Batch(cmds) => {
                assert_eq!(cmds.len(), 3);
                assert!(matches!(cmds[0], Cmd::FetchPatchsets(_)));
                assert!(matches!(cmds[1], Cmd::FetchLists));
                assert!(matches!(cmds[2], Cmd::FetchStats));
            }
            other => panic!("expected Cmd::Batch, got {other:?}"),
        }
    }

    #[test]
    fn toggle_focus_switches_panels() {
        let mut app = App::new(Config::default());
        assert_eq!(app.focus, FocusPanel::PatchsetList);

        let cmd = update(&mut app, Message::ToggleFocus);
        assert_eq!(app.focus, FocusPanel::Sidebar);
        assert!(matches!(cmd, Cmd::None));

        let cmd = update(&mut app, Message::ToggleFocus);
        assert_eq!(app.focus, FocusPanel::PatchsetList);
        assert!(matches!(cmd, Cmd::None));
    }

    #[test]
    fn scroll_ignored_when_sidebar_focused() {
        let mut app = app_with_patchsets(10);
        app.focus = FocusPanel::Sidebar;
        app.selected_index = 0;

        update(&mut app, Message::ScrollDown);
        assert_eq!(app.selected_index, 0); // unchanged

        update(&mut app, Message::ScrollUp);
        assert_eq!(app.selected_index, 0);

        update(&mut app, Message::HalfPageDown);
        assert_eq!(app.selected_index, 0);

        update(&mut app, Message::HalfPageUp);
        assert_eq!(app.selected_index, 0);
    }
}
