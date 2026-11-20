//! `Matrix` digital rain effect with glowing leading glyphs and fading green trails.

use fastrgb_core::{CanvasFrame, ParameterDescriptor, Rgb, Rgba};

use crate::rng::FastRng;
use crate::traits::{Effect, EffectCategory, EffectContext, EffectMetadata};

#[derive(Debug, Clone)]
struct RainDrop {
    y: f32,
    speed_mul: f32,
}

/// Digital rain columns with glowing head cells and adjustable trail persistence.
#[derive(Debug)]
pub struct MatrixEffect {
    drops: Vec<RainDrop>,
    rng: FastRng,
}

impl Default for MatrixEffect {
    fn default() -> Self {
        Self {
            drops: Vec::new(),
            rng: FastRng::seeded(0x3A7A_1C50),
        }
    }
}

impl MatrixEffect {
    /// Creates a new [`MatrixEffect`].
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl Effect for MatrixEffect {
    fn metadata(&self) -> EffectMetadata {
        EffectMetadata {
            id: "matrix",
            name: "Matrix",
            description: "Authentic Matrix digital rain with cascading trails and glowing leading glyphs.",
            publisher: "FastRGB",
            category: EffectCategory::Basic,
            uses_screen: false,
        }
    }

    fn descriptors(&self) -> Vec<ParameterDescriptor> {
        vec![
            ParameterDescriptor::color("color", "Rain Color", Rgb::new(0x00, 0xFF, 0x41)),
            ParameterDescriptor::color("headColor", "Leading Glow", Rgb::WHITE),
            ParameterDescriptor::number("fontSize", "Font Size", 6.0, 14.0, 1.0, 8.0),
            ParameterDescriptor::number("speed", "Fall Speed", 1.0, 10.0, 1.0, 4.0),
            ParameterDescriptor::number("fadeSpeed", "Trail Persistence", 1.0, 5.0, 1.0, 3.0),
        ]
    }

    fn reset(&mut self) {
        self.drops.clear();
    }

    fn render(&mut self, ctx: &EffectContext<'_>, canvas: &mut CanvasFrame) {
        let rain_color = ctx.params.color_or("color", Rgb::new(0x00, 0xFF, 0x41));
        let head_color = ctx.params.color_or("headColor", Rgb::WHITE);
        let font_size = ctx.params.f32_or("fontSize", 8.0).round().clamp(6.0, 14.0);
        let speed = ctx.params.f32_or("speed", 4.0);
        let fade_speed = ctx.params.f32_or("fadeSpeed", 3.0);
        let dt_scale = (ctx.dt_seconds * 30.0).max(0.05);

        let columns = (canvas.width() as f32 / font_size).floor().max(1.0) as usize;
        while self.drops.len() < columns {
            let drop = RainDrop {
                y: self.rng.range_f32(-100.0, 20.0),
                speed_mul: self.rng.range_f32(0.6, 1.4),
            };
            self.drops.push(drop);
        }
        self.drops.truncate(columns);

        let fade_alpha = (0.08 + (5.0 - fade_speed) * 0.03).clamp(0.03, 0.4);
        let scaled_fade = (1.0 - (1.0 - fade_alpha).powf(dt_scale)).clamp(0.02, 1.0);
        canvas.blend_all(Rgba::new(0, 0, 0, (scaled_fade * 255.0) as u8));

        let cell = (font_size * 0.75).round().max(3.0) as i32;
        let col_step = font_size as i32;

        for (i, drop) in self.drops.iter_mut().enumerate() {
            let x = (i as i32) * col_step;
            let y = drop.y.round() as i32;

            canvas.fill_rect(x, y - col_step, cell, cell, rain_color.to_rgba(235));
            canvas.fill_rect(x, y, cell, cell, head_color.to_rgba(255));

            drop.y += drop.speed_mul * (speed / 3.0) * (font_size * 0.5) * dt_scale;
            if drop.y > canvas.height() as f32 + 20.0 && self.rng.next_f32() > 0.94 {
                drop.y = self.rng.range_f32(-20.0, 0.0);
                drop.speed_mul = self.rng.range_f32(0.6, 1.4);
            }
        }
    }
}
