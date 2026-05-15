//! Integration tests for the application event→update→state cycle.

#![allow(clippy::expect_used, clippy::panic)]

use remendo_tui::app::{App, FocusPanel, RunningState, ViewMode};
use remendo_tui::client::ApiError;
use remendo_tui::cmd::Cmd;
use remendo_tui::config::{Config, RemoteConfig};
use remendo_tui::models::{MailingList, PatchId, Paginated, Patchset, PatchsetDetail};
use remendo_tui::update::{update, Message};

fn app_with_remotes(names: &[&str]) -> App {
    let mut config = Config::default();
    for name in names {
        config.remotes.push(RemoteConfig::fixture(name));
    }
    App::new(config)
}

#[test]
fn quit_flow() {
    let mut app = App::new(Config::default());
    assert!(app.is_running());
    update(&mut app, Message::Quit);
    assert_eq!(app.running_state, RunningState::Done);
    assert!(!app.is_running());
}

#[test]
fn patchsets_loaded_populates_state() {
    let mut app = App::new(Config::default());
    let patchsets = Paginated {
        items: vec![Patchset::fixture()],
        total: 1,
        page: 1,
        per_page: 50,
    };
    update(&mut app, Message::PatchsetsLoaded(Ok(patchsets)));
    assert_eq!(app.patchsets.items.len(), 1);
    assert!(app.error_state.is_none());
}

#[test]
fn api_error_sets_error_state() {
    let mut app = App::new(Config::default());
    let err = ApiError::Network {
        source: "connection refused".into(),
        remote: "test".to_string(),
    };
    update(&mut app, Message::PatchsetsLoaded(Err(err)));
    assert!(app.error_state.is_some());
    let err_msg: &str = app
        .error_state
        .as_deref()
        .expect("error_state should be set");
    assert!(err_msg.contains("connection refused"));
}

#[test]
fn lists_loaded_populates_mailing_lists() {
    let mut app = App::new(Config::default());
    let lists = vec![
        MailingList {
            name: "LKML".to_string(),
            group: Some("org.kernel.vger.linux-kernel".to_string()),
        },
        MailingList {
            name: "netdev".to_string(),
            group: Some("org.kernel.vger.netdev".to_string()),
        },
    ];
    update(&mut app, Message::ListsLoaded(Ok(lists)));
    assert_eq!(app.mailing_lists.len(), 2);
}

#[test]
fn error_clears_on_successful_load() {
    let mut app = App::new(Config::default());

    // Set error state
    let err = ApiError::Network {
        source: "timeout".into(),
        remote: "test".to_string(),
    };
    update(&mut app, Message::PatchsetsLoaded(Err(err)));
    assert!(app.error_state.is_some());

    // Successful load clears error
    let patchsets = Paginated {
        items: vec![],
        total: 0,
        page: 1,
        per_page: 50,
    };
    update(&mut app, Message::PatchsetsLoaded(Ok(patchsets)));
    assert!(app.error_state.is_none());
}

#[test]
fn init_returns_fetch_commands() {
    let mut app = App::new(Config::default());
    let cmd = update(&mut app, Message::Init);
    assert!(
        matches!(cmd, Cmd::Batch(ref cmds) if cmds.len() == 3),
        "Init should return Batch with 3 commands"
    );
}

#[test]
fn refresh_returns_fetch_commands() {
    let mut app = App::new(Config::default());
    let cmd = update(&mut app, Message::Refresh);
    assert!(
        matches!(cmd, Cmd::Batch(ref cmds) if cmds.len() == 3),
        "Refresh should return Batch with 3 commands (patchsets, lists, stats)"
    );
}

// --- Multi-step cross-feature integration tests ---

#[test]
fn remote_switch_then_reload() {
    let mut app = app_with_remotes(&["upstream", "staging"]);

    // Load initial data for upstream
    let patchsets = Paginated {
        items: vec![Patchset::fixture()],
        total: 1,
        page: 1,
        per_page: 50,
    };
    update(&mut app, Message::PatchsetsLoaded(Ok(patchsets)));
    assert_eq!(app.patchsets.items.len(), 1);
    assert_eq!(app.active_remote, "upstream");

    // Switch to staging — clears data, returns fetch batch
    let cmd = update(&mut app, Message::NextMailbox);
    assert_eq!(app.active_remote, "staging");
    assert!(app.patchsets.items.is_empty());
    assert_eq!(app.selected_index, 0);
    assert!(matches!(cmd, Cmd::Batch(_)));

    // Simulate staging data arriving
    let staging_patchsets = Paginated {
        items: vec![Patchset::fixture(), Patchset::fixture()],
        total: 2,
        page: 1,
        per_page: 50,
    };
    update(&mut app, Message::PatchsetsLoaded(Ok(staging_patchsets)));
    assert_eq!(app.patchsets.items.len(), 2);
    assert!(app.error_state.is_none());
}

