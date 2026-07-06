//! SEMANTIC-CORRECTNESS ORACLE — per-function bytecode byte-faithfulness between a VANILLA
//! cache and a REGEN (re-compilation of our decompiled source). Implements `gore as bytediff`
//! per `work/reversing/gore-as/specs/semantic-oracle.md`.
//!
//! Thesis: the AS compiler emits identical bytecode for identical source on the same engine
//! build, MODULO a handful of build-non-determinism sources — ref/type-id keys (runtime
//! pointers), jump absolutes (shift with instruction size), and constant raw encodings. This
//! tool normalizes exactly those (N1/N3/N4, plus an opt-in slot-renumber N2) and classifies each
//! aligned function IDENTICAL / BENIGN-DIFF / SEMANTIC-DIFF. A residual diff after normalization
//! is a difference the compiler was FORCED to make by different SOURCE — a real behavior change.
//!
//! Governing safety rule (`spec §3`): a false BENIGN hides a real bug (catastrophic); a false
//! SEMANTIC only wastes fix effort (cheap). Every normalizer is provably behavior-preserving; when
//! in doubt, leave the diff SEMANTIC.

use std::collections::HashMap;

use super::disasm::{disassemble, Instr};
use super::isa::BcType;
use super::refs::RefResolver;
use super::remap::{ref_sites, OperandId, RefIdentity, RefKind};

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
        "CALL" | "CALLBND" | "CALLINTF" => {
            side.refs.func_by_id(*ins.dwords.first()? as i32)
        }
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
        let Some(nx) = instrs.get(pos + k) else { return false };
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
    IntConst { value: i64, width: u8 },
    /// A float immediate compared bit-exactly by decoded f64 (N4, floatIsFloat64 build).
    FloatConst(u64),
    /// A jump target as an instruction index into the (normalized) list (N3). `None` = a target
    /// that has no counterpart instruction (dropped/added op) — always a SEMANTIC signal.
    JumpIndex(Option<usize>),
    /// A ref operand resolved to a portable identity (N1).
    Ref(OperandId),
    /// A resolved STR / __STATIC_NAME string literal, compared by text (N4).
    StaticName(Option<String>),
    /// A `TYPEID`/`Cast` operand that is a LARGE runtime object type-id feeding an `opCast`/`Cast`
    /// whose callee identity matches on both sides (GAP-C, batch-38). The raw id is a
    /// build-specific `asCTypeInfo` id that drifts across recompiles; the cast target is pinned by
    /// the adjacent (matched) opCast signature, so two such operands compare EQUAL regardless of
    /// the raw id. Only produced after the runtime-object-typeid + feeds-matching-opCast gate;
    /// a genuine primitive type-id stays a value-compared `Ref(Primitive)`.
    OpCastTypeId,
    /// A raw dword/qword the classifier does not model as slot/const/ref/jump — compared verbatim
    /// (conservative: an unmodeled operand difference stays SEMANTIC).
    RawDw(u32),
    RawQw(u64),
}

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
    /// N5 (`n5_scope`): one-sided vanilla strip of the `FScopeCycleCounter` RAII profiler-scope
    /// pair + the `FStatID` temp dtor — pure CPU-timing instrumentation, provably behavior-neutral
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

/// Read a qword (2 dwords LE) from the bytecode at absolute dword offset.
fn read_qw(code: &[i32], dw: usize) -> u64 {
    let lo = code[dw] as u32 as u64;
    let hi = code[dw + 1] as u32 as u64;
    lo | (hi << 32)
}

/// Build the offset->instruction-index map for N3 jump canonicalization.
fn offset_index_map(instrs: &[Instr]) -> HashMap<usize, usize> {
    instrs.iter().enumerate().map(|(i, ins)| (ins.offset_dw, i)).collect()
}

