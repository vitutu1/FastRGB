//! Error types for `fastrgb-devices`.

use thiserror::Error;

/// Errors returned by device registration, sampling, or output sinks.
#[derive(Debug, Error)]
pub enum DeviceError {
    /// Device with the given ID was not found.
    #[error("unknown device id: '{0}'")]
    UnknownDevice(String),
    /// An output sink failed to transmit LED frames.
    #[error("device sink '{sink}' failed: {reason}")]
    SinkFailure {
        /// Name of the failing sink.
        sink: String,
        /// Detailed failure reason.
        reason: String,
    },
}
