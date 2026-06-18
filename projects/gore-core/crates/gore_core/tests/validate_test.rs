use gore_core::{
    gen::{MetaConfig, OverrideValue, OverridesConfig, SingleOverride},
    model::{Class, PropType, Property, ReflectionModel},
    validate::{validate_config, ValidationError},
};

fn make_model() -> ReflectionModel {
    let mut m = ReflectionModel::default();
    m.classes.push(Class {
        name: "ItFo_Apple".to_string(),
        parent: None,
        properties: vec![
            Property { name: "m_Value".to_string(), prop_type: PropType::Int, offset: None },
            Property { name: "m_Weight".to_string(), prop_type: PropType::Float, offset: None },
            Property { name: "m_Buoyancy".to_string(), prop_type: PropType::Bool, offset: None },
        ],
    });
    m
}

fn make_config(overrides: Vec<SingleOverride>) -> OverridesConfig {
    OverridesConfig {
        meta: MetaConfig { name: "Test".to_string(), delay_ms: 0 },
        overrides,
    }
}

#[test]
fn valid_config_no_errors() {
    let model = make_model();
    let cfg = make_config(vec![SingleOverride {
        module: "Angelscript".to_string(),
        class: "ItFo_Apple".to_string(),
        field: "m_Value".to_string(),
        value: OverrideValue::Int(500),
    }]);
    let errors = validate_config(&cfg, &model);
    assert!(errors.is_empty(), "expected no errors, got: {errors:?}");
}

#[test]
fn unknown_class_error() {
    let model = make_model();
    let cfg = make_config(vec![SingleOverride {
        module: "Angelscript".to_string(),
        class: "NonExistentClass".to_string(),
        field: "m_Value".to_string(),
        value: OverrideValue::Int(1),
    }]);
    let errors = validate_config(&cfg, &model);
    assert!(errors.iter().any(|e| matches!(e, ValidationError::UnknownClass { class, .. } if class == "NonExistentClass")));
}

#[test]
fn unknown_field_error() {
    let model = make_model();
    let cfg = make_config(vec![SingleOverride {
        module: "Angelscript".to_string(),
        class: "ItFo_Apple".to_string(),
        field: "m_NoSuchField".to_string(),
        value: OverrideValue::Int(1),
    }]);
    let errors = validate_config(&cfg, &model);
    assert!(errors.iter().any(|e| matches!(e, ValidationError::UnknownField { field, .. } if field == "m_NoSuchField")));
}

#[test]
fn type_mismatch_int_on_float_field_error() {
    let model = make_model();
    let cfg = make_config(vec![SingleOverride {
        module: "Angelscript".to_string(),
        class: "ItFo_Apple".to_string(),
        field: "m_Weight".to_string(), // Float field
        value: OverrideValue::Int(5),   // Int value — mismatch
    }]);
    let errors = validate_config(&cfg, &model);
    assert!(
        errors.iter().any(|e| matches!(e, ValidationError::TypeMismatch { .. })),
        "expected TypeMismatch, got: {errors:?}"
    );
}

#[test]
fn type_mismatch_float_on_bool_field_error() {
    let model = make_model();
    let cfg = make_config(vec![SingleOverride {
        module: "Angelscript".to_string(),
        class: "ItFo_Apple".to_string(),
        field: "m_Buoyancy".to_string(), // Bool field
        value: OverrideValue::Float(1.0), // Float value — mismatch
    }]);
    let errors = validate_config(&cfg, &model);
    assert!(errors.iter().any(|e| matches!(e, ValidationError::TypeMismatch { .. })));
}

#[test]
fn int_value_on_int_field_is_valid() {
    // Int value on Int field — no type error
    let model = make_model();
    let cfg = make_config(vec![SingleOverride {
        module: "Angelscript".to_string(),
        class: "ItFo_Apple".to_string(),
        field: "m_Value".to_string(),
        value: OverrideValue::Int(999),
    }]);
    assert!(validate_config(&cfg, &model).is_empty());
}

#[test]
fn multiple_errors_collected() {
    // Two bad overrides — both errors returned, not short-circuited
    let model = make_model();
    let cfg = make_config(vec![
        SingleOverride {
            module: "Angelscript".to_string(),
            class: "BadClass".to_string(),
            field: "m_Value".to_string(),
            value: OverrideValue::Int(1),
        },
        SingleOverride {
            module: "Angelscript".to_string(),
            class: "ItFo_Apple".to_string(),
            field: "m_BadField".to_string(),
            value: OverrideValue::Int(1),
        },
    ]);
    let errors = validate_config(&cfg, &model);
    assert_eq!(errors.len(), 2);
}
