use serde::{Deserialize, Serialize};

/// The complete parsed reflection model from the SDK dump.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ReflectionModel {
    pub classes: Vec<Class>,
    pub enums: Vec<Enum>,
}

impl ReflectionModel {
    pub fn find_class(&self, name: &str) -> Option<&Class> {
        self.classes.iter().find(|c| c.name == name)
    }

    /// Walk the parent chain to find a property (stops at root or cycle).
    pub fn find_property_inherited<'a>(
        &'a self,
        class_name: &str,
        prop_name: &str,
    ) -> Option<&'a Property> {
        let mut current = class_name;
        let mut visited = std::collections::HashSet::new();
        loop {
            if !visited.insert(current.to_string()) {
                return None; // cycle guard
            }
            let cls = self.find_class(current)?;
            if let Some(p) = cls.find_property(prop_name) {
                return Some(p);
            }
            match &cls.parent {
                Some(p) => current = p.as_str(),
                None => return None,
            }
        }
    }

    pub fn find_enum(&self, name: &str) -> Option<&Enum> {
        self.enums.iter().find(|e| e.name == name)
    }

    /// Resolve `Opaque(name)` property types to `Enum(name)` for any type name
    /// matching a known enum in this model. Must run on a FULLY-assembled model
    /// — in particular after `gore-cli dump` merges per-file models, because an
    /// enum may be declared in a different `.hpp` than the class that uses it.
    pub fn resolve_enum_types(&mut self) {
        let enum_names: std::collections::HashSet<&str> =
            self.enums.iter().map(|e| e.name.as_str()).collect();
        for class in &mut self.classes {
            for prop in &mut class.properties {
                if let PropType::Opaque(name) = &prop.prop_type {
                    if enum_names.contains(name.as_str()) {
                        prop.prop_type = PropType::Enum(name.clone());
                    }
                }
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Class {
    pub name: String,
    pub parent: Option<String>,
    pub properties: Vec<Property>,
}

impl Class {
    pub fn find_property(&self, name: &str) -> Option<&Property> {
        self.properties.iter().find(|p| p.name == name)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Property {
    pub name: String,
    pub prop_type: PropType,
    pub offset: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PropType {
    Int,
    Float,
    Bool,
    String,
    /// Named enum type; value checked against Enum::members.
    Enum(String),
    /// Anything else from the SDK that we don't typecheck (maps, arrays, structs).
    Opaque(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Enum {
    pub name: String,
    pub members: Vec<String>,
    /// Backing integer per member (parallel to `members`). UE enums may have
    /// explicit/non-contiguous discriminants, so the index is not the value.
    /// Empty in older models; consumers then fall back to the index.
    #[serde(default)]
    pub values: Vec<i64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_enum_types_promotes_opaque_matching_known_enum() {
        // Simulates the merged-dump case: a class whose enum field was parsed as
        // Opaque (the enum lived in a different .hpp) gets promoted to Enum once
        // the enum declaration is present in the combined model.
        let mut model = ReflectionModel {
            classes: vec![Class {
                name: "UThing".to_string(),
                parent: None,
                properties: vec![
                    Property {
                        name: "m_Quality".to_string(),
                        prop_type: PropType::Opaque("EQuality".to_string()),
                        offset: None,
                    },
                    Property {
                        name: "m_Tags".to_string(),
                        prop_type: PropType::Opaque("FGameplayTagContainer".to_string()),
                        offset: None,
                    },
                ],
            }],
            enums: vec![Enum {
                name: "EQuality".to_string(),
                members: vec!["Low".into(), "High".into()],
                values: vec![0, 1],
            }],
        };
        model.resolve_enum_types();
        let props = &model.classes[0].properties;
        assert_eq!(props[0].prop_type, PropType::Enum("EQuality".to_string()));
        // A non-enum opaque type stays opaque.
        assert_eq!(
            props[1].prop_type,
            PropType::Opaque("FGameplayTagContainer".to_string())
        );
    }
}
