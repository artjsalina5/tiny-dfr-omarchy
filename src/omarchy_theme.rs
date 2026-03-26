use anyhow::{Context, Result};
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

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
