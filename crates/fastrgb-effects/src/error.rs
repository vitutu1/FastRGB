//! Error types for `fastrgb-effects`.

use fastrgb_core::CoreError;
use thiserror::Error;

/// Errors returned by the effect registry or engine.
#[derive(Debug, Error)]
pub enum EffectsError {
    /// Requested effect ID was not found in the registry.
    #[error("unknown effect id: '{0}'")]
    UnknownEffect(String),
    /// Attempted to update a parameter ID that the active effect does not declare.
    #[error("unknown parameter '{0}' for active effect")]
    UnknownParameter(String),
    /// Parameter validation failed in `fastrgb-core`.
    #[error(transparent)]
    Core(#[from] CoreError),
}
