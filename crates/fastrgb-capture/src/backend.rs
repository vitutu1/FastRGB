//! Trait-abstracted screen capture backends and background capture service.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use fastrgb_core::{Hsl, ZoneGrid};
use parking_lot::RwLock;

use crate::error::CaptureError;
use crate::extractor::{PixelOrder, ZoneExtractor};

/// Metadata about an available display/capture target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonitorInfo {
    /// Zero-based index used to select the monitor.
    pub index: usize,
    /// Human-readable display name.
    pub name: String,
    /// Native width in pixels.
    pub width: u32,
    /// Native height in pixels.
    pub height: u32,
}

/// Latest captured monitor snapshot ready for effects and UI preview.
#[derive(Debug, Clone)]
pub struct CapturedSnapshot {
    /// Extracted `28x20` HSL zones and `160x100` HD buffer.
    pub zone_grid: ZoneGrid,
    /// Source monitor width.
    pub source_width: u32,
    /// Source monitor height.
    pub source_height: u32,
    /// Timestamp when this frame was captured.
    pub captured_at: Instant,
}

/// Abstraction over a screen capture provider (DXGI, synthetic test source, etc.).
pub trait ScreenCaptureSource: Send {
    /// Captures the next frame and converts it into a [`CapturedSnapshot`].
    ///
    /// Returning `Ok(None)` indicates no new frame was available before timeout.
    ///
    /// # Errors
    ///
    /// Returns [`CaptureError`] if the underlying capture session fails unrecoverably.
    fn capture_next(&mut self, sequence: u64) -> Result<Option<CapturedSnapshot>, CaptureError>;
}

/// Factory trait for enumerating monitors and opening [`ScreenCaptureSource`] sessions.
pub trait CaptureBackend: Send + Sync {
    /// Lists all available capture monitors.
    ///
    /// # Errors
    ///
    /// Returns [`CaptureError::Backend`] if display enumeration fails.
    fn list_monitors(&self) -> Result<Vec<MonitorInfo>, CaptureError>;

    /// Opens a live capture session for `monitor_index`.
    ///
    /// # Errors
    ///
    /// Returns [`CaptureError::InvalidMonitorIndex`] or [`CaptureError::Backend`].
    fn open_monitor(
        &self,
        monitor_index: usize,
    ) -> Result<Box<dyn ScreenCaptureSource>, CaptureError>;
}

/// Deterministic synthetic capture backend for headless testing and offline demos.
#[derive(Debug, Default)]
pub struct SyntheticCaptureBackend;

impl CaptureBackend for SyntheticCaptureBackend {
    fn list_monitors(&self) -> Result<Vec<MonitorInfo>, CaptureError> {
        Ok(vec![MonitorInfo {
            index: 0,
            name: "Synthetic Test Display (320x200)".to_owned(),
            width: 320,
            height: 200,
        }])
    }

    fn open_monitor(
        &self,
        monitor_index: usize,
    ) -> Result<Box<dyn ScreenCaptureSource>, CaptureError> {
        if monitor_index != 0 {
            return Err(CaptureError::InvalidMonitorIndex(monitor_index));
        }
        Ok(Box::new(SyntheticSource {
            phase: 0.0,
            buf: vec![0; 320 * 200 * 4],
        }))
    }
}

struct SyntheticSource {
    phase: f32,
    buf: Vec<u8>,
}

impl ScreenCaptureSource for SyntheticSource {
    fn capture_next(&mut self, sequence: u64) -> Result<Option<CapturedSnapshot>, CaptureError> {
        let width = 320u32;
        let height = 200u32;
        for y in 0..height as usize {
            for x in 0..width as usize {
                let hue = ((x as f32) * 0.8 + (y as f32) * 0.5 + self.phase).rem_euclid(360.0);
                let rgb = Hsl::new(hue, 90.0, 50.0).to_rgb();
                let idx = (y * width as usize + x) * 4;
                self.buf[idx] = rgb.b;
                self.buf[idx + 1] = rgb.g;
                self.buf[idx + 2] = rgb.r;
                self.buf[idx + 3] = 255;
            }
        }
        self.phase = (self.phase + 4.0).rem_euclid(360.0);

        let zone_grid =
            ZoneExtractor::extract(width, height, PixelOrder::Bgra, &self.buf, sequence)?;
        Ok(Some(CapturedSnapshot {
            zone_grid,
            source_width: width,
            source_height: height,
            captured_at: Instant::now(),
        }))
    }
}

