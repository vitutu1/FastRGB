//! Fast $O(W \cdot H)$ CPU post-processing filters (color adjustments and sliding-window blur).

use fastrgb_core::{CanvasFrame, Hsl, Rgb, Rgba};

/// Color filter parameters matching CSS filter semantics in FastRGB's `Screen Ambience`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColorFilter {
    /// Hue rotation in degrees (`-180.0..=180.0`).
    pub hue_shift_deg: f32,
    /// Brightness delta percentage (`-100.0..=100.0`, where `0` = `100%`).
    pub brightness_pct: f32,
    /// Saturation delta percentage (`-100.0..=100.0`, where `0` = `100%`).
    pub saturation_pct: f32,
    /// Contrast delta percentage (`-100.0..=100.0`, where `0` = `100%`).
    pub contrast_pct: f32,
}

impl ColorFilter {
    /// Returns `true` when all parameters are neutral (`0`).
    #[must_use]
    pub fn is_identity(self) -> bool {
        self.hue_shift_deg.abs() < 0.01
            && self.brightness_pct.abs() < 0.01
            && self.saturation_pct.abs() < 0.01
            && self.contrast_pct.abs() < 0.01
    }

    /// Applies hue shift, saturation, brightness, and contrast to a single [`Rgb`] pixel.
    #[must_use]
    pub fn apply(self, rgb: Rgb) -> Rgb {
        let mut r = f32::from(rgb.r);
        let mut g = f32::from(rgb.g);
        let mut b = f32::from(rgb.b);

        if self.hue_shift_deg.abs() >= 0.01 || self.saturation_pct.abs() >= 0.01 {
            let hsl = Rgb::new(r as u8, g as u8, b as u8).to_hsl();
            let sat_scale = ((100.0 + self.saturation_pct) / 100.0).max(0.0);
            let shifted = Hsl::new(
                hsl.h + self.hue_shift_deg,
                (hsl.s * sat_scale).clamp(0.0, 100.0),
                hsl.l,
            )
            .to_rgb();
            r = f32::from(shifted.r);
            g = f32::from(shifted.g);
            b = f32::from(shifted.b);
        }

        let bright_scale = ((100.0 + self.brightness_pct) / 100.0).max(0.0);
        r *= bright_scale;
        g *= bright_scale;
        b *= bright_scale;

        let contrast_scale = ((100.0 + self.contrast_pct) / 100.0).max(0.0);
        if (contrast_scale - 1.0).abs() >= 0.001 {
            r = (r - 127.5) * contrast_scale + 127.5;
            g = (g - 127.5) * contrast_scale + 127.5;
            b = (b - 127.5) * contrast_scale + 127.5;
        }

        Rgb::new(
            r.round().clamp(0.0, 255.0) as u8,
            g.round().clamp(0.0, 255.0) as u8,
            b.round().clamp(0.0, 255.0) as u8,
        )
    }
}

/// Applies [`ColorFilter`] in-place across every pixel of `canvas`.
pub fn apply_color_filter(canvas: &mut CanvasFrame, filter: ColorFilter) {
    if filter.is_identity() {
        return;
    }
    for px in canvas.pixels_mut() {
        let out = filter.apply(px.to_rgb());
        *px = out.to_rgba(px.a);
    }
}

/// Applies an $O(W \cdot H)$ sliding-window separable blur (two passes for Gaussian-like
/// smoothness when `radius > 2`), independent of blur radius.
pub fn apply_box_blur(canvas: &mut CanvasFrame, radius: u32, scratch: &mut Vec<Rgba>) {
    if radius == 0 {
        return;
    }
    let w = canvas.width() as usize;
    let h = canvas.height() as usize;
    if w == 0 || h == 0 {
        return;
    }

    let clamped = radius.min(20) as usize;
    let pixels = canvas.pixels_mut();
    scratch.resize(pixels.len(), Rgba::BLACK);

    if clamped <= 2 {
        blur_pass(pixels, scratch, w, h, clamped);
    } else {
        // Two smaller passes approximate a smooth Gaussian falloff at O(W*H) cost.
        let r1 = (clamped / 2).max(1);
        let r2 = (clamped - r1).max(1);
        blur_pass(pixels, scratch, w, h, r1);
        blur_pass(pixels, scratch, w, h, r2);
    }
}

fn blur_pass(pixels: &mut [Rgba], scratch: &mut [Rgba], w: usize, h: usize, r: usize) {
    let win = (2 * r + 1) as u32;
    let half = win / 2;

    // Horizontal sliding window: pixels -> scratch
    for y in 0..h {
        let row = &pixels[y * w..(y + 1) * w];
        let dst = &mut scratch[y * w..(y + 1) * w];

        let mut sr = u32::from(row[0].r) * (r as u32 + 1);
        let mut sg = u32::from(row[0].g) * (r as u32 + 1);
        let mut sb = u32::from(row[0].b) * (r as u32 + 1);
        for dx in 1..=r {
            let p = row[dx.min(w - 1)];
            sr += u32::from(p.r);
            sg += u32::from(p.g);
            sb += u32::from(p.b);
        }

        for x in 0..w {
            dst[x] = Rgba::new(
                ((sr + half) / win) as u8,
                ((sg + half) / win) as u8,
                ((sb + half) / win) as u8,
                255,
            );
            let enter = row[(x + r + 1).min(w - 1)];
            let leave = row[x.saturating_sub(r)];
            sr = sr + u32::from(enter.r) - u32::from(leave.r);
            sg = sg + u32::from(enter.g) - u32::from(leave.g);
            sb = sb + u32::from(enter.b) - u32::from(leave.b);
        }
    }

    // Vertical sliding window: scratch -> pixels
    for x in 0..w {
        let first = scratch[x];
        let mut sr = u32::from(first.r) * (r as u32 + 1);
        let mut sg = u32::from(first.g) * (r as u32 + 1);
        let mut sb = u32::from(first.b) * (r as u32 + 1);
        for dy in 1..=r {
            let p = scratch[dy.min(h - 1) * w + x];
            sr += u32::from(p.r);
            sg += u32::from(p.g);
            sb += u32::from(p.b);
        }

        for y in 0..h {
            pixels[y * w + x] = Rgba::new(
                ((sr + half) / win) as u8,
                ((sg + half) / win) as u8,
                ((sb + half) / win) as u8,
                255,
            );
            let enter = scratch[(y + r + 1).min(h - 1) * w + x];
            let leave = scratch[y.saturating_sub(r) * w + x];
            sr = sr + u32::from(enter.r) - u32::from(leave.r);
            sg = sg + u32::from(enter.g) - u32::from(leave.g);
            sb = sb + u32::from(enter.b) - u32::from(leave.b);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sliding_window_blur_preserves_solid_color_and_smooths_edges() {
        let mut solid = CanvasFrame::new(32, 20).unwrap();
        let target = Rgba::new(42, 180, 230, 255);
        solid.fill(target);
        let mut scratch = Vec::new();
        apply_box_blur(&mut solid, 12, &mut scratch);
        assert_eq!(solid.get_pixel(0, 0).unwrap(), target);
        assert_eq!(solid.get_pixel(16, 10).unwrap(), target);

        let mut step = CanvasFrame::new(32, 20).unwrap();
        step.fill_rect(16, 0, 16, 20, Rgba::new(255, 255, 255, 255));
        apply_box_blur(&mut step, 6, &mut scratch);
        let mid_left = step.get_pixel(14, 10).unwrap().r;
        let mid_right = step.get_pixel(17, 10).unwrap().r;
        assert!(mid_left > 10 && mid_left < 200);
        assert!(mid_right > 100 && mid_right < 250);
    }
}
