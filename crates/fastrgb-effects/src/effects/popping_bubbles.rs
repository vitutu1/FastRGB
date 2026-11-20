//! `Popping Bubbles` expanding violet rings over an aquamarine background.

use fastrgb_core::{CanvasFrame, ParameterDescriptor, Rgb};

use super::helpers::stroke_circle;
use crate::rng::FastRng;
use crate::traits::{Effect, EffectCategory, EffectContext, EffectMetadata};

#[derive(Debug, Clone)]
struct PopRing {
    x: f32,
    y: f32,
    r: f32,
}

/// Playful expanding rings popping over a bright teal background.
#[derive(Debug)]
pub struct PoppingBubblesEffect {
    rings: Vec<PopRing>,
    spawn_timer: f32,
    rng: FastRng,
}

impl Default for PoppingBubblesEffect {
    fn default() -> Self {
        let mut rng = FastRng::seeded(0x9099_1A01);
        let rings = (0..5)
            .map(|i| PopRing {
                x: rng.range_f32(25.0, 295.0),
                y: rng.range_f32(20.0, 180.0),
                r: i as f32 * 4.0 + 2.0,
            })
            .collect();
        Self {
            rings,
            spawn_timer: 0.0,
            rng,
        }
    }
}

impl PoppingBubblesEffect {
    /// Creates a new [`PoppingBubblesEffect`].
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl Effect for PoppingBubblesEffect {
    fn metadata(&self) -> EffectMetadata {
        EffectMetadata {
            id: "popping_bubbles",
            name: "Popping Bubbles",
            description: "Crisp violet bubble rings expanding and popping across mint aquamarine.",
            publisher: "FastRGB",
            category: EffectCategory::Basic,
            uses_screen: false,
        }
    }

    fn descriptors(&self) -> Vec<ParameterDescriptor> {
        vec![
            ParameterDescriptor::color(
                "background",
                "Background Color",
                Rgb::new(0x5D, 0xFF, 0xE2),
            ),
            ParameterDescriptor::color("circleColor", "Ring Color", Rgb::new(0x7E, 0x00, 0xF5)),
            ParameterDescriptor::number("circleWidth", "Ring Width", 2.0, 20.0, 1.0, 10.0),
            ParameterDescriptor::number("radius", "Max Radius", 8.0, 50.0, 1.0, 20.0),
            ParameterDescriptor::number("speed", "Pop Speed", 1.0, 10.0, 0.5, 1.0),
        ]
    }

    fn reset(&mut self) {
        *self = Self::default();
    }

    fn render(&mut self, ctx: &EffectContext<'_>, canvas: &mut CanvasFrame) {
        let bg = ctx
            .params
            .color_or("background", Rgb::new(0x5D, 0xFF, 0xE2));
        let ring_col = ctx
            .params
            .color_or("circleColor", Rgb::new(0x7E, 0x00, 0xF5));
        let ring_w = ctx.params.f32_or("circleWidth", 10.0);
        let max_r = ctx.params.f32_or("radius", 20.0).max(5.0);
        let speed = ctx.params.f32_or("speed", 1.0);

        canvas.fill(bg.to_rgba(255));
        self.spawn_timer += ctx.dt_seconds * speed;
        if self.spawn_timer >= 0.3 && self.rings.len() < 16 {
            self.spawn_timer = 0.0;
            self.rings.push(PopRing {
                x: self.rng.range_f32(15.0, canvas.width() as f32 - 15.0),
                y: self.rng.range_f32(15.0, canvas.height() as f32 - 15.0),
                r: 2.0,
            });
        }

        let grow = ctx.dt_seconds * speed * 26.0;
        for ring in &mut self.rings {
            ring.r += grow;
            let fade = (1.0 - (ring.r / max_r).clamp(0.0, 1.0)) * 255.0;
            stroke_circle(
                canvas,
                ring.x,
                ring.y,
                ring.r,
                ring_w,
                ring_col.to_rgba(fade.round() as u8),
            );
        }
        self.rings.retain(|ring| ring.r < max_r);
    }
}
