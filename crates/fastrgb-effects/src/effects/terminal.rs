//! `Terminal` falling green phosphor code cascade.

use fastrgb_core::{CanvasFrame, ParameterDescriptor, Rgb, Rgba};

use crate::rng::FastRng;
use crate::traits::{Effect, EffectCategory, EffectContext, EffectMetadata};

const MAX_COLS: usize = 64;

/// Classic green phosphor terminal character drops falling down columns.
#[derive(Debug)]
pub struct TerminalEffect {
    drops: [f32; MAX_COLS],
    rng: FastRng,
    initialized: bool,
}

impl Default for TerminalEffect {
    fn default() -> Self {
        let mut rng = FastRng::seeded(0x7E2A_1AA1);
        let mut drops = [0.0f32; MAX_COLS];
        for d in &mut drops {
            *d = rng.range_f32(0.0, 26.0);
        }
        Self {
            drops,
            rng,
            initialized: false,
        }
    }
}

impl TerminalEffect {
    /// Creates a new [`TerminalEffect`].
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl Effect for TerminalEffect {
    fn metadata(&self) -> EffectMetadata {
        EffectMetadata {
            id: "terminal",
            name: "Terminal",
            description: "Crisp neon-green digital character streams cascading across columns.",
            publisher: "FastRGB",
            category: EffectCategory::Basic,
            uses_screen: false,
        }
    }

    fn descriptors(&self) -> Vec<ParameterDescriptor> {
        vec![
            ParameterDescriptor::color("color", "Glyph Color", Rgb::new(0x00, 0xFF, 0x22)),
            ParameterDescriptor::number("fontSize", "Cell Size", 5.0, 16.0, 1.0, 8.0),
            ParameterDescriptor::number("speed", "Cascade Speed", 1.0, 60.0, 1.0, 22.0),
        ]
    }

    fn reset(&mut self) {
        *self = Self::default();
    }

    fn render(&mut self, ctx: &EffectContext<'_>, canvas: &mut CanvasFrame) {
        let color = ctx.params.color_or("color", Rgb::new(0x00, 0xFF, 0x22));
        let cell = ctx.params.f32_or("fontSize", 8.0).round().clamp(5.0, 16.0) as i32;
        let speed = ctx.params.f32_or("speed", 22.0);

        if !self.initialized {
            canvas.fill(Rgba::BLACK);
            self.initialized = true;
        } else {
            canvas.blend_all(Rgba::new(0, 0, 0, 38));
        }

        let cols = ((canvas.width() as i32) / cell).clamp(1, MAX_COLS as i32) as usize;
        let rows = ((canvas.height() as i32) / cell).max(1) as f32;
        let adv = ctx.dt_seconds * speed;

        for c in 0..cols {
            let prev_row = self.drops[c].floor() as i32;
            self.drops[c] += adv;
            let row = self.drops[c].floor() as i32;

            if row != prev_row || self.rng.next_f32() < 0.35 {
                let x = c as i32 * cell;
                let y = prev_row * cell;
                let glyph_w = (cell - 2).max(2);
                let glyph_h = (cell - 1).max(3);
                // Bright head + subtle glyph cutout to suggest digital glyphs at RGB canvas scale.
                canvas.fill_rect(x + 1, y, glyph_w, glyph_h, color.to_rgba(255));
                if self.rng.next_f32() < 0.5 && glyph_w >= 4 && glyph_h >= 4 {
                    canvas.fill_rect(x + 2, y + 1, glyph_w - 2, 1, Rgba::BLACK);
                }
            }

            if self.drops[c] >= rows && self.rng.next_f32() > 0.92 {
                self.drops[c] = 0.0;
            }
        }
    }
}
