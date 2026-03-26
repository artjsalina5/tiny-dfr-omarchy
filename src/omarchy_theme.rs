#![allow(dead_code)]

use anyhow::{Context, Result};
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, LazyLock, Mutex};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Deserialize)]
pub struct OmarchyColors {
    pub accent: String,
    pub foreground: String,
    pub background: String,
    pub cursor: String,
    #[serde(default)]
    pub selection_foreground: String,
    #[serde(default)]
    pub selection_background: String,
    #[serde(default)]
    pub color0: String,
    #[serde(default)]
    pub color1: String,
    #[serde(default)]
    pub color2: String,
    #[serde(default)]
    pub color3: String,
    #[serde(default)]
    pub color4: String,
    #[serde(default)]
    pub color5: String,
    #[serde(default)]
    pub color6: String,
    #[serde(default)]
    pub color7: String,
    #[serde(default)]
    pub color8: String,
    #[serde(default)]
    pub color9: String,
    #[serde(default)]
    pub color10: String,
    #[serde(default)]
    pub color11: String,
    #[serde(default)]
    pub color12: String,
    #[serde(default)]
    pub color13: String,
    #[serde(default)]
    pub color14: String,
    #[serde(default)]
    pub color15: String,
}

impl OmarchyColors {
    pub fn accent_rgb(&self) -> Option<(f64, f64, f64)> {
        hex_to_rgb(&self.accent)
    }

    pub fn foreground_rgb(&self) -> Option<(f64, f64, f64)> {
        hex_to_rgb(&self.foreground)
    }

    pub fn background_rgb(&self) -> Option<(f64, f64, f64)> {
        hex_to_rgb(&self.background)
    }

    pub fn selection_background_rgb(&self) -> Option<(f64, f64, f64)> {
        hex_to_rgb(&self.selection_background)
    }
}

/// Manages Omarchy theme colors with caching to avoid excessive file I/O
struct OmarchyThemeManager {
    cached_colors: Option<OmarchyColors>,
    last_check: Instant,
    check_interval: Duration,
    theme_available: bool,
}

static THEME_MANAGER: LazyLock<Arc<Mutex<OmarchyThemeManager>>> = LazyLock::new(|| {
    Arc::new(Mutex::new(OmarchyThemeManager {
        cached_colors: None,
        last_check: Instant::now() - Duration::from_secs(60),
        check_interval: Duration::from_secs(5), // Check every 5s for theme changes
        theme_available: false,
    }))
});

impl OmarchyThemeManager {
    /// Get current Omarchy theme colors with caching
    /// Returns None if Omarchy is not installed or theme not configured
    pub fn get_colors() -> Option<OmarchyColors> {
        let mut mgr = THEME_MANAGER.lock().unwrap();

        // Rate-limit theme file reads to reduce I/O overhead
        if mgr.last_check.elapsed() < mgr.check_interval {
            return mgr.cached_colors.clone();
        }

        mgr.last_check = Instant::now();

        match load_omarchy_theme() {
            Ok(colors) => {
                if !mgr.theme_available {
                    eprintln!("Omarchy theme integration active: using theme colors for Touch Bar");
                    mgr.theme_available = true;
                }
                mgr.cached_colors = Some(colors.clone());
                Some(colors)
            }
            Err(_) => {
                // Silently fail - Omarchy not installed or theme not set
                // This is expected on non-Omarchy systems
                if mgr.theme_available {
                    eprintln!("Omarchy theme no longer available - falling back to config colors");
                    mgr.theme_available = false;
                }
                mgr.cached_colors = None;
                None
            }
        }
    }

    /// Invalidate cache to force reload on next access (e.g., after SIGHUP)
    pub fn invalidate_cache() {
        let mut mgr = THEME_MANAGER.lock().unwrap();
        mgr.cached_colors = None;
        mgr.last_check = Instant::now() - Duration::from_secs(60);
        eprintln!("Omarchy theme cache invalidated - will reload on next access");
    }

    /// Check if theme is currently available
    pub fn is_available() -> bool {
        THEME_MANAGER.lock().unwrap().theme_available
    }
}

/// Public API for accessing Omarchy theme colors
pub fn get_theme_colors() -> Option<OmarchyColors> {
    OmarchyThemeManager::get_colors()
}

/// Public API for invalidating theme cache (called on SIGHUP)
pub fn invalidate_theme_cache() {
    OmarchyThemeManager::invalidate_cache();
}

/// Public API to check if Omarchy theme integration is active
pub fn is_theme_available() -> bool {
    OmarchyThemeManager::is_available()
}

pub fn load_omarchy_theme() -> Result<OmarchyColors> {
    let home = std::env::var("HOME").context("HOME not set")?;
    let theme_path = PathBuf::from(home).join(".config/omarchy/current/theme/colors.toml");

    let contents =
        fs::read_to_string(&theme_path).context("Failed to read Omarchy theme colors")?;

    toml::from_str(&contents).context("Failed to parse Omarchy theme TOML")
}

fn hex_to_rgb(hex: &str) -> Option<(f64, f64, f64)> {
    let hex = hex.trim_start_matches('#');
    if hex.len() != 6 {
        return None;
    }

    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;

    Some((r as f64 / 255.0, g as f64 / 255.0, b as f64 / 255.0))
}
