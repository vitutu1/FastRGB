//! `DNA` rotating double helix effect.

use std::f32::consts::{PI, TAU};

use fastrgb_core::{CanvasFrame, ParameterDescriptor, Rgb};

use super::helpers::hsl_rgb;
use crate::traits::{Effect, EffectCategory, EffectContext, EffectMetadata};

const HELIX_NODES: usize = 26;

/// Oscillating DNA double helix with customizable strand linkage, rainbow, and spread.
#[derive(Debug)]
pub struct DnaEffect {
    phase: f32,
    cycle_hue: f32,
}

impl Default for DnaEffect {
    fn default() -> Self {
        Self {
            phase: 0.0,
            cycle_hue: 0.0,
        }
    }
}

impl DnaEffect {
    /// Creates a new [`DnaEffect`].
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl Effect for DnaEffect {
    fn metadata(&self) -> EffectMetadata {
        EffectMetadata {
            id: "dna",
            name: "DNA",
            description: "Rotating 3D double helix with configurable rungs, outer backbone, and color modes.",
            publisher: "FastRGB",
            category: EffectCategory::Basic,
            uses_screen: false,
        }
    }

    fn descriptors(&self) -> Vec<ParameterDescriptor> {
        vec![
            ParameterDescriptor::color("helixColor", "Helix Color", Rgb::new(0x00, 0xFF, 0x00)),
            ParameterDescriptor::color(
                "backgroundColor",
                "Background Color",
                Rgb::new(0x00, 0x20, 0x00),
            ),
            ParameterDescriptor::boolean("rainbow", "Rainbow Mode", false),
            ParameterDescriptor::boolean("colorCycleEnabled", "Color Cycle", false),
            ParameterDescriptor::number(
                "colorCycleSpeed",
                "Color Cycle Speed",
                1.0,
                100.0,
                1.0,
                50.0,
            ),
            ParameterDescriptor::number("speed", "Rotation Speed", 0.0, 100.0, 1.0, 50.0),
            ParameterDescriptor::boolean("direction", "Reverse Direction", false),
            ParameterDescriptor::number(
                "horizontalSpread",
                "Horizontal Spread",
                0.5,
                4.0,
                0.25,
                1.0,
            ),
            ParameterDescriptor::number(
                "verticalSpread",
                "Vertical Spread",
                10.0,
                100.0,
                1.0,
                50.0,
            ),
            ParameterDescriptor::boolean("outerLinks", "Outer Backbone", true),
            ParameterDescriptor::boolean("innerLinks", "Inner Base Rungs", true),
            ParameterDescriptor::number("linkSize", "Link Thickness", 1.0, 8.0, 0.5, 3.0),
        ]
    }

    fn reset(&mut self) {
        *self = Self::default();
    }

    fn render(&mut self, ctx: &EffectContext<'_>, canvas: &mut CanvasFrame) {
        let base = ctx
            .params
            .color_or("helixColor", Rgb::new(0x00, 0xFF, 0x00));
        let bg = ctx
            .params
            .color_or("backgroundColor", Rgb::new(0x00, 0x20, 0x00));
        let rainbow = ctx.params.bool_or("rainbow", false);
        let cycle_on = ctx.params.bool_or("colorCycleEnabled", false);
        let cycle_speed = ctx.params.f32_or("colorCycleSpeed", 50.0);
        let speed = ctx.params.f32_or("speed", 50.0);
        let reverse = ctx.params.bool_or("direction", false);
        let h_spread = ctx.params.f32_or("horizontalSpread", 1.0);
        let v_spread = ctx.params.f32_or("verticalSpread", 50.0) / 50.0;
        let outer = ctx.params.bool_or("outerLinks", true);
        let inner = ctx.params.bool_or("innerLinks", true);
        let thickness = ctx.params.f32_or("linkSize", 3.0);

        let dir_sign = if reverse { -1.0 } else { 1.0 };
        self.phase = (self.phase + dir_sign * ctx.dt_seconds * (speed / 12.0)).rem_euclid(TAU);
        if cycle_on {
            self.cycle_hue =
                (self.cycle_hue + ctx.dt_seconds * (cycle_speed * 1.4)).rem_euclid(360.0);
        }

        canvas.fill(bg.to_rgba(255));
        let w = canvas.width() as f32;
        let cy = canvas.height() as f32 * 0.5;
        let base_hsl = base.to_hsl();

        let mut strand_a = [(0.0f32, 0.0f32, 0.0f32); HELIX_NODES];
        let mut strand_b = [(0.0f32, 0.0f32, 0.0f32); HELIX_NODES];

        for i in 0..HELIX_NODES {
            let frac = i as f32 / (HELIX_NODES - 1) as f32;
            let x = frac * w;
            let theta = self.phase + frac * TAU * h_spread;
            let amp = 56.0 * v_spread;
            let ya = cy + theta.sin() * amp;
            let yb = cy + (theta + PI).sin() * amp;
            let za = theta.cos();
            let zb = (theta + PI).cos();
            strand_a[i] = (x, ya, za);
            strand_b[i] = (x, yb, zb);
        }

        let node_color = |i: usize| -> Rgb {
            if rainbow {
                hsl_rgb(
                    i as f32 * (360.0 / HELIX_NODES as f32) + self.cycle_hue,
                    100.0,
                    50.0,
                )
            } else if cycle_on {
                hsl_rgb(self.cycle_hue, 100.0, 50.0)
            } else {
                hsl_rgb(base_hsl.h, base_hsl.s, base_hsl.l)
            }
        };

        if outer {
            for i in 0..(HELIX_NODES - 1) {
                let col = node_color(i).to_rgba(230);
                canvas.draw_line(
                    strand_a[i].0,
                    strand_a[i].1,
                    strand_a[i + 1].0,
                    strand_a[i + 1].1,
                    thickness,
                    col,
                );
                canvas.draw_line(
                    strand_b[i].0,
                    strand_b[i].1,
                    strand_b[i + 1].0,
                    strand_b[i + 1].1,
                    thickness,
                    col,
                );
            }
        }

        for i in 0..HELIX_NODES {
            let col = node_color(i);
            if inner && i % 2 == 0 {
                canvas.draw_line(
                    strand_a[i].0,
                    strand_a[i].1,
                    strand_b[i].0,
                    strand_b[i].1,
                    (thickness * 0.75).max(1.0),
                    col.to_rgba(185),
                );
            }
            let ra = 5.5 + strand_a[i].2 * 2.5;
            let rb = 5.5 + strand_b[i].2 * 2.5;
            canvas.fill_circle(strand_a[i].0, strand_a[i].1, ra, col.to_rgba(255));
            canvas.fill_circle(strand_b[i].0, strand_b[i].1, rb, col.to_rgba(255));
        }
    }
}
