//! Visual-first minimalist `Library` effect grid inspired by React Bits `SpotlightCard`.

use std::collections::HashMap;
use std::path::PathBuf;

use fastrgb_effects::{EffectCategory, EffectMetadata};
use gpui::{
    Context, MouseButton, MouseDownEvent, ObjectFit, div, img, linear_color_stop, linear_gradient,
    prelude::*, px, rgb, rgba,
};

use crate::state::{FastRgbApp, LibraryFilter, NavSection};
use crate::ui::theme;

/// Number of minimalist effect cards shown per page (`3×4` visual gallery).
pub const LIBRARY_PAGE_SIZE: usize = 12;

/// Pre-resolves local thumbnail `.png` paths once at startup to avoid per-frame filesystem stats.
#[must_use]
pub fn build_thumbnail_cache(effects: &[EffectMetadata]) -> HashMap<&'static str, PathBuf> {
    let mut cache = HashMap::with_capacity(effects.len());
    for &meta in effects {
        if let Some(path) = resolve_effect_thumbnail(meta) {
            cache.insert(meta.id, path);
        }
    }
    cache
}

/// Resolves an optional local thumbnail `.png` path for an effect card.
pub fn resolve_effect_thumbnail(meta: EffectMetadata) -> Option<PathBuf> {
    let cat_dir = match meta.category {
        EffectCategory::Basic => "Basic",
        EffectCategory::Static => "Static",
        EffectCategory::Dynamic => "Dynamic",
    };

    let mut roots: Vec<PathBuf> = Vec::new();
    if let Ok(env_dir) = std::env::var("FASTRGB_THUMBNAILS_DIR") {
        roots.push(PathBuf::from(env_dir));
    }
    if let Ok(exe) = std::env::current_exe()
        && let Some(dir) = exe.parent()
    {
        roots.push(dir.join("thumbnails"));
    }
    roots.push(PathBuf::from("assets/thumbnails"));

    for root in &roots {
        let categorized = root.join(cat_dir).join(format!("{}.png", meta.name));
        if categorized.is_file() {
            return Some(categorized);
        }
        let flat = root.join(format!("{}.png", meta.name));
        if flat.is_file() {
            return Some(flat);
        }
    }
    None
}

