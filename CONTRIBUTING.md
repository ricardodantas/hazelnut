# Contributing to Hazelnut

Thank you for considering contributing to Hazelnut! 🧹

## Getting Started

1. Fork the repository
2. Clone your fork: `git clone https://github.com/YOUR_USERNAME/hazelnut`
3. Create a branch: `git checkout -b feature/your-feature`
4. Make your changes
5. Run tests: `cargo test`
6. Run clippy: `cargo clippy`
7. Format code: `cargo fmt`
8. Commit: `git commit -m "Add your feature"`
9. Push: `git push origin feature/your-feature`
10. Open a Pull Request

## Development Setup

```bash
# Clone
git clone https://github.com/ricardodantas/hazelnut
cd hazelnut

# Build
cargo build

# Run TUI
cargo run

# Run daemon
cargo run --bin hazelnutd run

# Run with sample config
cargo run -- --config examples/config.toml
```

## Code Style

- Use `cargo fmt` before committing
- Use `cargo clippy` and address all warnings
- Write tests for new functionality
- Add documentation for public APIs
- Keep commits focused and atomic

## Project Structure

```
src/
├── main.rs        # TUI binary entry point
├── daemon.rs      # Daemon binary entry point
├── lib.rs         # Library root
├── app/           # TUI application
│   ├── events.rs  # Key event handling
│   ├── state.rs   # Application state
│   └── ui.rs      # UI rendering
├── config/        # Configuration loading
├── rules/         # Rule engine
│   ├── action.rs  # Rule actions
│   ├── condition.rs # Rule conditions
│   └── engine.rs  # Rule evaluation
├── theme.rs       # Color themes
├── watcher/       # File system watcher
└── ipc/           # TUI-daemon communication
```

## Adding a New Theme

1. Add a variant to `Theme` enum in `src/theme.rs`
2. Add a `ThemeColors::your_theme()` implementation
3. Update `Theme::colors()` match
4. Update `Theme::name()` match
5. Add to `Theme::all()` array

## Adding a New Condition

1. Add field to `Condition` struct in `src/rules/condition.rs`
2. Update `Condition::matches()` to check your condition
3. Add tests
4. Update docs/configuration.md

## Adding a New Action

1. Add variant to `Action` enum in `src/rules/action.rs`
2. Implement the action in `Action::execute()`
3. Add tests
4. Update docs/configuration.md

## Questions?

Open an issue or start a discussion!
