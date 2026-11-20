//! Extensible RGB effect runtime, trait definitions, and built-in FastRGB effects.
//!
//! Adding a new effect requires only implementing [`Effect`] and registering its factory in
//! [`EffectRegistry::with_builtins`] (or via [`EffectRegistry::register`]).

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

pub mod effects;
pub mod engine;
pub mod error;
pub mod postprocess;
pub mod registry;
pub mod rng;
pub mod traits;

pub use engine::EffectEngine;
pub use error::EffectsError;
pub use registry::EffectRegistry;
pub use traits::{Effect, EffectCategory, EffectContext, EffectFactory, EffectMetadata};
