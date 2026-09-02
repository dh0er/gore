//! Preparing and checking an edit to a shipped conversation module.
//!
//! # Why this needs a checker at all
//!
//! Editing a vanilla module means recompiling it and splicing the result back onto the shipping
//! cache. The current emitter reconstructs `__InitDefaults` as class-scope `default` statements,
//! so an ordinary checkout authors every class default and the compiler regenerates those
//! methods. The old byte-exact carry remains only as a fallback when no existing class authors
//! defaults. Newly appended classes may still author their own defaults in that hybrid mode.
//!
//! That distinction is security-relevant. Once one class authors a default, every base class with
//! an `__InitDefaults` record must still be covered and every default target the checkout carried
//! must remain present; otherwise recompilation could silently replace an omitted value with an
//! engine default. A fully authored module may opt into the remapper's minimal new-symbol rows;
//! hybrid carry may retain the rows required only by newly appended classes.
//!
//! Every one of those refusals arrives after a two-minute compile that drives the game's own
//! compiler. This module answers the same questions offline, in milliseconds, from the same base
//! cache the compiler would use.
//!
//! # What that leaves an author
//!
//! Method bodies remain editable, including an existing `Subdialog` call. Reconstructed defaults
//! make caption, priority, rules and flags editable as source too. New classes, free functions,
//! strings and FName literals are reported as requiring `--allow-new-symbols`; existing class
//! layout and callable identities stay fixed because no runtime ABI migration for live vanilla
//! classes is proven.

use std::collections::{BTreeMap, BTreeSet};

use super::super::binds::NativeApi;
use super::super::emit_all::PreparedEmit;
use super::super::model::parse_modules;
use super::super::refs::RefResolver;
use super::graph::DialogError;

/// Native parents admitted for a completely authored conversation module.
///
/// These are intentionally not treated as general cache types. The verifier admits them only as
/// the direct parent of a newly declared class, and only when the target cache itself contains a
/// shipped class with that exact direct parent. Every other native parent remains fail-closed.
const NATIVE_CONVERSATION_BASES: [&str; 2] = ["UConversationCharacterSettings", "UG1RDialogTopic"];

/// One shipped module, taken out for editing.
#[derive(Debug, Clone, PartialEq)]
pub struct Checkout {
    /// The Modules TMap key, which `compile-module --op edit --module` needs.
    pub module: String,
    /// Where the compiler expects the file, which `--rel-path` needs.
    pub relative_path: String,
    /// The exact source the compiler would emit for this module.
    pub source: String,
    /// Base classes whose compiler-generated initializer is fully authored in `source`.
    pub default_classes: BTreeSet<String>,
    /// Other emitter-omitted `__*` methods which defaults cannot supersede.
    pub unsupported_generated_methods: Vec<String>,
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
        .map_err(|error| DialogError::Parse(error.to_string()))?
        .with_class_defaults(true);
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
        let module = &modules[index];
        let expected_defaults = module
            .classes
            .iter()
            .filter(|class| {
                class
                    .methods
                    .iter()
                    .any(|method| method.name == "__InitDefaults")
            })
            .map(|class| class.name.clone())
            .collect::<BTreeSet<_>>();
        let default_classes =
            super::super::default_source::classes_with_default_statements(&source)
                .map_err(DialogError::Parse)?
                .into_iter()
                .collect::<BTreeSet<_>>();
        let missing = expected_defaults
            .difference(&default_classes)
            .cloned()
            .collect::<Vec<_>>();
        if !missing.is_empty() {
            return Err(DialogError::Parse(format!(
                "module {module_name:?} did not emit complete class defaults for {}: {}",
                missing.len(),
                missing.join(", ")
            )));
        }
        let unsupported_generated_methods = module
            .classes
            .iter()
            .flat_map(|class| {
                class
                    .methods
                    .iter()
                    .filter(|method| {
                        method.name.starts_with("__") && method.name != "__InitDefaults"
                    })
                    .map(|method| format!("{}::{}", class.name, method.name))
            })
            .collect();
        taken.push(Checkout {
            module: (*module_name).to_owned(),
            relative_path,
            source,
            default_classes,
            unsupported_generated_methods,
        });
    }
    Ok(taken)
}

/// The names an edited module may refer to: everything the base cache already carries.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct KnownNames {
    pub types: BTreeSet<String>,
    /// Audited native parents used directly by shipped classes in this exact cache generation.
    pub native_conversation_bases: BTreeSet<String>,
    pub strings: BTreeSet<String>,
    pub static_names: BTreeSet<String>,
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
    let native_conversation_bases = modules
        .iter()
        .flat_map(|module| module.classes.iter())
        .filter_map(|class| class.super_class.as_deref())
        .filter(|name| NATIVE_CONVERSATION_BASES.contains(name))
        .map(str::to_owned)
        .collect();
    Ok(KnownNames {
        strings: refs.string_globals().map(str::to_owned).collect(),
        static_names: refs.static_names().map(str::to_owned).collect(),
        types,
        native_conversation_bases,
    })
}

impl KnownNames {
    fn has_type(&self, name: &str) -> bool {
        self.types.contains(name)
    }

    fn existing_type_case_insensitive(&self, name: &str) -> Option<&str> {
        self.types
            .iter()
            .chain(self.native_conversation_bases.iter())
            .find(|known| known.eq_ignore_ascii_case(name))
            .map(String::as_str)
    }

    fn has_native_conversation_base(&self, name: &str) -> bool {
        NATIVE_CONVERSATION_BASES.contains(&name) && self.native_conversation_bases.contains(name)
    }
}

// ─── Reading the structure out of authored source ────────────────────────────

/// One class as the source declares it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassOutline {
    /// The complete lexical namespace, without a leading `::`.
    pub namespace: String,
    pub name: String,
    /// `class` and `struct` are distinct declarations and may not be exchanged in-place.
    pub kind: String,
    pub super_class: Option<String>,
    /// Class-scope defaults, kept separate from fields and functions.
    pub defaults: Vec<DefaultOutline>,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefaultOutline {
    pub target: String,
    pub statement: String,
    pub line: usize,
}

#[derive(Debug, Clone)]
struct Token {
    text: String,
    line: usize,
    word: bool,
}

fn tokenize(source: &str) -> Result<Vec<Token>, String> {
    let bytes = source.as_bytes();
    let mut out = Vec::new();
    let mut index = 0usize;
    let mut line = 1usize;
    while index < bytes.len() {
        match bytes[index] {
            b'\r' => index += 1,
            b'\n' => {
                line += 1;
                index += 1;
            }
            byte if byte.is_ascii_whitespace() => index += 1,
            b'/' if bytes.get(index + 1) == Some(&b'/') => {
                index += 2;
                while index < bytes.len() && !matches!(bytes[index], b'\r' | b'\n') {
                    index += 1;
                }
            }
            b'/' if bytes.get(index + 1) == Some(&b'*') => {
                index += 2;
                let mut closed = false;
                while index < bytes.len() {
                    if bytes[index] == b'\n' {
                        line += 1;
                    }
                    if bytes[index] == b'*' && bytes.get(index + 1) == Some(&b'/') {
                        index += 2;
                        closed = true;
                        break;
                    }
                    index += 1;
                }
                if !closed {
                    return Err("source has an unterminated block comment".into());
                }
            }
            quote @ (b'\'' | b'\"') => {
                let start = index;
                let token_line = line;
                index += 1;
                let mut closed = false;
                while index < bytes.len() {
                    if bytes[index] == b'\\' {
                        index = (index + 2).min(bytes.len());
                    } else if bytes[index] == quote {
                        index += 1;
                        closed = true;
                        break;
                    } else {
                        if bytes[index] == b'\n' {
                            line += 1;
                        }
                        index += 1;
                    }
                }
                if !closed {
                    return Err("source has an unterminated quoted literal".into());
                }
                out.push(Token {
                    text: source[start..index].to_owned(),
                    line: token_line,
                    word: false,
                });
            }
            byte if byte.is_ascii_alphanumeric() || byte == b'_' => {
                let start = index;
                let token_line = line;
                index += 1;
                while index < bytes.len()
                    && (bytes[index].is_ascii_alphanumeric() || bytes[index] == b'_')
                {
                    index += 1;
                }
                out.push(Token {
                    text: source[start..index].to_owned(),
                    line: token_line,
                    word: true,
                });
            }
            byte if !byte.is_ascii() => {
                return Err(format!(
                    "line {line}: source contains a non-ASCII character outside comments or quoted literals"
                ));
            }
            _ => {
                let token_line = line;
                let start = index;
                index += 1;
                out.push(Token {
                    text: source[start..index].to_owned(),
                    line: token_line,
                    word: false,
                });
            }
        }
    }
    Ok(out)
}

