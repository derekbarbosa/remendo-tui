//! Widget rendering — the TEA `view` function.
//!
//! Renders the application state into a terminal frame.
//! This module contains no state mutation — it is a pure
//! function of `App` → visual output.

use crate::app::{App, FocusPanel, InputMode, ViewMode};
use crate::config::theme::ColorPalette;
use crate::models::{FindingCounts, PatchsetStatus};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, Borders, Cell, Clear, List, ListItem, ListState, Paragraph, Row, Table, TableState,
    Wrap,
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
    match app.view_mode {
        ViewMode::List => render_main_pane(app, frame, chunks[1], palette),
        ViewMode::Detail => render_detail_view(app, frame, chunks[1], palette),
    }

    // --- Help overlay (rendered on top of everything) ---
    if app.show_help {
        render_help_overlay(app, frame, palette);
    }
}

/// Return the border color for a panel based on whether it has focus.
fn panel_border_color(
    current_focus: FocusPanel,
    panel: FocusPanel,
    palette: &ColorPalette,
) -> ratatui::style::Color {
    if current_focus == panel {
        palette.accent.color()
    } else {
        palette.border.color()
    }
}

/// Render the remote/mailbox sidebar in the given area.
fn render_sidebar(app: &App, frame: &mut Frame, area: ratatui::layout::Rect, palette: &ColorPalette) {
    use crate::app::SidebarSection;

    let remote_count = app.config.remotes.len();
    let remote_height = u16::try_from(remote_count + 2).unwrap_or(5).min(area.height / 2);

    let sidebar_chunks = Layout::vertical([
        Constraint::Length(remote_height),
        Constraint::Min(3),
    ])
    .split(area);

    let highlight_style = Style::default()
        .bg(palette.selected_bg.color())
        .fg(palette.selected_fg.color());

    // --- Remotes section ---
    let remotes_focused = app.focus == FocusPanel::Sidebar
        && app.sidebar_section == SidebarSection::Remotes;
    let remotes_border = if remotes_focused {
        palette.accent.color()
    } else {
        palette.border.color()
    };
    let remotes_block = Block::default()
        .title(format!(" Remotes ({remote_count}) ").bold())
        .borders(Borders::ALL)
        .border_style(Style::default().fg(remotes_border));

    let remote_items: Vec<ListItem> = app
        .config
        .remotes
        .iter()
        .map(|r| ListItem::new(r.name.as_str()))
        .collect();

    let remotes_list = List::new(remote_items)
        .block(remotes_block)
        .highlight_style(highlight_style);

    let mut remotes_state = ListState::default();
    if remotes_focused {
        remotes_state.select(Some(app.active_remote_index));
    }
    frame.render_stateful_widget(remotes_list, sidebar_chunks[0], &mut remotes_state);

    // --- Mailing lists section ---
    let lists_focused = app.focus == FocusPanel::Sidebar
        && app.sidebar_section == SidebarSection::MailingLists;
    let lists_border = if lists_focused {
        palette.accent.color()
    } else {
        palette.border.color()
    };
    let list_count = app.mailing_lists.len();
    let lists_block = Block::default()
        .title(format!(" Lists ({list_count}) ").bold())
        .borders(Borders::ALL)
        .border_style(Style::default().fg(lists_border));

    if app.mailing_lists.is_empty() {
        let placeholder = Paragraph::new("(none)")
            .style(Style::default().fg(palette.muted.color()))
            .block(lists_block);
        frame.render_widget(placeholder, sidebar_chunks[1]);
    } else {
        let mut list_items: Vec<ListItem> = vec![ListItem::new("All")];
        for ml in &app.mailing_lists {
            list_items.push(ListItem::new(ml.name.as_str()));
        }

        let ml_list = List::new(list_items)
            .block(lists_block)
            .highlight_style(highlight_style);

        let mut ml_state = ListState::default();
        if lists_focused {
            ml_state.select(Some(app.sidebar_list_index));
        }
        frame.render_stateful_widget(ml_list, sidebar_chunks[1], &mut ml_state);
    }
}

