//! TEA command/effect system.
//!
//! `Cmd` values describe async side-effects that `update()` requests.
//! The `execute()` function spawns tokio tasks to perform them,
//! sending results back as `Message` variants through a channel.

use crate::bookmarks::BookmarkStore;
use crate::client::SashikoApi;
use crate::client::error::ApiError;
use crate::client::types::ListParams;
use crate::models::PatchId;
use crate::update::Message;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc;

/// A command describing an async side-effect.
///
/// Returned by `update()` to request work from the runtime.
/// The runtime calls `execute()` to perform it.
#[derive(Debug)]
pub enum Cmd {
    /// No effect — the update had nothing to request.
    None,
    /// Fetch the patchset list for the active remote.
    FetchPatchsets(ListParams),
    /// Fetch mailing lists for the active remote.
    FetchLists,
    /// Fetch server stats for the active remote.
    FetchStats,
    /// Fetch full detail for a specific patchset.
    FetchPatchsetDetail(PatchId),
    /// Open a file in the configured external editor.
    OpenEditor {
        /// Content to write to a temp file and open.
        content: String,
        /// The editor command to use.
        editor: String,
    },
    /// Persist bookmarks to disk.
    PersistBookmarks {
        /// The bookmark store to save.
        bookmarks: BookmarkStore,
        /// Path to the bookmarks file.
        path: PathBuf,
    },
    /// Fetch the message list for the active remote.
    FetchMessages(ListParams),
    /// Fetch full detail for a specific message.
    FetchMessageDetail(PatchId),
    /// Clear the API response cache, then execute a batch.
    ClearCacheAndBatch(Vec<Cmd>),
    /// Execute multiple commands concurrently.
    Batch(Vec<Cmd>),
}

/// Spawn an async fetch task if the client exists, or send an error message.
///
/// This helper eliminates the repeated `if let Some(client) = clients.get(...)
/// { clone + spawn } else { send error }` pattern across all fetch commands.
fn spawn_fetch<S, F, Fut, T>(
    clients: &HashMap<String, Arc<dyn SashikoApi>, S>,
    active_remote: &str,
    msg_tx: &mpsc::UnboundedSender<Message>,
    fetch: F,
    on_missing: fn(&str) -> Message,
) where
    S: ::std::hash::BuildHasher,
    F: FnOnce(Arc<dyn SashikoApi>) -> Fut + Send + 'static,
    Fut: std::future::Future<Output = T> + Send,
    T: Into<Message> + Send + 'static,
{
    if let Some(client) = clients.get(active_remote) {
        let client = Arc::clone(client);
        let tx = msg_tx.clone();
        tokio::spawn(async move {
            let result = fetch(client).await;
            let _ = tx.send(result.into());
        });
    } else {
        let _ = msg_tx.send(on_missing(active_remote));
    }
}

