//! Native screen capture, FastRGB `28x20` HSL zone extraction, and non-intrusive
//! window/canvas PNG snapshot tools.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

pub mod backend;
#[cfg(target_os = "windows")]
pub mod dxgi;
pub mod error;
pub mod extractor;
pub mod snapshot;

pub use backend::{
    CaptureBackend, CaptureService, CapturedSnapshot, MonitorInfo, SyntheticCaptureBackend,
};
#[cfg(target_os = "windows")]
pub use dxgi::DxgiCaptureBackend;
pub use error::CaptureError;
pub use extractor::{PixelOrder, ZoneExtractor};
pub use snapshot::{capture_window_to_png, save_canvas_to_png, save_rgba_to_png};
