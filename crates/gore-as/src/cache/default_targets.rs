//! Fail-closed preservation proof for authored class defaults.
//!
//! Source-level class presence is not enough to prove that a regenerated `__InitDefaults`
//! retained every assignment/call from the Shipping cache.  This plan inventories the semantic
//! root targeted by every base initializer statement directly from bytecode, proves the authored
//! source contains at least that multiset, then checks the remapped compiler output again.  Values
//! and arguments may change and new targets/classes may be appended; an existing target may not
//! silently disappear.

use std::collections::BTreeMap;

use super::disasm::{disassemble, Instr};
use super::model;
use super::refs::RefResolver;

const MAX_AUTHORED_DEFAULTS: usize = 250_000;

type TargetCounts = BTreeMap<String, usize>;
type ClassTargets = BTreeMap<ClassIdentity, TargetCounts>;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct ClassIdentity {
    namespace: String,
    name: String,
}

impl ClassIdentity {
    fn display(&self) -> String {
        if self.namespace.is_empty() {
            self.name.clone()
        } else {
            format!("{}::{}", self.namespace, self.name)
        }
    }
}

/// Base-bound proof for an existing module whose existing classes author defaults.
#[derive(Clone, Debug)]
pub(crate) struct ExistingDefaultTargetPlan {
    module_name: String,
    expected: ClassTargets,
}

impl ExistingDefaultTargetPlan {
    /// Build the proof before invoking a compiler.  `Ok(None)` means the source has no defaults
    /// for an existing default-bearing class (the defaults-free or appended-class hybrid path).
    pub(crate) fn prepare(
        base_cache: &[u8],
        module_name: &str,
        authored_source: &str,
    ) -> Result<Option<Self>, String> {
        let authored = authored_targets(authored_source)?;
        let base = raw_module_targets(base_cache, module_name)?;
        let authors_existing = base.keys().any(|identity| {
            authored
                .get(identity)
                .is_some_and(|targets| !targets.is_empty())
        });
        if !authors_existing {
            return Ok(None);
        }
        prove_contains(&base, &authored, "authored source")?;
        Ok(Some(Self {
            module_name: module_name.to_owned(),
            expected: base,
        }))
    }

    /// Verify the final one-module mini after remap, optional carry, and metadata restoration.
    pub(crate) fn verify(&self, remapped_mini: &[u8]) -> Result<(), String> {
        let actual = raw_module_targets(remapped_mini, &self.module_name)?;
        prove_contains(&self.expected, &actual, "regenerated module")
    }
}

fn prove_contains(
    expected: &ClassTargets,
    actual: &ClassTargets,
    side: &str,
) -> Result<(), String> {
    for (identity, expected_targets) in expected {
        let actual_targets = actual
            .get(identity)
            .ok_or_else(|| format!("{side} omits default-bearing class {}", identity.display()))?;
        for (target, expected_count) in expected_targets {
            let actual_count = actual_targets.get(target).copied().unwrap_or(0);
            if actual_count < *expected_count {
                return Err(format!(
                    "{side} omits existing default target {}::{target}: expected at least \
                     {expected_count}, found {actual_count}",
                    identity.display()
                ));
            }
        }
    }
    Ok(())
}

fn raw_module_targets(cache: &[u8], module_name: &str) -> Result<ClassTargets, String> {
    let modules = model::parse_modules(cache)
        .map_err(|error| format!("parsing default-target cache: {error}"))?;
    let matches = modules
        .iter()
        .filter(|module| module.name == module_name)
        .collect::<Vec<_>>();
    let [module] = matches.as_slice() else {
        return Err(format!(
            "default-target proof requires exactly one module named {module_name:?}, found {}",
            matches.len()
        ));
    };
    let refs = RefResolver::build(cache)
        .map_err(|error| format!("building default-target resolver: {error}"))?;
    let mut classes = BTreeMap::new();
    for class in &module.classes {
        let initializers = class
            .methods
            .iter()
            .filter(|method| method.name == "__InitDefaults")
            .collect::<Vec<_>>();
        if initializers.is_empty() {
            continue;
        }
        let [initializer] = initializers.as_slice() else {
            return Err(format!(
                "default-target proof found {} __InitDefaults methods in {}::{}",
                initializers.len(),
                module_name,
                class.name
            ));
        };
        let identity = ClassIdentity {
            namespace: class.namespace.clone(),
            name: class.name.clone(),
        };
        let targets = bytecode_targets(&initializer.bytecode, &refs).map_err(|reason| {
            format!(
                "inventorying {}::{}::__InitDefaults: {reason}",
                module_name,
                identity.display()
            )
        })?;
        if classes.insert(identity.clone(), targets).is_some() {
            return Err(format!(
                "default-target proof found duplicate class identity {}",
                identity.display()
            ));
        }
    }
    Ok(classes)
}

