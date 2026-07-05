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
}

impl Default for NormOpts {
    fn default() -> Self {
        // All ON except N2 (slot renumber) — the spec default.
        NormOpts { n1_refs: true, n2_slots: false, n3_jumps: true, n4_consts: true }
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

    for ins in instrs {
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

        let operands = normalize_operands(name, ins, &ref_at_dw, &off_to_idx, side, opts);
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
fn normalize_operands(
    name: &str,
    ins: &Instr,
    ref_at_dw: &HashMap<usize, OperandId>,
    off_to_idx: &HashMap<usize, usize>,
    side: &Side,
    opts: &NormOpts,
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
    for (pos, &dw_idx) in dword_indices.iter().enumerate() {
        let raw = *ins.dwords.get(pos).unwrap_or(&0);
        // N1 ref (func-id / type-id) at this dword index?
        if let Some(id) = ref_at_dw.get(&dw_idx) {
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
        // N4 constant?
        match const_dword_role(name, pos) {
            DwordRole::Int(width) => {
                out.push(Operand::IntConst { value: raw as i32 as i64, width });
            }
            DwordRole::None => out.push(Operand::RawDw(raw)),
        }
    }

    // --- Qword operands: ref ptr (N1), 64-bit const (N4 float/int), or raw. ---
    let qword_indices = qword_operand_indices(ins.op.fmt);
    for (pos, &dw_idx) in qword_indices.iter().enumerate() {
        let raw = *ins.qwords.get(pos).unwrap_or(&0);
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
        v
    }
    fn any(&self) -> bool {
        self.n1_refs || self.n2_slots || self.n3_jumps || self.n4_consts
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
    let (v_cmp, r_cmp, jit_fired) = {
        let vj = strip_jitentry(v_norm);
        let rj = strip_jitentry(r_norm);
        let fired = vj.len() != v_norm.len() || rj.len() != r_norm.len();
        (vj, rj, fired)
    };

    let norm_identical = !n2_slot_mismatch
        && v_cmp.len() == r_cmp.len()
        && v_cmp.iter().zip(&r_cmp).all(|(a, b)| a.norm_eq(b));

    if norm_identical {
        // BENIGN: determine WHICH normalizers were responsible by re-diffing raw operand roles.
        let fired = which_normalizers_fired(v_raw, r_raw, v_norm, r_norm, opts, jit_fired);
        // Defensive: if raw differs but NO normalizer is credited and no JitEntry fired, that is a
        // classifier blind spot — treat as SEMANTIC rather than silently benign.
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
}
