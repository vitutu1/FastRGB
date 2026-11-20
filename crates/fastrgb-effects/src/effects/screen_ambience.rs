//! `Screen Ambience` dynamic monitor-mirroring effect.

use fastrgb_core::{
    CanvasFrame, HD_HEIGHT, HD_WIDTH, Hsl, ParameterDescriptor, Rgba, ZONE_COLS, ZONE_COUNT,
    ZONE_ROWS, ZoneGrid,
};

use crate::postprocess::{ColorFilter, apply_box_blur, apply_color_filter};
use crate::traits::{Effect, EffectCategory, EffectContext, EffectMetadata};

/// Real-time screen-mirroring lighting effect ported from FastRGB's `Screen Ambience`.
#[derive(Debug, Default)]
pub struct ScreenAmbienceEffect {
    dominant_hue: Option<f32>,
    prev_zones: Vec<[f32; 3]>,
    prev_hd_pixels: Vec<[f32; 3]>,
    blur_scratch: Vec<Rgba>,
}

impl ScreenAmbienceEffect {
    /// Creates a new [`ScreenAmbienceEffect`].
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    fn smoothing_alpha(motion_smoothing: f32) -> f32 {
        if motion_smoothing <= 0.0 {
            1.0
        } else {
            (1.0 - motion_smoothing / 100.0).max(0.05)
        }
    }

    fn render_standard_zones(
        &mut self,
        grid: &ZoneGrid,
        picture_mode: &str,
        alpha: f32,
        canvas: &mut CanvasFrame,
    ) {
        if self.prev_zones.len() != ZONE_COUNT {
            self.prev_zones = grid
                .zones
                .iter()
                .map(|z| [z.hue, z.saturation, z.lightness])
                .collect();
        }

        let is_mono = picture_mode == "Mono";
        let is_vivid = picture_mode == "Vivid";
        let is_cinema = picture_mode == "Cinema";
        let is_dominant = picture_mode == "Dominant";

        if is_dominant {
            let mut dom_zone = 0usize;
            let mut dom_sat = 0.0_f32;
            for (idx, zone) in grid.zones.iter().enumerate() {
                if zone.lightness > 30.0 && zone.saturation > dom_sat {
                    dom_sat = zone.saturation;
                    dom_zone = idx;
                }
            }
            let target_hue = grid.zones[dom_zone].hue;
            match self.dominant_hue {
                None => self.dominant_hue = Some(target_hue),
                Some(ref mut hue) => {
                    if target_hue > *hue {
                        *hue = (*hue + 1.0).min(target_hue);
                    } else if target_hue < *hue {
                        *hue = (*hue - 1.0).max(target_hue);
                    }
                }
            }
        }

        let cell_w = (canvas.width() as usize) / ZONE_COLS;
        let cell_h = (canvas.height() as usize) / ZONE_ROWS;
        let smooth = alpha < 1.0;
        let inv = 1.0 - alpha;

        for (idx, zone) in grid.zones.iter().enumerate() {
            let mut hue = zone.hue;
            let mut sat = zone.saturation;
            let mut light = zone.lightness;

            if smooth {
                let prev = self.prev_zones[idx];
                hue = Hsl::lerp_hue(prev[0], hue, alpha);
                sat = alpha * sat + inv * prev[1];
                light = alpha * light + inv * prev[2];
            }
            self.prev_zones[idx] = [hue, sat, light];

            let final_color = if is_mono {
                Hsl::new(hue, 0.0, light)
            } else if is_vivid {
                Hsl::new(hue, sat + 10.0, light)
            } else if is_cinema {
                Hsl::new(hue, sat - 10.0, light)
            } else if is_dominant {
                Hsl::new(self.dominant_hue.unwrap_or(hue), sat, light)
            } else {
                Hsl::new(hue, sat, light)
            }
            .to_rgb()
            .to_rgba(255);

            let row = idx / ZONE_COLS;
            let col = idx % ZONE_COLS;
            let x0 = (col * cell_w) as i32;
            let y0 = (row * cell_h) as i32;
            let w = if col + 1 == ZONE_COLS {
                canvas.width() as i32 - x0
            } else {
                cell_w as i32
            };
            let h = if row + 1 == ZONE_ROWS {
                canvas.height() as i32 - y0
            } else {
                cell_h as i32
            };
            canvas.fill_rect(x0, y0, w, h, final_color);
        }
    }