/// Renders the visual-first `Library` view: single-line pill filters + zero-paragraph effect cards.
pub fn render_library_panel(app: &FastRgbApp, cx: &mut Context<FastRgbApp>) -> impl IntoElement {
    let active_id = app.engine.active_metadata().id;
    let ambient = theme::from_core_rgb(app.ambient_rgb);
    let filter = app.library_filter;

    let count_all = app.effects_list.len();
    let count_basic = app
        .effects_list
        .iter()
        .filter(|m| m.category == EffectCategory::Basic)
        .count();
    let count_dynamic = app
        .effects_list
        .iter()
        .filter(|m| m.category == EffectCategory::Dynamic)
        .count();
    let count_static = app
        .effects_list
        .iter()
        .filter(|m| m.category == EffectCategory::Static)
        .count();

    let filtered: Vec<_> = app
        .effects_list
        .iter()
        .copied()
        .filter(|meta| match filter {
            LibraryFilter::All => true,
            LibraryFilter::Static => meta.category == EffectCategory::Static,
            LibraryFilter::Basic => meta.category == EffectCategory::Basic,
            LibraryFilter::Dynamic => meta.category == EffectCategory::Dynamic,
        })
        .collect();

    let total_filtered = filtered.len();
    let total_pages = total_filtered.div_ceil(LIBRARY_PAGE_SIZE).max(1);
    let current_page = app.library_page.min(total_pages - 1);
    let start = current_page * LIBRARY_PAGE_SIZE;
    let end = (start + LIBRARY_PAGE_SIZE).min(total_filtered);
    let visible_effects = &filtered[start..end];

    div()
        .id("library-scroll")
        .flex_1()
        .min_h_0()
        .overflow_y_scroll()
        .px_6()
        .pb(px(96.0))
        .pt_1()
        .child(
            div()
                .w_full()
                .max_w(px(1240.0))
                .mx_auto()
                .flex()
                .flex_col()
                .gap_4()
                .child(
                    // Single quiet toolbar row (no duplicated title or bottom pagination)
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap_3()
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_1()
                                .p(px(3.0))
                                .rounded_full()
                                .bg(theme::bg_elevated())
                                .border_1()
                                .border_color(theme::border_hairline())
                                .children(
                                    [
                                        (LibraryFilter::All, "All", count_all),
                                        (LibraryFilter::Basic, "Basic", count_basic),
                                        (LibraryFilter::Dynamic, "Dynamic", count_dynamic),
                                        (LibraryFilter::Static, "Static", count_static),
                                    ]
                                    .into_iter()
                                    .map(
                                        |(kind, label, count)| {
                                            let is_sel = filter == kind;
                                            div()
                                                .id(gpui::SharedString::from(format!(
                                                    "lib-filter-{label}"
                                                )))
                                                .px_3()
                                                .py_1()
                                                .rounded_full()
                                                .cursor_pointer()
                                                .bg(if is_sel {
                                                    theme::bg_pill_active()
                                                } else {
                                                    gpui::transparent_black()
                                                })
                                                .flex()
                                                .items_center()
                                                .gap_1p5()
                                                .text_xs()
                                                .font_weight(gpui::FontWeight::MEDIUM)
                                                .on_click(cx.listener(move |this, _, _, cx| {
                                                    this.library_filter = kind;
                                                    this.library_page = 0;
                                                    cx.notify();
                                                }))
                                                .child(
                                                    div()
                                                        .text_color(if is_sel {
                                                            theme::ink_primary()
                                                        } else {
                                                            theme::ink_secondary()
                                                        })
                                                        .child(label),
                                                )
                                                .child(
                                                    div()
                                                        .text_color(if is_sel {
                                                            theme::ink_secondary()
                                                        } else {
                                                            theme::ink_muted()
                                                        })
                                                        .child(format!("{count}")),
                                                )
                                        },
                                    ),
                                ),
                        )
                        .when(total_pages > 1, |el| {
                            el.child(render_compact_pagination(current_page, total_pages, cx))
                        }),
                )
                .child(div().flex().flex_wrap().gap_3p5().children(
                    visible_effects.iter().copied().map(|meta| {
                        let is_active = meta.id == active_id;
                        let (c_start, c_end) = effect_card_gradient(meta.id);
                        let thumb_path = app.thumbnail_for(meta.id);
                        let preset = app.effect_preset(meta.id);
                        let has_custom = (preset.speed - 1.0).abs() > 0.02
                            || (preset.brightness - 1.0).abs() > 0.02
                            || preset.fps_cap.is_some();

                        let group_name = gpui::SharedString::from(format!("fx-card-{}", meta.id));
                        let mut card = div()
                            .id(gpui::SharedString::from(format!("lib-card-{}", meta.id)))
                            .group(group_name.clone())
                            .w(px(286.0))
                            .flex()
                            .flex_col()
                            .rounded(px(14.0))
                            .overflow_hidden()
                            .cursor_pointer()
                            .bg(theme::bg_surface())
                            .border_1()
                            .border_color(if is_active {
                                ambient.opacity(0.55)
                            } else {
                                theme::border_hairline()
                            })
                            .hover(|s| s.border_color(theme::border_hover()))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.select_effect(meta.id, cx);
                            }))
                            .on_mouse_down(
                                MouseButton::Right,
                                cx.listener(move |this, event: &MouseDownEvent, _, cx| {
                                    this.open_effect_context_menu(meta.id, event.position, cx);
                                }),
                            );

                        if is_active {
                            card = card.shadow(theme::shadow_glow(ambient.opacity(0.24), 22.0));
                        }

                        card.child(
                            div()
                                .relative()
                                .h(px(154.0))
                                .w_full()
                                .overflow_hidden()
                                .bg(linear_gradient(
                                    135.0,
                                    linear_color_stop(rgb(c_start), 0.0),
                                    linear_color_stop(rgb(c_end), 1.0),
                                ))
                                .when_some(thumb_path, |el, path| {
                                    el.child(
                                        img(path)
                                            .absolute()
                                            .inset_0()
                                            .size_full()
                                            .object_fit(ObjectFit::Cover),
                                    )
                                })
                                .child(div().absolute().inset_0().bg(linear_gradient(
                                    180.0,
                                    linear_color_stop(gpui::transparent_black(), 0.48),
                                    linear_color_stop(rgba(0x0C0D12DB), 1.0),
                                )))
                                .when(has_custom, |el| {
                                    el.child(
                                        div()
                                            .absolute()
                                            .top(px(9.0))
                                            .left(px(9.0))
                                            .px_2()
                                            .py_0p5()
                                            .rounded_full()
                                            .bg(rgba(0x06070AB2))
                                            .border_1()
                                            .border_color(theme::border_hairline())
                                            .text_xs()
                                            .text_color(theme::ink_secondary())
                                            .child(format!("{:.1}×", preset.speed)),
                                    )
                                })
                                .child(
                                    div()
                                        .id(gpui::SharedString::from(format!(
                                            "lib-edit-{}",
                                            meta.id
                                        )))
                                        .absolute()
                                        .top(px(9.0))
                                        .right(px(9.0))
                                        .px_2p5()
                                        .py_0p5()
                                        .rounded_full()
                                        .cursor_pointer()
                                        .bg(rgba(0x06070AB8))
                                        .border_1()
                                        .border_color(rgba(0xFFFFFF22))
                                        .text_xs()
                                        .font_weight(gpui::FontWeight::MEDIUM)
                                        .text_color(theme::ink_primary())
                                        .opacity(0.0)
                                        .group_hover(group_name, |s| s.opacity(1.0))
                                        .on_click(cx.listener(move |this, _, _, cx| {
                                            this.select_effect(meta.id, cx);
                                            this.set_nav_section(NavSection::Lighting, cx);
                                        }))
                                        .child("Edit"),
                                ),
                        )
                        .child(
                            // Zero-verbiage footer: effect name + active light pip only
                            div()
                                .px_3()
                                .py_2p5()
                                .flex()
                                .items_center()
                                .justify_between()
                                .gap_2()
                                .child(
                                    div()
                                        .text_sm()
                                        .font_weight(gpui::FontWeight::MEDIUM)
                                        .text_color(theme::ink_primary())
                                        .truncate()
                                        .child(meta.name),
                                )
                                .when(is_active, |el| {
                                    el.child(
                                        div()
                                            .size(px(7.0))
                                            .rounded_full()
                                            .bg(ambient)
                                            .shadow(theme::shadow_glow(ambient, 8.0)),
                                    )
                                }),
                        )
                    }),
                )),
        )
}

