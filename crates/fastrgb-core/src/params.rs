//! Declarative effect/device parameter descriptors and runtime parameter storage.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::color::Rgb;
use crate::error::CoreError;

/// Metadata describing the type, bounds, and default value of a parameter.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ParameterKind {
    /// Floating-point numeric slider.
    Number {
        /// Minimum allowed value.
        min: f32,
        /// Maximum allowed value.
        max: f32,
        /// Suggested UI step increment.
        step: f32,
        /// Default value.
        default: f32,
    },
    /// RGB color picker.
    Color {
        /// Default color.
        default: Rgb,
    },
    /// Boolean toggle switch.
    Boolean {
        /// Default state.
        default: bool,
    },
    /// Discrete string selection dropdown.
    Combobox {
        /// Ordered list of valid option strings.
        options: Vec<String>,
        /// Default option string.
        default: String,
    },
}

/// Declarative definition of an adjustable parameter exposed by an effect or device.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParameterDescriptor {
    /// Stable programmatic identifier (e.g. `"speed"`, `"picture_mode"`).
    pub id: String,
    /// Human-readable label shown in the UI.
    pub label: String,
    /// Optional tooltip / description.
    pub tooltip: Option<String>,
    /// Type and constraints of the parameter.
    pub kind: ParameterKind,
}

impl ParameterDescriptor {
    /// Builds a numeric slider descriptor.
    #[must_use]
    pub fn number(
        id: impl Into<String>,
        label: impl Into<String>,
        min: f32,
        max: f32,
        step: f32,
        default: f32,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            tooltip: None,
            kind: ParameterKind::Number {
                min,
                max,
                step,
                default,
            },
        }
    }

    /// Builds a color descriptor.
    #[must_use]
    pub fn color(id: impl Into<String>, label: impl Into<String>, default: Rgb) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            tooltip: None,
            kind: ParameterKind::Color { default },
        }
    }

    /// Builds a boolean toggle descriptor.
    #[must_use]
    pub fn boolean(id: impl Into<String>, label: impl Into<String>, default: bool) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            tooltip: None,
            kind: ParameterKind::Boolean { default },
        }
    }

    /// Builds a combobox selector descriptor.
    #[must_use]
    pub fn combobox(
        id: impl Into<String>,
        label: impl Into<String>,
        options: Vec<String>,
        default: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            tooltip: None,
            kind: ParameterKind::Combobox {
                options,
                default: default.into(),
            },
        }
    }

    /// Attaches an optional tooltip to the descriptor.
    #[must_use]
    pub fn with_tooltip(mut self, tooltip: impl Into<String>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }

    /// Returns the default [`ParameterValue`] implied by this descriptor.
    #[must_use]
    pub fn default_value(&self) -> ParameterValue {
        match &self.kind {
            ParameterKind::Number { default, .. } => ParameterValue::Number(*default),
            ParameterKind::Color { default } => ParameterValue::Color(*default),
            ParameterKind::Boolean { default } => ParameterValue::Boolean(*default),
            ParameterKind::Combobox { default, .. } => ParameterValue::Combobox(default.clone()),
        }
    }
}

/// Concrete runtime value for a parameter.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ParameterValue {
    /// Numeric value.
    Number(f32),
    /// RGB color value.
    Color(Rgb),
    /// Boolean value.
    Boolean(bool),
    /// Selected option value.
    Combobox(String),
}

impl ParameterValue {
    /// Returns the numeric value if this is [`ParameterValue::Number`].
    #[must_use]
    pub const fn as_f32(&self) -> Option<f32> {
        match self {
            Self::Number(v) => Some(*v),
            _ => None,
        }
    }

    /// Returns the color value if this is [`ParameterValue::Color`].
    #[must_use]
    pub const fn as_color(&self) -> Option<Rgb> {
        match self {
            Self::Color(c) => Some(*c),
            _ => None,
        }
    }

    /// Returns the boolean value if this is [`ParameterValue::Boolean`].
    #[must_use]
    pub const fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Boolean(b) => Some(*b),
            _ => None,
        }
    }

    /// Returns the selected option if this is [`ParameterValue::Combobox`].
    #[must_use]
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::Combobox(s) => Some(s.as_str()),
            _ => None,
        }
    }
}

