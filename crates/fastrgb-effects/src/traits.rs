//! Core traits and metadata types implemented by every RGB effect.

use fastrgb_core::{CanvasFrame, ParameterDescriptor, ParameterStore, ZoneGrid};

/// High-level grouping for effects in UI and presets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EffectCategory {
    /// Minimal static / classic lighting effects.
    Static,
    /// Procedural 2D / particle / shader animations (`Basic/`).
    Basic,
    /// Reactive effects driven by screen capture or external inputs.
    Dynamic,
}

/// Immutable metadata describing an effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EffectMetadata {
    /// Unique stable identifier (e.g. `"screen_ambience"`).
    pub id: &'static str,
    /// Human-readable display name.
    pub name: &'static str,
    /// Short description shown in the effect inspector.
    pub description: &'static str,
    /// Author or original publisher name.
    pub publisher: &'static str,
    /// Effect category.
    pub category: EffectCategory,
    /// Whether this effect consumes [`ZoneGrid`] screen capture frames.
    pub uses_screen: bool,
}

/// Per-frame read-only inputs supplied to [`Effect::render`].
pub struct EffectContext<'a> {
    /// Elapsed frame delta in seconds (clamped for simulation stability).
    pub dt_seconds: f32,
    /// Monotonic time since engine start in seconds.
    pub elapsed_seconds: f32,
    /// Optional latest screen capture zone data.
    pub screen: Option<&'a ZoneGrid>,
    /// Current validated parameter values for this effect.
    pub params: &'a ParameterStore,
}

/// Trait implemented by every decoupled RGB effect.
pub trait Effect: Send {
    /// Static metadata for this effect.
    fn metadata(&self) -> EffectMetadata;

    /// Declarative parameter descriptors used to build UI controls and validate updates.
    fn descriptors(&self) -> Vec<ParameterDescriptor>;

    /// Resets any internal animation/smoothing state.
    fn reset(&mut self);

    /// Advances internal state and renders a frame onto `canvas`.
    fn render(&mut self, ctx: &EffectContext<'_>, canvas: &mut CanvasFrame);
}

/// Constructor function pointer stored in [`crate::EffectRegistry`].
pub type EffectFactory = fn() -> Box<dyn Effect>;
