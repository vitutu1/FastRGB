//! Shared 2D drawing primitives and procedural engines for built-in effects.

use fastrgb_core::{CanvasFrame, Hsl, Rgb, Rgba};

use crate::rng::FastRng;

/// Linearly interpolates between two `Rgb` colors with clamped factor `t` in `[0.0, 1.0]`.
#[must_use]
pub fn lerp_rgb(a: Rgb, b: Rgb, t: f32) -> Rgb {
    let t = t.clamp(0.0, 1.0);
    let r = (a.r as f32 + (b.r as f32 - a.r as f32) * t).round();
    let g = (a.g as f32 + (b.g as f32 - a.g as f32) * t).round();
    let bl = (a.b as f32 + (b.b as f32 - a.b as f32) * t).round();
    Rgb::new(r as u8, g as u8, bl as u8)
}

/// Linearly interpolates between two `Rgba` colors with clamped factor `t` in `[0.0, 1.0]`.
#[must_use]
pub fn lerp_rgba(a: Rgba, b: Rgba, t: f32) -> Rgba {
    let t = t.clamp(0.0, 1.0);
    let r = (a.r as f32 + (b.r as f32 - a.r as f32) * t).round();
    let g = (a.g as f32 + (b.g as f32 - a.g as f32) * t).round();
    let bl = (a.b as f32 + (b.b as f32 - a.b as f32) * t).round();
    let al = (a.a as f32 + (b.a as f32 - a.a as f32) * t).round();
    Rgba::new(r as u8, g as u8, bl as u8, al as u8)
}

/// Samples a multi-stop RGBA gradient at `t` in `[0.0, 1.0]`.
#[must_use]
pub fn sample_rgba_stops(stops: &[(f32, Rgba)], t: f32) -> Rgba {
    if stops.is_empty() {
        return Rgba::BLACK;
    }
    let t = t.clamp(0.0, 1.0);
    if t <= stops[0].0 {
        return stops[0].1;
    }
    for win in stops.windows(2) {
        let (p0, c0) = win[0];
        let (p1, c1) = win[1];
        if t <= p1 {
            let span = (p1 - p0).max(1e-5);
            return lerp_rgba(c0, c1, (t - p0) / span);
        }
    }
    stops[stops.len() - 1].1
}

/// Samples a multi-stop RGB gradient at `t` in `[0.0, 1.0]`.
#[must_use]
pub fn sample_rgb_stops(stops: &[(f32, Rgb)], t: f32) -> Rgb {
    if stops.is_empty() {
        return Rgb::BLACK;
    }
    let t = t.clamp(0.0, 1.0);
    if t <= stops[0].0 {
        return stops[0].1;
    }
    for win in stops.windows(2) {
        let (p0, c0) = win[0];
        let (p1, c1) = win[1];
        if t <= p1 {
            let span = (p1 - p0).max(1e-5);
            return lerp_rgb(c0, c1, (t - p0) / span);
        }
    }
    stops[stops.len() - 1].1
}

/// Fills (or alpha-blends) a 1D linear gradient across the full canvas.
pub fn fill_linear_gradient(
    canvas: &mut CanvasFrame,
    horizontal: bool,
    repeat: bool,
    phase: f32,
    stops: &[(f32, Rgb)],
    alpha: u8,
) {
    if alpha == 0 || stops.is_empty() {
        return;
    }
    let w = canvas.width();
    let h = canvas.height();
    let wf = (w.saturating_sub(1)).max(1) as f32;
    let hf = (h.saturating_sub(1)).max(1) as f32;

    if horizontal {
        for x in 0..w {
            let raw = x as f32 / wf + phase;
            let t = if repeat { raw.rem_euclid(1.0) } else { raw };
            let c = sample_rgb_stops(stops, t).to_rgba(alpha);
            for y in 0..h {
                canvas.blend_pixel(x as i32, y as i32, c);
            }
        }
    } else {
        for y in 0..h {
            let raw = y as f32 / hf + phase;
            let t = if repeat { raw.rem_euclid(1.0) } else { raw };
            let c = sample_rgb_stops(stops, t).to_rgba(alpha);
            for x in 0..w {
                canvas.blend_pixel(x as i32, y as i32, c);
            }
        }
    }
}

