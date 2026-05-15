//! Widget rendering — the TEA `view` function.
//!
//! Renders the application state into a terminal frame.
//! This module contains no state mutation — it is a pure
//! function of `App` → visual output.

use crate::app::{App, FocusPanel};
use crate::config::theme::ColorPalette;
use crate::models::{FindingCounts, PatchsetStatus};
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, Borders, Cell, List, ListItem, ListState, Paragraph, Row, Table, TableState,
};
use ratatui::Frame;

/// Render the application state into the given frame.
///
/// This is the TEA `view` function — it reads `App` state
/// and produces visual output. No mutation occurs.
pub fn view(app: &App, frame: &mut Frame) {
    let area = frame.area();
    let palette = &app.config.theme.colors;

    // No remotes configured — show setup instructions (full screen)
    if app.config.remotes.is_empty() {
        let block = Block::default()
            .title(" remendo ".bold())
            .borders(Borders::ALL)
            .border_style(Style::default().fg(palette.border.color()));
        let content = "No remotes configured.\n\n\
             To get started:\n  \
             1. cp config.example.toml ~/.config/remendo/config.toml\n  \
             2. Edit the file and set your Sashiko instance URL\n  \
             3. Restart remendo";
        let paragraph = Paragraph::new(content).block(block);
        frame.render_widget(paragraph, area);
        return;
    }

    // Split layout: sidebar + main pane
    let chunks = Layout::horizontal([Constraint::Length(24), Constraint::Min(40)]).split(area);

    // --- Sidebar ---
    render_sidebar(app, frame, chunks[0], palette);

    // --- Main pane ---
    render_main_pane(app, frame, chunks[1], palette);
}

/// Render the remote/mailbox sidebar in the given area.
fn render_sidebar(app: &App, frame: &mut Frame, area: ratatui::layout::Rect, palette: &ColorPalette) {
    let border_color = if app.focus == FocusPanel::Sidebar {
        palette.accent.color()
    } else {
        palette.border.color()
    };

    let title = format!(" Remotes ({}) ", app.config.remotes.len());
    let block = Block::default()
        .title(title.bold())
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));

    let items: Vec<ListItem> = app
        .config
        .remotes
        .iter()
        .map(|r| ListItem::new(r.name.as_str()))
        .collect();

    let list = List::new(items)
        .block(block)
        .highlight_style(
            Style::default()
                .bg(palette.selected_bg.color())
                .fg(palette.selected_fg.color()),
        );

    let mut list_state = ListState::default();
    list_state.select(Some(app.active_remote_index));

    frame.render_stateful_widget(list, area, &mut list_state);
}

/// Render the main patchset pane (table or placeholder) in the given area.
fn render_main_pane(app: &App, frame: &mut Frame, area: ratatui::layout::Rect, palette: &ColorPalette) {
    let border_color = if app.focus == FocusPanel::PatchsetList {
        palette.accent.color()
    } else {
        palette.border.color()
    };

    let status = if let Some(ref err) = app.error_state {
        format!(" remendo | ERROR: {err} ")
    } else {
        let remote = if app.active_remote.is_empty() {
            "(no remote)"
        } else {
            &app.active_remote
        };
        format!(" remendo | {} | {} patchsets ", remote, app.patchsets.total)
    };

    let block = Block::default()
        .title(status.bold())
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));

    // Empty list — show loading or error placeholder
    if app.patchsets.items.is_empty() {
        let text = if app.error_state.is_some() {
            "Error loading patchsets. Press Ctrl-r to retry."
        } else {
            "Loading..."
        };
        let paragraph = Paragraph::new(text).block(block);
        frame.render_widget(paragraph, area);
        return;
    }

    // Build table rows from patchsets
    let rows: Vec<Row> = app
        .patchsets
        .items
        .iter()
        .map(|ps| {
            let status_style = status_color(ps.status, palette);
            let status_cell = Cell::from(ps.status.to_string()).style(status_style);

            let subject_cell = Cell::from(ps.subject());
            let author_cell =
                Cell::from(ps.author()).style(Style::default().fg(palette.muted.color()));

            let date_cell = Cell::from(format_date(ps.date))
                .style(Style::default().fg(palette.muted.color()));

            let findings_line = format_findings(&ps.findings, palette);
            let findings_cell = Cell::from(findings_line);

            let subsystems_text = if ps.subsystems.is_empty() {
                String::new()
            } else {
                ps.subsystems.join(", ")
            };
            let subsystems_cell =
                Cell::from(subsystems_text).style(Style::default().fg(palette.muted.color()));

            Row::new(vec![
                status_cell,
                subject_cell,
                author_cell,
                date_cell,
                findings_cell,
                subsystems_cell,
            ])
        })
        .collect();

    let header = Row::new(vec![
        Cell::from("Status"),
        Cell::from("Subject"),
        Cell::from("Author"),
        Cell::from("Date"),
        Cell::from("Findings"),
        Cell::from("Subsystems"),
    ])
    .style(Style::default().fg(palette.accent.color()).bold());

    let widths = [
        Constraint::Length(14),
        Constraint::Min(30),
        Constraint::Length(20),
        Constraint::Length(10),
        Constraint::Length(10),
        Constraint::Length(18),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(block)
        .row_highlight_style(
            Style::default()
                .bg(palette.selected_bg.color())
                .fg(palette.selected_fg.color()),
        );

    let mut table_state = TableState::default();
    table_state.select(Some(app.selected_index));

    frame.render_stateful_widget(table, area, &mut table_state);
}