/// Execute a command by spawning async tasks that send results
/// back through `msg_tx`.
///
/// The `active_remote` key is used to look up the correct client
/// from `clients`. If the remote is not found, an error message
/// is sent instead.
pub fn execute<S: ::std::hash::BuildHasher>(
    cmd: Cmd,
    clients: &HashMap<String, Arc<dyn SashikoApi>, S>,
    active_remote: &str,
    msg_tx: &mpsc::UnboundedSender<Message>,
) {
    match cmd {
        Cmd::None => {}
        Cmd::FetchPatchsets(params) => {
            tracing::debug!(
                remote = active_remote,
                page = params.page,
                search = ?params.search,
                mailing_list = ?params.mailing_list,
                "cmd: fetch patchsets"
            );
            spawn_fetch(
                clients,
                active_remote,
                msg_tx,
                |c| async move { Message::PatchsetsLoaded(c.patchsets(&params).await) },
                |r| Message::PatchsetsLoaded(Err(no_remote_error(r))),
            );
        }
        Cmd::FetchLists => {
            tracing::debug!(remote = active_remote, "cmd: fetch lists");
            spawn_fetch(
                clients,
                active_remote,
                msg_tx,
                |c| async move { Message::ListsLoaded(c.lists().await) },
                |r| Message::ListsLoaded(Err(no_remote_error(r))),
            );
        }
        Cmd::FetchStats => {
            tracing::debug!(remote = active_remote, "cmd: fetch stats");
            spawn_fetch(
                clients,
                active_remote,
                msg_tx,
                |c| async move { Message::StatsLoaded(c.stats().await) },
                |r| Message::StatsLoaded(Err(no_remote_error(r))),
            );
        }
        Cmd::FetchPatchsetDetail(id) => {
            tracing::debug!(remote = active_remote, id = %id, "cmd: fetch patchset detail");
            spawn_fetch(
                clients,
                active_remote,
                msg_tx,
                |c| async move {
                    Message::PatchsetDetailLoaded(Box::new(c.patchset_summary(&id).await))
                },
                |r| Message::PatchsetDetailLoaded(Box::new(Err(no_remote_error(r)))),
            );
        }
        Cmd::FetchMessages(params) => {
            tracing::debug!(
                remote = active_remote,
                page = params.page,
                "cmd: fetch messages"
            );
            spawn_fetch(
                clients,
                active_remote,
                msg_tx,
                |c| async move { Message::MessagesLoaded(c.messages(&params).await) },
                |r| Message::MessagesLoaded(Err(no_remote_error(r))),
            );
        }
        Cmd::FetchMessageDetail(id) => {
            tracing::debug!(remote = active_remote, id = %id, "cmd: fetch message detail");
            spawn_fetch(
                clients,
                active_remote,
                msg_tx,
                |c| async move { Message::MessageDetailLoaded(Box::new(c.message_detail(&id).await)) },
                |r| Message::MessageDetailLoaded(Box::new(Err(no_remote_error(r)))),
            );
        }
        Cmd::OpenEditor { .. } => {
            tracing::error!("Cmd::OpenEditor reached execute() — should be handled in main loop");
        }
        Cmd::PersistBookmarks { bookmarks, path } => {
            tracing::debug!("cmd: persist bookmarks");
            let tx = msg_tx.clone();
            tokio::task::spawn_blocking(move || {
                let result = bookmarks.save(&path);
                let _ = tx.send(Message::BookmarksPersisted(result));
            });
        }
        Cmd::ClearCacheAndBatch(cmds) => {
            if let Some(client) = clients.get(active_remote) {
                client.clear_cache();
                tracing::debug!(remote = active_remote, "cmd: cache cleared");
            }
            for c in cmds {
                execute(c, clients, active_remote, msg_tx);
            }
        }
        Cmd::Batch(cmds) => {
            for c in cmds {
                execute(c, clients, active_remote, msg_tx);
            }
        }
    }
}

