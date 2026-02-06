# AGENTS.md - Hazelnut Project

## Overview

**Hazelnut** 🌰 is a terminal-based automated file organizer inspired by [Hazel](https://www.noodlesoft.com/). It watches directories and automatically organizes files based on user-defined rules.

## Architecture

```
hazelnut/
├── src/
│   ├── main.rs          # TUI application entry point
│   ├── daemon.rs        # Background daemon entry point (hazelnutd)
│   ├── lib.rs           # Shared library code
│   ├── theme.rs         # Theme wrapper using ratatui-themes (15 themes)
│   ├── update.rs        # Update checking & self-update (crates.io API)
│   ├── app/             # TUI application logic
│   │   ├── mod.rs       # App initialization, background thread for updates
│   │   ├── state.rs     # Application state, daemon status detection
│   │   ├── ui.rs        # UI rendering (logo, tabs, views, popups)
│   │   └── events.rs    # Key event handling
│   ├── rules/           # Rule engine
│   │   ├── mod.rs       # Rule struct
│   │   ├── condition.rs # Rule conditions (name, type, date, size, etc.)
│   │   ├── action.rs    # Rule actions (move, rename, delete, etc.)
│   │   └── engine.rs    # Rule evaluation and execution
│   ├── watcher/         # File system watcher
│   │   ├── mod.rs       # Watcher implementation
│   │   └── handler.rs   # Event debouncing
│   ├── config/          # Configuration management
│   │   ├── mod.rs       # Config loading/saving
│   │   └── schema.rs    # Config file schema
│   └── ipc/             # Inter-process communication
│       └── mod.rs       # TUI <-> daemon protocol
├── docs/
│   └── configuration.md # Full config reference
├── Cargo.toml
├── README.md
├── AGENTS.md (this file)
└── CONTRIBUTING.md
```

## Key Features

### TUI (`hazelnut`)
- **Dashboard**: Logo, stats, quick actions
- **Rules view**: List, toggle enable/disable, create/edit/delete
- **Watches view**: List watched folders
- **Log view**: Activity history with timestamps
- **15 themes**: Powered by ratatui-themes (shared with Feedo)
- **Keybindings**: vim-style navigation (j/k), Tab to switch views, ? for help
- **Auto-update**: Background update check, one-key update via TUI or `hazelnut update` CLI
- **Daemon status**: Real-time daemon connection status in TUI

### Daemon (`hazelnutd`)
- Background file watching
- Rule execution on file changes
- PID/log files in `~/.local/state/hazelnut/`
- Signal handling (SIGHUP for reload, SIGTERM for stop)

### Rule Engine
**Conditions:**
- File extension (single or multiple)
- Name patterns (glob, regex)
- File size (greater/less than)
- File age (days old)
- Hidden files
- Directory check

**Actions:**
- Move to folder
- Copy to folder
- Rename with patterns ({name}, {date}, {ext})
- Trash (safe delete)
- Delete (permanent)
- Run shell command
- Archive (zip)

## Key Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| ratatui | 0.30 | TUI framework |
| ratatui-themes | 0.1 | Shared themes (15 themes) |
| crossterm | 0.29 | Terminal backend |
| tokio | 1.49 | Async runtime |
| notify | 9.0.0-rc.1 | Filesystem watcher |
| serde | 1.0 | Serialization |
| toml | 0.9 | Config format |
| clap | 4.5 | CLI parsing |
| chrono | 0.4 | Date/time handling |
| regex | 1.12 | Pattern matching |
| glob | 0.3 | Glob patterns |
| dirs | 6.0 | Home directory |

## Development Commands

```bash
# Run TUI in dev mode
cargo run

# Run TUI with custom config
cargo run -- --config path/to/config.toml

# Run daemon in foreground
cargo run --bin hazelnutd run

# Build release binaries
cargo build --release

# Run tests
cargo test

# Format code
cargo fmt

# Lint
cargo clippy

# List rules from CLI
cargo run -- list

# Dry-run rules on a directory
cargo run -- run --dir ~/Downloads

# Apply rules (no dry-run)
cargo run -- run --dir ~/Downloads --apply

# Check for updates
cargo run -- update
```

## Configuration

Default config: `~/.config/hazelnut/config.toml` (same path on all platforms)

```toml
[general]
log_level = "info"
dry_run = false
theme = "dracula"

[[watch]]
path = "~/Downloads"
recursive = false

[[rule]]
name = "PDFs to Documents"
enabled = true

[rule.condition]
extension = "pdf"

[rule.action]
type = "move"
destination = "~/Documents/PDFs"
```

## Cross-Platform Paths

Hazelnut uses consistent paths on all platforms:
- Config: `~/.config/hazelnut/config.toml`
- State (PID, logs): `~/.local/state/hazelnut/`

This avoids macOS-specific paths like `~/Library/Application Support/`.

## Current Status

✅ **Working:**
- Full TUI with 15 beautiful themes
- Config loading and parsing
- Rule engine with conditions and actions
- File watcher infrastructure
- CLI commands (list, check, run, update)
- Visual rule editor in TUI
- Auto-update with crates.io API
- Daemon status detection

🚧 **In Progress:**
- IPC between TUI and daemon

📋 **Planned:**
- Hot config reload
- Undo support
- Desktop notifications
- Rule templates
- Import from Hazel

## Themes

Press `t` in the TUI to open theme picker (15 themes from ratatui-themes):
- Catppuccin Mocha, Latte, Frappé, Macchiato
- Dracula
- Nord
- Gruvbox Dark/Light
- Tokyo Night
- Monokai Pro
- Solarized Dark/Light
- One Dark
- Everforest
- Rosé Pine

## Keybindings

| Key | Action |
|-----|--------|
| Tab / Shift+Tab | Switch views |
| 1-4 | Jump to view |
| j/k or ↑/↓ | Navigate |
| g/G | First/last item |
| Enter/Space | Toggle rule |
| n | New rule |
| e | Edit rule |
| d | Delete rule |
| D | Toggle daemon |
| t | Theme picker |
| s | Settings |
| U | Update (if available) |
| ? | Show help |
| A | About |
| q / Ctrl+c | Quit |

## Website

https://hazelnut.ricardodantas.me

## Related Projects

- **Feedo** — Terminal RSS reader (same author, shared themes)
- **ratatui-themes** — Shared theme library

## Binary Locations

After `cargo build --release`:
- TUI: `target/release/hazelnut`
- Daemon: `target/release/hazelnutd`
