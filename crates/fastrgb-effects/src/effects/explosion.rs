//! `Explosion` procedural angular-noise radial shockwave effect.

use fastrgb_core::{CanvasFrame, ParameterDescriptor, Rgb};

use super::helpers::hsl_rgb;
use crate::traits::{Effect, EffectCategory, EffectContext, EffectMetadata};

/// Smooth value noise helper matching `Explosion.html`'s 1D angular FBM shader.
fn hash1d(n: f32) -> f32 {
    ((n * 127.1).sin() * 43758.547).fract().abs()
}

fn value_noise1d(x: f32) -> f32 {
    let i = x.floor();
    let f = x - i;
    let u = f * f * (3.0 - 2.0 * f);
    let a = hash1d(i);
    let b = hash1d(i + 1.0);
    a + (b - a) * u
}

fn fbm1d(mut x: f32) -> f32 {
    let mut amp = 0.5;
    let mut sum = 0.0;
    for _ in 0..3 {
        sum += amp * value_noise1d(x);
        x *= 2.02;
        amp *= 0.5;
    }
    sum
}

fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let span = edge1 - edge0;
    if span.abs() < 1e-6 {
        return 0.0;
    }
    let t = ((x - edge0) / span).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Repeating radial shockwaves distorted by angular fractional Brownian motion noise.
#[derive(Debug, Default)]
pub struct ExplosionEffect {
    time: f32,
}

impl ExplosionEffect {
    /// Creates a new [`ExplosionEffect`].
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl Effect for ExplosionEffect {
    fn metadata(&self) -> EffectMetadata {
        EffectMetadata {
            id: "explosion",
            name: "Explosion",
            description: "Organic radial shockwave bursts sculpted by procedural angular noise.",
            publisher: "FastRGB",
            category: EffectCategory::Basic,
            uses_screen: false,
        }
    }

    fn descriptors(&self) -> Vec<ParameterDescriptor> {
        vec![
            ParameterDescriptor::color("color", "Wave Color", Rgb::new(0xFF, 0x00, 0x00)),
            ParameterDescriptor::boolean("rainbow", "Rainbow Mode", false),
            ParameterDescriptor::boolean("colorCycle", "Color Cycle", false),
            ParameterDescriptor::number("speed", "Speed", 1.0, 20.0, 1.0, 5.0),
            ParameterDescriptor::number("scale", "Cell Scale", 2.0, 20.0, 1.0, 10.0),
            ParameterDescriptor::number("frequency", "Wave Frequency", 1.0, 5.0, 1.0, 4.0),
            ParameterDescriptor::number("noiseLevel", "Turbulence", 0.0, 4.0, 0.25, 1.0),
        ]
    }

    fn reset(&mut self) {
        self.time = 0.0;
    }

    fn render(&mut self, ctx: &EffectContext<'_>, canvas: &mut CanvasFrame) {
        let base = ctx.params.color_or("color", Rgb::new(0xFF, 0x00, 0x00));
        let rainbow = ctx.params.bool_or("rainbow", false);
        let cycle = ctx.params.bool_or("colorCycle", false);
        let speed = ctx.params.f32_or("speed", 5.0);
        let scale = ctx.params.f32_or("scale", 10.0).round().clamp(2.0, 20.0) as i32;
        let freq = ctx.params.f32_or("frequency", 4.0).clamp(1.0, 5.5);
        let noise_lvl = ctx.params.f32_or("noiseLevel", 1.0);

        self.time += ctx.dt_seconds * speed * 0.45;
        let period = (7.0 - freq).max(1.0);
        let pulse = (self.time % period) * 1.25;

        let w = canvas.width() as i32;
        let h = canvas.height() as i32;
        let inv_min = 1.0 / (h.min(w) as f32).max(1.0);
        let base_hsl = base.to_hsl();

        canvas.fill(Rgb::BLACK.to_rgba(255));
        let mut py = 0;
        while py < h {
            let cy = (py as f32 + 0.5 - h as f32 * 0.5) * inv_min;
            let mut px = 0;
            while px < w {
                let cx = (px as f32 + 0.5 - w as f32 * 0.5) * inv_min;
                let len = (cx * cx + cy * cy).sqrt() * 2.0;
                let angle = cy.atan2(cx) + 3.2;
                let fbm = fbm1d(angle * 20.0) * noise_lvl;
                let front = pulse - fbm;
                let ring = smoothstep(front - 0.8, front, len)
                    * (1.0 - smoothstep(front - 0.1, front, len));
                let l = (ring * 55.0).clamp(0.0, 100.0);
                if l > 2.0 {
                    let hue = if rainbow {
                        angle.to_degrees() + len * 140.0 + self.time * 40.0
                    } else if cycle {
                        self.time * 65.0
                    } else {
                        base_hsl.h
                    };
                    let col = hsl_rgb(hue, 100.0, l).to_rgba(255);
                    canvas.fill_rect(px, py, scale, scale, col);
                }
                px += scale;
            }
            py += scale;
        }
    }
}