fn bytecode_targets(bytecode: &[i32], refs: &RefResolver) -> Result<TargetCounts, String> {
    let instructions = disassemble(bytecode).map_err(|error| error.to_string())?;
    let mut targets = BTreeMap::new();
    for (index, instruction) in instructions.iter().enumerate() {
        if is_unclassified_store(instruction) {
            return Err(format!(
                "unsupported store opcode {} at dword {}; default-target coverage is unproven",
                instruction.op.name, instruction.offset_dw
            ));
        }
        if instruction.op.name.starts_with("WRTV") {
            let member = store_member_operand(&instructions, index).ok_or_else(|| {
                format!(
                    "cannot prove member target for {} at dword {}",
                    instruction.op.name, instruction.offset_dw
                )
            })?;
            let target = resolve_member(member, refs).ok_or_else(|| {
                format!(
                    "cannot resolve member target for {} at dword {}",
                    instruction.op.name, instruction.offset_dw
                )
            })?;
            increment(&mut targets, target)?;
            continue;
        }
        if !is_call(instruction)
            || is_argument_constructor(&instructions, index)
            || is_value_call(&instructions, index)
        {
            continue;
        }
        let symbol = call_symbol(instruction, refs).ok_or_else(|| {
            format!(
                "cannot resolve {} at dword {}",
                instruction.op.name, instruction.offset_dw
            )
        })?;
        if let Some(member) = call_receiver_member_operand(&instructions, index) {
            let target = resolve_member(member, refs).ok_or_else(|| {
                format!(
                    "cannot resolve call receiver target at dword {}",
                    instruction.offset_dw
                )
            })?;
            increment(&mut targets, target)?;
            continue;
        }
        if is_compiler_internal(&symbol) {
            continue;
        }
        increment(&mut targets, symbol)?;
    }
    Ok(targets)
}

fn increment(counts: &mut TargetCounts, target: String) -> Result<(), String> {
    let count = counts.entry(target).or_default();
    *count = count
        .checked_add(1)
        .ok_or_else(|| "default-target count overflow".to_owned())?;
    Ok(())
}

fn store_member_operand(instructions: &[Instr], index: usize) -> Option<&Instr> {
    let before = instructions.get(index.checked_sub(1)?)?;
    if before.op.name == "LoadThisR" {
        return Some(before);
    }
    let mut at = index.checked_sub(1)?;
    if instructions.get(at)?.op.name == "PopRPtr" {
        at = at.checked_sub(1)?;
    }
    let mut root = None;
    while instructions.get(at)?.op.name == "ADDSi" {
        root = instructions.get(at);
        at = at.checked_sub(1)?;
    }
    (instructions.get(at)?.op.name == "PshVPtr" && instructions.get(at)?.words.first() == Some(&0))
        .then_some(root?)
}

fn call_receiver_member_operand(instructions: &[Instr], index: usize) -> Option<&Instr> {
    let mut at = index.checked_sub(1)?;
    let mut root = None;
    while instructions.get(at)?.op.name == "ADDSi" {
        root = instructions.get(at);
        at = at.checked_sub(1)?;
    }
    (instructions.get(at)?.op.name == "PshVPtr" && instructions.get(at)?.words.first() == Some(&0))
        .then_some(root?)
}

fn is_unclassified_store(instruction: &Instr) -> bool {
    matches!(
        instruction.op.name,
        "STOREOBJ" | "REFCPY" | "CpyVtoV4" | "CpyVtoV8" | "CpyRtoV4" | "CpyRtoV8" | "CpyGtoV4"
    )
}

fn resolve_member(instruction: &Instr, refs: &RefResolver) -> Option<String> {
    let offset = *instruction.words.first()? as i32;
    let owner = *instruction.dwords.first()? as i32;
    refs.member_identity(owner, offset)
        .map(|(name, _)| name.to_owned())
}

fn is_call(instruction: &Instr) -> bool {
    matches!(
        instruction.op.name,
        "CALL" | "CALLBND" | "CALLINTF" | "CALLSYS"
    )
}