/// Map a `PatchsetStatus` to a foreground color style.
fn status_color(status: PatchsetStatus, palette: &ColorPalette) -> Style {
    match status {
        PatchsetStatus::Pending => Style::default().fg(palette.foreground.color()),
        PatchsetStatus::InReview => Style::default().fg(palette.accent.color()),
        PatchsetStatus::Reviewed => Style::default().fg(palette.success.color()),
        PatchsetStatus::Failed | PatchsetStatus::FailedToApply => {
            Style::default().fg(palette.error.color())
        }
        PatchsetStatus::Cancelled => Style::default().fg(palette.warning.color()),
        PatchsetStatus::Incomplete | PatchsetStatus::Skipped | PatchsetStatus::Unknown => {
            Style::default().fg(palette.muted.color())
        }
    }
}

/// Format a unix timestamp as `YYYY-MM-DD` without external crate dependencies.
fn format_date(ts: Option<i64>) -> String {
    let Some(ts) = ts else {
        return String::from("—");
    };

    // Simple conversion from unix timestamp to YYYY-MM-DD.
    // Uses the algorithm for days since epoch -> civil date.
    let secs = ts;
    let days = secs / 86400;
    // Civil calendar conversion (Howard Hinnant's algorithm)
    let z = days + 719_468;
    let era = (if z >= 0 { z } else { z - 146_096 }) / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };

    format!("{y:04}-{m:02}-{d:02}")
}

