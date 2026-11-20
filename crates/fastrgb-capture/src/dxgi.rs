//! Windows DXGI Desktop Duplication backend powered by `windows-capture`.

use std::time::Instant;

use windows_capture::dxgi_duplication_api::{
    DxgiDuplicationApi, DxgiDuplicationFormat, Error as DxgiError,
};
use windows_capture::monitor::Monitor;

use crate::backend::{CaptureBackend, CapturedSnapshot, MonitorInfo, ScreenCaptureSource};
use crate::error::CaptureError;
use crate::extractor::{PixelOrder, ZoneExtractor};

/// Monitor capture backend using Windows DXGI Desktop Duplication.
#[derive(Debug, Default)]
pub struct DxgiCaptureBackend;

impl CaptureBackend for DxgiCaptureBackend {
    fn list_monitors(&self) -> Result<Vec<MonitorInfo>, CaptureError> {
        let monitors =
            Monitor::enumerate().map_err(|err| CaptureError::Backend(err.to_string()))?;
        let mut out = Vec::with_capacity(monitors.len());
        for (idx, monitor) in monitors.into_iter().enumerate() {
            let name = monitor
                .device_string()
                .or_else(|_| monitor.name())
                .unwrap_or_else(|_| format!("Display {}", idx + 1));
            let width = monitor.width().unwrap_or(1920);
            let height = monitor.height().unwrap_or(1080);
            out.push(MonitorInfo {
                index: idx,
                name: format!("{name} ({width}x{height})"),
                width,
                height,
            });
        }
        Ok(out)
    }

    fn open_monitor(
        &self,
        monitor_index: usize,
    ) -> Result<Box<dyn ScreenCaptureSource>, CaptureError> {
        let monitors =
            Monitor::enumerate().map_err(|err| CaptureError::Backend(err.to_string()))?;
        let monitor = monitors
            .into_iter()
            .nth(monitor_index)
            .ok_or(CaptureError::InvalidMonitorIndex(monitor_index))?;

        let dup = DxgiDuplicationApi::new_options(monitor, &[DxgiDuplicationFormat::Bgra8])
            .map_err(|err| CaptureError::Backend(err.to_string()))?;

        Ok(Box::new(DxgiSource {
            monitor_index,
            duplication: Some(dup),
            scratch: Vec::new(),
        }))
    }
}

struct DxgiSource {
    monitor_index: usize,
    duplication: Option<DxgiDuplicationApi>,
    scratch: Vec<u8>,
}

impl DxgiSource {
    fn recreate(&mut self) -> Result<(), CaptureError> {
        let existing = self.duplication.take();
        if let Some(dup) = existing {
            match dup.recreate_options(&[DxgiDuplicationFormat::Bgra8]) {
                Ok(recreated) => {
                    self.duplication = Some(recreated);
                    return Ok(());
                }
                Err(err) => {
                    // Fall back to fresh monitor lookup if handle became stale.
                    let _ = err;
                }
            }
        }

        let monitor = Monitor::enumerate()
            .map_err(|err| CaptureError::Backend(err.to_string()))?
            .into_iter()
            .nth(self.monitor_index)
            .ok_or(CaptureError::InvalidMonitorIndex(self.monitor_index))?;
        let fresh = DxgiDuplicationApi::new_options(monitor, &[DxgiDuplicationFormat::Bgra8])
            .map_err(|err| CaptureError::Backend(err.to_string()))?;
        self.duplication = Some(fresh);
        Ok(())
    }
}

impl ScreenCaptureSource for DxgiSource {
    fn capture_next(&mut self, sequence: u64) -> Result<Option<CapturedSnapshot>, CaptureError> {
        if self.duplication.is_none() {
            self.recreate()?;
        }
        let Some(dup) = self.duplication.as_mut() else {
            return Ok(None);
        };

        let mut frame = match dup.acquire_next_frame(16) {
            Ok(frame) => frame,
            Err(DxgiError::Timeout) => return Ok(None),
            Err(DxgiError::AccessLost) => {
                self.recreate()?;
                return Ok(None);
            }
            Err(err) => return Err(CaptureError::Backend(err.to_string())),
        };

        let buffer = frame
            .buffer()
            .map_err(|err| CaptureError::Backend(err.to_string()))?;
        let width = buffer.width();
        let height = buffer.height();
        let order = match buffer.format() {
            DxgiDuplicationFormat::Rgba8 => PixelOrder::Rgba,
            _ => PixelOrder::Bgra,
        };
        let raw = buffer.as_nopadding_buffer(&mut self.scratch);
        let zone_grid = ZoneExtractor::extract(width, height, order, raw, sequence)?;

        Ok(Some(CapturedSnapshot {
            zone_grid,
            source_width: width,
            source_height: height,
            captured_at: Instant::now(),
        }))
    }
}
