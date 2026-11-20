//! `TV Static` noise block effect.

use fastrgb_core::{CanvasFrame, Hsl, ParameterDescriptor, Rgb};

use crate::rng::FastRng;
use crate::traits::{Effect, EffectCategory, EffectContext, EffectMetadata};

/// Retro CRT static noise blocks with white speckles or 2–3 custom colors.
#[derive(Debug, Default)]
pub struct TvStaticEffect {
    rng: FastRng,
}

impl TvStaticEffect {
    /// Creates a new [`TvStaticEffect`].
    #[must_use]
    pub fn new() -> Self {
        Self {
            rng: FastRng::seeded(0x7057_A71C),
        }
    }
}

impl Effect for TvStaticEffect {
    fn metadata(&self) -> EffectMetadata {
        EffectMetadata {
            id: "tv_static",
            name: "TV Static",
            description: "TV static pixel noise with customizable block size and color palette.",
            publisher: "Jorge Pardo",
            category: EffectCategory::Basic,
            uses_screen: false,
        }
    }

    fn descriptors(&self) -> Vec<ParameterDescriptor> {
        vec![
            ParameterDescriptor::number("pixelSize", "Pixel Size", 1.0, 15.0, 1.0, 10.0),
            ParameterDescriptor::number(
                "whiteAmount",
                "Amount of White Pixels",
                0.0,
                10.0,
                1.0,
                1.0,
            ),
            ParameterDescriptor::combobox(
                "customColor",
                "Custom Colors",
                vec!["0".to_owned(), "2".to_owned(), "3".to_owned()],
                "0",
            ),
            ParameterDescriptor::color("color1", "Color 1", Rgb::new(0xFF, 0x00, 0x00)),
            ParameterDescriptor::color("color2", "Color 2", Rgb::new(0x00, 0x00, 0xFF)),
            ParameterDescriptor::color("color3", "Color 3", Rgb::new(0x00, 0xFF, 0x00)),
        ]
    }

    fn reset(&mut self) {
        self.rng = FastRng::seeded(0x7057_A71C);
    }

    fn render(&mut self, ctx: &EffectContext<'_>, canvas: &mut CanvasFrame) {
        let block = (ctx.params.f32_or("pixelSize", 10.0).round() as i32).clamp(1, 15);
        let white_amount = ctx.params.f32_or("whiteAmount", 1.0).round() as usize;
        let custom_mode = ctx.params.str_or("customColor", "0");
        let palette = [
            ctx.params.color_or("color1", Rgb::new(0xFF, 0x00, 0x00)),
            ctx.params.color_or("color2", Rgb::new(0x00, 0x00, 0xFF)),
            ctx.params.color_or("color3", Rgb::new(0x00, 0xFF, 0x00)),
        ];

        let w = canvas.width() as i32;
        let h = canvas.height() as i32;

        for x in (0..w).step_by(block as usize) {
            for y in (0..h).step_by(block as usize) {
                let rgb = match custom_mode {
                    "2" => palette[self.rng.gen_range_usize(2)],
                    "3" => palette[self.rng.gen_range_usize(3)],
                    _ => {
                        let roll = self.rng.gen_range_usize(white_amount + 2);
                        if roll < white_amount {
                            Rgb::WHITE
                        } else {
                            let hue = self.rng.gen_range_usize(360) as f32;
                            Hsl::new(hue, 100.0, 50.0).to_rgb()
                        }
                    }
                };
                canvas.fill_rect(x, y, block, block, rgb.to_rgba(255));
            }
        }
    }
}