/// Asynchronous background worker running a [`ScreenCaptureSource`] and publishing the latest
/// [`CapturedSnapshot`] without blocking UI or effect threads.
pub struct CaptureService {
    latest: Arc<RwLock<Option<Arc<CapturedSnapshot>>>>,
    last_error: Arc<RwLock<Option<String>>>,
    stop_flag: Option<Arc<AtomicBool>>,
    worker: Option<JoinHandle<()>>,
    active_monitor: Option<usize>,
}

impl Default for CaptureService {
    fn default() -> Self {
        Self::new()
    }
}

impl CaptureService {
    /// Creates an idle [`CaptureService`].
    #[must_use]
    pub fn new() -> Self {
        Self {
            latest: Arc::new(RwLock::new(None)),
            last_error: Arc::new(RwLock::new(None)),
            stop_flag: None,
            worker: None,
            active_monitor: None,
        }
    }

    /// Starts capturing from `monitor_index` using `backend`.
    ///
    /// Stops any existing capture worker cleanly before spawning the new one.
    ///
    /// # Errors
    ///
    /// Returns [`CaptureError`] if opening the monitor fails immediately.
    pub fn start(
        &mut self,
        backend: &dyn CaptureBackend,
        monitor_index: usize,
        target_interval: Duration,
    ) -> Result<(), CaptureError> {
        self.stop();

        let mut source = backend.open_monitor(monitor_index)?;
        let stop_flag = Arc::new(AtomicBool::new(false));
        let stop_clone = Arc::clone(&stop_flag);
        let latest_clone = Arc::clone(&self.latest);
        let error_clone = Arc::clone(&self.last_error);
        *error_clone.write() = None;

        let handle = thread::Builder::new()
            .name(format!("fastrgb-capture-{monitor_index}"))
            .spawn(move || {
                let mut sequence = 1u64;
                while !stop_clone.load(Ordering::Relaxed) {
                    let tick_start = Instant::now();
                    match source.capture_next(sequence) {
                        Ok(Some(snapshot)) => {
                            *latest_clone.write() = Some(Arc::new(snapshot));
                            sequence = sequence.saturating_add(1);
                        }
                        Ok(None) => {}
                        Err(err) => {
                            *error_clone.write() = Some(err.to_string());
                            thread::sleep(Duration::from_millis(100));
                        }
                    }

                    let elapsed = tick_start.elapsed();
                    if elapsed < target_interval {
                        thread::sleep(target_interval - elapsed);
                    }
                }
            })
            .map_err(|err| {
                CaptureError::Backend(format!("failed to spawn capture thread: {err}"))
            })?;

        self.stop_flag = Some(stop_flag);
        self.worker = Some(handle);
        self.active_monitor = Some(monitor_index);
        Ok(())
    }

    /// Stops the active capture worker thread if running.
    pub fn stop(&mut self) {
        if let Some(flag) = self.stop_flag.take() {
            flag.store(true, Ordering::Relaxed);
        }
        if let Some(handle) = self.worker.take() {
            let _ = handle.join();
        }
        self.active_monitor = None;
    }

    /// Returns `true` if a capture worker is currently running.
    #[must_use]
    pub fn is_running(&self) -> bool {
        self.worker.is_some()
    }

    /// Returns the currently active monitor index, if running.
    #[must_use]
    pub fn active_monitor(&self) -> Option<usize> {
        self.active_monitor
    }

    /// Returns the latest captured snapshot without blocking capture.
    #[must_use]
    pub fn latest(&self) -> Option<Arc<CapturedSnapshot>> {
        self.latest.read().clone()
    }

    /// Returns the most recent backend error message, if any.
    #[must_use]
    pub fn last_error(&self) -> Option<String> {
        self.last_error.read().clone()
    }
}

impl Drop for CaptureService {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn synthetic_service_produces_snapshots() {
        let backend = SyntheticCaptureBackend;
        let monitors = backend.list_monitors().unwrap();
        assert_eq!(monitors.len(), 1);

        let mut service = CaptureService::new();
        service
            .start(&backend, 0, Duration::from_millis(5))
            .unwrap();

        let mut captured = None;
        for _ in 0..40 {
            if let Some(snap) = service.latest() {
                captured = Some(snap);
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }

        service.stop();
        let snap = captured.expect("synthetic capture should produce a frame");
        assert_eq!(snap.source_width, 320);
        assert_eq!(snap.source_height, 200);
    }
}
