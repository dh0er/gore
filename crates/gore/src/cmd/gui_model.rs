//! `gore-cli gui-model` — convert a reflection model + item catalog into a
//! gore-mod GUI shape JSON.
//!
//! Usage:
//!   gore-cli gui-model --model model.json --catalog item_catalog.json -o gui.json
//!
//! For each entry in the item catalog the command walks the reflection model's
//! parent chain (via `ReflectionModel::find_property_inherited`) to collect all
//! inherited properties, then keeps only GUI-editable scalars:
//!   Int   → "int"
//!   Float → "float"
//!   Bool  → "bool"
//!   Enum  → "enum"
//!   (String / Opaque / container types are skipped)
//!
//! Output shape:
//! ```json
//! { "classes": { "<itemId>": { "fields": [ {"name":"m_Value","type":"int"} ] } } }
//! ```

use anyhow::{Context, Result};
use gore_catalog::parse_catalog;
use gore_reflect::model::{PropType, ReflectionModel};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fs, path::PathBuf};

/// A single GUI-editable field. This is the on-disk gore-mod model shape;
/// `gore-cli sync` reads the same shape from a runtime dump and re-emits it,
/// so the struct is both serialized and deserialized.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuiField {
    pub name: String,
    #[serde(rename = "type")]
    pub field_type: String,
    /// Member names of an `enum` field, in declaration order. Empty for
    /// non-enum fields; skipped from the JSON so non-enum output stays
    /// byte-identical.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub enum_values: Vec<String>,
    /// Backing integer per enum member (parallel to `enum_values`). The GUI
    /// writes this value, not the member index, so non-contiguous discriminants
    /// (e.g. `Mid = 5`) round-trip correctly. Skipped when empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub enum_value_ints: Vec<i64>,
    /// The field's real default value, read from the live CDO by the runtime
    /// dumper (`gore-cli sync`). Absent for header-derived models (`gui-model`),
    /// in which case the GUI falls back to a placeholder. Skipped from JSON when
    /// absent so header-derived output stays byte-identical.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<serde_json::Value>,
}

/// GUI shape for one item class.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuiClass {
    pub fields: Vec<GuiField>,
}

/// Top-level GUI model output.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct GuiModel {
    pub classes: BTreeMap<String, GuiClass>,
}

/// Resolve a catalog id to the actual class name in the reflection model,
/// trying the bare id then the UE `U`-prefixed form.
fn resolve_class_name(model: &ReflectionModel, id: &str) -> Option<String> {
    if model.find_class(id).is_some() {
        return Some(id.to_string());
    }
    let prefixed = format!("U{id}");
    if model.find_class(&prefixed).is_some() {
        return Some(prefixed);
    }
    None
}

fn prop_type_to_gui(pt: &PropType) -> Option<&'static str> {
    match pt {
        PropType::Int => Some("int"),
        PropType::Float => Some("float"),
        PropType::Bool => Some("bool"),
        PropType::Enum(_) => Some("enum"),
        PropType::String | PropType::Opaque(_) => None,
    }
}

/// Collect all inherited properties for a class (DFS up the parent chain),
/// returning them in declaration order (base class first, derived last).
fn collect_inherited_properties<'a>(
    model: &'a ReflectionModel,
    class_name: &str,
) -> Vec<&'a gore_reflect::model::Property> {
    // Walk to root, then reverse to get base-first order
    let mut chain: Vec<&str> = Vec::new();
    let mut current = class_name;
    let mut visited = std::collections::HashSet::new();
    loop {
        if !visited.insert(current) {
            break; // cycle guard
        }
        chain.push(current);
        match model.find_class(current).and_then(|c| c.parent.as_deref()) {
            Some(p) => current = p,
            None => break,
        }
    }
    chain.reverse(); // base first
    chain
        .into_iter()
        .flat_map(|name| {
            model
                .find_class(name)
                .map(|c| c.properties.iter())
                .into_iter()
                .flatten()
        })
        .collect()
}

