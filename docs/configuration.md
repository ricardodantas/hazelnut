# Configuration Reference

Hazelnut uses [TOML](https://toml.io) for configuration. This document provides a complete reference for all configuration options.

## Config File Location

The default configuration file location is:

```
~/.config/hazelnut/config.toml
```

You can specify a different config file with the `--config` flag:

```bash
hazelnut --config /path/to/config.toml
hazelnutd --config /path/to/config.toml run
```

## Complete Example

Here's a comprehensive example showing all available options:

```toml
# ═══════════════════════════════════════════════════════════════════════════════
# HAZELNUT CONFIGURATION
# ═══════════════════════════════════════════════════════════════════════════════

# ───────────────────────────────────────────────────────────────────────────────
# General Settings
# ───────────────────────────────────────────────────────────────────────────────

[general]
# Logging level: trace, debug, info, warn, error
log_level = "info"

# Path to log file (optional - logs to stdout if not set)
log_file = "~/.local/share/hazelnut/hazelnut.log"

# Dry run mode - preview actions without executing
# Useful for testing new rules
dry_run = false

# Debounce time in seconds
# Wait this long after a file change before processing
# Helps avoid processing files still being written
debounce_seconds = 2

# TUI theme (see Themes section below)
theme = "dracula"

# ───────────────────────────────────────────────────────────────────────────────
# Watch Folders
# ───────────────────────────────────────────────────────────────────────────────

# Basic watch - monitors for new/changed files
[[watch]]
path = "~/Downloads"

# Watch with options
[[watch]]
path = "~/Desktop"
recursive = false           # Don't watch subdirectories (default: false)
rules = ["screenshots"]     # Only apply these rules (empty = all rules)

# Recursive watch - monitors subdirectories too
[[watch]]
path = "~/Documents/Inbox"
recursive = true

# ───────────────────────────────────────────────────────────────────────────────
# Rules
# ───────────────────────────────────────────────────────────────────────────────

# Each rule has a condition and an action
# When a file matches the condition, the action is executed

[[rule]]
name = "Example Rule"
enabled = true              # Enable/disable rule (default: true)
stop_processing = false     # Stop checking other rules if matched (default: false)

[rule.condition]
# ... conditions here

[rule.action]
# ... action here
```

---

## General Settings

The `[general]` section configures global behavior.

| Setting | Type | Default | Description |
|---------|------|---------|-------------|
| `log_level` | string | `"info"` | Logging verbosity: `trace`, `debug`, `info`, `warn`, `error` |
| `log_file` | string | none | Path to log file. If not set, logs to stdout |
| `dry_run` | bool | `false` | Preview mode - show what would happen without doing it |
| `debounce_seconds` | int | `2` | Wait time before processing after file change |
| `theme` | string | `"dracula"` | TUI color theme |

### Available Themes

```toml
[general]
theme = "dracula"  # Options below
```

| Theme | Description |
|-------|-------------|
| `dracula` | Dark purple aesthetic (default) |
| `one-dark-pro` | Atom's iconic dark theme |
| `nord` | Arctic, bluish color palette |
| `catppuccin-mocha` | Warm pastel dark theme |
| `catppuccin-latte` | Warm pastel light theme |
| `gruvbox-dark` | Retro groove colors |
| `gruvbox-light` | Retro groove, light variant |
| `tokyo-night` | Futuristic dark blue |
| `solarized-dark` | Precision colors, dark |
| `solarized-light` | Precision colors, light |
| `monokai-pro` | Classic syntax highlighting |
| `rose-pine` | All natural pine, faux fur, soho vibes |
| `kanagawa` | Inspired by Katsushika Hokusai |
| `everforest` | Comfortable green forest theme |
| `cyberpunk` | Neon-soaked futuristic theme |

---

## Watch Configuration

Watch folders define which directories Hazelnut monitors for changes.

```toml
[[watch]]
path = "~/Downloads"
recursive = false
rules = []
```

| Field | Type | Default | Required | Description |
|-------|------|---------|----------|-------------|
| `path` | string | — | ✅ | Directory to watch. Supports `~` for home directory |
| `recursive` | bool | `false` | ❌ | Also watch subdirectories |
| `rules` | array | `[]` | ❌ | Rule names to apply. Empty = all rules |

### Examples

```toml
# Simple watch
[[watch]]
path = "~/Downloads"

# Recursive watch
[[watch]]
path = "~/Documents"
recursive = true

# Watch with specific rules only
[[watch]]
path = "~/Desktop"
rules = ["screenshots", "temp-files"]

# Multiple watches
[[watch]]
path = "~/Downloads"

[[watch]]
path = "~/Desktop"

[[watch]]
path = "/tmp/incoming"
recursive = true
```

---

## Rules

Rules are the core of Hazelnut. Each rule consists of:

1. **Metadata** - Name, enabled status, processing behavior
2. **Condition** - What files to match
3. **Action** - What to do with matched files

### Rule Structure

```toml
[[rule]]
name = "Rule Name"
enabled = true
stop_processing = false

[rule.condition]
# ... conditions

[rule.action]
# ... action
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | string | — | Human-readable rule name (required) |
| `enabled` | bool | `true` | Whether rule is active |
| `stop_processing` | bool | `false` | If true, stop checking other rules after this one matches |

---

## Conditions

Conditions determine which files a rule applies to. **All conditions must match** for a rule to trigger.

### File Name Conditions

#### `name_matches` — Glob Pattern

Match filename using glob/wildcard patterns.

```toml
[rule.condition]
name_matches = "Screenshot*.png"
```

**Pattern syntax:**

| Pattern | Matches |
|---------|---------|
| `*` | Any sequence of characters |
| `?` | Any single character |
| `[abc]` | Any character in set |
| `[a-z]` | Any character in range |

**Examples:**

```toml
# Screenshots
name_matches = "Screenshot*.png"

# Any image
name_matches = "*.{jpg,png,gif}"

# Files starting with 'report'
name_matches = "report*"

# Invoice with number
name_matches = "invoice_????.pdf"  # invoice_0001.pdf, etc.
```

#### `name_regex` — Regular Expression

Match filename using regex for complex patterns.

```toml
[rule.condition]
name_regex = "^invoice_\\d{4}\\.pdf$"
```

**Examples:**

```toml
# Invoice with 4-digit number
name_regex = "^invoice_\\d{4}\\.pdf$"

# Date-prefixed files
name_regex = "^\\d{4}-\\d{2}-\\d{2}_.*"

# UUID filenames
name_regex = "^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}\\."
```

> ⚠️ Remember to escape backslashes in TOML: `\\d` not `\d`

### File Extension Conditions

#### `extension` — Single Extension

Match files with a specific extension (case-insensitive).

```toml
[rule.condition]
extension = "pdf"
```

#### `extensions` — Multiple Extensions

Match files with any of the listed extensions.

```toml
[rule.condition]
extensions = ["jpg", "jpeg", "png", "gif", "webp"]
```

### File Size Conditions

Sizes are specified in **bytes**.

| Common Sizes | Bytes |
|--------------|-------|
| 1 KB | 1024 |
| 1 MB | 1048576 |
| 10 MB | 10485760 |
| 100 MB | 104857600 |
| 1 GB | 1073741824 |

#### `size_greater_than`

Match files larger than the specified size.

```toml
[rule.condition]
size_greater_than = 10485760  # > 10 MB
```

#### `size_less_than`

Match files smaller than the specified size.

```toml
[rule.condition]
size_less_than = 1048576  # < 1 MB
```

#### Size Range Example

```toml
[rule.condition]
size_greater_than = 1048576    # > 1 MB
size_less_than = 104857600     # < 100 MB
```

### File Age Conditions

Ages are specified in **days** based on the file's modification time.

#### `age_days_greater_than`

Match files older than the specified number of days.

```toml
[rule.condition]
age_days_greater_than = 30  # Older than 30 days
```

#### `age_days_less_than`

Match files newer than the specified number of days.

```toml
[rule.condition]
age_days_less_than = 7  # Newer than 7 days
```

### File Type Conditions

#### `is_directory`

Match directories (`true`) or files (`false`).

```toml
[rule.condition]
is_directory = false  # Only match files
```

```toml
[rule.condition]
is_directory = true  # Only match directories
```

#### `is_hidden`

Match hidden files (starting with `.`).

```toml
[rule.condition]
is_hidden = true  # Only hidden files
```

```toml
[rule.condition]
is_hidden = false  # Only visible files
```

### Combining Conditions

All conditions must match. This creates AND logic.

```toml
[[rule]]
name = "Large old PDFs"

[rule.condition]
extension = "pdf"
size_greater_than = 10485760    # > 10 MB
age_days_greater_than = 90      # > 90 days old
is_hidden = false               # Not hidden
```

---

## Actions

Actions define what to do with matched files.

### Move

Move file to a destination folder.

```toml
[rule.action]
type = "move"
destination = "~/Documents/Archive"
create_destination = true  # Create folder if missing (default: true)
overwrite = false          # Overwrite existing files (default: false)
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `destination` | string | — | Target directory (required) |
| `create_destination` | bool | `true` | Create directory if it doesn't exist |
| `overwrite` | bool | `false` | Overwrite if file exists at destination |

### Copy

Copy file to a destination (original remains).

```toml
[rule.action]
type = "copy"
destination = "~/Backup"
create_destination = true
overwrite = false
```

Same options as Move.

### Rename

Rename the file using a pattern.

```toml
[rule.action]
type = "rename"
pattern = "{date}_{name}.{ext}"
```

#### Pattern Variables

| Variable | Description | Example |
|----------|-------------|---------|
| `{name}` | Filename without extension | `document` |
| `{filename}` | Full filename with extension | `document.pdf` |
| `{ext}` | File extension (without dot) | `pdf` |
| `{path}` | Full file path | `/home/user/document.pdf` |
| `{dir}` | Parent directory path | `/home/user` |
| `{date}` | Current date (YYYY-MM-DD) | `2024-01-15` |
| `{datetime}` | Current datetime | `2024-01-15_14-30-00` |
| `{date:FORMAT}` | Custom date format | See below |

#### Custom Date Formats

Use `{date:FORMAT}` with [chrono format specifiers](https://docs.rs/chrono/latest/chrono/format/strftime/index.html):

| Pattern | Example |
|---------|---------|
| `{date:%Y%m%d}` | `20240115` |
| `{date:%d-%m-%Y}` | `15-01-2024` |
| `{date:%B %d, %Y}` | `January 15, 2024` |
| `{date:%Y-W%V}` | `2024-W03` (week number) |

**Examples:**

```toml
# Add date prefix
pattern = "{date}_{filename}"
# document.pdf → 2024-01-15_document.pdf

# Add datetime suffix
pattern = "{name}_{datetime}.{ext}"
# photo.jpg → photo_2024-01-15_14-30-00.jpg

# Custom format
pattern = "{date:%Y%m%d}_{name}.{ext}"
# invoice.pdf → 20240115_invoice.pdf
```

### Trash

Move file to system trash (recoverable).

```toml
[rule.action]
type = "trash"
```

### Delete

**Permanently** delete the file.

```toml
[rule.action]
type = "delete"
```

> ⚠️ **Warning**: This action is irreversible! Use `trash` if you want to be able to recover files.

### Run

Execute a shell command.

```toml
[rule.action]
type = "run"
command = "convert"
args = ["{path}", "-resize", "50%", "{dir}/{name}_small.{ext}"]
```

| Field | Type | Description |
|-------|------|-------------|
| `command` | string | Command to execute |
| `args` | array | Arguments (supports pattern variables) |

**Examples:**

```toml
# Compress images
[rule.action]
type = "run"
command = "convert"
args = ["{path}", "-quality", "80", "{path}"]

# Extract archives
[rule.action]
type = "run"
command = "unzip"
args = ["{path}", "-d", "{dir}/{name}"]

# Custom script
[rule.action]
type = "run"
command = "/home/user/scripts/process.sh"
args = ["{path}"]
```

### Archive

Create a zip archive of the file.

```toml
[rule.action]
type = "archive"
destination = "~/Archives"        # Optional - defaults to same directory
delete_original = false           # Delete source after archiving
```

### Nothing

Do nothing (useful for testing conditions).

```toml
[rule.action]
type = "nothing"
```

---

## Complete Rule Examples

### Organize Downloads

```toml
# ─── Images ───
[[rule]]
name = "Images to Pictures"
[rule.condition]
extensions = ["jpg", "jpeg", "png", "gif", "webp", "svg"]
[rule.action]
type = "move"
destination = "~/Pictures/Downloads"

# ─── Documents ───
[[rule]]
name = "PDFs to Documents"
[rule.condition]
extension = "pdf"
[rule.action]
type = "move"
destination = "~/Documents/PDFs"

[[rule]]
name = "Spreadsheets"
[rule.condition]
extensions = ["xlsx", "xls", "csv", "ods"]
[rule.action]
type = "move"
destination = "~/Documents/Spreadsheets"

# ─── Archives ───
[[rule]]
name = "Archives"
[rule.condition]
extensions = ["zip", "tar", "gz", "7z", "rar"]
[rule.action]
type = "move"
destination = "~/Downloads/Archives"

# ─── Videos ───
[[rule]]
name = "Videos"
[rule.condition]
extensions = ["mp4", "mkv", "avi", "mov", "webm"]
[rule.action]
type = "move"
destination = "~/Videos/Downloads"
```

### Clean Old Files

```toml
[[rule]]
name = "Delete old temp files"
[rule.condition]
extensions = ["tmp", "temp", "bak", "old"]
age_days_greater_than = 7
[rule.action]
type = "delete"

[[rule]]
name = "Trash old downloads"
[rule.condition]
age_days_greater_than = 60
[rule.action]
type = "trash"
```

### Process Screenshots

```toml
[[rule]]
name = "Move screenshots"
[rule.condition]
name_matches = "Screenshot*"
extensions = ["png", "jpg"]
[rule.action]
type = "move"
destination = "~/Pictures/Screenshots"
```

### Backup Important Files

```toml
[[rule]]
name = "Backup documents"
[rule.condition]
extensions = ["pdf", "docx", "xlsx"]
[rule.action]
type = "copy"
destination = "~/Backup/Documents"

[[rule]]
name = "Backup code"
[rule.condition]
extensions = ["py", "rs", "js", "ts", "go"]
[rule.action]
type = "copy"
destination = "~/Backup/Code"
```

### Rename with Dates

```toml
[[rule]]
name = "Date-prefix invoices"
[rule.condition]
name_regex = "^invoice.*\\.pdf$"
[rule.action]
type = "rename"
pattern = "{date:YYYY-MM-DD}_{filename}"

[[rule]]
name = "Rename photos with datetime"
[rule.condition]
extensions = ["jpg", "jpeg"]
name_regex = "^IMG_\\d+"
[rule.action]
type = "rename"
pattern = "photo_{datetime}.{ext}"
```

---

## Troubleshooting

### Validate Configuration

```bash
hazelnut check
# or
hazelnut check --config /path/to/config.toml
```

### Test Rules (Dry Run)

```bash
# Global dry run in config
[general]
dry_run = true

# Or via command line
hazelnut run           # Dry run by default
hazelnut run --apply   # Actually apply actions
```

### Debug Logging

```bash
# Via environment variable
HAZELNUT_LOG=debug hazelnut

# Or in config
[general]
log_level = "debug"
```

### Common Issues

**Rule not matching:**
- Check all conditions - they must ALL match
- Use `hazelnut run` to test rules in dry-run mode
- Enable debug logging to see what's happening

**Files not being watched:**
- Verify the watch path exists and is accessible
- Check if you need `recursive = true` for subdirectories
- Look at logs for watcher errors

**Permission errors:**
- Ensure Hazelnut has read/write access to source and destination
- Check if destination folder needs to be created

---

## Environment Variables

| Variable | Description |
|----------|-------------|
| `HAZELNUT_LOG` | Set log level (overrides config) |
| `HAZELNUT_CONFIG` | Default config file path |

```bash
HAZELNUT_LOG=debug hazelnut
HAZELNUT_CONFIG=/custom/path/config.toml hazelnutd run
```
