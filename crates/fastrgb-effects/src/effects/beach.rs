//! `Beach` ocean tide & white foam shoreline effect.

use fastrgb_core::{CanvasFrame, ParameterDescriptor, Rgb};

use crate::traits::{Effect, EffectCategory, EffectContext, EffectMetadata};

/// Gentle ocean waves and white foam lapping onto golden sand.
#[derive(Debug, Default)]
pub struct BeachEffect {
    t: f32,
}

impl BeachEffect {
    /// Creates a new [`BeachEffect`].
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl Effect for BeachEffect {
    fn metadata(&self) -> EffectMetadata {
        EffectMetadata {
            id: "beach",
            name: "Beach",
            description: "Gentle ocean waves with white seafoam lapping onto the shore.",
            publisher: "FastRGB",
            category: EffectCategory::Basic,
            uses_screen: false,
        }
    }

    fn descriptors(&self) -> Vec<ParameterDescriptor> {
        vec![
            ParameterDescriptor::color("seaColor", "Ocean Color", Rgb::new(0x00, 0x66, 0xAA)),
            ParameterDescriptor::color("sandColor", "Sand Color", Rgb::new(0xD4, 0xA3, 0x62)),
            ParameterDescriptor::color("foamColor", "Wave Foam", Rgb::WHITE),
            ParameterDescriptor::number("waveSpeed", "Tide Speed", 1.0, 10.0, 1.0, 4.0),
        ]
    }

    fn reset(&mut self) {
        self.t = 0.0;
    }

    fn render(&mut self, ctx: &EffectContext<'_>, canvas: &mut CanvasFrame) {
        let sea = ctx.params.color_or("seaColor", Rgb::new(0x00, 0x66, 0xAA));
        let sand = ctx.params.color_or("sandColor", Rgb::new(0xD4, 0xA3, 0x62));
        let foam = ctx.params.color_or("foamColor", Rgb::WHITE);
        let speed = ctx.params.f32_or("waveSpeed", 4.0);
        let dt_scale = (ctx.dt_seconds * 30.0).max(0.05);

        self.t += speed * 0.02 * dt_scale;
        let tide_height = self.t.sin() * 45.0 + 105.0;
        let shallow = Rgb::new(0, 180, 200);

        let w = canvas.width() as usize;
        let h = canvas.height() as usize;
        let pixels = canvas.pixels_mut();

        for x in 0..w {
            let xf = x as f32;
            let shore_y = tide_height
                + (xf * 0.04 + self.t * 2.0).sin() * 6.0
                + (xf * 0.02 + self.t).cos() * 4.0;
            for y in 0..h {
                let yf = y as f32;
                let color = if yf < shore_y - 3.0 {
                    let depth = (yf / shore_y.max(1.0)).clamp(0.0, 1.0);
                    sea.lerp(shallow, depth)
                } else if yf <= shore_y + 1.5 {
                    foam
                } else {
                    sand
                };
                pixels[y * w + x] = color.to_rgba(255);
            }
        }
    }
}
