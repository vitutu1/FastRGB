//! `Rainbow` spectrum sweep effect.

use fastrgb_core::{CanvasFrame, Hsl, ParameterDescriptor};

use crate::traits::{Effect, EffectCategory, EffectContext, EffectMetadata};

/// Continuous full-spectrum rainbow gradient moving horizontally or vertically.
#[derive(Debug)]
pub struct RainbowEffect {
    hue: f32,
}

impl Default for RainbowEffect {
    fn default() -> Self {
        Self { hue: 1.0 }
    }
}

impl RainbowEffect {
    /// Creates a new [`RainbowEffect`].
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl Effect for RainbowEffect {
    fn metadata(&self) -> EffectMetadata {
        EffectMetadata {
            id: "rainbow",
            name: "Rainbow",
            description: "Cycles through a continuous gradient of the full color spectrum.",
            publisher: "FastRGB",
            category: EffectCategory::Static,
            uses_screen: false,
        }
    }

    fn descriptors(&self) -> Vec<ParameterDescriptor> {
        vec![
            ParameterDescriptor::number("speed", "Rainbow Speed", 0.0, 100.0, 1.0, 50.0),
            ParameterDescriptor::boolean("vertical", "Vertical", false),
        ]
    }

    fn reset(&mut self) {
        self.hue = 1.0;
    }

    fn render(&mut self, ctx: &EffectContext<'_>, canvas: &mut CanvasFrame) {
        let speed = ctx.params.f32_or("speed", 50.0);
        let vertical = ctx.params.bool_or("vertical", false);

        let w = canvas.width() as usize;
        let h = canvas.height() as usize;
        let pixels = canvas.pixels_mut();

        if vertical {
            for y in 0..h {
                let color = Hsl::new((y as f32) - self.hue, 100.0, 50.0)
                    .to_rgb()
                    .to_rgba(255);
                let start = y * w;
                pixels[start..start + w].fill(color);
            }
        } else {
            let mut row_colors = Vec::with_capacity(w);
            for x in 0..w {
                row_colors.push(
                    Hsl::new((x as f32) - self.hue, 100.0, 50.0)
                        .to_rgb()
                        .to_rgba(255),
                );
            }
            for y in 0..h {
                let start = y * w;
                pixels[start..start + w].copy_from_slice(&row_colors);
            }
        }

        self.hue = (self.hue + (speed / 10.0) * (ctx.dt_seconds * 60.0)).rem_euclid(360.0);
    }
}
