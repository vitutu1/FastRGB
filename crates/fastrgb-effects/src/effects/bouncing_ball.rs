//! `Bouncing Ball` effect with optional trail and bounce-triggered random colors.

use std::collections::VecDeque;

use fastrgb_core::{CanvasFrame, ParameterDescriptor, Rgb};

use crate::rng::FastRng;
use crate::traits::{Effect, EffectCategory, EffectContext, EffectMetadata};

/// Bouncing ball animation with motion trail and reactive bounce modes.
#[derive(Debug)]
pub struct BouncingBallEffect {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    bg_color: Rgb,
    ball_color: Rgb,
    trail: VecDeque<(f32, f32)>,
    rng: FastRng,
}

impl Default for BouncingBallEffect {
    fn default() -> Self {
        Self {
            x: 100.0,
            y: 40.0,
            vx: 5.0,
            vy: 5.0,
            bg_color: Rgb::BLACK,
            ball_color: Rgb::new(0xFF, 0x00, 0x00),
            trail: VecDeque::with_capacity(5),
            rng: FastRng::seeded(0xBA11_B00C),
        }
    }
}

impl BouncingBallEffect {
    /// Creates a new [`BouncingBallEffect`].
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    fn random_color(&mut self) -> Rgb {
        Rgb::new(
            self.rng.gen_range_usize(256) as u8,
            self.rng.gen_range_usize(256) as u8,
            self.rng.gen_range_usize(256) as u8,
        )
    }
}

impl Effect for BouncingBallEffect {
    fn metadata(&self) -> EffectMetadata {
        EffectMetadata {
            id: "bouncing_ball",
            name: "Bouncing Ball",
            description: "Bouncing orb with optional trail and color-on-bounce modes.",
            publisher: "J0R6IT0",
            category: EffectCategory::Basic,
            uses_screen: false,
        }
    }

    fn descriptors(&self) -> Vec<ParameterDescriptor> {
        vec![
            ParameterDescriptor::color("bcColor", "Background Color", Rgb::BLACK),
            ParameterDescriptor::color("ballColor", "Ball Color", Rgb::new(0xFF, 0x00, 0x00)),
            ParameterDescriptor::combobox(
                "mode",
                "Mode",
                vec![
                    "Normal".to_owned(),
                    "Random BG on bounce".to_owned(),
                    "Random ball on bounce".to_owned(),
                ],
                "Normal",
            ),
            ParameterDescriptor::number("ballSpeed", "Ball Speed", 1.0, 15.0, 1.0, 5.0),
            ParameterDescriptor::number("ballSize", "Ball Size", 1.0, 40.0, 1.0, 30.0),
            ParameterDescriptor::boolean("trail", "Trail", true),
            ParameterDescriptor::color("trailColor", "Trail Color", Rgb::WHITE),
        ]
    }

    fn reset(&mut self) {
        *self = Self::default();
    }

    fn render(&mut self, ctx: &EffectContext<'_>, canvas: &mut CanvasFrame) {
        let bc_color = ctx.params.color_or("bcColor", Rgb::BLACK);
        let param_ball = ctx.params.color_or("ballColor", Rgb::new(0xFF, 0x00, 0x00));
        let mode = ctx.params.str_or("mode", "Normal");
        let speed = ctx.params.f32_or("ballSpeed", 5.0);
        let size = ctx.params.f32_or("ballSize", 30.0);
        let show_trail = ctx.params.bool_or("trail", true);
        let trail_color = ctx.params.color_or("trailColor", Rgb::WHITE);
        let dt_scale = (ctx.dt_seconds * 60.0).max(0.05);

        self.vx = self.vx.signum() * speed;
        self.vy = self.vy.signum() * speed;

        self.trail.push_front((self.x, self.y));
        if self.trail.len() > 4 {
            self.trail.pop_back();
        }

        self.x += self.vx * dt_scale;
        self.y += self.vy * dt_scale;

        let w = canvas.width() as f32;
        let h = canvas.height() as f32;
        let mut bounced = false;
        if self.x > w - size {
            self.x = w - size;
            self.vx = -speed;
            bounced = true;
        } else if self.x < size {
            self.x = size;
            self.vx = speed;
            bounced = true;
        }
        if self.y > h - size {
            self.y = h - size;
            self.vy = -speed;
            bounced = true;
        } else if self.y < size {
            self.y = size;
            self.vy = speed;
            bounced = true;
        }

        if bounced {
            if mode == "Random BG on bounce" {
                self.bg_color = self.random_color();
            } else if mode == "Random ball on bounce" {
                self.ball_color = self.random_color();
            }
        }

        let bg = if mode == "Random BG on bounce" {
            self.bg_color
        } else {
            bc_color
        };
        let ball = if mode == "Random ball on bounce" {
            self.ball_color
        } else {
            param_ball
        };

        canvas.fill(bg.to_rgba(255));

        if show_trail {
            const ALPHAS: [u8; 4] = [140, 100, 65, 35];
            for (i, &(tx, ty)) in self.trail.iter().enumerate() {
                let alpha = ALPHAS.get(i).copied().unwrap_or(20);
                let r = (size * (1.0 - (i as f32) * 0.14)).max(2.0);
                canvas.fill_circle(tx, ty, r, trail_color.to_rgba(alpha));
            }
        }

        canvas.fill_circle(self.x, self.y, size, ball.to_rgba(255));
    }
}
