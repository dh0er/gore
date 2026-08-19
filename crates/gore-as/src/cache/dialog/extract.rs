//! Recognition of dialog constructs in a topic class's bytecode.
//!
//! The generated code for a conversation is highly regular, and this module leans on that
//! regularity rather than on a general decompiler. Three shapes carry everything:
//!
//! * `__InitDefaults` writes class defaults, either as a direct member store
//!   (`LoadThisR <member>; WRTV<n> <slot>`) or as a call on a member
//!   (`PshVPtr this; ADDSi <member>; CALL<...>`).
//! * `Act_Implementation` is a sequence of statements, each bracketed by the story debugger's
//!   `DC(<id>, 2)` / `DC(<id>, 3)` instrumentation calls, which conveniently delimits statements
//!   for a backward argument scan.
//! * `IsVisible_Implementation` is a sequence of predicate calls combined by branches.
//!
//! Anything outside those shapes is reported, never guessed at: an unrecognized call keeps its
//! resolved symbol name, an unresolvable operand stays [`Arg::Opaque`], and both are counted.

use std::collections::{BTreeSet, HashMap};

use super::super::cfg;
use super::super::disasm::Instr;
use super::super::refs::RefResolver;
use super::super::types::token_keyword;
use super::model::{
    Arg, Caption, Check, CheckSource, Guard, Rule, RuleKind, Setting, Step, StepKind, TopicFlags,
};

/// Story-debugger instrumentation. `DC` brackets a statement, `DB` records a branch decision.
/// Both are compiled into every vanilla conversation and say nothing about the dialog itself.
const INSTRUMENTATION: [&str; 2] = ["DC", "DB"];

/// The synthesized FName-table accessor behind every `n"..."` literal.
const STATIC_NAME: &str = "__STATIC_NAME";

/// Prefix of the generated global that stands for a class reference.
const STATIC_TYPE_PREFIX: &str = "__StaticType_";

/// Upper bound on a backward argument scan, so a pathological function cannot cost quadratic
/// time. Statements in generated conversation code are far shorter than this.
const MAX_STATEMENT_INSTRUCTIONS: usize = 512;

/// How far back a caption assignment's source may sit. The generated composite is six
/// instructions long; the margin covers a differently ordered but equally direct one.
const CAPTION_WINDOW: usize = 16;

/// How far past a call a branch may sit for the call to count as its condition.
const PREDICATE_LOOKAHEAD: usize = 6;

/// Resolve a call instruction to its symbol name.
pub(super) fn call_symbol<'a>(instruction: &Instr, refs: &'a RefResolver) -> Option<&'a str> {
    match instruction.op.name {
        "CALL" | "CALLBND" | "CALLINTF" => refs.func_by_id(*instruction.dwords.first()? as i32),
        "CALLSYS" => refs.func_by_ptr(*instruction.qwords.first()? as i64),
        _ => None,
    }
}

/// A generated operator or lifetime helper (`$beh2`, `~FText`, `$opAssign`) rather than anything
/// the dialog itself does. `structure.rs` skips the same names when rendering source.
fn is_compiler_internal(name: &str) -> bool {
    name.starts_with('$')
        || name.starts_with('~')
        || name
            .strip_prefix("op")
            .is_some_and(|rest| rest.starts_with(|c: char| c.is_ascii_uppercase()))
}

/// A call whose result a branch consumes: `if (IsValid(x))` compiles to the call followed by a
/// conditional jump, with no store in between. That is a condition, not something the topic does.
fn is_predicate_call(instructions: &[Instr], index: usize) -> bool {
    for candidate in instructions
        .iter()
        .skip(index + 1)
        .take(PREDICATE_LOOKAHEAD)
    {
        match candidate.op.name {
            "JZ" | "JNZ" | "JS" | "JNS" | "JP" | "JNP" | "JLowZ" | "JLowNZ" => return true,
            "PSF" | "PshVPtr" | "PshRPtr" | "CHKREF" | "ClrVPtr" | "FREE" | "NOT" | "TZ"
            | "TNZ" => {}
            _ => return false,
        }
    }
    false
}

fn is_call(instruction: &Instr) -> bool {
    matches!(
        instruction.op.name,
        "CALL" | "CALLBND" | "CALLINTF" | "CALLSYS"
    )
}

