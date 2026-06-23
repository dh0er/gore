use gore_reflect::model::{Class, PropType, Property, ReflectionModel};
use gore_reflect::parser::parse_hpp_file;
use std::path::PathBuf;

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

#[test]
fn model_lookup_by_name() {
    let mut model = ReflectionModel::default();
    let cls = Class {
        name: "UItemDefinition".to_string(),
        parent: Some("UGothicObjectDefinition".to_string()),
        properties: vec![
            Property {
                name: "m_Value".to_string(),
                prop_type: PropType::Int,
                offset: None,
            },
            Property {
                name: "m_Weight".to_string(),
                prop_type: PropType::Float,
                offset: None,
            },
        ],
    };
    model.classes.push(cls);

    let found = model.find_class("UItemDefinition").expect("should find class");
    assert_eq!(found.name, "UItemDefinition");
    assert_eq!(found.properties.len(), 2);
    assert!(found.find_property("m_Value").is_some());
    assert!(found.find_property("m_NonExistent").is_none());
}

#[test]
fn model_find_property_on_ancestor() {
    // UItemDefinition inherits from UGothicObjectDefinition; find_property_inherited
    // must walk the parent chain.
    let mut model = ReflectionModel::default();
    model.classes.push(Class {
        name: "UGothicObjectDefinition".to_string(),
        parent: None,
        properties: vec![Property {
            name: "m_Name".to_string(),
            prop_type: PropType::String,
            offset: None,
        }],
    });
    model.classes.push(Class {
        name: "UItemDefinition".to_string(),
        parent: Some("UGothicObjectDefinition".to_string()),
        properties: vec![Property {
            name: "m_Value".to_string(),
            prop_type: PropType::Int,
            offset: None,
        }],
    });

    let prop = model
        .find_property_inherited("UItemDefinition", "m_Name")
        .expect("inherited prop must be found");
    assert_eq!(prop.name, "m_Name");
    assert!(matches!(prop.prop_type, PropType::String));
}

#[test]
fn parse_snippet_class_count() {
    let model = parse_hpp_file(&fixture_path("snippet.hpp")).unwrap();
    // 3 classes in the fixture
    assert_eq!(model.classes.len(), 3);
}

#[test]
fn parse_snippet_class_parent() {
    let model = parse_hpp_file(&fixture_path("snippet.hpp")).unwrap();
    let item_def = model.find_class("UItemDefinition").expect("class must exist");
    assert_eq!(item_def.parent.as_deref(), Some("UGothicObjectDefinition"));
}

#[test]
fn parse_snippet_properties() {
    let model = parse_hpp_file(&fixture_path("snippet.hpp")).unwrap();
    let item_def = model.find_class("UItemDefinition").unwrap();
    assert_eq!(item_def.properties.len(), 5);

    let val = item_def.find_property("m_Value").unwrap();
    assert!(matches!(val.prop_type, PropType::Int));

    let w = item_def.find_property("m_Weight").unwrap();
    assert!(matches!(w.prop_type, PropType::Float));

    let b = item_def.find_property("m_Buoyancy").unwrap();
    assert!(matches!(b.prop_type, PropType::Bool));
}

#[test]
fn parse_snippet_empty_class_body() {
    // ItFo_Apple has no leaf properties — still parses cleanly
    let model = parse_hpp_file(&fixture_path("snippet.hpp")).unwrap();
    let apple = model.find_class("ItFo_Apple").unwrap();
    assert_eq!(apple.parent.as_deref(), Some("UItemDefinition"));
    assert_eq!(apple.properties.len(), 0);
}

#[test]
fn parse_snippet_enum() {
    let model = parse_hpp_file(&fixture_path("snippet.hpp")).unwrap();
    let e = model.find_enum("EItemQuality").unwrap();
    assert_eq!(e.members, vec!["Low", "Medium", "High"]);
}
