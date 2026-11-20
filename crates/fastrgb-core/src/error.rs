//! Error types for `fastrgb-core`.

use thiserror::Error;

/// Errors returned by core domain operations.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum CoreError {
    /// Failed to parse a hexadecimal color string.
    #[error("invalid hex color string: {0}")]
    InvalidHexColor(String),
    /// Provided canvas dimensions are zero or exceed supported limits.
    #[error("invalid canvas dimensions: {width}x{height}")]
    InvalidCanvasDimensions {
        /// Requested width in pixels.
        width: u32,
        /// Requested height in pixels.
        height: u32,
    },
    /// Zone grid buffer length did not match the expected dimensions.
    #[error("invalid zone buffer length: expected {expected}, got {actual}")]
    InvalidZoneBufferLength {
        /// Expected number of elements.
        expected: usize,
        /// Actual number of elements provided.
        actual: usize,
    },
    /// Attempted to set a parameter with an incompatible value or unknown id.
    #[error("invalid parameter update for '{id}': {reason}")]
    InvalidParameter {
        /// Parameter identifier.
        id: String,
        /// Reason the update was rejected.
        reason: String,
    },
}
