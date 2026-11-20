//! `Neon Sunset Wave` scrolling 4-stop gradient effect.

use fastrgb_core::{CanvasFrame, ParameterDescriptor, Rgb};

use super::helpers::fill_linear_gradient;
use crate::traits::{Effect, EffectCategory, EffectContext, EffectMetadata};

const NEON_STOPS: [(f32, Rgb); 5] = [
    (0.0, Rgb::new(0xFF, 0x00, 0xFF)),
    (0.25, Rgb::new(0xFF, 0x00, 0x81)),
    (0.50, Rgb::new(0x00, 0x75, 0xFF)),
    (0.75, Rgb::new(0x00, 0xFF, 0xFF)),
    (1.0, Rgb::new(0xFF, 0x00, 0xFF)),
];

const SUNSET_STOPS: [(f32, Rgb); 5] = [
    (0.0, Rgb::new(0x8C, 0x1E, 0xFF)),
    (0.25, Rgb::new(0xFF, 0x00, 0x68)),
    (0.50, Rgb::new(0xFF, 0x5F, 0x00)),
    (0.75, Rgb::new(0xFF, 0xC0, 0x1C)),
    (1.0, Rgb::new(0x8C, 0x1E, 0xFF)),
];

/// Scrolling repeating multi-stop Neon or Sunset wave gradient.
#[derive(Debug, Default)]
pub struct NeonSunsetWaveEffect {
    phase: f32,
}

impl NeonSunsetWaveEffect {
    /// Creates a new [`NeonSunsetWaveEffect`].
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl Effect for NeonSunsetWaveEffect {
    fn metadata(&self) -> EffectMetadata {
        EffectMetadata {
            id: "neon_sunset_wave",
            name: "Neon Sunset Wave",
            description: "Smooth repeating Neon or Sunset wave flowing in four directions.",
            publisher: "FastRGB",
            category: EffectCategory::Basic,
            uses_screen: false,
        }
    }

    fn descriptors(&self) -> Vec<ParameterDescriptor> {
        vec![
            ParameterDescriptor::combobox(
                "preset",
                "Palette",
                vec!["Neon".to_owned(), "Sunset".to_owned()],
                "Neon",
            ),
            ParameterDescriptor::combobox(
                "direction",
                "Direction",
                vec![
                    "Right".to_owned(),
                    "Left".to_owned(),
                    "Down".to_owned(),
                    "Up".to_owned(),
                ],
                "Right",
            ),
            ParameterDescriptor::number("speed", "Speed", 1.0, 100.0, 1.0, 50.0),
        ]
    }

    fn reset(&mut self) {
        self.phase = 0.0;
    }

    fn render(&mut self, ctx: &EffectContext<'_>, canvas: &mut CanvasFrame) {
        let preset = ctx.params.str_or("preset", "Neon");
        let dir = ctx.params.str_or("direction", "Right");
        let speed = ctx.params.f32_or("speed", 50.0);

        let sign = match dir {
            "Right" | "Down" => -1.0,
            _ => 1.0,
        };
        let horizontal = matches!(dir, "Left" | "Right");
        self.phase = (self.phase + sign * ctx.dt_seconds * (speed / 75.0)).rem_euclid(1.0);

        let stops = if preset == "Sunset" {
            &SUNSET_STOPS
        } else {
            &NEON_STOPS
        };
        fill_linear_gradient(canvas, horizontal, true, self.phase, stops, 255);
    }
}
