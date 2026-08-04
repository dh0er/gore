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
            // Tolerate an opening brace on the same line ("class X : public Y {")
            // by stripping it before the member/method checks.
            let decl = decl.strip_suffix('{').map(str::trim).unwrap_or(decl);
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
                    // Multiple inheritance (`UBase, public IFoo`): keep only the
                    // first (primary) base; that is the UPROPERTY ancestor chain.
                    let p = rest[idx + ": public ".len()..]
                        .split(',')
                        .next()
                        .unwrap_or("")
                        .trim()
                        .to_string();
                    (n, Some(p))
                } else {
                    (rest.trim().to_string(), None)
                };
                in_class = Some(ClassBuilder {
                    name,
                    parent,
                    properties: vec![],
                });
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
            in_enum = Some(EnumBuilder::new(name));
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
                // Member lines: "Low = 0,", "High,", or "Mid UMETA(...)". Take
                // the leading identifier — stop at '=', ',', whitespace, or '('.
                let member: String = trimmed
                    .chars()
                    .take_while(|&c| c.is_ascii_alphanumeric() || c == '_')
                    .collect();
                if !member.is_empty() && !trimmed.starts_with("//") {
                    // Capture an explicit `= N` discriminant so non-contiguous
                    // enums keep their real backing value (the GUI writes the
                    // value, not the member index). Members without one follow
                    // C rules: previous value + 1, starting at 0.
                    //
                    // Look for the `=` only in the declaration part, before any
                    // `UMETA(...)` metadata (which also contains `=`). If an
                    // explicit value is present but is an expression we can't
                    // evaluate exactly (e.g. `1 << 1`), mark the enum unreliable
                    // so we omit ALL its backing values rather than emit wrong
                    // ones — the GUI then falls back to indices.
                    let decl = trimmed.split("UMETA").next().unwrap_or(trimmed);
                    match decl.split_once('=') {
                        Some((_, rhs)) => match parse_enum_value(rhs) {
                            Some(v) => eb.push(member, Some(v)),
                            None => {
                                eb.push(member, None);
                                eb.reliable = false;
                            }
                        },
                        None => eb.push(member, None),
                    }
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

    // Resolve enum-typed fields within this file. The merge step in
    // `gore-cli dump` runs the same pass again over the combined model so that
    // enums declared in a different `.hpp` than their using class also resolve.
    model.resolve_enum_types();

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

/// Parse an enum discriminant from the text right of `=` (e.g. ` 5,` or
/// ` 0x10`). Accepts ONLY a single integer literal (decimal or `0x` hex, with
/// optional leading `-`); anything else — an expression like `1 << 1`, a named
/// constant, or trailing tokens — yields None so the caller can omit the
/// enum's backing values instead of recording a wrong one.
fn parse_enum_value(rhs: &str) -> Option<i64> {
    // Drop a trailing `// ...` inline comment and anything past the `,`, then
    // the remaining token must be a single literal (no internal whitespace).
    let s = rhs
        .split("//")
        .next()
        .unwrap_or(rhs)
        .split(',')
        .next()
        .unwrap_or(rhs)
        .trim();
    if s.is_empty() || s.contains(char::is_whitespace) {
        return None;
    }
    let (neg, digits) = match s.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, s),
    };
    let magnitude = if let Some(hex) = digits
        .strip_prefix("0x")
        .or_else(|| digits.strip_prefix("0X"))
    {
        i64::from_str_radix(hex, 16).ok()?
    } else {
        digits.parse::<i64>().ok()?
    };
    Some(if neg { -magnitude } else { magnitude })
}

struct EnumBuilder {
    name: String,
    members: Vec<String>,
    values: Vec<i64>,
    next: i64,
    /// Cleared when a member has an explicit value we couldn't parse exactly;
    /// the built enum then carries no backing values (consumers fall back).
    reliable: bool,
}

impl EnumBuilder {
    fn new(name: String) -> Self {
        Self {
            name,
            members: vec![],
            values: vec![],
            next: 0,
            reliable: true,
        }
    }

    fn push(&mut self, member: String, explicit: Option<i64>) {
        let value = explicit.unwrap_or(self.next);
        self.members.push(member);
        self.values.push(value);
        self.next = value + 1;
    }

