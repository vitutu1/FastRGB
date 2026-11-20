//! `Jade` drifting emerald and teal energy clouds over an obsidian-green base.

use fastrgb_core::{CanvasFrame, ParameterDescriptor, Rgb, Rgba};

use super::helpers::{DriftParticle, DriftShape};
use crate::rng::FastRng;
use crate::traits::{Effect, EffectCategory, EffectContext, EffectMetadata};

const JADE_PALETTE: [Rgba; 3] = [
    Rgba::new(0x00, 0xCC, 0x84, 0x55),
    Rgba::new(0x00, 0x35, 0x2F, 0x66),
    Rgba::new(0x08, 0x3B, 0x1F, 0x66),
];

/// Translucent jade-emerald blooms floating upward and downward across dark malachite.
#[derive(Debug)]
pub struct JadeEffect {
    orbs: Vec<DriftParticle>,
    rng: FastRng,
    initialized: bool,
}

impl Default for JadeEffect {
    fn default() -> Self {
        let mut rng = FastRng::seeded(0x0ADE_E432);
        let mut orbs = Vec::with_capacity(28);
        for i in 0..28 {
            let dir = if i % 2 == 0 { -1.0 } else { 1.0 };
            orbs.push(DriftParticle {
                x: rng.range_f32(0.0, 320.0),
                y: rng.range_f32(0.0, 200.0),
                radius: rng.range_f32(20.0, 50.0),
                speed_x: rng.range_f32(-0.5, 0.5),
                speed_y: rng.range_f32(0.25, 0.85) * dir,
                color: JADE_PALETTE[i % JADE_PALETTE.len()],
                shape: DriftShape::FilledCircle,
                min_r: 20.0,
                max_r: 52.0,
                r_grow: rng.range_f32(0.05, 0.2),
            });
        }
        Self {
            orbs,
            rng,
            initialized: false,
        }
    }
}

impl JadeEffect {
    /// Creates a new [`JadeEffect`].
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl Effect for JadeEffect {
    fn metadata(&self) -> EffectMetadata {
        EffectMetadata {
            id: "jade",
            name: "Jade",
            description: "Drifting jade and malachite luminous clouds with silky trails.",
            publisher: "FastRGB",
            category: EffectCategory::Basic,
            uses_screen: false,
        }
    }

    fn descriptors(&self) -> Vec<ParameterDescriptor> {
        vec![
            ParameterDescriptor::color("bgColor", "Background Color", Rgb::new(0x00, 0x0A, 0x09)),
            ParameterDescriptor::number("speedRaw", "Drift Speed", 1.0, 30.0, 1.0, 10.0),
        ]
    }

    fn reset(&mut self) {
        *self = Self::default();
    }

    fn render(&mut self, ctx: &EffectContext<'_>, canvas: &mut CanvasFrame) {
        let bg = ctx.params.color_or("bgColor", Rgb::new(0x00, 0x0A, 0x09));
        let speed = ctx.params.f32_or("speedRaw", 10.0);

        if !self.initialized {
            canvas.fill(bg.to_rgba(255));
            self.initialized = true;
        } else {
            canvas.blend_all(Rgba::new(bg.r, bg.g, bg.b, 0x22));
        }

        let speed_scale = speed / 10.0;
        for orb in &mut self.orbs {
            orb.step_and_draw(canvas, &mut self.rng, speed_scale, ctx.dt_seconds);
        }
    }
}
