//! Minimalist Devices & Hardware view plus compact live LED Ribbons for the `Stage` view.

use fastrgb_devices::{DeviceCategory, LightingMode, VirtualDevice};
use gpui::{Bounds, Context, Corners, PaintQuad, Pixels, canvas, div, point, prelude::*, px, size};

use crate::state::FastRgbApp;
use crate::ui::{hardware_panel, theme};

/// Renders the compact single-row live LED Ribbons strip directly beneath the `Stage` hero canvas.
pub fn render_stage_ribbons(app: &FastRgbApp, cx: &mut Context<FastRgbApp>) -> impl IntoElement {
    let devices = app.devices.devices();
    let ambient = theme::from_core_rgb(app.ambient_rgb);
    let strip_w = if app.inspector_open {
        px(768.0)
    } else {
        px(912.0)
    };

    div()
        .w(strip_w)
        .flex()
        .items_center()
        .gap_2()
        .children(devices.iter().map(|dev| {
            let dev_id = dev.layout.id.clone();
            let short = short_device_name(&dev.layout.id, &dev.layout.name);
            let mode_badge = match dev.mode {
                LightingMode::Canvas => None,
                LightingMode::Forced(_) => Some("forced"),
                LightingMode::Off => Some("off"),
            };
            let is_off = matches!(dev.mode, LightingMode::Off);
            let is_forced = matches!(dev.mode, LightingMode::Forced(_));
            let sample_colors: Vec<_> = dev.last_colors.iter().copied().take(16).collect();

            div()
                .id(gpui::SharedString::from(format!(
                    "stage-ribbon-{}",
                    dev.layout.id
                )))
                .flex_1()
                .min_w_0()
                .px_2p5()
                .py_2()
                .rounded(px(10.0))
                .cursor_pointer()
                .bg(theme::bg_surface())
                .border_1()
                .border_color(if is_forced {
                    ambient.opacity(0.45)
                } else {
                    theme::border_hairline()
                })
                .flex()
                .flex_col()
                .gap_1p5()
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.cycle_device_mode(&dev_id, cx);
                }))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap_1()
                        .child(
                            div()
                                .text_xs()
                                .font_weight(gpui::FontWeight::MEDIUM)
                                .text_color(if is_off {
                                    theme::ink_muted()
                                } else {
                                    theme::ink_primary()
                                })
                                .truncate()
                                .child(short),
                        )
                        .when_some(mode_badge, |el, badge| {
                            el.child(
                                div()
                                    .text_xs()
                                    .text_color(if is_forced {
                                        ambient
                                    } else {
                                        theme::ink_muted()
                                    })
                                    .child(badge),
                            )
                        }),
                )
                .child(
                    div()
                        .w_full()
                        .h(px(6.0))
                        .p(px(1.0))
                        .rounded_full()
                        .bg(theme::bg_well())
                        .child(
                            canvas(
                                |_bounds, _window, _cx| {},
                                move |bounds: Bounds<Pixels>, (), window, _cx| {
                                    let n = sample_colors.len().max(1);
                                    let total_w: f32 = bounds.size.width.into();
                                    let h: f32 = bounds.size.height.into();
                                    let gap = 1.5_f32;
                                    let seg_w =
                                        ((total_w - gap * (n as f32 - 1.0)) / n as f32).max(1.5);
                                    let ox: f32 = bounds.origin.x.into();
                                    let oy: f32 = bounds.origin.y.into();
                                    let zero_edges = gpui::Edges::all(px(0.0));
                                    let no_border = gpui::transparent_black();

                                    for (i, color) in sample_colors.iter().enumerate() {
                                        let x = ox + (i as f32) * (seg_w + gap);
                                        let fill = if is_off {
                                            theme::ink_muted().opacity(0.22)
                                        } else {
                                            theme::from_core_rgb(*color)
                                        };
                                        window.paint_quad(PaintQuad {
                                            bounds: Bounds {
                                                origin: point(px(x), px(oy)),
                                                size: size(px(seg_w), px(h)),
                                            },
                                            corner_radii: Corners::all(px(h * 0.5)),
                                            background: fill.into(),
                                            border_widths: zero_edges,
                                            border_color: no_border,
                                            border_style: gpui::BorderStyle::Solid,
                                        });
                                    }
                                },
                            )
                            .size_full(),
                        ),
                )
        }))
}

