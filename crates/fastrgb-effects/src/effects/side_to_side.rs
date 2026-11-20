//! `Side to Side` sweeping wipe effect.

use fastrgb_core::{CanvasFrame, Hsl, ParameterDescriptor, Rgb};

use crate::traits::{Effect, EffectCategory, EffectContext, EffectMetadata};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SweepAxis {
    Horizontal { reverse: bool },
    Vertical { reverse: bool },
}

#[derive(Debug, Clone, Copy)]
struct ActiveSweep {
    axis: SweepAxis,
    pos: f32,
    hue: f32,
}

/// Rhythmic two-color (or rainbow) wipe sweeping horizontally, vertically, or in all 4 directions.
#[derive(Debug)]
pub struct SideToSideEffect {
    first_frame: bool,
    direction_step: usize,
    color_index: usize,
    rainbow_seed: f32,
    active: Option<ActiveSweep>,
}

impl Default for SideToSideEffect {
    fn default() -> Self {
        Self {
            first_frame: true,
            direction_step: 0,
            color_index: 0,
            rainbow_seed: 35.0,
            active: None,
        }
    }
}

impl SideToSideEffect {
    /// Creates a new [`SideToSideEffect`].
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    fn spawn_next_sweep(&mut self, direction: &str, canvas: &CanvasFrame) {
        self.color_index = (self.color_index + 1) % 2;
        self.rainbow_seed = (self.rainbow_seed + 137.508).rem_euclid(360.0);

        let axis = match direction {
            "Side to Side" => {
                self.direction_step = (self.direction_step + 1) % 2;
                SweepAxis::Horizontal {
                    reverse: self.direction_step == 1,
                }
            }
            "Up and Down" => {
                self.direction_step = (self.direction_step + 1) % 2;
                SweepAxis::Vertical {
                    reverse: self.direction_step == 1,
                }
            }
            _ => {
                self.direction_step = (self.direction_step + 1) % 4;
                match self.direction_step {
                    0 => SweepAxis::Horizontal { reverse: true },
                    1 => SweepAxis::Vertical { reverse: true },
                    2 => SweepAxis::Horizontal { reverse: false },
                    _ => SweepAxis::Vertical { reverse: false },
                }
            }
        };

        let pos = match axis {
            SweepAxis::Horizontal { .. } => canvas.width() as f32,
            SweepAxis::Vertical { .. } => canvas.height() as f32,
        };

        self.active = Some(ActiveSweep {
            axis,
            pos,
            hue: self.rainbow_seed,
        });
    }
}

impl Effect for SideToSideEffect {
    fn metadata(&self) -> EffectMetadata {
        EffectMetadata {
            id: "side_to_side",
            name: "Side to Side",
            description: "Smooth, rhythmic color sweeps shifting across the setup.",
            publisher: "FastRGB",
            category: EffectCategory::Static,
            uses_screen: false,
        }
    }

    fn descriptors(&self) -> Vec<ParameterDescriptor> {
        vec![
            ParameterDescriptor::combobox(
                "direction",
                "Direction",
                vec![
                    "Side to Side".to_owned(),
                    "Up and Down".to_owned(),
                    "All Directions".to_owned(),
                ],
                "All Directions",
            )
            .with_tooltip("Changes the movement of the colors."),
            ParameterDescriptor::color("color1", "Color 1", Rgb::new(0x08, 0x2E, 0xFF)),
            ParameterDescriptor::color("color2", "Color 2", Rgb::new(0xFF, 0x03, 0xAF)),
            ParameterDescriptor::boolean("rainbow", "Rainbow Mode", false)
                .with_tooltip("Cycles through vivid rainbow hues instead of Color 1 / Color 2."),
            ParameterDescriptor::number("speed", "Speed", 0.0, 100.0, 1.0, 15.0),
        ]
    }

    fn reset(&mut self) {
        *self = Self::default();
    }

    fn render(&mut self, ctx: &EffectContext<'_>, canvas: &mut CanvasFrame) {
        let direction = ctx.params.str_or("direction", "All Directions");
        let color1 = ctx.params.color_or("color1", Rgb::new(0x08, 0x2E, 0xFF));
        let color2 = ctx.params.color_or("color2", Rgb::new(0xFF, 0x03, 0xAF));
        let rainbow = ctx.params.bool_or("rainbow", false);
        let speed = ctx.params.f32_or("speed", 15.0);

        if self.first_frame {
            canvas.fill(color1.to_rgba(255));
            self.first_frame = false;
        }

        if self.active.is_none() {
            self.spawn_next_sweep(direction, canvas);
        }

        let Some(mut sweep) = self.active else {
            return;
        };

        let base_color = if self.color_index == 0 {
            color1
        } else {
            color2
        };
        let fill_color = if rainbow {
            let base_hsl = base_color.to_hsl();
            Hsl::new(
                sweep.hue,
                base_hsl.s.max(85.0),
                base_hsl.l.clamp(35.0, 65.0),
            )
            .to_rgb()
            .to_rgba(255)
        } else {
            base_color.to_rgba(255)
        };

        let w = canvas.width() as i32;
        let h = canvas.height() as i32;
        let pos_i = sweep.pos.round() as i32;

        match sweep.axis {
            SweepAxis::Horizontal { reverse } => {
                if reverse {
                    let rect_w = (w - pos_i).clamp(0, w);
                    canvas.fill_rect(0, 0, rect_w, h, fill_color);
                } else {
                    canvas.fill_rect(pos_i, 0, w - pos_i, h, fill_color);
                }
            }
            SweepAxis::Vertical { reverse } => {
                if reverse {
                    let rect_h = (h - pos_i).clamp(0, h);
                    canvas.fill_rect(0, 0, w, rect_h, fill_color);
                } else {
                    canvas.fill_rect(0, pos_i, w, h - pos_i, fill_color);
                }
            }
        }

        sweep.pos -= (speed / 10.0) * (ctx.dt_seconds * 60.0);
        if sweep.pos <= 0.0 {
            self.active = None;
        } else {
            self.active = Some(sweep);
        }
    }
}
