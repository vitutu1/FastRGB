//! `Plasma` multi-octave sinusoidal shader effect.

use std::f32::consts::PI;

use fastrgb_core::{CanvasFrame, ParameterDescriptor, Rgba};

use crate::traits::{Effect, EffectCategory, EffectContext, EffectMetadata};

/// Smooth multi-octave plasma waves cycling through vivid RGB spectra.
#[derive(Debug, Default)]
pub struct PlasmaEffect {
    time: f32,
}

impl PlasmaEffect {
    /// Creates a new [`PlasmaEffect`].
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl Effect for PlasmaEffect {
    fn metadata(&self) -> EffectMetadata {
        EffectMetadata {
            id: "plasma",
            name: "Plasma",
            description: "Blobs of multi-octave plasma flowing and stretching across your setup.",
            publisher: "FastRGB",
            category: EffectCategory::Basic,
            uses_screen: false,
        }
    }

    fn descriptors(&self) -> Vec<ParameterDescriptor> {
        vec![
            ParameterDescriptor::number("speed", "Flow Speed", 1.0, 10.0, 1.0, 4.0),
            ParameterDescriptor::number("scale", "Plasma Scale", 1.0, 10.0, 1.0, 5.0),
            ParameterDescriptor::number("brightness", "Brightness", 1.0, 10.0, 1.0, 8.0),
        ]
    }

    fn reset(&mut self) {
        self.time = 0.0;
    }

    fn render(&mut self, ctx: &EffectContext<'_>, canvas: &mut CanvasFrame) {
        let speed = ctx.params.f32_or("speed", 4.0);
        let scale = ctx.params.f32_or("scale", 5.0);
        let brightness = (ctx.params.f32_or("brightness", 8.0) / 10.0).clamp(0.05, 1.0);
        let dt_scale = (ctx.dt_seconds * 30.0).max(0.05);

        self.time += speed * 0.03 * dt_scale;
        let s = scale * 0.015;
        let t = self.time;
        let w = canvas.width() as usize;
        let h = canvas.height() as usize;
        let pixels = canvas.pixels_mut();

        for y in (0..h).step_by(2) {
            let yf = y as f32;
            let v2 = (yf * s + t * 1.3).sin();
            let cy = yf * s + (t * 0.5).cos();
            for x in (0..w).step_by(2) {
                let xf = x as f32;
                let v1 = (xf * s + t).sin();
                let v3 = ((xf + yf) * s + t).sin();
                let cx = xf * s + (t / 3.0).sin();
                let v4 = ((cx * cx + cy * cy + 1.0).sqrt() + t * 1.5).sin();
                let val = (v1 + v2 + v3 + v4) * 0.25;

                let phase = val * PI;
                let r = ((phase.sin() * 127.0 + 128.0) * brightness).clamp(0.0, 255.0) as u8;
                let g =
                    (((phase + 2.094).sin() * 127.0 + 128.0) * brightness).clamp(0.0, 255.0) as u8;
                let b =
                    (((phase + 4.188).sin() * 127.0 + 128.0) * brightness).clamp(0.0, 255.0) as u8;
                let px = Rgba::new(r, g, b, 255);

                pixels[y * w + x] = px;
                if x + 1 < w {
                    pixels[y * w + x + 1] = px;
                }
                if y + 1 < h {
                    pixels[(y + 1) * w + x] = px;
                    if x + 1 < w {
                        pixels[(y + 1) * w + x + 1] = px;
                    }
                }
            }
        }
    }
}
