//! Virtual/physical device manager coordinating sampling and output sink flushing.

use fastrgb_core::{CanvasFrame, Rgb};

use crate::error::DeviceError;
use crate::layout::{DeviceLayout, DevicePlacement, LightingMode};
use crate::presets::builtin_presets;
use crate::sampler::{CanvasSampler, SampledLedFrame};
use crate::sink::{DeviceOutputSink, VirtualPreviewSink};

/// Registered device instance with placement and per-device RGB controls.
#[derive(Debug, Clone)]
pub struct VirtualDevice {
    /// Device geometry.
    pub layout: DeviceLayout,
    /// Placement on the global `320x200` canvas.
    pub placement: DevicePlacement,
    /// Per-device lighting mode (`Canvas`, `Forced`, `Off`).
    pub mode: LightingMode,
    /// Per-device brightness (`0.0..=1.0`).
    pub brightness: f32,
    /// Latest sampled LED colors for UI rendering.
    pub last_colors: Vec<Rgb>,
}

/// Manages active devices, samples the effect canvas, and dispatches frames to [`DeviceOutputSink`]s.
pub struct DeviceManager {
    devices: Vec<VirtualDevice>,
    global_brightness: f32,
    preview_sink: VirtualPreviewSink,
    external_sinks: Vec<Box<dyn DeviceOutputSink>>,
    last_sink_error: Option<String>,
}

impl Default for DeviceManager {
    fn default() -> Self {
        Self::with_builtin_presets()
    }
}

impl DeviceManager {
    /// Creates a manager pre-populated with FastRGB virtual device presets.
    #[must_use]
    pub fn with_builtin_presets() -> Self {
        let devices = builtin_presets()
            .into_iter()
            .map(|preset| {
                let len = preset.layout.leds.len();
                VirtualDevice {
                    layout: preset.layout,
                    placement: preset.placement,
                    mode: LightingMode::Canvas,
                    brightness: 1.0,
                    last_colors: vec![Rgb::BLACK; len],
                }
            })
            .collect();

        Self {
            devices,
            global_brightness: 1.0,
            preview_sink: VirtualPreviewSink::new(),
            external_sinks: Vec::new(),
            last_sink_error: None,
        }
    }

    /// Registers an additional output sink (for hardware drivers or diagnostics).
    pub fn add_sink(&mut self, sink: Box<dyn DeviceOutputSink>) {
        self.external_sinks.push(sink);
    }

    /// Removes all attached external sinks (e.g. when disabling live hardware output).
    pub fn clear_sinks(&mut self) {
        self.external_sinks.clear();
    }

    /// Shuts down all attached external sinks with `shutdown_color`.
    pub fn shutdown_sinks(&mut self, shutdown_color: Rgb) {
        for sink in &mut self.external_sinks {
            let _ = sink.shutdown(shutdown_color);
        }
    }

    /// Returns all registered devices.
    #[must_use]
    pub fn devices(&self) -> &[VirtualDevice] {
        &self.devices
    }

    /// Global master brightness (`0.0..=1.0`).
    #[must_use]
    pub const fn global_brightness(&self) -> f32 {
        self.global_brightness
    }

    /// Sets global master brightness (`0.0..=1.0`).
    pub fn set_global_brightness(&mut self, brightness: f32) {
        self.global_brightness = brightness.clamp(0.0, 1.0);
    }

    /// Sets lighting mode for a specific device ID.
    ///
    /// # Errors
    ///
    /// Returns [`DeviceError::UnknownDevice`] if no device matches `device_id`.
    pub fn set_device_mode(
        &mut self,
        device_id: &str,
        mode: LightingMode,
    ) -> Result<(), DeviceError> {
        let dev = self
            .devices
            .iter_mut()
            .find(|d| d.layout.id == device_id)
            .ok_or_else(|| DeviceError::UnknownDevice(device_id.to_owned()))?;
        dev.mode = mode;
        Ok(())
    }

    /// Sets per-device brightness (`0.0..=1.0`).
    ///
    /// # Errors
    ///
    /// Returns [`DeviceError::UnknownDevice`] if no device matches `device_id`.
    pub fn set_device_brightness(
        &mut self,
        device_id: &str,
        brightness: f32,
    ) -> Result<(), DeviceError> {
        let dev = self
            .devices
            .iter_mut()
            .find(|d| d.layout.id == device_id)
            .ok_or_else(|| DeviceError::UnknownDevice(device_id.to_owned()))?;
        dev.brightness = brightness.clamp(0.0, 1.0);
        Ok(())
    }

    /// Updates the LED count for a 1D strip/fan channel (`grid_height == 1`), clamped to `1..=120`.
    ///
    /// # Errors
    ///
    /// Returns [`DeviceError::UnknownDevice`] if no device matches `device_id`.
    pub fn resize_strip_leds(
        &mut self,
        device_id: &str,
        led_count: u32,
    ) -> Result<(), DeviceError> {
        let dev = self
            .devices
            .iter_mut()
            .find(|d| d.layout.id == device_id)
            .ok_or_else(|| DeviceError::UnknownDevice(device_id.to_owned()))?;
        let clamped = led_count.clamp(1, 120);
        dev.layout.grid_width = clamped;
        dev.layout.grid_height = 1;
        dev.layout.leds = (0..clamped)
            .map(|i| crate::layout::LedPoint {
                name: format!("LED-{:02}", i + 1),
                x: i,
                y: 0,
            })
            .collect();
        dev.last_colors.resize(clamped as usize, Rgb::BLACK);
        Ok(())
    }

    /// Samples `canvas` for every registered device and flushes frames to all sinks.
    ///
    /// Returns sampled frames so callers can inspect or render them immediately.
    pub fn update_from_canvas(&mut self, canvas: &CanvasFrame) -> Vec<SampledLedFrame> {
        let mut frames = Vec::with_capacity(self.devices.len());
        for dev in &mut self.devices {
            let effective_brightness = (self.global_brightness * dev.brightness).clamp(0.0, 1.0);
            let frame = CanvasSampler::sample(
                canvas,
                &dev.layout,
                dev.placement,
                dev.mode,
                effective_brightness,
            );
            dev.last_colors.clone_from(&frame.colors);
            let _ = self.preview_sink.flush(&frame);
            for sink in &mut self.external_sinks {
                if let Err(err) = sink.flush(&frame) {
                    self.last_sink_error = Some(err.to_string());
                }
            }
            frames.push(frame);
        }
        frames
    }

    /// Consumes the most recent hardware sink flush error, if any.
    pub fn take_last_sink_error(&mut self) -> Option<String> {
        self.last_sink_error.take()
    }

    /// Returns the most recently sampled colors for `device_id`.
    #[must_use]
    pub fn preview_colors(&self, device_id: &str) -> Option<&[Rgb]> {
        self.preview_sink.colors_for(device_id)
    }
}
