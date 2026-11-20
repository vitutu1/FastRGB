//! `Halloween Storm` rolling dark clouds & purple lightning effect.

use fastrgb_core::{CanvasFrame, ParameterDescriptor, Rgb};

use crate::rng::FastRng;
use crate::traits::{Effect, EffectCategory, EffectContext, EffectMetadata};

/// Dark clouds drifting across a deep violet sky with periodic lightning flashes.
#[derive(Debug)]
pub struct HalloweenStormEffect {
    flash: f32,
    storm_timer: f32,
    rng: FastRng,
}

impl Default for HalloweenStormEffect {
    fn default() -> Self {
        Self {
            flash: 0.0,
            storm_timer: 0.0,
            rng: FastRng::seeded(0x4A11_0570),
        }
    }
}

impl HalloweenStormEffect {
    /// Creates a new [`HalloweenStormEffect`].
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl Effect for HalloweenStormEffect {
    fn metadata(&self) -> EffectMetadata {
        EffectMetadata {
            id: "halloween_storm",
            name: "Halloween Storm",
            description: "Black clouds rolling over a purple background with lightning strikes.",
            publisher: "FastRGB",
            category: EffectCategory::Basic,
            uses_screen: false,
        }
    }

    fn descriptors(&self) -> Vec<ParameterDescriptor> {
        vec![
            ParameterDescriptor::color("skyColor", "Sky Color", Rgb::new(0x55, 0x00, 0xAA)),
            ParameterDescriptor::color("cloudColor", "Cloud Shade", Rgb::new(0x11, 0x05, 0x22)),
            ParameterDescriptor::color("boltColor", "Lightning Tint", Rgb::new(0xDD, 0x88, 0xFF)),
            ParameterDescriptor::number("strikeSpeed", "Storm Activity", 1.0, 10.0, 1.0, 4.0),
        ]
    }

    fn reset(&mut self) {
        *self = Self::default();
    }

    fn render(&mut self, ctx: &EffectContext<'_>, canvas: &mut CanvasFrame) {
        let sky = ctx.params.color_or("skyColor", Rgb::new(0x55, 0x00, 0xAA));
        let cloud = ctx
            .params
            .color_or("cloudColor", Rgb::new(0x11, 0x05, 0x22));
        let bolt = ctx.params.color_or("boltColor", Rgb::new(0xDD, 0x88, 0xFF));
        let activity = ctx.params.f32_or("strikeSpeed", 4.0).max(1.0);
        let dt_scale = (ctx.dt_seconds * 30.0).max(0.05);

        self.storm_timer += dt_scale;
        if self.storm_timer > 50.0 / activity && self.rng.next_f32() < 0.08 * dt_scale {
            self.flash = 0.8;
            self.storm_timer = 0.0;
        }

        canvas.fill(sky.to_rgba(255));
        if self.flash > 0.05 {
            canvas.blend_all(bolt.to_rgba((self.flash.clamp(0.0, 1.0) * 255.0) as u8));
            self.flash *= 0.80_f32.powf(dt_scale);
        }

        let w = canvas.width() as f32;
        let cloud_rgba = cloud.to_rgba(245);
        for i in 0..5 {
            let cx = ((i as f32) * 80.0 + ctx.elapsed_seconds * 25.0).rem_euclid(w + 100.0) - 50.0;
            canvas.fill_circle(cx, 40.0, 55.0, cloud_rgba);
        }
    }
}
