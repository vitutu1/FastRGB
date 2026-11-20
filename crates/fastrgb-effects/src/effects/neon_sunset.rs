//! `Neon Sunset` glowing streaks effect.

use fastrgb_core::{CanvasFrame, ParameterDescriptor, Rgb};

use crate::rng::FastRng;
use crate::traits::{Effect, EffectCategory, EffectContext, EffectMetadata};

#[derive(Debug, Clone)]
struct Streak {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    horizontal: bool,
    length: f32,
    use_first_color: bool,
}

/// High-energy synthwave/neon streaks gliding across a deep tinted backdrop.
#[derive(Debug)]
pub struct NeonSunsetEffect {
    streaks: Vec<Streak>,
    rng: FastRng,
}

impl Default for NeonSunsetEffect {
    fn default() -> Self {
        Self {
            streaks: Vec::new(),
            rng: FastRng::seeded(0xAE0A_50A5),
        }
    }
}

impl NeonSunsetEffect {
    /// Creates a new [`NeonSunsetEffect`].
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    fn spawn(&mut self, horiz: bool, vert: bool, len: f32) -> Streak {
        let horizontal = match (horiz, vert) {
            (true, false) => true,
            (false, true) => false,
            _ => self.rng.gen_range_usize(2) == 0,
        };
        let forward = self.rng.gen_range_usize(2) == 0;
        let speed_mul = self.rng.range_f32(1.0, 2.4);
        if horizontal {
            Streak {
                x: if forward { -len } else { 320.0 + len },
                y: self.rng.range_f32(4.0, 196.0),
                vx: if forward { speed_mul } else { -speed_mul },
                vy: 0.0,
                horizontal: true,
                length: len,
                use_first_color: self.rng.gen_range_usize(2) == 0,
            }
        } else {
            Streak {
                x: self.rng.range_f32(4.0, 316.0),
                y: if forward { -len } else { 200.0 + len },
                vx: 0.0,
                vy: if forward { speed_mul } else { -speed_mul },
                horizontal: false,
                length: len,
                use_first_color: self.rng.gen_range_usize(2) == 0,
            }
        }
    }
}

impl Effect for NeonSunsetEffect {
    fn metadata(&self) -> EffectMetadata {
        EffectMetadata {
            id: "neon_sunset",
            name: "Neon Sunset",
            description: "A variety of Neon and Sunset glowing streaks.",
            publisher: "FastRGB",
            category: EffectCategory::Basic,
            uses_screen: false,
        }
    }

    fn descriptors(&self) -> Vec<ParameterDescriptor> {
        vec![
            ParameterDescriptor::combobox(
                "colorScheme",
                "Color Scheme",
                vec!["Neon".to_owned(), "Sunset".to_owned(), "Custom".to_owned()],
                "Neon",
            ),
            ParameterDescriptor::color("bgColor", "Background Color", Rgb::new(0x00, 0x19, 0x19)),
            ParameterDescriptor::color("color1", "Color 1", Rgb::new(0xFF, 0x00, 0x00)),
            ParameterDescriptor::color("color2", "Color 2", Rgb::new(0x00, 0x00, 0xFF)),
            ParameterDescriptor::number("speed", "Speed", 0.0, 100.0, 1.0, 24.0),
            ParameterDescriptor::number("size", "Streak size", 1.0, 100.0, 1.0, 33.0),
            ParameterDescriptor::number("streakMax", "Streak count", 1.0, 100.0, 1.0, 23.0),
            ParameterDescriptor::boolean("bStreaks", "Horizontal streaks", true),
            ParameterDescriptor::boolean("vStreaks", "Vertical Streaks", false),
        ]
    }

    fn reset(&mut self) {
        self.streaks.clear();
    }

    fn render(&mut self, ctx: &EffectContext<'_>, canvas: &mut CanvasFrame) {
        let scheme = ctx.params.str_or("colorScheme", "Neon");
        let (cl1, cl2, bg) = match scheme {
            "Neon" => (
                Rgb::new(0xFF, 0x00, 0xFF),
                Rgb::new(0x00, 0xFF, 0xFF),
                Rgb::new(0x00, 0x19, 0x19),
            ),
            "Sunset" => (
                Rgb::new(0xFF, 0x90, 0x1F),
                Rgb::new(0xFF, 0x29, 0x75),
                Rgb::new(0x08, 0x00, 0x02),
            ),
            _ => (
                ctx.params.color_or("color1", Rgb::new(0xFF, 0x00, 0x00)),
                ctx.params.color_or("color2", Rgb::new(0x00, 0x00, 0xFF)),
                ctx.params.color_or("bgColor", Rgb::new(0x00, 0x19, 0x19)),
            ),
        };
        let speed = ctx.params.f32_or("speed", 24.0);
        let size = ctx.params.f32_or("size", 33.0);
        let max_count = (ctx.params.f32_or("streakMax", 23.0).round() as usize).max(1) * 2;
        let horiz = ctx.params.bool_or("bStreaks", true);
        let vert = ctx.params.bool_or("vStreaks", false);
        let dt_scale = (ctx.dt_seconds * 60.0).max(0.05);

        canvas.fill(bg.to_rgba(255));

        if (horiz || vert) && self.streaks.len() < max_count {
            let streak = self.spawn(horiz, vert, size * 1.6);
            self.streaks.push(streak);
        }

        let speed_px = (speed / 12.0) * dt_scale;
        for streak in &mut self.streaks {
            let color = if streak.use_first_color { cl1 } else { cl2 };
            if streak.horizontal {
                canvas.fill_rect(
                    (streak.x - streak.length * 0.5).round() as i32,
                    (streak.y - 3.0).round() as i32,
                    streak.length.round() as i32,
                    6,
                    color.to_rgba(210),
                );
                streak.x += streak.vx * speed_px;
            } else {
                canvas.fill_rect(
                    (streak.x - 3.0).round() as i32,
                    (streak.y - streak.length * 0.5).round() as i32,
                    6,
                    streak.length.round() as i32,
                    color.to_rgba(210),
                );
                streak.y += streak.vy * speed_px;
            }
        }

        self.streaks.retain(|s| {
            s.x >= -s.length - 10.0
                && s.x <= 330.0 + s.length
                && s.y >= -s.length - 10.0
                && s.y <= 210.0 + s.length
        });
    }
}
