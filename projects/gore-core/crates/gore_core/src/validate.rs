use crate::{
    gen::{OverrideValue, OverridesConfig},
    model::{PropType, ReflectionModel},
};

#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error("class '{class}' not found in reflection model")]
    UnknownClass { class: String },

    #[error("field '{field}' not found on class '{class}' or any ancestor")]
    UnknownField { class: String, field: String },

    #[error("field '{field}' on '{class}' has type {expected_type} but value is {actual_type}")]
    TypeMismatch {
        class: String,
        field: String,
        expected_type: String,
        actual_type: String,
    },
}

/// Validate every override in `cfg` against the reflection `model`.
/// Returns all errors (does not short-circuit on first error).
pub fn validate_config(cfg: &OverridesConfig, model: &ReflectionModel) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    for o in &cfg.overrides {
        // 1. Class exists?
        if model.find_class(&o.class).is_none() {
            errors.push(ValidationError::UnknownClass {
                class: o.class.clone(),
            });
            // Can't check field without a class — skip to next override
            continue;
        }

        // 2. Field exists (on class or ancestor)?
        let prop = model.find_property_inherited(&o.class, &o.field);
        if prop.is_none() {
            errors.push(ValidationError::UnknownField {
                class: o.class.clone(),
                field: o.field.clone(),
            });
            continue;
        }

        // 3. Type matches?
        let prop = prop.unwrap();
        if let Some(err) = check_type_match(&o.class, &o.field, &prop.prop_type, &o.value) {
            errors.push(err);
        }
    }

    errors
}

fn check_type_match(
    class: &str,
    field: &str,
    expected: &PropType,
    value: &OverrideValue,
) -> Option<ValidationError> {
    let ok = match (expected, value) {
        (PropType::Int, OverrideValue::Int(_)) => true,
        (PropType::Float, OverrideValue::Float(_)) => true,
        (PropType::Bool, OverrideValue::Bool(_)) => true,
        (PropType::String, OverrideValue::Str(_)) => true,
        // Opaque fields: we allow any value (we don't know the C++ layout)
        (PropType::Opaque(_), _) => true,
        // Enum fields: only Str values checked; enum member validation is
        // a future enhancement (requires enum model lookup + member check).
        (PropType::Enum(_), OverrideValue::Str(_)) => true,
        _ => false,
    };

    if ok {
        None
    } else {
        let expected_type = match expected {
            PropType::Int => "int".to_string(),
            PropType::Float => "float".to_string(),
            PropType::Bool => "bool".to_string(),
            PropType::String => "string".to_string(),
            PropType::Enum(e) => format!("enum({e})"),
            PropType::Opaque(o) => format!("opaque({o})"),
        };
        let actual_type = match value {
            OverrideValue::Int(_) => "int",
            OverrideValue::Float(_) => "float",
            OverrideValue::Bool(_) => "bool",
            OverrideValue::Str(_) => "string",
        };
        Some(ValidationError::TypeMismatch {
            class: class.to_string(),
            field: field.to_string(),
            expected_type,
            actual_type: actual_type.to_string(),
        })
    }
}
