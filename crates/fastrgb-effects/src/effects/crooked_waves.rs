//! `Crooked Waves` diagonal Sweeping bars effect.

use std::f32::consts::FRAC_PI_4;

use fastrgb_core::{CanvasFrame, ParameterDescriptor, Rgb};

use super::helpers::{fill_rotated_rect, hsl_rgb};
use crate::traits::{Effect, EffectCategory, EffectContext, EffectMetadata};

/// Diagonal rotated waves sweeping across the canvas.
#[derive(Debug)]
pub struct CrookedWavesEffect {
    offset: f32,
    hue: f32,
}

impl Default for CrookedWavesEffect {
    fn default() -> Self {
        Self {
            offset: 0.0,
            hue: 220.0,
        }
    }
}

impl CrookedWavesEffect {
    /// Creates a new [`CrookedWavesEffect`].
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl Effect for CrookedWavesEffect {
    fn metadata(&self) -> EffectMetadata {
        EffectMetadata {
            id: "crooked_waves",
            name: "Crooked Waves",
            description: "Angled 45-degree color bars sweeping continuously across the canvas.",
            publisher: "FastRGB",
            category: EffectCategory::Basic,
            uses_screen: false,
        }
    }

    fn descriptors(&self) -> Vec<ParameterDescriptor> {
        vec![
            ParameterDescriptor::color("bgColor", "Background Color", Rgb::WHITE),
            ParameterDescriptor::color("color", "Wave Color", Rgb::new(0x0A, 0x00, 0xFF)),
            ParameterDescriptor::boolean("randomColors", "Cycle Wave Colors", false),
            ParameterDescriptor::number("speed", "Speed", 1.0, 20.0, 1.0, 3.0),
            ParameterDescriptor::number("barWidth", "Wave Width", 5.0, 120.0, 1.0, 30.0),
            ParameterDescriptor::number("barAmount", "Waves", 1.0, 8.0, 1.0, 2.0),
        ]
    }

    fn reset(&mut self) {
        *self = Self::default();
    }

    fn render(&mut self, ctx: &EffectContext<'_>, canvas: &mut CanvasFrame) {
        let bg = ctx.params.color_or("bgColor", Rgb::WHITE);
        let color = ctx.params.color_or("color", Rgb::new(0x0A, 0x00, 0xFF));
        let random_colors = ctx.params.bool_or("randomColors", false);
        let speed = ctx.params.f32_or("speed", 3.0);
        let bar_w = ctx.params.f32_or("barWidth", 30.0).max(4.0);
        let bar_count = ctx.params.f32_or("barAmount", 2.0).round().clamp(1.0, 8.0) as usize;

        let span = canvas.width() as f32 + canvas.height() as f32 + bar_w * 2.0;
        self.offset = (self.offset + ctx.dt_seconds * speed * 45.0).rem_euclid(span);
        self.hue = (self.hue + ctx.dt_seconds * 60.0).rem_euclid(360.0);

        canvas.fill(bg.to_rgba(255));
        let spacing = span / bar_count as f32;
        for i in 0..bar_count {
            let pos = ((self.offset + i as f32 * spacing) % span) - bar_w;
            let bar_col = if random_colors {
                hsl_rgb(
                    self.hue + i as f32 * (360.0 / bar_count as f32),
                    100.0,
                    50.0,
                )
            } else {
                color
            };
            fill_rotated_rect(
                canvas,
                pos,
                canvas.height() as f32 * 0.5,
                bar_w,
                460.0,
                -FRAC_PI_4,
                bar_col.to_rgba(255),
            );
        }
    }
}
