//! `Drizzle` diagonal neon streak showers over a deep violet backdrop.

use fastrgb_core::{CanvasFrame, ParameterDescriptor, Rgb, Rgba};

use super::helpers::hsl_rgb;
use crate::rng::FastRng;
use crate::traits::{Effect, EffectCategory, EffectContext, EffectMetadata};

#[derive(Debug, Clone)]
struct Streak {
    x0: f32,
    y0: f32,
    head_t: f32,
    tail_t: f32,
    length: f32,
    hue: f32,
}

/// Diagonal light streaks shooting downward with customizable fade, length, and color modes.
#[derive(Debug)]
pub struct DrizzleEffect {
    streaks: Vec<Streak>,
    hue_cycle: f32,
    rng: FastRng,
    initialized: bool,
}

impl Default for DrizzleEffect {
    fn default() -> Self {
        Self {
            streaks: Vec::new(),
            hue_cycle: 0.0,
            rng: FastRng::seeded(0xD212_21E0),
            initialized: false,
        }
    }
}

impl DrizzleEffect {
    /// Creates a new [`DrizzleEffect`].
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    fn spawn_streak(&mut self, base_hue: f32, hue_range: f32, base_len: f32) -> Streak {
        Streak {
            x0: self.rng.range_f32(20.0, 420.0),
            y0: self.rng.range_f32(-60.0, 140.0),
            head_t: 0.0,
            tail_t: 0.0,
            length: self.rng.range_f32(base_len * 0.55, base_len * 1.15),
            hue: base_hue + self.rng.range_f32(-hue_range * 0.5, hue_range * 0.5),
        }
    }
}

impl Effect for DrizzleEffect {
    fn metadata(&self) -> EffectMetadata {
        EffectMetadata {
            id: "drizzle",
            name: "Drizzle",
            description: "Crisp diagonal neon rain streaks shooting across a smooth dark purple backdrop.",
            publisher: "FastRGB",
            category: EffectCategory::Basic,
            uses_screen: false,
        }
    }

    fn descriptors(&self) -> Vec<ParameterDescriptor> {
        vec![
            ParameterDescriptor::combobox(
                "frontColorMode",
                "Streak Color Mode",
                vec![
                    "Gradient".to_owned(),
                    "Color Cycle".to_owned(),
                    "Static".to_owned(),
                ],
                "Gradient",
            ),
            ParameterDescriptor::color("bgColor", "Background Color", Rgb::new(0x32, 0x00, 0x56)),
            ParameterDescriptor::color("confHue", "Streak Color", Rgb::new(0xFF, 0x00, 0x00)),
            ParameterDescriptor::number("hueRange", "Hue Spread", 0.0, 360.0, 1.0, 80.0),
            ParameterDescriptor::number("confSpeed", "Speed", 1.0, 100.0, 1.0, 50.0),
            ParameterDescriptor::number("confOpacity", "Trail Fade", 1.0, 20.0, 1.0, 6.0),
            ParameterDescriptor::number("confWidth", "Streak Width", 1.0, 20.0, 1.0, 6.0),
            ParameterDescriptor::number("confLength", "Streak Length", 20.0, 180.0, 5.0, 70.0),
        ]
    }

    fn reset(&mut self) {
        *self = Self::default();
    }

    fn render(&mut self, ctx: &EffectContext<'_>, canvas: &mut CanvasFrame) {
        let mode = ctx.params.str_or("frontColorMode", "Gradient");
        let bg = ctx.params.color_or("bgColor", Rgb::new(0x32, 0x00, 0x56));
        let streak_col = ctx.params.color_or("confHue", Rgb::new(0xFF, 0x00, 0x00));
        let hue_range = ctx.params.f32_or("hueRange", 80.0);
        let speed = ctx.params.f32_or("confSpeed", 50.0);
        let fade_amt = ctx.params.f32_or("confOpacity", 6.0);
        let width = ctx.params.f32_or("confWidth", 6.0);
        let length = ctx.params.f32_or("confLength", 70.0);

        if !self.initialized {
            canvas.fill(bg.to_rgba(255));
            self.initialized = true;
        } else {
            let alpha = ((fade_amt * 12.0).clamp(16.0, 200.0)).round() as u8;
            canvas.blend_all(Rgba::new(bg.r, bg.g, bg.b, alpha));
        }

        self.hue_cycle = (self.hue_cycle + ctx.dt_seconds * 45.0).rem_euclid(360.0);
        let base_hue = streak_col.to_hsl().h;

        while self.streaks.len() < 12 {
            let s = self.spawn_streak(base_hue, hue_range, length);
            self.streaks.push(s);
        }

        let adv = ctx.dt_seconds * (speed * 4.5);
        for s in &mut self.streaks {
            if s.head_t < s.length {
                s.head_t += adv;
            } else {
                s.tail_t += adv;
            }

            let col = match mode {
                "Static" => streak_col,
                "Color Cycle" => hsl_rgb(self.hue_cycle, 100.0, 52.0),
                _ => hsl_rgb(s.hue, 100.0, 52.0),
            };

            let x1 = s.x0 - s.tail_t * 0.7;
            let y1 = s.y0 + s.tail_t * 0.7;
            let x2 = s.x0 - s.head_t * 0.7;
            let y2 = s.y0 + s.head_t * 0.7;
            canvas.draw_line(x1, y1, x2, y2, width, col.to_rgba(235));
        }
        self.streaks.retain(|s| s.tail_t < s.length);
    }
}
