//! `Hyperspace` 3D starfield warp jump effect.

use fastrgb_core::{CanvasFrame, ParameterDescriptor, Rgb, Rgba};

use crate::rng::FastRng;
use crate::traits::{Effect, EffectCategory, EffectContext, EffectMetadata};

#[derive(Debug, Clone)]
struct WarpStar {
    x: f32,
    y: f32,
    z: f32,
}

/// Radial 3D starlight streaks simulating a sci-fi hyperspace jump.
#[derive(Debug)]
pub struct HyperspaceEffect {
    stars: Vec<WarpStar>,
    rng: FastRng,
}

impl Default for HyperspaceEffect {
    fn default() -> Self {
        Self {
            stars: Vec::new(),
            rng: FastRng::seeded(0x57A4_5A1F),
        }
    }
}

impl HyperspaceEffect {
    /// Creates a new [`HyperspaceEffect`].
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    fn spawn_star(&mut self, full_z: bool) -> WarpStar {
        WarpStar {
            x: (self.rng.next_f32() - 0.5) * 640.0,
            y: (self.rng.next_f32() - 0.5) * 400.0,
            z: if full_z {
                300.0
            } else {
                self.rng.range_f32(1.0, 300.0)
            },
        }
    }
}

impl Effect for HyperspaceEffect {
    fn metadata(&self) -> EffectMetadata {
        EffectMetadata {
            id: "hyperspace",
            name: "Hyperspace",
            description: "Imitates travelling rapidly through starfields at warp speed.",
            publisher: "FastRGB",
            category: EffectCategory::Basic,
            uses_screen: false,
        }
    }

    fn descriptors(&self) -> Vec<ParameterDescriptor> {
        vec![
            ParameterDescriptor::color("starColor", "Star Color", Rgb::new(0x77, 0xCC, 0xFF)),
            ParameterDescriptor::number("warpSpeed", "Warp Speed", 2.0, 25.0, 1.0, 12.0),
            ParameterDescriptor::number("starCount", "Star Density", 50.0, 300.0, 1.0, 150.0),
        ]
    }

    fn reset(&mut self) {
        self.stars.clear();
    }

    fn render(&mut self, ctx: &EffectContext<'_>, canvas: &mut CanvasFrame) {
        let star_color = ctx.params.color_or("starColor", Rgb::new(0x77, 0xCC, 0xFF));
        let warp_speed = ctx.params.f32_or("warpSpeed", 12.0);
        let target = ctx.params.f32_or("starCount", 150.0).round() as usize;
        let dt_scale = (ctx.dt_seconds * 30.0).max(0.05);

        while self.stars.len() < target {
            let s = self.spawn_star(false);
            self.stars.push(s);
        }
        self.stars.truncate(target);

        let fade_a = (1.0 - (1.0 - 0.35_f32).powf(dt_scale)).clamp(0.05, 1.0);
        canvas.blend_all(Rgba::new(0, 0, 8, (fade_a * 255.0) as u8));

        let cx = canvas.width() as f32 * 0.5;
        let cy = canvas.height() as f32 * 0.5;
        let line_color = star_color.to_rgba(255);

        for i in 0..self.stars.len() {
            let s = &mut self.stars[i];
            let old_x = (s.x / s.z) * 150.0 + cx;
            let old_y = (s.y / s.z) * 150.0 + cy;

            s.z -= warp_speed * dt_scale;
            if s.z <= 1.0 {
                self.stars[i] = self.spawn_star(true);
                continue;
            }

            let new_x = (s.x / s.z) * 150.0 + cx;
            let new_y = (s.y / s.z) * 150.0 + cy;
            if (0.0..320.0).contains(&new_x) && (0.0..200.0).contains(&new_y) {
                canvas.draw_line(old_x, old_y, new_x, new_y, 1.5, line_color);
            }
        }
    }
}
