//! Widget rendering — the TEA `view` function.
//!
//! Renders the application state into a terminal frame.
//! This module contains no state mutation — it is a pure
//! function of `App` → visual output.

use crate::app::{App, FocusPanel, InputMode, ListContent, SortColumn, SortDirection, ViewMode};
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
        ViewMode::Loading => {
            // Render the list underneath, then overlay the loading dialog
            render_main_pane(app, frame, chunks[1], palette);
            render_loading_dialog(app, frame, palette);
        }
        ViewMode::Detail => match app.list_content {
            ListContent::Patchsets => render_detail_view(app, frame, chunks[1], palette),
            ListContent::Messages => render_message_detail(app, frame, chunks[1], palette),
        },
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
    let (content_label, content_total, total_pages) = match app.list_content {
        ListContent::Patchsets => ("patchsets", app.patchsets.total, app.patchsets.total_pages()),
        ListContent::Messages => ("messages", app.messages.total, app.messages.total_pages()),
    };
    let page_indicator = if total_pages > 1 {
        format!(" | page {}/{total_pages}", app.list_params.page)
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
    let bookmark_indicator = if app.show_bookmarks_only { " | [B] bookmarks" } else { "" };
    if let Some(ref stats) = app.stats {
        format!(
            " remendo | {remote} | v{} | {} pending | {} reviewing | {content_total} {content_label}{page_indicator}{search_indicator}{list_indicator}{bookmark_indicator} ",
            stats.version, stats.pending, stats.reviewing
        )
    } else {
        format!(" remendo | {remote} | {content_total} {content_label}{page_indicator}{search_indicator}{list_indicator}{bookmark_indicator} ")
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
    let items_empty = match app.list_content {
        ListContent::Patchsets => app.patchsets.items.is_empty(),
        ListContent::Messages => app.messages.items.is_empty(),
    };
    if items_empty {
        let text = if app.error_state.is_some() {
            "Error loading. Press Ctrl-r to retry.".to_string()
        } else if let Some(ref q) = app.list_params.search {
            format!("No results for \"{q}\"")
        } else {
            "Loading...".to_string()
        };
        let paragraph = Paragraph::new(text).block(block);
        frame.render_widget(paragraph, area);
        return;
    }

    // Branch rendering by list content type
    if app.list_content == ListContent::Messages {
        render_message_table(app, frame, area, block, palette);
        return;
    }

    // Build table rows from patchsets
    let rows: Vec<Row> = app
        .patchsets
        .items
        .iter()
        .filter(|ps| {
            !app.show_bookmarks_only
                || app.bookmarks.contains(&app.active_remote, ps.id)
        })
        .map(|ps| build_patchset_row(app, ps, palette))
        .collect();

    let sort_header = |label: &str, col: SortColumn| -> String {
        if app.sort_column == col {
            match app.sort_direction {
                SortDirection::Ascending => format!("{label} \u{25b2}"),
                SortDirection::Descending => format!("{label} \u{25bc}"),
            }
        } else {
            label.to_string()
        }
    };

    let header = Row::new(vec![
        Cell::from(""),
        Cell::from(sort_header("Status", SortColumn::Status)),
        Cell::from("Subject".to_string()),
        Cell::from(sort_header("Author", SortColumn::Author)),
        Cell::from(sort_header("Date", SortColumn::Date)),
        Cell::from(sort_header("Findings", SortColumn::Findings)),
        Cell::from("Subsystems".to_string()),
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
    let status_text = match (&ps.failed_reason, ps.status) {
        (Some(reason), PatchsetStatus::Failed | PatchsetStatus::FailedToApply) => {
            let mut text = format!("Failed: {reason}");
            text.truncate(14);
            text
        }
        _ => ps.status.to_string(),
    };
    let status_cell = Cell::from(status_text).style(status_style);
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
/// Render a loading dialog showing the selected patchset being fetched.
/// Render the message list table.
fn render_message_table(
    app: &App,
    frame: &mut Frame,
    area: Rect,
    block: Block<'_>,
    palette: &ColorPalette,
) {
    let rows: Vec<Row> = app
        .messages
        .items
        .iter()
        .map(|msg| {
            Row::new(vec![
                Cell::from(msg.subject.as_deref().unwrap_or("(no subject)")),
                Cell::from(msg.author.as_deref().unwrap_or("(unknown)"))
                    .style(Style::default().fg(palette.muted.color())),
                Cell::from(format_date(msg.date))
                    .style(Style::default().fg(palette.muted.color())),
                Cell::from(msg.mailing_list.as_deref().unwrap_or(""))
                    .style(Style::default().fg(palette.muted.color())),
            ])
        })
        .collect();

    let header = Row::new(vec![
        Cell::from("Subject"),
        Cell::from("Author"),
        Cell::from("Date"),
        Cell::from("List"),
    ])
    .style(Style::default().fg(palette.accent.color()).bold());

    let widths = [
        Constraint::Min(30),
        Constraint::Length(20),
        Constraint::Length(10),
        Constraint::Length(24),
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

/// Render the message detail view.
/// Classify a raw unified diff line by its leading prefix.
///
/// Used for `EmailMessage.diff` content which is pure diff output.
/// Context lines and unrecognized content render as `muted`.
fn classify_diff_line(line: &str, palette: &ColorPalette) -> Style {
    if line.starts_with("+++") || line.starts_with("---")
        || line.starts_with("diff ") || line.starts_with("index ")
    {
        Style::default().fg(palette.foreground.color()).bold()
    } else if line.starts_with("@@") {
        Style::default().fg(palette.accent.color()).bold()
    } else if line.starts_with('+') {
        Style::default().fg(palette.success.color())
    } else if line.starts_with('-') {
        Style::default().fg(palette.error.color())
    } else {
        Style::default().fg(palette.muted.color())
    }
}

/// Classify a line from a Sashiko inline review with context about
/// whether we've seen quoted diff content yet.
///
/// Sashiko inline reviews have a three-part structure:
///
/// 1. **Patch metadata/summary** — unquoted lines *before* any `>`-quoted
///    diff (commit hash, Author:, Subject:, description, Links).
///    Styled as `muted`.
///
/// 2. **Quoted diff** — lines with `>` prefix containing patch content.
///    Classified by diff role (green/red/cyan/bold/muted).
///
/// 3. **Reviewer commentary** — unquoted lines *after* the first `>`-quoted
///    section. This is the AI review analysis. Styled as `foreground`.
///
/// The `seen_quoted` flag tracks whether any `>`-quoted line has been
/// encountered. Before that, unquoted text is patch context. After,
/// unquoted text is reviewer commentary.
fn classify_review_line(line: &str, seen_quoted: bool, palette: &ColorPalette) -> Style {
    if line.is_empty() {
        return Style::default();
    }

    if line.starts_with('>') {
        // Quoted content — strip quoting and classify as diff
        let stripped = strip_review_quoting(line);
        classify_diff_line(stripped, palette)
    } else if seen_quoted {
        // Unquoted after we've seen quoted content — reviewer commentary
        Style::default().fg(palette.foreground.color())
    } else {
        // Unquoted before any quoted content — patch metadata/summary
        Style::default().fg(palette.muted.color())
    }
}

/// Strip leading `>` quoting prefixes from a review line.
///
/// Sashiko inline reviews quote patch content with `>` prefixes.
/// Handles `"> "`, `">"`, and nested quoting like `">> "`.
fn strip_review_quoting(line: &str) -> &str {
    let mut s = line;
    while s.starts_with('>') {
        s = s.strip_prefix('>').unwrap_or(s);
        // Consume one optional space after each >
        s = s.strip_prefix(' ').unwrap_or(s);
    }
    s
}

fn render_message_detail(app: &App, frame: &mut Frame, area: Rect, palette: &ColorPalette) {
    let border_color = panel_border_color(app.focus, FocusPanel::PatchsetList, palette);

    let Some(ref msg) = app.selected_message else {
        let block = Block::default()
            .title(" Loading message... ".bold())
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color));
        frame.render_widget(Paragraph::new("").block(block), area);
        return;
    };

    let subject = msg.subject.as_deref().unwrap_or("(no subject)");
    let title = format!(" {subject} ");
    let block = Block::default()
        .title(title.bold())
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));

    let mut lines: Vec<Line> = Vec::new();

    // Header
    lines.push(Line::from(vec![
        Span::styled("From: ", Style::default().fg(palette.muted.color())),
        Span::raw(msg.author.as_deref().unwrap_or("(unknown)")),
    ]));
    lines.push(Line::from(vec![
        Span::styled("Date: ", Style::default().fg(palette.muted.color())),
        Span::raw(format_date(msg.date)),
    ]));
    if let Some(ref to) = msg.to {
        lines.push(Line::from(vec![
            Span::styled("To: ", Style::default().fg(palette.muted.color())),
            Span::raw(to.as_str()),
        ]));
    }
    if let Some(ref cc) = msg.cc {
        lines.push(Line::from(vec![
            Span::styled("Cc: ", Style::default().fg(palette.muted.color())),
            Span::raw(cc.as_str()),
        ]));
    }
    if let Some(ref list) = msg.mailing_list {
        lines.push(Line::from(vec![
            Span::styled("List: ", Style::default().fg(palette.muted.color())),
            Span::raw(list.as_str()),
        ]));
    }

    lines.push(Line::raw(""));

    // Body
    if let Some(ref body) = msg.body {
        for body_line in body.lines() {
            lines.push(Line::raw(body_line));
        }
    } else {
        lines.push(Line::styled(
            "(no body)",
            Style::default().fg(palette.muted.color()),
        ));
    }

    // Diff section
    if let Some(ref diff) = msg.diff {
        lines.push(Line::raw(""));
        lines.push(Line::styled(
            "── Diff ──",
            Style::default().fg(palette.accent.color()).bold(),
        ));
        for diff_line in diff.lines() {
            let style = classify_diff_line(diff_line, palette);
            lines.push(Line::styled(diff_line, style));
        }
    }

    let scroll_offset = u16::try_from(app.detail_scroll_offset).unwrap_or(u16::MAX);
    let paragraph = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false })
        .scroll((scroll_offset, 0));

    frame.render_widget(paragraph, area);
}

