//! `Falling Stars` golden star blocks raining over a pink-to-violet gradient sky.

use fastrgb_core::{CanvasFrame, ParameterDescriptor, Rgb};

use super::helpers::fill_linear_gradient;
use crate::rng::FastRng;
use crate::traits::{Effect, EffectCategory, EffectContext, EffectMetadata};

#[derive(Debug, Clone)]
struct StarDrop {
    x: f32,
    y: f32,
    vx: f32,
}

/// Golden shooting star squares gliding over a neon twilight sky.
#[derive(Debug)]
pub struct FallingStarsEffect {
    stars: Vec<StarDrop>,
    spawn_timer: f32,
    rng: FastRng,
    initialized: bool,
}

impl Default for FallingStarsEffect {
    fn default() -> Self {
        let mut rng = FastRng::seeded(0x57A2_FA11);
        let stars = (0..10)
            .map(|_| StarDrop {
                x: rng.range_f32(0.0, 320.0),
                y: rng.range_f32(0.0, 190.0),
                vx: rng.range_f32(-0.8, 0.8),
            })
            .collect();
        Self {
            stars,
            spawn_timer: 0.0,
            rng,
            initialized: false,
        }
    }
}

impl FallingStarsEffect {
    /// Creates a new [`FallingStarsEffect`].
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl Effect for FallingStarsEffect {
    fn metadata(&self) -> EffectMetadata {
        EffectMetadata {
            id: "falling_stars",
            name: "Falling Stars",
            description: "Luminous golden stars descending over a pink-and-violet twilight gradient.",
            publisher: "FastRGB",
            category: EffectCategory::Basic,
            uses_screen: false,
        }
    }

    fn descriptors(&self) -> Vec<ParameterDescriptor> {
        vec![
            ParameterDescriptor::color("starColor", "Star Color", Rgb::new(0xFF, 0xF6, 0x65)),
            ParameterDescriptor::color("skyTop", "Sky Top", Rgb::new(0xE7, 0x43, 0xCC)),
            ParameterDescriptor::color("skyBottom", "Sky Bottom", Rgb::new(0x99, 0x43, 0xE7)),
            ParameterDescriptor::number("size", "Star Size", 4.0, 24.0, 1.0, 10.0),
            ParameterDescriptor::number("speed", "Fall Speed", 5.0, 80.0, 1.0, 30.0),
            ParameterDescriptor::boolean("diagonal", "Diagonal Drift", true),
        ]
    }

    fn reset(&mut self) {
        *self = Self::default();
    }

    fn render(&mut self, ctx: &EffectContext<'_>, canvas: &mut CanvasFrame) {
        let star_col = ctx.params.color_or("starColor", Rgb::new(0xFF, 0xF6, 0x65));
        let sky_top = ctx.params.color_or("skyTop", Rgb::new(0xE7, 0x43, 0xCC));
        let sky_bot = ctx.params.color_or("skyBottom", Rgb::new(0x99, 0x43, 0xE7));
        let size = ctx.params.f32_or("size", 10.0).round().clamp(3.0, 24.0) as i32;
        let speed = ctx.params.f32_or("speed", 30.0);
        let diagonal = ctx.params.bool_or("diagonal", true);

        let alpha = if !self.initialized {
            self.initialized = true;
            255
        } else {
            78
        };
        let stops = [(0.0, sky_top), (1.0, sky_bot)];
        fill_linear_gradient(canvas, false, false, 0.0, &stops, alpha);

        self.spawn_timer += ctx.dt_seconds * (speed / 12.0);
        if self.spawn_timer >= 0.4 && self.stars.len() < 26 {
            self.spawn_timer = 0.0;
            self.stars.push(StarDrop {
                x: self.rng.range_f32(0.0, canvas.width() as f32),
                y: -(size as f32),
                vx: self.rng.range_f32(-0.85, 0.85),
            });
        }

        let dy = ctx.dt_seconds * speed * 3.2;
        for star in &mut self.stars {
            star.y += dy;
            if diagonal {
                star.x += star.vx * dy * 0.55;
            }
            canvas.fill_rect(
                star.x.round() as i32,
                star.y.round() as i32,
                size,
                size,
                star_col.to_rgba(255),
            );
        }
        self.stars
            .retain(|s| s.y < canvas.height() as f32 + size as f32);
    }
}
