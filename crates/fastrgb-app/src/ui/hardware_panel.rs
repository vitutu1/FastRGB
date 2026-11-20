//! Minimalist hardware output control bar and collapsible low-level bus diagnostics drawer.

use fastrgb_hardware::HardwareMode;
use gpui::{Context, div, prelude::*, px};

use crate::state::FastRgbApp;
use crate::ui::icons::{IconKind, render_icon};
use crate::ui::theme;

/// Renders the unified top Hardware Output Bar (`Dry-Run` / `Go Live`, protocol toggles, `Rescan`).
pub fn render_hardware_bar(app: &FastRgbApp, cx: &mut Context<FastRgbApp>) -> impl IntoElement {
    let report = &app.hardware_report;
    let is_live = app.hardware_mode == HardwareMode::LiveHardware;
    let order_label = app.asus_argb_order.label();
    let gpu_mode_label = app.msi_gpu_mode.label();

    let (title, subtitle, sub_color) = if is_live {
        (
            "Live Hardware Active",
            "Streaming to ASUS HID · NVAPI GPU · Corsair SMBus",
            theme::ink_secondary(),
        )
    } else if report.external_rgb_running {
        (
            "Dry-Run Output",
            "External RGB controller detected · close before going live",
            theme::signal_warn(),
        )
    } else {
        (
            "Dry-Run Output",
            "Bus clear · ready for live output",
            theme::ink_secondary(),
        )
    };

    div()
        .w_full()
        .px_4()
        .py_3p5()
        .rounded(px(14.0))
        .bg(theme::bg_elevated())
        .border_1()
        .border_color(if is_live {
            theme::signal_live().opacity(0.35)
        } else {
            theme::border_hairline()
        })
        .flex()
        .items_center()
        .justify_between()
        .gap_4()
        .child(
            div()
                .flex()
                .items_center()
                .gap_3()
                .child(
                    div()
                        .size(px(34.0))
                        .rounded(px(10.0))
                        .flex()
                        .items_center()
                        .justify_center()
                        .bg(if is_live {
                            theme::signal_live().opacity(0.12)
                        } else {
                            theme::bg_hover()
                        })
                        .border_1()
                        .border_color(if is_live {
                            theme::signal_live().opacity(0.35)
                        } else {
                            theme::border_hairline()
                        })
                        .child(render_icon(
                            IconKind::Bolt,
                            16.0,
                            if is_live {
                                theme::signal_live()
                            } else {
                                theme::ink_secondary()
                            },
                        )),
                )
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
                                .child(title),
                        )
                        .child(div().text_xs().text_color(sub_color).child(subtitle)),
                ),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(
                    div()
                        .id("hw-argb-order-btn")
                        .px_3()
                        .py_1p5()
                        .rounded_full()
                        .cursor_pointer()
                        .bg(theme::bg_hover())
                        .border_1()
                        .border_color(theme::border_hairline())
                        .flex()
                        .items_center()
                        .gap_1p5()
                        .text_xs()
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.cycle_asus_argb_order(cx);
                        }))
                        .child(div().text_color(theme::ink_secondary()).child("ARGB"))
                        .child(
                            div()
                                .font_weight(gpui::FontWeight::MEDIUM)
                                .text_color(theme::ink_primary())
                                .child(order_label),
                        ),
                )
                .child(
                    div()
                        .id("hw-gpu-mode-btn")
                        .px_3()
                        .py_1p5()
                        .rounded_full()
                        .cursor_pointer()
                        .bg(theme::bg_hover())
                        .border_1()
                        .border_color(theme::border_hairline())
                        .flex()
                        .items_center()
                        .gap_1p5()
                        .text_xs()
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.toggle_msi_gpu_mode(cx);
                        }))
                        .child(div().text_color(theme::ink_secondary()).child("GPU"))
                        .child(
                            div()
                                .font_weight(gpui::FontWeight::MEDIUM)
                                .text_color(theme::ink_primary())
                                .child(gpu_mode_label),
                        ),
                )
                .child(
                    div()
                        .id("hw-rescan-btn")
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
                            this.refresh_hardware(cx);
                        }))
                        .child("Rescan"),
                )
                .child(
                    div()
                        .id("hw-live-toggle-btn")
                        .px_3p5()
                        .py_1p5()
                        .rounded_full()
                        .cursor_pointer()
                        .border_1()
                        .border_color(if is_live {
                            theme::signal_live().opacity(0.45)
                        } else {
                            gpui::transparent_black()
                        })
                        .bg(if is_live {
                            theme::signal_live().opacity(0.16)
                        } else {
                            theme::ink_primary()
                        })
                        .text_xs()
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(if is_live {
                            theme::signal_live()
                        } else {
                            theme::bg_void()
                        })
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.toggle_hardware_mode(cx);
                        }))
                        .child(if is_live {
                            "Switch to Dry-Run"
                        } else {
                            "Go Live"
                        }),
                ),
        )
}

