//! `Good Night!` effect (turns off all RGB LEDs).

use fastrgb_core::{CanvasFrame, ParameterDescriptor, Rgba};

use crate::traits::{Effect, EffectCategory, EffectContext, EffectMetadata};

/// Turns off all LEDs across the setup (`#000000`).
#[derive(Debug, Default)]
pub struct GoodNightEffect;

impl GoodNightEffect {
    /// Creates a new [`GoodNightEffect`].
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Effect for GoodNightEffect {
    fn metadata(&self) -> EffectMetadata {
        EffectMetadata {
            id: "good_night",
            name: "Good Night!",
            description: "Turn off all LEDs controlled by FastRGB.",
            publisher: "FeuerSturm",
            category: EffectCategory::Static,
            uses_screen: false,
        }
    }

    fn descriptors(&self) -> Vec<ParameterDescriptor> {
        Vec::new()
    }

    fn reset(&mut self) {}

    fn render(&mut self, _ctx: &EffectContext<'_>, canvas: &mut CanvasFrame) {
        canvas.fill(Rgba::BLACK);
    }
}
