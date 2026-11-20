//! `Rainbow Data` 4-quadrant outward digital pixel streams.

use std::f32::consts::FRAC_PI_2;

use fastrgb_core::{CanvasFrame, ParameterDescriptor, Rgb, Rgba};

use super::helpers::hsl_rgb;
use crate::rng::FastRng;
use crate::traits::{Effect, EffectCategory, EffectContext, EffectMetadata};

#[derive(Debug, Clone)]
struct DataPacket {
    offset: f32,
    dist: f32,
    quadrant: usize,
    hue: f32,
}

/// Digital data blocks radiating along four cardinal streams with customizable fade and color mode.
#[derive(Debug)]
pub struct RainbowDataEffect {
    packets: Vec<DataPacket>,
    cycle_hue: f32,
    spawn_accum: f32,
    rng: FastRng,
    initialized: bool,
}

impl Default for RainbowDataEffect {
    fn default() -> Self {
        Self {
            packets: Vec::with_capacity(48),
            cycle_hue: 0.0,
            spawn_accum: 0.0,
            rng: FastRng::seeded(0xDA7A_9A1A),
            initialized: false,
        }
    }
}

impl RainbowDataEffect {
    /// Creates a new [`RainbowDataEffect`].
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl Effect for RainbowDataEffect {
    fn metadata(&self) -> EffectMetadata {
        EffectMetadata {
            id: "rainbow_data",
            name: "Rainbow Data",
            description: "Four-way outward streaming digital pixel packets with trailing decay.",
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
                    "Rainbow Gradient".to_owned(),
                    "Color Cycle".to_owned(),
                    "Custom".to_owned(),
                    "Random".to_owned(),
                ],
                "Rainbow Gradient",
            ),
            ParameterDescriptor::color("pixColor", "Packet Color", Rgb::new(0xAE, 0xFF, 0x00)),
            ParameterDescriptor::color("backColor", "Background Color", Rgb::new(0x03, 0x03, 0x03)),
            ParameterDescriptor::number("size", "Block Size", 6.0, 40.0, 2.0, 20.0),
            ParameterDescriptor::number("speed", "Speed", 1.0, 100.0, 1.0, 50.0),
            ParameterDescriptor::number("fade", "Trail Fade", 10.0, 100.0, 5.0, 50.0),
        ]
    }

    fn reset(&mut self) {
        *self = Self::default();
    }

    fn render(&mut self, ctx: &EffectContext<'_>, canvas: &mut CanvasFrame) {
        let mode = ctx.params.str_or("colorMode", "Rainbow Gradient");
        let pix_col = ctx.params.color_or("pixColor", Rgb::new(0xAE, 0xFF, 0x00));
        let bg = ctx.params.color_or("backColor", Rgb::new(0x03, 0x03, 0x03));
        let size = ctx.params.f32_or("size", 20.0).max(4.0);
        let speed = ctx.params.f32_or("speed", 50.0);
        let fade = ctx.params.f32_or("fade", 50.0);

        if !self.initialized {
            canvas.fill(bg.to_rgba(255));
            self.initialized = true;
        } else {
            let fade_alpha = ((fade * 1.1).clamp(15.0, 160.0)).round() as u8;
            canvas.blend_all(Rgba::new(bg.r, bg.g, bg.b, fade_alpha));
        }

        self.cycle_hue = (self.cycle_hue + ctx.dt_seconds * 55.0).rem_euclid(360.0);
        self.spawn_accum += ctx.dt_seconds * (6.0 + speed * 0.35);
        while self.spawn_accum >= 1.0 && self.packets.len() < 80 {
            self.spawn_accum -= 1.0;
            self.packets.push(DataPacket {
                offset: self.rng.range_f32(-90.0, 90.0),
                dist: 0.0,
                quadrant: self.rng.gen_range_usize(4),
                hue: self.rng.range_f32(0.0, 360.0),
            });
        }

        let cx = canvas.width() as f32 * 0.5;
        let cy = canvas.height() as f32 * 0.5;
        let step = ctx.dt_seconds * (40.0 + speed * 3.0);
        let block = size.round() as i32;

        for p in &mut self.packets {
            p.dist += step;
            let angle = (p.quadrant as f32) * FRAC_PI_2;
            let (ca, sa) = (angle.cos(), angle.sin());
            let px = cx + ca * p.offset - sa * p.dist;
            let py = cy + sa * p.offset + ca * p.dist;

            let col = match mode {
                "Custom" => pix_col,
                "Color Cycle" => hsl_rgb(self.cycle_hue, 100.0, 50.0),
                "Random" => hsl_rgb(p.hue, 100.0, 50.0),
                _ => hsl_rgb(p.dist * 1.4 + p.quadrant as f32 * 45.0, 100.0, 50.0),
            };
            canvas.fill_rect(
                (px - size * 0.5).round() as i32,
                (py - size * 0.5).round() as i32,
                block,
                block,
                col.to_rgba(255),
            );
        }
        self.packets.retain(|p| p.dist < 220.0);
    }
}