/// Renders the calm, minimalist `Devices` view: Hardware Bar, Controllers, Hardware & Desk, and Diagnostics.
pub fn render_devices_view(app: &FastRgbApp, cx: &mut Context<FastRgbApp>) -> impl IntoElement {
    let devices = app.devices.devices();
    let controllers: Vec<&VirtualDevice> = devices
        .iter()
        .filter(|d| {
            matches!(
                d.layout.category,
                DeviceCategory::ArgbStrip | DeviceCategory::Motherboard | DeviceCategory::Fan
            )
        })
        .collect();
    let hardware_peripherals: Vec<&VirtualDevice> = devices
        .iter()
        .filter(|d| {
            matches!(
                d.layout.category,
                DeviceCategory::Gpu
                    | DeviceCategory::Ram
                    | DeviceCategory::Keyboard
                    | DeviceCategory::Mouse
            )
        })
        .collect();

    let ambient = theme::from_core_rgb(app.ambient_rgb);
    let ctrl_count = controllers.len();
    let periph_count = hardware_peripherals.len();

    div()
        .id("devices-view-scroll")
        .flex_1()
        .w_full()
        .min_h_0()
        .overflow_y_scroll()
        .px_6()
        .pb(px(96.0))
        .pt_1()
        .child(
            div()
                .w(px(1176.0))
                .mx_auto()
                .flex()
                .flex_col()
                .gap_6()
                .child(hardware_panel::render_hardware_bar(app, cx))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_3()
                        .child(section_header(
                            "Controllers",
                            format!("ASUS B850 · {ctrl_count} channels"),
                        ))
                        .child(render_device_grid(&controllers, ambient, cx)),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_3()
                        .child(section_header(
                            "Hardware & Desk",
                            format!("{periph_count} devices"),
                        ))
                        .child(render_device_grid(&hardware_peripherals, ambient, cx)),
                )
                .child(hardware_panel::render_diagnostics_drawer(app, cx)),
        )
}

fn section_header(title: &'static str, meta: String) -> impl IntoElement {
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
        .child(div().text_xs().text_color(theme::ink_muted()).child(meta))
}

fn render_device_grid(
    devices: &[&VirtualDevice],
    ambient: gpui::Hsla,
    cx: &mut Context<FastRgbApp>,
) -> impl IntoElement {
    div()
        .flex()
        .flex_wrap()
        .gap_3()
        .children(devices.iter().map(|dev| {
            let dev_id_mode = dev.layout.id.clone();
            let dev_id_dec = dev.layout.id.clone();
            let dev_id_inc = dev.layout.id.clone();
            let is_strip = dev.layout.category == DeviceCategory::ArgbStrip;
            let mode_label = match dev.mode {
                LightingMode::Canvas => "Canvas",
                LightingMode::Forced(_) => "Forced",
                LightingMode::Off => "Off",
            };
            let is_forced = matches!(dev.mode, LightingMode::Forced(_));
            let is_off = matches!(dev.mode, LightingMode::Off);
            let led_count = dev.layout.leds.len();
            let clean_name = clean_device_title(&dev.layout.id, &dev.layout.name);
            let clean_sub = clean_device_subtitle(&dev.layout.id, dev.layout.category, led_count);
            let led_colors: Vec<_> = dev.last_colors.iter().copied().take(32).collect();

            div()
                .w(px(384.0))
                .p_3p5()
                .rounded(px(14.0))
                .bg(theme::bg_surface())
                .border_1()
                .border_color(theme::border_hairline())
                .flex()
                .flex_col()
                .gap_3()
                .child(
                    div()
                        .flex()
                        .items_start()
                        .justify_between()
                        .gap_2()
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
                                        .child(clean_name),
                                )
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(theme::ink_muted())
                                        .child(clean_sub),
                                ),
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_1p5()
                                .when(is_strip, |el| {
                                    el.child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .rounded_full()
                                            .bg(theme::bg_well())
                                            .border_1()
                                            .border_color(theme::border_hairline())
                                            .overflow_hidden()
                                            .child(
                                                div()
                                                    .id(gpui::SharedString::from(format!(
                                                        "led-dec-{}",
                                                        dev.layout.id
                                                    )))
                                                    .px_2()
                                                    .py_0p5()
                                                    .cursor_pointer()
                                                    .text_xs()
                                                    .text_color(theme::ink_secondary())
                                                    .on_click(cx.listener(move |this, _, _, cx| {
                                                        this.adjust_strip_leds(&dev_id_dec, -4, cx);
                                                    }))
                                                    .child("−"),
                                            )
                                            .child(
                                                div()
                                                    .px_1()
                                                    .text_xs()
                                                    .text_color(theme::ink_primary())
                                                    .child(format!("{led_count}")),
                                            )
                                            .child(
                                                div()
                                                    .id(gpui::SharedString::from(format!(
                                                        "led-inc-{}",
                                                        dev.layout.id
                                                    )))
                                                    .px_2()
                                                    .py_0p5()
                                                    .cursor_pointer()
                                                    .text_xs()
                                                    .text_color(theme::ink_secondary())
                                                    .on_click(cx.listener(move |this, _, _, cx| {
                                                        this.adjust_strip_leds(&dev_id_inc, 4, cx);
                                                    }))
                                                    .child("+"),
                                            ),
                                    )
                                })
                                .child(
                                    div()
                                        .id(gpui::SharedString::from(format!(
                                            "dev-mode-{}",
                                            dev.layout.id
                                        )))
                                        .px_2p5()
                                        .py_1()
                                        .rounded_full()
                                        .cursor_pointer()
                                        .border_1()
                                        .border_color(if is_forced {
                                            ambient.opacity(0.45)
                                        } else {
                                            theme::border_hairline()
                                        })
                                        .bg(if is_forced {
                                            ambient.opacity(0.14)
                                        } else {
                                            theme::bg_hover()
                                        })
                                        .text_xs()
                                        .font_weight(gpui::FontWeight::MEDIUM)
                                        .text_color(if is_forced {
                                            ambient
                                        } else if is_off {
                                            theme::ink_muted()
                                        } else {
                                            theme::ink_secondary()
                                        })
                                        .on_click(cx.listener(move |this, _, _, cx| {
                                            this.cycle_device_mode(&dev_id_mode, cx);
                                        }))
                                        .child(mode_label),
                                ),
                        ),
                )
                .child({
                    let cols = 16usize;
                    let rows = led_colors.len().div_ceil(cols).max(1);
                    let matrix_h = px((rows as f32) * 18.0 + 14.0);
                    div()
                        .w_full()
                        .h(matrix_h)
                        .p_2()
                        .rounded(px(10.0))
                        .bg(theme::bg_well())
                        .border_1()
                        .border_color(theme::border_hairline())
                        .child(
                            canvas(
                                |_bounds, _window, _cx| {},
                                move |bounds: Bounds<Pixels>, (), window, _cx| {
                                    let w: f32 = bounds.size.width.into();
                                    let gap = 4.0_f32;
                                    let cell = ((w - gap * (cols as f32 - 1.0)) / cols as f32)
                                        .clamp(8.0, 15.0);
                                    let ox: f32 = bounds.origin.x.into();
                                    let oy: f32 = bounds.origin.y.into();
                                    let border = gpui::black().opacity(0.45);

                                    for (i, color) in led_colors.iter().enumerate() {
                                        let col = i % cols;
                                        let row = i / cols;
                                        let x = ox + (col as f32) * (cell + gap);
                                        let y = oy + (row as f32) * (cell + gap);
                                        let bg = if is_off {
                                            theme::ink_muted().opacity(0.18)
                                        } else {
                                            theme::from_core_rgb(*color)
                                        };
                                        window.paint_quad(PaintQuad {
                                            bounds: Bounds {
                                                origin: point(px(x), px(y)),
                                                size: size(px(cell), px(cell)),
                                            },
                                            corner_radii: Corners::all(px(3.0)),
                                            background: bg.into(),
                                            border_widths: gpui::Edges::all(px(0.5)),
                                            border_color: border,
                                            border_style: gpui::BorderStyle::Solid,
                                        });
                                    }
                                },
                            )
                            .size_full(),
                        )
                })
        }))
}

