//! Crisp HiDPI vector icons rendered natively through GPUI `PathBuilder`.

use gpui::{
    Bounds, Hsla, PathBuilder, Pixels, Point, Window, canvas, div, point, prelude::*, px, rgb, rgba,
};

/// Built-in minimalist vector icon glyphs used across the studio shell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IconKind {
    /// 2×2 grid icon for `Library`.
    Library,
    /// Radiant light icon for `Stage` (`Lighting`).
    Stage,
    /// Hardware microchip icon for `Devices`.
    Devices,
    /// Camera glyph for native PNG snapshot.
    Camera,
    /// Previous effect transport icon.
    Prev,
    /// Play triangle icon.
    Play,
    /// Pause twin-bar icon.
    Pause,
    /// Next effect transport icon.
    Next,
    /// Reset loop arrow icon.
    Reset,
    /// Sun brightness glyph.
    Sun,
    /// Vertical sliders glyph for `Tune`.
    Sliders,
    /// Right inspector panel toggle icon.
    PanelRight,
    /// Hardware power bolt icon.
    Bolt,
    /// Downward chevron for collapsible diagnostics.
    ChevronDown,
    /// Upward chevron for expanded diagnostics.
    ChevronUp,
    /// Minimal gear glyph for `Settings`.
    Settings,
    /// Custom titlebar minimize glyph (`−`).
    WinMinimize,
    /// Custom titlebar maximize glyph (`□`).
    WinMaximize,
    /// Custom titlebar restore glyph (overlapping squares).
    WinRestore,
    /// Custom titlebar close-to-tray glyph (`×`).
    WinClose,
}

/// Renders the official `FastRGB` kinetic slanted-prism monogram natively via HiDPI vector paths.
pub fn render_fastrgb_logo(size_px: f32) -> impl IntoElement {
    div()
        .size(px(size_px))
        .rounded(px((size_px * 0.25).round()))
        .bg(rgb(0x08090E))
        .border_1()
        .border_color(rgba(0xFFFFFF2E))
        .overflow_hidden()
        .flex_none()
        .child(
            canvas(
                |_bounds, _window, _cx| {},
                move |b: Bounds<Pixels>, (), window: &mut Window, _cx| {
                    let slope = 0.20_f32;
                    let y_top = 0.22_f32;
                    let x_top = 0.31_f32;
                    let xl = |y: f32| x_top - slope * (y - y_top);

                    let draw_beam = |window: &mut Window,
                                     y0: f32,
                                     h: f32,
                                     x_off: f32,
                                     len: f32,
                                     color: Hsla| {
                        let y1 = y0 + h;
                        let mut pb = PathBuilder::fill();
                        pb.move_to(pt(b, xl(y0) + x_off, y0));
                        pb.line_to(pt(b, xl(y0) + x_off + len, y0));
                        pb.line_to(pt(b, xl(y1) + x_off + len, y1));
                        pb.line_to(pt(b, xl(y1) + x_off, y1));
                        pb.close();
                        if let Ok(path) = pb.build() {
                            window.paint_path(path, color);
                        }
                    };

                    let stem_w = 0.145_f32;
                    // Top beam (Cyan -> Electric Blue)
                    draw_beam(window, 0.22, 0.14, 0.0, 0.50, rgb(0x38BDF8).into());
                    // Mid beam (Magenta)
                    draw_beam(window, 0.44, 0.14, 0.0, 0.40, rgb(0xEC4899).into());
                    // Emerald lower spark
                    draw_beam(
                        window,
                        0.66,
                        0.12,
                        stem_w + 0.055,
                        0.20,
                        rgb(0x10B981).into(),
                    );
                    // Crisp white-ice vertical stem
                    draw_beam(window, 0.22, 0.56, 0.0, stem_w, rgb(0xF4FAFF).into());
                },
            )
            .size_full(),
        )
}

/// Renders a crisp vector icon inside a `size_px × size_px` GPUI element.
pub fn render_icon(kind: IconKind, size_px: f32, color: Hsla) -> impl IntoElement {
    div().size(px(size_px)).flex_none().child(
        canvas(
            |_bounds, _window, _cx| {},
            move |bounds: Bounds<Pixels>, (), window: &mut Window, _cx| {
                paint_icon(kind, bounds, color, window);
            },
        )
        .size_full(),
    )
}