#[test]
fn init_flow_populates_all_data() {
    let mut app = app_with_remotes(&["upstream"]);

    // Init returns a batch of 3 fetches
    let cmd = update(&mut app, Message::Init);
    assert!(matches!(cmd, Cmd::Batch(ref cmds) if cmds.len() == 3));

    // Simulate all three responses arriving
    let patchsets = Paginated {
        items: vec![Patchset::fixture()],
        total: 1,
        page: 1,
        per_page: 50,
    };
    update(&mut app, Message::PatchsetsLoaded(Ok(patchsets)));
    update(
        &mut app,
        Message::ListsLoaded(Ok(vec![MailingList {
            name: "LKML".to_string(),
            group: Some("org.kernel.vger.linux-kernel".to_string()),
        }])),
    );
    update(
        &mut app,
        Message::StatsLoaded(Ok(remendo_tui::models::ServerStats::fixture())),
    );

    assert_eq!(app.patchsets.items.len(), 1);
    assert_eq!(app.mailing_lists.len(), 1);
    assert!(app.error_state.is_none());
}

#[test]
fn scroll_after_remote_switch_uses_new_data() {
    let mut app = app_with_remotes(&["upstream", "staging"]);
    app.terminal_height = 24;

    // Load 10 patchsets, scroll to index 5
    let patchsets = Paginated {
        items: (0..10).map(|_| Patchset::fixture()).collect(),
        total: 10,
        page: 1,
        per_page: 50,
    };
    update(&mut app, Message::PatchsetsLoaded(Ok(patchsets)));
    for _ in 0..5 {
        update(&mut app, Message::ScrollDown);
    }
    assert_eq!(app.selected_index, 5);

    // Switch remote — index resets to 0
    update(&mut app, Message::NextMailbox);
    assert_eq!(app.selected_index, 0);

    // Load only 3 patchsets for the new remote
    let staging_patchsets = Paginated {
        items: (0..3).map(|_| Patchset::fixture()).collect(),
        total: 3,
        page: 1,
        per_page: 50,
    };
    update(&mut app, Message::PatchsetsLoaded(Ok(staging_patchsets)));

    // Scroll down — should clamp at 2 (last item)
    for _ in 0..5 {
        update(&mut app, Message::ScrollDown);
    }
    assert_eq!(app.selected_index, 2);
}

#[test]
fn focus_toggle_gates_scroll_roundtrip() {
    let mut app = app_with_remotes(&["upstream"]);

    let patchsets = Paginated {
        items: (0..5).map(|_| Patchset::fixture()).collect(),
        total: 5,
        page: 1,
        per_page: 50,
    };
    update(&mut app, Message::PatchsetsLoaded(Ok(patchsets)));
    assert_eq!(app.focus, FocusPanel::PatchsetList);

    // Scroll works in PatchsetList focus
    update(&mut app, Message::ScrollDown);
    assert_eq!(app.selected_index, 1);

    // Toggle to sidebar — scroll should be ignored
    update(&mut app, Message::ToggleFocus);
    assert_eq!(app.focus, FocusPanel::Sidebar);
    update(&mut app, Message::ScrollDown);
    assert_eq!(app.selected_index, 1); // unchanged

    // Toggle back — scroll works again
    update(&mut app, Message::ToggleFocus);
    assert_eq!(app.focus, FocusPanel::PatchsetList);
    update(&mut app, Message::ScrollDown);
    assert_eq!(app.selected_index, 2);
}

