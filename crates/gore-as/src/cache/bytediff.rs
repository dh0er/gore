//! SEMANTIC-CORRECTNESS ORACLE — per-function bytecode byte-faithfulness between a VANILLA
//! cache and a REGEN (re-compilation of our decompiled source). Implements `gore as bytediff`
//! per `work/reversing/gore-as/specs/semantic-oracle.md`.
//!
//! Thesis: the AS compiler emits identical bytecode for identical source on the same engine
//! build, MODULO a handful of build-non-determinism sources — ref/type-id keys (runtime
//! pointers), jump absolutes (shift with instruction size), and constant raw encodings. This
//! tool normalizes exactly those (N1/N3/N4/N7, plus opt-in fail-closed slot-allocation proofs N2) and classifies each
//! aligned function IDENTICAL / BENIGN-DIFF / SEMANTIC-DIFF. A residual diff after normalization
//! is a difference the compiler was FORCED to make by different SOURCE — a real behavior change.
//!
//! Governing safety rule (`spec §3`): a false BENIGN hides a real bug (catastrophic); a false
//! SEMANTIC only wastes fix effort (cheap). Every normalizer is provably behavior-preserving; when
//! in doubt, leave the diff SEMANTIC.

use std::collections::{BTreeSet, HashMap, HashSet};

use super::cfg;
use super::disasm::{disassemble, Instr};
use super::isa::{BcType, OPCODES};
use super::refs::RefResolver;
use super::remap::{
    ref_sites, split_type_id_operand, valid_type_id_core, OperandId, RefIdentity, RefKind,
    TYPE_ID_OBJECT_MASK, TYPE_ID_QUALIFIER_MASK,
};

// =================================================================================================
// Operand role classification: split an instruction's positional words/dwords/qwords into roles
// (slot / immediate-const / jump-target / ref) so each normalizer touches only its own operands.
// =================================================================================================

/// The 9 conditional/unconditional relative jumps whose single DW operand is a byte(dword) offset
/// into the function — canonicalized to a target INSTRUCTION INDEX by N3.
fn is_jump_op(name: &str) -> bool {
    matches!(
        name,
        "JMP" | "JZ" | "JNZ" | "JS" | "JNS" | "JP" | "JNP" | "JLowZ" | "JLowNZ"
    )
}

/// Read a qword (2 dwords LE) directly from a decoded instruction's collected qword operands.
/// (The disassembler already collected the QW into `ins.qwords`; the first one is the ref ptr.)
fn instr_first_qword(ins: &Instr) -> Option<u64> {
    ins.qwords.first().copied()
}

/// Bare callee NAME of a call instruction, resolved via the side's `RefResolver`. `CALLSYS`
/// carries a func-PTR (QW @ dword 1); `CALL`/`CALLBND`/`CALLINTF` carry a func-ID (DW @ dword 1).
/// Returns `None` for a non-call op or an unresolved callee. Used by the batch-38 lookahead gates
/// (GAP-B `__STATIC_NAME`, GAP-C `opCast`) to identify a specific native callee WITHOUT string-
/// parsing the composed identity.
fn callee_name<'a>(ins: &Instr, side: &'a Side) -> Option<&'a str> {
    match ins.op.name {
        "CALLSYS" | "FuncPtr" | "Thiscall1" => {
            side.refs.func_by_ptr(instr_first_qword(ins)? as i64)
        }
        "CALL" | "CALLBND" | "CALLINTF" => side.refs.func_by_id(*ins.dwords.first()? as i32),
        _ => None,
    }
}

/// GAP-B gate: the instruction AFTER `pos` is a `CALLSYS` whose callee is the synthesized
/// `__STATIC_NAME(int)` FName-pool accessor — i.e. the `PshC4` at `pos` pushed a StaticNames
/// pool INDEX, not an integer value.
fn next_is_static_name(instrs: &[Instr], pos: usize, side: &Side) -> bool {
    instrs
        .get(pos + 1)
        .and_then(|nx| callee_name(nx, side))
        .is_some_and(|n| n == "__STATIC_NAME")
}

/// GAP-C gate: the `TYPEID`/`Cast` at `pos` feeds an `opCast`/`Cast` call. Scan a SHORT forward
/// window; the pushed type-id is consumed by the next call, which must be `opCast` (the pattern is
/// `TYPEID; PSF; PshVPtr; CALLSYS opCast`). Stop at the first call op — if it is NOT `opCast`, the
/// type-id feeds something else and the gate does NOT fire (stays a value-compared Primitive).
fn feeds_matching_opcast(instrs: &[Instr], pos: usize, side: &Side) -> bool {
    // Window kept tight (the opCast is 1–3 ops after the TYPEID in every observed case).
    const WINDOW: usize = 4;
    for k in 1..=WINDOW {
        let Some(nx) = instrs.get(pos + k) else {
            return false;
        };
        let is_call = matches!(nx.op.name, "CALLSYS" | "CALL" | "CALLBND" | "CALLINTF");
        if is_call {
            // The FIRST call after the TYPEID must be the opCast that consumes it.
            return callee_name(nx, side) == Some("opCast");
        }
    }
    false
}

/// A single normalized operand token. Equality of two `Operand`s (across caches) means the
/// operands are semantically the same after the relevant normalizer.
#[derive(Debug, Clone, PartialEq)]
enum Operand {
    /// A frame slot (word). Raw slot number, or a canonical ordinal under N2.
    Slot(i32),
    /// A plain word arg that is neither a slot nor a ref (e.g. RET pop-size, GETOBJ offset).
    Word(u16),
    /// An integer immediate compared by decoded value (N4). `width` disambiguates 1/2/4/8-byte.
    IntConst {
        value: i64,
        width: u8,
    },
    /// A float immediate compared bit-exactly by decoded f64 (N4, floatIsFloat64 build).
    FloatConst(u64),
    /// A jump target as an instruction index into the (normalized) list (N3). `None` = a target
    /// that has no counterpart instruction (dropped/added op) — always a SEMANTIC signal.
    JumpIndex(Option<usize>),
    /// A ref operand resolved to a portable identity (N1).
    Ref(OperandId),
    /// A bytecode type-id resolved through this cache's T2 -> complete T1 identity while retaining
    /// the operand-local handle/const qualifiers and the core id's object-kind bits. The latter
    /// matter because one T1 declaration may have several T2 aliases with different runtime
    /// object semantics.
    TypeId(TypeIdIdentity),
    /// The coupled member operands of `ADDSi`, `LoadThisR`, `LoadRObjR`, or `LoadVObjR`, resolved
    /// as one property declaration:
    /// complete owner T1 identity + T7 name + complete T1 identity of T7 `OldTypeId`.
    /// A site is emitted in this form only when every link resolves; otherwise both wire operands
    /// remain raw so an offset/id drift cannot be hidden.
    Property(PropertyIdentity),
    /// A resolved STR / __STATIC_NAME string literal, compared by text (N4).
    StaticName(Option<String>),
    /// A `TYPEID`/`Cast` operand that is a LARGE runtime object type-id feeding an `opCast`/`Cast`
    /// whose callee identity matches on both sides (GAP-C, batch-38). The raw id is a
    /// build-specific `asCTypeInfo` id that drifts across recompiles; the cast target is pinned by
    /// the adjacent (matched) opCast signature, so two such operands compare EQUAL regardless of
    /// the raw id. Only produced after the runtime-object-typeid + feeds-matching-opCast gate;
    /// a genuine primitive type-id stays a value-compared `Ref(Primitive)`.
    OpCastTypeId,
    /// N7: a LARGE runtime object type-id resolved, through the side's OWN type table, to the
    /// type it names. The numeric `asCTypeInfo` id is assigned as the engine registers types and
    /// drifts whenever the set or order of registrations changes — the same build noise N1
    /// normalizes away for reference keys. What the operand MEANS is the type, so two such
    /// operands compare equal when they resolve to the same type identity, and the handle and
    /// const-handle flag bits travel with the token so a handle is never equated with a value.
    /// Fail-closed: an id either side cannot resolve stays a value-compared `Ref(Primitive)`.
    TypeIdentity(String),
    /// A raw dword/qword the classifier does not model as slot/const/ref/jump — compared verbatim
    /// (conservative: an unmodeled operand difference stays SEMANTIC).
    RawDw(u32),
    RawQw(u64),
}

#[derive(Debug, Clone, PartialEq)]
struct TypeIdIdentity {
    identity: OperandId,
    qualifiers: u32,
    object_kind: u32,
}

#[derive(Debug, Clone, PartialEq)]
struct PropertyIdentity {
    owner: TypeIdIdentity,
    name: String,
    old_type: TypeIdIdentity,
}

const TYPE_ID_OBJHANDLE: u32 = 0x4000_0000;
const TYPE_ID_HANDLETOCONST: u32 = 0x2000_0000;

/// One instruction after normalization: opcode name + ordered operand tokens.
#[derive(Debug, Clone)]
struct NormInstr {
    op: &'static str,
    operands: Vec<Operand>,
}

impl NormInstr {
    /// Structural equality: same opcode AND same normalized operands.
    fn norm_eq(&self, other: &NormInstr) -> bool {
        self.op == other.op && self.operands == other.operands
    }
}

/// Which normalizers were requested (spec `--normalize`; N2 off by default).
#[derive(Debug, Clone, Copy)]
pub struct NormOpts {
    pub n1_refs: bool,
    pub n2_slots: bool,
    pub n3_jumps: bool,
    pub n4_consts: bool,
    /// N5 (`n5_scope`): exact-frame strip of `FScopeCycleCounter` profiler scopes and matching
    /// `FStatID` temp dtors on either side — pure CPU-timing instrumentation, behavior-neutral
    /// (`specs/final-residue.md §B.2`). Default ON.
    pub n5_scope: bool,
    /// N6 (`n6_reguard`): one-sided vanilla fold of a short-circuit boolean-cascade re-guard that
    /// is PROVABLY DOMINATED by an identical earlier same-slot null-guard with no intervening write
    /// (`specs/final-residue.md §B.1`). Default ON.
    pub n6_reguard: bool,
}

impl Default for NormOpts {
    fn default() -> Self {
        // All ON except N2 (slot renumber) — the spec default. N5/N6 (benign-attribution strips)
        // are ON by default, mirroring N1/N3/N4.
        NormOpts {
            n1_refs: true,
            n2_slots: false,
            n3_jumps: true,
            n4_consts: true,
            n5_scope: true,
            n6_reguard: true,
        }
    }
}

/// Everything one side needs to normalize/resolve its functions.
pub struct Side {
    pub refs: RefResolver,
    pub ident: RefIdentity,
}

impl Side {
    pub fn build(bytes: &[u8]) -> anyhow::Result<Side> {
        let refs = RefResolver::build(bytes)?;
        let ident = RefIdentity::build(bytes)?;
        Ok(Side { refs, ident })
    }
}

/// N7: the type a runtime object type-id names, as a token that carries the handle and
/// const-handle flags alongside the resolved name. The id's index part is what the engine assigns
/// at registration time and what drifts; the flags are part of what the operand means.
///
/// `None` when this side's table cannot resolve the id, which leaves the operand compared by
/// value — the fail-closed default.
fn resolved_type_identity(id: &OperandId, side: &Side) -> Option<String> {
    const OBJHANDLE: u32 = 0x4000_0000;
    const HANDLETOCONST: u32 = 0x2000_0000;
    let OperandId::Primitive(raw) = id else {
        return None;
    };
    let raw = *raw as u32;
    for candidate in [raw, raw & !OBJHANDLE, raw & !(OBJHANDLE | HANDLETOCONST)] {
        if let Some(name) = side.refs.type_by_id_composed(candidate as i32) {
            return Some(type_identity_token(&name, raw));
        }
    }
    None
}

/// The N7 token: the resolved type name plus the handle and const-handle flags the id carries.
/// Those two bits are part of what the operand means — a handle is not the value — while the
/// index below them is what the engine assigns at registration time and what drifts.
fn type_identity_token(name: &str, raw: u32) -> String {
    const OBJHANDLE: u32 = 0x4000_0000;
    const HANDLETOCONST: u32 = 0x2000_0000;
    format!("{name}#{:#x}", raw & (OBJHANDLE | HANDLETOCONST))
}

/// Read a qword (2 dwords LE) from the bytecode at absolute dword offset.
fn read_qw(code: &[i32], dw: usize) -> u64 {
    let lo = code[dw] as u32 as u64;
    let hi = code[dw + 1] as u32 as u64;
    lo | (hi << 32)
}

/// Build the offset->instruction-index map for N3 jump canonicalization.
fn offset_index_map(instrs: &[Instr]) -> HashMap<usize, usize> {
    instrs
        .iter()
        .enumerate()
        .map(|(i, ins)| (ins.offset_dw, i))
        .collect()
}

/// Normalize one function's disassembled instructions into `NormInstr`s.
///
/// This applies N1 (refs->identity), N3 (jumps->index), N4 (consts by value; STR/__STATIC_NAME
/// by string). N2 (slot renumber) is applied afterwards on the whole list if enabled.
fn normalize(code: &[i32], instrs: &[Instr], side: &Side, opts: &NormOpts) -> Vec<NormInstr> {
    let off_to_idx = offset_index_map(instrs);
    let mut out = Vec::with_capacity(instrs.len());

    for (pos, ins) in instrs.iter().enumerate() {
        let name = ins.op.name;
        // Ref-operand dword indices (relative to the instruction start) → identity, via the
        // SHARED ref_sites classification (N1). Collect into a per-dword-index lookup so the
        // positional walk below can substitute them.
        let mut ref_at_dw: HashMap<usize, Operand> = HashMap::new();
        if opts.n1_refs {
            for site in ref_sites(name) {
                let base = ins.offset_dw + site.dw_index;
                if base >= code.len() {
                    continue;
                }
                let operand = match site.kind {
                    RefKind::GlobalPtr | RefKind::FuncPtr | RefKind::TypePtr => {
                        if base + 1 >= code.len() {
                            continue;
                        }
                        Operand::Ref(
                            side.ident
                                .resolve_ptr(site.kind, read_qw(code, base) as i64),
                        )
                    }
                    RefKind::FuncId => Operand::Ref(side.ident.resolve_id(site.kind, code[base])),
                    RefKind::TypeId => normalize_type_id_operand(code[base], &side.ident),
                };
                ref_at_dw.insert(site.dw_index, operand);
            }
        }

        let operands =
            normalize_operands(name, ins, &ref_at_dw, &off_to_idx, side, opts, instrs, pos);
        out.push(NormInstr { op: name, operands });
    }

    if opts.n2_slots {
        let _ = renumber_slots(&mut out);
    }
    out
}

/// Turn one instruction's positional words/dwords/qwords into role-tagged `Operand` tokens.
///
/// Roles are decided by (opcode, BcType). The positional layout mirrors `disasm::disassemble`:
/// words in source order, then dwords, then qwords. We map each positional operand to the dword
/// index it occupies so N1 ref substitutions (keyed by dword index) apply.
#[allow(clippy::too_many_arguments)]
fn normalize_operands(
    name: &str,
    ins: &Instr,
    ref_at_dw: &HashMap<usize, Operand>,
    off_to_idx: &HashMap<usize, usize>,
    side: &Side,
    opts: &NormOpts,
    instrs: &[Instr],
    pos: usize,
) -> Vec<Operand> {
    let mut out = Vec::new();

    // The offset and owner type-id jointly address T7. Normalizing either operand in isolation is
    // unsafe: equal offsets may name different fields on different owners, while offset drift is
    // benign only when both caches resolve the complete same declaration. Resolve atomically, or
    // keep the entire pair raw (fail closed).
    if opts.n1_refs && matches!(name, "ADDSi" | "LoadThisR" | "LoadRObjR" | "LoadVObjR") {
        return match resolve_property_operand(ins, side) {
            Some(property) => resolved_property_operands(name, ins, property),
            None => raw_property_operands(name, ins),
        };
    }

    // --- Word operands (16-bit): occupy dword 0 (hi/lo) or dword 1 hi depending on BcType. ---
    // For N1, ref operands are never in the word slots (all ref sites are dword/qword), so words
    // are only Slot/Word roles. Classify slots vs plain words per BcType.
    push_word_operands(name, ins, &mut out);

    // --- Dword operands: jump target (N3), ref id (N1), int const (N4), static-name idx (N4),
    //     or a plain raw dword. The dword operand's absolute dword index tells us which. ---
    // Determine the dword-index of each positional dword operand from the BcType.
    let dword_indices = dword_operand_indices(ins.op.fmt);
    for (dop, &dw_idx) in dword_indices.iter().enumerate() {
        let raw = *ins.dwords.get(dop).unwrap_or(&0);
        // N1 ref (func-id / type-id) at this dword index?
        if let Some(operand) = ref_at_dw.get(&dw_idx) {
            // GAP-C (batch-38): a `TYPEID`/`Cast` operand that is a LARGE runtime object type-id
            // (an `asCTypeInfo` id not in T2, mask bits set) is build-specific and drifts. When it
            // feeds an `opCast`/`Cast` whose callee identity matches on both sides (verified at the
            // opCast op's own index), the cast TARGET is pinned by that signature, so collapse the
            // drifting id to a single canonical token. A genuine primitive type-id stays a
            // value-compared Ref(Primitive).
            if (matches!(operand, Operand::Ref(id) if id.is_runtime_object_typeid())
                || matches!(operand, Operand::RawDw(_) if is_unresolved_runtime_type_id(raw as i32, &side.ident)))
                && feeds_matching_opcast(instrs, pos, side)
            {
                out.push(Operand::OpCastTypeId);
                continue;
            }
            // N7: any other runtime object type-id resolves to the type it names.
            if let Operand::Ref(id) = operand {
                if id.is_runtime_object_typeid() {
                    if let Some(identity) = resolved_type_identity(id, side) {
                        out.push(Operand::TypeIdentity(identity));
                        continue;
                    }
                }
            }
            out.push(operand.clone());
            continue;
        }
        // N3 jump target?
        if opts.n3_jumps && is_jump_op(name) {
            // Target byte(dword) offset is relative to the END of the jump instruction.
            let target_off =
                (ins.offset_dw as i64 + ins.op.size_dwords as i64 + raw as i32 as i64) as usize;
            out.push(Operand::JumpIndex(off_to_idx.get(&target_off).copied()));
            continue;
        }
        // GAP-B (batch-38): a `PshC4 <idx>` immediately followed by `CALLSYS __STATIC_NAME` is an
        // FName-literal pool index, NOT an integer value. The StaticNames pool is rebuilt per-cache
        // (different size), so the same name lands at a different slot — resolve the index to TEXT
        // and compare by string (mirror the `STR` handling below). The tight next-instruction gate
        // keeps a real integer literal (not feeding __STATIC_NAME) comparing by value.
        if opts.n4_consts && name == "PshC4" && next_is_static_name(instrs, pos, side) {
            let s = side
                .refs
                .static_name(raw as i32 as i64)
                .map(|s| s.to_string());
            out.push(Operand::StaticName(s));
            continue;
        }
        // N4 constant?
        match const_dword_role(name, dop) {
            DwordRole::Int(width) => {
                out.push(Operand::IntConst {
                    value: raw as i32 as i64,
                    width,
                });
            }
            DwordRole::None => out.push(Operand::RawDw(raw)),
        }
    }

    // --- Qword operands: ref ptr (N1), 64-bit const (N4 float/int), or raw. ---
    let qword_indices = qword_operand_indices(ins.op.fmt);
    for (qop, &dw_idx) in qword_indices.iter().enumerate() {
        let raw = *ins.qwords.get(qop).unwrap_or(&0);
        if let Some(operand) = ref_at_dw.get(&dw_idx) {
            out.push(operand.clone());
            continue;
        }
        match const_qword_role(name) {
            QwordRole::Float => out.push(Operand::FloatConst(raw)),
            QwordRole::None => out.push(Operand::RawQw(raw)),
        }
    }

    // STR (opcode 60) carries a W index into the string/name pool — resolve to text (N4).
    if name == "STR" {
        // The W arg was pushed as a word above (as Word); replace it with a resolved StaticName.
        // STR's W is the pool index. Recompute here and swap the trailing Word.
        if let Some(&w) = ins.words.first() {
            let s = side.refs.static_name(w as i64).map(|s| s.to_string());
            // Replace the first Word operand we pushed with the resolved name.
            if let Some(first) = out.first_mut() {
                *first = Operand::StaticName(s);
            }
        }
    }

    let _ = opts;
    out
}

/// Resolve a raw bytecode type-id without losing operand-local semantics.
///
/// T2 is keyed by the core id only. Handle/const qualifiers therefore have to be split before
/// lookup and retained beside the portable T1 identity. Unknown flag bits, impossible qualified
/// non-object ids, invalid object-kind combinations, and missing T2/T1 links remain raw.
fn normalize_type_id_operand(raw: i32, ident: &RefIdentity) -> Operand {
    if let Some(identity) = resolve_type_id_identity(raw, ident) {
        return Operand::TypeId(identity);
    }
    Operand::RawDw(raw as u32)
}

