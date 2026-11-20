//! Canvas-to-LED sampling engine.

use fastrgb_core::{CanvasFrame, Rgb};

use crate::layout::{DeviceLayout, DevicePlacement, LightingMode};

/// Output RGB frame sampled for a specific device.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SampledLedFrame {
    /// Target device identifier.
    pub device_id: String,
    /// Sampled color per LED, aligned 1:1 with `DeviceLayout::leds`.
    pub colors: Vec<Rgb>,
}

/// Stateless sampler mapping a [`CanvasFrame`] onto a [`DeviceLayout`].
#[derive(Debug, Default)]
pub struct CanvasSampler;

impl CanvasSampler {
    /// Samples LED colors for `layout` placed at `placement` with `mode` and `brightness` (`0.0..=1.0`).
    #[must_use]
    pub fn sample(
        canvas: &CanvasFrame,
        layout: &DeviceLayout,
        placement: DevicePlacement,
        mode: LightingMode,
        brightness: f32,
    ) -> SampledLedFrame {
        let brightness = brightness.clamp(0.0, 1.0);
        let mut colors = Vec::with_capacity(layout.leds.len());

        match mode {
            LightingMode::Off => {
                colors.resize(layout.leds.len(), Rgb::BLACK);
            }
            LightingMode::Forced(forced) => {
                colors.resize(layout.leds.len(), forced.scale_brightness(brightness));
            }
            LightingMode::Canvas => {
                let gw = layout.grid_width.max(1);
                let gh = layout.grid_height.max(1);
                // Area-box half-extents so sparse effects (like Matrix columns or Fireflies)
                // never fall through single-pixel gaps onto black pixels.
                let half_dx = (placement.width_nx / (gw as f32 * 2.0)).clamp(0.004, 0.025);
                let half_dy = (placement.height_ny / (gh as f32 * 2.0)).clamp(0.004, 0.025);

                for led in &layout.leds {
                    let lx = if gw <= 1 {
                        0.5
                    } else {
                        (led.x.min(gw - 1) as f32 + 0.5) / gw as f32
                    };
                    let ly = if gh <= 1 {
                        0.5
                    } else {
                        (led.y.min(gh - 1) as f32 + 0.5) / gh as f32
                    };

                    let cx = (placement.origin_nx + lx * placement.width_nx).clamp(0.0, 1.0);
                    let cy = (placement.origin_ny + ly * placement.height_ny).clamp(0.0, 1.0);
                    let raw = sample_cell_area(canvas, cx, cy, half_dx, half_dy);
                    colors.push(raw.scale_brightness(brightness));
                }
            }
        }

        SampledLedFrame {
            device_id: layout.id.clone(),
            colors,
        }
    }
}

/// Samples a `3x3` area around `(cx, cy)` and blends average + peak luminance so thin bright
/// streaks (e.g. `Matrix` columns) illuminate physical LEDs reliably without single-pixel aliasing.
fn sample_cell_area(canvas: &CanvasFrame, cx: f32, cy: f32, half_dx: f32, half_dy: f32) -> Rgb {
    const OFFSETS: [f32; 3] = [-0.65, 0.0, 0.65];
    let mut sum_r = 0u32;
    let mut sum_g = 0u32;
    let mut sum_b = 0u32;
    let mut max_luma = 0u32;
    let mut peak = Rgb::BLACK;

    for oy in OFFSETS {
        let ny = (cy + oy * half_dy).clamp(0.0, 1.0);
        for ox in OFFSETS {
            let nx = (cx + ox * half_dx).clamp(0.0, 1.0);
            let c = canvas.sample_normalized(nx, ny);
            sum_r += u32::from(c.r);
            sum_g += u32::from(c.g);
            sum_b += u32::from(c.b);
            let luma = u32::from(c.r) + u32::from(c.g) * 2 + u32::from(c.b);
            if luma > max_luma {
                max_luma = luma;
                peak = c;
            }
        }
    }

    let avg_r = (sum_r / 9) as u8;
    let avg_g = (sum_g / 9) as u8;
    let avg_b = (sum_b / 9) as u8;

    // Preserve uniform/solid regions identically while lifting sparse thin streaks into view.
    let avg_luma = u32::from(avg_r) + u32::from(avg_g) * 2 + u32::from(avg_b);
    if max_luma > avg_luma * 2 && max_luma > 48 {
        Rgb::new(
            ((u16::from(avg_r) + u16::from(peak.r)) / 2) as u8,
            ((u16::from(avg_g) + u16::from(peak.g)) / 2) as u8,
            ((u16::from(avg_b) + u16::from(peak.b)) / 2) as u8,
        )
    } else {
        Rgb::new(avg_r, avg_g, avg_b)
    }
}

#[cfg(test)]
mod tests {
    use fastrgb_core::Rgba;

    use super::*;
    use crate::layout::{DeviceCategory, LedPoint};

    #[test]
    fn sampler_respects_placement_and_forced_mode() {
        let mut canvas = CanvasFrame::new(100, 100).unwrap();
        canvas.fill_rect(0, 0, 50, 100, Rgba::new(255, 0, 0, 255));
        canvas.fill_rect(50, 0, 50, 100, Rgba::new(0, 0, 255, 255));

        let layout = DeviceLayout {
            id: "strip".into(),
            name: "Strip".into(),
            vendor: "Test".into(),
            category: DeviceCategory::ArgbStrip,
            grid_width: 2,
            grid_height: 1,
            leds: vec![
                LedPoint {
                    name: "L".into(),
                    x: 0,
                    y: 0,
                },
                LedPoint {
                    name: "R".into(),
                    x: 1,
                    y: 0,
                },
            ],
        };

        let frame = CanvasSampler::sample(
            &canvas,
            &layout,
            DevicePlacement::default(),
            LightingMode::Canvas,
            1.0,
        );
        assert_eq!(frame.colors[0], Rgb::new(255, 0, 0));
        assert_eq!(frame.colors[1], Rgb::new(0, 0, 255));

        let forced = CanvasSampler::sample(
            &canvas,
            &layout,
            DevicePlacement::default(),
            LightingMode::Forced(Rgb::new(0, 200, 0)),
            0.5,
        );
        assert_eq!(forced.colors[0], Rgb::new(0, 100, 0));
    }
}
