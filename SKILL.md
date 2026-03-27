# tiny-dfr-omarchy Development SKILL

**Domain**: Apple T2/Silicon MacBook Touch Bar Daemon Development  
**Purpose**: Specialized workflows and best practices for maintaining and extending tiny-dfr-omarchy  
**Language**: Rust  
**When to use**: Any modifications to core daemon logic, suspend/resume code, systemd services, or hardware integration

## When This Skill Applies

Use this skill whenever working on:
- Rust daemon code (`src/*.rs`)
- Systemd service files (`etc/systemd/system/*.service`)
- BCE (Bridge Co-processor Engine) communication and suspend/resume logic
- Touch Bar hardware integration or device initialization
- libinput seat binding and input device discovery
- Display rendering or DRM framebuffer management
- Configuration system and hot-reload mechanisms

## Core Architecture Overview

### Hardware Stack
- **T2 Coprocessor**: Apple Bridge Co-processor Engine (BCE) for system management
- **Touch Bar Device**: Multitouch interface via HID (hid-multitouch driver, event11)
- **DRM Framebuffer**: Display output rendered via cairo-rs on `/dev/dri/card1`
- **Keyboard**: HID keyboard backlight control via hid-appletb-kbd
- **libinput**: Multi-seat input processing with udev device discovery

### Software Layers
```
┌─────────────────────────────────────────────┐
│  tiny-dfr-omarchy daemon (root)             │
│  - Event loop (epoll on DRM/input/config)   │
│  - Cairo rendering (UI/icons)               │
│  - Hyprland IPC (window context)            │
└──────────────┬──────────────────────────────┘
               ↓
┌─────────────────────────────────────────────┐
│  libinput (multi-seat: seat0, seat-touchbar)│
│  - Touch Bar: /dev/input/event11            │
│  - Keyboard: /dev/input/event*              │
└──────────────┬──────────────────────────────┘
               ↓
┌─────────────────────────────────────────────┐
│  Kernel Drivers                             │
│  - apple-bce (BCE communication)            │
│  - appletbdrm (DRM driver for framebuffer)  │
│  - hid-appletb-* (keyboard/backlight)       │
│  - hid-multitouch (Touch Bar multitouch)    │
└─────────────────────────────────────────────┘
```

## Coding Practices

### Rust Code Quality

**Format before every commit:**
```bash
cargo fmt
```

**Check for issues:**
```bash
cargo clippy --all-targets --all-features
```

**Build both debug and release:**
```bash
cargo check              # Fast syntax validation
cargo build --release   # Optimized binary
```

**Error Handling Pattern**
- Always use `anyhow::Result<T>` for fallible operations
- Propagate errors with `?` operator
- Use `anyhow!("message")` for custom errors with context
- Document why `.unwrap()` is safe when used (should be rare)
- Log errors with `eprintln!()` before recovery attempts

Good:
```rust
fn initialize_device() -> Result<Device> {
    let dri_path = find_dri_device()
        .context("Failed to locate DRI device")?;
    let fd = fs::open(&dri_path)?;
    Ok(Device::new(fd)?)
}
```

Bad:
```rust
fn initialize_device() -> Result<Device> {
    let fd = fs::open("/dev/dri/card1").unwrap(); // ✗ No error context
    Ok(Device::new(fd).unwrap())
}
```

### Service File Consistency

All three systemd service units must maintain identical structure:
- **tiny-dfr-omarchy.service** (primary)
- **tiny-dfr.service** (legacy alias)
- **omarchy-dynamic-function-row-daemon.service** (legacy alias)

Ensure all three have:
- Same `ExecStartPre` sequence (udev settle → device readiness waits)
- Same `Restart=always` and `RestartSec=1` policies
- Same `Conflicts=` guards against concurrent instances
- Same security settings block
- Same `[Install]` target

Update all three simultaneously to prevent divergence.

### Suspend/Resume Modifications

When touching BCE communication or suspend logic:

1. **Test with actual suspend:** Changes must be validated with `systemctl suspend`
2. **Check Both Interfaces:** Ensure both mailbox and VHCI code paths are considered
3. **Non-Fatal Fallbacks:** BCE resets should fail gracefully (warn, not panic)
4. **Timing Windows:** Respect the 1.2s digitizer recovery deadline
5. **Device Readiness:** Ensure udev finishes before libinput rebinds seats

Key timing:
- Suspend completion: ~5-7s
- Resume init: ~3s (BCE reload)
- PCI retraining: ~4s
- Touch Bar reset: ~3s
- Total: ~10 seconds expected

### Device Initialization Patterns

**libinput Seat-Binding Race**
- Problem: libinput initializes before udev finishes device registration
- Solution: ExecStartPre waits ensure udev complete: `udevadm settle -t 4`
- Then verify devices exist: `wait-for-device.sh touchbar-minimal 4`
- Do NOT skip these waits; they prevent "digitizer not found" failures

**BCE Command Interface Selection**
```rust
// Probe mailbox first (explicit commands supported)
if has_mailbox_interface() {
    // Full featured: supports PAUSE, RESUME, RESET_TOUCHBAR
}

// Fall back to VHCI (limited to port enumeration)
if has_vhci_interface() {
    // Only supports device state management, NOT userspace resets
    // Return Ok(()) gracefully for non-fatal operations
}
```

## Common Tasks

### Adding Suspend/Resume Logic
1. Modify `bce_send_*()` functions in `src/bce_interface.rs`
2. Update timing: adjust `BCE_READY_TIMEOUT_SECS` and `SYS_WAIT_*` constants
3. Reference BCE driver: check `/sys/class/bce/vhci/cmd` interface availability
4. Test both interfaces: mailbox and VHCI paths
5. **Test with actual suspend:** `systemctl suspend`

