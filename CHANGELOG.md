# Changelog

All notable changes to tiny-dfr-omarchy will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.6.5] - 2026-03-26

### Added

- **BCE kernel messaging integration** for reliable suspend/resume coordination
  - Direct mailbox commands via `/sys/class/bce/vhci/cmd` interface
  - VHCI virtual USB host controller support for device enumeration
  - Coordinated barriers for PCI link stabilization and DMA verification
  - Non-fatal fallback handling for missing kernel interfaces

- **Automatic digitizer recovery** with BCE reset fallback
  - 1.2-second timer triggers recovery if digitizer absent post-resume
  - Forced runtime reinitialization when Touch Bar device nodes exist but digitizer missing
  - Immediate libinput event dispatch after recovery to absorb device-add events
  - Graceful degradation when BCE reset operations fail

- **Service-level startup hardening**
  - ExecStartPre udev settle (4s) ensures device registration completes
  - wait-for-device.sh intelligent polling for touchbar-minimal and hid-kbd
  - RestartSec=1 for quick recovery on daemon failures
  - Conflicts directive prevents concurrent daemon instances across all service names

- **Omarchy theme integration**
  - Real-time color synchronization with Omarchy desktop theme
  - Hook system for automatic theme updates
  - Theme state caching to avoid repeated file I/O

- **Helper utilities**
  - `omarchy-touchbar-status`: Service and hardware status checker
  - `omarchy-touchbar-restart`: Daemon restart with diagnostics
  - `omarchy-touchbar-debug`: Comprehensive hardware state dump
  - `tiny-dfr-kbd-backlight`: Keyboard backlight persistence across suspend/resume

- **Hyprland IPC improvements**
  - Automatic reconnection when Hyprland restarts
  - Rate limiting to prevent IPC floods
  - Connection health monitoring with recovery logic

- **Background thread optimizations**
  - Icon cache loader for async icon loading
  - Battery monitor thread with state caching
  - System monitor for time updates and scheduled cleanups
  - Thread-safe path caching instead of Cairo handle sharing

### Fixed

- **Suspend/resume reliability**
  - Made BCE waits non-fatal: PCI link readiness (4s) and DMA verification (3s) with graceful degradation
  - Removed premature backlight-zero touch event suppression gate
  - Fixed libinput seat-binding race by ensuring udev device registration finishes before daemon initialization
  - Stabilized resume timing to ~10 seconds (BCE reload 3s + PCI 4s + Touch Bar reset 3s + udev/libinput rebinding)
  - Fixed daemon dependency failures by enforcing single primary service via Conflicts directive

- **Touch input persistence**
  - Touch Bar digitizer now reliably detects input after resume
  - Eliminated "no devices" error after wake from suspend
  - Fixed seat assignment races between udev and libinput

- **Configuration system**
  - Repaired non-working Omarchy menu command mappings
  - Fixed Hyprland_Expand_ActiveWindow deserialization
  - Removed XDG per-user config paths, use system-only hierarchy
  - Fixed brightness control conflicts with uwsm lock

- **Hyprland integration**
  - Fixed button showing "N/A" after Hyprland restart
  - Resolved window deserialization failures with rate limiting
  - Fixed button updates after navigation timeout and back button press

- **Display and rendering**
  - Fixed text overflow outside button boundaries
  - Corrected display brightness control
  - Added visual feedback for Expand, Hyprland, and Command button actions

### Changed

- **Service naming**
  - Primary service renamed to `tiny-dfr-omarchy.service`
  - `tiny-dfr.service` and `omarchy-dynamic-function-row-daemon.service` are now symbolic links
  - All three service names conflict with each other to prevent concurrent execution

- **suspend-fix-t2.service improvements**
  - Modprobe sequencing optimized for faster resume
  - Added coordinated barriers for PCI link and DMA readiness
  - Keyboard backlight save/restore integrated into suspend sequence
  - WiFi module reload made optional (disabled by default for faster wake)

- **Configuration defaults**
  - MediaLayerDefault now true for Omarchy-flavored installations
  - JetBrainsMono Nerd Font set as default for icon glyph support
  - Expandable timeout increased to 10 seconds

- **Installer improvements**
  - Omarchy-native detection and integration
  - Automatic dependency installation via omarchy-pkg-add when available
  - Legacy service cleanup during installation

### Performance

- Resume latency reduced from 15-20s to ~10s
- Keyboard backlight restoration happens early in resume sequence
- Icon loading no longer blocks main rendering loop
- Battery and time widgets use cached data to avoid I/O stalls

### Deprecated

- Old suspend workaround scripts (replaced by suspend-fix-t2.service):
  - `t2-suspend.service` (obsolete)
  - `suspend-wifi-unload.service` (obsolete)
  - `resume-wifi-reload.service` (obsolete)
  - `fix-kbd-backlight.service` (obsolete)
  - `t2-wakeup-guard.service` (integrated into suspend-fix-t2.service)

### Documentation

- Updated README with comprehensive installation and configuration instructions
- Added AUR installation section
- Documented kernel parameter requirements
- Added troubleshooting guide with common issues
- Created CONTRIBUTING.md with development guidelines
- Added AGENTS.md for AI coding agent workflows

## [0.5.0] - 2024-XX-XX

### Added

- Initial Omarchy fork from upstream tiny-dfr
- Basic suspend/resume support
- Hyprland integration foundation
- Multi-level expandable menus
- Custom command system
- Configuration hot-reload with inotify

### Fixed

- Basic suspend/resume functionality (Credit: Beanlord)

---

## Release Links

- [0.6.5](https://github.com/artjsalina5/tiny-dfr-omarchy/releases/tag/v0.6.5)
- [Unreleased changes](https://github.com/artjsalina5/tiny-dfr-omarchy/compare/v0.6.5...HEAD)

## Upgrade Notes

### 0.5.0 → 0.6.5

**Important**: This update requires the updated `apple-bce` kernel driver for optimal suspend/resume reliability. See [apple-bce-drv PR #7](https://github.com/t2linux/apple-bce-drv/pull/7).

**Breaking changes**:
- Primary service renamed to `tiny-dfr-omarchy.service` (legacy names remain as symbolic links)
- Configuration hierarchy changed: `/home/user/.config/tiny-dfr/` no longer used
- System config location: `/etc/tiny-dfr/` (system-wide only)

**Migration steps**:

1. Update kernel and apple-bce driver if available
2. Reinstall package or run `install-tiny-dfr.sh`
3. Enable new services:
   ```bash
   sudo systemctl disable tiny-dfr.service  # if old version was enabled
   sudo systemctl enable --now tiny-dfr-omarchy.service
   sudo systemctl enable --now suspend-fix-t2.service
   ```
4. Move custom configs from `~/.config/tiny-dfr/` to `/etc/tiny-dfr/`:
   ```bash
   sudo cp ~/.config/tiny-dfr/*.toml /etc/tiny-dfr/
   ```
5. Review and update kernel parameters for suspend/resume reliability

**Known issues**:
- Resume takes ~10 seconds (intentional for hardware stability)
- iGPU systems require hypridle DPMS fix (documented in README)
- WiFi may need manual reload on MacBook Pro 16,1 (optional fix available)