fn brace_pairs(tokens: &[Token]) -> Result<Vec<Option<usize>>, String> {
    let mut pairs = vec![None; tokens.len()];
    let mut stack = Vec::new();
    for (index, token) in tokens.iter().enumerate() {
        match token.text.as_str() {
            "{" => stack.push(index),
            "}" => {
                let Some(open) = stack.pop() else {
                    return Err(format!("line {}: unmatched closing brace", token.line));
                };
                pairs[open] = Some(index);
                pairs[index] = Some(open);
            }
            _ => {}
        }
    }
    if let Some(open) = stack.pop() {
        return Err(format!("line {}: unclosed block", tokens[open].line));
    }
    Ok(pairs)
}

fn normalized(tokens: &[Token]) -> String {
    tokens
        .iter()
        .map(|token| token.text.as_str())
        .collect::<Vec<_>>()
        .join(" ")
}

fn compact(tokens: &[Token]) -> String {
    tokens.iter().map(|token| token.text.as_str()).collect()
}

fn qualified_name(namespace: &str, name: &str) -> String {
    if namespace.is_empty() {
        name.to_owned()
    } else {
        format!("{namespace}::{name}")
    }
}

// Body-change reporting is informational only; the safety checks above use the tokenized outline.
fn strip_comment(line: &str) -> &str {
    line.split_once("//").map_or(line, |(code, _)| code)
}

fn default_target(tokens: &[Token]) -> Result<String, String> {
    let mut at = 0usize;
    if tokens.get(at).map(|token| token.text.as_str()) == Some(":")
        && tokens.get(at + 1).map(|token| token.text.as_str()) == Some(":")
    {
        at += 2;
    }
    if tokens.get(at).map(|token| token.text.as_str()) == Some("this")
        && tokens.get(at + 1).map(|token| token.text.as_str()) == Some(".")
    {
        at += 2;
    }
    let first = tokens
        .get(at)
        .filter(|token| token.word)
        .ok_or_else(|| "class default has no semantic target".to_owned())?;
    let mut target = first.text.clone();
    at += 1;
    while tokens.get(at).map(|token| token.text.as_str()) == Some(":")
        && tokens.get(at + 1).map(|token| token.text.as_str()) == Some(":")
    {
        let segment = tokens
            .get(at + 2)
            .filter(|token| token.word)
            .ok_or_else(|| "class default has an incomplete qualified target".to_owned())?;
        target.push_str("::");
        target.push_str(&segment.text);
        at += 3;
    }
    Ok(target)
}

fn parse_class(
    tokens: &[Token],
    pairs: &[Option<usize>],
    namespace: &str,
    declaration: usize,
    open: usize,
    close: usize,
) -> Result<ClassOutline, String> {
    let name_token = tokens
        .get(declaration + 1)
        .filter(|token| token.word)
        .ok_or_else(|| format!("line {}: class has no name", tokens[declaration].line))?;
    let colon = tokens[declaration + 2..open]
        .iter()
        .position(|token| token.text == ":")
        .map(|offset| declaration + 2 + offset);
    let super_class = colon
        .map(|colon| compact(&tokens[colon + 1..open]))
        .filter(|name| !name.is_empty());
    let mut class = ClassOutline {
        namespace: namespace.to_owned(),
        name: name_token.text.clone(),
        kind: tokens[declaration].text.clone(),
        super_class,
        defaults: Vec::new(),
        fields: Vec::new(),
        members: Vec::new(),
    };
    let mut index = open + 1;
    let mut item_start = index;
    while index < close {
        if tokens[index].text == "default" {
            let start = index;
            let mut end = index + 1;
            while end < close && tokens[end].text != ";" {
                if tokens[end].text == "{" {
                    let nested = pairs[end].ok_or_else(|| {
                        format!("line {}: unclosed default expression", tokens[end].line)
                    })?;
                    end = nested + 1;
                } else {
                    end += 1;
                }
            }
            if end >= close {
                return Err(format!(
                    "line {}: class default has no terminating semicolon",
                    tokens[start].line
                ));
            }
            class.defaults.push(DefaultOutline {
                target: default_target(&tokens[start + 1..end])?,
                statement: normalized(&tokens[start..=end]),
                line: tokens[start].line,
            });
            index = end + 1;
            item_start = index;
            continue;
        }
        match tokens[index].text.as_str() {
            "{" => {
                let end = pairs[index]
                    .ok_or_else(|| format!("line {}: unclosed member body", tokens[index].line))?;
                let declaration = normalized(&tokens[item_start..index]);
                if !declaration.is_empty() {
                    class.members.push(declaration);
                }
                index = end + 1;
                item_start = index;
            }
            ";" => {
                let declaration = normalized(&tokens[item_start..=index]);
                if !declaration.is_empty() {
                    if tokens[item_start..index]
                        .iter()
                        .any(|token| token.text == "(")
                    {
                        class.members.push(declaration);
                    } else {
                        class.fields.push(declaration);
                    }
                }
                index += 1;
                item_start = index;
            }
            _ => index += 1,
        }
    }
    Ok(class)
}

#[derive(Debug, Clone)]
struct ClassSpan {
    namespace: String,
    declaration: usize,
    open: usize,
    close: usize,
}

#[derive(Debug, Default)]
struct ModuleItems {
    classes: Vec<ClassSpan>,
    functions: Vec<String>,
}

fn namespace_path(tokens: &[Token]) -> Result<String, String> {
    let mut segments = Vec::new();
    let mut at = 0usize;
    while at < tokens.len() {
        let Some(segment) = tokens.get(at).filter(|token| token.word) else {
            return Err("namespace name must contain only identifiers separated by `::`".into());
        };
        segments.push(segment.text.clone());
        at += 1;
        if at == tokens.len() {
            break;
        }
        if tokens.get(at).map(|token| token.text.as_str()) != Some(":")
            || tokens.get(at + 1).map(|token| token.text.as_str()) != Some(":")
        {
            return Err("namespace name must contain only identifiers separated by `::`".into());
        }
        at += 2;
    }
    if segments.is_empty() {
        Err("namespace declaration has no name".into())
    } else {
        Ok(segments.join("::"))
    }
}

fn unsupported_scope_declaration(tokens: &[Token]) -> String {
    let declaration = normalized(tokens);
    let line = tokens.first().map_or(1, |token| token.line);
    format!(
        "line {line}: unsupported module-scope declaration `{declaration}`; dialog edits only \
         inventory namespaces, classes/structs and free functions"
    )
}

fn is_free_function_header(tokens: &[Token]) -> bool {
    let mut paren_depth = 0usize;
    let mut saw_parameters = false;
    for token in tokens {
        match token.text.as_str() {
            "(" => {
                paren_depth += 1;
                saw_parameters = true;
            }
            ")" => {
                let Some(depth) = paren_depth.checked_sub(1) else {
                    return false;
                };
                paren_depth = depth;
            }
            "=" if paren_depth == 0 => return false,
            _ => {}
        }
    }
    saw_parameters && paren_depth == 0
}

