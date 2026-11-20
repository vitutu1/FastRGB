//! `Sakura` drifting cherry blossom petals with sinusoidal wind sway.

use std::f32::consts::TAU;

use fastrgb_core::{CanvasFrame, ParameterDescriptor, Rgb, Rgba};

use super::helpers::{fill_rotated_rect, hsl_rgb};
use crate::rng::FastRng;
use crate::traits::{Effect, EffectCategory, EffectContext, EffectMetadata};

#[derive(Debug, Clone)]
struct Petal {
    x: f32,
    y: f32,
    vy: f32,
    sway_phase: f32,
    scale: f32,
    variant: u8,
}

/// Drifting sakura petals tumbling across a warm magenta-plum night sky.
#[derive(Debug)]
pub struct SakuraEffect {
    petals: Vec<Petal>,
    cycle_hue: f32,
    rng: FastRng,
    initialized: bool,
}

impl Default for SakuraEffect {
    fn default() -> Self {
        let mut rng = FastRng::seeded(0x5A40_2A01);
        let petals = (0..64)
            .map(|i| Petal {
                x: rng.range_f32(0.0, 320.0),
                y: rng.range_f32(0.0, 200.0),
                vy: rng.range_f32(0.5, 1.4),
                sway_phase: rng.range_f32(0.0, TAU),
                scale: rng.range_f32(0.55, 1.15),
                variant: (i % 3) as u8,
            })
            .collect();
        Self {
            petals,
            cycle_hue: 280.0,
            rng,
            initialized: false,
        }
    }
}

impl SakuraEffect {
    /// Creates a new [`SakuraEffect`].
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl Effect for SakuraEffect {
    fn metadata(&self) -> EffectMetadata {
        EffectMetadata {
            id: "sakura",
            name: "Sakura",
            description: "Gentle drifting cherry blossom petals swaying across a soft plum dusk.",
            publisher: "FastRGB",
            category: EffectCategory::Basic,
            uses_screen: false,
        }
    }

    fn descriptors(&self) -> Vec<ParameterDescriptor> {
        vec![
            ParameterDescriptor::color("color1", "Petal Color", Rgb::new(0xBC, 0x2C, 0xFF)),
            ParameterDescriptor::color("bgColor", "Background Color", Rgb::new(0x54, 0x18, 0x46)),
            ParameterDescriptor::number("speed", "Wind Speed", 1.0, 60.0, 1.0, 15.0),
            ParameterDescriptor::number("size", "Petal Size", 10.0, 80.0, 1.0, 33.0),
            ParameterDescriptor::number("blossomAmount", "Petal Count", 10.0, 64.0, 1.0, 61.0),
            ParameterDescriptor::number("blossomFade", "Trail Persistence", 10.0, 100.0, 1.0, 48.0),
            ParameterDescriptor::boolean("colorCycle", "Color Cycle", false),
            ParameterDescriptor::number("cycleSpeed", "Cycle Speed", 1.0, 100.0, 1.0, 50.0),
        ]
    }

    fn reset(&mut self) {
        *self = Self::default();
    }

    fn render(&mut self, ctx: &EffectContext<'_>, canvas: &mut CanvasFrame) {
        let base_col = ctx.params.color_or("color1", Rgb::new(0xBC, 0x2C, 0xFF));
        let bg = ctx.params.color_or("bgColor", Rgb::new(0x54, 0x18, 0x46));
        let speed = ctx.params.f32_or("speed", 15.0);
        let size = ctx.params.f32_or("size", 33.0);
        let count = ctx
            .params
            .f32_or("blossomAmount", 61.0)
            .round()
            .clamp(10.0, 64.0) as usize;
        let fade = ctx.params.f32_or("blossomFade", 48.0);
        let cycle = ctx.params.bool_or("colorCycle", false);
        let cycle_speed = ctx.params.f32_or("cycleSpeed", 50.0);

        if !self.initialized {
            canvas.fill(bg.to_rgba(255));
            self.initialized = true;
        } else {
            let fade_alpha = (18.0 + (100.0 - fade) * 0.9).clamp(16.0, 180.0) as u8;
            canvas.blend_all(Rgba::new(bg.r, bg.g, bg.b, fade_alpha));
        }

        if cycle {
            self.cycle_hue =
                (self.cycle_hue + ctx.dt_seconds * (cycle_speed * 0.9)).rem_euclid(360.0);
        }
        let base_hsl = base_col.to_hsl();
        let w = canvas.width() as f32;
        let h = canvas.height() as f32;
        let step = (ctx.dt_seconds * 60.0) * (speed / 15.0);
        let petal_dim = (size * 0.34).max(4.0);

        for p in &mut self.petals[..count] {
            p.sway_phase = (p.sway_phase + ctx.dt_seconds * 2.1).rem_euclid(TAU);
            p.x += (p.sway_phase.sin() * 0.9 + 0.35) * step;
            p.y += p.vy * step * 0.85;

            if p.y > h + 16.0 {
                p.y = -12.0;
                p.x = self.rng.range_f32(0.0, w);
            }
            if p.x > w + 16.0 {
                p.x = -12.0;
            } else if p.x < -16.0 {
                p.x = w + 12.0;
            }

            let hue_base = if cycle { self.cycle_hue } else { base_hsl.h };
            let (dh, l) = match p.variant {
                0 => (0.0, (base_hsl.l + 10.0).min(86.0)),
                1 => (14.0, (base_hsl.l + 22.0).min(92.0)),
                _ => (-12.0, base_hsl.l.clamp(35.0, 75.0)),
            };
            let col = hsl_rgb(hue_base + dh, base_hsl.s.max(55.0), l);
            let pw = petal_dim * p.scale;
            let ph = (petal_dim * 0.55 * p.scale).max(2.5);
            fill_rotated_rect(canvas, p.x, p.y, pw, ph, p.sway_phase, col.to_rgba(215));
        }
    }
}
