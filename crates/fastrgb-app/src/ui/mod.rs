//! Obsidian Instrument minimalist studio shell: full-bleed `DotField` backdrop, floating top pill
//! navigation, routed views (`Library`, `Stage`, `Devices`), unified Glass Inspector, and floating
//! bottom transport dock.

pub mod canvas_view;
pub mod capture_panel;
pub mod controls;
pub mod devices_panel;
pub mod dot_field;
pub mod hardware_panel;
pub mod icons;
pub mod library_panel;
pub mod settings_panel;
pub mod theme;

use fastrgb_effects::EffectCategory;
use fastrgb_hardware::HardwareMode;
use gpui::{
    Context, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, ObjectFit, Window,
    WindowControlArea, anchored, canvas, deferred, div, img, linear_color_stop, linear_gradient,
    prelude::*, px, rgb, rgba,
};

use crate::state::{
    EffectContextMenu, FastRgbApp, NavSection, SliderDragTarget, ratio_from_fps_cap,
    ratio_from_speed,
};
use crate::ui::icons::{IconKind, render_fastrgb_logo, render_icon};

/// Renders the entire FastRGB native GPUI studio window.
pub fn render_root(
    app: &mut FastRgbApp,
    window: &mut Window,
    cx: &mut Context<FastRgbApp>,
) -> impl IntoElement {
    let mut retired = Vec::new();
    if app.nav_section != NavSection::Lighting {
        retired.append(&mut app.retired_canvas_images);
        retired.append(&mut app.retired_preview_images);
    }
    let context_menu = app.context_menu.clone();
    let toast_text = app.active_toast().map(str::to_owned);

    div()
        .relative()
        .size_full()
        .overflow_hidden()
        .bg(theme::bg_void())
        .text_color(theme::ink_primary())
        .on_mouse_move(cx.listener(|this, ev: &MouseMoveEvent, _, cx| {
            this.update_pointer_position(ev.position, cx);
            if this.dragging_slider.is_some() {
                this.update_slider_drag(ev.position.x, cx);
            }
        }))
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(|this, ev: &MouseDownEvent, _, cx| {
                this.trigger_dot_ripple(ev.position, cx);
            }),
        )
        .on_mouse_up(
            MouseButton::Left,
            cx.listener(|this, _: &MouseUpEvent, _, _cx| {
                this.end_slider_drag();
            }),
        )
        .when(!retired.is_empty(), |el| {
            let to_drop = retired;
            el.child(
                canvas(
                    move |_bounds, window, _cx| {
                        for old in to_drop {
                            let _ = window.drop_image(old);
                        }
                    },
                    |_bounds, (), _window, _cx| {},
                )
                .size_0(),
            )
        })
        // Layer 0: Interactive DotField Backdrop (configurable in Settings)
        .when(app.settings.dot_field_enabled, |el| {
            el.child(dot_field::render_dot_field_backdrop(
                &app.dot_field,
                app.ambient_rgb,
            ))
        })
        // Layer 1: Frameless Shell (Floating Top Bar + Active Viewport)
        .child(
            div()
                .relative()
                .size_full()
                .flex()
                .flex_col()
                .child(render_top_bar(app, window, cx))
                .child(render_main_section(app, window, cx)),
        )
        // Layer 2: Floating Glass Bottom Transport Dock
        .child(render_floating_dock(app, cx))
        // Layer 3: Transient Toast Feedback
        .when_some(toast_text, |el, msg| {
            el.child(
                div()
                    .absolute()
                    .top(px(64.0))
                    .right(px(24.0))
                    .px_3p5()
                    .py_1p5()
                    .rounded_full()
                    .bg(theme::bg_surface_solid())
                    .border_1()
                    .border_color(theme::border_hover())
                    .shadow(theme::shadow_popover())
                    .text_xs()
                    .text_color(theme::ink_primary())
                    .child(msg),
            )
        })
        // Layer 4: Quick Tune Context Popover
        .when_some(context_menu, |el, menu| {
            el.child(render_effect_context_popover(app, menu, cx))
        })
}