fn scan_scope(
    tokens: &[Token],
    pairs: &[Option<usize>],
    mut at: usize,
    end: usize,
    namespace: &str,
    items: &mut ModuleItems,
) -> Result<(), String> {
    while at < end {
        if tokens[at].text == ";" {
            at += 1;
            continue;
        }

        if tokens[at].text == "namespace" {
            let mut open = at + 1;
            while open < end && !matches!(tokens[open].text.as_str(), "{" | ";") {
                open += 1;
            }
            if open >= end || tokens[open].text != "{" {
                let declaration_end = open.min(end.saturating_sub(1));
                return Err(unsupported_scope_declaration(&tokens[at..=declaration_end]));
            }
            let local = namespace_path(&tokens[at + 1..open])
                .map_err(|reason| format!("line {}: {reason}", tokens[at].line))?;
            let nested = qualified_name(namespace, &local);
            let close = pairs[open]
                .ok_or_else(|| format!("line {}: namespace body is not closed", tokens[at].line))?;
            if close > end {
                return Err(format!(
                    "line {}: namespace crosses its containing scope",
                    tokens[at].line
                ));
            }
            scan_scope(tokens, pairs, open + 1, close, &nested, items)?;
            at = close + 1;
            continue;
        }

        let start = at;
        while at < end && !matches!(tokens[at].text.as_str(), "{" | ";") {
            at += 1;
        }
        if at >= end {
            return Err(unsupported_scope_declaration(&tokens[start..end]));
        }
        if tokens[at].text == ";" {
            return Err(unsupported_scope_declaration(&tokens[start..=at]));
        }

        let open = at;
        let close = pairs[open].ok_or_else(|| {
            format!(
                "line {}: module-scope block is not closed",
                tokens[start].line
            )
        })?;
        if close > end {
            return Err(format!(
                "line {}: declaration crosses its containing scope",
                tokens[start].line
            ));
        }
        let header = &tokens[start..open];
        let first = header
            .first()
            .filter(|token| token.word)
            .map(|token| token.text.as_str());
        if matches!(first, Some("class" | "struct")) {
            items.classes.push(ClassSpan {
                namespace: namespace.to_owned(),
                declaration: start,
                open,
                close,
            });
        } else if is_free_function_header(header)
            && !header
                .iter()
                .any(|token| matches!(token.text.as_str(), "class" | "struct"))
            && !matches!(
                first,
                Some(
                    "delegate"
                        | "enum"
                        | "event"
                        | "funcdef"
                        | "if"
                        | "import"
                        | "interface"
                        | "mixin"
                        | "switch"
                )
            )
        {
            let declaration = normalized(header);
            items
                .functions
                .push(qualified_name(namespace, &declaration));
        } else {
            return Err(unsupported_scope_declaration(&tokens[start..=close]));
        }
        at = close + 1;
    }
    Ok(())
}

fn module_items(tokens: &[Token], pairs: &[Option<usize>]) -> Result<ModuleItems, String> {
    let mut items = ModuleItems::default();
    scan_scope(tokens, pairs, 0, tokens.len(), "", &mut items)?;
    Ok(items)
}

/// Read the declarations and class-scope defaults out of emitted or hand-edited module source.
/// Comments and literals are lexed before braces are interpreted; malformed source fails closed.
pub fn read_outline(source: &str) -> Result<SourceOutline, String> {
    super::super::default_source::reject_preprocessor_directives(source)?;
    let tokens = tokenize(source)?;
    let pairs = brace_pairs(&tokens)?;
    let items = module_items(&tokens, &pairs)?;

    let mut outline = SourceOutline::default();
    for span in &items.classes {
        outline.classes.push(parse_class(
            &tokens,
            &pairs,
            &span.namespace,
            span.declaration,
            span.open,
            span.close,
        )?);
    }
    outline.functions = items.functions;
    Ok(outline)
}

// ─── The contract ────────────────────────────────────────────────────────────

/// One reason a compile of this edit would be refused, or would not survive the trip back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Violation {
    SourceInvalid {
        side: &'static str,
        reason: String,
    },
    MissingClassDefaults {
        class: String,
    },
    DefaultTargetRemoved {
        class: String,
        target: String,
        expected: usize,
        found: usize,
    },
    UnsupportedGeneratedMethod {
        name: String,
    },
    ClassRemoved {
        name: String,
    },
    ClassNamespaceChanged {
        name: String,
        expected: String,
        found: String,
    },
    ClassKindChanged {
        name: String,
        expected: String,
        found: String,
    },
    DuplicateClassIdentity {
        side: &'static str,
        identity: String,
        found: usize,
    },
    AmbiguousClassName {
        side: &'static str,
        name: String,
        identities: Vec<String>,
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
    FunctionRemoved {
        declaration: String,
    },
    DuplicateFunctionIdentity {
        declaration: String,
        expected: usize,
        found: usize,
    },
    /// A type the base cache does not carry. A strict remap has nothing to bind it to.
    UnknownType {
        name: String,
    },
    /// A newly declared class reuses a type identity already owned by the base cache.
    ExistingTypeCollision {
        name: String,
        existing: String,
    },
    /// Two newly authored classes collapse to the same compiler identity under case folding.
    NewTypeCollision {
        first: String,
        second: String,
    },
}