/// Draws a hollow stroked circle centered at `(cx, cy)`.
pub fn stroke_circle(
    canvas: &mut CanvasFrame,
    cx: f32,
    cy: f32,
    radius: f32,
    line_width: f32,
    color: Rgba,
) {
    if radius <= 0.0 || line_width <= 0.0 || color.a == 0 {
        return;
    }
    let half_w = (line_width * 0.5).max(0.5);
    let r_inner = (radius - half_w).max(0.0);
    let r_outer = radius + half_w;
    let r_in2 = r_inner * r_inner;
    let r_out2 = r_outer * r_outer;

    let x0 = ((cx - r_outer).floor() as i32).max(0);
    let y0 = ((cy - r_outer).floor() as i32).max(0);
    let x1 = ((cx + r_outer).ceil() as i32).min(canvas.width() as i32 - 1);
    let y1 = ((cy + r_outer).ceil() as i32).min(canvas.height() as i32 - 1);

    for y in y0..=y1 {
        let dy = (y as f32 + 0.5) - cy;
        for x in x0..=x1 {
            let dx = (x as f32 + 0.5) - cx;
            let d2 = dx * dx + dy * dy;
            if d2 >= r_in2 && d2 <= r_out2 {
                canvas.blend_pixel(x, y, color);
            }
        }
    }
}

/// Fills a rotated rectangle centered at `(cx, cy)` with dimensions `(width, height)` rotated by `angle_rad`.
pub fn fill_rotated_rect(
    canvas: &mut CanvasFrame,
    cx: f32,
    cy: f32,
    width: f32,
    height: f32,
    angle_rad: f32,
    color: Rgba,
) {
    if width <= 0.0 || height <= 0.0 || color.a == 0 {
        return;
    }
    fill_rotated_rect_internal(canvas, cx, cy, width, height, angle_rad, |_| color);
}

/// Fills a rotated rectangle whose color varies smoothly along its local X axis via `stops`.
pub fn fill_rotated_rect_gradient(
    canvas: &mut CanvasFrame,
    cx: f32,
    cy: f32,
    width: f32,
    height: f32,
    angle_rad: f32,
    stops: &[(f32, Rgba)],
) {
    if width <= 0.0 || height <= 0.0 || stops.is_empty() {
        return;
    }
    fill_rotated_rect_internal(canvas, cx, cy, width, height, angle_rad, |t| {
        sample_rgba_stops(stops, t)
    });
}

fn fill_rotated_rect_internal<F>(
    canvas: &mut CanvasFrame,
    cx: f32,
    cy: f32,
    width: f32,
    height: f32,
    angle_rad: f32,
    color_at_u: F,
) where
    F: Fn(f32) -> Rgba,
{
    let hw = width * 0.5;
    let hh = height * 0.5;
    let cos_a = angle_rad.cos();
    let sin_a = angle_rad.sin();
    let extent_x = hw * cos_a.abs() + hh * sin_a.abs();
    let extent_y = hw * sin_a.abs() + hh * cos_a.abs();

    let x0 = ((cx - extent_x).floor() as i32).max(0);
    let y0 = ((cy - extent_y).floor() as i32).max(0);
    let x1 = ((cx + extent_x).ceil() as i32).min(canvas.width() as i32 - 1);
    let y1 = ((cy + extent_y).ceil() as i32).min(canvas.height() as i32 - 1);

    for y in y0..=y1 {
        let dy = (y as f32 + 0.5) - cy;
        for x in x0..=x1 {
            let dx = (x as f32 + 0.5) - cx;
            let lx = dx * cos_a + dy * sin_a;
            let ly = -dx * sin_a + dy * cos_a;
            if lx.abs() <= hw && ly.abs() <= hh {
                let u = (lx + hw) / width.max(1e-5);
                let col = color_at_u(u);
                if col.a > 0 {
                    canvas.blend_pixel(x, y, col);
                }
            }
        }
    }
}

/// Grid dimensions used by [`CellularFireGrid`] (matches FastRGB's `320x200` / `4` scaling).
pub const FIRE_COLS: usize = 320;
/// Vertical cell count (`200 / 4 = 50`).
pub const FIRE_ROWS: usize = 50;
/// Pixel height per cellular fire row.
pub const FIRE_CELL_H: i32 = 4;

/// Shared cellular upward fire simulation engine used by `Fire` and `Neon Fire`.
#[derive(Debug, Clone)]
pub struct CellularFireGrid {
    dots: Vec<u8>,
    accumulator: f32,
    rng: FastRng,
}

impl CellularFireGrid {
    /// Initializes a cold cellular fire grid with a full-heat bottom ignition row (`index = 35`).
    #[must_use]
    pub fn new(seed: u64) -> Self {
        let mut dots = vec![0u8; FIRE_COLS * FIRE_ROWS];
        let bottom = (FIRE_ROWS - 1) * FIRE_COLS;
        dots[bottom..bottom + FIRE_COLS].fill(35);
        Self {
            dots,
            accumulator: 0.0,
            rng: FastRng::seeded(seed),
        }
    }

