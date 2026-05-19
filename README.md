<div align="center">
<h1> Remendo </h1>

![Remendo Logo](static/assets/remendo-patch-medium.png)
</div>

<div align="center">

[![CRAP score](https://img.shields.io/badge/dynamic/json?url=https%3A%2F%2Fderekbarbosa.github.io%2Fremendo-tui%2Fcrap-badge.json&query=%24.message&label=CRAP&color=green)](https://derekbarbosa.github.io/remendo-tui/crap/) [![License: AGPL-3.0](https://img.shields.io/badge/License-AGPL--3.0-blue.svg)](LICENSE)

![GitHub Pull Requests](https://img.shields.io/github/issues-pr/derekbarbosa/remendo-tui) ![GitHub Issues](https://img.shields.io/github/issues/derekbarbosa/remendo-tui) ![GitHub last commit](https://img.shields.io/github/last-commit/derekbarbosa/remendo-tui)

</div>

> "Remendo" is a Portuguese word that translates to *patch*, *mend*, or *repair*.
> It refers to a piece of material used to cover a hole or strengthen a damaged
> area

`remendo` is a terminal-based interface for viewing
[Sashiko](https://sashiko.dev/) patch reviews.

Inspired by mutt and other terminal mail user agents, it lets you monitor and
navigate AI-generated Linux kernel code reviews across multiple Sashiko
instances from a single viewport.

## Why?

The [Sashiko web UI](https://sashiko.dev/) works well for a single instance. But
if you deploy or monitor multiple Sashiko instances across different mailing
lists and kernel subsystems, switching between browser tabs gets tedious.

`remendo` treats each Sashiko instance as a **remote** (like a mail inbox),
letting you:

- Browse patchsets, patches, and AI reviews from your terminal
- Switch between remotes with a single keypress
- Search and filter by mailing list, subsystem, or status
- Bookmark review threads for later reference
- View raw review logs in your preferred `$EDITOR`

## Installation

### From source

Requires [Rust](https://rustup.rs/) (edition 2024, stable toolchain).

```sh
git clone https://github.com/your-org/remendo-tui.git
cd remendo-tui
cargo build --release
# Binary is at target/release/remendo-tui
```

### Cargo install (once published)

```sh
cargo install remendo-tui
```

## Quick Start

### 1. Create a configuration file

`remendo` looks for its config at `~/.config/remendo/config.toml` (or
`$XDG_CONFIG_HOME/remendo/config.toml`). The easiest way to get started is
to copy the included example:

```sh
mkdir -p ~/.config/remendo
cp config.example.toml ~/.config/remendo/config.toml
```

Then edit `~/.config/remendo/config.toml` and set your Sashiko instance URL:

```toml
[[remotes]]
name = "upstream"
url = "https://sashiko.dev"
```

The example file is fully commented and documents every option. See
[`config.example.toml`](config.example.toml) for the complete reference.

If no config file is found, `remendo` starts with compiled-in defaults and an
empty remote list.

### 2. Launch

```sh
remendo-tui
```

The interface shows a sidebar listing your configured remotes on the left and
the patchset list for the active remote on the right.

### 3. Navigate

`remendo` uses vim-inspired keybindings by default:

| Key | Action |
|-----|--------|
| `j` / `k` | Scroll down / up |
| `Ctrl-d` / `Ctrl-u` | Half-page down / up |
| `]` / `[` | Next / previous page |
| `Enter` | Open selected patchset |
| `Esc` | Close current view |
| `Tab` / `Shift-Tab` | Next / previous remote |
| `Ctrl-s` | Focus sidebar |
| `/` | Search |
| `r` | View raw review log |
| `Ctrl-r` | Refresh |
| `?` | Help |
| `q` | Quit |

All keybindings are configurable. See the
[Configuration Guide](docs/CONFIGURATION.md) for details.

### 4. Quit

Press `q` to exit. The terminal is always restored to its original state, even
if the application encounters an error.

## Documentation

- **[User Guide](docs/USER_GUIDE.md)** -- detailed walkthrough of all features
  and workflows
- **[Configuration Guide](docs/CONFIGURATION.md)** -- complete reference for
  `config.toml`, keybindings, themes, and all options

## Debug Logging

`remendo` writes debug logs to `~/.local/state/remendo/remendo.log` (or
`$XDG_STATE_HOME/remendo/remendo.log`). Control the log level with the
`REMENDO_LOG` environment variable:

```sh
# Default (info-level for remendo, nothing from deps)
remendo-tui

# Debug mode (all messages and commands logged)
REMENDO_LOG=debug remendo-tui

# Full trace (every tick, render, scroll event)
REMENDO_LOG=remendo_tui=trace remendo-tui

# Debug with dependency logs
REMENDO_LOG=remendo_tui=debug,reqwest=debug remendo-tui
```

Falls back to `RUST_LOG` if `REMENDO_LOG` is not set.

## Project Status

`remendo` is under active development. The interactive TUI is functional:

- Domain models for all Sashiko API entities
- TOML configuration with XDG paths, keybindings, and theming
- Async event loop (Elm/TEA architecture) with tokio
- HTTP client for Sashiko read-only API endpoints
- Remote sidebar with mailing list filtering
- Scrollable patchset list with pagination (`]`/`[`)
- Patchset detail view with inline AI reviews
- Vim-style search (`/`) with server-side filtering
- Help overlay (`?`) with all keybinding mappings
- Bookmark system with persistent storage (`b` to toggle)
- Comment navigation in detail view (`n`/`N` to jump between reviews)
- In-memory API response caching with configurable TTL
- File-based debug logging via `REMENDO_LOG`
- Column sorting (`s`/`S`) and bookmark filtering (`B`)
- Diff syntax highlighting in detail and review views
- 316 unit, integration, and doc tests
- CRAP score analysis with CI regression detection

Next priorities: surfacing review data in the TUI (the Sashiko review
API client is already implemented), authentication token injection, and
disk-based cache persistence. See `apogee_artifacts/features/ROADMAP.md`
for the full series plan.

## Requirements

- A running [Sashiko](https://github.com/sashiko-dev/sashiko) instance to
  connect to
- A terminal emulator with support for alternate screen and mouse events
- Rust stable toolchain (for building from source)

## License

AGPL-3.0-or-later. See [LICENSE](LICENSE) for details.