fn call_params<'a>(
    instruction: &Instr,
    refs: &'a RefResolver,
) -> Option<&'a [super::super::types::DataType]> {
    match instruction.op.name {
        "CALL" | "CALLBND" | "CALLINTF" => {
            refs.func_params_by_id(*instruction.dwords.first()? as i32)
        }
        "CALLSYS" => refs.func_params_by_ptr(*instruction.qwords.first()? as i64),
        _ => None,
    }
}

/// Ends a statement: control flow, a member store, or a function return.
fn ends_statement(instruction: &Instr) -> bool {
    matches!(instruction.op.name, "RET" | "JMP" | "JMPP")
        || instruction.op.name.starts_with("WRTV")
        || matches!(
            instruction.op.name,
            "JZ" | "JNZ" | "JS" | "JNS" | "JP" | "JNP" | "JLowZ" | "JLowNZ"
        )
}

/// A call whose only job is to build an argument for the call that follows it, recognized
/// structurally: the destination slot is pushed both immediately before and immediately after,
/// as in `PGA "KEY"; PSF w6; CALL LocText; PSF w6`.
fn is_argument_constructor(instructions: &[Instr], index: usize) -> bool {
    let (Some(before), Some(after)) = (
        index.checked_sub(1).and_then(|i| instructions.get(i)),
        instructions.get(index + 1),
    ) else {
        return false;
    };
    before.op.name == "PSF"
        && after.op.name == "PSF"
        && before.words.first().is_some()
        && before.words.first() == after.words.first()
}

/// A call used as a value rather than as a statement: its result is stored into a slot or
/// re-pushed as an argument of the enclosing call.
fn is_value_call(instructions: &[Instr], index: usize) -> bool {
    instructions.get(index + 1).is_some_and(|next| {
        matches!(
            next.op.name,
            "STOREOBJ" | "PshRPtr" | "CpyRtoV4" | "CpyRtoV8"
        )
    })
}

/// Constant immediate carried by a `SetV*`/`PshC*` instruction.
fn immediate(instruction: &Instr) -> Option<i64> {
    match instruction.op.name {
        "SetV1" | "SetV2" | "SetV4" | "PshC4" => instruction.dwords.first().map(|v| *v as i64),
        "SetV8" | "PshC8" => instruction.qwords.first().map(|v| *v as i64),
        _ => None,
    }
}

/// Turn one operand push into a resolved argument. Returns `None` for instructions that are not
/// operand pushes at all, so the caller can skip receivers and bookkeeping.
fn push_argument(instruction: &Instr, refs: &RefResolver) -> Option<Arg> {
    match instruction.op.name {
        "PshNull" => Some(Arg::Null),
        "PshGPtr" | "PGA" => {
            let pointer = *instruction.qwords.first()? as i64;
            if let Some(class) = refs.staticclass_class_by_ptr(pointer) {
                return Some(Arg::Class {
                    name: class.to_owned(),
                });
            }
            if refs.global_is_string(pointer) {
                return refs.global_by_ptr(pointer).map(|text| Arg::Text {
                    value: text.to_owned(),
                });
            }
            if let Some(class) = refs.type_by_ptr(pointer) {
                return Some(Arg::Class {
                    name: class.to_owned(),
                });
            }
            let name = refs.global_by_ptr(pointer)?;
            // A `UX::StaticClass()` argument reaches the call site as the generated
            // `__StaticType_UX` global rather than as a call.
            if let Some(class) = name.strip_prefix(STATIC_TYPE_PREFIX) {
                return Some(Arg::Class {
                    name: class.to_owned(),
                });
            }
            Some(match refs.global_ns(pointer) {
                Some(namespace) if !namespace.is_empty() => Arg::Symbol {
                    name: format!("{namespace}::{name}"),
                },
                _ => Arg::Symbol {
                    name: name.to_owned(),
                },
            })
        }
        "PshC4" | "PshC8" => immediate(instruction).map(|value| Arg::Int { value }),
        _ => None,
    }
}

