//! Minimalist dynamic parameter UI controls driven strictly by [`ParameterDescriptor`] metadata.

use std::cell::Cell;
use std::rc::Rc;

use fastrgb_core::{ParameterDescriptor, ParameterKind, ParameterValue, Rgb};
use gpui::{
    Bounds, Context, MouseButton, MouseDownEvent, Pixels, canvas, div, prelude::*, px, rgba,
};

use crate::state::{FastRgbApp, SliderDragTarget};
use crate::ui::theme;

const COLOR_SWATCHES: [Rgb; 8] = [
    Rgb::new(0x00, 0xBB, 0xFF),
    Rgb::new(0xEC, 0x48, 0x99),
    Rgb::new(0x63, 0x66, 0xF1),
    Rgb::new(0x10, 0xB9, 0x81),
    Rgb::new(0xF5, 0x9E, 0x0B),
    Rgb::new(0xEF, 0x44, 0x44),
    Rgb::new(0xA8, 0x55, 0xF7),
    Rgb::new(0xFF, 0xFF, 0xFF),
];

/// Renders a sleek click-and-drag minimalist horizontal range slider with dynamic ambient fill.
pub fn render_range_slider(
    id: impl Into<gpui::SharedString>,
    width: f32,
    ratio: f32,
    ambient: Rgb,
    target: SliderDragTarget,
    cx: &mut Context<FastRgbApp>,
) -> impl IntoElement {
    let ratio = ratio.clamp(0.0, 1.0);
    let track_h = 4.0_f32;
    let thumb_size = 12.0_f32;
    let hit_h = 20.0_f32;
    let usable_span = (width - thumb_size).max(1.0);
    let thumb_left = usable_span * ratio;
    let fill_w = (thumb_left + thumb_size * 0.5).clamp(0.0, width);
    let bounds_cell: Rc<Cell<Bounds<Pixels>>> = Rc::new(Cell::new(Bounds::default()));
    let bounds_prepaint = Rc::clone(&bounds_cell);
    let bounds_click = Rc::clone(&bounds_cell);

    div()
        .id(id.into())
        .relative()
        .w(px(width))
        .h(px(hit_h))
        .flex()
        .items_center()
        .cursor_pointer()
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |this, ev: &MouseDownEvent, _, cx| {
                this.begin_slider_drag(target.clone(), bounds_click.get(), ev.position.x, cx);
            }),
        )
        .child(
            div()
                .relative()
                .w_full()
                .h(px(track_h))
                .rounded_full()
                .overflow_hidden()
                .bg(rgba(0xFFFFFF1A))
                .child(
                    div()
                        .h_full()
                        .w(px(fill_w))
                        .bg(theme::from_core_rgb(ambient)),
                ),
        )
        .child(
            div()
                .absolute()
                .top(px((hit_h - thumb_size) * 0.5))
                .left(px(thumb_left))
                .w(px(thumb_size))
                .h(px(thumb_size))
                .rounded_full()
                .bg(gpui::white())
                .border_2()
                .border_color(theme::bg_surface_solid()),
        )
        .child(
            canvas(
                move |bounds, _window, _cx| {
                    bounds_prepaint.set(bounds);
                },
                |_bounds, (), _window, _cx| {},
            )
            .absolute()
            .inset_0()
            .size_full(),
        )
}

/// Renders a frameless minimalist control for any [`ParameterDescriptor`] inside the Glass Inspector.
pub fn render_parameter_control(
    desc: &ParameterDescriptor,
    value: Option<&ParameterValue>,
    ambient: Rgb,
    cx: &mut Context<FastRgbApp>,
) -> impl IntoElement + use<> {
    let id = desc.id.clone();
    let label = desc.label.clone();

    match &desc.kind {
        ParameterKind::Number {
            min,
            max,
            step,
            default,
        } => {
            let cur = value.and_then(ParameterValue::as_f32).unwrap_or(*default);
            let span = (max - min).max(0.001);
            let ratio = ((cur - min) / span).clamp(0.0, 1.0);
            let val_text = if *step < 1.0 {
                format!("{cur:.1}")
            } else {
                format!("{cur:.0}")
            };

            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .child(
                            div()
                                .text_xs()
                                .text_color(theme::ink_secondary())
                                .child(label),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(theme::ink_primary())
                                .child(val_text),
                        ),
                )
                .child(render_range_slider(
                    format!("param-slider-{id}"),
                    268.0,
                    ratio,
                    ambient,
                    SliderDragTarget::Parameter {
                        id,
                        min: *min,
                        max: *max,
                        step: *step,
                    },
                    cx,
                ))
                .into_any_element()
        }
        ParameterKind::Boolean { default } => {
            let cur = value.and_then(ParameterValue::as_bool).unwrap_or(*default);
            render_boolean_row(id, label, cur, ambient, cx).into_any_element()
        }
        ParameterKind::Combobox { options, default } => {
            let cur = value
                .and_then(ParameterValue::as_str)
                .unwrap_or(default.as_str())
                .to_owned();
            render_segmented_control(id, label, options, &cur, cx).into_any_element()
        }
        ParameterKind::Color { default } => {
            let cur = value.and_then(ParameterValue::as_color).unwrap_or(*default);
            render_color_swatches(id, label, cur, cx).into_any_element()
        }
    }
}

