//! Minimal dependency-free CLI argument parser for `fastrgb-app`.

use std::path::PathBuf;

/// Runtime launch options parsed from process arguments.
#[derive(Debug, Clone)]
pub struct AppOptions {
    /// Initial effect ID to activate on startup.
    pub initial_effect: String,
    /// Force the synthetic display capture backend instead of DXGI Desktop Duplication.
    pub synthetic_capture: bool,
    /// Optional path to save a native `PrintWindow` PNG snapshot of the app window and exit.
    pub snapshot_window: Option<PathBuf>,
    /// Optional path to save a PNG snapshot of the `320x200` RGB canvas and exit.
    pub snapshot_canvas: Option<PathBuf>,
    /// Delay in milliseconds before taking automated `--snapshot-*` captures.
    pub snapshot_delay_ms: u64,
    /// Print discovered system HID / NVAPI hardware in read-only mode and exit.
    pub probe_hardware: bool,
    /// Initial UI section (`library`, `lighting`, or `devices`).
    pub initial_section: String,
    /// Launch hidden directly into the Windows System Tray (used by Windows autostart).
    pub minimized: bool,
}

impl Default for AppOptions {
    fn default() -> Self {
        Self {
            initial_effect: "rainbow".to_owned(),
            synthetic_capture: false,
            snapshot_window: None,
            snapshot_canvas: None,
            snapshot_delay_ms: 450,
            probe_hardware: false,
            initial_section: "lighting".to_owned(),
            minimized: false,
        }
    }
}

impl AppOptions {
    /// Parses CLI arguments from an iterator (typically `std::env::args().skip(1)`).
    #[must_use]
    pub fn parse<I>(args: I) -> Self
    where
        I: IntoIterator<Item = String>,
    {
        let mut opts = Self::default();
        let mut iter = args.into_iter();
        while let Some(arg) = iter.next() {
            match arg.as_str() {
                "--effect" => {
                    if let Some(val) = iter.next() {
                        opts.initial_effect = val;
                    }
                }
                "--synthetic-capture" => {
                    opts.synthetic_capture = true;
                }
                "--snapshot-window" => {
                    if let Some(val) = iter.next() {
                        opts.snapshot_window = Some(PathBuf::from(val));
                    }
                }
                "--snapshot-canvas" => {
                    if let Some(val) = iter.next() {
                        opts.snapshot_canvas = Some(PathBuf::from(val));
                    }
                }
                "--snapshot-delay-ms" => {
                    if let Some(val) = iter.next()
                        && let Ok(ms) = val.parse::<u64>()
                    {
                        opts.snapshot_delay_ms = ms;
                    }
                }
                "--probe-hardware" => {
                    opts.probe_hardware = true;
                }
                "--tab" => {
                    if let Some(val) = iter.next() {
                        opts.initial_section = val;
                    }
                }
                "--minimized" => {
                    opts.minimized = true;
                }
                _ => {}
            }
        }
        opts
    }

    /// Returns `true` if automated snapshot-and-exit mode was requested.
    #[must_use]
    pub fn is_snapshot_mode(&self) -> bool {
        self.snapshot_window.is_some() || self.snapshot_canvas.is_some()
    }
}
