#!/bin/bash
# install-tiny-dfr.sh
# Installation script for Omarchy Dynamic Function Row Daemon on T2 MacBooks

set -e

SKIP_DEPS=false
DAEMON_BIN="tiny-dfr-omarchy"
LEGACY_BIN="tiny-dfr"
DAEMON_SERVICE="${DAEMON_BIN}.service"
LEGACY_SERVICE="tiny-dfr.service"
LEGACY_OMARCHY_SERVICE="omarchy-dynamic-function-row-daemon.service"
LEGACY_OMARCHY_BIN="omarchy-dynamic-function-row-daemon"

install_arch_deps() {
    local user_home="$1"
    local omarchy_bin_dir="${user_home}/.local/share/omarchy/bin"

    # Prefer Omarchy package helper when available in Omarchy-managed installs.
    if [[ -x "${omarchy_bin_dir}/omarchy-pkg-add" ]]; then
        echo "Installing dependencies with Omarchy package helper..."
        "${omarchy_bin_dir}/omarchy-pkg-add" rust cargo cairo libinput freetype2 fontconfig librsvg
        return
    fi

    if command -v omarchy-pkg-add &> /dev/null; then
        echo "Installing dependencies with omarchy-pkg-add..."
        omarchy-pkg-add rust cargo cairo libinput freetype2 fontconfig librsvg
        return
    fi

    echo "Installing dependencies (Arch/pacman)..."
    sudo pacman -S --needed --noconfirm rust cargo cairo libinput freetype2 fontconfig librsvg
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --skip-deps)
            SKIP_DEPS=true
            shift
            ;;
        *)
            echo "Unknown option: $1" >&2
            echo "Supported: --skip-deps" >&2
            exit 1
            ;;
    esac
done

echo "Installing tiny-dfr-omarchy for T2 MacBook..."

# Check if running on T2 Mac
if ! grep -q "MacBookPro16,1\|MacBookAir" /sys/class/dmi/id/product_name 2>/dev/null; then
    echo "Warning: This doesn't appear to be a T2 MacBook"
    read -p "Continue anyway? (y/N): " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 1
    fi
fi

# Check if running as root
if [[ $EUID -eq 0 ]]; then
    echo "This script should not be run as root. Please run as a regular user."
    exit 1
fi

CURRENT_USER=$(whoami)
USER_HOME="/home/$CURRENT_USER"

# Install dependencies based on distro (skip if --skip-deps)
if ! $SKIP_DEPS; then
    if command -v pacman &> /dev/null; then
        install_arch_deps "$USER_HOME"
    elif command -v apt &> /dev/null; then
        echo "Installing dependencies (Debian/Ubuntu)..."
        sudo apt update
        sudo apt install -y build-essential rustc cargo libcairo2-dev libinput-dev libfreetype6-dev libfontconfig1-dev librsvg2-dev
    elif command -v dnf &> /dev/null; then
        echo "Installing dependencies (Fedora)..."
        sudo dnf install -y rust cargo cairo-devel libinput-devel freetype-devel fontconfig-devel librsvg2-devel
    else
        echo "Unsupported distribution. Install dependencies manually or use --skip-deps."
        echo "Required: rust, cargo, cairo, libinput, freetype, fontconfig, librsvg"
        exit 1
    fi
else
    echo "Skipping dependency installation (--skip-deps)"
fi

# Build from source
echo "Building daemon binary..."
cargo build --release --bin tiny-dfr

