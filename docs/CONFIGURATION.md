# Configuration Guide

`remendo` is configured through a single TOML file. All settings have
sensible compiled-in defaults -- you only need to override what you want
to change. If the config file is missing or contains errors, `remendo`
starts with defaults and displays a warning.

> **Quick start:** Copy the annotated example config from the repository
> root and edit it:
>
> ```sh
> mkdir -p ~/.config/remendo
> cp config.example.toml ~/.config/remendo/config.toml
> ```
>
> See [`config.example.toml`](../config.example.toml) for a fully
> commented starting point.

## Config File Location

`remendo` follows the [XDG Base Directory Specification](https://specifications.freedesktop.org/basedir-spec/latest/):

| Path | Purpose |
|------|---------|
| `$XDG_CONFIG_HOME/remendo/config.toml` | Configuration file |
| `$XDG_CACHE_HOME/remendo/` | Cached API responses |
| `$XDG_STATE_HOME/remendo/` | Persistent state (bookmarks, read markers) |

On most Linux systems, these resolve to:

```
~/.config/remendo/config.toml
~/.cache/remendo/
~/.local/state/remendo/
```

If `$XDG_CONFIG_HOME` is not set and no home directory is detected, `remendo`
falls back to `./config.toml` in the current working directory.

## Full Example Configuration

```toml
# Editor for viewing raw review logs.
# Falls back to $VISUAL, then $EDITOR, then "vi".
editor = "nvim"

# Cache settings
[cache]
# Override for cache directory (default: $XDG_CACHE_HOME/remendo/)
# dir = "/tmp/remendo-cache"
# Time-to-live for cached API responses, in seconds.
ttl_seconds = 300

# Remote Sashiko instances ("mailboxes").
# At least one remote is needed for useful operation.
# The first remote listed is the default active mailbox.
[[remotes]]
name = "upstream"
url = "https://sashiko.dev"

[[remotes]]
name = "staging"
url = "https://sashiko-staging.example.com"
# Environment variable containing an auth token (optional).
# The env var NAME is stored here, not the token itself.
auth_env = "SASHIKO_STAGING_TOKEN"
# Per-remote timeout override (default: 15 seconds)
timeout_seconds = 30
# Per-remote retry override (default: 3)
max_retries = 5

# Keybindings: maps action names to key combos.
[keybindings]
quit = "q"
scroll_down = "j"
scroll_up = "k"
scroll_half_page_down = "C-d"
scroll_half_page_up = "C-u"
next_mailbox = "Tab"
prev_mailbox = "S-Tab"
open_thread = "Enter"
close_thread = "Esc"
refresh = "C-r"
search = "/"
bookmark_toggle = "b"
view_raw_log = "r"
help = "?"
focus_sidebar = "C-s"
next_comment = "n"
prev_comment = "N"

# Theme / color scheme
[theme.colors]
foreground = "#d8d8d8"
background = "#181818"
accent = "#6a9fb5"
error = "#ac4242"
warning = "#f4bf75"
success = "#90a959"
info = "#75b5aa"
selected_bg = "#383838"
selected_fg = "#f8f8f8"
border = "#585858"
muted = "#6b6b6b"
```

## Sections Reference

### `editor`

**Type:** String (optional)
**Default:** `$VISUAL` -> `$EDITOR` -> `"vi"`

The command used to open raw review logs in an external editor. The
resolution chain is:

1. `editor` field in config.toml (if set)
2. `$VISUAL` environment variable
3. `$EDITOR` environment variable
4. `"vi"` (fallback)

```toml
editor = "nvim"
```

### `[cache]`

Controls local caching of API responses.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `dir` | String (path) | `$XDG_CACHE_HOME/remendo/` | Override for the cache directory |
| `ttl_seconds` | Integer | `300` | How long cached responses are considered fresh |

```toml
[cache]
ttl_seconds = 60
```

### `[[remotes]]`

Defines the Sashiko instances to connect to. Each remote is displayed
as a separate "mailbox" in the sidebar. You can define multiple remotes.

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `name` | String | Yes | -- | Display name for this remote |
| `url` | String | Yes | -- | Base URL of the Sashiko instance |
| `auth_env` | String | No | -- | Environment variable name containing a bearer token |
| `timeout_seconds` | Integer | No | `15` | HTTP request timeout in seconds |
| `max_retries` | Integer | No | `3` | Maximum retry attempts on transient failures |

```toml
[[remotes]]
name = "upstream"
url = "https://sashiko.dev"
```

**Notes:**
- The first remote in the list is the default active mailbox on launch.
- Remote order in the config file determines sidebar order.
- The `auth_env` field stores the **name** of an environment variable, not
  the token itself. Set the actual token in your shell environment.
- Retry logic uses exponential backoff (500ms base, 2x factor, 10s max).
  Only server errors (5xx) and connection failures are retried; client
  errors (4xx) fail immediately.

### `[keybindings]`

Maps semantic actions to key combinations. The format is
`action_name = "key_combo"`.

#### Key Combo Syntax

| Prefix | Modifier |
|--------|----------|
| `C-` | Ctrl |
| `S-` | Shift |
| `A-` | Alt |

Modifiers can be combined: `C-S-x` means Ctrl+Shift+x.

Single characters are written directly: `q`, `j`, `/`.

Special key names (case-insensitive):

```
Enter, Tab, Esc, Space, Backspace, Delete,
Up, Down, Left, Right, Home, End, PageUp, PageDown,
F1 through F12
```

#### Available Actions

| Action | Default Key | Description |
|--------|-------------|-------------|
| `quit` | `q` | Exit the application |
| `scroll_down` | `j` | Scroll down one item |
| `scroll_up` | `k` | Scroll up one item |
| `scroll_half_page_down` | `Ctrl-d` | Scroll down half a page |
| `scroll_half_page_up` | `Ctrl-u` | Scroll up half a page |
| `next_mailbox` | `Tab` | Switch to the next remote |
| `prev_mailbox` | `Shift-Tab` | Switch to the previous remote |
| `open_thread` | `Enter` | Open the selected patchset/thread |
| `close_thread` | `Esc` | Close the current detail view |
| `refresh` | `Ctrl-r` | Refresh the current view |
| `search` | `/` | Activate the search prompt |
| `bookmark_toggle` | `b` | Toggle bookmark on the selected item |
| `view_raw_log` | `r` | View the raw review log in `$EDITOR` |
| `help` | `?` | Show the help overlay |
| `focus_sidebar` | `Ctrl-s` | Focus the sidebar panel |
| `next_comment` | `n` | Jump to the next review comment |
| `prev_comment` | `N` | Jump to the previous review comment |
| `next_page` | `]` | Navigate to the next page of results |
| `prev_page` | `[` | Navigate to the previous page of results |

Unknown action names in the config are silently ignored (with a logged
warning). This ensures forward compatibility when upgrading `remendo`.

#### Custom Keybindings Example

Override only the keys you want to change. Unspecified actions keep
their defaults.

```toml
[keybindings]
quit = "C-q"          # Ctrl-q instead of q
refresh = "F5"        # F5 instead of Ctrl-r
search = "C-f"        # Ctrl-f instead of /
```

### `[theme.colors]`

Defines the semantic color palette for the TUI. Colors can be specified
as hex RGB (`#rrggbb`) or named colors.

#### Color Roles

| Role | Default | Description |
|------|---------|-------------|
| `foreground` | `#d8d8d8` | Default text color |
| `background` | `#181818` | Default background |
| `accent` | `#6a9fb5` | Highlights, active elements |
| `error` | `#ac4242` | Error messages, critical findings |
| `warning` | `#f4bf75` | Warnings, medium-severity findings |
| `success` | `#90a959` | Success indicators |
| `info` | `#75b5aa` | Informational text |
| `selected_bg` | `#383838` | Background of selected items |
| `selected_fg` | `#f8f8f8` | Text color of selected items |
| `border` | `#585858` | Borders and separators |
| `muted` | `#6b6b6b` | Dimmed/secondary text |

#### Named Colors

In addition to hex values, these named colors are supported:

```
black, red, green, yellow, blue, magenta, cyan, white,
gray, dark_gray, light_red, light_green, light_yellow,
light_blue, light_magenta, light_cyan, reset
```

#### Custom Theme Example

```toml
[theme.colors]
# Solarized Dark inspired
foreground = "#839496"
background = "#002b36"
accent = "#268bd2"
error = "#dc322f"
warning = "#b58900"
success = "#859900"
info = "#2aa198"
selected_bg = "#073642"
selected_fg = "#fdf6e3"
border = "#586e75"
muted = "#657b83"
```

If a color value can't be parsed, it falls back to the terminal's default
color with a logged warning. Invalid colors never crash the application.

## Error Handling

`remendo` is designed to never crash on configuration problems:

| Condition | Behavior |
|-----------|----------|
| No config file found | Start with defaults; log info message |
| TOML syntax error | Start with defaults; show warning in status bar |
| Invalid field value | Skip the field, use its default; log warning |
| Invalid remote URL | Skip that remote; other remotes still work |
| Invalid color value | Fall back to terminal default color |
| Unparseable key combo | Skip that binding; log warning |
| Unknown action name | Skip silently; forward-compatible |
| No remotes defined | Start with empty mailbox list |
