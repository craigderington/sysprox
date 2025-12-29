# Sysprox

A beautiful Terminal User Interface (TUI) for monitoring and controlling systemd services on Linux.

## Overview

Sysprox provides a gorgeous, interactive interface to systemd - making it easy to monitor service status, view logs, track resource usage, and control services without memorizing complex systemctl commands.

**Status:** ✅ **Rust Port Complete!** Phase 1-6 Complete! 🎉

## Features

### ✅ Completed (Phase 1 - MVP)
- [x] Service discovery and listing via D-Bus
- [x] Real-time service status monitoring (auto-refresh every 5s)
- [x] Interactive TUI dashboard with service table
- [x] Filter services by state (all, running, stopped, failed)
- [x] **NEW: Instant search** - Type `/` to search services by name or description
- [x] Colored status icons (● active, ○ inactive, ✗ failed) with emojis
- [x] Error handling and loading states
- [x] Keyboard navigation (vim-style + arrow keys)

### ✅ Completed (Phase 2)
- [x] Service detail view with full information
  - Status overview (load state, PID, restarts)
  - Resource usage metrics (memory, tasks, CPU time)
  - Dependencies preview (wants, after)
  - Beautiful boxed layout with colors
- [x] **Real-time log streaming** with journalctl integration
  - Follow mode with auto-scroll
  - Pause/resume streaming
  - Scroll through logs
  - Clear logs buffer
  - Jump to top/bottom
  - Beautiful bordered viewport with emojis

### 🚧 In Progress (Phase 3)
- [ ] Service control (start/stop/restart with confirmations)
- [ ] Live CPU/memory graphs in detail view
- [ ] Real-time metric updates

### Planned
- **Phase 2:** Real-time updates via WebSocket, animations
- **Phase 3:** Resource visualization (CPU, memory, network)
- **Phase 4:** Historical data and time-travel debugging
- **Phase 5:** Multiple visualization modes, custom groups

See [CLAUDE.md](CLAUDE.md) for the complete roadmap.

## Requirements

- Linux with systemd
- Rust 1.70 or later (for building from source)

## Installation

### From Source

```bash
git clone https://github.com/craigderington/sysprox.git
cd sysprox
cargo build --release
sudo install target/release/sysprox /usr/local/bin/
```

Or use the Makefile:

```bash
make install
```

## Usage

Simply run:

```bash
sysprox
```

### Keyboard Shortcuts

**Navigation:**
- `j/k` or `↓/↑`: Move up/down in list
- `q` or `Ctrl+C`: Quit

**Filters:**
- `a`: Show all services
- `r`: Show running services only
- `s`: Show stopped services only
- `f`: Show failed services only

## Development

### Project Structure

```
sysprox/
├── Cargo.toml           # Rust package manifest
├── src/
│   ├── main.rs          # Entry point with clap CLI
│   ├── lib.rs           # Module exports
│   ├── app.rs           # Main application state
│   ├── events.rs        # Event handling
│   ├── error.rs         # Error types
│   ├── config.rs        # Configuration
│   ├── systemd/         # Systemd integration
│   │   ├── client.rs    # Zbus D-Bus client
│   │   ├── models.rs    # Data structures
│   │   ├── journal.rs   # Log streaming
│   │   ├── control.rs   # Service control
│   │   └── metrics.rs   # Metrics collection
│   └── ui/              # TUI components
│       ├── dashboard.rs # Service list
│       ├── detail.rs    # Detail view
│       ├── logs.rs      # Log viewer
│       └── styles.rs    # Color palette
├── go-reference/        # Original Go implementation
└── CLAUDE.md            # Project vision & architecture
```

### Building

```bash
# Debug build
cargo build

# Release build (optimized)
cargo build --release

# Or use Makefile
make build
make release
```

### Running

```bash
# Run directly
cargo run --release

# Or run the binary
./target/release/sysprox
```

## Technology Stack

- **Language:** Rust
- **TUI Framework:** [ratatui](https://github.com/ratatui-org/ratatui) + [crossterm](https://github.com/crossterm-rs/crossterm)
- **Async Runtime:** [Tokio](https://tokio.rs)
- **Systemd Integration:** [zbus](https://gitlab.freedesktop.org/dbus/zbus) (async D-Bus)
- **CLI Framework:** [clap](https://github.com/clap-rs/clap)
- **Config:** [serde](https://serde.rs) + [serde_yaml](https://github.com/dtolnay/serde-yaml)
- **Error Handling:** [anyhow](https://github.com/dtolnay/anyhow) + [thiserror](https://github.com/dtolnay/thiserror)

**Original Go Implementation:** Available in `go-reference/` directory

## Philosophy

**Observer and Controller, Not Owner**

Unlike process managers that spawn and own processes, Sysprox is designed to observe and control existing systemd services. It provides a delightful interface to the services already running on your Linux system.

## Comparison

- **vs systemctl:** Visual, real-time interface with multiple services at once
- **vs systemd-manager (GUI):** Runs in terminal, SSH-friendly, keyboard-first
- **vs htop/top:** Service-level view with systemd integration

## License

MIT

## Author

Craig Derington ([@craigderington](https://github.com/craigderington))

## Acknowledgments

Inspired by the need for better systemd monitoring tools and built with the excellent Charm TUI libraries.
