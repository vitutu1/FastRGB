//! Contextual screen capture module rendered inside the Glass Inspector for screen-reactive effects.

use gpui::{Bounds, Context, Corners, Pixels, Window, canvas, div, prelude::*, px, rgba};

use crate::state::FastRgbApp;
use crate::ui::theme;

/// Renders live DXGI monitor selection and preview thumbnail without redundant chrome.
pub fn render_capture_panel(
    app: &mut FastRgbApp,
    _window: &mut Window,
    cx: &mut Context<FastRgbApp>,
) -> impl IntoElement {
    let capture_running = app.capture_service.is_running();
    let preview_image = app.capture_preview_image.clone();
    let retired_previews = std::mem::take(&mut app.retired_preview_images);
    let selected_monitor = app.selected_monitor;
    let monitors = app.monitors.clone();

    div()
        .flex()
        .flex_col()
        .gap_2p5()
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .text_xs()
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .text_color(theme::ink_muted())
                        .child("Screen capture"),
                )
                .child(
                    div()
                        .id("capture-toggle-btn")
                        .px_2p5()
                        .py_0p5()
                        .rounded_full()
                        .cursor_pointer()
                        .border_1()
                        .border_color(if capture_running {
                            theme::signal_live().opacity(0.45)
                        } else {
                            theme::border_hairline()
                        })
                        .bg(if capture_running {
                            theme::signal_live().opacity(0.16)
                        } else {
                            theme::bg_hover()
                        })
                        .text_xs()
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .text_color(if capture_running {
                            theme::signal_live()
                        } else {
                            theme::ink_secondary()
                        })
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.toggle_capture(cx);
                        }))
                        .child(if capture_running { "Live" } else { "Start" }),
                ),
        )
        .child(
            div()
                .flex()
                .flex_wrap()
                .gap_1()
                .p(px(3.0))
                .rounded(px(10.0))
                .bg(theme::bg_well())
                .border_1()
                .border_color(theme::border_hairline())
                .children(monitors.into_iter().map(|mon| {
                    let is_sel = mon.index == selected_monitor;
                    let idx = mon.index;
                    let short_label = format!("Display {} · {}×{}", idx + 1, mon.width, mon.height);
                    div()
                        .id(gpui::SharedString::from(format!("mon-{idx}")))
                        .flex_1()
                        .px_2()
                        .py_1()
                        .rounded(px(7.0))
                        .text_center()
                        .cursor_pointer()
                        .bg(if is_sel {
                            theme::bg_pill_active()
                        } else {
                            gpui::transparent_black()
                        })
                        .text_xs()
                        .text_color(if is_sel {
                            theme::ink_primary()
                        } else {
                            theme::ink_secondary()
                        })
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.select_monitor(idx, cx);
                            this.push_toast(format!("Capture switched to Display {}", idx + 1), cx);
                        }))
                        .child(short_label)
                })),
        )
        .child(
            div()
                .relative()
                .w_full()
                .h(px(138.0))
                .rounded(px(10.0))
                .overflow_hidden()
                .bg(theme::bg_well())
                .border_1()
                .border_color(theme::border_hairline())
                .child(
                    canvas(
                        move |_bounds, window, _cx| {
                            for old in retired_previews {
                                let _ = window.drop_image(old);
                            }
                        },
                        move |bounds: Bounds<Pixels>, (), window, _cx| {
                            if let Some(ref img) = preview_image {
                                let _ = window.paint_image(
                                    bounds,
                                    Corners::all(px(9.0)),
                                    img.clone(),
                                    0,
                                    false,
                                );
                            }
                        },
                    )
                    .size_full(),
                )
                .child(
                    div()
                        .absolute()
                        .top(px(8.0))
                        .right(px(8.0))
                        .px_2()
                        .py_0p5()
                        .rounded_full()
                        .bg(rgba(0x050507BA))
                        .text_xs()
                        .text_color(if capture_running {
                            theme::signal_live()
                        } else {
                            theme::ink_muted()
                        })
                        .child(if capture_running {
                            "● DXGI"
                        } else {
                            "Paused"
                        }),
                ),
        )
}
