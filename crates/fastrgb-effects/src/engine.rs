//! Stateful effect runner managing the active effect, parameter store, and `320x200` canvas.

use fastrgb_core::{CanvasFrame, ParameterDescriptor, ParameterStore, ParameterValue, ZoneGrid};

use crate::error::EffectsError;
use crate::registry::EffectRegistry;
use crate::traits::{Effect, EffectContext, EffectMetadata};

/// Maximum per-step delta time to prevent huge simulation jumps after pauses.
const MAX_DT_SECONDS: f32 = 0.1;

/// Orchestrates an active [`Effect`], its validated [`ParameterStore`], and its [`CanvasFrame`].
pub struct EffectEngine {
    active: Box<dyn Effect>,
    descriptors: Vec<ParameterDescriptor>,
    params: ParameterStore,
    canvas: CanvasFrame,
    elapsed_seconds: f32,
    speed_multiplier: f32,
}

impl EffectEngine {
    /// Creates an engine initialized with `initial_effect_id` from `registry`.
    ///
    /// # Errors
    ///
    /// Returns [`EffectsError::UnknownEffect`] if `initial_effect_id` is not registered.
    pub fn new(registry: &EffectRegistry, initial_effect_id: &str) -> Result<Self, EffectsError> {
        let active = registry.create(initial_effect_id)?;
        let descriptors = active.descriptors();
        let params = ParameterStore::from_descriptors(&descriptors);
        Ok(Self {
            active,
            descriptors,
            params,
            canvas: CanvasFrame::standard(),
            elapsed_seconds: 0.0,
            speed_multiplier: 1.0,
        })
    }

    /// Switches the active effect to `effect_id` and resets its canvas and parameters.
    ///
    /// # Errors
    ///
    /// Returns [`EffectsError::UnknownEffect`] if `effect_id` is not registered.
    pub fn switch_effect(
        &mut self,
        registry: &EffectRegistry,
        effect_id: &str,
    ) -> Result<(), EffectsError> {
        let next = registry.create(effect_id)?;
        self.descriptors = next.descriptors();
        self.params = ParameterStore::from_descriptors(&self.descriptors);
        self.active = next;
        self.canvas.clear();
        Ok(())
    }

    /// Returns static metadata for the currently active effect.
    #[must_use]
    pub fn active_metadata(&self) -> EffectMetadata {
        self.active.metadata()
    }

    /// Returns parameter descriptors declared by the active effect.
    #[must_use]
    pub fn descriptors(&self) -> &[ParameterDescriptor] {
        &self.descriptors
    }

    /// Returns current parameter values for the active effect.
    #[must_use]
    pub fn params(&self) -> &ParameterStore {
        &self.params
    }

    /// Updates a parameter on the active effect after validating against its descriptor.
    ///
    /// # Errors
    ///
    /// Returns [`EffectsError::UnknownParameter`] or [`EffectsError::Core`] if invalid.
    pub fn set_parameter(&mut self, id: &str, value: ParameterValue) -> Result<(), EffectsError> {
        let descriptor = self
            .descriptors
            .iter()
            .find(|desc| desc.id == id)
            .ok_or_else(|| EffectsError::UnknownParameter(id.to_owned()))?;
        self.params.set_validated(descriptor, value)?;
        Ok(())
    }

    /// Global playback speed multiplier (`0.02..=3.0`, default `1.0`).
    #[must_use]
    pub const fn speed_multiplier(&self) -> f32 {
        self.speed_multiplier
    }

    /// Sets global playback speed multiplier clamped to `0.02..=3.0`.
    pub fn set_speed_multiplier(&mut self, speed: f32) {
        self.speed_multiplier = speed.clamp(0.02, 3.0);
    }

    /// Cumulative scaled effect elapsed time in seconds.
    #[must_use]
    pub const fn elapsed_seconds(&self) -> f32 {
        self.elapsed_seconds
    }

    /// Resets the active effect's internal animation state and clears the canvas.
    pub fn reset_active(&mut self) {
        self.active.reset();
        self.canvas.clear();
    }