fn render_loading_dialog(app: &App, frame: &mut Frame, palette: &ColorPalette) {
    let Some(ref ctx) = app.loading_context else {
        return;
    };

    let popup_width: u16 = 50;
    let popup_height: u16 = 7;
    let area = centered_rect(popup_width, popup_height, frame.area());

    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(" Loading Patchset ".bold())
        .borders(Borders::ALL)
        .border_style(Style::default().fg(palette.accent.color()));

    let text = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  ID: ", Style::default().fg(palette.muted.color())),
            Span::styled(
                ctx.patchset_id.to_string(),
                Style::default().fg(palette.accent.color()),
            ),
            Span::raw("  "),
            Span::styled(
                format!("[{}]", ctx.status),
                Style::default().fg(palette.foreground.color()),
            ),
        ]),
        Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::raw(&ctx.subject),
        ]),
        Line::from(""),
        Line::styled(
            "  Loading detail...",
            Style::default().fg(palette.muted.color()),
        ),
    ];

    let paragraph = Paragraph::new(text).block(block);
    frame.render_widget(paragraph, area);
}

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

    // Show failure reason if present
    if let Some(ref reason) = detail.failed_reason {
        lines.push(Line::styled(
            format!("Failed: {reason}"),
            Style::default().fg(palette.error.color()),
        ));
    }

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
        let has_logs = detail
            .baseline_logs
            .as_ref()
            .is_some_and(|s| !s.is_empty());
        let mut spans = vec![Span::styled(
            format!("Baseline: {branch} @ {commit}"),
            Style::default().fg(palette.info.color()),
        )];
        if has_logs {
            spans.push(Span::styled(
                "  [logs: L]",
                Style::default().fg(palette.accent.color()),
            ));
        }
        lines.push(Line::from(spans));
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
                    let mut seen_quoted = false;
                    for review_line in inline.lines() {
                        if review_line.starts_with('>') {
                            seen_quoted = true;
                        }
                        let style = classify_review_line(review_line, seen_quoted, palette);
                        lines.push(Line::from(vec![
                            Span::raw("    "),
                            Span::styled(review_line, style),
                        ]));
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

/// Build thread message lines for the patchset detail view.
///
/// Each message in the patchset's associated thread is rendered as two
/// lines: author + date, then subject. This is a flat list — messages
/// are rendered in API-returned order without indentation.
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

    // --- diff-syntax-highlighting tests ---

    // Raw diff lines (EmailMessage.diff path)

    #[test]
    fn classify_diff_line_addition() {
        let palette = ColorPalette::default();
        let style = classify_diff_line("+    x = compute();", &palette);
        assert_eq!(style.fg, Some(palette.success.color()));
    }

    #[test]
    fn classify_diff_line_deletion() {
        let palette = ColorPalette::default();
        let style = classify_diff_line("-    return 0;", &palette);
        assert_eq!(style.fg, Some(palette.error.color()));
    }

    #[test]
    fn classify_diff_line_hunk_header() {
        let palette = ColorPalette::default();
        let style = classify_diff_line("@@ -10,3 +10,4 @@ int main(void)", &palette);
        assert_eq!(style.fg, Some(palette.accent.color()));
    }

    #[test]
    fn classify_diff_line_file_header_plus() {
        let palette = ColorPalette::default();
        let style = classify_diff_line("+++ b/foo.c", &palette);
        assert_eq!(style.fg, Some(palette.foreground.color()));
    }

    #[test]
    fn classify_diff_line_file_header_minus() {
        let palette = ColorPalette::default();
        let style = classify_diff_line("--- a/foo.c", &palette);
        assert_eq!(style.fg, Some(palette.foreground.color()));
    }

    #[test]
    fn classify_diff_line_diff_header() {
        let palette = ColorPalette::default();
        let style = classify_diff_line("diff --git a/foo.c b/foo.c", &palette);
        assert_eq!(style.fg, Some(palette.foreground.color()));
    }

    #[test]
    fn classify_diff_line_index_header() {
        let palette = ColorPalette::default();
        let style = classify_diff_line("index abc123..def456 100644", &palette);
        assert_eq!(style.fg, Some(palette.foreground.color()));
    }

    #[test]
    fn classify_diff_line_context() {
        let palette = ColorPalette::default();
        let style = classify_diff_line(" int x = 0;", &palette);
        assert_eq!(style.fg, Some(palette.muted.color()));
    }

    #[test]
    fn classify_diff_line_empty() {
        let palette = ColorPalette::default();
        let style = classify_diff_line("", &palette);
        assert_eq!(style.fg, Some(palette.muted.color()));
    }

    // --- classify_review_line tests (inline_review path) ---

    // Quoted diff lines — always colored by diff role regardless of seen_quoted

    #[test]
    fn review_line_quoted_addition() {
        let palette = ColorPalette::default();
        let style = classify_review_line("> +    x = compute();", true, &palette);
        assert_eq!(style.fg, Some(palette.success.color()));
    }

    #[test]
    fn review_line_quoted_deletion() {
        let palette = ColorPalette::default();
        let style = classify_review_line("> -    return 0;", true, &palette);
        assert_eq!(style.fg, Some(palette.error.color()));
    }

    #[test]
    fn review_line_quoted_hunk_header() {
        let palette = ColorPalette::default();
        let style = classify_review_line("> @@ -10,3 +10,4 @@", true, &palette);
        assert_eq!(style.fg, Some(palette.accent.color()));
    }

    #[test]
    fn review_line_quoted_file_header() {
        let palette = ColorPalette::default();
        let style = classify_review_line("> +++ b/foo.c", true, &palette);
        assert_eq!(style.fg, Some(palette.foreground.color()));
    }

    #[test]
    fn review_line_double_quoted() {
        let palette = ColorPalette::default();
        let style = classify_review_line(">> +added in nested quote", true, &palette);
        assert_eq!(style.fg, Some(palette.success.color()));
    }

    #[test]
    fn review_line_quoted_no_space() {
        let palette = ColorPalette::default();
        let style = classify_review_line(">+added line", true, &palette);
        assert_eq!(style.fg, Some(palette.success.color()));
    }

    #[test]
    fn review_line_quoted_context() {
        let palette = ColorPalette::default();
        let style = classify_review_line(">  context line", true, &palette);
        assert_eq!(style.fg, Some(palette.muted.color()));
    }

    // Patch metadata — unquoted text BEFORE any quoted diff (seen_quoted=false)

    #[test]
    fn review_line_metadata_commit_hash() {
        let palette = ColorPalette::default();
        let style = classify_review_line(
            "commit 37076247c47b0c3300cbefe9a1791f066ad33f2e",
            false,
            &palette,
        );
        // Patch metadata is muted
        assert_eq!(style.fg, Some(palette.muted.color()));
    }

    #[test]
    fn review_line_metadata_author() {
        let palette = ColorPalette::default();
        let style = classify_review_line("Author: Audra Mitchell <audra@redhat.com>", false, &palette);
        assert_eq!(style.fg, Some(palette.muted.color()));
    }

    #[test]
    fn review_line_metadata_subject() {
        let palette = ColorPalette::default();
        let style = classify_review_line(
            "Subject: mm/hugetlb: fix avoid_reserve to allow taking folio from subpool",
            false,
            &palette,
        );
        assert_eq!(style.fg, Some(palette.muted.color()));
    }

    #[test]
    fn review_line_metadata_link() {
        let palette = ColorPalette::default();
        let style = classify_review_line(
            "Link: https://lkml.kernel.org/r/20250107204002.2683356-1-peterx@redhat.com",
            false,
            &palette,
        );
        assert_eq!(style.fg, Some(palette.muted.color()));
    }

    #[test]
    fn review_line_metadata_description() {
        let palette = ColorPalette::default();
        let style = classify_review_line(
            "This commit backports an upstream change to allow hugetlb COW faults",
            false,
            &palette,
        );
        assert_eq!(style.fg, Some(palette.muted.color()));
    }

    // Reviewer commentary — unquoted text AFTER quoted diff (seen_quoted=true)

    #[test]
    fn review_line_commentary_is_foreground() {
        let palette = ColorPalette::default();
        let style = classify_review_line("LGTM. The null check looks correct.", true, &palette);
        assert_eq!(style.fg, Some(palette.foreground.color()));
    }

    #[test]
    fn review_line_commentary_suggestion() {
        let palette = ColorPalette::default();
        let style = classify_review_line("Consider adding a dev_err() log here.", true, &palette);
        assert_eq!(style.fg, Some(palette.foreground.color()));
    }

    #[test]
    fn review_line_commentary_question() {
        let palette = ColorPalette::default();
        let style = classify_review_line("Should this be guarded by a mutex?", true, &palette);
        assert_eq!(style.fg, Some(palette.foreground.color()));
    }

    // Empty lines — always unstyled regardless of phase

    #[test]
    fn review_line_empty_is_unstyled() {
        let palette = ColorPalette::default();
        let style = classify_review_line("", false, &palette);
        assert_eq!(style.fg, None);
    }

    #[test]
    fn review_line_empty_after_quoted_is_unstyled() {
        let palette = ColorPalette::default();
        let style = classify_review_line("", true, &palette);
        assert_eq!(style.fg, None);
    }

    // Full inline review simulation — test the stateful rendering loop

    #[test]
    fn review_stateful_classification_full_review() {
        let palette = ColorPalette::default();
        let review = concat!(
            "commit abc123\n",
            "Author: dev@example.com\n",
            "Subject: Fix null deref\n",
            "\n",
            "This fixes the bug.\n",
            "\n",
            "> diff --git a/foo.c b/foo.c\n",
            "> --- a/foo.c\n",
            "> +++ b/foo.c\n",
            "> @@ -10,3 +10,4 @@\n",
            ">  int x = 0;\n",
            "> -    return 0;\n",
            "> +    x = compute();\n",
            "\n",
            "Good fix. Consider also checking for overflow.\n",
        );

        let mut seen_quoted = false;
        let mut styles: Vec<Option<ratatui::style::Color>> = Vec::new();
        for line in review.lines() {
            if line.starts_with('>') {
                seen_quoted = true;
            }
            let style = classify_review_line(line, seen_quoted, &palette);
            styles.push(style.fg);
        }

        // Lines 0-2: commit, Author, Subject — metadata (muted)
        assert_eq!(styles[0], Some(palette.muted.color()), "commit hash");
        assert_eq!(styles[1], Some(palette.muted.color()), "Author");
        assert_eq!(styles[2], Some(palette.muted.color()), "Subject");
        // Line 3: empty
        assert_eq!(styles[3], None, "empty separator");
        // Line 4: description prose — metadata (muted, before any >)
        assert_eq!(styles[4], Some(palette.muted.color()), "description");
        // Line 5: empty
        assert_eq!(styles[5], None, "empty separator");
        // Lines 6-12: quoted diff content
        assert_eq!(styles[6], Some(palette.foreground.color()), "diff header"); // bold
        assert_eq!(styles[7], Some(palette.foreground.color()), "--- header"); // bold
        assert_eq!(styles[8], Some(palette.foreground.color()), "+++ header"); // bold
        assert_eq!(styles[9], Some(palette.accent.color()), "@@ hunk");
        assert_eq!(styles[10], Some(palette.muted.color()), "context");
        assert_eq!(styles[11], Some(palette.error.color()), "deletion");
        assert_eq!(styles[12], Some(palette.success.color()), "addition");
        // Line 13: empty after quoted section
        assert_eq!(styles[13], None, "empty separator");
        // Line 14: reviewer commentary (foreground, after quoted)
        assert_eq!(styles[14], Some(palette.foreground.color()), "commentary");
    }

    // strip_review_quoting unit tests

    #[test]
    fn strip_review_quoting_no_prefix() {
        assert_eq!(strip_review_quoting("+added line"), "+added line");
    }

    #[test]
    fn strip_review_quoting_single_level() {
        assert_eq!(strip_review_quoting("> +added line"), "+added line");
    }

    #[test]
    fn strip_review_quoting_double_level() {
        assert_eq!(strip_review_quoting(">> +added line"), "+added line");
    }

    #[test]
    fn strip_review_quoting_no_space() {
        assert_eq!(strip_review_quoting(">+added line"), "+added line");
    }

    #[test]
    fn strip_review_quoting_empty() {
        assert_eq!(strip_review_quoting(""), "");
    }

    #[test]
    fn strip_review_quoting_only_chevron() {
        assert_eq!(strip_review_quoting("> "), "");
    }

    // --- thread rendering test ---

    #[test]
    fn detail_thread_lines_renders_flat() {
        use crate::models::{PatchsetDetail, ThreadMessage};

        let mut detail = PatchsetDetail::fixture();
        detail.thread = vec![
            ThreadMessage {
                id: 1,
                message_id: Some("msg-1@example.com".to_string()),
                author: Some("developer@kernel.org".to_string()),
                date: Some(1_778_690_980),
                subject: Some("[PATCH v2 0/3] Fix null deref".to_string()),
                in_reply_to: None,
            },
            ThreadMessage {
                id: 2,
                message_id: Some("msg-2@example.com".to_string()),
                author: Some("maintainer@kernel.org".to_string()),
                date: Some(1_778_700_000),
                subject: Some("Re: [PATCH v2 0/3] Fix null deref".to_string()),
                in_reply_to: Some("msg-1@example.com".to_string()),
            },
        ];

        let palette = ColorPalette::default();
        let mut lines: Vec<Line> = Vec::new();
        detail_thread_lines(&detail, &palette, &mut lines);

        // Line 0: section header
        assert!(lines[0].to_string().contains("Thread (2)"));

        // Lines 1-2: first message (flat, no indent)
        assert!(lines[1].spans[0].content.contains("developer@kernel.org"));

        // Lines 3-4: second message (flat, no indent)
        assert!(lines[3].spans[0].content.contains("maintainer@kernel.org"));

        // 1 header + 2 messages × 2 lines = 5 lines
        assert_eq!(lines.len(), 5);
    }

    #[test]
    fn detail_header_lines_basic() {
        let palette = ColorPalette::default();
        let detail = crate::models::PatchsetDetail::fixture();
        let mut lines: Vec<Line> = Vec::new();
        detail_header_lines(&detail, &palette, &mut lines);
        // Should have at least status line + empty separator
        assert!(!lines.is_empty());
        // Last line should be the empty separator
        assert!(lines.last().unwrap().spans.is_empty() || lines.last().unwrap().to_string().is_empty());
    }

    #[test]
    fn detail_header_lines_with_failure_reason() {
        let palette = ColorPalette::default();
        let mut detail = crate::models::PatchsetDetail::fixture();
        detail.failed_reason = Some("compile error".to_string());
        let mut lines: Vec<Line> = Vec::new();
        detail_header_lines(&detail, &palette, &mut lines);
        let text: String = lines.iter().map(|l| l.to_string()).collect::<Vec<_>>().join("\n");
        assert!(text.contains("compile error"), "got: {text}");
    }

    #[test]
    fn detail_header_lines_with_baseline() {
        let palette = ColorPalette::default();
        let mut detail = crate::models::PatchsetDetail::fixture();
        detail.baseline = Some(crate::models::Baseline {
            branch: Some("main".to_string()),
            commit: Some("abc123def456".to_string()),
            repo_url: None,
        });
        let mut lines: Vec<Line> = Vec::new();
        detail_header_lines(&detail, &palette, &mut lines);
        let text: String = lines.iter().map(|l| l.to_string()).collect::<Vec<_>>().join("\n");
        assert!(text.contains("Baseline:"), "got: {text}");
        assert!(text.contains("main"), "got: {text}");
    }

    #[test]
    fn detail_header_lines_with_model() {
        let palette = ColorPalette::default();
        let mut detail = crate::models::PatchsetDetail::fixture();
        detail.model_name = Some("gpt-4".to_string());
        detail.provider = Some("openai".to_string());
        let mut lines: Vec<Line> = Vec::new();
        detail_header_lines(&detail, &palette, &mut lines);
        let text: String = lines.iter().map(|l| l.to_string()).collect::<Vec<_>>().join("\n");
        assert!(text.contains("Model: openai/gpt-4"), "got: {text}");
    }

    #[test]
    fn detail_patches_lines_empty() {
        let palette = ColorPalette::default();
        let mut detail = crate::models::PatchsetDetail::fixture();
        detail.patches.clear();
        let mut lines: Vec<Line> = Vec::new();
        detail_patches_lines(&detail, &palette, &mut lines);
        let text: String = lines.iter().map(|l| l.to_string()).collect::<Vec<_>>().join("\n");
        assert!(text.contains("Patches (0)"), "got: {text}");
        assert!(text.contains("(no patches)"), "got: {text}");
    }

    #[test]
    fn detail_patches_lines_with_patches() {
        let palette = ColorPalette::default();
        let detail = crate::models::PatchsetDetail::fixture();
        let mut lines: Vec<Line> = Vec::new();
        detail_patches_lines(&detail, &palette, &mut lines);
        // Should have header + at least one patch line
        assert!(lines.len() >= 2);
        let text: String = lines.iter().map(|l| l.to_string()).collect::<Vec<_>>().join("\n");
        assert!(text.contains("Patches ("), "got: {text}");
    }
}
