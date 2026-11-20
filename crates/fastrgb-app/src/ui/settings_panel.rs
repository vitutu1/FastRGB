//! Minimalist `Settings` view following the Obsidian Instrument design language:
//! unified translucent sheets, 1px hairline dividers, tactile switches, and segmented pills.

use fastrgb_hardware::{MsiGpuControlMode, RgbOrder};
use gpui::{Context, Hsla, div, prelude::*, px, rgba};

use crate::state::FastRgbApp;
use crate::ui::icons::render_fastrgb_logo;
use crate::ui::theme;

/// Renders the `Settings` section (`General`, `Performance & Hardware`, `About`).
pub fn render_settings_view(app: &FastRgbApp, cx: &mut Context<FastRgbApp>) -> impl IntoElement {
    let ambient = theme::from_core_rgb(app.ambient_rgb);
    let autostart = app.start_with_windows;
    let start_minimized = app.settings.start_minimized;
    let live_on_startup = app.settings.live_on_startup;
    let dot_field_enabled = app.settings.dot_field_enabled;
    let fps_cap = app.target_fps_cap;
    let argb_order = app.asus_argb_order;
    let gpu_mode = app.msi_gpu_mode;

    div()
        .id("settings-view-scroll")
        .flex_1()
        .w_full()
        .min_h_0()
        .overflow_y_scroll()
        .px_6()
        .pb(px(96.0))
        .pt_2()
        .child(
            div()
                .w(px(760.0))
                .mx_auto()
                .flex()
                .flex_col()
                .gap_5()
                // Section 1: General & Startup
                .child(settings_group(
                    "General",
                    "Startup, tray & visual backdrop",
                    div()
                        .flex()
                        .flex_col()
                        .child(toggle_setting_row(
                            "set-autostart",
                            "Start with Windows",
                            "Launch FastRGB automatically on sign-in",
                            autostart,
                            ambient,
                            cx.listener(|this, _, _, cx| {
                                this.toggle_autostart(cx);
                            }),
                        ))
                        .child(hairline())
                        .child(toggle_setting_row(
                            "set-minimized",
                            "Start minimized to tray",
                            "Keep window hidden in the system tray at startup",
                            start_minimized,
                            ambient,
                            cx.listener(|this, _, _, cx| {
                                this.toggle_start_minimized(cx);
                            }),
                        ))
                        .child(hairline())
                        .child(toggle_setting_row(
                            "set-live-startup",
                            "Auto-enable Live Output",
                            "Attach HID, NVAPI & SMBus automatically when bus is clear",
                            live_on_startup,
                            ambient,
                            cx.listener(|this, _, _, cx| {
                                this.toggle_live_on_startup(cx);
                            }),
                        ))
                        .child(hairline())
                        .child(toggle_setting_row(
                            "set-dotfield",
                            "Interactive DotField",
                            "Reactive LED-matrix background animation",
                            dot_field_enabled,
                            ambient,
                            cx.listener(|this, _, _, cx| {
                                this.toggle_dot_field(cx);
                            }),
                        )),
                ))
                // Section 2: Performance & Hardware
                .child(settings_group(
                    "Performance & Hardware",
                    "Bus pacing & controller protocols",
                    div()
                        .flex()
                        .flex_col()
                        .child(setting_row(
                            "RGB frame cap",
                            "Protects SMBus, I2C and HID controllers",
                            div()
                                .p(px(3.0))
                                .rounded(px(10.0))
                                .bg(theme::bg_well())
                                .border_1()
                                .border_color(theme::border_hairline())
                                .flex()
                                .items_center()
                                .gap_1()
                                .children(
                                    [
                                        (Some(15), "15"),
                                        (Some(30), "30"),
                                        (Some(45), "45"),
                                        (None, "60 fps"),
                                    ]
                                    .into_iter()
                                    .map(|(cap, label)| {
                                        let sel = fps_cap == cap;
                                        div()
                                            .id(gpui::SharedString::from(format!(
                                                "set-fps-{label}"
                                            )))
                                            .px_2p5()
                                            .py_1()
                                            .rounded(px(7.0))
                                            .cursor_pointer()
                                            .bg(if sel {
                                                theme::bg_pill_active()
                                            } else {
                                                gpui::transparent_black()
                                            })
                                            .text_xs()
                                            .font_weight(gpui::FontWeight::MEDIUM)
                                            .text_color(if sel {
                                                theme::ink_primary()
                                            } else {
                                                theme::ink_secondary()
                                            })
                                            .on_click(cx.listener(move |this, _, _, cx| {
                                                this.set_default_fps_cap(cap, cx);
                                            }))
                                            .child(label)
                                    }),
                                ),
                        ))
                        .child(hairline())
                        .child(setting_row(
                            "ASUS ARGB wire order",
                            "Channel byte order for 3-pin ARGB headers",
                            div()
                                .p(px(3.0))
                                .rounded(px(10.0))
                                .bg(theme::bg_well())
                                .border_1()
                                .border_color(theme::border_hairline())
                                .flex()
                                .items_center()
                                .gap_0p5()
                                .children(
                                    [
                                        RgbOrder::Rgb,
                                        RgbOrder::Grb,
                                        RgbOrder::Bgr,
                                        RgbOrder::Brg,
                                        RgbOrder::Rbg,
                                        RgbOrder::Gbr,
                                    ]
                                    .into_iter()
                                    .map(|order| {
                                        let sel = argb_order == order;
                                        div()
                                            .id(gpui::SharedString::from(format!(
                                                "set-order-{}",
                                                order.label()
                                            )))
                                            .px_2()
                                            .py_1()
                                            .rounded(px(7.0))
                                            .cursor_pointer()
                                            .bg(if sel {
                                                theme::bg_pill_active()
                                            } else {
                                                gpui::transparent_black()
                                            })
                                            .text_xs()
                                            .font_weight(gpui::FontWeight::MEDIUM)
                                            .text_color(if sel {
                                                theme::ink_primary()
                                            } else {
                                                theme::ink_secondary()
                                            })
                                            .on_click(cx.listener(move |this, _, _, cx| {
                                                this.set_asus_argb_order(order, cx);
                                            }))
                                            .child(order.label())
                                    }),
                                ),
                        ))
                        .child(hairline())
                        .child(setting_row(
                            "MSI GPU mode",
                            "NVAPI I2C addressing on RTX 4070 SUPER",
                            div()
                                .p(px(3.0))
                                .rounded(px(10.0))
                                .bg(theme::bg_well())
                                .border_1()
                                .border_color(theme::border_hairline())
                                .flex()
                                .items_center()
                                .gap_1()
                                .children(
                                    [
                                        (MsiGpuControlMode::SingleZone, "SingleZone"),
                                        (MsiGpuControlMode::PerLed, "PerLed"),
                                    ]
                                    .into_iter()
                                    .map(|(mode, label)| {
                                        let sel = gpu_mode == mode;
                                        div()
                                            .id(gpui::SharedString::from(format!(
                                                "set-gpu-{label}"
                                            )))
                                            .px_3()
                                            .py_1()
                                            .rounded(px(7.0))
                                            .cursor_pointer()
                                            .bg(if sel {
                                                theme::bg_pill_active()
                                            } else {
                                                gpui::transparent_black()
                                            })
                                            .text_xs()
                                            .font_weight(gpui::FontWeight::MEDIUM)
                                            .text_color(if sel {
                                                theme::ink_primary()
                                            } else {
                                                theme::ink_secondary()
                                            })
                                            .on_click(cx.listener(move |this, _, _, cx| {
                                                this.set_msi_gpu_mode(mode, cx);
                                            }))
                                            .child(label)
                                    }),
                                ),
                        )),
                ))
                // Section 3: Compact About & Reset
                .child(
                    div()
                        .w_full()
                        .px_4()
                        .py_3p5()
                        .rounded(px(14.0))
                        .bg(theme::bg_elevated())
                        .border_1()
                        .border_color(theme::border_hairline())
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap_4()
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_3()
                                .child(render_fastrgb_logo(32.0))
                                .child(
                                    div()
                                        .flex()
                                        .flex_col()
                                        .gap_0p5()
                                        .child(
                                            div()
                                                .text_sm()
                                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                                .text_color(theme::ink_primary())
                                                .child("FastRGB"),
                                        )
                                        .child(
                                            div()
                                                .text_xs()
                                                .text_color(theme::ink_muted())
                                                .child("v1.0.1 · Native Rust + GPUI"),
                                        ),
                                ),
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_2()
                                .child(
                                    div()
                                        .id("set-hide-tray")
                                        .px_3()
                                        .py_1p5()
                                        .rounded_full()
                                        .cursor_pointer()
                                        .bg(theme::bg_hover())
                                        .border_1()
                                        .border_color(theme::border_hairline())
                                        .text_xs()
                                        .font_weight(gpui::FontWeight::MEDIUM)
                                        .text_color(theme::ink_secondary())
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.hide_to_tray(cx);
                                        }))
                                        .child("Hide to Tray"),
                                )
                                .child(
                                    div()
                                        .id("set-reset-defaults")
                                        .px_3()
                                        .py_1p5()
                                        .rounded_full()
                                        .cursor_pointer()
                                        .bg(theme::bg_hover())
                                        .border_1()
                                        .border_color(theme::border_hairline())
                                        .text_xs()
                                        .font_weight(gpui::FontWeight::MEDIUM)
                                        .text_color(theme::ink_secondary())
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.reset_settings_to_default(cx);
                                        }))
                                        .child("Reset defaults"),
                                ),
                        ),
                ),
        )
}

