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
        // 1. Class exists? Resolve the bare/UE spellings symmetrically: the
        //    model may carry either the UE `U`-prefixed name (`UItMi_Orenugget`)
        //    or the bare AngelScript name (`ItMi_Orenugget`, as some SDK fixtures
        //    parse), and the override may use either, so try the name as-is, with
        //    a `U` added, and with a leading `U` stripped.
        let stripped = o.class.strip_prefix('U').filter(|r| {
            r.chars().next().is_some_and(|c| c.is_ascii_uppercase())
        });
        let candidates = [
            o.class.clone(),
            format!("U{}", o.class),
            stripped.map(str::to_string).unwrap_or_default(),
        ];
        let resolved_class = candidates
            .iter()
            .find(|c| !c.is_empty() && model.find_class(c).is_some())
            .cloned();
        let Some(resolved_class) = resolved_class else {
            errors.push(ValidationError::UnknownClass {
                class: o.class.clone(),
            });
            // Can't check field without a class — skip to next override
            continue;
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
        // Enum fields accept a member name (Str) or its backing integer (UE
        // enums are int-backed, and overrides.toml uses the same value_int key
        // as elsewhere). Member-name validation is a future enhancement.
        (PropType::Enum(_), OverrideValue::Str(_) | OverrideValue::Int(_)) => true,
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

    #[test]
    fn u_prefixed_override_resolves_against_bare_model() {
        // Model carries the bare AngelScript name; override uses the UE spelling.
        let model = ReflectionModel {
            classes: vec![Class {
                name: "ItFo_Apple".to_string(),
                parent: None,
                properties: vec![Property {
                    name: "m_Value".to_string(),
                    prop_type: PropType::Int,
                    offset: None,
                }],
            }],
            enums: vec![],
        };
        let cfg = make_config("UItFo_Apple", "m_Value", OverrideValue::Int(7));
        assert!(
            validate_config(&cfg, &model).is_empty(),
            "U-spelled override must resolve against a bare model class"
        );
    }

    #[test]
    fn enum_field_accepts_int_and_str() {
        // UE enums are int-backed; an enum field must accept value_int as well
        // as a member name (value_str).
        let model = ReflectionModel {
            classes: vec![Class {
                name: "UThing".to_string(),
                parent: None,
                properties: vec![Property {
                    name: "m_Quality".to_string(),
                    prop_type: PropType::Enum("EQuality".to_string()),
                    offset: None,
                }],
            }],
            enums: vec![],
        };
        let int_cfg = make_config("UThing", "m_Quality", OverrideValue::Int(2));
        assert!(
            validate_config(&int_cfg, &model).is_empty(),
            "enum field must accept an int override"
        );
        let str_cfg = make_config("UThing", "m_Quality", OverrideValue::Str("High".into()));
        assert!(
            validate_config(&str_cfg, &model).is_empty(),
            "enum field must accept a string member override"
        );
        // A bool is still rejected.
        let bad = make_config("UThing", "m_Quality", OverrideValue::Bool(true));
        assert!(
            !validate_config(&bad, &model).is_empty(),
            "enum field must reject a bool override"
        );
    }
}
