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
        *)
            echo "Error: Unknown device type '$DEVICE_TYPE'" >&2
            echo "Supported types: drm, hid-backlight, bce, hid-kbd" >&2
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
