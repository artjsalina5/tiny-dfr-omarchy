#!/bin/bash
# wait-for-device.sh - Intelligent device polling for suspend/resume
# Usage: wait-for-device.sh <device_type> <max_wait_seconds>

set -euo pipefail

DEVICE_TYPE="${1:-}"
MAX_WAIT="${2:-10}"

if [[ -z "$DEVICE_TYPE" ]]; then
    echo "Error: Device type required" >&2
    echo "Usage: $0 <device_type> <max_wait_seconds>" >&2
    echo "Device types: drm, hid-backlight, bce, hid-kbd" >&2
    exit 1
fi

check_device() {
    case "$DEVICE_TYPE" in
        drm)
            [[ -e /dev/dri/card1 ]]
            ;;
        hid-backlight)
            [[ -e /sys/class/backlight/apple-touch-bar-backlight ]]
            ;;
        bce)
            [[ -d /sys/bus/pci/drivers/apple-bce ]] && lsmod | grep -q apple_bce
            ;;
        hid-kbd)
            # Check for any Touch Bar input device
            compgen -G "/dev/input/by-id/*Touch_Bar*" > /dev/null
            ;;
        touchbar-minimal)
            # Check only essential components for Touch Bar rendering
            [[ -e /dev/dri/card0 ]] && \
            [[ -e /sys/class/backlight/appletb_backlight ]]
            ;;
        touchbar-bce)
            # BCE readiness check for suspend/resume coordination
            # Supports both tiny-dfr mailbox interface and upstream bce-vhci.
            if [[ -f "/sys/class/bce/vhci/cmd_status" ]]; then
                local status=$(cat "/sys/class/bce/vhci/cmd_status")
                [[ "$status" != *"VHCI_CMD_T2_PAUSE"* ]] && [[ "$status" != *"SUSPEND"* ]]
            elif [[ -f "/sys/class/bce-vhci/bce-vhci/power/runtime_status" ]]; then
                local runtime_status=$(cat "/sys/class/bce-vhci/bce-vhci/power/runtime_status")
                [[ "$runtime_status" != "suspended" ]]
            elif [[ -d "/sys/module/apple_bce" ]] && [[ -e "/dev/bce-vhci" || -d "/sys/class/bce-vhci/bce-vhci" ]]; then
                true
            else
                false
            fi
            ;;
        touchbar-full)
            # Full Touch Bar readiness check: BCE + DRM + Backlight
            check_device touchbar-bce && \
            [ -e "/dev/dri/card0" ] && \
            [ -e "/sys/class/backlight/appletb_backlight" ]
            ;;
        touchbar-pci-link)
            # Specifically wait for PCI Express link training (critical after resume)
            if [[ -f "/sys/class/pci_bus/0000:04/link_status" ]]; then
                local link_status=$(cat "/sys/class/pci_bus/0000:04/link_status")
                [[ "$link_status" == *"LINK_STATE_TRAINING_COMPLETE"* ]] || \
                [[ "$link_status" == *"LINK_ACTIVE"* ]]
            else
                false
            fi
            ;;
        *)
            echo "Error: Unknown device type '$DEVICE_TYPE'" >&2
            echo "Supported types: drm, hid-backlight, bce, hid-kbd, touchbar-minimal, touchbar-bce, touchbar-full, touchbar-pci-link" >&2
            exit 1
            ;;
    esac
}

# Poll for device with 0.1s interval
elapsed=0
interval=0.1

while (( $(echo "$elapsed < $MAX_WAIT" | bc -l) )); do
    if check_device; then
        echo "Device '$DEVICE_TYPE' ready after ${elapsed}s" >&2
        exit 0
    fi
    sleep "$interval"
    elapsed=$(echo "$elapsed + $interval" | bc -l)
done

echo "Timeout: Device '$DEVICE_TYPE' not ready after ${MAX_WAIT}s" >&2
exit 1
