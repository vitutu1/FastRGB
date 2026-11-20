//! `Gradient Pinwheel` polar conic sweep effect.

use std::f32::consts::PI;

use fastrgb_core::{CanvasFrame, Hsl, ParameterDescriptor, Rgb};

use crate::traits::{Effect, EffectCategory, EffectContext, EffectMetadata};

/// Smooth rotating pinwheel with rainbow or 2–4 custom interpolated colors.
#[derive(Debug, Default)]
pub struct GradientPinwheelEffect {
    rotation_deg: f32,
}

impl GradientPinwheelEffect {
    /// Creates a new [`GradientPinwheelEffect`].
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl Effect for GradientPinwheelEffect {
    fn metadata(&self) -> EffectMetadata {
        EffectMetadata {
            id: "gradient_pinwheel",
            name: "Gradient Pinwheel",
            description: "Smooth rotating pinwheel with rainbow or custom color transitions.",
            publisher: "Derek Huber",
            category: EffectCategory::Basic,
            uses_screen: false,
        }
    }

    fn descriptors(&self) -> Vec<ParameterDescriptor> {
        vec![
            ParameterDescriptor::boolean("rainbow", "Rainbow", false),
            ParameterDescriptor::number("colorCount", "Number Of Colors", 2.0, 4.0, 1.0, 4.0),
            ParameterDescriptor::color("color1", "Color 1", Rgb::new(0xFF, 0x80, 0x00)),
            ParameterDescriptor::color("color2", "Color 2", Rgb::new(0x80, 0x00, 0xFF)),
            ParameterDescriptor::color("color3", "Color 3", Rgb::new(0xFF, 0x00, 0x80)),
            ParameterDescriptor::color("color4", "Color 4", Rgb::new(0xFF, 0xFF, 0x00)),
            ParameterDescriptor::number("pinwheelDensity", "Pinwheel Density", 1.0, 10.0, 1.0, 1.0),
            ParameterDescriptor::number("speed", "Speed", 0.0, 100.0, 1.0, 50.0),
            ParameterDescriptor::number("xPos", "Center X Position", 0.0, 320.0, 1.0, 160.0),
            ParameterDescriptor::number("yPos", "Center Y Position", 0.0, 200.0, 1.0, 100.0),
            ParameterDescriptor::boolean("reversed", "Reverse Pinwheel", false),
        ]
    }

    fn reset(&mut self) {
        self.rotation_deg = 0.0;
    }

    fn render(&mut self, ctx: &EffectContext<'_>, canvas: &mut CanvasFrame) {
        let rainbow = ctx.params.bool_or("rainbow", false);
        let color_count = (ctx.params.f32_or("colorCount", 4.0).round() as usize).clamp(2, 4);
        let colors = [
            ctx.params.color_or("color1", Rgb::new(0xFF, 0x80, 0x00)),
            ctx.params.color_or("color2", Rgb::new(0x80, 0x00, 0xFF)),
            ctx.params.color_or("color3", Rgb::new(0xFF, 0x00, 0x80)),
            ctx.params.color_or("color4", Rgb::new(0xFF, 0xFF, 0x00)),
        ];
        let density = ctx.params.f32_or("pinwheelDensity", 1.0).max(1.0);
        let speed = ctx.params.f32_or("speed", 50.0);
        let cx = ctx.params.f32_or("xPos", 160.0);
        let cy = ctx.params.f32_or("yPos", 100.0);
        let reversed = ctx.params.bool_or("reversed", false);

        let dir = if reversed { -1.0 } else { 1.0 };
        self.rotation_deg =
            (self.rotation_deg + speed * ctx.dt_seconds * 2.0 * dir).rem_euclid(360.0);

        let w = canvas.width() as usize;
        let h = canvas.height() as usize;
        let pixels = canvas.pixels_mut();
        let seg_deg = 360.0 / (color_count as f32 * density);

        for y in 0..h {
            let dy = y as f32 - cy;
            for x in 0..w {
                let dx = x as f32 - cx;
                let angle_deg = (dy.atan2(dx) * (180.0 / PI) - self.rotation_deg).rem_euclid(360.0);
                let rgb = if rainbow {
                    Hsl::new(angle_deg * density, 100.0, 50.0).to_rgb()
                } else {
                    let seg_f = angle_deg / seg_deg;
                    let seg_idx = seg_f.floor() as usize;
                    let t = seg_f - seg_idx as f32;
                    let c0 = colors[seg_idx % color_count];
                    let c1 = colors[(seg_idx + 1) % color_count];
                    c0.lerp(c1, t)
                };
                pixels[y * w + x] = rgb.to_rgba(255);
            }
        }
    }
}
