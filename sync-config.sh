#!/bin/bash
# Personal script to sync config files to system locations
# Untracked - not for distribution

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CONFIG_SOURCE="$SCRIPT_DIR/share/tiny-dfr"
CONFIG_DEST="/etc/tiny-dfr"

# System-wide config (requires sudo)
if [[ $EUID -ne 0 ]]; then
    echo "Syncing system config requires sudo..."
    sudo "$SCRIPT_DIR/sync-config.sh" "$@"
    exit 0
fi

echo "Syncing config files to $CONFIG_DEST..."

# Create config directory if needed
mkdir -p "$CONFIG_DEST"

# Copy config files
cp "$CONFIG_SOURCE/config.toml" "$CONFIG_DEST/config.toml"
cp "$CONFIG_SOURCE/commands.toml" "$CONFIG_DEST/commands.toml"
cp "$CONFIG_SOURCE/expandables.toml" "$CONFIG_DEST/expandables.toml"
cp "$CONFIG_SOURCE/hyprland.toml" "$CONFIG_DEST/hyprland.toml"

# Set appropriate permissions
chmod 644 "$CONFIG_DEST"/*.toml

echo "✓ Config files synced to $CONFIG_DEST"
echo "  - config.toml"
echo "  - commands.toml"
echo "  - expandables.toml"
echo "  - hyprland.toml"
