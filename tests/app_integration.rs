//! Integration tests for the application event→update→state cycle.

#![allow(clippy::expect_used)]

use remendo_tui::app::{App, RunningState};
use remendo_tui::client::ApiError;
use remendo_tui::config::Config;
use remendo_tui::models::{MailingList, Paginated, Patchset};
use remendo_tui::update::{Message, update};

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