    /// Advances the active effect by `dt_seconds` (scaled by `speed_multiplier`) and updates the
    /// `320x200` canvas.
    pub fn step(&mut self, dt_seconds: f32, screen: Option<&ZoneGrid>) {
        let clamped = dt_seconds.clamp(0.0, MAX_DT_SECONDS);
        let scaled_dt = clamped * self.speed_multiplier;
        self.elapsed_seconds += scaled_dt;
        let ctx = EffectContext {
            dt_seconds: scaled_dt,
            elapsed_seconds: self.elapsed_seconds,
            screen,
            params: &self.params,
        };
        self.active.render(&ctx, &mut self.canvas);
    }

    /// Returns the most recently rendered `320x200` canvas frame.
    #[must_use]
    pub fn canvas(&self) -> &CanvasFrame {
        &self.canvas
    }
}

#[cfg(test)]
mod tests {
    use fastrgb_core::{HD_HEIGHT, HD_WIDTH, Rgb, Rgba, ZONE_COUNT, ZoneSample};

    use super::*;

    #[test]
    fn all_builtin_effects_step_cleanly() {
        let registry = EffectRegistry::with_builtins();
        let list = registry.list();
        assert_eq!(list.len(), 47);

        let mut engine = EffectEngine::new(&registry, "solid_color").unwrap();
        let fake_grid = ZoneGrid::new(
            vec![
                ZoneSample {
                    hue: 210.0,
                    saturation: 80.0,
                    lightness: 50.0,
                };
                ZONE_COUNT
            ],
            vec![Rgba::new(20, 120, 240, 255); (HD_WIDTH * HD_HEIGHT) as usize],
            1,
        )
        .unwrap();

        for meta in list {
            engine.switch_effect(&registry, meta.id).unwrap();
            engine.step(1.0 / 60.0, Some(&fake_grid));
            engine.step(1.0 / 60.0, Some(&fake_grid));
            assert_eq!(engine.canvas().width(), 320);
            assert_eq!(engine.canvas().height(), 200);
        }
    }

    #[test]
    fn solid_color_without_breathing_renders_exact_color() {
        let registry = EffectRegistry::with_builtins();
        let mut engine = EffectEngine::new(&registry, "solid_color").unwrap();
        let target = Rgb::new(0x12, 0xAB, 0xCD);
        engine
            .set_parameter("color", ParameterValue::Color(target))
            .unwrap();
        engine
            .set_parameter("breathe", ParameterValue::Boolean(false))
            .unwrap();
        engine.step(1.0 / 60.0, None);
        assert_eq!(
            engine.canvas().get_pixel(160, 100).unwrap().to_rgb(),
            target
        );
    }

    #[test]
    fn screen_ambience_hd_mode_and_filters_work() {
        let registry = EffectRegistry::with_builtins();
        let mut engine = EffectEngine::new(&registry, "screen_ambience").unwrap();
        engine
            .set_parameter("picture_mode", ParameterValue::Combobox("HD".into()))
            .unwrap();
        engine
            .set_parameter("motion_smoothing", ParameterValue::Number(0.0))
            .unwrap();
        engine
            .set_parameter("blur_amount", ParameterValue::Number(0.0))
            .unwrap();

        let grid = ZoneGrid::new(
            vec![ZoneSample::default(); ZONE_COUNT],
            vec![Rgba::new(40, 90, 200, 255); (HD_WIDTH * HD_HEIGHT) as usize],
            1,
        )
        .unwrap();
        engine.step(1.0 / 60.0, Some(&grid));
        assert_eq!(
            engine.canvas().get_pixel(10, 10).unwrap().to_rgb(),
            Rgb::new(40, 90, 200)
        );
    }

    #[test]
    fn speed_multiplier_scales_step_delta() {
        let registry = EffectRegistry::with_builtins();
        let mut engine = EffectEngine::new(&registry, "rainbow").unwrap();
        engine.set_speed_multiplier(2.5);
        engine.step(0.05, None);
        assert!((engine.elapsed_seconds() - 0.125).abs() < 1e-5);

        engine.set_speed_multiplier(10.0);
        assert!((engine.speed_multiplier() - 3.0).abs() < 1e-5);

        engine.set_speed_multiplier(0.001);
        assert!((engine.speed_multiplier() - 0.02).abs() < 1e-5);

        engine.set_speed_multiplier(0.05);
        assert!((engine.speed_multiplier() - 0.05).abs() < 1e-5);
    }
}
