//! Preparing and checking an edit to a shipped conversation module.
//!
//! # Why this needs a checker at all
//!
//! Editing a vanilla module means recompiling it and splicing the result back onto the shipping
//! cache. Two mechanisms decide what survives that trip, and both fail closed:
//!
//! * The source emitter deliberately omits the compiler-generated `__InitDefaults`, and
//!   [`super::super::generated_defaults`] carries those records back byte-for-byte — but only
//!   when every surrounding identity is unchanged. Same classes, same order, same properties,
//!   same constructors, same methods with the same signatures. A class added, a field added, or
//!   a `default` statement written by hand, and the carry refuses.
//! * The remap that rebinds the recompiled module to the shipping cache's keyspace is strict for
//!   an edit: `--allow-new-symbols` is rejected outright when generated defaults are carried. So
//!   the edited body may only name types and string literals the base cache already has.
//!
//! Every one of those refusals arrives after a two-minute compile that drives the game's own
//! compiler. This module answers the same questions offline, in milliseconds, from the same base
//! cache the compiler would use.
//!
//! # What that leaves an author
//!
//! Method **bodies**. `compare_function` checks a method's name, signature and UFUNCTION
//! metadata and never its bytecode, so what a topic *does* — its lines, its effects, their order
//! and their branches — and when it is visible are both open, as is which existing topics a
//! `Subdialog` offers. What a topic *is* — caption, priority, rules, flags — lives in the
//! generated defaults and is carried back unchanged, so it cannot be edited this way.

use std::collections::BTreeSet;

use super::super::binds::NativeApi;
use super::super::emit_all::PreparedEmit;
use super::super::model::parse_modules;
use super::super::refs::RefResolver;
use super::graph::DialogError;

/// One shipped module, taken out for editing.
#[derive(Debug, Clone, PartialEq)]
pub struct Checkout {
    /// The Modules TMap key, which `compile-module --op edit --module` needs.
    pub module: String,
    /// Where the compiler expects the file, which `--rel-path` needs.
    pub relative_path: String,
    /// The exact source the compiler would emit for this module.
    pub source: String,
}

/// Take one module out of a cache as editable source.
pub fn checkout(
    cache: &[u8],
    module_name: &str,
    native_api: Option<NativeApi>,
) -> Result<Checkout, DialogError> {
    let mut taken = checkout_many(cache, std::slice::from_ref(&module_name), native_api)?;
    Ok(taken.remove(0))
}

/// Take several modules out of one cache, preparing the emitter once.
///
/// Preparing it costs a parse and a resolver build over the whole cache, so a caller that wants
/// more than one module must not go through [`checkout`] in a loop.
pub fn checkout_many(
    cache: &[u8],
    module_names: &[&str],
    native_api: Option<NativeApi>,
) -> Result<Vec<Checkout>, DialogError> {
    let modules = parse_modules(cache).map_err(|error| DialogError::Parse(error.to_string()))?;
    let mut refs =
        RefResolver::build(cache).map_err(|error| DialogError::Parse(error.to_string()))?;
    let mut indices = Vec::with_capacity(module_names.len());
    for module_name in module_names {
        let matches: Vec<usize> = modules
            .iter()
            .enumerate()
            .filter(|(_, module)| module.name == *module_name)
            .map(|(index, _)| index)
            .collect();
        let [index] = matches.as_slice() else {
            return Err(DialogError::Parse(format!(
                "module {module_name:?} matches {} modules in this cache; it has to match \
                 exactly one",
                matches.len()
            )));
        };
        indices.push(*index);
    }

    let prepared = PreparedEmit::new(&modules, &mut refs, native_api)
        .map_err(|error| DialogError::Parse(error.to_string()))?;
    let mut taken = Vec::with_capacity(indices.len());
    for (index, module_name) in indices.into_iter().zip(module_names) {
        let relative_path = prepared
            .module_relative_path(index)
            .ok_or_else(|| {
                DialogError::Parse(format!("module {module_name:?} has no output path"))
            })?
            .to_owned();
        let source = prepared
            .emit_module(index)
            .map_err(|error| DialogError::Parse(error.to_string()))?;
        taken.push(Checkout {
            module: (*module_name).to_owned(),
            relative_path,
            source,
        });
    }
    Ok(taken)
}

