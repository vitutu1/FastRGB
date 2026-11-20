//! `Fireflies` glowing particle field effect.

use fastrgb_core::{CanvasFrame, Hsl, ParameterDescriptor, Rgb};

use crate::rng::FastRng;
use crate::traits::{Effect, EffectCategory, EffectContext, EffectMetadata};

#[derive(Debug, Clone)]
struct Firefly {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    max_life: f32,
    life: f32,
    hue: f32,
}

/// Field of drifting glowing fireflies with single, random, or rainbow hues.
#[derive(Debug)]
pub struct FirefliesEffect {
    flies: Vec<Firefly>,
    rng: FastRng,
}

impl Default for FirefliesEffect {
    fn default() -> Self {
        Self {
            flies: Vec::new(),
            rng: FastRng::seeded(0xF1AE_F11E),
        }
    }
}

impl FirefliesEffect {
    /// Creates a new [`FirefliesEffect`].
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    fn spawn(&mut self) -> Firefly {
        let max_life = self.rng.range_f32(30.0, 100.0);
        let vx = self.rng.range_f32(-1.0, 1.0);
        let vy = self.rng.range_f32(-1.0, 1.0);
        Firefly {
            x: self.rng.range_f32(0.0, 320.0),
            y: self.rng.range_f32(0.0, 200.0),
            vx,
            vy,
            max_life,
            life: max_life,
            hue: (self.rng.gen_range_usize(12) * 30) as f32,
        }
    }
}

impl Effect for FirefliesEffect {
    fn metadata(&self) -> EffectMetadata {
        EffectMetadata {
            id: "fireflies",
            name: "Fireflies",
            description: "A field of glowing fireflies with customizable color, size, and speed.",
            publisher: "FastRGB",
            category: EffectCategory::Basic,
            uses_screen: false,
        }
    }

    fn descriptors(&self) -> Vec<ParameterDescriptor> {
        vec![
            ParameterDescriptor::color("bg", "Background Color", Rgb::BLACK),
            ParameterDescriptor::number("fg", "Firefly Hue", 0.0, 360.0, 1.0, 70.0),
            ParameterDescriptor::combobox(
                "color_mode",
                "Color Mode",
                vec![
                    "Single".to_owned(),
                    "Random".to_owned(),
                    "Rainbow".to_owned(),
                ],
                "Single",
            ),
            ParameterDescriptor::number("size", "Firefly Size", 1.0, 10.0, 1.0, 2.0),
            ParameterDescriptor::number("speed", "Speed", 0.0, 100.0, 1.0, 50.0),
            ParameterDescriptor::number("count", "Number of fireflies", 1.0, 50.0, 1.0, 25.0),
        ]
    }

    fn reset(&mut self) {
        self.flies.clear();
    }

    fn render(&mut self, ctx: &EffectContext<'_>, canvas: &mut CanvasFrame) {
        let bg = ctx.params.color_or("bg", Rgb::BLACK);
        let fg_hue = ctx.params.f32_or("fg", 70.0);
        let mode = ctx.params.str_or("color_mode", "Single");
        let radius = ctx.params.f32_or("size", 2.0).max(1.0);
        let speed = ctx.params.f32_or("speed", 50.0);
        let target_count = ctx.params.f32_or("count", 25.0).round().max(1.0) as usize;
        let dt_scale = (ctx.dt_seconds * 60.0).max(0.05);

        canvas.fill(bg.to_rgba(255));

        while self.flies.len() < target_count {
            let fly = self.spawn();
            self.flies.push(fly);
        }
        self.flies.truncate(target_count);

        for fly in &mut self.flies {
            let progress = 1.0 - (fly.life / fly.max_life).clamp(0.0, 1.0);
            let alpha = (progress * std::f32::consts::PI).sin().clamp(0.0, 1.0);
            let hue = match mode {
                "Single" => fg_hue,
                _ => fly.hue,
            };
            let color = Hsl::new(hue, 100.0, 50.0)
                .to_rgb()
                .to_rgba((alpha * 255.0).round() as u8);
            canvas.fill_circle(fly.x, fly.y, radius * 1.5, color);

            let vel_scale = (speed / 50.0) * dt_scale;
            fly.x = (fly.x + fly.vx * vel_scale).rem_euclid(canvas.width() as f32);
            fly.y = (fly.y + fly.vy * vel_scale).rem_euclid(canvas.height() as f32);
            fly.life -= 0.7 * dt_scale;
        }

        self.flies.retain(|f| f.life > 0.0);
    }
}
