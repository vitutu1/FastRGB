//! `Bombing Run` stealth bomber flyover dropping expanding HSL blast circles.

use fastrgb_core::{CanvasFrame, ParameterDescriptor, Rgb, Rgba};

use super::helpers::{fill_rotated_rect, hsl_rgb};
use crate::rng::FastRng;
use crate::traits::{Effect, EffectCategory, EffectContext, EffectMetadata};

#[derive(Debug, Clone)]
struct Blast {
    x: f32,
    y: f32,
    radius: f32,
    hue: f32,
    alpha: f32,
}

/// Stealth bomber sweeping overhead while dropping expanding shockwave circles.
#[derive(Debug)]
pub struct BombingRunEffect {
    plane_x: f32,
    drop_timer: f32,
    blasts: Vec<Blast>,
    rng: FastRng,
}

impl Default for BombingRunEffect {
    fn default() -> Self {
        Self {
            plane_x: -40.0,
            drop_timer: 0.0,
            blasts: Vec::with_capacity(12),
            rng: FastRng::seeded(0xB0AB_1945),
        }
    }
}

impl BombingRunEffect {
    /// Creates a new [`BombingRunEffect`].
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl Effect for BombingRunEffect {
    fn metadata(&self) -> EffectMetadata {
        EffectMetadata {
            id: "bombing_run",
            name: "Bombing Run",
            description: "High-altitude bomber flyover dropping glowing expanding impact blooms.",
            publisher: "FastRGB",
            category: EffectCategory::Basic,
            uses_screen: false,
        }
    }

    fn descriptors(&self) -> Vec<ParameterDescriptor> {
        vec![
            ParameterDescriptor::color("bgColor", "Background Color", Rgb::new(0x02, 0x26, 0x00)),
            ParameterDescriptor::number("bombColor1", "Blast Hue", 0.0, 360.0, 1.0, 0.0),
            ParameterDescriptor::boolean("rgbomb", "Rainbow Blasts", false),
            ParameterDescriptor::number("speed", "Flyover Speed", 10.0, 100.0, 1.0, 50.0),
            ParameterDescriptor::number("radius", "Blast Radius", 10.0, 60.0, 1.0, 30.0),
        ]
    }

    fn reset(&mut self) {
        *self = Self::default();
    }

    fn render(&mut self, ctx: &EffectContext<'_>, canvas: &mut CanvasFrame) {
        let bg = ctx.params.color_or("bgColor", Rgb::new(0x02, 0x26, 0x00));
        let base_hue = ctx.params.f32_or("bombColor1", 0.0);
        let rainbow = ctx.params.bool_or("rgbomb", false);
        let speed = ctx.params.f32_or("speed", 50.0);
        let max_r = ctx.params.f32_or("radius", 30.0).max(8.0);

        canvas.fill(bg.to_rgba(255));
        let w = canvas.width() as f32;
        let fly_speed = 65.0 + speed * 2.2;

        self.plane_x += ctx.dt_seconds * fly_speed;
        if self.plane_x > w + 60.0 {
            self.plane_x = -60.0;
        }

        self.drop_timer += ctx.dt_seconds * (speed / 25.0);
        if self.drop_timer >= 0.45 && self.plane_x > 10.0 && self.plane_x < w - 10.0 {
            self.drop_timer = 0.0;
            let hue = if rainbow {
                self.rng.range_f32(0.0, 360.0)
            } else {
                base_hue
            };
            self.blasts.push(Blast {
                x: self.plane_x,
                y: self.rng.range_f32(65.0, canvas.height() as f32 - 25.0),
                radius: 4.0,
                hue,
                alpha: 1.0,
            });
        }

        for b in &mut self.blasts {
            b.radius += ctx.dt_seconds * max_r * 1.8;
            b.alpha -= ctx.dt_seconds * 0.9;
            let a = (b.alpha.clamp(0.0, 1.0) * 230.0).round() as u8;
            let col = hsl_rgb(b.hue, 100.0, 52.0);
            canvas.fill_circle(b.x, b.y, b.radius, col.to_rgba(a));
        }
        self.blasts.retain(|b| b.alpha > 0.0);

        // Swept bomber silhouette flying across the top band.
        let hull = Rgba::new(0x0B, 0x0B, 0x0B, 255);
        canvas.fill_rect(self.plane_x.round() as i32 - 20, 26, 40, 8, hull);
        fill_rotated_rect(canvas, self.plane_x - 2.0, 30.0, 10.0, 42.0, 0.25, hull);
        canvas.fill_rect(self.plane_x.round() as i32 - 24, 22, 10, 16, hull);
    }
}
