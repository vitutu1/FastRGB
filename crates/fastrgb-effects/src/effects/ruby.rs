//! `Ruby` deep crimson crystalline flashes, diagonal ruby facets, and periodic heart pulse.

use std::f32::consts::FRAC_PI_4;

use fastrgb_core::{CanvasFrame, ParameterDescriptor, Rgb, Rgba};

use super::helpers::fill_rotated_rect;
use crate::rng::FastRng;
use crate::traits::{Effect, EffectCategory, EffectContext, EffectMetadata};

#[derive(Debug, Clone)]
struct RubyGlint {
    x: f32,
    y: f32,
    size: i32,
    life: f32,
}

/// Deep faceted crimson ruby glints with diagonal reflections and a slow pulse flash.
#[derive(Debug)]
pub struct RubyEffect {
    scroll: f32,
    pulse_timer: f32,
    pulse_alpha: f32,
    glints: Vec<RubyGlint>,
    rng: FastRng,
    initialized: bool,
}

impl Default for RubyEffect {
    fn default() -> Self {
        let mut rng = FastRng::seeded(0xAD0B_4401);
        let glints = (0..18)
            .map(|_| RubyGlint {
                x: rng.range_f32(0.0, 310.0),
                y: rng.range_f32(0.0, 190.0),
                size: rng.range_f32(6.0, 14.0).round() as i32,
                life: rng.next_f32(),
            })
            .collect();
        Self {
            scroll: 0.0,
            pulse_timer: 0.0,
            pulse_alpha: 0.0,
            glints,
            rng,
            initialized: false,
        }
    }
}

impl RubyEffect {
    /// Creates a new [`RubyEffect`].
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl Effect for RubyEffect {
    fn metadata(&self) -> EffectMetadata {
        EffectMetadata {
            id: "ruby",
            name: "Ruby",
            description: "Faceted ruby gemstones shimmering with diagonal reflections and a crimson pulse.",
            publisher: "FastRGB",
            category: EffectCategory::Basic,
            uses_screen: false,
        }
    }

    fn descriptors(&self) -> Vec<ParameterDescriptor> {
        vec![
            ParameterDescriptor::color("bgColor", "Base Ruby Color", Rgb::new(0x52, 0x00, 0x00)),
            ParameterDescriptor::color("facetColor", "Facet Highlight", Rgb::new(0xBD, 0x00, 0x21)),
            ParameterDescriptor::number("speed", "Facet Speed", 1.0, 100.0, 1.0, 50.0),
            ParameterDescriptor::number("pulseSpeed", "Pulse Speed", 1.0, 100.0, 1.0, 50.0),
        ]
    }

    fn reset(&mut self) {
        *self = Self::default();
    }

    fn render(&mut self, ctx: &EffectContext<'_>, canvas: &mut CanvasFrame) {
        let bg = ctx.params.color_or("bgColor", Rgb::new(0x52, 0x00, 0x00));
        let facet = ctx
            .params
            .color_or("facetColor", Rgb::new(0xBD, 0x00, 0x21));
        let speed = ctx.params.f32_or("speed", 50.0);
        let pulse_speed = ctx.params.f32_or("pulseSpeed", 50.0);

        if !self.initialized {
            canvas.fill(bg.to_rgba(255));
            self.initialized = true;
        } else {
            canvas.blend_all(Rgba::new(bg.r, bg.g, bg.b, 0x35));
        }

        let span = canvas.width() as f32 + 160.0;
        self.scroll = (self.scroll + ctx.dt_seconds * (speed * 1.4)).rem_euclid(span);

        for i in 0..3 {
            let x = ((self.scroll + i as f32 * 140.0) % span) - 80.0;
            fill_rotated_rect(
                canvas,
                x,
                canvas.height() as f32 * 0.5,
                28.0,
                320.0,
                FRAC_PI_4,
                facet.to_rgba(125),
            );
            fill_rotated_rect(
                canvas,
                x - 45.0,
                canvas.height() as f32 * 0.5,
                16.0,
                320.0,
                FRAC_PI_4,
                Rgba::new(0x1F, 0x02, 0x05, 160),
            );
        }

        self.pulse_timer += ctx.dt_seconds * (0.4 + pulse_speed / 80.0);
        if self.pulse_timer >= 1.5 {
            self.pulse_timer = 0.0;
            self.pulse_alpha = 0.55;
        }
        if self.pulse_alpha > 0.01 {
            canvas.blend_all(Rgba::new(
                0x59,
                0x09,
                0x00,
                (self.pulse_alpha * 255.0) as u8,
            ));
            self.pulse_alpha = (self.pulse_alpha - ctx.dt_seconds * 1.2).max(0.0);
        }

        for g in &mut self.glints {
            g.life += ctx.dt_seconds * (0.9 + speed * 0.02);
            if g.life >= 1.0 {
                g.life = 0.0;
                g.x = self.rng.range_f32(5.0, canvas.width() as f32 - 15.0);
                g.y = self.rng.range_f32(5.0, canvas.height() as f32 - 15.0);
            }
            let lum = ((g.life * std::f32::consts::PI).sin().clamp(0.0, 1.0) * 230.0) as u8;
            canvas.fill_rect(
                g.x.round() as i32,
                g.y.round() as i32,
                g.size,
                g.size,
                Rgba::new(0xFF, 0x00, 0x00, lum),
            );
        }
    }
}
