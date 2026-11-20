//! Dynamic registry mapping effect IDs to effect constructors.

use std::collections::BTreeMap;

use crate::effects;
use crate::error::EffectsError;
use crate::traits::{Effect, EffectFactory, EffectMetadata};

struct RegisteredEffect {
    metadata: EffectMetadata,
    factory: EffectFactory,
}

/// Registry of available RGB effects.
///
/// New effects can be added without modifying any UI or device code by calling
/// [`EffectRegistry::register`].
#[derive(Default)]
pub struct EffectRegistry {
    entries: BTreeMap<&'static str, RegisteredEffect>,
}

impl EffectRegistry {
    /// Creates an empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a registry populated with all built-in FastRGB effects.
    #[must_use]
    pub fn with_builtins() -> Self {
        let mut registry = Self::new();
        registry.register(|| Box::new(effects::AuroraEffect::new()));
        registry.register(|| Box::new(effects::BeachEffect::new()));
        registry.register(|| Box::new(effects::BlackHoleEffect::new()));
        registry.register(|| Box::new(effects::BombingRunEffect::new()));
        registry.register(|| Box::new(effects::BouncingBallEffect::new()));
        registry.register(|| Box::new(effects::BubblesEffect::new()));
        registry.register(|| Box::new(effects::CamoFadeEffect::new()));
        registry.register(|| Box::new(effects::CottonCandyEffect::new()));
        registry.register(|| Box::new(effects::CrimsonEffect::new()));
        registry.register(|| Box::new(effects::CrookedWavesEffect::new()));
        registry.register(|| Box::new(effects::CyberEffect::new()));
        registry.register(|| Box::new(effects::DnaEffect::new()));
        registry.register(|| Box::new(effects::DrizzleEffect::new()));
        registry.register(|| Box::new(effects::ElectricColorsEffect::new()));
        registry.register(|| Box::new(effects::ExplosionEffect::new()));
        registry.register(|| Box::new(effects::FallingStarsEffect::new()));
        registry.register(|| Box::new(effects::FireEffect::new()));
        registry.register(|| Box::new(effects::FirefliesEffect::new()));
        registry.register(|| Box::new(effects::GoodNightEffect::new()));
        registry.register(|| Box::new(effects::GradientPinwheelEffect::new()));
        registry.register(|| Box::new(effects::HalloweenStormEffect::new()));
        registry.register(|| Box::new(effects::HyperspaceEffect::new()));
        registry.register(|| Box::new(effects::JadeEffect::new()));
        registry.register(|| Box::new(effects::LavaLampEffect::new()));
        registry.register(|| Box::new(effects::MatrixEffect::new()));
        registry.register(|| Box::new(effects::NeonFireEffect::new()));
        registry.register(|| Box::new(effects::NeonShiftEffect::new()));
        registry.register(|| Box::new(effects::NeonSunsetEffect::new()));
        registry.register(|| Box::new(effects::NeonSunsetWaveEffect::new()));
        registry.register(|| Box::new(effects::PlasmaEffect::new()));
        registry.register(|| Box::new(effects::PoppingBubblesEffect::new()));
        registry.register(|| Box::new(effects::PsychedelicDreamEffect::new()));
        registry.register(|| Box::new(effects::RainbowDataEffect::new()));
        registry.register(|| Box::new(effects::RainbowEffect::new()));
        registry.register(|| Box::new(effects::RainstormEffect::new()));
        registry.register(|| Box::new(effects::RubyEffect::new()));
        registry.register(|| Box::new(effects::SakuraEffect::new()));
        registry.register(|| Box::new(effects::ScreenAmbienceEffect::new()));
        registry.register(|| Box::new(effects::SideToSideEffect::new()));
        registry.register(|| Box::new(effects::SolidColorEffect::new()));
        registry.register(|| Box::new(effects::SpaceEffect::new()));
        registry.register(|| Box::new(effects::SpinEffect::new()));
        registry.register(|| Box::new(effects::TerminalEffect::new()));
        registry.register(|| Box::new(effects::TitaniumEffect::new()));
        registry.register(|| Box::new(effects::TvStaticEffect::new()));
        registry.register(|| Box::new(effects::VaporWaveEffect::new()));
        registry.register(|| Box::new(effects::VoronoiEffect::new()));
        registry
    }

    /// Registers an effect constructor.
    pub fn register(&mut self, factory: EffectFactory) {
        let instance = factory();
        let metadata = instance.metadata();
        self.entries
            .insert(metadata.id, RegisteredEffect { metadata, factory });
    }

    /// Lists metadata for all registered effects in deterministic ID order.
    #[must_use]
    pub fn list(&self) -> Vec<EffectMetadata> {
        self.entries.values().map(|entry| entry.metadata).collect()
    }

    /// Instantiates an effect by its ID.
    ///
    /// # Errors
    ///
    /// Returns [`EffectsError::UnknownEffect`] if no effect with `id` is registered.
    pub fn create(&self, id: &str) -> Result<Box<dyn Effect>, EffectsError> {
        let entry = self
            .entries
            .get(id)
            .ok_or_else(|| EffectsError::UnknownEffect(id.to_owned()))?;
        Ok((entry.factory)())
    }
}
