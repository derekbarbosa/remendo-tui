//! remendo-tui binary entry point.

use color_eyre::Result;
use remendo_tui::app::RunningState;
use remendo_tui::config::Config;
use remendo_tui::{event, tui, ui, update};

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

    let mut app = remendo_tui::app::App::new(config);

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
