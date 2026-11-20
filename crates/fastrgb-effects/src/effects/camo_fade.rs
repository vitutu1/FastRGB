//! `Camo Fade` multi-stop gradient wave with ambient pulse accents.

use fastrgb_core::{CanvasFrame, ParameterDescriptor, Rgb};

use super::helpers::fill_linear_gradient;
use crate::traits::{Effect, EffectCategory, EffectContext, EffectMetadata};

/// Multi-stop warm/violet/navy gradient sweep with drifting highlights.
#[derive(Debug, Default)]
pub struct CamoFadeEffect {
    phase: f32,
}

impl CamoFadeEffect {
    /// Creates a new [`CamoFadeEffect`].
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl Effect for CamoFadeEffect {
    fn metadata(&self) -> EffectMetadata {
        EffectMetadata {
            id: "camo_fade",
            name: "Camo Fade",
            description: "Layered warm ochre, magenta, violet, and deep cobalt gradient waves.",
            publisher: "FastRGB",
            category: EffectCategory::Basic,
            uses_screen: false,
        }
    }

    fn descriptors(&self) -> Vec<ParameterDescriptor> {
        vec![
            ParameterDescriptor::number("speed", "Speed", 1.0, 100.0, 1.0, 35.0),
            ParameterDescriptor::color("color1", "Stop 1", Rgb::new(0x9E, 0x60, 0x23)),
            ParameterDescriptor::color("color2", "Stop 2", Rgb::new(0x9E, 0x21, 0x56)),
            ParameterDescriptor::color("color3", "Stop 3", Rgb::new(0x5C, 0x41, 0x78)),
            ParameterDescriptor::color("color4", "Stop 4", Rgb::new(0x00, 0x3E, 0x80)),
        ]
    }

    fn reset(&mut self) {
        self.phase = 0.0;
    }

    fn render(&mut self, ctx: &EffectContext<'_>, canvas: &mut CanvasFrame) {
        let speed = ctx.params.f32_or("speed", 35.0);
        let c1 = ctx.params.color_or("color1", Rgb::new(0x9E, 0x60, 0x23));
        let c2 = ctx.params.color_or("color2", Rgb::new(0x9E, 0x21, 0x56));
        let c3 = ctx.params.color_or("color3", Rgb::new(0x5C, 0x41, 0x78));
        let c4 = ctx.params.color_or("color4", Rgb::new(0x00, 0x3E, 0x80));

        self.phase = (self.phase + ctx.dt_seconds * (speed / 120.0)).rem_euclid(1.0);
        let stops = [(0.0, c1), (0.25, c2), (0.50, c3), (0.75, c4), (1.0, c1)];
        fill_linear_gradient(canvas, true, true, self.phase, &stops, 255);
    }
}