/// Collect the GUI-editable fields for a resolved class: walk the inherited
/// property chain (base-first), keep only scalar/enum types, and let a derived
/// class's definition override a shadowed base property (last-wins on the same
/// name, original field order preserved).
fn gui_fields_for(model: &ReflectionModel, class_name: &str) -> Vec<GuiField> {
    // Resolve each property name to its MOST-DERIVED type first (last-wins over
    // the base-first chain), preserving first-seen order. Only then filter to
    // GUI-editable types — so a base editable field shadowed by a derived
    // non-editable type (e.g. base Int -> derived FString/opaque) is dropped,
    // not left behind as a stale editable field.
    let props = collect_inherited_properties(model, class_name);
    let mut order: Vec<&str> = Vec::new();
    let mut type_by_name: std::collections::HashMap<&str, &PropType> =
        std::collections::HashMap::new();
    for prop in &props {
        if !type_by_name.contains_key(prop.name.as_str()) {
            order.push(prop.name.as_str());
        }
        type_by_name.insert(prop.name.as_str(), &prop.prop_type);
    }
    order
        .into_iter()
        .filter_map(|name| {
            let pt = type_by_name[name];
            let field_type = prop_type_to_gui(pt)?;
            // For enum fields, expose the dropdown ONLY when the backing values
            // are known — the GUI writes the discriminant, so without it we'd
            // emit member names the editor would (wrongly) encode as indices.
            // Enums with no/unreliable values (unparsed expressions) get no
            // choices and the GUI hides them.
            let (enum_values, enum_value_ints) = match pt {
                PropType::Enum(enum_name) => match model.find_enum(enum_name) {
                    Some(e) if !e.values.is_empty() => (e.members.clone(), e.values.clone()),
                    _ => (Vec::new(), Vec::new()),
                },
                _ => (Vec::new(), Vec::new()),
            };
            Some(GuiField {
                name: name.to_string(),
                field_type: field_type.to_string(),
                enum_values,
                enum_value_ints,
                default: None,
            })
        })
        .collect()
}