fn call_symbol(instruction: &Instr, refs: &RefResolver) -> Option<String> {
    let (name, namespace) = match instruction.op.name {
        "CALL" | "CALLBND" | "CALLINTF" => {
            let id = *instruction.dwords.first()? as i32;
            (refs.func_by_id(id)?, refs.func_ns_by_id(id))
        }
        "CALLSYS" => {
            let pointer = *instruction.qwords.first()? as i64;
            (refs.func_by_ptr(pointer)?, refs.func_ns_by_ptr(pointer))
        }
        _ => return None,
    };
    Some(match namespace {
        Some(namespace) if !namespace.is_empty() => format!("{namespace}::{name}"),
        _ => name.to_owned(),
    })
}

fn is_compiler_internal(name: &str) -> bool {
    let bare = name.rsplit("::").next().unwrap_or(name);
    bare == "__STATIC_NAME"
        || bare.starts_with('$')
        || bare.starts_with('~')
        || bare
            .strip_prefix("op")
            .is_some_and(|rest| rest.starts_with(|character: char| character.is_ascii_uppercase()))
}

fn is_argument_constructor(instructions: &[Instr], index: usize) -> bool {
    let (Some(before), Some(after)) = (
        index.checked_sub(1).and_then(|at| instructions.get(at)),
        instructions.get(index + 1),
    ) else {
        return false;
    };
    before.op.name == "PSF"
        && after.op.name == "PSF"
        && before.words.first().is_some()
        && before.words.first() == after.words.first()
}

fn is_value_call(instructions: &[Instr], index: usize) -> bool {
    instructions.get(index + 1).is_some_and(|next| {
        matches!(
            next.op.name,
            "STOREOBJ" | "PshRPtr" | "CpyRtoV4" | "CpyRtoV8"
        )
    })
}

#[derive(Clone, Debug)]
struct Token {
    text: String,
    line: usize,
    word: bool,
}

fn authored_targets(source: &str) -> Result<ClassTargets, String> {
    super::default_source::reject_preprocessor_directives(source)?;
    let tokens = tokenize(source)?;
    let pairs = brace_pairs(&tokens)?;
    let mut classes = BTreeMap::new();
    scan_source_scope(&tokens, &pairs, 0, tokens.len(), "", &mut classes)?;
    Ok(classes)
}

fn scan_source_scope(
    tokens: &[Token],
    pairs: &[Option<usize>],
    mut at: usize,
    end: usize,
    namespace: &str,
    classes: &mut ClassTargets,
) -> Result<(), String> {
    while at < end {
        if tokens[at].word && tokens[at].text == "namespace" {
            let Some(open) =
                (at + 1..end).find(|index| matches!(tokens[*index].text.as_str(), "{" | ";"))
            else {
                return Err(format!("line {}: namespace has no body", tokens[at].line));
            };
            if tokens[open].text == ";" {
                at = open + 1;
                continue;
            }
            let close = pairs[open]
                .ok_or_else(|| format!("line {}: namespace body is not closed", tokens[at].line))?;
            let local = qualified_tokens(&tokens[at + 1..open])?;
            let nested = if namespace.is_empty() {
                local
            } else {
                format!("{namespace}::{local}")
            };
            scan_source_scope(tokens, pairs, open + 1, close, &nested, classes)?;
            at = close + 1;
            continue;
        }
        if tokens[at].word && matches!(tokens[at].text.as_str(), "class" | "struct") {
            let name = tokens
                .get(at + 1)
                .filter(|token| token.word)
                .ok_or_else(|| format!("line {}: class has no name", tokens[at].line))?;
            let Some(open) =
                (at + 2..end).find(|index| matches!(tokens[*index].text.as_str(), "{" | ";"))
            else {
                return Err(format!("line {}: class has no body", tokens[at].line));
            };
            if tokens[open].text == ";" {
                at = open + 1;
                continue;
            }
            let close = pairs[open]
                .ok_or_else(|| format!("line {}: class body is not closed", tokens[at].line))?;
            let identity = ClassIdentity {
                namespace: namespace.to_owned(),
                name: name.text.clone(),
            };
            let defaults = class_defaults(tokens, pairs, open + 1, close)?;
            if classes.insert(identity.clone(), defaults).is_some() {
                return Err(format!(
                    "authored source contains duplicate class identity {}",
                    identity.display()
                ));
            }
            at = close + 1;
            continue;
        }
        if tokens[at].text == "{" {
            let close = pairs[at]
                .ok_or_else(|| format!("line {}: source block is not closed", tokens[at].line))?;
            at = close + 1;
        } else {
            at += 1;
        }
    }
    Ok(())
}

