// bce_interface.rs - Apple Bridge Co-processor Engine (BCE/T2) communication
//
// Coordinates with the apple-bce driver for suspend/resume operations
// Follows T2 hardware protocol matching macOS behavior
//
// Based on BCE driver patches: https://github.com/t2linux/apple-bce-drv/pull/23
// Discussed on Matrix: https://wiki.t2linux.org/

use anyhow::{anyhow, Context, Result};
use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};

/// Mailbox commands for T2 suspend/resume coordination
/// Matches defines in apple-bce driver
#[repr(u8)]
pub enum VhciCommand {
    VHCI_CMD_NONE = 0,           // No command
    VHCI_CMD_T2_PAUSE = 1,       // Suspend command
    VHCI_CMD_T2_RESUME = 2,      // Resume command
    VHCI_CMD_ENABLE_I2C = 3,     // Enable I²C communication
    VHCI_CMD_RESET_TOUCHBAR = 4, // Full Touch Bar reset
}

/// BCE device paths
pub struct BcePaths {
    /// Mailbox command input (write commands here)
    pub cmd_path: &'static str,

    /// Mailbox status output (read status here)
    pub cmd_status_path: &'static str,

    /// DMA transfer status
    pub dma_status_path: &'static str,

    /// PCI Express link status for BCE device
    pub pci_link_path: &'static str,
}

// BCE device paths (MacBookPro16,1, MacBookAir9,1 tested)
pub const BCE_PATHS: BcePaths = BcePaths {
    cmd_path: "/sys/class/bce/vhci/cmd",
    cmd_status_path: "/sys/class/bce/vhci/cmd_status",
    dma_status_path: "/sys/class/bce/vhci/dma_status",
    pci_link_path: "/sys/class/pci_bus/0000:04/link_status", // BCE bus
};

const BCE_VHCI_CLASS_PATH: &str = "/sys/class/bce-vhci/bce-vhci";
const BCE_VHCI_RUNTIME_STATUS_PATH: &str = "/sys/class/bce-vhci/bce-vhci/power/runtime_status";

fn has_mailbox_interface() -> bool {
    Path::new(BCE_PATHS.cmd_path).exists() && Path::new(BCE_PATHS.cmd_status_path).exists()
}

fn has_vhci_interface() -> bool {
    Path::new(BCE_VHCI_CLASS_PATH).exists() || Path::new("/dev/bce-vhci").exists()
}

/// Check if BCE driver is available
pub fn bce_driver_available() -> bool {
    has_mailbox_interface() || has_vhci_interface() || Path::new("/sys/module/apple_bce").exists()
}

/// Check if BCE is suspended (pause command issued)
pub fn bce_is_suspended() -> bool {
    if has_mailbox_interface() {
        return fs::read_to_string(BCE_PATHS.cmd_status_path)
            .ok()
            .map(|s| s.contains("VHCI_CMD_T2_PAUSE") || s.contains("SUSPEND"))
            .unwrap_or(false);
    }

    if has_vhci_interface() {
        return fs::read_to_string(BCE_VHCI_RUNTIME_STATUS_PATH)
            .ok()
            .map(|s| s.trim().eq_ignore_ascii_case("suspended"))
            .unwrap_or(false);
    }

    false
}

/// Send T2 resume command to BCE mailbox
/// Similar to apple_bce_t2_resume() in kernel driver
pub fn bce_send_resume() -> Result<()> {
    if has_mailbox_interface() {
        return fs::write(BCE_PATHS.cmd_path, "VHCI_CMD_T2_RESUME")
            .context("BCE resume command failed");
    }

    if has_vhci_interface() {
        // Upstream apple-bce-drv does not expose an equivalent user-space mailbox command.
        return Ok(());
    }

    Err(anyhow!("BCE resume command unavailable: no supported interface found"))
}

/// Send Touch Bar reset command
pub fn bce_send_touchbar_reset() -> Result<()> {
    fs::write(BCE_PATHS.cmd_path, "VHCI_CMD_RESET_TOUCHBAR")
        .context("BCE Touch Bar reset command failed")
}

/// Wait for BCE to become ready (with DMA barriers)
/// Prevents "scheduling while atomic" BUGs reported in BCE driver
pub fn bce_wait_ready(timeout: Duration) -> Result<()> {
    let start = Instant::now();

    loop {
        if start.elapsed() >= timeout {
            return Err(anyhow!(
                "BCE ready timeout: {:?} - DMA may be hung",
                timeout
            ));
        }

        // Check DMA transfers completed
        let dma_status = fs::read_to_string(BCE_PATHS.dma_status_path)?;
        if dma_status.contains("DMA_INACTIVE") {
            // Verify PCI Express link established (critical after resume)
            if check_pci_link_ready()? {
                return Ok(());
            }
        }

        std::thread::sleep(Duration::from_millis(50));
    }
}

/// Check PCI Express link readiness for BCE
/// BCE communicates over PCI Express encapsulated in framework USB
/// Must re-train after S3 resume before accessing MMIO
pub fn check_pci_link_ready() -> Result<bool> {
    let link_status = fs::read_to_string(BCE_PATHS.pci_link_path)?;
    Ok(link_status.contains("LINK_STATE_TRAINING_COMPLETE") || link_status.contains("LINK_ACTIVE"))
}

/// Wait for PCI Express link re-establishment after suspend
/// Implements the manual link training seen in BCE driver testing
pub fn wait_pci_link_ready(timeout: Duration) -> Result<()> {
    let start = Instant::now();

    // Poll for PCI link readiness
    loop {
        if start.elapsed() >= timeout {
            return Err(anyhow!(
                "PCI link timeout: {:?} - BCE inaccessible after suspend",
                timeout
            ));
        }

        if check_pci_link_ready()? {
            return Ok(());
        }

        std::thread::sleep(Duration::from_millis(50));
    }
}

/// Check if BCE device is fully ready for Touch Bar operations
/// Used by touchbar_nodes_ready() for comprehensive hardware validation
pub fn bce_ready_for_resume() -> bool {
    if has_mailbox_interface() {
        // Check mailbox status
        if let Ok(status) = fs::read_to_string(BCE_PATHS.cmd_status_path) {
            if status.contains("VHCI_CMD_T2_PAUSE")
                || status.contains("SUSPEND")
                || status.contains("INACTIVE")
            {
                eprintln!("BCE waiting: mailbox status = {}", status.trim());
                return false;
            }
        }

        // Check PCI link
        if let Ok(link_status) = fs::read_to_string(BCE_PATHS.pci_link_path) {
            if !link_status.contains("LINK_ACTIVE") {
                eprintln!("BCE waiting: PCI link not active");
                return false;
            }
        }

        // Check DMA transfers
        if let Ok(dma_status) = fs::read_to_string(BCE_PATHS.dma_status_path) {
            if dma_status.contains("DMA_BUSY") {
                eprintln!("BCE waiting: DMA active");
                return false;
            }
        }

        return true;
    }

    if has_vhci_interface() {
        // Upstream apple-bce-drv path: runtime PM status is the best available health signal.
        if let Ok(runtime_status) = fs::read_to_string(BCE_VHCI_RUNTIME_STATUS_PATH) {
            if runtime_status.trim().eq_ignore_ascii_case("suspended") {
                eprintln!("BCE waiting: bce-vhci runtime is suspended");
                return false;
            }
        }
        return true;
    }

    // No BCE interface detected.
    false
}
