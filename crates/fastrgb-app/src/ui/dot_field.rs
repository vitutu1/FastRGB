//! High-framerate native GPUI `DotField` backdrop inspired by React Bits.
//!
//! Renders an interactive dormant LED matrix in a single GPU-batched `canvas()` pass with cursor
//! bulge, click ripples, subtle ambient waves, and live tinting from the active RGB effect.

use fastrgb_core::Rgb;
use gpui::{
    Bounds, Corners, PaintQuad, Pixels, Point, Window, canvas, div, linear_color_stop,
    linear_gradient, point, prelude::*, px, size,
};
use smallvec::SmallVec;

use crate::ui::theme;

const DOT_SPACING: f32 = 28.0;
const CURSOR_RADIUS: f32 = 148.0;
const MAX_RIPPLES: usize = 5;

/// Expanding circular light ripple spawned on pointer click.
#[derive(Debug, Clone, Copy)]
pub struct DotRipple {
    /// Origin X in window pixels.
    pub x: f32,
    /// Origin Y in window pixels.
    pub y: f32,
    /// Current wave radius in pixels.
    pub radius: f32,
    /// Current normalized opacity (`0.0..=1.0`).
    pub alpha: f32,
}

/// Lightweight per-frame state for the interactive `DotField` backdrop.
#[derive(Debug, Clone)]
pub struct DotFieldState {
    /// Target pointer position.
    pub pointer: Option<Point<Pixels>>,
    /// Spring-smoothed pointer X.
    pub smooth_x: f32,
    /// Spring-smoothed pointer Y.
    pub smooth_y: f32,
    /// Active expanding click ripples.
    pub ripples: SmallVec<[DotRipple; MAX_RIPPLES]>,
    /// Elapsed animation clock in seconds.
    pub clock_sec: f32,
}

impl Default for DotFieldState {
    fn default() -> Self {
        Self {
            pointer: None,
            smooth_x: -9999.0,
            smooth_y: -9999.0,
            ripples: SmallVec::new(),
            clock_sec: 0.0,
        }
    }
}

impl DotFieldState {
    /// Updates cursor target coordinates from a window mouse-move event.
    pub fn set_pointer(&mut self, pos: Point<Pixels>) {
        let px_x: f32 = pos.x.into();
        let px_y: f32 = pos.y.into();
        if self.pointer.is_none() {
            self.smooth_x = px_x;
            self.smooth_y = px_y;
        }
        self.pointer = Some(pos);
    }

    /// Spawns a light ripple radiating outward from `pos`.
    pub fn spawn_ripple(&mut self, pos: Point<Pixels>) {
        if self.ripples.len() >= MAX_RIPPLES {
            self.ripples.remove(0);
        }
        self.ripples.push(DotRipple {
            x: pos.x.into(),
            y: pos.y.into(),
            radius: 0.0,
            alpha: 0.75,
        });
    }

    /// Advances smooth cursor inertia, wave clock, and active ripples by `dt` seconds.
    pub fn step(&mut self, dt: f32) {
        let dt_clamped = dt.clamp(0.0, 0.05);
        self.clock_sec = (self.clock_sec + dt_clamped) % 1000.0;

        if let Some(pos) = self.pointer {
            let tx: f32 = pos.x.into();
            let ty: f32 = pos.y.into();
            let k = 1.0 - (-18.0 * dt_clamped).exp();
            self.smooth_x += (tx - self.smooth_x) * k;
            self.smooth_y += (ty - self.smooth_y) * k;
        }

        let step_px = 520.0 * dt_clamped;
        let decay = 0.06_f32.powf(dt_clamped * 1.4);
        self.ripples.retain_mut(|r| {
            r.radius += step_px;
            r.alpha *= decay;
            r.radius < 340.0 && r.alpha > 0.02
        });
    }

    /// Returns `true` while ripples or cursor lag require additional animation frames.
    #[must_use]
    pub fn has_active_motion(&self) -> bool {
        if !self.ripples.is_empty() {
            return true;
        }
        if let Some(pos) = self.pointer {
            let tx: f32 = pos.x.into();
            let ty: f32 = pos.y.into();
            return (tx - self.smooth_x).abs() > 0.75 || (ty - self.smooth_y).abs() > 0.75;
        }
        false
    }
}