fn render_top_bar(
    app: &FastRgbApp,
    window: &mut Window,
    cx: &mut Context<FastRgbApp>,
) -> impl IntoElement {
    let nav = app.nav_section;
    let is_live = app.hardware_mode == HardwareMode::LiveHardware;
    let has_warn = !is_live && app.hardware_report.external_rgb_running;
    let ambient = theme::from_core_rgb(app.ambient_rgb);
    let is_maximized = window.is_maximized();
    let shown_fps = if app.playing {
        app.fps_estimate.round() as u32
    } else {
        0
    };

    div()
        .h(px(58.0))
        .w_full()
        .pl_5()
        .pr_3()
        .flex()
        .items_center()
        .justify_between()
        .gap_3()
        // Left: Draggable Titlebar Zone + Official FastRGB Logo & Wordmark
        .child(
            div()
                .flex_1()
                .h_full()
                .flex()
                .items_center()
                .gap_2p5()
                .window_control_area(WindowControlArea::Drag)
                .child(
                    div()
                        .rounded(px(6.0))
                        .shadow(theme::shadow_glow(ambient.opacity(0.32), 14.0))
                        .child(render_fastrgb_logo(24.0)),
                )
                .child(
                    div()
                        .text_sm()
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(theme::ink_primary())
                        .child("FastRGB"),
                ),
        )
        // Center: Floating Glass Pill Nav (Library · Stage · Devices · Settings)
        .child(
            div()
                .flex_none()
                .p(px(4.0))
                .rounded_full()
                .bg(theme::bg_elevated())
                .border_1()
                .border_color(theme::border_hairline())
                .shadow(theme::shadow_dock())
                .flex()
                .items_center()
                .gap_1()
                .children(
                    [
                        (NavSection::Library, "Library", IconKind::Library),
                        (NavSection::Lighting, "Stage", IconKind::Stage),
                        (NavSection::Devices, "Devices", IconKind::Devices),
                        (NavSection::Settings, "Settings", IconKind::Settings),
                    ]
                    .into_iter()
                    .map(|(section, label, icon)| {
                        let active = nav == section;
                        let icon_color = if active {
                            theme::ink_primary()
                        } else {
                            theme::ink_secondary()
                        };
                        div()
                            .id(gpui::SharedString::from(format!("nav-pill-{label}")))
                            .px_4()
                            .py_1p5()
                            .rounded_full()
                            .cursor_pointer()
                            .bg(if active {
                                theme::bg_pill_active()
                            } else {
                                gpui::transparent_black()
                            })
                            .border_1()
                            .border_color(if active {
                                rgba(0xFFFFFF20).into()
                            } else {
                                gpui::transparent_black()
                            })
                            .flex()
                            .items_center()
                            .gap_2()
                            .text_xs()
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .text_color(icon_color)
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.set_nav_section(section, cx);
                            }))
                            .child(render_icon(icon, 14.0, icon_color))
                            .child(label)
                    }),
                ),
        )
        // Right: Draggable Spacer + Quiet Telemetry + Live Pill + Snapshot + Custom Window Controls
        .child(
            div()
                .flex_1()
                .h_full()
                .flex()
                .items_center()
                .justify_end()
                .gap_2()
                .child(
                    div()
                        .flex_1()
                        .h_full()
                        .window_control_area(WindowControlArea::Drag),
                )
                .child(
                    div()
                        .px_2p5()
                        .py_1()
                        .rounded_full()
                        .bg(rgba(0xFFFFFF08))
                        .border_1()
                        .border_color(theme::border_hairline())
                        .flex()
                        .items_center()
                        .gap_1()
                        .text_xs()
                        .child(
                            div()
                                .font_weight(gpui::FontWeight::MEDIUM)
                                .text_color(theme::ink_secondary())
                                .child(format!("{shown_fps}")),
                        )
                        .child(div().text_color(theme::ink_muted()).child("fps")),
                )
                .child({
                    let dot_color = if is_live {
                        theme::signal_live()
                    } else if has_warn {
                        theme::signal_warn()
                    } else {
                        theme::ink_muted()
                    };
                    let mut pill = div()
                        .id("top-hw-pill")
                        .px_3()
                        .py_1()
                        .rounded_full()
                        .cursor_pointer()
                        .bg(theme::bg_elevated())
                        .border_1()
                        .border_color(if is_live {
                            theme::signal_live().opacity(0.42)
                        } else {
                            theme::border_hairline()
                        })
                        .flex()
                        .items_center()
                        .gap_1p5()
                        .text_xs()
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .text_color(if is_live {
                            theme::ink_primary()
                        } else {
                            theme::ink_secondary()
                        })
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.toggle_hardware_mode(cx);
                        }));

                    if is_live {
                        pill = pill
                            .shadow(theme::shadow_glow(theme::signal_live().opacity(0.2), 18.0));
                    }

                    pill.child(div().size(px(6.0)).rounded_full().bg(dot_color))
                        .child(if is_live { "Live" } else { "Dry-Run" })
                })
                .child(
                    div()
                        .id("top-snap-btn")
                        .size(px(28.0))
                        .rounded_full()
                        .cursor_pointer()
                        .bg(theme::bg_elevated())
                        .border_1()
                        .border_color(theme::border_hairline())
                        .hover(|s| s.border_color(theme::border_hover()).bg(theme::bg_hover()))
                        .flex()
                        .items_center()
                        .justify_center()
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.capture_app_window_snapshot(window, cx);
                            this.push_toast("Saved window PNG to captures/", cx);
                        }))
                        .on_mouse_down(
                            MouseButton::Right,
                            cx.listener(|this, _, _, cx| {
                                this.capture_canvas_snapshot(cx);
                                this.push_toast("Saved RGB canvas PNG to captures/", cx);
                            }),
                        )
                        .child(render_icon(IconKind::Camera, 13.0, theme::ink_secondary())),
                )
                .child(
                    div()
                        .w(px(1.0))
                        .h(px(18.0))
                        .mx_0p5()
                        .bg(theme::border_hairline()),
                )
                // Custom Obsidian Window Controls (Minimize, Maximize/Restore, Close → System Tray)
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_1()
                        .child(
                            div()
                                .id("win-btn-min")
                                .size(px(28.0))
                                .rounded_full()
                                .cursor_pointer()
                                .hover(|s| s.bg(theme::bg_hover()))
                                .flex()
                                .items_center()
                                .justify_center()
                                .on_click(|_, window, _| {
                                    window.minimize_window();
                                })
                                .child(render_icon(
                                    IconKind::WinMinimize,
                                    13.0,
                                    theme::ink_secondary(),
                                )),
                        )
                        .child(
                            div()
                                .id("win-btn-max")
                                .size(px(28.0))
                                .rounded_full()
                                .cursor_pointer()
                                .hover(|s| s.bg(theme::bg_hover()))
                                .flex()
                                .items_center()
                                .justify_center()
                                .on_click(|_, window, _| {
                                    window.zoom_window();
                                })
                                .child(render_icon(
                                    if is_maximized {
                                        IconKind::WinRestore
                                    } else {
                                        IconKind::WinMaximize
                                    },
                                    12.0,
                                    theme::ink_secondary(),
                                )),
                        )
                        .child(
                            div()
                                .id("win-btn-close-tray")
                                .size(px(28.0))
                                .rounded_full()
                                .cursor_pointer()
                                .hover(|s| s.bg(rgba(0xF43F5E30)))
                                .flex()
                                .items_center()
                                .justify_center()
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.hide_to_tray(cx);
                                }))
                                .child(render_icon(
                                    IconKind::WinClose,
                                    13.0,
                                    theme::ink_secondary(),
                                )),
                        ),
                ),
        )
}

