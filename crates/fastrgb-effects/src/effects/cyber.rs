//! `Cyber` alternating neon acid-lime orbs and cyberpunk cyan glitch sparkle phases.

use fastrgb_core::{CanvasFrame, ParameterDescriptor, Rgb, Rgba};

use super::helpers::{DriftParticle, DriftShape};
use crate::rng::FastRng;
use crate::traits::{Effect, EffectCategory, EffectContext, EffectMetadata};

#[derive(Debug, Clone)]
struct CyberOrb {
    particle: DriftParticle,
}

/// Alternating acid-yellow kinetic orbs and cyberpunk glitch rain phase.
#[derive(Debug)]
pub struct CyberEffect {
    orbs: Vec<CyberOrb>,
    phase_timer: f32,
    rng: FastRng,
    initialized: bool,
}

impl Default for CyberEffect {
    fn default() -> Self {
        let mut rng = FastRng::seeded(0xC7BE_2077);
        let palette = [
            Rgba::new(0x00, 0xD0, 0xEE, 175),
            Rgba::new(0xBD, 0xC9, 0x00, 175),
            Rgba::new(0x3D, 0x3D, 0x00, 165),
        ];
        let mut orbs = Vec::with_capacity(24);
        for i in 0..24 {
            orbs.push(CyberOrb {
                particle: DriftParticle {
                    x: rng.range_f32(0.0, 320.0),
                    y: rng.range_f32(0.0, 200.0),
                    radius: rng.range_f32(14.0, 42.0),
                    speed_x: rng.range_f32(-1.1, 1.1),
                    speed_y: rng.range_f32(-0.8, 0.8),
                    color: palette[i % palette.len()],
                    shape: DriftShape::FilledCircle,
                    min_r: 12.0,
                    max_r: 48.0,
                    r_grow: 0.12,
                },
            });
        }
        Self {
            orbs,
            phase_timer: 0.0,
            rng,
            initialized: false,
        }
    }
}

impl CyberEffect {
    /// Creates a new [`CyberEffect`].
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl Effect for CyberEffect {
    fn metadata(&self) -> EffectMetadata {
        EffectMetadata {
            id: "cyber",
            name: "Cyber",
            description: "Acid-lime cyberpunk energy orbs alternating with high-voltage cyan glitch static.",
            publisher: "FastRGB",
            category: EffectCategory::Basic,
            uses_screen: false,
        }
    }

    fn descriptors(&self) -> Vec<ParameterDescriptor> {
        vec![
            ParameterDescriptor::color("bgColor", "Background Color", Rgb::new(0x94, 0xA2, 0x00)),
            ParameterDescriptor::color("orbColor", "Glitch Accent", Rgb::new(0x00, 0xD0, 0xEE)),
            ParameterDescriptor::number("speed", "Speed", 1.0, 100.0, 1.0, 50.0),
        ]
    }

    fn reset(&mut self) {
        *self = Self::default();
    }

    fn render(&mut self, ctx: &EffectContext<'_>, canvas: &mut CanvasFrame) {
        let bg = ctx.params.color_or("bgColor", Rgb::new(0x94, 0xA2, 0x00));
        let accent = ctx.params.color_or("orbColor", Rgb::new(0x00, 0xD0, 0xEE));
        let speed = ctx.params.f32_or("speed", 50.0);

        let speed_scale = speed / 35.0;
        self.phase_timer = (self.phase_timer + ctx.dt_seconds * speed_scale).rem_euclid(10.0);
        let glitch_phase = self.phase_timer > 6.6;

        if glitch_phase {
            canvas.blend_all(Rgba::new(0x06, 0x08, 0x0A, 96));
            for _ in 0..24 {
                let x = self.rng.range_f32(0.0, canvas.width() as f32 - 8.0) as i32;
                let y = self.rng.range_f32(0.0, canvas.height() as f32 - 4.0) as i32;
                let w = self.rng.range_f32(4.0, 26.0) as i32;
                let col = if self.rng.next_f32() < 0.6 {
                    accent
                } else {
                    bg
                };
                canvas.fill_rect(x, y, w, 3, col.to_rgba(240));
            }
        } else {
            if !self.initialized {
                canvas.fill(bg.to_rgba(255));
                self.initialized = true;
            } else {
                canvas.blend_all(Rgba::new(bg.r, bg.g, bg.b, 48));
            }
            for o in &mut self.orbs {
                o.particle
                    .step_and_draw(canvas, &mut self.rng, speed_scale, ctx.dt_seconds);
            }
        }
    }
}
