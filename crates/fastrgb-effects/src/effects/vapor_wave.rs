//! `Vapor Wave` angled glowing cyan bars sweeping across a vivid magenta ground.

use fastrgb_core::{CanvasFrame, ParameterDescriptor, Rgb, Rgba};

use super::helpers::fill_rotated_rect_gradient;
use crate::rng::FastRng;
use crate::traits::{Effect, EffectCategory, EffectContext, EffectMetadata};

#[derive(Debug, Clone)]
struct VaporBar {
    x: f32,
    width: f32,
    speed: f32,
    angle: f32,
}

/// Synthwave angled cyan light beams gliding over magenta.
#[derive(Debug)]
pub struct VaporWaveEffect {
    bars: Vec<VaporBar>,
    spawn_timer: f32,
    rng: FastRng,
}

impl Default for VaporWaveEffect {
    fn default() -> Self {
        let mut rng = FastRng::seeded(0x7A90_44AF);
        let mut bars = Vec::with_capacity(6);
        for _ in 0..4 {
            let start_x = rng.range_f32(0.0, 320.0);
            bars.push(Self::spawn_bar(&mut rng, start_x));
        }
        Self {
            bars,
            spawn_timer: 0.0,
            rng,
        }
    }
}

impl VaporWaveEffect {
    /// Creates a new [`VaporWaveEffect`].
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    fn spawn_bar(rng: &mut FastRng, start_x: f32) -> VaporBar {
        const ANGLES_DEG: [f32; 5] = [-45.0, -25.0, 0.0, 25.0, 45.0];
        let angle = ANGLES_DEG[rng.gen_range_usize(ANGLES_DEG.len())].to_radians();
        VaporBar {
            x: start_x,
            width: rng.range_f32(25.0, 60.0),
            speed: rng.range_f32(55.0, 120.0),
            angle,
        }
    }
}

impl Effect for VaporWaveEffect {
    fn metadata(&self) -> EffectMetadata {
        EffectMetadata {
            id: "vapor_wave",
            name: "Vapor Wave",
            description: "Angled glowing cyan ribbons gliding over vivid vaporwave magenta.",
            publisher: "FastRGB",
            category: EffectCategory::Basic,
            uses_screen: false,
        }
    }

    fn descriptors(&self) -> Vec<ParameterDescriptor> {
        vec![
            ParameterDescriptor::color("bgColor", "Background Color", Rgb::new(0xFF, 0x06, 0xF4)),
            ParameterDescriptor::color("barColor", "Bar Color", Rgb::new(0x00, 0xC5, 0xFF)),
            ParameterDescriptor::number("effectSpeed", "Speed", 1.0, 20.0, 1.0, 5.0),
            ParameterDescriptor::number("barWidth", "Bar Width", 10.0, 80.0, 1.0, 25.0),
            ParameterDescriptor::number("barSpawn", "Density", 1.0, 10.0, 1.0, 5.0),
        ]
    }

    fn reset(&mut self) {
        *self = Self::default();
    }

    fn render(&mut self, ctx: &EffectContext<'_>, canvas: &mut CanvasFrame) {
        let bg = ctx.params.color_or("bgColor", Rgb::new(0xFF, 0x06, 0xF4));
        let bar_col = ctx.params.color_or("barColor", Rgb::new(0x00, 0xC5, 0xFF));
        let speed = ctx.params.f32_or("effectSpeed", 5.0);
        let base_w = ctx.params.f32_or("barWidth", 25.0);
        let density = ctx.params.f32_or("barSpawn", 5.0).round().clamp(1.0, 10.0) as usize;

        canvas.fill(bg.to_rgba(255));
        let speed_scale = speed / 5.0;

        self.spawn_timer += ctx.dt_seconds * speed_scale;
        if self.bars.len() < density + 2 && self.spawn_timer > (1.2 / density as f32) {
            self.spawn_timer = 0.0;
            let mut b = Self::spawn_bar(&mut self.rng, -60.0);
            b.width = base_w + self.rng.range_f32(0.0, 30.0);
            self.bars.push(b);
        }

        let stops = [
            (0.0, Rgba::new(bg.r, bg.g, bg.b, 0)),
            (0.5, Rgba::new(bar_col.r, bar_col.g, bar_col.b, 235)),
            (1.0, Rgba::new(bg.r, bg.g, bg.b, 0)),
        ];

        for b in &mut self.bars {
            b.x += b.speed * speed_scale * ctx.dt_seconds;
            fill_rotated_rect_gradient(
                canvas,
                b.x,
                canvas.height() as f32 * 0.5,
                b.width,
                340.0,
                b.angle,
                &stops,
            );
        }
        self.bars.retain(|b| b.x < canvas.width() as f32 + 90.0);
    }
}
