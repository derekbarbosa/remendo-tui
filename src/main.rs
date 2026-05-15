//! remendo-tui — a terminal-based interface for interacting with
//! agentic patch review mechanisms (Sashiko instances).
#![warn(clippy::pedantic, clippy::style, clippy::perf)]
#![deny(clippy::unwrap_used)]

/// Application state and lifecycle.
pub mod app;

/// Application configuration: remotes, keybindings, theme, paths.
pub mod config;

/// Terminal events and event-to-message translation.
pub mod event;

/// Domain data models for Sashiko API entities.
pub mod models;

/// Widget renderer.
pub mod ui;

/// Terminal user interface lifecycle.
pub mod tui;

/// Application state updater (TEA update function).
pub mod update;

use app::RunningState;
use color_eyre::Result;
use config::Config;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    color_eyre::install()?;

    let (config, warnings) = Config::load().unwrap_or_else(|e| {
        eprintln!("config error: {e}, using defaults");
        (Config::default(), vec![])
    });

    for warning in &warnings {
        eprintln!("config warning: {warning}");
    }

    let mut tui = tui::Tui::new(4.0, 30.0)?;
    tui.enter()?;

    let mut app = app::App::new(config);

    loop {
        tui.draw(|f| ui::view(&app, f))?;

        let evt = tui.next().await?;
        if let Some(msg) = event::handle_event(&app, &evt) {
            update::update(&mut app, msg);
        }

        if app.running_state == RunningState::Done {
            break;
        }
    }

    tui.exit()?;
    Ok(())
}
