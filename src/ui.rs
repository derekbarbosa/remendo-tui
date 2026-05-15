//! Widget rendering — the TEA `view` function.
//!
//! Renders the application state into a terminal frame.
//! This module contains no state mutation — it is a pure
//! function of `App` → visual output.

use crate::app::App;
use ratatui::Frame;
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::style::Stylize;

/// Render the application state into the given frame.
///
/// This is the TEA `view` function — it reads `App` state
/// and produces visual output. No mutation occurs.
pub fn view(app: &App, frame: &mut Frame) {
    let area = frame.area();

    let status = if let Some(ref err) = app.error_state {
        format!(" remendo | ERROR: {err} ")
    } else {
        let remote = if app.active_remote.is_empty() {
            "(no remote)"
        } else {
            &app.active_remote
        };
        format!(
            " remendo | {} | {} patchsets ",
            remote, app.patchsets.total
        )
    };

    let block = Block::default()
        .title(status.bold())
        .borders(Borders::ALL);

    let content = if app.patchsets.items.is_empty() {
        "No patchsets loaded. Configure a remote in config.toml.".to_string()
    } else {
        format!("{} patchsets loaded", app.patchsets.items.len())
    };

    let paragraph = Paragraph::new(content).block(block);
    frame.render_widget(paragraph, area);
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::config::Config;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    #[test]
    fn view_renders_without_panic() {
        let app = App::new(Config::default());
        let mut terminal =
            Terminal::new(TestBackend::new(80, 24)).expect("create test terminal");
        terminal
            .draw(|f| view(&app, f))
            .expect("draw should not fail");
    }

    #[test]
    fn view_renders_error_state() {
        let mut app = App::new(Config::default());
        app.error_state = Some("connection refused".to_string());
        let mut terminal =
            Terminal::new(TestBackend::new(80, 24)).expect("create test terminal");
        terminal
            .draw(|f| view(&app, f))
            .expect("draw with error should not fail");
    }
}
