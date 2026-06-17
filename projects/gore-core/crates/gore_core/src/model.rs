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
}
