//! Decoupled virtual device layouts, canvas-to-LED color sampling, and hardware sink traits.
//!
//! Hardware transport drivers (USB HID, SMBus, WLED, OpenRGB) can later implement
//! [`DeviceOutputSink`] without modifying effects, capture, or UI code.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

pub mod error;
pub mod layout;
pub mod manager;
pub mod presets;
pub mod sampler;
pub mod sink;

pub use error::DeviceError;
pub use layout::{DeviceCategory, DeviceLayout, DevicePlacement, LedPoint, LightingMode};
pub use manager::{DeviceManager, VirtualDevice};
pub use sampler::{CanvasSampler, SampledLedFrame};
pub use sink::{DeviceOutputSink, VirtualPreviewSink};
