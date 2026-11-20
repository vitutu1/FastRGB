//! Frameless hero `16:10` RGB canvas stage with reactive ambient aura and minimal floating HUD.

use gpui::{
    Bounds, Context, Corners, PaintQuad, Pixels, Window, canvas, div, linear_color_stop,
    linear_gradient, point, prelude::*, px, rgba, size,
};

use crate::state::FastRgbApp;
use crate::ui::icons::{IconKind, render_icon};
use crate::ui::theme;

/// Renders the frameless hero RGB canvas stage with floating corner HUD pills (`Overlay`, `Params`).
pub fn render_canvas_stage(
    app: &mut FastRgbApp,
    _window: &mut Window,
    cx: &mut Context<FastRgbApp>,
) -> impl IntoElement {
    let current_image = app.canvas_image.clone();
    let retired = std::mem::take(&mut app.retired_canvas_images);
    let show_overlay = app.show_device_overlay;
    let inspector_open = app.inspector_open;
    let devices_snapshot = app.devices.devices().to_vec();
    let ambient = app.ambient_rgb;
    let ambient_hsla = theme::from_core_rgb(ambient);

    let stage_w = if inspector_open { px(768.0) } else { px(912.0) };
    let stage_h = if inspector_open { px(480.0) } else { px(570.0) };

    div()
        .w(stage_w)
        .h(stage_h)
        .p(px(1.0))
        .rounded(px(20.0))
        .bg(linear_gradient(
            145.0,
            linear_color_stop(rgba(0xFFFFFF29), 0.0),
            linear_color_stop(ambient_hsla.opacity(0.32), 1.0),
        ))
        .shadow(theme::shadow_stage(ambient))
        .child(
            div()
                .relative()
                .size_full()
                .rounded(px(19.0))
                .overflow_hidden()
                .bg(gpui::rgb(0x020204))
                .child(
                    canvas(
                        move |_bounds, window, _cx| {
                            for old in retired {
                                let _ = window.drop_image(old);
                            }
                        },
                        move |bounds: Bounds<Pixels>, (), window, _cx| {
                            if let Some(ref img) = current_image {
                                let _ = window.paint_image(
                                    bounds,
                                    Corners::all(px(18.0)),
                                    img.clone(),
                                    0,
                                    false,
                                );
                            }

                            if show_overlay {
                                for dev in &devices_snapshot {
                                    let p = dev.placement;
                                    let dx = bounds.origin.x + bounds.size.width * p.origin_nx;
                                    let dy = bounds.origin.y + bounds.size.height * p.origin_ny;
                                    let dw = bounds.size.width * p.width_nx;
                                    let dh = bounds.size.height * p.height_ny;

                                    window.paint_quad(PaintQuad {
                                        bounds: Bounds {
                                            origin: point(dx, dy),
                                            size: size(dw, dh),
                                        },
                                        corner_radii: Corners::all(px(6.0)),
                                        background: rgba(0x05060938).into(),
                                        border_widths: gpui::Edges::all(px(1.0)),
                                        border_color: gpui::white().opacity(0.24),
                                        border_style: gpui::BorderStyle::Solid,
                                    });

                                    let gw = dev.layout.grid_width.max(1) as f32;
                                    let gh = dev.layout.grid_height.max(1) as f32;
                                    for (i, led) in dev.layout.leds.iter().enumerate() {
                                        let color =
                                            dev.last_colors.get(i).copied().unwrap_or_default();
                                        let nx = (led.x as f32 + 0.5) / gw;
                                        let ny = (led.y as f32 + 0.5) / gh;
                                        let lx = dx + dw * nx - px(2.5);
                                        let ly = dy + dh * ny - px(2.5);
                                        window.paint_quad(PaintQuad {
                                            bounds: Bounds {
                                                origin: point(lx, ly),
                                                size: size(px(5.0), px(5.0)),
                                            },
                                            corner_radii: Corners::all(px(2.5)),
                                            background: theme::from_core_rgb(color).into(),
                                            border_widths: gpui::Edges::all(px(0.6)),
                                            border_color: gpui::black().opacity(0.55),
                                            border_style: gpui::BorderStyle::Solid,
                                        });
                                    }
                                }
                            }
                        },
                    )
                    .size_full(),
                )
                .child(
                    // Minimal floating corner HUD
                    div()
                        .absolute()
                        .top(px(12.0))
                        .left(px(14.0))
                        .right(px(14.0))
                        .flex()
                        .items_center()
                        .justify_between()
                        .child(
                            div()
                                .id("stage-overlay-pill")
                                .px_2p5()
                                .py_1()
                                .rounded_full()
                                .cursor_pointer()
                                .bg(rgba(0x08090DA6))
                                .border_1()
                                .border_color(rgba(0xFFFFFF1C))
                                .flex()
                                .items_center()
                                .gap_1p5()
                                .text_xs()
                                .font_weight(gpui::FontWeight::MEDIUM)
                                .text_color(if show_overlay {
                                    theme::ink_primary()
                                } else {
                                    theme::ink_secondary()
                                })
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.show_device_overlay = !this.show_device_overlay;
                                    cx.notify();
                                }))
                                .child(div().size(px(6.0)).rounded_full().bg(if show_overlay {
                                    ambient_hsla
                                } else {
                                    theme::ink_muted()
                                }))
                                .child("Overlay"),
                        )
                        .child(
                            div()
                                .id("stage-params-pill")
                                .px_2p5()
                                .py_1()
                                .rounded_full()
                                .cursor_pointer()
                                .bg(rgba(0x08090DA6))
                                .border_1()
                                .border_color(rgba(0xFFFFFF1C))
                                .flex()
                                .items_center()
                                .gap_1p5()
                                .text_xs()
                                .font_weight(gpui::FontWeight::MEDIUM)
                                .text_color(if inspector_open {
                                    theme::ink_primary()
                                } else {
                                    theme::ink_secondary()
                                })
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.inspector_open = !this.inspector_open;
                                    cx.notify();
                                }))
                                .child(render_icon(
                                    IconKind::PanelRight,
                                    13.0,
                                    if inspector_open {
                                        theme::ink_primary()
                                    } else {
                                        theme::ink_secondary()
                                    },
                                ))
                                .child("Params"),
                        ),
                ),
        )
}
