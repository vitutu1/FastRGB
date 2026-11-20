//! `Rainstorm` rain droplets and lightning flash effect.

use fastrgb_core::{CanvasFrame, ParameterDescriptor, Rgb, Rgba};

use crate::rng::FastRng;
use crate::traits::{Effect, EffectCategory, EffectContext, EffectMetadata};

#[derive(Debug, Clone)]
struct DropParticle {
    x: f32,
    y: f32,
    length: f32,
    speed_factor: f32,
}

/// Pouring slanted rain with intermittent lightning flashes.
#[derive(Debug)]
pub struct RainstormEffect {
    drops: Vec<DropParticle>,
    flash_alpha: f32,
    flash_timer: f32,
    rng: FastRng,
}

impl Default for RainstormEffect {
    fn default() -> Self {
        Self {
            drops: Vec::new(),
            flash_alpha: 0.0,
            flash_timer: 0.0,
            rng: FastRng::seeded(0x2A1A_570A),
        }
    }
}

impl RainstormEffect {
    /// Creates a new [`RainstormEffect`].
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    fn spawn_drop(&mut self, random_y: bool) -> DropParticle {
        DropParticle {
            x: self.rng.range_f32(-25.0, 345.0),
            y: if random_y {
                self.rng.range_f32(0.0, 200.0)
            } else {
                self.rng.range_f32(-20.0, 0.0)
            },
            length: self.rng.range_f32(8.0, 20.0),
            speed_factor: self.rng.range_f32(0.8, 1.3),
        }
    }
}

impl Effect for RainstormEffect {
    fn metadata(&self) -> EffectMetadata {
        EffectMetadata {
            id: "rainstorm",
            name: "Rainstorm",
            description: "Pouring rain interspersed with lightning bolts and flashes.",
            publisher: "FastRGB",
            category: EffectCategory::Basic,
            uses_screen: false,
        }
    }

    fn descriptors(&self) -> Vec<ParameterDescriptor> {
        vec![
            ParameterDescriptor::color("rainColor", "Rain Color", Rgb::new(0x33, 0x88, 0xFF)),
            ParameterDescriptor::color("lightningColor", "Lightning Color", Rgb::WHITE),
            ParameterDescriptor::number("rainSpeed", "Rain Speed", 5.0, 30.0, 1.0, 16.0),
            ParameterDescriptor::number("density", "Rain Density", 20.0, 150.0, 1.0, 80.0),
            ParameterDescriptor::number(
                "lightningFreq",
                "Lightning Frequency",
                1.0,
                10.0,
                1.0,
                3.0,
            ),
        ]
    }

    fn reset(&mut self) {
        *self = Self::default();
    }

    fn render(&mut self, ctx: &EffectContext<'_>, canvas: &mut CanvasFrame) {
        let rain_color = ctx.params.color_or("rainColor", Rgb::new(0x33, 0x88, 0xFF));
        let lightning_color = ctx.params.color_or("lightningColor", Rgb::WHITE);
        let rain_speed = ctx.params.f32_or("rainSpeed", 16.0);
        let density = ctx.params.f32_or("density", 80.0).round() as usize;
        let freq = ctx.params.f32_or("lightningFreq", 3.0).max(1.0);
        let dt_scale = (ctx.dt_seconds * 30.0).max(0.05);

        while self.drops.len() < density {
            let d = self.spawn_drop(true);
            self.drops.push(d);
        }
        self.drops.truncate(density);

        if self.flash_alpha > 0.05 {
            canvas.blend_all(
                lightning_color.to_rgba((self.flash_alpha.clamp(0.0, 1.0) * 255.0) as u8),
            );
            self.flash_alpha *= 0.82_f32.powf(dt_scale);
        } else {
            canvas.blend_all(Rgba::new(5, 8, 18, 102));
        }

        self.flash_timer += dt_scale;
        if self.flash_timer > 60.0 / freq && self.rng.next_f32() < 0.06 * dt_scale {
            self.flash_alpha = self.rng.range_f32(0.3, 1.0);
            self.flash_timer = 0.0;
        }

        let rain_rgba = rain_color.to_rgba(240);
        let h = canvas.height() as f32;
        for i in 0..self.drops.len() {
            let (x, y, length, speed_factor) = {
                let d = &self.drops[i];
                (d.x, d.y, d.length, d.speed_factor)
            };
            canvas.draw_line(x, y, x - 3.0, y + length, 1.2, rain_rgba);
            let spd = speed_factor * rain_speed * dt_scale;
            let next_x = x - 1.5 * (spd / 10.0);
            let next_y = y + spd;
            if next_y > h {
                self.drops[i] = self.spawn_drop(false);
            } else {
                self.drops[i].x = next_x;
                self.drops[i].y = next_y;
            }
        }
    }
}
