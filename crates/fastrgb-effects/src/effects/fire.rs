//! `Fire` and `Neon Fire` bottom-up cellular fire simulation effects.

use fastrgb_core::{CanvasFrame, ParameterDescriptor, Rgb};

use super::helpers::{CellularFireGrid, hsl_rgb};
use crate::traits::{Effect, EffectCategory, EffectContext, EffectMetadata};

#[rustfmt::skip]
const CLASSIC_FIRE_PALETTE: [Rgb; 36] = [
    Rgb::new(0x07, 0x07, 0x07), Rgb::new(0x1f, 0x07, 0x07), Rgb::new(0x2f, 0x0f, 0x07), Rgb::new(0x47, 0x0f, 0x07),
    Rgb::new(0x57, 0x17, 0x07), Rgb::new(0x67, 0x1f, 0x07), Rgb::new(0x77, 0x1f, 0x07), Rgb::new(0x8f, 0x27, 0x07),
    Rgb::new(0x9f, 0x2f, 0x07), Rgb::new(0xaf, 0x3f, 0x07), Rgb::new(0xbf, 0x47, 0x07), Rgb::new(0xc7, 0x47, 0x07),
    Rgb::new(0xdf, 0x4f, 0x07), Rgb::new(0xdf, 0x57, 0x07), Rgb::new(0xdf, 0x57, 0x07), Rgb::new(0xd7, 0x5f, 0x07),
    Rgb::new(0xd7, 0x67, 0x0f), Rgb::new(0xcf, 0x6f, 0x0f), Rgb::new(0xcf, 0x77, 0x0f), Rgb::new(0xcf, 0x7f, 0x0f),
    Rgb::new(0xcf, 0x87, 0x17), Rgb::new(0xc7, 0x87, 0x17), Rgb::new(0xc7, 0x8f, 0x17), Rgb::new(0xc7, 0x97, 0x1f),
    Rgb::new(0xbf, 0x9f, 0x1f), Rgb::new(0xbf, 0x9f, 0x1f), Rgb::new(0xbf, 0xa7, 0x27), Rgb::new(0xbf, 0xa7, 0x27),
    Rgb::new(0xbf, 0xaf, 0x2f), Rgb::new(0xb7, 0xaf, 0x2f), Rgb::new(0xb7, 0xb7, 0x2f), Rgb::new(0xb7, 0xb7, 0x37),
    Rgb::new(0xcf, 0xcf, 0x6f), Rgb::new(0xdf, 0xdf, 0x9f), Rgb::new(0xef, 0xef, 0xc7), Rgb::new(0xff, 0xff, 0xff),
];

#[rustfmt::skip]
const NEON_FIRE_PALETTE: [Rgb; 36] = [
    Rgb::new(0xff, 0x00, 0x80), Rgb::new(0xff, 0x00, 0x80), Rgb::new(0xf7, 0x00, 0x8a), Rgb::new(0xf7, 0x00, 0x8a),
    Rgb::new(0xe6, 0x00, 0x9e), Rgb::new(0xdd, 0x00, 0xa9), Rgb::new(0xd4, 0x00, 0xb4), Rgb::new(0xcc, 0x00, 0xbe),
    Rgb::new(0xb3, 0x00, 0xde), Rgb::new(0xa2, 0x00, 0xf3), Rgb::new(0x9a, 0x00, 0xfd), Rgb::new(0x8c, 0x06, 0xff),
    Rgb::new(0x80, 0x12, 0xff), Rgb::new(0x74, 0x1e, 0xff), Rgb::new(0x64, 0x2f, 0xff), Rgb::new(0x5b, 0x37, 0xff),
    Rgb::new(0x52, 0x41, 0xff), Rgb::new(0x4b, 0x48, 0xff), Rgb::new(0x40, 0x52, 0xff), Rgb::new(0x3a, 0x58, 0xff),
    Rgb::new(0x30, 0x62, 0xff), Rgb::new(0x28, 0x6a, 0xff), Rgb::new(0x25, 0x6d, 0xff), Rgb::new(0x1f, 0x73, 0xff),
    Rgb::new(0x18, 0x7a, 0xff), Rgb::new(0x10, 0x85, 0xff), Rgb::new(0x10, 0x88, 0xff), Rgb::new(0x11, 0x90, 0xff),
    Rgb::new(0x12, 0x95, 0xff), Rgb::new(0x13, 0xa2, 0xff), Rgb::new(0x14, 0xae, 0xff), Rgb::new(0x15, 0xb6, 0xff),
    Rgb::new(0x16, 0xc3, 0xff), Rgb::new(0x17, 0xd4, 0xff), Rgb::new(0x18, 0xde, 0xff), Rgb::new(0x19, 0xef, 0xff),
];

fn build_hue_palette(base_hue: f32) -> [Rgb; 36] {
    let mut palette = [Rgb::BLACK; 36];
    for (i, slot) in palette.iter_mut().enumerate() {
        let frac = i as f32 / 35.0;
        let lit = (frac * 78.0).clamp(4.0, 96.0);
        let sat = if frac > 0.8 {
            85.0 - (frac - 0.8) * 200.0
        } else {
            100.0
        };
        *slot = hsl_rgb(base_hue + frac * 18.0, sat.max(25.0), lit);
    }
    palette[35] = Rgb::WHITE;
    palette
}

