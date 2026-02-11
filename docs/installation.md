# Installation

Hazelnut can be installed on macOS, Linux, and Windows.

## Quick Install

| Platform | Method | Command |
|----------|--------|---------|
| **macOS** | Homebrew | `brew install ricardodantas/tap/hazelnut` |
| **Linux** | Homebrew | `brew install ricardodantas/tap/hazelnut` |
| **Linux** | Arch Linux (pacman) | `pacman -S hazelnut` |
| **Linux** | Cargo | `cargo install hazelnut` |
| **Windows** | Cargo | `cargo install hazelnut` |
| **Windows** | Binary | [Download from Releases](https://github.com/ricardodantas/hazelnut/releases) |

## macOS

### Homebrew (Recommended)

The fastest way to install Hazelnut on macOS. Uses pre-built binaries — no compilation required.

```bash
brew install ricardodantas/tap/hazelnut
```

This installs both:
- `hazelnut` — the TUI (terminal user interface)
- `hazelnutd` — the background daemon

### Cargo

If you have the Rust toolchain installed:

```bash
cargo install hazelnut
```

> **Note**: This compiles from source and takes a few minutes.

## Linux

### Arch Linux (pacman)

```bash
pacman -S hazelnut
```

### Homebrew (Recommended)

Homebrew works on Linux too! Uses pre-built binaries for fast installation.

```bash
# Install Homebrew for Linux if you don't have it
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"

# Install Hazelnut
brew install ricardodantas/tap/hazelnut
```

Supported architectures:
- x86_64 (Intel/AMD)
- aarch64 (ARM64)

### Cargo

If you prefer using Cargo:

```bash
cargo install hazelnut
```

### Manual Download

Download the appropriate binary from [GitHub Releases](https://github.com/ricardodantas/hazelnut/releases):

| Architecture | File |
|--------------|------|
| x86_64 (glibc) | `hazelnut-*-x86_64-unknown-linux-gnu.tar.gz` |
| x86_64 (musl) | `hazelnut-*-x86_64-unknown-linux-musl.tar.gz` |
| ARM64 | `hazelnut-*-aarch64-unknown-linux-gnu.tar.gz` |

Extract and install:

```bash
tar -xzf hazelnut-*.tar.gz
sudo mv hazelnut hazelnutd /usr/local/bin/
```

## Windows

### Cargo (Recommended)

If you have the Rust toolchain installed:

```bash
cargo install hazelnut
```

### Manual Download

Download `hazelnut-*-x86_64-pc-windows-msvc.zip` from [GitHub Releases](https://github.com/ricardodantas/hazelnut/releases).

Extract and add the directory to your PATH.

> **Note**: The daemon (`hazelnutd`) is not available on Windows. Only the TUI (`hazelnut`) works on Windows.

## Building from Source

### Requirements

- **Rust 1.93+** (uses Edition 2024 features)
- Git

### Steps

```bash
# Clone the repository
git clone https://github.com/ricardodantas/hazelnut
cd hazelnut

# Build release binaries
cargo build --release

# The binaries will be at:
# - target/release/hazelnut
# - target/release/hazelnutd (Unix only)

# Or install directly to ~/.cargo/bin
cargo install --path .
```

## Verify Installation

```bash
# Check version
hazelnut --version

# Check daemon (Unix only)
hazelnutd --version
```

## Updating

### Homebrew

```bash
brew upgrade ricardodantas/tap/hazelnut
```

### Cargo

```bash
cargo install hazelnut
```

### TUI

Press `U` in the Hazelnut TUI when an update is available. Hazelnut will detect whether it was installed via Homebrew or Cargo and run the appropriate update command.

## Uninstalling

### Homebrew

```bash
brew uninstall hazelnut
```

### Cargo

```bash
cargo uninstall hazelnut
```

### Manual

Remove the binaries from your PATH:

```bash
rm /usr/local/bin/hazelnut /usr/local/bin/hazelnutd
```

Also remove config and data:

```bash
rm -rf ~/.config/hazelnut
rm -rf ~/.local/state/hazelnut
```