/// Renders the full-bleed interactive `DotField` and soft reactive ambient vignette.
pub fn render_dot_field_backdrop(state: &DotFieldState, ambient: Rgb) -> impl IntoElement {
    let pointer_active = state.pointer.is_some();
    let sx = state.smooth_x;
    let sy = state.smooth_y;
    let clock = state.clock_sec;
    let ripples = state.ripples.clone();
    let ambient_hsla = theme::from_core_rgb(ambient);

    div()
        .absolute()
        .inset_0()
        .size_full()
        .child(div().absolute().inset_0().size_full().bg(linear_gradient(
            180.0,
            linear_color_stop(ambient_hsla.opacity(0.085), 0.0),
            linear_color_stop(gpui::transparent_black(), 0.42),
        )))
        .child(
            canvas(
                |_bounds, _window, _cx| {},
                move |bounds: Bounds<Pixels>, (), window: &mut Window, _cx| {
                    let width: f32 = bounds.size.width.into();
                    let height: f32 = bounds.size.height.into();
                    let origin_x: f32 = bounds.origin.x.into();
                    let origin_y: f32 = bounds.origin.y.into();

                    let cols = (width / DOT_SPACING).ceil() as i32;
                    let rows = (height / DOT_SPACING).ceil() as i32;
                    let radius_sq = CURSOR_RADIUS * CURSOR_RADIUS;
                    let ar = f32::from(ambient.r);
                    let ag = f32::from(ambient.g);
                    let ab = f32::from(ambient.b);
                    let zero_edges = gpui::Edges::all(px(0.0));
                    let no_border = gpui::transparent_black();

                    for row in 0..rows {
                        let by = origin_y + (row as f32 + 0.5) * DOT_SPACING;
                        for col in 0..cols {
                            let bx = origin_x + (col as f32 + 0.5) * DOT_SPACING;
                            let mut dx_out = 0.0_f32;
                            let mut dy_out = 0.0_f32;
                            let mut glow = 0.0_f32;

                            if pointer_active {
                                let dx = bx - sx;
                                let dy = by - sy;
                                let dist_sq = dx * dx + dy * dy;
                                if dist_sq < radius_sq && dist_sq > 0.01 {
                                    let dist = dist_sq.sqrt();
                                    let force = 1.0 - dist / CURSOR_RADIUS;
                                    let smooth = force * force;
                                    let push = smooth * 11.5;
                                    let inv = 1.0 / dist;
                                    dx_out += dx * inv * push;
                                    dy_out += dy * inv * push;
                                    glow = glow.max(smooth);
                                }
                            }

                            for rp in &ripples {
                                let dx = bx - rp.x;
                                let dy = by - rp.y;
                                let dist = (dx * dx + dy * dy).sqrt();
                                let band = (dist - rp.radius).abs();
                                if band < 36.0 {
                                    let k = (1.0 - band / 36.0) * rp.alpha;
                                    glow = glow.max(k * 0.85);
                                    if dist > 1.0 {
                                        let inv = 1.0 / dist;
                                        dx_out += dx * inv * k * 5.0;
                                        dy_out += dy * inv * k * 5.0;
                                    }
                                }
                            }

                            let hash = ((col as u32).wrapping_mul(73_856_093)
                                ^ (row as u32).wrapping_mul(19_349_663))
                                & 0xFF;
                            let phase = bx * 0.014 + by * 0.011 + (hash as f32) * 0.0024;
                            let wave = 0.5 + 0.5 * (clock * 1.1 + phase).sin();
                            let twinkle = if hash > 244 {
                                (0.5 + 0.5 * (clock * 3.2 + phase * 3.0).sin()) * 0.14
                            } else {
                                0.0
                            };

                            let alpha = (0.065 + wave * 0.038 + twinkle + glow * 0.62).min(0.84);
                            let diam = 2.1 + glow * 2.5;
                            let half = diam * 0.5;

                            let dot_color = if glow > 0.03 {
                                let mix = (glow * 1.15).min(1.0);
                                let cr = (182.0 + (ar - 182.0) * mix).round() as u8;
                                let cg = (188.0 + (ag - 188.0) * mix).round() as u8;
                                let cb = (202.0 + (ab - 202.0) * mix).round() as u8;
                                theme::from_core_rgb(Rgb::new(cr, cg, cb)).opacity(alpha)
                            } else {
                                theme::from_core_rgb(Rgb::new(210, 218, 235)).opacity(alpha)
                            };

                            window.paint_quad(PaintQuad {
                                bounds: Bounds {
                                    origin: point(px(bx + dx_out - half), px(by + dy_out - half)),
                                    size: size(px(diam), px(diam)),
                                },
                                corner_radii: Corners::all(px(half)),
                                background: dot_color.into(),
                                border_widths: zero_edges,
                                border_color: no_border,
                                border_style: gpui::BorderStyle::Solid,
                            });
                        }
                    }
                },
            )
            .size_full(),
        )
}
