//! `Titanium` monochrome metallic spark flashes and vertical sweeps.

use fastrgb_core::{CanvasFrame, ParameterDescriptor, Rgb, Rgba};

use crate::rng::FastRng;
use crate::traits::{Effect, EffectCategory, EffectContext, EffectMetadata};

#[derive(Debug, Clone)]
struct TitaniumParticle {
    x: f32,
    y: f32,
    w: i32,
    h: i32,
    vy: f32,
    vx: f32,
    shade: u8,
    alpha: u8,
}

/// Brushed monochrome metallic glints and sweeping shimmer bars.
#[derive(Debug)]
pub struct TitaniumEffect {
    particles: Vec<TitaniumParticle>,
    rng: FastRng,
    initialized: bool,
}

impl Default for TitaniumEffect {
    fn default() -> Self {
        let mut rng = FastRng::seeded(0x717A_110A);
        let mut particles = Vec::with_capacity(42);
        for i in 0..42 {
            let shade = rng.range_f32(105.0, 250.0).round() as u8;
            let (w, h, vy, vx, alpha) = match i % 3 {
                0 => (10, 10, 0.0, rng.range_f32(-0.3, 0.3), 75),
                1 => (4, 18, rng.range_f32(0.8, 2.2), 0.0, 210),
                _ => (4, 18, -rng.range_f32(0.8, 2.2), 0.0, 210),
            };
            particles.push(TitaniumParticle {
                x: rng.range_f32(0.0, 320.0),
                y: rng.range_f32(0.0, 200.0),
                w,
                h,
                vy,
                vx,
                shade,
                alpha,
            });
        }
        Self {
            particles,
            rng,
            initialized: false,
        }
    }
}

impl TitaniumEffect {
    /// Creates a new [`TitaniumEffect`].
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl Effect for TitaniumEffect {
    fn metadata(&self) -> EffectMetadata {
        EffectMetadata {
            id: "titanium",
            name: "Titanium",
            description: "Sleek monochrome metallic flashes and brushed vertical titanium streaks.",
            publisher: "FastRGB",
            category: EffectCategory::Basic,
            uses_screen: false,
        }
    }

    fn descriptors(&self) -> Vec<ParameterDescriptor> {
        vec![
            ParameterDescriptor::color("bgColor", "Background Color", Rgb::new(0x10, 0x10, 0x10)),
            ParameterDescriptor::number("speedRaw", "Shimmer Speed", 1.0, 30.0, 1.0, 10.0),
        ]
    }

    fn reset(&mut self) {
        *self = Self::default();
    }

    fn render(&mut self, ctx: &EffectContext<'_>, canvas: &mut CanvasFrame) {
        let bg = ctx.params.color_or("bgColor", Rgb::new(0x10, 0x10, 0x10));
        let speed = ctx.params.f32_or("speedRaw", 10.0);

        if !self.initialized {
            canvas.fill(bg.to_rgba(255));
            self.initialized = true;
        } else {
            canvas.blend_all(Rgba::new(bg.r, bg.g, bg.b, 0x50));
        }

        let w = canvas.width() as f32;
        let h = canvas.height() as f32;
        let step = (ctx.dt_seconds * 60.0) * (speed / 10.0);

        for p in &mut self.particles {
            p.x = (p.x + p.vx * step).rem_euclid(w);
            p.y = (p.y + p.vy * step).rem_euclid(h);
            if p.vy == 0.0 && self.rng.next_f32() < 0.08 {
                p.x = self.rng.range_f32(0.0, w - 10.0);
                p.y = self.rng.range_f32(0.0, h - 10.0);
            }
            canvas.fill_rect(
                p.x.round() as i32,
                p.y.round() as i32,
                p.w,
                p.h,
                Rgba::new(p.shade, p.shade, p.shade, p.alpha),
            );
        }
    }
}