impl Violation {
    /// One sentence a person can act on.
    pub fn explain(&self) -> String {
        match self {
            Violation::SourceInvalid { side, reason } => {
                format!("the {side} source could not be inventoried safely: {reason}")
            }
            Violation::MissingClassDefaults { class } => format!(
                "class {class} no longer authors any defaults. Its shipped `__InitDefaults` \
                 would be lost instead of regenerated"
            ),
            Violation::DefaultTargetRemoved {
                class,
                target,
                expected,
                found,
            } => format!(
                "class {class} now carries {found} `default {target}` statement(s) instead of \
                 {expected}. Values and arguments may change and defaults may be added, but a \
                 shipped target may not disappear silently"
            ),
            Violation::UnsupportedGeneratedMethod { name } => format!(
                "{name} is an emitter-omitted generated method which class-scope defaults do not \
                 supersede; this module cannot use the authored-default edit path"
            ),
            Violation::ClassRemoved { name } => {
                format!("class {name} is missing. An edited module has to keep every class")
            }
            Violation::ClassNamespaceChanged {
                name,
                expected,
                found,
            } => format!(
                "class {name} moved from namespace `{expected}` to `{found}`. Existing class identities must keep their complete namespace"
            ),
            Violation::ClassKindChanged {
                name,
                expected,
                found,
            } => format!(
                "{name} is declared as `{found}` instead of `{expected}`. An existing class and struct are not interchangeable"
            ),
            Violation::DuplicateClassIdentity {
                side,
                identity,
                found,
            } => format!(
                "the {side} source declares {identity} {found} times. A qualified class/struct identity must be unique"
            ),
            Violation::AmbiguousClassName {
                side,
                name,
                identities,
            } => format!(
                "the {side} source uses the bare class name {name} for multiple identities: {}. The dialog pipeline requires class names to be unambiguous within a module",
                identities.join(", ")
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
            Violation::FunctionRemoved { declaration } => {
                format!("the shipped free-function declaration `{declaration}` is missing")
            }
            Violation::DuplicateFunctionIdentity {
                declaration,
                expected,
                found,
            } => format!(
                "the authored source declares free function `{declaration}` {found} times; at most {expected} occurrence(s) are allowed by the pristine module"
            ),
            Violation::UnknownType { name } => format!(
                "{name} is neither a type from the base cache nor a class declared by this \
                 overlay, so the compiler cannot resolve it"
            ),
            Violation::ExistingTypeCollision { name, existing } => format!(
                "new class {name} collides with existing cache type {existing} under \
                 case-insensitive compiler identity matching"
            ),
            Violation::NewTypeCollision { first, second } => format!(
                "new classes {first} and {second} collide under case-insensitive compiler identity matching"
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangedDefault {
    pub class: String,
    pub target: String,
}

/// The verdict on one edited module.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditReport {
    pub violations: Vec<Violation>,
    pub changed: Vec<ChangedBody>,
    pub changed_defaults: Vec<ChangedDefault>,
    pub added_classes: Vec<String>,
    pub added_functions: Vec<String>,
    pub new_strings: Vec<String>,
    pub new_static_names: Vec<String>,
    /// True when the authored source is byte-identical to the shipped one.
    pub unchanged: bool,
}

impl EditReport {
    pub fn is_carryable(&self) -> bool {
        self.violations.is_empty()
    }

    pub fn requires_new_symbols(&self) -> bool {
        !self.added_classes.is_empty()
            || !self.added_functions.is_empty()
            || !self.new_strings.is_empty()
            || !self.new_static_names.is_empty()
    }
}

fn class_identity(class: &ClassOutline) -> String {
    qualified_name(&class.namespace, &class.name)
}

fn class_identity_counts(outline: &SourceOutline) -> std::collections::BTreeMap<String, usize> {
    let mut counts = std::collections::BTreeMap::new();
    for class in &outline.classes {
        *counts.entry(class_identity(class)).or_default() += 1;
    }
    counts
}

fn bare_class_identities(
    outline: &SourceOutline,
) -> std::collections::BTreeMap<String, BTreeSet<String>> {
    let mut identities = std::collections::BTreeMap::<String, BTreeSet<String>>::new();
    for class in &outline.classes {
        identities
            .entry(class.name.clone())
            .or_default()
            .insert(class_identity(class));
    }
    identities
}

fn function_counts(functions: &[String]) -> std::collections::BTreeMap<String, usize> {
    let mut counts = std::collections::BTreeMap::new();
    for function in functions {
        *counts.entry(function.clone()).or_default() += 1;
    }
    counts
}

fn added_parent_resolves(
    class: &ClassOutline,
    parent: &str,
    declared_identities: &BTreeSet<String>,
    module_class_names: &BTreeSet<&str>,
    known: &KnownNames,
) -> bool {
    let exact_native_parent = class.kind == "class"
        && !parent.starts_with("::")
        && !parent.contains("::")
        && known.has_native_conversation_base(parent);
    let parent = parent.strip_prefix("::").unwrap_or(parent);
    if parent.contains("::") {
        declared_identities.contains(parent)
    } else {
        declared_identities.contains(&qualified_name(&class.namespace, parent))
            || (!module_class_names.contains(parent)
                && (known.has_type(parent) || exact_native_parent))
    }
}

/// Check an authored module against the source it was taken from.
pub fn verify(checkout: &Checkout, authored: &str, known: &KnownNames) -> EditReport {
    let pristine = checkout.source.as_str();
    let mut violations = Vec::new();
    let base = match read_outline(pristine) {
        Ok(outline) => outline,
        Err(reason) => {
            return EditReport {
                violations: vec![Violation::SourceInvalid {
                    side: "pristine",
                    reason,
                }],
                changed: Vec::new(),
                changed_defaults: Vec::new(),
                added_classes: Vec::new(),
                added_functions: Vec::new(),
                new_strings: Vec::new(),
                new_static_names: Vec::new(),
                unchanged: pristine == authored,
            };
        }
    };
    let edit = match read_outline(authored) {
        Ok(outline) => outline,
        Err(reason) => {
            return EditReport {
                violations: vec![Violation::SourceInvalid {
                    side: "authored",
                    reason,
                }],
                changed: Vec::new(),
                changed_defaults: Vec::new(),
                added_classes: Vec::new(),
                added_functions: Vec::new(),
                new_strings: Vec::new(),
                new_static_names: Vec::new(),
                unchanged: pristine == authored,
            };
        }
    };

    let base_identity_counts = class_identity_counts(&base);
    let edit_identity_counts = class_identity_counts(&edit);
    for (side, counts) in [
        ("pristine", &base_identity_counts),
        ("authored", &edit_identity_counts),
    ] {
        for (identity, found) in counts.iter().filter(|(_, count)| **count > 1) {
            violations.push(Violation::DuplicateClassIdentity {
                side,
                identity: identity.clone(),
                found: *found,
            });
        }
    }
    for (side, outline) in [("pristine", &base), ("authored", &edit)] {
        for (name, identities) in bare_class_identities(outline)
            .into_iter()
            .filter(|(_, identities)| identities.len() > 1)
        {
            violations.push(Violation::AmbiguousClassName {
                side,
                name,
                identities: identities.into_iter().collect(),
            });
        }
    }

    let base_identities = base_identity_counts
        .keys()
        .cloned()
        .collect::<BTreeSet<_>>();
    let edit_identities = edit_identity_counts
        .keys()
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut moved_names = BTreeSet::new();
    for base_class in &base.classes {
        let identity = class_identity(base_class);
        if edit_identities.contains(&identity) {
            continue;
        }
        let same_name = edit
            .classes
            .iter()
            .filter(|class| class.name == base_class.name)
            .collect::<Vec<_>>();
        if let [moved] = same_name.as_slice() {
            violations.push(Violation::ClassNamespaceChanged {
                name: base_class.name.clone(),
                expected: base_class.namespace.clone(),
                found: moved.namespace.clone(),
            });
            moved_names.insert(base_class.name.clone());
        } else if same_name.is_empty() {
            violations.push(Violation::ClassRemoved { name: identity });
        }
    }
    let added_identities = edit_identities
        .difference(&base_identities)
        .filter(|identity| {
            let name = identity
                .rsplit("::")
                .next()
                .unwrap_or_else(|| identity.as_str());
            !moved_names.contains(name)
        })
        .cloned()
        .collect::<BTreeSet<_>>();
    let added_classes = edit
        .classes
        .iter()
        .filter(|class| added_identities.contains(&class_identity(class)))
        .map(|class| class.name.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let edit_names: BTreeSet<&str> = edit
        .classes
        .iter()
        .map(|class| class.name.as_str())
        .collect();

    let mut remaining_functions = edit.functions.clone();
    for declaration in &base.functions {
        if let Some(position) = remaining_functions
            .iter()
            .position(|item| item == declaration)
        {
            remaining_functions.remove(position);
        } else {
            violations.push(Violation::FunctionRemoved {
                declaration: declaration.clone(),
            });
        }
    }
    let base_function_counts = function_counts(&base.functions);
    for (declaration, found) in function_counts(&edit.functions) {
        let expected = base_function_counts
            .get(&declaration)
            .copied()
            .unwrap_or(0)
            .max(1);
        if found > expected {
            violations.push(Violation::DuplicateFunctionIdentity {
                declaration,
                expected,
                found,
            });
        }
    }

    let mut changed = Vec::new();
    let mut changed_defaults = Vec::new();
    for base_class in &base.classes {
        let identity = class_identity(base_class);
        let Some(edit_class) = edit
            .classes
            .iter()
            .find(|class| class_identity(class) == identity)
        else {
            continue;
        };
        if edit_class.kind != base_class.kind {
            violations.push(Violation::ClassKindChanged {
                name: identity.clone(),
                expected: base_class.kind.clone(),
                found: edit_class.kind.clone(),
            });
        }
        if edit_class.super_class != base_class.super_class {
            violations.push(Violation::ClassReparented {
                name: identity.clone(),
                expected: base_class.super_class.clone(),
                found: edit_class.super_class.clone(),
            });
        }
        if edit_class.fields != base_class.fields {
            violations.push(Violation::FieldsChanged {
                class: identity.clone(),
                expected: base_class.fields.len(),
                found: edit_class.fields.len(),
            });
        }
        if edit_class.members.len() != base_class.members.len() {
            violations.push(Violation::MembersChanged {
                class: identity.clone(),
                expected: base_class.members.len(),
                found: edit_class.members.len(),
            });
            continue;
        }
        for (expected, found) in base_class.members.iter().zip(&edit_class.members) {
            if expected != found {
                violations.push(Violation::MemberSignatureChanged {
                    class: identity.clone(),
                    expected: expected.clone(),
                    found: found.clone(),
                });
            }
        }

        if (checkout.default_classes.contains(&base_class.name)
            || checkout.default_classes.contains(&identity))
            && edit_class.defaults.is_empty()
        {
            violations.push(Violation::MissingClassDefaults {
                class: identity.clone(),
            });
        }
        let mut base_targets = std::collections::BTreeMap::<&str, usize>::new();
        let mut edit_targets = std::collections::BTreeMap::<&str, usize>::new();
        for default in &base_class.defaults {
            *base_targets.entry(default.target.as_str()).or_default() += 1;
        }
        for default in &edit_class.defaults {
            *edit_targets.entry(default.target.as_str()).or_default() += 1;
        }
        for (target, expected) in base_targets {
            let found = edit_targets.get(target).copied().unwrap_or(0);
            if found < expected {
                violations.push(Violation::DefaultTargetRemoved {
                    class: identity.clone(),
                    target: target.to_owned(),
                    expected,
                    found,
                });
            }
        }
        let targets = base_class
            .defaults
            .iter()
            .chain(&edit_class.defaults)
            .map(|default| default.target.as_str())
            .collect::<BTreeSet<_>>();
        for target in targets {
            let base_statements = base_class
                .defaults
                .iter()
                .filter(|default| default.target == target)
                .map(|default| default.statement.as_str())
                .collect::<Vec<_>>();
            let edit_statements = edit_class
                .defaults
                .iter()
                .filter(|default| default.target == target)
                .map(|default| default.statement.as_str())
                .collect::<Vec<_>>();
            if base_statements != edit_statements {
                changed_defaults.push(ChangedDefault {
                    class: identity.clone(),
                    target: target.to_owned(),
                });
            }
        }
    }

    if !checkout.default_classes.is_empty() {
        violations.extend(
            checkout
                .unsupported_generated_methods
                .iter()
                .cloned()
                .map(|name| Violation::UnsupportedGeneratedMethod { name }),
        );
    }

    let mut added_casefold = BTreeMap::<String, String>::new();
    for class in edit
        .classes
        .iter()
        .filter(|class| added_identities.contains(&class_identity(class)))
    {
        let folded = class.name.to_ascii_lowercase();
        if let Some(first) = added_casefold.get(&folded) {
            if first != &class.name {
                violations.push(Violation::NewTypeCollision {
                    first: first.clone(),
                    second: class.name.clone(),
                });
            }
        } else {
            added_casefold.insert(folded, class.name.clone());
        }
        if let Some(existing) = known.existing_type_case_insensitive(&class.name) {
            violations.push(Violation::ExistingTypeCollision {
                name: class.name.clone(),
                existing: existing.to_owned(),
            });
        }
        if let Some(parent) = class.super_class.as_deref() {
            if !added_parent_resolves(class, parent, &edit_identities, &edit_names, known) {
                violations.push(Violation::UnknownType {
                    name: parent.to_owned(),
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

    let (name_violations, new_strings, new_static_names) =
        unknown_names(pristine, authored, known, &edit_names);
    violations.extend(name_violations);

    EditReport {
        unchanged: pristine == authored,
        violations,
        changed,
        changed_defaults,
        added_classes,
        added_functions: remaining_functions,
        new_strings,
        new_static_names,
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

fn member_label(tokens: &[Token]) -> String {
    let mut start = 0usize;
    if tokens
        .first()
        .is_some_and(|token| token.text == "UFUNCTION")
        && tokens.get(1).is_some_and(|token| token.text == "(")
    {
        let mut depth = 0usize;
        for (index, token) in tokens.iter().enumerate().skip(1) {
            match token.text.as_str() {
                "(" => depth += 1,
                ")" => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        start = index + 1;
                        break;
                    }
                }
                _ => {}
            }
        }
    }
    let mut label = String::new();
    for token in &tokens[start..] {
        match token.text.as_str() {
            "(" | "[" => {
                while label.ends_with(' ') {
                    label.pop();
                }
                label.push_str(&token.text);
            }
            ")" | "]" => {
                while label.ends_with(' ') {
                    label.pop();
                }
                label.push_str(&token.text);
            }
            "," => {
                while label.ends_with(' ') {
                    label.pop();
                }
                label.push_str(", ");
            }
            "::" | "." => {
                while label.ends_with(' ') {
                    label.pop();
                }
                label.push_str(&token.text);
            }
            _ => {
                if !label.is_empty()
                    && !label
                        .chars()
                        .last()
                        .is_some_and(|last| matches!(last, ' ' | '(' | '[' | ':' | '.'))
                {
                    label.push(' ');
                }
                label.push_str(&token.text);
            }
        }
    }
    label
}

/// Every member body in a module's source, keyed by `(class, declaration)`.
fn bodies(source: &str) -> Vec<((String, String), String)> {
    let Ok(tokens) = tokenize(source) else {
        return Vec::new();
    };
    let Ok(pairs) = brace_pairs(&tokens) else {
        return Vec::new();
    };
    let Ok(items) = module_items(&tokens, &pairs) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for span in items.classes {
        let Some(class) = tokens
            .get(span.declaration + 1)
            .map(|token| qualified_name(&span.namespace, &token.text))
        else {
            continue;
        };
        let mut index = span.open + 1;
        let mut item_start = index;
        while index < span.close {
            if tokens[index].text == "default" {
                while index < span.close && tokens[index].text != ";" {
                    index += 1;
                }
                index = (index + 1).min(span.close);
                item_start = index;
                continue;
            }
            match tokens[index].text.as_str() {
                "{" => {
                    let Some(end) = pairs[index] else {
                        return Vec::new();
                    };
                    let declaration = member_label(&tokens[item_start..index]);
                    if tokens[item_start..index]
                        .iter()
                        .any(|token| token.text == "(")
                    {
                        out.push((
                            (class.clone(), declaration),
                            normalized(&tokens[index + 1..end]),
                        ));
                    }
                    index = end + 1;
                    item_start = index;
                }
                ";" => {
                    index += 1;
                    item_start = index;
                }
                _ => index += 1,
            }
        }
    }
    out
}

/// Types and string literals the authored source introduces that the base cache cannot bind.
fn unknown_names(
    pristine: &str,
    authored: &str,
    known: &KnownNames,
    declared: &BTreeSet<&str>,
) -> (Vec<Violation>, Vec<String>, Vec<String>) {
    let mut violations = Vec::new();
    let mut seen_types = BTreeSet::new();

    for name in static_class_names(authored) {
        if static_class_names(pristine).contains(&name) {
            continue;
        }
        if !known.has_type(&name)
            && !declared.contains(name.as_str())
            && seen_types.insert(name.clone())
        {
            violations.push(Violation::UnknownType { name });
        }
    }

    let base_strings = string_literals(pristine);
    let mut new_strings = Vec::new();
    let mut seen_strings = BTreeSet::new();
    for value in string_literals(authored) {
        if base_strings.contains(&value) {
            continue;
        }
        if !known.strings.contains(&value) && seen_strings.insert(value.clone()) {
            new_strings.push(value);
        }
    }

    let base_static_names = static_name_literals(pristine);
    let mut new_static_names = Vec::new();
    let mut seen_static_names = BTreeSet::new();
    for value in static_name_literals(authored) {
        if base_static_names.contains(&value) {
            continue;
        }
        if !known.static_names.contains(&value) && seen_static_names.insert(value.clone()) {
            new_static_names.push(value);
        }
    }
    (violations, new_strings, new_static_names)
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

/// Double-quoted literals split by their two independent remap domains.
fn quoted_literals(source: &str) -> (BTreeSet<String>, BTreeSet<String>) {
    let mut strings = BTreeSet::new();
    let mut static_names = BTreeSet::new();
    let Ok(tokens) = tokenize(source) else {
        return (strings, static_names);
    };
    for (index, token) in tokens.iter().enumerate() {
        if !token.text.starts_with('"') || !token.text.ends_with('"') {
            continue;
        }
        let value = serde_json::from_str::<String>(&token.text).unwrap_or_else(|_| {
            token
                .text
                .trim_start_matches('"')
                .trim_end_matches('"')
                .to_owned()
        });
        if value.is_empty() {
            continue;
        }
        if tokens
            .get(index.wrapping_sub(1))
            .is_some_and(|prefix| prefix.word && prefix.text == "n")
        {
            static_names.insert(value);
        } else {
            strings.insert(value);
        }
    }
    (strings, static_names)
}

fn string_literals(source: &str) -> BTreeSet<String> {
    quoted_literals(source).0
}

fn static_name_literals(source: &str) -> BTreeSet<String> {
    quoted_literals(source).1
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parsed_default_target(statement: &str) -> Result<String, String> {
        let tokens = tokenize(statement)?;
        default_target(&tokens[1..tokens.len() - 1])
    }

    #[test]
    fn class_default_targets_use_the_compilers_semantic_root() {
        assert_eq!(
            parsed_default_target("default this.Rules.HideIfKnows(this);").unwrap(),
            "Rules"
        );
        assert_eq!(
            parsed_default_target("default G1R::Register(this);").unwrap(),
            "G1R::Register"
        );
        assert_eq!(
            parsed_default_target("default ::G1R::Register(this);").unwrap(),
            "G1R::Register"
        );
    }

    const PRISTINE: &str = r#"
class UChoiceOne : UTopic_Hero__NPC
{
    default Caption = LocText("EXISTING_KEY");
    default PriorityRank = 2;
    default Rules.HideIfKnows(this);
    default bIsFollowupTopic = false;

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

    fn checkout(source: &str) -> Checkout {
        Checkout {
            module: "Dialog.NPC".to_owned(),
            relative_path: "Dialog/NPC.as".to_owned(),
            source: source.to_owned(),
            default_classes: ["UChoiceOne".to_owned()].into_iter().collect(),
            unsupported_generated_methods: Vec::new(),
        }
    }

    fn known() -> KnownNames {
        KnownNames {
            types: ["UChoiceOne".to_owned(), "UTopic_Hero__NPC".to_owned()]
                .into_iter()
                .collect(),
            native_conversation_bases: NATIVE_CONVERSATION_BASES
                .into_iter()
                .map(str::to_owned)
                .collect(),
            strings: ["EXISTING_KEY".to_owned()].into_iter().collect(),
            static_names: BTreeSet::new(),
        }
    }

    fn empty_checkout() -> Checkout {
        Checkout {
            module: "Story.G1R.Conversation.Conversation_TEST_NPC".to_owned(),
            relative_path: "Story/G1R/Conversation/Conversation_TEST_NPC.as".to_owned(),
            source: String::new(),
            default_classes: BTreeSet::new(),
            unsupported_generated_methods: Vec::new(),
        }
    }

    const NEW_CONVERSATION: &str = r#"
namespace G1R::Conversation
{
class UConversationCharacterSettings_G1R_TEST_NPC : UConversationCharacterSettings
{
    UConversationCharacterSettings_G1R_TEST_NPC()
    {
        super();
        return;
    }
}

class UTopic_Hero__TEST_NPC : UG1RDialogTopic
{
}

class UChoiceTestStart : UTopic_Hero__TEST_NPC
{
    default Caption = LocText("TEST_START");
    default PriorityRank = 1;

    UFUNCTION()
    void Act_Implementation()
    {
        Subdialog(this, UChoiceTestChild::StaticClass());
        return;
    }
}

class UChoiceTestChild : UTopic_Hero__TEST_NPC
{
    default Caption = LocText("TEST_CHILD");
    default PriorityRank = 1;
    default bIsSubTopic = true;

    UFUNCTION()
    void Act_Implementation()
    {
        this.EndConversation();
        return;
    }
}
}
"#;

    #[test]
    fn an_untouched_checkout_is_carryable_and_reports_nothing_changed() {
        let report = verify(&checkout(PRISTINE), PRISTINE, &known());
        assert!(report.is_carryable(), "{:?}", report.violations);
        assert!(report.unchanged);
        assert!(report.changed.is_empty());
        assert!(report.changed_defaults.is_empty());
    }

    #[test]
    fn a_body_edit_is_carryable_and_names_the_method() {
        let edited = PRISTINE.replace("this.EndConversation();", "this.ReturnToLastSelection();");
        let report = verify(&checkout(PRISTINE), &edited, &known());
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
    fn a_namespaced_subdialog_body_edit_is_reported() {
        let pristine = format!("namespace G1R::Conversation\n{{\n{PRISTINE}\n}}\n");
        let edited = pristine.replace(
            "this.EndConversation();",
            "Subdialog(this, UChoiceOne::StaticClass());",
        );
        let report = verify(&checkout(&pristine), &edited, &known());
        assert!(report.is_carryable(), "{:?}", report.violations);
        assert!(report.changed.iter().any(|body| {
            body.class == "G1R::Conversation::UChoiceOne"
                && body.member.contains("Act_Implementation")
        }));
    }

    #[test]
    fn an_untouched_namespaced_checkout_preserves_the_qualified_identity() {
        let pristine = format!("namespace G1R::Conversation\n{{\n{PRISTINE}\n}}\n");
        let report = verify(&checkout(&pristine), &pristine, &known());
        assert!(report.is_carryable(), "{:?}", report.violations);
        assert!(report.unchanged);
        let outline = read_outline(&pristine).unwrap();
        assert_eq!(outline.classes[0].namespace, "G1R::Conversation");
    }

    #[test]
    fn moving_an_existing_class_out_of_its_namespace_is_refused() {
        let pristine = format!("namespace G1R::Conversation\n{{\n{PRISTINE}\n}}\n");
        let report = verify(&checkout(&pristine), PRISTINE, &known());
        assert!(report.violations.iter().any(|violation| matches!(
            violation,
            Violation::ClassNamespaceChanged {
                name,
                expected,
                found,
            } if name == "UChoiceOne" && expected == "G1R::Conversation" && found.is_empty()
        )));
    }

    #[test]
    fn a_new_topic_resolves_a_bare_module_parent_only_in_the_same_namespace() {
        let pristine = format!("namespace G1R::Conversation\n{{\n{PRISTINE}\n}}\n");
        let same_namespace = format!(
            "namespace G1R::Conversation\n{{\n{PRISTINE}\nclass UChoiceTwo : UChoiceOne {{ default Caption = LocText(\"NEW_KEY\"); }}\n}}\n"
        );
        let report = verify(&checkout(&pristine), &same_namespace, &known());
        assert!(report.is_carryable(), "{:?}", report.violations);
        assert_eq!(report.added_classes, ["UChoiceTwo"]);

        let global = format!(
            "{pristine}\nclass UChoiceTwo : UChoiceOne {{ default Caption = LocText(\"NEW_KEY\"); }}\n"
        );
        let report = verify(&checkout(&pristine), &global, &known());
        assert!(report.violations.contains(&Violation::UnknownType {
            name: "UChoiceOne".to_owned()
        }));
    }

    #[test]
    fn a_new_class_in_another_namespace_can_use_a_qualified_module_parent() {
        let pristine = format!("namespace G1R::Conversation\n{{\n{PRISTINE}\n}}\n");
        let edited = format!(
            "{pristine}\nnamespace Modded {{ class UChoiceTwo : G1R::Conversation::UChoiceOne {{ default Caption = LocText(\"NEW_KEY\"); }} }}\n"
        );
        let report = verify(&checkout(&pristine), &edited, &known());
        assert!(report.is_carryable(), "{:?}", report.violations);
        assert_eq!(report.added_classes, ["UChoiceTwo"]);
    }

    #[test]
    fn duplicate_and_ambiguous_class_identities_are_refused() {
        let pristine = format!("namespace A {{ {PRISTINE} }}");
        let duplicate = format!("{pristine}\nnamespace A {{ {PRISTINE} }}");
        let report = verify(&checkout(&pristine), &duplicate, &known());
        assert!(report.violations.iter().any(|violation| matches!(
            violation,
            Violation::DuplicateClassIdentity { side: "authored", identity, found }
                if identity == "A::UChoiceOne" && *found == 2
        )));

        let ambiguous = format!("{pristine}\nnamespace B {{ {PRISTINE} }}");
        let report = verify(&checkout(&pristine), &ambiguous, &known());
        assert!(report.violations.iter().any(|violation| matches!(
            violation,
            Violation::AmbiguousClassName { side: "authored", name, .. }
                if name == "UChoiceOne"
        )));
    }

    #[test]
    fn changing_a_class_into_a_struct_is_refused() {
        let edited = PRISTINE.replacen("class UChoiceOne", "struct UChoiceOne", 1);
        let report = verify(&checkout(PRISTINE), &edited, &known());
        assert!(report.violations.iter().any(|violation| matches!(
            violation,
            Violation::ClassKindChanged { name, expected, found }
                if name == "UChoiceOne" && expected == "class" && found == "struct"
        )));
    }

    #[test]
    fn namespaced_free_functions_are_inventoried_and_duplicate_helpers_are_refused() {
        let pristine = format!(
            "namespace G1R::Conversation {{\nFText Caption(const FName Text) {{ return FText::FromString(Text.ToString()); }}\n{PRISTINE}\n}}"
        );
        let untouched = verify(&checkout(&pristine), &pristine, &known());
        assert!(untouched.is_carryable(), "{:?}", untouched.violations);
        let outline = read_outline(&pristine).unwrap();
        assert_eq!(
            outline.functions,
            ["G1R::Conversation::FText Caption ( const FName Text )"]
        );

        let duplicate = pristine.replace(
            PRISTINE,
            &format!(
                "FText Caption(const FName Text) {{ return FText::FromString(Text.ToString()); }}\n{PRISTINE}"
            ),
        );
        let report = verify(&checkout(&pristine), &duplicate, &known());
        assert!(report.violations.iter().any(|violation| matches!(
            violation,
            Violation::DuplicateFunctionIdentity { declaration, expected: 1, found: 2 }
                if declaration.starts_with("G1R::Conversation::FText Caption")
        )));
    }

    #[test]
    fn free_function_default_arguments_are_inventoried_without_admitting_assignments() {
        let pristine = format!(
            "namespace G1R::Conversation {{\nvoid Helper(int Count = 1, FName Key = n\"KEY\") {{ return; }}\n{PRISTINE}\n}}"
        );
        let outline =
            read_outline(&pristine).expect("default arguments are part of a function header");
        assert_eq!(
            outline.functions,
            ["G1R::Conversation::void Helper ( int Count = 1 , FName Key = n \"KEY\" )"]
        );
        let untouched = verify(&checkout(&pristine), &pristine, &known());
        assert!(untouched.is_carryable(), "{:?}", untouched.violations);

        let assigned =
            format!("{PRISTINE}\nvoid HiddenFactory(int Count = 1) = Factory() {{ return; }}\n");
        let report = verify(&checkout(PRISTINE), &assigned, &known());
        assert!(report.violations.iter().any(|violation| matches!(
            violation,
            Violation::SourceInvalid { side: "authored", reason }
                if reason.contains("unsupported module-scope declaration")
        )));
    }

    #[test]
    fn a_new_namespaced_free_function_requires_new_symbols() {
        let pristine = format!("namespace G1R::Conversation {{ {PRISTINE} }}");
        let edited = pristine.replace(
            PRISTINE,
            &format!("void Helper() {{ return; }}\n{PRISTINE}"),
        );
        let report = verify(&checkout(&pristine), &edited, &known());
        assert!(report.is_carryable(), "{:?}", report.violations);
        assert_eq!(
            report.added_functions,
            ["G1R::Conversation::void Helper ( )"]
        );
        assert!(report.requires_new_symbols());
    }

    #[test]
    fn unsupported_module_scope_declarations_fail_closed() {
        for declaration in [
            "enum EState { Idle }",
            "int HiddenGlobal;",
            "auto HiddenFactory = function() { return 1; }",
        ] {
            let edited = format!("{PRISTINE}\n{declaration}\n");
            let report = verify(&checkout(PRISTINE), &edited, &known());
            assert!(
                report.violations.iter().any(|violation| matches!(
                    violation,
                    Violation::SourceInvalid { side: "authored", reason }
                        if reason.contains("unsupported module-scope declaration")
                )),
                "{declaration}: {:?}",
                report.violations
            );
        }
    }

    #[test]
    fn non_ascii_outside_comments_and_literals_fails_closed_without_panicking() {
        let authored = format!("\u{feff}{PRISTINE}");
        let report = verify(&checkout(PRISTINE), &authored, &known());
        assert!(report.violations.iter().any(|violation| matches!(
            violation,
            Violation::SourceInvalid { side: "authored", reason }
                if reason.contains("non-ASCII character") && reason.contains("line 1")
        )));

        let ordinary = format!(
            "// Grüße\n/* Käse */\n{}",
            PRISTINE.replace("EXISTING_KEY", "GRÜSSE")
        );
        let outline = read_outline(&ordinary).expect("comments and literals may contain Unicode");
        assert_eq!(outline.classes.len(), 1);
        assert_eq!(outline.classes[0].defaults.len(), 4);
    }

    #[test]
    fn a_new_same_module_topic_is_accepted_and_requires_new_symbols() {
        let edited = format!(
            "{PRISTINE}\nclass UChoiceTwo : UChoiceOne\n{{\n    default Caption = LocText(\"NEW_KEY\");\n}}\n"
        );
        let report = verify(&checkout(PRISTINE), &edited, &known());
        assert!(report.is_carryable(), "{:?}", report.violations);
        assert_eq!(report.added_classes, ["UChoiceTwo"]);
        assert!(report.requires_new_symbols());
        assert_eq!(report.new_strings, ["NEW_KEY"]);
    }

    #[test]
    fn a_new_class_may_not_reuse_a_base_cache_type_case_insensitively() {
        let edited = format!(
            "{PRISTINE}\nclass uexistingcachetopic : UChoiceOne\n{{\n    default Caption = LocText(\"NEW_KEY\");\n}}\n"
        );
        let mut names = known();
        names.types.insert("UExistingCacheTopic".to_owned());
        let report = verify(&checkout(PRISTINE), &edited, &names);
        assert!(report
            .violations
            .contains(&Violation::ExistingTypeCollision {
                name: "uexistingcachetopic".to_owned(),
                existing: "UExistingCacheTopic".to_owned(),
            }));
    }

    #[test]
    fn a_new_class_may_not_impersonate_an_admitted_native_conversation_base() {
        let edited = format!(
            "{PRISTINE}\nclass ug1rdialogtopic : UChoiceOne\n{{\n    default Caption = LocText(\"NEW_KEY\");\n}}\n"
        );
        let report = verify(&checkout(PRISTINE), &edited, &known());
        assert!(
            report
                .violations
                .contains(&Violation::ExistingTypeCollision {
                    name: "ug1rdialogtopic".to_owned(),
                    existing: "UG1RDialogTopic".to_owned(),
                })
        );
    }

    #[test]
    fn two_new_classes_may_not_differ_only_by_case() {
        let edited = format!(
            "{PRISTINE}\nclass UNewTopic : UChoiceOne {{}}\nclass unewtopic : UChoiceOne {{}}\n"
        );
        let report = verify(&checkout(PRISTINE), &edited, &known());
        assert!(
            report.violations.contains(&Violation::NewTypeCollision {
                first: "UNewTopic".to_owned(),
                second: "unewtopic".to_owned(),
            })
        );
    }

    #[test]
    fn a_complete_new_conversation_scaffold_accepts_only_proven_native_parents() {
        let report = verify(&empty_checkout(), NEW_CONVERSATION, &known());
        assert!(report.is_carryable(), "{:?}", report.violations);
        assert_eq!(
            report.added_classes,
            [
                "UChoiceTestChild",
                "UChoiceTestStart",
                "UConversationCharacterSettings_G1R_TEST_NPC",
                "UTopic_Hero__TEST_NPC",
            ]
        );
        assert_eq!(report.new_strings, ["TEST_CHILD", "TEST_START"]);
        assert!(report.requires_new_symbols());
    }

    #[test]
    fn a_new_conversation_refuses_a_native_parent_not_proven_by_the_target_cache() {
        let mut names = known();
        names
            .native_conversation_bases
            .remove("UConversationCharacterSettings");
        let report = verify(&empty_checkout(), NEW_CONVERSATION, &names);
        assert!(report.violations.contains(&Violation::UnknownType {
            name: "UConversationCharacterSettings".to_owned()
        }));
    }

    #[test]
    fn arbitrary_native_parents_remain_refused_even_if_a_caller_labels_them_known() {
        let mut names = known();
        names
            .native_conversation_bases
            .insert("UInventedNativeBase".to_owned());
        let source =
            "class UTopic_Hero__TEST_NPC : UInventedNativeBase { default PriorityRank = 1; }";
        let report = verify(&empty_checkout(), source, &names);
        assert!(report.violations.contains(&Violation::UnknownType {
            name: "UInventedNativeBase".to_owned()
        }));
    }

    #[test]
    fn native_conversation_parents_are_admitted_only_in_direct_base_position() {
        let source = NEW_CONVERSATION.replace(
            "this.EndConversation();",
            "Subdialog(this, UConversationCharacterSettings::StaticClass());",
        );
        let report = verify(&empty_checkout(), &source, &known());
        assert!(report.violations.contains(&Violation::UnknownType {
            name: "UConversationCharacterSettings".to_owned()
        }));
    }

    #[test]
    fn a_struct_cannot_use_the_audited_native_conversation_parent_gate() {
        let source = "struct UFakeSettings : UConversationCharacterSettings { }";
        let report = verify(&empty_checkout(), source, &known());
        assert!(report.violations.contains(&Violation::UnknownType {
            name: "UConversationCharacterSettings".to_owned()
        }));
    }

    #[test]
    fn a_globally_qualified_native_parent_does_not_bypass_the_exact_gate() {
        let source = "class UFakeSettings : ::UConversationCharacterSettings { }";
        let report = verify(&empty_checkout(), source, &known());
        assert!(report.violations.contains(&Violation::UnknownType {
            name: "::UConversationCharacterSettings".to_owned()
        }));
    }

    #[test]
    fn caption_priority_rules_and_flags_are_editable() {
        let edited = PRISTINE
            .replace("LocText(\"EXISTING_KEY\")", "LocText(\"NEW_KEY\")")
            .replace("PriorityRank = 2", "PriorityRank = 7")
            .replace(
                "Rules.HideIfKnows(this)",
                "Rules.AllowIfCharacterHasKnowledgeOf(this, n\"KNOWS\")",
            )
            .replace("bIsFollowupTopic = false", "bIsFollowupTopic = true");
        let report = verify(&checkout(PRISTINE), &edited, &known());
        assert!(report.is_carryable(), "{:?}", report.violations);
        let targets = report
            .changed_defaults
            .iter()
            .map(|change| change.target.as_str())
            .collect::<BTreeSet<_>>();
        assert_eq!(
            targets,
            ["Caption", "PriorityRank", "Rules", "bIsFollowupTopic"]
                .into_iter()
                .collect()
        );
        assert!(report.requires_new_symbols());
    }

    #[test]
    fn a_partial_or_empty_default_set_is_refused() {
        let partial = PRISTINE.replace("    default PriorityRank = 2;\n", "");
        let report = verify(&checkout(PRISTINE), &partial, &known());
        assert!(report.violations.iter().any(|violation| matches!(
            violation,
            Violation::DefaultTargetRemoved { target, .. } if target == "PriorityRank"
        )));

        let empty = PRISTINE
            .lines()
            .filter(|line| !line.trim_start().starts_with("default "))
            .collect::<Vec<_>>()
            .join("\n");
        let report = verify(&checkout(PRISTINE), &empty, &known());
        assert!(report.violations.iter().any(|violation| matches!(
            violation,
            Violation::MissingClassDefaults { class } if class == "UChoiceOne"
        )));
    }

    #[test]
    fn preprocessor_conditionals_cannot_satisfy_authored_default_coverage() {
        let conditional = PRISTINE
            .replace(
                "    default Caption = LocText(\"EXISTING_KEY\");",
                "    #if WITH_DIALOG_CAPTION\n    default Caption = LocText(\"EXISTING_KEY\");\n    #endif",
            );
        let report = verify(&checkout(PRISTINE), &conditional, &known());
        assert!(
            report.violations.iter().any(|violation| matches!(
                violation,
                Violation::SourceInvalid { side: "authored", reason }
                    if reason.contains("preprocessor directive `#if`")
                        && reason.contains("before compiler preprocessing")
            )),
            "{:?}",
            report.violations
        );

        let conditional_else = PRISTINE.replace(
            "    default PriorityRank = 2;",
            "    #if KEEP_SHIPPED_PRIORITY\n    default PriorityRank = 2;\n    #else\n    default PriorityRank = 9;\n    #endif",
        );
        let report = verify(&checkout(PRISTINE), &conditional_else, &known());
        assert!(
            report.violations.iter().any(|violation| matches!(
                violation,
                Violation::SourceInvalid { side: "authored", reason }
                    if reason.contains("preprocessor directive `#if`")
            )),
            "{:?}",
            report.violations
        );
    }

    #[test]
    fn preprocessor_spelling_in_comments_and_literals_remains_ordinary_source() {
        let source = format!(
            "// #if COMMENT_ONLY\n/* #else */\n{}\n// #endif\n",
            PRISTINE.replace("EXISTING_KEY", "#if_LITERAL")
        );
        let outline = read_outline(&source).expect("comments and literals are not directives");
        assert_eq!(outline.classes.len(), 1);
        assert_eq!(outline.classes[0].defaults.len(), 4);
    }

    #[test]
    fn a_new_method_is_refused() {
        let edited = PRISTINE.replace(
            "    UFUNCTION()\n    void Act_Implementation()",
            "    void Helper()\n    {\n        return;\n    }\n    UFUNCTION()\n    void Act_Implementation()",
        );
        let report = verify(&checkout(PRISTINE), &edited, &known());
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
        let report = verify(&checkout(PRISTINE), &edited, &known());
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
        let report = verify(&checkout(PRISTINE), &edited, &known());
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
        let report = verify(&checkout(PRISTINE), &edited, &known());
        assert!(report.is_carryable(), "{:?}", report.violations);
    }

    #[test]
    fn a_brand_new_text_id_selects_new_symbol_remap_and_a_shipped_one_does_not() {
        let invented = PRISTINE.replace("this.EndConversation();", "LocText(\"BRAND_NEW_KEY\");");
        let report = verify(&checkout(PRISTINE), &invented, &known());
        assert!(report.is_carryable(), "{:?}", report.violations);
        assert_eq!(report.new_strings, ["BRAND_NEW_KEY"]);
        assert!(report.requires_new_symbols());

        let shipped = PRISTINE.replace("this.EndConversation();", "LocText(\"EXISTING_KEY\");");
        let report = verify(&checkout(PRISTINE), &shipped, &known());
        assert!(report.is_carryable(), "{:?}", report.violations);
        assert!(!report.requires_new_symbols());
    }

    #[test]
    fn fname_literals_use_the_static_name_domain_even_when_string_text_exists() {
        let mut names = known();
        names.strings.insert("SHARED_TEXT".to_owned());
        names.static_names.insert("KNOWN_FNAME".to_owned());

        let invented = PRISTINE.replace("this.EndConversation();", "Remember(n\"SHARED_TEXT\");");
        let report = verify(&checkout(PRISTINE), &invented, &names);
        assert!(report.is_carryable(), "{:?}", report.violations);
        assert!(report.new_strings.is_empty());
        assert_eq!(report.new_static_names, ["SHARED_TEXT"]);
        assert!(report.requires_new_symbols());

        let shipped = PRISTINE.replace("this.EndConversation();", "Remember(n\"KNOWN_FNAME\");");
        let report = verify(&checkout(PRISTINE), &shipped, &names);
        assert!(report.is_carryable(), "{:?}", report.violations);
        assert!(!report.requires_new_symbols());
    }

    #[test]
    fn a_switch_default_label_is_not_a_class_default() {
        let edited = PRISTINE.replace(
            "this.EndConversation();",
            "switch (1) { default: this.EndConversation(); break; }",
        );
        let report = verify(&checkout(PRISTINE), &edited, &known());
        assert!(report.is_carryable(), "{:?}", report.violations);
    }

    #[test]
    fn another_omitted_generated_method_is_refused_with_authored_defaults() {
        let mut taken = checkout(PRISTINE);
        taken.unsupported_generated_methods = vec!["UChoiceOne::__Factory".to_owned()];
        let report = verify(&taken, PRISTINE, &known());
        assert!(report.violations.iter().any(|violation| matches!(
            violation,
            Violation::UnsupportedGeneratedMethod { name } if name.ends_with("::__Factory")
        )));
    }

    #[test]
    fn the_outline_reads_classes_fields_and_members() {
        let source =
            "class UA : UB\n{\n    int Value;\n    UFUNCTION()\n    void Go()\n    {\n    }\n}\n";
        let outline = read_outline(source).unwrap();
        assert_eq!(outline.classes.len(), 1);
        assert_eq!(outline.classes[0].namespace, "");
        assert_eq!(outline.classes[0].name, "UA");
        assert_eq!(outline.classes[0].kind, "class");
        assert_eq!(outline.classes[0].super_class.as_deref(), Some("UB"));
        assert_eq!(outline.classes[0].fields, vec!["int Value ;".to_owned()]);
        assert_eq!(
            outline.classes[0].members,
            vec!["UFUNCTION ( ) void Go ( )".to_owned()]
        );
    }

    #[test]
    fn namespaced_classes_and_defaults_are_inventoried() {
        let outline = read_outline(
            "namespace G1R::Conversation { class UChoice : UBase { default Caption = LocText(\"ID\"); void Act() { } } }",
        )
        .unwrap();
        assert_eq!(outline.classes.len(), 1);
        assert_eq!(outline.classes[0].namespace, "G1R::Conversation");
        assert_eq!(outline.classes[0].name, "UChoice");
        assert_eq!(outline.classes[0].defaults[0].target, "Caption");
    }
}