/// Coerce integer immediates to floats where the callee's signature says so. Without this a
/// radius reads as `1134559232` instead of `300`.
fn coerce_arguments(args: &mut [Arg], call: &Instr, refs: &RefResolver) {
    let Some(params) = call_params(call, refs) else {
        return;
    };
    if params.len() != args.len() {
        return;
    }
    for (arg, param) in args.iter_mut().zip(params) {
        if !matches!(token_keyword(param.token), "float" | "float32") {
            continue;
        }
        if let Arg::Int { value } = arg {
            *arg = Arg::Float {
                value: f32::from_bits(*value as u32),
            };
        }
    }
}

/// Collect the arguments of the call at `index`, scanning back to `start`.
///
/// Operands are pushed right to left, so the collected sequence is reversed. `n"..."` literals
/// arrive as a `__STATIC_NAME(<id>)` call whose result is re-pushed, and are resolved here.
fn arguments(instructions: &[Instr], start: usize, index: usize, refs: &RefResolver) -> Vec<Arg> {
    let mut collected = Vec::new();
    let mut position = start;
    while position < index {
        let instruction = &instructions[position];
        if is_call(instruction) {
            if call_symbol(instruction, refs) == Some(STATIC_NAME) {
                // The id was pushed just before; the resolved literal replaces it.
                if let Some(Arg::Int { value }) = collected.pop() {
                    if let Some(name) = refs.static_name(value) {
                        collected.push(Arg::Name {
                            value: name.to_owned(),
                        });
                    } else {
                        collected.push(Arg::Opaque);
                    }
                }
            }
            position += 1;
            continue;
        }
        if let Some(arg) = push_argument(instruction, refs) {
            collected.push(arg);
        }
        position += 1;
    }
    collected.reverse();
    let mut collected = collected;
    coerce_arguments(&mut collected, &instructions[index], refs);
    collected
}

/// Start index of the statement containing `index`.
fn statement_start(instructions: &[Instr], index: usize) -> usize {
    let floor = index.saturating_sub(MAX_STATEMENT_INSTRUCTIONS);
    let mut position = index;
    while position > floor {
        let candidate = position - 1;
        let instruction = &instructions[candidate];
        if ends_statement(instruction) {
            return candidate + 1;
        }
        if is_call(instruction)
            && !is_argument_constructor(instructions, candidate)
            && !is_value_call(instructions, candidate)
        {
            return candidate + 1;
        }
        position = candidate;
    }
    floor
}

/// The receiver member of a call written as `this.<member>...`, e.g. `Rules` in
/// `this.Rules.HideIfKnows(...)`.
fn receiver_member<'a>(
    instructions: &[Instr],
    index: usize,
    refs: &'a RefResolver,
) -> Option<&'a str> {
    let member = instructions.get(index.checked_sub(1)?)?;
    let this = instructions.get(index.checked_sub(2)?)?;
    if member.op.name != "ADDSi" || this.op.name != "PshVPtr" || this.words.first() != Some(&0) {
        return None;
    }
    let offset = *member.words.first()? as i32;
    let type_id = *member.dwords.first()? as i32;
    refs.member(type_id, offset)
}

/// Everything `__InitDefaults` declares for one topic class.
pub(super) struct Defaults {
    pub caption: Caption,
    pub priority: Option<i64>,
    pub flags: TopicFlags,
    pub rules: Vec<Rule>,
    pub settings: Vec<Setting>,
}

impl Default for Defaults {
    fn default() -> Self {
        Self {
            caption: Caption::Unresolved,
            priority: None,
            flags: TopicFlags::default(),
            rules: Vec::new(),
            settings: Vec::new(),
        }
    }
}

fn rule_kind(name: &str) -> RuleKind {
    match name {
        "HideIfKnows" => RuleKind::HideIfKnows,
        "HideIfKnowsId" => RuleKind::HideIfKnowsId,
        "AllowIfCharacterHasKnowledgeOf" => RuleKind::AllowIfCharacterHasKnowledgeOf,
        "AllowIfCharacterHasKnowledgeOfId" => RuleKind::AllowIfCharacterHasKnowledgeOfId,
        "RequireCharacterHasListenedTo" => RuleKind::RequireCharacterHasListenedTo,
        "RequireCharacterHasNotListenedTo" => RuleKind::RequireCharacterHasNotListenedTo,
        "RequireCharacterCloseToWaypoint" => RuleKind::RequireCharacterCloseToWaypoint,
        "Add" => RuleKind::Add,
        other => RuleKind::Other {
            name: other.to_owned(),
        },
    }
}