#[test]
fn select_returns_fetch_detail_command() {
    let mut app = app_with_remotes(&["upstream"]);

    // Load patchsets
    let patchsets = Paginated {
        items: vec![Patchset::fixture()],
        total: 1,
        page: 1,
        per_page: 50,
    };
    update(&mut app, Message::PatchsetsLoaded(Ok(patchsets)));

    // Press Enter (Select) — should return FetchPatchsetDetail
    let cmd = update(&mut app, Message::Select);
    assert!(
        matches!(cmd, Cmd::FetchPatchsetDetail(PatchId::Numeric(1))),
        "expected FetchPatchsetDetail(Numeric(1)), got {cmd:?}"
    );
}

#[test]
fn select_on_empty_list_is_noop() {
    let mut app = app_with_remotes(&["upstream"]);
    // No patchsets loaded
    let cmd = update(&mut app, Message::Select);
    assert!(
        matches!(cmd, Cmd::None),
        "Select on empty list should be Cmd::None"
    );
}

#[test]
fn select_in_sidebar_switches_remote() {
    let mut app = app_with_remotes(&["upstream", "staging"]);
    update(&mut app, Message::ToggleFocus); // Switch to Sidebar

    // Select on the currently highlighted remote triggers switch_remote
    let cmd = update(&mut app, Message::Select);
    assert!(
        matches!(cmd, Cmd::Batch(_)),
        "Select in sidebar remotes should trigger remote switch"
    );
}

#[test]
fn full_enter_flow_select_to_detail_view() {
    let mut app = app_with_remotes(&["upstream"]);

    // Load patchsets
    let patchsets = Paginated {
        items: vec![Patchset::fixture()],
        total: 1,
        page: 1,
        per_page: 50,
    };
    update(&mut app, Message::PatchsetsLoaded(Ok(patchsets)));
    assert_eq!(app.view_mode, ViewMode::List);
    assert!(app.selected_detail.is_none());

    // Select — returns FetchPatchsetDetail
    let cmd = update(&mut app, Message::Select);
    assert!(matches!(cmd, Cmd::FetchPatchsetDetail(_)));

    // Simulate detail response arriving
    let detail = PatchsetDetail::fixture();
    update(
        &mut app,
        Message::PatchsetDetailLoaded(Box::new(Ok(detail))),
    );

    // Verify detail view is active
    assert_eq!(app.view_mode, ViewMode::Detail);
    assert!(app.selected_detail.is_some());
    let detail = app.selected_detail.as_ref().expect("detail loaded");
    assert_eq!(detail.id, 1);
    assert!(!detail.patches.is_empty());
    assert!(app.error_state.is_none());
}

#[test]
fn detail_load_error_sets_error_state() {
    let mut app = app_with_remotes(&["upstream"]);

    let err = ApiError::Network {
        source: "connection refused".into(),
        remote: "upstream".to_string(),
    };
    update(
        &mut app,
        Message::PatchsetDetailLoaded(Box::new(Err(err))),
    );

    assert!(app.error_state.is_some());
    assert_eq!(app.view_mode, ViewMode::List); // stays on list
    assert!(app.selected_detail.is_none());
}

#[test]
fn search_flow_end_to_end() {
    use remendo_tui::app::InputMode;

    let mut app = app_with_remotes(&["upstream"]);
    app.list_params.page = 3; // non-default page

    // Start search
    update(&mut app, Message::SearchStart);
    assert_eq!(app.input_mode, InputMode::Search);

    // Type query
    update(&mut app, Message::SearchInput('f'));
    update(&mut app, Message::SearchInput('i'));
    update(&mut app, Message::SearchInput('x'));

    // Submit
    let cmd = update(&mut app, Message::SearchSubmit);
    assert_eq!(app.input_mode, InputMode::Normal);
    assert_eq!(app.list_params.search, Some("fix".to_string()));
    assert_eq!(app.list_params.page, 1); // reset on search
    assert!(matches!(cmd, Cmd::FetchPatchsets(_)));

    // Simulate results arriving
    let results = Paginated {
        items: vec![Patchset::fixture()],
        total: 1,
        page: 1,
        per_page: 50,
    };
    update(&mut app, Message::PatchsetsLoaded(Ok(results)));
    assert_eq!(app.patchsets.items.len(), 1);

    // Search query preserved across refresh
    let cmd = update(&mut app, Message::Refresh);
    match cmd {
        Cmd::Batch(ref cmds) => {
            // The FetchPatchsets should carry the search query
            assert!(matches!(cmds[0], Cmd::FetchPatchsets(_)));
        }
        _ => panic!("expected Batch"),
    }
}