/// Renders the closed-by-default collapsible `Bus diagnostics` drawer for HID / NVAPI / SMBus details.
pub fn render_diagnostics_drawer(
    app: &FastRgbApp,
    cx: &mut Context<FastRgbApp>,
) -> impl IntoElement {
    let report = &app.hardware_report;
    let is_open = app.show_diagnostics;
    let smbus_txt = if report.smbus_available {
        "SMBus live"
    } else {
        "SMBus dry-run"
    };
    let summary = format!(
        "Bus diagnostics · {} HID · {} NVAPI · {smbus_txt}",
        report.matched_hid.len(),
        report.nvidia_gpus.len()
    );

    div()
        .w_full()
        .rounded(px(14.0))
        .bg(theme::bg_surface())
        .border_1()
        .border_color(theme::border_hairline())
        .overflow_hidden()
        .child(
            div()
                .id("bus-diag-toggle")
                .w_full()
                .px_4()
                .py_3()
                .flex()
                .items_center()
                .justify_between()
                .cursor_pointer()
                .on_click(cx.listener(|this, _, _, cx| {
                    this.show_diagnostics = !this.show_diagnostics;
                    cx.notify();
                }))
                .child(
                    div()
                        .text_xs()
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .text_color(theme::ink_secondary())
                        .child(summary),
                )
                .child(render_icon(
                    if is_open {
                        IconKind::ChevronUp
                    } else {
                        IconKind::ChevronDown
                    },
                    14.0,
                    theme::ink_muted(),
                )),
        )
        .when(is_open, |el| {
            el.child(
                div()
                    .w_full()
                    .px_4()
                    .pb_4()
                    .pt_3()
                    .border_t_1()
                    .border_color(theme::border_hairline())
                    .flex()
                    .flex_col()
                    .gap_2()
                    .when_some(app.hardware_status.clone(), |inner, msg| {
                        inner.child(
                            div()
                                .px_2p5()
                                .py_1p5()
                                .rounded(px(7.0))
                                .bg(theme::bg_well())
                                .text_xs()
                                .text_color(theme::from_core_rgb(app.ambient_rgb))
                                .child(msg),
                        )
                    })
                    .children(report.matched_hid.iter().map(|m| {
                        diag_row(
                            m.driver_name.to_owned(),
                            format!(
                                "{:04X}:{:04X} · MI {:?} · {}B",
                                m.info.vendor_id,
                                m.info.product_id,
                                m.info.interface,
                                m.info.output_report_len
                            ),
                            theme::signal_live(),
                        )
                    }))
                    .children(report.nvidia_gpus.iter().map(|gpu| {
                        diag_row(
                            gpu.name.clone(),
                            format!(
                                "NVAPI [{idx}] · {:04X}:{:04X}",
                                gpu.vendor_id,
                                gpu.device_id,
                                idx = gpu.index
                            ),
                            theme::signal_live(),
                        )
                    }))
                    .children(report.notes.iter().map(|note| {
                        let c = if report.smbus_available {
                            theme::signal_live()
                        } else {
                            theme::signal_warn()
                        };
                        div().px_1().text_xs().text_color(c).child(note.clone())
                    }))
                    .children(report.all_hid.iter().take(12).map(|hid| {
                        let label = if hid.product.is_empty() {
                            "HID Interface".to_owned()
                        } else {
                            hid.product.clone()
                        };
                        diag_row(
                            label,
                            format!(
                                "{:04X}:{:04X} · UP {:04X} · {}B",
                                hid.vendor_id,
                                hid.product_id,
                                hid.usage_page,
                                hid.output_report_len
                            ),
                            theme::ink_muted(),
                        )
                    })),
            )
        })
}

fn diag_row(left: String, right: String, right_color: gpui::Hsla) -> impl IntoElement {
    div()
        .w_full()
        .px_2p5()
        .py_1p5()
        .rounded(px(7.0))
        .bg(theme::bg_well())
        .flex()
        .items_center()
        .justify_between()
        .gap_3()
        .text_xs()
        .child(div().text_color(theme::ink_secondary()).child(left))
        .child(div().text_color(right_color).child(right))
}