/// Build the title bar status string for the main pane.
fn build_main_title(app: &App) -> String {
    if let Some(ref err) = app.error_state {
        return format!(" remendo | ERROR: {err} ");
    }
    let remote = if app.active_remote.is_empty() {
        "(no remote)"
    } else {
        &app.active_remote
    };
    let total_pages = app.patchsets.total_pages();
    let page_indicator = if total_pages > 1 {
        format!(" | page {}/{total_pages}", app.patchsets.page)
    } else {
        String::new()
    };
    let search_indicator = app
        .list_params
        .search
        .as_ref()
        .map_or(String::new(), |q| format!(" | q: \"{q}\""));
    let list_indicator = app
        .list_params
        .mailing_list
        .as_ref()
        .map_or(String::new(), |l| format!(" | list: {l}"));
    if let Some(ref stats) = app.stats {
        format!(
            " remendo | {} | v{} | {} pending | {} reviewing | {} patchsets{page_indicator}{search_indicator}{list_indicator} ",
            remote, stats.version, stats.pending, stats.reviewing, app.patchsets.total
        )
    } else {
        format!(" remendo | {} | {} patchsets{page_indicator}{search_indicator}{list_indicator} ", remote, app.patchsets.total)
    }
}

/// Render the main patchset pane (table or placeholder) in the given area.
fn render_main_pane(app: &App, frame: &mut Frame, area: ratatui::layout::Rect, palette: &ColorPalette) {
    let border_color = panel_border_color(app.focus, FocusPanel::PatchsetList, palette);
    let status = build_main_title(app);

    let block = Block::default()
        .title(status.bold())
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));

    // Empty list — show loading, error, or no-results placeholder
    if app.patchsets.items.is_empty() {
        let text = if app.error_state.is_some() {
            "Error loading patchsets. Press Ctrl-r to retry.".to_string()
        } else if let Some(ref q) = app.list_params.search {
            format!("No results for \"{q}\"")
        } else {
            "Loading...".to_string()
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
        .map(|ps| build_patchset_row(app, ps, palette))
        .collect();

    let header = Row::new(vec![
        Cell::from(""),
        Cell::from("Status"),
        Cell::from("Subject"),
        Cell::from("Author"),
        Cell::from("Date"),
        Cell::from("Findings"),
        Cell::from("Subsystems"),
    ])
    .style(Style::default().fg(palette.accent.color()).bold());

    let widths = [
        Constraint::Length(3),
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

    // Search bar at the bottom of the main pane
    if app.input_mode == InputMode::Search {
        let search_area = Rect::new(
            area.x + 1,
            area.y + area.height.saturating_sub(2),
            area.width.saturating_sub(2),
            1,
        );
        let search_text = format!("/{}", app.search_buffer);
        frame.render_widget(
            Paragraph::new(search_text).style(Style::default().fg(palette.accent.color())),
            search_area,
        );
        #[allow(clippy::cast_possible_truncation)]
        frame.set_cursor_position((
            search_area.x + 1 + app.search_cursor as u16,
            search_area.y,
        ));
    }
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

/// Build a single table row for a patchset in the list view.
fn build_patchset_row<'a>(
    app: &'a App,
    ps: &'a crate::models::Patchset,
    palette: &'a ColorPalette,
) -> Row<'a> {
    let bookmark_cell = if app.bookmarks.contains(&app.active_remote, ps.id) {
        Cell::from(" * ").style(Style::default().fg(palette.accent.color()))
    } else {
        Cell::from("   ")
    };

    let status_style = status_color(ps.status, palette);
    let status_cell = Cell::from(ps.status.to_string()).style(status_style);
    let subject_cell = Cell::from(ps.subject());
    let author_cell = Cell::from(ps.author()).style(Style::default().fg(palette.muted.color()));
    let date_cell =
        Cell::from(format_date(ps.date)).style(Style::default().fg(palette.muted.color()));
    let findings_cell = Cell::from(format_findings(&ps.findings, palette));
    let subsystems_text = if ps.subsystems.is_empty() {
        String::new()
    } else {
        ps.subsystems.join(", ")
    };
    let subsystems_cell =
        Cell::from(subsystems_text).style(Style::default().fg(palette.muted.color()));

    Row::new(vec![
        bookmark_cell,
        status_cell,
        subject_cell,
        author_cell,
        date_cell,
        findings_cell,
        subsystems_cell,
    ])
}

/// Compute a centered rectangle within the given area.
fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let w = width.min(area.width);
    let h = height.min(area.height);
    let x = area.x + (area.width.saturating_sub(w)) / 2;
    let y = area.y + (area.height.saturating_sub(h)) / 2;
    Rect::new(x, y, w, h)
}

