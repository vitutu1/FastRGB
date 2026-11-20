//! Error types for `fastrgb-capture`.

use fastrgb_core::CoreError;
use thiserror::Error;

/// Errors returned by capture backends, zone extraction, or snapshot exporters.
#[derive(Debug, Error)]
pub enum CaptureError {
    /// No monitor was found for the requested index.
    #[error("monitor index {0} is out of range")]
    InvalidMonitorIndex(usize),
    /// A capture session is already active or could not be initialized.
    #[error("capture backend error: {0}")]
    Backend(String),
    /// Provided raw frame dimensions or byte length were invalid.
    #[error("invalid frame buffer ({width}x{height}): {reason}")]
    InvalidFrameBuffer {
        /// Frame width.
        width: u32,
        /// Frame height.
        height: u32,
        /// Detailed reason.
        reason: String,
    },
    /// Native OS window capture failed.
    #[error("native window snapshot failed: {0}")]
    WindowSnapshot(String),
    /// Image encoding or filesystem error.
    #[error("image I/O error: {0}")]
    ImageIo(String),
    /// Core validation failure.
    #[error(transparent)]
    Core(#[from] CoreError),
}
