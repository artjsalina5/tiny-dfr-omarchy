# tiny-dfr-omarchy

**Version 0.6.5**

Touch Bar daemon for Apple T2 and Silicon Macs. Omarchy-flavored fork of [tiny-dfr](https://github.com/AsahiLinux/tiny-dfr) with working suspend/resume, Hyprland integration, and comprehensive configuration.

## Features

- **Reliable suspend/resume** on T2 MacBooks with automatic digitizer recovery
- Touch input persists after wake with ~10 second resume time
- Hyprland window context support with per-app button layouts
- Multi-level expandable menus with navigation stack
- Keyboard backlight control with brightness persistence
- Icon caching and battery monitoring with background threads
- Omarchy desktop environment integration
- Easy configuration with hot-reload support

## Installation
### AUR Package

>[!Note}
>Not implemented yet due to superuser requirements. If anyone wants to assist in making this become a reality, I will merge pull requests in support of this.

### Manual Installation

```bash
git clone https://github.com/artjsalina5/tiny-dfr-omarchy.git
cd tiny-dfr-omarchy
./install-tiny-dfr.sh
```

Installs dependencies, builds from source, and sets up systemd services.

### Building from Source

```bash
cargo build --release
sudo install -Dm755 target/release/tiny-dfr /usr/bin/tiny-dfr-omarchy
```

See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup.

## Requirements

### Hardware
- Apple MacBook with Touch Bar (T2 or Apple Silicon)
- Tested on MacBook Pro 16,1 (2019) with Omarchy 3.5.0

### Kernel Modules

Required kernel modules (usually auto-loaded):
- `apple-bce` - Bridge Co-processor Engine driver
- `hid-appletb-kbd` - Touch Bar keyboard driver
- `hid-appletb-bl` - Touch Bar backlight driver  
- `appletbdrm` - DRM driver for Touch Bar display

### Kernel Parameters
Add these to your bootloader configuration for reliable suspend/resume:
>[!Note]
> `mce=off` is added to get rid of the annoying hardware error messages that occur when booting the t2linux kernel. These are not indicative of real hardware errors, but problems with the t2linux kernel drivers that do not affect the computer. Setting `mce=nobootlog` would probably work the same, but I have not verified that. Welcome to pull requests if someone wants to test that for me.

```
intel_iommu=on iommu=pt pcie_ports=compat mem_sleep_default=deep pcie_aspm=off mce=off
```

## Suspend/Resume System

### BCE Messaging Architecture

Reliable suspend/resume coordination relies on a kernel-to-userspace messaging system via the [apple-bce driver](https://github.com/t2linux/apple-bce-drv). This system combines two communication paths for compatibility and extensibility:

- **Mailbox interface** (`/sys/class/bce/vhci/cmd`): Direct mailbox commands for BCE control, including `VHCI_CMD_T2_PAUSE`, `VHCI_CMD_T2_RESUME`, and `VHCI_CMD_RESET_TOUCHBAR`. Matches hardware protocol used by macOS firmware.

- **VHCI interface**: Virtual USB host controller integration for graceful port enumeration and device state management during resume.

Both pathways implement coordinated barriers (PCI link stabilization, DMA status verification) to ensure Touch Bar hardware state is fully reinitialized before libinput rescan begins. This design was developed through community collaboration on the apple-bce-drv project (see [resume fixes PR #7](https://github.com/t2linux/apple-bce-drv/pull/7) and related kernel commits) and ensures robust device recovery across kernel versions.

### Resume Timing

**Resume from suspend takes approximately 10 seconds.** This includes:

- BCE module reload and initialization (~3s)
- PCI link retraining (~4s)
- Touch Bar hardware reset and device enumeration (~3s)
- udev seat reassignment and libinput rebinding

This delay is necessary for reliable digitizer recovery. Recent fixes addressed input device seat-binding races and missing device discovery by ensuring udev finishes device registration before libinput initialization.

### Service Architecture

The package provides two systemd services:

- **`tiny-dfr-omarchy.service`** (primary): Main daemon with startup hardening
  - Device readiness gates with `udevadm settle` and `wait-for-device.sh`
  - Automatic restart on failure with 1-second delay
  - Conflicts with legacy service names to prevent concurrent instances

- **`suspend-fix-t2.service`**: Suspend/resume coordination service
  - Module sequencing: unload Touch Bar drivers before suspend
  - BCE power management with coordinated barriers
  - Post-resume daemon restart after device readiness

**Legacy service names** (`tiny-dfr.service`, `omarchy-dynamic-function-row-daemon.service`) are symbolic links to the primary service for backward compatibility.

### WiFi Fix (MacBook Pro 16,1 Only)

If WiFi fails to restore after suspend on Late 2019 MacBook Pro 16,1, uncomment the WiFi module reload lines in `/etc/systemd/system/suspend-fix-t2.service`:

```bash
sudoedit /etc/systemd/system/suspend-fix-t2.service
```

Uncomment these lines:

```ini
# WiFi - uncomment if network fails after suspend
ExecStart=/usr/bin/modprobe -r brcmfmac_wcc
ExecStart=/usr/bin/modprobe -r brcmfmac

# WiFi - uncomment if network fails after suspend  
ExecStop=/usr/bin/modprobe brcmfmac
ExecStop=/usr/bin/modprobe brcmfmac_wcc
```

Then reload systemd:

```bash
sudo systemctl daemon-reload
```

### iGPU Configuration (Required for Integrated GPU)

If you are using the integrated GPU (iGPU), you **must** apply the hypridle configuration fix.

This ensures that display DPMS (power management) properly coordinates with suspend/resume timing. Without this fix, the main laptop screen will **remain unresponsive with the backlight on** after wake.

Add to your `~/.config/hypr/hypridle.conf`:

```conf
listener {
    timeout = 300
    on-timeout = loginctl lock-session
    after_sleep_cmd = bash -lc 'sleep 1; hyprctl dispatch dpms off; sleep 1; hyprctl dispatch dpms on'
    inhibit_sleep = 3  # wait until screen is locked
}
```

## Configuration

Config files load in priority order:

1. `/usr/share/tiny-dfr/` (package defaults)
2. `/etc/tiny-dfr/` (system-wide overrides)

Copy files from `/usr/share/tiny-dfr/` to `/etc/tiny-dfr/` to customize. The daemon automatically reloads configuration when files change (no restart needed).

### Configuration Files

- **`config.toml`**: Display settings, brightness, fonts, navigation behavior
- **`commands.toml`**: Custom shell commands mapped to button actions
- **`expandables.toml`**: Multi-level menu definitions with button layouts
- **`hyprland.toml`**: Per-application button layouts using Hyprland IPC

See example configurations in `/usr/share/tiny-dfr/` or the [share/tiny-dfr/](share/tiny-dfr/) directory.

### Custom Commands

Terminal applications require a wrapper to launch correctly:

```toml
Command_Terminal = "tiny-dfr-terminal-exec btop"
Command_FileManager = "tiny-dfr-terminal-exec ranger"
```

The `tiny-dfr-terminal-exec` helper automatically detects your terminal emulator or uses `$TERMINAL` environment variable.

### Expandable Menus

Create multi-level menus with the `Expand_` prefix:

```toml
[Expand_Media]
buttons = [
    { icon = "play_pause", action = "KEY_PLAYPAUSE" },
    { icon = "volume_down", action = "KEY_VOLUMEDOWN" },
    { icon = "volume_up", action = "KEY_VOLUMEUP" },
]
```

Reference from main layer:

```toml
{ icon = "apps", action = "Expand_Media" }
```

### Hyprland Integration

The daemon monitors active windows and switches button layouts automatically:

```toml
[Firefox]
buttons = [
    { icon = "back", action = "KeyCombos_ALT_LEFT" },
    { icon = "refresh", action = "KeyCombos_CTRL_R" },
    { icon = "search", action = "KeyCombos_CTRL_F" },
]
```

Window matching uses the application class name from `hyprctl clients`.

## Omarchy Integration

Ships with Omarchy desktop defaults including:

- Screenshot and screen recording tools
- Color picker and clipboard manager
- System monitor integration
- Theme synchronization via hook system

The Omarchy menu is accessible via `Expand_Omarchy` button.

### Installing Omarchy Theme Hook

To synchronize Touch Bar colors with Omarchy themes:

```bash
install -Dm755 /usr/share/doc/tiny-dfr-omarchy/examples/theme-set \
  ~/.config/omarchy/hooks/theme-set
```

## Helper Utilities

The package includes several helper scripts:

- **`omarchy-touchbar-status`**: Check Touch Bar daemon and hardware status
- **`omarchy-touchbar-restart`**: Restart daemon with diagnostics
- **`omarchy-touchbar-debug`**: Detailed hardware and driver state dump
- **`tiny-dfr-kbd-backlight`**: Keyboard backlight control (used by suspend service)
- **`wait-for-device.sh`**: Intelligent device polling for service startup

## Troubleshooting

### Check Service Status

```bash
systemctl status tiny-dfr-omarchy.service
omarchy-touchbar-status
```

### View Logs

```bash
journalctl -u tiny-dfr-omarchy.service -f
journalctl -u suspend-fix-t2.service -f
```

### Check Hardware

```bash
ls -la /dev/dri/card*          # Should show card1 for Touch Bar
sudo evtest                     # Select Touch Bar input device
lsmod | grep -E 'bce|appletb'  # Verify kernel modules loaded
```

### Debug Output

```bash
omarchy-touchbar-debug
```

### Common Issues

**Touch Bar blank after boot**: Wait 10-15 seconds for device initialization. Check `journalctl -u tiny-dfr-omarchy.service` for errors.

**Touch input not working after suspend**: This should be fixed in 0.6.5. If issues persist, check that `suspend-fix-t2.service` is enabled and running.

**Screen stays black after wake (iGPU)**: Apply the hypridle DPMS fix documented above.

**WiFi not working after suspend**: Apply the WiFi fix for MacBook Pro 16,1 documented above.

## Project Resources

- **Repository**: https://github.com/artjsalina5/tiny-dfr-omarchy
- **AUR Package**: `tiny-dfr-omarchy`
- **Issues**: https://github.com/artjsalina5/tiny-dfr-omarchy/issues
- **Upstream**: https://github.com/AsahiLinux/tiny-dfr (original project)
- **T2Linux Wiki**: https://wiki.t2linux.org/

## Contributing

Contributions are welcome! Please read [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

Key areas for contribution:
- Testing on different T2 Mac models
- Additional application configurations for Hyprland integration
- Icon and theme improvements
- Documentation and troubleshooting guides

## Credits

### Author

**Arturo Salinas** <artjsalina5@gmail.com>

Omarchy fork maintainer and developer of suspend/resume reliability improvements, BCE kernel messaging integration, and Hyprland IPC features.

### Original Project

Based on [tiny-dfr](https://github.com/AsahiLinux/tiny-dfr) by the Asahi Linux project.

### Special Thanks

- **WhatAmISupposedToPutHere** - Original tiny-dfr author
- **Beanlord** - Suspend/resume fix contributions
- **T2Linux Community** - Hardware documentation and kernel driver development
- **apple-bce-drv contributors** - BCE kernel driver and resume coordination (PR #7)

## License

Licensed under MIT

- MIT License - See [LICENSE-MIT](LICENSE-MIT)

Original work Copyright (c) 2023 WhatAmISupposedToPutHere  

## Changelog

See [CHANGELOG.md](CHANGELOG.md) for version history and release notes.