/// The narrow GAP-C exception for a runtime object id missing from T2. The operand remains raw
/// everywhere else; only an adjacent resolved `opCast` signature may pin and collapse it.
fn is_unresolved_runtime_type_id(raw: i32, ident: &RefIdentity) -> bool {
    let (core, flags) = split_type_id_operand(raw);
    let object_kind = core as u32 & TYPE_ID_OBJECT_MASK;
    flags & !TYPE_ID_QUALIFIER_MASK == 0
        && !(flags & TYPE_ID_HANDLETOCONST != 0 && flags & TYPE_ID_OBJHANDLE == 0)
        && valid_type_id_core(core)
        && object_kind != 0
        && matches!(
            ident.resolve_id(RefKind::TypeId, core),
            OperandId::Primitive(_)
        )
}

/// Resolve the portable semantic identity of a type-id operand.
///
/// Named/object/enum ids use complete T1 identity and retain their valid qualifier/object-kind
/// bits. Primitive ids have no T1 row; their fixed core value is itself the portable identity and
/// qualifiers/object-kind are canonically zero. Anything else is unresolved or invalid.
fn resolve_type_id_identity(raw: i32, ident: &RefIdentity) -> Option<TypeIdIdentity> {
    let (core, flags) = split_type_id_operand(raw);
    let object_kind = core as u32 & TYPE_ID_OBJECT_MASK;
    if flags & !TYPE_ID_QUALIFIER_MASK != 0
        || (flags & TYPE_ID_HANDLETOCONST != 0 && flags & TYPE_ID_OBJHANDLE == 0)
        || (flags != 0 && (!valid_type_id_core(core) || object_kind == 0))
    {
        return None;
    }

    match ident.resolve_id(RefKind::TypeId, core) {
        identity @ OperandId::Named { .. } if valid_type_id_core(core) => Some(TypeIdIdentity {
            identity,
            qualifiers: flags,
            object_kind,
        }),
        identity @ OperandId::Primitive(_) if flags == 0 && object_kind == 0 => {
            Some(TypeIdIdentity {
                identity,
                qualifiers: 0,
                object_kind: 0,
            })
        }
        OperandId::Named { .. }
        | OperandId::Primitive(_)
        | OperandId::RawPtr(_)
        | OperandId::RawId(_) => None,
    }
}

fn resolve_property_operand(ins: &Instr, side: &Side) -> Option<PropertyIdentity> {
    let raw_owner = *ins.dwords.first()? as i32;
    let member_offset = i32::from(*ins.words.last()? as i16);
    let (owner_core, owner_flags) = split_type_id_operand(raw_owner);
    if owner_flags & !TYPE_ID_QUALIFIER_MASK != 0 {
        return None;
    }
    let owner = match normalize_type_id_operand(raw_owner, &side.ident) {
        Operand::TypeId(identity) if matches!(&identity.identity, OperandId::Named { .. }) => {
            identity
        }
        _ => return None,
    };
    let (name, raw_old_type) = side.refs.member_identity(owner_core, member_offset)?;
    let old_type = resolve_type_id_identity(raw_old_type, &side.ident)?;
    Some(PropertyIdentity {
        owner,
        name: name.to_owned(),
        old_type,
    })
}

fn resolved_property_operands(name: &str, ins: &Instr, property: PropertyIdentity) -> Vec<Operand> {
    let mut resolved = Vec::with_capacity(2);
    if matches!(name, "LoadRObjR" | "LoadVObjR") {
        if let Some(&slot) = ins.words.first() {
            resolved.push(Operand::Slot(i32::from(slot as i16)));
        }
    }
    resolved.push(Operand::Property(property));
    resolved
}

fn raw_property_operands(name: &str, ins: &Instr) -> Vec<Operand> {
    let mut raw = Vec::with_capacity(3);
    if matches!(name, "LoadRObjR" | "LoadVObjR") {
        if let Some(&slot) = ins.words.first() {
            raw.push(Operand::Slot(i32::from(slot as i16)));
        }
    }
    if let Some(&offset) = ins.words.last() {
        raw.push(Operand::Word(offset));
    }
    if let Some(&owner) = ins.dwords.first() {
        raw.push(Operand::RawDw(owner));
    }
    raw
}

/// Push the word (16-bit) operands of an instruction as Slot/Word tokens per BcType.
///
/// `rW`/`wW` word slots are frame slots (Slot); a bare `W` arg is a plain word (Word). The
/// per-BcType word roles mirror `disasm`'s word-collection order.
fn push_word_operands(name: &str, ins: &Instr, out: &mut Vec<Operand>) {
    use BcType::*;
    // (is_slot?) per word position, in the order `disasm` collected them.
    let roles: &[bool] = match ins.op.fmt {
        W_ARG => &[false], // plain word (e.g. RET size, ChkNullS var-is-actually-slot)
        wW_ARG | rW_ARG => &[true], // one slot
        wW_rW_ARG | rW_rW_ARG => &[true, true],
        wW_W_ARG => &[true, false],
        W_rW_ARG => &[false, true],
        wW_rW_rW_ARG => &[true, true, true],
        rW_DW_ARG | wW_DW_ARG | rW_QW_ARG | wW_QW_ARG => &[true], // leading slot, then DW/QW
        W_DW_ARG => &[false], // leading plain word (ADDSi/LoadThisR: word=offset), then DW
        wW_rW_DW_ARG => &[true, true],
        rW_W_DW_ARG => &[true, false],
        rW_DW_DW_ARG => &[true],
        // no word operands:
        NO_ARG | INFO | DW_ARG | QW_ARG | DW_DW_ARG | QW_DW_ARG => &[],
    };
    // ChkNullS (W_ARG) actually addresses a slot; RET/ThrowException W is a plain size/int.
    // Keep W_ARG conservative as Word EXCEPT the few W_ARG ops whose W is a slot.
    let w_is_slot = matches!(name, "ChkNullS");
    for (i, &is_slot) in roles.iter().enumerate() {
        let w = *ins.words.get(i).unwrap_or(&0);
        let slot = is_slot || (roles.len() == 1 && i == 0 && w_is_slot && ins.op.fmt == W_ARG);
        if slot {
            out.push(Operand::Slot(w as i16 as i32));
        } else {
            out.push(Operand::Word(w));
        }
    }
}

/// Absolute dword index (within the instruction) of each positional DW operand, per BcType.
fn dword_operand_indices(fmt: BcType) -> Vec<usize> {
    use BcType::*;
    match fmt {
        DW_ARG => vec![1],
        rW_DW_ARG | wW_DW_ARG | W_DW_ARG => vec![1],
        DW_DW_ARG => vec![1, 2],
        wW_rW_DW_ARG | rW_W_DW_ARG => vec![2],
        rW_DW_DW_ARG => vec![1, 2],
        QW_DW_ARG => vec![3],
        _ => vec![],
    }
}

/// Absolute dword index of each positional QW operand, per BcType.
fn qword_operand_indices(fmt: BcType) -> Vec<usize> {
    use BcType::*;
    match fmt {
        QW_ARG | wW_QW_ARG | rW_QW_ARG | QW_DW_ARG => vec![1],
        _ => vec![],
    }
}

enum DwordRole {
    Int(u8),
    None,
}

/// Role of a positional DW operand for N4 (constants). `pos` is the index among the instruction's
/// DW operands. Only ops whose DW is a genuine literal/value are Int; jump/ref DWs are handled
/// upstream so they never reach here.
fn const_dword_role(name: &str, pos: usize) -> DwordRole {
    match name {
        // 32-bit integer/float immediates pushed/set as values.
        "PshC4" | "SetV4" | "SetV1" | "SetV2" => DwordRole::Int(4),
        // SetG4: DW @ dword 3 is the literal value (the QW @1 is the global ptr, handled as ref).
        "SetG4" => DwordRole::Int(4),
        // immediate-arithmetic / immediate-compare literals.
        "ADDIi" | "SUBIi" | "MULIi" | "ADDIf" | "SUBIf" | "MULIf" => DwordRole::Int(4),
        "CMPIi" | "CMPIf" | "CMPIu" => DwordRole::Int(4),
        // SetListSize (rW_DW_DW): both DWs are sizes (dword 1 = offset, dword 2 = size) — values.
        "SetListSize" => DwordRole::Int(4),
        // PshListElmnt (rW_DW): DW is an offset into the init list — a value.
        "PshListElmnt" => DwordRole::Int(4),
        // AllocMem (wW_DW): DW is a byte size — a value.
        "AllocMem" => DwordRole::Int(4),
        // GETOBJ/GETOBJREF/GETREF (W_rW): no DW operand; not reached.
        _ => {
            let _ = pos;
            DwordRole::None
        }
    }
}

enum QwordRole {
    Float,
    None,
}

/// Role of the QW operand for N4. PshC8/SetV8 carry a 64-bit constant. This build is
/// floatIsFloat64, so a `PshC8`/`SetV8` used for a `float`/`double` literal is a float bit
/// pattern; but the same op also carries int64. We compare PshC8/SetV8 bit-exactly regardless
/// (a float64 and an int64 with the same bits are indistinguishable at the operand level and
/// compare identically either way), tagging them Float so NaN-payload bits are preserved.
fn const_qword_role(name: &str) -> QwordRole {
    match name {
        "PshC8" | "SetV8" => QwordRole::Float,
        _ => QwordRole::None,
    }
}

/// N2 — first-use slot renumbering (opt-in). Walk the stream in order; assign each distinct raw
/// slot a canonical ordinal on first appearance; rewrite `Slot` operands to their ordinal.
/// Behavior-preserving ONLY when both sides share the same slot SHAPE — the caller GUARDS on
/// distinct-slot-count equality before trusting an N2 BENIGN (see `classify`).
fn renumber_slots(instrs: &mut [NormInstr]) -> bool {
    let mut map: HashMap<i32, i32> = HashMap::new();
    let mut next = 0i32;
    let mut changed = false;
    for ni in instrs.iter_mut() {
        for op in ni.operands.iter_mut() {
            if let Operand::Slot(s) = op {
                let raw = *s;
                let canon = *map.entry(*s).or_insert_with(|| {
                    let v = next;
                    next += 1;
                    v
                });
                changed |= raw != canon;
                *op = Operand::Slot(canon);
            }
        }
    }
    changed
}

/// Distinct raw slot count in a normalized instruction list (for the N2 guard).
fn distinct_slot_count(instrs: &[NormInstr]) -> usize {
    let mut set = std::collections::HashSet::new();
    for ni in instrs {
        for op in &ni.operands {
            if let Operand::Slot(s) = op {
                set.insert(*s);
            }
        }
    }
    set.len()
}

/// Explicit frame-slot reads/writes for one decoded instruction. The AngelScript bytecode
/// formats encode these roles directly (`rW`/`wW`), so this is exhaustive over the ISA table and
/// does not rely on a mnemonic allowlist. Taking a slot address is a read/escape; `ChkNullS` is the
/// sole bare-`W` instruction whose operand is actually a frame slot. A small explicit set of VM
/// read-modify-write instructions is upgraded after the format-role pass because the ISA format
/// records only how their operand is encoded, not that the old slot value is consumed.
fn slot_accesses(ins: &Instr) -> (HashSet<i32>, HashSet<i32>) {
    use BcType::*;

    let mut reads = HashSet::new();
    let mut writes = HashSet::new();
    let slot = |index: usize| ins.words.get(index).map(|w| *w as i16 as i32);
    let mut apply = |index: usize, read: bool, write: bool| {
        if let Some(s) = slot(index) {
            if read {
                reads.insert(s);
            }
            if write {
                writes.insert(s);
            }
        }
    };

    match ins.op.fmt {
        wW_ARG | wW_DW_ARG | wW_QW_ARG => apply(0, false, true),
        rW_ARG | rW_DW_ARG | rW_QW_ARG | rW_DW_DW_ARG => apply(0, true, false),
        wW_rW_ARG | wW_rW_DW_ARG => {
            apply(0, false, true);
            apply(1, true, false);
        }
        rW_rW_ARG => {
            apply(0, true, false);
            apply(1, true, false);
        }
        wW_rW_rW_ARG => {
            apply(0, false, true);
            apply(1, true, false);
            apply(2, true, false);
        }
        wW_W_ARG => apply(0, false, true),
        W_rW_ARG => apply(1, true, false),
        rW_W_DW_ARG => apply(0, true, false),
        W_ARG if ins.op.name == "ChkNullS" => apply(0, true, false),
        INFO | NO_ARG | W_ARG | DW_ARG | QW_ARG | DW_DW_ARG | QW_DW_ARG | W_DW_ARG => {}
    }
    // Opcode-table rW/wW roles describe operand encoding, not every read-modify-write effect.
    // These VM instructions consume the old slot value and replace/clear it in-place.
    if matches!(
        ins.op.name,
        "NOT"
            | "NEGi"
            | "NEGf"
            | "NEGd"
            | "IncVi"
            | "DecVi"
            | "BNOT"
            | "NEGi64"
            | "BNOT64"
            | "FREE"
            | "LOADOBJ"
            | "RefCpyV"
            | "FreeNullV8"
    ) {
        apply(0, true, true);
    }
    (reads, writes)
}

#[derive(Debug)]
struct SlotLiveness {
    live_out: Vec<HashSet<i32>>,
    /// Explicit branch/dispatch destinations. Ordinary sequential fallthrough is not included.
    branch_targets: HashSet<usize>,
}

