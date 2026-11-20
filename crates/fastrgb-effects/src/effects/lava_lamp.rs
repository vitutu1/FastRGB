//! `Lava Lamp` rising thermal wax metaballs effect.

use fastrgb_core::{CanvasFrame, ParameterDescriptor, Rgb};

use crate::rng::FastRng;
use crate::traits::{Effect, EffectCategory, EffectContext, EffectMetadata};

#[derive(Debug, Clone)]
struct WaxBlob {
    x: f32,
    y: f32,
    vx: f32,
    vy_mul: f32,
    radius: f32,
}

/// Rising molten wax blobs floating inside a tinted liquid column.
#[derive(Debug)]
pub struct LavaLampEffect {
    blobs: Vec<WaxBlob>,
    rng: FastRng,
}

impl Default for LavaLampEffect {
    fn default() -> Self {
        Self {
            blobs: Vec::new(),
            rng: FastRng::seeded(0x1A7A_1A3B),
        }
    }
}

impl LavaLampEffect {
    /// Creates a new [`LavaLampEffect`].
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    fn spawn_blob(&mut self, random_y: bool) -> WaxBlob {
        WaxBlob {
            x: self.rng.range_f32(50.0, 270.0),
            y: if random_y {
                self.rng.range_f32(0.0, 200.0)
            } else {
                230.0
            },
            vx: self.rng.range_f32(-0.2, 0.2),
            vy_mul: self.rng.range_f32(0.5, 1.5),
            radius: self.rng.range_f32(22.0, 40.0),
        }
    }
}

impl Effect for LavaLampEffect {
    fn metadata(&self) -> EffectMetadata {
        EffectMetadata {
            id: "lava_lamp",
            name: "Lava Lamp",
            description: "Colorful thermal wax blobs rising and drifting like a lava lamp.",
            publisher: "FastRGB",
            category: EffectCategory::Basic,
            uses_screen: false,
        }
    }

    fn descriptors(&self) -> Vec<ParameterDescriptor> {
        vec![
            ParameterDescriptor::color("waxColor", "Wax Color", Rgb::new(0xFF, 0x00, 0x55)),
            ParameterDescriptor::color("fluidColor", "Liquid Color", Rgb::new(0x1A, 0x00, 0x33)),
            ParameterDescriptor::number("blobCount", "Blob Count", 3.0, 10.0, 1.0, 6.0),
            ParameterDescriptor::number("speed", "Flow Speed", 1.0, 8.0, 1.0, 3.0),
        ]
    }

    fn reset(&mut self) {
        self.blobs.clear();
    }

    fn render(&mut self, ctx: &EffectContext<'_>, canvas: &mut CanvasFrame) {
        let wax = ctx.params.color_or("waxColor", Rgb::new(0xFF, 0x00, 0x55));
        let fluid = ctx
            .params
            .color_or("fluidColor", Rgb::new(0x1A, 0x00, 0x33));
        let count = ctx.params.f32_or("blobCount", 6.0).round() as usize;
        let speed = ctx.params.f32_or("speed", 3.0);
        let dt_scale = (ctx.dt_seconds * 30.0).max(0.05);

        while self.blobs.len() < count {
            let b = self.spawn_blob(true);
            self.blobs.push(b);
        }
        self.blobs.truncate(count);

        canvas.fill(fluid.to_rgba(255));
        let wax_rgba = wax.to_rgba(245);

        for i in 0..self.blobs.len() {
            let b = &mut self.blobs[i];
            b.x = (b.x + b.vx * dt_scale).clamp(30.0, 290.0);
            b.y -= b.vy_mul * (speed / 3.0) * dt_scale;
            if b.y < -30.0 {
                self.blobs[i] = self.spawn_blob(false);
            }
            let b = &self.blobs[i];
            canvas.fill_circle(b.x, b.y, b.radius, wax_rgba);
        }
    }
}