/// The names an edited module may refer to: everything the base cache already carries.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct KnownNames {
    pub types: BTreeSet<String>,
    pub strings: BTreeSet<String>,
}

/// Collect the type names and string literals the base cache can bind an edit to.
pub fn known_names(cache: &[u8]) -> Result<KnownNames, DialogError> {
    let refs = RefResolver::build(cache).map_err(|error| DialogError::Parse(error.to_string()))?;
    let modules = parse_modules(cache).map_err(|error| DialogError::Parse(error.to_string()))?;
    let mut types: BTreeSet<String> = modules
        .iter()
        .flat_map(|module| module.classes.iter())
        .map(|class| class.name.clone())
        .collect();
    types.extend(
        modules
            .iter()
            .flat_map(|module| module.enums.iter())
            .map(|entry| entry.name.clone()),
    );
    Ok(KnownNames {
        strings: refs.string_globals().map(str::to_owned).collect(),
        types,
    })
}

impl KnownNames {
    fn has_type(&self, name: &str) -> bool {
        self.types.contains(name)
    }
}

// ─── Reading the structure out of authored source ────────────────────────────

/// One class as the source declares it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassOutline {
    pub name: String,
    pub super_class: Option<String>,
    /// Member declarations, in order, as written.
    pub fields: Vec<String>,
    /// Member function declarations, in order, as written.
    pub members: Vec<String>,
}

/// The parts of a module's source that the recompile path compares.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SourceOutline {
    pub classes: Vec<ClassOutline>,
    /// Free function declarations at module scope, in order.
    pub functions: Vec<String>,
}

fn strip_comment(line: &str) -> &str {
    match line.find("//") {
        Some(position) => &line[..position],
        None => line,
    }
}

/// Read the declarations out of emitted or hand-edited module source.
///
/// This is deliberately a scanner rather than a parser: both sides of every comparison start as
/// the emitter's own output, whose shape is known, and a scanner cannot mistake a body edit for a
/// declaration change the way a partial parser could.
pub fn read_outline(source: &str) -> SourceOutline {
    let mut outline = SourceOutline::default();
    let mut depth = 0i32;
    let mut current: Option<ClassOutline> = None;

    for raw in source.lines() {
        let line = strip_comment(raw).trim();
        let opens = line.matches('{').count() as i32;
        let closes = line.matches('}').count() as i32;

        if depth == 0 && !line.is_empty() {
            if let Some(rest) = line.strip_prefix("class ") {
                let (name, super_class) = match rest.split_once(':') {
                    Some((name, parent)) => (
                        name.trim().trim_end_matches('{').trim().to_owned(),
                        Some(parent.trim().trim_end_matches('{').trim().to_owned()),
                    ),
                    None => (rest.trim().trim_end_matches('{').trim().to_owned(), None),
                };
                current = Some(ClassOutline {
                    name,
                    super_class,
                    fields: Vec::new(),
                    members: Vec::new(),
                });
            } else if line.contains('(') && !line.starts_with('#') && closes == 0 {
                // A free function's declaration line, e.g. `void Helper(int Value)`.
                outline.functions.push(normalize_declaration(line));
            }
        } else if depth == 1 && !line.is_empty() {
            if let Some(class) = current.as_mut() {
                if line == "UFUNCTION()"
                    || line.starts_with("UPROPERTY")
                    || line == "{"
                    || line == "}"
                {
                    // Attribute lines carry no identity of their own.
                } else if line.contains('(') {
                    class.members.push(normalize_declaration(line));
                } else if line.ends_with(';') {
                    class.fields.push(normalize_declaration(line));
                }
            }
        }

        let was_inside = depth > 0;
        depth += opens - closes;
        if depth <= 0 {
            depth = 0;
            // Only a class we were actually inside is finished here. The declaration line itself
            // leaves depth at zero, because the emitter puts the opening brace on the next line.
            if was_inside {
                if let Some(class) = current.take() {
                    outline.classes.push(class);
                }
            }
        }
    }
    if let Some(class) = current.take() {
        outline.classes.push(class);
    }
    outline
}

