//! `Neon Shift` palette cross-fade effect.

use fastrgb_core::{CanvasFrame, Hsl, ParameterDescriptor};

use crate::traits::{Effect, EffectCategory, EffectContext, EffectMetadata};

/// FastRGB signature neon palette (`[hue, sat, lightness]`).
const NEON_PALETTE: [(f32, f32, f32); 4] = [
    (322.0, 63.0, 70.0),
    (280.0, 50.0, 50.0),
    (230.0, 36.0, 52.0),
    (202.0, 64.0, 70.0),
];

/// Smoothly cycles through electric pink, purple, indigo, and cyan hues.
#[derive(Debug, Default)]
pub struct NeonShiftEffect {
    phase: f32,
    palette_index: usize,
}

impl NeonShiftEffect {
    /// Creates a new [`NeonShiftEffect`].
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl Effect for NeonShiftEffect {
    fn metadata(&self) -> EffectMetadata {
        EffectMetadata {
            id: "neon_shift",
            name: "Neon Shift",
            description: "Smoothly cycles through a vibrant loop of electric blue, purple, and pink.",
            publisher: "FastRGB",
            category: EffectCategory::Static,
            uses_screen: false,
        }
    }

    fn descriptors(&self) -> Vec<ParameterDescriptor> {
        vec![ParameterDescriptor::number(
            "speed",
            "Shift Speed",
            0.0,
            100.0,
            1.0,
            50.0,
        )]
    }

    fn reset(&mut self) {
        self.phase = 0.0;
        self.palette_index = 0;
    }

    fn render(&mut self, ctx: &EffectContext<'_>, canvas: &mut CanvasFrame) {
        let speed = ctx.params.f32_or("speed", 50.0);
        let (hue, _sat, lit) = NEON_PALETTE[self.palette_index];

        // Match original 5% alpha decay per 60 Hz frame, scaled cleanly by dt.
        let alpha_f = 1.0 - (1.0 - 0.05_f32).powf((ctx.dt_seconds * 60.0).max(0.01));
        let alpha = (alpha_f * 255.0).round().clamp(1.0, 255.0) as u8;
        let target = Hsl::new(hue, 100.0, lit * 0.5).to_rgb().to_rgba(alpha);
        canvas.blend_all(target);

        self.phase += (speed / 4.0) * (ctx.dt_seconds * 60.0);
        if self.phase > 500.0 {
            self.phase = 0.0;
            self.palette_index = (self.palette_index + 1) % NEON_PALETTE.len();
        }
    }
}
