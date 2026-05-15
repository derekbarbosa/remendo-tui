//! TEA command/effect system.
//!
//! `Cmd` values describe async side-effects that `update()` requests.
//! The `execute()` function spawns tokio tasks to perform them,
//! sending results back as `Message` variants through a channel.

use crate::client::error::ApiError;
use crate::client::types::ListParams;
use crate::client::SashikoApi;
use crate::models::PatchId;
use crate::update::Message;
use std::collections::HashMap;
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
    /// Execute multiple commands concurrently.
    Batch(Vec<Cmd>),
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
