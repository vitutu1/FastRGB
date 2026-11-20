//! Pure domain types and color/canvas primitives for FastRGB Rust Port.
//!
//! This crate intentionally has zero OS, UI, capture, or hardware I/O dependencies.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

pub mod canvas;
pub mod color;
pub mod error;
pub mod params;
pub mod zone;

pub use canvas::{CanvasFrame, DEFAULT_CANVAS_HEIGHT, DEFAULT_CANVAS_WIDTH};
pub use color::{Hsl, Rgb, Rgba};
pub use error::CoreError;
pub use params::{ParameterDescriptor, ParameterKind, ParameterStore, ParameterValue};
pub use zone::{HD_HEIGHT, HD_WIDTH, ZONE_COLS, ZONE_COUNT, ZONE_ROWS, ZoneGrid, ZoneSample};
