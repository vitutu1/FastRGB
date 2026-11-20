//! Built-in virtual device layout presets for visual RGB validation.

use std::f32::consts::TAU;

use crate::layout::{DeviceCategory, DeviceLayout, DevicePlacement, LedPoint};

/// Standard virtual device preset with recommended canvas placement.
#[derive(Debug, Clone)]
pub struct DevicePreset {
    /// Device geometry.
    pub layout: DeviceLayout,
    /// Suggested canvas placement.
    pub placement: DevicePlacement,
}

/// Returns the default set of virtual + hardware-matched RGB devices for the rig:
/// ASUS TUF B850 ARGB Fan/Strip Headers (Channels 1-3), Corsair Dominator Platinum RAM,
/// MSI RTX 4070 SUPER Gaming X Slim, plus virtual desk peripherals.
#[must_use]
pub fn builtin_presets() -> Vec<DevicePreset> {
    vec![
        asus_argb_channel_preset(1, 0.05, 0.06, 20),
        asus_argb_channel_preset(2, 0.05, 0.16, 20),
        asus_argb_channel_preset(3, 0.05, 0.26, 20),
        corsair_dominator_dimm_preset(1, 0.65, 0.06, "0x19"),
        corsair_dominator_dimm_preset(2, 0.72, 0.06, "0x1B"),
        msi_4070_super_gaming_x_preset(),
        tkl_keyboard_preset(),
        gaming_mouse_preset(),
    ]
}

fn asus_argb_channel_preset(
    channel: u8,
    origin_nx: f32,
    origin_ny: f32,
    led_count: u32,
) -> DevicePreset {
    let leds = (0..led_count)
        .map(|i| LedPoint {
            name: format!("Ch{channel}-LED-{:02}", i + 1),
            x: i,
            y: 0,
        })
        .collect();

    DevicePreset {
        layout: DeviceLayout {
            id: format!("asus_b850_argb_ch{channel}"),
            name: format!("ASUS B850 ARGB Ch{channel} (Fans/Strip)"),
            vendor: "ASUS Aura (0x0B05:0x19AF)".to_owned(),
            category: DeviceCategory::ArgbStrip,
            grid_width: led_count,
            grid_height: 1,
            leds,
        },
        placement: DevicePlacement {
            origin_nx,
            origin_ny,
            width_nx: 0.56,
            height_ny: 0.07,
        },
    }
}

fn corsair_dominator_dimm_preset(
    stick: u8,
    origin_nx: f32,
    origin_ny: f32,
    smbus_addr: &str,
) -> DevicePreset {
    let leds = (0..12u32)
        .map(|led| LedPoint {
            name: format!("DIMM{stick}-LED{:02}", led + 1),
            x: 0,
            y: led,
        })
        .collect();

    DevicePreset {
        layout: DeviceLayout {
            id: format!("corsair_dominator_dimm{stick}"),
            name: format!("Corsair Dominator · DIMM {stick}"),
            vendor: format!("Corsair (SMBus {smbus_addr})"),
            category: DeviceCategory::Ram,
            grid_width: 1,
            grid_height: 12,
            leds,
        },
        placement: DevicePlacement {
            origin_nx,
            origin_ny,
            width_nx: 0.055,
            height_ny: 0.34,
        },
    }
}

fn msi_4070_super_gaming_x_preset() -> DevicePreset {
    let leds = (0..3u32)
        .map(|i| LedPoint {
            name: format!("GPU-Zone-{}", i + 1),
            x: i,
            y: 0,
        })
        .collect();

    DevicePreset {
        layout: DeviceLayout {
            id: "msi_4070_super_gaming_x".to_owned(),
            name: "MSI RTX 4070 SUPER Gaming X Slim".to_owned(),
            vendor: "MSI / NVAPI (I2C 0x68)".to_owned(),
            category: DeviceCategory::Gpu,
            grid_width: 3,
            grid_height: 1,
            leds,
        },
        placement: DevicePlacement {
            origin_nx: 0.80,
            origin_ny: 0.14,
            width_nx: 0.15,
            height_ny: 0.18,
        },
    }
}