/// Validated runtime store of parameter values keyed by parameter ID.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ParameterStore {
    values: BTreeMap<String, ParameterValue>,
}

impl ParameterStore {
    /// Initializes a store with default values for every descriptor.
    #[must_use]
    pub fn from_descriptors(descriptors: &[ParameterDescriptor]) -> Self {
        let mut values = BTreeMap::new();
        for desc in descriptors {
            values.insert(desc.id.clone(), desc.default_value());
        }
        Self { values }
    }

    /// Validates and sets a parameter value against its descriptor.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::InvalidParameter`] if the parameter ID is unknown or type mismatches.
    pub fn set_validated(
        &mut self,
        descriptor: &ParameterDescriptor,
        value: ParameterValue,
    ) -> Result<(), CoreError> {
        let validated = match (&descriptor.kind, value) {
            (ParameterKind::Number { min, max, .. }, ParameterValue::Number(v)) => {
                ParameterValue::Number(v.clamp(*min, *max))
            }
            (ParameterKind::Color { .. }, ParameterValue::Color(c)) => ParameterValue::Color(c),
            (ParameterKind::Boolean { .. }, ParameterValue::Boolean(b)) => {
                ParameterValue::Boolean(b)
            }
            (ParameterKind::Combobox { options, .. }, ParameterValue::Combobox(selected)) => {
                if !options.iter().any(|opt| opt == &selected) {
                    return Err(CoreError::InvalidParameter {
                        id: descriptor.id.clone(),
                        reason: format!("option '{selected}' is not in {options:?}"),
                    });
                }
                ParameterValue::Combobox(selected)
            }
            (_, other) => {
                return Err(CoreError::InvalidParameter {
                    id: descriptor.id.clone(),
                    reason: format!("incompatible value {other:?}"),
                });
            }
        };

        self.values.insert(descriptor.id.clone(), validated);
        Ok(())
    }

    /// Looks up a value by parameter ID.
    #[must_use]
    pub fn get(&self, id: &str) -> Option<&ParameterValue> {
        self.values.get(id)
    }

    /// Returns a numeric parameter or `fallback`.
    #[must_use]
    pub fn f32_or(&self, id: &str, fallback: f32) -> f32 {
        self.get(id)
            .and_then(ParameterValue::as_f32)
            .unwrap_or(fallback)
    }

    /// Returns a color parameter or `fallback`.
    #[must_use]
    pub fn color_or(&self, id: &str, fallback: Rgb) -> Rgb {
        self.get(id)
            .and_then(ParameterValue::as_color)
            .unwrap_or(fallback)
    }

    /// Returns a boolean parameter or `fallback`.
    #[must_use]
    pub fn bool_or(&self, id: &str, fallback: bool) -> bool {
        self.get(id)
            .and_then(ParameterValue::as_bool)
            .unwrap_or(fallback)
    }

    /// Returns a combobox parameter or `fallback`.
    #[must_use]
    pub fn str_or<'a>(&'a self, id: &str, fallback: &'a str) -> &'a str {
        self.get(id)
            .and_then(ParameterValue::as_str)
            .unwrap_or(fallback)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parameter_store_clamps_numbers_and_rejects_bad_combo() {
        let descriptors = vec![
            ParameterDescriptor::number("speed", "Speed", 0.0, 100.0, 1.0, 50.0),
            ParameterDescriptor::combobox("mode", "Mode", vec!["A".into(), "B".into()], "A"),
        ];
        let mut store = ParameterStore::from_descriptors(&descriptors);
        store
            .set_validated(&descriptors[0], ParameterValue::Number(250.0))
            .unwrap();
        assert_eq!(store.f32_or("speed", 0.0), 100.0);

        let err = store
            .set_validated(&descriptors[1], ParameterValue::Combobox("Z".into()))
            .unwrap_err();
        assert!(matches!(err, CoreError::InvalidParameter { .. }));
    }
}
