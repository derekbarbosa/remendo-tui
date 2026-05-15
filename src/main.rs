//! remendo-tui binary entry point.

use color_eyre::Result;
use remendo_tui::app::RunningState;
use remendo_tui::client::{CachingClient, HttpClient, SashikoApi};
use remendo_tui::config::Config;
use std::time::Duration;
use remendo_tui::{cmd, event, tui, ui, update};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing_subscriber::EnvFilter;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    color_eyre::install()?;

    // Initialize file-based tracing subscriber.
    // REMENDO_LOG env var controls filtering (fallback: RUST_LOG, default: remendo_tui=info).
    // Log file: $XDG_STATE_HOME/remendo/remendo.log
    let _log_guard = init_tracing();

    // Load config BEFORE terminal init so warnings print to stderr visibly.
    let (config, warnings) = Config::load().unwrap_or_else(|e| {
        eprintln!("config error: {e}, using defaults");
        (Config::default(), vec![])
    });

    tracing::info!(
        version = env!("CARGO_PKG_VERSION"),
        remotes = config.remotes.len(),
        "remendo starting"
    );

    for warning in &warnings {
        tracing::warn!(%warning, "config warning");
        eprintln!("config warning: {warning}");
    }

    // Construct HTTP clients for all configured remotes, wrapped in caching decorator.
    let cache_ttl = Duration::from_secs(config.cache.ttl_seconds);
    let mut clients: HashMap<String, Arc<dyn SashikoApi>> = HashMap::new();
    for remote in &config.remotes {
        match HttpClient::new(remote) {
            Ok(c) => {
                tracing::info!(remote = %remote.name, ttl_secs = config.cache.ttl_seconds, "connected to remote");
                eprintln!("remendo: connected to remote '{}'", remote.name);
                let cached: Arc<dyn SashikoApi> =
                    Arc::new(CachingClient::new(Arc::new(c), cache_ttl));
                clients.insert(remote.name.clone(), cached);
            }
            Err(e) => {
                tracing::error!(remote = %remote.name, error = %e, "failed to create client");
                eprintln!(
                    "remendo: failed to create client for '{}': {e}",
                    remote.name
                );
            }
        }
    }

    if clients.is_empty() {
        tracing::warn!("no remotes configured");
        eprintln!(
            "remendo: no remotes configured. \
             See config.example.toml or docs/CONFIGURATION.md"
        );
    }

    // Message channel for async API results → main loop.
    let (msg_tx, mut msg_rx) = mpsc::unbounded_channel::<update::Message>();

    // Terminal init happens here — after config loading and client
    // construction. Tui::new() has no side effects.
    let mut tui = tui::Tui::new(4.0, 30.0);
    tui.enter()?;

    let mut app = remendo_tui::app::App::new(config);

    // Load bookmarks from state directory
    let state_dir = dirs::state_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("remendo");
    let bookmarks_path = state_dir.join("bookmarks.json");
    app.bookmarks = remendo_tui::bookmarks::BookmarkStore::load(&bookmarks_path);
    app.bookmarks_path = bookmarks_path;
    tracing::info!(count = app.bookmarks.len(), "bookmarks loaded");

    // Main event loop: dual-channel select over terminal events
    // and async API results.
    loop {
        tui.draw(|f| ui::view(&app, f))?;

        let msg = tokio::select! {
            evt = tui.next() => {
                let evt = evt?;
                event::handle_event(&app, &evt)
            }
            Some(msg) = msg_rx.recv() => Some(msg),
        };

        if let Some(msg) = msg {
            let command = update::update(&mut app, msg);
            // Handle OpenEditor in the main loop where we can suspend/resume the TUI
            if let cmd::Cmd::OpenEditor { content, editor } = command {
                if let Err(e) = run_editor(&mut tui, &content, &editor) {
                    tracing::error!(error = %e, "editor failed");
                    app.error_state = Some(format!("editor error: {e}"));
                }
            } else {
                cmd::execute(command, &clients, &app.active_remote, &msg_tx);
            }
        }

        if app.running_state == RunningState::Done {
            break;
        }
    }

    tui.exit()?;
    tracing::info!("remendo exiting");
    Ok(())
}

/// Suspend the TUI, run an editor on the given content, then resume.
fn run_editor(tui: &mut tui::Tui, content: &str, editor: &str) -> Result<()> {
    use std::io::Write;

    // Write content to a temp file
    let mut tmp = tempfile::Builder::new()
        .prefix("remendo-review-")
        .suffix(".txt")
        .tempfile()?;
    tmp.write_all(content.as_bytes())?;
    let path = tmp.path().to_path_buf();

    // Suspend TUI — restore normal terminal for the editor
    tui.suspend()?;

    // Run editor synchronously
    let status = std::process::Command::new(editor).arg(&path).status();

    // Resume TUI — re-enter alternate screen and raw mode
    tui.resume()?;

    match status {
        Ok(s) if s.success() => Ok(()),
        Ok(s) => {
            tracing::warn!(status = %s, "editor exited with non-zero status");
            Ok(())
        }
        Err(e) => Err(color_eyre::eyre::eyre!("failed to launch '{editor}': {e}")),
    }
}

/// Initialize file-based tracing subscriber.
///
/// Returns a `WorkerGuard` that must be held until the end of `main()`
/// to ensure all buffered log events are flushed.
fn init_tracing() -> tracing_appender::non_blocking::WorkerGuard {
    let log_dir = dirs::state_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("remendo");

    // Best-effort directory creation; if it fails, appender will fail
    // gracefully on first write.
    let _ = std::fs::create_dir_all(&log_dir);

    let file_appender = tracing_appender::rolling::never(&log_dir, "remendo.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    let env_filter = EnvFilter::try_from_env("REMENDO_LOG")
        .or_else(|_| EnvFilter::try_from_env("RUST_LOG"))
        .unwrap_or_else(|_| EnvFilter::new("remendo_tui=info"));

    tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_writer(non_blocking)
        .with_ansi(false)
        .with_target(true)
        .with_thread_ids(false)
        .init();

    guard
}
