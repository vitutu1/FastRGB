//! Screen zone representation matching FastRGB's `engine.zone` contract.

use crate::color::{Hsl, Rgba};
use crate::error::CoreError;

/// Standard FastRGB zone grid column count.
pub const ZONE_COLS: usize = 28;
/// Standard FastRGB zone grid row count.
pub const ZONE_ROWS: usize = 20;
/// Total zone count (`28 * 20 = 560`).
pub const ZONE_COUNT: usize = ZONE_COLS * ZONE_ROWS;

/// Downsampled HD screen buffer width for `Screen Ambience` HD mode.
pub const HD_WIDTH: u32 = 160;
/// Downsampled HD screen buffer height for `Screen Ambience` HD mode.
pub const HD_HEIGHT: u32 = 100;

/// Average HSL values for a single screen zone.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ZoneSample {
    /// Hue in degrees (`0.0..360.0`).
    pub hue: f32,
    /// Saturation percentage (`0.0..=100.0`).
    pub saturation: f32,
    /// Lightness percentage (`0.0..=100.0`).
    pub lightness: f32,
}

impl From<Hsl> for ZoneSample {
    fn from(value: Hsl) -> Self {
        Self {
            hue: value.h,
            saturation: value.s,
            lightness: value.l,
        }
    }
}

/// Extracted screen data consumed by screen-reactive effects.
#[derive(Debug, Clone, PartialEq)]
pub struct ZoneGrid {
    /// `28x20` (`560`) zone HSL averages in row-major order.
    pub zones: Vec<ZoneSample>,
    /// `160x100` downsampled RGBA image buffer.
    pub hd_rgba: Vec<Rgba>,
    /// Monotonic capture sequence counter.
    pub sequence: u64,
}

impl Default for ZoneGrid {
    fn default() -> Self {
        Self {
            zones: vec![ZoneSample::default(); ZONE_COUNT],
            hd_rgba: vec![Rgba::BLACK; (HD_WIDTH * HD_HEIGHT) as usize],
            sequence: 0,
        }
    }
}

impl ZoneGrid {
    /// Constructs a validated [`ZoneGrid`].
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::InvalidZoneBufferLength`] if either buffer size does not match
    /// `ZONE_COUNT` or `HD_WIDTH * HD_HEIGHT`.
    pub fn new(
        zones: Vec<ZoneSample>,
        hd_rgba: Vec<Rgba>,
        sequence: u64,
    ) -> Result<Self, CoreError> {
        if zones.len() != ZONE_COUNT {
            return Err(CoreError::InvalidZoneBufferLength {
                expected: ZONE_COUNT,
                actual: zones.len(),
            });
        }
        let expected_hd = (HD_WIDTH * HD_HEIGHT) as usize;
        if hd_rgba.len() != expected_hd {
            return Err(CoreError::InvalidZoneBufferLength {
                expected: expected_hd,
                actual: hd_rgba.len(),
            });
        }
        Ok(Self {
            zones,
            hd_rgba,
            sequence,
        })
    }
}