fn render_main_section(
    app: &mut FastRgbApp,
    window: &mut Window,
    cx: &mut Context<FastRgbApp>,
) -> impl IntoElement {
    match app.nav_section {
        NavSection::Library => library_panel::render_library_panel(app, cx).into_any_element(),
        NavSection::Devices => devices_panel::render_devices_view(app, cx).into_any_element(),
        NavSection::Lighting => render_stage_view(app, window, cx).into_any_element(),
        NavSection::Settings => settings_panel::render_settings_view(app, cx).into_any_element(),
    }
}

fn render_stage_view(
    app: &mut FastRgbApp,
    window: &mut Window,
    cx: &mut Context<FastRgbApp>,
) -> impl IntoElement {
    let active_meta = app.engine.active_metadata();
    let descriptors = app.engine.descriptors().to_vec();
    let params = app.engine.params().clone();
    let ambient = app.ambient_rgb;
    let inspector_open = app.inspector_open;
    let speed = app.engine.speed_multiplier();
    let fps_cap = app.target_fps_cap;
    let cat_label = match active_meta.category {
        EffectCategory::Basic => "Basic",
        EffectCategory::Static => "Static",
        EffectCategory::Dynamic => "Dynamic",
    };

    div()
        .flex_1()
        .min_h_0()
        .w_full()
        .px_6()
        .pt_1()
        .pb(px(86.0))
        .flex()
        .justify_center()
        .child(
            div()
                .w_full()
                .max_w(px(1380.0))
                .h_full()
                .min_h_0()
                .flex()
                .gap_5()
                // Left: Centered Hero Canvas + Single-Row LED Ribbons
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .h_full()
                        .flex()
                        .flex_col()
                        .items_center()
                        .justify_center()
                        .gap_4()
                        .child(canvas_view::render_canvas_stage(app, window, cx))
                        .child(devices_panel::render_stage_ribbons(app, cx)),
                )
                // Right: Unified Collapsible Glass Inspector
                .when(inspector_open, |el| {
                    el.child(
                        div()
                            .w(px(308.0))
                            .h_full()
                            .min_h_0()
                            .rounded(px(20.0))
                            .bg(theme::bg_elevated())
                            .border_1()
                            .border_color(theme::border_hairline())
                            .overflow_hidden()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .id("glass-inspector-scroll")
                                    .flex_1()
                                    .min_h_0()
                                    .overflow_y_scroll()
                                    .p_4()
                                    .flex()
                                    .flex_col()
                                    .gap_4()
                                    // Minimal Effect Header (Title + Meta + Reset, zero description)
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
                                                            .text_base()
                                                            .font_weight(gpui::FontWeight::SEMIBOLD)
                                                            .text_color(theme::ink_primary())
                                                            .child(active_meta.name),
                                                    )
                                                    .child(
                                                        div()
                                                            .text_xs()
                                                            .text_color(theme::ink_muted())
                                                            .child(format!(
                                                                "{cat_label} · {}",
                                                                active_meta.publisher
                                                            )),
                                                    ),
                                            )
                                            .child(
                                                div()
                                                    .id("insp-reset-btn")
                                                    .px_2p5()
                                                    .py_1()
                                                    .rounded_full()
                                                    .cursor_pointer()
                                                    .bg(theme::bg_hover())
                                                    .border_1()
                                                    .border_color(theme::border_hairline())
                                                    .text_xs()
                                                    .font_weight(gpui::FontWeight::MEDIUM)
                                                    .text_color(theme::ink_secondary())
                                                    .on_click(cx.listener(|this, _, _, cx| {
                                                        this.engine.reset_active();
                                                        this.step_simulation(1.0 / 60.0);
                                                        this.push_toast("Effect reset", cx);
                                                    }))
                                                    .child("Reset"),
                                            ),
                                    )
                                    .child(hairline())
                                    // Dynamic Effect Parameters (frameless inside single glass sheet)
                                    .when(!descriptors.is_empty(), |inner| {
                                        let mut children = Vec::with_capacity(descriptors.len());
                                        for desc in &descriptors {
                                            let value = params.get(&desc.id);
                                            children.push(controls::render_parameter_control(
                                                desc, value, ambient, cx,
                                            ));
                                        }
                                        inner
                                            .child(div().flex().flex_col().gap_3p5().children(children))
                                            .child(hairline())
                                    })
                                    // Compact Timing Block (Speed + FPS limit sliders only)
                                    .child(
                                        div()
                                            .flex()
                                            .flex_col()
                                            .gap_3()
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .font_weight(gpui::FontWeight::MEDIUM)
                                                    .text_color(theme::ink_muted())
                                                    .child("Timing"),
                                            )
                                            .child(
                                                div()
                                                    .flex()
                                                    .flex_col()
                                                    .gap_1()
                                                    .child(
                                                        div()
                                                            .flex()
                                                            .items_center()
                                                            .justify_between()
                                                            .text_xs()
                                                            .child(
                                                                div()
                                                                    .text_color(
                                                                        theme::ink_secondary(),
                                                                    )
                                                                    .child("Speed"),
                                                            )
                                                            .child(
                                                                div()
                                                                    .text_color(
                                                                        theme::ink_primary(),
                                                                    )
                                                                    .child(format!("{speed:.2}×")),
                                                            ),
                                                    )
                                                    .child(controls::render_range_slider(
                                                        "insp-speed-slider",
                                                        268.0,
                                                        ratio_from_speed(speed),
                                                        ambient,
                                                        SliderDragTarget::Speed(active_meta.id),
                                                        cx,
                                                    )),
                                            )
                                            .child(
                                                div()
                                                    .flex()
                                                    .flex_col()
                                                    .gap_1()
                                                    .child(
                                                        div()
                                                            .flex()
                                                            .items_center()
                                                            .justify_between()
                                                            .text_xs()
                                                            .child(
                                                                div()
                                                                    .text_color(
                                                                        theme::ink_secondary(),
                                                                    )
                                                                    .child("FPS limit"),
                                                            )
                                                            .child(
                                                                div()
                                                                    .text_color(
                                                                        theme::ink_primary(),
                                                                    )
                                                                    .child(fps_cap.map_or_else(
                                                                        || "60 fps".to_owned(),
                                                                        |f| format!("{f} fps"),
                                                                    )),
                                                            ),
                                                    )
                                                    .child(controls::render_range_slider(
                                                        "insp-fps-slider",
                                                        268.0,
                                                        ratio_from_fps_cap(fps_cap),
                                                        ambient,
                                                        SliderDragTarget::FpsCap(active_meta.id),
                                                        cx,
                                                    )),
                                            ),
                                    )
                                    // Contextual Screen Capture (only expanded when effect uses screen)
                                    .when(active_meta.uses_screen, |inner| {
                                        inner.child(hairline()).child(
                                            capture_panel::render_capture_panel(app, window, cx),
                                        )
                                    }),
                            ),
                    )
                }),
        )
}