/// Apply what the closed rule vocabulary knows about its own arguments.
///
/// These rules are native functions, and the cache's tables carry no parameter types for them,
/// so the generic signature-driven coercion cannot fire. The radius of a waypoint rule is a
/// float, and printing its bit pattern as an integer would be worse than useless.
fn type_rule_arguments(kind: &RuleKind, args: &mut [Arg]) {
    if !matches!(kind, RuleKind::RequireCharacterCloseToWaypoint) {
        return;
    }
    if let Some(arg @ Arg::Int { .. }) = args.get_mut(2) {
        if let Arg::Int { value } = arg {
            *arg = Arg::Float {
                value: f32::from_bits(*value as u32),
            };
        }
    }
}

/// Read the generated defaults of one topic class.
pub(super) fn defaults(instructions: &[Instr], refs: &RefResolver) -> Defaults {
    let mut result = Defaults::default();
    let mut slots: HashMap<u16, i64> = HashMap::new();

    for (index, instruction) in instructions.iter().enumerate() {
        if let (Some(slot), Some(value)) = (instruction.words.first(), immediate(instruction)) {
            if instruction.op.name.starts_with("SetV") {
                slots.insert(*slot, value);
            }
        }

        if instruction.op.name == "LoadThisR" {
            let Some(write) = instructions.get(index + 1) else {
                continue;
            };
            if !write.op.name.starts_with("WRTV") {
                continue;
            }
            let (Some(offset), Some(type_id), Some(slot)) = (
                instruction.words.first(),
                instruction.dwords.first(),
                write.words.first(),
            ) else {
                continue;
            };
            let Some(member) = refs.member(*type_id as i32, *offset as i32) else {
                continue;
            };
            let value = slots.get(slot).copied();
            apply_default(&mut result, member, value, instructions, index, refs);
            continue;
        }

        if !is_call(instruction) {
            continue;
        }
        let Some(name) = call_symbol(instruction, refs) else {
            continue;
        };
        if name == STATIC_NAME {
            continue;
        }
        let Some(member) = receiver_member(instructions, index, refs) else {
            continue;
        };
        if member == "Caption" {
            if let Some(caption) = caption_at(instructions, index, refs) {
                result.caption = caption;
            }
            continue;
        }
        let member = member.to_owned();
        let name = name.to_owned();
        let start = statement_start(instructions, index);
        // The receiver push sits between the arguments and the call and is not an argument.
        let args = arguments(instructions, start, index.saturating_sub(2), refs);
        if member == "Rules" {
            let kind = rule_kind(&name);
            let mut args = args;
            type_rule_arguments(&kind, &mut args);
            result.rules.push(Rule { kind, args });
        } else {
            result.settings.push(Setting {
                target: format!("{member}.{name}"),
                args,
            });
        }
    }

    result
}

/// Recognize `default Caption = LocText("KEY")` / `= FText::FromString("...")` at the assignment
/// call that ends the composite.
fn caption_at(instructions: &[Instr], index: usize, refs: &RefResolver) -> Option<Caption> {
    // `PGA "KEY"; PSF <slot>; CALL LocText; PSF <slot>; PshVPtr this; ADDSi Caption; CALLSYS assign`
    let floor = index.saturating_sub(CAPTION_WINDOW);
    let mut position = index;
    while position > floor {
        position -= 1;
        let candidate = &instructions[position];
        if !is_call(candidate) || !is_argument_constructor(instructions, position) {
            continue;
        }
        let source = call_symbol(candidate, refs)?;
        let literal = instructions.get(position.checked_sub(2)?)?;
        if literal.op.name != "PGA" {
            return None;
        }
        let pointer = *literal.qwords.first()? as i64;
        if !refs.global_is_string(pointer) {
            return None;
        }
        let text = refs.global_by_ptr(pointer)?.to_owned();
        if text.trim().is_empty() {
            return None;
        }
        return Some(if source == "LocText" {
            Caption::LocKey { key: text }
        } else {
            Caption::Literal { text }
        });
    }
    None
}

