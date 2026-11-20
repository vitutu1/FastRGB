//! `Spin` Archimedes spiral effect.

use std::f32::consts::PI;

use fastrgb_core::{CanvasFrame, Hsl, ParameterDescriptor, Rgb};

use crate::traits::{Effect, EffectCategory, EffectContext, EffectMetadata};

/// Rotating spiral with rainbow or custom foreground color.
#[derive(Debug, Default)]
pub struct SpinEffect;

impl SpinEffect {
    /// Creates a new [`SpinEffect`].
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Effect for SpinEffect {
    fn metadata(&self) -> EffectMetadata {
        EffectMetadata {
            id: "spin",
            name: "Spin",
            description: "A spinning spiral effect with customizable color, line width, and speed.",
            publisher: "FastRGB",
            category: EffectCategory::Basic,
            uses_screen: false,
        }
    }

    fn descriptors(&self) -> Vec<ParameterDescriptor> {
        vec![
            ParameterDescriptor::color("bg", "Background Color", Rgb::BLACK),
            ParameterDescriptor::color("fg", "Foreground Color", Rgb::new(0x00, 0x00, 0xFF)),
            ParameterDescriptor::boolean("rainbow", "Rainbow mode", true),
            ParameterDescriptor::number("colorSpeed", "Color change speed", 1.0, 10.0, 1.0, 3.0),
            ParameterDescriptor::boolean("reverse", "Reverse", false),
            ParameterDescriptor::number("speed", "Speed", 0.0, 100.0, 1.0, 50.0),
            ParameterDescriptor::number("size", "Line width", 1.0, 20.0, 1.0, 5.0),
            ParameterDescriptor::number("lineGap", "Line gap", 1.0, 10.0, 1.0, 4.0),
        ]
    }

    fn reset(&mut self) {}

    fn render(&mut self, ctx: &EffectContext<'_>, canvas: &mut CanvasFrame) {
        let bg = ctx.params.color_or("bg", Rgb::BLACK);
        let fg = ctx.params.color_or("fg", Rgb::new(0x00, 0x00, 0xFF));
        let rainbow = ctx.params.bool_or("rainbow", true);
        let color_speed = ctx.params.f32_or("colorSpeed", 3.0);
        let reverse = ctx.params.bool_or("reverse", false);
        let speed = ctx.params.f32_or("speed", 50.0);
        let thickness = ctx.params.f32_or("size", 5.0);
        let gap = ctx.params.f32_or("lineGap", 4.0);

        canvas.fill(bg.to_rgba(255));

        let ang_off = if speed > 0.0 {
            let denom = (150.0 - speed).abs().max(5.0);
            let sign = if reverse { -1.0 } else { 1.0 };
            (ctx.elapsed_seconds * 1000.0 / denom) * sign
        } else {
            0.0
        };

        let color = if rainbow {
            let hue = ang_off * (color_speed + 2.0) * 18.0;
            Hsl::new(hue, 100.0, 50.0).to_rgb().to_rgba(255)
        } else {
            fg.to_rgba(255)
        };

        let cx = canvas.width() as f32 * 0.5;
        let cy = canvas.height() as f32 * 0.5;
        let steps = 48usize;
        let increment = (2.0 * PI) / (steps as f32);
        let mut prev_x = cx;
        let mut prev_y = cy;
        let mut theta = increment;
        let max_theta = 16.0 * PI;

        while theta < max_theta {
            let nx = cx + theta * (theta + ang_off).cos() * gap;
            let ny = cy + theta * (theta + ang_off).sin() * gap;
            canvas.draw_line(prev_x, prev_y, nx, ny, thickness, color);
            prev_x = nx;
            prev_y = ny;
            theta += increment;
        }
    }
}