fn render_floating_dock(app: &FastRgbApp, cx: &mut Context<FastRgbApp>) -> impl IntoElement {
    let meta = app.engine.active_metadata();
    let brightness = app.devices.global_brightness();
    let active_id = meta.id;
    let ambient = app.ambient_rgb;
    let thumb = app.thumbnail_for(meta.id);
    let (c0, c1) = library_panel::effect_card_gradient(meta.id);

    div()
        .absolute()
        .bottom(px(18.0))
        .left_0()
        .right_0()
        .flex()
        .justify_center()
        .child(
            div()
                .px_2p5()
                .py_1p5()
                .rounded_full()
                .bg(rgba(0x0D0E13DB))
                .border_1()
                .border_color(rgba(0xFFFFFF1C))
                .shadow(theme::shadow_dock())
                .flex()
                .items_center()
                .gap_3p5()
                // Active Effect Pill (click or right-click to Quick Tune)
                .child(
                    div()
                        .id("dock-now-btn")
                        .pr_2p5()
                        .pl_0p5()
                        .py_0p5()
                        .rounded_full()
                        .cursor_pointer()
                        .flex()
                        .items_center()
                        .gap_2p5()
                        .on_click(cx.listener(move |this, ev: &gpui::ClickEvent, _, cx| {
                            this.open_effect_context_menu(active_id, ev.position(), cx);
                        }))
                        .on_mouse_down(
                            MouseButton::Right,
                            cx.listener(move |this, ev: &MouseDownEvent, _, cx| {
                                this.open_effect_context_menu(active_id, ev.position, cx);
                            }),
                        )
                        .child(
                            div()
                                .size(px(30.0))
                                .rounded_full()
                                .overflow_hidden()
                                .border_1()
                                .border_color(rgba(0xFFFFFF2E))
                                .bg(linear_gradient(
                                    135.0,
                                    linear_color_stop(rgb(c0), 0.0),
                                    linear_color_stop(rgb(c1), 1.0),
                                ))
                                .when_some(thumb, |el, path| {
                                    el.child(
                                        img(path)
                                            .size_full()
                                            .rounded_full()
                                            .object_fit(ObjectFit::Cover),
                                    )
                                }),
                        )
                        .child(
                            div()
                                .max_w(px(136.0))
                                .truncate()
                                .text_xs()
                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                .text_color(theme::ink_primary())
                                .child(meta.name),
                        ),
                )
                .child(dock_separator())
                // Iconic Transport Cluster
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_1()
                        .child(dock_icon_btn(
                            "dock-prev",
                            IconKind::Prev,
                            cx.listener(|this, _, _, cx| this.step_effect(-1, cx)),
                        ))
                        .child(
                            div()
                                .id("dock-play-pause")
                                .size(px(36.0))
                                .rounded_full()
                                .cursor_pointer()
                                .bg(theme::ink_primary())
                                .flex()
                                .items_center()
                                .justify_center()
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.toggle_playback(cx);
                                }))
                                .child(render_icon(
                                    if app.playing {
                                        IconKind::Pause
                                    } else {
                                        IconKind::Play
                                    },
                                    15.0,
                                    theme::bg_void(),
                                )),
                        )
                        .child(dock_icon_btn(
                            "dock-next",
                            IconKind::Next,
                            cx.listener(|this, _, _, cx| this.step_effect(1, cx)),
                        ))
                        .child(dock_icon_btn(
                            "dock-reset",
                            IconKind::Reset,
                            cx.listener(|this, _, _, cx| {
                                this.engine.reset_active();
                                this.step_simulation(1.0 / 60.0);
                                this.push_toast("Simulation reset", cx);
                            }),
                        )),
                )
                .child(dock_separator())
                // Brightness Slider + Quick Tune Trigger
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .pl_1()
                        .child(render_icon(IconKind::Sun, 14.0, theme::ink_muted()))
                        .child(controls::render_range_slider(
                            "dock-bright-slider",
                            84.0,
                            ((brightness - 0.05) / 0.95).clamp(0.0, 1.0),
                            ambient,
                            SliderDragTarget::Brightness(active_id),
                            cx,
                        ))
                        .child(
                            div()
                                .w(px(34.0))
                                .text_right()
                                .text_xs()
                                .text_color(theme::ink_secondary())
                                .child(format!("{:.0}%", brightness * 100.0)),
                        )
                        .child(
                            div()
                                .id("dock-tune-btn")
                                .size(px(30.0))
                                .rounded_full()
                                .cursor_pointer()
                                .bg(theme::bg_elevated())
                                .border_1()
                                .border_color(theme::border_hairline())
                                .flex()
                                .items_center()
                                .justify_center()
                                .on_click(cx.listener(move |this, ev: &gpui::ClickEvent, _, cx| {
                                    if this.context_menu.is_some() {
                                        this.close_context_menu(cx);
                                    } else {
                                        this.open_effect_context_menu(active_id, ev.position(), cx);
                                    }
                                }))
                                .child(render_icon(
                                    IconKind::Sliders,
                                    14.0,
                                    theme::ink_secondary(),
                                )),
                        ),
                ),
        )
}

