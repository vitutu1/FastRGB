//! Device LED geometry, canvas placement, and per-device lighting mode configuration.

use fastrgb_core::Rgb;
use serde::{Deserialize, Serialize};

/// Hardware device category matching FastRGB's `DeviceType()`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DeviceCategory {
    /// Keyboard matrix.
    Keyboard,
    /// Multi-zone gaming mouse.
    Mouse,
    /// Linear addressable RGB strip (including ARGB fans wired as strips).
    ArgbStrip,
    /// Circular/ring ARGB fan.
    Fan,
    /// Motherboard onboard / header controller.
    Motherboard,
    /// Addressable DDR4/DDR5 RAM modules.
    Ram,
    /// Discrete GPU lighting controller.
    Gpu,
    /// Custom or generic controller.
    Controller,
}

/// Single addressable LED coordinate inside a device's local grid (`[0..grid_width, 0..grid_height)`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LedPoint {
    /// Human-readable LED label (e.g. `"Esc"`, `"Ring-01"`).
    pub name: String,
    /// Local X coordinate inside the device grid.
    pub x: u32,
    /// Local Y coordinate inside the device grid.
    pub y: u32,
}

/// Physical/logical LED layout for a device (equivalent to FastRGB `Size()`, `LedNames()`, `LedPositions()`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceLayout {
    /// Unique stable device ID.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Manufacturer or preset author.
    pub vendor: String,
    /// Device category.
    pub category: DeviceCategory,
    /// Local grid width (`Size()[0]`).
    pub grid_width: u32,
    /// Local grid height (`Size()[1]`).
    pub grid_height: u32,
    /// Addressable LED coordinates.
    pub leds: Vec<LedPoint>,
}

/// Normalized `[0.0, 1.0]` placement of a device onto the global `320x200` effect canvas.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DevicePlacement {
    /// Top-left normalized X (`0.0..=1.0`).
    pub origin_nx: f32,
    /// Top-left normalized Y (`0.0..=1.0`).
    pub origin_ny: f32,
    /// Normalized width on the canvas (`0.05..=1.0`).
    pub width_nx: f32,
    /// Normalized height on the canvas (`0.05..=1.0`).
    pub height_ny: f32,
}

impl Default for DevicePlacement {
    fn default() -> Self {
        Self {
            origin_nx: 0.0,
            origin_ny: 0.0,
            width_nx: 1.0,
            height_ny: 1.0,
        }
    }
}

/// Lighting override mode matching FastRGB's `LightingMode` parameter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum LightingMode {
    /// Sample live colors from the active effect canvas.
    #[default]
    Canvas,
    /// Override all LEDs with a solid forced color.
    Forced(Rgb),
    /// Turn off all LEDs on this device.
    Off,
}
