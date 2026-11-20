//! `Electric Colors` fading rotated spark bursts effect.

use std::f32::consts::TAU;

use fastrgb_core::{CanvasFrame, ParameterDescriptor, Rgb, Rgba};

use super::helpers::{fill_rotated_rect, hsl_rgb};
use crate::rng::FastRng;
use crate::traits::{Effect, EffectCategory, EffectContext, EffectMetadata};

#[derive(Debug, Clone)]
struct Spark {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    angle: f32,
    hue: f32,
    life: f32,
}

/// Fading electric shards popping randomly across the canvas.
#[derive(Debug)]
pub struct ElectricColorsEffect {
    sparks: Vec<Spark>,
    cycle_hue: f32,
    spawn_accum: f32,
    rng: FastRng,
    initialized: bool,
}

impl Default for ElectricColorsEffect {
    fn default() -> Self {
        Self {
            sparks: Vec::with_capacity(28),
            cycle_hue: 0.0,
            spawn_accum: 0.0,
            rng: FastRng::seeded(0xE1EC_721C),
            initialized: false,
        }
    }
}

impl ElectricColorsEffect {
    /// Creates a new [`ElectricColorsEffect`].
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl Effect for ElectricColorsEffect {
    fn metadata(&self) -> EffectMetadata {
        EffectMetadata {
            id: "electric_colors",
            name: "Electric Colors",
            description: "High-voltage spark bars popping and trailing across the canvas.",
            publisher: "FastRGB",
            category: EffectCategory::Basic,
            uses_screen: false,
        }
    }

    fn descriptors(&self) -> Vec<ParameterDescriptor> {
        vec![
            ParameterDescriptor::combobox(
                "colorMode",
                "Color Mode",
                vec![
                    "Custom".to_owned(),
                    "Rainbow".to_owned(),
                    "Color Cycle".to_owned(),
                ],
                "Custom",
            ),
            ParameterDescriptor::color("frontColor", "Spark Color", Rgb::new(0x11, 0xFF, 0x00)),
            ParameterDescriptor::color("bgColor", "Background Color", Rgb::BLACK),
            ParameterDescriptor::number("speed", "Spawn Speed", 1.0, 100.0, 1.0, 50.0),
            ParameterDescriptor::number("gWidth", "Spark Width", 10.0, 120.0, 5.0, 50.0),
            ParameterDescriptor::number("gHeight", "Spark Height", 10.0, 120.0, 5.0, 50.0),
            ParameterDescriptor::boolean("gRot", "Random Rotation", false),
        ]
    }

    fn reset(&mut self) {
        *self = Self::default();
    }

    fn render(&mut self, ctx: &EffectContext<'_>, canvas: &mut CanvasFrame) {
        let mode = ctx.params.str_or("colorMode", "Custom");
        let front = ctx
            .params
            .color_or("frontColor", Rgb::new(0x11, 0xFF, 0x00));
        let bg = ctx.params.color_or("bgColor", Rgb::BLACK);
        let speed = ctx.params.f32_or("speed", 50.0);
        let g_w = (ctx.params.f32_or("gWidth", 50.0) / 10.0).max(2.0);
        let g_h = ctx.params.f32_or("gHeight", 50.0).max(4.0);
        let random_rot = ctx.params.bool_or("gRot", false);

        if !self.initialized {
            canvas.fill(bg.to_rgba(255));
            self.initialized = true;
        } else {
            canvas.blend_all(Rgba::new(bg.r, bg.g, bg.b, 34));
        }

        self.cycle_hue = (self.cycle_hue + ctx.dt_seconds * 70.0).rem_euclid(360.0);
        self.spawn_accum += ctx.dt_seconds * (8.0 + speed * 0.45);
        while self.spawn_accum >= 1.0 && self.sparks.len() < 40 {
            self.spawn_accum -= 1.0;
            let x = self.rng.range_f32(10.0, canvas.width() as f32 - 10.0);
            let y = self.rng.range_f32(10.0, canvas.height() as f32 - 10.0);
            let angle = if random_rot {
                self.rng.range_f32(0.0, TAU)
            } else {
                0.0
            };
            self.sparks.push(Spark {
                x,
                y,
                w: g_w,
                h: g_h,
                angle,
                hue: self.rng.range_f32(0.0, 360.0),
                life: 1.0,
            });
        }

        for s in &mut self.sparks {
            let col = match mode {
                "Rainbow" => hsl_rgb(s.hue, 100.0, 52.0),
                "Color Cycle" => hsl_rgb(self.cycle_hue, 100.0, 52.0),
                _ => front,
            };
            let alpha = (s.life * 255.0).round().clamp(0.0, 255.0) as u8;
            fill_rotated_rect(canvas, s.x, s.y, s.w, s.h, s.angle, col.to_rgba(alpha));
            s.life -= ctx.dt_seconds * 2.2;
        }
        self.sparks.retain(|s| s.life > 0.0);
    }
}