/// Format `FindingCounts` as compact severity indicators.
///
/// Non-zero severities are shown in descending order (e.g., `1C 2H`).
/// High and critical counts use the error color.
fn format_findings<'a>(findings: &FindingCounts, palette: &'a ColorPalette) -> Line<'a> {
    if findings.is_empty() {
        return Line::from("—").style(Style::default().fg(palette.muted.color()));
    }

    let mut spans: Vec<Span<'a>> = Vec::new();
    let error_style = Style::default().fg(palette.error.color());
    let warn_style = Style::default().fg(palette.warning.color());
    let default_style = Style::default().fg(palette.muted.color());

    if findings.critical > 0 {
        if !spans.is_empty() {
            spans.push(Span::raw(" "));
        }
        spans.push(Span::styled(format!("{}C", findings.critical), error_style));
    }
    if findings.high > 0 {
        if !spans.is_empty() {
            spans.push(Span::raw(" "));
        }
        spans.push(Span::styled(format!("{}H", findings.high), error_style));
    }
    if findings.medium > 0 {
        if !spans.is_empty() {
            spans.push(Span::raw(" "));
        }
        spans.push(Span::styled(format!("{}M", findings.medium), warn_style));
    }
    if findings.low > 0 {
        if !spans.is_empty() {
            spans.push(Span::raw(" "));
        }
        spans.push(Span::styled(
            format!("{}L", findings.low),
            default_style,
        ));
    }

    Line::from(spans)
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::models::{FindingCounts, Paginated, Patchset};
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    fn make_config_with_remotes(names: &[&str]) -> Config {
        let mut config = Config::default();
        for name in names {
            config.remotes.push(crate::config::RemoteConfig {
                name: (*name).to_string(),
                url: format!("https://{name}.example.com"),
                auth_env: None,
                timeout_seconds: 15,
                max_retries: 3,
            });
        }
        config
    }

    #[test]
    fn view_renders_without_panic() {
        let app = App::new(Config::default());
        let mut terminal = Terminal::new(TestBackend::new(80, 24)).expect("create test terminal");
        terminal
            .draw(|f| view(&app, f))
            .expect("draw should not fail");
        insta::assert_snapshot!(terminal.backend());
    }

    #[test]
    fn view_renders_error_state() {
        let mut app = App::new(Config::default());
        app.error_state = Some("connection refused".to_string());
        let mut terminal = Terminal::new(TestBackend::new(80, 24)).expect("create test terminal");
        terminal
            .draw(|f| view(&app, f))
            .expect("draw with error should not fail");
        insta::assert_snapshot!(terminal.backend());
    }

    #[test]
    fn view_renders_patchset_table() {
        let config = make_config_with_remotes(&["upstream"]);
        let mut app = App::new(config);
        app.patchsets = Paginated {
            items: vec![Patchset::fixture()],
            total: 1,
            page: 1,
            per_page: 50,
        };
        let mut terminal = Terminal::new(TestBackend::new(120, 24)).expect("create test terminal");
        terminal
            .draw(|f| view(&app, f))
            .expect("draw with patchsets should not fail");
        insta::assert_snapshot!(terminal.backend());
    }

    #[test]
    fn view_renders_loading_state() {
        let config = make_config_with_remotes(&["upstream"]);
        let app = App::new(config);
        let mut terminal = Terminal::new(TestBackend::new(80, 24)).expect("create test terminal");
        terminal
            .draw(|f| view(&app, f))
            .expect("draw loading should not fail");
        insta::assert_snapshot!(terminal.backend());
    }

    #[test]
    fn view_renders_sidebar_with_multiple_remotes() {
        let config = make_config_with_remotes(&["upstream", "staging", "local"]);
        let app = App::new(config);
        let mut terminal = Terminal::new(TestBackend::new(120, 24)).expect("create test terminal");
        terminal
            .draw(|f| view(&app, f))
            .expect("draw with sidebar should not fail");
        insta::assert_snapshot!(terminal.backend());
    }

    #[test]
    fn view_renders_sidebar_focus() {
        let config = make_config_with_remotes(&["upstream", "staging"]);
        let mut app = App::new(config);
        app.focus = FocusPanel::Sidebar;
        let mut terminal = Terminal::new(TestBackend::new(120, 24)).expect("create test terminal");
        terminal
            .draw(|f| view(&app, f))
            .expect("draw with sidebar focus should not fail");
        insta::assert_snapshot!(terminal.backend());
    }

    #[test]
    fn format_date_valid() {
        // 2026-05-13 (approx)
        let result = format_date(Some(1_778_690_980));
        assert_eq!(result, "2026-05-13");
    }

    #[test]
    fn format_date_none() {
        assert_eq!(format_date(None), "—");
    }

    #[test]
    fn format_date_epoch() {
        assert_eq!(format_date(Some(0)), "1970-01-01");
    }

    #[test]
    fn format_findings_empty() {
        let palette = ColorPalette::default();
        let fc = FindingCounts::default();
        let line = format_findings(&fc, &palette);
        // Should show the dash placeholder
        assert_eq!(line.spans.len(), 1);
    }

    #[test]
    fn format_findings_mixed() {
        let palette = ColorPalette::default();
        let fc = FindingCounts {
            low: 1,
            medium: 2,
            high: 1,
            critical: 0,
        };
        let line = format_findings(&fc, &palette);
        // Should have spans for H, M, L with separators
        let text: String = line.spans.iter().map(|s| s.content.to_string()).collect();
        assert!(text.contains("1H"));
        assert!(text.contains("2M"));
        assert!(text.contains("1L"));
    }

    #[test]
    fn format_findings_critical_only() {
        let palette = ColorPalette::default();
        let fc = FindingCounts {
            low: 0,
            medium: 0,
            high: 0,
            critical: 3,
        };
        let line = format_findings(&fc, &palette);
        let text: String = line.spans.iter().map(|s| s.content.to_string()).collect();
        assert_eq!(text, "3C");
    }

    #[test]
    fn status_color_maps_correctly() {
        let palette = ColorPalette::default();
        let reviewed = status_color(PatchsetStatus::Reviewed, &palette);
        assert_eq!(reviewed.fg, Some(palette.success.color()));

        let failed = status_color(PatchsetStatus::Failed, &palette);
        assert_eq!(failed.fg, Some(palette.error.color()));

        let in_review = status_color(PatchsetStatus::InReview, &palette);
        assert_eq!(in_review.fg, Some(palette.accent.color()));
    }
}