fn render_compact_pagination(
    current_page: usize,
    total_pages: usize,
    cx: &mut Context<FastRgbApp>,
) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .gap_1()
        .p(px(3.0))
        .rounded_full()
        .bg(theme::bg_elevated())
        .border_1()
        .border_color(theme::border_hairline())
        .child(
            div()
                .id("lib-page-prev")
                .px_2()
                .py_0p5()
                .rounded_full()
                .cursor_pointer()
                .text_xs()
                .text_color(if current_page > 0 {
                    theme::ink_primary()
                } else {
                    theme::ink_muted()
                })
                .on_click(cx.listener(move |this, _, _, cx| {
                    if this.library_page > 0 {
                        this.library_page -= 1;
                        cx.notify();
                    }
                }))
                .child("‹"),
        )
        .children((0..total_pages).map(|page| {
            let active = page == current_page;
            div()
                .id(gpui::SharedString::from(format!("lib-page-num-{page}")))
                .px_2()
                .py_0p5()
                .rounded_full()
                .cursor_pointer()
                .bg(if active {
                    theme::bg_pill_active()
                } else {
                    gpui::transparent_black()
                })
                .text_xs()
                .text_color(if active {
                    theme::ink_primary()
                } else {
                    theme::ink_muted()
                })
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.library_page = page;
                    cx.notify();
                }))
                .child(format!("{}", page + 1))
        }))
        .child(
            div()
                .id("lib-page-next")
                .px_2()
                .py_0p5()
                .rounded_full()
                .cursor_pointer()
                .text_xs()
                .text_color(if current_page + 1 < total_pages {
                    theme::ink_primary()
                } else {
                    theme::ink_muted()
                })
                .on_click(cx.listener(move |this, _, _, cx| {
                    if this.library_page + 1 < total_pages {
                        this.library_page += 1;
                        cx.notify();
                    }
                }))
                .child("›"),
        )
}

/// Returns a signature two-stop procedural gradient fallback for an effect card or dock swatch.
#[must_use]
pub const fn effect_card_gradient(id: &str) -> (u32, u32) {
    match id.as_bytes() {
        b"aurora" => (0x00C9A7, 0x0EA5E9),
        b"black_hole" => (0x8B5CF6, 0xF43F5E),
        b"bouncing_ball" => (0xEF4444, 0xF59E0B),
        b"bubbles" => (0xEC4899, 0x6366F1),
        b"fireflies" => (0x10B981, 0x84CC16),
        b"good_night" => (0x1E293B, 0x090D16),
        b"gradient_pinwheel" => (0xF97316, 0x9333EA),
        b"lava_lamp" => (0xF43F5E, 0xFB923C),
        b"matrix" => (0x22C55E, 0x052E16),
        b"neon_shift" => (0xA855F7, 0xEC4899),
        b"neon_sunset" | b"neon_sunset_wave" => (0xF97316, 0xEC4899),
        b"plasma" => (0x38BDF8, 0xA855F7),
        b"rainbow" | b"rainbow_data" => (0xEC4899, 0x3B82F6),
        b"sakura" => (0xF472B6, 0xA78BFA),
        b"screen_ambience" => (0x0EA5E9, 0x6366F1),
        b"side_to_side" => (0x2563EB, 0xD946EF),
        b"spin" => (0x3B82F6, 0x06B6D4),
        b"tv_static" => (0x475569, 0x94A3B8),
        b"vapor_wave" => (0x22D3EE, 0xE879F9),
        b"voronoi" => (0x991B1B, 0x0F766E),
        _ => (0x0284C7, 0x10B981),
    }
}
