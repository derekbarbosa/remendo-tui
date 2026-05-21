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
}