### Fixing libinput Device Issues
1. Check udev rules: `cat /etc/udev/rules.d/99-touchbar-*.rules`
2. Verify device enumeration: `udevadm info -e | grep -i touch`
3. Check seat assignment: `loginctl seat-status seat-touchbar`
4. Look at daemon startup logs for device-open timing
5. If missing: likely udev race → adjust ExecStartPre waits

### Updating Service Files
1. Edit the primary unit: `etc/systemd/system/tiny-dfr-omarchy.service`
2. Copy changes to: `tiny-dfr.service` and `omarchy-dynamic-function-row-daemon.service`
3. Validate: `systemd-analyze verify` for all three
4. Reload: `sudo systemctl daemon-reload`
5. Verify identity: check `systemctl status` shows same settings

### Handling iGPU Display Issues
- Problem: Screen unresponsive after suspend on iGPU systems
- Root cause: DPMS not properly coordinated with resume timing
- Fix location: `~/.config/hypr/hypridle.conf`
- Required: `after_sleep_cmd = bash -lc 'sleep 1; hyprctl dispatch dpms off; sleep 1; hyprctl dispatch dpms on'`
- Document in README and test on iGPU hardware

## Debugging Workflows

### Capture Suspend Logs
```bash
# Before suspend: enable persistent journal
sudo mkdir -p /var/log/journal

# Suspend and watch
sudo systemctl suspend

# View logs (follow resume messages)
sudo journalctl -u suspend-fix-t2.service -u tiny-dfr-omarchy.service -b --no-pager | tail -100
```

### Monitor Device Presence During Resume
```bash
# In one terminal, watch device files:
watch -n 0.1 'ls -la /dev/input/event11 2>&1; ls -la /dev/dri/card1 2>&1'

# In another, suspend:
sudo systemctl suspend
```

### Test Digitizer Recovery Timing
```bash
# Check 1.2s deadline trigger in logs:
sudo journalctl -u tiny-dfr-omarchy.service -f | grep -i "digitizer\|recovery\|bce"
```

### Verify Service Coordination
```bash
# Check all three units are properly coordinated:
systemd-analyze verify etc/systemd/system/tiny-dfr-omarchy.service
systemd-analyze verify etc/systemd/system/tiny-dfr.service
systemd-analyze verify etc/systemd/system/omarchy-dynamic-function-row-daemon.service

# Verify no concurrent instances:
systemctl status tiny-dfr-omarchy.service tiny-dfr.service omarchy-dynamic-function-row-daemon.service
```

## Performance Considerations

### Event Loop Latency
- Main loop is epoll-based, waiting on: DRM vblank, libinput events, config inotify
- Avoid blocking operations in event handlers
- Keep rendering frame budget ~16ms (60 FPS)
- Use non-blocking I/O where possible

### Memory Usage
- Static config is lazily loaded once, then hot-reloaded via inotify
- Icon cache is bounded; old unused icons are evicted
- Global state uses `LazyLock<Arc<Mutex<>>>` pattern for thread-safe access
- Aim for <50MB resident set size

### Startup Time
- DRM device discovery: ~100ms
- libinput initialization: ~200ms
- Font loading: ~50ms
- Total: target <1 second from systemd start

## Testing Strategies

### Unit Testing Pure Functions
- Config parsing: write tests for TOML deserialization
- Color conversion: test RGB ↔ HSV transformations
- Layout calculations: test button positioning logic

Use:
```bash
cargo test --lib
```

### Integration Testing
- Hardware: requires Touch Bar device (real Mac or VM)
- Mocking: mock DRM framebuffer and libinput events
- Systemd: verify service files with `systemd-analyze`

### Manual Testing Checklist
- [ ] Daemon starts: `sudo systemctl start tiny-dfr-omarchy`
- [ ] Touch Bar renders: confirm visual output
- [ ] Touch input works: tap buttons, verify logs
- [ ] Config reload: edit config, verify hot-reload
- [ ] Suspend/resume: `systemctl suspend`, verify digitizer recovers
- [ ] iGPU systems: test with hypridle.conf fix applied

## Repository Maintenance

### Before Committing
```bash
cargo fmt                      # Format all code
cargo clippy                   # Check for warnings
cargo build --release          # Verify builds
systemd-analyze verify etc/systemd/system/*.service  # Validate units
```

### Cleanup Tasks
- Remove `eprintln!()` debug statements before merge
- Ensure all three service files stay in sync
- Keep AGENTS.md and README.md in sync with code changes
- Update SKILL.md when adding new patterns or workflows

### Consistency Checks
```bash
# Verify service file consistency:
diff <(sed 's/tiny-dfr-omarchy/PRIMARY/g' etc/systemd/system/tiny-dfr-omarchy.service) \
     <(sed 's/tiny-dfr/PRIMARY/g' etc/systemd/system/tiny-dfr.service) \
     <(sed 's/omarchy-dynamic-function-row-daemon/PRIMARY/g' etc/systemd/system/omarchy-dynamic-function-row-daemon.service)
```

## Key References

- **Apple BCE Driver**: https://github.com/t2linux/apple-bce-drv (especially PR #7 for resume fixes)
- **tiny-dfr Original**: https://github.com/AsahiLinux/tiny-dfr
- **Rust Error Handling**: Use `anyhow` crate patterns consistently
- **systemd Services**: See `man systemd.service` for directive reference
- **Linux DRM/KMS**: Knowledge of direct rendering manager helpful for display code
- **libinput**: Multi-seat design important for understanding Touch Bar input binding
