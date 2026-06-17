//! Line-by-line parser for UE4SS `CXXHeaderDump/*.hpp` files.
//!
//! Parses class declarations, their public fields, and enum declarations.
//! Does NOT parse function signatures (not needed for CDO overrides).

use std::{
    fs::File,
    io::{BufRead, BufReader},
    path::Path,
};

use crate::model::{Class, Enum, PropType, Property, ReflectionModel};

#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("IO error reading {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
}

/// Parse a single `.hpp` file into a `ReflectionModel`.
/// For a full SDK dump, call this for each file and merge results.
pub fn parse_hpp_file(path: &Path) -> Result<ReflectionModel, ParseError> {
    let f = File::open(path).map_err(|e| ParseError::Io {
        path: path.display().to_string(),
        source: e,
    })?;
    parse_hpp_reader(BufReader::new(f))
}

pub fn parse_hpp_reader<R: BufRead>(reader: R) -> Result<ReflectionModel, ParseError> {
    let mut model = ReflectionModel::default();
    let mut in_class: Option<ClassBuilder> = None;
    let mut in_enum: Option<EnumBuilder> = None;
    let mut in_public = false;

    for raw in reader.lines() {
        let line = raw.map_err(|e| ParseError::Io {
            path: "<reader>".into(),
            source: e,
        })?;
        let trimmed = line.trim();

        // --- class header comment ----------------------------------------
        // "// Class Angelscript.ItFo_Apple"
        if trimmed.starts_with("// Class ") {
            if let Some(b) = in_class.take() {
                model.classes.push(b.build());
            }
            in_enum = None;
            in_public = false;
            // ignore — real declaration is on the `class X : public Y` line
            continue;
        }

        // --- enum header comment ------------------------------------------
        // "// Enum Angelscript.EItemQuality"
        if trimmed.starts_with("// Enum ") {
            if let Some(b) = in_class.take() {
                model.classes.push(b.build());
            }
            in_enum = None;
            in_public = false;
            continue;
        }

        // --- class declaration --------------------------------------------
        // "class ItFo_Apple : public UItemDefinition"
        // Must distinguish a real declaration from a member/method that merely
        // begins with a `class Foo*`/`class Foo(` token (UE headers write
        // `class UFoo* m_x;` for forward-declared pointer members). A real decl
        // has no '{', '*', '(' and does not end with ';'.
        if trimmed.starts_with("class ") {
            let decl = match trimmed.find("//") {
                Some(i) => trimmed[..i].trim(),
                None => trimmed,
            };
            let is_real_decl = !decl.contains('{')
                && !decl.contains('*')
                && !decl.contains('(')
                && !decl.ends_with(';');
            if is_real_decl {
                if let Some(b) = in_class.take() {
                    model.classes.push(b.build());
                }
                let rest = &decl["class ".len()..];
                let (name, parent) = if let Some(idx) = rest.find(": public ") {
                    let n = rest[..idx].trim().to_string();
                    let p = rest[idx + ": public ".len()..].trim().to_string();
                    (n, Some(p))
                } else {
                    (rest.trim().to_string(), None)
                };
                in_class = Some(ClassBuilder { name, parent, properties: vec![] });
                // UE4SS headers list members directly after `{` with no
                // `public:` label, so fields are parsed for the whole body.
                in_public = true;
                continue;
            }
            // else: a `class Foo* member;` field — fall through to field parsing.
        }

        // --- enum class declaration ---------------------------------------
        // "enum class EItemQuality : uint8_t"
        if let Some(rest) = trimmed.strip_prefix("enum class ") {
            if let Some(b) = in_class.take() {
                model.classes.push(b.build());
            }
            let name = rest.split(':').next().unwrap_or(rest).trim().to_string();
            in_enum = Some(EnumBuilder { name, members: vec![] });
            continue;
        }

        // --- public: section ----------------------------------------------
        if trimmed == "public:" {
            in_public = true;
            continue;
        }

        // --- closing brace ------------------------------------------------
        if trimmed.starts_with("};") {
            if let Some(b) = in_class.take() {
                model.classes.push(b.build());
            }
            if let Some(b) = in_enum.take() {
                model.enums.push(b.build());
            }
            in_public = false;
            continue;
        }

        // --- enum member --------------------------------------------------
        // "    Low = 0,"
        if let Some(eb) = in_enum.as_mut() {
            if !trimmed.is_empty() && trimmed != "{" {
                // member line: "Low = 0," — take the name before '='
                let member = trimmed.split('=').next().unwrap_or("").trim().to_string();
                if !member.is_empty() && !member.starts_with("//") {
                    eb.members.push(member);
                }
            }
            continue;
        }

        // --- field declaration in public section --------------------------
        // "    int32_t m_Value; // 0x50(0x04)"
        if in_public {
            if let Some(cb) = in_class.as_mut() {
                if let Some(prop) = try_parse_field(trimmed) {
                    cb.properties.push(prop);
                }
            }
        }
    }

    // flush any trailing builder
    if let Some(b) = in_class.take() {
        model.classes.push(b.build());
    }
    if let Some(b) = in_enum.take() {
        model.enums.push(b.build());
    }

    Ok(model)
}

