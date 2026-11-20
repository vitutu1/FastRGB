//! Output sink boundary separating RGB generation from future hardware addressing.

use std::collections::BTreeMap;

use fastrgb_core::Rgb;

use crate::error::DeviceError;
use crate::sampler::SampledLedFrame;

/// Target sink receiving sampled RGB LED frames.
///
/// Phase 2 hardware drivers (USB HID, SMBus/I2C, WinUSB, network controllers) implement this
/// trait without altering effects, capture, or UI.
pub trait DeviceOutputSink: Send {
    /// Human-readable sink name for diagnostics.
    fn name(&self) -> &str;

    /// Receives a freshly sampled frame for a device.
    ///
    /// # Errors
    ///
    /// Returns [`DeviceError::SinkFailure`] if transmission fails.
    fn flush(&mut self, frame: &SampledLedFrame) -> Result<(), DeviceError>;

    /// Applies shutdown illumination across all devices handled by this sink.
    ///
    /// # Errors
    ///
    /// Returns [`DeviceError::SinkFailure`] if shutdown transmission fails.
    fn shutdown(&mut self, _shutdown_color: Rgb) -> Result<(), DeviceError> {
        Ok(())
    }
}

/// In-memory output sink used by the GPUI preview to display live virtual LEDs.
#[derive(Debug, Default)]
pub struct VirtualPreviewSink {
    latest: BTreeMap<String, Vec<Rgb>>,
}

impl VirtualPreviewSink {
    /// Creates an empty [`VirtualPreviewSink`].
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the most recently sampled LED colors for `device_id`.
    #[must_use]
    pub fn colors_for(&self, device_id: &str) -> Option<&[Rgb]> {
        self.latest.get(device_id).map(Vec::as_slice)
    }
}

impl DeviceOutputSink for VirtualPreviewSink {
    fn name(&self) -> &str {
        "virtual-preview"
    }

    fn flush(&mut self, frame: &SampledLedFrame) -> Result<(), DeviceError> {
        self.latest
            .insert(frame.device_id.clone(), frame.colors.clone());
        Ok(())
    }

    fn shutdown(&mut self, shutdown_color: Rgb) -> Result<(), DeviceError> {
        for colors in self.latest.values_mut() {
            colors.fill(shutdown_color);
        }
        Ok(())
    }
}