fn pt(bounds: Bounds<Pixels>, nx: f32, ny: f32) -> Point<Pixels> {
    point(
        bounds.origin.x + bounds.size.width * nx,
        bounds.origin.y + bounds.size.height * ny,
    )
}

fn add_rect(pb: &mut PathBuilder, b: Bounds<Pixels>, x: f32, y: f32, w: f32, h: f32) {
    pb.move_to(pt(b, x, y));
    pb.line_to(pt(b, x + w, y));
    pb.line_to(pt(b, x + w, y + h));
    pb.line_to(pt(b, x, y + h));
    pb.close();
}

fn add_line(pb: &mut PathBuilder, b: Bounds<Pixels>, x0: f32, y0: f32, x1: f32, y1: f32) {
    pb.move_to(pt(b, x0, y0));
    pb.line_to(pt(b, x1, y1));
}

fn add_circle_approx(pb: &mut PathBuilder, b: Bounds<Pixels>, cx: f32, cy: f32, r: f32) {
    let segs = 14usize;
    for i in 0..=segs {
        let a = (i as f32 / segs as f32) * std::f32::consts::TAU;
        let p = pt(b, cx + a.cos() * r, cy + a.sin() * r);
        if i == 0 {
            pb.move_to(p);
        } else {
            pb.line_to(p);
        }
    }
    pb.close();
}