/// Collapse whitespace so re-indentation is not mistaken for a signature change.
fn normalize_declaration(line: &str) -> String {
    let line = line.trim_end_matches('{').trim();
    line.split_whitespace().collect::<Vec<_>>().join(" ")
}

// ─── The contract ────────────────────────────────────────────────────────────

/// One reason a compile of this edit would be refused, or would not survive the trip back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Violation {
    /// A `default` statement makes the compiler generate `__InitDefaults`, and the carry refuses
    /// to overwrite an authored one with the base's.
    AuthoredDefault {
        line: usize,
    },
    ClassAdded {
        name: String,
    },
    ClassRemoved {
        name: String,
    },
    ClassReordered {
        expected: String,
        found: String,
    },
    ClassReparented {
        name: String,
        expected: Option<String>,
        found: Option<String>,
    },
    FieldsChanged {
        class: String,
        expected: usize,
        found: usize,
    },
    MembersChanged {
        class: String,
        expected: usize,
        found: usize,
    },
    MemberSignatureChanged {
        class: String,
        expected: String,
        found: String,
    },
    FunctionsChanged {
        expected: usize,
        found: usize,
    },
    /// A type the base cache does not carry. A strict remap has nothing to bind it to.
    UnknownType {
        name: String,
    },
    /// A string literal the base cache does not carry, for the same reason.
    UnknownString {
        value: String,
    },
}

impl Violation {
    /// One sentence a person can act on.
    pub fn explain(&self) -> String {
        match self {
            Violation::AuthoredDefault { line } => format!(
                "line {line}: a `default` statement. Captions, priority, rules and flags are \
                 carried back from the shipped module unchanged, so they cannot be edited here"
            ),
            Violation::ClassAdded { name } => format!(
                "class {name} is new. An edited module has to keep exactly the classes it shipped \
                 with; a new topic needs its own module"
            ),
            Violation::ClassRemoved { name } => {
                format!("class {name} is missing. An edited module has to keep every class")
            }
            Violation::ClassReordered { expected, found } => format!(
                "class order changed: {expected} was declared here, {found} is now. The order is \
                 part of the module's identity"
            ),
            Violation::ClassReparented {
                name,
                expected,
                found,
            } => format!(
                "class {name} now derives from {} instead of {}",
                found.as_deref().unwrap_or("nothing"),
                expected.as_deref().unwrap_or("nothing")
            ),
            Violation::FieldsChanged {
                class,
                expected,
                found,
            } => format!(
                "class {class} declares {found} member variable(s) instead of {expected}. The \
                 property layout is compared byte-for-byte"
            ),
            Violation::MembersChanged {
                class,
                expected,
                found,
            } => format!(
                "class {class} declares {found} member function(s) instead of {expected}. Methods \
                 may change what they do, but not which of them there are"
            ),
            Violation::MemberSignatureChanged {
                class,
                expected,
                found,
            } => format!(
                "class {class}: `{found}` does not match the shipped declaration `{expected}`. A \
                 body may change; a signature may not"
            ),
            Violation::FunctionsChanged { expected, found } => {
                format!("the module declares {found} free function(s) instead of {expected}")
            }
            Violation::UnknownType { name } => format!(
                "{name} is not a type this cache carries, so an edit cannot bind to it. Only names \
                 the shipped game already has are reachable from an edited module"
            ),
            Violation::UnknownString { value } => format!(
                "the literal {value:?} is not in this cache's string table. An edited module can \
                 only use text ids the game already ships; a brand-new one needs its own module"
            ),
        }
    }
}

/// Which methods an edit actually rewrites.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangedBody {
    pub class: String,
    pub member: String,
}

/// The verdict on one edited module.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditReport {
    pub violations: Vec<Violation>,
    pub changed: Vec<ChangedBody>,
    /// True when the authored source is byte-identical to the shipped one.
    pub unchanged: bool,
}

