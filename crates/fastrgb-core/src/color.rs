//! Color models and color space conversions (`Rgb`, `Rgba`, `Hsl`).

use serde::{Deserialize, Serialize};

use crate::error::CoreError;

/// 24-bit sRGB color.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Rgb {
    /// Red channel (`0..=255`).
    pub r: u8,
    /// Green channel (`0..=255`).
    pub g: u8,
    /// Blue channel (`0..=255`).
    pub b: u8,
}

impl Rgb {
    /// Black (`#000000`).
    pub const BLACK: Self = Self { r: 0, g: 0, b: 0 };
    /// White (`#FFFFFF`).
    pub const WHITE: Self = Self {
        r: 255,
        g: 255,
        b: 255,
    };

    /// Creates a new [`Rgb`] color.
    #[must_use]
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    /// Parses a `#RRGGBB` or `RRGGBB` hexadecimal color string.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::InvalidHexColor`] if the input is not 6 hex digits (with optional `#`).
    pub fn from_hex(hex: &str) -> Result<Self, CoreError> {
        let trimmed = hex.trim();
        let digits = trimmed.strip_prefix('#').unwrap_or(trimmed);
        if digits.len() != 6 {
            return Err(CoreError::InvalidHexColor(hex.to_owned()));
        }
        let r = u8::from_str_radix(&digits[0..2], 16)
            .map_err(|_| CoreError::InvalidHexColor(hex.to_owned()))?;
        let g = u8::from_str_radix(&digits[2..4], 16)
            .map_err(|_| CoreError::InvalidHexColor(hex.to_owned()))?;
        let b = u8::from_str_radix(&digits[4..6], 16)
            .map_err(|_| CoreError::InvalidHexColor(hex.to_owned()))?;
        Ok(Self { r, g, b })
    }

    /// Formats the color as an uppercase `#RRGGBB` string.
    #[must_use]
    pub fn to_hex(self) -> String {
        format!("#{:02X}{:02X}{:02X}", self.r, self.g, self.b)
    }

    /// Converts this color to [`Rgba`] with full opacity (`255`).
    #[must_use]
    pub const fn to_rgba(self, a: u8) -> Rgba {
        Rgba {
            r: self.r,
            g: self.g,
            b: self.b,
            a,
        }
    }

    /// Scales all channels by `factor` clamped to `0.0..=1.0`.
    #[must_use]
    pub fn scale_brightness(self, factor: f32) -> Self {
        let f = factor.clamp(0.0, 1.0);
        Self {
            r: (f32::from(self.r) * f).round().clamp(0.0, 255.0) as u8,
            g: (f32::from(self.g) * f).round().clamp(0.0, 255.0) as u8,
            b: (f32::from(self.b) * f).round().clamp(0.0, 255.0) as u8,
        }
    }

    /// Linearly interpolates between `self` and `other` with `t` clamped to `0.0..=1.0`.
    #[must_use]
    pub fn lerp(self, other: Self, t: f32) -> Self {
        let t = t.clamp(0.0, 1.0);
        let inv = 1.0 - t;
        Self {
            r: (f32::from(self.r) * inv + f32::from(other.r) * t).round() as u8,
            g: (f32::from(self.g) * inv + f32::from(other.g) * t).round() as u8,
            b: (f32::from(self.b) * inv + f32::from(other.b) * t).round() as u8,
        }
    }

    /// Converts this RGB color to [`Hsl`] (`h: 0..360`, `s: 0..=100`, `l: 0..=100`).
    #[must_use]
    pub fn to_hsl(self) -> Hsl {
        let r = f32::from(self.r) / 255.0;
        let g = f32::from(self.g) / 255.0;
        let b = f32::from(self.b) / 255.0;

        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let delta = max - min;
        let l = (max + min) * 0.5;

        if delta <= f32::EPSILON {
            return Hsl::new(0.0, 0.0, l * 100.0);
        }

        let denom = 1.0 - (2.0 * l - 1.0).abs();
        let s = if denom > f32::EPSILON {
            (delta / denom).clamp(0.0, 1.0)
        } else {
            0.0
        };

        let raw_h = if (max - r).abs() <= f32::EPSILON {
            ((g - b) / delta).rem_euclid(6.0)
        } else if (max - g).abs() <= f32::EPSILON {
            (b - r) / delta + 2.0
        } else {
            (r - g) / delta + 4.0
        };

        Hsl::new(raw_h * 60.0, s * 100.0, l * 100.0)
    }

    /// Converts packed `0xRRGGBB` into [`Rgb`].
    #[must_use]
    pub const fn from_u32(hex: u32) -> Self {
        Self {
            r: ((hex >> 16) & 0xFF) as u8,
            g: ((hex >> 8) & 0xFF) as u8,
            b: (hex & 0xFF) as u8,
        }
    }

    /// Packs this color into `0xRRGGBB`.
    #[must_use]
    pub const fn to_u32(self) -> u32 {
        ((self.r as u32) << 16) | ((self.g as u32) << 8) | (self.b as u32)
    }
}

/// 32-bit RGBA color.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Rgba {
    /// Red channel (`0..=255`).
    pub r: u8,
    /// Green channel (`0..=255`).
    pub g: u8,
    /// Blue channel (`0..=255`).
    pub b: u8,
    /// Alpha channel (`0..=255`).
    pub a: u8,
}

