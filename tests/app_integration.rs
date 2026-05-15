//! Integration tests for the application event→update→state cycle.

#![allow(clippy::expect_used)]

use remendo_tui::app::{App, FocusPanel, RunningState};
use remendo_tui::client::ApiError;
use remendo_tui::cmd::Cmd;
use remendo_tui::config::{Config, RemoteConfig};
use remendo_tui::models::{MailingList, Paginated, Patchset};
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
        matches!(cmd, Cmd::Batch(ref cmds) if cmds.len() == 2),
        "Refresh should return Batch with 2 commands"
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
