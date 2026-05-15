//! remendo-tui binary entry point.

use color_eyre::Result;
use remendo_tui::app::RunningState;
use remendo_tui::client::{HttpClient, SashikoApi};
use remendo_tui::config::Config;
use remendo_tui::{cmd, event, tui, ui, update};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    color_eyre::install()?;

    // Load config BEFORE terminal init so warnings print to stderr visibly.
    let (config, warnings) = Config::load().unwrap_or_else(|e| {
        eprintln!("config error: {e}, using defaults");
        (Config::default(), vec![])
    });

    for warning in &warnings {
        eprintln!("config warning: {warning}");
    }

    // Construct HTTP clients for all configured remotes.
    let mut clients: HashMap<String, Arc<dyn SashikoApi>> = HashMap::new();
    for remote in &config.remotes {
        match HttpClient::new(remote) {
            Ok(c) => {
                eprintln!("remendo: connected to remote '{}'", remote.name);
                clients.insert(remote.name.clone(), Arc::new(c));
            }
            Err(e) => {
                eprintln!(
                    "remendo: failed to create client for '{}': {e}",
                    remote.name
                );
            }
        }
    }

    if clients.is_empty() {
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
            cmd::execute(command, &clients, &app.active_remote, &msg_tx);
        }

        if app.running_state == RunningState::Done {
            break;
        }
    }

    tui.exit()?;
    Ok(())
}