impl EditReport {
    pub fn is_carryable(&self) -> bool {
        self.violations.is_empty()
    }
}

/// Check an authored module against the source it was taken from.
pub fn verify(pristine: &str, authored: &str, known: &KnownNames) -> EditReport {
    let mut violations = Vec::new();

    for (index, raw) in authored.lines().enumerate() {
        let line = strip_comment(raw).trim();
        if line == "default" || line.starts_with("default ") || line.starts_with("default\t") {
            violations.push(Violation::AuthoredDefault { line: index + 1 });
        }
    }

    let base = read_outline(pristine);
    let edit = read_outline(authored);

    let base_names: BTreeSet<&str> = base.classes.iter().map(|c| c.name.as_str()).collect();
    let edit_names: BTreeSet<&str> = edit.classes.iter().map(|c| c.name.as_str()).collect();
    for name in edit_names.difference(&base_names) {
        violations.push(Violation::ClassAdded {
            name: (*name).to_owned(),
        });
    }
    for name in base_names.difference(&edit_names) {
        violations.push(Violation::ClassRemoved {
            name: (*name).to_owned(),
        });
    }

    if base.functions.len() != edit.functions.len() {
        violations.push(Violation::FunctionsChanged {
            expected: base.functions.len(),
            found: edit.functions.len(),
        });
    }

    // Order is only worth reporting once the sets agree: one inserted class would otherwise
    // shift every class after it and bury the real cause under a list of consequences.
    let same_set = base_names == edit_names;

    let mut changed = Vec::new();
    for (position, base_class) in base.classes.iter().enumerate() {
        let Some(edit_class) = edit.classes.get(position) else {
            continue;
        };
        if edit_class.name != base_class.name {
            if same_set {
                violations.push(Violation::ClassReordered {
                    expected: base_class.name.clone(),
                    found: edit_class.name.clone(),
                });
            }
            continue;
        }
        if edit_class.super_class != base_class.super_class {
            violations.push(Violation::ClassReparented {
                name: base_class.name.clone(),
                expected: base_class.super_class.clone(),
                found: edit_class.super_class.clone(),
            });
        }
        if edit_class.fields != base_class.fields {
            violations.push(Violation::FieldsChanged {
                class: base_class.name.clone(),
                expected: base_class.fields.len(),
                found: edit_class.fields.len(),
            });
        }
        if edit_class.members.len() != base_class.members.len() {
            violations.push(Violation::MembersChanged {
                class: base_class.name.clone(),
                expected: base_class.members.len(),
                found: edit_class.members.len(),
            });
            continue;
        }
        for (expected, found) in base_class.members.iter().zip(&edit_class.members) {
            if expected != found {
                violations.push(Violation::MemberSignatureChanged {
                    class: base_class.name.clone(),
                    expected: expected.clone(),
                    found: found.clone(),
                });
            }
        }
    }

    for (name, member) in changed_bodies(pristine, authored) {
        changed.push(ChangedBody {
            class: name,
            member,
        });
    }

    violations.extend(unknown_names(pristine, authored, known));

    EditReport {
        unchanged: pristine == authored,
        violations,
        changed,
    }
}

/// Which `class::member` bodies differ between the two sources.
fn changed_bodies(pristine: &str, authored: &str) -> Vec<(String, String)> {
    let base = bodies(pristine);
    let edit = bodies(authored);
    let mut changed = Vec::new();
    for (key, body) in &edit {
        match base.iter().find(|(other, _)| other == key) {
            Some((_, original)) if original == body => {}
            _ => changed.push(key.clone()),
        }
    }
    changed
}

