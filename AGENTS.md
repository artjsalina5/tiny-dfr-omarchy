# AGENTS.md - tiny-dfr (Omarchy Fork)

Touch Bar daemon for Apple T2 and Silicon Macs. Rust project with Hyprland integration and suspend/resume support.

## Build Commands

### Standard Build
```bash
cargo build                    # Debug build
cargo build --release          # Release build (optimized)
```

### Development
```bash
cargo check                    # Fast compile check without binary
cargo clippy                   # Lint (expect warnings on current code)
cargo fmt                      # Format code with rustfmt
cargo run                      # Build and run (requires root/Touch Bar hardware)
```

### Testing
```bash
cargo test                     # Run all tests (currently no test suite)
cargo test <name>              # Run specific test by name
cargo test -- --nocapture      # Show stdout during tests
```

**Note**: This project has no tests currently. Tests would require mocking DRM/input devices.

### Installation
```bash
./install-tiny-dfr.sh          # Build, install binary, and setup systemd services
```

## Project Structure

```
src/
├── main.rs                    # Entry point, event loop, rendering
├── config.rs                  # TOML config loading, hot-reload with inotify
├── display.rs                 # DRM framebuffer management
├── hyprland.rs                # Hyprland IPC socket integration
├── backlight.rs               # Touch Bar brightness control
├── keyboard_backlight.rs      # Keyboard backlight control
├── battery_monitor.rs         # Battery state monitoring
├── icon_cache.rs              # Icon lookup/caching (freedesktop)
├── fonts.rs                   # FontConfig FFI bindings
├── pixel_shift.rs             # Anti-burn-in pixel shifting
├── system_monitor.rs          # CPU/memory monitoring
└── user_cache.rs              # User environment detection

share/tiny-dfr/                # Default configs (TOML)
bin/                           # Helper scripts
etc/                           # Systemd service files
```

## Code Style Guidelines

### Imports
- Group imports: `std` → external crates → local modules
- Use explicit imports for clarity: `use std::sync::{Arc, Mutex};`
- Alphabetize within groups when practical
- Use module-level imports over inline `use` statements

```rust
use anyhow::{anyhow, Result};
use cairo::{Context, ImageSurface};
use std::collections::HashMap;
use std::fs::File;

mod backlight;
mod config;
use crate::config::Config;
```

### Formatting
- **rustfmt**: Run `cargo fmt` before commits (project uses standard rustfmt)
- **Line length**: 100 chars (rustfmt default)
- **Indentation**: 4 spaces
- **Trailing commas**: Yes in multi-line collections

### Types and Naming
- **Structs**: `PascalCase` (e.g., `NavigationState`, `DrmBackend`)
- **Functions/variables**: `snake_case` (e.g., `find_backlight`, `last_interaction_time`)
- **Constants**: `SCREAMING_SNAKE_CASE` (e.g., `BUTTON_SPACING_PX`, `ICON_SIZE`)
- **Modules**: `snake_case` (e.g., `battery_monitor.rs`)
- Type annotations: Explicit on struct fields, often inferred in functions
- Use newtype patterns for clarity when wrapping FFI types

```rust
const TIMEOUT_MS: i32 = 10 * 1000;
struct Card(File);  // Newtype for DRM card
```

### Error Handling
- Use `anyhow::Result<T>` for functions that can fail
- Use `anyhow!("message")` for custom errors
- Propagate errors with `?` operator
- Use `.unwrap()` only when impossible to fail (document why)
- Log errors to stderr with `eprintln!()` before recovery/retry

```rust
use anyhow::{anyhow, Result};

fn try_open_card(path: &Path) -> Result<DrmBackend> {
    let card = Card::open(path)?;
    card.set_client_capability(ClientCapability::Atomic, true)?;
    // ...
    Ok(backend)
}
```

### Concurrency and State
- Use `std::sync::LazyLock<Arc<Mutex<T>>>` for global state
- Wrap shared state with `Arc<Mutex<>>` for thread-safe access
- Spawn threads with `std::thread::spawn` for background tasks
- Always document thread ownership and lifetimes

```rust
static BATTERY_STATE: std::sync::LazyLock<Arc<Mutex<Option<BatteryInfo>>>> =
    std::sync::LazyLock::new(|| Arc::new(Mutex::new(None)));
```

### Comments and Documentation
- Use `//` for inline comments
- Document complex algorithms and hardware interactions
- Add comments explaining "why" not "what"
- No rustdoc comments (`///`) on private items unless complex

### Serde and Config
- Use `#[derive(Deserialize, Debug, Clone)]` for config structs
- Custom deserializers for enums with string prefixes (e.g., `ButtonAction`)
- Use `#[serde(rename = "...")]` for JSON/TOML field name mapping
- TOML files: snake_case keys, cascading config (share → /etc → user)

### Platform-Specific
- This is Linux-only (T2 MacBooks with Asahi Linux/similar)
- Requires root for `/dev/dri/card*` and `/dev/input/event*` access
- Uses `privdrop` crate to drop privileges after device access
- FFI bindings: use `#[repr(C)]` and document C API source

### Common Patterns
- **Retry loops**: Used for device initialization (backlight, DRM)
- **Epoll event handling**: Main event loop waits on multiple file descriptors
- **Cairo rendering**: All UI drawn with cairo-rs on DRM framebuffers
- **Inotify**: Watch config files for hot-reload without restart

## Key Dependencies
- `cairo-rs`: UI rendering
- `drm`: Direct Rendering Manager (Linux DRM/KMS API)
- `input`, `input-linux`: libinput and evdev for Touch Bar input
- `nix`: Unix system calls (epoll, signals, inotify)
- `serde`, `toml`: Config deserialization
- `udev`: Device hotplug detection
- `anyhow`: Error handling

## Common Tasks

### Adding a New Config Option
1. Add field to config struct in `src/config.rs` with `#[derive(Deserialize)]`
2. Update default TOML in `share/tiny-dfr/config.toml`
3. Use `config_manager.get_config()` to access in main loop
4. Config auto-reloads via inotify (no restart needed)

### Adding a New Button Action
1. Add variant to `ButtonAction` enum in `src/config.rs`
2. Implement deserialization logic if custom format needed
3. Handle action in `handle_button_tap()` in `src/main.rs`
4. Document in `share/tiny-dfr/expandables.toml` examples

### Debugging Touch Bar Issues
- Check DRM device: `ls -la /dev/dri/card*` (usually card1 for Touch Bar)
- Check input events: `sudo evtest` to see Touch Bar input device
- Enable verbose logs: add `eprintln!()` statements in event loop
- Test rendering: run with `sudo` to access devices

## Notes for AI Coding Agents

- **No tests**: Add unit tests for pure functions (config parsing, color conversion)
- **Hardware dependency**: Most code requires Touch Bar hardware; mock DRM/input for testing
- **Root required**: Binary must run as root; use VMs or real hardware for testing
- **Clippy warnings exist**: Current code has some clippy warnings; fix them when touching files
- **Formatting**: Some files need `cargo fmt` - always format before committing
- **Error handling**: Prefer explicit error messages over generic panics
- **Performance**: Main loop is latency-sensitive (touch input); avoid blocking operations
- **Config reload**: Use `ConfigManager::check_for_updates()` pattern for hot-reloadable state