pub fn run(model_path: PathBuf, catalog_path: PathBuf, out: PathBuf) -> Result<()> {
    let model_json = fs::read_to_string(&model_path)
        .with_context(|| format!("reading model '{}'", model_path.display()))?;
    let model: ReflectionModel = serde_json::from_str(&model_json).context("parsing model.json")?;

    let catalog_json = fs::read_to_string(&catalog_path)
        .with_context(|| format!("reading catalog '{}'", catalog_path.display()))?;
    let catalog_entries = parse_catalog(&catalog_json).context("parsing item_catalog.json")?;

    let mut gui = GuiModel::default();

    for entry in &catalog_entries {
        // Catalog ids are Angelscript names (e.g. `ItMi_Orenugget`); the
        // reflection model (parsed from C++ headers) declares them with the UE
        // `U` class prefix (`UItMi_Orenugget`). Resolve either spelling.
        let Some(class_name) = resolve_class_name(&model, &entry.id) else {
            continue;
        };
        gui.classes.insert(
            entry.id.clone(),
            GuiClass {
                fields: gui_fields_for(&model, &class_name),
            },
        );
    }

    if let Some(parent) = out.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    let json = serde_json::to_string_pretty(&gui)?;
    fs::write(&out, format!("{json}\n")).with_context(|| format!("writing '{}'", out.display()))?;

    println!(
        "Wrote GUI model for {} classes -> {}",
        gui.classes.len(),
        out.display()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use gore_reflect::model::{Class, Enum, Property};

    fn make_model() -> ReflectionModel {
        ReflectionModel {
            classes: vec![
                Class {
                    name: "UItemBase".to_string(),
                    parent: None,
                    properties: vec![
                        Property {
                            name: "m_Value".to_string(),
                            prop_type: PropType::Int,
                            offset: Some(0x50),
                        },
                        Property {
                            name: "m_Weight".to_string(),
                            prop_type: PropType::Float,
                            offset: Some(0x54),
                        },
                        Property {
                            name: "m_Name".to_string(),
                            prop_type: PropType::String,
                            offset: Some(0x58),
                        },
                    ],
                },
                Class {
                    name: "ItMi_Gold".to_string(),
                    parent: Some("UItemBase".to_string()),
                    properties: vec![
                        Property {
                            name: "m_Amount".to_string(),
                            prop_type: PropType::Int,
                            offset: Some(0x80),
                        },
                        Property {
                            name: "m_Quality".to_string(),
                            prop_type: PropType::Enum("EItemQuality".to_string()),
                            offset: Some(0x84),
                        },
                    ],
                },
            ],
            enums: vec![Enum {
                name: "EItemQuality".to_string(),
                members: vec!["Low".to_string(), "Medium".to_string()],
                values: vec![0, 1],
            }],
        }
    }

    fn make_catalog_json() -> String {
        r#"[{"category":"misc","id":"ItMi_Gold","path":"/Script/Angelscript.ItMi_Gold"}]"#
            .to_string()
    }

    #[test]
    fn gui_model_inherits_properties() {
        let model = make_model();
        let catalog = parse_catalog(&make_catalog_json()).unwrap();

        let mut gui = GuiModel::default();
        for entry in &catalog {
            if model.find_class(&entry.id).is_none() {
                continue;
            }
            let props = collect_inherited_properties(&model, &entry.id);
            let mut seen: std::collections::HashSet<&str> = std::collections::HashSet::new();
            let mut fields = Vec::new();
            for prop in props {
                if !seen.insert(prop.name.as_str()) {
                    continue;
                }
                if let Some(t) = prop_type_to_gui(&prop.prop_type) {
                    fields.push(GuiField {
                        name: prop.name.clone(),
                        field_type: t.to_string(),
                        enum_values: Vec::new(),
                        enum_value_ints: Vec::new(),
                        default: None,
                    });
                }
            }
            gui.classes.insert(entry.id.clone(), GuiClass { fields });
        }

        let cls = gui.classes.get("ItMi_Gold").unwrap();
        let names: Vec<&str> = cls.fields.iter().map(|f| f.name.as_str()).collect();
        // Base class fields come first
        assert_eq!(names, ["m_Value", "m_Weight", "m_Amount", "m_Quality"]);
        // String field is excluded
        assert!(!names.contains(&"m_Name"));
        // Types
        let by_name: std::collections::HashMap<&str, &str> = cls
            .fields
            .iter()
            .map(|f| (f.name.as_str(), f.field_type.as_str()))
            .collect();
        assert_eq!(by_name["m_Value"], "int");
        assert_eq!(by_name["m_Weight"], "float");
        assert_eq!(by_name["m_Amount"], "int");
        assert_eq!(by_name["m_Quality"], "enum");
    }

    #[test]
    fn gui_model_skips_opaque() {
        let model = ReflectionModel {
            classes: vec![Class {
                name: "ItFo_Apple".to_string(),
                parent: None,
                properties: vec![
                    Property {
                        name: "m_Tags".to_string(),
                        prop_type: PropType::Opaque("GameplayTagContainer".to_string()),
                        offset: None,
                    },
                    Property {
                        name: "m_Saturation".to_string(),
                        prop_type: PropType::Float,
                        offset: None,
                    },
                ],
            }],
            enums: vec![],
        };
        let catalog = parse_catalog(
            r#"[{"category":"food","id":"ItFo_Apple","path":"/Script/Angelscript.ItFo_Apple"}]"#,
        )
        .unwrap();

        let props = collect_inherited_properties(&model, "ItFo_Apple");
        let mut fields = Vec::new();
        for prop in props {
            if let Some(t) = prop_type_to_gui(&prop.prop_type) {
                fields.push(GuiField {
                    name: prop.name.clone(),
                    field_type: t.to_string(),
                    enum_values: Vec::new(),
                    enum_value_ints: Vec::new(),
                    default: None,
                });
            }
        }
        // m_Tags (Opaque) must be excluded
        assert!(!fields.iter().any(|f| f.name == "m_Tags"));
        assert!(fields.iter().any(|f| f.name == "m_Saturation"));
        let _ = catalog; // used above for context
    }

    #[test]
    fn resolves_u_prefixed_class_from_bare_catalog_id() {
        // Model declares the class with the UE `U` prefix; catalog id is bare.
        let model = ReflectionModel {
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
        };
        assert_eq!(
            resolve_class_name(&model, "ItMi_Orenugget").as_deref(),
            Some("UItMi_Orenugget")
        );
        assert_eq!(resolve_class_name(&model, "Nope"), None);
    }

    #[test]
    fn derived_class_overrides_base_field_type() {
        // Base declares m_X as Int; derived re-declares m_X as Float. The
        // derived definition must win, in the base field's original position.
        let model = ReflectionModel {
            classes: vec![
                Class {
                    name: "UBase".to_string(),
                    parent: None,
                    properties: vec![
                        Property {
                            name: "m_X".to_string(),
                            prop_type: PropType::Int,
                            offset: None,
                        },
                        Property {
                            name: "m_Y".to_string(),
                            prop_type: PropType::Bool,
                            offset: None,
                        },
                    ],
                },
                Class {
                    name: "UDerived".to_string(),
                    parent: Some("UBase".to_string()),
                    properties: vec![Property {
                        name: "m_X".to_string(),
                        prop_type: PropType::Float,
                        offset: None,
                    }],
                },
            ],
            enums: vec![],
        };
        let fields = gui_fields_for(&model, "UDerived");
        let names: Vec<&str> = fields.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, ["m_X", "m_Y"], "no duplicate m_X; base order kept");
        let m_x = fields.iter().find(|f| f.name == "m_X").unwrap();
        assert_eq!(
            m_x.field_type, "float",
            "derived Float must override base Int"
        );
    }

    #[test]
    fn derived_nonenum_shadow_drops_base_editable_field() {
        // Base m_X is editable (Int); derived re-declares m_X as a String
        // (non-GUI). The field must NOT appear as editable in the GUI model.
        let model = ReflectionModel {
            classes: vec![
                Class {
                    name: "UBase".to_string(),
                    parent: None,
                    properties: vec![
                        Property {
                            name: "m_X".to_string(),
                            prop_type: PropType::Int,
                            offset: None,
                        },
                        Property {
                            name: "m_Y".to_string(),
                            prop_type: PropType::Bool,
                            offset: None,
                        },
                    ],
                },
                Class {
                    name: "UDerived".to_string(),
                    parent: Some("UBase".to_string()),
                    properties: vec![Property {
                        name: "m_X".to_string(),
                        prop_type: PropType::String,
                        offset: None,
                    }],
                },
            ],
            enums: vec![],
        };
        let fields = gui_fields_for(&model, "UDerived");
        let names: Vec<&str> = fields.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(
            names,
            ["m_Y"],
            "shadowed-by-String m_X must be dropped: {names:?}"
        );
    }

    #[test]
    fn unreliable_enum_without_values_is_not_exposed() {
        // An enum whose backing values couldn't be parsed (empty values) must
        // emit no choices, so the GUI hides it rather than writing wrong indices.
        let model = ReflectionModel {
            classes: vec![Class {
                name: "UThing".to_string(),
                parent: None,
                properties: vec![Property {
                    name: "m_Q".to_string(),
                    prop_type: PropType::Enum("EFlags".to_string()),
                    offset: None,
                }],
            }],
            enums: vec![Enum {
                name: "EFlags".to_string(),
                members: vec!["A".to_string(), "B".to_string()],
                values: vec![], // unreliable -> dropped by the parser
            }],
        };
        let fields = gui_fields_for(&model, "UThing");
        let q = fields.iter().find(|f| f.name == "m_Q").unwrap();
        assert!(q.enum_values.is_empty());
        assert!(q.enum_value_ints.is_empty());
    }

    #[test]
    fn enum_field_carries_members_in_declaration_order() {
        let model = make_model();
        let fields = gui_fields_for(&model, "ItMi_Gold");
        let quality = fields.iter().find(|f| f.name == "m_Quality").unwrap();
        assert_eq!(quality.field_type, "enum");
        assert_eq!(quality.enum_values, ["Low", "Medium"]);
        assert_eq!(quality.enum_value_ints, [0, 1]);
        // Non-enum fields carry no choices.
        let value = fields.iter().find(|f| f.name == "m_Value").unwrap();
        assert!(value.enum_values.is_empty());
    }

    #[test]
    fn enum_values_omitted_from_json_for_non_enum_fields() {
        // skip_serializing_if keeps non-enum output byte-identical (no key).
        let f = GuiField {
            name: "m_Value".into(),
            field_type: "int".into(),
            enum_values: vec![],
            enum_value_ints: vec![],
            default: None,
        };
        assert_eq!(
            serde_json::to_string(&f).unwrap(),
            r#"{"name":"m_Value","type":"int"}"#
        );
        let e = GuiField {
            name: "m_Quality".into(),
            field_type: "enum".into(),
            enum_values: vec!["Low".into(), "High".into()],
            enum_value_ints: vec![5, 9],
            default: None,
        };
        assert_eq!(
            serde_json::to_string(&e).unwrap(),
            r#"{"name":"m_Quality","type":"enum","enum_values":["Low","High"],"enum_value_ints":[5,9]}"#
        );
    }

    #[test]
    fn unresolved_enum_yields_no_choices() {
        // Enum field whose named enum isn't in the model -> empty members, which
        // the GUI then skips rather than rendering an empty dropdown.
        let model = ReflectionModel {
            classes: vec![Class {
                name: "UThing".to_string(),
                parent: None,
                properties: vec![Property {
                    name: "m_Q".to_string(),
                    prop_type: PropType::Enum("EMissing".to_string()),
                    offset: None,
                }],
            }],
            enums: vec![],
        };
        let fields = gui_fields_for(&model, "UThing");
        assert_eq!(fields[0].field_type, "enum");
        assert!(fields[0].enum_values.is_empty());
    }

    #[test]
    fn prop_type_mapping() {
        assert_eq!(prop_type_to_gui(&PropType::Int), Some("int"));
        assert_eq!(prop_type_to_gui(&PropType::Float), Some("float"));
        assert_eq!(prop_type_to_gui(&PropType::Bool), Some("bool"));
        assert_eq!(
            prop_type_to_gui(&PropType::Enum("EFoo".to_string())),
            Some("enum")
        );
        assert_eq!(prop_type_to_gui(&PropType::String), None);
        assert_eq!(
            prop_type_to_gui(&PropType::Opaque("GameplayTagContainer".to_string())),
            None
        );
    }
}