fn tkl_keyboard_preset() -> DevicePreset {
    const ROWS: [&[&str]; 6] = [
        &[
            "Esc", "F1", "F2", "F3", "F4", "F5", "F6", "F7", "F8", "F9", "F10", "F11", "F12",
            "Prt", "Scr", "Pau",
        ],
        &[
            "`", "1", "2", "3", "4", "5", "6", "7", "8", "9", "0", "-", "=", "Bksp", "Ins", "Hom",
            "PgU",
        ],
        &[
            "Tab", "Q", "W", "E", "R", "T", "Y", "U", "I", "O", "P", "[", "]", "\\", "Del", "End",
            "PgD",
        ],
        &[
            "Caps", "A", "S", "D", "F", "G", "H", "J", "K", "L", ";", "'", "Enter",
        ],
        &[
            "LShift", "Z", "X", "C", "V", "B", "N", "M", ",", ".", "/", "RShift", "Up",
        ],
        &[
            "LCtrl", "Win", "LAlt", "Space", "RAlt", "Fn", "Menu", "RCtrl", "Left", "Down", "Right",
        ],
    ];

    let mut leds = Vec::new();
    for (y, row) in ROWS.iter().enumerate() {
        for (x, name) in row.iter().enumerate() {
            leds.push(LedPoint {
                name: (*name).to_owned(),
                x: x as u32,
                y: y as u32,
            });
        }
    }

    DevicePreset {
        layout: DeviceLayout {
            id: "virtual_keyboard_tkl".to_owned(),
            name: "Virtual TKL Keyboard".to_owned(),
            vendor: "FastRGB Virtual".to_owned(),
            category: DeviceCategory::Keyboard,
            grid_width: 17,
            grid_height: 6,
            leds,
        },
        placement: DevicePlacement {
            origin_nx: 0.08,
            origin_ny: 0.48,
            width_nx: 0.56,
            height_ny: 0.42,
        },
    }
}

fn gaming_mouse_preset() -> DevicePreset {
    let coords = [
        ("Scroll", 2, 1),
        ("DPI", 2, 2),
        ("Left-Top", 0, 1),
        ("Right-Top", 4, 1),
        ("Left-Mid", 0, 3),
        ("Right-Mid", 4, 3),
        ("Logo", 2, 5),
        ("Left-Rear", 1, 6),
        ("Right-Rear", 3, 6),
        ("Base-Rear", 2, 7),
    ];
    let leds = coords
        .into_iter()
        .map(|(name, x, y)| LedPoint {
            name: name.to_owned(),
            x,
            y,
        })
        .collect();

    DevicePreset {
        layout: DeviceLayout {
            id: "virtual_mouse".to_owned(),
            name: "Virtual Gaming Mouse".to_owned(),
            vendor: "FastRGB Virtual".to_owned(),
            category: DeviceCategory::Mouse,
            grid_width: 5,
            grid_height: 8,
            leds,
        },
        placement: DevicePlacement {
            origin_nx: 0.72,
            origin_ny: 0.50,
            width_nx: 0.16,
            height_ny: 0.36,
        },
    }
}

#[allow(dead_code)]
fn argb_strip_preset() -> DevicePreset {
    let count = 30u32;
    let leds = (0..count)
        .map(|i| LedPoint {
            name: format!("LED-{:02}", i + 1),
            x: i,
            y: 0,
        })
        .collect();

    DevicePreset {
        layout: DeviceLayout {
            id: "virtual_argb_strip".to_owned(),
            name: "Desk ARGB Strip (30 LEDs)".to_owned(),
            vendor: "FastRGB Virtual".to_owned(),
            category: DeviceCategory::ArgbStrip,
            grid_width: count,
            grid_height: 1,
            leds,
        },
        placement: DevicePlacement {
            origin_nx: 0.05,
            origin_ny: 0.12,
            width_nx: 0.90,
            height_ny: 0.08,
        },
    }
}

#[allow(dead_code)]
fn argb_fan_preset() -> DevicePreset {
    let led_count = 16usize;
    let mut leds = Vec::with_capacity(led_count);
    for i in 0..led_count {
        let angle = (i as f32 / led_count as f32) * TAU;
        let x = (3.0 + angle.cos() * 3.0).round().clamp(0.0, 6.0) as u32;
        let y = (3.0 + angle.sin() * 3.0).round().clamp(0.0, 6.0) as u32;
        leds.push(LedPoint {
            name: format!("Ring-{:02}", i + 1),
            x,
            y,
        });
    }

    DevicePreset {
        layout: DeviceLayout {
            id: "virtual_argb_fan".to_owned(),
            name: "ARGB Case Fan (16 LEDs)".to_owned(),
            vendor: "FastRGB Virtual".to_owned(),
            category: DeviceCategory::Fan,
            grid_width: 7,
            grid_height: 7,
            leds,
        },
        placement: DevicePlacement {
            origin_nx: 0.70,
            origin_ny: 0.16,
            width_nx: 0.22,
            height_ny: 0.24,
        },
    }
}
