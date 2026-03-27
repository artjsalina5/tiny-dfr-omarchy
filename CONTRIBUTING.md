# Contributing to tiny-dfr-omarchy

Thank you for your interest in contributing to tiny-dfr-omarchy! This document provides guidelines and information for contributors.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Development Setup](#development-setup)
- [Code Style](#code-style)
- [Making Changes](#making-changes)
- [Testing](#testing)
- [Submitting Changes](#submitting-changes)
- [Hardware Requirements](#hardware-requirements)
- [Project Resources](#project-resources)

## Code of Conduct

Be respectful and constructive. This project welcomes contributions from everyone who shares the goal of improving Touch Bar support on Linux.

## Getting Started

### Prerequisites

- Rust toolchain (stable channel)
- Apple MacBook with Touch Bar (T2 or Silicon) for testing
- Linux distribution with systemd
- Required kernel modules: `apple-bce`, `hid-appletb-kbd`, `hid-appletb-bl`, `appletbdrm`

### Areas for Contribution

We welcome contributions in these areas:

1. **Testing**: Report issues on different T2 Mac models
2. **Configuration**: Add application configs for Hyprland integration
3. **Icons**: Improve icon sets and theme support
4. **Documentation**: Improve guides, troubleshooting, translations
5. **Bug fixes**: Address issues in suspend/resume, rendering, input handling
6. **Features**: New button actions, expandable menu types, integrations

## Development Setup

### Clone the Repository

```bash
git clone https://github.com/artjsalina5/tiny-dfr-omarchy.git
cd tiny-dfr-omarchy
```

### Install Dependencies

**Arch Linux / Omarchy:**
```bash
sudo pacman -S rust cargo cairo libinput freetype2 fontconfig librsvg
```

**Ubuntu / Debian:**
```bash
sudo apt install cargo libcairo2-dev libinput-dev libfreetype6-dev \
                 libfontconfig1-dev librsvg2-dev pkg-config
```

**Fedora:**
```bash
sudo dnf install cargo cairo-devel libinput-devel freetype-devel \
                 fontconfig-devel librsvg2-devel
```

### Build the Project

```bash
cargo build                  # Debug build
cargo build --release        # Release build (optimized)
```

### Run the Daemon

**Important**: The daemon requires root access to `/dev/dri/card*` and `/dev/input/event*` devices.

```bash
sudo ./target/release/tiny-dfr
```

Or use cargo:

```bash
sudo cargo run
```

## Code Style

### Follow Rust Standards

This project follows standard Rust conventions. See [AGENTS.md](AGENTS.md) for detailed code style guidelines.

### Key Points

- **Formatting**: Run `cargo fmt` before committing
- **Linting**: Run `cargo clippy` and address warnings when practical
- **Imports**: Group as `std` → external crates → local modules
- **Naming**: 
  - Types: `PascalCase`
  - Functions/variables: `snake_case`
  - Constants: `SCREAMING_SNAKE_CASE`
- **Error handling**: Use `anyhow::Result<T>` for functions that can fail
- **Comments**: Explain "why" not "what", document hardware interactions

### Example

```rust
use anyhow::{anyhow, Result};
use std::fs::File;
use std::path::Path;

use crate::display::DrmBackend;

const TIMEOUT_MS: i32 = 10 * 1000;

fn open_device(path: &Path) -> Result<DrmBackend> {
    let card = File::open(path)?;
    // Initialize DRM backend with atomic modesetting
    DrmBackend::new(card)
}
```

### Before Committing

```bash
cargo fmt                    # Format code
cargo clippy                 # Check for common issues
cargo build                  # Ensure it compiles
```

## Making Changes

### Branch Naming

Use descriptive branch names:

- `feat/add-battery-widget` - New features
- `fix/suspend-race-condition` - Bug fixes  
- `docs/update-readme` - Documentation
- `refactor/cleanup-config` - Code improvements

### Commit Messages

Follow conventional commit format:

```
<type>: <short description>

<optional longer description>

<optional footer>
```

**Types:**
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `refactor`: Code restructuring without behavior change
- `perf`: Performance improvement
- `test`: Adding tests
- `chore`: Maintenance tasks

**Examples:**

```
feat: add battery percentage widget to status bar

Implements real-time battery monitoring with background thread.
Updates every 30 seconds to avoid excessive I/O.

Closes #42
```

```
fix: resolve digitizer timeout after resume

Increases BCE reset timeout from 800ms to 1.2s to allow full
hardware initialization. Adds graceful fallback if reset fails.
```

### Configuration Changes

When adding new config options:

1. Add field to struct in `src/config.rs` with `#[derive(Deserialize)]`
2. Update default config in `share/tiny-dfr/config.toml`
3. Document in README.md and add example usage
4. Config automatically reloads via inotify (no daemon restart needed)

### Adding Button Actions

To add new button action types:

1. Add variant to `ButtonAction` enum in `src/config.rs`
2. Implement deserialization if needed
3. Handle action in `handle_button_tap()` in `src/main.rs`
4. Document in `share/tiny-dfr/expandables.toml` with example
5. Add to README.md configuration section

## Testing

### Current State

This project currently has no automated test suite due to hardware dependencies (DRM devices, Touch Bar input devices).

### Manual Testing

Test on real hardware with these scenarios:

1. **Basic functionality**:
   - Touch Bar displays and responds to input
   - Buttons trigger correct actions
   - Expandable menus navigate correctly

2. **Suspend/resume**:
   - System suspends cleanly
   - Touch Bar recovers after wake (~10 seconds)
   - Touch input works after resume
   - Check `journalctl -u tiny-dfr-omarchy.service` for errors

3. **Configuration reload**:
   - Edit `/etc/tiny-dfr/config.toml`
   - Changes apply without restart
   - Check logs for reload confirmation

4. **Hyprland integration** (if using Hyprland):
   - Button layout switches based on active window
   - Hyprland restart reconnects automatically

### Test Checklist for PRs

- [ ] Code compiles without errors (`cargo build`)
- [ ] Code formatted (`cargo fmt`)
- [ ] Clippy checks pass or issues documented (`cargo clippy`)
- [ ] Tested on real hardware (list Mac model)
- [ ] Suspend/resume cycle tested if touching related code
- [ ] Configuration changes documented
- [ ] No sensitive information (PII) in commits

## Submitting Changes

### Pull Request Process

1. **Fork the repository** and create your branch
2. **Make your changes** following code style guidelines
3. **Test thoroughly** on real hardware
4. **Update documentation** if needed (README.md, config files)
5. **Commit** with clear, descriptive messages
6. **Push** to your fork
7. **Open a Pull Request** with:
   - Clear description of changes
   - Reasoning/motivation
   - Testing performed (hardware model, scenarios tested)
   - Related issue numbers (if applicable)

### PR Description Template

```markdown
## Summary
Brief description of what this PR does.

## Motivation
Why is this change needed? What problem does it solve?

## Changes
- Bullet list of specific changes
- Include any breaking changes

## Testing
- Hardware: MacBook Pro 16,1 (2019)
- Tested scenarios:
  - Boot and initialization
  - Suspend/resume cycle
  - Configuration reload

## Related Issues
Fixes #123
Relates to #456
```

### Review Process

- Maintainer will review code for correctness, style, and documentation
- Feedback will be provided via PR comments
- Make requested changes and push updates
- Once approved, PR will be merged

## Hardware Requirements

### Development Hardware

You need an Apple MacBook with Touch Bar for development and testing:

- **T2 MacBooks**: MacBook Pro 2016-2020 with Touch Bar
- **Apple Silicon**: M1/M2 MacBooks (if driver support available)

### Testing Without Hardware

If you don't have Touch Bar hardware:

- Documentation improvements
- Code review
- Configuration file examples
- Issue triage and user support

## Project Resources

### Documentation

- **[AGENTS.md](AGENTS.md)**: Detailed code style and architecture for AI agents
- **[SKILL.md](SKILL.md)**: Domain-specific development workflows
- **[README.md](README.md)**: User-facing documentation

### External Resources

- **Upstream project**: [AsahiLinux/tiny-dfr](https://github.com/AsahiLinux/tiny-dfr)
- **Apple BCE driver**: [t2linux/apple-bce-drv](https://github.com/t2linux/apple-bce-drv)
- **T2Linux Wiki**: https://wiki.t2linux.org/
- **T2Linux Discord**: Community support and discussion

### Getting Help

- **GitHub Issues**: Bug reports and feature requests
- **T2Linux Discord**: Real-time community support
- **T2Linux Wiki**: Hardware documentation and guides

## Questions?

If you have questions about contributing, please:

1. Check existing issues and documentation
2. Ask in GitHub Discussions or Issues
3. Join the T2Linux Discord community

Thank you for contributing to tiny-dfr-omarchy!
