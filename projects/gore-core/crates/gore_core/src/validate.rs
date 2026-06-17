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
        // 1. Class exists? — try the bare id first, then the UE U-prefix variant
        //    (gui-model stores classes as "UFoo" but overrides may say "Foo").
        let resolved_class = if model.find_class(&o.class).is_some() {
            o.class.clone()
        } else {
            let u_prefixed = format!("U{}", o.class);
            if model.find_class(&u_prefixed).is_some() {
                u_prefixed
            } else {
                errors.push(ValidationError::UnknownClass {
                    class: o.class.clone(),
                });
                // Can't check field without a class — skip to next override
                continue;
            }
        };

        // 2. Field exists (on class or ancestor)?
        let prop = model.find_property_inherited(&resolved_class, &o.field);
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

#[cfg(test)]
mod validate_tests {
    use super::*;
    use crate::{
        gen::{MetaConfig, OverrideValue, OverridesConfig, SingleOverride},
        model::{Class, PropType, Property, ReflectionModel},
    };

    fn make_model_with_u_prefix() -> ReflectionModel {
        ReflectionModel {
            classes: vec![Class {
                name: "UItMi_Orenugget".to_string(),
                parent: None,
                properties: vec![Property {
                    name: "m_Value".to_string(),
                    prop_type: PropType::Int,
                    offset: None,
                }],
            }],
            enums: vec![],
        }
    }

    fn make_config(class: &str, field: &str, value: OverrideValue) -> OverridesConfig {
        OverridesConfig {
            meta: MetaConfig { name: "TestMod".to_string(), delay_ms: 0 },
            overrides: vec![SingleOverride {
                class: class.to_string(),
                field: field.to_string(),
                value,
            }],
        }
    }

    #[test]
    fn bare_id_resolves_to_u_prefixed_class() {
        let model = make_model_with_u_prefix();
        // class in config = "ItMi_Orenugget" (no U prefix)
        // model has "UItMi_Orenugget"
        let cfg = make_config("ItMi_Orenugget", "m_Value", OverrideValue::Int(100));
        let errors = validate_config(&cfg, &model);
        assert!(
            errors.is_empty(),
            "expected no validation errors for bare-id override, got: {errors:?}"
        );
    }

    #[test]
    fn fully_prefixed_class_still_works() {
        let model = make_model_with_u_prefix();
        let cfg = make_config("UItMi_Orenugget", "m_Value", OverrideValue::Int(100));
        let errors = validate_config(&cfg, &model);
        assert!(errors.is_empty(), "expected no errors for U-prefixed override: {errors:?}");
    }

    #[test]
    fn truly_unknown_class_still_errors() {
        let model = make_model_with_u_prefix();
        let cfg = make_config("DoesNotExist", "m_Value", OverrideValue::Int(1));
        let errors = validate_config(&cfg, &model);
        assert!(
            errors.iter().any(|e| matches!(e, ValidationError::UnknownClass { .. })),
            "expected UnknownClass error, got: {errors:?}"
        );
    }
}
