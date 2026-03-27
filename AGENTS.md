# AGENTS.md - tiny-dfr-omarchy

Touch Bar daemon for Apple T2 and Silicon Macs. Rust project with Hyprland integration and reliable suspend/resume support.

## Recent Developments (March 2026)

### Suspend/Resume Reliability Overhaul

- Implemented automatic digitizer recovery with BCE reset fallback (~1.2s timer)
- Made BCE waits non-fatal: PCI link readiness (4s) and DMA verification (3s) with graceful degradation
- Removed premature backlight-zero touch event suppression gate
- Added service-level startup hardening: `ExecStartPre` udev settle (4s) + device readiness waits
- Fixed libinput seat-binding race by ensuring udev finishes device registration before daemon initialization
- Enforced single primary service via `Conflicts=` directive across all three unit names
- Resume timing stabilized to ~10 seconds (BCE reload 3s + PCI retraining 4s + Touch Bar reset 3s + udev/libinput rebinding)

### BCE Kernel-Userspace Messaging

- Mailbox interface: Direct commands (`VHCI_CMD_T2_PAUSE`, `VHCI_CMD_T2_RESUME`, `VHCI_CMD_RESET_TOUCHBAR`)
- VHCI interface: Virtual USB host controller for graceful device state management
- Both paths implement coordinated barriers (PCI link stabilization, DMA status verification)
- Non-fatal fallback: vhci interface lacks userspace reset, gracefully returns Ok(())
- See [apple-bce-drv PR#7](https://github.com/t2linux/apple-bce-drv/pull/7) for kernel-side reliability improvements

### iGPU Configuration Requirement

- **Must apply hypridle.conf fix** on systems with integrated GPU
- DPMS coordination ensures display wakes properly post-suspend
- Without fix: screen remains unresponsive with backlight on
- Configuration: `after_sleep_cmd = bash -lc 'sleep 1; hyprctl dispatch dpms off; sleep 1; hyprctl dispatch dpms on'`

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

## Debugging Touch Bar Issues

- Check DRM device: `ls -la /dev/dri/card*` (usually card1 for Touch Bar)
- Check input events: `sudo evtest` to see Touch Bar input device
- Enable verbose logs: add `eprintln!()` statements in event loop
- Test rendering: run with `sudo` to access devices

## Suspend/Resume Architecture

### Service Coordination Flow

- **suspend-fix-t2.service**: Modprobe sequencing + DMA/PCI link coordination
  - ExecStart: Save backlight, stop daemons, unload Touch Bar modules, power down BCE
  - ExecStop: Reload BCE, touch modules, wait for device readiness
  - ExecStopPost: udev settle (3s) + reset failed state + restart tiny-dfr-omarchy
- **tiny-dfr-omarchy.service** (primary): Startup hardening with device readiness gates
  - ExecStartPre: udevadm settle (4s), wait-for-device touchbar-minimal & hid-kbd (4s each)
  - RestartSec=1 for quick recovery if needed
  - Conflicts= guards prevent concurrent instances of any three daemon units
- **tiny-dfr.service** & **omarchy-dynamic-function-row-daemon.service**: Legacy aliases with identical startup sequence

### Digitizer Recovery Logic

- Proactive monitoring: 1.2s timer triggers BCE reset if digitizer absent while device nodes exist
- Post-resume fallback: If Touch Bar device missing after 1.2s, attempt BCE reset + forced runtime reinit
- Immediate dispatch: Schedule libinput event processing after recovery to absorb device-add events
- Graceful degradation: BCE reset failures don't block main loop (non-fatal errors)

## Notes for AI Coding Agents

- **No tests**: Add unit tests for pure functions (config parsing, color conversion)
- **Hardware dependency**: Most code requires Touch Bar hardware; mock DRM/input for testing
- **Root required**: Binary must run as root; use VMs or real hardware for testing
- **Formatting**: Always run `cargo fmt` before committing (all source files now properly formatted)
- **Error handling**: Prefer explicit error messages over generic panics; use `anyhow::Result<T>`
- **Performance**: Main loop is latency-sensitive (touch input); avoid blocking operations
- **Config reload**: Use `ConfigManager::check_for_updates()` pattern for hot-reloadable state
- **Suspend/resume**: Test with actual suspend cycle if touching BCE or service-level code
- **iGPU systems**: Verify hypridle.conf fix applied when testing on integrated GPU machines
- **Service files**: Maintain consistency across tiny-dfr-omarchy.service, tiny-dfr.service, and omarchy-dynamic-function-row-daemon.service
