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
use gore_core::{catalog::parse_catalog, model::{PropType, ReflectionModel}};
use serde::Serialize;
use std::{collections::BTreeMap, fs, path::PathBuf};

/// A single GUI-editable field.
#[derive(Debug, Clone, Serialize)]
pub struct GuiField {
    pub name: String,
    #[serde(rename = "type")]
    pub field_type: String,
}

/// GUI shape for one item class.
#[derive(Debug, Clone, Serialize)]
pub struct GuiClass {
    pub fields: Vec<GuiField>,
}

/// Top-level GUI model output.
#[derive(Debug, Default, Serialize)]
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
) -> Vec<&'a gore_core::model::Property> {
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
    let props = collect_inherited_properties(model, class_name);
    let mut idx_by_name: std::collections::HashMap<&str, usize> =
        std::collections::HashMap::new();
    let mut fields: Vec<GuiField> = Vec::new();
    for prop in props {
        let Some(type_str) = prop_type_to_gui(&prop.prop_type) else {
            continue;
        };
        if let Some(&i) = idx_by_name.get(prop.name.as_str()) {
            fields[i].field_type = type_str.to_string();
        } else {
            idx_by_name.insert(prop.name.as_str(), fields.len());
            fields.push(GuiField {
                name: prop.name.clone(),
                field_type: type_str.to_string(),
            });
        }
    }
    fields
}

pub fn run(model_path: PathBuf, catalog_path: PathBuf, out: PathBuf) -> Result<()> {
    let model_json = fs::read_to_string(&model_path)
        .with_context(|| format!("reading model '{}'", model_path.display()))?;
    let model: ReflectionModel = serde_json::from_str(&model_json)
        .context("parsing model.json")?;

    let catalog_json = fs::read_to_string(&catalog_path)
        .with_context(|| format!("reading catalog '{}'", catalog_path.display()))?;
    let catalog_entries = parse_catalog(&catalog_json)
        .context("parsing item_catalog.json")?;

    let mut gui = GuiModel::default();

    for entry in &catalog_entries {
        // Catalog ids are Angelscript names (e.g. `ItMi_Orenugget`); the
        // reflection model (parsed from C++ headers) declares them with the UE
        // `U` class prefix (`UItMi_Orenugget`). Resolve either spelling.
        let Some(class_name) = resolve_class_name(&model, &entry.id) else {
            continue;
        };
        gui.classes
            .insert(entry.id.clone(), GuiClass { fields: gui_fields_for(&model, &class_name) });
    }

    if let Some(parent) = out.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    let json = serde_json::to_string_pretty(&gui)?;
    fs::write(&out, format!("{json}\n"))
        .with_context(|| format!("writing '{}'", out.display()))?;

    println!("Wrote GUI model for {} classes -> {}", gui.classes.len(), out.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use gore_core::model::{Class, Enum, Property};

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
                    fields.push(GuiField { name: prop.name.clone(), field_type: t.to_string() });
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
        let by_name: std::collections::HashMap<&str, &str> =
            cls.fields.iter().map(|f| (f.name.as_str(), f.field_type.as_str())).collect();
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
        let catalog =
            parse_catalog(r#"[{"category":"food","id":"ItFo_Apple","path":"/Script/Angelscript.ItFo_Apple"}]"#)
                .unwrap();

        let props = collect_inherited_properties(&model, "ItFo_Apple");
        let mut fields = Vec::new();
        for prop in props {
            if let Some(t) = prop_type_to_gui(&prop.prop_type) {
                fields.push(GuiField { name: prop.name.clone(), field_type: t.to_string() });
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
                        Property { name: "m_X".to_string(), prop_type: PropType::Int, offset: None },
                        Property { name: "m_Y".to_string(), prop_type: PropType::Bool, offset: None },
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
        assert_eq!(m_x.field_type, "float", "derived Float must override base Int");
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
