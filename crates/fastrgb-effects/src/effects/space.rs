//! `Space` layered cosmic nebula blooms with outward streaking starfield.

use std::f32::consts::TAU;

use fastrgb_core::{CanvasFrame, ParameterDescriptor, Rgb, Rgba};

use super::helpers::hsl_rgb;
use crate::rng::FastRng;
use crate::traits::{Effect, EffectCategory, EffectContext, EffectMetadata};

#[derive(Debug, Clone)]
struct NebulaOrb {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    layer: usize,
    phase: f32,
}

#[derive(Debug, Clone)]
struct Star {
    angle: f32,
    dist: f32,
    speed: f32,
}

/// Deep space pulsing cosmic nebula layers with radial starlight warping outward.
#[derive(Debug)]
pub struct SpaceEffect {
    nebulae: Vec<NebulaOrb>,
    stars: Vec<Star>,
    cycle_hue: f32,
    time: f32,
    rng: FastRng,
}

impl Default for SpaceEffect {
    fn default() -> Self {
        let mut rng = FastRng::seeded(0x59AC_EC05);
        let mut nebulae = Vec::with_capacity(20);
        for layer in 0..4 {
            for _ in 0..5 {
                nebulae.push(NebulaOrb {
                    x: rng.range_f32(20.0, 300.0),
                    y: rng.range_f32(20.0, 180.0),
                    vx: rng.range_f32(-14.0, 14.0),
                    vy: rng.range_f32(-10.0, 10.0),
                    layer,
                    phase: rng.range_f32(0.0, TAU),
                });
            }
        }
        let stars = (0..50)
            .map(|_| Star {
                angle: rng.range_f32(0.0, TAU),
                dist: rng.range_f32(2.0, 185.0),
                speed: rng.range_f32(35.0, 110.0),
            })
            .collect();
        Self {
            nebulae,
            stars,
            cycle_hue: 0.0,
            time: 0.0,
            rng,
        }
    }
}

impl SpaceEffect {
    /// Creates a new [`SpaceEffect`].
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl Effect for SpaceEffect {
    fn metadata(&self) -> EffectMetadata {
        EffectMetadata {
            id: "space",
            name: "Space",
            description: "Layered pulsing violet-indigo nebulae with an outward-streaming starfield.",
            publisher: "FastRGB",
            category: EffectCategory::Basic,
            uses_screen: false,
        }
    }

    fn descriptors(&self) -> Vec<ParameterDescriptor> {
        vec![
            ParameterDescriptor::color("backColor", "Space Backdrop", Rgb::new(0x00, 0x11, 0x27)),
            ParameterDescriptor::color("starColor", "Star Color", Rgb::WHITE),
            ParameterDescriptor::number("starSize", "Star Size", 2.0, 20.0, 1.0, 10.0),
            ParameterDescriptor::number("effectSpeed", "Travel Speed", 5.0, 100.0, 1.0, 50.0),
            ParameterDescriptor::boolean("colorCycle", "Cycle Nebula Hue", true),
        ]
    }

    fn reset(&mut self) {
        *self = Self::default();
    }

    fn render(&mut self, ctx: &EffectContext<'_>, canvas: &mut CanvasFrame) {
        let bg = ctx.params.color_or("backColor", Rgb::new(0x00, 0x11, 0x27));
        let star_col = ctx.params.color_or("starColor", Rgb::WHITE);
        let star_size = (ctx.params.f32_or("starSize", 10.0) * 0.25).clamp(1.0, 6.0);
        let speed = ctx.params.f32_or("effectSpeed", 50.0);
        let cycle = ctx.params.bool_or("colorCycle", true);

        canvas.fill(bg.to_rgba(255));
        let speed_scale = speed / 50.0;
        self.time += ctx.dt_seconds * speed_scale;
        if cycle {
            self.cycle_hue =
                (self.cycle_hue + ctx.dt_seconds * 18.0 * speed_scale).rem_euclid(360.0);
        }

        let w = canvas.width() as f32;
        let h = canvas.height() as f32;
        for neb in &mut self.nebulae {
            neb.x = (neb.x + neb.vx * ctx.dt_seconds * speed_scale).rem_euclid(w);
            neb.y = (neb.y + neb.vy * ctx.dt_seconds * speed_scale).rem_euclid(h);
            let rad = (50.0 - 10.0 * neb.layer as f32) + (self.time + neb.phase).sin() * 14.0;
            let hue = 280.0 + (neb.layer as f32) * 28.0 + self.cycle_hue;
            let col = hsl_rgb(hue, 100.0, 34.0);
            canvas.fill_circle(
                neb.x,
                neb.y,
                rad.max(12.0),
                Rgba::new(col.r, col.g, col.b, 32),
            );
        }

        let cx = w * 0.5;
        let cy = h * 0.5;
        for s in &mut self.stars {
            let prev_d = s.dist;
            s.dist += s.speed * speed_scale * ctx.dt_seconds * (0.4 + s.dist / 95.0);
            if s.dist > 195.0 {
                s.dist = self.rng.range_f32(1.0, 12.0);
                s.angle = self.rng.range_f32(0.0, TAU);
                continue;
            }
            let x0 = cx + s.angle.cos() * prev_d;
            let y0 = cy + s.angle.sin() * prev_d;
            let x1 = cx + s.angle.cos() * s.dist;
            let y1 = cy + s.angle.sin() * s.dist;
            let alpha = ((s.dist / 120.0).clamp(0.15, 1.0) * 255.0).round() as u8;
            canvas.draw_line(
                x0,
                y0,
                x1,
                y1,
                (star_size * (0.4 + s.dist / 160.0)).max(1.0),
                star_col.to_rgba(alpha),
            );
        }
    }
}