/// Every member body in a module's source, keyed by `(class, declaration)`.
fn bodies(source: &str) -> Vec<((String, String), String)> {
    let mut out = Vec::new();
    let mut depth = 0i32;
    let mut class = String::new();
    let mut member: Option<(String, String)> = None;
    let mut body = String::new();

    for raw in source.lines() {
        let line = strip_comment(raw).trim();
        let opens = line.matches('{').count() as i32;
        let closes = line.matches('}').count() as i32;

        if depth == 0 {
            if let Some(rest) = line.strip_prefix("class ") {
                class = rest
                    .split(':')
                    .next()
                    .unwrap_or(rest)
                    .trim()
                    .trim_end_matches('{')
                    .trim()
                    .to_owned();
            }
        } else if depth == 1 && line.contains('(') && !line.starts_with("UFUNCTION") {
            member = Some((class.clone(), normalize_declaration(line)));
            body.clear();
        } else if depth >= 2 {
            body.push_str(line);
            body.push('\n');
        }

        // A member is finished when its body closes, not when its declaration line ends: the
        // emitter puts the opening brace on the line after the declaration.
        let was_inside_body = depth >= 2;
        depth += opens - closes;
        if was_inside_body && depth <= 1 {
            if let Some(key) = member.take() {
                out.push((key, std::mem::take(&mut body)));
            }
        }
        if depth < 0 {
            depth = 0;
        }
    }
    out
}

/// Types and string literals the authored source introduces that the base cache cannot bind.
fn unknown_names(pristine: &str, authored: &str, known: &KnownNames) -> Vec<Violation> {
    let mut violations = Vec::new();
    let mut seen = BTreeSet::new();

    for name in static_class_names(authored) {
        if static_class_names(pristine).contains(&name) {
            continue;
        }
        if !known.has_type(&name) && seen.insert(name.clone()) {
            violations.push(Violation::UnknownType { name });
        }
    }

    let base_strings = string_literals(pristine);
    for value in string_literals(authored) {
        if base_strings.contains(&value) {
            continue;
        }
        if !known.strings.contains(&value) && seen.insert(value.clone()) {
            violations.push(Violation::UnknownString { value });
        }
    }
    violations
}

/// Class names used as `UX::StaticClass()`, which is how a body names a type.
fn static_class_names(source: &str) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    for line in source.lines() {
        let line = strip_comment(line);
        let mut rest = line;
        while let Some(position) = rest.find("::StaticClass()") {
            let head = &rest[..position];
            let start = head
                .rfind(|character: char| !(character.is_alphanumeric() || character == '_'))
                .map(|index| index + 1)
                .unwrap_or(0);
            let name = &head[start..];
            if !name.is_empty() {
                names.insert(name.to_owned());
            }
            rest = &rest[position + "::StaticClass()".len()..];
        }
    }
    names
}

/// Double-quoted literals, which reach the cache as string-table entries.
fn string_literals(source: &str) -> BTreeSet<String> {
    let mut values = BTreeSet::new();
    for line in source.lines() {
        let line = strip_comment(line);
        let mut chars = line.char_indices().peekable();
        while let Some((index, character)) = chars.next() {
            if character != '"' {
                continue;
            }
            let mut value = String::new();
            let mut closed = false;
            for (_, next) in chars.by_ref() {
                if next == '"' {
                    closed = true;
                    break;
                }
                value.push(next);
            }
            if closed && !value.is_empty() {
                values.insert(value);
            }
            let _ = index;
        }
    }
    values
}

#[cfg(test)]
mod tests {
    use super::*;

    const PRISTINE: &str = r#"
class UChoiceOne : UTopic_Hero__NPC
{
    UChoiceOne()
    {
        super();
        return;
    }
    UFUNCTION()
    void Act_Implementation()
    {
        this.EndConversation();
        return;
    }
}
"#;

    fn known() -> KnownNames {
        KnownNames {
            types: ["UChoiceOne".to_owned(), "UTopic_Hero__NPC".to_owned()]
                .into_iter()
                .collect(),
            strings: ["EXISTING_KEY".to_owned()].into_iter().collect(),
        }
    }

    #[test]
    fn an_untouched_checkout_is_carryable_and_reports_nothing_changed() {
        let report = verify(PRISTINE, PRISTINE, &known());
        assert!(report.is_carryable(), "{:?}", report.violations);
        assert!(report.unchanged);
        assert!(report.changed.is_empty());
    }

