//! Obsidian Instrument design tokens — achromatic obsidian surfaces that let active RGB light lead.

use fastrgb_core::Rgb;
use gpui::{BoxShadow, Hsla, point, px, rgb, rgba};

/// Absolute obsidian void backdrop (`#050507`) hosting the interactive `DotField`.
#[must_use]
pub fn bg_void() -> Hsla {
    rgb(0x050507).into()
}

/// Floating glass bar / inspector surface (`rgba(15, 16, 21, 0.86)`).
#[must_use]
pub fn bg_elevated() -> Hsla {
    rgba(0x0F1015DB).into()
}

/// Matte translucent card surface (`rgba(19, 21, 27, 0.78)`).
#[must_use]
pub fn bg_surface() -> Hsla {
    rgba(0x13151BC7).into()
}

/// Solid dark popover / thumb-border surface (`#0F1015`).
#[must_use]
pub fn bg_surface_solid() -> Hsla {
    rgb(0x0F1015).into()
}

/// Deeply recessed well for slider tracks, segmented pills, and LED matrices (`rgba(6, 7, 10, 0.86)`).
#[must_use]
pub fn bg_well() -> Hsla {
    rgba(0x06070ADB).into()
}

/// Subtle translucent hover fill (`rgba(255, 255, 255, 0.06)`).
#[must_use]
pub fn bg_hover() -> Hsla {
    rgba(0xFFFFFF0F).into()
}

/// Active pill fill (`rgba(255, 255, 255, 0.11)`).
#[must_use]
pub fn bg_pill_active() -> Hsla {
    rgba(0xFFFFFF1C).into()
}

/// Quiet 1px structural hairline (`rgba(255, 255, 255, 0.08)`).
#[must_use]
pub fn border_hairline() -> Hsla {
    rgba(0xFFFFFF14).into()
}

/// Emphasized border for hovered containers (`rgba(255, 255, 255, 0.17)`).
#[must_use]
pub fn border_hover() -> Hsla {
    rgba(0xFFFFFF2B).into()
}

/// Crisp active border (`rgba(255, 255, 255, 0.28)`).
#[must_use]
pub fn border_active() -> Hsla {
    rgba(0xFFFFFF47).into()
}

/// High-contrast primary foreground (`#F4F5F7`).
#[must_use]
pub fn ink_primary() -> Hsla {
    rgb(0xF4F5F7).into()
}

/// Readable secondary foreground (`#8C91A0`).
#[must_use]
pub fn ink_secondary() -> Hsla {
    rgb(0x8C91A0).into()
}

/// Quiet metadata & unit foreground (`#525766`).
#[must_use]
pub fn ink_muted() -> Hsla {
    rgb(0x525766).into()
}

/// Physical live hardware output accent (`#34D399`).
#[must_use]
pub fn signal_live() -> Hsla {
    rgb(0x34D399).into()
}

/// Bus contention warning accent (`#FBBF24`).
#[must_use]
pub fn signal_warn() -> Hsla {
    rgb(0xFBBF24).into()
}

/// Converts a [`Rgb`] color into a GPUI [`Hsla`].
#[must_use]
pub fn from_core_rgb(c: Rgb) -> Hsla {
    rgb(c.to_u32()).into()
}

/// Floating pill/dock elevation shadow.
#[must_use]
pub fn shadow_dock() -> Vec<BoxShadow> {
    vec![BoxShadow {
        color: rgba(0x000000A8).into(),
        offset: point(px(0.0), px(14.0)),
        blur_radius: px(34.0),
        spread_radius: px(0.0),
    }]
}

/// Stage canvas elevation + live reactive ambient aura shadow.
#[must_use]
pub fn shadow_stage(ambient: Rgb) -> Vec<BoxShadow> {
    vec![
        BoxShadow {
            color: rgba(0x000000B0).into(),
            offset: point(px(0.0), px(22.0)),
            blur_radius: px(56.0),
            spread_radius: px(0.0),
        },
        BoxShadow {
            color: from_core_rgb(ambient).opacity(0.24),
            offset: point(px(0.0), px(0.0)),
            blur_radius: px(48.0),
            spread_radius: px(-8.0),
        },
    ]
}

/// Soft ambient glow for an active effect card or live indicator.
#[must_use]
pub fn shadow_glow(color: Hsla, radius: f32) -> Vec<BoxShadow> {
    vec![BoxShadow {
        color,
        offset: point(px(0.0), px(0.0)),
        blur_radius: px(radius),
        spread_radius: px(0.0),
    }]
}

/// Elevated popover shadow.
#[must_use]
pub fn shadow_popover() -> Vec<BoxShadow> {
    vec![BoxShadow {
        color: rgba(0x000000C8).into(),
        offset: point(px(0.0), px(18.0)),
        blur_radius: px(44.0),
        spread_radius: px(0.0),
    }]
}