fn class_defaults(
    tokens: &[Token],
    pairs: &[Option<usize>],
    mut at: usize,
    end: usize,
) -> Result<TargetCounts, String> {
    let mut defaults = BTreeMap::new();
    let mut total = 0usize;
    while at < end {
        if tokens[at].word && tokens[at].text == "default" {
            let mut finish = at + 1;
            while finish < end && tokens[finish].text != ";" {
                if tokens[finish].text == "{" {
                    finish = pairs[finish].ok_or_else(|| {
                        format!("line {}: default expression is not closed", tokens[at].line)
                    })?;
                }
                finish += 1;
            }
            if finish >= end {
                return Err(format!(
                    "line {}: class default has no terminating semicolon",
                    tokens[at].line
                ));
            }
            let target = source_root_target(&tokens[at + 1..finish])
                .map_err(|reason| format!("line {}: {reason}", tokens[at].line))?;
            increment(&mut defaults, target)?;
            total = total
                .checked_add(1)
                .ok_or_else(|| "authored default count overflow".to_owned())?;
            if total > MAX_AUTHORED_DEFAULTS {
                return Err(format!(
                    "authored class has more than {MAX_AUTHORED_DEFAULTS} default statements"
                ));
            }
            at = finish + 1;
            continue;
        }
        if tokens[at].text == "{" {
            at = pairs[at].ok_or_else(|| {
                format!("line {}: class member body is not closed", tokens[at].line)
            })? + 1;
        } else {
            at += 1;
        }
    }
    Ok(defaults)
}

