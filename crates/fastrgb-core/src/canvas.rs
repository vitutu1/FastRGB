//! Offscreen RGB canvas buffer (`320x200` standard FastRGB resolution).

use crate::color::{Rgb, Rgba};
use crate::error::CoreError;

/// Default FastRGB effect canvas width in pixels.
pub const DEFAULT_CANVAS_WIDTH: u32 = 320;
/// Default FastRGB effect canvas height in pixels.
pub const DEFAULT_CANVAS_HEIGHT: u32 = 200;

/// Reusable pixel buffer representing the rendered effect canvas.
///
/// Effects draw into this buffer and device samplers read LED colors from it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanvasFrame {
    width: u32,
    height: u32,
    pixels: Vec<Rgba>,
}

impl Default for CanvasFrame {
    fn default() -> Self {
        Self {
            width: DEFAULT_CANVAS_WIDTH,
            height: DEFAULT_CANVAS_HEIGHT,
            pixels: vec![Rgba::BLACK; (DEFAULT_CANVAS_WIDTH * DEFAULT_CANVAS_HEIGHT) as usize],
        }
    }
}

impl CanvasFrame {
    /// Creates a new black canvas with the specified dimensions.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::InvalidCanvasDimensions`] if width or height is zero.
    pub fn new(width: u32, height: u32) -> Result<Self, CoreError> {
        if width == 0 || height == 0 {
            return Err(CoreError::InvalidCanvasDimensions { width, height });
        }
        let len = (width as usize)
            .checked_mul(height as usize)
            .ok_or(CoreError::InvalidCanvasDimensions { width, height })?;
        Ok(Self {
            width,
            height,
            pixels: vec![Rgba::BLACK; len],
        })
    }

    /// Creates a standard `320x200` FastRGB canvas.
    #[must_use]
    pub fn standard() -> Self {
        Self::default()
    }

    /// Canvas width in pixels.
    #[must_use]
    pub const fn width(&self) -> u32 {
        self.width
    }

    /// Canvas height in pixels.
    #[must_use]
    pub const fn height(&self) -> u32 {
        self.height
    }

    /// Clears the entire canvas to opaque black.
    pub fn clear(&mut self) {
        self.pixels.fill(Rgba::BLACK);
    }

    /// Fills the entire canvas with an opaque color.
    pub fn fill(&mut self, color: Rgba) {
        self.pixels.fill(color);
    }

    /// Alpha-blends a single color over the entire canvas.
    pub fn blend_all(&mut self, src: Rgba) {
        if src.a == 255 {
            self.fill(src);
            return;
        }
        if src.a == 0 {
            return;
        }
        for px in &mut self.pixels {
            *px = px.blend_over(src);
        }
    }

    /// Sets a pixel at `(x, y)` if inside bounds.
    pub fn set_pixel(&mut self, x: i32, y: i32, color: Rgba) {
        if x < 0 || y < 0 {
            return;
        }
        let ux = x as u32;
        let uy = y as u32;
        if ux >= self.width || uy >= self.height {
            return;
        }
        let idx = (uy * self.width + ux) as usize;
        self.pixels[idx] = color;
    }

    /// Alpha-blends a single pixel at `(x, y)` if inside bounds.
    pub fn blend_pixel(&mut self, x: i32, y: i32, color: Rgba) {
        if color.a == 0 || x < 0 || y < 0 {
            return;
        }
        let ux = x as u32;
        let uy = y as u32;
        if ux >= self.width || uy >= self.height {
            return;
        }
        let idx = (uy * self.width + ux) as usize;
        self.pixels[idx] = self.pixels[idx].blend_over(color);
    }

    /// Draws a filled circle centered at `(cx, cy)` with `radius`, clipped to canvas bounds.
    pub fn fill_circle(&mut self, cx: f32, cy: f32, radius: f32, color: Rgba) {
        if radius <= 0.0 || color.a == 0 {
            return;
        }
        let r2 = radius * radius;
        let x0 = ((cx - radius).floor() as i32).max(0);
        let y0 = ((cy - radius).floor() as i32).max(0);
        let x1 = ((cx + radius).ceil() as i32).min(self.width as i32 - 1);
        let y1 = ((cy + radius).ceil() as i32).min(self.height as i32 - 1);
        for y in y0..=y1 {
            let dy = (y as f32 + 0.5) - cy;
            for x in x0..=x1 {
                let dx = (x as f32 + 0.5) - cx;
                if dx * dx + dy * dy <= r2 {
                    self.blend_pixel(x, y, color);
                }
            }
        }
    }