fn settings_group(
    title: &'static str,
    subtitle: &'static str,
    body: impl IntoElement,
) -> impl IntoElement {
    div()
        .w_full()
        .flex()
        .flex_col()
        .gap_2p5()
        .child(
            div()
                .px_0p5()
                .flex()
                .items_baseline()
                .justify_between()
                .child(
                    div()
                        .text_sm()
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(theme::ink_primary())
                        .child(title),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(theme::ink_muted())
                        .child(subtitle),
                ),
        )
        .child(
            div()
                .w_full()
                .rounded(px(14.0))
                .bg(theme::bg_elevated())
                .border_1()
                .border_color(theme::border_hairline())
                .overflow_hidden()
                .child(body),
        )
}

fn setting_row(
    title: &'static str,
    subtitle: &'static str,
    control: impl IntoElement,
) -> impl IntoElement {
    div()
        .w_full()
        .px_4()
        .py_3()
        .flex()
        .items_center()
        .justify_between()
        .gap_4()
        .child(
            div()
                .flex()
                .flex_col()
                .gap_0p5()
                .child(
                    div()
                        .text_xs()
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .text_color(theme::ink_primary())
                        .child(title),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(theme::ink_muted())
                        .child(subtitle),
                ),
        )
        .child(control)
}

fn toggle_setting_row(
    id: &'static str,
    title: &'static str,
    subtitle: &'static str,
    checked: bool,
    ambient: Hsla,
    on_click: impl Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    div()
        .id(id)
        .w_full()
        .px_4()
        .py_3()
        .cursor_pointer()
        .flex()
        .items_center()
        .justify_between()
        .gap_4()
        .on_click(on_click)
        .child(
            div()
                .flex()
                .flex_col()
                .gap_0p5()
                .child(
                    div()
                        .text_xs()
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .text_color(theme::ink_primary())
                        .child(title),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(theme::ink_muted())
                        .child(subtitle),
                ),
        )
        .child(
            div()
                .relative()
                .w(px(36.0))
                .h(px(20.0))
                .rounded_full()
                .border_1()
                .border_color(if checked {
                    ambient.opacity(0.55)
                } else {
                    theme::border_hairline()
                })
                .bg(if checked {
                    ambient.opacity(0.18)
                } else {
                    rgba(0xFFFFFF18).into()
                })
                .child(
                    div()
                        .absolute()
                        .top(px(2.5))
                        .left(px(if checked { 18.0 } else { 2.5 }))
                        .size(px(13.0))
                        .rounded_full()
                        .bg(if checked {
                            gpui::white()
                        } else {
                            theme::ink_secondary()
                        }),
                ),
        )
}

fn hairline() -> impl IntoElement {
    div().w_full().h(px(1.0)).bg(theme::border_hairline())
}