/// Apply one `this.<member> = <value>` default.
fn apply_default(
    result: &mut Defaults,
    member: &str,
    value: Option<i64>,
    instructions: &[Instr],
    index: usize,
    refs: &RefResolver,
) {
    match member {
        "bIsSubTopic" => result.flags.is_sub_topic = value.unwrap_or(0) != 0,
        "bIsAmbientTopic" => result.flags.is_ambient = value.unwrap_or(0) != 0,
        "bIsFollowupTopic" => result.flags.is_followup = value.unwrap_or(0) != 0,
        "ForCharacter" | "WithCharacter" => {
            let name = nearest_static_name(instructions, index, refs);
            if member == "ForCharacter" {
                result.flags.for_character = name;
            } else {
                result.flags.with_character = name;
            }
        }
        "Caption" => {}
        // `PriorityRank` orders the menu; the engine default arrives as the unsigned spelling
        // of -1, which is an absent rank rather than the largest one.
        "PriorityRank" => {
            result.priority = value.filter(|rank| *rank != u32::MAX as i64);
        }
        // Story-debugger instrumentation, like the `DC`/`DB` calls it pairs with.
        "DebugId" => {}
        _ => {
            let args = match value {
                Some(value) => vec![Arg::Int { value }],
                None => vec![Arg::Opaque],
            };
            result.settings.push(Setting {
                target: member.to_owned(),
                args,
            });
        }
    }
}

/// The FName literal resolved closest before `index`.
fn nearest_static_name(instructions: &[Instr], index: usize, refs: &RefResolver) -> Option<String> {
    let floor = index.saturating_sub(MAX_STATEMENT_INSTRUCTIONS);
    let mut position = index;
    while position > floor {
        position -= 1;
        let instruction = &instructions[position];
        if is_call(instruction) && call_symbol(instruction, refs) == Some(STATIC_NAME) {
            let id = instructions
                .get(position.wrapping_sub(1))
                .and_then(immediate)?;
            return refs.static_name(id).map(str::to_owned);
        }
    }
    None
}

/// Per-instruction control-flow facts used to guard steps.
struct Flow {
    /// Block index per instruction index.
    block_of: Vec<usize>,
    /// Whether each block runs on every execution of the function.
    unconditional: Vec<bool>,
    /// Names of predicate calls made by the branches deciding each block.
    hints: Vec<Vec<String>>,
}

impl Flow {
    fn guard(&self, instruction: usize) -> Guard {
        let Some(block) = self.block_of.get(instruction).copied() else {
            return Guard::unconditional();
        };
        if self.unconditional.get(block).copied().unwrap_or(true) {
            return Guard::unconditional();
        }
        Guard {
            conditional: true,
            hints: self.hints.get(block).cloned().unwrap_or_default(),
        }
    }
}

fn flow(instructions: &[Instr], refs: &RefResolver) -> Flow {
    let graph = cfg::build(instructions);
    let count = graph.blocks.len();
    let mut block_of = vec![0usize; instructions.len()];
    let mut index_of_start = HashMap::new();
    for (block, entry) in graph.blocks.iter().enumerate() {
        index_of_start.insert(entry.start_dw, block);
        for instruction in entry.instr_lo..entry.instr_hi.min(instructions.len()) {
            block_of[instruction] = block;
        }
    }

    let successors: Vec<Vec<usize>> = graph
        .blocks
        .iter()
        .map(|entry| {
            entry
                .succs
                .iter()
                .filter_map(|start| index_of_start.get(start).copied())
                .collect()
        })
        .collect();

    // A block runs on every execution when it lies on every path from entry to a return, which
    // is exactly post-dominance of the entry block. Anything else is guarded by a branch.
    let unconditional = post_dominators_of_entry(&successors, count);

    let mut hints = vec![Vec::new(); count];
    for (block, entry) in graph.blocks.iter().enumerate() {
        if successors[block].len() < 2 {
            continue;
        }
        let names = predicate_names(instructions, entry.instr_lo, entry.instr_hi, refs);
        if names.is_empty() {
            continue;
        }
        for reached in reachable(&successors, block, count) {
            if reached == block || unconditional[reached] {
                continue;
            }
            for name in &names {
                if !hints[reached].contains(name) {
                    hints[reached].push(name.clone());
                }
            }
        }
    }

    Flow {
        block_of,
        unconditional,
        hints,
    }
}