struct ClassBuilder {
    name: String,
    parent: Option<String>,
    properties: Vec<Property>,
}

impl ClassBuilder {
    fn build(self) -> Class {
        Class {
            name: self.name,
            parent: self.parent,
            properties: self.properties,
        }
    }
}

struct EnumBuilder {
    name: String,
    members: Vec<String>,
}

impl EnumBuilder {
    fn build(self) -> Enum {
        Enum {
            name: self.name,
            members: self.members,
        }
    }
}

/// Try to parse a C++ field declaration line into a `Property`.
/// Returns `None` for lines that are comments, empty, or not field decls.
fn try_parse_field(line: &str) -> Option<Property> {
    if line.is_empty() || line.starts_with("//") || line == "{" {
        return None;
    }
    // Strip inline comment: "int32_t m_Value; // 0x50(0x04)" -> "int32_t m_Value;"
    let code = match line.find("//") {
        Some(i) => line[..i].trim(),
        None => line.trim(),
    };
    // Must end with ';'
    let code = code.strip_suffix(';')?.trim();
    // Skip method declarations (e.g. "void SetItemType(FGameplayTag x)").
    if code.contains('(') {
        return None;
    }
    // Split on last whitespace to get (type_tokens, name)
    let last_ws = code.rfind(|c: char| c.is_whitespace())?;
    let type_str = code[..last_ws].trim();
    let name = code[last_ws..].trim().to_string();
    if name.is_empty() || type_str.is_empty() {
        return None;
    }
    // Skip function pointers / complex stuff (contain '(' or '*' in name)
    if name.contains('(') || name.contains('*') {
        return None;
    }
    let prop_type = map_cpp_type(type_str);
    Some(Property {
        name,
        prop_type,
        offset: None,
    })
}

#[cfg(test)]
mod real_format_tests {
    use super::*;
    use crate::model::PropType;

    // Mirrors the real CXXHeaderDump format: no `public:` label, UE int
    // typedefs (`int32`), methods after fields, `class Foo* member;` members.
    const SNIPPET: &str = "\
class UItemDefinition : public UGothicObjectDefinition
{
    int32 m_Value;                                  // 0x0080 (size: 0x4)
    float m_Weight;                                 // 0x008C (size: 0x4)
    bool m_AutoTarget;                              // 0x0310 (size: 0x1)
    TSoftClassPtr<AItemVisual> m_ItemVisual;        // 0x0098 (size: 0x28)
    class UItemDefinition* m_ReplaceBy;             // 0x0308 (size: 0x8)
    void SetItemType(FGameplayTag GameplayTag);
}; // Size: 0x320
";

    #[test]
    fn parses_real_dump_format() {
        let model = parse_hpp_reader(SNIPPET.as_bytes()).unwrap();
        // Exactly one real class — the `class UFoo* member;` line must NOT
        // become its own class.
        assert_eq!(model.classes.len(), 1, "classes: {:?}",
            model.classes.iter().map(|c| &c.name).collect::<Vec<_>>());
        let c = &model.classes[0];
        assert_eq!(c.name, "UItemDefinition");
        assert_eq!(c.parent.as_deref(), Some("UGothicObjectDefinition"));
        // Scalars parsed without a `public:` label, UE typedefs mapped,
        // the method skipped.
        let by: std::collections::HashMap<&str, &PropType> =
            c.properties.iter().map(|p| (p.name.as_str(), &p.prop_type)).collect();
        assert_eq!(by.get("m_Value"), Some(&&PropType::Int));
        assert_eq!(by.get("m_Weight"), Some(&&PropType::Float));
        assert_eq!(by.get("m_AutoTarget"), Some(&&PropType::Bool));
        assert!(by.contains_key("m_ItemVisual")); // opaque, but present
        assert!(by.contains_key("m_ReplaceBy")); // opaque pointer member
        assert!(!c.properties.iter().any(|p| p.name.contains("SetItemType")));
    }
}

fn map_cpp_type(cpp: &str) -> PropType {
    match cpp {
        // C++ std typedefs and UE typedefs (the dump uses UE names: int32, uint8…)
        "int32_t" | "int64_t" | "int16_t" | "int8_t"
        | "uint32_t" | "uint64_t" | "uint16_t" | "uint8_t"
        | "int32" | "int64" | "int16" | "int8"
        | "uint32" | "uint64" | "uint16" | "uint8"
        | "int" | "long" | "short" | "char" | "byte" => PropType::Int,
        "float" | "double" => PropType::Float,
        "bool" => PropType::Bool,
        "FString" | "FName" | "FText" => PropType::String,
        other => {
            // Might be an enum type or an opaque struct; we can't tell without
            // the full model, so store as Opaque — validate.rs will resolve.
            PropType::Opaque(other.to_string())
        }
    }
}