    fn build(self) -> Enum {
        Enum {
            name: self.name,
            members: self.members,
            // Drop backing values entirely if any explicit value was an
            // un-parseable expression — better no values than wrong ones.
            values: if self.reliable { self.values } else { vec![] },
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
    // Strip a C++ bitfield width, e.g. "bool bAutoTarget : 1" -> "bool bAutoTarget"
    // (UE bool UPROPERTYs are often bitfields). Only treat a trailing `: <digits>`
    // as a width; leave `::` scope-resolution and other colons intact.
    let code = match code.rfind(':') {
        Some(i) if !code[..i].ends_with(':') => {
            let after = code[i + 1..].trim();
            if !after.is_empty() && after.bytes().all(|b| b.is_ascii_digit()) {
                code[..i].trim()
            } else {
                code
            }
        }
        _ => code,
    };
    // Split on last whitespace to get (type_tokens, name)
    let last_ws = code.rfind(|c: char| c.is_whitespace())?;
    let type_str = code[..last_ws].trim();
    let name = code[last_ws..].trim().to_string();
    if name.is_empty() || type_str.is_empty() {
        return None;
    }
    // Skip function pointers / complex declarators: methods ('('), pointer
    // members ('*'), and fixed-size C array / padding fields ('[', e.g.
    // `uint8 Pad_00[0x8];`) which are not editable scalar properties.
    if name.contains('(') || name.contains('*') || name.contains('[') {
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
    fn parses_class_with_same_line_opening_brace() {
        let snippet = "\
class UThing : public UBase {
    int32 m_X;   // 0x0010 (size: 0x4)
};
";
        let model = parse_hpp_reader(snippet.as_bytes()).unwrap();
        assert_eq!(model.classes.len(), 1);
        assert_eq!(model.classes[0].name, "UThing");
        assert_eq!(model.classes[0].parent.as_deref(), Some("UBase"));
        assert!(model.classes[0].properties.iter().any(|p| p.name == "m_X"));
    }

    #[test]
    fn multiple_inheritance_keeps_primary_base() {
        let snippet = "\
class UChild : public UBase, public IFoo
{
    int32 m_X;   // 0x0010 (size: 0x4)
};
";
        let model = parse_hpp_reader(snippet.as_bytes()).unwrap();
        assert_eq!(model.classes[0].parent.as_deref(), Some("UBase"));
    }

    #[test]
    fn enum_members_strip_commas_and_metadata() {
        let snippet = "\
enum class EQuality : uint8
{
    Low,
    High UMETA(DisplayName = \"Very High\"),
    Mid = 5,
};
";
        let model = parse_hpp_reader(snippet.as_bytes()).unwrap();
        let e = model.enums.iter().find(|e| e.name == "EQuality").unwrap();
        assert_eq!(e.members, ["Low", "High", "Mid"], "got: {:?}", e.members);
        // Backing values: auto 0, auto 1, explicit 5 (not the index 2).
        assert_eq!(e.values, [0, 1, 5], "got: {:?}", e.values);
    }

    #[test]
    fn enum_with_unparseable_expression_omits_backing_values() {
        // `1 << 1` can't be evaluated exactly here; rather than record a wrong
        // value (1), the whole enum drops its backing values so the GUI falls
        // back to indices instead of writing a wrong discriminant.
        let snippet = "\
enum class EFlags : uint8
{
    FlagA = 1,
    FlagB = 1 << 1,
};
";
        let model = parse_hpp_reader(snippet.as_bytes()).unwrap();
        let e = model.enums.iter().find(|e| e.name == "EFlags").unwrap();
        assert_eq!(e.members, ["FlagA", "FlagB"]);
        assert!(e.values.is_empty(), "got: {:?}", e.values);
    }

    #[test]
    fn enum_member_with_inline_comment_keeps_value() {
        // `= 42 // note` (no comma before the comment) must still parse, not
        // poison the whole enum.
        let snippet = "\
enum class EThing : uint8
{
    A,
    B = 42 // hand-tuned
};
";
        let model = parse_hpp_reader(snippet.as_bytes()).unwrap();
        let e = model.enums.iter().find(|e| e.name == "EThing").unwrap();
        assert_eq!(e.values, [0, 42], "got: {:?}", e.values);
    }

    #[test]
    fn parses_bitfield_bool_property() {
        let snippet = "\
class UThing : public UObject
{
    bool bAutoTarget : 1;   // 0x0010 (size: 0x1)
    int32 m_Value;          // 0x0014 (size: 0x4)
};
";
        let model = parse_hpp_reader(snippet.as_bytes()).unwrap();
        let c = &model.classes[0];
        let by: std::collections::HashMap<&str, &PropType> = c
            .properties
            .iter()
            .map(|p| (p.name.as_str(), &p.prop_type))
            .collect();
        assert_eq!(
            by.get("bAutoTarget"),
            Some(&&PropType::Bool),
            "bitfield name/type: {:?}",
            c.properties.iter().map(|p| &p.name).collect::<Vec<_>>()
        );
        assert!(
            !c.properties.iter().any(|p| p.name == "1"),
            "no bogus '1' field"
        );
        assert_eq!(by.get("m_Value"), Some(&&PropType::Int));
    }

    #[test]
    fn skips_fixed_size_array_and_padding_fields() {
        let snippet = "\
class UThing : public UObject
{
    int32 m_Value;        // 0x0010 (size: 0x4)
    uint8 Pad_00[0x8];    // 0x0014 (size: 0x8)
    float m_Speed[4];     // 0x001C (size: 0x10)
};
";
        let model = parse_hpp_reader(snippet.as_bytes()).unwrap();
        let c = &model.classes[0];
        let names: Vec<&str> = c.properties.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(
            names,
            ["m_Value"],
            "array/padding declarators must be skipped"
        );
    }

    #[test]
    fn parses_real_dump_format() {
        let model = parse_hpp_reader(SNIPPET.as_bytes()).unwrap();
        // Exactly one real class — the `class UFoo* member;` line must NOT
        // become its own class.
        assert_eq!(
            model.classes.len(),
            1,
            "classes: {:?}",
            model.classes.iter().map(|c| &c.name).collect::<Vec<_>>()
        );
        let c = &model.classes[0];
        assert_eq!(c.name, "UItemDefinition");
        assert_eq!(c.parent.as_deref(), Some("UGothicObjectDefinition"));
        // Scalars parsed without a `public:` label, UE typedefs mapped,
        // the method skipped.
        let by: std::collections::HashMap<&str, &PropType> = c
            .properties
            .iter()
            .map(|p| (p.name.as_str(), &p.prop_type))
            .collect();
        assert_eq!(by.get("m_Value"), Some(&&PropType::Int));
        assert_eq!(by.get("m_Weight"), Some(&&PropType::Float));
        assert_eq!(by.get("m_AutoTarget"), Some(&&PropType::Bool));
        assert!(by.contains_key("m_ItemVisual")); // opaque, but present
        assert!(by.contains_key("m_ReplaceBy")); // opaque pointer member
        assert!(!c.properties.iter().any(|p| p.name.contains("SetItemType")));
    }
}

#[cfg(test)]
mod enum_resolve_tests {
    use super::*;
    use crate::model::PropType;

    const SNIPPET_WITH_ENUM: &str = "\
// Enum Angelscript.EItemQuality
enum class EItemQuality : uint8_t
{
    Low = 0,
    Medium = 1,
    High = 2,
}; // Size: 0x1

// Class Angelscript.UItMi_Orenugget
class UItMi_Orenugget : public UItemDefinition
{
    EItemQuality m_Quality;                          // 0x0100 (size: 0x1)
    int32 m_Value;                                   // 0x0104 (size: 0x4)
};
";

    #[test]
    fn enum_field_resolved_to_prop_type_enum() {
        let model = parse_hpp_reader(SNIPPET_WITH_ENUM.as_bytes()).unwrap();

        // Enum must be parsed
        assert_eq!(model.enums.len(), 1);
        assert_eq!(model.enums[0].name, "EItemQuality");

        // Class must be parsed
        assert_eq!(model.classes.len(), 1);
        let c = &model.classes[0];

        let quality_prop = c
            .properties
            .iter()
            .find(|p| p.name == "m_Quality")
            .expect("m_Quality property must exist");

        // After post-pass, must be Enum(...) not Opaque(...)
        assert_eq!(
            quality_prop.prop_type,
            PropType::Enum("EItemQuality".to_string()),
            "expected PropType::Enum(\"EItemQuality\"), got {:?}",
            quality_prop.prop_type
        );

        // Scalar field must remain Int
        let value_prop = c
            .properties
            .iter()
            .find(|p| p.name == "m_Value")
            .expect("m_Value property must exist");
        assert_eq!(value_prop.prop_type, PropType::Int);
    }

    #[test]
    fn non_enum_opaque_stays_opaque() {
        let snippet = "\
class UFoo : public UBar
{
    TSoftClassPtr<AItemVisual> m_Visual;
};
";
        let model = parse_hpp_reader(snippet.as_bytes()).unwrap();
        let c = &model.classes[0];
        let prop = c.properties.iter().find(|p| p.name == "m_Visual").unwrap();
        assert!(
            matches!(&prop.prop_type, PropType::Opaque(_)),
            "non-enum type should remain Opaque, got {:?}",
            prop.prop_type
        );
    }
}

fn map_cpp_type(cpp: &str) -> PropType {
    match cpp {
        // C++ std typedefs and UE typedefs (the dump uses UE names: int32, uint8…)
        "int32_t" | "int64_t" | "int16_t" | "int8_t" | "uint32_t" | "uint64_t" | "uint16_t"
        | "uint8_t" | "int32" | "int64" | "int16" | "int8" | "uint32" | "uint64" | "uint16"
        | "uint8" | "int" | "long" | "short" | "char" | "byte" => PropType::Int,
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