fn dock_icon_btn(
    id: &'static str,
    icon: IconKind,
    on_click: impl Fn(&gpui::ClickEvent, &mut Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    div()
        .id(id)
        .size(px(32.0))
        .rounded_full()
        .cursor_pointer()
        .flex()
        .items_center()
        .justify_center()
        .on_click(on_click)
        .child(render_icon(icon, 15.0, theme::ink_secondary()))
}

fn dock_separator() -> impl IntoElement {
    div().w(px(1.0)).h(px(20.0)).bg(theme::border_hairline())
}

fn hairline() -> impl IntoElement {
    div().w_full().h(px(1.0)).bg(theme::border_hairline())
}

fn render_effect_context_popover(
    app: &FastRgbApp,
    menu: EffectContextMenu,
    cx: &mut Context<FastRgbApp>,
) -> impl IntoElement {
    let preset = app.effect_preset(menu.effect_id);
    let is_active = app.engine.active_metadata().id == menu.effect_id;
    let id = menu.effect_id;
    let ambient = app.ambient_rgb;

    deferred(
        anchored()
            .position(menu.position)
            .snap_to_window_with_margin(px(12.0))
            .child(
                div()
                    .id("effect-context-menu")
                    .w(px(280.0))
                    .p_4()
                    .flex()
                    .flex_col()
                    .gap_3p5()
                    .rounded(px(14.0))
                    .bg(rgba(0x0D0E13EB))
                    .border_1()
                    .border_color(theme::border_hover())
                    .shadow(theme::shadow_popover())
                    .on_mouse_down_out(cx.listener(|this, _, _, cx| {
                        if this.dragging_slider.is_none() {
                            this.close_context_menu(cx);
                        }
                    }))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap_2()
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_color(theme::ink_primary())
                                    .child(menu.effect_name),
                            )
                            .child(
                                div()
                                    .id("ctx-activate-btn")
                                    .px_2p5()
                                    .py_0p5()
                                    .rounded_full()
                                    .cursor_pointer()
                                    .border_1()
                                    .border_color(if is_active {
                                        theme::signal_live().opacity(0.45)
                                    } else {
                                        gpui::transparent_black()
                                    })
                                    .bg(if is_active {
                                        theme::signal_live().opacity(0.16)
                                    } else {
                                        theme::ink_primary()
                                    })
                                    .text_xs()
                                    .font_weight(gpui::FontWeight::MEDIUM)
                                    .text_color(if is_active {
                                        theme::signal_live()
                                    } else {
                                        theme::bg_void()
                                    })
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        this.select_effect(id, cx);
                                        this.close_context_menu(cx);
                                    }))
                                    .child(if is_active { "Active" } else { "Activate" }),
                            ),
                    )
                    // Speed
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1p5()
                            .child(
                                div()
                                    .flex()
                                    .justify_between()
                                    .text_xs()
                                    .child(div().text_color(theme::ink_secondary()).child("Speed"))
                                    .child(
                                        div()
                                            .text_color(theme::ink_primary())
                                            .child(format!("{:.2}×", preset.speed)),
                                    ),
                            )
                            .child(controls::render_range_slider(
                                "ctx-speed-range",
                                246.0,
                                ratio_from_speed(preset.speed),
                                ambient,
                                SliderDragTarget::Speed(id),
                                cx,
                            ))
                            .child(div().flex().gap_1().children(
                                [0.15f32, 0.5, 1.0, 2.0].into_iter().map(|spd| {
                                    let selected = (preset.speed - spd).abs() < 0.03;
                                    div()
                                        .id(gpui::SharedString::from(format!("ctx-spd-{spd}")))
                                        .flex_1()
                                        .py_0p5()
                                        .text_center()
                                        .rounded(px(6.0))
                                        .cursor_pointer()
                                        .border_1()
                                        .border_color(if selected {
                                            theme::border_active()
                                        } else {
                                            theme::border_hairline()
                                        })
                                        .bg(if selected {
                                            theme::bg_pill_active()
                                        } else {
                                            theme::bg_hover()
                                        })
                                        .text_xs()
                                        .text_color(if selected {
                                            theme::ink_primary()
                                        } else {
                                            theme::ink_muted()
                                        })
                                        .on_click(cx.listener(move |this, _, _, cx| {
                                            this.set_effect_speed(id, spd, cx);
                                        }))
                                        .child(format!("{spd}×"))
                                }),
                            )),
                    )
                    // FPS Cap
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1p5()
                            .child(
                                div()
                                    .flex()
                                    .justify_between()
                                    .text_xs()
                                    .child(
                                        div().text_color(theme::ink_secondary()).child("FPS cap"),
                                    )
                                    .child(div().text_color(theme::ink_primary()).child(
                                        preset.fps_cap.map_or_else(
                                            || "60 fps".to_owned(),
                                            |f| format!("{f} fps"),
                                        ),
                                    )),
                            )
                            .child(controls::render_range_slider(
                                "ctx-fps-range",
                                246.0,
                                ratio_from_fps_cap(preset.fps_cap),
                                ambient,
                                SliderDragTarget::FpsCap(id),
                                cx,
                            ))
                            .child(
                                div().flex().gap_1().children(
                                    [
                                        (Some(15), "15"),
                                        (Some(30), "30"),
                                        (Some(45), "45"),
                                        (None, "60"),
                                    ]
                                    .into_iter()
                                    .map(|(cap, label)| {
                                        let selected = preset.fps_cap == cap;
                                        div()
                                            .id(gpui::SharedString::from(format!(
                                                "ctx-fps-{label}"
                                            )))
                                            .flex_1()
                                            .py_0p5()
                                            .text_center()
                                            .rounded(px(6.0))
                                            .cursor_pointer()
                                            .border_1()
                                            .border_color(if selected {
                                                theme::border_active()
                                            } else {
                                                theme::border_hairline()
                                            })
                                            .bg(if selected {
                                                theme::bg_pill_active()
                                            } else {
                                                theme::bg_hover()
                                            })
                                            .text_xs()
                                            .text_color(if selected {
                                                theme::ink_primary()
                                            } else {
                                                theme::ink_muted()
                                            })
                                            .on_click(cx.listener(move |this, _, _, cx| {
                                                this.set_effect_fps_cap(id, cap, cx);
                                            }))
                                            .child(label)
                                    }),
                                ),
                            ),
                    )
                    // Brightness
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(
                                div()
                                    .flex()
                                    .justify_between()
                                    .text_xs()
                                    .child(
                                        div()
                                            .text_color(theme::ink_secondary())
                                            .child("Brightness"),
                                    )
                                    .child(
                                        div()
                                            .text_color(theme::ink_primary())
                                            .child(format!("{:.0}%", preset.brightness * 100.0)),
                                    ),
                            )
                            .child(controls::render_range_slider(
                                "ctx-bright-range",
                                246.0,
                                ((preset.brightness - 0.05) / 0.95).clamp(0.0, 1.0),
                                ambient,
                                SliderDragTarget::Brightness(id),
                                cx,
                            )),
                    ),
            ),
    )
    .with_priority(10)
}