/// Blocks that lie on every path from the entry block to a return.
fn post_dominators_of_entry(successors: &[Vec<usize>], count: usize) -> Vec<bool> {
    if count == 0 {
        return Vec::new();
    }
    let exits: Vec<usize> = (0..count)
        .filter(|block| successors[*block].is_empty())
        .collect();
    if exits.is_empty() {
        return vec![true; count];
    }

    let mut predecessors = vec![Vec::new(); count];
    for (block, targets) in successors.iter().enumerate() {
        for target in targets {
            predecessors[*target].push(block);
        }
    }

    // Iterative dataflow: a block post-dominates the entry when every path onward from the
    // entry meets it. Compute the set of blocks each block must pass through on the way out.
    let mut on_every_path: Vec<BTreeSet<usize>> = (0..count)
        .map(|block| {
            if exits.contains(&block) {
                BTreeSet::from([block])
            } else {
                (0..count).collect()
            }
        })
        .collect();

    let mut changed = true;
    let mut rounds = 0usize;
    while changed && rounds < count.saturating_mul(2) + 8 {
        changed = false;
        rounds += 1;
        for block in (0..count).rev() {
            if successors[block].is_empty() {
                continue;
            }
            let mut next: Option<BTreeSet<usize>> = None;
            for target in &successors[block] {
                next = Some(match next {
                    None => on_every_path[*target].clone(),
                    Some(current) => current
                        .intersection(&on_every_path[*target])
                        .copied()
                        .collect(),
                });
            }
            let mut next = next.unwrap_or_default();
            next.insert(block);
            if next != on_every_path[block] {
                on_every_path[block] = next;
                changed = true;
            }
        }
    }

    let entry = on_every_path[0].clone();
    (0..count).map(|block| entry.contains(&block)).collect()
}

fn reachable(successors: &[Vec<usize>], from: usize, count: usize) -> Vec<usize> {
    let mut seen = vec![false; count];
    let mut stack = vec![from];
    let mut order = Vec::new();
    while let Some(block) = stack.pop() {
        if seen[block] {
            continue;
        }
        seen[block] = true;
        order.push(block);
        for target in &successors[block] {
            if !seen[*target] {
                stack.push(*target);
            }
        }
    }
    order
}

/// Names of the calls a deciding block makes, minus instrumentation and plain accessors.
fn predicate_names(
    instructions: &[Instr],
    lo: usize,
    hi: usize,
    refs: &RefResolver,
) -> Vec<String> {
    let mut names = Vec::new();
    for (offset, instruction) in instructions
        .iter()
        .take(hi.min(instructions.len()))
        .skip(lo)
        .enumerate()
    {
        if !is_call(instruction) {
            continue;
        }
        let Some(name) = call_symbol(instruction, refs) else {
            continue;
        };
        if INSTRUMENTATION.contains(&name)
            || name == STATIC_NAME
            || is_compiler_internal(name)
            || name.starts_with("Get")
        {
            continue;
        }
        // Only a call the branch consumes can be part of the decision; a statement that happens
        // to sit in the same block is not a reason this step runs.
        let position = lo + offset;
        if !is_value_call(instructions, position) && !is_predicate_call(instructions, position) {
            continue;
        }
        if !names.iter().any(|existing: &String| existing == name) {
            names.push(name.to_owned());
        }
    }
    names
}

/// What reading one `Act_Implementation` produced.
pub(super) struct Act {
    pub steps: Vec<Step>,
    pub suppressed: usize,
    pub unresolved: usize,
    pub says_incomplete: usize,
}

