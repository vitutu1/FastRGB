//! `Cotton Candy` pastel cyan and magenta clouds drifting over a split purple-blue backdrop.

use fastrgb_core::{CanvasFrame, ParameterDescriptor, Rgb, Rgba};

use super::helpers::{DriftParticle, DriftShape};
use crate::rng::FastRng;
use crate::traits::{Effect, EffectCategory, EffectContext, EffectMetadata};

/// Translucent pastel cyan and magenta clouds gliding across violet-blue bands.
#[derive(Debug)]
pub struct CottonCandyEffect {
    orbs: Vec<DriftParticle>,
    rng: FastRng,
}

impl Default for CottonCandyEffect {
    fn default() -> Self {
        let mut rng = FastRng::seeded(0xC077_0AC4);
        let mut orbs = Vec::with_capacity(30);
        for i in 0..30 {
            let is_cyan = i % 2 == 0;
            let color = if is_cyan {
                Rgba::new(0x00, 0xFF, 0xFF, 0x77)
            } else {
                Rgba::new(0xFF, 0x00, 0xFF, 0x77)
            };
            let dir = if is_cyan { 1.0 } else { -1.0 };
            orbs.push(DriftParticle {
                x: rng.range_f32(0.0, 320.0),
                y: rng.range_f32(0.0, 200.0),
                radius: rng.range_f32(40.0, 65.0),
                speed_x: rng.range_f32(0.3, 1.2) * dir,
                speed_y: rng.range_f32(0.15, 0.75) * dir,
                color,
                shape: DriftShape::FilledCircle,
                min_r: 40.0,
                max_r: 65.0,
                r_grow: 0.0,
            });
        }
        Self { orbs, rng }
    }
}

impl CottonCandyEffect {
    /// Creates a new [`CottonCandyEffect`].
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl Effect for CottonCandyEffect {
    fn metadata(&self) -> EffectMetadata {
        EffectMetadata {
            id: "cotton_candy",
            name: "Cotton Candy",
            description: "Soft floating pastel cyan and magenta clouds over split twilight bands.",
            publisher: "FastRGB",
            category: EffectCategory::Basic,
            uses_screen: false,
        }
    }

    fn descriptors(&self) -> Vec<ParameterDescriptor> {
        vec![
            ParameterDescriptor::color("bgColor1", "Top Band", Rgb::new(0x54, 0x00, 0x74)),
            ParameterDescriptor::color("bgColor2", "Center Band", Rgb::new(0x00, 0x4A, 0x79)),
            ParameterDescriptor::color("bgColor3", "Bottom Band", Rgb::new(0x73, 0x00, 0x73)),
            ParameterDescriptor::number("speed", "Cloud Speed", 1.0, 100.0, 1.0, 30.0),
        ]
    }

    fn reset(&mut self) {
        *self = Self::default();
    }

    fn render(&mut self, ctx: &EffectContext<'_>, canvas: &mut CanvasFrame) {
        let bg1 = ctx.params.color_or("bgColor1", Rgb::new(0x54, 0x00, 0x74));
        let bg2 = ctx.params.color_or("bgColor2", Rgb::new(0x00, 0x4A, 0x79));
        let bg3 = ctx.params.color_or("bgColor3", Rgb::new(0x73, 0x00, 0x73));
        let speed = ctx.params.f32_or("speed", 30.0);

        let w = canvas.width() as i32;
        let h = canvas.height() as i32;
        let third = h / 3;
        canvas.fill_rect(0, 0, w, third, bg1.to_rgba(255));
        canvas.fill_rect(0, third, w, third, bg2.to_rgba(255));
        canvas.fill_rect(0, third * 2, w, h - third * 2, bg3.to_rgba(255));

        let speed_scale = speed / 30.0;
        for orb in &mut self.orbs {
            orb.step_and_draw(canvas, &mut self.rng, speed_scale, ctx.dt_seconds);
        }
    }
}
