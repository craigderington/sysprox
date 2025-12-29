# Sysprox Installation Guide

## Installation via Cargo

### Quick Install from GitHub
```bash
cargo install --git https://github.com/yourusername/sysprox
```

### Installation Checklist

Before publishing, update `Cargo.toml`:
- [x] Update `repository` URL to actual GitHub repo
- [x] Update `authors` email if needed
- [x] Verify version number (currently 0.1.1)

## System Requirements

### Operating System
- **Linux only** (systemd-based distributions)
- Tested on: Arch Linux, Ubuntu 20.04+, Fedora 35+, Debian 11+

### Required Packages (Runtime)

#### Debian/Ubuntu:
```bash
sudo apt install systemd dbus
```

#### Fedora/RHEL/CentOS:
```bash
sudo dnf install systemd dbus
```

#### Arch Linux:
```bash
sudo pacman -S systemd dbus
```

**Note:** These are typically already installed on modern Linux systems.

### Build Requirements (for cargo install)

```bash
# Debian/Ubuntu
sudo apt install build-essential pkg-config libssl-dev

# Fedora/RHEL/CentOS
sudo dnf groupinstall "Development Tools"
sudo dnf install pkg-config openssl-devel

# Arch Linux
sudo pacman -S base-devel
```

## Runtime Dependencies

### Core Requirements
1. **systemd** (v230+, tested on v259)
   - Provides the service management daemon
   - Includes `systemctl` and `journalctl` commands

2. **D-Bus** (system bus)
   - Used for systemd communication
   - Usually included with systemd

### Optional
- **sudo** or **polkit** - For service control operations (start/stop/restart)
  - Read-only monitoring works without elevated privileges
  - Service control requires root or polkit authorization

## Verification

After installation, verify the setup:

```bash
# Check sysprox is installed
which sysprox
sysprox --version

# Verify systemd is accessible
systemctl --version

# Test D-Bus connection
systemctl list-units --type=service --state=running

# Check journalctl access
journalctl -u systemd-journald -n 5
```

## Binary Distribution

The compiled binary has minimal dependencies:
- ✅ **Statically linked** Rust code (zbus, tokio, etc.)
- ✅ Only requires standard **glibc** (present on all Linux systems)
- ✅ No external library dependencies beyond libc

```bash
$ ldd sysprox
    linux-vdso.so.1
    libgcc_s.so.1 => /usr/lib/libgcc_s.so.1
    libm.so.6 => /usr/lib/libm.so.6
    libc.so.6 => /usr/lib/libc.so.6
    /lib64/ld-linux-x86-64.so.2
```

This means you can distribute the binary directly to other Linux systems!

## Distribution Packages (Future)

Planned package formats:
- [ ] AUR (Arch User Repository)
- [ ] `.deb` (Debian/Ubuntu)
- [ ] `.rpm` (Fedora/RHEL)
- [ ] Homebrew (for Linux)

## Permissions

### Read-Only Mode (No Special Permissions)
```bash
# Anyone can run sysprox to monitor services
sysprox
```

### Service Control (Requires Root)
```bash
# Start/stop/restart services requires sudo
sudo sysprox
```

**Note:** Sysprox includes a secure password prompt dialog that will appear when privileged operations are attempted. You can also run the entire application with sudo to avoid repeated password prompts.

Or configure polkit for passwordless control (advanced).

## Troubleshooting

### "No such device or address"
- You're not running in a terminal (TTY required)
- Solution: Run from a real terminal, not via cron/systemd

### "Permission denied" when controlling services
- Service control requires root privileges
- Solution: Run with `sudo sysprox`

### "Failed to connect to D-Bus"
- D-Bus system bus not running
- Solution: `sudo systemctl start dbus`

### "systemd not found"
- Not a systemd-based distribution
- Solution: Sysprox only works on systemd-based Linux

## Building from Source

```bash
# Clone repository
git clone https://github.com/yourusername/sysprox
cd sysprox

# Build release binary
cargo build --release

# Install to ~/.cargo/bin
cargo install --path .

# Or install system-wide
sudo cp target/release/sysprox /usr/local/bin/
```

## Uninstallation

```bash
# If installed via cargo
cargo uninstall sysprox

# If copied manually
sudo rm /usr/local/bin/sysprox
```

---

**Status:** ✅ Fully functional v0.1.1
**License:** MIT
**Rust Version:** 1.70+
**Features:** Full TUI, service management, real-time logs, metrics, password prompts, auto-updates
