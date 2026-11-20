//! Native Rust + GPUI entry point for FastRGB.
#![cfg_attr(
    all(target_os = "windows", not(debug_assertions)),
    windows_subsystem = "windows"
)]

mod autostart;
mod cli;
mod settings;
mod state;
mod tray;
mod ui;

use gpui::{
    App, Application, Bounds, TitlebarOptions, WindowBounds, WindowOptions, prelude::*, px, size,
};

use crate::cli::AppOptions;
use crate::state::FastRgbApp;

fn main() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .try_init();

    let options = AppOptions::parse(std::env::args().skip(1));
    if options.probe_hardware {
        let report = fastrgb_hardware::discover_hardware();
        println!(
            "External RGB controller running: {}",
            report.external_rgb_running
        );
        println!("Matched HID controllers ({}):", report.matched_hid.len());
        for matched in &report.matched_hid {
            println!(
                "  - {} | VID=0x{:04X} PID=0x{:04X} MI={:?} UsagePage=0x{:04X} Path={}",
                matched.driver_name,
                matched.info.vendor_id,
                matched.info.product_id,
                matched.info.interface,
                matched.info.usage_page,
                matched.info.path
            );
        }
        println!(
            "NVIDIA GPUs via nvapi64.dll ({}):",
            report.nvidia_gpus.len()
        );
        for gpu in &report.nvidia_gpus {
            println!(
                "  - [{}] {} | VEN=0x{:04X} DEV=0x{:04X} SUBSYS=0x{:04X}{:04X}",
                gpu.index,
                gpu.name,
                gpu.vendor_id,
                gpu.device_id,
                gpu.sub_device_id,
                gpu.sub_vendor_id
            );
            if gpu.sub_vendor_id == 0x1462 {
                match fastrgb_hardware::NvapiDriver::load() {
                    Ok(drv) => {
                        for line in
                            drv.probe_i2c_settings(gpu.index, fastrgb_hardware::MSI_GPU_I2C_ADDR)
                        {
                            println!("      {line}");
                        }
                    }
                    Err(err) => println!("      NVAPI load ERR: {err}"),
                }
            }
        }
        println!("All HID interfaces ({}):", report.all_hid.len());
        for hid in &report.all_hid {
            println!(
                "  - VID=0x{:04X} PID=0x{:04X} MI={:?} Usage=0x{:04X}:0x{:04X} Out={} Prod='{}' Mfg='{}'",
                hid.vendor_id,
                hid.product_id,
                hid.interface,
                hid.usage_page,
                hid.usage,
                hid.output_report_len,
                hid.product,
                hid.manufacturer
            );
        }
        for note in &report.notes {
            println!("Note: {note}");
        }
        return;
    }

    Application::new().run(move |cx: &mut App| {
        cx.on_window_closed(|cx| {
            cx.quit();
        })
        .detach();

        let bounds = Bounds::centered(None, size(px(1280.0), px(820.0)), cx);
        let start_visible = !options.minimized;
        let window_options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            titlebar: Some(TitlebarOptions {
                title: Some("FastRGB".into()),
                appears_transparent: true,
                traffic_light_position: None,
            }),
            focus: start_visible && !options.is_snapshot_mode(),
            show: start_visible,
            ..Default::default()
        };

        let opts = options.clone();
        if let Err(err) = cx.open_window(window_options, move |window, cx| {
            let view = cx.new(|cx| FastRgbApp::new(opts, window, cx));
            let view_for_close = view.clone();
            window.on_window_should_close(cx, move |_window, cx| {
                let mut should_close = false;
                view_for_close.update(cx, |app, cx| {
                    if app.allow_exit || app.options.is_snapshot_mode() {
                        should_close = true;
                    } else {
                        app.hide_to_tray(cx);
                        should_close = false;
                    }
                });
                should_close
            });
            view
        }) {
            tracing::error!("failed to open FastRGB GPUI window: {err}");
            cx.quit();
            return;
        }

        if start_visible && !options.is_snapshot_mode() {
            cx.activate(true);
        }
    });
}