    /// Steps the cellular propagation at `steps_per_sec` and renders using `palette`.
    pub fn step_and_render(
        &mut self,
        canvas: &mut CanvasFrame,
        dt: f32,
        steps_per_sec: f32,
        fl_height: i32,
        palette: &[Rgb; 36],
        bg: Rgb,
    ) {
        self.accumulator += dt * steps_per_sec;
        let ticks = (self.accumulator.floor() as usize).min(6);
        self.accumulator -= ticks as f32;

        for _ in 0..ticks {
            self.step_once(fl_height.max(2) as usize);
        }

        canvas.fill(bg.to_rgba(255));
        for y in 0..FIRE_ROWS {
            let row_off = y * FIRE_COLS;
            for x in 0..FIRE_COLS {
                let idx = self.dots[row_off + x] as usize;
                if idx > 0 {
                    let col = palette[idx.min(35)].to_rgba(255);
                    canvas.fill_rect(x as i32, y as i32 * FIRE_CELL_H, 1, FIRE_CELL_H, col);
                }
            }
        }
    }

    fn step_once(&mut self, decay_mod: usize) {
        for x in 0..FIRE_COLS {
            for y in 1..FIRE_ROWS {
                let src_idx = y * FIRE_COLS + x;
                let heat = self.dots[src_idx];
                if heat == 0 {
                    self.dots[(y - 1) * FIRE_COLS + x] = 0;
                    continue;
                }
                let rand_decay = self.rng.gen_range_usize(decay_mod) as isize;
                let dst_x =
                    (x as isize + (1 - (rand_decay % 3))).rem_euclid(FIRE_COLS as isize) as usize;
                let dst_y = y - 1;
                let cooling = (rand_decay & 1) as u8;
                self.dots[dst_y * FIRE_COLS + dst_x] = heat.saturating_sub(cooling);
            }
        }
    }
}

/// Visual primitive drawn by [`DriftParticle`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriftShape {
    /// Filled translucent circle.
    FilledCircle,
    /// Hollow stroked ring.
    Ring {
        /// Stroke width in pixels.
        line_width: u8,
    },
}

/// Shared floating orb/ring particle used by ambient atmospheric effects.
#[derive(Debug, Clone)]
pub struct DriftParticle {
    /// Horizontal center coordinate in canvas pixels.
    pub x: f32,
    /// Vertical center coordinate in canvas pixels.
    pub y: f32,
    /// Current circle or ring radius in pixels.
    pub radius: f32,
    /// Horizontal velocity per 60 FPS frame.
    pub speed_x: f32,
    /// Vertical velocity per 60 FPS frame.
    pub speed_y: f32,
    /// Particle fill or stroke color with alpha.
    pub color: Rgba,
    /// Visual primitive shape to draw.
    pub shape: DriftShape,
    /// Lower radius bound when breathing or resetting.
    pub min_r: f32,
    /// Upper radius bound triggering respawn.
    pub max_r: f32,
    /// Radial growth per 60 FPS frame (`0.0` for constant radius).
    pub r_grow: f32,
}

impl DriftParticle {
    /// Advances position and radius with horizontal/vertical wrap-around and renders onto `canvas`.
    pub fn step_and_draw(
        &mut self,
        canvas: &mut CanvasFrame,
        rng: &mut FastRng,
        speed_scale: f32,
        dt: f32,
    ) {
        let frame_scale = (dt * 60.0).min(3.0);
        self.x += self.speed_x * speed_scale * frame_scale;
        self.y += self.speed_y * speed_scale * frame_scale;

        if self.r_grow != 0.0 {
            self.radius += self.r_grow * frame_scale;
            if self.radius >= self.max_r || self.radius <= self.min_r {
                self.radius = self.min_r;
                self.x = rng.range_f32(0.0, canvas.width() as f32);
                self.y = rng.range_f32(0.0, canvas.height() as f32);
            }
        }

        let w = canvas.width() as f32;
        let h = canvas.height() as f32;
        if self.x > w + self.radius {
            self.x = -self.radius;
        } else if self.x < -self.radius {
            self.x = w + self.radius;
        }
        if self.y > h + self.radius {
            self.y = -self.radius;
        } else if self.y < -self.radius {
            self.y = h + self.radius;
        }

        match self.shape {
            DriftShape::FilledCircle => canvas.fill_circle(self.x, self.y, self.radius, self.color),
            DriftShape::Ring { line_width } => {
                stroke_circle(
                    canvas,
                    self.x,
                    self.y,
                    self.radius,
                    f32::from(line_width),
                    self.color,
                );
            }
        }
    }
}

/// Converts HSL in standard degrees `[0, 360)` and `[0.0, 100.0]` S/L into `Rgb`.
#[must_use]
pub fn hsl_rgb(hue: f32, sat: f32, lit: f32) -> Rgb {
    Hsl::new(
        hue.rem_euclid(360.0),
        sat.clamp(0.0, 100.0),
        lit.clamp(0.0, 100.0),
    )
    .to_rgb()
}