fn short_device_name(id: &str, fallback: &str) -> String {
    match id {
        "asus_b850_argb_ch1" => "Ch1".to_owned(),
        "asus_b850_argb_ch2" => "Ch2".to_owned(),
        "asus_b850_argb_ch3" => "Ch3".to_owned(),
        "corsair_dominator_dimm1" => "DIMM 1".to_owned(),
        "corsair_dominator_dimm2" => "DIMM 2".to_owned(),
        "corsair_dominator_platinum" => "RAM".to_owned(),
        "msi_4070_super_gaming_x" => "4070S".to_owned(),
        "virtual_keyboard_tkl" => "Board".to_owned(),
        "virtual_mouse" => "Mouse".to_owned(),
        _ => fallback.to_owned(),
    }
}

fn clean_device_title(id: &str, fallback: &str) -> String {
    match id {
        "asus_b850_argb_ch1" => "ASUS B850 · Ch1".to_owned(),
        "asus_b850_argb_ch2" => "ASUS B850 · Ch2".to_owned(),
        "asus_b850_argb_ch3" => "ASUS B850 · Ch3".to_owned(),
        "corsair_dominator_dimm1" => "Corsair Dominator · DIMM 1".to_owned(),
        "corsair_dominator_dimm2" => "Corsair Dominator · DIMM 2".to_owned(),
        "corsair_dominator_platinum" => "Corsair Dominator".to_owned(),
        "msi_4070_super_gaming_x" => "MSI RTX 4070 SUPER".to_owned(),
        "virtual_keyboard_tkl" => "TKL Keyboard".to_owned(),
        "virtual_mouse" => "Gaming Mouse".to_owned(),
        _ => fallback.to_owned(),
    }
}

fn clean_device_subtitle(id: &str, category: DeviceCategory, leds: usize) -> String {
    let kind = match id {
        "corsair_dominator_dimm1" => "SMBus 0x19",
        "corsair_dominator_dimm2" => "SMBus 0x1B",
        _ => match category {
            DeviceCategory::ArgbStrip => "Fans / Strip",
            DeviceCategory::Gpu => "NVAPI I2C",
            DeviceCategory::Ram => "DIMM",
            DeviceCategory::Keyboard | DeviceCategory::Mouse => "Virtual layout",
            DeviceCategory::Fan => "Fan channel",
            DeviceCategory::Motherboard => "Motherboard",
            DeviceCategory::Controller => "Controller",
        },
    };
    format!("{kind} · {leds} LEDs")
}
