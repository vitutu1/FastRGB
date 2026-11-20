//! `Psychedelic Dream` rotating 3-arm Archimedean spirals over fading neon tiles.

use std::f32::consts::TAU;

use fastrgb_core::{CanvasFrame, ParameterDescriptor, Rgb, Rgba};

use crate::rng::FastRng;
use crate::traits::{Effect, EffectCategory, EffectContext, EffectMetadata};

/// Hypnotic three-arm magenta/cyan/violet Archimedean spirals over tiled dark violet trails.
#[derive(Debug)]
pub struct PsychedelicDreamEffect {
    angle_offset: f32,
    rng: FastRng,
    initialized: bool,
}

impl Default for PsychedelicDreamEffect {
    fn default() -> Self {
        Self {
            angle_offset: 0.0,
            rng: FastRng::seeded(0x957C_DE4A),
            initialized: false,
        }
    }
}

impl PsychedelicDreamEffect {
    /// Creates a new [`PsychedelicDreamEffect`].
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl Effect for PsychedelicDreamEffect {
    fn metadata(&self) -> EffectMetadata {
        EffectMetadata {
            id: "psychedelic_dream",
            name: "Psychedelic Dream",
            description: "Tri-arm rotating Archimedean spirals weaving through glowing neon tiles.",
            publisher: "FastRGB",
            category: EffectCategory::Basic,
            uses_screen: false,
        }
    }

    fn descriptors(&self) -> Vec<ParameterDescriptor> {
        vec![
            ParameterDescriptor::color("bgColor", "Background Color", Rgb::new(0x24, 0x00, 0x4B)),
            ParameterDescriptor::number("speed", "Spiral Speed", 1.0, 100.0, 1.0, 50.0),
        ]
    }

    fn reset(&mut self) {
        *self = Self::default();
    }

    fn render(&mut self, ctx: &EffectContext<'_>, canvas: &mut CanvasFrame) {
        let bg = ctx.params.color_or("bgColor", Rgb::new(0x24, 0x00, 0x4B));
        let speed = ctx.params.f32_or("speed", 50.0);

        if !self.initialized {
            canvas.fill(bg.to_rgba(255));
            self.initialized = true;
        } else {
            canvas.blend_all(Rgba::new(bg.r, bg.g, bg.b, 38));
        }

        // Occasional ambient square tile flashes in the background.
        let tile_colors = [
            Rgba::new(0x49, 0x00, 0x49, 85),
            Rgba::new(0x0A, 0x00, 0x49, 85),
            Rgba::new(0x00, 0x41, 0x41, 85),
        ];
        for _ in 0..3 {
            let tx = self.rng.range_f32(0.0, 270.0).round() as i32;
            let ty = self.rng.range_f32(0.0, 150.0).round() as i32;
            let tc = tile_colors[self.rng.gen_range_usize(tile_colors.len())];
            canvas.fill_rect(tx, ty, 50, 50, tc);
        }

        self.angle_offset = (self.angle_offset + ctx.dt_seconds * (speed / 22.0)).rem_euclid(TAU);
        let cx = canvas.width() as f32 * 0.5;
        let cy = canvas.height() as f32 * 0.5;

        let arms = [
            (0.0f32, Rgba::new(0xFF, 0x00, 0xFF, 235)),
            (TAU / 3.0, Rgba::new(0x00, 0xFF, 0xFF, 235)),
            (2.0 * TAU / 3.0, Rgba::new(0x8B, 0x00, 0xFF, 235)),
        ];

        for (phase, color) in arms {
            let mut prev_x = cx;
            let mut prev_y = cy;
            for step in 1..42 {
                let t = step as f32 * 0.34;
                let r = t * 13.0;
                let a = t + self.angle_offset + phase;
                let x = cx + a.cos() * r;
                let y = cy + a.sin() * r;
                canvas.draw_line(prev_x, prev_y, x, y, 4.0, color);
                prev_x = x;
                prev_y = y;
            }
        }
    }
}
