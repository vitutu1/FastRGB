//! `Crimson` expanding hollow crimson rings over a trailing blood-red canvas.

use fastrgb_core::{CanvasFrame, ParameterDescriptor, Rgb, Rgba};

use super::helpers::{DriftParticle, DriftShape};
use crate::rng::FastRng;
use crate::traits::{Effect, EffectCategory, EffectContext, EffectMetadata};

const CRIMSON_PALETTE: [Rgba; 3] = [
    Rgba::new(0x82, 0x00, 0x00, 0x88),
    Rgba::new(0xA2, 0x01, 0x00, 0x88),
    Rgba::new(0xFF, 0x00, 0x08, 0x88),
];

/// Layered expanding crimson ripple rings drifting across deep ruby tones.
#[derive(Debug)]
pub struct CrimsonEffect {
    rings: Vec<DriftParticle>,
    rng: FastRng,
    initialized: bool,
}

impl Default for CrimsonEffect {
    fn default() -> Self {
        let mut rng = FastRng::seeded(0xC21A_50AA);
        let mut rings = Vec::with_capacity(30);
        for i in 0..30 {
            let dir = if i % 2 == 0 { -1.0 } else { 1.0 };
            rings.push(DriftParticle {
                x: rng.range_f32(0.0, 320.0),
                y: rng.range_f32(0.0, 200.0),
                radius: rng.range_f32(6.0, 42.0),
                speed_x: rng.range_f32(-0.5, 0.5),
                speed_y: rng.range_f32(0.25, 0.9) * dir,
                color: CRIMSON_PALETTE[i % CRIMSON_PALETTE.len()],
                shape: DriftShape::Ring { line_width: 3 },
                min_r: 5.0,
                max_r: 46.0,
                r_grow: rng.range_f32(0.15, 0.45),
            });
        }
        Self {
            rings,
            rng,
            initialized: false,
        }
    }
}

impl CrimsonEffect {
    /// Creates a new [`CrimsonEffect`].
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl Effect for CrimsonEffect {
    fn metadata(&self) -> EffectMetadata {
        EffectMetadata {
            id: "crimson",
            name: "Crimson",
            description: "Layered hollow crimson ripples expanding and trailing over deep ruby.",
            publisher: "FastRGB",
            category: EffectCategory::Basic,
            uses_screen: false,
        }
    }

    fn descriptors(&self) -> Vec<ParameterDescriptor> {
        vec![
            ParameterDescriptor::color("bgColor", "Background Color", Rgb::new(0x32, 0x00, 0x00)),
            ParameterDescriptor::number("speedRaw", "Ripple Speed", 1.0, 20.0, 1.0, 5.0),
            ParameterDescriptor::number("rippleNum", "Ripple Count", 5.0, 30.0, 1.0, 30.0),
        ]
    }

    fn reset(&mut self) {
        *self = Self::default();
    }

    fn render(&mut self, ctx: &EffectContext<'_>, canvas: &mut CanvasFrame) {
        let bg = ctx.params.color_or("bgColor", Rgb::new(0x32, 0x00, 0x00));
        let speed = ctx.params.f32_or("speedRaw", 5.0);
        let count = ctx
            .params
            .f32_or("rippleNum", 30.0)
            .round()
            .clamp(5.0, 30.0) as usize;

        if !self.initialized {
            canvas.fill(bg.to_rgba(255));
            self.initialized = true;
        } else {
            canvas.blend_all(Rgba::new(bg.r, bg.g, bg.b, 0x22));
        }

        let speed_scale = speed / 5.0;
        for ring in &mut self.rings[..count] {
            ring.step_and_draw(canvas, &mut self.rng, speed_scale, ctx.dt_seconds);
        }
    }
}