/// Render the help overlay showing all keybinding mappings.
fn render_help_overlay(app: &App, frame: &mut Frame, palette: &ColorPalette) {
    // Collect and sort bindings by action label
    let mut entries: Vec<_> = app
        .config
        .keybindings
        .bindings
        .iter()
        .map(|(combo, action)| (combo.to_string(), action.label()))
        .collect();
    entries.sort_by(|a, b| a.1.cmp(b.1));

    let popup_width: u16 = 38;
    let popup_height = u16::try_from(entries.len() + 4).unwrap_or(24).min(40);
    let area = centered_rect(popup_width, popup_height, frame.area());

    frame.render_widget(Clear, area);

    let rows: Vec<Row> = entries
        .iter()
        .map(|(key, label)| {
            Row::new(vec![
                Cell::from(key.as_str()).style(Style::default().fg(palette.accent.color())),
                Cell::from(*label),
            ])
        })
        .collect();

    let widths = [Constraint::Length(12), Constraint::Min(18)];
    let block = Block::default()
        .title(" Keybindings ".bold())
        .borders(Borders::ALL)
        .border_style(Style::default().fg(palette.accent.color()));

    let table = Table::new(rows, widths).block(block);
    frame.render_widget(table, area);
}

/// Render the patchset detail view in the given area.
fn render_detail_view(app: &App, frame: &mut Frame, area: Rect, palette: &ColorPalette) {
    let border_color = panel_border_color(app.focus, FocusPanel::PatchsetList, palette);

    let Some(ref detail) = app.selected_detail else {
        let block = Block::default()
            .title(" Loading detail... ".bold())
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color));
        frame.render_widget(Paragraph::new("").block(block), area);
        return;
    };

    let subject = detail.subject.as_deref().unwrap_or("(no subject)");
    let title = format!(" {subject} ");
    let block = Block::default()
        .title(title.bold())
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));

    let mut lines: Vec<Line> = Vec::new();
    detail_header_lines(detail, palette, &mut lines);
    detail_patches_lines(detail, palette, &mut lines);
    detail_thread_lines(detail, palette, &mut lines);

    let scroll_offset = u16::try_from(app.detail_scroll_offset).unwrap_or(u16::MAX);
    let paragraph = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false })
        .scroll((scroll_offset, 0));

    frame.render_widget(paragraph, area);
}

/// Build header lines for the detail view.
fn detail_header_lines<'a>(
    detail: &'a crate::models::PatchsetDetail,
    palette: &'a ColorPalette,
    lines: &mut Vec<Line<'a>>,
) {
    let status_style = status_color(detail.status, palette);
    lines.push(Line::from(vec![
        Span::styled(detail.status.to_string(), status_style),
        Span::raw("  "),
        Span::styled(
            detail.author.as_deref().unwrap_or("(unknown)"),
            Style::default().fg(palette.muted.color()),
        ),
        Span::raw("  "),
        Span::styled(
            format_date(detail.date),
            Style::default().fg(palette.muted.color()),
        ),
    ]));

    if let (Some(total), Some(received)) = (detail.total_parts, detail.received_parts) {
        let sub_text = if detail.subsystems.is_empty() {
            String::new()
        } else {
            format!("   {}", detail.subsystems.join(", "))
        };
        lines.push(Line::from(vec![
            Span::styled(
                format!("Parts: {received}/{total}"),
                Style::default().fg(palette.muted.color()),
            ),
            Span::styled(sub_text, Style::default().fg(palette.muted.color())),
        ]));
    }

    if let Some(ref baseline) = detail.baseline {
        let branch = baseline.branch.as_deref().unwrap_or("?");
        let commit = baseline
            .commit
            .as_deref()
            .map_or("?", |c| if c.len() > 12 { &c[..12] } else { c });
        lines.push(Line::styled(
            format!("Baseline: {branch} @ {commit}"),
            Style::default().fg(palette.info.color()),
        ));
    }

    if let Some(ref model) = detail.model_name {
        let provider = detail.provider.as_deref().unwrap_or("?");
        lines.push(Line::styled(
            format!("Model: {provider}/{model}"),
            Style::default().fg(palette.muted.color()),
        ));
    }

    lines.push(Line::raw(""));
}