    #[test]
    fn a_body_edit_is_carryable_and_names_the_method() {
        let edited = PRISTINE.replace("this.EndConversation();", "this.ReturnToLastSelection();");
        let report = verify(PRISTINE, &edited, &known());
        assert!(report.is_carryable(), "{:?}", report.violations);
        assert!(!report.unchanged);
        assert_eq!(
            report.changed,
            vec![ChangedBody {
                class: "UChoiceOne".to_owned(),
                member: "void Act_Implementation()".to_owned(),
            }]
        );
    }

    #[test]
    fn a_new_class_is_refused() {
        let edited = format!("{PRISTINE}\nclass UChoiceTwo : UTopic_Hero__NPC\n{{\n}}\n");
        let report = verify(PRISTINE, &edited, &known());
        assert!(report.violations.contains(&Violation::ClassAdded {
            name: "UChoiceTwo".to_owned()
        }));
    }

    #[test]
    fn an_authored_default_is_refused() {
        let edited = PRISTINE.replace(
            "    UFUNCTION()",
            "    default PriorityRank = 3;\n    UFUNCTION()",
        );
        let report = verify(PRISTINE, &edited, &known());
        assert!(report
            .violations
            .iter()
            .any(|violation| matches!(violation, Violation::AuthoredDefault { .. })));
    }

    #[test]
    fn a_new_method_is_refused() {
        let edited = PRISTINE.replace(
            "    UFUNCTION()\n    void Act_Implementation()",
            "    void Helper()\n    {\n        return;\n    }\n    UFUNCTION()\n    void Act_Implementation()",
        );
        let report = verify(PRISTINE, &edited, &known());
        assert!(report
            .violations
            .iter()
            .any(|violation| matches!(violation, Violation::MembersChanged { .. })));
    }

    #[test]
    fn a_signature_change_is_refused_where_a_body_change_is_not() {
        let edited = PRISTINE.replace(
            "void Act_Implementation()",
            "void Act_Implementation(int X)",
        );
        let report = verify(PRISTINE, &edited, &known());
        assert!(report
            .violations
            .iter()
            .any(|violation| matches!(violation, Violation::MemberSignatureChanged { .. })));
    }

    #[test]
    fn a_type_the_cache_does_not_carry_is_refused() {
        let edited = PRISTINE.replace(
            "this.EndConversation();",
            "Subdialog(this, UChoiceInvented::StaticClass());",
        );
        let report = verify(PRISTINE, &edited, &known());
        assert!(report.violations.contains(&Violation::UnknownType {
            name: "UChoiceInvented".to_owned()
        }));
    }

    #[test]
    fn a_type_the_cache_carries_is_accepted() {
        let edited = PRISTINE.replace(
            "this.EndConversation();",
            "Subdialog(this, UChoiceOne::StaticClass());",
        );
        let report = verify(PRISTINE, &edited, &known());
        assert!(report.is_carryable(), "{:?}", report.violations);
    }

    #[test]
    fn a_brand_new_text_id_is_refused_and_a_shipped_one_is_not() {
        let invented = PRISTINE.replace("this.EndConversation();", "LocText(\"BRAND_NEW_KEY\");");
        let report = verify(PRISTINE, &invented, &known());
        assert!(report.violations.contains(&Violation::UnknownString {
            value: "BRAND_NEW_KEY".to_owned()
        }));

        let shipped = PRISTINE.replace("this.EndConversation();", "LocText(\"EXISTING_KEY\");");
        let report = verify(PRISTINE, &shipped, &known());
        assert!(report.is_carryable(), "{:?}", report.violations);
    }

    #[test]
    fn the_outline_reads_classes_fields_and_members() {
        let source =
            "class UA : UB\n{\n    int Value;\n    UFUNCTION()\n    void Go()\n    {\n    }\n}\n";
        let outline = read_outline(source);
        assert_eq!(outline.classes.len(), 1);
        assert_eq!(outline.classes[0].name, "UA");
        assert_eq!(outline.classes[0].super_class.as_deref(), Some("UB"));
        assert_eq!(outline.classes[0].fields, vec!["int Value;".to_owned()]);
        assert_eq!(outline.classes[0].members, vec!["void Go()".to_owned()]);
    }
}
