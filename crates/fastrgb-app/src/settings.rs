//! Persistent user preferences stored as JSON under `%APPDATA%\FastRGB\settings.json`.

use std::fs;
use std::path::PathBuf;

use fastrgb_hardware::{MsiGpuControlMode, RgbOrder};
use serde::{Deserialize, Serialize};

use crate::state::MAX_RGB_FPS;

/// Persistent user-configurable FastRGB settings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AppSettings {
    /// Launch hidden in the Windows System Tray when started automatically or manually.
    pub start_minimized: bool,
    /// Automatically attach live HID / NVAPI / SMBus output on startup when no contention is found.
    pub live_on_startup: bool,
    /// Render the interactive `DotField` canvas backdrop behind studio surfaces.
    pub dot_field_enabled: bool,
    /// Default RGB simulation & hardware bus cap (`None` = `60 FPS` max, or `Some(5..=60)`).
    pub rgb_fps_cap: Option<u32>,
    /// Last active master brightness (`0.05..=1.0`).
    pub brightness: f32,
    /// Last selected effect ID restored on next launch.
    pub last_effect_id: String,
    /// Persisted ASUS Aura ARGB wire byte order (`RGB`, `GRB`, ...).
    pub asus_argb_order: String,
    /// Persisted MSI Lovelace GPU I2C control mode (`SingleZone` or `PerLed`).
    pub msi_gpu_mode: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            start_minimized: true,
            live_on_startup: false,
            dot_field_enabled: true,
            rgb_fps_cap: None,
            brightness: 1.0,
            last_effect_id: "rainbow".to_owned(),
            asus_argb_order: "RGB".to_owned(),
            msi_gpu_mode: "SingleZone".to_owned(),
        }
    }
}

impl AppSettings {
    /// Resolves the per-user settings file path (`%APPDATA%\FastRGB\settings.json`).
    #[must_use]
    pub fn file_path() -> Option<PathBuf> {
        let base = std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
        Some(base.join("FastRGB").join("settings.json"))
    }

    /// Loads persisted settings from disk, falling back cleanly to [`AppSettings::default`].
    #[must_use]
    pub fn load() -> Self {
        let Some(path) = Self::file_path() else {
            return Self::default();
        };
        let Ok(raw) = fs::read_to_string(path) else {
            return Self::default();
        };
        serde_json::from_str::<Self>(&raw)
            .map(Self::sanitized)
            .unwrap_or_default()
    }

    /// Saves settings atomically to `%APPDATA%\FastRGB\settings.json`.
    pub fn save(&self) {
        let Some(path) = Self::file_path() else {
            return;
        };
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = fs::write(path, json);
        }
    }

    /// Parses the stored `asus_argb_order` string into a typed [`RgbOrder`].
    #[must_use]
    pub fn parsed_argb_order(&self) -> RgbOrder {
        match self.asus_argb_order.to_ascii_uppercase().as_str() {
            "GRB" => RgbOrder::Grb,
            "BGR" => RgbOrder::Bgr,
            "BRG" => RgbOrder::Brg,
            "RBG" => RgbOrder::Rbg,
            "GBR" => RgbOrder::Gbr,
            _ => RgbOrder::Rgb,
        }
    }

    /// Parses the stored `msi_gpu_mode` string into a typed [`MsiGpuControlMode`].
    #[must_use]
    pub fn parsed_msi_gpu_mode(&self) -> MsiGpuControlMode {
        match self.msi_gpu_mode.to_ascii_lowercase().as_str() {
            "perled" | "per-led" | "per_led" => MsiGpuControlMode::PerLed,
            _ => MsiGpuControlMode::SingleZone,
        }
    }

    fn sanitized(mut self) -> Self {
        self.brightness = self.brightness.clamp(0.05, 1.0);
        self.rgb_fps_cap = self.rgb_fps_cap.and_then(|f| {
            let clamped = f.clamp(5, MAX_RGB_FPS);
            if clamped >= MAX_RGB_FPS {
                None
            } else {
                Some(clamped)
            }
        });
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_roundtrip_and_sanitization() -> Result<(), serde_json::Error> {
        let s = AppSettings {
            start_minimized: false,
            live_on_startup: true,
            dot_field_enabled: false,
            rgb_fps_cap: Some(144),
            brightness: 2.5,
            last_effect_id: "matrix".to_owned(),
            asus_argb_order: "GRB".to_owned(),
            msi_gpu_mode: "PerLed".to_owned(),
        };
        let json = serde_json::to_string(&s)?;
        let parsed = serde_json::from_str::<AppSettings>(&json)?.sanitized();
        assert_eq!(parsed.rgb_fps_cap, None);
        assert!((parsed.brightness - 1.0).abs() < 1e-5);
        assert_eq!(parsed.parsed_argb_order(), RgbOrder::Grb);
        assert_eq!(parsed.parsed_msi_gpu_mode(), MsiGpuControlMode::PerLed);
        Ok(())
    }
}