/// Build a fail-closed instruction CFG and solve frame-slot liveness. `cfg::build` deliberately
/// tolerates malformed edges for decompiler recovery; an equality oracle cannot. Therefore every
/// ordinary jump target and every `JMPP` dispatch row is validated before its graph is accepted.
fn slot_liveness(instrs: &[Instr]) -> Option<SlotLiveness> {
    if instrs.is_empty() {
        return Some(SlotLiveness {
            live_out: Vec::new(),
            branch_targets: HashSet::new(),
        });
    }

    let off_to_idx: HashMap<usize, usize> = instrs
        .iter()
        .enumerate()
        .map(|(i, ins)| (ins.offset_dw, i))
        .collect();
    if off_to_idx.len() != instrs.len() {
        return None;
    }

    // Preflight JMPP before cfg::build: its recovery helper reserves `N` rows, so constrain N to
    // the remaining decoded stream first (also proves the complete dispatch table shape).
    for (i, ins) in instrs
        .iter()
        .enumerate()
        .filter(|(_, ins)| ins.op.name == "JMPP")
    {
        let rows = (*ins.dwords.first()? as usize).checked_add(1)?;
        if rows > instrs.len().checked_sub(i + 1)? {
            return None;
        }
        for k in 0..rows {
            let expected_dw = ins.offset_dw.checked_add(2 + 2 * k)?;
            let row = instrs.get(i + 1 + k)?;
            if row.offset_dw != expected_dw || (k + 1 < rows && row.op.name != "JMP") {
                return None;
            }
        }
    }

    let graph = cfg::build(instrs);
    if graph.blocks.is_empty() {
        return None;
    }

    let mut successors = vec![Vec::<usize>::new(); instrs.len()];
    let mut covered = vec![false; instrs.len()];
    let mut block_ending_at = HashMap::<usize, usize>::new();
    for (block_no, block) in graph.blocks.iter().enumerate() {
        if block.instr_lo >= block.instr_hi
            || block.instr_hi > instrs.len()
            || instrs[block.instr_lo].offset_dw != block.start_dw
        {
            return None;
        }
        for i in block.instr_lo..block.instr_hi {
            if std::mem::replace(&mut covered[i], true) {
                return None;
            }
            if i + 1 < block.instr_hi {
                successors[i].push(i + 1);
            }
        }
        let last = block.instr_hi - 1;
        block_ending_at.insert(last, block_no);
        for target_dw in &block.succs {
            let target = *off_to_idx.get(target_dw)?;
            if !successors[last].contains(&target) {
                successors[last].push(target);
            }
        }
    }
    if covered.iter().any(|covered| !covered) {
        return None;
    }

    let mut branch_targets = HashSet::new();
    for (i, ins) in instrs.iter().enumerate() {
        if is_jump_op(ins.op.name) {
            let rel = *ins.dwords.first()? as i32 as i64;
            let target_dw = (ins.offset_dw as i64)
                .checked_add(ins.op.size_dwords as i64)?
                .checked_add(rel)?;
            if target_dw < 0 {
                return None;
            }
            let target = *off_to_idx.get(&(target_dw as usize))?;
            branch_targets.insert(target);
            let block_no = *block_ending_at.get(&i)?;
            if !successors[i].contains(&target) || graph.blocks[block_no].instr_hi - 1 != i {
                return None;
            }
        } else if ins.op.name == "JMPP" {
            let block_no = *block_ending_at.get(&i)?;
            let block = &graph.blocks[block_no];
            if block.succs.is_empty() || block.instr_hi - 1 != i {
                return None;
            }
            for target_dw in &block.succs {
                branch_targets.insert(*off_to_idx.get(target_dw)?);
            }
        }
    }

    let accesses: Vec<_> = instrs.iter().map(slot_accesses).collect();
    let mut live_in = vec![HashSet::<i32>::new(); instrs.len()];
    let mut live_out = vec![HashSet::<i32>::new(); instrs.len()];
    loop {
        let mut changed = false;
        for i in (0..instrs.len()).rev() {
            let mut next_out = HashSet::new();
            for &succ in &successors[i] {
                next_out.extend(live_in[succ].iter().copied());
            }
            let (reads, writes) = &accesses[i];
            let mut next_in = reads.clone();
            next_in.extend(next_out.iter().filter(|s| !writes.contains(s)).copied());
            if next_out != live_out[i] {
                live_out[i] = next_out;
                changed = true;
            }
            if next_in != live_in[i] {
                live_in[i] = next_in;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }

    Some(SlotLiveness {
        live_out,
        branch_targets,
    })
}

fn is_value_copy_producer(name: &str) -> bool {
    matches!(
        name,
        "CpyRtoV4" | "RDR4" | "sbTOi" | "swTOi" | "ubTOi" | "uwTOi"
    )
}

/// N2b — coalesce an immediately copied, dead primitive expression temporary:
///
/// ```text
///     PRODUCER temp, ...; CpyVtoV4 local, temp
///       => PRODUCER local, ...
/// ```
///
/// Only the six exact producer opcodes observed in the qualified final residue are accepted. The
/// source temp must be dead on every CFG path after the copy, the producer may not read the target
/// local, and no branch/dispatch may enter at the copy. The transformation is prepared on a clone
/// and committed atomically; malformed CFG/jump metadata leaves the stream unchanged.
fn coalesce_dead_value_copies(norm: &mut Vec<NormInstr>, raw: &[Instr]) -> usize {
    if norm.len() != raw.len() || raw.len() < 2 {
        return 0;
    }
    let Some(liveness) = slot_liveness(raw) else {
        return 0;
    };
    let accesses: Vec<_> = raw.iter().map(slot_accesses).collect();
    let mut candidates = Vec::<(usize, usize, i32, i32)>::new();

    for i in 0..raw.len() - 1 {
        let producer = &raw[i];
        let copy = &raw[i + 1];
        if !is_value_copy_producer(producer.op.name) || copy.op.name != "CpyVtoV4" {
            continue;
        }
        let Some(src) = producer.words.first().map(|w| *w as i16 as i32) else {
            continue;
        };
        let (Some(dst), Some(copy_src)) = (
            copy.words.first().map(|w| *w as i16 as i32),
            copy.words.get(1).map(|w| *w as i16 as i32),
        ) else {
            continue;
        };
        if src == dst
            || src != copy_src
            || accesses[i].1.len() != 1
            || !accesses[i].1.contains(&src)
            || accesses[i].0.contains(&dst)
            || accesses[i + 1].0 != HashSet::from([src])
            || accesses[i + 1].1 != HashSet::from([dst])
            || liveness.live_out[i + 1].contains(&src)
            || liveness.branch_targets.contains(&(i + 1))
        {
            continue;
        }
        let norm_shape_matches = matches!(norm[i].operands.first(), Some(Operand::Slot(s)) if *s == src)
            && matches!(norm[i + 1].operands.as_slice(), [Operand::Slot(d), Operand::Slot(s)] if *d == dst && *s == src)
            && norm[i].op == producer.op.name
            && norm[i + 1].op == "CpyVtoV4";
        if norm_shape_matches {
            candidates.push((i, i + 1, src, dst));
        }
    }
    if candidates.is_empty() {
        return 0;
    }

    let mut drop = vec![false; norm.len()];
    for &(_, copy, _, _) in &candidates {
        if std::mem::replace(&mut drop[copy], true) {
            return 0;
        }
    }
    // N3 edges must remain exact after instruction removal. A target into a removed copy is also
    // rejected above from the raw CFG, but repeat the check on normalized metadata so this pass is
    // safe even if its caller supplies inconsistent raw/normalized streams.
    for ni in norm.iter() {
        for op in &ni.operands {
            if let Operand::JumpIndex(target) = op {
                let Some(target) = *target else {
                    return 0;
                };
                if target > drop.len() || (target < drop.len() && drop[target]) {
                    return 0;
                }
            }
        }
    }

    let mut rewritten = norm.clone();
    for &(producer, _, src, dst) in &candidates {
        match rewritten[producer].operands.first_mut() {
            Some(Operand::Slot(slot)) if *slot == src => *slot = dst,
            _ => return 0,
        }
    }
    let mut old_to_new = vec![None; rewritten.len()];
    let mut next = 0usize;
    for (i, dropped) in drop.iter().copied().enumerate() {
        if !dropped {
            old_to_new[i] = Some(next);
            next += 1;
        }
    }
    for (i, ni) in rewritten.iter_mut().enumerate() {
        if drop[i] {
            continue;
        }
        for op in &mut ni.operands {
            if let Operand::JumpIndex(Some(target)) = op {
                *target = if *target == old_to_new.len() {
                    next
                } else {
                    old_to_new.get(*target).and_then(|x| *x).unwrap_or(*target)
                };
            }
        }
    }
    let mut i = 0usize;
    rewritten.retain(|_| {
        let keep = !drop[i];
        i += 1;
        keep
    });
    *norm = rewritten;
    candidates.len()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct NormSlotRole {
    operand: usize,
    read: bool,
    write: bool,
}

/// Recover slot read/write roles from the opcode's ISA format and verify that every normalized
/// `Slot` operand is accounted for. This mirrors [`slot_accesses`] but operates after N5/N6/N2b,
/// where the retained instruction stream no longer has a 1:1 raw-index mapping.
fn norm_slot_roles(ni: &NormInstr) -> Option<Vec<NormSlotRole>> {
    use BcType::*;

    let fmt = OPCODES.iter().find(|op| op.name == ni.op)?.fmt;
    let mut roles = Vec::new();
    let mut add = |operand: usize, read: bool, write: bool| {
        roles.push(NormSlotRole {
            operand,
            read,
            write,
        });
    };
    match fmt {
        wW_ARG | wW_DW_ARG | wW_QW_ARG => add(0, false, true),
        rW_ARG | rW_DW_ARG | rW_QW_ARG | rW_DW_DW_ARG => add(0, true, false),
        wW_rW_ARG | wW_rW_DW_ARG => {
            add(0, false, true);
            add(1, true, false);
        }
        rW_rW_ARG => {
            add(0, true, false);
            add(1, true, false);
        }
        wW_rW_rW_ARG => {
            add(0, false, true);
            add(1, true, false);
            add(2, true, false);
        }
        wW_W_ARG => add(0, false, true),
        W_rW_ARG => add(1, true, false),
        rW_W_DW_ARG => add(0, true, false),
        W_ARG if ni.op == "ChkNullS" => add(0, true, false),
        INFO | NO_ARG | W_ARG | DW_ARG | QW_ARG | DW_DW_ARG | QW_DW_ARG | W_DW_ARG => {}
    }
    if matches!(
        ni.op,
        "NOT"
            | "NEGi"
            | "NEGf"
            | "NEGd"
            | "IncVi"
            | "DecVi"
            | "BNOT"
            | "NEGi64"
            | "BNOT64"
            | "FREE"
            | "LOADOBJ"
            | "RefCpyV"
            | "FreeNullV8"
    ) {
        let role = roles.iter_mut().find(|role| role.operand == 0)?;
        role.read = true;
        role.write = true;
    }
    if roles
        .iter()
        .any(|role| !matches!(ni.operands.get(role.operand), Some(Operand::Slot(_))))
        || ni.operands.iter().enumerate().any(|(i, operand)| {
            matches!(operand, Operand::Slot(_)) && !roles.iter().any(|role| role.operand == i)
        })
    {
        return None;
    }
    Some(roles)
}

/// Exact instruction-level successors for a retained normalized stream. Unlike the recovery CFG,
/// every ordinary jump must have an N3 target, every target must be in-range (the end sentinel is
/// an exit), and every JMPP dispatch table must retain its verified row topology.
fn norm_successors(instrs: &[NormInstr]) -> Option<Vec<Vec<usize>>> {
    let mut successors = vec![Vec::new(); instrs.len()];
    for (i, ni) in instrs.iter().enumerate() {
        let mut add = |target: usize| -> Option<()> {
            if target > instrs.len() {
                return None;
            }
            if target < instrs.len() && !successors[i].contains(&target) {
                successors[i].push(target);
            }
            Some(())
        };
        if is_jump_op(ni.op) {
            let mut targets = ni.operands.iter().filter_map(|operand| match operand {
                Operand::JumpIndex(target) => Some(*target),
                _ => None,
            });
            let target = targets.next()??;
            if targets.next().is_some() {
                return None;
            }
            add(target)?;
            if ni.op != "JMP" && i + 1 < instrs.len() {
                add(i + 1)?;
            }
        } else if ni.op == "JMPP" {
            let mut maxima = ni.operands.iter().filter_map(|operand| match operand {
                Operand::RawDw(max) => Some(*max as usize),
                _ => None,
            });
            let rows = maxima.next()?.checked_add(1)?;
            if maxima.next().is_some() || rows > instrs.len().checked_sub(i + 1)? {
                return None;
            }
            for k in 0..rows {
                let row = i + 1 + k;
                if k + 1 < rows && instrs[row].op != "JMP" {
                    return None;
                }
                add(row)?;
            }
        } else if !matches!(ni.op, "RET" | "ThrowException") && i + 1 < instrs.len() {
            add(i + 1)?;
        }
    }
    Some(successors)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum FlowOrigin {
    Entry(i32),
    Write { instruction: usize, operand: usize },
}

type OriginSet = BTreeSet<FlowOrigin>;
type FlowState = HashMap<i32, OriginSet>;

#[derive(Debug)]
struct SlotFlow {
    incoming: Vec<FlowState>,
    roles: Vec<Vec<NormSlotRole>>,
    /// Slots whose storage identity escapes (address/lvalue push). These retain a strict global
    /// bijection; physical live-range splitting is permitted only for non-escaping value slots.
    pinned: HashSet<i32>,
}

fn merge_flow_state(into: &mut FlowState, from: &FlowState) {
    for (&slot, origins) in from {
        into.entry(slot).or_default().extend(origins);
    }
}

/// Reaching-definition analysis for normalized frame slots. Every explicit write is named by its
/// aligned instruction/operand position, independent of the compiler's chosen physical slot.
/// All instructions must be reachable from entry; otherwise this stronger N2 proof declines.
fn analyze_slot_flow(instrs: &[NormInstr]) -> Option<SlotFlow> {
    if instrs.is_empty() {
        return Some(SlotFlow {
            incoming: Vec::new(),
            roles: Vec::new(),
            pinned: HashSet::new(),
        });
    }
    let successors = norm_successors(instrs)?;
    let roles: Vec<_> = instrs.iter().map(norm_slot_roles).collect::<Option<_>>()?;

    let mut reachable = vec![false; instrs.len()];
    let mut stack = vec![0usize];
    while let Some(i) = stack.pop() {
        if std::mem::replace(&mut reachable[i], true) {
            continue;
        }
        stack.extend(successors[i].iter().copied());
    }
    if reachable.iter().any(|reachable| !reachable) {
        return None;
    }

    let mut predecessors = vec![Vec::<usize>::new(); instrs.len()];
    for (from, targets) in successors.iter().enumerate() {
        for &target in targets {
            predecessors[target].push(from);
        }
    }
    let all_slots: HashSet<i32> = instrs
        .iter()
        .flat_map(|ni| ni.operands.iter())
        .filter_map(|operand| match operand {
            Operand::Slot(slot) => Some(*slot),
            _ => None,
        })
        .collect();
    let entry: FlowState = all_slots
        .iter()
        .map(|&slot| (slot, BTreeSet::from([FlowOrigin::Entry(slot)])))
        .collect();
    let mut incoming = vec![FlowState::new(); instrs.len()];
    let mut outgoing = vec![FlowState::new(); instrs.len()];
    loop {
        let mut changed = false;
        for i in 0..instrs.len() {
            let mut next_in = FlowState::new();
            if i == 0 {
                merge_flow_state(&mut next_in, &entry);
            }
            for &pred in &predecessors[i] {
                merge_flow_state(&mut next_in, &outgoing[pred]);
            }
            let mut next_out = next_in.clone();
            for role in roles[i].iter().filter(|role| role.write) {
                let Operand::Slot(slot) = instrs[i].operands[role.operand] else {
                    return None;
                };
                next_out.insert(
                    slot,
                    BTreeSet::from([FlowOrigin::Write {
                        instruction: i,
                        operand: role.operand,
                    }]),
                );
            }
            if incoming[i] != next_in {
                incoming[i] = next_in;
                changed = true;
            }
            if outgoing[i] != next_out {
                outgoing[i] = next_out;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }

    let mut pinned = HashSet::new();
    for (i, ni) in instrs.iter().enumerate() {
        // PSF is an explicit frame address; VAR and PshVPtr can carry lvalue/RVO storage into a
        // call. Keep their storage identity under one global alpha-renaming rather than splitting
        // it by reaching definition.
        // PSF exposes the frame address, VAR exposes its encoded frame offset, and LDV loads the
        // frame address into the VM register. PshVPtr only pushes the pointer VALUE stored in the
        // slot, so ordinary reaching-definition identity is sufficient for it. SetV1/SetV2 and
        // the 8/16-bit reads/conversions also need no storage pin: the VM defines/zero-extends a
        // complete DWORD for their frame result (AngelScript as_context.cpp).
        let storage_sensitive = matches!(ni.op, "PSF" | "VAR" | "LDV");
        if storage_sensitive {
            for role in &roles[i] {
                if let Operand::Slot(slot) = ni.operands[role.operand] {
                    pinned.insert(slot);
                }
            }
        }
    }

    Some(SlotFlow {
        incoming,
        roles,
        pinned,
    })
}

fn bind_bijection(
    left: i32,
    right: i32,
    left_to_right: &mut HashMap<i32, i32>,
    right_to_left: &mut HashMap<i32, i32>,
) -> bool {
    if left_to_right
        .get(&left)
        .is_some_and(|mapped| *mapped != right)
        || right_to_left
            .get(&right)
            .is_some_and(|mapped| *mapped != left)
    {
        return false;
    }
    left_to_right.insert(left, right);
    right_to_left.insert(right, left);
    true
}

fn origin_sets_equivalent(
    left: &OriginSet,
    right: &OriginSet,
    entry_left_to_right: &mut HashMap<i32, i32>,
    entry_right_to_left: &mut HashMap<i32, i32>,
) -> bool {
    let left_writes: BTreeSet<_> = left
        .iter()
        .filter_map(|origin| match origin {
            FlowOrigin::Write {
                instruction,
                operand,
            } => Some((*instruction, *operand)),
            FlowOrigin::Entry(_) => None,
        })
        .collect();
    let right_writes: BTreeSet<_> = right
        .iter()
        .filter_map(|origin| match origin {
            FlowOrigin::Write {
                instruction,
                operand,
            } => Some((*instruction, *operand)),
            FlowOrigin::Entry(_) => None,
        })
        .collect();
    if left_writes != right_writes {
        return false;
    }
    let left_entries: Vec<_> = left
        .iter()
        .filter_map(|origin| match origin {
            FlowOrigin::Entry(slot) => Some(*slot),
            FlowOrigin::Write { .. } => None,
        })
        .collect();
    let right_entries: Vec<_> = right
        .iter()
        .filter_map(|origin| match origin {
            FlowOrigin::Entry(slot) => Some(*slot),
            FlowOrigin::Write { .. } => None,
        })
        .collect();
    match (left_entries.as_slice(), right_entries.as_slice()) {
        ([], []) => true,
        // ABI-visible `this`/parameter/RVO offsets are <= 0 and are fixed by the signature, not
        // local register allocation. Never alpha-rename them: that could hide a parameter swap.
        ([left], [right]) if *left <= 0 || *right <= 0 => left == right,
        ([left], [right]) => {
            bind_bijection(*left, *right, entry_left_to_right, entry_right_to_left)
        }
        _ => false,
    }
}

/// Strong N2 proof for register reuse/splitting. Opcode and every non-slot operand must already
/// match. At each read operand, both sides must have the exact same aligned reaching writes (or a
/// consistent bijection of entry values). Address/lvalue slots additionally keep a global storage
/// bijection. This permits only physical allocation differences; a changed producer, CFG edge,
/// operand order, reaching definition, or escaped-storage identity remains SEMANTIC.
fn flow_equivalent_slots(left: &[NormInstr], right: &[NormInstr]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    let (Some(left_flow), Some(right_flow)) = (analyze_slot_flow(left), analyze_slot_flow(right))
    else {
        return false;
    };
    let mut entry_l2r = HashMap::new();
    let mut entry_r2l = HashMap::new();
    let mut storage_l2r = HashMap::new();
    let mut storage_r2l = HashMap::new();

    for i in 0..left.len() {
        if left[i].op != right[i].op
            || left[i].operands.len() != right[i].operands.len()
            || left_flow.roles[i] != right_flow.roles[i]
        {
            return false;
        }
        for operand in 0..left[i].operands.len() {
            match (&left[i].operands[operand], &right[i].operands[operand]) {
                (Operand::Slot(left_slot), Operand::Slot(right_slot)) => {
                    let Some(role) = left_flow.roles[i]
                        .iter()
                        .find(|role| role.operand == operand)
                    else {
                        return false;
                    };
                    if (left_flow.pinned.contains(left_slot)
                        || right_flow.pinned.contains(right_slot))
                        && ((*left_slot <= 0 || *right_slot <= 0) && left_slot != right_slot
                            || !bind_bijection(
                                *left_slot,
                                *right_slot,
                                &mut storage_l2r,
                                &mut storage_r2l,
                            ))
                    {
                        return false;
                    }
                    if role.read {
                        let Some(left_origins) = left_flow.incoming[i].get(left_slot) else {
                            return false;
                        };
                        let Some(right_origins) = right_flow.incoming[i].get(right_slot) else {
                            return false;
                        };
                        if !origin_sets_equivalent(
                            left_origins,
                            right_origins,
                            &mut entry_l2r,
                            &mut entry_r2l,
                        ) {
                            return false;
                        }
                    }
                }
                (left_operand, right_operand) if left_operand == right_operand => {}
                _ => return false,
            }
        }
    }
    true
}

// =================================================================================================
// Verdict + classification.
// =================================================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Identical,
    Benign,
    Semantic,
}

/// Which normalizers were responsible for collapsing a difference (BENIGN audit trail).
#[derive(Debug, Default, Clone, Copy)]
pub struct NormFired {
    pub n1_refs: bool,
    pub n2_slots: bool,
    pub n3_jumps: bool,
    pub n4_consts: bool,
    pub n5_scope: bool,
    pub n6_reguard: bool,
}

impl NormFired {
    pub fn labels(&self) -> Vec<&'static str> {
        let mut v = Vec::new();
        if self.n1_refs {
            v.push("N1:refs");
        }
        if self.n2_slots {
            v.push("N2:slots");
        }
        if self.n3_jumps {
            v.push("N3:jumps");
        }
        if self.n4_consts {
            v.push("N4:consts");
        }
        if self.n5_scope {
            v.push("N5:scope");
        }
        if self.n6_reguard {
            v.push("N6:reguard");
        }
        v
    }
    fn any(&self) -> bool {
        self.n1_refs
            || self.n2_slots
            || self.n3_jumps
            || self.n4_consts
            || self.n5_scope
            || self.n6_reguard
    }
}

/// Result of diffing one aligned function pair.
#[derive(Debug, Clone)]
pub struct FuncDiff {
    pub name: String,
    pub verdict: Verdict,
    pub fired: NormFired,
    pub v_ops: usize,
    pub r_ops: usize,
    /// For SEMANTIC: index of the first diverging normalized instruction (if lengths let us find
    /// one), plus a human hint.
    pub first_divergence: Option<usize>,
    pub hint: Option<String>,
    /// For SEMANTIC: rendered divergence window (both sides, ±context).
    pub window: Option<String>,
}

/// Compare two RAW instruction lists element-wise (no normalization). True == byte/structure
/// identical (same op + same raw words/dwords/qwords).
fn raw_eq(a: &[Instr], b: &[Instr]) -> bool {
    a.len() == b.len()
        && a.iter().zip(b).all(|(x, y)| {
            x.op.name == y.op.name
                && x.words == y.words
                && x.dwords == y.dwords
                && x.qwords == y.qwords
        })
}

/// The strictly-vetted benign OP-level tolerance set (spec §3.5). Currently: only `JitEntry`
/// scaffolding (opcode 175) — a JIT hook the VM may or may not emit, no semantic effect. NOTE:
/// dup null-check / CHKREF / ChkNullV are DELIBERATELY EXCLUDED (spec: benign in some fns, the
/// real breaker in others). We only tolerate a `JitEntry` that appears on ONE side with no
/// counterpart; anything else stays SEMANTIC.
fn is_benign_only_op(name: &str) -> bool {
    name == "JitEntry"
}

/// Filter a normalized list to drop `JitEntry` scaffolding (so a JIT-hook-only difference does
/// not force SEMANTIC). Returns the filtered list; the caller notes it fired only if it changed
/// the length.
fn strip_jitentry(instrs: &[NormInstr]) -> Vec<NormInstr> {
    instrs
        .iter()
        .filter(|ni| !is_benign_only_op(ni.op))
        .cloned()
        .collect()
}

// =================================================================================================
// N5 / N6 — benign-attribution instrumentation/control-flow strips
// (`specs/final-residue.md` PART B).
//
// N6 is a one-sided VANILLA fold. N5 strips only exact known profiler frames, on either side:
// different source spellings can retain an FScopeCycleCounter dtor in vanilla but an FStatID temp
// dtor in regen. Both identities are proven timing-only instrumentation. Neither normalizer pads or
// inserts operations. Governing safety rule (`bytediff.rs:12`): a false BENIGN hides a real bug
// (catastrophic); when a pattern cannot be PROVEN inert/dominated, it is left in place and the
// mismatch keeps the function SEMANTIC.
// =================================================================================================

/// The (owner-type-name, method-name) of a normalized CALLSYS/CALL instruction's callee, if it
/// resolved to a `Named` function identity. Keys N5 on a cache-INDEPENDENT identity (the raw
/// func-ptr drifts across builds; the resolved owner+method does not — mirrors GAP-B/GAP-C keying
/// on `callee_name`). Returns `None` for a non-call op or an unresolved callee.
fn callsys_owner_method(ni: &NormInstr) -> Option<(&str, &str)> {
    if !matches!(
        ni.op,
        "CALLSYS" | "CALL" | "CALLBND" | "CALLINTF" | "Thiscall1" | "FuncPtr"
    ) {
        return None;
    }
    ni.operands.iter().find_map(|o| match o {
        Operand::Ref(id) => id.func_owner_method(),
        _ => None,
    })
}

/// N5 — `FScopeCycleCounter` RAII profiler-scope strip (`specs/final-residue.md §B.2`).
///
/// Removes exact call frames for the three inert RAII identities:
///   * `FScopeCycleCounter::$beh0`  — the RAII scope-counter CTOR (snapshots `Cycles()`)
///   * `FScopeCycleCounter::$beh2`  — the RAII scope-counter DTOR (accumulates elapsed cycles)
///   * `FStatID::$beh2`             — the transient `FStatID` temp DTOR that only fed the ctor
///
/// The `FStatID::$beh0` CTOR is KEPT (both sides emit it — NOT stripped). Exact arity is mandatory:
/// the FScope ctor must have exactly two immediately-preceding PSFs (FStatID argument then distinct
/// destination slot), while either dtor must have exactly one. A missing/extra frame push, aliased
/// ctor slots, unresolved identity, or jump into a would-be-dropped frame rejects the entire strip.
/// Retained N3 jump targets are rebased after removal.
///
/// Behaviour proof (§B.2): the three callees touch no game object / ability / actor / return
/// register — they read `FPlatformTime::Cycles()` and accumulate into a named `TStatId` CPU
/// counter (compiled-in `SCOPE_CYCLE_COUNTER` instrumentation). Removing them changes only the
/// stats-HUD timing readout. Provably behavior-neutral.
///
/// Returns the number of profiler CALLSYS ops removed.
fn strip_benign_scopes(v: &mut Vec<NormInstr>) -> usize {
    let psf_slot = |ni: &NormInstr| match (ni.op, ni.operands.as_slice()) {
        ("PSF", [Operand::Slot(slot)]) => Some(*slot),
        _ => None,
    };

    // Identify exact inert call frames. Any recognized-but-malformed profiler call rejects the
    // entire pass atomically: partially deleting one lifetime while retaining another would make
    // the normalizer harder to reason about and could hide malformed control flow.
    let mut drop: Vec<bool> = vec![false; v.len()];
    let mut removed = 0usize;
    for i in 0..v.len() {
        match callsys_owner_method(&v[i]) {
            Some(("FScopeCycleCounter", "$beh0")) => {
                let exact_two_psfs = i >= 2
                    && (i < 3 || v[i - 3].op != "PSF")
                    && psf_slot(&v[i - 2])
                        .zip(psf_slot(&v[i - 1]))
                        .is_some_and(|(stat, scope)| stat != scope);
                if !exact_two_psfs {
                    return 0;
                }
                if drop[i - 2] || drop[i - 1] || drop[i] {
                    return 0;
                }
                drop[i - 2] = true;
                drop[i - 1] = true;
                drop[i] = true;
                removed += 1;
            }
            Some(("FScopeCycleCounter", "$beh2")) | Some(("FStatID", "$beh2")) => {
                let exact_one_psf =
                    i >= 1 && (i < 2 || v[i - 2].op != "PSF") && psf_slot(&v[i - 1]).is_some();
                if !exact_one_psf {
                    return 0;
                }
                if drop[i - 1] || drop[i] {
                    return 0;
                }
                drop[i - 1] = true;
                drop[i] = true;
                removed += 1;
            }
            _ => {}
        }
    }
    if removed == 0 {
        return 0;
    }
    // A branch into a call frame would make frame removal a control-flow rewrite rather than an
    // instrumentation deletion. Fail the entire pass atomically.
    if v.iter().any(|ni| {
        ni.operands.iter().any(|op| match op {
            Operand::JumpIndex(Some(target)) => {
                *target > drop.len() || (*target < drop.len() && drop[*target])
            }
            _ => false,
        })
    }) {
        return 0;
    }

    // Build old-index -> retained-index before removal and rebase every retained N3 edge. Targets
    // of dropped ops were rejected above, so every concrete target has one exact new index.
    let mut old_to_new = vec![None; v.len()];
    let mut next = 0usize;
    for (i, dropped) in drop.iter().copied().enumerate() {
        if !dropped {
            old_to_new[i] = Some(next);
            next += 1;
        }
    }
    for (i, ni) in v.iter_mut().enumerate() {
        if drop[i] {
            continue;
        }
        for op in &mut ni.operands {
            if let Operand::JumpIndex(Some(target)) = op {
                if *target == old_to_new.len() {
                    *target = next;
                } else if let Some(Some(rebased)) = old_to_new.get(*target) {
                    *target = *rebased;
                }
            }
        }
    }
    let mut idx = 0usize;
    v.retain(|_| {
        let keep = !drop[idx];
        idx += 1;
        keep
    });
    removed
}

/// True if a normalized op WRITES the object/ref frame slot `slot` — the anti-false-benign clause-3
/// scan for N6. Any op that reassigns / re-references the guarded slot between the dominating guard
/// and the re-guard breaks domination (the guarded value may have changed), so the re-guard is NOT
/// redundant and must NOT be folded. Conservative: an op we are unsure about is treated as a write.
fn writes_slot(ni: &NormInstr, slot: i32) -> bool {
    // Ops whose FIRST slot operand is a destination (write) of an object/ref slot.
    let dst_first = matches!(
        ni.op,
        "RefCpyV"        // ref-copy INTO a slot (the re-guard's own load target — a write)
            | "STOREOBJ" // store an object handle INTO a slot
            | "CpyVtoV4" | "CpyRtoV4" | "CpyGtoV4" | "LdGRdR4"
            | "SetV1" | "SetV2" | "SetV4" | "SetV8"
            | "FreeV" | "FreeNullV8" | "AllocMem"
    );
    if dst_first {
        if let Some(Operand::Slot(s)) = ni.operands.first() {
            if *s == slot {
                return true;
            }
        }
    }
    false
}

/// N6 — dominated short-circuit boolean-cascade re-guard fold (`specs/final-residue.md §B.1`).
///
/// A re-guard is a null-check on a STABLE object slot that was ALREADY null-checked earlier in the
/// SAME boolean cascade, on a value that has NOT been reassigned since:
///   S1 (object slot):  PshVPtr <slot X> ; RefCpyV <slot Y> ; CmpPtrNull <slot Y> ; TNZ
/// The 4-op S1 window is stripped from the VANILLA list ONLY when PROVABLY DOMINATED:
///   (clause 1) an identical earlier guard on the SAME source slot X exists in this function, AND
///   (clause 3) no write to slot X intervenes between the dominating guard and the re-guard.
/// (Clause 2 of the spec — the re-guard sits in an accumulator-merge term — is subsumed here by
/// requiring the exact S1 op-shape AND a `TNZ` terminator, which only the cascade emits.)
///
/// The S2 form (`STOREOBJ <slot>; CmpPtrNull <slot>; TNZ`) is DELIBERATELY NOT folded: its own
/// `STOREOBJ` reassigns the slot with a FRESH call result, so it guards a new value — a genuine
/// safe-nav null-check, never a redundant re-check. Folding it would risk hiding a real dropped
/// guard on a fresh result (catastrophic). Conservatism over coverage (`bytediff.rs:12`).
///
/// Behaviour proof (§B.1): in an `&&`/`||` short-circuit chain, term-k's guard `X != null` is
/// reached only when term-(k-1) held, and the earlier identical guard already established
/// `X != null`; with no intervening write to X, the value is unchanged → the re-guard is provably
/// dead. Clause 3 is the load-bearing anti-false-benign guard: a *genuine* dropped null-check (slot
/// never guarded earlier, clause 1 fails; or reassigned since, clause 3 fails) is left in place and
/// the length mismatch keeps the function SEMANTIC.
///
/// One-sided (vanilla only — regen's decompiled accumulator emits one guard per slot, never the
/// per-term re-guards). Returns the number of re-guard windows folded.
fn fold_dominated_reguards(v: &mut Vec<NormInstr>) -> usize {
    let mut drop: Vec<bool> = vec![false; v.len()];
    let mut folded = 0usize;

    let mut i = 0usize;
    while i < v.len() {
        // Try S1 at i: PshVPtr <X> ; RefCpyV <Y> ; CmpPtrNull <Y> ; TNZ
        if let Some((src, _guard_slot, win_len)) = match_reguard(v, i) {
            // clause 1 + clause 3: an identical earlier guard on the SAME source with no
            // intervening write to that slot.
            if dominating_guard_exists(v, i, &src) {
                for slot in drop.iter_mut().take(i + win_len).skip(i) {
                    *slot = true;
                }
                folded += 1;
                i += win_len;
                continue;
            }
        }
        i += 1;
    }

    if folded > 0 {
        // Removing an entered window would be a control-flow rewrite, not a redundant expression
        // fold. Reject atomically on an unresolved/out-of-range target or any edge into a dropped
        // instruction, then rebase every retained N3 target (including the end sentinel).
        if v.iter().any(|ni| {
            ni.operands.iter().any(|op| match op {
                Operand::JumpIndex(Some(target)) => {
                    *target > drop.len() || (*target < drop.len() && drop[*target])
                }
                Operand::JumpIndex(None) => true,
                _ => false,
            })
        }) {
            return 0;
        }
        let mut old_to_new = vec![None; v.len()];
        let mut next = 0usize;
        for (i, dropped) in drop.iter().copied().enumerate() {
            if !dropped {
                old_to_new[i] = Some(next);
                next += 1;
            }
        }
        for (i, ni) in v.iter_mut().enumerate() {
            if drop[i] {
                continue;
            }
            for op in &mut ni.operands {
                if let Operand::JumpIndex(Some(target)) = op {
                    *target = if *target == old_to_new.len() {
                        next
                    } else {
                        old_to_new[*target].expect("dropped targets rejected above")
                    };
                }
            }
        }
        let mut idx = 0usize;
        v.retain(|_| {
            let keep = !drop[idx];
            idx += 1;
            keep
        });
    }
    folded
}

/// The identity of a re-guard's SOURCE — what value is being re-checked. For an S1 re-guard the
/// source is the `PshVPtr <slot X>` operand (a stable frame/member object slot). Compared on the
/// NORMALIZED operand (post-N1/N2).
#[derive(Debug, Clone, PartialEq)]
enum ReguardSrc {
    /// S1: `PshVPtr <slot>` re-loaded a stack/frame object slot for a null-check.
    Push(Operand),
}

/// If a FOLDABLE S1 re-guard window begins at `pos`, return `(source, guarded_slot, window_len)`.
/// Only the S1 4-op `PshVPtr X; RefCpyV Y; CmpPtrNull Y; TNZ` form is foldable (S2 is never folded
/// — see [`fold_dominated_reguards`] docs). Otherwise `None`.
fn match_reguard(v: &[NormInstr], pos: usize) -> Option<(ReguardSrc, i32, usize)> {
    if pos + 3 < v.len()
        && v[pos].op == "PshVPtr"
        && v[pos + 1].op == "RefCpyV"
        && v[pos + 2].op == "CmpPtrNull"
        && v[pos + 3].op == "TNZ"
    {
        let push = v[pos].operands.first()?.clone();
        // Only a slot-addressed push is a stable, trackable source (clause-3 needs a slot).
        if !matches!(push, Operand::Slot(_)) {
            return None;
        }
        let y = slot_of(&v[pos + 1])?;
        // The CmpPtrNull must test the SAME slot Y that RefCpyV loaded.
        if slot_of(&v[pos + 2]) == Some(y) {
            return Some((ReguardSrc::Push(push), y, 4));
        }
    }
    None
}

/// The first `Slot` operand of an instruction (the addressed frame slot), if any.
fn slot_of(ni: &NormInstr) -> Option<i32> {
    ni.operands.iter().find_map(|o| match o {
        Operand::Slot(s) => Some(*s),
        _ => None,
    })
}

/// Clause 1 + clause 3 of the domination gate: scanning BACKWARD from the re-guard at `pos`, is
/// there an EARLIER guard on the identical source, with NO write to that source's slot in between?
///
/// For an S1 `Push(slot X)` source the dominating guard is an earlier S1 window (term `TNZ` form or
/// cascade-head `JNZ` form) whose `PshVPtr` operand equals `X`. If a write to slot X is found before
/// a matching guard, the re-guard is NOT dominated (clause 3 fails) → return false (leave SEMANTIC).
fn dominating_guard_exists(v: &[NormInstr], pos: usize, src: &ReguardSrc) -> bool {
    // The slot whose non-null-ness must be preserved (for the intervening-write scan).
    let ReguardSrc::Push(Operand::Slot(guard_slot)) = src else {
        return false;
    };
    let guard_slot = *guard_slot;

    let mut j = pos;
    while j > 0 {
        j -= 1;
        // clause 3: any intervening WRITE to the guarded slot breaks domination — UNLESS the write
        // op is itself the START of the matching earlier guard window (a dominator's own load is
        // not a hostile reassignment). Check for the guard shape at `j` first.
        if writes_slot(&v[j], guard_slot) {
            if guard_matches_source(v, j, src) {
                return true;
            }
            return false;
        }
        // clause 1: an earlier guard on the identical source dominates.
        if guard_matches_source(v, j, src) {
            return true;
        }
    }
    false
}

/// True if a guard window starting at index `j` guards the SAME source as `src` — an S1 term guard
/// (`...; TNZ`) whose push equals `src`'s push, OR the cascade-HEAD guard (`...; JNZ`) on the same
/// push. Both establish `X != null`, so either dominates a later re-guard on `X`.
fn guard_matches_source(v: &[NormInstr], j: usize, src: &ReguardSrc) -> bool {
    if let Some((ReguardSrc::Push(b), _slot, _len)) = match_reguard(v, j) {
        let ReguardSrc::Push(a) = src;
        return *a == b;
    }
    // Also accept the CASCADE-HEAD guard shape `PshVPtr X; RefCpyV Y; CmpPtrNull Y; JNZ` (the very
    // first term branches into the merge with JNZ, not TNZ) as a dominator for an S1 source.
    head_guard_matches(v, j, src)
}

/// The FIRST term of an `&&` cascade guards with `... ; JNZ` (branch into the accumulator merge)
/// rather than `TNZ`. Recognise `PshVPtr X ; RefCpyV Y ; CmpPtrNull Y ; JNZ` as a dominating guard
/// for an S1 `Push(X)` source (clause 1), so a later `TNZ` re-guard on the same X is dominated.
fn head_guard_matches(v: &[NormInstr], j: usize, src: &ReguardSrc) -> bool {
    let ReguardSrc::Push(push) = src;
    if j + 3 < v.len()
        && v[j].op == "PshVPtr"
        && v[j + 1].op == "RefCpyV"
        && v[j + 2].op == "CmpPtrNull"
        && is_jump_op(v[j + 3].op)
    {
        let y = slot_of(&v[j + 1]);
        if y.is_some() && slot_of(&v[j + 2]) == y {
            return v[j].operands.first() == Some(push);
        }
    }
    false
}

/// Classify one aligned function pair given both sides' normalized + raw instruction lists.
#[allow(clippy::too_many_arguments)]
fn classify(
    name: String,
    v_raw: &[Instr],
    r_raw: &[Instr],
    v_norm: &[NormInstr],
    r_norm: &[NormInstr],
    opts: &NormOpts,
    context: usize,
) -> FuncDiff {
    let v_ops = v_raw.len();
    let r_ops = r_raw.len();

    // Fast path: raw byte/structure identity ⇒ IDENTICAL (no normalizer needed).
    if raw_eq(v_raw, r_raw) {
        return FuncDiff {
            name,
            verdict: Verdict::Identical,
            fired: NormFired::default(),
            v_ops,
            r_ops,
            first_divergence: None,
            hint: None,
            window: None,
        };
    }

    // N2b coalesces only an exact producer -> dead-temp copy pair proven by whole-CFG liveness.
    // It runs before any strip while normalized and raw instruction indices are still 1:1.
    let mut v_prestrip = v_norm.to_vec();
    let mut r_prestrip = r_norm.to_vec();
    let (v_copy_count, r_copy_count) = if opts.n2_slots {
        (
            coalesce_dead_value_copies(&mut v_prestrip, v_raw),
            coalesce_dead_value_copies(&mut r_prestrip, r_raw),
        )
    } else {
        (0, 0)
    };
    let copy_fired = v_copy_count > 0 || r_copy_count > 0;

    // Compare normalized lists, tolerating JitEntry scaffolding.
    let (mut v_cmp, mut r_cmp, jit_fired) = {
        let vj = strip_jitentry(&v_prestrip);
        let rj = strip_jitentry(&r_prestrip);
        let fired = vj.len() != v_prestrip.len() || rj.len() != r_prestrip.len();
        (vj, rj, fired)
    };

    // N5/N6 benign-attribution strips. N5 removes only exact known profiler frames from either
    // side (vanilla FScope vs regen FStat temp lifetimes can differ); N6 remains vanilla-only.
    // Scope-first keeps N6's contiguous windows intact. Neither operation pads a stream.
    let scope_fired = if opts.n5_scope {
        let v_removed = strip_benign_scopes(&mut v_cmp);
        let r_removed = strip_benign_scopes(&mut r_cmp);
        v_removed > 0 || r_removed > 0
    } else {
        false
    };
    let reguard_fired = if opts.n6_reguard {
        fold_dominated_reguards(&mut v_cmp) > 0
    } else {
        false
    };

    // N2 has two progressively stronger, fail-closed proofs. First-use alpha-renaming remains the
    // cheap path and retains its equal-distinct-slot-count guard. If physical register reuse/split
    // changes that shape, the reaching-definition proof may still establish exact value-flow
    // equivalence. Keep the raw-slot streams intact for the latter; renumber only clones.
    let raw_cmp_identical =
        v_cmp.len() == r_cmp.len() && v_cmp.iter().zip(&r_cmp).all(|(a, b)| a.norm_eq(b));
    let (simple_n2_identical, flow_n2_identical) = if opts.n2_slots && !raw_cmp_identical {
        let mut v_simple = v_cmp.clone();
        let mut r_simple = r_cmp.clone();
        let slot_counts_equal = distinct_slot_count(&v_simple) == distinct_slot_count(&r_simple);
        let _ = renumber_slots(&mut v_simple);
        let _ = renumber_slots(&mut r_simple);
        let simple = slot_counts_equal
            && v_simple.len() == r_simple.len()
            && v_simple
                .iter()
                .zip(&r_simple)
                .all(|(left, right)| left.norm_eq(right));
        let flow = !simple && flow_equivalent_slots(&v_cmp, &r_cmp);
        (simple, flow)
    } else {
        (false, false)
    };
    let norm_identical = raw_cmp_identical || simple_n2_identical || flow_n2_identical;
    let poststrip_n2_fired = simple_n2_identical || flow_n2_identical;

    if norm_identical {
        // BENIGN: determine WHICH normalizers were responsible by re-diffing raw operand roles.
        let mut fired = which_normalizers_fired(v_raw, r_raw, v_norm, r_norm, opts, jit_fired);
        fired.n2_slots |= poststrip_n2_fired || copy_fired;
        fired.n5_scope = scope_fired;
        fired.n6_reguard = reguard_fired;
        // Defensive: if raw differs but NO normalizer is credited and no JitEntry/N5/N6 fired, that
        // is a classifier blind spot — treat as SEMANTIC rather than silently benign.
        if !fired.any() && !jit_fired {
            return semantic(name, v_raw, r_raw, v_norm, r_norm, context, opts);
        }
        return FuncDiff {
            name,
            verdict: Verdict::Benign,
            fired,
            v_ops,
            r_ops,
            first_divergence: None,
            hint: None,
            window: None,
        };
    }

    semantic(name, v_raw, r_raw, v_norm, r_norm, context, opts)
}

/// Build the SEMANTIC-DIFF result with a localized divergence window.
fn semantic(
    name: String,
    v_raw: &[Instr],
    r_raw: &[Instr],
    v_norm: &[NormInstr],
    r_norm: &[NormInstr],
    context: usize,
    opts: &NormOpts,
) -> FuncDiff {
    let v_ops = v_raw.len();
    let r_ops = r_raw.len();
    // First diverging normalized index (element-wise up to the shorter length).
    let mut first = None;
    let n = v_norm.len().min(r_norm.len());
    for i in 0..n {
        if !v_norm[i].norm_eq(&r_norm[i]) {
            first = Some(i);
            break;
        }
    }
    if first.is_none() && v_norm.len() != r_norm.len() {
        first = Some(n); // divergence is the length mismatch at the tail
    }

    let hint = semantic_hint(v_norm, r_norm, first, opts);
    let window = render_window(v_norm, r_norm, first, context);

    FuncDiff {
        name,
        verdict: Verdict::Semantic,
        fired: NormFired::default(),
        v_ops,
        r_ops,
        first_divergence: first,
        hint: Some(hint),
        window: Some(window),
    }
}

/// Pattern-match the divergence against the spec §5 defect catalog for a fix-agent hint.
fn semantic_hint(
    v: &[NormInstr],
    r: &[NormInstr],
    first: Option<usize>,
    _opts: &NormOpts,
) -> String {
    // Force-stub (RVODEF): regen body is a single typed return.
    if r.len() <= 2 && v.len() > r.len() {
        return "regen body is near-empty (≤2 ops) — likely force-stub / default-return (#6)"
            .to_string();
    }
    // JZ/branch count inside a shorter regen ⇒ in-loop condition drop (emit_linear #1).
    let vj = v.iter().filter(|n| is_jump_op(n.op)).count();
    let rj = r.iter().filter(|n| is_jump_op(n.op)).count();
    if rj < vj {
        return format!(
            "regen has FEWER jumps ({rj} vs {vj}) — likely emit_linear in-loop condition drop (#1) \
             or dropped `if`"
        );
    }
    // Extra/missing null-guard around the divergence.
    if let Some(i) = first {
        let near = |list: &[NormInstr], idx: usize| -> bool {
            let lo = idx.saturating_sub(2);
            let hi = (idx + 2).min(list.len());
            list[lo..hi].iter().any(|n| {
                matches!(
                    n.op,
                    "CHKREF" | "ChkNullV" | "ChkNullS" | "CmpPtrNull" | "ChkRefS"
                )
            })
        };
        if near(v, i) != near(r, i.min(r.len())) {
            return "extra/missing null-guard (CHKREF/ChkNullV/CmpPtrNull) near divergence — \
                    classify SEMANTIC, human to decide (dup null-check is benign in some fns, the \
                    real breaker in others)"
                .to_string();
        }
        // Conversion-op mismatch (enum/int/float width, #2/#3).
        let is_conv = |n: &&NormInstr| {
            matches!(
                n.op,
                "sbTOi"
                    | "swTOi"
                    | "ubTOi"
                    | "uwTOi"
                    | "iTOb"
                    | "iTOw"
                    | "iTOf"
                    | "fTOi"
                    | "dTOf"
                    | "fTOd"
                    | "iTOd"
                    | "dTOi"
                    | "i64TOi"
                    | "iTOi64"
                    | "Cast"
            )
        };
        let vc = v.iter().filter(is_conv).count();
        let rc = r.iter().filter(is_conv).count();
        if vc != rc {
            return format!(
                "conversion-op count differs ({rc} vs {vc}) — likely enum↔int / float-width cast \
                 mismatch (#2/#3)"
            );
        }
    }
    // Arg-order swap heuristic: same op multiset but Push slot order differs before an aligned CALL.
    "residual divergence survived N1–N4 — inspect the window (candidate: arg-order swap #0, \
     cross-block carry #4, or struct-ctor grouping #5)"
        .to_string()
}

/// Render both sides' ±context window around the first divergence, operands as resolved names.
fn render_window(v: &[NormInstr], r: &[NormInstr], first: Option<usize>, context: usize) -> String {
    let center = first.unwrap_or(0);
    let lo = center.saturating_sub(context);
    let mut s = String::new();
    s.push_str("    vanilla:\n");
    render_side(&mut s, v, lo, center + context + 1);
    s.push_str("    regen:\n");
    render_side(&mut s, r, lo, center + context + 1);
    s
}

fn render_side(s: &mut String, list: &[NormInstr], lo: usize, hi: usize) {
    let hi = hi.min(list.len());
    for (i, ni) in list.iter().enumerate().take(hi).skip(lo) {
        let ops: Vec<String> = ni.operands.iter().map(render_operand).collect();
        s.push_str(&format!(
            "      [{i:04}] {:<14} {}\n",
            ni.op,
            ops.join(", ")
        ));
    }
    if lo >= list.len() {
        s.push_str("      <past end>\n");
    }
}

fn render_operand(op: &Operand) -> String {
    match op {
        Operand::Slot(s) => format!("v{s}"),
        Operand::Word(w) => format!("w{w}"),
        Operand::IntConst { value, width } => format!("i{width}:{value}"),
        Operand::FloatConst(bits) => {
            let f = f64::from_bits(*bits);
            format!("f64:{f}(0x{bits:x})")
        }
        Operand::JumpIndex(Some(i)) => format!("->[{i:04}]"),
        Operand::JumpIndex(None) => "->[??]".to_string(),
        Operand::Ref(id) => id.display(),
        Operand::TypeId(id) => render_type_id_identity(id),
        Operand::Property(property) => format!(
            "property(owner={},name={},old_type={})",
            render_type_id_identity(&property.owner),
            property.name,
            render_type_id_identity(&property.old_type)
        ),
        Operand::StaticName(Some(s)) => format!("n\"{s}\""),
        Operand::StaticName(None) => "n<?>".to_string(),
        Operand::OpCastTypeId => "opcast-typeid".to_string(),
        Operand::TypeIdentity(name) => format!("type {name}"),
        Operand::RawDw(d) => format!("0x{d:x}"),
        Operand::RawQw(q) => format!("0x{q:x}"),
    }
}

fn render_type_id_identity(id: &TypeIdIdentity) -> String {
    format!(
        "{}[kind={:#x},qual={:#x}]",
        id.identity.display(),
        id.object_kind,
        id.qualifiers
    )
}

/// Determine which normalizers actually collapsed a difference (for the BENIGN audit trail).
/// A normalizer is "credited" if turning it off would make the normalized lists differ. We
/// approximate this cheaply per operand-class by comparing raw operands vs normalized operands.
fn which_normalizers_fired(
    v_raw: &[Instr],
    r_raw: &[Instr],
    v_norm: &[NormInstr],
    r_norm: &[NormInstr],
    opts: &NormOpts,
    jit_fired: bool,
) -> NormFired {
    let mut fired = NormFired::default();
    // Only meaningful when normalized lists have equal length (they do — we're in the BENIGN arm).
    let n = v_norm.len().min(r_norm.len());
    for i in 0..n {
        for (vo, ro) in v_norm[i].operands.iter().zip(&r_norm[i].operands) {
            match (vo, ro) {
                (Operand::Ref(a), Operand::Ref(b)) if a == b => {
                    // A ref matched after N1. Did the RAW operand differ? If the raw dwords/qwords
                    // of these instructions differ, N1 was responsible.
                    if opts.n1_refs
                        && (v_raw[i].qwords != r_raw[i].qwords
                            || raw_id_differs(&v_raw[i], &r_raw[i]))
                    {
                        fired.n1_refs = true;
                    }
                }
                (Operand::TypeId(a), Operand::TypeId(b)) if a == b => {
                    if opts.n1_refs && v_raw[i].dwords != r_raw[i].dwords {
                        fired.n1_refs = true;
                    }
                }
                (Operand::Property(a), Operand::Property(b)) if a == b => {
                    if opts.n1_refs
                        && (v_raw[i].words != r_raw[i].words || v_raw[i].dwords != r_raw[i].dwords)
                    {
                        fired.n1_refs = true;
                    }
                }
                (Operand::JumpIndex(a), Operand::JumpIndex(b)) if a == b => {
                    if opts.n3_jumps && v_raw[i].dwords != r_raw[i].dwords {
                        fired.n3_jumps = true;
                    }
                }
                (Operand::FloatConst(a), Operand::FloatConst(b)) if a == b => {
                    if opts.n4_consts && v_raw[i].qwords != r_raw[i].qwords {
                        fired.n4_consts = true;
                    }
                }
                (Operand::IntConst { value: a, .. }, Operand::IntConst { value: b, .. })
                    if a == b => {}
                (Operand::StaticName(a), Operand::StaticName(b)) if a == b => {
                    if opts.n4_consts
                        && (v_raw[i].words != r_raw[i].words || v_raw[i].dwords != r_raw[i].dwords)
                    {
                        fired.n4_consts = true;
                    }
                }
                (Operand::OpCastTypeId, Operand::OpCastTypeId) => {
                    // GAP-C: a large runtime opCast type-id collapsed by N1 (a ref-identity
                    // refinement). Credit N1 when the raw type-id dword actually differed.
                    if opts.n1_refs && v_raw[i].dwords != r_raw[i].dwords {
                        fired.n1_refs = true;
                    }
                }
                (Operand::Slot(a), Operand::Slot(b)) if a == b => {
                    if opts.n2_slots && v_raw[i].words != r_raw[i].words {
                        fired.n2_slots = true;
                    }
                }
                _ => {}
            }
        }
    }
    // If N2 is on and any slot mapping was applied while raw words differ somewhere, credit it.
    fired.n2_slots = fired.n2_slots && opts.n2_slots;
    if jit_fired {
        // JitEntry stripping is a benign op-level tolerance; not one of N1–N4 but note nothing
        // extra (the report shows it via the fired labels being possibly empty + jit note).
    }
    fired
}

/// True if the func-id / type-id dword operand (dword 1 or 3) differs between two instructions —
/// a proxy for "N1 had work to do" on id-based refs (whose raw ids differ across builds).
fn raw_id_differs(a: &Instr, b: &Instr) -> bool {
    a.dwords != b.dwords
}

// =================================================================================================
// Alignment: modules by name, functions by DISPLAY name + name-resolved signature.
// =================================================================================================
//
// Function source = the WALKER (`collect_function_bytecodes`), which yields EVERY Func the spec
// §2.3 calls for (free fns, class methods, ctors, behavior fns, global-init fns) with the display
// name already composed (`module.Class::method`). We key alignment by the full display name PLUS
// the RESOLVED signature (param + ret base-names via each cache's RefResolver, so ptr differences
// don't break alignment; overloads distinguished by the param-type list). Module ONLY-IN-* is a
// separate name-set diff over `module_names`.

/// A function's alignment key: `display\x1fparam-base-types->ret#is_method`, types resolved to
/// NAMES via each cache's RefResolver so ptr differences don't break alignment (spec §2.3).
fn func_key(fc: &super::walk_modules::FuncCode, refs: &RefResolver) -> String {
    let params: Vec<String> = fc.param_types.iter().map(|p| p.base_name(refs)).collect();
    format!(
        "{}\u{1f}{}->{}#{}",
        fc.func,
        params.join(","),
        fc.ret.base_name(refs),
        fc.is_method as u8
    )
}

/// A whole-cache diff report.
#[derive(Debug, Default)]
pub struct Report {
    pub diffs: Vec<FuncDiff>,
    pub only_in_vanilla_modules: Vec<String>,
    pub only_in_regen_modules: Vec<String>,
    pub only_in_vanilla_funcs: Vec<String>,
    pub only_in_regen_funcs: Vec<String>,
}

impl Report {
    pub fn count(&self, v: Verdict) -> usize {
        self.diffs.iter().filter(|d| d.verdict == v).count()
    }
    pub fn any_semantic(&self) -> bool {
        self.diffs.iter().any(|d| d.verdict == Verdict::Semantic)
            || self.alignment_loss_count() != 0
    }

    /// Modules/functions which could not be aligned are semantic unknowns, never benign absence.
    /// A release gate must fail even when every remaining aligned function compares cleanly.
    pub fn alignment_loss_count(&self) -> usize {
        self.only_in_vanilla_modules.len()
            + self.only_in_regen_modules.len()
            + self.only_in_vanilla_funcs.len()
            + self.only_in_regen_funcs.len()
    }
}

/// Filters for which functions/modules to diff.
#[derive(Debug, Default, Clone)]
pub struct Filters {
    pub module: Option<String>,
    pub func: Option<String>,
}

/// Run the full bytediff over two caches. `context` = window size for SEMANTIC reports.
pub fn run(
    v_bytes: &[u8],
    r_bytes: &[u8],
    opts: &NormOpts,
    filters: &Filters,
    context: usize,
) -> anyhow::Result<Report> {
    let v_side = Side::build(v_bytes)?;
    let r_side = Side::build(r_bytes)?;

    let mut report = Report::default();

    // --- Module alignment by name (a dropped/added module is a defect signal). ---
    let v_modnames = super::walk_modules::module_names(v_bytes)?;
    let r_modnames = super::walk_modules::module_names(r_bytes)?;
    let v_modset: std::collections::HashSet<&str> = v_modnames.iter().map(|s| s.as_str()).collect();
    let r_modset: std::collections::HashSet<&str> = r_modnames.iter().map(|s| s.as_str()).collect();
    for m in &v_modnames {
        if !r_modset.contains(m.as_str()) {
            report.only_in_vanilla_modules.push(m.clone());
        }
    }
    for m in &r_modnames {
        if !v_modset.contains(m.as_str()) {
            report.only_in_regen_modules.push(m.clone());
        }
    }

    // --- Function alignment (WALKER: every Func the spec §2.3 lists). ---
    let v_fns = super::walk_modules::collect_function_bytecodes(v_bytes)?;
    let r_fns = super::walk_modules::collect_function_bytecodes(r_bytes)?;

    // Substring filters (module = a substring of the display prefix; func = substring of display).
    let want_v = |fc: &super::walk_modules::FuncCode| -> bool {
        filters
            .module
            .as_ref()
            .map_or(true, |m| fc.func.contains(m.as_str()))
            && filters
                .func
                .as_ref()
                .map_or(true, |f| fc.func.contains(f.as_str()))
    };

    // Index regen functions by alignment key (dup keys -> ordered Vec, consumed positionally so
    // N overloads on each side pair up 1:1).
    let mut r_index: HashMap<String, Vec<&super::walk_modules::FuncCode>> = HashMap::new();
    for fc in &r_fns {
        r_index
            .entry(func_key(fc, &r_side.refs))
            .or_default()
            .push(fc);
    }
    let mut r_used: HashMap<String, usize> = HashMap::new();

    for vfc in v_fns.iter().filter(|f| want_v(f)) {
        let key = func_key(vfc, &v_side.refs);
        let rfc = r_index.get(&key).and_then(|cs| {
            let used = r_used.entry(key.clone()).or_insert(0);
            let pick = cs.get(*used).copied();
            if pick.is_some() {
                *used += 1;
            }
            pick
        });
        match rfc {
            Some(rfc) => {
                let d = diff_one(
                    &vfc.func,
                    &vfc.bytecode,
                    &rfc.bytecode,
                    &v_side,
                    &r_side,
                    opts,
                    context,
                );
                report.diffs.push(d);
            }
            None => report.only_in_vanilla_funcs.push(vfc.func.clone()),
        }
    }

    // Regen functions with no vanilla counterpart (synthesized) — only when unfiltered, and count
    // only the regen overloads beyond what vanilla had for the same key.
    if filters.func.is_none() && filters.module.is_none() {
        let mut v_cap: HashMap<String, usize> = HashMap::new();
        for vfc in &v_fns {
            *v_cap.entry(func_key(vfc, &v_side.refs)).or_default() += 1;
        }
        let mut r_seen: HashMap<String, usize> = HashMap::new();
        for rfc in &r_fns {
            let key = func_key(rfc, &r_side.refs);
            let cap = v_cap.get(&key).copied().unwrap_or(0);
            let seen = r_seen.entry(key).or_insert(0);
            if *seen >= cap {
                report.only_in_regen_funcs.push(rfc.func.clone());
            }
            *seen += 1;
        }
    }

    Ok(report)
}

/// Diff one aligned function pair (disassemble both, normalize, classify). Disassembly failure on
/// either side is itself a SEMANTIC signal (a malformed body), never a crash.
fn diff_one(
    name: &str,
    v_code: &[i32],
    r_code: &[i32],
    v_side: &Side,
    r_side: &Side,
    opts: &NormOpts,
    context: usize,
) -> FuncDiff {
    let v_raw = match disassemble(v_code) {
        Ok(x) => x,
        Err(e) => return disasm_fail(name, "vanilla", e.to_string(), v_code.len(), r_code.len()),
    };
    let r_raw = match disassemble(r_code) {
        Ok(x) => x,
        Err(e) => return disasm_fail(name, "regen", e.to_string(), v_raw.len(), r_code.len()),
    };
    // N2 must run only AFTER N5/N6 remove instrumentation-only slots. Canonicalizing first-use
    // slot order before the strip irreversibly lets profiler temporaries perturb semantic locals;
    // the stronger reaching-definition proof likewise must see the retained CFG. Direct
    // normalization tests still exercise the cheap N2 path in `normalize`; the production pair
    // pipeline deliberately defers all slot-allocation proofs to `classify`.
    let mut prestrip_opts = *opts;
    prestrip_opts.n2_slots = false;
    let v_norm = normalize(v_code, &v_raw, v_side, &prestrip_opts);
    let r_norm = normalize(r_code, &r_raw, r_side, &prestrip_opts);
    classify(
        name.to_string(),
        &v_raw,
        &r_raw,
        &v_norm,
        &r_norm,
        opts,
        context,
    )
}

fn disasm_fail(name: &str, which: &str, err: String, v_ops: usize, r_ops: usize) -> FuncDiff {
    FuncDiff {
        name: name.to_string(),
        verdict: Verdict::Semantic,
        fired: NormFired::default(),
        v_ops,
        r_ops,
        first_divergence: None,
        hint: Some(format!("{which} bytecode failed to disassemble: {err}")),
        window: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Load a gitignored RE sample from the local scratch tree
    /// (`work/reversing/gore-as/samples`), or SKIP the calling test when it's absent — e.g. on CI,
    /// where `work/` isn't checked out. Mirrors `real_pair`'s `.ok()?` skip for the big samples so
    /// these fast gates still run locally without breaking a portable `cargo test`.
    macro_rules! sample_or_skip {
        ($name:expr) => {{
            let p = format!(
                "{}/../../work/reversing/gore-as/samples/{}",
                env!("CARGO_MANIFEST_DIR"),
                $name
            );
            match std::fs::read(&p) {
                Ok(bytes) => bytes,
                Err(_) => {
                    eprintln!("[skip] RE sample {} not present at {p}", $name);
                    return;
                }
            }
        }};
    }

    /// Self-identity on a small real cache: a cache vs itself must be PERFECTLY identical —
    /// every function IDENTICAL, 0 BENIGN, 0 SEMANTIC, no alignment loss. This is the correctness
    /// gate for N1/N3/N4.
    #[test]
    fn self_identity_richtest() {
        let b = sample_or_skip!("PrecompiledScript.richtest.Cache");
        let rep = run(&b, &b, &NormOpts::default(), &Filters::default(), 6).expect("run");
        assert_eq!(
            rep.count(Verdict::Benign),
            0,
            "self-diff must have 0 BENIGN"
        );
        assert_eq!(
            rep.count(Verdict::Semantic),
            0,
            "self-diff must have 0 SEMANTIC"
        );
        assert!(
            rep.only_in_vanilla_funcs.is_empty(),
            "no dropped fns in self-diff"
        );
        assert!(
            rep.only_in_regen_funcs.is_empty(),
            "no added fns in self-diff"
        );
        assert!(!rep.diffs.is_empty(), "richtest has at least one function");
        assert_eq!(rep.count(Verdict::Identical), rep.diffs.len());
    }

    /// Self-identity must ALSO hold with N2 (slot renumber) enabled — a cache vs itself has
    /// identical slot shapes, so N2 is a no-op that keeps everything IDENTICAL (raw-eq fast path
    /// actually short-circuits, but assert the N2 path doesn't regress).
    #[test]
    fn self_identity_richtest_with_n2() {
        let b = sample_or_skip!("PrecompiledScript.richtest.Cache");
        let mut opts = NormOpts::default();
        opts.n2_slots = true;
        let rep = run(&b, &b, &opts, &Filters::default(), 6).expect("run");
        assert_eq!(rep.count(Verdict::Semantic), 0);
        assert_eq!(rep.count(Verdict::Benign), 0);
        assert_eq!(rep.count(Verdict::Identical), rep.diffs.len());
    }

    #[test]
    fn self_identity_visproof() {
        let b = sample_or_skip!("PrecompiledScript.visproof.Cache");
        let rep = run(&b, &b, &NormOpts::default(), &Filters::default(), 6).expect("run");
        assert_eq!(rep.count(Verdict::Semantic), 0);
        assert_eq!(rep.count(Verdict::Benign), 0);
        assert_eq!(rep.count(Verdict::Identical), rep.diffs.len());
    }

    // ---- Synthetic-bytecode normalizer unit tests (N3 jumps, N4 consts, N2 slots) ----

    /// Encode a NO_ARG op (opcode in byte 0 of one dword).
    fn no_arg(opcode: u8) -> Vec<i32> {
        vec![opcode as i32]
    }
    /// Encode a DW_ARG op: opcode dword + 32-bit arg dword.
    fn dw_arg(opcode: u8, arg: i32) -> Vec<i32> {
        vec![opcode as i32, arg]
    }
    /// Encode an rW_ARG op: opcode | (slot in the high word of dword 0).
    fn rw_arg(opcode: u8, slot: u16) -> Vec<i32> {
        vec![opcode as i32 | ((slot as i32) << 16)]
    }
    /// Encode a wW_rW_ARG op: destination in dword-0 high word, source in dword-1 low word.
    fn ww_rw_arg(opcode: u8, dst: u16, src: u16) -> Vec<i32> {
        vec![opcode as i32 | ((dst as i32) << 16), src as i32]
    }
    /// Encode an rW_DW_ARG op (ordinary comparisons/JMPP).
    fn rw_dw_arg(opcode: u8, slot: u16, arg: i32) -> Vec<i32> {
        vec![opcode as i32 | ((slot as i32) << 16), arg]
    }
    /// Encode a W_DW_ARG op (`ADDSi` / `LoadThisR`): member offset + owner type-id.
    fn w_dw_arg(opcode: u8, word: u16, arg: i32) -> Vec<i32> {
        vec![opcode as i32 | ((word as i32) << 16), arg]
    }
    /// Encode an rW_W_DW_ARG op (`LoadRObjR` / `LoadVObjR`): slot + offset + owner type-id.
    fn rw_w_dw_arg(opcode: u8, slot: u16, word: u16, arg: i32) -> Vec<i32> {
        vec![opcode as i32 | ((slot as i32) << 16), word as i32, arg]
    }
    /// Encode a QW_ARG op: opcode dword + 64-bit arg (2 dwords LE).
    fn qw_arg(opcode: u8, arg: u64) -> Vec<i32> {
        vec![opcode as i32, arg as u32 as i32, (arg >> 32) as u32 as i32]
    }

    // opcodes (from isa.rs): JMP=11, JZ=12, SUSPEND=63, RET=10 (W_ARG),
    // PshC8=47 (QW), PshV4=3 (rW), PopPtr=0 (NO_ARG), TZ=18 (NO_ARG).

    /// Build a `Side` scaffold from the richtest sample, or SKIP the calling test when it's absent.
    /// N3/N4/N2 don't need real tail tables; a tiny sample gives a valid RefResolver/RefIdentity.
    macro_rules! side_or_skip {
        () => {{
            let b = sample_or_skip!("PrecompiledScript.richtest.Cache");
            Side::build(&b).expect("side")
        }};
    }

    /// Build a normalized instruction list from raw bytecode (test helper).
    fn norm(code: &[i32], side: &Side, opts: &NormOpts) -> Vec<NormInstr> {
        let raw = disassemble(code).expect("disasm");
        normalize(code, &raw, side, opts)
    }

    fn push_sia(out: &mut Vec<u8>, value: &str) {
        if value.is_empty() {
            out.extend_from_slice(&0i32.to_le_bytes());
        } else {
            out.extend_from_slice(&(value.len() as i32).to_le_bytes());
            out.extend_from_slice(value.as_bytes());
            out.push(0);
        }
    }

    /// Build a zero-module cache carrying only the T1/T2/T7 rows needed by the N1 tests.
    /// Each type tuple is `(type-id, ptr, name, module, namespace)`; each property tuple is
    /// `(owner type-id, member offset, property name, OldTypeId)`.
    fn n1_side(
        types: &[(i32, i64, &str, &str, &str)],
        properties: &[(i32, i32, &str, i32)],
    ) -> Side {
        n1_side_with_functions(types, properties, &[])
    }

    fn n1_side_with_functions(
        types: &[(i32, i64, &str, &str, &str)],
        properties: &[(i32, i32, &str, i32)],
        functions: &[(i64, &str)],
    ) -> Side {
        use super::super::header::{CacheHeader, CACHE_MAGIC};

        let mut cache = vec![0u8; CacheHeader::SIZE];
        cache[0x10..0x14].copy_from_slice(&CACHE_MAGIC.to_le_bytes());
        // Module count at 0x14 stays zero, so the seven tail tables begin immediately at 0x18.
        cache.extend_from_slice(&(types.len() as i32).to_le_bytes());
        for &(_, ptr, name, module, namespace) in types {
            cache.extend_from_slice(&ptr.to_le_bytes());
            push_sia(&mut cache, name);
            push_sia(&mut cache, module);
            push_sia(&mut cache, namespace);
            cache.extend_from_slice(&0i32.to_le_bytes()); // no template subtypes
        }
        cache.extend_from_slice(&(types.len() as i32).to_le_bytes());
        for &(type_id, ptr, _, _, _) in types {
            cache.extend_from_slice(&type_id.to_le_bytes());
            cache.extend_from_slice(&ptr.to_le_bytes());
        }
        cache.extend_from_slice(&(functions.len() as i32).to_le_bytes());
        for &(ptr, name) in functions {
            cache.extend_from_slice(&ptr.to_le_bytes());
            push_sia(&mut cache, name);
            push_sia(&mut cache, ""); // module
            push_sia(&mut cache, ""); // namespace
            cache.extend_from_slice(&0i32.to_le_bytes()); // bIsConst
            cache.extend_from_slice(&0i32.to_le_bytes()); // bIsImportedDecl
            cache.extend_from_slice(&0i32.to_le_bytes()); // bIsMethod
            cache.extend_from_slice(&0i64.to_le_bytes()); // ObjectType
            cache.extend_from_slice(&0i32.to_le_bytes()); // no params
            for _ in 0..6 {
                cache.extend_from_slice(&0i32.to_le_bytes()); // return DataType flags
            }
            cache.extend_from_slice(&0i64.to_le_bytes()); // return TypeInfo
            cache.extend_from_slice(&0x52i32.to_le_bytes()); // return void token
        }
        cache.extend_from_slice(&0i32.to_le_bytes()); // T4 FunctionIdReferenceToPointer
        cache.extend_from_slice(&0i32.to_le_bytes()); // T5 GlobalReferences
        cache.extend_from_slice(&0i32.to_le_bytes()); // T6 StaticNames
        cache.extend_from_slice(&(properties.len() as i32).to_le_bytes());
        for &(owner, offset, name, old_type_id) in properties {
            let key = ((owner as i64) << 1) | ((offset as i64) << 33) | 1;
            cache.extend_from_slice(&key.to_le_bytes());
            push_sia(&mut cache, name);
            cache.extend_from_slice(&old_type_id.to_le_bytes());
        }
        Side::build(&cache).expect("synthetic N1 side")
    }

    #[test]
    fn n1_unresolved_runtime_type_id_collapses_only_at_resolved_opcast() {
        const OPCAST_A: i64 = 0x7010;
        const OPCAST_B: i64 = 0x7020;
        let a = n1_side_with_functions(&[], &[], &[(OPCAST_A, "opCast")]);
        let b = n1_side_with_functions(&[], &[], &[(OPCAST_B, "opCast")]);
        let big_a = 0x4800_3464u32 as i32;
        let big_b = 0x4800_3443u32 as i32;
        let mut left_code = dw_arg(76, big_a);
        left_code.extend(qw_arg(61, OPCAST_A as u64));
        let mut right_code = dw_arg(76, big_b);
        right_code.extend(qw_arg(61, OPCAST_B as u64));
        let diff = diff_one(
            "opcast-gate",
            &left_code,
            &right_code,
            &a,
            &b,
            &NormOpts::default(),
            1,
        );
        assert_eq!(diff.verdict, Verdict::Benign);
        assert!(diff.fired.n1_refs);

        let lone_a = norm(&dw_arg(76, big_a), &a, &NormOpts::default());
        let lone_b = norm(&dw_arg(76, big_b), &b, &NormOpts::default());
        assert!(matches!(lone_a[0].operands[0], Operand::RawDw(_)));
        assert!(matches!(lone_b[0].operands[0], Operand::RawDw(_)));
        assert!(!lone_a[0].norm_eq(&lone_b[0]));
    }

    #[test]
    fn n1_type_id_splits_core_and_preserves_qualifier_and_object_kind() {
        const APP_A: i32 = 0x0400_0100;
        const APP_B: i32 = 0x0400_0200;
        const SCRIPT_B: i32 = 0x0800_0200;
        const HANDLE: i32 = 0x4000_0000;
        const HANDLE_CONST: i32 = 0x6000_0000;

        let a = n1_side(&[(APP_A, 0x1010, "FShared", "Core", "G1R")], &[]);
        let b = n1_side(&[(APP_B, 0x2020, "FShared", "Core", "G1R")], &[]);
        let b_other_kind = n1_side(&[(SCRIPT_B, 0x3030, "FShared", "Core", "G1R")], &[]);
        let opts = NormOpts::default();

        let left = norm(&dw_arg(76, APP_A | HANDLE), &a, &opts);
        let same = norm(&dw_arg(76, APP_B | HANDLE), &b, &opts);
        assert!(
            left[0].norm_eq(&same[0]),
            "same T1 + qualifier + object-kind must survive T2 allocation drift"
        );
        assert!(matches!(left[0].operands[0], Operand::TypeId(_)));
        let benign = diff_one(
            "type-id-drift",
            &dw_arg(76, APP_A | HANDLE),
            &dw_arg(76, APP_B | HANDLE),
            &a,
            &b,
            &opts,
            1,
        );
        assert_eq!(benign.verdict, Verdict::Benign);
        assert!(benign.fired.n1_refs);

        let other_qualifier = norm(&dw_arg(76, APP_B | HANDLE_CONST), &b, &opts);
        assert!(
            !left[0].norm_eq(&other_qualifier[0]),
            "handle-to-const is semantic even when the complete T1 identity matches"
        );

        let other_kind = norm(&dw_arg(76, SCRIPT_B | HANDLE), &b_other_kind, &opts);
        assert!(
            !left[0].norm_eq(&other_kind[0]),
            "application/script object-kind aliases must not collapse"
        );
    }

    #[test]
    fn n1_type_id_unknown_flags_and_unresolved_links_stay_raw() {
        const TYPE_A: i32 = 0x0400_0100;
        const TYPE_B: i32 = 0x0400_0200;
        let a = n1_side(&[(TYPE_A, 0x1010, "FShared", "Core", "G1R")], &[]);
        let b = n1_side(&[(TYPE_B, 0x2020, "FShared", "Core", "G1R")], &[]);
        let empty = n1_side(&[], &[]);
        let opts = NormOpts::default();

        let unknown_a = norm(&dw_arg(76, (TYPE_A as u32 | 0x8000_0000) as i32), &a, &opts);
        let unknown_b = norm(&dw_arg(76, (TYPE_B as u32 | 0x8000_0000) as i32), &b, &opts);
        assert!(matches!(unknown_a[0].operands[0], Operand::RawDw(_)));
        assert!(
            !unknown_a[0].norm_eq(&unknown_b[0]),
            "an unknown high flag must retain the exact wire id"
        );

        let const_without_handle = norm(&dw_arg(76, TYPE_A | 0x2000_0000), &a, &opts);
        assert!(
            matches!(const_without_handle[0].operands[0], Operand::RawDw(_)),
            "HANDLETOCONST without OBJHANDLE is an invalid qualifier combination"
        );

        let unresolved_a = norm(&dw_arg(76, TYPE_A | 0x4000_0000), &empty, &opts);
        let unresolved_b = norm(&dw_arg(76, TYPE_B | 0x4000_0000), &empty, &opts);
        assert!(matches!(unresolved_a[0].operands[0], Operand::RawDw(_)));
        assert!(
            !unresolved_a[0].norm_eq(&unresolved_b[0]),
            "a missing T2/T1 link must not turn allocation drift benign"
        );
    }

    #[test]
    fn n1_property_identity_accepts_only_complete_equal_t1_t7_chain() {
        const OWNER_A: i32 = 0x0400_0100;
        const VALUE_A: i32 = 0x0400_0110;
        const OWNER_B: i32 = 0x0400_0200;
        const VALUE_B: i32 = 0x0400_0210;
        let a = n1_side(
            &[
                (OWNER_A, 0x1010, "AOwner", "Story", "G1R"),
                (VALUE_A, 0x1110, "FValue", "Core", "G1R"),
            ],
            &[(OWNER_A, 8, "Field", VALUE_A)],
        );
        let b = n1_side(
            &[
                (OWNER_B, 0x2020, "AOwner", "Story", "G1R"),
                (VALUE_B, 0x2120, "FValue", "Core", "G1R"),
            ],
            &[(OWNER_B, 12, "Field", VALUE_B)],
        );
        let opts = NormOpts::default();
        for opcode in [79, 178] {
            let left = norm(&w_dw_arg(opcode, 8, OWNER_A), &a, &opts);
            let right = norm(&w_dw_arg(opcode, 12, OWNER_B), &b, &opts);
            assert!(
                left[0].norm_eq(&right[0]),
                "{} offset/type allocation drift must be benign for an exact property identity",
                left[0].op
            );
            assert!(matches!(left[0].operands[0], Operand::Property(_)));
            let benign = diff_one(
                "property-drift",
                &w_dw_arg(opcode, 8, OWNER_A),
                &w_dw_arg(opcode, 12, OWNER_B),
                &a,
                &b,
                &opts,
                1,
            );
            assert_eq!(benign.verdict, Verdict::Benign);
            assert!(benign.fired.n1_refs);
        }
        for opcode in [184, 185] {
            let left = norm(&rw_w_dw_arg(opcode, 3, 8, OWNER_A), &a, &opts);
            let right = norm(&rw_w_dw_arg(opcode, 3, 12, OWNER_B), &b, &opts);
            assert!(
                left[0].norm_eq(&right[0]),
                "{} property identity must cover its rW_W_DW form",
                left[0].op
            );
            assert_eq!(left[0].operands.len(), 2);
            assert_eq!(left[0].operands[0], Operand::Slot(3));
            assert!(matches!(left[0].operands[1], Operand::Property(_)));
            let benign = diff_one(
                "property-drift",
                &rw_w_dw_arg(opcode, 3, 8, OWNER_A),
                &rw_w_dw_arg(opcode, 3, 12, OWNER_B),
                &a,
                &b,
                &opts,
                1,
            );
            assert_eq!(benign.verdict, Verdict::Benign);
            assert!(benign.fired.n1_refs);

            let different_slot = norm(&rw_w_dw_arg(opcode, 4, 12, OWNER_B), &b, &opts);
            assert!(
                !left[0].norm_eq(&different_slot[0]),
                "{} leading slot remains semantic with N2 disabled",
                left[0].op
            );
        }
    }

    #[test]
    fn n1_property_accepts_primitive_old_type_id() {
        const OWNER_A: i32 = 0x0400_0100;
        const OWNER_B: i32 = 0x0400_0200;
        const BOOL_TYPE_ID: i32 = 0x41;
        let a = n1_side(
            &[(OWNER_A, 0x1010, "AOwner", "Story", "G1R")],
            &[(OWNER_A, 8, "Enabled", BOOL_TYPE_ID)],
        );
        let b = n1_side(
            &[(OWNER_B, 0x2020, "AOwner", "Story", "G1R")],
            &[(OWNER_B, 12, "Enabled", BOOL_TYPE_ID)],
        );
        let opts = NormOpts::default();
        let left = norm(&w_dw_arg(79, 8, OWNER_A), &a, &opts);
        let right = norm(&w_dw_arg(79, 12, OWNER_B), &b, &opts);
        assert!(left[0].norm_eq(&right[0]));
        let Operand::Property(property) = &left[0].operands[0] else {
            panic!("primitive OldTypeId must still form a resolved property identity");
        };
        assert!(matches!(
            &property.old_type.identity,
            OperandId::Primitive(id) if *id == BOOL_TYPE_ID
        ));
        assert_eq!(property.old_type.object_kind, 0);
        assert_eq!(property.old_type.qualifiers, 0);
    }

    #[test]
    fn n1_property_same_name_different_owner_or_old_type_stays_semantic() {
        const OWNER_A: i32 = 0x0400_0100;
        const VALUE_A: i32 = 0x0400_0110;
        const OWNER_B: i32 = 0x0400_0200;
        const VALUE_B: i32 = 0x0400_0210;
        let baseline = n1_side(
            &[
                (OWNER_A, 0x1010, "AOwner", "Story", "G1R"),
                (VALUE_A, 0x1110, "FValue", "Core", "G1R"),
            ],
            &[(OWNER_A, 8, "Field", VALUE_A)],
        );
        let different_owner = n1_side(
            &[
                (OWNER_B, 0x2020, "AOtherOwner", "Story", "G1R"),
                (VALUE_B, 0x2120, "FValue", "Core", "G1R"),
            ],
            &[(OWNER_B, 12, "Field", VALUE_B)],
        );
        let different_old_type = n1_side(
            &[
                (OWNER_B, 0x2020, "AOwner", "Story", "G1R"),
                (VALUE_B, 0x2120, "FOtherValue", "Core", "G1R"),
            ],
            &[(OWNER_B, 12, "Field", VALUE_B)],
        );
        let opts = NormOpts::default();
        let left = norm(&w_dw_arg(79, 8, OWNER_A), &baseline, &opts);
        let owner_diff = norm(&w_dw_arg(79, 12, OWNER_B), &different_owner, &opts);
        let old_type_diff = norm(&w_dw_arg(79, 12, OWNER_B), &different_old_type, &opts);
        assert!(
            !left[0].norm_eq(&owner_diff[0]),
            "a same-named property on another complete owner identity is semantic"
        );
        assert!(
            !left[0].norm_eq(&old_type_diff[0]),
            "a same-named property with another OldTypeId T1 identity is semantic"
        );
    }

    #[test]
    fn n1_property_unresolved_site_keeps_both_operands_raw() {
        const OWNER_A: i32 = 0x0400_0100;
        const OWNER_B: i32 = 0x0400_0200;
        let a = n1_side(&[(OWNER_A, 0x1010, "AOwner", "Story", "G1R")], &[]);
        let b = n1_side(&[(OWNER_B, 0x2020, "AOwner", "Story", "G1R")], &[]);
        let opts = NormOpts::default();
        let left = norm(&w_dw_arg(79, 8, OWNER_A), &a, &opts);
        let right = norm(&w_dw_arg(79, 12, OWNER_B), &b, &opts);
        assert_eq!(
            left[0].operands,
            [Operand::Word(8), Operand::RawDw(OWNER_A as u32)]
        );
        assert_eq!(
            right[0].operands,
            [Operand::Word(12), Operand::RawDw(OWNER_B as u32)]
        );
        assert!(
            !left[0].norm_eq(&right[0]),
            "without T7/OldTypeId proof, even matching owner T1 identities stay semantic"
        );
    }

    #[test]
    fn n1_property_duplicate_t7_key_is_always_raw() {
        const OWNER: i32 = 0x0400_0100;
        const VALUE: i32 = 0x0400_0110;
        let types = [
            (OWNER, 0x1010, "AOwner", "Story", "G1R"),
            (VALUE, 0x1110, "FValue", "Core", "G1R"),
        ];
        let identical_duplicate = n1_side(
            &types,
            &[(OWNER, 8, "Field", VALUE), (OWNER, 8, "Field", VALUE)],
        );
        let conflicting_duplicate = n1_side(
            &types,
            &[(OWNER, 8, "Field", VALUE), (OWNER, 8, "OtherField", OWNER)],
        );
        let opts = NormOpts::default();
        for side in [&identical_duplicate, &conflicting_duplicate] {
            let normalized = norm(&w_dw_arg(79, 8, OWNER), side, &opts);
            assert_eq!(
                normalized[0].operands,
                [Operand::Word(8), Operand::RawDw(OWNER as u32)],
                "every duplicate T7 key must fail closed, even when both rows are identical"
            );
        }
    }

    #[test]
    fn n1_property_render_includes_owner_and_old_type_flags() {
        const OWNER: i32 = 0x0400_0100;
        const VALUE: i32 = 0x0800_0110;
        const HANDLE_CONST: i32 = 0x6000_0000;
        let side = n1_side(
            &[
                (OWNER, 0x1010, "AOwner", "Story", "G1R"),
                (VALUE, 0x1110, "FValue", "Core", "G1R"),
            ],
            &[(OWNER, 8, "Field", VALUE | HANDLE_CONST)],
        );
        let normalized = norm(&w_dw_arg(79, 8, OWNER), &side, &NormOpts::default());
        let rendered = render_operand(&normalized[0].operands[0]);
        assert!(rendered.contains("owner="), "{rendered}");
        assert!(rendered.contains("old_type="), "{rendered}");
        assert!(rendered.contains("kind=0x4000000,qual=0x0"), "{rendered}");
        assert!(
            rendered.contains("kind=0x8000000,qual=0x60000000"),
            "{rendered}"
        );
    }

    #[test]
    fn n1_property_semantic_report_exposes_flag_drift() {
        const OWNER_A: i32 = 0x0400_0100;
        const VALUE_A: i32 = 0x0800_0110;
        const OWNER_B: i32 = 0x0400_0200;
        const VALUE_B: i32 = 0x0800_0210;
        let a = n1_side(
            &[
                (OWNER_A, 0x1010, "AOwner", "Story", "G1R"),
                (VALUE_A, 0x1110, "FValue", "Core", "G1R"),
            ],
            &[(OWNER_A, 8, "Field", VALUE_A | 0x4000_0000)],
        );
        let b = n1_side(
            &[
                (OWNER_B, 0x2020, "AOwner", "Story", "G1R"),
                (VALUE_B, 0x2120, "FValue", "Core", "G1R"),
            ],
            &[(OWNER_B, 12, "Field", VALUE_B | 0x6000_0000)],
        );
        let diff = diff_one(
            "flag-drift",
            &w_dw_arg(79, 8, OWNER_A),
            &w_dw_arg(79, 12, OWNER_B),
            &a,
            &b,
            &NormOpts::default(),
            1,
        );
        assert_eq!(diff.verdict, Verdict::Semantic);
        let window = diff.window.expect("semantic diff has a rendered window");
        assert!(window.contains("owner="), "{window}");
        assert!(window.contains("old_type="), "{window}");
        assert!(window.contains("qual=0x40000000"), "{window}");
        assert!(window.contains("qual=0x60000000"), "{window}");
    }

    /// Minimal normalized companion for N2b unit tests. These streams contain only slot operands
    /// plus ordinary jumps, so no cache-backed ref resolver is needed.
    fn slot_norm(raw: &[Instr]) -> Vec<NormInstr> {
        let offsets: HashMap<usize, usize> = raw
            .iter()
            .enumerate()
            .map(|(i, ins)| (ins.offset_dw, i))
            .collect();
        raw.iter()
            .map(|ins| {
                let mut operands = Vec::new();
                push_word_operands(ins.op.name, ins, &mut operands);
                if is_jump_op(ins.op.name) {
                    let target = ins.dwords.first().and_then(|raw_rel| {
                        let target_dw = ins.offset_dw as i64
                            + ins.op.size_dwords as i64
                            + (*raw_rel as i32 as i64);
                        (target_dw >= 0)
                            .then_some(target_dw as usize)
                            .and_then(|dw| offsets.get(&dw).copied())
                    });
                    operands.push(Operand::JumpIndex(target));
                }
                NormInstr {
                    op: ins.op.name,
                    operands,
                }
            })
            .collect()
    }

    /// N3: two functions with the SAME instruction COUNT but where an earlier instruction has a
    /// different SIZE (a benign const-width change: PshC4 2dw vs PshC8 3dw) — the JMP's absolute
    /// byte(dword) offset shifts, but its target INSTRUCTION INDEX is the same, so N3 makes the
    /// jump operand compare EQUAL (the control-flow edge is preserved).
    #[test]
    fn n3_jump_index_equal_despite_offset_shift() {
        let side = side_or_skip!();
        let opts = NormOpts::default();
        // Layout (both sides, 4 instructions): [const][JMP -> RET][TZ][RET].
        // Side A uses PshC4 (2 dwords); Side B uses PshC8 (3 dwords) — one dword bigger. The JMP's
        // raw offset differs (same index distance, but computed the same relative way), yet both
        // resolve to the RET at instruction index 3.
        // A: PshC4@dw0(2) JMP@dw2(2,off1)->dw(2+2+1)=dw5 TZ@dw4? wait recompute below.
        // Compute A: PshC4 dw0..1; JMP dw2..3; TZ dw4; RET dw5. JMP target index 3 (RET) => off =
        //   target_dw(5) - (jmp_dw(2)+size(2)) = 1.
        let mut a = Vec::new();
        a.extend(dw_arg(2, 7)); // PshC4 7 @ dw0 (2 dwords)
        a.extend(dw_arg(11, 1)); // JMP @ dw2 size2 off1 -> dw5 = RET (index 3)
        a.extend(no_arg(18)); // TZ @ dw4
        a.push(10); // RET @ dw5
                    // B: PshC8 dw0..2 (3 dwords); JMP dw3..4; TZ dw5; RET dw6. off = 6 - (3+2) = 1.
        let mut b = Vec::new();
        b.extend(qw_arg(47, 7)); // PshC8 7 @ dw0 (3 dwords)
        b.extend(dw_arg(11, 1)); // JMP @ dw3 size2 off1 -> dw6 = RET (index 3)
        b.extend(no_arg(18)); // TZ @ dw5
        b.push(10); // RET @ dw6
        let na = norm(&a, &side, &opts);
        let nb = norm(&b, &side, &opts);
        assert_eq!(na.len(), 4);
        assert_eq!(nb.len(), 4);
        let a_jmp = &na[1];
        let b_jmp = &nb[1];
        assert_eq!(a_jmp.op, "JMP");
        assert_eq!(b_jmp.op, "JMP");
        // Both jumps target the RET at instruction index 3 — equal despite different raw offset
        // origin (A off computed from dw2, B from dw3).
        assert_eq!(a_jmp.operands[0], Operand::JumpIndex(Some(3)));
        assert_eq!(b_jmp.operands[0], Operand::JumpIndex(Some(3)));
        assert_eq!(
            a_jmp.operands, b_jmp.operands,
            "jump edge preserved across size shift"
        );
    }

    /// N3 guard: a jump whose target INDEX differs (branches to a structurally different
    /// instruction) stays a real difference — the operands are NOT equal.
    #[test]
    fn n3_jump_to_different_index_differs() {
        let side = side_or_skip!();
        let opts = NormOpts::default();
        // A: JMP skips 0 ops (targets the very next). B: JMP skips 1 op (targets one later).
        let mut a = Vec::new();
        a.extend(dw_arg(11, 0)); // JMP @ dw0 offset0 -> dw2
        a.extend(no_arg(18)); // TZ @ dw2
        a.push(10); // RET @ dw3
        let mut b = Vec::new();
        b.extend(dw_arg(11, 1)); // JMP @ dw0 offset1 -> dw3 (skips the TZ)
        b.extend(no_arg(18)); // TZ @ dw2
        b.push(10); // RET @ dw3
        let na = norm(&a, &side, &opts);
        let nb = norm(&b, &side, &opts);
        assert_ne!(
            na[0].operands, nb[0].operands,
            "different jump target index must differ"
        );
    }

    /// N4: a 64-bit float constant compares by DECODED value (bit pattern). Same value => equal;
    /// a genuinely different value => not equal. (Same-value floats already share bits, so this
    /// mainly asserts we decode + compare the qword, not a raw pass-through that could mis-handle.)
    #[test]
    fn n4_float_const_by_value() {
        let side = side_or_skip!();
        let opts = NormOpts::default();
        let same_a = norm(&qw_arg(47, 3.5f64.to_bits()), &side, &opts);
        let same_b = norm(&qw_arg(47, 3.5f64.to_bits()), &side, &opts);
        assert_eq!(same_a[0].operands, same_b[0].operands);
        assert_eq!(same_a[0].operands[0], Operand::FloatConst(3.5f64.to_bits()));
        let diff = norm(&qw_arg(47, 4.5f64.to_bits()), &side, &opts);
        assert_ne!(
            same_a[0].operands, diff[0].operands,
            "different float value must differ"
        );
    }

    /// N4: a 32-bit integer immediate (PshC4=2, DW_ARG) compares by decoded value.
    #[test]
    fn n4_int_const_by_value() {
        let side = side_or_skip!();
        let opts = NormOpts::default();
        let a = norm(&dw_arg(2, 42), &side, &opts);
        let b = norm(&dw_arg(2, 42), &side, &opts);
        assert_eq!(a[0].operands, b[0].operands);
        assert_eq!(
            a[0].operands[0],
            Operand::IntConst {
                value: 42,
                width: 4
            }
        );
        let c = norm(&dw_arg(2, 43), &side, &opts);
        assert_ne!(a[0].operands, c[0].operands);
    }

    /// N2 (opt-in): two streams that use DIFFERENT raw slot numbers but in the SAME first-use
    /// order canonicalize to identical ordinals. With N2 OFF they differ; with N2 ON they match.
    #[test]
    fn n2_slot_renumber_first_use() {
        let side = side_or_skip!();
        // A uses slots 5 then 8; B uses slots 2 then 9 — same first-use ORDER.
        let mut a = Vec::new();
        a.extend(rw_arg(3, 5)); // PshV4 v5
        a.extend(rw_arg(3, 8)); // PshV4 v8
        let mut b = Vec::new();
        b.extend(rw_arg(3, 2)); // PshV4 v2
        b.extend(rw_arg(3, 9)); // PshV4 v9

        let off = NormOpts::default();
        let na = norm(&a, &side, &off);
        let nb = norm(&b, &side, &off);
        assert_ne!(
            na[0].operands, nb[0].operands,
            "raw slots differ with N2 off"
        );

        let mut on = NormOpts::default();
        on.n2_slots = true;
        let na = norm(&a, &side, &on);
        let nb = norm(&b, &side, &on);
        assert_eq!(
            na[0].operands, nb[0].operands,
            "first-use order maps to same ordinal"
        );
        assert_eq!(na[1].operands, nb[1].operands);
        assert_eq!(na[0].operands[0], Operand::Slot(0));
        assert_eq!(na[1].operands[0], Operand::Slot(1));
    }

    #[test]
    fn n2_dead_value_copy_coalesces_only_qualified_producers() {
        // The first two producers write only a destination; the four integer narrowers also read
        // an independent input. All six are the exact residue shapes qualified on the real cache.
        for (name, opcode, has_input) in [
            ("CpyRtoV4", 85, false),
            ("RDR4", 94, false),
            ("sbTOi", 105, true),
            ("swTOi", 106, true),
            ("ubTOi", 107, true),
            ("uwTOi", 108, true),
        ] {
            let mut code = if has_input {
                ww_rw_arg(opcode, 5, 7)
            } else {
                rw_arg(opcode, 5)
            };
            code.extend(ww_rw_arg(80, 1, 5)); // CpyVtoV4 local_1, temp_5
            code.extend(rw_arg(3, 1)); // PshV4 local_1
            let raw = disassemble(&code).expect("disasm");
            let mut normalized = slot_norm(&raw);
            assert_eq!(
                coalesce_dead_value_copies(&mut normalized, &raw),
                1,
                "{name} must coalesce"
            );
            assert_eq!(normalized.len(), 2, "{name}: copy not removed");
            assert_eq!(normalized[0].op, name);
            assert_eq!(normalized[0].operands[0], Operand::Slot(1));
            assert_eq!(normalized[1].operands[0], Operand::Slot(1));
        }
    }

    #[test]
    fn n2_dead_value_copy_rebases_retained_jump_target() {
        let mut code = rw_arg(94, 5); // RDR4 temp_5
        code.extend(ww_rw_arg(80, 1, 5)); // copy at index 1
        code.extend(dw_arg(11, 0)); // JMP index 2 -> PshV4 index 3
        code.extend(rw_arg(3, 1));
        let raw = disassemble(&code).expect("disasm");
        let mut normalized = slot_norm(&raw);
        assert_eq!(normalized[2].operands, [Operand::JumpIndex(Some(3))]);
        assert_eq!(coalesce_dead_value_copies(&mut normalized, &raw), 1);
        assert_eq!(normalized.len(), 3);
        assert_eq!(normalized[1].op, "JMP");
        assert_eq!(normalized[1].operands, [Operand::JumpIndex(Some(2))]);
    }

    #[test]
    fn n2_dead_value_copy_rejects_live_source_and_producer_target_read() {
        let mut live_code = rw_arg(94, 5);
        live_code.extend(ww_rw_arg(80, 1, 5));
        live_code.extend(rw_arg(3, 5)); // temp_5 remains live after the copy
        let live_raw = disassemble(&live_code).expect("disasm");
        let mut live_norm = slot_norm(&live_raw);
        assert_eq!(coalesce_dead_value_copies(&mut live_norm, &live_raw), 0);
        assert_eq!(live_norm.len(), live_raw.len());

        let mut read_code = ww_rw_arg(105, 5, 1); // sbTOi temp_5, local_1
        read_code.extend(ww_rw_arg(80, 1, 5));
        read_code.extend(rw_arg(3, 1));
        let read_raw = disassemble(&read_code).expect("disasm");
        let mut read_norm = slot_norm(&read_raw);
        assert_eq!(coalesce_dead_value_copies(&mut read_norm, &read_raw), 0);
        assert_eq!(read_norm.len(), read_raw.len());

        let mut free_code = rw_arg(94, 5);
        free_code.extend(ww_rw_arg(80, 1, 5));
        free_code.extend([65 | (5 << 16), 0, 0]); // FREE reads/releases then clears temp_5
        let free_raw = disassemble(&free_code).expect("disasm");
        let mut free_norm = slot_norm(&free_raw);
        assert_eq!(coalesce_dead_value_copies(&mut free_norm, &free_raw), 0);
        assert_eq!(free_norm.len(), free_raw.len());
    }

    #[test]
    fn n2_dead_value_copy_rejects_branch_into_copy() {
        let mut code = dw_arg(11, 1); // JMP index 0 -> copy at index 2
        code.extend(rw_arg(94, 5)); // producer at index 1 (skipped by the branch)
        code.extend(ww_rw_arg(80, 1, 5));
        code.extend(rw_arg(3, 1));
        let raw = disassemble(&code).expect("disasm");
        let mut normalized = slot_norm(&raw);
        assert_eq!(normalized[0].operands, [Operand::JumpIndex(Some(2))]);
        assert_eq!(coalesce_dead_value_copies(&mut normalized, &raw), 0);
        assert_eq!(normalized.len(), raw.len());
    }

    #[test]
    fn n2_dead_value_copy_rejects_malformed_jump_and_jmpp_atomically() {
        let mut invalid_jump = dw_arg(11, 999); // target is outside the decoded stream
        invalid_jump.extend(rw_arg(94, 5));
        invalid_jump.extend(ww_rw_arg(80, 1, 5));
        invalid_jump.extend(rw_arg(3, 1));

        let mut invalid_jmpp = rw_dw_arg(57, 9, 1); // two rows required; row 0 is not a JMP
        invalid_jmpp.extend(rw_arg(94, 5));
        invalid_jmpp.extend(ww_rw_arg(80, 1, 5));
        invalid_jmpp.extend(rw_arg(3, 1));

        for code in [invalid_jump, invalid_jmpp] {
            let raw = disassemble(&code).expect("disasm");
            let mut normalized = slot_norm(&raw);
            let before = normalized.clone();
            assert_eq!(coalesce_dead_value_copies(&mut normalized, &raw), 0);
            assert_eq!(normalized.len(), before.len());
            assert!(normalized
                .iter()
                .zip(&before)
                .all(|(left, right)| left.norm_eq(right)));
        }
    }

    #[test]
    fn n2_flow_accepts_only_disjoint_non_escaping_register_split() {
        let left = vec![
            ni_setv4(1, 11),
            ni_slot("PshV4", 1),
            ni_setv4(1, 22), // physical slot 1 is reused for a disjoint value
            ni_slot("PshV4", 1),
            ni("RET"),
        ];
        let right = vec![
            ni_setv4(5, 11),
            ni_slot("PshV4", 5),
            ni_setv4(6, 22), // allocator splits the second value into another slot
            ni_slot("PshV4", 6),
            ni("RET"),
        ];
        assert_ne!(distinct_slot_count(&left), distinct_slot_count(&right));
        assert!(flow_equivalent_slots(&left, &right));

        let mut wrong_source = right.clone();
        wrong_source[3] = ni_slot("PshV4", 5); // reads definition #0, not definition #2
        assert!(!flow_equivalent_slots(&left, &wrong_source));
    }

    #[test]
    fn n2_flow_rejects_escaping_storage_split_and_abi_slot_rename() {
        let escaped_left = vec![
            ni_setv4(1, 11),
            ni_slot("PSF", 1),
            ni_setv4(1, 22),
            ni_slot("PSF", 1),
            ni("RET"),
        ];
        let escaped_right = vec![
            ni_setv4(5, 11),
            ni_slot("PSF", 5),
            ni_setv4(6, 22),
            ni_slot("PSF", 6),
            ni("RET"),
        ];
        assert!(
            !flow_equivalent_slots(&escaped_left, &escaped_right),
            "address-taken storage must retain one global slot bijection"
        );

        let abi_left = vec![ni_slot("PshV4", 0), ni("RET")];
        let abi_right = vec![ni_slot("PshV4", -2), ni("RET")];
        assert!(
            !flow_equivalent_slots(&abi_left, &abi_right),
            "signature-defined parameter/this offsets must never be alpha-renamed"
        );
    }

    #[test]
    fn n2_flow_uses_loop_reaching_definitions() {
        let left = vec![
            ni_setv4(1, 0),
            ni_slot("PshV4", 1),
            ni_setv4(1, 1),
            ni_jump("JNZ", 1),
            ni_slot("PshV4", 1),
            ni("RET"),
        ];
        let split_across_backedge = vec![
            ni_setv4(5, 0),
            ni_slot("PshV4", 5),
            ni_setv4(6, 1),
            ni_jump("JNZ", 1),
            ni_slot("PshV4", 6),
            ni("RET"),
        ];
        assert!(
            !flow_equivalent_slots(&left, &split_across_backedge),
            "the backedge reaches write #2 on the left but not slot 5 on the right"
        );
    }

    #[test]
    fn n2_flow_models_in_place_vm_writes() {
        let instrs = vec![
            ni_setv4(1, 7),
            ni_slot("NEGi", 1),
            ni_slot("PshV4", 1),
            ni("RET"),
        ];
        let flow = analyze_slot_flow(&instrs).expect("flow");
        assert_eq!(
            flow.incoming[2].get(&1),
            Some(&BTreeSet::from([FlowOrigin::Write {
                instruction: 1,
                operand: 0,
            }])),
            "PshV4 must observe the NEGi read-modify-write, not the older SetV4"
        );

        let free = ni_slot("FREE", 1);
        let roles = norm_slot_roles(&free).expect("FREE roles");
        assert_eq!(
            roles,
            [NormSlotRole {
                operand: 0,
                read: true,
                write: true,
            }]
        );
    }

    #[test]
    fn word_role_rwwdw_keeps_plain_word_out_of_n2_slots() {
        // LoadRObjR (184) is rW_W_DW: only operand 0 is a slot; operand 1 is a plain W.
        let code = vec![184 | (5 << 16), 17, 23];
        let raw = disassemble(&code).expect("disasm");
        let mut operands = Vec::new();
        push_word_operands("LoadRObjR", &raw[0], &mut operands);
        assert_eq!(operands[0], Operand::Slot(5));
        assert_eq!(operands[1], Operand::Word(17));
    }

    // ---- GAP-B / GAP-C gate tests (batch-38) ----

    /// GAP-B negative gate: a bare `PshC4 <n>` NOT followed by a `CALLSYS __STATIC_NAME` is a
    /// plain integer literal — it must stay `IntConst` (compared by value), never resolve through
    /// the StaticNames pool. (The positive path — a real `__STATIC_NAME` callee — is covered by the
    /// `#[ignore]`d real-cache regression `gap_b_static_name_index_benign`.)
    #[test]
    fn gap_b_lone_pshc4_stays_int_const() {
        let side = side_or_skip!();
        let opts = NormOpts::default();
        // PshC4 4369 followed by TZ (opcode 18, NOT __STATIC_NAME) — a real integer literal.
        let mut code = Vec::new();
        code.extend(dw_arg(2, 4369)); // PshC4 4369
        code.extend(no_arg(18)); // TZ
        let n = norm(&code, &side, &opts);
        assert_eq!(
            n[0].operands[0],
            Operand::IntConst {
                value: 4369,
                width: 4
            },
            "PshC4 not feeding __STATIC_NAME must stay an integer literal"
        );
    }

    /// GAP-C negative gate: a `TYPEID <large>` whose value is a large runtime object type-id but
    /// which does NOT feed an `opCast` (no matching call follows) must stay raw, so two different
    /// large ids still differ (SEMANTIC). This proves the gate requires the opCast, not merely a
    /// large id.
    #[test]
    /// N7 keeps the handle bits in the token: the same registered type used as a value, as a
    /// handle and as a const handle are three different operands, and only the drifting index
    /// below those bits is normalized away.
    #[test]
    fn n7_type_identity_separates_handle_flags() {
        let value = type_identity_token("AGothicCharacter", 0x0C00_1234);
        let handle = type_identity_token("AGothicCharacter", 0x4C00_1234);
        let const_handle = type_identity_token("AGothicCharacter", 0x6C00_1234);
        let other_index = type_identity_token("AGothicCharacter", 0x4C00_9999);
        assert_ne!(value, handle, "a value must not equal a handle");
        assert_ne!(handle, const_handle, "a handle must not equal a const handle");
        assert_eq!(
            handle, other_index,
            "the same type with the same flags is the same operand whatever index it drifted to"
        );
    }

    #[test]
    fn gap_c_typeid_without_opcast_stays_primitive() {
        let side = side_or_skip!();
        let opts = NormOpts::default();
        // TYPEID (opcode 76, DW_ARG) with a large runtime id, followed only by RET — no opCast.
        let big_a = 1207972964u32 as i32;
        let big_b = 1207972931u32 as i32;
        let mut a = Vec::new();
        a.extend(dw_arg(76, big_a)); // TYPEID <big_a>
        a.push(10); // RET
        let mut b = Vec::new();
        b.extend(dw_arg(76, big_b)); // TYPEID <big_b>
        b.push(10); // RET
        let na = norm(&a, &side, &opts);
        let nb = norm(&b, &side, &opts);
        // Both stay RawDw(<id>) (large id absent from richtest T2), compared by value → the two
        // DIFFER (no opcast gate fired).
        assert_ne!(
            na[0].operands, nb[0].operands,
            "a large TYPEID not feeding opCast must NOT be collapsed"
        );
        assert!(
            matches!(na[0].operands[0], Operand::RawDw(_)),
            "unfed large TYPEID stays raw, got {:?}",
            na[0].operands[0]
        );
    }

    // ---- Real-cache GAP-A/B/C regression flips (batch-38) ----
    // These read the large (gitignored, ~120 MB) vanilla + regen samples, so they are `#[ignore]`d
    // to keep routine `cargo test` fast/portable. Run on demand:
    //   cargo test --release -p gore-as --lib -- --ignored gap_
    // Each asserts a KNOWN member of the gap-class flips SEMANTIC→(IDENTICAL|BENIGN), while a
    // KNOWN real bug STAYS semantic.

    fn real_pair() -> Option<(Vec<u8>, Vec<u8>)> {
        let base = format!(
            "{}/../../work/reversing/gore-as/samples",
            env!("CARGO_MANIFEST_DIR")
        );
        let v = std::fs::read(format!("{base}/cache_A.Cache")).ok()?;
        let r = std::fs::read(format!("{base}/regen_batch36.Cache")).ok()?;
        Some((v, r))
    }

    fn verdict_of(v: &[u8], r: &[u8], func: &str) -> Vec<(String, Verdict)> {
        // Match the measurement configuration: N2 slot-renumber ON (the `--norm-slots` scoreboard),
        // so a pure first-use slot renumber (a separate benign class) doesn't mask the gap flip.
        let mut opts = NormOpts::default();
        opts.n2_slots = true;
        let filters = Filters {
            module: None,
            func: Some(func.to_string()),
        };
        let rep = run(v, r, &opts, &filters, 3).expect("run");
        rep.diffs
            .iter()
            .map(|d| (d.name.clone(), d.verdict))
            .collect()
    }

    /// GAP-A: the `GenericVoiclines::StaticClass` family (vanilla ns `G1R::GenericVoiceline`,
    /// regen empty) collapses SEMANTIC→BENIGN.
    #[test]
    #[ignore = "reads large gitignored sample caches; run with --ignored"]
    fn gap_a_namespace_drift_benign() {
        let Some((v, r)) = real_pair() else { return };
        let vs = verdict_of(&v, &r, "GenericVoiclines::StaticClass");
        assert!(!vs.is_empty(), "expected StaticClass functions");
        assert!(
            vs.iter().all(|(_, verd)| *verd != Verdict::Semantic),
            "GAP-A StaticClass ns-drift must be benign, got {vs:?}"
        );
    }

    /// GAP-B: the `GetHero` topic-getter (PshC4 pool-index → __STATIC_NAME) collapses to BENIGN.
    #[test]
    #[ignore = "reads large gitignored sample caches; run with --ignored"]
    fn gap_b_static_name_index_benign() {
        let Some((v, r)) = real_pair() else { return };
        let vs = verdict_of(&v, &r, "::GetHero");
        assert!(!vs.is_empty(), "expected GetHero functions");
        assert!(
            vs.iter().all(|(_, verd)| *verd != Verdict::Semantic),
            "GAP-B __STATIC_NAME index-drift must be benign, got a SEMANTIC in {vs:?}"
        );
    }

    /// GAP-C: `AIAgentConfig_Biter::Spawn` (lone opCast TYPEID drift) collapses to BENIGN.
    #[test]
    #[ignore = "reads large gitignored sample caches; run with --ignored"]
    fn gap_c_opcast_typeid_benign() {
        let Some((v, r)) = real_pair() else { return };
        let vs = verdict_of(&v, &r, "AIAgentConfig_Biter::Spawn");
        assert!(
            vs.iter().any(|(n, _)| n.contains("Spawn")),
            "expected Spawn, got {vs:?}"
        );
        assert!(
            vs.iter()
                .filter(|(n, _)| n.contains("AIAgentConfig_Biter::Spawn"))
                .all(|(_, verd)| *verd != Verdict::Semantic),
            "GAP-C opCast type-id drift must be benign, got {vs:?}"
        );
    }

    /// REGRESSION GUARD: a KNOWN real bug (op-count divergence) must STAY semantic after the
    /// refinements — the metric-honesty fix must not hide a behavioral bug.
    #[test]
    #[ignore = "reads large gitignored sample caches; run with --ignored"]
    fn real_bug_stays_semantic() {
        let Some((v, r)) = real_pair() else { return };
        // op-count divergence (v=20, r=30) — a genuine dropped/added-logic defect.
        let vs = verdict_of(&v, &r, "LoadOrCreateDataGame_Implementation");
        assert!(
            vs.iter().any(|(_, verd)| *verd == Verdict::Semantic),
            "LoadOrCreateDataGame must stay SEMANTIC, got {vs:?}"
        );
        // dropped-logic force-stub (UCBT_CompleteSequence::Tick, v=104 r=2).
        let vs2 = verdict_of(&v, &r, "UCBT_CompleteSequence::Tick");
        assert!(
            vs2.iter().any(|(_, verd)| *verd == Verdict::Semantic),
            "UCBT_CompleteSequence::Tick must stay SEMANTIC, got {vs2:?}"
        );
    }

    // =============================================================================================
    // N5 / N6 — benign-attribution one-sided vanilla strips (batch-48, `specs/final-residue.md` B).
    // Synthetic unit tests operate DIRECTLY on `Vec<NormInstr>` (the strips' input), so they need
    // no encoded bytecode — the exact op-shapes + a `Named` CALLSYS callee (via the remap
    // test-constructor) suffice.
    // =============================================================================================

    /// A NO-operand normalized instruction (e.g. TNZ, PshRPtr).
    fn ni(op: &'static str) -> NormInstr {
        NormInstr {
            op,
            operands: vec![],
        }
    }
    /// A normalized instruction addressing a single frame slot.
    fn ni_slot(op: &'static str, slot: i32) -> NormInstr {
        NormInstr {
            op,
            operands: vec![Operand::Slot(slot)],
        }
    }
    fn ni_setv4(slot: i32, value: i64) -> NormInstr {
        NormInstr {
            op: "SetV4",
            operands: vec![Operand::Slot(slot), Operand::IntConst { value, width: 4 }],
        }
    }
    /// A normalized CALLSYS whose callee resolves to `owner::method` (via the remap test ctor).
    fn ni_callsys(owner: &str, method: &str) -> NormInstr {
        NormInstr {
            op: "CALLSYS",
            operands: vec![Operand::Ref(OperandId::named_func_for_test(owner, method))],
        }
    }
    fn ni_jump(op: &'static str, target: usize) -> NormInstr {
        NormInstr {
            op,
            operands: vec![Operand::JumpIndex(Some(target))],
        }
    }

    /// An S1 re-guard window on object slot `x` loading into temp `y`, terminated by `TNZ`.
    fn s1_reguard(x: i32, y: i32) -> Vec<NormInstr> {
        vec![
            ni_slot("PshVPtr", x),
            ni_slot("RefCpyV", y),
            ni_slot("CmpPtrNull", y),
            ni("TNZ"),
        ]
    }
    /// The cascade-HEAD guard (first term): same shape but `JNZ` into the merge instead of `TNZ`.
    fn s1_head_guard(x: i32, y: i32) -> Vec<NormInstr> {
        vec![
            ni_slot("PshVPtr", x),
            ni_slot("RefCpyV", y),
            ni_slot("CmpPtrNull", y),
            ni_slot("JNZ", 0), // jump target operand irrelevant to the guard shape
        ]
    }

    /// N6: a dominated S1 re-guard (an identical earlier same-slot guard, no intervening write)
    /// FOLDS — the strip removes its 4-op window.
    #[test]
    fn n6_dominated_s1_folds() {
        let mut v = Vec::new();
        v.extend(s1_head_guard(3, 7)); // dominating head guard on slot 3
        v.push(ni("PshRPtr")); // some inert filler (no write to slot 3)
        v.extend(s1_reguard(3, 7)); // the re-guard on the SAME slot 3 -> should fold
        let before = v.len();
        let folded = fold_dominated_reguards(&mut v);
        assert_eq!(folded, 1, "the dominated re-guard must fold exactly once");
        assert_eq!(v.len(), before - 4, "a 4-op S1 window is removed");
        // The head guard survives (it is the dominator, not a re-guard).
        assert_eq!(v[0].op, "PshVPtr");
    }

    /// N6 GUARD (clause 3 — the load-bearing anti-false-benign clause): the SAME S1 re-guard but
    /// with an intervening WRITE to the guarded slot does NOT fold (the value may have changed).
    #[test]
    fn n6_reguard_with_intervening_write_stays() {
        let mut v = Vec::new();
        v.extend(s1_head_guard(3, 7)); // dominating guard on slot 3
        v.push(ni_slot("STOREOBJ", 3)); // <-- REASSIGNS slot 3 (a real write)
        v.extend(s1_reguard(3, 7)); // re-guard on slot 3 -> must NOT fold (not dominated)
        let before = v.len();
        let folded = fold_dominated_reguards(&mut v);
        assert_eq!(
            folded, 0,
            "an intervening write to the guarded slot blocks the fold"
        );
        assert_eq!(v.len(), before, "nothing removed");
    }

    /// N6 GUARD (clause 1): an S1 re-guard with NO earlier guard on that slot does NOT fold — a
    /// genuine first null-check must stay (a real dropped guard elsewhere would present this shape).
    #[test]
    fn n6_reguard_without_dominator_stays() {
        let mut v = Vec::new();
        v.push(ni("PshRPtr"));
        v.extend(s1_reguard(3, 7)); // only guard on slot 3, nothing earlier -> not dominated
        let before = v.len();
        let folded = fold_dominated_reguards(&mut v);
        assert_eq!(
            folded, 0,
            "a re-guard with no dominating earlier guard must stay"
        );
        assert_eq!(v.len(), before);
    }

    /// N6: a DIFFERENT-slot earlier guard does not dominate — the re-guard on slot 4 with an
    /// earlier guard only on slot 3 stays (clause 1 requires the IDENTICAL source).
    #[test]
    fn n6_reguard_different_slot_stays() {
        let mut v = Vec::new();
        v.extend(s1_head_guard(3, 7)); // guard on slot 3
        v.extend(s1_reguard(4, 7)); // re-guard on slot 4 -> different source, not dominated
        let folded = fold_dominated_reguards(&mut v);
        assert_eq!(folded, 0, "a re-guard on a different slot is not dominated");
    }

    /// N6 CONSERVATISM (S2 form): an S2 re-guard whose slot is RE-STORED (a fresh `STOREOBJ`, i.e.
    /// a fresh call result) is NOT dominated — the store is a write that breaks clause 3, so the
    /// re-guard stays. This is the correct safe behavior: an S2 `STOREOBJ; CmpPtrNull; TNZ` guards
    /// a NEW value, never a proven-non-null earlier one. (A false fold here would hide a genuine
    /// dropped null-check on a fresh call result — catastrophic.)
    #[test]
    fn n6_s2_restore_is_not_dominated() {
        let mut v = Vec::new();
        // earlier S2 guard on slot 5.
        v.push(ni_slot("STOREOBJ", 5));
        v.push(ni_slot("CmpPtrNull", 5));
        v.push(ni("TNZ"));
        v.push(ni("PshRPtr"));
        // a SECOND S2 on slot 5 — its own STOREOBJ re-writes slot 5 (a fresh call result).
        v.push(ni_slot("STOREOBJ", 5));
        v.push(ni_slot("CmpPtrNull", 5));
        v.push(ni("TNZ"));
        let before = v.len();
        let folded = fold_dominated_reguards(&mut v);
        assert_eq!(
            folded, 0,
            "a re-stored S2 slot is a fresh value, never dominated"
        );
        assert_eq!(v.len(), before);
    }

    /// N6 (S1 dominated by an S1 head guard on a STORED slot): an S1 re-load+guard of a slot that
    /// was guarded earlier by a HEAD guard (JNZ form) on the SAME slot, with no intervening write,
    /// folds. This is the real cascade shape (a `self`/member object slot re-checked per term).
    #[test]
    fn n6_s1_dominated_by_head_guard_folds() {
        let mut v = Vec::new();
        v.extend(s1_head_guard(5, 1)); // head guard on slot 5 (PshVPtr v5; ...; JNZ)
        v.push(ni("PshRPtr")); // inert, no write to slot 5
        v.push(ni_slot("CpyVtoV4", 2)); // writes slot 2 (NOT slot 5) — irrelevant
        v.extend(s1_reguard(5, 1)); // re-guard on slot 5 -> dominated, folds
        let folded = fold_dominated_reguards(&mut v);
        assert_eq!(
            folded, 1,
            "S1 re-guard on slot 5 dominated by the earlier head guard folds"
        );
    }

    #[test]
    fn n6_rebases_retained_jump_and_rejects_entry_into_folded_window() {
        let mut rebased = vec![ni_jump("JZ", 10)];
        rebased.extend(s1_head_guard(5, 1));
        rebased.push(ni("PshRPtr"));
        rebased.extend(s1_reguard(5, 1));
        rebased.push(ni("RET"));
        assert_eq!(rebased.len(), 11);
        assert_eq!(fold_dominated_reguards(&mut rebased), 1);
        assert_eq!(rebased.len(), 7);
        assert_eq!(rebased[0].operands, [Operand::JumpIndex(Some(6))]);

        let mut entered = vec![ni_jump("JZ", 6)]; // enters the would-be re-guard directly
        entered.extend(s1_head_guard(5, 1));
        entered.push(ni("PshRPtr"));
        entered.extend(s1_reguard(5, 1));
        entered.push(ni("RET"));
        let original = entered.clone();
        assert_eq!(fold_dominated_reguards(&mut entered), 0);
        assert_eq!(entered.len(), original.len());
        assert!(entered
            .iter()
            .zip(&original)
            .all(|(left, right)| left.norm_eq(right)));
    }

    /// N5: the `FScopeCycleCounter` RAII ctor/dtor pair + `FStatID` temp dtor strip; the kept-on-
    /// both-sides `FStatID::$beh0` ctor is NOT stripped.
    #[test]
    fn n5_scope_strips_raii_keeps_fstatid_ctor() {
        let mut v = vec![
            ni_slot("PSF", 0),
            ni_callsys("FStatID", "$beh0"), // KEPT (both sides emit it)
            ni_slot("PSF", 0),              // ctor argument: FStatID
            ni_slot("PSF", 1),              // ctor destination: FScopeCycleCounter
            ni_callsys("FScopeCycleCounter", "$beh0"), // strip (with both PSFs)
            ni_slot("PSF", 0),
            ni_callsys("FStatID", "$beh2"), // strip (with its PSF)
            ni("PshRPtr"),                  // body op
            ni_slot("PSF", 1),
            ni_callsys("FScopeCycleCounter", "$beh2"), // strip (with its PSF)
            ni("RET"),
        ];
        let removed = strip_benign_scopes(&mut v);
        assert_eq!(removed, 3, "three inert scope CALLSYS ops removed");
        // The ctor removes two PSFs; each dtor removes one: 7 ops gone; 11 -> 4.
        assert_eq!(
            v.len(),
            4,
            "the FStatID::$beh0 ctor + its PSF + body + RET survive"
        );
        // The kept ctor is still present.
        assert!(v
            .iter()
            .any(|n| callsys_owner_method(n) == Some(("FStatID", "$beh0"))));
        // No FScopeCycleCounter / FStatID::$beh2 op survives.
        assert!(!v.iter().any(|n| matches!(
            callsys_owner_method(n),
            Some(("FScopeCycleCounter", _)) | Some(("FStatID", "$beh2"))
        )));
    }

    /// N5 GUARD: a `CALLSYS` to an UNRELATED callee is never stripped.
    #[test]
    fn n5_scope_leaves_unrelated_callsys() {
        let mut v = vec![
            ni_slot("PSF", 0),
            ni_callsys("AGothicCharacterState", "IsTrulyPartOfGuild"),
            ni("RET"),
        ];
        let before = v.len();
        let removed = strip_benign_scopes(&mut v);
        assert_eq!(removed, 0);
        assert_eq!(v.len(), before);
    }

    /// N5 GUARD: every recognized profiler identity must have its exact physical frame arity.
    /// One/three-PSF ctors, aliased ctor slots, and zero/two-PSF dtors reject the whole pass.
    #[test]
    fn n5_scope_rejects_near_miss_frame_arities_atomically() {
        let cases = vec![
            vec![
                ni_slot("PSF", 0),
                ni_callsys("FScopeCycleCounter", "$beh0"),
                ni("RET"),
            ],
            vec![
                ni_slot("PSF", 9),
                ni_slot("PSF", 0),
                ni_slot("PSF", 1),
                ni_callsys("FScopeCycleCounter", "$beh0"),
                ni("RET"),
            ],
            vec![
                ni_slot("PSF", 0),
                ni_slot("PSF", 0),
                ni_callsys("FScopeCycleCounter", "$beh0"),
                ni("RET"),
            ],
            vec![ni_callsys("FStatID", "$beh2"), ni("RET")],
            vec![
                ni_slot("PSF", 0),
                ni_slot("PSF", 1),
                ni_callsys("FScopeCycleCounter", "$beh2"),
                ni("RET"),
            ],
        ];
        for (case, mut frame) in cases.into_iter().enumerate() {
            let original = frame.clone();
            assert_eq!(
                strip_benign_scopes(&mut frame),
                0,
                "near-miss case {case} must reject"
            );
            assert_eq!(frame.len(), original.len(), "case {case} length changed");
            assert!(
                frame.iter().zip(&original).all(|(a, b)| a.norm_eq(b)),
                "case {case} must remain instruction-for-instruction"
            );
        }
    }

    /// N5 GUARD: a branch into a would-be-dropped frame rejects the pass atomically.
    #[test]
    fn n5_scope_rejects_jump_into_dropped_frame() {
        let mut v = vec![
            ni_jump("JNZ", 2),
            ni_slot("PSF", 0),
            ni_slot("PSF", 1),
            ni_callsys("FScopeCycleCounter", "$beh0"),
            ni("RET"),
        ];
        let original = v.clone();
        assert_eq!(strip_benign_scopes(&mut v), 0);
        assert_eq!(v.len(), original.len());
        assert!(v.iter().zip(&original).all(|(a, b)| a.norm_eq(b)));
    }

    /// N5 rebases retained N3 targets both to a later retained op and to the end sentinel.
    #[test]
    fn n5_scope_rebases_retained_and_end_jump_targets() {
        let mut v = vec![
            ni_jump("JNZ", 5),
            ni_jump("JZ", 6),
            ni_slot("PSF", 0),
            ni_slot("PSF", 1),
            ni_callsys("FScopeCycleCounter", "$beh0"),
            ni("RET"),
        ];
        assert_eq!(strip_benign_scopes(&mut v), 1);
        assert_eq!(v.len(), 3);
        assert_eq!(v[0].operands, [Operand::JumpIndex(Some(2))]);
        assert_eq!(v[1].operands, [Operand::JumpIndex(Some(3))]);
        assert_eq!(v[2].op, "RET");
    }

    /// Self-identity MUST stay 162828/0/0 with N5/N6 ON — the raw-eq fast path returns IDENTICAL
    /// before any normalization, so the strips never run on a self-diff (spec §B.4.1).
    #[test]
    #[ignore = "reads large gitignored sample caches; run with --ignored"]
    fn self_identity_with_n5_n6() {
        let base = format!(
            "{}/../../work/reversing/gore-as/samples",
            env!("CARGO_MANIFEST_DIR")
        );
        let Ok(b) = std::fs::read(format!("{base}/cache_A.Cache")) else {
            return;
        };
        // Default opts have N5/N6 ON; also assert with N2 on (the --norm-slots config).
        for n2 in [false, true] {
            let mut opts = NormOpts::default();
            opts.n2_slots = n2;
            let rep = run(&b, &b, &opts, &Filters::default(), 6).expect("run");
            assert_eq!(
                rep.count(Verdict::Semantic),
                0,
                "self-diff SEMANTIC must be 0 (n2={n2})"
            );
            assert_eq!(
                rep.count(Verdict::Benign),
                0,
                "self-diff BENIGN must be 0 (n2={n2})"
            );
            assert_eq!(rep.count(Verdict::Identical), rep.diffs.len());
        }
    }

    fn real_pair_47() -> Option<(Vec<u8>, Vec<u8>)> {
        let base = format!(
            "{}/../../work/reversing/gore-as/samples",
            env!("CARGO_MANIFEST_DIR")
        );
        let v = std::fs::read(format!("{base}/cache_A.Cache")).ok()?;
        let r = std::fs::read(format!("{base}/regen_batch47.Cache")).ok()?;
        Some((v, r))
    }

    #[test]
    fn alignment_loss_is_a_semantic_release_failure() {
        let mut report = Report::default();
        assert!(!report.any_semantic());
        report.only_in_regen_funcs.push("M::Added()".to_owned());
        assert_eq!(report.alignment_loss_count(), 1);
        assert!(report.any_semantic());
    }

    /// CURATED REAL-BUG REGRESSION (batch-48): the four functions that MUST stay SEMANTIC after
    /// N5/N6 — a false BENIGN here would hide a real bug (catastrophic). None has a foldable
    /// scope/re-guard-only diff, so the strips must not flip them.
    #[test]
    #[ignore = "reads large gitignored sample caches; run with --ignored"]
    fn n5_n6_curated_regression_stays_semantic() {
        let Some((v, r)) = real_pair_47() else { return };
        for f in [
            "LoadOrCreateDataGame_Implementation", // op-count divergence
            "UCBT_CompleteSequence::Tick",         // 104->2 force-stub (has a dropped scope!)
            "OnGracefulExitRequested",             // dropped this.<field>=true member-store
            "DoWhenEventStarted",                  // documented dead-loop
        ] {
            let vs = verdict_of(&v, &r, f);
            assert!(!vs.is_empty(), "expected functions matching {f:?}");
            // SAFETY PROPERTY: no matched function may be marked BENIGN (a false BENIGN hides a
            // real bug — catastrophic). Sibling IDENTICAL stubs (e.g. a 1-op `DoWhenEventStarted`
            // override) are fine; the KNOWN-buggy one must remain SEMANTIC.
            assert!(
                vs.iter().all(|(_, verd)| *verd != Verdict::Benign),
                "{f}: N5/N6 must NEVER flip a real-bug function to BENIGN, got {vs:?}"
            );
            assert!(
                vs.iter().any(|(_, verd)| *verd == Verdict::Semantic),
                "{f}: the known-buggy function must STAY SEMANTIC after N5/N6, got {vs:?}"
            );
        }
    }

    /// The N5 scope strip must actually ENGAGE on a real scope function: with N5 ON the vanilla
    /// side loses its `FScopeCycleCounter`/`FStatID::$beh2` ops. We assert the mechanism works by
    /// checking that a scope function's raw disasm CONTAINS the scope callees (so the strip has
    /// something to remove) — proving the identity-keying resolves on real data, independent of
    /// whether the function flips (it stays SEMANTIC due to additional accumulator/default-init
    /// residue — documented in the batch-48 journal entry).
    #[test]
    #[ignore = "reads large gitignored sample caches; run with --ignored"]
    fn n5_scope_engages_on_real_function() {
        let Some((v, r)) = real_pair_47() else { return };
        let v_side = Side::build(&v).expect("v side");
        let _ = Side::build(&r);
        let fns = crate::cache::walk_modules::collect_function_bytecodes(&v).expect("walk");
        let opts = NormOpts::default();
        let target = fns
            .iter()
            .find(|f| f.func.contains("UCBT_Inverter::Tick"))
            .expect("UCBT_Inverter::Tick present in vanilla");
        let raw = disassemble(&target.bytecode).expect("disasm");
        let mut norm = normalize(&target.bytecode, &raw, &v_side, &opts);
        // The scope callees resolve by identity on real data.
        let scope_ops = norm
            .iter()
            .filter(|n| {
                matches!(
                    callsys_owner_method(n),
                    Some(("FScopeCycleCounter", "$beh0"))
                        | Some(("FScopeCycleCounter", "$beh2"))
                        | Some(("FStatID", "$beh2"))
                )
            })
            .count();
        assert!(
            scope_ops >= 2,
            "UCBT_Inverter::Tick has ≥2 inert scope callees, found {scope_ops}"
        );
        let before = norm.len();
        let removed = strip_benign_scopes(&mut norm);
        assert_eq!(
            removed, scope_ops,
            "strip removes exactly the resolved scope callees"
        );
        assert!(
            norm.len() < before,
            "the strip shortened the vanilla stream"
        );
        // The kept FStatID::$beh0 ctor survives.
        assert!(norm
            .iter()
            .any(|n| callsys_owner_method(n) == Some(("FStatID", "$beh0"))));
    }
}