impl Rgba {
    /// Opaque black (`#000000FF`).
    pub const BLACK: Self = Self {
        r: 0,
        g: 0,
        b: 0,
        a: 255,
    };

    /// Creates an [`Rgba`] value.
    #[must_use]
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    /// Drops the alpha channel and returns [`Rgb`].
    #[must_use]
    pub const fn to_rgb(self) -> Rgb {
        Rgb {
            r: self.r,
            g: self.g,
            b: self.b,
        }
    }

    /// Composites `src` over `self` using straight-alpha Porter-Duff `source-over`.
    #[must_use]
    pub fn blend_over(self, src: Self) -> Self {
        if src.a == 255 {
            return src;
        }
        if src.a == 0 {
            return self;
        }
        let a = f32::from(src.a) / 255.0;
        let inv = 1.0 - a;
        Self {
            r: (f32::from(src.r) * a + f32::from(self.r) * inv)
                .round()
                .clamp(0.0, 255.0) as u8,
            g: (f32::from(src.g) * a + f32::from(self.g) * inv)
                .round()
                .clamp(0.0, 255.0) as u8,
            b: (f32::from(src.b) * a + f32::from(self.b) * inv)
                .round()
                .clamp(0.0, 255.0) as u8,
            a: 255,
        }
    }
}

/// HSL color representation matching CSS/FastRGB semantics.
///
/// - `h`: Hue in degrees, normalized to `[0.0, 360.0)`.
/// - `s`: Saturation percentage in `0.0..=100.0`.
/// - `l`: Lightness percentage in `0.0..=100.0`.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct Hsl {
    /// Hue in degrees (`[0.0, 360.0)`).
    pub h: f32,
    /// Saturation (`0.0..=100.0`).
    pub s: f32,
    /// Lightness (`0.0..=100.0`).
    pub l: f32,
}

impl Hsl {
    /// Creates an [`Hsl`] color with normalized hue and clamped saturation/lightness.
    #[must_use]
    pub fn new(h: f32, s: f32, l: f32) -> Self {
        Self {
            h: h.rem_euclid(360.0),
            s: s.clamp(0.0, 100.0),
            l: l.clamp(0.0, 100.0),
        }
    }

    /// Converts this HSL color to [`Rgb`].
    #[must_use]
    pub fn to_rgb(self) -> Rgb {
        let h = self.h.rem_euclid(360.0);
        let s = (self.s / 100.0).clamp(0.0, 1.0);
        let l = (self.l / 100.0).clamp(0.0, 1.0);

        if s <= f32::EPSILON {
            let v = (l * 255.0).round().clamp(0.0, 255.0) as u8;
            return Rgb::new(v, v, v);
        }

        let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
        let hp = h / 60.0;
        let x = c * (1.0 - ((hp % 2.0) - 1.0).abs());
        let (r1, g1, b1) = if hp < 1.0 {
            (c, x, 0.0)
        } else if hp < 2.0 {
            (x, c, 0.0)
        } else if hp < 3.0 {
            (0.0, c, x)
        } else if hp < 4.0 {
            (0.0, x, c)
        } else if hp < 5.0 {
            (x, 0.0, c)
        } else {
            (c, 0.0, x)
        };

        let m = l - c * 0.5;
        Rgb::new(
            ((r1 + m) * 255.0).round().clamp(0.0, 255.0) as u8,
            ((g1 + m) * 255.0).round().clamp(0.0, 255.0) as u8,
            ((b1 + m) * 255.0).round().clamp(0.0, 255.0) as u8,
        )
    }

    /// Shortest-arc angular interpolation between two hues in degrees (`[0.0, 360.0)`).
    #[must_use]
    pub fn lerp_hue(prev: f32, next: f32, alpha: f32) -> f32 {
        let mut diff = next - prev;
        if diff > 180.0 {
            diff -= 360.0;
        } else if diff < -180.0 {
            diff += 360.0;
        }
        (prev + diff * alpha.clamp(0.0, 1.0)).rem_euclid(360.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_roundtrip() {
        let color = Rgb::from_hex("#00bbff").unwrap();
        assert_eq!(color, Rgb::new(0x00, 0xBB, 0xFF));
        assert_eq!(color.to_hex(), "#00BBFF");
    }

    #[test]
    fn hsl_rgb_roundtrip_primaries() {
        for rgb in [
            Rgb::new(255, 0, 0),
            Rgb::new(0, 255, 0),
            Rgb::new(0, 0, 255),
            Rgb::new(0, 187, 255),
            Rgb::new(128, 128, 128),
        ] {
            let back = rgb.to_hsl().to_rgb();
            assert!((i16::from(rgb.r) - i16::from(back.r)).abs() <= 1);
            assert!((i16::from(rgb.g) - i16::from(back.g)).abs() <= 1);
            assert!((i16::from(rgb.b) - i16::from(back.b)).abs() <= 1);
        }
    }

    #[test]
    fn shortest_arc_hue_lerp_wraps_correctly() {
        let mid = Hsl::lerp_hue(350.0, 10.0, 0.5);
        assert!((mid - 0.0).abs() < 0.001 || (mid - 360.0).abs() < 0.001);
    }
}