# Install
echo "Installing ${DAEMON_BIN}..."
# Stop service if running to avoid "Text file busy" error
sudo systemctl stop "$DAEMON_SERVICE" 2>/dev/null || true
sudo systemctl stop "$LEGACY_SERVICE" 2>/dev/null || true
sudo systemctl stop "$LEGACY_OMARCHY_SERVICE" 2>/dev/null || true
sudo install -Dm755 target/release/tiny-dfr "/usr/bin/${DAEMON_BIN}"
sudo ln -sf "/usr/bin/${DAEMON_BIN}" "/usr/bin/${LEGACY_BIN}"
sudo ln -sf "/usr/bin/${DAEMON_BIN}" "/usr/bin/${LEGACY_OMARCHY_BIN}"
sudo mkdir -p /usr/share/tiny-dfr
sudo cp share/tiny-dfr/* /usr/share/tiny-dfr/
sudo cp etc/systemd/system/suspend-fix-t2.service /etc/systemd/system/
sudo install -Dm644 etc/systemd/system/tiny-dfr-omarchy.service "/etc/systemd/system/${DAEMON_SERVICE}"
sudo ln -sf "/etc/systemd/system/${DAEMON_SERVICE}" "/etc/systemd/system/${LEGACY_SERVICE}"
sudo ln -sf "/etc/systemd/system/${DAEMON_SERVICE}" "/etc/systemd/system/${LEGACY_OMARCHY_SERVICE}"
sudo install -Dm755 bin/tiny-dfr-terminal-exec /usr/bin/tiny-dfr-terminal-exec
sudo install -Dm755 bin/wait-for-device.sh /usr/bin/wait-for-device.sh
sudo install -Dm755 bin/tiny-dfr-kbd-backlight /usr/bin/tiny-dfr-kbd-backlight
sudo install -Dm755 bin/omarchy-touchbar-status /usr/bin/omarchy-touchbar-status
sudo install -Dm755 bin/omarchy-touchbar-restart /usr/bin/omarchy-touchbar-restart
sudo install -Dm755 bin/omarchy-touchbar-debug /usr/bin/omarchy-touchbar-debug

# Install udev rules
sudo cp etc/udev/rules.d/99-touchbar-seat.rules /etc/udev/rules.d/
sudo cp etc/udev/rules.d/99-touchbar-tiny-dfr.rules /etc/udev/rules.d/
sudo udevadm control --reload-rules
sudo udevadm trigger

# Setup systemd service
sudo systemctl daemon-reload

# Enforce a single daemon unit to avoid multiple instances contending for Touch Bar input.
sudo systemctl disable --now "$LEGACY_SERVICE" 2>/dev/null || true
sudo systemctl disable --now "$LEGACY_OMARCHY_SERVICE" 2>/dev/null || true
sudo systemctl reset-failed "$LEGACY_SERVICE" "$LEGACY_OMARCHY_SERVICE" 2>/dev/null || true

# Avoid duplicate suspend handlers on Omarchy installs.
if systemctl list-unit-files t2-suspend.service >/dev/null 2>&1; then
    echo "Detected Omarchy t2-suspend.service; disabling it to avoid duplicate resume workflows..."
    sudo systemctl disable --now t2-suspend.service 2>/dev/null || true
fi

sudo systemctl enable "$DAEMON_SERVICE"
sudo systemctl enable suspend-fix-t2.service

# Detect user environment for proper configuration
echo "Detecting user environment..."
USER_UID=$(id -u $CURRENT_USER)
RUNTIME_DIR="/run/user/$USER_UID"

# Detect Wayland display
WAYLAND_DISPLAY_VALUE="wayland-1"  # default
if [ -d "$RUNTIME_DIR" ]; then
    for socket in "$RUNTIME_DIR"/wayland-*; do
        if [ -S "$socket" ] && [[ ! "$socket" == *.lock ]]; then
            WAYLAND_DISPLAY_VALUE=$(basename "$socket")
            break
        fi
    done
fi

# Detect user's actual PATH locations
USER_PATHS=""
for path_candidate in \
    "$USER_HOME/.local/share/omarchy/bin" \
    "$USER_HOME/.local/bin" \
    "$USER_HOME/.config/nvm/versions/node/latest/bin" \
    "$USER_HOME/.local/share/pnpm" \
    "$USER_HOME/.cargo/bin" \
    "$USER_HOME/.npm-global/bin" \
    "$USER_HOME/bin"; do
    if [ -d "$path_candidate" ]; then
        USER_PATHS="$USER_PATHS:$path_candidate"
    fi
done

echo "Detected user: $CURRENT_USER"
echo "Detected UID: $USER_UID"
echo "Detected Wayland display: $WAYLAND_DISPLAY_VALUE"
echo "Detected user paths: $USER_PATHS"

# Install Omarchy theme hook for instant Touch Bar theme updates
mkdir -p "$USER_HOME/.config/omarchy/hooks"
install -Dm755 etc/omarchy/hooks/theme-set "$USER_HOME/.config/omarchy/hooks/theme-set"

# Install configs (always update with backups)
sudo mkdir -p /etc/tiny-dfr

TS=$(date +%Y%m%d%H%M%S)
for f in config.toml commands.toml expandables.toml hyprland.toml; do
  if [ -f "/etc/tiny-dfr/$f" ]; then
    sudo cp "/etc/tiny-dfr/$f" "/etc/tiny-dfr/$f.bak.$TS"
  fi
  sudo cp "/usr/share/tiny-dfr/$f" "/etc/tiny-dfr/$f"
done

# Create user-specific environment configuration
sudo tee /etc/tiny-dfr/user-env.toml > /dev/null <<EOF
# Auto-generated user environment configuration
[user_environment]
username = "$CURRENT_USER"
uid = $USER_UID
home_dir = "$USER_HOME"
runtime_dir = "$RUNTIME_DIR"
wayland_display = "$WAYLAND_DISPLAY_VALUE"
user_paths = "$USER_PATHS"
EOF

# Set MediaLayerDefault = true
echo "Setting MediaLayerDefault = true in config..."
sudo sed -i 's/MediaLayerDefault = false/MediaLayerDefault = true/' /etc/tiny-dfr/config.toml

# Restart the service to apply config changes
echo "Restarting ${DAEMON_SERVICE}..."
sudo systemctl restart "$DAEMON_SERVICE"

echo ""
echo "Checking Omarchy integration..."
if command -v omarchy-menu &> /dev/null; then
    echo "✓ Omarchy commands found"
    if [ -f "$USER_HOME/.config/omarchy/current/theme/colors.toml" ]; then
        echo "✓ Omarchy theme detected (theme sync available)"
    fi
else
    echo "⚠ Omarchy commands not found - some Touch Bar buttons may not work"
    echo "  Install Omarchy for full integration: https://omarchy.org"
fi

echo ""
echo "✓ Installation complete!"
echo ""

