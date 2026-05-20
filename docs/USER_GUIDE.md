# User Guide

This guide walks through how to use `remendo` to browse and review
Linux kernel patches across one or more Sashiko instances.

## Concepts

Before diving in, here are the key terms used throughout `remendo`:

| Term | Meaning |
|------|---------|
| **Remote** | A configured Sashiko instance URL, treated like a mail inbox |
| **Mailbox** | User-facing name for a remote (displayed in the sidebar) |
| **Patchset** | A group of related patches submitted together (e.g., `[PATCH v2 0/5]`) |
| **Patch** | An individual diff within a patchset |
| **Review** | An AI-generated code review of a single patch, produced by Sashiko |
| **Finding** | A specific issue identified during review (Low/Medium/High/Critical severity) |
| **Thread** | Messages associated with a patchset on the Sashiko instance |

### How Sashiko Works

[Sashiko](https://sashiko.dev/) is an agentic patch review platform. It ingests
submitted patches and runs them through a multi-stage AI review pipeline. Each
patch receives a review with findings ranked by severity. `remendo` connects to
one or more Sashiko instances via their REST API to display these reviews in
your terminal.

## Getting Started

### First Run

If you launch `remendo` without a config file, it starts with defaults and
an empty remote list:

```sh
remendo
```

You'll see an empty interface with a message prompting you to configure a
remote. Press `q` to quit.

### Adding Your First Remote

The easiest way to get started is to copy the example config from the
repository:

```sh
mkdir -p ~/.config/remendo
cp config.example.toml ~/.config/remendo/config.toml
```

Then edit `~/.config/remendo/config.toml` and set your Sashiko URL:

```toml
[[remotes]]
name = "upstream"
url = "https://sashiko.dev"
```

The example file is fully commented and documents every available option.
See [`config.example.toml`](../config.example.toml) for details.

Launch again. The sidebar shows "upstream" as your active mailbox, and
the main pane loads the patchset list from sashiko.dev.

### Adding Multiple Remotes

```toml
[[remotes]]
name = "upstream"
url = "https://sashiko.dev"

[[remotes]]
name = "staging"
url = "https://sashiko-staging.example.com"

[[remotes]]
name = "local"
url = "http://localhost:8080"
```

The sidebar lists all three remotes. The first one is active by default.

## Navigating the Interface

`remendo` has a layout inspired by terminal mail clients:

```
+----------+--------------------------------------------------+
| Sidebar  |  Main Pane                                       |
|          |                                                   |
| upstream |  [PATCH v2 0/4] Fix null deref  | Reviewed  | 1M |
| staging  |  [PATCH 1/1] Add bounds check   | Pending   | 0C |
| local    |  [PATCH v3 0/7] Refactor locks  | In Review | 2H |
|          |                                                   |
+----------+--------------------------------------------------+
| Status bar                                                   |
+--------------------------------------------------------------+
```

### Sidebar

The sidebar lists your configured remotes. The active remote is
highlighted. Switch between remotes with:

- `Tab` -- next remote
- `Shift-Tab` -- previous remote
- `Ctrl-s` -- focus the sidebar for direct selection

### Patchset List

The main pane shows patchsets from the active remote. Each entry shows:

- **Subject** -- the patchset title
- **Status** -- current lifecycle state (Pending, In Review, Reviewed, Failed, etc.)
- **Findings** -- severity indicator (count of High/Critical findings)
- **Subsystems** -- kernel subsystem tags

Navigate the list with:

- `j` / `k` -- move selection down / up
- `Ctrl-d` / `Ctrl-u` -- half-page down / up
- `Enter` -- open the selected patchset

### Sorting the Patchset List

Press `s` to cycle the sort column through:

```
Default (API order) → Status → Date → Findings → Author → Default …
```

Press `S` (Shift-s) to reverse the sort direction without changing the
column. The active column header shows `▲` (ascending) or `▼`
(descending).

Sorting is client-side and applies to the current page only. Each new
page is re-sorted automatically. Switching remotes resets the sort to
the default API order.

### Bookmark Filtering

Press `B` (Shift-b) to toggle a display filter that shows only
bookmarked patchsets. The title bar shows `| [B] bookmarks` when the
filter is active. Press `B` again to show all patchsets.

The bookmark filter composes with sorting: when both are active, the
visible rows are the bookmarked subset in the current sort order.

Note: `b` toggles the bookmark on an individual item (persists), while
`B` toggles the display filter (resets on remote switch).

### Patchset Detail

When you open a patchset, you see:

- **Header** -- subject, author, date, status, baseline
- **Patches** -- list of individual diffs in the series
- **Reviews** -- AI review results for each patch, with syntax-highlighted
  inline reviews
- **Findings** -- issues found, sorted by severity
- **Thread** -- messages associated with this patchset

Navigate within the detail view with:

- `j` / `k` -- scroll detail content
- `Ctrl-d` / `Ctrl-u` -- half-page scroll
- `r` -- view the raw review log in your `$EDITOR`
- `n` / `N` -- next / previous finding or comment
- `Esc` -- return to the patchset list

### Inline Review Styling

Inline reviews display AI-generated code reviews from the Sashiko
review pipeline with three visually distinct sections:

- **Patch metadata** (dimmed) -- commit hash, author, subject, and
  description text that provides context about the patch under review.

- **Quoted diff** (color-coded) -- the original patch content, quoted
  with `>` prefix. Additions are green, deletions are red, hunk headers
  are cyan, and file headers are bold.

- **Reviewer commentary** (bright) -- the AI's analysis, suggestions,
  and questions. This is the primary content and renders in the default
  foreground color for maximum readability.

## Searching and Filtering

Press `/` to activate the search prompt. Type your query and press
`Enter`. The patchset list filters to show matching results.

Search matches against:
- Patchset subject lines
- Author names and email addresses
- Subsystem tags

To clear the search and return to the full list, press `Esc`. You can
also press `/` and submit an empty query.

### Mailing List Filter

If the Sashiko instance tracks multiple mailing lists, you can filter
the patchset list by mailing list. This narrows results to patches
submitted to a specific list (e.g., `netdev`, `linux-iio`).

## Bookmarks

Press `b` to bookmark the currently selected patchset. Bookmarked items
are visually marked with a `*` indicator in the list. Bookmarks persist
across sessions (stored in `$XDG_STATE_HOME/remendo/bookmarks.json`).

Press `b` again on a bookmarked patchset to remove the bookmark.

## Viewing Raw Logs

Press `r` on a patchset or review to open the raw AI review log in your
configured editor. This shows the full, unformatted review output
including all review stages:

1. Commit goal analysis
2. High-level implementation verification
3. Execution flow verification
4. Resource management (UAF, leaks)
5. Locking and synchronization
6. Security audit
7. Hardware engineer's review
8. Verification and severity estimation
9. Report generation

The editor used follows this resolution chain:
1. `editor` field in config.toml
2. `$VISUAL` environment variable
3. `$EDITOR` environment variable
4. `vi` (fallback)

When you close the editor, `remendo` resumes where you left off.

## Working with Multiple Remotes

### Use Cases

**Multi-instance monitoring:** Track the upstream public instance
alongside your organization's private staging instance.

**Local development:** Monitor a Sashiko instance running on
`localhost:8080` while also watching the production deployment.

**Subsystem-focused:** Configure separate remotes for different
kernel subsystems if you run per-subsystem Sashiko instances.

### Switching Remotes

- `Tab` cycles forward through remotes
- `Shift-Tab` cycles backward
- `Ctrl-s` focuses the sidebar for direct selection

The patchset list updates immediately when you switch remotes.

### Per-Remote Configuration

Each remote can have its own timeout and retry settings:

```toml
[[remotes]]
name = "slow-server"
url = "https://sashiko-slow.example.com"
timeout_seconds = 30
max_retries = 5
```

## Understanding Review Statuses

Patchsets move through a lifecycle:

```
Incomplete --> Pending --> In Review --> Reviewed
                                    \-> Failed
                                    \-> Failed to Apply
                   \-> Cancelled
                   \-> Skipped
```

| Status | Meaning |
|--------|---------|
| **Incomplete** | Not all patches in the series have been received |
| **Pending** | Queued for review |
| **In Review** | Currently being reviewed by the AI |
| **Reviewed** | Review complete; findings are available |
| **Failed** | Review encountered an error |
| **Failed to Apply** | Patch could not be applied to the git baseline |
| **Cancelled** | Review was manually cancelled |
| **Skipped** | Patchset was skipped (e.g., duplicate) |

## Understanding Finding Severities

Each finding in a review is assigned a severity:

| Severity | Meaning |
|----------|---------|
| **Low** | Minor style or informational issue |
| **Medium** | Worth investigating; may indicate a real problem |
| **High** | Significant issue likely to cause bugs |
| **Critical** | Severe issue requiring immediate attention |

The patchset list shows a summary of findings by severity. The detail
view shows each finding with its full description and severity rationale.

## Keyboard Reference

### Global

| Key | Action |
|-----|--------|
| `q` | Quit |
| `?` | Help overlay |
| `Ctrl-r` | Refresh current view |
| `Tab` | Next remote |
| `Shift-Tab` | Previous remote |
| `Ctrl-s` | Focus sidebar |

### List View

| Key | Action |
|-----|--------|
| `j` / `k` | Scroll down / up |
| `Ctrl-d` / `Ctrl-u` | Half-page down / up |
| `]` / `[` | Next / previous page |
| `Enter` | Open selected item |
| `/` | Search |
| `Esc` | Clear search filter |
| `b` | Toggle bookmark |
| `B` | Toggle bookmark-only filter |
| `s` | Cycle sort column |
| `S` | Reverse sort direction |
| `m` | Toggle patchset / message list |

### Detail View

| Key | Action |
|-----|--------|
| `j` / `k` | Scroll content |
| `Ctrl-d` / `Ctrl-u` | Half-page scroll |
| `n` / `N` | Next / previous comment |
| `r` | View raw log |
| `L` | View baseline application log |
| `Esc` | Close detail view |

All keybindings are customizable. See the
[Configuration Guide](CONFIGURATION.md) for the full keybinding
reference and syntax.

## Troubleshooting

### "No patchsets loaded"

- Check that your remote URL is correct and reachable
- Verify the Sashiko instance is running (`curl https://your-url/api/stats`)
- Check for network errors in the status bar

### Terminal looks broken after crash

`remendo` installs a panic hook that restores the terminal on any crash.
If the terminal is still in a bad state, run:

```sh
reset
```

### Config file not found

`remendo` logs a warning and uses defaults. Check that your config file
is at the expected XDG path:

```sh
echo "${XDG_CONFIG_HOME:-$HOME/.config}/remendo/config.toml"
```

### Connection timeouts

Increase the per-remote timeout in your config:

```toml
[[remotes]]
name = "slow"
url = "https://sashiko.example.com"
timeout_seconds = 30
max_retries = 5
```

### Colors look wrong

Ensure your terminal supports 24-bit (truecolor) rendering. Most modern
terminals do (kitty, alacritty, wezterm, iTerm2). If not, use named
colors (`red`, `blue`, etc.) instead of hex values in your theme config.