/// Read the body of one `Act_Implementation`.
pub(super) fn act(instructions: &[Instr], refs: &RefResolver) -> Act {
    let control = flow(instructions, refs);
    let mut result = Act {
        steps: Vec::new(),
        suppressed: 0,
        unresolved: 0,
        says_incomplete: 0,
    };

    for index in 0..instructions.len() {
        let instruction = &instructions[index];
        if !is_call(instruction) {
            continue;
        }
        let Some(name) = call_symbol(instruction, refs) else {
            result.unresolved += 1;
            continue;
        };
        if INSTRUMENTATION.contains(&name)
            || name == STATIC_NAME
            || is_compiler_internal(name)
            || is_argument_constructor(instructions, index)
            || is_value_call(instructions, index)
            || is_predicate_call(instructions, index)
        {
            result.suppressed += 1;
            continue;
        }

        let name = name.to_owned();
        let start = statement_start(instructions, index);
        let guard = control.guard(index);
        let kind = match name.as_str() {
            "Say" | "SayViaGlobalVoice" => {
                let step = say(instructions, start, index, refs);
                if let StepKind::Say {
                    speaker, loc_key, ..
                } = &step
                {
                    // A globally voiced line has no character accessor to find, so only its
                    // missing text counts as something the extractor failed to read.
                    let wants_speaker = name == "Say";
                    if loc_key.is_none() || (wants_speaker && speaker.is_none()) {
                        result.says_incomplete += 1;
                    }
                }
                step
            }
            "Subdialog" => StepKind::Subdialog {
                children: arguments(instructions, start, index, refs)
                    .into_iter()
                    .filter_map(|arg| match arg {
                        Arg::Class { name } => Some(name),
                        _ => None,
                    })
                    .collect(),
            },
            "ReturnToLastSelection" => StepKind::ReturnToLastSelection,
            "EndConversation" => StepKind::EndConversation,
            _ => StepKind::Call {
                name,
                args: arguments(instructions, start, index, refs),
            },
        };
        result.steps.push(Step { guard, kind });
    }

    result
}

/// Read one `Say(<speaker>.GetAI(), LocText("KEY"), <expression>, ...)` statement.
fn say(instructions: &[Instr], start: usize, index: usize, refs: &RefResolver) -> StepKind {
    let mut loc_key = None;
    let mut expression = None;
    let mut speaker = None;

    for position in start..index {
        let instruction = &instructions[position];
        match instruction.op.name {
            "PGA" => {
                if let Some(pointer) = instruction.qwords.first() {
                    let pointer = *pointer as i64;
                    if refs.global_is_string(pointer) {
                        if let Some(text) = refs.global_by_ptr(pointer) {
                            loc_key = Some(text.to_owned());
                        }
                    }
                }
            }
            "PshGPtr" => {
                if let Some(pointer) = instruction.qwords.first() {
                    if let Some(name) = refs.global_by_ptr(*pointer as i64) {
                        if name.starts_with("Expression") {
                            expression = Some(name.to_owned());
                        }
                    }
                }
            }
            _ => {
                if is_call(instruction) {
                    if let Some(name) = call_symbol(instruction, refs) {
                        if let Some(resolved) = speaker_of(name, instructions, position, refs) {
                            speaker = Some(resolved);
                        }
                    }
                }
            }
        }
    }

    StepKind::Say {
        speaker,
        loc_key,
        expression,
    }
}

/// The character an accessor call yields: `GetViper` -> `Viper`, `GetCharacter(n"Hero")` -> `Hero`.
fn speaker_of(
    name: &str,
    instructions: &[Instr],
    index: usize,
    refs: &RefResolver,
) -> Option<String> {
    if name == "GetCharacter" {
        return nearest_static_name(instructions, index, refs);
    }
    let rest = name.strip_prefix("Get")?;
    if rest.is_empty() || rest == "AI" {
        return None;
    }
    Some(rest.to_owned())
}

/// Read one `IsVisible_Implementation` into the calls it makes.
pub(super) fn visibility(instructions: &[Instr], refs: &RefResolver) -> Vec<Check> {
    let control = flow(instructions, refs);
    let mut checks: Vec<Check> = Vec::new();

    for index in 0..instructions.len() {
        let instruction = &instructions[index];
        if let Some(field) = field_read(instruction, refs) {
            // A state flag read straight off the world or the topic is as much a condition as a
            // predicate call, and is the only thing some overrides look at.
            if !field.starts_with("field_0x") {
                push_check(
                    &mut checks,
                    Check {
                        source: CheckSource::Field,
                        name: field.to_owned(),
                        args: Vec::new(),
                        conditional: control.guard(index).conditional,
                    },
                );
            }
            continue;
        }
        if !is_call(instruction) {
            continue;
        }
        let Some(name) = call_symbol(instruction, refs) else {
            continue;
        };
        if INSTRUMENTATION.contains(&name)
            || name == STATIC_NAME
            || is_compiler_internal(name)
            || name.starts_with("Get")
            || name == "IsStoryDebuggerAttached"
            || is_argument_constructor(instructions, index)
        {
            continue;
        }
        let name = name.to_owned();
        let start = statement_start(instructions, index);
        let args = arguments(instructions, start, index, refs);
        let conditional = control.guard(index).conditional;
        push_check(
            &mut checks,
            Check {
                source: CheckSource::Call,
                name,
                args,
                conditional,
            },
        );
    }

    checks
}

