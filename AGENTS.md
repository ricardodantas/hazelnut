# AGENTS.md - Hazelnut Project

## Overview

**Hazelnut** 🧹 is a terminal-based automated file organizer inspired by [Hazel](https://www.noodlesoft.com/). It watches directories and automatically organizes files based on user-defined rules.

## Architecture

```
hazelnut/
├── src/
│   ├── main.rs          # TUI application entry point
│   ├── daemon.rs        # Background daemon entry point (hazelnutd)
│   ├── lib.rs           # Shared library code
│   ├── theme.rs         # 8 beautiful color themes
│   ├── app/             # TUI application logic
│   │   ├── mod.rs       # App initialization
│   │   ├── state.rs     # Application state
│   │   ├── ui.rs        # UI rendering (logo, tabs, views)
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
- **Rules view**: List, toggle enable/disable
- **Watches view**: List watched folders
- **Log view**: Activity history with timestamps
- **8 themes**: Catppuccin, Dracula, Nord, Gruvbox, Tokyo Night, Monokai, Ocean, Sunset
- **Keybindings**: vim-style navigation (j/k), Tab to switch views, ? for help

### Daemon (`hazelnutd`)
- Background file watching
- Rule execution on file changes
- IPC communication with TUI (planned)

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
| crossterm | 0.29 | Terminal backend |
| tokio | 1.49 | Async runtime |
| notify | 9.0.0-rc.1 | Filesystem watcher |
| serde | 1.0 | Serialization |
| toml | 0.9 | Config format |
| clap | 4.5 | CLI parsing |
| chrono | 0.4 | Date/time handling |
| regex | 1.12 | Pattern matching |
| glob | 0.3 | Glob patterns |
| dirs | 6.0 | XDG directories |

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
```

## Configuration

Default config: `~/.config/hazelnut/config.toml`

```toml
[general]
log_level = "info"
dry_run = false

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

## Current Status

✅ **Working:**
- Full TUI with beautiful themes
- Config loading and parsing
- Rule engine with conditions and actions
- File watcher infrastructure
- CLI commands (list, check, run)

🚧 **In Progress:**
- Rule editor in TUI
- Daemon service management
- IPC between TUI and daemon

📋 **Planned:**
- Hot config reload
- Undo support
- Desktop notifications
- Rule templates
- Import from Hazel

## Theme Cycling

Press `Ctrl+t` in the TUI to cycle through themes:
1. Catppuccin Mocha (default) - Warm and cozy
2. Dracula - Dark and vibrant
3. Nord - Cool and calm
4. Gruvbox Dark - Retro warm
5. Tokyo Night - Modern dark
6. Monokai Pro - Classic dark
7. Ocean Deep - Cool blue depths
8. Sunset Glow - Warm twilight

## Keybindings

| Key | Action |
|-----|--------|
| Tab / Shift+Tab | Switch views |
| 1-4 | Jump to view |
| j/k or ↑/↓ | Navigate |
| g/G | First/last item |
| Enter/Space | Toggle rule |
| Ctrl+t | Cycle theme |
| ? | Show help |
| q / Ctrl+c | Quit |

## Binary Locations

After `cargo build --release`:
- TUI: `target/release/hazelnut` (3.4 MB)
- Daemon: `target/release/hazelnutd` (2.5 MB)