fn source_root_target(tokens: &[Token]) -> Result<String, String> {
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

fn qualified_tokens(tokens: &[Token]) -> Result<String, String> {
    let mut out = String::new();
    for token in tokens {
        if token.word || token.text == ":" {
            out.push_str(&token.text);
        } else {
            return Err(format!(
                "line {}: namespace name contains unsupported token {:?}",
                token.line, token.text
            ));
        }
    }
    if out.is_empty() {
        Err("namespace has no name".to_owned())
    } else {
        Ok(out)
    }
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
                    return Err("source has an unterminated block comment".to_owned());
                }
            }
            quote @ (b'\'' | b'\"') => {
                let token_line = line;
                let start = index;
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
                    return Err("source has an unterminated quoted literal".to_owned());
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
                    "line {line}: non-ASCII source token outside a comment or literal is unsupported"
                ));
            }
            _ => {
                out.push(Token {
                    text: source[index..index + 1].to_owned(),
                    line,
                    word: false,
                });
                index += 1;
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
                let open = stack
                    .pop()
                    .ok_or_else(|| format!("line {}: unmatched closing brace", token.line))?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::isa::OPCODES;

    fn identity(name: &str) -> ClassIdentity {
        ClassIdentity {
            namespace: "G1R::Conversation".to_owned(),
            name: name.to_owned(),
        }
    }

    fn instruction(name: &str, words: &[u16], dwords: &[u32]) -> Instr {
        Instr {
            offset_dw: 0,
            op: OPCODES
                .iter()
                .find(|opcode| opcode.name == name)
                .expect("test opcode exists"),
            words: words.to_vec(),
            dwords: dwords.to_vec(),
            qwords: Vec::new(),
        }
    }

    #[test]
    fn authored_inventory_uses_semantic_roots_and_ignores_other_scopes() {
        let source = r#"
            enum EThing { A, B }
            namespace G1R::Conversation {
                class UTopic {
                    default DebugId = 1;
                    default ForceSettings.RangeType = ERange(3);
                    default Rules.HideIfKnows(UTopic);
                    void Act_Implementation() { default NotAClassDefault = 9; }
                }
                int Unrelated() { return 2; }
            }
        "#;
        let inventory = authored_targets(source).unwrap();
        assert_eq!(
            inventory[&identity("UTopic")],
            BTreeMap::from([
                ("DebugId".to_owned(), 1),
                ("ForceSettings".to_owned(), 1),
                ("Rules".to_owned(), 1),
            ])
        );
    }

    #[test]
    fn preservation_requires_every_existing_target_multiplicity() {
        let class = identity("UTopic");
        let expected = BTreeMap::from([(
            class.clone(),
            BTreeMap::from([("DebugId".to_owned(), 1), ("Rules".to_owned(), 2)]),
        )]);
        let missing = BTreeMap::from([(
            class.clone(),
            BTreeMap::from([("DebugId".to_owned(), 1), ("Rules".to_owned(), 1)]),
        )]);
        let error = prove_contains(&expected, &missing, "authored source").unwrap_err();
        assert!(error.contains("UTopic::Rules"), "{error}");
        assert!(error.contains("expected at least 2, found 1"), "{error}");

        let extended = BTreeMap::from([(
            class,
            BTreeMap::from([
                ("DebugId".to_owned(), 1),
                ("Rules".to_owned(), 3),
                ("NewFlag".to_owned(), 1),
            ]),
        )]);
        prove_contains(&expected, &extended, "authored source").unwrap();
    }

    #[test]
    fn target_operands_are_tied_to_the_terminal_store_or_call() {
        let store = vec![
            instruction("PshVPtr", &[0], &[]),
            instruction("ADDSi", &[10], &[1]),
            instruction("LoadThisR", &[20], &[2]),
            instruction("WRTV4", &[4], &[]),
        ];
        let selected = store_member_operand(&store, 3).expect("direct store target");
        assert_eq!(selected.op.name, "LoadThisR");
        assert_eq!(selected.words, vec![20]);

        let call = vec![
            instruction("PshVPtr", &[0], &[]),
            instruction("ADDSi", &[10], &[1]),
            instruction("PshVPtr", &[0], &[]),
            instruction("ADDSi", &[30], &[3]),
            instruction("CALLSYS", &[], &[]),
        ];
        let selected =
            call_receiver_member_operand(&call, 4).expect("contiguous call receiver target");
        assert_eq!(selected.words, vec![30]);
    }

    #[test]
    fn unsupported_store_families_are_fail_closed() {
        for name in [
            "STOREOBJ", "REFCPY", "CpyVtoV4", "CpyVtoV8", "CpyRtoV4", "CpyRtoV8", "CpyGtoV4",
        ] {
            assert!(
                is_unclassified_store(&instruction(name, &[], &[])),
                "{name}"
            );
        }
        assert!(!is_unclassified_store(&instruction("WRTV4", &[], &[])));
    }

    #[test]
    fn source_inventory_rejects_partial_or_malformed_defaults() {
        let malformed = "class UTopic { default Rules.HideIfKnows(1); default Caption = 2;";
        assert!(authored_targets(malformed).is_err());
        let duplicate = "class UTopic {} class UTopic {}";
        assert!(authored_targets(duplicate)
            .unwrap_err()
            .contains("duplicate class identity"));
    }

    #[test]
    #[ignore = "requires GORE_AS_CACHE, GORE_AS_DEFAULT_SOURCE, and GORE_AS_DEFAULT_MODULE"]
    fn real_authored_source_covers_every_raw_default_target() {
        let cache = std::fs::read(
            std::env::var_os("GORE_AS_CACHE").expect("set GORE_AS_CACHE to the pristine cache"),
        )
        .expect("read GORE_AS_CACHE");
        let source = std::fs::read_to_string(
            std::env::var_os("GORE_AS_DEFAULT_SOURCE")
                .expect("set GORE_AS_DEFAULT_SOURCE to the authored module"),
        )
        .expect("read GORE_AS_DEFAULT_SOURCE");
        let module = std::env::var("GORE_AS_DEFAULT_MODULE")
            .expect("set GORE_AS_DEFAULT_MODULE to the exact cache module name");
        let plan = ExistingDefaultTargetPlan::prepare(&cache, &module, &source)
            .expect("authored defaults must cover the raw target inventory")
            .expect("real oracle source did not author any existing default-bearing class");
        if let Some(regen_path) = std::env::var_os("GORE_AS_REGEN") {
            let regen = std::fs::read(regen_path).expect("read GORE_AS_REGEN");
            plan.verify(&regen)
                .expect("regenerated cache must retain every base default target");
        }
    }

    #[test]
    #[ignore = "requires GORE_AS_CACHE, GORE_AS_INCOMPLETE_DEFAULT_SOURCE, and GORE_AS_DEFAULT_MODULE"]
    fn real_incomplete_authored_source_is_rejected() {
        let cache = std::fs::read(
            std::env::var_os("GORE_AS_CACHE").expect("set GORE_AS_CACHE to the pristine cache"),
        )
        .expect("read GORE_AS_CACHE");
        let source = std::fs::read_to_string(
            std::env::var_os("GORE_AS_INCOMPLETE_DEFAULT_SOURCE")
                .expect("set GORE_AS_INCOMPLETE_DEFAULT_SOURCE to an incomplete authored module"),
        )
        .expect("read GORE_AS_INCOMPLETE_DEFAULT_SOURCE");
        let module = std::env::var("GORE_AS_DEFAULT_MODULE")
            .expect("set GORE_AS_DEFAULT_MODULE to the exact cache module name");
        let error = ExistingDefaultTargetPlan::prepare(&cache, &module, &source)
            .expect_err("an incomplete authored source must fail closed");
        assert!(error.contains("omits existing default target"), "{error}");
    }
}