    fn render_hd(&mut self, grid: &ZoneGrid, alpha: f32, canvas: &mut CanvasFrame) {
        let expected = (HD_WIDTH * HD_HEIGHT) as usize;
        if grid.hd_rgba.len() != expected {
            return;
        }
        if self.prev_hd_pixels.len() != expected {
            self.prev_hd_pixels = grid
                .hd_rgba
                .iter()
                .map(|px| [f32::from(px.r), f32::from(px.g), f32::from(px.b)])
                .collect();
        }

        let smooth = alpha < 1.0;
        let inv = 1.0 - alpha;
        let dst_w = canvas.width() as usize;
        let dst_h = canvas.height() as usize;
        let pixels = canvas.pixels_mut();

        for y in 0..HD_HEIGHT as usize {
            for x in 0..HD_WIDTH as usize {
                let src_idx = y * (HD_WIDTH as usize) + x;
                let src = grid.hd_rgba[src_idx];
                let out = if smooth {
                    let prev = &mut self.prev_hd_pixels[src_idx];
                    prev[0] = alpha * f32::from(src.r) + inv * prev[0];
                    prev[1] = alpha * f32::from(src.g) + inv * prev[1];
                    prev[2] = alpha * f32::from(src.b) + inv * prev[2];
                    Rgba::new(
                        prev[0].round().clamp(0.0, 255.0) as u8,
                        prev[1].round().clamp(0.0, 255.0) as u8,
                        prev[2].round().clamp(0.0, 255.0) as u8,
                        255,
                    )
                } else {
                    Rgba::new(src.r, src.g, src.b, 255)
                };

                // Upscale 160x100 cleanly into 320x200.
                let dx = x * 2;
                let dy = y * 2;
                if dy + 1 < dst_h && dx + 1 < dst_w {
                    pixels[dy * dst_w + dx] = out;
                    pixels[dy * dst_w + dx + 1] = out;
                    pixels[(dy + 1) * dst_w + dx] = out;
                    pixels[(dy + 1) * dst_w + dx + 1] = out;
                }
            }
        }
    }

    fn render_standby(elapsed: f32, canvas: &mut CanvasFrame) {
        let w = canvas.width() as usize;
        let h = canvas.height() as usize;
        let pixels = canvas.pixels_mut();
        for y in 0..h {
            for x in 0..w {
                let hue = (x as f32 * 0.4 + y as f32 * 0.3 + elapsed * 30.0).rem_euclid(360.0);
                pixels[y * w + x] = Hsl::new(hue, 55.0, 22.0).to_rgb().to_rgba(255);
            }
        }
    }
}

impl Effect for ScreenAmbienceEffect {
    fn metadata(&self) -> EffectMetadata {
        EffectMetadata {
            id: "screen_ambience",
            name: "Screen Ambience",
            description: "Mirrors your display in real-time using 28x20 HSL zones or 160x100 HD sampling.",
            publisher: "WhirlwindFX",
            category: EffectCategory::Dynamic,
            uses_screen: true,
        }
    }

    fn descriptors(&self) -> Vec<ParameterDescriptor> {
        vec![
            ParameterDescriptor::combobox(
                "picture_mode",
                "Picture Mode",
                vec![
                    "Standard".to_owned(),
                    "Cinema".to_owned(),
                    "Mono".to_owned(),
                    "Vivid".to_owned(),
                    "Dominant".to_owned(),
                    "HD".to_owned(),
                ],
                "Standard",
            ),
            ParameterDescriptor::number("blur_amount", "Blur", 0.0, 20.0, 1.0, 4.0),
            ParameterDescriptor::number(
                "motion_smoothing",
                "Motion Smoothing",
                0.0,
                100.0,
                1.0,
                50.0,
            ),
            ParameterDescriptor::number("boost", "Color Shift", -180.0, 180.0, 1.0, 0.0),
            ParameterDescriptor::number("brightness", "Brightness", -100.0, 100.0, 1.0, 0.0),
            ParameterDescriptor::number("saturation", "Saturation", -100.0, 100.0, 1.0, 0.0),
            ParameterDescriptor::number("contrast", "Contrast", -100.0, 100.0, 1.0, 0.0),
        ]
    }

    fn reset(&mut self) {
        self.dominant_hue = None;
        self.prev_zones.clear();
        self.prev_hd_pixels.clear();
    }

    fn render(&mut self, ctx: &EffectContext<'_>, canvas: &mut CanvasFrame) {
        let Some(grid) = ctx.screen else {
            Self::render_standby(ctx.elapsed_seconds, canvas);
            return;
        };

        let picture_mode = ctx.params.str_or("picture_mode", "Standard");
        let motion_smoothing = ctx.params.f32_or("motion_smoothing", 50.0);
        let blur_amount = ctx.params.f32_or("blur_amount", 4.0).round().max(0.0) as u32;
        let filter = ColorFilter {
            hue_shift_deg: ctx.params.f32_or("boost", 0.0),
            brightness_pct: ctx.params.f32_or("brightness", 0.0),
            saturation_pct: ctx.params.f32_or("saturation", 0.0),
            contrast_pct: ctx.params.f32_or("contrast", 0.0),
        };

        let alpha = Self::smoothing_alpha(motion_smoothing);
        if picture_mode == "HD" {
            self.render_hd(grid, alpha, canvas);
        } else {
            self.render_standard_zones(grid, picture_mode, alpha, canvas);
        }

        apply_color_filter(canvas, filter);
        if blur_amount > 0 {
            let mut scratch = std::mem::take(&mut self.blur_scratch);
            apply_box_blur(canvas, blur_amount, &mut scratch);
            self.blur_scratch = scratch;
        }
    }
}
