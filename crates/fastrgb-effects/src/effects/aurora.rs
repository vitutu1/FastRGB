//! `Aurora` Northern Lights effect.

use fastrgb_core::{CanvasFrame, Hsl, ParameterDescriptor, Rgb};

use crate::rng::FastRng;
use crate::traits::{Effect, EffectCategory, EffectContext, EffectMetadata};

#[derive(Debug, Clone)]
struct AuroraStreak {
    x: f32,
    y: f32,
    width: f32,
}

/// Flowing Northern Lights curtains with customizable colors, density, and hue cycle.
#[derive(Debug)]
pub struct AuroraEffect {
    streaks: Vec<AuroraStreak>,
    front_cycle: f32,
    rng: FastRng,
}

impl Default for AuroraEffect {
    fn default() -> Self {
        let mut rng = FastRng::seeded(0xA0B0_2026);
        let mut streaks = Vec::with_capacity(16);
        for _ in 0..15 {
            streaks.push(AuroraStreak {
                x: rng.range_f32(0.0, 280.0),
                y: rng.range_f32(80.0, 240.0),
                width: rng.range_f32(10.0, 25.0),
            });
        }
        Self {
            streaks,
            front_cycle: 0.0,
            rng,
        }
    }
}

impl AuroraEffect {
    /// Creates a new [`AuroraEffect`].
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl Effect for AuroraEffect {
    fn metadata(&self) -> EffectMetadata {
        EffectMetadata {
            id: "aurora",
            name: "Aurora",
            description: "The colors of the Northern Lights illuminate your devices.",
            publisher: "FastRGB",
            category: EffectCategory::Basic,
            uses_screen: false,
        }
    }

    fn descriptors(&self) -> Vec<ParameterDescriptor> {
        vec![
            ParameterDescriptor::number("effectSpeed", "Animation Speed", 0.0, 100.0, 1.0, 40.0),
            ParameterDescriptor::number("amount", "Aurora Density", 0.0, 100.0, 1.0, 61.0),
            ParameterDescriptor::color(
                "frontColor",
                "Main Aurora Color",
                Rgb::new(0x00, 0xFF, 0xFF),
            ),
            ParameterDescriptor::color("backColor", "Background Color", Rgb::new(0x00, 0x5F, 0x49)),
            ParameterDescriptor::boolean("colorCycle", "Color Cycle", false),
            ParameterDescriptor::number("cycleSpeed", "Color Cycle Speed", 0.0, 100.0, 1.0, 50.0),
        ]
    }

    fn reset(&mut self) {
        *self = Self::default();
    }

    fn render(&mut self, ctx: &EffectContext<'_>, canvas: &mut CanvasFrame) {
        let speed = ctx.params.f32_or("effectSpeed", 40.0);
        let amount = ctx.params.f32_or("amount", 61.0);
        let front_hsl = ctx
            .params
            .color_or("frontColor", Rgb::new(0x00, 0xFF, 0xFF))
            .to_hsl();
        let back_hsl = ctx
            .params
            .color_or("backColor", Rgb::new(0x00, 0x5F, 0x49))
            .to_hsl();
        let color_cycle = ctx.params.bool_or("colorCycle", false);
        let cycle_speed = ctx.params.f32_or("cycleSpeed", 50.0);
        let dt_scale = (ctx.dt_seconds * 60.0).max(0.05);

        let bg_alpha = (1.0 - (1.0 - 0.04_f32).powf(dt_scale)).clamp(0.01, 1.0);
        let bg = Hsl::new(back_hsl.h + self.front_cycle, back_hsl.s, back_hsl.l)
            .to_rgb()
            .to_rgba((bg_alpha * 255.0).round() as u8);
        canvas.blend_all(bg);

        let target_count = (amount / 4.0).floor() as usize;
        if self.streaks.len() < target_count {
            self.streaks.push(AuroraStreak {
                x: self.rng.range_f32(-80.0, 70.0),
                y: self.rng.range_f32(80.0, 240.0),
                width: self.rng.range_f32(10.0, 25.0),
            });
        }

        if color_cycle {
            self.front_cycle =
                (self.front_cycle + (cycle_speed / 50.0) * dt_scale).rem_euclid(360.0);
        } else {
            self.front_cycle = 0.0;
        }

        let w = canvas.width() as i32;
        let h = canvas.height() as i32;
        let step_x = (speed / 38.0) * dt_scale;

        for streak in &mut self.streaks {
            streak.y = (streak.y + self.rng.range_f32(-3.0, 3.0) * dt_scale).clamp(40.0, 260.0);
            streak.x += step_x;

            let x0 = (streak.x - streak.width * 0.5).round() as i32;
            let x1 = (streak.x + streak.width * 0.5).round() as i32;
            let y_bottom = (streak.y.round() as i32).min(h - 1);
            let y_top = ((streak.y - 200.0).round() as i32).max(0);

            for y in y_top..=y_bottom {
                let t = ((streak.y - y as f32) / 200.0).clamp(0.0, 1.0);
                let alpha = if t < 0.25 {
                    0.9 - (t / 0.25) * 0.15
                } else if t < 0.5 {
                    0.75 - ((t - 0.25) / 0.25) * 0.05
                } else if t < 0.75 {
                    0.7 - ((t - 0.5) / 0.25) * 0.6
                } else {
                    0.1
                };
                let hue = front_hsl.h + self.front_cycle + t * 200.0;
                let col = Hsl::new(hue, front_hsl.s, front_hsl.l)
                    .to_rgb()
                    .to_rgba((alpha * 255.0 * 0.35).round().clamp(1.0, 255.0) as u8);
                for x in x0.max(0)..=x1.min(w - 1) {
                    canvas.blend_pixel(x, y, col);
                }
            }
        }

        self.streaks.retain(|s| s.x <= canvas.width() as f32);
    }
}