/// Create an `ApiError` for when no client exists for a remote name.
fn no_remote_error(remote: &str) -> ApiError {
    ApiError::Configuration(format!("no client configured for remote '{remote}'"))
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::client::MockClient;
    use crate::models::{
        EmailMessage, MailingList, Paginated, Patchset, PatchsetDetail, ServerStats,
    };

    #[test]
    fn cmd_none_is_default() {
        // Cmd::None should be constructible
        let cmd = Cmd::None;
        assert!(matches!(cmd, Cmd::None));
    }

    #[test]
    fn cmd_batch_composes() {
        let cmd = Cmd::Batch(vec![
            Cmd::FetchLists,
            Cmd::FetchPatchsets(ListParams::default()),
            Cmd::FetchStats,
        ]);
        assert!(matches!(cmd, Cmd::Batch(cmds) if cmds.len() == 3));
    }

    /// Build a single-remote client map with the given `MockClient`.
    fn mock_clients(mock: Arc<MockClient>) -> HashMap<String, Arc<dyn SashikoApi>> {
        let mut clients: HashMap<String, Arc<dyn SashikoApi>> = HashMap::new();
        clients.insert("test".to_string(), mock as Arc<dyn SashikoApi>);
        clients
    }

    // -- execute() tests --

    #[tokio::test]
    async fn execute_none_sends_nothing() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let clients: HashMap<String, Arc<dyn SashikoApi>> = HashMap::new();
        execute(Cmd::None, &clients, "test", &tx);
        // Give a moment for any spurious sends
        tokio::task::yield_now().await;
        assert!(rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn execute_fetch_patchsets_ok() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let mock = Arc::new(MockClient::new());
        mock.set_patchsets(Ok(Paginated {
            items: vec![Patchset::fixture()],
            total: 1,
            page: 1,
            per_page: 50,
        }));
        let clients = mock_clients(mock);
        execute(
            Cmd::FetchPatchsets(ListParams::default()),
            &clients,
            "test",
            &tx,
        );
        let msg = rx.recv().await.expect("should receive message");
        assert!(matches!(msg, Message::PatchsetsLoaded(Ok(p)) if p.items.len() == 1));
    }

    #[tokio::test]
    async fn execute_fetch_patchsets_missing_remote() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let clients: HashMap<String, Arc<dyn SashikoApi>> = HashMap::new();
        execute(
            Cmd::FetchPatchsets(ListParams::default()),
            &clients,
            "missing",
            &tx,
        );
        let msg = rx.recv().await.expect("should receive error");
        assert!(matches!(msg, Message::PatchsetsLoaded(Err(_))));
    }

    #[tokio::test]
    async fn execute_fetch_lists_ok() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let mock = Arc::new(MockClient::new());
        mock.set_lists(Ok(vec![MailingList::fixture()]));
        let clients = mock_clients(mock);
        execute(Cmd::FetchLists, &clients, "test", &tx);
        let msg = rx.recv().await.expect("should receive message");
        assert!(matches!(msg, Message::ListsLoaded(Ok(lists)) if lists.len() == 1));
    }

    #[tokio::test]
    async fn execute_fetch_lists_missing_remote() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let clients: HashMap<String, Arc<dyn SashikoApi>> = HashMap::new();
        execute(Cmd::FetchLists, &clients, "missing", &tx);
        let msg = rx.recv().await.expect("should receive error");
        assert!(matches!(msg, Message::ListsLoaded(Err(_))));
    }

    #[tokio::test]
    async fn execute_fetch_stats_ok() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let mock = Arc::new(MockClient::new());
        mock.set_stats(Ok(ServerStats::fixture()));
        let clients = mock_clients(mock);
        execute(Cmd::FetchStats, &clients, "test", &tx);
        let msg = rx.recv().await.expect("should receive message");
        assert!(matches!(msg, Message::StatsLoaded(Ok(_))));
    }

    #[tokio::test]
    async fn execute_fetch_stats_missing_remote() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let clients: HashMap<String, Arc<dyn SashikoApi>> = HashMap::new();
        execute(Cmd::FetchStats, &clients, "missing", &tx);
        let msg = rx.recv().await.expect("should receive error");
        assert!(matches!(msg, Message::StatsLoaded(Err(_))));
    }

    #[tokio::test]
    async fn execute_fetch_patchset_detail_ok() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let mock = Arc::new(MockClient::new());
        mock.set_patch_detail(Ok(PatchsetDetail::fixture()));
        let clients = mock_clients(mock);
        execute(
            Cmd::FetchPatchsetDetail(PatchId::Numeric(1)),
            &clients,
            "test",
            &tx,
        );
        let msg = rx.recv().await.expect("should receive message");
        assert!(matches!(msg, Message::PatchsetDetailLoaded(ref r) if r.is_ok()));
    }

    #[tokio::test]
    async fn execute_fetch_patchset_detail_missing_remote() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let clients: HashMap<String, Arc<dyn SashikoApi>> = HashMap::new();
        execute(
            Cmd::FetchPatchsetDetail(PatchId::Numeric(1)),
            &clients,
            "missing",
            &tx,
        );
        let msg = rx.recv().await.expect("should receive error");
        assert!(matches!(msg, Message::PatchsetDetailLoaded(ref r) if r.is_err()));
    }

    #[tokio::test]
    async fn execute_fetch_messages_ok() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let mock = Arc::new(MockClient::new());
        mock.set_messages(Ok(Paginated {
            items: vec![EmailMessage::fixture()],
            total: 1,
            page: 1,
            per_page: 50,
        }));
        let clients = mock_clients(mock);
        execute(
            Cmd::FetchMessages(ListParams::default()),
            &clients,
            "test",
            &tx,
        );
        let msg = rx.recv().await.expect("should receive message");
        assert!(matches!(msg, Message::MessagesLoaded(Ok(p)) if p.items.len() == 1));
    }

    #[tokio::test]
    async fn execute_fetch_messages_missing_remote() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let clients: HashMap<String, Arc<dyn SashikoApi>> = HashMap::new();
        execute(
            Cmd::FetchMessages(ListParams::default()),
            &clients,
            "missing",
            &tx,
        );
        let msg = rx.recv().await.expect("should receive error");
        assert!(matches!(msg, Message::MessagesLoaded(Err(_))));
    }

    #[tokio::test]
    async fn execute_fetch_message_detail_ok() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let mock = Arc::new(MockClient::new());
        mock.set_message_detail(Ok(EmailMessage::fixture()));
        let clients = mock_clients(mock);
        execute(
            Cmd::FetchMessageDetail(PatchId::Numeric(1)),
            &clients,
            "test",
            &tx,
        );
        let msg = rx.recv().await.expect("should receive message");
        assert!(matches!(msg, Message::MessageDetailLoaded(ref r) if r.is_ok()));
    }

    #[tokio::test]
    async fn execute_fetch_message_detail_missing_remote() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let clients: HashMap<String, Arc<dyn SashikoApi>> = HashMap::new();
        execute(
            Cmd::FetchMessageDetail(PatchId::Numeric(1)),
            &clients,
            "missing",
            &tx,
        );
        let msg = rx.recv().await.expect("should receive error");
        assert!(matches!(msg, Message::MessageDetailLoaded(ref r) if r.is_err()));
    }

    #[tokio::test]
    async fn execute_open_editor_is_noop() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let clients: HashMap<String, Arc<dyn SashikoApi>> = HashMap::new();
        execute(
            Cmd::OpenEditor {
                content: "test".to_string(),
                editor: "vim".to_string(),
            },
            &clients,
            "test",
            &tx,
        );
        tokio::task::yield_now().await;
        assert!(rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn execute_persist_bookmarks() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let clients: HashMap<String, Arc<dyn SashikoApi>> = HashMap::new();
        let tmp = tempfile::NamedTempFile::new().expect("tempfile");
        execute(
            Cmd::PersistBookmarks {
                bookmarks: BookmarkStore::new(),
                path: tmp.path().to_path_buf(),
            },
            &clients,
            "test",
            &tx,
        );
        let msg = rx.recv().await.expect("should receive message");
        assert!(matches!(msg, Message::BookmarksPersisted(Ok(()))));
    }

    #[tokio::test]
    async fn execute_batch_runs_all() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let mock = Arc::new(MockClient::new());
        mock.set_stats(Ok(ServerStats::fixture()));
        mock.set_lists(Ok(vec![MailingList::fixture()]));
        let clients = mock_clients(mock);
        execute(
            Cmd::Batch(vec![Cmd::FetchStats, Cmd::FetchLists]),
            &clients,
            "test",
            &tx,
        );
        let msg1 = rx.recv().await.expect("first message");
        let msg2 = rx.recv().await.expect("second message");
        // Both should succeed (order may vary since async)
        let mut got_stats = false;
        let mut got_lists = false;
        for msg in [msg1, msg2] {
            match msg {
                Message::StatsLoaded(Ok(_)) => got_stats = true,
                Message::ListsLoaded(Ok(_)) => got_lists = true,
                _ => {}
            }
        }
        assert!(got_stats, "should have received StatsLoaded");
        assert!(got_lists, "should have received ListsLoaded");
    }

    #[tokio::test]
    async fn execute_clear_cache_and_batch() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let mock = Arc::new(MockClient::new());
        mock.set_stats(Ok(ServerStats::fixture()));
        let clients = mock_clients(mock);
        execute(
            Cmd::ClearCacheAndBatch(vec![Cmd::FetchStats]),
            &clients,
            "test",
            &tx,
        );
        let msg = rx.recv().await.expect("should receive message");
        assert!(matches!(msg, Message::StatsLoaded(Ok(_))));
    }
}