fn paint_icon(kind: IconKind, b: Bounds<Pixels>, color: Hsla, window: &mut Window) {
    let stroke_w = (f32::from(b.size.width) * 0.115).clamp(1.25, 2.0);

    match kind {
        IconKind::Play => {
            let mut pb = PathBuilder::fill();
            pb.move_to(pt(b, 0.28, 0.18));
            pb.line_to(pt(b, 0.84, 0.50));
            pb.line_to(pt(b, 0.28, 0.82));
            pb.close();
            if let Ok(path) = pb.build() {
                window.paint_path(path, color);
            }
        }
        IconKind::Pause => {
            let mut pb = PathBuilder::fill();
            add_rect(&mut pb, b, 0.24, 0.18, 0.18, 0.64);
            add_rect(&mut pb, b, 0.58, 0.18, 0.18, 0.64);
            if let Ok(path) = pb.build() {
                window.paint_path(path, color);
            }
        }
        IconKind::Library => {
            let mut pb = PathBuilder::stroke(px(stroke_w));
            add_rect(&mut pb, b, 0.14, 0.14, 0.28, 0.28);
            add_rect(&mut pb, b, 0.58, 0.14, 0.28, 0.28);
            add_rect(&mut pb, b, 0.14, 0.58, 0.28, 0.28);
            add_rect(&mut pb, b, 0.58, 0.58, 0.28, 0.28);
            if let Ok(path) = pb.build() {
                window.paint_path(path, color);
            }
        }
        IconKind::Stage | IconKind::Sun => {
            let mut pb = PathBuilder::stroke(px(stroke_w));
            add_circle_approx(&mut pb, b, 0.50, 0.50, 0.18);
            add_line(&mut pb, b, 0.50, 0.10, 0.50, 0.21);
            add_line(&mut pb, b, 0.50, 0.79, 0.50, 0.90);
            add_line(&mut pb, b, 0.10, 0.50, 0.21, 0.50);
            add_line(&mut pb, b, 0.79, 0.50, 0.90, 0.50);
            add_line(&mut pb, b, 0.22, 0.22, 0.29, 0.29);
            add_line(&mut pb, b, 0.71, 0.71, 0.78, 0.78);
            add_line(&mut pb, b, 0.78, 0.22, 0.71, 0.29);
            add_line(&mut pb, b, 0.29, 0.71, 0.22, 0.78);
            if let Ok(path) = pb.build() {
                window.paint_path(path, color);
            }
        }
        IconKind::Devices => {
            let mut pb = PathBuilder::stroke(px(stroke_w));
            add_rect(&mut pb, b, 0.22, 0.22, 0.56, 0.56);
            add_rect(&mut pb, b, 0.39, 0.39, 0.22, 0.22);
            add_line(&mut pb, b, 0.38, 0.08, 0.38, 0.22);
            add_line(&mut pb, b, 0.62, 0.08, 0.62, 0.22);
            add_line(&mut pb, b, 0.38, 0.78, 0.38, 0.92);
            add_line(&mut pb, b, 0.62, 0.78, 0.62, 0.92);
            add_line(&mut pb, b, 0.08, 0.38, 0.22, 0.38);
            add_line(&mut pb, b, 0.08, 0.62, 0.22, 0.62);
            add_line(&mut pb, b, 0.78, 0.38, 0.92, 0.38);
            add_line(&mut pb, b, 0.78, 0.62, 0.92, 0.62);
            if let Ok(path) = pb.build() {
                window.paint_path(path, color);
            }
        }
        IconKind::Camera => {
            let mut pb = PathBuilder::stroke(px(stroke_w));
            pb.move_to(pt(b, 0.12, 0.32));
            pb.line_to(pt(b, 0.30, 0.32));
            pb.line_to(pt(b, 0.38, 0.20));
            pb.line_to(pt(b, 0.62, 0.20));
            pb.line_to(pt(b, 0.70, 0.32));
            pb.line_to(pt(b, 0.88, 0.32));
            pb.line_to(pt(b, 0.88, 0.80));
            pb.line_to(pt(b, 0.12, 0.80));
            pb.close();
            add_circle_approx(&mut pb, b, 0.50, 0.56, 0.14);
            if let Ok(path) = pb.build() {
                window.paint_path(path, color);
            }
        }
        IconKind::Prev => {
            let mut pb = PathBuilder::stroke(px(stroke_w));
            add_line(&mut pb, b, 0.22, 0.22, 0.22, 0.78);
            pb.move_to(pt(b, 0.78, 0.22));
            pb.line_to(pt(b, 0.38, 0.50));
            pb.line_to(pt(b, 0.78, 0.78));
            pb.close();
            if let Ok(path) = pb.build() {
                window.paint_path(path, color);
            }
        }
        IconKind::Next => {
            let mut pb = PathBuilder::stroke(px(stroke_w));
            add_line(&mut pb, b, 0.78, 0.22, 0.78, 0.78);
            pb.move_to(pt(b, 0.22, 0.22));
            pb.line_to(pt(b, 0.62, 0.50));
            pb.line_to(pt(b, 0.22, 0.78));
            pb.close();
            if let Ok(path) = pb.build() {
                window.paint_path(path, color);
            }
        }
        IconKind::Reset => {
            let mut pb = PathBuilder::stroke(px(stroke_w));
            let segs = 12usize;
            for i in 0..=segs {
                let a = -0.35 + (i as f32 / segs as f32) * 4.85;
                let p = pt(b, 0.50 + a.cos() * 0.32, 0.50 + a.sin() * 0.32);
                if i == 0 {
                    pb.move_to(p);
                } else {
                    pb.line_to(p);
                }
            }
            add_line(&mut pb, b, 0.16, 0.22, 0.20, 0.42);
            add_line(&mut pb, b, 0.38, 0.38, 0.20, 0.42);
            if let Ok(path) = pb.build() {
                window.paint_path(path, color);
            }
        }
        IconKind::Sliders => {
            let mut pb = PathBuilder::stroke(px(stroke_w));
            add_line(&mut pb, b, 0.22, 0.15, 0.22, 0.44);
            add_line(&mut pb, b, 0.22, 0.62, 0.22, 0.85);
            add_line(&mut pb, b, 0.12, 0.53, 0.32, 0.53);

            add_line(&mut pb, b, 0.50, 0.15, 0.50, 0.28);
            add_line(&mut pb, b, 0.50, 0.46, 0.50, 0.85);
            add_line(&mut pb, b, 0.40, 0.37, 0.60, 0.37);

            add_line(&mut pb, b, 0.78, 0.15, 0.78, 0.56);
            add_line(&mut pb, b, 0.78, 0.74, 0.78, 0.85);
            add_line(&mut pb, b, 0.68, 0.65, 0.88, 0.65);
            if let Ok(path) = pb.build() {
                window.paint_path(path, color);
            }
        }
        IconKind::PanelRight => {
            let mut pb = PathBuilder::stroke(px(stroke_w));
            add_rect(&mut pb, b, 0.14, 0.16, 0.72, 0.68);
            add_line(&mut pb, b, 0.62, 0.16, 0.62, 0.84);
            if let Ok(path) = pb.build() {
                window.paint_path(path, color);
            }
        }
        IconKind::Bolt => {
            let mut pb = PathBuilder::stroke(px(stroke_w));
            pb.move_to(pt(b, 0.56, 0.12));
            pb.line_to(pt(b, 0.20, 0.56));
            pb.line_to(pt(b, 0.50, 0.56));
            pb.line_to(pt(b, 0.44, 0.88));
            pb.line_to(pt(b, 0.80, 0.44));
            pb.line_to(pt(b, 0.50, 0.44));
            pb.close();
            if let Ok(path) = pb.build() {
                window.paint_path(path, color);
            }
        }
        IconKind::ChevronDown => {
            let mut pb = PathBuilder::stroke(px(stroke_w));
            pb.move_to(pt(b, 0.24, 0.38));
            pb.line_to(pt(b, 0.50, 0.64));
            pb.line_to(pt(b, 0.76, 0.38));
            if let Ok(path) = pb.build() {
                window.paint_path(path, color);
            }
        }
        IconKind::ChevronUp => {
            let mut pb = PathBuilder::stroke(px(stroke_w));
            pb.move_to(pt(b, 0.24, 0.62));
            pb.line_to(pt(b, 0.50, 0.36));
            pb.line_to(pt(b, 0.76, 0.62));
            if let Ok(path) = pb.build() {
                window.paint_path(path, color);
            }
        }
        IconKind::Settings => {
            let mut pb = PathBuilder::stroke(px(stroke_w));
            add_circle_approx(&mut pb, b, 0.50, 0.50, 0.16);
            add_circle_approx(&mut pb, b, 0.50, 0.50, 0.32);
            for i in 0..8 {
                let a = (i as f32 / 8.0) * std::f32::consts::TAU;
                let x0 = 0.50 + a.cos() * 0.33;
                let y0 = 0.50 + a.sin() * 0.33;
                let x1 = 0.50 + a.cos() * 0.44;
                let y1 = 0.50 + a.sin() * 0.44;
                add_line(&mut pb, b, x0, y0, x1, y1);
            }
            if let Ok(path) = pb.build() {
                window.paint_path(path, color);
            }
        }
        IconKind::WinMinimize => {
            let mut pb = PathBuilder::stroke(px(stroke_w));
            add_line(&mut pb, b, 0.22, 0.52, 0.78, 0.52);
            if let Ok(path) = pb.build() {
                window.paint_path(path, color);
            }
        }
        IconKind::WinMaximize => {
            let mut pb = PathBuilder::stroke(px(stroke_w));
            add_rect(&mut pb, b, 0.22, 0.22, 0.56, 0.56);
            if let Ok(path) = pb.build() {
                window.paint_path(path, color);
            }
        }
        IconKind::WinRestore => {
            let mut pb = PathBuilder::stroke(px(stroke_w));
            add_rect(&mut pb, b, 0.20, 0.32, 0.46, 0.46);
            pb.move_to(pt(b, 0.34, 0.32));
            pb.line_to(pt(b, 0.34, 0.20));
            pb.line_to(pt(b, 0.80, 0.20));
            pb.line_to(pt(b, 0.80, 0.66));
            pb.line_to(pt(b, 0.66, 0.66));
            if let Ok(path) = pb.build() {
                window.paint_path(path, color);
            }
        }
        IconKind::WinClose => {
            let mut pb = PathBuilder::stroke(px(stroke_w));
            add_line(&mut pb, b, 0.24, 0.24, 0.76, 0.76);
            add_line(&mut pb, b, 0.76, 0.24, 0.24, 0.76);
            if let Ok(path) = pb.build() {
                window.paint_path(path, color);
            }
        }
    }
}
