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

/// Execute a command by spawning async tasks that send results
/// back through `msg_tx`.
///
/// The `active_remote` key is used to look up the correct client
/// from `clients`. If the remote is not found, an error message
/// is sent instead.
#[allow(clippy::too_many_lines)]
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
            if let Some(client) = clients.get(active_remote) {
                let client = Arc::clone(client);
                let tx = msg_tx.clone();
                tokio::spawn(async move {
                    let result = client.patchsets(&params).await;
                    let _ = tx.send(Message::PatchsetsLoaded(result));
                });
            } else {
                let _ = msg_tx.send(Message::PatchsetsLoaded(Err(no_remote_error(
                    active_remote,
                ))));
            }
        }
        Cmd::FetchLists => {
            tracing::debug!(remote = active_remote, "cmd: fetch lists");
            if let Some(client) = clients.get(active_remote) {
                let client = Arc::clone(client);
                let tx = msg_tx.clone();
                tokio::spawn(async move {
                    let result = client.lists().await;
                    let _ = tx.send(Message::ListsLoaded(result));
                });
            } else {
                let _ = msg_tx.send(Message::ListsLoaded(Err(no_remote_error(active_remote))));
            }
        }
        Cmd::FetchStats => {
            tracing::debug!(remote = active_remote, "cmd: fetch stats");
            if let Some(client) = clients.get(active_remote) {
                let client = Arc::clone(client);
                let tx = msg_tx.clone();
                tokio::spawn(async move {
                    let result = client.stats().await;
                    let _ = tx.send(Message::StatsLoaded(result));
                });
            } else {
                let _ = msg_tx.send(Message::StatsLoaded(Err(no_remote_error(active_remote))));
            }
        }
        Cmd::FetchPatchsetDetail(id) => {
            tracing::debug!(remote = active_remote, id = %id, "cmd: fetch patchset detail");
            if let Some(client) = clients.get(active_remote) {
                let client = Arc::clone(client);
                let tx = msg_tx.clone();
                tokio::spawn(async move {
                    let result = client.patchset_summary(&id).await;
                    let _ = tx.send(Message::PatchsetDetailLoaded(Box::new(result)));
                });
            } else {
                let _ = msg_tx.send(Message::PatchsetDetailLoaded(Box::new(Err(
                    no_remote_error(active_remote),
                ))));
            }
        }
        Cmd::FetchMessages(params) => {
            tracing::debug!(
                remote = active_remote,
                page = params.page,
                "cmd: fetch messages"
            );
            if let Some(client) = clients.get(active_remote) {
                let client = Arc::clone(client);
                let tx = msg_tx.clone();
                tokio::spawn(async move {
                    let result = client.messages(&params).await;
                    let _ = tx.send(Message::MessagesLoaded(result));
                });
            } else {
                let _ = msg_tx.send(Message::MessagesLoaded(Err(no_remote_error(active_remote))));
            }
        }
        Cmd::FetchMessageDetail(id) => {
            tracing::debug!(remote = active_remote, id = %id, "cmd: fetch message detail");
            if let Some(client) = clients.get(active_remote) {
                let client = Arc::clone(client);
                let tx = msg_tx.clone();
                tokio::spawn(async move {
                    let result = client.message_detail(&id).await;
                    let _ = tx.send(Message::MessageDetailLoaded(Box::new(result)));
                });
            } else {
                let _ = msg_tx.send(Message::MessageDetailLoaded(Box::new(Err(
                    no_remote_error(active_remote),
                ))));
            }
        }
        Cmd::OpenEditor { .. } => {
            // OpenEditor is handled in the main loop (main.rs) where the TUI
            // can be suspended before launching the editor. It should never
            // reach execute().
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
