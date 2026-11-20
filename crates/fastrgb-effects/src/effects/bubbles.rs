//! `Bubbles` drifting translucent orbs effect.

use fastrgb_core::{CanvasFrame, Hsl, ParameterDescriptor, Rgb};

use crate::rng::FastRng;
use crate::traits::{Effect, EffectCategory, EffectContext, EffectMetadata};

#[derive(Debug, Clone)]
struct Bubble {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    base_size: f32,
    brightness: f32,
}

/// Floating bouncing bubbles with single-color, rainbow, or color-cycle modes.
#[derive(Debug)]
pub struct BubblesEffect {
    bubbles: Vec<Bubble>,
    cycle_hue: f32,
}

impl Default for BubblesEffect {
    fn default() -> Self {
        let mut rng = FastRng::seeded(0xB0BB_1E55);
        let mut bubbles = Vec::with_capacity(22);
        for i in 0..22 {
            let mut vx = rng.range_f32(-1.5, 1.5);
            if vx.abs() < 0.25 {
                vx = 0.7;
            }
            let mut vy = rng.range_f32(-1.5, 1.5);
            if vy.abs() < 0.25 {
                vy = -0.7;
            }
            bubbles.push(Bubble {
                x: rng.range_f32(25.0, 295.0),
                y: rng.range_f32(25.0, 175.0),
                vx,
                vy,
                base_size: rng.range_f32(10.0, 20.0),
                brightness: ((i % 10 + 2) as f32 * 4.0) / 50.0,
            });
        }
        Self {
            bubbles,
            cycle_hue: 0.0,
        }
    }
}

impl BubblesEffect {
    /// Creates a new [`BubblesEffect`].
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl Effect for BubblesEffect {
    fn metadata(&self) -> EffectMetadata {
        EffectMetadata {
            id: "bubbles",
            name: "Bubbles",
            description: "A field of translucent bubbles with controllable speed and color mode.",
            publisher: "FastRGB",
            category: EffectCategory::Basic,
            uses_screen: false,
        }
    }

    fn descriptors(&self) -> Vec<ParameterDescriptor> {
        vec![
            ParameterDescriptor::combobox(
                "color_mode",
                "Color Mode",
                vec![
                    "Single Color".to_owned(),
                    "Rainbow".to_owned(),
                    "Color Cycle".to_owned(),
                ],
                "Single Color",
            ),
            ParameterDescriptor::color("bgColor", "Background Color", Rgb::BLACK),
            ParameterDescriptor::color("color", "Color", Rgb::new(0xFF, 0x00, 0x66)),
            ParameterDescriptor::number("speed", "Bubble Speed", 0.0, 100.0, 1.0, 10.0),
            ParameterDescriptor::number("size", "Bubble Size", 1.0, 10.0, 1.0, 5.0),
        ]
    }

    fn reset(&mut self) {
        *self = Self::default();
    }

    fn render(&mut self, ctx: &EffectContext<'_>, canvas: &mut CanvasFrame) {
        let mode = ctx.params.str_or("color_mode", "Single Color");
        let bg = ctx.params.color_or("bgColor", Rgb::BLACK);
        let color = ctx.params.color_or("color", Rgb::new(0xFF, 0x00, 0x66));
        let speed = ctx.params.f32_or("speed", 10.0);
        let size = ctx.params.f32_or("size", 5.0);
        let dt_scale = (ctx.dt_seconds * 60.0).max(0.05);

        canvas.fill(bg.to_rgba(255));
        let w = canvas.width() as f32;
        let h = canvas.height() as f32;

        self.cycle_hue = (self.cycle_hue + (speed / 25.0) * dt_scale).rem_euclid(360.0);

        for b in &mut self.bubbles {
            let radius = b.base_size * (size / 5.0);
            let rgb = match mode {
                "Color Cycle" => Hsl::new(self.cycle_hue, 100.0, 50.0).to_rgb(),
                "Rainbow" => Hsl::new(b.x, 100.0, 50.0).to_rgb(),
                _ => color,
            };
            let alpha = (b.brightness.clamp(0.1, 1.0) * 255.0).round() as u8;
            canvas.fill_circle(b.x, b.y, radius, rgb.to_rgba(alpha));

            if speed > 0.0 {
                let s = (speed / 10.0) * dt_scale;
                b.x += b.vx * s;
                b.y += b.vy * s;
                if b.x + b.base_size >= w || b.x - b.base_size <= 0.0 {
                    b.vx = -b.vx;
                    b.x = b.x.clamp(b.base_size, w - b.base_size);
                }
                if b.y + b.base_size >= h || b.y - b.base_size <= 0.0 {
                    b.vy = -b.vy;
                    b.y = b.y.clamp(b.base_size, h - b.base_size);
                }
            }
        }
    }
}