/// Build patches + inline review lines for the detail view.
fn detail_patches_lines<'a>(
    detail: &'a crate::models::PatchsetDetail,
    palette: &'a ColorPalette,
    lines: &mut Vec<Line<'a>>,
) {
    let total_parts = detail.total_parts.unwrap_or(0);
    lines.push(Line::styled(
        format!("── Patches ({}) ──", detail.patches.len()),
        Style::default().fg(palette.accent.color()).bold(),
    ));

    if detail.patches.is_empty() {
        lines.push(Line::styled(
            "(no patches)",
            Style::default().fg(palette.muted.color()),
        ));
    } else {
        for patch in &detail.patches {
            let idx = patch.part_index.unwrap_or(0);
            let status_str = patch
                .status
                .as_ref()
                .map_or_else(|| "?".to_string(), ToString::to_string);
            let patch_subject = patch.subject.as_deref().unwrap_or("(no subject)");
            lines.push(Line::from(vec![
                Span::styled(
                    format!("{idx}/{total_parts}  "),
                    Style::default().fg(palette.accent.color()),
                ),
                Span::styled(
                    format!("[{status_str}]  "),
                    Style::default().fg(palette.success.color()),
                ),
                Span::raw(patch_subject),
            ]));

            let review = detail.reviews.iter().find(|r| r.patch_id == patch.id);
            if let Some(rev) = review {
                if let Some(ref summary) = rev.summary {
                    lines.push(Line::styled(
                        format!("  Summary: {summary}"),
                        Style::default().fg(palette.muted.color()),
                    ));
                }
                if let Some(ref inline) = rev.inline_review {
                    lines.push(Line::raw(""));
                    for review_line in inline.lines() {
                        lines.push(Line::styled(
                            format!("    {review_line}"),
                            Style::default().fg(palette.foreground.color()),
                        ));
                    }
                    lines.push(Line::raw(""));
                }
            } else {
                lines.push(Line::styled(
                    "  (no review)",
                    Style::default().fg(palette.muted.color()),
                ));
            }
        }
    }

    lines.push(Line::raw(""));
}

/// Build thread message lines for the detail view.
fn detail_thread_lines<'a>(
    detail: &'a crate::models::PatchsetDetail,
    palette: &'a ColorPalette,
    lines: &mut Vec<Line<'a>>,
) {
    lines.push(Line::styled(
        format!("── Thread ({}) ──", detail.thread.len()),
        Style::default().fg(palette.accent.color()).bold(),
    ));

    if detail.thread.is_empty() {
        lines.push(Line::styled(
            "(no messages)",
            Style::default().fg(palette.muted.color()),
        ));
    } else {
        for msg in &detail.thread {
            let author = msg.author.as_deref().unwrap_or("(unknown)");
            let date = format_date(msg.date);
            let subj = msg.subject.as_deref().unwrap_or("(no subject)");
            lines.push(Line::from(vec![
                Span::styled(author, Style::default().fg(palette.accent.color())),
                Span::raw("  "),
                Span::styled(date, Style::default().fg(palette.muted.color())),
            ]));
            lines.push(Line::styled(
                format!("  {subj}"),
                Style::default().fg(palette.foreground.color()),
            ));
        }
    }
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
            config.remotes.push(crate::config::RemoteConfig::fixture(name));
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