fn build_rainbow_palette(hue_offset: f32, dev: f32) -> [Rgb; 36] {
    let mut palette = [Rgb::BLACK; 36];
    for (i, slot) in palette.iter_mut().enumerate() {
        let frac = i as f32 / 35.0;
        let hue = hue_offset + (i as f32) * dev.max(1.0) * 3.0;
        let lit = (18.0 + frac * 62.0).clamp(10.0, 92.0);
        *slot = hsl_rgb(hue, 100.0, lit);
    }
    palette
}

/// Classic cellular flame effect with custom, rainbow, and random color modes.
#[derive(Debug)]
pub struct FireEffect {
    grid: CellularFireGrid,
    hue: f32,
}

impl Default for FireEffect {
    fn default() -> Self {
        Self {
            grid: CellularFireGrid::new(0xF1AE_0001),
            hue: 30.0,
        }
    }
}

impl FireEffect {
    /// Creates a new [`FireEffect`].
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl Effect for FireEffect {
    fn metadata(&self) -> EffectMetadata {
        EffectMetadata {
            id: "fire",
            name: "Fire",
            description: "Rising cellular fire simulation with classic, custom, and rainbow palettes.",
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
                    "Default".to_owned(),
                    "Custom".to_owned(),
                    "Rainbow".to_owned(),
                    "Random".to_owned(),
                ],
                "Default",
            ),
            ParameterDescriptor::color("bgColor", "Background Color", Rgb::BLACK),
            ParameterDescriptor::number("speed", "Flame Speed", 1.0, 100.0, 1.0, 50.0),
            ParameterDescriptor::number("flHeight", "Flame Height", 2.0, 20.0, 1.0, 10.0),
            ParameterDescriptor::number("cstHue", "Custom Hue", 0.0, 360.0, 1.0, 30.0),
            ParameterDescriptor::number("rainDev", "Rainbow Spread", 1.0, 10.0, 1.0, 3.0),
        ]
    }

    fn reset(&mut self) {
        *self = Self::default();
    }

    fn render(&mut self, ctx: &EffectContext<'_>, canvas: &mut CanvasFrame) {
        let mode = ctx.params.str_or("colorMode", "Default");
        let bg = ctx.params.color_or("bgColor", Rgb::BLACK);
        let speed = ctx.params.f32_or("speed", 50.0);
        let fl_height = ctx.params.f32_or("flHeight", 10.0).round() as i32;
        let custom_hue = ctx.params.f32_or("cstHue", 30.0);
        let rain_dev = ctx.params.f32_or("rainDev", 3.0);

        self.hue = (self.hue + ctx.dt_seconds * (speed * 1.2)).rem_euclid(360.0);
        let palette = match mode {
            "Custom" => build_hue_palette(custom_hue),
            "Rainbow" => build_rainbow_palette(self.hue, rain_dev),
            "Random" => build_hue_palette(self.hue),
            _ => CLASSIC_FIRE_PALETTE,
        };
        let steps_per_sec = 8.0 + (speed / 100.0) * 52.0;
        self.grid.step_and_render(
            canvas,
            ctx.dt_seconds,
            steps_per_sec,
            fl_height,
            &palette,
            bg,
        );
    }
}

/// Synthwave magenta-to-cyan neon cellular fire effect.
#[derive(Debug)]
pub struct NeonFireEffect {
    grid: CellularFireGrid,
}

impl Default for NeonFireEffect {
    fn default() -> Self {
        Self {
            grid: CellularFireGrid::new(0xAE0A_F1AE),
        }
    }
}

impl NeonFireEffect {
    /// Creates a new [`NeonFireEffect`].
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl Effect for NeonFireEffect {
    fn metadata(&self) -> EffectMetadata {
        EffectMetadata {
            id: "neon_fire",
            name: "Neon Fire",
            description: "Hot neon magenta and electric cyan flames rising from the base.",
            publisher: "FastRGB",
            category: EffectCategory::Basic,
            uses_screen: false,
        }
    }

    fn descriptors(&self) -> Vec<ParameterDescriptor> {
        vec![
            ParameterDescriptor::color("bgColor", "Background Color", Rgb::BLACK),
            ParameterDescriptor::number("speed", "Flame Speed", 1.0, 100.0, 1.0, 50.0),
            ParameterDescriptor::number("flHeight", "Flame Height", 2.0, 20.0, 1.0, 10.0),
        ]
    }

    fn reset(&mut self) {
        *self = Self::default();
    }

    fn render(&mut self, ctx: &EffectContext<'_>, canvas: &mut CanvasFrame) {
        let bg = ctx.params.color_or("bgColor", Rgb::BLACK);
        let speed = ctx.params.f32_or("speed", 50.0);
        let fl_height = ctx.params.f32_or("flHeight", 10.0).round() as i32;
        let steps_per_sec = 8.0 + (speed / 100.0) * 52.0;
        self.grid.step_and_render(
            canvas,
            ctx.dt_seconds,
            steps_per_sec,
            fl_height,
            &NEON_FIRE_PALETTE,
            bg,
        );
    }
}
