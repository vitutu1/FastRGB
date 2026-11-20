//! `Solid Color` effect (with optional sinusoidal breathing).

use std::f32::consts::PI;

use fastrgb_core::{CanvasFrame, ParameterDescriptor, Rgb, Rgba};

use crate::traits::{Effect, EffectCategory, EffectContext, EffectMetadata};

/// Single steady or breathing color across the entire canvas.
#[derive(Debug, Default)]
pub struct SolidColorEffect {
    phase: f32,
}

impl SolidColorEffect {
    /// Creates a new [`SolidColorEffect`].
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl Effect for SolidColorEffect {
    fn metadata(&self) -> EffectMetadata {
        EffectMetadata {
            id: "solid_color",
            name: "Solid Color",
            description: "Steady single hue across the setup with optional smooth breathing.",
            publisher: "FastRGB",
            category: EffectCategory::Static,
            uses_screen: false,
        }
    }

    fn descriptors(&self) -> Vec<ParameterDescriptor> {
        vec![
            ParameterDescriptor::color("color", "Color", Rgb::new(0x00, 0xBB, 0xFF)),
            ParameterDescriptor::boolean("breathe", "Breathing", true),
            ParameterDescriptor::number("speed", "Breathing Speed", 0.0, 100.0, 1.0, 75.0),
        ]
    }

    fn reset(&mut self) {
        self.phase = 0.0;
    }

    fn render(&mut self, ctx: &EffectContext<'_>, canvas: &mut CanvasFrame) {
        let color = ctx.params.color_or("color", Rgb::new(0x00, 0xBB, 0xFF));
        let breathe = ctx.params.bool_or("breathe", true);
        let speed = ctx.params.f32_or("speed", 75.0);

        let alpha = if breathe {
            let a = self.phase.sin() * 0.5 + 0.5;
            self.phase = (self.phase + (PI / 4.0) * (speed / 1000.0) * (ctx.dt_seconds * 60.0))
                .rem_euclid(2.0 * PI);
            (a * 255.0).round().clamp(0.0, 255.0) as u8
        } else {
            255
        };

        let blended = Rgba::BLACK.blend_over(color.to_rgba(alpha));
        canvas.fill(blended);
    }
}