fn render_boolean_row(
    param_id: String,
    label: String,
    current: bool,
    ambient: Rgb,
    cx: &mut Context<FastRgbApp>,
) -> impl IntoElement {
    let amb = theme::from_core_rgb(ambient);
    div()
        .id(gpui::SharedString::from(format!("bool-{param_id}")))
        .flex()
        .items_center()
        .justify_between()
        .py_0p5()
        .cursor_pointer()
        .on_click(cx.listener(move |this, _, _, cx| {
            this.update_parameter(&param_id, ParameterValue::Boolean(!current), cx);
        }))
        .child(
            div()
                .text_xs()
                .text_color(theme::ink_secondary())
                .child(label),
        )
        .child(
            div()
                .relative()
                .w(px(34.0))
                .h(px(19.0))
                .rounded_full()
                .border_1()
                .border_color(if current {
                    amb.opacity(0.55)
                } else {
                    theme::border_hairline()
                })
                .bg(if current {
                    amb.opacity(0.18)
                } else {
                    rgba(0xFFFFFF18).into()
                })
                .child(
                    div()
                        .absolute()
                        .top(px(2.0))
                        .left(px(if current { 17.0 } else { 2.0 }))
                        .size(px(13.0))
                        .rounded_full()
                        .bg(if current {
                            gpui::white()
                        } else {
                            theme::ink_secondary()
                        }),
                ),
        )
}

fn render_segmented_control(
    param_id: String,
    label: String,
    options: &[String],
    current: &str,
    cx: &mut Context<FastRgbApp>,
) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_1p5()
        .child(
            div()
                .text_xs()
                .text_color(theme::ink_secondary())
                .child(label),
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
                .children(options.iter().enumerate().map(|(idx, opt)| {
                    let opt_owned = opt.clone();
                    let id_clone = param_id.clone();
                    let is_selected = opt == current;
                    div()
                        .id(gpui::SharedString::from(format!("combo-{param_id}-{idx}")))
                        .flex_1()
                        .px_2()
                        .py_1()
                        .rounded(px(7.0))
                        .text_center()
                        .cursor_pointer()
                        .bg(if is_selected {
                            theme::bg_pill_active()
                        } else {
                            gpui::transparent_black()
                        })
                        .text_xs()
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .text_color(if is_selected {
                            theme::ink_primary()
                        } else {
                            theme::ink_secondary()
                        })
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.update_parameter(
                                &id_clone,
                                ParameterValue::Combobox(opt_owned.clone()),
                                cx,
                            );
                        }))
                        .child(opt.clone())
                })),
        )
}

fn render_color_swatches(
    param_id: String,
    label: String,
    current: Rgb,
    cx: &mut Context<FastRgbApp>,
) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .text_xs()
                        .text_color(theme::ink_secondary())
                        .child(label),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(theme::ink_primary())
                        .child(current.to_hex()),
                ),
        )
        .child(
            div()
                .flex()
                .flex_wrap()
                .gap_2()
                .children(COLOR_SWATCHES.iter().enumerate().map(|(idx, &swatch)| {
                    let id_clone = param_id.clone();
                    let is_active = swatch == current;
                    div()
                        .id(gpui::SharedString::from(format!("swatch-{param_id}-{idx}")))
                        .size(px(20.0))
                        .rounded_full()
                        .cursor_pointer()
                        .border_2()
                        .border_color(if is_active {
                            gpui::white()
                        } else {
                            gpui::transparent_black()
                        })
                        .bg(theme::from_core_rgb(swatch))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.update_parameter(&id_clone, ParameterValue::Color(swatch), cx);
                        }))
                })),
        )
}