/// Normalize one function's disassembled instructions into `NormInstr`s.
///
/// This applies N1 (refs->identity), N3 (jumps->index), N4 (consts by value; STR/__STATIC_NAME
/// by string). N2 (slot renumber) is applied afterwards on the whole list if enabled.
fn normalize(
    code: &[i32],
    instrs: &[Instr],
    side: &Side,
    opts: &NormOpts,
) -> Vec<NormInstr> {
    let off_to_idx = offset_index_map(instrs);
    let mut out = Vec::with_capacity(instrs.len());

    for (pos, ins) in instrs.iter().enumerate() {
        let name = ins.op.name;
        // Ref-operand dword indices (relative to the instruction start) → identity, via the
        // SHARED ref_sites classification (N1). Collect into a per-dword-index lookup so the
        // positional walk below can substitute them.
        let mut ref_at_dw: HashMap<usize, OperandId> = HashMap::new();
        if opts.n1_refs {
            for site in ref_sites(name) {
                let base = ins.offset_dw + site.dw_index;
                if base >= code.len() {
                    continue;
                }
                let id = match site.kind {
                    RefKind::GlobalPtr | RefKind::FuncPtr | RefKind::TypePtr => {
                        if base + 1 >= code.len() {
                            continue;
                        }
                        side.ident.resolve_ptr(site.kind, read_qw(code, base) as i64)
                    }
                    RefKind::FuncId | RefKind::TypeId => {
                        side.ident.resolve_id(site.kind, code[base])
                    }
                };
                ref_at_dw.insert(site.dw_index, id);
            }
        }

        let operands = normalize_operands(name, ins, &ref_at_dw, &off_to_idx, side, opts, instrs, pos);
        out.push(NormInstr { op: name, operands });
    }

    if opts.n2_slots {
        renumber_slots(&mut out);
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
    ref_at_dw: &HashMap<usize, OperandId>,
    off_to_idx: &HashMap<usize, usize>,
    side: &Side,
    opts: &NormOpts,
    instrs: &[Instr],
    pos: usize,
) -> Vec<Operand> {
    let mut out = Vec::new();

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
        if let Some(id) = ref_at_dw.get(&dw_idx) {
            // GAP-C (batch-38): a `TYPEID`/`Cast` operand that is a LARGE runtime object type-id
            // (an `asCTypeInfo` id not in T2, mask bits set) is build-specific and drifts. When it
            // feeds an `opCast`/`Cast` whose callee identity matches on both sides (verified at the
            // opCast op's own index), the cast TARGET is pinned by that signature, so collapse the
            // drifting id to a single canonical token. A genuine primitive type-id stays a
            // value-compared Ref(Primitive).
            if id.is_runtime_object_typeid() && feeds_matching_opcast(instrs, pos, side) {
                out.push(Operand::OpCastTypeId);
                continue;
            }
            out.push(Operand::Ref(id.clone()));
            continue;
        }
        // N3 jump target?
        if opts.n3_jumps && is_jump_op(name) {
            // Target byte(dword) offset is relative to the END of the jump instruction.
            let target_off = (ins.offset_dw as i64 + ins.op.size_dwords as i64 + raw as i32 as i64) as usize;
            out.push(Operand::JumpIndex(off_to_idx.get(&target_off).copied()));
            continue;
        }
        // GAP-B (batch-38): a `PshC4 <idx>` immediately followed by `CALLSYS __STATIC_NAME` is an
        // FName-literal pool index, NOT an integer value. The StaticNames pool is rebuilt per-cache
        // (different size), so the same name lands at a different slot — resolve the index to TEXT
        // and compare by string (mirror the `STR` handling below). The tight next-instruction gate
        // keeps a real integer literal (not feeding __STATIC_NAME) comparing by value.
        if opts.n4_consts && name == "PshC4" && next_is_static_name(instrs, pos, side) {
            let s = side.refs.static_name(raw as i32 as i64).map(|s| s.to_string());
            out.push(Operand::StaticName(s));
            continue;
        }
        // N4 constant?
        match const_dword_role(name, dop) {
            DwordRole::Int(width) => {
                out.push(Operand::IntConst { value: raw as i32 as i64, width });
            }
            DwordRole::None => out.push(Operand::RawDw(raw)),
        }
    }

    // --- Qword operands: ref ptr (N1), 64-bit const (N4 float/int), or raw. ---
    let qword_indices = qword_operand_indices(ins.op.fmt);
    for (qop, &dw_idx) in qword_indices.iter().enumerate() {
        let raw = *ins.qwords.get(qop).unwrap_or(&0);
        if let Some(id) = ref_at_dw.get(&dw_idx) {
            out.push(Operand::Ref(id.clone()));
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

/// Push the word (16-bit) operands of an instruction as Slot/Word tokens per BcType.
///
/// `rW`/`wW` word slots are frame slots (Slot); a bare `W` arg is a plain word (Word). The
/// per-BcType word roles mirror `disasm`'s word-collection order.
fn push_word_operands(name: &str, ins: &Instr, out: &mut Vec<Operand>) {
    use BcType::*;
    // (is_slot?) per word position, in the order `disasm` collected them.
    let roles: &[bool] = match ins.op.fmt {
        W_ARG => &[false],            // plain word (e.g. RET size, ChkNullS var-is-actually-slot)
        wW_ARG | rW_ARG => &[true],   // one slot
        wW_rW_ARG | rW_rW_ARG => &[true, true],
        wW_W_ARG => &[true, false],
        W_rW_ARG => &[false, true],
        wW_rW_rW_ARG => &[true, true, true],
        rW_DW_ARG | wW_DW_ARG | rW_QW_ARG | wW_QW_ARG => &[true], // leading slot, then DW/QW
        W_DW_ARG => &[false],        // leading plain word (ADDSi/LoadThisR: word=offset), then DW
        wW_rW_DW_ARG | rW_W_DW_ARG => &[true, true],
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
fn renumber_slots(instrs: &mut [NormInstr]) {
    let mut map: HashMap<i32, i32> = HashMap::new();
    let mut next = 0i32;
    for ni in instrs.iter_mut() {
        for op in ni.operands.iter_mut() {
            if let Operand::Slot(s) = op {
                let canon = *map.entry(*s).or_insert_with(|| {
                    let v = next;
                    next += 1;
                    v
                });
                *op = Operand::Slot(canon);
            }
        }
    }
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
    instrs.iter().filter(|ni| !is_benign_only_op(ni.op)).cloned().collect()
}

// =================================================================================================
// N5 / N6 — benign-attribution ONE-SIDED VANILLA strips (`specs/final-residue.md` PART B).
//
// Both remove a PROVEN-INERT subsequence from the VANILLA normalized list only (regen already
// lacks it), so they can only ever SHORTEN vanilla toward the (shorter) regen — never pad, never
// manufacture a match by insertion. Governing safety rule (`bytediff.rs:12`): a false BENIGN hides
// a real bug (catastrophic); when a pattern cannot be PROVEN inert/dominated, it is left in place
// and the length mismatch keeps the function SEMANTIC.
// =================================================================================================

/// The (owner-type-name, method-name) of a normalized CALLSYS/CALL instruction's callee, if it
/// resolved to a `Named` function identity. Keys N5 on a cache-INDEPENDENT identity (the raw
/// func-ptr drifts across builds; the resolved owner+method does not — mirrors GAP-B/GAP-C keying
/// on `callee_name`). Returns `None` for a non-call op or an unresolved callee.
fn callsys_owner_method(ni: &NormInstr) -> Option<(&str, &str)> {
    if !matches!(ni.op, "CALLSYS" | "CALL" | "CALLBND" | "CALLINTF" | "Thiscall1" | "FuncPtr") {
        return None;
    }
    ni.operands.iter().find_map(|o| match o {
        Operand::Ref(id) => id.func_owner_method(),
        _ => None,
    })
}

/// N5 — `FScopeCycleCounter` RAII profiler-scope strip (`specs/final-residue.md §B.2`).
///
/// Removes, from the VANILLA list, each `[PSF <slot>; CALLSYS <inert-scope-callee>]` PAIR where the
/// callee resolves EXACTLY to one of the three inert RAII identities:
///   * `FScopeCycleCounter::$beh0`  — the RAII scope-counter CTOR (snapshots `Cycles()`)
///   * `FScopeCycleCounter::$beh2`  — the RAII scope-counter DTOR (accumulates elapsed cycles)
///   * `FStatID::$beh2`             — the transient `FStatID` temp DTOR that only fed the ctor
///
/// The `FStatID::$beh0` CTOR is KEPT (both sides emit it — NOT stripped). Each CALLSYS is removed
/// together with its immediately-preceding `PSF <same-or-any slot>` frame-push (that push exists
/// only to address the RAII object for this call); if the preceding op is not a `PSF`, only the
/// CALLSYS is removed (defensive — never remove an unrelated op).
///
/// Behaviour proof (§B.2): the three callees touch no game object / ability / actor / return
/// register — they read `FPlatformTime::Cycles()` and accumulate into a named `TStatId` CPU
/// counter (compiled-in `SCOPE_CYCLE_COUNTER` instrumentation). Removing them changes only the
/// stats-HUD timing readout. Provably behavior-neutral.
///
/// Returns the number of CALLSYS scope-ops removed (each with its paired push).
fn strip_benign_scopes(v: &mut Vec<NormInstr>) -> usize {
    // Identify the indices of the inert scope CALLSYS ops and their paired preceding PSF.
    let mut drop: Vec<bool> = vec![false; v.len()];
    let mut removed = 0usize;
    for i in 0..v.len() {
        let is_scope = matches!(
            callsys_owner_method(&v[i]),
            Some(("FScopeCycleCounter", "$beh0"))
                | Some(("FScopeCycleCounter", "$beh2"))
                | Some(("FStatID", "$beh2"))
        );
        if !is_scope {
            continue;
        }
        drop[i] = true;
        removed += 1;
        // Pair the immediately-preceding PSF frame-push (only if it is a PSF and not already
        // claimed by another dropped call).
        if i > 0 && v[i - 1].op == "PSF" && !drop[i - 1] {
            drop[i - 1] = true;
        }
    }
    if removed > 0 {
        let mut idx = 0usize;
        v.retain(|_| {
            let keep = !drop[idx];
            idx += 1;
            keep
        });
    }
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
    let ReguardSrc::Push(Operand::Slot(guard_slot)) = src else { return false };
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

    // N2 GUARD: if slot renumber is on but the two sides have a different distinct-slot COUNT, a
    // real structural difference (added/dropped local) exists — DO NOT let N2 collapse it. We
    // detect this and, if it's the only thing making them differ, still classify SEMANTIC.
    let n2_slot_mismatch =
        opts.n2_slots && distinct_slot_count(v_norm) != distinct_slot_count(r_norm);

    // Compare normalized lists, tolerating JitEntry scaffolding.
    let (mut v_cmp, r_cmp, jit_fired) = {
        let vj = strip_jitentry(v_norm);
        let rj = strip_jitentry(r_norm);
        let fired = vj.len() != v_norm.len() || rj.len() != r_norm.len();
        (vj, rj, fired)
    };

    // N5/N6 — benign-attribution ONE-SIDED VANILLA strips (`specs/final-residue.md` PART B). Apply
    // #2 (scope strip) BEFORE #1 (re-guard fold) per §B.3 (disjoint, but scope-first keeps the
    // re-guard scanner's contiguous windows intact). Each only ever SHORTENS the vanilla side, so
    // neither can pad a match into existence.
    let scope_fired = if opts.n5_scope { strip_benign_scopes(&mut v_cmp) > 0 } else { false };
    let reguard_fired = if opts.n6_reguard { fold_dominated_reguards(&mut v_cmp) > 0 } else { false };

    let norm_identical = !n2_slot_mismatch
        && v_cmp.len() == r_cmp.len()
        && v_cmp.iter().zip(&r_cmp).all(|(a, b)| a.norm_eq(b));

    if norm_identical {
        // BENIGN: determine WHICH normalizers were responsible by re-diffing raw operand roles.
        let mut fired = which_normalizers_fired(v_raw, r_raw, v_norm, r_norm, opts, jit_fired);
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
            list[lo..hi]
                .iter()
                .any(|n| matches!(n.op, "CHKREF" | "ChkNullV" | "ChkNullS" | "CmpPtrNull" | "ChkRefS"))
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
                "sbTOi" | "swTOi" | "ubTOi" | "uwTOi" | "iTOb" | "iTOw" | "iTOf" | "fTOi"
                    | "dTOf" | "fTOd" | "iTOd" | "dTOi" | "i64TOi" | "iTOi64" | "Cast"
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
fn render_window(
    v: &[NormInstr],
    r: &[NormInstr],
    first: Option<usize>,
    context: usize,
) -> String {
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
        s.push_str(&format!("      [{i:04}] {:<14} {}\n", ni.op, ops.join(", ")));
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
        Operand::StaticName(Some(s)) => format!("n\"{s}\""),
        Operand::StaticName(None) => "n<?>".to_string(),
        Operand::OpCastTypeId => "opcast-typeid".to_string(),
        Operand::RawDw(d) => format!("0x{d:x}"),
        Operand::RawQw(q) => format!("0x{q:x}"),
    }
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
                        && (v_raw[i].qwords != r_raw[i].qwords || raw_id_differs(&v_raw[i], &r_raw[i]))
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
        filters.module.as_ref().map_or(true, |m| fc.func.contains(m.as_str()))
            && filters.func.as_ref().map_or(true, |f| fc.func.contains(f.as_str()))
    };

    // Index regen functions by alignment key (dup keys -> ordered Vec, consumed positionally so
    // N overloads on each side pair up 1:1).
    let mut r_index: HashMap<String, Vec<&super::walk_modules::FuncCode>> = HashMap::new();
    for fc in &r_fns {
        r_index.entry(func_key(fc, &r_side.refs)).or_default().push(fc);
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
                let d = diff_one(&vfc.func, &vfc.bytecode, &rfc.bytecode, &v_side, &r_side, opts, context);
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
    let v_norm = normalize(v_code, &v_raw, v_side, opts);
    let r_norm = normalize(r_code, &r_raw, r_side, opts);
    classify(name.to_string(), &v_raw, &r_raw, &v_norm, &r_norm, opts, context)
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

    fn sample(name: &str) -> Vec<u8> {
        std::fs::read(format!(
            "{}/../../work/reversing/gore-as/samples/{name}",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap_or_else(|_| panic!("read sample {name}"))
    }

    /// Self-identity on a small real cache: a cache vs itself must be PERFECTLY identical —
    /// every function IDENTICAL, 0 BENIGN, 0 SEMANTIC, no alignment loss. This is the correctness
    /// gate for N1/N3/N4.
    #[test]
    fn self_identity_richtest() {
        let b = sample("PrecompiledScript.richtest.Cache");
        let rep = run(&b, &b, &NormOpts::default(), &Filters::default(), 6).expect("run");
        assert_eq!(rep.count(Verdict::Benign), 0, "self-diff must have 0 BENIGN");
        assert_eq!(rep.count(Verdict::Semantic), 0, "self-diff must have 0 SEMANTIC");
        assert!(rep.only_in_vanilla_funcs.is_empty(), "no dropped fns in self-diff");
        assert!(rep.only_in_regen_funcs.is_empty(), "no added fns in self-diff");
        assert!(!rep.diffs.is_empty(), "richtest has at least one function");
        assert_eq!(rep.count(Verdict::Identical), rep.diffs.len());
    }

    /// Self-identity must ALSO hold with N2 (slot renumber) enabled — a cache vs itself has
    /// identical slot shapes, so N2 is a no-op that keeps everything IDENTICAL (raw-eq fast path
    /// actually short-circuits, but assert the N2 path doesn't regress).
    #[test]
    fn self_identity_richtest_with_n2() {
        let b = sample("PrecompiledScript.richtest.Cache");
        let mut opts = NormOpts::default();
        opts.n2_slots = true;
        let rep = run(&b, &b, &opts, &Filters::default(), 6).expect("run");
        assert_eq!(rep.count(Verdict::Semantic), 0);
        assert_eq!(rep.count(Verdict::Benign), 0);
        assert_eq!(rep.count(Verdict::Identical), rep.diffs.len());
    }

    #[test]
    fn self_identity_visproof() {
        let b = sample("PrecompiledScript.visproof.Cache");
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
    /// Encode a QW_ARG op: opcode dword + 64-bit arg (2 dwords LE).
    fn qw_arg(opcode: u8, arg: u64) -> Vec<i32> {
        vec![opcode as i32, arg as u32 as i32, (arg >> 32) as u32 as i32]
    }

    // opcodes (from isa.rs): JMP=11, JZ=12, SUSPEND=63, RET=10 (W_ARG),
    // PshC8=47 (QW), PshV4=3 (rW), PopPtr=0 (NO_ARG), TZ=18 (NO_ARG).

    fn side() -> Side {
        // N3/N4/N2 don't need real tail tables; a tiny sample gives a valid RefResolver/RefIdentity.
        let b = sample("PrecompiledScript.richtest.Cache");
        Side::build(&b).expect("side")
    }

    /// Build a normalized instruction list from raw bytecode (test helper).
    fn norm(code: &[i32], side: &Side, opts: &NormOpts) -> Vec<NormInstr> {
        let raw = disassemble(code).expect("disasm");
        normalize(code, &raw, side, opts)
    }

    /// N3: two functions with the SAME instruction COUNT but where an earlier instruction has a
    /// different SIZE (a benign const-width change: PshC4 2dw vs PshC8 3dw) — the JMP's absolute
    /// byte(dword) offset shifts, but its target INSTRUCTION INDEX is the same, so N3 makes the
    /// jump operand compare EQUAL (the control-flow edge is preserved).
    #[test]
    fn n3_jump_index_equal_despite_offset_shift() {
        let side = side();
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
        assert_eq!(a_jmp.operands, b_jmp.operands, "jump edge preserved across size shift");
    }

    /// N3 guard: a jump whose target INDEX differs (branches to a structurally different
    /// instruction) stays a real difference — the operands are NOT equal.
    #[test]
    fn n3_jump_to_different_index_differs() {
        let side = side();
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
        assert_ne!(na[0].operands, nb[0].operands, "different jump target index must differ");
    }

    /// N4: a 64-bit float constant compares by DECODED value (bit pattern). Same value => equal;
    /// a genuinely different value => not equal. (Same-value floats already share bits, so this
    /// mainly asserts we decode + compare the qword, not a raw pass-through that could mis-handle.)
    #[test]
    fn n4_float_const_by_value() {
        let side = side();
        let opts = NormOpts::default();
        let same_a = norm(&qw_arg(47, 3.5f64.to_bits()), &side, &opts);
        let same_b = norm(&qw_arg(47, 3.5f64.to_bits()), &side, &opts);
        assert_eq!(same_a[0].operands, same_b[0].operands);
        assert_eq!(same_a[0].operands[0], Operand::FloatConst(3.5f64.to_bits()));
        let diff = norm(&qw_arg(47, 4.5f64.to_bits()), &side, &opts);
        assert_ne!(same_a[0].operands, diff[0].operands, "different float value must differ");
    }

    /// N4: a 32-bit integer immediate (PshC4=2, DW_ARG) compares by decoded value.
    #[test]
    fn n4_int_const_by_value() {
        let side = side();
        let opts = NormOpts::default();
        let a = norm(&dw_arg(2, 42), &side, &opts);
        let b = norm(&dw_arg(2, 42), &side, &opts);
        assert_eq!(a[0].operands, b[0].operands);
        assert_eq!(a[0].operands[0], Operand::IntConst { value: 42, width: 4 });
        let c = norm(&dw_arg(2, 43), &side, &opts);
        assert_ne!(a[0].operands, c[0].operands);
    }

    /// N2 (opt-in): two streams that use DIFFERENT raw slot numbers but in the SAME first-use
    /// order canonicalize to identical ordinals. With N2 OFF they differ; with N2 ON they match.
    #[test]
    fn n2_slot_renumber_first_use() {
        let side = side();
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
        assert_ne!(na[0].operands, nb[0].operands, "raw slots differ with N2 off");

        let mut on = NormOpts::default();
        on.n2_slots = true;
        let na = norm(&a, &side, &on);
        let nb = norm(&b, &side, &on);
        assert_eq!(na[0].operands, nb[0].operands, "first-use order maps to same ordinal");
        assert_eq!(na[1].operands, nb[1].operands);
        assert_eq!(na[0].operands[0], Operand::Slot(0));
        assert_eq!(na[1].operands[0], Operand::Slot(1));
    }

    // ---- GAP-B / GAP-C gate tests (batch-38) ----

    /// GAP-B negative gate: a bare `PshC4 <n>` NOT followed by a `CALLSYS __STATIC_NAME` is a
    /// plain integer literal — it must stay `IntConst` (compared by value), never resolve through
    /// the StaticNames pool. (The positive path — a real `__STATIC_NAME` callee — is covered by the
    /// `#[ignore]`d real-cache regression `gap_b_static_name_index_benign`.)
    #[test]
    fn gap_b_lone_pshc4_stays_int_const() {
        let side = side();
        let opts = NormOpts::default();
        // PshC4 4369 followed by TZ (opcode 18, NOT __STATIC_NAME) — a real integer literal.
        let mut code = Vec::new();
        code.extend(dw_arg(2, 4369)); // PshC4 4369
        code.extend(no_arg(18)); // TZ
        let n = norm(&code, &side, &opts);
        assert_eq!(
            n[0].operands[0],
            Operand::IntConst { value: 4369, width: 4 },
            "PshC4 not feeding __STATIC_NAME must stay an integer literal"
        );
    }

    /// GAP-C negative gate: a `TYPEID <large>` whose value is a large runtime object type-id but
    /// which does NOT feed an `opCast` (no matching call follows) must stay a value-compared
    /// `Ref(Primitive)`, so two different large ids still differ (SEMANTIC). This proves the gate
    /// requires the opCast, not merely a large id.
    #[test]
    fn gap_c_typeid_without_opcast_stays_primitive() {
        let side = side();
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
        // Both resolve to Ref(Primitive(<id>)) (large id absent from richtest T2), compared by
        // value → the two DIFFER (no opcast gate fired).
        assert_ne!(
            na[0].operands, nb[0].operands,
            "a large TYPEID not feeding opCast must NOT be collapsed"
        );
        assert!(
            matches!(na[0].operands[0], Operand::Ref(_)),
            "unfed large TYPEID stays a value-compared Ref, got {:?}",
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
        let base = format!("{}/../../work/reversing/gore-as/samples", env!("CARGO_MANIFEST_DIR"));
        let v = std::fs::read(format!("{base}/cache_A.Cache")).ok()?;
        let r = std::fs::read(format!("{base}/regen_batch36.Cache")).ok()?;
        Some((v, r))
    }

    fn verdict_of(v: &[u8], r: &[u8], func: &str) -> Vec<(String, Verdict)> {
        // Match the measurement configuration: N2 slot-renumber ON (the `--norm-slots` scoreboard),
        // so a pure first-use slot renumber (a separate benign class) doesn't mask the gap flip.
        let mut opts = NormOpts::default();
        opts.n2_slots = true;
        let filters = Filters { module: None, func: Some(func.to_string()) };
        let rep = run(v, r, &opts, &filters, 3).expect("run");
        rep.diffs.iter().map(|d| (d.name.clone(), d.verdict)).collect()
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
        assert!(vs.iter().any(|(n, _)| n.contains("Spawn")), "expected Spawn, got {vs:?}");
        assert!(
            vs.iter().filter(|(n, _)| n.contains("AIAgentConfig_Biter::Spawn")).all(|(_, verd)| *verd != Verdict::Semantic),
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
        NormInstr { op, operands: vec![] }
    }
    /// A normalized instruction addressing a single frame slot.
    fn ni_slot(op: &'static str, slot: i32) -> NormInstr {
        NormInstr { op, operands: vec![Operand::Slot(slot)] }
    }
    /// A normalized CALLSYS whose callee resolves to `owner::method` (via the remap test ctor).
    fn ni_callsys(owner: &str, method: &str) -> NormInstr {
        NormInstr {
            op: "CALLSYS",
            operands: vec![Operand::Ref(OperandId::named_func_for_test(owner, method))],
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
        assert_eq!(folded, 0, "an intervening write to the guarded slot blocks the fold");
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
        assert_eq!(folded, 0, "a re-guard with no dominating earlier guard must stay");
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
        assert_eq!(folded, 0, "a re-stored S2 slot is a fresh value, never dominated");
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
        assert_eq!(folded, 1, "S1 re-guard on slot 5 dominated by the earlier head guard folds");
    }

    /// N5: the `FScopeCycleCounter` RAII ctor/dtor pair + `FStatID` temp dtor strip; the kept-on-
    /// both-sides `FStatID::$beh0` ctor is NOT stripped.
    #[test]
    fn n5_scope_strips_raii_keeps_fstatid_ctor() {
        let mut v = vec![
            ni_slot("PSF", 0),
            ni_callsys("FStatID", "$beh0"), // KEPT (both sides emit it)
            ni_slot("PSF", 0),
            ni_callsys("FScopeCycleCounter", "$beh0"), // strip (with its PSF)
            ni_slot("PSF", 0),
            ni_callsys("FStatID", "$beh2"), // strip (with its PSF)
            ni("PshRPtr"), // body op
            ni_slot("PSF", 1),
            ni_callsys("FScopeCycleCounter", "$beh2"), // strip (with its PSF)
            ni("RET"),
        ];
        let removed = strip_benign_scopes(&mut v);
        assert_eq!(removed, 3, "three inert scope CALLSYS ops removed");
        // Each removed CALLSYS took its paired PSF too: 3 pairs = 6 ops gone; 10 -> 4.
        assert_eq!(v.len(), 4, "the FStatID::$beh0 ctor + its PSF + body + RET survive");
        // The kept ctor is still present.
        assert!(v.iter().any(|n| callsys_owner_method(n) == Some(("FStatID", "$beh0"))));
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

    /// Self-identity MUST stay 162828/0/0 with N5/N6 ON — the raw-eq fast path returns IDENTICAL
    /// before any normalization, so the strips never run on a self-diff (spec §B.4.1).
    #[test]
    #[ignore = "reads large gitignored sample caches; run with --ignored"]
    fn self_identity_with_n5_n6() {
        let base = format!("{}/../../work/reversing/gore-as/samples", env!("CARGO_MANIFEST_DIR"));
        let Ok(b) = std::fs::read(format!("{base}/cache_A.Cache")) else { return };
        // Default opts have N5/N6 ON; also assert with N2 on (the --norm-slots config).
        for n2 in [false, true] {
            let mut opts = NormOpts::default();
            opts.n2_slots = n2;
            let rep = run(&b, &b, &opts, &Filters::default(), 6).expect("run");
            assert_eq!(rep.count(Verdict::Semantic), 0, "self-diff SEMANTIC must be 0 (n2={n2})");
            assert_eq!(rep.count(Verdict::Benign), 0, "self-diff BENIGN must be 0 (n2={n2})");
            assert_eq!(rep.count(Verdict::Identical), rep.diffs.len());
        }
    }

    fn real_pair_47() -> Option<(Vec<u8>, Vec<u8>)> {
        let base = format!("{}/../../work/reversing/gore-as/samples", env!("CARGO_MANIFEST_DIR"));
        let v = std::fs::read(format!("{base}/cache_A.Cache")).ok()?;
        let r = std::fs::read(format!("{base}/regen_batch47.Cache")).ok()?;
        Some((v, r))
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
            "UCBT_CompleteSequence::Tick",          // 104->2 force-stub (has a dropped scope!)
            "OnGracefulExitRequested",              // dropped this.<field>=true member-store
            "DoWhenEventStarted",                   // documented dead-loop
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
        assert!(scope_ops >= 2, "UCBT_Inverter::Tick has ≥2 inert scope callees, found {scope_ops}");
        let before = norm.len();
        let removed = strip_benign_scopes(&mut norm);
        assert_eq!(removed, scope_ops, "strip removes exactly the resolved scope callees");
        assert!(norm.len() < before, "the strip shortened the vanilla stream");
        // The kept FStatID::$beh0 ctor survives.
        assert!(norm.iter().any(|n| callsys_owner_method(n) == Some(("FStatID", "$beh0"))));
    }
}
