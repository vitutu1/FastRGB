//! Fast CPU downsampling from screen frames into FastRGB's `28x20` HSL zones and
//! `160x100` HD buffer.

use fastrgb_core::{
    HD_HEIGHT, HD_WIDTH, Rgb, Rgba, ZONE_COLS, ZONE_COUNT, ZONE_ROWS, ZoneGrid, ZoneSample,
};

use crate::error::CaptureError;

/// Byte layout of a 4-byte-per-pixel captured frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelOrder {
    /// `[B, G, R, A]` (native Windows DXGI / GPUI order).
    Bgra,
    /// `[R, G, B, A]`.
    Rgba,
}

/// Extracts FastRGB `ZoneGrid` (`28x20` HSL + `160x100` RGBA) from raw screen frames.
#[derive(Debug, Default)]
pub struct ZoneExtractor;

impl ZoneExtractor {
    /// Extracts a [`ZoneGrid`] from a tightly packed 32-bit pixel buffer.
    ///
    /// # Errors
    ///
    /// Returns [`CaptureError::InvalidFrameBuffer`] if dimensions are zero or `pixels.len()`
    /// is smaller than `width * height * 4`.
    pub fn extract(
        width: u32,
        height: u32,
        order: PixelOrder,
        pixels: &[u8],
        sequence: u64,
    ) -> Result<ZoneGrid, CaptureError> {
        if width == 0 || height == 0 {
            return Err(CaptureError::InvalidFrameBuffer {
                width,
                height,
                reason: "dimensions must be non-zero".to_owned(),
            });
        }
        let expected_bytes = (width as usize)
            .checked_mul(height as usize)
            .and_then(|px| px.checked_mul(4))
            .ok_or_else(|| CaptureError::InvalidFrameBuffer {
                width,
                height,
                reason: "frame size overflow".to_owned(),
            })?;
        if pixels.len() < expected_bytes {
            return Err(CaptureError::InvalidFrameBuffer {
                width,
                height,
                reason: format!(
                    "expected at least {expected_bytes} bytes, got {}",
                    pixels.len()
                ),
            });
        }

        let hd_rgba = Self::downsample_hd(width, height, order, pixels);
        let zones = Self::compute_zones(&hd_rgba);
        Ok(ZoneGrid::new(zones, hd_rgba, sequence)?)
    }

    fn downsample_hd(width: u32, height: u32, order: PixelOrder, pixels: &[u8]) -> Vec<Rgba> {
        let mut out = Vec::with_capacity((HD_WIDTH * HD_HEIGHT) as usize);
        let src_w = width as usize;
        let src_h = height as usize;

        for y in 0..HD_HEIGHT as usize {
            let sy = ((y * src_h) / HD_HEIGHT as usize).min(src_h - 1);
            let row_offset = sy * src_w * 4;
            for x in 0..HD_WIDTH as usize {
                let sx = ((x * src_w) / HD_WIDTH as usize).min(src_w - 1);
                let i = row_offset + sx * 4;
                let rgba = match order {
                    PixelOrder::Bgra => Rgba::new(pixels[i + 2], pixels[i + 1], pixels[i], 255),
                    PixelOrder::Rgba => Rgba::new(pixels[i], pixels[i + 1], pixels[i + 2], 255),
                };
                out.push(rgba);
            }
        }
        out
    }

    fn compute_zones(hd_rgba: &[Rgba]) -> Vec<ZoneSample> {
        // Compute 28x20 zone averages directly from the 160x100 HD buffer so extraction stays
        // sub-millisecond even on 4K displays.
        let mut zones = Vec::with_capacity(ZONE_COUNT);
        let hd_w = HD_WIDTH as usize;
        let hd_h = HD_HEIGHT as usize;

        for row in 0..ZONE_ROWS {
            let y0 = (row * hd_h) / ZONE_ROWS;
            let y1 = (((row + 1) * hd_h) / ZONE_ROWS).max(y0 + 1);
            for col in 0..ZONE_COLS {
                let x0 = (col * hd_w) / ZONE_COLS;
                let x1 = (((col + 1) * hd_w) / ZONE_COLS).max(x0 + 1);

                let mut sr = 0u32;
                let mut sg = 0u32;
                let mut sb = 0u32;
                let mut count = 0u32;

                for y in y0..y1 {
                    let row_off = y * hd_w;
                    for x in x0..x1 {
                        let px = hd_rgba[row_off + x];
                        sr += u32::from(px.r);
                        sg += u32::from(px.g);
                        sb += u32::from(px.b);
                        count += 1;
                    }
                }

                let avg = Rgb::new((sr / count) as u8, (sg / count) as u8, (sb / count) as u8);
                zones.push(ZoneSample::from(avg.to_hsl()));
            }
        }

        zones
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extractor_produces_560_zones_and_hd_buffer() {
        let width = 320u32;
        let height = 200u32;
        let mut bgra = Vec::with_capacity((width * height * 4) as usize);
        for _ in 0..(width * height) {
            // BGRA for pure red (#FF0000)
            bgra.extend_from_slice(&[0, 0, 255, 255]);
        }

        let grid = ZoneExtractor::extract(width, height, PixelOrder::Bgra, &bgra, 7).unwrap();
        assert_eq!(grid.sequence, 7);
        assert_eq!(grid.zones.len(), ZONE_COUNT);
        assert_eq!(grid.hd_rgba.len(), (HD_WIDTH * HD_HEIGHT) as usize);
        assert_eq!(grid.hd_rgba[0], Rgba::new(255, 0, 0, 255));
        assert!((grid.zones[0].hue - 0.0).abs() < 0.5);
        assert!(grid.zones[0].saturation > 99.0);
    }
}