    /// Draws a thick line segment from `(x0, y0)` to `(x1, y1)` using circular brush stamps.
    pub fn draw_line(&mut self, x0: f32, y0: f32, x1: f32, y1: f32, thickness: f32, color: Rgba) {
        let dx = x1 - x0;
        let dy = y1 - y0;
        let dist = (dx * dx + dy * dy).sqrt();
        let steps = (dist.ceil() as usize).max(1);
        let radius = (thickness * 0.5).max(0.75);
        for i in 0..=steps {
            let t = i as f32 / steps as f32;
            self.fill_circle(x0 + dx * t, y0 + dy * t, radius, color);
        }
    }

    /// Returns the pixel at `(x, y)` if inside bounds.
    #[must_use]
    pub fn get_pixel(&self, x: u32, y: u32) -> Option<Rgba> {
        if x >= self.width || y >= self.height {
            return None;
        }
        Some(self.pixels[(y * self.width + x) as usize])
    }

    /// Fills an axis-aligned rectangle clipped to canvas bounds (opaque or alpha-blended).
    pub fn fill_rect(&mut self, x: i32, y: i32, w: i32, h: i32, color: Rgba) {
        if w <= 0 || h <= 0 || color.a == 0 {
            return;
        }
        let x0 = x.max(0) as u32;
        let y0 = y.max(0) as u32;
        let x1 = (x.saturating_add(w)).min(self.width as i32).max(0) as u32;
        let y1 = (y.saturating_add(h)).min(self.height as i32).max(0) as u32;
        if x0 >= x1 || y0 >= y1 {
            return;
        }

        let stride = self.width as usize;
        if color.a == 255 {
            for row in y0..y1 {
                let start = (row as usize) * stride + (x0 as usize);
                let end = (row as usize) * stride + (x1 as usize);
                self.pixels[start..end].fill(color);
            }
        } else {
            for row in y0..y1 {
                let start = (row as usize) * stride + (x0 as usize);
                let end = (row as usize) * stride + (x1 as usize);
                for px in &mut self.pixels[start..end] {
                    *px = px.blend_over(color);
                }
            }
        }
    }

    /// Samples a color using normalized `[0.0, 1.0]` coordinates.
    #[must_use]
    pub fn sample_normalized(&self, nx: f32, ny: f32) -> Rgb {
        let x = ((nx.clamp(0.0, 1.0)) * (self.width.saturating_sub(1) as f32)).round() as u32;
        let y = ((ny.clamp(0.0, 1.0)) * (self.height.saturating_sub(1) as f32)).round() as u32;
        self.get_pixel(x, y).unwrap_or(Rgba::BLACK).to_rgb()
    }

    /// Samples a color at a virtual device grid coordinate `(gx, gy)` within a `(gw, gh)` layout.
    #[must_use]
    pub fn sample_grid(&self, gx: u32, gy: u32, gw: u32, gh: u32) -> Rgb {
        let nx = if gw <= 1 {
            0.5
        } else {
            gx as f32 / (gw - 1) as f32
        };
        let ny = if gh <= 1 {
            0.5
        } else {
            gy as f32 / (gh - 1) as f32
        };
        self.sample_normalized(nx, ny)
    }

    /// Returns a slice of raw pixels.
    #[must_use]
    pub fn pixels(&self) -> &[Rgba] {
        &self.pixels
    }

    /// Returns a mutable slice of raw pixels.
    pub fn pixels_mut(&mut self) -> &mut [Rgba] {
        &mut self.pixels
    }

    /// Writes packed BGRA8 bytes into `out` (format expected by GPUI `RenderImage`).
    pub fn write_bgra_bytes(&self, out: &mut Vec<u8>) {
        out.clear();
        out.reserve(self.pixels.len() * 4);
        for px in &self.pixels {
            out.push(px.b);
            out.push(px.g);
            out.push(px.r);
            out.push(px.a);
        }
    }

    /// Returns packed RGBA8 bytes for PNG encoding.
    #[must_use]
    pub fn to_rgba_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.pixels.len() * 4);
        for px in &self.pixels {
            out.push(px.r);
            out.push(px.g);
            out.push(px.b);
            out.push(px.a);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fill_rect_clips_safely() {
        let mut canvas = CanvasFrame::new(10, 10).unwrap();
        canvas.fill_rect(-5, -5, 8, 8, Rgb::new(255, 10, 20).to_rgba(255));
        assert_eq!(canvas.get_pixel(0, 0).unwrap().r, 255);
        assert_eq!(canvas.get_pixel(2, 2).unwrap().r, 255);
        assert_eq!(canvas.get_pixel(3, 3).unwrap().r, 0);
    }
}