/// Add a check unless the same one was already recorded. An override commonly repeats its test
/// once per branch arm, and listing it twice tells a reader nothing.
fn push_check(checks: &mut Vec<Check>, check: Check) {
    if checks.iter().any(|existing| {
        existing.source == check.source
            && existing.name == check.name
            && existing.args == check.args
    }) {
        return;
    }
    checks.push(check);
}

/// The state field a `LoadThisR`/`LoadRObjR`/`LoadVObjR` reads.
fn field_read<'a>(instruction: &Instr, refs: &'a RefResolver) -> Option<&'a str> {
    let (offset, type_id) = match instruction.op.name {
        "LoadThisR" => (
            *instruction.words.first()? as i32,
            *instruction.dwords.first()? as i32,
        ),
        "LoadRObjR" | "LoadVObjR" => (
            *instruction.words.get(1)? as i32,
            *instruction.dwords.first()? as i32,
        ),
        _ => return None,
    };
    refs.member(type_id, offset)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::isa::OPCODES;

    fn instruction(name: &str, words: &[u16], dwords: &[u32], qwords: &[u64]) -> Instr {
        Instr {
            offset_dw: 0,
            op: OPCODES
                .iter()
                .find(|opcode| opcode.name == name)
                .expect("known opcode"),
            words: words.to_vec(),
            dwords: dwords.to_vec(),
            qwords: qwords.to_vec(),
        }
    }

    #[test]
    fn argument_constructor_needs_the_same_slot_on_both_sides() {
        let instructions = vec![
            instruction("PSF", &[6], &[], &[]),
            instruction("CALL", &[], &[1], &[]),
            instruction("PSF", &[6], &[], &[]),
        ];
        assert!(is_argument_constructor(&instructions, 1));

        let instructions = vec![
            instruction("PSF", &[22], &[], &[]),
            instruction("CALL", &[], &[1], &[]),
            instruction("PSF", &[12], &[], &[]),
        ];
        assert!(!is_argument_constructor(&instructions, 1));
    }

    #[test]
    fn a_stored_call_result_is_a_value_not_a_statement() {
        let instructions = vec![
            instruction("CALLSYS", &[], &[], &[0x10]),
            instruction("STOREOBJ", &[8], &[], &[]),
        ];
        assert!(is_value_call(&instructions, 0));

        let instructions = vec![
            instruction("CALLSYS", &[], &[], &[0x10]),
            instruction("SetV1", &[1], &[3], &[]),
        ];
        assert!(!is_value_call(&instructions, 0));
    }

    #[test]
    fn a_member_store_ends_the_statement() {
        assert!(ends_statement(&instruction("WRTV4", &[7], &[], &[])));
        assert!(ends_statement(&instruction("RET", &[2], &[], &[])));
        assert!(!ends_statement(&instruction("PshNull", &[], &[], &[])));
    }

    #[test]
    fn speakers_drop_their_accessor_prefix() {
        let refs = RefResolver::default();
        assert_eq!(
            speaker_of("GetViper", &[], 0, &refs),
            Some("Viper".to_owned())
        );
        assert_eq!(speaker_of("GetAI", &[], 0, &refs), None);
        assert_eq!(speaker_of("Say", &[], 0, &refs), None);
    }

    #[test]
    fn rules_map_onto_the_closed_vocabulary() {
        assert_eq!(rule_kind("HideIfKnows"), RuleKind::HideIfKnows);
        assert_eq!(
            rule_kind("Whatever"),
            RuleKind::Other {
                name: "Whatever".to_owned()
            }
        );
    }

    #[test]
    fn a_block_behind_a_branch_is_guarded() {
        // entry -> {a, join}, a -> join, join -> exit.
        let successors = vec![vec![1, 2], vec![2], vec![]];
        let unconditional = post_dominators_of_entry(&successors, 3);
        assert!(unconditional[0], "the entry always runs");
        assert!(!unconditional[1], "the branch arm does not");
        assert!(unconditional[2], "the join does");
    }
}
