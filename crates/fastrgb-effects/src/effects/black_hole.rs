//! `Black Hole` gravitational accretion disk effect.

use std::f32::consts::TAU;

use fastrgb_core::{CanvasFrame, ParameterDescriptor, Rgb, Rgba};

use crate::rng::FastRng;
use crate::traits::{Effect, EffectCategory, EffectContext, EffectMetadata};

#[derive(Debug, Clone)]
struct AccretionParticle {
    angle: f32,
    dist: f32,
    speed: f32,
    size: f32,
}

/// Swirling relativistic accretion disk around an absolute-black event horizon.
#[derive(Debug)]
pub struct BlackHoleEffect {
    particles: Vec<AccretionParticle>,
    rng: FastRng,
}

impl Default for BlackHoleEffect {
    fn default() -> Self {
        let mut rng = FastRng::seeded(0xB1AC_401E);
        let mut particles = Vec::with_capacity(220);
        for _ in 0..220 {
            particles.push(AccretionParticle {
                angle: rng.range_f32(0.0, TAU),
                dist: rng.range_f32(28.0, 138.0),
                speed: rng.range_f32(0.02, 0.04),
                size: rng.range_f32(0.8, 3.0),
            });
        }
        Self { particles, rng }
    }
}

impl BlackHoleEffect {
    /// Creates a new [`BlackHoleEffect`].
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl Effect for BlackHoleEffect {
    fn metadata(&self) -> EffectMetadata {
        EffectMetadata {
            id: "black_hole",
            name: "Black Hole",
            description: "Swirling gravitational black hole with relativistic accretion disk.",
            publisher: "FastRGB",
            category: EffectCategory::Basic,
            uses_screen: false,
        }
    }

    fn descriptors(&self) -> Vec<ParameterDescriptor> {
        vec![
            ParameterDescriptor::color("diskColor", "Accretion Color", Rgb::new(0xFF, 0x77, 0x00)),
            ParameterDescriptor::number("spinSpeed", "Vortex Speed", 1.0, 10.0, 1.0, 5.0),
            ParameterDescriptor::number("radius", "Event Horizon Size", 15.0, 45.0, 1.0, 28.0),
        ]
    }

    fn reset(&mut self) {
        *self = Self::default();
    }

    fn render(&mut self, ctx: &EffectContext<'_>, canvas: &mut CanvasFrame) {
        let disk_color = ctx.params.color_or("diskColor", Rgb::new(0xFF, 0x77, 0x00));
        let spin_speed = ctx.params.f32_or("spinSpeed", 5.0);
        let horizon_r = ctx.params.f32_or("radius", 28.0);
        let dt_scale = (ctx.dt_seconds * 30.0).max(0.05);

        let fade_a = (1.0 - (1.0 - 0.22_f32).powf(dt_scale)).clamp(0.04, 1.0);
        canvas.blend_all(Rgba::new(0, 0, 0, (fade_a * 255.0) as u8));

        let cx = canvas.width() as f32 * 0.5;
        let cy = canvas.height() as f32 * 0.5;
        let disk_rgba = disk_color.to_rgba(230);

        for p in &mut self.particles {
            p.angle = (p.angle + p.speed * (spin_speed / 3.0) * dt_scale).rem_euclid(TAU);
            p.dist -= 0.35 * dt_scale;
            if p.dist < horizon_r {
                p.dist = self.rng.range_f32(100.0, 140.0);
            }
            let x = cx + p.angle.cos() * p.dist * 1.5;
            let y = cy + p.angle.sin() * p.dist * 0.65;
            canvas.fill_circle(x, y, p.size, disk_rgba);
        }

        // Photon sphere glow + black event horizon
        canvas.fill_circle(cx, cy, horizon_r * 1.35, Rgba::new(255, 200, 100, 95));
        canvas.fill_circle(cx, cy, horizon_r, Rgba::BLACK);
    }
}
