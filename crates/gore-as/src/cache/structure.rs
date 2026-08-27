//! Structured decompiler (D3.3): CFG -> if/else + while/for + statements.
//!
//! Strategy (compiler-generated code is reducible): decompile each basic block to
//! NAME-based statements (`local_1 = local_1 + local_3;`), recover the terminator
//! condition from the trailing `CMP*` + conditional jump, then structure the
//! offset-ordered blocks by recognising the compiler idioms documented in
//! `work/reversing/gore-as/findings/decompile-controlflow.md`:
//!   - loop: `init; JMP test; body; test: <cmp> Jcc body` (bottom-test for/while)
//!   - if/else: `<cmp> Jcc else; then; JMP after; else: ...; after:`
//! Anything not recognised is emitted as labelled blocks with `goto` annotations —
//! never fails.

use std::collections::HashMap;
use std::fmt::Write as _;

use super::cfg::{self, Cfg};
use super::disasm::{disassemble, Instr};
use super::isa::BcType;
use super::refs::RefResolver;
use super::types::DataType;
use super::walk_modules::FuncCode;

/// A pushed call argument: its rendered text plus whether it originated from an integer
/// value (a constant or `int` slot), so it can be cast to the callee's expected `bool`/enum.
#[derive(Clone, Default, PartialEq)]
struct Arg {
    s: String,
    is_int: bool,
    /// Recovered base type name of the value (e.g. `FGameplayTag`, `AGothicCharacter`), when
    /// known — lets call sites detect an arg whose type can't match the callee parameter.
    ty: Option<String>,
    /// Raw bits if this is an integer CONSTANT (`PshC4`/`PshC8`) — so a constant feeding a
    /// float/double parameter can be reinterpreted as its real IEEE-754 value.
    cbits: Option<ConstBits>,
    /// True when this arg was pushed via `PSF` (push stack-frame address of a local) — i.e. it
    /// is the ADDRESS of a slot, used as an out / RVO / in-place-ctor receiver. Lets the CALLSYS
    /// handler recover `slot = T(args)` from a `$beh0` construct behaviour instead of dropping it.
    is_psf: bool,
    /// True for a synthesized implicit-context marker (`__WorldContext` generator global): it
    /// occupies a stack slot so the split arithmetic matches the native's real frame (whose arity
    /// counts the hidden `WorldContextObject`), but it is NOT a source-level arg — build_call
    /// strips it after collection and reduces the render arity accordingly.
    is_ctx: bool,
    /// True for an int-slot arg whose value provably came from a MEMBER READ (RDR* after a
    /// member-ref load) — a real data value (an XP amount, a config int), not a stranded SetV
    /// temporary. The nested-call retain keeps these (`GiveExperience(hero, local_23)` must not
    /// lose its `this.XP_...`-sourced Amount to an intervening `GetHero()`).
    keep: bool,
    /// batch-25a (G2): ENUM value type of a NATIVE struct's field, from the ADDSi arm's
    /// `native_field_type` lookup — carried ONLY so PopRPtr can hand it to the WRTV1 guard
    /// (scoped option: it is deliberately NOT merged into `ty`, so call-arg/argtype gates
    /// never see it).
    nfty: Option<String>,
    /// batch-32d: base value type of a CONST object-handle NATIVE field
    /// (`refs::native_field_const_object`) — carried ONLY so the RefCpyV arm can emit the
    /// CONSTSTORE marker when such a member read is copied into a same-typed local
    /// (`const UItemDefinition` -> `UItemDefinition` implconv, CharacterAI_Gothic:3002).
    /// Like `nfty`, deliberately NOT merged into `ty`.
    nf_const: Option<String>,
    /// batch-27 (Cast-diamond carry): pushed by a plain slot/const/global push opcode in
    /// `block_stmts` — safe to carry across a recognized Cast diamond; never set for
    /// pending-call-result pushes (`PshRPtr`) or synthetic pushes.
    carryable: bool,
    /// batch-33d: the carried render is a CONTAINER READ (the batch-32c/33c pure-elem
    /// pendings) whose re-evaluation at the join is only proven safe under the CLASSIC
    /// D7-constrained (opCast+TYPEID) arms. RELAXED guarded-assignment arms may contain
    /// arbitrary calls that could mutate the container, so such entries must not pass the
    /// relaxed carry gate. Static-name FName literals stay `false` (pure literals).
    reeval: bool,
}
impl Arg {
    fn int(s: String) -> Arg {
        Arg {
            s,
            is_int: true,
            ..Default::default()
        }
    }
    fn iconst(s: String, cbits: ConstBits) -> Arg {
        Arg {
            s,
            is_int: true,
            cbits: Some(cbits),
            ..Default::default()
        }
    }
    fn obj(s: String) -> Arg {
        Arg {
            s,
            ..Default::default()
        }
    }
    fn typed(s: String, ty: Option<String>) -> Arg {
        Arg {
            s,
            ty,
            ..Default::default()
        }
    }
    /// A `PSF`-pushed slot address (out / RVO / in-place-ctor receiver), carrying the slot's
    /// recovered type so the construct can render `slot = <ty>(args)`.
    fn psf(s: String, ty: Option<String>) -> Arg {
        Arg {
            s,
            ty,
            is_psf: true,
            ..Default::default()
        }
    }
    /// A synthesized implicit `__WorldContext` marker (see `is_ctx`).
    fn ctx() -> Arg {
        Arg {
            is_ctx: true,
            ..Default::default()
        }
    }
    /// Tag this arg as originating from a plain push opcode (see `carryable`). Applied at the
    /// `block_stmts` push SITES, deliberately not inside the constructors — `build_call` also
    /// constructs Args, which must never be carried.
    fn carry(mut self) -> Arg {
        self.carryable = true;
        self
    }
}

const AS_PTR_SIZE: i32 = 2;
/// Placeholder for a value the decompiler couldn't resolve (e.g. PshRPtr with no live
/// register). Statements that would emit it are dropped rather than producing bad source.
const UNRESOLVED: &str = "\u{1}unresolved";
/// An ARGMISMATCH sentinel (`\u{2}<code>`) tagged with a short cause code: marks a statement
/// the decompiler couldn't recover, forcing the stub fallback. Distinct from UNRESOLVED so it
/// is NOT stripped by the unresolved-statement retain (it must survive to reach the emitter);
/// the code is extracted for stub-reason aggregation.
fn amm(code: &str) -> String {
    format!("\u{2}{code}")
}
/// Sentinel for a non-void RET whose return value couldn't be recovered. Unlike an ARGMISMATCH
/// this does NOT stub the whole function: the emitter replaces it with a default-valued
/// return so the recovered body survives (far more faithful than a bare stub).
pub(crate) const RVODEF: &str = "\u{3}rvodef";
/// Marks a store whose RHS is a CONST object handle of the destination local's EXACT type
/// (batch-21 Class C). The emitter strips the marker and declares the destination local
/// `const T` — the vanilla form; a same-type `Cast<T>` wrap does NOT strip const in-game.
pub(crate) const CONSTSTORE: char = '\u{4}';

/// batch-25g: call-argument slot hints derived on the emit side by pairing value-pushed
/// slots with the callee's declared parameter types (`infer_slot_types`' stack model):
/// - `float_slots`: slots pushed into a FLOAT-FAMILY by-value parameter — their `SetV*`
///   constants are IEEE-754 bits (the SavePuzzleState switch cases wrote `local_1 =
///   1073741824;` = 2.0f) and extend the float-literal render set;
/// - `keep_ints`: slots pushed into an INT-FAMILY by-value parameter — REAL args a nested
///   call's stack-split retain must not purge (`Math::Min(a, <nested call>)` lost `a`,
///   spec batch23-nomatch.md family G; the float member of the same purge family was the
///   SaveWorldFloatData payload).
#[derive(Default)]
pub struct ArgSlotHints {
    pub float_slots: std::collections::HashSet<i32>,
    pub keep_ints: std::collections::HashSet<i32>,
}

/// Structured statement body for a function (no signature wrapper), indented at `depth`.
/// Returns an error annotation string on disasm failure (never panics).
pub fn body_statements(f: &FuncCode, refs: &RefResolver, depth: usize) -> String {
    body_statements_ctor(f, refs, depth, None, None, None, None, None, None, None)
}

/// Like [`body_statements`], but with class context for type-aware casts:
/// - `super_ctor`: super class name, so a call to its ctor on `this` -> `super(...)`.
/// - `ret_ty`: the function's return type, so `return <int>` casts to `bool`/enum.
/// - `fields`: the owning class's field name -> base type name, so a `this.field = <int>`
///   assignment casts the RHS to a `bool`/enum field.
#[allow(clippy::too_many_arguments)]
pub fn body_statements_ctor(
    f: &FuncCode,
    refs: &RefResolver,
    depth: usize,
    super_ctor: Option<&str>,
    ret_ty: Option<&DataType>,
    fields: Option<&HashMap<String, String>>,
    param_types: Option<&[String]>,
    class_name: Option<&str>,
    local_types: Option<&HashMap<i32, String>>,
    hints: Option<&ArgSlotHints>,
) -> String {
    let instrs = match disassemble(&f.bytecode) {
        Ok(i) => i,
        Err(e) => return format!("{}// disasm error: {e}\n", "    ".repeat(depth)),
    };
    let g = cfg::build(&instrs);
    let mut float_slots = float_operand_slots(&instrs, ret_ty);
    // batch-25g: slots pushed into a float-family by-value parameter carry IEEE-754 bits in
    // their SetV* constants too (evidence = the callee's declared signature).
    if let Some(h) = hints {
        float_slots.extend(h.float_slots.iter().copied());
    }
    // AS_PTR_SIZE-aware frame-offset -> param-index map (2-dword handles/refs + hidden RVO slot),
    // self-correcting on observed offsets. Built once per function; consulted by slot_name/slot_type.
    let (param_off_map, rvo_off) = super::decompile::build_param_off_map_rvo(f, &instrs, refs);
    let keep_ints = hints.map(|h| &h.keep_ints);
    let ctx = Ctx {
        f,
        refs,
        instrs: &instrs,
        super_ctor,
        ret_ty,
        fields,
        param_types,
        class_name,
        local_types,
        float_slots,
        param_off_map,
        rvo_off,
        keep_ints,
        rvo_switch_region: std::cell::Cell::new(false),
    };
    let idx_of: HashMap<usize, usize> = g
        .blocks
        .iter()
        .enumerate()
        .map(|(i, b)| (b.start_dw, i))
        .collect();
    let mut body = String::new();
    let mut st = Structurer {
        ctx: &ctx,
        g: &g,
        idx_of: &idx_of,
        exit_join: None,
        exit_join_is_ret: false,
        exit_ret_rows_ok: false,
        exit_rvo_return: false,
        exit_mixed_rvo_ret_rows_ok: false,
        exit_scan_floor: 0,
        carry: None,
        loop_scope: None,
    };
    st.emit_range(0, g.blocks.len(), depth, &mut body);
    body
}

/// If `v` is a top-level assignment expression `lhs = rhs` (the RVO return-slot write pattern),
/// return the RHS — `return lhs = rhs;` is a parse error, the RHS is the real returned value.
fn strip_return_assign(v: &str) -> &str {
    let b = v.as_bytes();
    let mut depth = 0i32;
    let mut i = 0;
    while i + 2 < b.len() {
        match b[i] {
            b'(' | b'[' => depth += 1,
            b')' | b']' => depth -= 1,
            b' ' if depth == 0 && b[i + 1] == b'=' && b[i + 2] == b' ' => return v[i + 3..].trim(),
            _ => {}
        }
        i += 1;
    }
    v
}

/// batch-43 (Fix 2, switch-rvo-return.md §4.2): fold a struct-RVO case region's trailing
/// out-slot store + synthetic return into a single per-case value return. The region text was
/// emitted with `exit_rvo_return` set, so its last two meaningful lines are
/// `<ws>__return = <val>;` then `<ws>return __return;`. Replace both with `<ws>return <val>;`.
///
/// Returns `Some(folded_text)` when the fold applied — which DOUBLES as gate G-RVO's per-region
/// out-slot-write proof: a `return __return;` with NO immediately-preceding `__return = <val>;`
/// store (and no resolved `<val>`) means the region does NOT provably write the out-slot on this
/// path, so the caller MUST bail the whole switch (never emit a case that silently returns a
/// default struct). `None` = proof failed / no fold site → bail.
fn fold_rvo_return(region: &str) -> Option<String> {
    let lines: Vec<&str> = region.lines().collect();
    // locate the synthetic `return __return;` (there must be exactly the tail exit)
    let ret_idx = lines
        .iter()
        .rposition(|l| l.trim_end() == "return __return;" || l.trim() == "return __return;")?;
    if ret_idx == 0 {
        return None; // no preceding store line at all
    }
    let store = lines[ret_idx - 1];
    let ws: String = store.chars().take_while(|c| c.is_whitespace()).collect();
    let body = store.trim();
    // the preceding line MUST be `__return = <val>;` with a resolved, non-empty, non-sentinel val
    let val = body.strip_prefix("__return = ")?.strip_suffix(';')?;
    if val.is_empty()
        || val.contains('\u{1}')
        || val.contains('\u{2}')
        || val == UNRESOLVED
        || val.contains("__return")
        || val.contains(RVODEF)
    {
        return None;
    }
    let mut out: Vec<String> = lines[..ret_idx - 1].iter().map(|s| s.to_string()).collect();
    out.push(format!("{ws}return {val};"));
    for l in &lines[ret_idx + 1..] {
        out.push(l.to_string());
    }
    // preserve a trailing newline (writeln! always leaves one)
    Some(format!("{}\n", out.join("\n")))
}

/// Return the resolved RHS of exactly one synthetic hidden-RVO-slot store in `stmts`.
///
/// A mixed switch can have both ordinary `break -> JOIN` paths and early struct-value returns
/// lowered as `__return = value; ...cleanup...; JMP shared_RET`.  Such a bare-RET edge is safe to
/// recover only when its own source block proves the out-slot write.  Requiring exactly one clean
/// store keeps the proof local and fail-closed: a missing, duplicate, unresolved, or self-referent
/// write leaves the whole switch on the existing stub path.
fn single_clean_rvo_store(stmts: &[String]) -> Option<&str> {
    let mut value = None;
    for stmt in stmts {
        let t = stmt.trim();
        if !t.starts_with("__return =") {
            continue;
        }
        let val = t.strip_prefix("__return = ")?.strip_suffix(';')?.trim();
        if value.is_some()
            || val.is_empty()
            || val.contains('\u{1}')
            || val.contains('\u{2}')
            || val == UNRESOLVED
            || val.contains("__return")
            || val.contains(RVODEF)
        {
            return None;
        }
        value = Some(val);
    }
    value
}

/// batch-27b: if `v` is a top-level assignment expression `lhs = rhs` with a PLAIN slot /
/// member-path lhs (the batch-26 operator/RVO fold shapes: `local_4 = this.GetDisplayName()`,
/// `local_18 = (a + b)`), return the lhs. The in-game compiler rejects an assignment
/// expression consumed as a call argument ("[E] No matching signatures to
/// 'FString::Append(local_4 = FString)'", capture.batch26-0705 CombatMoves.as), so the
/// `PshRPtr` arm flushes such a pending as its own statement and pushes the (now-written)
/// lhs slot instead. Comparisons never match (`==`/`!=`/`<=`/`>=` are not ` = `); anything
/// with a non-identifier lhs (literal, string, expression) returns None.
fn assign_lhs(v: &str) -> Option<&str> {
    let b = v.as_bytes();
    let mut depth = 0i32;
    for i in 0..b.len().saturating_sub(2) {
        match b[i] {
            b'(' | b'[' => depth += 1,
            b')' | b']' => depth -= 1,
            // a quote before the top-level ` = ` -> lhs would contain a string literal; the
            // depth counter does not model quoting, so never match past one.
            b'"' => return None,
            b' ' if depth == 0 && b[i + 1] == b'=' && b[i + 2] == b' ' => {
                let lhs = v[..i].trim();
                let plain = !lhs.is_empty()
                    && lhs
                        .bytes()
                        .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'.');
                return plain.then_some(lhs);
            }
            _ => {}
        }
    }
    None
}

/// batch-37 (Idiom S, ctor-member-init.md §4.1): true iff `arg` is a clean ASSIGNABLE member/
/// local lvalue — a `this.<field>` (or deeper `this.a.b`) member path, or a bare `local_N` slot.
/// Used to gate the ctor value-type member-default-init recovery: the `$beh0` copy/value-init
/// behaviour whose receiver is such an lvalue is `this.<field> = <value>;`, not a temp construct.
/// FALSE for: bare `this` (that is the whole-object copyctor stub, which must stay authoritative),
/// the hidden `__return` RVO slot (handled by the `$beh0(__return, src)` return arm above), a PSF
/// address, and any `$`/`~`/`\u{2}`/UNRESOLVED behaviour/sentinel marker. Same "plain lhs"
/// character rule as `assign_lhs` (identifiers, `_`, `.`), so no expression/literal can slip in.
fn is_lvalue_arg(arg: &Arg) -> bool {
    if arg.is_psf {
        return false;
    }
    let s = arg.s.as_str();
    // reject sentinels / behaviour markers / empties outright.
    if s.is_empty()
        || s == UNRESOLVED
        || s.starts_with('$')
        || s.starts_with('~')
        || s.contains('\u{2}')
        || s.contains('\u{1}')
    {
        return false;
    }
    // bare `this` = whole-object copyctor (no source form); `__return` = RVO slot (own arm).
    if s == "this" || s == "__return" {
        return false;
    }
    // plain member/local path: identifier chars + `.` only, and the HEAD must be an assignable
    // root (`this`, a `local_` slot, or a plain identifier param) — never a call/index/literal.
    if !s
        .bytes()
        .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'.')
    {
        return false;
    }
    let head = s.split('.').next().unwrap_or(s);
    (s.starts_with("this.")) || head.starts_with("local_") || {
        // a bare param/slot name (`Foo`, `local_3`) with no `.` is assignable; a bare `this`
        // was already rejected. A dotted head that is a plain identifier (a param.member) is
        // also assignable. Require the head be a valid identifier start (not a digit).
        head.bytes()
            .next()
            .is_some_and(|b| b.is_ascii_alphabetic() || b == b'_')
    }
}

/// A `$beh0` value copy-constructor whose source is a PSF slot is recoverable only when every
/// available type witness agrees: one parameter, PSF receiver/source, identical recovered slot
/// types, and a behavior owner equal to that type. This is the compiler's
/// `PSF source; PSF destination; CALLSYS T::$beh0(const T&)` local-copy lowering. Any missing
/// or conflicting witness stays on the historical drop path.
fn is_proven_same_type_psf_copy(
    recv: &Arg,
    args: &[Arg],
    owner: Option<&str>,
    param_count: Option<usize>,
) -> bool {
    let Some(dst_ty) = recv.ty.as_deref() else {
        return false;
    };
    recv.is_psf
        && param_count == Some(1)
        && owner == Some(dst_ty)
        && matches!(args, [src] if src.is_psf && src.ty.as_deref() == Some(dst_ty))
}

/// Decompile a function to a self-contained `function(...) { ... }` (readable, not recompilable).
pub fn decompile(f: &FuncCode, refs: &RefResolver) -> String {
    let params: Vec<String> = f
        .param_names
        .iter()
        .enumerate()
        .map(|(i, n)| {
            if n.is_empty() {
                format!("arg{i}")
            } else {
                n.clone()
            }
        })
        .collect();
    // Pass the return type so RVO / value-return functions decompile faithfully in the CLI
    // (the ret_ty-less `body_statements` path renders `return;` for a struct-by-value return,
    // masking the emitter's real `return <rvo_local>;`). Diagnostic-only: the authoritative
    // emitter is emit.rs (which already threads `Some(&f.ret)`); this only aligns the readable
    // `as decompile` view with it. Other context (fields/class/hints) stays None as before.
    let body = body_statements_ctor(f, refs, 1, None, Some(&f.ret), None, None, None, None, None)
        .replace(RVODEF, "/* unrecovered */ {}")
        .replace(CONSTSTORE, "");
    format!(
        "// {}\nfunction({})\n{{\n{}}}\n",
        f.func,
        params.join(", "),
        body
    )
}

struct Ctx<'a> {
    f: &'a FuncCode,
    refs: &'a RefResolver,
    instrs: &'a [Instr],
    super_ctor: Option<&'a str>,
    ret_ty: Option<&'a DataType>,
    fields: Option<&'a HashMap<String, String>>,
    param_types: Option<&'a [String]>,
    class_name: Option<&'a str>,
    local_types: Option<&'a HashMap<i32, String>>,
    /// Slots that appear as an operand of a float/double arithmetic or compare op, so a
    /// `SetV4`/`SetV8` constant written to one is rendered as a float literal, not raw int bits.
    float_slots: std::collections::HashSet<i32>,
    /// AS_PTR_SIZE-aware frame-offset -> parameter-index map (see `model::param_slot_map`).
    param_off_map: HashMap<i32, usize>,
    /// Frame offset of the hidden RVO out-pointer slot for a by-value-struct return, if any.
    /// A `CopyScript` writing this slot is the function's `return <src>`.
    rvo_off: Option<i32>,
    /// batch-25g: slots value-pushed into a KNOWN int-family by-value parameter (see
    /// [`ArgSlotHints::keep_ints`]) — pushed with `keep` so the nested-call stack-split
    /// retain spares them.
    keep_ints: Option<&'a std::collections::HashSet<i32>>,
    /// batch-43 (Fix 2): set ONLY while emitting a struct-RVO switch's per-case region (the
    /// bare-RET-join shape, GetDebugColor). While set, a `$beh0(__return, src)` in a JMP-
    /// terminated block emits `__return = src;` as a STATEMENT (so the switch's per-case
    /// `return <val>;` fold recovers it), instead of stashing it in the block-local `ret_val`
    /// that a JMP-terminated block would drop. OUTSIDE a switch RVO region it stays the byte-
    /// identical `ret_val` path — this keeps batch-43 strictly switch-scoped (the general
    /// branch/loop RVO-return recovery is the deferred Fix 3 / loop-body-cfg lever).
    rvo_switch_region: std::cell::Cell<bool>,
}

/// Collect slots used as an operand of a float/double arithmetic or compare op. Every word
/// operand of those ops is a float/double value, so a constant feeding such a slot is float.
/// Additionally, a slot copied into the VALUE REGISTER (`CpyVtoR4`/`CpyVtoR8`) in a function
/// whose return type is the matching-width float family IS the float return payload, so its
/// `SetV*` constants are IEEE-754 bits too (e.g. the per-case `SetV8 w4, 0xc04b...` returns
/// in `GetScanSweepAngleDeg` are -55.0, not -4590434657685733376).
fn float_operand_slots(
    instrs: &[Instr],
    ret_ty: Option<&DataType>,
) -> std::collections::HashSet<i32> {
    let is_float_op = |n: &str| {
        matches!(
            n,
            "ADDf"
                | "SUBf"
                | "MULf"
                | "DIVf"
                | "MODf"
                | "NEGf"
                | "IncVf"
                | "DecVf"
                | "ADDIf"
                | "SUBIf"
                | "MULIf"
                | "CMPf"
                | "CMPIf"
                | "ADDd"
                | "SUBd"
                | "MULd"
                | "DIVd"
                | "MODd"
                | "NEGd"
                | "CMPd"
        )
    };
    let mut slots = std::collections::HashSet::new();
    let ret_tok = ret_ty.map(|t| t.token);
    for ins in instrs {
        if is_float_op(ins.op.name) {
            for &wd in &ins.words {
                slots.insert(wd as i16 as i32);
            }
        }
        // return-register copies in a float-returning function (width-matched: 0x51 `float`
        // and 0x5E `double` are 8-byte in this fork, 0x50 `float32` is 4-byte).
        let ret_float_copy = match ins.op.name {
            "CpyVtoR8" => matches!(ret_tok, Some(0x51) | Some(0x5E)),
            "CpyVtoR4" => ret_tok == Some(0x50),
            _ => false,
        };
        if ret_float_copy {
            if let Some(&wd) = ins.words.first() {
                slots.insert(wd as i16 as i32);
            }
        }
    }
    slots
}

impl Ctx<'_> {
    fn slot_name(&self, off: i32) -> String {
        if off > 0 {
            return format!("local_{off}");
        }
        if self.f.is_method && off == 0 {
            return "this".into();
        }
        // The hidden RVO out-pointer slot is NOT a parameter — naming it via the param fallback
        // mislabels it as parameter 0 (e.g. `_QuestClass`). Give it a distinct synthetic name so
        // a stray reference is recognisable; the CopyScript-to-return rewrite normally consumes it.
        if self.rvo_off == Some(off) {
            return "__return".into();
        }
        self.param_or_arg(self.param_idx(off))
    }

    /// Recovered base type name for a slot: object-local type for `off > 0`, parameter type
    /// for params; None for `this` / unknowns.
    fn slot_type(&self, off: i32) -> Option<String> {
        if off > 0 {
            return self.local_types.and_then(|m| m.get(&off)).cloned();
        }
        let idx = self.param_idx(off);
        self.param_types
            .and_then(|p| p.get(idx))
            .filter(|t| !t.is_empty())
            .cloned()
    }

    /// Resolve a negative frame offset to its parameter index via the AS_PTR_SIZE-aware map,
    /// falling back to the old linear formula for an unmapped (variadic/defaulted-tail) slot.
    fn param_idx(&self, off: i32) -> usize {
        if let Some(&idx) = self.param_off_map.get(&off) {
            return idx;
        }
        if self.f.is_method {
            (-off - AS_PTR_SIZE) as usize
        } else {
            (-off) as usize
        }
    }

    /// Render a `return` statement from an optional recovered value, applying the same
    /// fix-ups as the `RET` arm (RVO `return slot = v` stripping, declared-bool bareness,
    /// int -> bool/enum return casts, RVODEF default for an unrecovered non-void value).
    /// Shared by the `RET` opcode arm and the switch-recovery `JMP -> RET-row` return exits.
    fn return_stmt(&self, v: Option<String>) -> String {
        let non_void = self.ret_ty.map(|t| t.token != 0x52).unwrap_or(false);
        if !non_void {
            return "return;".into();
        }
        match v {
            Some(v) => {
                let v = strip_return_assign(&v).to_string();
                let declared_ty = v
                    .strip_prefix("local_")
                    .and_then(|d| d.parse::<i32>().ok())
                    .and_then(|n| self.slot_type(n));
                let v = match self.ret_ty {
                    Some(rt) if looks_int(&v) => {
                        let tn = if rt.token == 0x41 {
                            "bool".to_string()
                        } else {
                            rt.base_name(self.refs)
                        };
                        if declared_ty.as_deref() == Some(tn.as_str()) {
                            v
                        } else {
                            cast_to_typename(&v, &tn, self.refs).unwrap_or(v)
                        }
                    }
                    _ => v,
                };
                // batch-25b (G4 RET shape, specs/batch23-nomatch.md): `return local_N;` where
                // the slot's recovered type is a PROVABLY less-derived object type than the
                // function's return type (a covariant-erased producer: `UCBT_Tree local` vs
                // `UCBT_Tree_SelectAttackBase` return) fails "Can't implicitly convert".
                // Wrap in the game's own null-safe downcast: `return Cast<RetTy>(local_N);`.
                // Gate: bare local slot, BOTH heads known object types (U*/A*), and the
                // return head provably derives from the slot head (script hierarchy, or the
                // engine axioms in `provably_derived`) — never wrapped when unsure.
                let v = match (
                    self.ret_ty,
                    v.strip_prefix("local_").and_then(|d| d.parse::<i32>().ok()),
                ) {
                    (Some(rt), Some(slot)) => {
                        let rh = rt.base_name(self.refs);
                        match self.slot_type(slot) {
                            Some(st) if provably_derived(&rh, &st, self.refs) => {
                                format!("Cast<{}>({v})", qualify_class_name(&rh, self.refs))
                            }
                            _ => v,
                        }
                    }
                    _ => v,
                };
                format!("return {v};")
            }
            None => format!("return {RVODEF};"),
        }
    }

    /// True when the return VALUE travels through the hidden RVO out-pointer slot (a genuine
    /// by-value struct return) — so the value/object registers can never carry the payload
    /// (batch-24a, specs/batch23-cantconvert.md G1). `rvo_off` alone is NOT sufficient: the
    /// param-map heuristic also classifies ENUM returns (token 5) as RVO, yet enums return in
    /// the value register — mirroring the switch recovery's `register_based` test, an enum
    /// return stays register-based here too (else every enum function's CpyVtoR* return
    /// capture would be discarded).
    fn ret_via_rvo(&self) -> bool {
        self.rvo_off.is_some()
            && !self
                .ret_ty
                .map(|t| is_enum_name(&t.base_name(self.refs)))
                .unwrap_or(true)
    }

    /// True when the function returns a value BY REFERENCE (`T& f()`): the return payload is a
    /// live lvalue (a member-container element like `this.RoleGroups[i]`), not an RVO struct
    /// copy or a register scalar. batch-35a: a ref-returning function whose RET row is a shared
    /// bare `RET wN` fed by an opIndex/member chain built in EACH predecessor block loses that
    /// chain at the block boundary (block_stmts flushes a live pending as a bare statement, and
    /// the bare-RET scan-back then defaults to a garbage int slot -> "Not a valid reference").
    /// Used to opt those predecessor blocks into rendering their trailing ref-pending as
    /// `return <chain>;` (the cross-block reference carry).
    fn ret_is_ref(&self) -> bool {
        self.ret_ty
            .map(|t| t.token != 0x52 && t.is_reference)
            .unwrap_or(false)
    }

    /// The frame slot that carries this function's RETURN value OUT (batch-43, Fix 1):
    /// - struct-by-value RVO: the hidden `__return` out-slot (`self.rvo_off`);
    /// - object/scalar/enum: the slot loaded into the return register by the terminal
    ///   `LOADOBJ`/`CpyVtoR*` IMMEDIATELY before the function's final `RET` (e.g.
    ///   `PopBucketFront`'s `LOADOBJ w2; RET` → slot 2). `None` when the terminal return has
    ///   no such single-slot fill (a plain `RET`, or a computed/expression return).
    ///
    /// Used only to widen a copy-INTO-the-return-value (`RefCpyV`) — a phantom copy into a
    /// normal local stays dropped; a copy into THIS slot IS the return-value write and must
    /// survive. Conservative: matches at most one slot, from the function's own tail.
    fn return_out_slot(&self) -> Option<i32> {
        if let Some(off) = self.rvo_off {
            if self.ret_via_rvo() {
                return Some(off);
            }
        }
        // scan for the function's final RET; the instr before it must be a single-slot
        // return-register fill whose operand names the out-slot.
        let n = self.instrs.len();
        if n < 2 {
            return None;
        }
        let last = &self.instrs[n - 1];
        if last.op.name != "RET" {
            return None;
        }
        let prev = &self.instrs[n - 2];
        if matches!(
            prev.op.name,
            "LOADOBJ" | "CpyVtoR4" | "CpyVtoR8" | "CpyVtoR1"
        ) {
            if let Some(&wd) = prev.words.first() {
                return Some(wd as i16 as i32);
            }
        }
        None
    }

    fn is_return_out_slot(&self, slot: i32) -> bool {
        self.return_out_slot() == Some(slot)
    }

    /// Name for parameter slot `idx`: the stored name, else `arg{idx}` — which MUST match
    /// how `emit::render_params` declares unnamed params (also `arg{idx}`), so a body
    /// reference resolves to a declared parameter.
    fn param_or_arg(&self, idx: usize) -> String {
        if let Some(n) = self.f.param_names.get(idx) {
            if !n.is_empty() {
                return n.clone();
            }
        }
        format!("arg{idx}")
    }

    /// batch-45a (FIX-1): true when `name` is EXACTLY a declared parameter (by rendered name)
    /// whose type is a NON-CONST object handle or reference — the only source a REFCPY
    /// object-store may accept beyond `local_`/`Cast<`/`nullptr`. A REFCPY copies a handle, so
    /// the param must be an object handle or a reference (`this.m_Target = NewTarget;`); a const
    /// param (`is_read_only`/`is_object_const`) is EXCLUDED (copying it into a non-const member
    /// fails generate-mode "Can't implicitly convert 'const T' to 'T'" — the batch-41d cascade
    /// class, and the documented AddFitnessMultiplier-style const-param bail). Matches against the
    /// same names `param_or_arg` renders, so a body `src.s` resolves back to its param slot. `this`
    /// is not a param name, so it never matches here (its back-link store stays bailed).
    fn param_src_ok(&self, name: &str) -> bool {
        // Locate the param by its rendered name (stored name, or the `arg{idx}` fallback that
        // `param_or_arg` emits for an unnamed param).
        let idx = self
            .f
            .param_names
            .iter()
            .position(|n| !n.is_empty() && n == name)
            .or_else(|| {
                name.strip_prefix("arg")
                    .and_then(|d| d.parse::<usize>().ok())
                    .filter(|&i| {
                        i < self.f.param_types.len()
                            && self
                                .f
                                .param_names
                                .get(i)
                                .map(|n| n.is_empty())
                                .unwrap_or(false)
                    })
            });
        let Some(idx) = idx else { return false };
        let ty = match self.f.param_types.get(idx) {
            Some(t) => t,
            None => return false,
        };
        // Object handle OR reference (a REFCPY targets handles/refs), and NON-const.
        (ty.is_object_handle || ty.is_reference) && !ty.is_read_only && !ty.is_object_const
    }

    /// Whether the object PARAMETER `name` is const — read-only or a const handle. `param_src_ok`
    /// refuses those outright; a copy of one is still recoverable when its only consumer is a
    /// comparison, which const cannot break.
    fn param_is_const(&self, name: &str) -> bool {
        let idx = self
            .f
            .param_names
            .iter()
            .position(|n| !n.is_empty() && n == name);
        match idx.and_then(|i| self.f.param_types.get(i)) {
            Some(t) => t.is_read_only || t.is_object_const,
            None => false,
        }
    }

    /// batch-45c (FIX-4): true when `name` is EXACTLY a declared object/reference PARAMETER —
    /// const-AGNOSTIC (unlike `param_src_ok`), because the only use is a null-guard fold
    /// (`if (<param> == nullptr)`), a comparison that is const-safe. Never matches `this` (not a
    /// param name) or a `local_`/temp.
    fn param_object_ref(&self, name: &str) -> bool {
        let idx = self
            .f
            .param_names
            .iter()
            .position(|n| !n.is_empty() && n == name)
            .or_else(|| {
                name.strip_prefix("arg")
                    .and_then(|d| d.parse::<usize>().ok())
                    .filter(|&i| {
                        i < self.f.param_types.len()
                            && self
                                .f
                                .param_names
                                .get(i)
                                .map(|n| n.is_empty())
                                .unwrap_or(false)
                    })
            });
        let Some(idx) = idx else { return false };
        match self.f.param_types.get(idx) {
            Some(t) => t.is_object_handle || t.is_reference,
            None => false,
        }
    }
}

fn s16(w: u16) -> i32 {
    w as i16 as i32
}

/// Recover a non-void function's return value when the RET's own block produced none: scan
/// backwards for the nearest `CpyVtoR*`/`LOADOBJ` that filled the return register in a
/// dominating block, stopping at a previous RET so we never cross into an unrelated value.
fn scan_back_retval(ctx: &Ctx, before: usize) -> Option<String> {
    scan_back_retval_floor(ctx, before, 0)
}

/// [`scan_back_retval`] bounded below by `floor` (instruction index): used by the switch
/// recovery's synthesized `return`s so a case's value never leaks in from a PRECEDING case
/// region (the linear scan would otherwise cross the region boundary).
/// Which loop shape produced a condition, and what its test block carried, behind
/// `GORE_AS_LOOP_DIAG`.
fn loop_diag(path: &str, stmts: &[String], cond: &str) {
    if std::env::var_os("GORE_AS_LOOP_DIAG").is_some() {
        eprintln!("[loop] {path} cond={cond} stmts={stmts:?}");
    }
}

/// `local_N = <value>;` as a loop header's only statement, tested by `local_N != 0` or
/// `local_N == 0`: the loop's condition is that value, or its negation.
fn fold_loop_header_store(stmts: &[String], cond: &str) -> Option<String> {
    if stmts.is_empty() {
        return None;
    }
    let is_local = |name: &str| {
        name.strip_prefix("local_")
            .is_some_and(|rest| !rest.is_empty() && rest.bytes().all(|b| b.is_ascii_digit()))
    };
    // The header builds one value in one slot, in steps: read it, then negate it. Compose the
    // steps back into the expression they spell.
    let mut slot: Option<&str> = None;
    let mut value = String::new();
    for stmt in stmts {
        let (target, rhs) = stmt.trim().strip_suffix(';')?.split_once(" = ")?;
        if !is_local(target) || rhs.is_empty() || *slot.get_or_insert(target) != target {
            return None;
        }
        value = match count_word(rhs, target) {
            0 if value.is_empty() => rhs.to_owned(),
            // A step that reads the slot back continues the expression built so far.
            1 if !value.is_empty() => rhs.replace(target, &format!("({value})")),
            _ => return None,
        };
    }
    // A slot the tables call `bool` is tested bare; an int-declared one through `!= 0`. The
    // branch's own inversion can arrive as a double negation — `!(!(local_1))` is `local_1`.
    let slot = slot?;
    let mut cond = cond;
    while let Some(inner) = cond
        .strip_prefix("!(!(")
        .and_then(|rest| rest.strip_suffix("))"))
    {
        cond = inner;
    }
    // A BARE test says the slot is a bool, so the value it was built from is one too. A test
    // against zero says the opposite — the slot is an int, and the comparison carries the type.
    // Dropping it there writes `while (1)`, which the compiler refuses ("Expression must be of
    // boolean type"); an integer LITERAL is the one value that can lose it, as `true`/`false`.
    let literal = value.parse::<i64>().ok();
    match cond {
        _ if cond == slot => Some(value),
        _ if cond == format!("!{slot}") || cond == format!("!({slot})") => {
            Some(format!("!({value})"))
        }
        _ => match (cond.strip_prefix(slot)?, literal) {
            (" != 0", Some(n)) => Some(bool_literal(n != 0)),
            (" == 0", Some(n)) => Some(bool_literal(n == 0)),
            (" != 0", None) => Some(format!("({value}) != 0")),
            (" == 0", None) => Some(format!("({value}) == 0")),
            _ => None,
        },
    }
}

/// AngelScript's spelling of a boolean constant.
fn bool_literal(value: bool) -> String {
    match value {
        true => "true".to_owned(),
        false => "false".to_owned(),
    }
}

fn scan_back_retval_floor(ctx: &Ctx, before: usize, floor: usize) -> Option<String> {
    for i in (floor..before).rev() {
        let ins = &ctx.instrs[i];
        match ins.op.name {
            "CpyVtoR4" | "CpyVtoR8" | "CpyVtoR1" | "LOADOBJ" => {
                return Some(ctx.slot_name(ins.words.first().copied().map(s16).unwrap_or(0)));
            }
            "RET" => return None,
            _ => {}
        }
    }
    None
}

/// `local_N = <expr>;` immediately before `return local_N;` is one `return <expr>;`. The slot is
/// the compiler's own temporary there, so naming it costs a copy vanilla does not have. Folds
/// only when the value does not read the slot back.
fn fold_return_into_store(stmts: &mut Vec<String>, exit: String) -> String {
    let Some(slot) = exit
        .strip_prefix("return ")
        .and_then(|value| value.strip_suffix(';'))
        .filter(|value| value.starts_with("local_") && !value.contains(['.', '(', ' ', '[']))
    else {
        return exit;
    };
    let Some((target, value)) = stmts
        .last()
        .and_then(|last| last.trim().strip_suffix(';'))
        .and_then(|last| last.split_once(" = "))
    else {
        return exit;
    };
    if target.trim() != slot || value.contains(slot) {
        return exit;
    }
    let folded = format!("return {value};");
    stmts.pop();
    folded
}

/// The boolean value a `CMP*` + `T*` pair leaves in the value register.
fn materialized_comparison(c: &Cmp) -> Option<String> {
    if let Some(expr) = &c.expr {
        return Some(expr.clone());
    }
    let op = c.op?;
    (!c.a.is_empty() && !c.b.is_empty() && c.a != UNRESOLVED && c.b != UNRESOLVED)
        .then(|| format!("({} {op} {})", c.a, c.b))
}

/// Conditional-jump opcode (mirrors `cfg::is_cond_jump`, which is private to that module).
fn is_cond_op(n: &str) -> bool {
    matches!(
        n,
        "JZ" | "JNZ" | "JS" | "JNS" | "JP" | "JNP" | "JLowZ" | "JLowNZ"
    )
}

/// Resolve a Cast/TYPEID operand to a target typename. AngelScript bytecode typeids carry
/// flag bits in the high word — most notably `asTYPEID_OBJHANDLE` (0x40000000) and
/// `asTYPEID_HANDLETOCONST` (0x20000000) — that are NOT part of the key stored in the cache's
/// TypeIdReferenceToPointer table (which keys on the object-type id incl. SCRIPTOBJECT/TEMPLATE
/// bits but WITHOUT the handle/const flags). Strip those flags before lookup, trying the raw id
/// first for robustness. Returns the resolved typename, or None if unresolvable.
pub(crate) fn resolve_cast_typeid(refs: &RefResolver, tid: i32) -> Option<String> {
    const OBJHANDLE: u32 = 0x4000_0000;
    const HANDLETOCONST: u32 = 0x2000_0000;
    let raw = tid as u32;
    for cand in [raw, raw & !OBJHANDLE, raw & !(OBJHANDLE | HANDLETOCONST)] {
        if let Some(t) = refs.type_by_id(cand as i32) {
            return Some(t.to_string());
        }
    }
    None
}

/// A primitive numeric-conversion opcode (`dst = (cast) src`); the cast is implicit in
/// type-erased AngelScript source, so we render the plain copy `dst = src`.
fn is_numeric_cast(n: &str) -> bool {
    matches!(
        n,
        "iTOf"
            | "fTOi"
            | "uTOf"
            | "fTOu"
            | "iTOd"
            | "dTOi"
            | "uTOd"
            | "dTOu"
            | "fTOd"
            | "dTOf"
            | "iTOb"
            | "iTOw"
            | "sbTOi"
            | "swTOi"
            | "ubTOi"
            | "uwTOi"
            | "i64TOi"
            | "iTOi64"
            | "uTOi64"
            | "i64TOf"
            | "fTOi64"
            | "i64TOd"
            | "dTOi64"
            | "u64TOf"
            | "fTOu64"
            | "u64TOd"
            | "dTOu64"
    )
}

/// For a float/double -> integer narrowing cast, the AngelScript target type to make the
/// conversion explicit (`int(x)`); `None` for widenings/same-width casts that stay implicit.
fn narrowing_cast_target(n: &str) -> Option<&'static str> {
    Some(match n {
        "fTOi" | "dTOi" => "int",
        "fTOu" | "dTOu" => "uint",
        "fTOi64" | "dTOi64" => "int64",
        "fTOu64" | "dTOu64" => "uint64",
        // batch-28 (specs/batch27-floatwarnings.md §3): float(=f64) -> float32 narrows too —
        // the bare copy render was the whole C2 class ("Implicit conversion from float64 to
        // float32 loses precision"). Vanilla compiled clean, so the source carried this cast;
        // the dTOf in the bytecode IS that cast.
        "dTOf" => "float32",
        _ => return None,
    })
}

/// The raw bits of a `SetV*` constant written to a slot — so a later store into a float/
/// double field can reinterpret them as the real IEEE-754 value instead of an int literal.
#[derive(Clone, Copy, PartialEq)]
enum ConstBits {
    W4(u32),
    W8(u64),
}

/// Format raw constant bits as a float (`f` suffix) or double AngelScript literal.
fn fmt_float(b: ConstBits, double: bool) -> String {
    let v: f64 = match b {
        ConstBits::W4(x) => f32::from_bits(x) as f64,
        ConstBits::W8(x) => f64::from_bits(x),
    };
    // Rust's `{:?}` prints non-finite floats as `inf`/`-inf`/`NaN`, and the float32 branch then
    // appends `f` -> `inff`, neither of which is a valid AngelScript literal ("'inff' is not
    // declared"). AS has no inf/nan literal, so emit the closest compile-valid magnitude: the
    // type's max finite value for ±inf (preserves the "very large / forever" sentinel intent
    // these constants carry), and 0 for NaN.
    if !v.is_finite() {
        return match (v.is_nan(), v.is_sign_negative(), double) {
            (true, _, true) => "0.0".into(),
            (true, _, false) => "0.0f".into(),
            // An OVERFLOWING decimal literal is the closest thing to an infinity literal this
            // language has: it parses, and IEEE round-to-nearest takes it to ±inf, which is the
            // bit pattern vanilla holds. The type's max finite value — what this used to emit —
            // comes back one ULP low.
            (false, false, true) => "1e309".into(),
            (false, true, true) => "-1e309".into(),
            (false, false, false) => "1e39f".into(),
            (false, true, false) => "-1e39f".into(),
        };
    }
    if double {
        format!("{v:?}")
    } else {
        format!("{:?}f", v as f32)
    }
}

/// Render the constant last written to `slot` as a float/double literal. None if untracked.
fn float_lit(m: &HashMap<i32, ConstBits>, slot: i32, double: bool) -> Option<String> {
    Some(fmt_float(*m.get(&slot)?, double))
}

/// A recovered branch condition: either a binary comparison (`a <op> b`) or a single boolean
/// `expr` (a bool slot / call result tested by JLowZ).
#[derive(Clone, Default)]
struct Cmp {
    a: String,
    b: String,
    op: Option<&'static str>, // relational op from a T* test (overrides the jump-derived one)
    expr: Option<String>,     // a complete boolean condition (when set, a/b are ignored)
    expr_bool: bool,          // `expr` is already bool-typed (render bare, don't wrap `!= 0`)
}

/// Build a call expression from the pushed-arg stack. For a method, the receiver is the
/// last-pushed entry (top of stack); the rest are args in push order. Operator-overload
/// methods (opAssign/opAdd/opEquals/...) render as the source operator. Returns None for
/// compiler-generated behaviors ($behN construct/destruct) that have no source form.
///
/// `Thiscall1` is a Hazelight VM opcode with a fixed stack delta of -3 dwords: one
/// 32-bit operand plus the two-dword receiver pointer. That physical frame is independent
/// of the source declaration. In particular, template helpers such as `TArray::Last()`
/// have zero rendered parameters but still push the default `IndexFromEnd = 0` operand.
/// Use this physical arity for the nested-call stack split while leaving the declaration /
/// Binds arity in charge of which operands are rendered at source level.
fn call_frame_arity(opcode: &str, declared: Option<usize>) -> Option<usize> {
    if opcode == "Thiscall1" {
        Some(1)
    } else {
        declared
    }
}

/// Remove one call's physical frame from the top of the abstract operand stack, preserving
/// deeper operands that belong to an enclosing call. Literal constants and proven member
/// reads are valid deferred enclosing arguments; untracked int-slot temporaries are not.
fn take_call_frame(stack: &mut Vec<Arg>, need: Option<usize>) -> Vec<Arg> {
    match need {
        Some(k) if stack.len() > k => {
            let own = stack.split_off(stack.len() - k);
            stack.retain(|x| !x.is_int || x.cbits.is_some() || x.keep);
            own
        }
        _ => std::mem::take(stack),
    }
}

#[allow(clippy::too_many_arguments)]
fn build_call(
    stack: &mut Vec<Arg>,
    f: &str,
    is_method: bool,
    super_ctor: Option<&str>,
    params: Option<&[DataType]>,
    native_arity: Option<usize>,
    trusted_arity: Option<usize>,
    target_owner: Option<&str>,
    cur_class: Option<&str>,
    non_virtual: bool,
    ret_ty: Option<&str>,
    ret_is_ref: bool,
    global_shadowed: bool,
    refs: &RefResolver,
) -> Option<String> {
    // The callee's declared default arguments, so a call can be rendered the way it was
    // written rather than the way it was compiled.
    let arg_defaults = target_owner
        .or(cur_class)
        .and_then(|owner| refs.param_defaults(owner, f))
        .or_else(|| refs.param_defaults("", f));
    if f.starts_with('$') || f.starts_with('~') || f == "__STATIC_NAME" {
        // EDIT C (the dominant FName form): `__STATIC_NAME` is the synthesized name-table accessor
        // (`const FName& __STATIC_NAME(int Id)` per the exe's registered decl) that fetches
        // `FAngelscriptManager::StaticNames[Id]` — populated at load from the cache's StaticNames
        // tail table (table 5) — into the return register; a following `PshRPtr` re-pushes it as
        // the ENCLOSING call's arg. Dropping it (return None) loses that arg → `this.GetCharacter()`
        // (0-arg). Resolve the pushed Id through RefResolver::static_name and render the UE-AS
        // FName literal `n"Name"` so it FLOWS into `pending` and PshRPtr restores both ARITY and
        // VALUE (`this.GetCharacter(n"Hero")`). AS FName has NO int ctor, so the historical
        // `FName(<idx>)` render never compiled ("No matching signatures to 'FName(const int)'");
        // it remains only as the arity-preserving fallback for an unresolvable Id (non-constant
        // operand or out-of-range index, e.g. a mini-cache with empty tail tables).
        // Gate: exactly one int operand on top (the Id).
        if f == "__STATIC_NAME" {
            if let Some(top) = stack.last() {
                if top.is_int && !top.s.is_empty() {
                    let idx = stack.pop().unwrap();
                    // constant bits first (authoritative for PshC4), rendered text as fallback
                    // (covers a tracked int slot whose rendered form is the decimal value).
                    let id = match idx.cbits {
                        Some(ConstBits::W4(b)) => Some(b as i32 as i64),
                        Some(ConstBits::W8(b)) => Some(b as i64),
                        None => idx.s.parse::<i64>().ok(),
                    };
                    if let Some(name) = id.and_then(|i| refs.static_name(i)) {
                        return Some(format!("n\"{}\"", esc(name)));
                    }
                    return Some(format!("FName({})", idx.s));
                }
            }
        }
        // generated construct/destruct behavior ($beh, ~Dtor — implicit in AS source) and the
        // synthesized static-name accessor (__STATIC_NAME) have no valid source form; emitting
        // them produces stray `~`/identifier tokens that abort the whole module's parse.
        //
        // EDIT A: clearing the WHOLE stack also annihilates an ENCLOSING call's already-pushed
        // args when this behaviour runs in the MIDDLE of that call's arg-push sequence (the RVO /
        // by-value out-param idiom). Truncate ONLY this behaviour's own receiver + params (the
        // PSF'd slot it constructs + its declared args) so enclosing operands survive.
        match params {
            Some(p) => {
                let consume = (p.len() + 1).min(stack.len()); // ctor args + 1 receiver(PSF out-slot)
                stack.truncate(stack.len() - consume);
            }
            // No param info: drop only a top PSF out-slot (the thing it just constructed); never a
            // genuine sibling arg. If the top isn't a PSF slot, leave the stack untouched.
            None => {
                if stack.last().map(|a| a.is_psf).unwrap_or(false) {
                    stack.pop();
                }
            }
        }
        return None;
    }
    // EDIT B-PRIME: `mem::take` empties the ENTIRE operand stack — so a NESTED call eats the
    // enclosing call's deeper args (proven: GetHero/GetDistanceTo). When a TRUSTED arity is known
    // (script param count, which is authoritative; or Binds native arity), take ONLY the top
    // `need` entries (this call's receiver + args) and LEAVE deeper entries for the enclosing call.
    // Untrusted -> keep the original take-all (cache native param counts are unreliable).
    // A method returning a struct BY VALUE (F*/T* head) also pushes a hidden RVO out-slot, so its
    // frame is params + receiver + 1; account for it or the split drops a real leading arg.
    // A method returning a struct BY VALUE pushes a hidden RVO out-slot, so its frame is
    // params + receiver + 1. But a method returning by REFERENCE (`TArray& Last()`, container
    // accessors) does NOT — and ret_ty is only the base-name string, so the is_reference flag is
    // erased. Blindly adding +1 for any F/T-head return over-widens the split window and EATS an
    // enclosing call's arg (proven: TArray::Last inside `container.Last(i).AddTag(tag)` ate the
    // tag). Data-driven guard: only count the RVO slot when it is ACTUALLY on the stack — the
    // entry directly below the receiver is a PSF slot whose type head equals the return-type head
    // (the same condition the Fix-b3 out-slot probe uses). This can only SHRINK the window vs the
    // old heuristic, never widen, so it cannot break a real by-value RVO recovery.
    // batch-20 Class D: a method returning BY REFERENCE (`FString& Append(...)` — fluent
    // builders) has NO hidden RVO out-slot, but `ret_ty` is the base-name string with the
    // refness erased — the probes below misread a genuine PSF'd struct ARG of the same type
    // (Append's `const FString&in` fed by a ToString RVO local) as the out-slot, stealing the
    // arg (`.Append()`) and prefixing a bogus `out =` (48 in-game errors). `ret_is_ref` carries
    // the T3 DataType's bIsReference; every RVO probe is gated on it.
    fn tyhead(s: &str) -> &str {
        s.split('<').next().unwrap_or(s)
    }
    let rvo_slot = !ret_is_ref
        && ret_ty
            .map(|t| matches!(tyhead(t).bytes().next(), Some(b'F') | Some(b'T')))
            .unwrap_or(false)
        && {
            // The hidden RVO out-slot sits BELOW the receiver for a method (top = receiver) and
            // ON TOP for a free call (no receiver pushed). Same data-driven evidence gate as the
            // Fix-b3 probe (is_psf + type-head == return head), so it only widens the window when
            // the out-slot is genuinely present. (Free calls returning a struct by value push it:
            // GotoPosition/Say/GiveItemTo/FInGameTime::Now.)
            let idx = if is_method { 2 } else { 1 };
            stack.len() >= idx && {
                let slot = &stack[stack.len() - idx];
                slot.is_psf && slot.ty.as_deref().map(tyhead) == ret_ty.map(tyhead)
            }
        };
    let need = trusted_arity.map(|n| n + is_method as usize + rvo_slot as usize);
    let collected: Vec<Arg> = take_call_frame(stack, need);
    // Implicit `__WorldContext` markers occupied a stack slot so the split arithmetic matched the
    // native's real frame (whose declared arity counts the hidden WorldContextObject param). They
    // are NOT source args — the UE-AngelScript compiler auto-injects the world context — so strip
    // them from the rendered args and lower the effective arity by the count removed. Without this,
    // dropping them at push time made the split one entry too deep and stole a neighbouring call's
    // arg (GetNPCState-shifts-args family).
    let ctx_count = collected.iter().filter(|x| x.is_ctx).count();
    // batch-32b (N5, specs/batch31-nomatch-illegalop.md §1.6): the hidden WorldContextObject
    // param must be stripped from the PARAM LIST too, not just from the args — passing the
    // full T3 list shifted EVERY positional param consumer (maybe_reverse_args scored
    // ShowTopSubtitle's Duration against the WCO UObject (invisible) while the reversed
    // correct order put FText there (+1) and kept the wrong push order; cast_arg/render_args
    // paired the same shifted types). The WCO's PARAM POSITION varies by binding: free
    // natives push the marker LAST (= source param 0, leading — ShowTopSubtitle/SpawnAIAgent
    // disasm), script-struct method natives push it FIRST (= trailing param —
    // FInGameDate::IsLessThanXDaysAgo, FInGameTime::Now: `PshGPtr __WorldContext` before the
    // real args). Classify by stack position: a marker within the top-of-frame overhead
    // (receiver + RVO slot) is a LEADING param; anything deeper is TRAILING. A naive
    // front-strip mis-paired the conversation IsValid/IsLessThanXDaysAgo family (int arg
    // scored against the trailing WCO's UObject -> NEW argint stubs).
    let overhead = is_method as usize + rvo_slot as usize;
    let ctx_lead = collected
        .iter()
        .enumerate()
        .filter(|(j, x)| x.is_ctx && collected.len() - 1 - j <= overhead)
        .count();
    let ctx_trail = ctx_count - ctx_lead;
    let mut a: Vec<Arg> = collected
        .into_iter()
        .filter(|x| !x.is_ctx && !x.s.is_empty() && x.s != UNRESOLVED)
        .collect();
    let params = params.map(|p| {
        let lead = ctx_lead.min(p.len());
        let trail = ctx_trail.min(p.len() - lead);
        &p[lead..p.len() - trail]
    });
    // Effective arity: the in-game compile validates against the shipped Binds.Cache, so its native
    // arity is authoritative — prefer it over the script FunctionReferences param count. Falls back.
    // Subtract any stripped implicit-context markers (they inflate both the frame and the arity;
    // the params fallback is already ctx-stripped above).
    let arity = native_arity
        .map(|n| n.saturating_sub(ctx_count))
        .or_else(|| params.map(|p| p.len()));
    // Receiver: a METHOD call always pushes its receiver as the top entry (the cache param count is
    // unreliable here — it often COUNTS the implicit `this`), so detect by the bIsMethod flag.
    let has_recv = is_method && !a.is_empty();
    if has_recv {
        let recv = a.pop().unwrap();
        // Fix b3 — RVO STRUCT-RETURN: a script method returning a struct BY VALUE
        // (`FQuestRequirement MakeRequirement(...)`) is lowered as: push real args; push a hidden
        // `PSF <out_slot>` RVO destination; push receiver; CALL/CALLINTF. The callee writes its
        // return into `<out_slot>`. Recover it as `out_slot = f(real_args);` (analogous to the
        // existing `$beh0`/`opCast` PSF out-slot arms) instead of dropping the result. Run BEFORE
        // the arity trim: the hidden out-slot inflates the arg count one past `arity`, so removing
        // it first restores the correct count (else the trim drops a real leading arg).
        //
        // EXCLUDE operator-overload methods (opAssign/opAdd/opEquals/...): they return their
        // operand type (often a struct/template returned BY REFERENCE, not via an RVO out-slot)
        // and are lowered as `recv <op> arg` below. Their PSF'd value arg can share the return
        // type's head (e.g. `member.opAssign(states)` returning `TArray` with a PSF'd `TArray`
        // value), which would falsely match the out-slot probe and rewrite `states = opAssign()`.
        fn head(s: &str) -> &str {
            s.split('<').next().unwrap_or(s)
        }
        // A self-call that names an ANCESTOR's function while the current class declares that
        // very method with the same arity is `Super::` — see the arm below. It has to be known
        // HERE too: a struct-by-value return is rendered by the RVO probe further down, which
        // would otherwise write `out = this.Method(...)` and recurse.
        let is_super_call = recv.s == "this"
            && match (target_owner, cur_class, params) {
                (Some(owner), Some(cur), Some(declared)) => {
                    owner != cur
                        && refs.is_subclass(cur, owner)
                        && (non_virtual || refs.class_overrides_method(cur, f, declared.len()))
                }
                _ => false,
            };
        let is_operator = assign_op(f).is_some() || binop_method(f).is_some();
        if let Some(rh) = ret_ty.map(head).filter(|_| !is_operator && !ret_is_ref) {
            if matches!(
                bare_type_name(rh).bytes().next(),
                Some(b'F') | Some(b'T') | Some(b'E')
            ) {
                // the RVO out-slot = a PSF arg whose type head equals the return-type head.
                // batch-29c (3a, specs/batch29-errortail.md): the ABI pushes
                // [args..., dest, recv], so after the recv pop the dest is the LAST entry —
                // probe with rposition. The bottom-up probe stole a same-headed struct ARG
                // (FVector::RotateAngleAxis's Axis) and slid the real dest into the arg list
                // (`local_54 = local_6.RotateAngleAxis(local_48, local_62);` with w48 the
                // true dest). Single-PSF cases (the 495-site Iterator/GetActorLocation
                // population) pick the same entry — no regression surface.
                if let Some(pos) = a
                    .iter()
                    .rposition(|x| x.is_psf && x.ty.as_deref().map(head) == Some(rh))
                {
                    let out = a.remove(pos).s;
                    if let Some(w) = arity {
                        let w = w.min(a.len());
                        if a.len() > w {
                            a.drain(..a.len() - w);
                        }
                    }
                    maybe_reverse_args(&mut a, params, refs);
                    // Include the receiver — this is a METHOD RVO struct-return (has_recv popped
                    // `recv`); omitting it emitted `out = Iterator()` (495×) / `out =
                    // GetActorLocation()` instead of `out = recv.Iterator()` -> "No matching
                    // signatures". `this`-receivers render `this.Method()` (legal, matches the
                    // normal method-render path below).
                    if is_super_call {
                        return Some(format!(
                            "{out} = Super::{f}({})",
                            render_args(&a, params, refs, arg_defaults)
                        ));
                    }
                    return Some(format!(
                        "{out} = {}.{f}({})",
                        wrap_uobject_recv(&recv, target_owner, refs),
                        render_args(&a, params, refs, arg_defaults)
                    ));
                }
            }
        }
        // G1c (batch-26): CALLSYS value-operator with an RVO destination slot. The fork's
        // Binds register FVector-family operators as returning the struct BY VALUE, so the
        // caller pushes a hidden PSF dest directly BELOW the receiver: [args..., dest, recv].
        // The is_operator exclusion above (protecting ref-returning opAssign) let the generic
        // arity trim eat the REAL rhs and substitute the dest -> discarded `(a / dest);`
        // statements (458 in-game "Result of expression is unused") and silently-wrong
        // compound assigns (`x += dead_temp`). Gate on the SAME data-driven rvo_slot probe
        // that already widened the split window: a ref-returning operator pushes no dest and
        // can never match (ret_is_ref + F/T-head are inside rvo_slot).
        if is_operator
            && rvo_slot
            && a.last()
                .map(|x| x.is_psf && x.ty.as_deref().map(head) == ret_ty.map(head))
                .unwrap_or(false)
        {
            let dest = a.pop().unwrap();
            if let Some(w) = arity {
                let w = w.min(a.len());
                if a.len() > w {
                    a.drain(..a.len() - w);
                }
            }
            match (assign_op(f), binop_method(f), a.first()) {
                (Some(op), _, Some(rhs)) => {
                    // compound/plain assign: the result lives in the RECEIVER; `dest` is the
                    // dead return-value temp. Mirror the existing arm's copyctor stub gate.
                    if op == "=" && recv.s == "this" {
                        return Some(amm("copyctor"));
                    }
                    let r = params
                        .and_then(|p| p.first())
                        .map(|pt| cast_arg(rhs, pt, refs))
                        .unwrap_or_else(|| rhs.s.clone());
                    return Some(format!("{} {op} {}", recv.s, r));
                }
                (None, Some(op), Some(rhs)) => {
                    // pure binop: the DEST receives the result.
                    let r = params
                        .and_then(|p| p.first())
                        .map(|pt| cast_arg(rhs, pt, refs))
                        .unwrap_or_else(|| rhs.s.clone());
                    return Some(format!("{} = ({} {op} {})", dest.s, recv.s, r));
                }
                // rhs unrecovered (short/take-all stack): restore and fall through to the
                // existing arms -> status-quo render, zero regression.
                _ => a.push(dest),
            }
        }
        // trim phantom extras: keep only the last `w` user args (the cache arity may include the
        // now-popped `this`, so cap below the popped count, never above).
        if let Some(w) = arity {
            let w = w.min(a.len());
            if a.len() > w {
                a.drain(..a.len() - w);
            }
        }
        // super-class constructor on `this` -> `super(args)` (before is_type_name, since the
        // super name is itself a type name).
        if super_ctor == Some(f) && recv.s == "this" {
            maybe_reverse_args(&mut a, params, refs); // super calls are reverse-pushed too
            return Some(format!(
                "super({})",
                render_args(&a, params, refs, arg_defaults)
            ));
        }
        // BUG (a) — SUPER-CALL: a NON-VIRTUAL (`CALL`) dispatch on `this` to a method owned by a
        // STRICT ANCESTOR of the current class is a `Super::method()` call, not `this.method()`
        // (a genuine virtual self-call compiles to CALLINTF, never a CALL to the base func-id).
        // A VIRTUAL (`CALLINTF`) self-call that names the ANCESTOR's function while the current
        // class declares that very method is `Super::` too: a plain `this.` call would compile
        // to the class's own override — infinite recursion, and a function identity the base
        // cache does not have, which costs the module its splicability.
        if is_super_call {
            maybe_reverse_args(&mut a, params, refs); // super calls are reverse-pushed too
            return Some(format!(
                "Super::{f}({})",
                render_args(&a, params, refs, arg_defaults)
            ));
        }
        // a call whose name is a type = an in-place constructor (member struct default ctor) —
        // implicit in AS source, emit nothing.
        if refs.is_type_name(f) {
            return None;
        }
        // implicit-conversion behaviours (opImplConv/opConv) have NO source form: the conversion
        // re-fires implicitly from the assignment/argument target type. An explicit `.opImplConv()`
        // makes AS enumerate all overloads with no target type -> "Multiple matching signatures".
        // Render the receiver itself so it flows into the store and the compiler re-derives the
        // conversion from the destination's declared type.
        if matches!(f, "opImplConv" | "opConv") {
            return Some(recv.s.clone());
        }
        // operator-overload methods -> source operators (cast the RHS to the operand type).
        if let Some(op) = assign_op(f) {
            if op == "=" && recv.s == "this" {
                return Some(amm("copyctor")); // generated struct copy-ctor/assign — stub
            }
            match a.first() {
                Some(rhs) => {
                    let r = params
                        .and_then(|p| p.first())
                        .map(|pt| cast_arg(rhs, pt, refs))
                        .unwrap_or_else(|| rhs.s.clone());
                    return Some(format!("{} {op} {}", recv.s, r));
                }
                None => return None, // unresolved RHS -> skip rather than emit `x = <bad>`
            }
        }
        if let Some(op) = binop_method(f) {
            if let Some(rhs) = a.first() {
                let r = params
                    .and_then(|p| p.first())
                    .map(|pt| cast_arg(rhs, pt, refs))
                    .unwrap_or_else(|| rhs.s.clone());
                return Some(format!("({} {op} {})", recv.s, r));
            }
        }
        maybe_reverse_args(&mut a, params, refs);
        cast_container_args(f, recv.ty.as_deref(), &mut a, refs);
        Some(format!(
            "{}.{f}({})",
            wrap_uobject_recv(&recv, target_owner, refs),
            render_args(&a, params, refs, arg_defaults)
        ))
    } else {
        if refs.is_type_name(f) {
            // A factory GLOBAL whose name collides with a registered type name. Two flavours:
            //
            // EDIT C: a VALUE-type factory call (FName/E*/T* — NOT U*/A*, which use ALLOC) whose
            // result is built into the return register and re-pushed by a following `PshRPtr` as
            // an ENCLOSING call's arg. Dropping it (return None) loses that arg → the consuming
            // call renders 0-arg (`this.GetCharacter()`). Instead render it as `T(args)` so it
            // FLOWS into `pending` and PshRPtr recovers it (`this.GetCharacter(FName(...))`).
            // Restores ARITY; the literal may be a name-table index (acceptable per spec). Gate
            // strictly: value type AND non-empty args (a 0-arg in-place default ctor stays None).
            //
            // batch-42: an OBJECT-factory GLOBAL (U*/A*) — a CALL-dispatched global that RETURNS a
            // handle (NOT an ALLOC in-place ctor: ALLOC is the factory's OWN body, not the call
            // site). Names like `UCBT_Root(prio)`, `UTauntConfigManager()`,
            // `UCBT_AttackTokenRequest()`, the BT-wrapper family `Inverter`/`Succeeder`/… collide
            // with their return type name, so `is_type_name` is true and the arm dropped them as if
            // they were struct ctors — stranding the `STOREOBJ wN` slot that the downstream member
            // store (`this.x = local_N`), chained call receiver (`Parent.DoNode(local_N, …)`), or
            // `LOADOBJ; RET` then reads as a phantom (near-empty factories degenerate to `return;`).
            // Recover as `f(args)` so it FLOWS into `pending`; the bytecode's own `STOREOBJ` then
            // captures it into `local_N` (so the member-ref REFCPY store — batch-41b — sees a plain
            // `local_N` src that already passes its gate, and `LOADOBJ; RET` renders `return f(…)`).
            // DISCRIMINATOR (both data points already in hand): a real in-place ctor returns VOID
            // (token 0x52) and/or is a METHOD the idiom-C ctor path consumed BEFORE build_call; a
            // factory returns a non-void OBJECT handle (ret_ty head U/A, token 5) AND is a non-method
            // free global. Gate on the RETURN TYPE, NOT arg count — a 0-arg `UCBT_AttackTokenRequest()`
            // MUST recover. Any un-provable operand (no U/A class-head return, void, sentinel/
            // unresolved arg) BAILS to the current drop below — never guess (a wrong factory fed into
            // a member store / return is worse than a dropped one, and a mis-fire on a genuine in-place
            // ctor = silent DOUBLE construction).
            let head = f.split('<').next().unwrap_or(f);
            let is_value = matches!(
                bare_type_name(head).bytes().next(),
                Some(b'F') | Some(b'E') | Some(b'T')
            );
            if is_value && !a.is_empty() {
                maybe_reverse_args(&mut a, params, refs);
                return Some(format!(
                    "{f}({})",
                    render_args(&a, params, refs, arg_defaults)
                ));
            }
            // OBJECT factory: non-method (structurally: an in-place ctor is void and/or a method the
            // idiom-C arm consumed first), return type resolves to a real U/A class head, non-void.
            // `bare_type_name` first: a namespaced return (`AutomatedTest::UAIState_Test_…`)
            // starts with the NAMESPACE, so the class-head test read `Au` and refused a genuine
            // factory — its `STOREOBJ` slot then stayed unwritten and every use of it dangled.
            let is_object_factory = !is_method
                && matches!(ret_ty.map(tyhead).map(bare_type_name), Some(rh) if
                    matches!(rh.bytes().next(), Some(b'U') | Some(b'A'))
                    // U/A + uppercase-2nd-char = a real class head (rejects `uint`-ish primitives).
                    && rh.as_bytes().get(1).map(|c| c.is_ascii_uppercase()).unwrap_or(false)
                    && rh != "void");
            if is_object_factory {
                maybe_reverse_args(&mut a, params, refs);
                let rendered = render_args(&a, params, refs, arg_defaults);
                // Never emit a sentinel-marked / unresolved arg list (a definite mismatch): bail.
                if !rendered.contains('\u{2}')
                    && !rendered.contains('\u{1}')
                    && rendered != UNRESOLVED
                {
                    return Some(format!("{f}({rendered})"));
                }
            }
            return None; // free-standing in-place constructor — implicit in AS source
        }
        // an operator-overload method with no stack receiver (its target was in the reference
        // register, e.g. a member opAssign) can't render as a free call — skip it rather than
        // emit `opAssign(...)`, which never resolves.
        if assign_op(f).is_some() || binop_method(f).is_some() {
            return None;
        }
        // implicit-conversion behaviour with its receiver in the reference register (no stack
        // receiver) — never a free call; drop it (the conversion re-fires from the target type).
        if matches!(f, "opImplConv" | "opConv") {
            return None;
        }
        // GENERATED per-component static accessors (`UMyComponent::Get(Actor[, FName])`,
        // GetOrCreate, Create): their DECLARATIONS are intentionally skipped from the emit
        // (signature collisions), so a bare call `Get(local_2, NAME_None)` never resolves
        // ("No matching signatures ... Did you mean 'Sit'?", 362 in-game errors). The real
        // UE-AngelScript form is the class-qualified static: `UMyComponent::Get(local_2)`.
        // Gate: exact generated-getter name, an owning class on the callee (the generated fns
        // carry their component class as ObjectType), and >= 1 collected arg (the actor) —
        // 0-param statics like UQuestSubsystem::Get are native CALLSYS, not this path.
        // (0-arg form = the generated per-subsystem `Get()` accessor — same skip class, same
        // qualified-static fix: `UMySubsystem::Get()` is the native Hazelight subsystem idiom.)
        if matches!(f, "Get" | "GetOrCreate" | "Create") {
            // Owner: the callee's ObjectType when recorded; else the RETURN type — a generated
            // accessor returns exactly the component class it is generated on, so the return
            // head is the qualifying class (`UPyrolaserOriginPointComponent Get(AActor, FName)`).
            let owner = target_owner.filter(|o| o.starts_with('U')).or_else(|| {
                ret_ty
                    .map(|t| t.split('<').next().unwrap_or(t))
                    .filter(|t| t.starts_with('U') && refs.is_type_name(t))
            });
            if let Some(owner) = owner {
                maybe_reverse_args(&mut a, params, refs);
                return Some(format!(
                    "{owner}::{f}({})",
                    render_args(&a, params, refs, arg_defaults)
                ));
            }
        }
        // batch-24b: AngelScript member lookup SHADOWS globals — an unqualified call to a free
        // SCRIPT global from inside a class whose (native) ancestry has a same-named member
        // resolves to the member and fails (`WaitSeconds(this.AI, 2.0)` diagnosed against
        // `UAbilityTaskCoroutine::WaitSeconds(float32)`, 33 sites; CastSpell/IsMoving/IsDead/
        // CanMoveIntoDirection likewise). The global-scope qualifier `::f(...)` always legally
        // names a genuine global (over-qualification is harmless), so gated call sites render
        // qualified. Computed AFTER the early-returns above so name-based matches (behaviours,
        // is_type_name ctors, owner-qualified Get/GetOrCreate/Create, operators) are untouched.
        let f: std::borrow::Cow<str> = if global_shadowed {
            format!("::{f}").into()
        } else {
            f.into()
        };
        // Free-call RVO struct-return (mirror of the method Fix-b3 arm): a free/static function
        // returning a struct BY VALUE pushes a hidden PSF out-slot. Recover `out = f(args)`
        // instead of leaking the out-slot as a leading arg (GotoPosition/Say/GiveItemTo/
        // FInGameTime::Now). Same data-driven gate as Fix-b3: a PSF arg whose type head equals the
        // return-type head; a call without a genuine out-slot can't match. Gated on !ret_is_ref
        // (batch-20 Class D): a BY-REFERENCE return has no out-slot, and the probe would steal a
        // same-typed by-ref struct arg.
        if let Some(rh) = ret_ty.map(tyhead).filter(|_| !ret_is_ref) {
            if matches!(
                bare_type_name(rh).bytes().next(),
                Some(b'F') | Some(b'T') | Some(b'E')
            ) {
                // batch-32b (N5): the free-call ABI pushes [args..., dest] — the RVO dest is
                // the TOP entry (build_call's own rvo_slot probe: idx=1 for free calls), so
                // probe with rposition, mirroring the batch-29c method-arm fix (line ~691).
                // The bottom-up probe stole a PSF'd struct ARG of the same head: ApplyFormat's
                // FString SPECIFIER became the dest while the real dest slid into the args
                // (`local_36 = ApplyFormat(local_44, local_14); local_40.Append(local_44);`
                // — 6 GA_Falling errors + 1 by-accident int-overload mis-bind). Single-PSF
                // sites (string-literal Specifier, CombatSituations ×8) pick the same entry.
                if let Some(pos) = a
                    .iter()
                    .rposition(|x| x.is_psf && x.ty.as_deref().map(tyhead) == Some(rh))
                {
                    let out = a.remove(pos).s;
                    if let Some(w) = arity {
                        let w = w.min(a.len());
                        if a.len() > w {
                            a.drain(..a.len() - w);
                        }
                    }
                    maybe_reverse_args(&mut a, params, refs);
                    return Some(format!(
                        "{out} = {f}({})",
                        render_args(&a, params, refs, arg_defaults)
                    ));
                }
            }
        }
        // trim leading phantom extras for a known free arity too.
        if let Some(w) = arity {
            if a.len() > w {
                a.drain(..a.len() - w);
            }
        }
        maybe_reverse_args(&mut a, params, refs);
        Some(format!(
            "{f}({})",
            render_args(&a, params, refs, arg_defaults)
        ))
    }
}

/// Batch19 class 1 — UObject-typed method receivers: a receiver slot the cache records as
/// plain `UObject` (a compiler temp for a `&&`-chain / IsValid arg, or a generic-getter result
/// like `GetTypedOuter`) fails method lookup on every non-UObject method ("No matching
/// signatures to 'UObject::GetCharacterState()'", 255 in-game errors). The CONSUMING method's
/// owner class is known from the callee's T3 ObjectType — wrap the receiver in the standard
/// UE-AS downcast idiom `Cast<Owner>(recv)` AT THE CALL SITE. Call-site wrapping (option (b))
/// is chosen over retyping the DECLARATION (option (a)) because the slot is also written from
/// producers (`RefCpyV` handle copies, generic getters) whose types we cannot prove compatible
/// with the owner — a retyped declaration could break those assignments (upcast-breaking),
/// while a local `Cast<>` can only affect the one call that is already broken.
///
/// batch-25d (specs/batch23-nomatch.md D): the original gate `recv.ty == "UObject"` EXACTLY
/// was too narrow — the same disease presents on the other engine BASE types the cache
/// records for iterator/temp producers (`AActor local_28; local_28.GetCharacterState();`,
/// 26 in-game errors: GetCharacterState x12, GetInventory x3, GetAvatar x3, GetAI x2, ...).
/// The receiver gate is widened to a short EXPLICIT base-class list (never arbitrary types),
/// with a matching skip for calls the receiver ALREADY satisfies: the owner being the
/// receiver type itself, one of its known ENGINE ancestors (methods genuinely on AActor must
/// not wrap an ACharacter receiver — the spec's `owner != AActor` caution, generalized), or
/// a script-hierarchy-proven ancestor. Owner must still be a real object class (U*/A*).
fn wrap_uobject_recv(recv: &Arg, owner: Option<&str>, refs: &RefResolver) -> String {
    /// The explicit base-receiver list (spec batch23-nomatch.md §4) — do NOT widen to
    /// arbitrary recovered types.
    const BASE_RECV: &[&str] = &[
        "UObject",
        "AActor",
        "APawn",
        "ACharacter",
        "UActorComponent",
        "UAbilitySystemComponent",
    ];
    /// Known ENGINE ancestors of each listed base (UObject < AActor < APawn < ACharacter;
    /// UObject < UActorComponent < UAbilitySystemComponent).
    fn engine_ancestors(t: &str) -> &'static [&'static str] {
        match t {
            "AActor" => &["UObject"],
            "APawn" => &["AActor", "UObject"],
            "ACharacter" => &["APawn", "AActor", "UObject"],
            "UActorComponent" => &["UObject"],
            "UAbilitySystemComponent" => &["UActorComponent", "UObject"],
            _ => &[],
        }
    }
    let Some(rt) = recv.ty.as_deref() else {
        return recv.s.clone();
    };
    if recv.s == "this" || !BASE_RECV.contains(&rt) {
        return recv.s.clone();
    }
    let Some(o) = owner else {
        return recv.s.clone();
    };
    if o == rt
        || !matches!(o.bytes().next(), Some(b'U') | Some(b'A'))
        || engine_ancestors(rt).contains(&o)
        || refs.is_subclass(rt, o)
    {
        return recv.s.clone();
    }
    format!("Cast<{}>({})", qualify_class_name(&o, refs), recv.s)
}

/// Count DEFINITE type mismatches when pairing args[i] with params[i] (mirrors `cast_arg`'s
/// "this arg can't possibly match" rule). A value-type (F/E/T) head-mismatch or an object
/// arg that is a known non-subclass of a known-script param both count; everything else
/// (unknown types, int->primitive casts, engine upcasts) is treated as a possible match so
/// the score never penalizes a legitimately-ordered call.
fn arg_mismatch_count(a: &[Arg], params: &[DataType], refs: &RefResolver) -> usize {
    let head = |s: &str| s.split('<').next().unwrap_or(s).to_string();
    let is_value = |s: &str| {
        matches!(
            bare_type_name(s).bytes().next(),
            Some(b'F') | Some(b'E') | Some(b'T')
        )
    };
    let is_obj = |s: &str| matches!(bare_type_name(s).bytes().next(), Some(b'U') | Some(b'A'));
    let mut n = 0;
    for (i, arg) in a.iter().enumerate() {
        let Some(pt) = params.get(i) else { continue };
        if pt.token != 5 {
            continue; // primitive/enum param: int casts handle it, not a definite mismatch
        }
        // mirror cast_arg: an int-origin arg feeding a token-5 param that is NOT an enum
        // (the only int-castable token-5 target) can never match — this is the exact
        // condition that produces amm("argint") at render time. Count it so
        // maybe_reverse_args sees the evidence and can flip a reverse-pushed call. An int
        // const carries `ty: None`, so without this it was invisible to the scorer.
        if arg.is_int {
            if cast_to_typename("0", &pt.base_name(refs), refs).is_none() {
                n += 1;
            }
            continue;
        }
        let Some(at) = &arg.ty else { continue };
        let (ph, ah) = (head(&pt.base_name(refs)), head(at));
        if is_value(&ph) && is_value(&ah) && ph != ah {
            n += 1;
        } else if is_obj(&ph)
            && is_obj(&ah)
            && ah != ph
            && refs.is_script_class(&ah)
            && refs.is_script_class(&ph)
            && !refs.is_subclass(&ah, &ph)
        {
            n += 1;
        } else if is_value(&ph) && is_obj(&ah) {
            n += 1; // an object can never satisfy a value-struct (F/E/T) parameter
        } else if is_obj(&ph) && is_value(&ah) {
            n += 1; // a value-struct can never satisfy an object parameter
        }
    }
    n
}

/// AngelScript pushes call arguments such that, for some calls (notably mixin/member-style
/// calls), the collected stack order is the REVERSE of the source parameter order. Detect this
/// purely by type evidence: if reversing the args produces STRICTLY fewer definite type
/// mismatches against the declared params, the call was reverse-pushed -> reverse it. This is
/// self-validating (a correctly-ordered call already has 0 mismatches, so it is never touched),
/// so it cannot regress calls that already type-check.
///
/// Handles trailing-default omission: a call may pass FEWER args than the callee has params
/// (the trailing ones default), e.g. `NewObject(Outer, Class)` for `NewObject(Outer, Class,
/// Name=, bTransient=, Template=)`. The provided args still align to the FIRST params, so the
/// reversal is scored against `params[0..a.len()]` (more args than params is never valid -> skip).
fn maybe_reverse_args(a: &mut Vec<Arg>, params: Option<&[DataType]>, refs: &RefResolver) {
    let Some(params) = params else { return };
    if a.len() < 2 || a.len() > params.len() {
        return;
    }
    // Bytecode pushes call args in REVERSE source order for EVERY call type (proven across method,
    // free, super, native calls — 7 opcode-level confirmations, 0 counterexamples). So the
    // collected order is reverse-source and reversal is the PRIOR, not the exception. Keep the
    // collected order only when reversing makes the type pairing strictly WORSE (the list is
    // corrupted and reversal would misalign it further). This corrects the large class of
    // type-symmetric multi-arg calls (both orders type-check, so the old strict-improvement gate
    // left them in the wrong push order) — a byte-faithfulness win, and un-stubs the asymmetric
    // ones. A genuinely source-ordered correct call is asymmetric (fwd=0 < rev) -> not touched.
    let fwd = arg_mismatch_count(a, params, refs);
    let mut rev = a.clone();
    rev.reverse();
    if arg_mismatch_count(&rev, params, refs) <= fwd {
        a.reverse();
    }
}

/// Batch-21 Class B: container methods (`TArray`/`TSet::Add`, `TMap::Add`/`opIndex`) take the
/// receiver's template SUBTYPE(s) as parameters, but the native bind's stored param DataTypes
/// are generic placeholders — so `cast_arg` can't see that an int arg feeds an enum/bool
/// subtype (AngelScript has no implicit int->enum/int->bool): `TArray<EPhysicalSurface>
/// local; local.Add(1);` fails "No matching signatures to 'TArray::Add(int)'" (54 in-game).
/// Derive the expected types from the receiver's COMPOSED type name (`TMap<ECombatRole,
/// float>` — locals via obj_locals, this-class members via the fields map) and wrap int args
/// in place. Foreign-member receivers with unknown types stay untouched (status-quo error).
fn cast_container_args(method: &str, recv_ty: Option<&str>, a: &mut [Arg], refs: &RefResolver) {
    let Some(t) = recv_ty else { return };
    let t = t.trim_start_matches("const ");
    let Some((head, rest)) = t.split_once('<') else {
        return;
    };
    let Some(inner) = rest.strip_suffix('>') else {
        return;
    };
    // top-level comma split (subtypes can nest: TMap<ECombatRole, TArray<int>>)
    let mut subs: Vec<&str> = Vec::new();
    let (mut depth, mut start) = (0usize, 0usize);
    for (i, c) in inner.char_indices() {
        match c {
            '<' => depth += 1,
            '>' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                subs.push(inner[start..i].trim());
                start = i + 1;
            }
            _ => {}
        }
    }
    subs.push(inner[start..].trim());
    let expect: &[&str] = match (head, method, subs.len()) {
        // single-value element methods (capture shapes: Add(int) 28x, Contains(int) 10x,
        // AddUnique/Remove(int) 2x) — the arg is the element type T.
        ("TArray" | "TSet", "Add" | "AddUnique" | "Remove" | "RemoveSingle" | "Contains", 1) => {
            &subs[..1]
        }
        // Add(K, V) takes both subtypes positionally.
        ("TMap", "Add", 2) => &subs[..2],
        // key-first methods: only the K arg is wrapped (Find's 2nd arg is the V out-slot,
        // FindOrAdd/opIndex/Contains/Remove take just the key).
        ("TMap", "opIndex" | "Find" | "FindOrAdd" | "Contains" | "Remove", 2) => &subs[..1],
        _ => return,
    };
    for (arg, want) in a.iter_mut().zip(expect) {
        // The container natives take `const T&in`, so int-slot args arrive PSF-pushed
        // (address of the slot: is_psf=true, is_int=false, ty=None — int locals aren't in
        // the typed-locals map). A slot with a KNOWN non-int type (bool/float/object local)
        // is a real typed value and must stay bare.
        if !(arg.is_int || (arg.is_psf && arg.ty.is_none())) {
            continue;
        }
        if let Some(c) = cast_to_typename(&arg.s, want, refs) {
            arg.s = c;
            arg.is_int = false;
            arg.cbits = None;
            arg.ty = Some((*want).to_string());
        }
    }
}

/// Render args joined by ", ", casting each int arg to the callee's expected param type.
fn render_args(
    a: &[Arg],
    params: Option<&[DataType]>,
    refs: &RefResolver,
    defaults: Option<&[String]>,
) -> String {
    let mut rendered: Vec<String> = a
        .iter()
        .enumerate()
        .map(|(i, arg)| match params.and_then(|p| p.get(i)) {
            Some(pt) => cast_arg(arg, pt, refs),
            None => arg.s.clone(),
        })
        .collect();
    // Drop trailing arguments that just restate the declared default. The bytecode always
    // materialises them, so recovering them literally is not wrong — but it is a DIFFERENT
    // source program, and one that makes the compiler emit construct behaviours the base cache
    // never had (`TSubclassOf<UConversationTopic>::$beh0()`), which then fail to remap.
    if let Some(defaults) = defaults {
        while let Some(last) = rendered.len().checked_sub(1) {
            let Some(default) = defaults.get(last).filter(|value| !value.is_empty()) else {
                break;
            };
            if super::refs::pack_tokens(&rendered[last]) != *default {
                break;
            }
            rendered.pop();
        }
    }
    rendered.join(", ")
}

/// Cast an int-origin argument to the callee's `bool`/enum param (AngelScript has no
/// implicit int->bool or int->enum conversion). Non-int args pass through unchanged.
fn cast_arg(arg: &Arg, pt: &DataType, refs: &RefResolver) -> String {
    if !arg.is_int {
        // a value-type (F-struct / E-enum, which have NO inheritance) arg whose recovered
        // type differs from the parameter type can't possibly match -> the arg recovery is
        // wrong; stub. (Object U*/A* params are skipped: upcasts are legal.)
        if pt.token == 5 {
            if let Some(at) = &arg.ty {
                // compare the type "head" (before any `<...>`) so covariant template
                // instantiations (e.g. TSubclassOf<Derived> vs <Base>) aren't flagged.
                let head = |s: &str| s.split('<').next().unwrap_or(s).to_string();
                let is_value = |s: &str| {
                    matches!(
                        bare_type_name(s).bytes().next(),
                        Some(b'F') | Some(b'E') | Some(b'T')
                    )
                };
                let is_obj =
                    |s: &str| matches!(bare_type_name(s).bytes().next(), Some(b'U') | Some(b'A'));
                let (ph, ah) = (head(&pt.base_name(refs)), head(at));
                // value types (F/E/T) have no inheritance — any head mismatch is wrong.
                // Emitting the raw arg (e.g. an FName where an FVector is wanted) won't
                // type-check, so force the stub fallback via the ARGMISMATCH sentinel.
                if is_value(&ph) && is_value(&ah) && ph != ah {
                    return amm("argtype");
                }
                // objects (U*/A*): an arg that isn't the param or a subclass of it is wrong —
                // but only when BOTH are known script classes (else an engine upcast we can't
                // verify; stay conservative and allow it).
                if is_obj(&ph)
                    && is_obj(&ah)
                    && ah != ph
                    && refs.is_script_class(&ah)
                    && refs.is_script_class(&ph)
                    && !refs.is_subclass(&ah, &ph)
                {
                    return amm("argtype");
                }
            }
        }
        // batch-31c (N3 Fix 1, spec batch31-nomatch-illegalop §1.5): enum-as-int at enum
        // params of ANY refness — a PSF'd UNTYPED slot (int locals are not in the typed-
        // locals map) feeding an enum param takes the same E(x) wrap the by-value int path
        // below applies. `const E&in` accepts the temporary; a Fix-2-typed `E&out` slot
        // arrives with ty=Some so this gate never fires for it (arg gate identical to
        // cast_container_args, structure.rs batch-25e). Damage.as SendGameplayEvent ×4 /
        // CreateRelativeMemoriesToCrime ×3.
        if pt.token == 5 && arg.is_psf && arg.ty.is_none() {
            let base = pt.base_name(refs);
            if is_enum_name(&base) {
                return format!("{base}({})", arg.s);
            }
        }
        // a KNOWN-enum arg feeding an INT-family param needs the explicit int(...) cast
        // (AngelScript has no implicit enum->int): `SetLevel(int)` fed an ERelationship
        // fails "Can't implicitly convert from 'ERelationship' to 'int'" (~150 in-game).
        if pt.token != 5 {
            if let Some(at) = &arg.ty {
                if is_enum_name(at)
                    && matches!(
                        pt.base_name(refs).as_str(),
                        "int" | "int8" | "int16" | "int64" | "uint" | "uint8" | "uint16" | "uint64"
                    )
                {
                    return format!("int({})", arg.s);
                }
            }
        }
        return arg.s.clone();
    }
    // an integer constant feeding a float/double param carries IEEE-754 bits, not an int.
    if let Some(cb) = arg.cbits {
        match pt.token {
            0x50 => return fmt_float(cb, false), // float32 -> `Nf` literal
            0x51 | 0x5E => return fmt_float(cb, true), // float (64-bit here) / double -> plain
            _ => {}
        }
    }
    if pt.token == 0x41 {
        // bool param
        return format!("({} != 0)", arg.s);
    }
    // batch-30c: an int-family SLOT feeding a SMALL-int parameter warns twice in-game
    // ("signed to unsigned" + "truncates", warnings-as-errors) — the batch-28b small-int
    // RETYPE residue (slots whose op profile failed its SetV-only gate, e.g. the
    // SetMovementMode NewCustomMode args fed from reads). Explicit narrowing cast; a
    // slot already retyped small renders a same-type no-op cast.
    match pt.token {
        0x45 => return format!("int8({})", arg.s),
        0x46 => return format!("int16({})", arg.s),
        0x4C => return format!("uint8({})", arg.s),
        0x4D => return format!("uint16({})", arg.s),
        // A 32-bit constant feeding a `uint`/`uint64` parameter is unsigned data rendered
        // through a signed slot: an ARGB colour hex reaches `FLinearColor::MakeFromHex` as
        // `-8364214` and warns "Implicit conversion changed sign of value" (warnings are
        // errors here). Re-render the SAME bits unsigned; a non-negative constant and any
        // non-constant argument are left exactly as they were.
        0x4B | 0x4E => {
            if let Some(ConstBits::W4(bits)) = arg.cbits {
                if (bits as i32) < 0 {
                    return bits.to_string();
                }
            }
        }
        _ => {}
    }
    if pt.token == 5 {
        // object/enum identifier type: UE enums are `E<Upper>...`; cast int -> enum
        let base = pt.base_name(refs);
        if let Some(c) = cast_to_typename(&arg.s, &base, refs) {
            return c;
        }
        // an int arg to a non-enum object/struct param can't convert at all — the arg
        // recovery is wrong; mark the body for the stub fallback (emitting the raw int
        // would compile-fail, e.g. passing `0` to an FName/UObject parameter).
        return amm("argint");
    }
    arg.s.clone()
}

/// Wrap a call result in `Cast<DstType>(...)` when it's stored into an object local of a
/// different (derived) type — the cache erases the covariant return type of template getters
/// like `GetTypedOuter<T>`/`SpawnedStorage<T>` to the base, so AS rejects the implicit
/// downcast. Only applies between UObject/AActor types (`U*`/`A*`).
///
/// batch-20 Class B: when the call returns a CONST object handle (`src_const`) and the
/// destination local is the SAME (non-const) type, the plain store fails "Can't implicitly
/// convert from 'const UCharacterAIState' to 'UCharacterAIState'". `Cast<T>` strips the const
/// when it actually CASTS: PROVEN in the batch-19 capture — `local_4 =
/// Cast<AGothicCharacter>(Querier);` with `const UObject Querier` compiles clean
/// (AIAgentConfig_Navigation_Human.as GetAIFromQuerier).
///
/// batch-21 Class C REVISION: the batch-20 EXACT-TYPE wrap `Cast<X>(const X)` does NOT strip
/// const in-game — the batch-20 capture shows every exact-type Cast site still failing
/// "No conversion from 'const X' to 'X'" (site counts match 1:1: UTerritoryConfig 12/12,
/// UCharacterDefinition 9/9, UComboAttackConfig 5/5, ...). A same-type Cast is a no-op that
/// keeps the const. The vanilla form is a CONST local declaration, so the exact-type arm now
/// emits the store BARE prefixed with the [`CONSTSTORE`] marker; the emitter strips it and
/// declares the destination local `const T` (assigning a later NON-const value to a const
/// handle stays legal, so mixed-source slots are safe).
fn downcast(
    rhs: String,
    src_ty: Option<String>,
    src_const: bool,
    dst_ty: Option<&String>,
    refs: &RefResolver,
) -> String {
    let is_obj = |s: &str| s.starts_with('U') || s.starts_with('A');
    match (src_ty, dst_ty) {
        (Some(s), Some(d)) if is_obj(&s) && is_obj(d) && s != *d => {
            // An upcast (src derives from dst) is implicit in AngelScript — wrapping it in
            // `Cast<Base>(derived)` can fail in-game compile. Only emit Cast for an actual
            // downcast / unrelated covariant-erased type.
            if refs.is_subclass(&s, d) {
                rhs
            } else {
                format!("Cast<{}>({rhs})", qualify_class_name(d, refs))
            }
        }
        (Some(s), Some(d)) if src_const && is_obj(&s) && s == *d => format!("{CONSTSTORE}{rhs}"),
        _ => rhs,
    }
}

/// Render the RHS of `field = <int>` for a field whose type name is `tyname`:
/// numeric fields take the int as-is; bool/enum get the cast; anything else (FName,
/// F-structs, U*/A* objects) can't hold an int — return UNRESOLVED so just THIS assignment
/// is dropped (a generator-inlined CDO default we can't reconstruct) and the rest of the
/// function still recovers, rather than stubbing the whole body.
fn field_assign_rhs(rhs: &str, tyname: &str, refs: &RefResolver) -> String {
    if let Some(c) = cast_to_typename(rhs, tyname, refs) {
        return c; // bool / enum
    }
    match tyname {
        "int" | "uint" | "int8" | "int16" | "int64" | "uint8" | "uint16" | "uint64" | "float"
        | "float32" | "double" | "?" => rhs.to_string(),
        _ => UNRESOLVED.to_string(),
    }
}

/// batch-25b (G4): true when object type `dst` PROVABLY derives from (and differs from)
/// object type `src` — the gate for the Cast-wrap on RET / RefCpyV dataflow. Sources of
/// proof, in order:
/// - the script-class hierarchy (walks script supers; super NAMES may be native classes,
///   so e.g. `ALightningRayVisual -> ... -> AActor` resolves),
/// - `src == UObject`: every U*/A* type derives UObject (engine axiom),
/// - `src == AActor` and `dst` is `A*`-prefixed: the UE naming convention reserves the `A`
///   prefix for AActor-derived classes.
/// Native-only pairs with no recorded chain return false (never wrap when unsure) —
/// documented residue: base-typed slots between two NATIVE classes stay unwrapped.
/// Template heads (`TSubclassOf<...>`) and value types never qualify.
fn provably_derived(dst: &str, src: &str, refs: &RefResolver) -> bool {
    fn head(s: &str) -> &str {
        s.split('<')
            .next()
            .unwrap_or(s)
            .trim_start_matches("const ")
    }
    let (dst, src) = (head(dst), head(src));
    let is_obj = |s: &str| {
        let b = s.as_bytes();
        matches!(b.first(), Some(b'U') | Some(b'A'))
            && b.get(1).map(|c| c.is_ascii_uppercase()).unwrap_or(false)
    };
    if dst == src || !is_obj(dst) || !is_obj(src) {
        return false;
    }
    if refs.is_subclass(dst, src) {
        return true;
    }
    if src == "UObject" {
        return true;
    }
    if src == "AActor" && dst.starts_with('A') {
        return true;
    }
    // batch-30a (C6d, specs/batch29-errortail.md §6d): same convention one level down — a
    // dataflow that assigns an ACharacter-typed value into an `A*`-typed slot only exists
    // in the bytecode because vanilla compiled it, i.e. the destination class IS
    // ACharacter-derived; the wrap direction (Cast<Derived>) is the safe one.
    // (GA_FallingRagdoll RefCpyV residue: `ACharacter& -> AGothicCharacter`.)
    if src == "ACharacter" && dst.starts_with('A') {
        return true;
    }
    false
}

/// True if `tyname` is a UE enum type (`E<Upper>...`) — same shape `cast_to_typename` keys on.
/// Tolerates a leading `const ` (a const-qualified enum is still an enum for cast purposes).
/// A native field type that is a scalar but provably NOT a bool. Used to stop the 1-byte-write
/// bool heuristic from converting an integer class default into `(N != 0)`.
fn is_proven_non_bool_scalar(value: &str) -> bool {
    matches!(
        value.trim_start_matches("const "),
        "int8"
            | "uint8"
            | "int16"
            | "uint16"
            | "int"
            | "int32"
            | "uint"
            | "uint32"
            | "int64"
            | "uint64"
    )
}

/// The declaration-local part of a possibly namespace-qualified type name: `G1R::EWeather` ->
/// `EWeather`. Every family predicate here keys on the leading letters of the BARE name, so a
/// qualified render must be reduced first or an enum stops looking like one.
pub(crate) fn bare_type_name(tyname: &str) -> &str {
    let unqualified = tyname.trim_start_matches("const ");
    match unqualified.rsplit_once("::") {
        Some((_, tail)) => tail,
        None => unqualified,
    }
}

/// Qualify a bare class name for use in an EXPRESSION (`G1R::UStoryG1R::StaticClass()`,
/// `default MainStoryClass = G1R::UStoryG1R;`). A class declared in a namespace is not reachable
/// by its bare name from anywhere else, and `UStoryG1R::StaticClass()` is then read as a
/// The mark a then-arm carries when its last block jumped BACK to the test: a loop the block
/// detectors could not take, because its condition is computed across several blocks. The
/// emitter turns the pair into a `while` once the condition is one expression, and drops the
/// mark when it cannot -- leaving exactly the `if` that stood here before.
pub(crate) const LOOP_BACK_EDGE: &str = "//__gore_back_edge";

/// NAMESPACE access, which fails with "Namespace 'UStoryG1R' doesn't exist".
pub(crate) fn qualify_class_name(name: &str, refs: &RefResolver) -> String {
    match refs.type_ns_by_name(name) {
        Some(namespace) if !name.contains("::") => format!("{namespace}::{name}"),
        _ => name.to_string(),
    }
}

pub(crate) fn is_enum_name(tyname: &str) -> bool {
    let b = bare_type_name(tyname).as_bytes();
    b.len() >= 2 && b[0] == b'E' && b[1].is_ascii_uppercase()
}

/// Coarse type family for the batch-29b `CpyVtoR4/R8` fold gate: the value register can only
/// carry the function's return payload when the pending call's return family matches the
/// function's. `bool` / int-family / float-family buckets; everything else keys on its
/// template-stripped head name (`TMap<A,B>` == `TMap`), so same-head folds keep the status quo.
fn ty_family(t: &str) -> &str {
    let t = t.trim_start_matches("const ");
    match t {
        "bool" => "bool",
        "int" | "int8" | "int16" | "int64" | "uint" | "uint8" | "uint16" | "uint64" => "int",
        "float" | "float32" | "double" => "float",
        _ => t.split('<').next().unwrap_or(t),
    }
}

/// Wrap an enum-typed RHS being stored into an INT slot as `int(expr)`. AngelScript has no
/// implicit enum->int conversion, so an enum field-read / enum-returning call stored into an
/// `int` local fails to compile. Only fires when the value is a known enum AND the dest is an
/// int slot (so enum->enum and enum->enum-param copies stay bare).
/// batch-30c: FLOAT-FAMILY-gated field VALUE type for a member load — cross-module script
/// field maps first, then the in-crate native rows (FVector/FRotator components). Returns
/// None for everything non-float so the callers' conservative fallbacks stay in charge
/// (a broad precise-type flip would change bool/enum member renders that compile today).
fn float_field_type(refs: &RefResolver, tid: i32, field: &str) -> Option<String> {
    let cls = refs.type_by_id(tid)?;
    refs.field_type_by_class(cls, field)
        .or_else(|| refs.native_field_type(cls, field))
        // The declared type from `Binds.Cache` covers ordinary members of native classes and
        // structs, which neither channel above knows. Without it a float default keeps its raw
        // IEEE-754 bits: `PostProcessSettings.ChromaticAberrationStartOffset = 1045220557;`
        // instead of `= 0.1f;` — the compiler then coerces the int and the value is off by ten
        // orders of magnitude.
        .or_else(|| refs.native_field_value_type(cls, field))
        .filter(|t| matches!(*t, "float" | "float32" | "double"))
        .map(|s| s.to_string())
}

fn enum_to_int(rhs: String, src_ty: Option<&str>, dst_is_int: bool) -> String {
    match src_ty {
        Some(t) if dst_is_int && is_enum_name(t) => format!("int({rhs})"),
        _ => rhs,
    }
}

/// Cast an int RHS to a named target type: `bool` -> `(x != 0)`, UE enum
/// (`E<Upper>...`) -> `EEnum(x)`. Returns None when no cast applies.
fn cast_to_typename(rhs: &str, tyname: &str, refs: &RefResolver) -> Option<String> {
    if tyname == "bool" {
        // Already a bool: `(true != 0)` is "No conversion from 'int' to 'bool'".
        if matches!(rhs, "true" | "false") {
            return None;
        }
        return Some(format!("({rhs} != 0)"));
    }
    // The CHECK reduces to the bare name; the CAST keeps the qualified one, which is what has
    // to be written when the enum lives in a namespace.
    let bare = bare_type_name(tyname);
    let b = bare.as_bytes();
    if b.len() >= 2 && b[0] == b'E' && b[1].is_ascii_uppercase() {
        // The cache carries the enumerator NAMES. A constant written as its name is what the
        // source had, and it is not the same expression as one built by a conversion: the
        // compiler stores a named constant where the destination already is, while a conversion
        // is built first and the destination looked up afterwards.
        if let Some(entry) = rhs
            .parse::<i32>()
            .ok()
            .and_then(|value| refs.enumerator_name(&bare, value))
        {
            return Some(format!("{tyname}::{entry}"));
        }
        return Some(format!("{tyname}({rhs})"));
    }
    None
}

/// batch-32c (D9 pure-element carry): true for a side-effect-free container element read
/// rendered from a LOCAL receiver — `local_N.opIndex(<simple>)` with a parenthesis-free,
/// quote-free arg (constants / bare locals / plain member chains). Only such pendings may
/// be tagged carryable across a cast diamond (see `pending_is_pure_elem`): a local
/// container cannot be mutated by the diamond's D7-constrained arms (opCast+TYPEID) or the
/// pure pre-join getters of the proven population, so re-evaluating the read at the join
/// observes the same element.
fn is_pure_elem_read(s: &str) -> bool {
    let Some((recv, call)) = s.split_once(".opIndex(") else {
        return false;
    };
    let Some(args) = call.strip_suffix(')') else {
        return false;
    };
    // batch-33c: receiver widened from bare locals to `this.`-rooted plain member chains
    // (`this.m_MinionsDieHandles.opIndex(local_6)` — MCQueen OnMinionDie's dropped
    // UDelegateHandleContainer arg). The purity argument is unchanged: the D7-constrained
    // diamond arms (opCast+TYPEID only) cannot mutate ANY container, member or local —
    // the LOCAL restriction was belt, not load-bearing. Receivers with calls/indexing/
    // quotes stay rejected by the charset.
    let recv_ok = if let Some(rest) = recv.strip_prefix("local_") {
        !rest.is_empty() && rest.bytes().all(|b| b.is_ascii_digit())
    } else if let Some(rest) = recv.strip_prefix("this.") {
        !rest.is_empty()
            && rest
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'.'))
    } else {
        false
    };
    recv_ok
        && args
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'.' | b',' | b' ' | b'-'))
}

/// batch-33c (D9 pure-read carry, iterator getters): true for a rendered 0-arg
/// `GetKey()`/`GetValue()` on a plain identifier-chain receiver (`Entry.GetKey()` — the
/// UGE_Ex_Damage::GetDamageByMagicCircle param iterator, whose FGameplayTag DamageTag arg
/// died at the D9 bail). Owner-gated at the tagging site to TMap(Const)Iterator, whose
/// getters are pure reads: re-evaluating at the diamond join cannot observe a different
/// value (the D7-constrained arms cannot advance an iterator).
fn is_pure_iter_get(s: &str) -> bool {
    let Some(recv) = s
        .strip_suffix(".GetKey()")
        .or_else(|| s.strip_suffix(".GetValue()"))
    else {
        return false;
    };
    !recv.is_empty()
        && recv
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'.'))
}

/// True if a rendered operand is an integer slot/constant (safe to cast to bool/enum).
/// Excludes already-typed operands (params, fields, calls) so we never double-cast.
/// A bare integer LITERAL (not a slot name). A literal the compiler can prove fits its target
/// needs no narrowing cast; a slot does.
fn is_int_literal(s: &str) -> bool {
    let s = s.strip_prefix('-').unwrap_or(s);
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit())
}

fn looks_int(s: &str) -> bool {
    let s = s.strip_prefix('-').unwrap_or(s);
    if let Some(r) = s.strip_prefix("local_") {
        return !r.is_empty() && r.bytes().all(|b| b.is_ascii_digit());
    }
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit())
}

/// Compound-assignment operator method names -> operator (statement-producing).
fn assign_op(f: &str) -> Option<&'static str> {
    Some(match f {
        "opAssign" => "=",
        "opAddAssign" => "+=",
        "opSubAssign" => "-=",
        "opMulAssign" => "*=",
        "opDivAssign" => "/=",
        _ => return None,
    })
}

/// Binary operator method names -> operator (value-producing).
fn binop_method(f: &str) -> Option<&'static str> {
    Some(match f {
        "opAdd" => "+",
        "opSub" => "-",
        "opMul" => "*",
        "opDiv" => "/",
        "opEquals" => "==",
        _ => return None,
    })
}

/// Whole-word occurrences of `word` in `text` (an identifier is not a substring of a longer one).
fn count_word(text: &str, word: &str) -> usize {
    word_positions(text, word).len()
}

pub(crate) fn word_positions(text: &str, word: &str) -> Vec<usize> {
    let is_word = |b: u8| b.is_ascii_alphanumeric() || b == b'_';
    let bytes = text.as_bytes();
    let mut found = Vec::new();
    let mut at = 0usize;
    while let Some(hit) = text[at..].find(word) {
        let start = at + hit;
        let end = start + word.len();
        let before_ok = start == 0 || !is_word(bytes[start - 1]);
        let after_ok = end == bytes.len() || !is_word(bytes[end]);
        if before_ok && after_ok {
            found.push(start);
        }
        at = end;
    }
    found
}

/// `<receiver>.<Method>(<slot>);` — a one-argument call that takes the slot's type by value or by
/// const reference, which is where a TEMPORARY is legal (`PursuedCrimes.Add(FCrimeSetup());`).
/// The verdict comes from the cache's own parameter table, over every one-parameter row of that
/// name: one non-const-reference overload disqualifies the whole name, so the substitution can
/// never turn into "cannot pass a temporary into a non-const reference parameter".
fn temporary_argument_call(
    statement: &str,
    slot: &str,
    ty: &str,
    value: &str,
    refs: &RefResolver,
    sole_mention: bool,
    resolved: &HashMap<(String, usize), Vec<bool>>,
) -> Option<String> {
    // The sole-argument form, decided by the TYPE-keyed table.
    if let Some(call) = statement.strip_suffix(&format!("({slot});")) {
        let method = call.rsplit(['.', ':']).next()?;
        if !method.is_empty()
            && method
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_')
            && refs.one_arg_call_accepts_temporary(method, ty)
        {
            return Some(format!("{call}({value});"));
        }
        return None;
    }
    // Any OTHER argument position. Two things have to be true, and neither is the by-name table:
    // the slot is mentioned NOWHERE else in the body — a value the caller reads back is read back
    // somewhere — and the parameter of the overload THIS call resolves to takes a value rather
    // than a non-const reference. Deciding the second from the callee's name instead costs 35
    // errors, and adding only the first still leaves 15.
    if !sole_mention {
        return None;
    }
    let call = statement.trim_end().strip_suffix(';')?.trim_end();
    if !call.ends_with(')') || count_word(call, slot) != 1 {
        return None;
    }
    let bytes = call.as_bytes();
    let mut depth = 0i32;
    let mut open = None;
    for at in (0..bytes.len()).rev() {
        match bytes[at] {
            b')' => depth += 1,
            b'(' => {
                depth -= 1;
                if depth == 0 {
                    open = Some(at);
                    break;
                }
            }
            _ => {}
        }
    }
    let open = open?;
    let method = call[..open].rsplit(['.', ':']).next()?;
    if method.is_empty()
        || !method
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_')
    {
        return None;
    }
    let inner = &call[open + 1..call.len() - 1];
    let mut arguments: Vec<&str> = Vec::new();
    let (mut depth, mut start) = (0i32, 0usize);
    for (at, byte) in inner.bytes().enumerate() {
        match byte {
            b'(' | b'<' | b'[' => depth += 1,
            b')' | b'>' | b']' => depth -= 1,
            b',' if depth == 0 => {
                arguments.push(inner[start..at].trim());
                start = at + 1;
            }
            _ => {}
        }
    }
    if !inner.trim().is_empty() {
        arguments.push(inner[start..].trim());
    }
    let position = arguments.iter().position(|argument| *argument == slot)?;
    resolved
        .get(&(method.to_owned(), arguments.len()))
        .and_then(|accepts| accepts.get(position))
        .copied()
        .unwrap_or(false)
        .then(|| {
            let mut rebuilt = arguments.clone();
            rebuilt[position] = value;
            format!("{}({});", &call[..open], rebuilt.join(", "))
        })
}

/// UHT reserves the `b<Uppercase>` prefix for bool UPROPERTYs, so the last path segment of a
/// member reference tells whether the field is one even when no type channel resolved it.
fn is_ue_bool_field(reference: &str) -> bool {
    let field = reference.rsplit(['.', ':']).next().unwrap_or("");
    let bytes = field.as_bytes();
    bytes.len() >= 2 && bytes[0] == b'b' && bytes[1].is_ascii_uppercase()
}

/// Emit `dst = rhs;` if a result is available (object store).
fn flush_store(out: &mut Vec<String>, dst: String, rhs: Option<String>) {
    if let Some(r) = rhs {
        out.push(format!("{dst} = {r};"));
    }
}

/// Escape a string literal for AS source.
fn esc(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}

/// Type carried by a member-address register when `PshRPtr` turns it back into a call
/// argument. Prefer the field's precise script value type; native structs have no field-value
/// metadata in the script cache, so an independently resolved native ENUM type is the next
/// trustworthy witness. `owner_fallback` is PropertyReferences' declaring/owner type and stays
/// last: it is useful for member receivers, but is not the field's value type.
fn member_ref_push_type(
    precise_value: Option<&str>,
    native_enum: Option<&str>,
    owner_fallback: Option<&str>,
) -> Option<String> {
    precise_value
        .or(native_enum)
        .or(owner_fallback)
        .map(str::to_string)
}

/// Decompile one block's instruction range into statements; also return the
/// pending comparison (operands of the last CMP*) for condition recovery.
fn block_stmts(ctx: &Ctx, lo: usize, hi: usize) -> (Vec<String>, Option<Cmp>) {
    let (out, cmp, _) = block_stmts_in(ctx, lo, hi, Vec::new(), false);
    (out, cmp)
}

/// [`block_stmts`] with an explicit INITIAL operand stack and the block's LEFTOVER stack in
/// the return (batch-27 Cast-diamond carry): the carried entries occupy the DEEPEST positions,
/// below everything the block pushes — exactly the runtime layout — and the leftover is
/// returned verbatim after the final flush (the UNRESOLVED retain applies to statements only).
///
/// batch-35a (cross-block reference carry): when `ret_ref_tail` is set (this block flows to a
/// ref-returning function's shared bare `RET` row and ends with a live by-reference pending),
/// the FINAL flush renders that pending as `return <chain>;` instead of a discarded bare
/// statement — the reference lvalue survives the block boundary the RET-row scan-back cannot
/// cross. Only the by-reference pending (`pending_is_ref`) qualifies; any other trailing
/// pending flushes as a bare statement exactly as before.
fn block_stmts_in(
    ctx: &Ctx,
    lo: usize,
    hi: usize,
    init: Vec<Arg>,
    ret_ref_tail: bool,
) -> (Vec<String>, Option<Cmp>, Vec<Arg>) {
    let mut out = Vec::new();
    let mut cmp: Option<Cmp> = None;
    let mut cond: Option<(String, bool)> = None; // (bool value tested by a jump, is-bool-typed)
                                                 // batch-30c: true when a compare / register-load executed AFTER the newest call —
                                                 // order gate for the stale-cmp-vs-live-bool-pending decision at conditional jumps.
    let mut test_after_call = true;
    let mut stack: Vec<Arg> = init; // pushed pointer/value expressions
    let mut value_reg: Option<String> = None;
    let mut obj_reg: Option<String> = None;
    let mut ref_reg: Option<String> = None; // Idiom-B member address
    let mut ref_reg_ty: Option<String> = None; // field type name behind ref_reg (for casts)
                                               // batch-32b (N5, TMap::Find reversal): the field's PRECISE value type behind ref_reg,
                                               // resolved through the this-class fields map / cross-module class-fields index ONLY —
                                               // never the member_type OWNER-type fallback (`CrimeEntry.Directness` typed FCrimeEntry
                                               // instead of ECrimeDirectness false-flagged the reversed order in arg_mismatch_count,
                                               // suppressing the correct `Find(Key, out)` flip). Consumed ONLY by the PshRPtr arg-push
                                               // (arg typing for scoring/cast pairing); the RDR/WRTV wrap logic keeps reading
                                               // `ref_reg_ty` so member-store renders are unchanged. None = unknown = status quo.
    let mut ref_reg_vty: Option<String> = None;
    // batch-25a (G2): ENUM value type of a native struct's field behind ref_reg, from the
    // in-crate/native-API field metadata (`refs::native_field_type`). Used by the WRTV1 guard
    // and, when the precise script-field channel is absent, by PshRPtr's call-argument typing.
    // The lookup is enum-filtered at every assignment, so non-enum native fields remain unknown.
    let mut ref_reg_nfty: Option<String> = None;
    // Unfiltered NATIVE field value type behind ref_reg (Binds.Cache field decls + the in-crate
    // rows). `ref_reg_nfty` keeps its enum filter for the historical consumers; this channel is
    // consumed ONLY by the WRTV rescue below, where a field DECLARED ON A NATIVE BASE (absent
    // from every script-side map) otherwise leaves both type channels empty.
    let mut ref_reg_nvty: Option<String> = None;
    let mut set_consts: HashMap<i32, ConstBits> = HashMap::new(); // last SetV* constant per slot
                                                                  // Slots whose current value came from a MEMBER READ (RDR* after a member-ref load) — real
                                                                  // data values, kept by the nested-call retain (see Arg.keep). Invalidated on overwrite.
    let mut member_read_slots: std::collections::HashSet<i32> = std::collections::HashSet::new();
    // batch-41d: slots currently holding a CONST object handle (a const-returning CALL result,
    // batch-20/21 `downcast` CONSTSTORE, or a const-member-read RefCpyV, batch-32d). A REFCPY
    // member STORE (Fix 1) of such a slot into a non-const member is a "Can't implicitly convert
    // from 'const T' to 'T'" error in generate-mode — so Fix 1 BAILS on a const source (the
    // store is dropped, exactly as pre-batch-41, keeping the slot's const decl intact). Cleared
    // when the slot is re-written with a non-const value.
    let mut const_obj_slots: std::collections::HashSet<i32> = std::collections::HashSet::new();
    // batch-45c (FIX-4): slot -> PARAMETER name, recorded when `RefCpyV wSlot` copies a param
    // into a slot immediately consumed by `CmpPtrNull wSlot` in a short-circuit null-guard
    // (`Me != null && …`). The copy is FOLDED (not materialised) so the guard renders
    // `if (<param> == nullptr)` on the param directly — the vanilla dropped the RefCpyV, leaving
    // the guard testing an UNWRITTEN slot. Folding (vs materialising `local_N = <param>;`) is
    // const-safe: the GVL `Me` param is `const AGothicCharacterState`, so a slot copy would need a
    // const decl (batch-41 cascade risk); a direct null-compare of the const param has no
    // conversion. Consumed + cleared by the very next `CmpPtrNull`.
    let mut guard_param_alias: HashMap<i32, String> = HashMap::new();
    // batch-46a (FIX-5): slot NAME -> value-type name, recorded when a 0-param `$beh0` CONSTRUCT
    // behaviour default-initialises a PSF'd temp slot of a value type (`PSF t; CALLSYS $beh0`,
    // owner e.g. `FGameplayTag`). The construct's own arm skips it (the PSF slot carries no `.ty`,
    // so the `is_value` gate is false) and it falls through the generic `$`-drop — but the temp is
    // then copy-assigned into a member (`PSF t; PshVPtr this; ADDSi <field>; CALLSYS $beh0(1-param)`
    // = the FGameplayTag default-init triad). Recorded here so the Idiom-S copy-assign arm can
    // recover `this.<field> = T();` (a default-constructed value) instead of dropping the whole
    // triad to an empty ctor body. Keyed on the CONSTRUCT owner type so the copy-assign only fires
    // when the source temp was default-built of the SAME value type. Cleared when the slot is
    // overwritten by any non-construct producer.
    let mut default_ctor_temp: HashMap<String, String> = HashMap::new();
    // Per-call parameter flags read from the pointer the call ACTUALLY uses, keyed by name and
    // arity. The global by-name table merges every overload in the cache and answers the
    // constness question wrongly for a name the engine reuses; the pointer names one row.
    let mut resolved_params: HashMap<(String, usize), Vec<bool>> = HashMap::new();
    let mut pending: Option<String> = None; // unconsumed call/ctor result
    let mut pending_ty: Option<String> = None; // recovered type of `pending` (call return type)
                                               // batch-20 Class B: the call returns a CONST object handle (`const UCharacterAIState`).
                                               // `pending_ty` is the const-stripped base name (comparisons everywhere key on it), so the
                                               // constness travels in this parallel flag; the STOREOBJ arm uses it to Cast-strip.
    let mut pending_const: bool = false;
    // batch-29a (C8): the last CALL* returns BY REFERENCE (ret DataType.is_reference from the
    // cache) — the following RDRx dereferences the register the call just filled, so the RDR
    // destination slot receives the call's VALUE. Set alongside `pending_ty` at every call
    // producer (false for non-call producers: ALLOC/CallPtr), so a stale flag can never pair
    // with a live `pending`; cleared by the flush macros with the rest of the pending state.
    let mut pending_is_ref: bool = false;
    // batch-31b (N2b, spec batch31-nomatch-illegalop §1.2): `pending` holds the rendered
    // `n"..."` FName literal of the `PshC4 <id> ; CALLSYS __STATIC_NAME ; PshRPtr` idiom —
    // a PURE literal (no side effect can be reordered), so the PshRPtr push may be tagged
    // `.carry()` and survive the D9 cast-diamond carryability gate (SendGlobalEvent /
    // SpawnRay lost their pre-diamond FName args to the D9 bail). Set ONLY by the CALLSYS
    // arm's __STATIC_NAME resolution; every other pending producer resets it, so a stale
    // flag can never pair with a live non-literal `pending`. Deliberately NOT widened to
    // other pendings (opIndex results etc. are side-effect-bearing — N4 territory).
    let mut pending_is_static_name: bool = false;
    // batch-32c (N4 site-1, spec batch31-nomatch-illegalop §1.8 corrected by disasm): the
    // pending is a side-effect-free container ELEMENT READ from a LOCAL receiver
    // (`local_12.opIndex(0)` — TArray opIndex Thiscall1). Its PshRPtr push may be tagged
    // `.carry()`: re-evaluating the read at the diamond join cannot observe a different
    // value (D7 restricts the arms to opCast+TYPEID, and the pre-join calls of the proven
    // population are pure getters that cannot mutate a local container). Without the tag D9
    // bailed and BOTH leftover args died (IgnoreActorWhenMoving rendered 0-arg ×2). Same
    // lifecycle as pending_is_static_name: set only by the CALLSYS/Thiscall1 arm, reset by
    // every other pending producer.
    let mut pending_is_pure_elem: bool = false;
    let mut ret_val: Option<String> = None;
    // Target type of the most recent TYPEID push — the implicit type operand of the following
    // `opCast` behaviour call (the lowered form of `Cast<T>(x)`). Resolved to a typename so the
    // opCast can be rendered as `Cast<T>(src)` instead of a discarded `src.opCast(out)`.
    let mut last_typeid: Option<String> = None;
    let name = |off: i32| ctx.slot_name(off);
    let w = |ins: &Instr, i: usize| s16(ins.words.get(i).copied().unwrap_or(0));

    macro_rules! flush {
        () => {
            pending_ty = None;
            pending_const = false;
            pending_is_ref = false;
            if let Some(p) = pending.take() {
                out.push(format!("{p};"));
            }
        };
    }

    // Fix b2 — flush a still-pending statement-position call result before the NEXT call would
    // overwrite/drop it (e.g. `MakeRequirement(...)` then `Add(...)`, then a dtor that returns
    // None). UNLIKE `flush!`, this DROPS a pending that carries an ARGMISMATCH sentinel (\u{2})
    // or is unresolved: at the call boundary such a result was previously silently overwritten by
    // the next call's `pending = ...`. Surfacing it as an emitted statement would propagate the
    // sentinel and force-stub the whole function (regression). So only genuinely-recovered,
    // statement-valid results are emitted; bad ones stay dropped exactly as before.
    macro_rules! flush_b2 {
        () => {
            if let Some(p) = pending.take() {
                if !p.contains('\u{2}') && !p.contains(UNRESOLVED) {
                    out.push(format!("{p};"));
                }
            }
            pending_ty = None;
            pending_const = false;
            pending_is_ref = false;
        };
    }

    // Index of a statement that a construct/destruct behaviour's pre-call flush just emitted.
    // A fluent chain is built as one `pending` expression, and the temporary destructor between
    // two links ends it — the next link then finds no pending and takes a leftover operand as
    // its receiver. Kept for exactly one instruction, so a `PshRPtr` that immediately follows
    // can take the statement back and carry the chain on.
    let mut behaviour_flushed: Option<usize> = None;
    let insns = &ctx.instrs[lo..hi];
    for k in 0..insns.len() {
        let ins = &insns[k];
        let n = ins.op.name;
        let flushed_by_behaviour = behaviour_flushed.take();
        // Invalidate a cached SetV* constant when this op overwrites that slot with a
        // NON-constant value (copy, call-result deref, arithmetic, conversion). Otherwise a
        // later float/double field store reads the stale literal instead of the live value.
        let overwrites_slot = !matches!(n, "SetV4" | "SetV8" | "SetV1")
            && (bin_op(n).is_some()
                || iconst_op(n).is_some()
                || n.contains("TO") // numeric conversions (iTOf, fTOi, …) write their dst slot
                || n.starts_with("CpyVtoV")
                || n.starts_with("CpyRtoV")
                || n.starts_with("RDR")
                || matches!(n, "IncVi" | "IncVf" | "DecVi" | "DecVf" | "NEGi" | "NEGf" | "NEGd" | "NOT"));
        if overwrites_slot {
            if let Some(&wd) = ins.words.first() {
                set_consts.remove(&(wd as i16 as i32));
                member_read_slots.remove(&(wd as i16 as i32)); // (RDR* re-inserts in its arm)
            }
        }
        // SetV* is excluded from overwrites_slot (it REGISTERS a const) but still replaces a
        // member-read value with a temporary — invalidate the keep flag.
        if matches!(n, "SetV4" | "SetV8" | "SetV1") {
            if let Some(&wd) = ins.words.first() {
                member_read_slots.remove(&(wd as i16 as i32));
            }
        }
        match n {
            // ---- pushes ----
            // (plain slot/const/global pushes are tagged `.carry()` — safe to carry across a
            // recognized Cast diamond; see `Arg::carryable`.)
            "PshC4" => {
                let b = ins.dwords.first().copied().unwrap_or(0);
                stack.push(Arg::iconst((b as i32).to_string(), ConstBits::W4(b)).carry());
            }
            "PshC8" => {
                let b = ins.qwords.first().copied().unwrap_or(0);
                stack.push(Arg::iconst((b as i64).to_string(), ConstBits::W8(b)).carry());
            }
            "PshV4" | "PshV8" => {
                // A bool slot pushed as an arg must render BARE (`WasCancelled`), not as
                // `(WasCancelled != 0)` — AngelScript has no `bool != 0`. cast_arg wraps every
                // is_int arg feeding a bool param, so a genuine bool slot has to carry its type
                // (is_int=false) to pass through unwrapped. Only exactly-`bool` slots are typed;
                // enum/int slots stay int so their enum/`!= 0` casts still fire.
                let off = w(ins, 0);
                if ctx.slot_type(off).as_deref() == Some("bool") {
                    stack.push(Arg::typed(name(off), Some("bool".to_string())).carry());
                } else if matches!(
                    ctx.slot_type(off).as_deref(),
                    Some("float" | "float32" | "double")
                ) {
                    // batch-20 Class C: a float-family slot pushed by value is a REAL arg (e.g.
                    // SetByCallerMagnitude's Magnitude pushed before a chained GetSpec()); typed
                    // (is_int=false) it survives the nested-call stack-split retain and renders
                    // bare. Left as Arg::int it was dropped as a stranded temporary (17 in-game
                    // errors: "No matching signatures to 'SetByCallerMagnitude(FGameplayTag)'").
                    stack.push(Arg::typed(name(off), ctx.slot_type(off)).carry());
                } else if let Some(cb) = set_consts.get(&off).copied() {
                    // The slot holds a tracked SetV constant (the SetV1/SetV4 -> PshV4 idiom for a
                    // literal flag/amount, e.g. Say's `bUnskippable`). Carry its cbits so the
                    // nested-call retain (`!is_int || cbits.is_some()`) keeps it as a REAL arg
                    // instead of dropping it as a stranded temporary.
                    stack.push(Arg::iconst(name(off), cb).carry());
                } else if member_read_slots.contains(&off) {
                    // Member-read value (this.XP_... amount) — a real arg the retain must keep.
                    stack.push(
                        Arg {
                            s: name(off),
                            is_int: true,
                            keep: true,
                            ..Default::default()
                        }
                        .carry(),
                    );
                } else if ctx.keep_ints.is_some_and(|s| s.contains(&off)) {
                    // batch-25g (spec family G kin): the emit-side pairing proved every value
                    // push of this slot feeds a KNOWN int-family parameter — a real arg
                    // (Math::Min's second operand below a nested call), not a stranded SetV
                    // temporary; the nested-call retain must keep it.
                    stack.push(
                        Arg {
                            s: name(off),
                            is_int: true,
                            keep: true,
                            ..Default::default()
                        }
                        .carry(),
                    );
                } else {
                    stack.push(Arg::int(name(off)).carry());
                }
            }
            "PshVPtr" => stack.push(Arg::typed(name(w(ins, 0)), ctx.slot_type(w(ins, 0))).carry()),
            "PSF" => {
                // &local, unless it's the destination of a following ALLOC
                if insns.get(k + 1).map(|i| i.op.name) != Some("ALLOC") {
                    // &local at the AS source level is implicit (param decides &in/&out) — no `&`.
                    // Tag is_psf so a following `$beh0` construct can recover `slot = T(args)`.
                    stack.push(Arg::psf(name(w(ins, 0)), ctx.slot_type(w(ins, 0))).carry());
                }
                // else: this PSF is the destination local for the following ALLOC; don't push.
            }
            "PshRPtr" => {
                // batch-31a (N9, batch29-3f residue): a VOID call leaves the value register
                // untouched — its pending render can never be what PshRPtr pushes. Consuming
                // it as an arg passed a void call EXPRESSION into the enclosing call
                // ("No matching signatures to 'TMap::Add(AGothicCharacter&, void)'" —
                // AICombatRoleSystem:534 / FXDefinitions_Human:69). Flush the void call as
                // its own statement (flush_b2 keeps the drop-discipline for sentinel/
                // unresolved pendings) and fall through to the register path: ref_reg may
                // hold the REAL operand; an empty register pushes UNRESOLVED (statement-level
                // drop beats silent arg theft, per spec batch31-nomatch-illegalop §1.11).
                // The recovered operand is pushed UNTYPED: `ref_reg_ty` for a foreign member
                // load is member_type's OWNER type (the documented ADDSi poison), and letting
                // it reach cast_arg's value-head guard false-flags the genuinely-recovered
                // operand (AICombatRoleSystem: `TargetRoleGroup.RoleType` typed
                // FCombatRoleGroup vs the TMap's ECombatRole value -> spurious argtype stub).
                // None = unknown = conservative match, same rule the ADDSi arm documents.
                if pending.is_some() && pending_ty.as_deref() == Some("void") {
                    flush_b2!();
                    let s = match value_reg.take() {
                        Some(v) => v,
                        None => ref_reg.clone().unwrap_or_else(|| UNRESOLVED.into()),
                    };
                    stack.push(Arg::typed(s, None));
                    continue;
                }
                // A behaviour's flush ended a chain one instruction ago: take that statement
                // back and carry it as the operand it was about to become.
                if pending.is_none() {
                    if let Some(at) = flushed_by_behaviour.filter(|at| *at + 1 == out.len()) {
                        let statement = out.remove(at);
                        let expr = statement.trim_end_matches(';').to_string();
                        stack.push(Arg::typed(expr, None));
                        continue;
                    }
                }
                // The value register holds a just-completed call's return value; PshRPtr pushes it
                // back onto the operand stack as the NEXT call's argument (e.g. the receiver/arg of
                // a chained call). Prefer that live call result over the stale member-ref register.
                if let Some(p) = pending.take() {
                    // batch-27b: an ASSIGNMENT-shaped pending (batch-26 operator/RVO fold,
                    // `local_4 = this.GetDisplayName()`) must not flow into an arg/receiver
                    // position — the fork rejects assignment expressions as call arguments.
                    // Emit it as its own statement first and push the written lhs slot: the
                    // assignment completes before the consuming call on both renders, so the
                    // dataflow is identical.
                    if let Some(lhs) = assign_lhs(&p) {
                        let lhs = lhs.to_string();
                        out.push(format!("{p};"));
                        stack.push(Arg::typed(lhs, pending_ty.take()));
                    } else if pending_is_static_name || pending_is_pure_elem {
                        // batch-31b (N2b): the pending is the PURE `n"..."`/`FName(...)`
                        // literal of the __STATIC_NAME idiom — no side effect can be
                        // reordered by carrying it across a cast diamond, so it may pass
                        // the D9 carryability gate like any plain const push.
                        // batch-32c widens this to pure local-container element reads
                        // (`local_12.opIndex(0)` — see pending_is_pure_elem).
                        // batch-33d: container reads are tagged reeval — they may only
                        // cross CLASSIC (opCast-arm) diamonds, never the relaxed ones.
                        let mut a = Arg::typed(p, pending_ty.take()).carry();
                        a.reeval = pending_is_pure_elem;
                        stack.push(a);
                    } else {
                        stack.push(Arg::typed(p, pending_ty.take()));
                    }
                } else {
                    let (s, ty) = match value_reg.take() {
                        Some(v) => (v, None),
                        // batch-32b: prefer the poison-free precise field value type for the
                        // ARG TYPING (ref_reg_ty may be member_type's OWNER type, which
                        // false-flags reversal scoring / cast pairing); fall back to the old
                        // channel so unresolvable fields keep the status-quo behavior.
                        None => (
                            ref_reg.clone().unwrap_or_else(|| UNRESOLVED.into()),
                            member_ref_push_type(
                                ref_reg_vty.as_deref(),
                                ref_reg_nfty.as_deref(),
                                ref_reg_ty.as_deref(),
                            ),
                        ),
                    };
                    stack.push(Arg::typed(s, ty));
                }
            }
            "PGA" | "PshGPtr" | "PshG4" => {
                let ptr = ins.qwords.first().copied().unwrap_or(0) as i64;
                if ctx.refs.global_is_string(ptr) {
                    stack.push(
                        Arg::obj(format!(
                            "\"{}\"",
                            esc(ctx.refs.global_by_ptr(ptr).unwrap_or(""))
                        ))
                        .carry(),
                    );
                } else {
                    let nm = ctx.refs.global_by_ptr(ptr).unwrap_or("global?");
                    if let Some(cls) = nm.strip_prefix("__StaticType_") {
                        // Generator class-pointer global. The BARE class name is what compiles
                        // into this push: writing `X::StaticClass()` instead compiles into
                        // `CALL StaticClass` plus a `TSubclassOf` conversion — a different
                        // program, and one that makes the compiler GENERATE a `StaticClass`
                        // free function the base cache never had, which then fails to remap
                        // ("no matching symbol in the base cache"). Vanilla contains both forms;
                        // only this one pushes the global, so render what was actually written.
                        stack.push(Arg::obj(qualify_class_name(cls, ctx.refs)).carry());
                    } else if nm.starts_with("__") {
                        // other implicit generator global (e.g. __WorldContext) — not a
                        // source-level identifier. Push a MARKER (not dropped): the native's arity
                        // counts the hidden WorldContextObject param, so dropping it makes the
                        // split take one entry too deep and steal a neighbouring call's arg
                        // (GetNPCState family). The marker keeps stack arithmetic honest;
                        // build_call strips it from the rendered args + lowers arity by 1.
                        stack.push(Arg::ctx().carry());
                    } else if let Some(ns) = ctx.refs.global_ns(ptr) {
                        stack.push(Arg::obj(format!("{ns}::{nm}")).carry()); // e.g. `FColor::Red`
                    } else {
                        stack.push(Arg::obj(nm.to_string()).carry());
                    }
                }
            }
            "PshNull" => stack.push(Arg::obj("nullptr".into()).carry()),
            "VAR" => stack.push(Arg::int(name(w(ins, 0)))),
            "FuncPtr" => stack.push(Arg::obj("funcptr".into())),
            // ---- member access (Idiom A: rewrite top of stack in place) ----
            "ADDSi" => {
                let off = ins.words.first().copied().unwrap_or(0) as i32;
                let tid = ins.dwords.first().copied().unwrap_or(0) as i32;
                let field = ctx
                    .refs
                    .member(tid, off)
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| format!("field_0x{off:x}"));
                // field VALUE type — from the enclosing class map, else (batch-21 Class B) the
                // injected per-class field maps keyed by the ADDSi type-id's class name, which
                // resolve FOREIGN script-class/struct members correctly (`SaveState.WeatherModifiers`
                // -> `TMap<EWeather, float32>`). `member_type(tid,off)` stays unused here: it
                // returns the OWNER struct (PropertyReferences.OldTypeId = the CONTAINING type),
                // NOT the field's value type; using it poisons foreign member reads (e.g.
                // `HitResult.BoneName` typed as `FHitResult` instead of `FName`) -> false
                // value-head mismatch -> spurious argtype stub. None = unknown = conservative match.
                let fty = ctx
                    .fields
                    .and_then(|m| m.get(&field))
                    .cloned()
                    .or_else(|| {
                        ctx.refs
                            .type_by_id(tid)
                            .and_then(|cls| ctx.refs.field_type_by_class(cls, &field))
                            .map(|s| s.to_string())
                    })
                    // batch-25e (E): composed CONTAINER types of KNOWN native-class fields — the
                    // receiver's `TMap<K, V>` must reach cast_container_args or the int keys of
                    // Add/FindOrAdd/Find on e.g. `m_CustomCollisionResponse` never get their enum
                    // wrap (native owners store no field value type in the script cache).
                    // Table-driven (refs::known_native_field_subtype), capture-candidate-verified.
                    .or_else(|| {
                        ctx.refs
                            .type_by_id(tid)
                            .and_then(|cls| ctx.refs.known_native_field_subtype(cls, &field))
                            .map(|s| s.to_string())
                    })
                    // batch-40 (specs/rgt-and-methods-triage.md PART 1): the FLOAT-FAMILY-gated
                    // precise field type for a NATIVE-struct member store — mirrors the LoadThisR/
                    // LoadRObjR Idiom-B path (line ~1915/1941). The stack-built Idiom-A member store
                    // (`PSF; ADDSi field; PopRPtr; WRTV4 <constSlot>`) never consulted native_field_type
                    // for float, so a `SetV4 w,0x437a0000` const feeding `FLightValues.SourceWidth`
                    // (float32) left `top.ty = None` -> `ref_reg_ty = None` -> the WRTV float_lit
                    // reinterpret didn't fire -> `SourceWidth = 1132068864` (int bits of 250.0f), which
                    // the AS compiler then iTOf-coerces to 1.13e9 (WideShot camera / SetupTransitions
                    // weights, ~140 fns). float_field_type only ever returns float/float32/double, so
                    // this is the PROVABLY-float gate the safety rule demands; every non-float field
                    // stays None and the int RHS is untouched. Absent binds (raw baseline) this
                    // resolves None too, so the stub gate is unaffected.
                    .or_else(|| float_field_type(ctx.refs, tid, &field));
                // batch-25a (G2): when the normal chain can't type the field (NATIVE struct
                // owner), consult the in-crate native-field table — carried in the SEPARATE
                // nfty channel so only the WRTV1 guard sees it (never the call-arg gates).
                let nfty = if fty.is_none() {
                    ctx.refs
                        .type_by_id(tid)
                        .and_then(|cls| ctx.refs.native_field_type(cls, &field))
                        .filter(|t| is_enum_name(t))
                        .map(|s| s.to_string())
                } else {
                    None
                };
                if let Some(top) = stack.last_mut() {
                    top.s = format!("{}.{field}", top.s);
                    top.is_int = false; // now a member access, not a bare int slot
                    top.ty = fty;
                    top.nfty = nfty;
                    // batch-32d: const object-handle native field (always assigned so a
                    // stale flag from an earlier chain link can never leak forward).
                    top.nf_const = ctx
                        .refs
                        .type_by_id(tid)
                        .and_then(|cls| ctx.refs.native_field_const_object(cls, &field))
                        .map(|s| s.to_string());
                }
            }
            "RDSPtr" => {} // deref in place: no change to the rendered name
            // ---- member access (Idiom B: register) ----
            "LoadThisR" => {
                let off = ins.words.first().copied().unwrap_or(0) as i32;
                let tid = ins.dwords.first().copied().unwrap_or(0) as i32;
                let field = ctx
                    .refs
                    .member(tid, off)
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| format!("field_0x{off:x}"));
                // The class field-type map holds the real field VALUE type; member_type()
                // resolves PropertyReferences OldTypeId which is the OWNER class, not the
                // field type — so prefer the map and only fall back to member_type.
                // batch-30c: before that owner-name fallback, try the FLOAT-FAMILY-gated
                // precise sources (cross-module script field maps + the in-crate native
                // float rows) so a float64 member read into an int slot takes the RDR8
                // int(...) wrap instead of the bare precision-warning render. Gated to
                // float names only: a broad flip (e.g. foreign bool fields going bare
                // where the unknowable int(...) wrap compiles today) would regress.
                ref_reg_ty = ctx
                    .fields
                    .and_then(|m| m.get(&field))
                    .cloned()
                    .or_else(|| float_field_type(ctx.refs, tid, &field))
                    .or_else(|| ctx.refs.member_type(tid, off).map(|s| s.to_string()));
                ref_reg_nfty = ctx
                    .refs
                    .type_by_id(tid)
                    .and_then(|cls| ctx.refs.native_field_type(cls, &field))
                    .filter(|t| is_enum_name(t))
                    .map(|s| s.to_string());
                ref_reg_nvty = ctx
                    .refs
                    .type_by_id(tid)
                    .and_then(|cls| {
                        ctx.refs
                            .native_field_value_type(cls, &field)
                            .or_else(|| ctx.refs.native_field_type(cls, &field))
                    })
                    .map(|s| s.to_string());
                // batch-32b: precise value type (poison-free sources only — see decl comment).
                ref_reg_vty = ctx.fields.and_then(|m| m.get(&field)).cloned().or_else(|| {
                    ctx.refs
                        .type_by_id(tid)
                        .and_then(|cls| ctx.refs.field_type_by_class(cls, &field))
                        .map(|s| s.to_string())
                });
                // LoadThisR loads from slot 0. In a METHOD that is `this`; in a FREE (mixin)
                // function slot 0 is parameter 0, so hardcoding `this.` emits an undeclared base
                // -> "'field' is not a member of 'Unknown'". slot_name(0) renders both correctly.
                ref_reg = Some(format!("{}.{field}", name(0)));
            }
            "LoadRObjR" | "LoadVObjR" => {
                let obj = name(w(ins, 0));
                let off = ins.words.get(1).copied().unwrap_or(0) as i32;
                let tid = ins.dwords.first().copied().unwrap_or(0) as i32;
                let field = ctx
                    .refs
                    .member(tid, off)
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| format!("field_0x{off:x}"));
                // batch-30c: float-family-gated precise resolution first (see LoadThisR) —
                // e.g. `local = Start.Z;` (FVector, double) into an int slot must know the
                // source is float so the RDR8 wrap fires.
                ref_reg_ty = float_field_type(ctx.refs, tid, &field)
                    .or_else(|| ctx.refs.member_type(tid, off).map(|s| s.to_string())); // foreign object field
                ref_reg_nfty = ctx
                    .refs
                    .type_by_id(tid)
                    .and_then(|cls| ctx.refs.native_field_type(cls, &field))
                    .filter(|t| is_enum_name(t))
                    .map(|s| s.to_string());
                ref_reg_nvty = ctx
                    .refs
                    .type_by_id(tid)
                    .and_then(|cls| {
                        ctx.refs
                            .native_field_value_type(cls, &field)
                            .or_else(|| ctx.refs.native_field_type(cls, &field))
                    })
                    .map(|s| s.to_string());
                // batch-32b: precise value type (poison-free sources only — see decl comment).
                ref_reg_vty = ctx
                    .refs
                    .type_by_id(tid)
                    .and_then(|cls| ctx.refs.field_type_by_class(cls, &field))
                    .map(|s| s.to_string());
                ref_reg = Some(format!("{obj}.{field}"));
            }
            _ if n.starts_with("RDR") => {
                // batch-29a (C8, specs/batch29-errortail.md §8): a ref-RETURNING call immediately
                // dereferenced — RDRx reads the register the call just filled, so the destination
                // slot receives the call's VALUE (`local_1 = Task.GetResult();`). Without this the
                // pending was flushed as a discarded statement (91 "Result of expression is
                // unused" [W] module-killers) and the destination slot stayed garbage. Gates:
                // `ref_reg` empty (member loads keep the path below byte-identical) and the
                // call's ret DataType.is_reference (data-driven from the cache).
                if ref_reg.is_none() && pending.is_some() && pending_is_ref {
                    let p = pending.take().unwrap();
                    let dst_slot = w(ins, 0);
                    let dst_is_int = dst_slot > 0 && ctx.slot_type(dst_slot).is_none();
                    // Mirror the member-RDR wraps below: an UNKNOWN/object-typed or float-family
                    // ref-returned value read into an int-declared slot takes int(...) (warnings
                    // are errors in-game); a known enum takes enum_to_int; known bool/int stay
                    // bare; RDR8 stays bare (no UE enum is 8 bytes).
                    let unknowable = match pending_ty.as_deref() {
                        None => true,
                        // batch-30c: a template-`?` return (iterator Proceed/opIndex element)
                        // read into an int-declared slot takes the wrap too — int(x) is
                        // neutral for real ints, and the float32-element reads were the
                        // NormalizeWeights precision-warning residue.
                        Some("?") => true,
                        Some(t) => {
                            let t = t.trim_start_matches("const ");
                            matches!(
                                t.bytes().next(),
                                Some(b'U') | Some(b'A') | Some(b'F') | Some(b'T')
                            ) && t
                                .as_bytes()
                                .get(1)
                                .map(|c| c.is_ascii_uppercase())
                                .unwrap_or(false)
                        }
                    };
                    let float_src = matches!(
                        pending_ty
                            .as_deref()
                            .map(|t| t.trim_start_matches("const ")),
                        Some("float" | "float32" | "double")
                    );
                    // batch-30c: RDR8 joins the wrap when the source is KNOWN float-family —
                    // a float64 read into an int slot warns (module-killer); int64 reads
                    // (the reason RDR8 was excluded) stay bare via !float_src.
                    let rhs =
                        if dst_is_int && (unknowable || float_src) && (n != "RDR8" || float_src) {
                            format!("int({p})")
                        } else {
                            enum_to_int(p, pending_ty.as_deref(), dst_is_int)
                        };
                    out.push(format!("{} = {rhs};", name(dst_slot)));
                    member_read_slots.insert(dst_slot); // real data value, not a SetV temporary
                    pending_ty = None;
                    pending_const = false;
                    pending_is_ref = false;
                    continue;
                }
                flush!();
                if let Some(r) = &ref_reg {
                    let dst_slot = w(ins, 0);
                    let dst_is_int = dst_slot > 0 && ctx.slot_type(dst_slot).is_none();
                    // Known-enum source -> int(...) (enum_to_int). Source of UNKNOWN value type
                    // (a FOREIGN member: the cache stores no value type for foreign fields —
                    // PropertyReferences.OldTypeId is the OWNER — and the fields map covers only
                    // this-class) -> wrap too: `local = int(agent.Relationship)` — the hidden
                    // foreign ENUM reads (~150 in-game "Can't implicitly convert E* to int")
                    // compile, and int(x) is neutral for every other type an RDR1/2/4 can read
                    // into an int-defaulted slot (int/bool/float are explicit-int-constructible).
                    // RDR8 keeps the old path (no UE enum is 8 bytes; int64/double reads stay bare).
                    let unknowable = match ref_reg_ty.as_deref() {
                        None => true,
                        Some(t) => {
                            let t = t.trim_start_matches("const ");
                            matches!(
                                t.bytes().next(),
                                Some(b'U') | Some(b'A') | Some(b'F') | Some(b'T')
                            ) && t
                                .as_bytes()
                                .get(1)
                                .map(|c| c.is_ascii_uppercase())
                                .unwrap_or(false)
                        }
                    };
                    // A KNOWN float-family member read into an int-declared slot keeps the
                    // int(...) wrap: warnings are errors in the game compile, and the bare read
                    // is "Implicit conversion from float to integer loses precision". (Before
                    // batch-21's inherited-fields map these reads were mostly `None`-typed and
                    // took the unknowable wrap; a KNOWN bool/int stays bare — proven clean.)
                    let float_src = matches!(
                        ref_reg_ty
                            .as_deref()
                            .map(|t| t.trim_start_matches("const ")),
                        Some("float" | "float32" | "double")
                    );
                    // batch-30c: RDR8 joins the wrap for KNOWN float-family members (the
                    // float64 member -> int slot precision-warning residue: foreign script
                    // config floats, FVector.Z/FRotator.Yaw); int64 member reads stay bare.
                    let rhs =
                        if dst_is_int && (unknowable || float_src) && (n != "RDR8" || float_src) {
                            format!("int({r})")
                        } else {
                            enum_to_int(r.clone(), ref_reg_ty.as_deref(), dst_is_int)
                        };
                    out.push(format!("{} = {rhs};", name(dst_slot)));
                    member_read_slots.insert(dst_slot); // real data value, not a SetV temporary
                }
            }
            _ if n.starts_with("WRTV") => {
                // batch-47d (opIndex-lvalue WRTV, specs/final-tail-triage.md §3.9 DropEmptyTeams):
                // an array-element WRITE `arr[i] = <value>;` lowers as `PshV4 i; PSF arr;
                // Thiscall1 opIndex; WRTV*` — the opIndex Thiscall1 returns a REFERENCE to the
                // element (into `pending`, `pending_is_ref`), and the following WRTV writes the
                // value THROUGH it. There is no PopRPtr, so `ref_reg` is unset and the store
                // dropped: `flush!` emitted the opIndex as a bare `arr[i];` statement and the WRTV
                // silently vanished (DropEmptyTeams' `_IdxToRemappedIdx[i] = <newIdx>` compaction
                // store-backs [0085],[0099] — a REAL mutation bug: the remap table never written).
                // Capture the ref-returning opIndex lvalue BEFORE flush! and route the WRTV into
                // it. Gated HARD: `ref_reg` genuinely unset, `pending_is_ref` (the opIndex returns
                // a reference — a value-returning opIndex would be a temp, never an lvalue), and
                // the pending is a RESOLVED `.opIndex(` call expr (ends ')', no sentinel/unresolved)
                // — so the lvalue is provable and the store cannot target a garbage receiver.
                let opindex_lvalue = if ref_reg.is_none() && pending_is_ref {
                    pending
                        .as_deref()
                        .filter(|p| {
                            p.contains(".opIndex(")
                                && p.ends_with(')')
                                && !p.contains('\u{2}')
                                && !p.contains('\u{1}')
                                && *p != UNRESOLVED
                        })
                        .map(|p| p.to_string())
                } else {
                    None
                };
                if let Some(lval) = opindex_lvalue {
                    // consume the pending (do NOT flush it as a bare statement) and set it as the
                    // write destination; a following field-typed cast is not needed for an element
                    // write (the element type is the array's, matched by the value's own decl).
                    pending = None;
                    pending_ty = None;
                    pending_is_ref = false;
                    let slot = w(ins, 0);
                    let raw = name(slot);
                    let rhs = match ref_reg_ty.as_deref() {
                        Some("float32") => {
                            float_lit(&set_consts, slot, false).unwrap_or(raw.clone())
                        }
                        Some("float") | Some("double") => {
                            float_lit(&set_consts, slot, true).unwrap_or(raw.clone())
                        }
                        _ => raw,
                    };
                    out.push(format!("{lval} = {rhs};"));
                    continue;
                }
                flush!();
                if let Some(r) = &ref_reg {
                    let slot = w(ins, 0);
                    let raw = name(slot);
                    // a constant slot stored into a float/double field carries IEEE-754 bits,
                    // not an int — decode it; else apply the bool/enum/incompatible cast.
                    let source_is_bool = ctx.slot_type(slot).as_deref() == Some("bool");
                    let target_is_bool = ref_reg_ty.as_deref() == Some("bool")
                        || ref_reg_vty.as_deref() == Some("bool");
                    let mut rhs = match ref_reg_ty.as_deref() {
                        _ if source_is_bool && target_is_bool => raw.clone(),
                        Some("float32") => {
                            float_lit(&set_consts, slot, false).unwrap_or(raw.clone())
                        }
                        Some("float") | Some("double") => {
                            float_lit(&set_consts, slot, true).unwrap_or(raw.clone())
                        }
                        Some(t) if looks_int(&raw) => field_assign_rhs(&raw, t, ctx.refs),
                        _ => raw.clone(),
                    };
                    // batch-45b (FIX-3): a member-write on a FOREIGN object (a call-result /
                    // this-member via LoadRObjR/LoadThisR) where `ref_reg_ty` resolved to the
                    // field's OWNER class (member_type's OldTypeId is the owner, not the value
                    // type — see the LoadThisR comment) makes `field_assign_rhs` bail to
                    // UNRESOLVED for a `local_N` RHS, so the whole store dropped and the
                    // quest/document/routine flag was NEVER set (`UStoryG1R::Get().<flag> = ...;`
                    // — a REAL behavioural bug: 18 RegionTrait + 2 Document + Crime writers).
                    // Retry with `ref_reg_vty` (the precise VALUE type from field_type_by_class /
                    // the this-class field map) — a real PRIMITIVE value type. Bail-safe: only
                    // rescues an already-DROPPED store (rhs still UNRESOLVED) and only when vty is
                    // a real primitive value type (never an object/owner name that would re-bail),
                    // so no working store changes.
                    //
                    // `ref_reg_nvty` extends the same rescue to fields DECLARED ON A NATIVE BASE.
                    // Those appear in no script-side map at all, so both channels above are empty
                    // and every such store dropped — which silently erased the scalar class
                    // defaults of every item, weapon and config class (`m_Value`,
                    // `m_SuperArmorDamageBase`, …), since their fields belong to native
                    // `UItemDefinition`/`UWeaponDefinition` rather than to the script class.
                    if rhs == UNRESOLVED {
                        for vty in [ref_reg_vty.as_deref(), ref_reg_nvty.as_deref()]
                            .into_iter()
                            .flatten()
                        {
                            if rhs != UNRESOLVED {
                                break;
                            }
                            let v = vty.trim_start_matches("const ");
                            let is_primitive = matches!(
                                v,
                                "int"
                                    | "uint"
                                    | "int8"
                                    | "int16"
                                    | "int64"
                                    | "uint8"
                                    | "uint16"
                                    | "uint64"
                                    | "float"
                                    | "float32"
                                    | "double"
                                    | "bool"
                            ) || is_enum_name(v);
                            if is_primitive {
                                let cand = match v {
                                    "float32" => float_lit(&set_consts, slot, false)
                                        .unwrap_or_else(|| raw.clone()),
                                    "float" | "double" => float_lit(&set_consts, slot, true)
                                        .unwrap_or_else(|| raw.clone()),
                                    _ if looks_int(&raw) => field_assign_rhs(&raw, v, ctx.refs),
                                    _ => raw.clone(),
                                };
                                if cand != UNRESOLVED {
                                    rhs = cand;
                                }
                            }
                        }
                    }
                    // batch-49a (FIX-3-ext, specs/final-residue.md §A.3a OnGracefulExitRequested):
                    // a 1-byte member write (`SetV1 w1,1; LoadThisR bShouldExitState; WRTV1 w1`)
                    // where BOTH type channels failed — `ref_reg_ty` resolved to the OWNER class
                    // (`member_type`'s OldTypeId is the owner, e.g. `UCharacterAIState`) and
                    // `ref_reg_vty` is None (the field is an INHERITED native bool not in the
                    // this-class map) — so `field_assign_rhs` bailed to UNRESOLVED and the store
                    // dropped: `this.bShouldExitState = true` was LOST before the
                    // `StopWaitingAndContinueTask` call (a REAL exit-state write loss, 2 fns).
                    // Rescue ONLY when the field name follows the engine-enforced UE bool
                    // convention (`b` + ASCII-uppercase), so the field PROVABLY denotes a bool
                    // UPROPERTY (UHT reserves the `b<Upper>` prefix for bools; a non-bool field
                    // never carries it). Bail-safe: rewrites ONLY an already-DROPPED store
                    // (rhs still UNRESOLVED), only for a 1-byte write (bool/int8/uint8 width),
                    // only for an int-family source, and renders the same `(x != 0)` bool wrap the
                    // heuristic below produces for owner-typed fields — so no working store changes.
                    if rhs == UNRESOLVED && n == "WRTV1" && looks_int(&raw) && is_ue_bool_field(r) {
                        if !matches!(raw.as_str(), "true" | "false") {
                            rhs = format!("({raw} != 0)");
                        }
                    }
                    // Same convention, the other way round: the store was NOT dropped but the
                    // constant was already resolved to an int LITERAL, so both wraps below (which
                    // require an untransformed bare slot) skip it and `bRunning = 1` reaches the
                    // compiler as "Can't implicitly convert from 'int' to 'bool&'". A `b<Upper>`
                    // field is a bool UPROPERTY by UHT's own rule, so the literal takes the bool
                    // form. Bounded to a 1-byte write of a plain integer literal.
                    if n == "WRTV1" && is_int_literal(&rhs) && is_ue_bool_field(r) {
                        if !matches!(rhs.as_str(), "true" | "false") {
                            rhs = format!("({rhs} != 0)");
                        }
                    }
                    // batch-25a (G2, specs/batch23-cantconvert.md): a 1-byte write to a NATIVE
                    // struct's field whose value type the in-crate native-field table resolves
                    // to an ENUM is the enum ORDINAL store (`SetV1 w80, 0x2 ... WRTV1 w80`), not
                    // a bool — render `field = EVerticalAlignment(local_80);` via the existing
                    // enum machinery instead of letting the bool heuristic below produce
                    // `(local_80 != 0)` ("bool -> E*&", 50 in-game errors, 14 fns). Guards
                    // mirror the bool heuristic: untransformed bare int slot only, and never a
                    // slot already KNOWN bool/enum (bare enum->enum / bool sources stay bare).
                    if n == "WRTV1"
                        && rhs == raw
                        && rhs != UNRESOLVED
                        && looks_int(&raw)
                        && ctx.slot_type(slot).as_deref() != Some("bool")
                        && !ctx
                            .slot_type(slot)
                            .as_deref()
                            .map(is_enum_name)
                            .unwrap_or(false)
                    {
                        if let Some(ety) = ref_reg_nfty.as_deref() {
                            if let Some(c) = cast_to_typename(&raw, ety, ctx.refs) {
                                rhs = c;
                            }
                        }
                    }
                    // A 1-byte write (WRTV1) to a field whose type we could NOT resolve to bool
                    // (a foreign nested member -> ref_reg_ty is the owner/None) is almost always a
                    // bool UPROPERTY, whose auto-generated accessor is `bool&`: `field = intSlot`
                    // fails "int -> bool&" (e.g. `m_CanAttack = local_1` where local_1 is a
                    // decompiled const-`0` bool). Cast the RHS to bool. Guard: the RHS was NOT
                    // already transformed above (a bare slot), is resolved, and is not a KNOWN
                    // bool slot (which compiles bare and must not become the illegal `bool != 0`).
                    // ENUM guards: uint8 ENUMS are also 1-byte writes. When the SOURCE slot is a
                    // known enum (an enum param stored into a same-enum field:
                    // `this.FocusCharacterType = PerceptionCharacterType` — the bool wrap made it
                    // `(Param != 0)` -> "bool -> EPerceptionCharacterType&", 41 in-game errors),
                    // or the FIELD's value type is a known enum (this-class fields map), the write
                    // is enum->enum: keep it bare.
                    // NATIVE guard: the heuristic reads "1-byte write to a field of unknown type",
                    // and a field declared on a native base used to be exactly that. Now that the
                    // native declaration is readable, a proven non-bool 1-byte field keeps its
                    // integer store — `m_GroundRaysAmount = 2` (uint8) had become
                    // `m_GroundRaysAmount = (2 != 0)`, which is both a different value and a
                    // compile error ("Can't implicitly convert from 'bool' to 'uint8&'").
                    if n == "WRTV1"
                        && ref_reg_ty.as_deref() != Some("bool")
                        && !ref_reg_ty.as_deref().map(is_enum_name).unwrap_or(false)
                        && !ref_reg_nvty
                            .as_deref()
                            .is_some_and(is_proven_non_bool_scalar)
                        && rhs == raw
                        && rhs != UNRESOLVED
                        && ctx.slot_type(slot).as_deref() != Some("bool")
                        && !ctx
                            .slot_type(slot)
                            .as_deref()
                            .map(is_enum_name)
                            .unwrap_or(false)
                    {
                        if !matches!(rhs.as_str(), "true" | "false") {
                            rhs = format!("({rhs} != 0)");
                        }
                    }
                    // The mirror case: a KNOWN bool field written from an int LITERAL (the
                    // `SetV1 w,1` folded into the store). The generated accessor is `bool&`, so
                    // `= 1` is "Can't implicitly convert from 'int' to 'bool&'". A bool SOURCE
                    // slot still stores bare — `bool != 0` would be the illegal form there.
                    if n == "WRTV1" && target_is_bool && !source_is_bool && is_int_literal(&rhs) {
                        if !matches!(rhs.as_str(), "true" | "false") {
                            rhs = format!("({rhs} != 0)");
                        }
                    }
                    // Storing a slot into a NARROW native field warns twice ("signed to
                    // unsigned" + "truncates") and warnings are errors here. Narrow the store
                    // explicitly, mirroring the small-int parameter cast in `cast_arg`. Only a
                    // proven native type narrower than the 4-byte slot qualifies, and only for a
                    // bare slot RHS — a literal that already fits, and every rendering the rules
                    // above produced, are left alone.
                    if matches!(n, "WRTV1" | "WRTV2") && rhs == raw && rhs != UNRESOLVED {
                        if let Some(narrow) = ref_reg_nvty
                            .as_deref()
                            .map(|t| t.trim_start_matches("const "))
                            .filter(|t| matches!(*t, "int8" | "uint8" | "int16" | "uint16"))
                        {
                            if !is_int_literal(&raw) {
                                rhs = format!("{narrow}({rhs})");
                            }
                        }
                    }
                    out.push(format!("{r} = {rhs};"));
                }
            }
            // ---- constants / arithmetic into slots ----
            "SetV8" => {
                flush!();
                let bits = ins.qwords.first().copied().unwrap_or(0); // 64-bit const is in qwords
                set_consts.insert(w(ins, 0), ConstBits::W8(bits));
                // A constant feeding a float/double-used slot is an IEEE-754 value, not an int.
                let rhs = if ctx.float_slots.contains(&w(ins, 0)) {
                    fmt_float(ConstBits::W8(bits), true)
                } else {
                    (bits as i64).to_string()
                };
                out.push(format!("{} = {};", name(w(ins, 0)), rhs));
            }
            "SetV4" | "SetV1" => {
                flush!();
                let bits = ins.dwords.first().copied().unwrap_or(0);
                set_consts.insert(w(ins, 0), ConstBits::W4(bits));
                let rhs = if ctx.slot_type(w(ins, 0)).as_deref() == Some("bool") && bits <= 1 {
                    (bits != 0).to_string()
                } else if ctx.float_slots.contains(&w(ins, 0)) {
                    fmt_float(ConstBits::W4(bits), false)
                } else if ctx
                    .slot_type(w(ins, 0))
                    .as_deref()
                    .is_some_and(is_enum_name)
                {
                    // batch-31c (N3 Fix 2): an ENUM-typed slot (out-param slot typing)
                    // written a raw ordinal needs the explicit conversion — AS has no
                    // implicit int->enum (`EInventoryTypes local_7 = 0;` fails).
                    //
                    // Where the cache carries the enumerator's NAME, write that instead. It is
                    // what the source had, and it is not the same expression: a named constant
                    // goes straight where the destination is, while a conversion is built first
                    // and the destination looked up afterwards.
                    let ty = ctx.slot_type(w(ins, 0)).unwrap();
                    match ctx.refs.enumerator_name(&ty, bits as i32) {
                        Some(entry) => format!("{ty}::{entry}"),
                        None => format!("{ty}({})", bits as i32),
                    }
                } else {
                    (bits as i32).to_string()
                };
                out.push(format!("{} = {};", name(w(ins, 0)), rhs));
            }
            "CpyVtoV4" | "CpyVtoV8" => {
                flush!();
                let (dst, src) = (w(ins, 0), w(ins, 1));
                // batch-20 Class C: a slot declared `bool` (bool& out-ref retype) written from an
                // int-family slot needs the explicit wrap — AS has no implicit int->bool. A
                // genuine bool source stays bare (`bool != 0` doesn't compile either).
                let rhs = if ctx.slot_type(dst).as_deref() == Some("bool")
                    && ctx.slot_type(src).is_none()
                    && looks_int(&name(src))
                {
                    format!("({} != 0)", name(src))
                } else if ctx.slot_type(dst).is_none()
                    && matches!(
                        ctx.slot_type(src).as_deref(),
                        Some("float" | "float32" | "double")
                    )
                {
                    // batch-30c: a float-family-typed slot copied into an UNTYPED
                    // (int-declared) slot is the batch-28 poison residue — the explicit
                    // int(...) kills the precision warning (module-killer), mirroring the
                    // member-RDR wraps; a genuinely-int destination truncates identically.
                    format!("int({})", name(src))
                } else if ctx.slot_type(dst).as_deref() == Some("float32")
                    && matches!(ctx.slot_type(src).as_deref(), Some("float" | "double"))
                {
                    // batch-30c: float64 -> float32 slot copy warns too; float32(x) is the
                    // batch-28 in-game-proven explicit-narrowing syntax.
                    format!("float32({})", name(src))
                } else if ctx.slot_type(dst).as_deref().is_some_and(is_enum_name)
                    && ctx.slot_type(src).is_none()
                    && looks_int(&name(src))
                {
                    // batch-31c (N3 Fix 2): ENUM-typed dst (out-param slot typing) written
                    // from an int slot — mirror of the bool wrap above (no implicit
                    // int->enum in AS): `local_7 = EInventoryTypes(local_8);`.
                    format!("{}({})", ctx.slot_type(dst).unwrap(), name(src))
                } else {
                    name(src)
                };
                out.push(format!("{} = {rhs};", name(dst)));
            }
            _ if bin_op(n).is_some() && ins.words.len() >= 3 => {
                flush!();
                out.push(format!(
                    "{} = {} {} {};",
                    name(w(ins, 0)),
                    name(w(ins, 1)),
                    bin_op(n).unwrap(),
                    name(w(ins, 2))
                ));
            }
            _ if iconst_op(n).is_some() => {
                flush!();
                let bits = ins.dwords.first().copied().unwrap_or(0);
                // ADDIf/SUBIf/MULIf carry an IEEE-754 float immediate, not an int (like CMPIf).
                let c = if n.ends_with('f') {
                    fmt_float(ConstBits::W4(bits), false)
                } else {
                    (bits as i32).to_string()
                };
                out.push(format!(
                    "{} = {} {} {};",
                    name(w(ins, 0)),
                    name(w(ins, 1)),
                    iconst_op(n).unwrap(),
                    c
                ));
            }
            "IncVi" | "IncVf" => {
                flush!();
                out.push(format!("{0} = {0} + 1;", name(w(ins, 0))));
            }
            "DecVi" | "DecVf" => {
                flush!();
                out.push(format!("{0} = {0} - 1;", name(w(ins, 0))));
            }
            // asBC_INCi/DECi (NO_ARG): ++/-- the int at the value/ref register — an lvalue that
            // is a member or deref (LoadThisR/LoadRObjR set ref_reg), unlike IncVi/DecVi which
            // carry a slot operand. The recovered ref expression is evaluated exactly once by
            // the VM; prefix ++/-- preserves that alias/evaluation-order contract and round-trips
            // to the direct opcode. `x = x + 1` instead evaluates a complex lvalue twice and
            // lowers to a read/add/write sequence.
            "INCi" | "INCi64" | "INCi16" | "INCi8" => {
                flush!();
                if let Some(r) = &ref_reg {
                    out.push(format!("++{r};"));
                }
            }
            "DECi" | "DECi64" | "DECi16" | "DECi8" => {
                flush!();
                if let Some(r) = &ref_reg {
                    out.push(format!("--{r};"));
                }
            }
            "NEGi" | "NEGf" | "NEGd" => {
                flush!();
                out.push(format!("{0} = -{0};", name(w(ins, 0))));
            }
            // asBC NOT (opcode 6) is the boolean logical invert. Proven bool locals can use the
            // faithful source operation; unresolved/int scratch keeps the compile-safe integer
            // toggle because AngelScript rejects `!` on int.
            "NOT" => {
                flush!();
                let s = name(w(ins, 0));
                if ctx.slot_type(w(ins, 0)).as_deref() == Some("bool") {
                    out.push(format!("{s} = !{s};"));
                } else {
                    out.push(format!("{s} = int({s} == 0);"));
                }
            }
            // ---- comparisons ----
            "CMPi" | "CMPu" | "CMPf" | "CMPd" | "CMPi64" | "CMPu64" => {
                test_after_call = true;
                cmp = Some(Cmp {
                    a: name(w(ins, 0)),
                    b: name(w(ins, 1)),
                    ..Default::default()
                });
            }
            "CMPIi" | "CMPIu" => {
                test_after_call = true;
                let c = ins.dwords.first().copied().unwrap_or(0) as i32;
                let s = w(ins, 0);
                // an enum compared to an int literal needs explicit int(enum) — AngelScript has
                // no implicit enum<->int (e.g. `if (_AlternativeState != 0)`).
                let a = if ctx
                    .slot_type(s)
                    .as_deref()
                    .map(is_enum_name)
                    .unwrap_or(false)
                {
                    format!("int({})", name(s))
                } else {
                    name(s)
                };
                cmp = Some(Cmp {
                    a,
                    b: c.to_string(),
                    ..Default::default()
                });
            }
            // CMPIf's dword immediate is an IEEE-754 float payload, not an int — render it as
            // a float literal so e.g. `x < 1.0f` doesn't become `x < 1065353216`.
            "CMPIf" => {
                test_after_call = true;
                let bits = ins.dwords.first().copied().unwrap_or(0);
                cmp = Some(Cmp {
                    a: name(w(ins, 0)),
                    b: fmt_float(ConstBits::W4(bits), false),
                    ..Default::default()
                });
            }
            "CmpPtrNull" => {
                test_after_call = true;
                // batch-45c (FIX-4): if the preceding RefCpyV folded a param into this guard slot
                // (`RefCpyV wN(<param>) ; CmpPtrNull wN`), render the guard on the param directly
                // (`<param> == nullptr`) — the vanilla copy was dropped, leaving `local_N`
                // unwritten. `remove` so a later reuse of the slot never re-aliases.
                let slot = w(ins, 0);
                let a = guard_param_alias
                    .remove(&slot)
                    .unwrap_or_else(|| name(slot));
                cmp = Some(Cmp {
                    a,
                    b: "nullptr".into(),
                    ..Default::default()
                });
            }
            // a test op turns the CMP register into a bool; it carries the real relational
            // operator (the jump only carries the true/false sense).
            "TZ" => {
                if let Some(c) = &mut cmp {
                    c.op = Some("==");
                }
            }
            "TNZ" => {
                if let Some(c) = &mut cmp {
                    c.op = Some("!=");
                }
            }
            "TS" => {
                if let Some(c) = &mut cmp {
                    c.op = Some("<");
                }
            }
            "TNS" => {
                if let Some(c) = &mut cmp {
                    c.op = Some(">=");
                }
            }
            "TP" => {
                if let Some(c) = &mut cmp {
                    c.op = Some(">");
                }
            }
            "TNP" => {
                if let Some(c) = &mut cmp {
                    c.op = Some("<=");
                }
            }
            // ---- calls ----
            "CALL" | "CALLINTF" | "CALLBND" => {
                test_after_call = false;
                // Fix b2 — FLUSH a pending statement-position call result before this call starts,
                // so a chained statement call (e.g. MakeRequirement then Add) isn't silently
                // overwritten. Drops sentinel/unresolved pendings (see flush_b2 doc).
                flush_b2!();
                let id = ins.dwords.first().copied().unwrap_or(0) as i32;
                let orig = ctx.refs.func_by_id(id).unwrap_or("func?").to_string();
                // batch-25f: a free fn whose declaration was collision-renamed (`Name_g<mi>`)
                // must resolve to the renamed leaf at call sites in EVERY module — the
                // emit-side text pass only rewrites the DECLARING module's source, so
                // cross-module callers kept the now-nonexistent original name (14 in-game
                // errors: GetCurrent/FetchContext/ApplyTo/GetTuning). Name-keyed lookups
                // (native arity) keep the ORIGINAL name; only the render uses the rename.
                let f = ctx
                    .refs
                    .renamed_free_fn_by_id(id)
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| orig.clone());
                // batch-39 (Idiom C, ctor-member-init.md §3/§4.2): a SCRIPT-STRUCT in-place
                // multi-arg constructor with an RVO out-slot. Shape (WeaponBehaviorSet family,
                // ~135 ctors): `<args...> ; PSF <out> ; CALL <StructType-ctor>`, where the ctor
                // is registered as a struct METHOD returning VOID (it constructs AT the PSF'd
                // out-slot, the receiver on stack TOP), and a following `CopyScript` copies the
                // out-slot into a member (`this.AttackRightSingle = local_60`). build_call pops
                // the out-slot as the receiver, hits `is_type_name(f) -> return None` (line ~867),
                // and DROPS the whole build — leaving the member assigned a default-constructed
                // struct and the recovered arg scalars dead. Recover it as `out = Struct(args);`
                // so the build FLOWS into the CopyScript store (which already renders). The
                // nested sub-struct temp args (FGameplayTag/TSet/TSubclassOf via $beh0/RVO) are
                // separate slots that the $beh0 / RVO-free-call arms already assign or default-
                // construct in EMIT mode (obj_locals gives them types -> the RVO out-slot probe
                // fires and the duplicate is consumed); the FGameplayTag 0-arg default temp is
                // dropped but its slot stays a legal DECLARED default-constructed local, exactly
                // as vanilla passes a default temp. This arm runs only in EMIT mode where slots
                // are typed; decompile mode (untyped slots) fails the gates below and is unchanged.
                //
                // SAFETY (spec top risk — a WRONG/partial arg list is worse than a dropped one):
                // recover ONLY when ALL args resolve CLEANLY. Bail (fall through to the status-quo
                // drop) on ANY of: arg count != declared params; a value-type PSF temp arg with an
                // UNKNOWN type (an unrecovered slot, not a declared temp); an empty/UNRESOLVED/
                // $-/~-/\u{1}-/\u{2}-marked arg; or a render that yields the \u{2} arg-mismatch
                // sentinel (a definite arg-type mismatch, e.g. a mis-resolved const). Never emit a
                // partial or guessed arg list.
                let idiom_c_done = 'idiom_c: {
                    if n != "CALL"
                        || !ctx.refs.is_type_name(&f)
                        || !ctx.refs.is_method_by_id(id)
                        || ctx
                            .refs
                            .func_ret_by_id(id)
                            .map(|d| d.base_name(ctx.refs))
                            .as_deref()
                            != Some("void")
                    {
                        break 'idiom_c false;
                    }
                    let Some(params) = ctx.refs.func_params_by_id(id).filter(|p| !p.is_empty())
                    else {
                        break 'idiom_c false;
                    };
                    let np = params.len();
                    // out-slot = stack TOP: a PSF'd slot whose type head == the struct name and
                    // whose name is a plain local/member lvalue (never a sentinel). is_lvalue_arg
                    // rejects PSF outright (it targets the non-PSF member-store receiver), so the
                    // out-slot's plain-name check is inlined here.
                    fn head(s: &str) -> &str {
                        s.split('<').next().unwrap_or(s)
                    }
                    let plain_lvalue = |s: &str| -> bool {
                        !s.is_empty()
                            && s != UNRESOLVED
                            && !s.contains('\u{1}')
                            && !s.contains('\u{2}')
                            && s.bytes()
                                .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'.')
                            && (s.starts_with("local_") || s.starts_with("this."))
                    };
                    let out_ok = stack
                        .last()
                        .map(|r| {
                            r.is_psf
                                && r.ty.as_deref().map(head) == Some(f.as_str())
                                && plain_lvalue(&r.s)
                        })
                        .unwrap_or(false);
                    // Need out-slot + exactly `np` args below it (STRICT: `>` not `>=` so a
                    // short stack that can't supply the full arg list bails, never guesses).
                    if !out_ok || stack.len() <= np {
                        break 'idiom_c false;
                    }
                    // Validate the args WITHOUT popping (peek), so a bail leaves the stack
                    // untouched and falls through to build_call (exact status-quo behaviour).
                    let base = stack.len() - 1 - np; // index of the deepest of this ctor's args
                    let mut args: Vec<Arg> = stack[base..stack.len() - 1].to_vec();
                    maybe_reverse_args(&mut args, Some(params), ctx.refs);
                    // Bail unless every arg is a cleanly-recovered value:
                    //  - a PSF temp arg MUST carry a known type (a declared temp slot); an
                    //    untyped PSF is an unrecovered slot -> a dangling local reference.
                    //  - no empty / UNRESOLVED / $-/~-/\u{1}-/\u{2}-marked arg.
                    let args_clean = args.iter().all(|a| {
                        !a.s.is_empty()
                            && a.s != UNRESOLVED
                            && !a.s.starts_with('$')
                            && !a.s.starts_with('~')
                            && !a.s.contains('\u{1}')
                            && !a.s.contains('\u{2}')
                            && !(a.is_psf && a.ty.is_none())
                    });
                    if !args_clean {
                        break 'idiom_c false;
                    }
                    let rendered = render_args(&args, Some(params), ctx.refs, None);
                    // A definite arg-type mismatch surfaces as the \u{2} sentinel from cast_arg
                    // -> bail (keep the member's default-constructed struct) rather than emit an
                    // uncompilable build.
                    if rendered.contains('\u{2}') || rendered.contains('\u{1}') {
                        break 'idiom_c false;
                    }
                    // All args resolved cleanly: commit. Pop the out-slot + args and emit the
                    // build; the following CopyScript store (already recovered) flows it into the
                    // member (`this.<field> = <out_slot>`).
                    let out_s = stack[stack.len() - 1].s.clone();
                    stack.truncate(base);
                    flush!();
                    out.push(format!("{out_s} = {f}({rendered});"));
                    true
                };
                if idiom_c_done {
                    pending_is_static_name = false;
                    pending_is_pure_elem = false;
                    continue;
                }
                pending = if f == "StaticClass" {
                    // Fix b1 — StaticClass takes 0 operands; the stack holds the ENCLOSING call's
                    // already-pushed args. Do NOT clear it (clearing destroys those args).
                    pending_ty = None;
                    pending_const = false;
                    pending_is_ref = false;
                    // The class is the StaticClass func's NAMESPACE last-segment (objtype is
                    // NULL for StaticClass; the target class lives in the namespace), not the
                    // calling class — `local = UFoo::StaticClass()` from inside UBar must say UFoo.
                    let cls = ctx
                        .refs
                        .staticclass_class_by_id(id)
                        .or_else(|| ctx.refs.func_owner_by_id(id))
                        .or(ctx.class_name)
                        .unwrap_or("UObject");
                    Some(format!(
                        "{}::StaticClass()",
                        qualify_class_name(cls, ctx.refs)
                    ))
                } else {
                    pending_ty = ctx.refs.func_ret_by_id(id).map(|d| d.base_name(ctx.refs));
                    // CALL-by-id = SCRIPT function: its authoritative signature is the module-region
                    // one WE emit (GetG1R renders `UStoryG1R GetG1R()`, no const), while the
                    // tail-table entry spuriously carries bIsObjectConst (2757 GetG1R stores compile
                    // CLEAN in the batch-19 capture). Never const-wrap script-call results.
                    // batch-41e CONFIRMED: the return DataType flag `is_object_const` is NOT a
                    // reliable const-return signal — GetG1R (UG1RQuest) carries is_object_const=true
                    // identically to GetSelectedItem, yet its emitted decl is legitimately non-const
                    // (emit.rs `ret_sig` re-adds `const` only when the BODY returns a const-marked
                    // slot, which GetG1R does not). Const-wrapping on the flag alone cascades const
                    // to ~900 GetG1R + GetRootNode call sites that contradict their non-const decls.
                    // So the blanket stays; GetSelectedItem's const-member-getter recovery is
                    // reverted at the source instead (the RefCpyV `const_ret_getter` bail below,
                    // batch-41e).
                    pending_const = false;
                    // CALL/CALLINTF/CALLBND by id targets a script FunctionReference. Its cache
                    // declaration is the exact declaration we emit, so both split and render arity
                    // must come from that record. Consulting Binds by name here can collide with a
                    // different native/mixin form: the Binds zero-arg
                    // `GetGothicAbilitySystemComponent()` stole the required Character argument
                    // from the one-arg script global at all 57 hotfix call sites. Native/default-
                    // argument arity remains confined to CALLSYS/Thiscall1 below.
                    let na = None;
                    let trusted = ctx.refs.func_params_by_id(id).map(|p| p.len());
                    let owner = ctx.refs.func_owner_by_id(id);
                    let ret_is_ref = ctx
                        .refs
                        .func_ret_by_id(id)
                        .map(|d| d.is_reference)
                        .unwrap_or(false);
                    pending_is_ref = ret_is_ref; // batch-29a: RDR may consume this call's result
                                                 // batch-30c: a ref-returning call clobbers the VM value register — the
                                                 // same register member loads fill — so a stale member `ref_reg` can no
                                                 // longer be what a later RDR reads (the 29a gate-rest: GetResult()
                                                 // discarded + a stale member read consumed in its place).
                    if ret_is_ref {
                        ref_reg = None;
                        ref_reg_ty = None;
                        ref_reg_nfty = None;
                        ref_reg_vty = None;
                    }
                    // batch-24b shadow gate: a free SCRIPT global (no owner, global namespace)
                    // rendered inside a class method is shadowed by any same-named member in
                    // the class's (native or script) ancestry. Member-name existence (T3
                    // method names, script method decls, Binds when loaded) over-approximates
                    // "such a member exists somewhere" — `::`-qualifying a non-shadowed global
                    // resolves identically, so false positives are harmless; no name sources
                    // -> false (status quo).
                    // batch-32b (N6): the namespaced-fn exclusion is GONE — the emitter never
                    // writes namespace blocks, so a vanilla-namespaced script fn (e.g.
                    // FPerceptionCharacterType::GetName) is a bare GLOBAL in our tree and is
                    // shadowed inside class bodies exactly like an unnamespaced one; `::f`
                    // resolves it. The old gate left `GetName(EPerceptionCharacterType(...))`
                    // resolving against the inherited `UObject::GetName()` (EventResponses ×2,
                    // + the universal-UObject-member rows in member_name_exists).
                    let global_shadowed = !ctx.refs.is_method_by_id(id)
                        && owner.is_none()
                        && ctx.class_name.is_some()
                        && ctx.refs.member_name_exists(&f);
                    // A free/static SCRIPT function declared in a namespace has to be called
                    // qualified, exactly like the native namespaced calls below. The binding
                    // puts a class's companion functions in a namespace named after the class
                    // (`UEffect_GiveExperience::ApplyTo`), so an unqualified call picks a
                    // same-named overload from somewhere else — or none at all. Name-keyed
                    // lookups above keep using the bare `f`; only the render is qualified.
                    let called = match ctx.refs.func_ns_by_id(id) {
                        Some(namespace) if !ctx.refs.is_method_by_id(id) => {
                            format!("{namespace}::{f}")
                        }
                        _ => f.clone(),
                    };
                    build_call(
                        &mut stack,
                        &called,
                        ctx.refs.is_method_by_id(id),
                        ctx.super_ctor,
                        ctx.refs.func_params_by_id(id),
                        na,
                        trusted,
                        owner,
                        ctx.class_name,
                        n == "CALL",
                        pending_ty.as_deref(),
                        ret_is_ref,
                        global_shadowed,
                        ctx.refs,
                    )
                };
                pending_is_static_name = false;
                pending_is_pure_elem = false;
            }
            "CALLSYS" | "Thiscall1" => {
                test_after_call = false;
                // Fix b2 — flush a pending statement-position call result before this call begins
                // (e.g. MakeRequirement's result must be emitted before `Add(...)` overwrites it).
                let before_flush = out.len();
                flush_b2!();
                let ptr = ins.qwords.first().copied().unwrap_or(0) as i64;
                let f = ctx.refs.func_by_ptr(ptr).unwrap_or("syscall?").to_string();
                if let Some(params) = ctx.refs.func_params_by_ptr(ptr) {
                    let accepts: Vec<bool> = params
                        .iter()
                        .map(|p| !p.is_reference || p.is_object_const || p.is_read_only)
                        .collect();
                    match resolved_params.get_mut(&(f.clone(), accepts.len())) {
                        Some(seen) => {
                            for (slot, accepted) in seen.iter_mut().zip(&accepts) {
                                *slot &= *accepted;
                            }
                        }
                        None => {
                            resolved_params.insert((f.clone(), accepts.len()), accepts);
                        }
                    }
                }
                // A behaviour emits no statement of its own and consumes only its own operands,
                // so a statement flushed for it may still belong to a chain the very next
                // `PshRPtr` continues. Note it; the flush itself stays, so nothing is lost if
                // the next instruction is anything else.
                if (f.starts_with('$') || f.starts_with('~')) && out.len() > before_flush {
                    behaviour_flushed = Some(out.len() - 1);
                }
                if f == "opCast" {
                    // `opCast` is the AngelScript handle-downcast behaviour — the lowered form of
                    // `T@ dst = Cast<T>(src)`. The cache renders it `src.opCast(out)` with the cast
                    // RESULT written into the `out` arg slot, then a following RefCpyV copies that
                    // slot into the real destination/return local. Emitting the bare `opCast` call
                    // (and dropping the out-write) leaves the destination unwritten → the function
                    // returns null. Recover it as an assignment `out = Cast<T>(src);` so the value
                    // FLOWS into the store/return. T comes from the preceding TYPEID.
                    let src = stack.pop().map(|a| a.s); // top = receiver = source handle
                    let dst = stack.pop().map(|a| a.s); // next = the `out` destination slot
                    let t = last_typeid.take();
                    if let (Some(dst), Some(src)) = (dst, src) {
                        flush!();
                        // Emit a typed `Cast<T>(src)` only for a real object/script target type
                        // (U*/A*); fall back to a bare passthrough `dst = src` when T is
                        // unresolved or not an object — the value must still FLOW into the store
                        // so the destination/return local is written (never a discarded cast).
                        let rhs = match &t {
                            Some(ty) if ty.starts_with('U') || ty.starts_with('A') => {
                                format!("Cast<{}>({src})", qualify_class_name(ty, ctx.refs))
                            }
                            _ => src,
                        };
                        out.push(format!("{dst} = {rhs};"));
                    }
                    continue;
                }
                if f == "$beh0" {
                    if std::env::var_os("GORE_AS_DEFAULTS_DEBUG").is_some() {
                        eprintln!(
                            "[beh0] params={:?} psf={:?} top={:?} owner={:?}",
                            ctx.refs.func_params_by_ptr(ptr).map(|p| p.len()),
                            stack.last().map(|t| t.is_psf),
                            stack.last().map(|t| t.s.clone()),
                            ctx.refs.func_owner_by_ptr(ptr)
                        );
                    }
                    // batch-46a (FIX-5, GameplayTag default-init triad): a 0-param `$beh0` CONSTRUCT
                    // on a PSF'd temp slot of a VALUE type (owner F*/T*/E*) default-initialises that
                    // temp (`PSF t; CALLSYS $beh0(0-param)`). The temp carries no `.ty` (a bare PSF
                    // slot), so the value-construct arm below skips it and it falls through the
                    // generic `$`-drop. RECORD `slot -> owner_type` here so the following 1-param
                    // copy-assign into a member (`… ADDSi <field>; CALLSYS $beh0(1-param)`) can
                    // recover `this.<field> = T();` from an otherwise-empty ctor body. Recording
                    // only (no `continue`) — the existing arms proceed exactly as before, so this is
                    // additive and cannot alter any current render. Gate: 0-param, PSF receiver, a
                    // value-type owner (never U*/A* objects, which use ALLOC not this behaviour).
                    if ctx.refs.func_params_by_ptr(ptr).map(|p| p.len()) == Some(0) {
                        if let Some(top) = stack.last() {
                            if top.is_psf {
                                if let Some(owner) = ctx.refs.func_owner_by_ptr(ptr) {
                                    if matches!(
                                        owner.bytes().next(),
                                        Some(b'F') | Some(b'T') | Some(b'E')
                                    ) {
                                        default_ctor_temp.insert(top.s.clone(), owner.to_string());
                                    }
                                }
                            }
                        }
                    }
                    // `$beh0` is the AngelScript value-type / struct in-place CONSTRUCT behaviour:
                    // `PSF <slot> ; <args...> ; CALLSYS $beh0` constructs the value AT the PSF'd
                    // slot (the receiver, top of stack). build_call drops every `$`-prefixed
                    // behaviour and clears the stack, so the construct AND its write to the slot
                    // vanished, leaving the slot uninitialised (then read downstream as garbage /
                    // passed to a call). Recover it as `slot = T(args);` so the value FLOWS.
                    if let Some(recv) = stack.last().cloned() {
                        // G1c-b (batch-26 stage 2): 1-arg copy-construct whose receiver is the
                        // hidden RVO return slot (PshVPtr-pushed -> is_psf false):
                        // `$beh0(__return, src)` == `return src;`. The PSF gate below can never
                        // match it (return slot is not PSF'd), so it fell to the generic `$`-drop
                        // and the function returned the RVODEF default (`return __r;` — compiles,
                        // loses the value). Mirrors the CopyScript dst=="__return" capture.
                        if recv.s == "__return"
                            && !recv.is_psf
                            && ctx.refs.func_params_by_ptr(ptr).map(|p| p.len()) == Some(1)
                            && stack.len() >= 2
                        {
                            let src = stack[stack.len() - 2].clone();
                            if !src.s.is_empty()
                                && src.s != UNRESOLVED
                                && !src.s.starts_with('\u{2}')
                            {
                                stack.truncate(stack.len() - 2);
                                // batch-43 (Fix 2) / batch-47b (FIX-2b): a `$beh0(__return, src)`
                                // that DEFAULT-CONSTRUCTS-then-returns the RVO struct in a block
                                // ending in JMP (not RET) — the JMP goes to the shared tail RET and
                                // `ret_val` is block-local, so the value would be LOST (the tail RET
                                // ships the RVODEF default). Emit `__return = src;` as a STATEMENT so
                                // emit.rs declares `{ret} __return;` and folds the tail
                                // `return RVODEF;` to `return __return;` (region_exit_stmt +
                                // fold_rvo_return in the switch case). batch-43 scoped this to
                                // `rvo_switch_region`; FIX-2b generalises it to ANY non-RET block
                                // (the branch/loop early-return that was deferred). When the block
                                // ends in RET, keep the byte-identical `ret_val` path (folds to
                                // `return src;` with no duplicate store).
                                if ctx.instrs[hi - 1].op.name != "RET" {
                                    flush!();
                                    out.push(format!("__return = {};", src.s));
                                } else {
                                    ret_val = Some(src.s);
                                }
                                continue;
                            }
                        }
                        // Gate (a): receiver is a PSF'd slot with a known VALUE/struct/template
                        // type (F*/T*/E*). Never a `$`/`?` placeholder, never an object (U*/A*):
                        // object construction uses ALLOC, not this in-place behaviour.
                        let is_value = recv
                            .ty
                            .as_deref()
                            .and_then(|t| t.bytes().next())
                            .map(|b| matches!(b, b'F' | b'T' | b'E'))
                            .unwrap_or(false);
                        if recv.is_psf && is_value {
                            let ty = recv.ty.clone().unwrap();
                            stack.pop(); // remove the receiver; the rest are the ctor args
                            let params = ctx.refs.func_params_by_ptr(ptr);
                            // Consume ONLY this ctor's own args from the TOP (mirror build_call's
                            // EDIT A/B-PRIME) — a whole-stack `mem::take` would also drain the
                            // ENCLOSING call's operands (the 4 EQuestState args of MakeRequirement),
                            // re-dropping them once Fix b1 keeps them on the stack.
                            let mut args: Vec<Arg> = match params.map(|p| p.len()) {
                                Some(k) if stack.len() > k => stack.split_off(stack.len() - k),
                                _ => std::mem::take(&mut stack),
                            }
                            .into_iter()
                            .filter(|x| !x.s.is_empty() && x.s != UNRESOLVED)
                            .collect();
                            // Ctor args are reverse-pushed like every other call's (top =
                            // params[0]); rendering the collected (deepest-first) order emitted
                            // them BACKWARDS — `FTimerDynamicDelegate(n"Fn", this)` against the
                            // declared `(UObject Object, FName FunctionName)` ctor (43 in-game
                            // "No matching signatures to 'FTimerDynamicDelegate(const FName,
                            // <obj> const)'"). Same reverse-by-default scoring as every call arm.
                            maybe_reverse_args(&mut args, params, ctx.refs);
                            // (batch-29c note: the spec's §2.1 suggestion to drop a lone
                            // `nullptr` ctor arg is deliberately NOT taken — the corpus has
                            // thousands of clean `TSubclassOf<T>(nullptr)` sites outside the
                            // error tail, so the null-handle ctor provably compiles and the
                            // composed render below joins that population byte-faithfully.)
                            // Gate (b): a PSF arg normally means a copy/convert ctor whose true
                            // source may be an unrecovered pending call result, so it stays dropped.
                            // The strictly-proved same-type local-copy lowering is safe, however:
                            // `PSF source:T; PSF dest:T; T::$beh0(const T&)`. Render that through
                            // the ordinary `dest = T(source)` construct path; every missing or
                            // conflicting type witness retains the historical drop.
                            let any_psf_arg = args.iter().any(|a| a.is_psf);
                            let proven_psf_copy = is_proven_same_type_psf_copy(
                                &recv,
                                &args,
                                ctx.refs.func_owner_by_ptr(ptr),
                                params.map(|p| p.len()),
                            );
                            // Gate (c): arg count matches the ctor's declared param count (no
                            // spurious leftover operands on the stack).
                            let count_ok = params.map(|p| p.len() == args.len()).unwrap_or(false);
                            // Gate (b) exists because a PSF arg MAY be an unrecovered pending
                            // call result. When this block has already WRITTEN that slot, it is
                            // nothing of the sort — it is a plain local, and dropping the
                            // construct loses a real value (`Texts.Add(FVoiceLine(local_4, …))`
                            // vanished, leaving `local_4` unread and the module's defaults
                            // frozen). Multi-parameter only: a one-parameter construct from a
                            // same-typed PSF slot is the copy behaviour that `proven_psf_copy`
                            // already rules on.
                            // A one-parameter construct from a DIFFERENT type is a conversion
                            // (`TSoftObjectPtr<X>(FSoftObjectPath)`), not the copy behaviour the
                            // strict rule guards; a same-typed one stays with `proven_psf_copy`.
                            let converts = params
                                .and_then(|p| p.first())
                                .map(|only| only.base_name(ctx.refs) != ty)
                                .unwrap_or(false);
                            let psf_args_written_here = (params.map(|p| p.len()).unwrap_or(0) >= 2
                                || converts)
                                && args.iter().filter(|a| a.is_psf).all(|a| {
                                    out.iter().any(|statement| {
                                        statement
                                            .trim_start()
                                            .strip_prefix(a.s.as_str())
                                            .is_some_and(|rest| rest.starts_with(" = "))
                                    })
                                });
                            if !args.is_empty()
                                && (!any_psf_arg || proven_psf_copy || psf_args_written_here)
                                && count_ok
                            {
                                let rendered = render_args(&args, params, ctx.refs, None);
                                // Gate (d): a definite arg-type mismatch -> drop (keep prior
                                // behaviour: slot unwritten) rather than emit the `\u{2}` sentinel
                                // that would force-stub the whole function.
                                if !rendered.contains('\u{2}') {
                                    flush!();
                                    out.push(format!("{} = {ty}({rendered});", recv.s));
                                }
                            }
                            continue;
                        }
                        // batch-37 (Idiom S, ctor-member-init.md §2/§4.1): value-type FIELD
                        // default-init. `PGA/CALL <const> ; PshVPtr this ; ADDSi this.<field> ;
                        // CALLSYS $beh0` is the ctor-path 1-arg value copy/init behaviour whose
                        // receiver is a `this.<field>` member LVALUE (not PSF -> the construct arm
                        // above skipped it; not `__return` -> the return arm above skipped it). The
                        // A/B control proves the render path exists: the RUNTIME `FString::opAssign`
                        // (a DIFFERENT ptr, name `opAssign` not `$beh0`) renders the identical shape
                        // via build_call's method arm, but this ctor-init `$beh0` was dropped by the
                        // generic `$`-behaviour guard -> the store vanished (Armor ctors decompiled
                        // to an EMPTY body; corpus-wide 0 `this.X = "..."` stores). Recover it as
                        // `this.<field> = <value>;` so the member default FLOWS. Census (cache_A):
                        // 1867 member-store sites across FString/FName/FGameplayTag/FVector/
                        // TSoftObjectPtr/FRotator/FKey — every clean rhs a literal/enum/global.
                        //
                        // SAFETY (spec top risk): a WRONG rhs (mis-resolved const / dropped &inout
                        // cast) is WORSE than a dropped one. Fire ONLY on a 1-value-param behaviour
                        // whose receiver `is_lvalue_arg` (this.<field>/local_N; NOT bare `this`,
                        // NOT __return, NOT PSF/sentinel) AND whose rhs is a cleanly-resolved value
                        // (not empty/UNRESOLVED/$/~/\u{2}, not PSF). Any failure BAILS to the
                        // status-quo drop (fall through to the generic `$`-guard). The rhs is cast
                        // to the param type via the SAME `cast_arg` path as the method arm, so a
                        // definite value-type head mismatch force-drops via the `\u{2}` sentinel.
                        if is_lvalue_arg(&recv)
                            && ctx.refs.func_params_by_ptr(ptr).map(|p| p.len()) == Some(1)
                            && stack.len() >= 2
                        {
                            let rhs = stack[stack.len() - 2].clone();
                            // batch-46a (FIX-5): GameplayTag default-init triad. The 1-param `$beh0`
                            // COPY-ASSIGN (`this.<field> = t`) whose source `t` is a PSF temp that a
                            // preceding 0-param `$beh0` default-CONSTRUCTED (recorded in
                            // `default_ctor_temp`). The generic Idiom-S `rhs_ok` rejects a PSF source,
                            // so the whole triad dropped -> empty ctor. Recover `this.<field> = T();`
                            // (a default-constructed value). SAFETY: fire ONLY when (a) the source
                            // slot was default-constructed of a value type, and (b) that type EQUALS
                            // this copy-assign's owner (the field's value type) — so the render is
                            // provably `<field's own type>()`, never a fabricated value. Any mismatch
                            // or unrecorded slot falls through to the normal `rhs_ok` bail. `T()` is a
                            // pure default ctor (no args), so it cannot introduce a wrong value.
                            if rhs.is_psf {
                                if let Some(ctor_ty) = default_ctor_temp.get(&rhs.s) {
                                    let assign_owner = ctx.refs.func_owner_by_ptr(ptr);
                                    if assign_owner == Some(ctor_ty.as_str()) {
                                        let ty = ctor_ty.clone();
                                        default_ctor_temp.remove(&rhs.s); // consume the record
                                        stack.truncate(stack.len() - 2); // consume recv + temp
                                        flush!();
                                        out.push(format!("{} = {ty}();", recv.s));
                                        continue;
                                    }
                                }
                            }
                            // A PSF source is normally an unrecovered temporary, which is why it
                            // was rejected. When this block has already WRITTEN that slot, it is
                            // a plain local carrying a real value, and dropping the store loses
                            // it: `FontHoldToSkip = local_12;` vanished from a constructor and
                            // left the member default-constructed and empty.
                            let psf_written_here = rhs.is_psf
                                && out.iter().any(|statement| {
                                    statement
                                        .trim_start()
                                        .strip_prefix(rhs.s.as_str())
                                        .is_some_and(|rest| rest.starts_with(" = "))
                                });
                            let rhs_ok = (!rhs.is_psf || psf_written_here)
                                && !rhs.s.is_empty()
                                && rhs.s != UNRESOLVED
                                && !rhs.s.starts_with('$')
                                && !rhs.s.starts_with('~')
                                && !rhs.s.contains('\u{2}')
                                && !rhs.s.contains('\u{1}');
                            if rhs_ok {
                                // cast the rhs to the init param type exactly like the method
                                // operator arm — an FName/enum/soft-object default takes its proper
                                // literal form; a definite value-type mismatch yields the `\u{2}`
                                // sentinel, which the guard below turns into a status-quo drop.
                                let r = ctx
                                    .refs
                                    .func_params_by_ptr(ptr)
                                    .and_then(|p| p.first())
                                    .map(|pt| cast_arg(&rhs, pt, ctx.refs))
                                    .unwrap_or_else(|| rhs.s.clone());
                                if !r.contains('\u{2}') && !r.contains('\u{1}') {
                                    stack.truncate(stack.len() - 2); // consume recv + rhs
                                    flush!();
                                    out.push(format!("{} = {};", recv.s, r));
                                    continue;
                                }
                            }
                        }
                    }
                    // not a recoverable in-place construct / member-init -> fall through to the
                    // generic `$` drop (operands cleared by build_call; status-quo behaviour).
                }
                pending = if f == "StaticClass" {
                    // Fix b1 — do NOT clear the stack; StaticClass takes 0 operands and the entries
                    // present belong to an ENCLOSING call.
                    pending_ty = None;
                    pending_const = false;
                    pending_is_ref = false;
                    let cls = ctx
                        .refs
                        .staticclass_class_by_ptr(ptr)
                        .or_else(|| ctx.refs.func_owner_by_ptr(ptr))
                        .or(ctx.class_name)
                        .unwrap_or("UObject");
                    Some(format!(
                        "{}::StaticClass()",
                        qualify_class_name(cls, ctx.refs)
                    ))
                } else {
                    pending_ty = ctx.refs.func_ret_by_ptr(ptr).map(|d| d.base_name(ctx.refs));
                    // batch-31e (capture.batch30-0705 ForceRemoveDamage regression): the T3
                    // entry for `TSubclassOf::GetDefaultObject` records the call-site
                    // SPECIALIZED return (UElectrifiedArena_GornSleeper), but our render
                    // collapses the TSubclassOf receiver onto the raw `T::StaticClass()`
                    // expression, which the LIVE compiler resolves to the UClass overload
                    // returning UObject ("Can't implicitly convert from 'UObject' to
                    // 'UElectrifiedArena_GornSleeper'"). Render with the SOURCE-level type:
                    // with the specialized pending_ty the STOREOBJ downcast saw src == dst
                    // (30a's exact-type merge keeps the vanilla obj_locals type) and skipped
                    // the load-bearing Cast — batch-29 only compiled here by accident (a
                    // member MIS-typing to UWeaponDefinition forced a Cast). Gated to the
                    // exact broken shape — an untyped receiver whose rendered expression IS
                    // a `T::StaticClass()` call (statically UClass in-game). Receivers
                    // recovered as TSubclassOf<T> (or unrecovered foreign members, which
                    // resolve the typed overload in-game) keep the specialized type; a wider
                    // untyped-receiver gate added a benign-but-unfaithful Cast on 9 clean
                    // sites (incl. the CombatMoves sentinel) — rejected as non-minimal.
                    if f == "GetDefaultObject"
                        && ctx.refs.func_owner_by_ptr(ptr) == Some("TSubclassOf")
                        && stack
                            .last()
                            .is_some_and(|r| r.ty.is_none() && r.s.ends_with("::StaticClass()"))
                    {
                        pending_ty = Some("UObject".to_string());
                    }
                    pending_const = ctx
                        .refs
                        .func_ret_by_ptr(ptr)
                        .is_some_and(|d| d.is_object_handle && d.is_object_const);
                    let na = ctx.refs.native_arity_by_ptr(ptr, &f);
                    // A free/static native function in a namespace (Gameplay, Math, System, ...)
                    // must be called qualified `Namespace::func(...)` or the global lookup fails
                    // with "No matching signatures". (Methods carry no namespace -> rendered via
                    // their receiver, unchanged. Arity lookup still uses the bare name + owner.)
                    let qualified = match ctx.refs.func_ns_by_ptr(ptr) {
                        Some(ns) => format!("{ns}::{f}"),
                        None => f.clone(),
                    };
                    // NATIVE call by ptr: prefer the Binds native arity (`na`) for RENDER arity,
                    // but for the STACK SPLIT fall back to the cache FunctionReference param count
                    // when Binds is absent, instead of take-all. Take-all (`mem::take`) DRAINED the
                    // whole operand stack, so a nested native call (e.g. `Topic.GetOther()`,
                    // `state.GetAI()` — 0-param getters) run in the middle of an enclosing call's
                    // arg-push ANNIHILATED the enclosing args -> 0/under-arg calls (Remember(),
                    // UnlockDocumentSegment(), AddTag(), Subdialog). This is rendering-neutral for
                    // the call being built (the existing top-`w` trim selects the same args; an
                    // implicit-`this` overcount steals one deeper entry that the trim then drops),
                    // and only PRESERVES deeper enclosing operands that take-all destroyed.
                    let trusted = call_frame_arity(
                        n,
                        na.or_else(|| ctx.refs.func_params_by_ptr(ptr).map(|p| p.len())),
                    );
                    // target_owner (T3 ObjectType) was only wired for CALL/CALLINTF; pass it for
                    // native calls too so the UObject-receiver Cast wrap (batch19 class 1) and the
                    // generated-accessor qualification see the owning class of CALLSYS methods.
                    let ret_is_ref = ctx
                        .refs
                        .func_ret_by_ptr(ptr)
                        .map(|d| d.is_reference)
                        .unwrap_or(false);
                    pending_is_ref = ret_is_ref; // batch-29a: RDR may consume this call's result
                                                 // batch-30c: mirror of the by-id arm — a ref-returning native call
                                                 // clobbers the value register; invalidate a stale member ref_reg.
                    if ret_is_ref {
                        ref_reg = None;
                        ref_reg_ty = None;
                        ref_reg_nfty = None;
                        ref_reg_vty = None;
                    }
                    build_call(
                        &mut stack,
                        &qualified,
                        ctx.refs.is_method_by_ptr(ptr),
                        ctx.super_ctor,
                        ctx.refs.func_params_by_ptr(ptr),
                        na,
                        trusted,
                        ctx.refs.func_owner_by_ptr(ptr),
                        ctx.class_name,
                        false,
                        pending_ty.as_deref(),
                        ret_is_ref,
                        false,
                        ctx.refs,
                    )
                };
                // batch-31b: tag a resolved static-name FName literal (see the flag's doc).
                // build_call returns the literal only for the accessor name; a failed gate
                // (non-constant Id operand) falls to the `$`-drop and returns None.
                pending_is_static_name = f == "__STATIC_NAME" && pending.is_some();
                // batch-32c: tag a pure local-container element read (see the flag's doc).
                // batch-33c: + TMap(Const)Iterator's pure GetKey/GetValue getters (owner-
                // gated by-ptr; see is_pure_iter_get).
                pending_is_pure_elem = (f == "opIndex"
                    && pending.as_deref().map(is_pure_elem_read).unwrap_or(false))
                    || (matches!(f.as_str(), "GetKey" | "GetValue")
                        && matches!(
                            ctx.refs.func_owner_by_ptr(ptr),
                            Some("TMapIterator" | "TMapConstIterator")
                        )
                        && pending.as_deref().map(is_pure_iter_get).unwrap_or(false));
            }
            "CallPtr" => {
                let f = name(w(ins, 0));
                pending_ty = None;
                pending_const = false;
                pending_is_ref = false;
                pending_is_static_name = false;
                pending_is_pure_elem = false;
                pending = build_call(
                    &mut stack,
                    &f,
                    false,
                    ctx.super_ctor,
                    None,
                    None,
                    None,
                    None,
                    ctx.class_name,
                    false,
                    None,
                    false,
                    false,
                    ctx.refs,
                );
            }
            // ---- object construction ----
            "ALLOC" => {
                let tptr = ins.qwords.first().copied().unwrap_or(0) as i64;
                let ty = ctx.refs.type_by_ptr(tptr).unwrap_or("Object").to_string();
                let args: Vec<String> = std::mem::take(&mut stack)
                    .into_iter()
                    .filter(|a| !a.s.is_empty())
                    .map(|a| a.s)
                    .collect();
                pending_ty = Some(ty.clone());
                pending_const = false;
                pending_is_ref = false;
                pending_is_pure_elem = false;
                pending_is_static_name = false;
                pending = Some(format!("{ty}({})", args.join(", ")));
            }
            // ---- result capture ----
            "STOREOBJ" => {
                let slot = w(ins, 0);
                let rhs = match pending.take() {
                    Some(p) => Some(downcast(
                        p,
                        pending_ty.take(),
                        std::mem::take(&mut pending_const),
                        ctx.local_types.and_then(|m| m.get(&slot)),
                        ctx.refs,
                    )),
                    None => obj_reg.take(),
                };
                // batch-41d: track whether this slot now holds a CONST object handle (the
                // downcast() CONSTSTORE marker means a const-returning call result stored
                // same-type). A later REFCPY member store of this slot must bail (const->non-const
                // field). Cleared for a non-const write so a slot reused later is not stale.
                if rhs.as_deref().is_some_and(|r| r.contains(CONSTSTORE)) {
                    const_obj_slots.insert(slot);
                } else {
                    const_obj_slots.remove(&slot);
                }
                flush_store(&mut out, name(slot), rhs);
            }
            "CpyRtoV4" | "CpyRtoV8" => {
                // batch-21 Class C shape 3: a VOID call has no register value to copy —
                // `local_N = VoidCall();` fails "No conversion from 'void' to 'int'". Emit the
                // call as its own statement; the copied register value (stale, unmodeled) is
                // unrecoverable, so the assignment is dropped (slot keeps its prior value).
                if pending.is_some() && pending_ty.as_deref() == Some("void") {
                    if let Some(p) = pending.take() {
                        out.push(format!("{p};"));
                    }
                } else if let Some(p) = pending.take() {
                    // an enum-returning call stored into an int slot needs an explicit int(...)
                    let dst_slot = w(ins, 0);
                    let dst_is_int = dst_slot > 0 && ctx.slot_type(dst_slot).is_none();
                    let rhs = enum_to_int(p, pending_ty.as_deref(), dst_is_int);
                    out.push(format!("{} = {rhs};", name(dst_slot)));
                } else if let Some(condition) = k
                    .checked_sub(1)
                    .map(|j| &insns[j])
                    .filter(|prev| {
                        matches!(prev.op.name, "TZ" | "TNZ" | "TS" | "TNS" | "TP" | "TNP")
                    })
                    .and(cmp.as_ref())
                    .and_then(materialized_comparison)
                {
                    // The value register holds the RESULT of the comparison the `T*` op just
                    // tested: the source read it as a value, not as a branch. Dropped, the slot
                    // is never written and every read of it renders the declaration's default —
                    // a wrong VALUE, not merely a different shape (measured: 124 functions, 30 of
                    // them comparing against a non-zero constant we returned as zero).
                    // AngelScript has no implicit bool-to-int, so an int-typed slot takes the
                    // same explicit form the `NOT` rendering already uses.
                    let dst = w(ins, 0);
                    let value = if ctx.slot_type(dst).as_deref() == Some("bool") {
                        condition
                    } else {
                        format!("int{condition}")
                    };
                    out.push(format!("{} = {value};", name(dst)));
                    cmp = None;
                } else if n == "CpyRtoV8" && ref_reg.is_some() {
                    // batch-32a (A5, specs/batch29-errortail.md §1.2 / illegal-op-round2.md A5):
                    // `LoadRObjR/LoadVObjR ; CpyRtoV8 wD` captures the member ADDRESS the load
                    // just put in the register into a slot — an alias the body then uses as the
                    // member's value (`local_52.opIndex(i).RoleType`, RebalanceRoles: 22
                    // "Illegal operation on 'int'"). Emit `dst = obj.field;`. Two hard gates:
                    // (1) the PREVIOUS instruction must be the member load — a stale `ref_reg`
                    //     surviving across calls/compares must not resurrect here (CMPd;TZ;
                    //     CpyRtoV4 copies TEST results, and only CALL arms clear ref_reg);
                    // (2) the emit-side typing half must have adopted the field's VALUE type
                    //     for the destination (final slot_type == the cross-module-resolved
                    //     value type) — untypeable/conflicted sites keep the status-quo DROP
                    //     instead of assigning an object expression into an `int` decl.
                    // Value semantics are a deep copy where the bytecode held a reference —
                    // the documented A5 caveat (uses in the proven cluster are reads/element
                    // refs); decompile mode (no local_types) fails gate (2) and is unchanged.
                    let prev_load = k
                        .checked_sub(1)
                        .map(|j| &insns[j])
                        .filter(|p| matches!(p.op.name, "LoadRObjR" | "LoadVObjR"));
                    if let Some(pl) = prev_load {
                        let off = pl.words.get(1).copied().unwrap_or(0) as i32;
                        let tid = pl.dwords.first().copied().unwrap_or(0) as i32;
                        let vty = ctx.refs.member(tid, off).and_then(|fname| {
                            ctx.refs
                                .type_by_id(tid)
                                .and_then(|cls| ctx.refs.field_type_by_class(cls, fname))
                        });
                        let dst_slot = w(ins, 0);
                        if let Some(vty) = vty {
                            if ctx.slot_type(dst_slot).as_deref() == Some(vty) {
                                if let Some(r) = &ref_reg {
                                    out.push(format!("{} = {r};", name(dst_slot)));
                                    member_read_slots.insert(dst_slot); // real data value, not a SetV temporary
                                }
                            }
                        }
                    }
                }
                pending_ty = None;
                pending_const = false;
            }
            "LOADOBJ" => obj_reg = Some(name(w(ins, 0))),
            "CpyVtoR4" | "CpyVtoR8" => {
                test_after_call = true;
                // batch-21 Class C shape 3: a pending VOID call has no value to copy — the
                // register content comes from the SLOT operand (e.g. a bool& out-param the call
                // just wrote: `CalculateDistanceToTarget(.., local_3); return local_3;`).
                // Flush the call as its own statement and use the slot.
                //
                // batch-29b (C6e, specs/batch29-errortail.md §5): the same slot-over-pending
                // discipline for two more provably-wrong folds. (1) An ASSIGNMENT-shaped
                // pending (batch-26 operator/RVO-dest fold) is a STATEMENT — the register
                // holds the SLOT operand's value, not the assignment expression. (2) A call
                // whose return-type FAMILY mismatches the enclosing function's return type
                // can't be the return payload either — the proven class is a BOOL
                // `TMap::Find(key, out)` folded into a FLOAT return whose true payload is
                // the out-param slot (`return this.Severities.Find(ERelationship(local_5),
                // local_2);` — 3 "No conversion from 'bool' to 'float'" errors). Flush and
                // use the slot; unknown types on either side keep the status-quo fold.
                let foldable = pending
                    .as_deref()
                    .map(|p| assign_lhs(p).is_none())
                    .unwrap_or(true)
                    && match (
                        pending_ty.as_deref(),
                        ctx.ret_ty.map(|t| t.base_name(ctx.refs)),
                    ) {
                        (Some("void"), _) => false,
                        (Some(pt), Some(rt)) => ty_family(pt) == ty_family(&rt),
                        _ => true, // either side unknown -> status quo (fold)
                    };
                if pending.is_some() && !foldable {
                    flush!();
                }
                // batch-24a (G1): in an RVO function the 4/8-byte value register NEVER carries
                // the on-stack struct payload — it holds branch/loop condition values (e.g.
                // `local_11 = int(iter.CanProceed)`), and a stale capture surviving to RET
                // emitted `return local_11;` from an FVector function (138 int -> F*/T*
                // cant-convert errors). The one legitimate ret_val source for RVO functions is
                // the CopyScript-to-`__return` capture below.
                if !ctx.ret_via_rvo() {
                    ret_val = pending.take().or_else(|| Some(name(w(ins, 0))));
                }
            }
            "CpyVtoR1" => {
                test_after_call = true;
                // a bool moved into the test register: feeds either RET or a conditional jump.
                // A call result is already bool-typed; a slot holds the bool as an int —
                // EXCEPT a slot DECLARED bool (a `bool` param / bool-typed local), which must
                // render bare: `if (bIncludeDefeated != 0)` fails "No conversion from 'int'
                // to 'bool'" (batch-21 Class C, 36 in-game errors).
                //
                // batch-21 Class C shape 3: a pending VOID call can never be the tested/returned
                // VALUE — the register content genuinely comes from the SLOT operand (typically a
                // bool& out-param the call just wrote: `CalculateDistanceToTarget(.., local_3);
                // return local_3;`). Folding it in rendered `return VoidCall(...)` / `if
                // (VoidCall(...) == 0)` ("No conversion from 'void' to ..."). Flush the call as
                // its own statement and use the slot.
                //
                // batch-29b (C5, specs/batch29-errortail.md §5): folding `pending` is only
                // valid when the pending expression IS the tested bool (a bool-returning call
                // immediately consumed — the `if (X.IsValid())` population). An ASSIGNMENT-
                // shaped pending (batch-26 operator/RVO-dest fold: `local_4 =
                // this.GetSensedLivingEnemies(...)`) is a STATEMENT whose value the register
                // does NOT hold — CpyVtoR1 copies the NAMED SLOT operand (bytecode proof:
                // CheckAnyHostilesOrEnemiesSensed tests w65532, a bool param), and the folded
                // render `if (x = call() == 0)` is unparseable (18 parse [E]s, 4 module
                // killers) AND tests the wrong value. A non-bool pending can't be the 1-byte
                // test value either. Flush both as their own statement and test the slot.
                let foldable = pending_ty.as_deref() == Some("bool")
                    && pending
                        .as_deref()
                        .map(|p| assign_lhs(p).is_none())
                        .unwrap_or(false);
                if pending.is_some() && !foldable {
                    flush!();
                }
                let is_bool = (pending.is_some() && pending_ty.as_deref() == Some("bool"))
                    || (pending.is_none() && ctx.slot_type(w(ins, 0)).as_deref() == Some("bool"));
                let v = pending.take().unwrap_or_else(|| name(w(ins, 0)));
                cond = Some((v.clone(), is_bool));
                // batch-24a (G1): `cond` handling unchanged (CpyVtoR1 still feeds branches),
                // but a test-register bool is never an RVO struct payload — don't let it
                // linger as a stale return-value candidate (see CpyVtoR4/R8 above).
                if !ctx.ret_via_rvo() {
                    ret_val = Some(v);
                }
            }
            "RET" => {
                let non_void = ctx.ret_ty.map(|t| t.token != 0x52).unwrap_or(false);
                // a value only belongs in a non-void return; for void, `ret_val` may hold a
                // condition value (CpyVtoR1) that must NOT become `return x;`.
                // Capture the return value BEFORE flush!: a directly-returned call/ctor result
                // lives in `pending` (e.g. `CALL`/`ALLOC` then `RET`), and flush! would emit it
                // as a standalone statement, leaving the return a default (RVODEF).
                let mut v = if non_void && ctx.ret_via_rvo() {
                    // batch-24a (G1): an RVO struct payload never travels through the
                    // object/value registers — `ret_val` here can only be the CopyScript-to-
                    // `__return` capture, or a pending CALLSYS opAssign that writes the RVO
                    // slot ITSELF (`__return = <rhs>`; strip_return_assign folds it back to
                    // `return <rhs>;` — the pre-fix text of those legitimate sites). No
                    // obj_reg/other-pending fallback, no scan-back (both resurrect stale
                    // branch-condition captures -> `return local_11;` from an FVector
                    // function). None -> RVODEF; the emitter folds it to `return __return;`
                    // when the body wrote the slot.
                    ret_val.take().or_else(|| {
                        pending
                            .as_deref()
                            .is_some_and(|p| p.starts_with("__return = "))
                            .then(|| pending.take())
                            .flatten()
                    })
                } else if non_void {
                    // batch-21 Class C shape 3: a pending VOID call is never the return VALUE
                    // (`return CalculateDistanceToTarget(...);` from a bool function fails
                    // "No conversion from 'void' to 'bool'") — leave it for flush! below
                    // (statement position) and fall back to scan-back / the typed default.
                    ret_val.take().or_else(|| obj_reg.take()).or_else(|| {
                        (pending_ty.as_deref() != Some("void"))
                            .then(|| pending.take())
                            .flatten()
                    })
                } else {
                    None
                };
                flush!();
                // batch-44a (E4, specs/loop-body-cfg-ext.md §2.4): recover the terminal
                // register-return value when the RET's own capture is a sentinel. Today the
                // scan-back fires ONLY when `v` is exactly None; but a value-type RVO destructor
                // (`PSF wT; CALLSYS ~T`/`$beh2`) intervening between the value copy
                // (`CpyRtoV4 wN`) and the terminal `CpyVtoR4 wN; RET` can leave `v` holding a
                // sentinel string (empty / `~`-dtor / `$`-behaviour / ARGMISMATCH `\u{2}`) rather
                // than the genuine return slot. Extend the guard so the scan-back ALSO runs for a
                // sentinel `v` and recovers the terminal `CpyVtoR4 wN` slot as `return local_N;`.
                // Additive + bail-safe: this can only REPLACE a broken `return;`/`return ~x;` with
                // the scan-back slot (`.or(v)` keeps the sentinel if the scan finds nothing); a
                // legitimate `return <expr>;` (a real slot/call) is a non-sentinel `v` and is
                // untouched. RVO functions are excluded (they never scan-back — batch-24a G1).
                if non_void
                    && !ctx.ret_via_rvo()
                    && v.as_deref().map_or(true, |s| {
                        s.is_empty()
                            || s.starts_with('~')
                            || s.starts_with('$')
                            || s.contains('\u{2}')
                    })
                {
                    v = scan_back_retval(ctx, lo + k).or(v);
                }
                // value fix-ups (RVO-assign strip, declared-bool, int -> bool/enum cast) and
                // the RVODEF default all live in the shared helper (also used by the switch
                // recovery's `JMP -> RET-row` return exits).
                out.push(ctx.return_stmt(v));
            }
            // Idiom-A member store: an ADDSi chain builds `this.a.b` on the stack top, then
            // PopRPtr moves that address into the reference register for the following WRTV.
            // (Ignoring it left the member expression on the stack -> phantom call args.)
            "PopRPtr" => {
                if let Some(top) = stack.pop() {
                    ref_reg = Some(top.s);
                    ref_reg_ty = top.ty;
                    ref_reg_nfty = top.nfty; // batch-25a: native enum field type for WRTV1
                    ref_reg_vty = None; // batch-32b: the popped entry's ty is already precise
                }
            }
            // RefCpyV (wW_ARG): copy the top-of-stack handle into the destination slot named by
            // the word operand — `dst_slot = src`. This is the step that moves an opCast result
            // (or any local handle) from a temp into the real destination/return local; dropping
            // it was a primary cause of `return`ing an unwritten (null) local. Emit it as a
            // statement (NOT a stack push) so it cannot reintroduce phantom call args.
            //
            // GUARD: only emit when the source is a recovered local slot (`local_N`) or a
            // `Cast<...>` expression — a genuine temporary whose copy completes a recovered
            // dataflow. Copying a const PARAMETER (rendered as its bare name) into a non-const
            // local yields "Can't implicitly convert from 'const X' to 'X'", so those stay
            // dropped (the prior conservative behaviour); the value is re-read where used.
            "RefCpyV" => {
                let dst = name(w(ins, 0));
                if let Some(top) = stack.pop() {
                    // batch-41a (Fix 2, S2/S5): also accept a MEMBER-ACCESS source
                    // (`this.member`, built by `PshVPtr;ADDSi;RDSPtr`). Such a source is a legal
                    // RHS of a handle assignment (the field's accessor is `T@`) and was NEVER the
                    // const PARAMETER the original guard excluded. This un-stubs the object-member
                    // getters (`return this.member;`) and fixes the null-check operand for the
                    // null-guard shape. Exclude call/ctor expressions (`(`) — those have their own
                    // paths — sentinels (\u{2}) and UNRESOLVED.
                    let member_src = top.s.contains('.')
                        && !top.s.contains('(')
                        && !top.s.contains('\u{2}')
                        && top.s != UNRESOLVED;
                    // batch-41e (CASE B): BAIL when this const native-field member read would
                    // become the function's RETURNED value, forcing an object-const RETURN type.
                    // A const-member getter body (`return this.<constMember>;`) makes emit.rs render
                    // `const T GetX()`; every caller then needs a `const T` slot, and the script
                    // CALL arm cannot const-propagate (the return DataType flag is unreliable —
                    // GetG1R carries is_object_const=true identically yet is legitimately non-const,
                    // so flagging on it cascades const to ~900 unrelated call sites). Rather than a
                    // fragile, wide const-cascade, keep the ONE getter stubbed (its pre-batch-41
                    // state): dropping this store leaves the returned slot undeclared, so the
                    // LOADOBJ..RET stubs the whole function (1 stub, ZERO caller cascade).
                    // Precise gate: source is a const native-field read (`top.nf_const`) AND the
                    // enclosing function returns an object-const handle of the SAME type. This
                    // matches ONLY GetSelectedItem (`const UItemDefinition` return). It does NOT
                    // touch GetSelectedItemAction (return is non-const UActionKeywords) nor the
                    // CharacterAI_Gothic const-member READS that feed null-checks / GetClass()
                    // receivers (their functions return FItemActionHandler / bool / void, never an
                    // object-const handle of the member's type) — those keep their batch-32d/41
                    // `const T` local recovery.
                    let const_ret_getter = top.nf_const.is_some()
                        && ctx.ret_ty.is_some_and(|d| {
                            d.is_object_handle
                                && d.is_object_const
                                && d.base_name(ctx.refs) == *top.nf_const.as_deref().unwrap_or("")
                        });
                    // batch-43 (Fix 1, switch-rvo-return.md §4.1): accept a CALL-EXPRESSION
                    // source ONLY when the destination is the function's return out-slot — the
                    // object/struct being popped INTO the return value (`<out> = arr.opIndex(0)`
                    // in `PopBucketFront`, an opIndex result RefCpyV'd into the returned slot).
                    // A call-expr source is normally dropped (a phantom copy could reorder side
                    // effects into a non-return local), but into the out-slot the copy IS the
                    // return-value write and must survive so the terminal `LOADOBJ <out>; RET`
                    // renders `return <out>;` (else the popped element is silently lost). Gate
                    // hard: a resolved call expression (`…)`), never a sentinel/unresolved/
                    // destructor, and the destination is exactly the proven out-slot.
                    let call_src_to_out = ctx.is_return_out_slot(w(ins, 0))
                        && top.s.ends_with(')')
                        && !top.s.contains('\u{2}')
                        && !top.s.contains('\u{1}')
                        && top.s != UNRESOLVED
                        && !top.s.starts_with('~');
                    // batch-45c (FIX-4): a param copied into a slot that the VERY NEXT op
                    // null-guards (`RefCpyV wN(<param>) ; CmpPtrNull wN`) in a short-circuit
                    // `<param> != null && …` chain. Vanilla drops the RefCpyV, so `local_N` is
                    // never written and `if (local_N == nullptr)` guards garbage. FOLD the copy:
                    // record slot -> param so the following CmpPtrNull renders `if (<param> ==
                    // nullptr)` directly (no slot decl -> const-safe for the `const` GVL `Me`).
                    // Hard gate: source is an object/ref PARAM, and insns[k+1] is exactly
                    // `CmpPtrNull` on this SAME slot (never touches a non-guard RefCpyV).
                    let dst_slot0 = w(ins, 0);
                    let param_guard_fold = ctx.param_object_ref(&top.s)
                        && insns
                            .get(k + 1)
                            .is_some_and(|nx| nx.op.name == "CmpPtrNull" && w(nx, 0) == dst_slot0);
                    if param_guard_fold {
                        flush!();
                        guard_param_alias.insert(dst_slot0, top.s.clone());
                        continue;
                    }
                    // batch-46c (FIX-7): the RefCpyV result is consumed ONLY by a following
                    // `CmpPtr` in this block (the `if (this.Children.Last() == FailedNode)` guard of
                    // the UCBT `OnChildNode{Failed,Succeeded}` overrides). A getter CALL-expression
                    // source (`this.Children.Last()`) and a bare object PARAMETER source both drop
                    // today (the `ok` whitelist rejects `(`-bearing call exprs and non-local params),
                    // leaving `local_4`/`local_6` UNDECLARED in `if (local_4 != local_6)`. Accept the
                    // copy ONLY when the destination slot is an operand of a following `CmpPtr` BEFORE
                    // it is re-defined — the copy materialises the compare operand, so it cannot
                    // reorder a side effect past its sole consumer (the batch-43 call-src caution was
                    // about copies flowing into arbitrary non-return locals; here the copy is
                    // immediately compared, never re-observed). Bounded forward scan within the block
                    // locates that `CmpPtr` and returns the PAIRED operand slot (the compare partner).
                    let cmp_paired_slot: Option<i32> = {
                        let mut paired = None;
                        for j in (k + 1)..insns.len().min(k + 9) {
                            let nx = &insns[j];
                            match nx.op.name {
                                "CmpPtr" if w(nx, 0) == dst_slot0 => {
                                    paired = Some(w(nx, 1));
                                    break;
                                }
                                "CmpPtr" if w(nx, 1) == dst_slot0 => {
                                    paired = Some(w(nx, 0));
                                    break;
                                }
                                // the slot is re-defined (output word 0) by another value producer
                                // before any compare -> not this idiom, bail.
                                "RefCpyV" | "CpyRtoV4" | "CpyRtoV8" | "RDR4" | "RDR8" | "SetV4"
                                | "SetV8" | "SetV1"
                                    if w(nx, 0) == dst_slot0 =>
                                {
                                    break;
                                }
                                _ => {}
                            }
                        }
                        paired
                    };
                    let feeds_following_cmp = cmp_paired_slot.is_some();
                    // A getter is a resolved CALL expression (ends `)`, no sentinel/unresolved).
                    let src_is_getter = top.s.ends_with(')')
                        && !top.s.contains('\u{2}')
                        && !top.s.contains('\u{1}')
                        && top.s != UNRESOLVED
                        && !top.s.starts_with('~');
                    // batch-46c (FIX-7): recover the getter feeding a `CmpPtr` — a container
                    // accessor (`this.Nodes.Last(0)`, `this.m_Targets.opIndex(0)`) whose result is
                    // consumed only by the compare. Safe: a call result is non-const, materialising
                    // it into a slot for the compare cannot reorder a side effect past its sole use.
                    let getter_into_cmp = feeds_following_cmp && src_is_getter;
                    // batch-49c (opIndex-read-into-null-compare, specs/final-tail-triage.md §3.11
                    // Contains): a container/map element read whose result is null-guarded by the
                    // VERY NEXT op — `… CALLSYS opIndex; PshRPtr; RDSPtr; RefCpyV wN; CmpPtrNull wN`
                    // (`local_N = m_SpawnedAreas[type]; if (local_N == nullptr)`). Today the
                    // getter call-expr source drops (the `ok` whitelist rejects `(`-bearing exprs
                    // outside the out-slot), leaving `local_N` UNDECLARED and the null-guard reading
                    // an uninitialised slot — a real bug (Contains returns garbage-nullness).
                    // FIX-4's `param_guard_fold` only folds a PARAM into CmpPtrNull; FIX-7's
                    // `getter_into_cmp` only fires for the binary `CmpPtr`. Add the getter→CmpPtrNull
                    // case: materialise the copy so the compare reads the real element. Safe — the
                    // getter result is non-const and its SOLE consumer is the immediate CmpPtrNull,
                    // so no side effect is reordered past its use (same argument as getter_into_cmp).
                    let getter_into_null_cmp = src_is_getter
                        && insns
                            .get(k + 1)
                            .is_some_and(|nx| nx.op.name == "CmpPtrNull" && w(nx, 0) == dst_slot0);
                    // batch-46c (FIX-7): recover the PARAM operand of a getter-vs-param compare
                    // (`if (this.Nodes.Last() == FailedNode)` — the OnChildNode override idiom, and
                    // the `opIndex(0) == OtherActor` overlap checks). Gated HARD on the paired
                    // CmpPtr operand being a GETTER-produced slot (a `RefCpyV <paired>` whose source
                    // is a call expression, scanned across the whole block): this scopes the param
                    // materialisation to the getter-compare idiom and AVOIDS the wide "every param
                    // into a null/identity guard" population. CRITICAL: use `param_src_ok` (NOT the
                    // const-agnostic `param_object_ref`) so a CONST/read-only param is EXCLUDED —
                    // materialising `const AActor TargetActor` into a non-const slot is the batch-41/
                    // 45c "const -> non-const" generate-mode cascade (a const compare partner would
                    // stay dropped; FIX-4 folds const params, it never materialises them). The
                    // OnChildNode `FailedNode`/`SucceededNode` + overlap `OtherActor` params are all
                    // non-const, so the idiom is fully covered; const-param compares bail safely.
                    let param_into_cmp = ctx.param_src_ok(&top.s)
                        && cmp_paired_slot.is_some_and(|ps| {
                            // find the `RefCpyV <ps>` that produced the paired slot, then confirm it
                            // is a GETTER result and NOT another param. The getter idiom lowers as
                            // `Thiscall1/CALLSYS; PshRPtr; RDSPtr; RefCpyV <ps>` — the deref `RDSPtr`
                            // IMMEDIATELY precedes the copy (a PARAM copy is `PshVPtr; RefCpyV`, with
                            // no RDSPtr). Requiring `insns[qj-1] == RDSPtr` AND a getter call in the
                            // short lead distinguishes a getter partner from a param-vs-param compare
                            // (CheckTargetChanged's `PreviousTarget == NewTarget`, where a nearby
                            // FStatID profiler CALLSYS falsely satisfied a call-only probe).
                            (1..insns.len()).any(|qj| {
                                insns[qj].op.name == "RefCpyV"
                                    && w(&insns[qj], 0) == ps
                                    && insns[qj - 1].op.name == "RDSPtr"
                                    && insns[qj.saturating_sub(4)..qj]
                                        .iter()
                                        .any(|g| matches!(g.op.name, "Thiscall1" | "CALLSYS"))
                            })
                        });
                    // batch-47a (FIX-8, specs/final-tail-triage.md §3.3): materialise a NON-const
                    // object/ref PARAM copy `local_N = <param>;` when the copy's dest slot is later
                    // CONSUMED by a `CmpPtr` (binary pointer compare, either operand) or by a
                    // MEMBER-ACCESS base (`PshVPtr <dst>; ADDSi …` — the param-alias store idiom)
                    // several ops later, and is NOT immediately null-guarded (that adjacent-guard
                    // case is FIX-4's fold). Today FIX-4 only folds when insns[k+1] is CmpPtrNull;
                    // FIX-7's `param_into_cmp` only fires when the compare PARTNER is a getter-
                    // produced RefCpyV slot. Neither covers `if (OtherActor == GetAvatar())`
                    // (partner produced by STOREOBJ, not RefCpyV), the pure param-vs-param
                    // `if (PreviousTarget == NewTarget)` (CheckTargetChanged), nor the param-alias
                    // `local_2 = Data; local_2.SourceActor = SourceActor;` (InitializeValidatorData)
                    // — so the copy drops and the comparison/member-access reads an UNINITIALISED
                    // slot: a real behavioural bug (verified vanilla+regen on
                    // UGA_FireDemon_FireExplosion::OnTargetReceived — regen compares garbage vs
                    // GetAvatar()). Bounded forward scan for the FIRST consumer within the block;
                    // BAIL if the slot is re-defined (alias-shadowed) before any consumer, so a
                    // reused slot can never be mis-materialised. Const-safe: `param_src_ok` excludes
                    // read-only/object-const params (the b41d/45c const-cascade class) and `this`,
                    // exactly like FIX-4/FIX-7 — a materialised copy of a non-const object PARAM into
                    // a plain local is a legal handle assign. Additive: a copy that is neither a
                    // getter-compare nor an immediate null-guard was DROPPED before, so recovering
                    // it can only ADD the vanilla store back.
                    // A CONST param, or `this`, copied for a COMPARISON. `param_src_ok` refuses
                    // both — rightly, for a copy that is written through or handed on, where
                    // const would not hold. A comparison cannot break it, and dropping the copy
                    // is not a byte difference but a WRONG PROGRAM: the comparison then reads an
                    // uninitialised slot, so `if (Node == OtherNode)` became `if (Node == null)`
                    // and the function always returned false.
                    let const_src_into_cmp = (top.s == "this"
                        || (ctx.param_object_ref(&top.s) && ctx.param_is_const(&top.s)))
                        && insns[k + 1..].iter().any(|nx| {
                            nx.op.name == "CmpPtr"
                                && (w(nx, 0) == dst_slot0 || w(nx, 1) == dst_slot0)
                        })
                        && !insns[k + 1..].iter().any(|nx| {
                            matches!(
                                nx.op.name,
                                "RefCpyV" | "STOREOBJ" | "ClrVPtr" | "FreeNullV8"
                            ) && w(nx, 0) == dst_slot0
                        });
                    let param_into_later_use = ctx.param_src_ok(&top.s) && {
                        let mut consumed = false;
                        for j in (k + 1)..insns.len() {
                            let nx = &insns[j];
                            // first consumer is a binary pointer compare on this slot -> materialise.
                            if nx.op.name == "CmpPtr"
                                && (w(nx, 0) == dst_slot0 || w(nx, 1) == dst_slot0)
                            {
                                consumed = true;
                                break;
                            }
                            // first consumer is a member-access base: `PshVPtr <dst>; ADDSi …`
                            // (the param-alias `local_N.field = …` store) -> materialise. The very
                            // next op after the PshVPtr must be an ADDSi (member offset) so a bare
                            // PshVPtr that merely re-pushes the handle as a call arg does NOT trip
                            // this (that use reads the SAME undefined slot but is covered by the
                            // param name flowing through the push, not a slot alias).
                            if nx.op.name == "PshVPtr"
                                && w(nx, 0) == dst_slot0
                                && insns.get(j + 1).is_some_and(|a| a.op.name == "ADDSi")
                            {
                                consumed = true;
                                break;
                            }
                            // the slot is RE-DEFINED (output word 0) by another value producer
                            // before any consumer -> alias-shadow, this copy is not the one read; bail.
                            if matches!(
                                nx.op.name,
                                "RefCpyV"
                                    | "CpyRtoV4"
                                    | "CpyRtoV8"
                                    | "RDR4"
                                    | "RDR8"
                                    | "SetV4"
                                    | "SetV8"
                                    | "SetV1"
                                    | "STOREOBJ"
                                    | "FreeNullV8"
                                    | "ClrVPtr"
                            ) && w(nx, 0) == dst_slot0
                            {
                                break;
                            }
                        }
                        consumed
                    };
                    // batch-50a (opIndex-swap read-into-temp, specs/final-tail-triage.md §3.11
                    // ShuffleItemArray): a Fisher-Yates swap `temp = arr[i]; arr[i] = arr[j];
                    // arr[j] = temp;` lowers the SAVE-TO-TEMP as
                    // `PshV4 i; PshVPtr arr; Thiscall1 opIndex; PshRPtr; RDSPtr; RefCpyV wTemp`
                    // — the opIndex getter result copied into a plain local. Today the call-expr
                    // source drops (the `ok` whitelist accepts a `(`-bearing source ONLY into the
                    // return out-slot, batch-43 `call_src_to_out`), leaving `local_Temp` UNDECLARED;
                    // the later `arr[j] = local_Temp;` REFCPY store then reads an uninitialised slot
                    // and the whole swap corrupts. FIX-7's `getter_into_cmp`/batch-49c's
                    // `getter_into_null_cmp` only fire when the dest feeds a CmpPtr/CmpPtrNull; this
                    // adds the case where the dest is later consumed as the SOURCE HANDLE of a
                    // REFCPY store (`PshVPtr <dst>` whose enclosing statement stores it back). Gate
                    // HARD: source is a RESOLVED opIndex element read (`.opIndex(` … ends ')', no
                    // sentinel/unresolved), and a bounded forward scan finds the FIRST consumer of
                    // this slot to be a `PshVPtr <dst>` that is NOT a member-access base (next op is
                    // NOT ADDSi — that would be the param-alias shape) and NOT re-defined before —
                    // i.e. the slot is pushed as a bare handle (a REFCPY store source or call arg),
                    // never compared or member-dotted. Safe: the opIndex read is a pure element read
                    // (batch-32c `is_pure_elem_read` purity class) whose SOLE later use is the
                    // store-back; materialising `local_Temp = arr[i];` cannot reorder a side effect
                    // past a use it did not have before (the copy was DROPPED). Additive — recovers
                    // the vanilla store only when the swap-temp shape is proven.
                    let src_is_opindex_read = top.s.contains(".opIndex(")
                        && top.s.ends_with(')')
                        && !top.s.contains('\u{2}')
                        && !top.s.contains('\u{1}')
                        && top.s != UNRESOLVED
                        && !top.s.starts_with('~');
                    let getter_into_store_rhs = src_is_opindex_read && {
                        let mut consumed = false;
                        for j in (k + 1)..insns.len() {
                            let nx = &insns[j];
                            // first consumer is a bare-handle push of this slot (NOT a member base:
                            // next op must not be ADDSi) -> the swap-back store source. Materialise.
                            if nx.op.name == "PshVPtr"
                                && w(nx, 0) == dst_slot0
                                && !insns.get(j + 1).is_some_and(|a| a.op.name == "ADDSi")
                            {
                                consumed = true;
                                break;
                            }
                            // re-defined before any consumer -> alias-shadow, not this copy; bail.
                            if matches!(
                                nx.op.name,
                                "RefCpyV"
                                    | "CpyRtoV4"
                                    | "CpyRtoV8"
                                    | "RDR4"
                                    | "RDR8"
                                    | "SetV4"
                                    | "SetV8"
                                    | "SetV1"
                                    | "STOREOBJ"
                                    | "FreeNullV8"
                                    | "ClrVPtr"
                            ) && w(nx, 0) == dst_slot0
                            {
                                break;
                            }
                        }
                        consumed
                    };
                    let ok = !top.s.is_empty()
                        && top.s != dst
                        && !const_ret_getter
                        && (top.s.starts_with("local_")
                            || top.s.starts_with("Cast<")
                            || member_src
                            || call_src_to_out
                            || getter_into_cmp
                            || getter_into_null_cmp
                            || param_into_cmp
                            || param_into_later_use
                            || const_src_into_cmp
                            || getter_into_store_rhs);
                    if ok {
                        flush!();
                        // batch-25b (G4 assign shape): a slot-to-slot handle copy whose DEST
                        // type provably derives from the SOURCE type (`AGothicCharacter
                        // local_8 = UObject local_24;` — an iterator/temp producer erased the
                        // covariant type) fails "Can't implicitly convert". Wrap the source in
                        // the standard downcast. Same provably_derived gate as the RET shape;
                        // unknown/unprovable pairs render bare (status quo).
                        let dst_slot = w(ins, 0);
                        let rhs = match (dst_slot > 0).then(|| ctx.slot_type(dst_slot)).flatten() {
                            Some(dt)
                                if top
                                    .ty
                                    .as_deref()
                                    .map(|st| provably_derived(&dt, st, ctx.refs))
                                    .unwrap_or(false) =>
                            {
                                format!("Cast<{}>({})", qualify_class_name(&dt, ctx.refs), top.s)
                            }
                            _ => top.s.clone(),
                        };
                        // batch-32d: the copied source is a CONST object-handle native field
                        // of the destination's EXACT declared type (`const UItemDefinition`
                        // member read into `UItemDefinition local_20;` — a same-type Cast
                        // does NOT strip const in-game, batch-21 Class C). Emit the
                        // CONSTSTORE marker so the emitter declares the destination
                        // `const T` — the downcast() exact-type rule, mirrored for the
                        // RefCpyV member-read shape (CharacterAI_Gothic:3002).
                        // a const source needs a const destination, or the declaration will
                        // not hold what was copied into it
                        // Always const: the copy exists only to be compared, and a const
                        // destination holds a non-const source just as well as the other way
                        // round would not.
                        let const_source = const_src_into_cmp;
                        if const_source
                            || (top.nf_const.is_some()
                                && top.nf_const.as_deref()
                                    == (dst_slot > 0)
                                        .then(|| ctx.slot_type(dst_slot))
                                        .flatten()
                                        .as_deref())
                        {
                            out.push(format!("{dst} = {CONSTSTORE}{rhs};"));
                            // batch-41d: the dest slot now holds a const object handle; a later
                            // REFCPY member store of it must bail (const->non-const field).
                            if dst_slot > 0 {
                                const_obj_slots.insert(dst_slot);
                            }
                        } else {
                            out.push(format!("{dst} = {rhs};"));
                            // non-const write clears any stale const provenance for the slot.
                            if dst_slot > 0 {
                                const_obj_slots.remove(&dst_slot);
                            }
                        }
                    }
                }
            }
            // REFCPY (NO_ARG): object-handle STORE — `dst.member = src` (the object analogue of
            // WRTV*; there is no WRTV for handles). Stack top = the member lvalue built by
            // `PshVPtr obj; ADDSi member` (optionally RDSPtr); second = the source handle. Recover
            // it as a statement; BAIL (drop, exactly as before — pop both, emit nothing) on any
            // un-provable operand. A WRONG store silently corrupts an object graph (worse than a
            // dropped store), so the source-provenance gate is strict (batch-41b, Fix 1, S1/S3).
            "REFCPY" => {
                let dst = stack.pop(); // member lvalue (top)
                let src = stack.pop(); // source handle (second)
                if let (Some(dst), Some(src)) = (&dst, &src) {
                    // GATE:
                    //  dst is a MEMBER lvalue: contains '.', not empty/UNRESOLVED/sentinel, and NOT
                    //  a BARE slot (a bare local_N dst would be a RefCpyV, not a REFCPY member
                    //  store). batch-45a (FIX-1) permits a dotted `local_N.field` STRUCT-TEMP field
                    //  dest (e.g. `local_2.EventTag = SourceActor;` — a config/FGameplayEventData
                    //  field init before a SendGameplayEvent/validator call); only a bare `local_N`
                    //  with no `.` (which the '.'-contains test already excludes) is rejected.
                    //  src is a PROVEN handle: `local_N` (a STOREOBJ/allocated slot), a `Cast<…>`,
                    //  `nullptr` (the S3 clear via PshNull), or — batch-45a (FIX-1) — a NON-CONST
                    //  object/ref PARAMETER (`this.m_Target = NewTarget;`, `SetParentNode(Value)`).
                    //  NEVER a member/temp/`this`/const-param whose const-ness or aliasing we can't
                    //  prove (mirrors the RefCpyV arm's original caution — copying a const
                    //  param/member handle into a non-const dest fails "Can't implicitly convert";
                    //  `this` back-links stay bailed, `param_src_ok` never matches `this`).
                    let dst_ok = dst.s.contains('.')
                        && !dst.s.is_empty()
                        && dst.s != UNRESOLVED
                        && !dst.s.contains('\u{2}');
                    // batch-47c (FIX-1b, specs/final-tail-triage.md §3.7): accept a MEMBER-ACCESS
                    // source (`this.m_XardasChar`, built by `PshVPtr this; ADDSi member; RDSPtr`)
                    // into a member/struct-temp dest — `local_44.Instigator = this.m_XardasChar;`
                    // (DoTeleport) / `this.<dst> = this.<src>;`. This mirrors the widen batch-41a
                    // applied to the RefCpyV *read* arm (`member_src`), now for the REFCPY *store*
                    // arm. A member handle field is a legal RHS of a handle assignment. Gates:
                    // contains '.' (a real member access), no '(' (never a call/ctor expr — those
                    // have their own path and could reorder side effects), and — CRITICALLY —
                    // NON-const: `nf_const` set means the source is a `const T` native field, and
                    // storing it into a non-const member is the b41d "Can't implicitly convert from
                    // 'const T' to 'T'" cascade, so a const member source stays dropped exactly as
                    // before. A bare `this` has no '.' and is already excluded (its back-link store
                    // stays bailed). BAIL-safe: any un-provable source keeps the current drop.
                    let member_src =
                        src.s.contains('.') && !src.s.contains('(') && src.nf_const.is_none();
                    // batch-50a (opIndex-swap middle store, specs/final-tail-triage.md §3.11
                    // ShuffleItemArray): the Fisher-Yates `arr[i] = arr[j];` store has a getter
                    // call-expr SOURCE `arr[j]` (an opIndex element READ) and a member-lvalue DEST
                    // `arr[i]` (an opIndex element WRITE). Today the source drops (`member_src`
                    // rejects `(`-bearing exprs; it is neither `local_`/`Cast<`/nullptr/param), so
                    // the middle swap store vanishes and only `arr[j] = temp;` survives — a broken
                    // half-swap. Accept a RESOLVED opIndex element read as the source (a pure
                    // element read, `is_pure_elem_read` purity class): `arr[i] = arr[j];` is a legal
                    // handle assignment and the read has no side effect that dropping/keeping it
                    // reorders. Gate HARD: `.opIndex(` present, ends ')', no sentinel/unresolved —
                    // so the RHS is a proven element read, never a mutating call.
                    let src_is_opindex_elem = src.s.contains(".opIndex(")
                        && src.s.ends_with(')')
                        && !src.s.contains('\u{2}')
                        && !src.s.contains('\u{1}')
                        && src.s != UNRESOLVED
                        && !src.s.starts_with('~');
                    let src_ok = !src.s.is_empty()
                        && src.s != UNRESOLVED
                        && !src.s.contains('\u{2}')
                        && (src.s.starts_with("local_")
                            || src.s.starts_with("Cast<")
                            || src.s == "nullptr"
                            || member_src
                            || src_is_opindex_elem
                            || ctx.param_src_ok(&src.s));
                    // batch-41d (CLASS 1b): the source slot holds a CONST object handle (a
                    // const-returning call result / const member read). Storing it into a
                    // (non-const) member is a "Can't implicitly convert from 'const T' to 'T'"
                    // error in generate-mode. BAIL to the drop — the slot keeps its const decl
                    // and the store is dropped exactly as pre-batch-41 (safe: it was dropped
                    // before batch-41b and those functions compiled).
                    let src_slot = src
                        .s
                        .strip_prefix("local_")
                        .and_then(|d| d.parse::<i32>().ok());
                    let src_is_const = src_slot.is_some_and(|n| const_obj_slots.contains(&n));
                    if dst_ok && src_ok && !src_is_const {
                        flush!();
                        // batch-41d (CLASS 2): the member field type PROVABLY derives from the
                        // source's type (`this.ActiveActionTask (UAITask_CombatMove) =
                        // local_36 (UAbilityTaskGeneric&)` — a base-typed call result into a
                        // derived member). AS rejects the implicit downcast; wrap Cast<field>(src).
                        // Same provably_derived gate as the RefCpyV read path; never wrap when the
                        // pair is unprovable (renders bare = status quo).
                        let rhs = match (dst.ty.as_deref(), src.ty.as_deref()) {
                            (Some(fty), Some(sty)) if provably_derived(fty, sty, ctx.refs) => {
                                let head = fty
                                    .split('<')
                                    .next()
                                    .unwrap_or(fty)
                                    .trim_start_matches("const ");
                                format!("Cast<{}>({})", qualify_class_name(head, ctx.refs), src.s)
                            }
                            _ => src.s.clone(),
                        };
                        out.push(format!("{} = {};", dst.s, rhs));
                    }
                    // else: both popped, nothing emitted = the status-quo drop.
                }
            }
            // The TYPEID push is the implicit type operand of the following opCast/cast syscall
            // (NOT counted in the cache param list) — it is not a real stack arg, so don't push
            // it. Capture its resolved typename as the target T of the upcoming `opCast` so the
            // cast renders `Cast<T>(src)` instead of a discarded `src.opCast(out)`.
            "TYPEID" => {
                let tid = ins.dwords.first().copied().unwrap_or(0) as i32;
                last_typeid = resolve_cast_typeid(ctx.refs, tid);
            }
            // primitive numeric conversions (iTOf/fTOi/dTOf/sbTOi/...): `dst = src`. A
            // float/double -> integer narrowing must be made EXPLICIT (`dst = int(src)`) or the
            // compiler rejects it as an implicit precision loss; widenings stay implicit.
            n2 if is_numeric_cast(n2) => {
                flush!();
                let (dst, src) = (name(w(ins, 0)), name(w(ins, 1)));
                // a numeric conversion FROM an enum source needs an explicit int(enum) — AS has
                // no implicit enum->int (e.g. sbTOi of an EQuestState param into an int slot).
                let src = if ctx
                    .slot_type(w(ins, 1))
                    .as_deref()
                    .map(is_enum_name)
                    .unwrap_or(false)
                {
                    format!("int({src})")
                } else {
                    src
                };
                match narrowing_cast_target(n2) {
                    Some(t) => out.push(format!("{dst} = {t}({src});")),
                    None => out.push(format!("{dst} = {src};")),
                }
            }
            "SetV2" => {
                flush!();
                let bits = ins.dwords.first().copied().unwrap_or(0);
                set_consts.insert(w(ins, 0), ConstBits::W4(bits));
                out.push(format!("{} = {};", name(w(ins, 0)), bits as i32));
            }
            "CmpPtr" => {
                cmp = Some(Cmp {
                    a: name(w(ins, 0)),
                    b: name(w(ins, 1)),
                    ..Default::default()
                })
            }
            "OBJTYPE" => stack.push(Arg::obj("objtype".into())), // +2: RTTI objtype ptr
            "STR" => stack.push(Arg::obj("\"\"".into())),        // +3: string-constant push
            "PshListElmnt" => stack.push(Arg::int(name(w(ins, 0)))), // +2: list element
            // asBC_COPY (W_DW_ARG = byte-size + object-type ptr) and asBC_CopyScript (QW_ARG =
            // object-type ptr) are the SAME operation — a script value-type / struct copy =
            // the source-level assignment `dest = src;`. Per asBC_COPY (as_context.cpp) the
            // DESTINATION pointer is popped FIRST (it is the stack TOP), the SOURCE is the next
            // entry below it. The compiler pushes SRC first then DEST, so DEST ends up on top —
            // e.g. `PSF <localSrc>; PshVPtr this; ADDSi <member>; CopyScript` is `this.member =
            // localSrc;`, and the RVO struct-return `PSF <local>; PshVPtr <retSlot>; CopyScript`
            // is `<retSlot> = <local>;`. (Earlier this arm had src/dst swapped, which emitted
            // every struct copy/member-init/RVO-return BACKWARDS — `local = this.member` and
            // `local = retSlot`.) Both operands arrive as fully-rendered member/local exprs;
            // dropping it left the destination (member or RVO temp) unwritten -> garbage/null.
            "CopyScript" | "COPY" => {
                let dst = stack.pop();
                let src = stack.pop();
                if let (Some(dst), Some(src)) = (dst, src) {
                    if !src.s.is_empty() && src.s != UNRESOLVED {
                        // RVO struct-return: a copy whose DEST is the hidden return slot is the
                        // function's `return <src>;` — capture it as the return value (the slot
                        // itself has no source name) instead of emitting `__return = <src>;`.
                        if dst.s == "__return" {
                            // batch-47b (FIX-2b, specs/final-tail-triage.md §3.2): the RVO copy-out
                            // (`CopyScript` into `__return`) can sit in a mid-function BRANCH/LOOP
                            // block that ends in JMP to the shared tail RET (e.g. an early
                            // `if (…) return FRelativeCrimeMemoryFilter(local_16, Witness);`). There
                            // `ret_val` is block-local and NEVER consumed — the tail RET emits the
                            // RVODEF default, so `return __r;` (a default-constructed struct) ships
                            // on that path, LOSING the built value. This generalises the batch-43
                            // switch-region carry (which emitted `__return = src;` only inside a
                            // `rvo_switch_region`) to ANY non-RET-terminated block: emit the store as
                            // a STATEMENT so emit.rs declares `{ret} __return;` and folds the tail
                            // `return RVODEF;` to `return __return;` — the value flows on this path.
                            // When the block DOES end in RET, keep the byte-identical `ret_val` path
                            // (its RET arm folds to `return <src>;` with no duplicate store).
                            if ctx.instrs[hi - 1].op.name != "RET" {
                                flush!();
                                out.push(format!("__return = {};", src.s));
                            } else {
                                ret_val = Some(src.s);
                            }
                        } else if !dst.s.is_empty() && dst.s != UNRESOLVED && dst.s != src.s {
                            flush!();
                            out.push(format!("{} = {};", dst.s, src.s));
                        }
                    }
                }
            }
            // asBC_Cast (DW_ARG=target typeid): a script-handle DOWNCAST. Pop the source handle
            // and push the cast RESULT `Cast<T>(src)` (typed) so the following store writes the
            // real object instead of dropping it. T is the instruction's own typeid operand.
            // (This opcode is unused by the current cache — the fork lowers casts to the `opCast`
            // behaviour above — but is handled for completeness/robustness.) Fall back to a bare
            // passthrough when T is unresolved or not an object, so the value always FLOWS.
            "Cast" => {
                if let Some(src) = stack.pop() {
                    let tid = ins.dwords.first().copied().unwrap_or(0) as i32;
                    match resolve_cast_typeid(ctx.refs, tid) {
                        Some(ty) if ty.starts_with('U') || ty.starts_with('A') => {
                            stack.push(Arg::typed(
                                format!("Cast<{}>({})", qualify_class_name(&ty, ctx.refs), src.s),
                                Some(ty),
                            ));
                        }
                        _ => stack.push(src),
                    }
                }
            }
            // conditional jump: if no comparison was recovered, the tested value is the live
            // call result / bool register — use it as the branch condition (consume so it's
            // not flushed as a stray statement).
            "JZ" | "JNZ" | "JS" | "JNS" | "JP" | "JNP" | "JLowZ" | "JLowNZ" => {
                if cond.is_none() && cmp.is_none() {
                    if let Some(p) = pending.take() {
                        cond = Some((p, pending_ty.as_deref() == Some("bool")));
                    }
                } else if pending.is_some()
                    && pending_ty.as_deref() == Some("bool")
                    && !test_after_call
                {
                    // batch-30c (C9, the Conversation IsVisible family): a BOOL-returning
                    // call issued AFTER the recovered comparison clobbered the value
                    // register — the jump tests the call's result, not the stale compare
                    // (`....IsValid();` discarded + `if (local_9 != 1)` testing a compare
                    // from several statements earlier). Prefer the live bool pending —
                    // but ONLY when no compare/register-load executed after the call
                    // (`test_after_call`): a CmpPtrNull/CpyVtoR between the call and the
                    // jump re-fills the register, and that test stays authoritative.
                    let p = pending.take().unwrap();
                    cond = Some((p, true));
                    cmp = None;
                }
            }
            // FreeNullV8 vN: the compiler's `return null` prologue — free slot N and set it to a
            // null object handle. Model as `local_N = nullptr;` so the following `LOADOBJ vN; RET`
            // renders `return nullptr;` (previously it was ignored -> the null-return branch
            // rendered empty). Additive and always correct — nulling its own slot IS the opcode's
            // VM semantics; a rare non-return slot-reuse also legitimately nulls the slot
            // (a redundant null-init of an already-null-defaulted object local, value-preserving).
            // Depends on Fix 2 (the member-load must be recovered first so the null-check tests the
            // right operand). batch-41c, Fix 3, S4.
            "FreeNullV8" => {
                flush!();
                let slot = w(ins, 0);
                out.push(format!("{} = nullptr;", name(slot)));
                // clear any stale const / member-read provenance for this slot
                set_consts.remove(&slot);
                member_read_slots.remove(&slot);
            }
            // ---- pure VM housekeeping / flow: ignore ----
            // NOTE: `ThrowException` (throw) and `JMPP` (jump-table/switch) are NOT housekeeping
            // — they're unmodeled control transfers, and `cfg.rs` leaves JMPP successors unknown.
            // They fall through to the `// opcode` marker below so `stub_reason` stubs the body
            // rather than emitting recompilable source with the throw/switch silently dropped.
            "SUSPEND" | "JitEntry" | "PopPtr" | "SwapPtr" | "ClrHi" | "ClrVPtr" | "FREE"
            | "FinConstruct" | "CHKREF" | "ChkRefS" | "ChkNullV" | "ChkNullS"
            | "DestructScript" | "SaveReturnValue" | "ResolveObjectPtr" | "GETOBJ"
            | "GETOBJREF" | "GETREF" | "JMP" => {}
            // (CopyScript/COPY are handled above as `dest = src;`. FinConstruct/DestructScript
            // stay ignored: implicit AS construct/destruct that appear in nearly every function —
            // emitting/stubbing them would be wrong.) Any genuinely unmodeled opcode falls through
            // to the marker + stub rather than emitting recompilable source with it silently dropped.
            _ => {
                flush!();
                out.push(format!("// {} {}", n, operand_str(ins)));
            }
        }
    }
    // batch-35a (cross-block reference carry): this block flows to a ref-returning function's
    // shared bare RET row and its trailing pending is a live by-reference lvalue chain
    // (`this.RoleCategoryContainers.opIndex(...).RoleGroups.opIndex(...)` -> FCombatRoleGroup&).
    // Render it as the return value the RET row cannot recover by scan-back (the reference is
    // not a slot). Gated to a genuine ref pending that carries no sentinel/unresolved marker
    // (those must never become a return) — everything else flushes as a bare statement below.
    if ret_ref_tail
        && pending_is_ref
        && pending
            .as_deref()
            .is_some_and(|p| !p.contains('\u{2}') && !p.contains(UNRESOLVED))
    {
        let chain = pending.take().unwrap();
        out.push(format!("return {chain};"));
    }
    // Final block flush: there is no later instruction that can observe the pending
    // result metadata, so avoid the ordinary `flush!` reset assignments here. Keeping
    // this separate also lets `clippy -D warnings` prove that every metadata write in
    // the instruction loop has a potential consumer.
    if let Some(s) = pending.take() {
        // Unlike `flush_b2!`, a final pending ARGMISMATCH sentinel must surface so the emitter
        // takes its fail-closed stub path. Filtering it here silently dropped the bad statement.
        out.push(format!("{s};"));
    }
    out.retain(|s| !s.contains(UNRESOLVED)); // drop statements with an unresolved value
                                             // A temp that only ever held a DEFAULT-CONSTRUCTED value: `PSF t; CALLSYS $beh0(0-param)`
                                             // builds it, and the construct behaviour has no source form of its own, so nothing declares
                                             // `t` and every read of it dangles (`this.m_CameraBehaviour = local_40;` with no local_40).
                                             // The Idiom-S arm above already recovers the `$beh0` copy-assign shape and consumes its
                                             // record; what is left here is the RUNTIME `opAssign` shape and plain argument reads. Write
                                             // the value where it is read — but only for a temp nothing else ever assigns, so the
                                             // substituted `T()` is provably the only value it can hold.
                                             // A `b<Upper>` field written from an INT LITERAL: UHT reserves that prefix for bool
                                             // UPROPERTYs, whose generated accessor is `bool&`, so `bRunning = 1;` is "Can't implicitly
                                             // convert from 'int' to 'bool&'". The store reaches here through several different arms
                                             // (member write, property setter, chained lvalue), so the form is corrected once, on the
                                             // finished statement. Only a literal — a bare slot may itself be declared bool, where
                                             // `local != 0` would be the illegal form.
    for statement in out.iter_mut() {
        let Some((target, rhs)) = statement
            .strip_suffix(';')
            .and_then(|line| line.rsplit_once(" = "))
        else {
            continue;
        };
        if !is_ue_bool_field(target) {
            continue;
        }
        // An int LITERAL always needs the form. A bare slot needs it unless the slot is itself
        // declared `bool`, where `local != 0` would be the illegal one.
        let int_slot = rhs
            .strip_prefix("local_")
            .and_then(|rest| rest.parse::<i32>().ok())
            .is_some_and(|slot| ctx.slot_type(slot).as_deref() != Some("bool"));
        if is_int_literal(rhs) || int_slot {
            *statement = format!("{target} = ({rhs} != 0);");
        }
    }
    if std::env::var_os("GORE_AS_DEFAULTS_DEBUG").is_some() && !default_ctor_temp.is_empty() {
        eprintln!("[ctor-temp] {default_ctor_temp:?}");
    }
    for (slot, ty) in &default_ctor_temp {
        // Any write to the slot — the whole value, or a member or element of it — is a write the
        // substituted `T()` would throw away. Reading only whole-value assignments let a
        // configured query be passed as a freshly default-constructed one (measured: the
        // argument arrived empty where vanilla had filled it in).
        let assigned = out.iter().any(|s| {
            s.trim_start()
                .strip_prefix(slot.as_str())
                .is_some_and(|rest| {
                    rest.starts_with(" = ")
                        || rest.split_once(" = ").is_some_and(|(path, _)| {
                            path.starts_with(['.', '['])
                                && !path.contains('(')
                                && !path.contains(' ')
                        })
                })
        });
        // A bare template head (`TArray`) is not a constructible type name, and a temporary
        // cannot be passed by non-const reference or receive a non-const method call — so the
        // substitution is limited to the one place it is provably a plain value: the right-hand
        // side of a whole-value assignment.
        if assigned || (ty.starts_with('T') && !ty.contains('<')) {
            continue;
        }
        let value = format!("{ty}()");
        // A slot the body mentions exactly once was never read back, which is what an `&out`
        // parameter's caller does.
        let sole_mention = out.iter().filter(|s| count_word(s, slot) > 0).count() == 1;
        for statement in out.iter_mut() {
            if let Some(lhs) = statement
                .strip_suffix(&format!(" = {slot};"))
                .filter(|lhs| count_word(lhs, slot) == 0)
            {
                *statement = format!("{lhs} = {value};");
                continue;
            }
            // A container's own mutator takes its element by CONST reference, so a temporary is
            // legal there too — `PursuedCrimes.Add(FCrimeEntry());` is what the source wrote.
            // Proven per site from the receiver's own field type, never from the method name.
            if let Some(rewritten) = temporary_argument_call(
                statement,
                slot,
                ty,
                &value,
                ctx.refs,
                sole_mention,
                &resolved_params,
            ) {
                *statement = rewritten;
            }
        }
    }
    // no binary comparison but a bool value was tested -> use it as the branch condition so
    // the jump renders `if (cond != 0)` instead of `if (? != ?)`.
    if cmp.is_none() {
        if let Some((c, is_bool)) = cond.take() {
            if !c.is_empty() && c != UNRESOLVED {
                cmp = Some(Cmp {
                    expr: Some(c),
                    expr_bool: is_bool,
                    ..Default::default()
                });
            }
        }
    }
    (out, cmp, stack)
}

/// Condition rendered for the branch being TAKEN, given the CMP operands + jump op.
fn branch_cond(cmp: &Option<Cmp>, jump: &str) -> String {
    // single boolean condition (a bool slot / call result tested by JLowZ/JLowNZ): the bool is
    // held in an int slot, so test against 0 (int-safe; negate() swaps `!= 0` <-> `== 0`).
    if let Some(c) = cmp {
        if let Some(e) = &c.expr {
            if c.expr_bool {
                // already bool-typed (a call result) -> render bare / negated.
                return match jump {
                    "JNZ" | "JLowNZ" | "JNS" | "JP" => e.clone(),
                    _ => format!("!({e})"),
                };
            }
            return match jump {
                "JNZ" | "JLowNZ" | "JNS" | "JP" => format!("{e} != 0"),
                _ => format!("{e} == 0"),
            };
        }
    }
    let (a, b) = match cmp {
        Some(c) => (c.a.clone(), c.b.clone()),
        None => ("?".into(), "?".into()),
    };
    // A T* op (TZ/TNZ/TS/...) already turned the CMP register into a bool carrying the
    // relation (`c.op`); the following jump then selects the TAKEN sense exactly as for the
    // bool-expr case above (JZ -> taken when the relation is false). Apply that sense instead
    // of emitting the raw relation, which the structurer would then negate into an inverted
    // if/while.
    if let Some(op) = cmp.as_ref().and_then(|c| c.op) {
        let cond = format!("{a} {op} {b}");
        return match jump {
            "JNZ" | "JLowNZ" | "JNS" | "JP" => cond,
            _ => negate(&cond),
        };
    }
    // No T*: the conditional jump itself encodes the relation.
    let op = match jump {
        "JS" => "<",
        "JNS" => ">=",
        "JP" => ">",
        "JNP" => "<=",
        "JZ" | "JLowZ" => "==",
        "JNZ" | "JLowNZ" => "!=",
        _ => "?",
    };
    format!("{a} {op} {b}")
}

fn negate(cond: &str) -> String {
    // `!` is a VALUE operator in this language: it cannot fold into the jump, so every one that
    // survives costs the compiler a spill — `CpyRtoV4; NOT; CpyVtoR1` in front of the branch that
    // would otherwise have tested the result where it stood. Turning the comparison around is what
    // vanilla wrote, and a `!(X)` that is already there comes off instead of doubling.
    if let Some(inner) = cond.strip_prefix("!(").and_then(|c| c.strip_suffix(')')) {
        if wraps_the_whole_condition(inner) {
            return inner.to_owned();
        }
    }
    // Not inside a short circuit: turning one relation of `a == b && c` negates only that half,
    // and De Morgan would change which operand is evaluated first.
    if !cond.contains("&&") && !cond.contains("||") {
        for (op, neg) in [
            (" <= ", " > "),
            (" >= ", " < "),
            (" < ", " >= "),
            (" > ", " <= "),
            (" == ", " != "),
            (" != ", " == "),
        ] {
            if let Some(p) = cond.find(op) {
                return format!("{}{}{}", &cond[..p], neg, &cond[p + op.len()..]);
            }
        }
    }
    format!("!({cond})")
}

/// True when stripping one leading `!(` and its trailing `)` left a balanced expression — so the
/// pair really wrapped the whole condition and did not close something inside it.
fn wraps_the_whole_condition(inner: &str) -> bool {
    let mut depth = 0i32;
    for b in inner.bytes() {
        match b {
            b'(' => depth += 1,
            b')' => depth -= 1,
            _ => {}
        }
        if depth < 0 {
            return false;
        }
    }
    depth == 0
}

struct Structurer<'a> {
    ctx: &'a Ctx<'a>,
    g: &'a Cfg,
    idx_of: &'a HashMap<usize, usize>,
    /// Switch-region exit context (set only while emitting a recovered `switch` case region):
    /// the JOIN offset a `JMP` renders as `break;` for, whether JOIN is the function's bare
    /// `RET` row (then a `JMP` there renders as a synthesized `return ...;` instead), and
    /// whether `JMP`s to OTHER bare-RET rows may render as returns (register-based-return
    /// functions only — an RVO-struct return can't be synthesized from the value register).
    exit_join: Option<usize>,
    exit_join_is_ret: bool,
    exit_ret_rows_ok: bool,
    /// batch-43 (Fix 2, switch-rvo-return.md §4.2): RVO-struct-return switch mode. When set, a
    /// case region's exit `JMP` to the shared bare-RET join renders `return <val>;` folded from
    /// the region's trailing `__return = <val>;` out-slot store (a struct-by-value return travels
    /// through the hidden RVO slot, never the value register, so the register scan-back is empty
    /// and would otherwise emit `return <default-struct>;` — silently wrong). Fires ONLY under the
    /// heavily-gated G-RVO proof (`ret_via_rvo` + bare-RET join + EVERY region proves an `__return`
    /// write); any gap bails the whole switch to the stub.
    exit_rvo_return: bool,
    /// Mixed-RVO switch mode: the switch has a normal forward JOIN for `break` paths, while one
    /// or more case blocks return a struct through the hidden RVO slot and jump to a separate
    /// bare RET row. Set only after EVERY such edge proves one clean block-local `__return`
    /// store; [`Self::region_exit_stmt`] may then render those edges as `return __return;`.
    exit_mixed_rvo_ret_rows_ok: bool,
    /// Instruction-index floor for the synthesized-return value scan (the current case
    /// region's first instruction — a value must not leak in from a preceding region).
    exit_scan_floor: usize,
    /// batch-27: (join block index, operand stack surviving a recognized Cast diamond).
    /// Created at the end of the is_cond arm ONLY when [`Self::diamond_join`] proves the
    /// construct is exactly the null-check Cast diamond; consumed exactly once (`take()`)
    /// by the very next loop iteration of `emit_range`.
    carry: Option<(usize, Vec<Arg>)>,
    /// batch-36: active loop-exit context, set only while emitting a loop body via `emit_range`.
    /// `continue_off` = the loop-continue dword offset (where a `continue;` lands: the latch
    /// block start for a bottom-test loop). `break_off` = the loop-exit dword offset (where
    /// `break;` lands: the latch's non-back-edge successor). Saved/restored around the recursive
    /// body emission; nested loops push/pop so break/continue always bind to the innermost loop.
    loop_scope: Option<LoopScope>,
}

#[derive(Clone, Copy)]
struct LoopScope {
    continue_off: usize,
    break_off: usize,
}

const COMPOUND_HEADER_MAX_BLOCKS: usize = 8;
const COMPOUND_HEADER_MAX_PATHS: usize = 16;
const COMPOUND_HEADER_MAX_EXPR_NODES: usize = 64;

/// A deliberately small expression language for compiler-materialized loop predicates. It is
/// used only while proving a bounded, side-effect-free header DAG; unsupported bytecode keeps the
/// function on the existing stub path.
#[derive(Clone, PartialEq, Eq)]
enum HeaderExpr {
    Atom(String),
    Int(i64),
    UInt(u64),
    Real(String),
    Bool(bool),
    Cast(&'static str, Box<HeaderExpr>),
    Cmp(Box<HeaderExpr>, HeaderRel, Box<HeaderExpr>),
    And(Vec<HeaderExpr>),
    Or(Vec<HeaderExpr>),
    Not(Box<HeaderExpr>),
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum HeaderRel {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

impl HeaderRel {
    fn negated(self) -> Self {
        match self {
            Self::Eq => Self::Ne,
            Self::Ne => Self::Eq,
            Self::Lt => Self::Ge,
            Self::Le => Self::Gt,
            Self::Gt => Self::Le,
            Self::Ge => Self::Lt,
        }
    }

    fn text(self) -> &'static str {
        match self {
            Self::Eq => "==",
            Self::Ne => "!=",
            Self::Lt => "<",
            Self::Le => "<=",
            Self::Gt => ">",
            Self::Ge => ">=",
        }
    }
}

impl HeaderExpr {
    fn nodes(&self) -> usize {
        match self {
            Self::Atom(_) | Self::Int(_) | Self::UInt(_) | Self::Real(_) | Self::Bool(_) => 1,
            Self::Cast(_, v) | Self::Not(v) => 1 + v.nodes(),
            Self::Cmp(a, _, b) => 1 + a.nodes() + b.nodes(),
            Self::And(v) | Self::Or(v) => 1 + v.iter().map(Self::nodes).sum::<usize>(),
        }
    }

    fn render(&self) -> String {
        match self {
            Self::Atom(s) => s.clone(),
            Self::Int(v) => v.to_string(),
            Self::UInt(v) => v.to_string(),
            Self::Real(v) => v.clone(),
            Self::Bool(v) => v.to_string(),
            Self::Cast(t, v) => format!("{t}({})", v.render()),
            Self::Cmp(a, op, b) => format!("{} {} {}", a.render(), op.text(), b.render()),
            Self::And(v) => v
                .iter()
                .map(|x| format!("({})", x.render()))
                .collect::<Vec<_>>()
                .join(" && "),
            Self::Or(v) => v
                .iter()
                .map(|x| format!("({})", x.render()))
                .collect::<Vec<_>>()
                .join(" || "),
            Self::Not(v) => format!("!({})", v.render()),
        }
    }
}

fn header_not(v: HeaderExpr) -> HeaderExpr {
    match v {
        HeaderExpr::Bool(v) => HeaderExpr::Bool(!v),
        HeaderExpr::Cmp(a, op, b) => HeaderExpr::Cmp(a, op.negated(), b),
        HeaderExpr::Not(v) => *v,
        v => HeaderExpr::Not(Box::new(v)),
    }
}

fn header_and(values: Vec<HeaderExpr>) -> HeaderExpr {
    let mut flat = Vec::new();
    for value in values {
        match value {
            HeaderExpr::Bool(false) => return HeaderExpr::Bool(false),
            HeaderExpr::Bool(true) => {}
            HeaderExpr::And(v) => flat.extend(v),
            v => flat.push(v),
        }
    }
    flat.dedup();

    // `x == A && x != B` makes the inequality redundant when A != B, and impossible when
    // A == B. This is enough to collapse the common compiler lowering of `x == A || x == B`
    // without introducing a general-purpose theorem prover.
    let equalities: Vec<(HeaderExpr, HeaderExpr)> = flat
        .iter()
        .filter_map(|v| match v {
            HeaderExpr::Cmp(a, HeaderRel::Eq, b) => Some(((**a).clone(), (**b).clone())),
            _ => None,
        })
        .collect();
    for (a, b) in &equalities {
        if flat.iter().any(
            |v| matches!(v, HeaderExpr::Cmp(na, HeaderRel::Ne, nb) if **na == *a && **nb == *b),
        ) {
            return HeaderExpr::Bool(false);
        }
    }
    flat.retain(|v| {
        !matches!(v, HeaderExpr::Cmp(a, HeaderRel::Ne, b)
        if equalities.iter().any(|(ea, eb)| {
            **a == *ea
                && matches!((&**b, eb), (HeaderExpr::Int(n), HeaderExpr::Int(m)) if n != m)
        }))
    });
    match flat.len() {
        0 => HeaderExpr::Bool(true),
        1 => flat.pop().unwrap(),
        _ => HeaderExpr::And(flat),
    }
}

fn header_or(values: Vec<HeaderExpr>) -> HeaderExpr {
    let mut flat = Vec::new();
    for value in values {
        match value {
            HeaderExpr::Bool(true) => return HeaderExpr::Bool(true),
            HeaderExpr::Bool(false) => {}
            HeaderExpr::Or(v) => flat.extend(v),
            v => flat.push(v),
        }
    }
    flat.dedup();
    match flat.len() {
        0 => HeaderExpr::Bool(false),
        1 => flat.pop().unwrap(),
        _ => HeaderExpr::Or(flat),
    }
}

#[derive(Clone)]
struct HeaderValue {
    expr: HeaderExpr,
    boolish: bool,
    /// The complete VM value register is proven to contain canonical 0/1. A one-byte
    /// `CpyVtoR1` is boolish for JLow*, but does not prove the upper register bytes for J*.
    full_bool: bool,
    ty: Option<String>,
}

#[derive(Clone, Default)]
struct HeaderState {
    slots: HashMap<i32, HeaderValue>,
    cmp: Option<(HeaderExpr, HeaderExpr)>,
    value_reg: Option<HeaderValue>,
}

fn header_truth_expr(value: HeaderValue) -> Option<HeaderExpr> {
    if !value.boolish {
        return None;
    }
    Some(match value.expr {
        HeaderExpr::Int(0) | HeaderExpr::UInt(0) => HeaderExpr::Bool(false),
        HeaderExpr::Int(1) | HeaderExpr::UInt(1) => HeaderExpr::Bool(true),
        v @ (HeaderExpr::Bool(_)
        | HeaderExpr::Cmp(_, _, _)
        | HeaderExpr::And(_)
        | HeaderExpr::Or(_)
        | HeaderExpr::Not(_)) => v,
        v => v,
    })
}

fn header_bool_jump_taken(value: HeaderValue, jump: &str) -> Option<HeaderExpr> {
    let full_bool = value.full_bool;
    let value = header_truth_expr(value)?;
    Some(match jump {
        "JLowZ" => header_not(value),
        "JLowNZ" => value,
        _ if !full_bool => return None,
        "JZ" | "JNP" => header_not(value),
        "JNZ" | "JP" => value,
        "JS" => HeaderExpr::Bool(false),
        "JNS" => HeaderExpr::Bool(true),
        _ => return None,
    })
}

/// Return whether `ins` reads and/or writes `slot` through an explicit stack-var operand. The
/// `rW`/`wW` roles are part of the VM instruction format, so this remains conservative across the
/// full opcode table instead of maintaining a partial mnemonic list. Taking a slot's address is an
/// `rW` use and therefore counts as a read/escape. `ChkNullS` is the one bare-W slot operand.
fn explicit_slot_access(ins: &Instr, slot: i32) -> (bool, bool) {
    use BcType::*;
    let mut reads = false;
    let mut writes = false;
    let mut apply = |index: usize, read: bool, write: bool| {
        if ins.words.get(index).copied().map(s16) == Some(slot) {
            reads |= read;
            writes |= write;
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
    if ins.op.name == "NOT" && ins.words.first().copied().map(s16) == Some(slot) {
        reads = true;
        writes = true;
    }
    (reads, writes)
}

fn header_numeric_cast_target(op: &str, dst_ty: Option<&str>) -> Option<&'static str> {
    Some(match op {
        "iTOf" | "uTOf" | "i64TOf" | "u64TOf" => "float32",
        "iTOd" | "uTOd" | "fTOd" | "i64TOd" | "u64TOd" => "float",
        "fTOi" | "dTOi" | "sbTOi" | "swTOi" | "ubTOi" | "uwTOi" => "int",
        "fTOu" | "dTOu" => "uint",
        "dTOf" => "float32",
        "iTOb" => match dst_ty? {
            "int8" => "int8",
            "uint8" => "uint8",
            _ => return None,
        },
        "iTOw" => match dst_ty? {
            "int16" => "int16",
            "uint16" => "uint16",
            _ => return None,
        },
        "i64TOi" => match dst_ty? {
            "int" => "int",
            "uint" => "uint",
            _ => return None,
        },
        "iTOi64" | "uTOi64" => match dst_ty? {
            "int64" => "int64",
            "uint64" => "uint64",
            _ => return None,
        },
        "fTOi64" | "dTOi64" => "int64",
        "fTOu64" | "dTOu64" => "uint64",
        _ => return None,
    })
}

fn header_numeric_cast_source(op: &str) -> Option<&'static str> {
    Some(match op {
        "iTOf" | "iTOd" | "iTOb" | "iTOw" | "iTOi64" => "int",
        "uTOf" | "uTOd" | "uTOi64" => "uint",
        "fTOi" | "fTOu" | "fTOd" | "fTOi64" | "fTOu64" => "float32",
        "dTOi" | "dTOu" | "dTOf" | "dTOi64" | "dTOu64" => "float",
        "i64TOf" | "i64TOd" | "i64TOi" => "int64",
        "u64TOf" | "u64TOd" => "uint64",
        "sbTOi" => "int8",
        "swTOi" => "int16",
        "ubTOi" => "uint8",
        "uwTOi" => "uint16",
        _ => return None,
    })
}

fn header_numeric_cast_operand(op: &str, source: HeaderValue) -> Option<HeaderExpr> {
    let source_ty = header_numeric_cast_source(op)?;
    if source.ty.as_deref() == Some(source_ty)
        // For an enum slot, the compiler knows the actual underlying byte/word signedness.
        // Rendering `int(enum_value)` reproduces sb/sw/ub/uwTOi; pre-casting it to int8/uint8
        // adds a redundant conversion and discards the enum metadata that made the opcode exact.
        || (source.ty.as_deref().is_some_and(is_enum_name)
            && matches!(op, "sbTOi" | "swTOi" | "ubTOi" | "uwTOi"))
    {
        Some(source.expr)
    } else {
        Some(HeaderExpr::Cast(source_ty, Box::new(source.expr)))
    }
}

fn header_integral_width(ty: &str) -> Option<usize> {
    Some(match ty {
        "bool" | "int8" | "uint8" => 8,
        "int16" | "uint16" => 16,
        "int" | "uint" => 32,
        "int64" | "uint64" => 64,
        _ => return None,
    })
}

fn header_primitive_cast_target(ty: &str) -> Option<&'static str> {
    Some(match ty {
        "bool" => "bool",
        "int8" => "int8",
        "uint8" => "uint8",
        "int16" => "int16",
        "uint16" => "uint16",
        "int" => "int",
        "uint" => "uint",
        "int64" => "int64",
        "uint64" => "uint64",
        "float32" => "float32",
        "float" | "double" | "float64" => "float",
        _ => return None,
    })
}

fn header_copy_type_matches_width(value: &HeaderValue, ty: &str, copy_bits: usize) -> bool {
    match ty {
        // Test results live in the full four-byte value register even though their source-level
        // type is bool; this is the compiler's normal TZ -> CpyRtoV4 materialization.
        "bool" => value.boolish && copy_bits == 32,
        "float32" => copy_bits == 32,
        "float" | "double" | "float64" => copy_bits == 64,
        ty => header_integral_width(ty) == Some(copy_bits),
    }
}

/// Retype a raw same-width VM copy using the destination slot, never the source slot. Primitive
/// signed/unsigned copies are modulo-bit-preserving in AngelScript when written as an explicit
/// cast. Other cross-type bit reinterpretations (notably integer/float and an enum destination
/// with unknown underlying signedness) are rejected rather than guessed.
fn header_copy_value(
    mut value: HeaderValue,
    dst_ty: Option<String>,
    copy_bits: usize,
) -> Option<HeaderValue> {
    // Without the destination type a raw copy may be an integer/float bit reinterpretation. Any
    // later numeric operation would turn that into an AngelScript numeric conversion, so reject.
    // The sole safe exception is a proven canonical boolean materialization: its complete V4
    // payload is 0/1 independently of the unnamed destination's inferred integer spelling.
    let Some(dst_ty) = dst_ty else {
        if value.boolish && value.full_bool && copy_bits == 32 {
            value.ty = None;
            return Some(value);
        }
        return None;
    };
    if value.ty.as_deref() == Some(dst_ty.as_str()) {
        return header_copy_type_matches_width(&value, &dst_ty, copy_bits).then_some(value);
    }

    let dst_cast = header_primitive_cast_target(&dst_ty)?;
    let safe = match value.ty.as_deref() {
        Some("bool") => value.boolish && header_integral_width(&dst_ty) == Some(copy_bits),
        Some(src) if is_enum_name(src) => header_integral_width(&dst_ty) == Some(copy_bits),
        Some(src) => {
            let integral = header_integral_width(src) == Some(copy_bits)
                && header_integral_width(&dst_ty) == Some(copy_bits);
            let same_float_width = matches!(
                (src, dst_ty.as_str(), copy_bits),
                ("float32", "float32", 32)
                    | (
                        "float" | "double" | "float64",
                        "float" | "double" | "float64",
                        64
                    )
            );
            integral || same_float_width
        }
        None => {
            header_integral_width(&dst_ty) == Some(copy_bits)
                && matches!(
                    &value.expr,
                    HeaderExpr::Int(_)
                        | HeaderExpr::UInt(_)
                        | HeaderExpr::Bool(_)
                        | HeaderExpr::Cmp(_, _, _)
                        | HeaderExpr::And(_)
                        | HeaderExpr::Or(_)
                        | HeaderExpr::Not(_)
                )
        }
    };
    if !safe {
        return None;
    }
    value.expr = HeaderExpr::Cast(dst_cast, Box::new(value.expr));
    value.ty = Some(dst_ty);
    Some(value)
}

/// VM integer comparisons are bit/integer operations. Casting a known float, handle, or object
/// expression to an integer in source would be a numeric conversion and is not necessarily the
/// bytecode operation that was executed, so only proven integral-like source values are accepted.
fn header_is_integral_type(ty: Option<&str>) -> bool {
    match ty {
        None => true,
        Some(
            "bool" | "int8" | "uint8" | "int16" | "uint16" | "int" | "uint" | "int64" | "uint64",
        ) => true,
        Some(ty) => is_enum_name(ty),
    }
}

fn header_set_value(op: &str, raw: u64, ty: Option<String>) -> Option<HeaderValue> {
    let (expr, boolish) = match op {
        "SetV1" => {
            let raw = raw as u8;
            match ty.as_deref() {
                Some("bool") if raw <= 1 => (HeaderExpr::Int(raw as i64), true),
                Some("int8") => (HeaderExpr::Int(raw as i8 as i64), false),
                Some("uint8") => (HeaderExpr::UInt(raw as u64), false),
                // A SetV1 enum's underlying byte signedness is not present in the cache's local
                // type name. Guessing `i8` breaks 0xff followed by ubTOi, while guessing `u8`
                // breaks sbTOi. Keep this uncommon shape on the visible fallback path.
                Some(t) if is_enum_name(t) => return None,
                None if raw <= 1 => (HeaderExpr::Int(raw as i64), true),
                None => (HeaderExpr::Int(raw as i8 as i64), false),
                _ => return None,
            }
        }
        "SetV2" => {
            let raw = raw as u16;
            let expr = match ty.as_deref() {
                Some("int16") | None => HeaderExpr::Int(raw as i16 as i64),
                Some("uint16") => HeaderExpr::UInt(raw as u64),
                _ => return None,
            };
            (expr, false)
        }
        "SetV4" => {
            let raw = raw as u32;
            let expr = match ty.as_deref() {
                Some("float32") if f32::from_bits(raw).is_finite() => {
                    HeaderExpr::Real(fmt_float(ConstBits::W4(raw), false))
                }
                Some("uint") => HeaderExpr::UInt(raw as u64),
                Some("int") | None => HeaderExpr::Int(raw as i32 as i64),
                Some(t) if is_enum_name(t) => HeaderExpr::Int(raw as i32 as i64),
                _ => return None,
            };
            (expr, false)
        }
        "SetV8" => {
            let expr = match ty.as_deref() {
                Some("float" | "double" | "float64") if f64::from_bits(raw).is_finite() => {
                    HeaderExpr::Real(fmt_float(ConstBits::W8(raw), true))
                }
                Some("uint64") => HeaderExpr::UInt(raw),
                Some("int64") | None => HeaderExpr::Int(raw as i64),
                _ => return None,
            };
            (expr, false)
        }
        _ => return None,
    };
    Some(HeaderValue {
        expr,
        boolish,
        full_bool: false,
        ty,
    })
}

fn header_set_slot_type(known_ty: Option<String>, float_hint: bool, op: &str) -> Option<String> {
    if known_ty.is_some() || !float_hint {
        return known_ty;
    }
    match op {
        "SetV4" => Some("float32".into()),
        "SetV8" => Some("float".into()),
        _ => None,
    }
}

impl Structurer<'_> {
    fn jump_op(&self, bi: usize) -> &'static str {
        let b = &self.g.blocks[bi];
        self.ctx.instrs[b.instr_hi - 1].op.name
    }

    /// Emit blocks `[i, stop)` (block indices) at the given indent.
    /// `next` is always forced strictly greater than the current index (loop-safe).
    fn emit_range(&mut self, mut i: usize, stop: usize, depth: usize, out: &mut String) {
        let ind = "    ".repeat(depth);
        let mut guard = 0usize;
        while i < stop {
            guard += 1;
            if guard > self.g.blocks.len() + 4 {
                let _ = writeln!(out, "{ind}// <structurer bailout>");
                break;
            }
            let prev = i;
            let b = &self.g.blocks[i];
            let mut next;

            // batch-27: operand stack carried across a recognized Cast diamond — consumed by
            // the join block (always the immediately-next emitted block; see carry creation
            // below). take() unconditionally: if this block is emitted by an arm that cannot
            // accept an initial stack (switch / loop heads), the carry is dropped -> status-quo
            // behavior.
            let init: Vec<Arg> = match self.carry.take() {
                Some((t, s)) if t == i => s,
                _ => Vec::new(),
            };

            if let Some(after) = self.try_emit_switch(i, stop, depth, out) {
                // recovered compiler switch idiom (guards + JMPP dispatch); the whole
                // construct was emitted, continue after its JOIN.
                next = after;
            } else if let Some((body_end, cond)) = self.top_test_while(i, stop) {
                // top-test loop: `header: <cmp> Jcc exit; body; JMP header`
                let _ = writeln!(out, "{ind}while ({cond})");
                let _ = writeln!(out, "{ind}{{");
                self.emit_range(i + 1, body_end, depth + 1, out);
                let _ = writeln!(out, "{ind}}}");
                next = body_end;
            } else if let Some((latch, cond, cont_off, brk_off)) =
                self.uncond_latch_loop_recoverable(i, stop)
            {
                // batch-36 Stage C: mid-test loop with an UNCONDITIONAL-JMP latch, clean top-test
                // form, AND a FULLY recoverable body (Gate 2 passed inside the detector). The
                // header block `i` holds the `while (cond)` test (excluded); the body is
                // [i+1, latch) (the latch JMPs back to `i`, also excluded). `continue_off` =
                // header offset; `break_off` = the exit-test target.
                //
                // SAFETY: unlike loop_latch (where the loop is ALWAYS already detected), a
                // NEWLY-detected uncond-JMP loop only wins when its body fully structures — if
                // Gate 2 bails, `uncond_latch_loop_recoverable` returns None and this block falls
                // through to the `is_cond` arm, keeping the EXACT status-quo emission (the header
                // as an `if`). A newly-detected loop never emits a linearized/lossy body.
                let _ = writeln!(out, "{ind}while ({cond})");
                let _ = writeln!(out, "{ind}{{");
                let ls = LoopScope {
                    continue_off: cont_off,
                    break_off: brk_off,
                };
                let saved = self.loop_scope;
                self.loop_scope = Some(ls);
                self.emit_range(i + 1, latch, depth + 1, out);
                self.loop_scope = saved;
                let _ = writeln!(out, "{ind}}}");
                next = latch + 1;
            } else if let Some((body, exit, cond, cont_off, brk_off)) =
                self.compound_latch_loop_recoverable(i, stop)
            {
                // A bounded materialized top-test whose unique back-edge can occur inside a
                // nested switch case. The physical loop body extends to the proven loop exit,
                // including case/default blocks laid out after that back-edge.
                let _ = writeln!(out, "{ind}while ({cond})");
                let _ = writeln!(out, "{ind}{{");
                let saved = self.loop_scope;
                self.loop_scope = Some(LoopScope {
                    continue_off: cont_off,
                    break_off: brk_off,
                });
                self.emit_range(body, exit, depth + 1, out);
                self.loop_scope = saved;
                let _ = writeln!(out, "{ind}}}");
                next = exit;
            } else if let Some((test_idx, cond, body_head, break_off)) =
                self.test_first_foreach(i, stop)
            {
                // batch-44b (E2): a TEST-FIRST foreach. Block `i` is a bare `JMP →test` that enters
                // the loop AT the CanProceed test (guarding the first iteration). The iterator
                // SETUP (`PSF wIt; CALLSYS Iterator`) lives in block `i` before the JMP — emit it
                // ONCE, before the loop (loop-invariant construction). Then render the top-test
                // `while (it.CanProceed) { body }` where body = [body_head, test_idx) (the test
                // block IS the condition, re-lowered by the compiler to the same entry-JMP form) —
                // never the bottom-test `while (uninit_slot != 0)` the status-quo `loop_latch` arm
                // would emit. `continue_off` = test block start (a `continue;` re-tests);
                // `break_off` = the test's forward exit. Gate 2 already proved (in the detector)
                // that the body fully structures under this scope; on ANY deviation the detector
                // returned None and this arm never fired (the status-quo bottom-test stands).
                //
                // batch-44d: emit the entry block's setup statements WITH the incoming operand-stack
                // carry (`init`) — the Cast-diamond carry from a preceding block. A setup call whose
                // args were pushed BEFORE a Cast diamond (e.g. `BrushComponent.GetOverlappingActors(
                // out, filter)` — out/filter PSF'd in an earlier block, carried across the
                // null-check diamond into this entry/join block) needs that carry, else the call
                // renders 0-arg ("No matching signatures"). `emit_linear` starts with an EMPTY stack
                // and dropped those args; `block_stmts_in` with `init` restores them.
                let (mut stmts, _, _) =
                    block_stmts_in(self.ctx, b.instr_lo, b.instr_hi, init, false);
                // batch-44d: the entry block's trailing `<coll>.Iterator()` result is dropped by
                // build_call (the TMap/TSet Iterator lowers with NO PSF out-slot → a bare statement).
                // The iterator local has NO default constructor, so the hoisted `TMapIterator
                // local_It;` + `local_It.CanProceed` fails to compile. Capture the trailing
                // `Iterator()` into the iterator slot so `rewrite_iterator_decl_init` turns it into
                // `auto local_It = coll.Iterator();`.
                if let Some(it_slot) = self.foreach_iter_slot(test_idx) {
                    let it_name = self.ctx.slot_name(it_slot);
                    if let Some(last) = stmts.iter_mut().rev().find(|s| !s.trim().is_empty()) {
                        let t = last.trim();
                        if (t.ends_with(".Iterator();") || t.ends_with(".Iterator()"))
                            && assign_lhs(t).is_none()
                        {
                            *last = format!("{it_name} = {t}");
                        }
                    }
                }
                for s in &stmts {
                    let _ = writeln!(out, "{ind}{s}");
                }
                let _ = writeln!(out, "{ind}while ({cond})");
                let _ = writeln!(out, "{ind}{{");
                let test_off = self.g.blocks[test_idx].start_dw;
                let ls = LoopScope {
                    continue_off: test_off,
                    break_off,
                };
                let saved = self.loop_scope;
                self.loop_scope = Some(ls);
                self.emit_range(body_head, test_idx, depth + 1, out);
                self.loop_scope = saved;
                let _ = writeln!(out, "{ind}}}");
                next = test_idx + 1;
            } else if let Some(latch) = self.loop_latch(i, stop) {
                let (lstmts, lcmp) = block_stmts(
                    self.ctx,
                    self.g.blocks[latch].instr_lo,
                    self.g.blocks[latch].instr_hi,
                );
                let cond = branch_cond(&lcmp, self.jump_op(latch));
                // The latch's own statements are part of the test, re-run on every iteration —
                // see `fold_loop_header_store`. Dropped, they leave `while (local_1)` over a slot
                // nothing fills.
                loop_diag("latch", lstmts.as_slice(), &cond);
                let cond = fold_loop_header_store(lstmts.as_slice(), &cond).unwrap_or(cond);
                let _ = writeln!(out, "{ind}while ({cond})");
                let _ = writeln!(out, "{ind}{{");
                // batch-36 (Stage A): the loop body is blocks [i, latch) — block `i` (the loop
                // header the latch back-edges to) holds the first body statements + often the
                // first inner guard; the latch itself supplies the `while (cond)` test and is
                // excluded (stop = latch). `continue_off` = latch start (a `continue;` re-tests);
                // `break_off` = the latch's non-back-edge successor (the loop exit). Only recurse
                // when the body has a genuine inner branch (Gate 1) AND every branch resolves to a
                // recognized if/switch/break/continue/return shape (Gate 2); otherwise keep the
                // exact status-quo `emit_linear` call byte-for-byte.
                let latch_off = self.g.blocks[latch].start_dw;
                let break_off = self.g.blocks[latch]
                    .succs
                    .iter()
                    .copied()
                    .find(|&s| s > latch_off);
                if let Some(break_off) = break_off {
                    let ls = LoopScope {
                        continue_off: latch_off,
                        break_off,
                    };
                    // `loop_scope` must be set BEFORE Gate 2 — its nested-switch dry run
                    // (`switch_span` -> `try_emit_switch`) consults it to accept loop
                    // continue/break as switch exits, so the dry run and the real emit agree.
                    let saved = self.loop_scope;
                    self.loop_scope = Some(ls);
                    if self.body_has_inner_branch(i, latch, ls)
                        && self.loop_body_recoverable(i, latch, ls)
                    {
                        self.emit_range(i, latch, depth + 1, out);
                        self.loop_scope = saved;
                    } else {
                        self.loop_scope = saved;
                        self.emit_linear(i, latch + 1, depth + 1, out, true);
                    }
                } else {
                    self.emit_linear(i, latch + 1, depth + 1, out, true);
                }
                let _ = writeln!(out, "{ind}}}");
                next = latch + 1;
            } else if let Some((cond, brk)) = self.loop_break_continue(i) {
                // batch-36 (Stage A): inside a recursed loop body, a conditional jump whose taken
                // target is the loop break/continue offset — render `if (cond) { break;/continue; }`
                // and fall through to the remainder. `cond` is oriented so the keyword fires on the
                // TAKEN edge; the fall edge (block i+1) continues the body.
                let (stmts, _, _) = block_stmts_in(self.ctx, b.instr_lo, b.instr_hi, init, false);
                for s in &stmts {
                    let _ = writeln!(out, "{ind}{s}");
                }
                let _ = writeln!(out, "{ind}if ({cond})");
                let _ = writeln!(out, "{ind}{{");
                let _ = writeln!(out, "{ind}    {brk}");
                let _ = writeln!(out, "{ind}}}");
                next = i + 1;
            } else if self.is_cond(i) {
                let (stmts, cmp, leftover) =
                    block_stmts_in(self.ctx, b.instr_lo, b.instr_hi, init, false);
                for s in &stmts {
                    let _ = writeln!(out, "{ind}{s}");
                }
                let jop = self.jump_op(i);
                let fall = b.succs.get(1).copied();
                let taken = b.succs.first().copied();
                let then_idx = fall.and_then(|o| self.idx_of.get(&o).copied());
                let else_idx = taken.and_then(|o| self.idx_of.get(&o).copied());
                let cond = self.fall_condition(&cmp, jop);
                if let Some((straight, ret_idx)) = self.bool_return_diamond(then_idx, else_idx) {
                    // A guarded return and a bare one after it — NOT an `else` branch, which would
                    // make the second return jump to the shared exit instead of falling into it
                    // (measured: one extra `JMP` per function).
                    let _ = writeln!(out, "{ind}if ({cond})");
                    let _ = writeln!(out, "{ind}{{");
                    let _ = writeln!(out, "{ind}    return {straight};");
                    let _ = writeln!(out, "{ind}}}");
                    let _ = writeln!(out, "{ind}return {};", !straight);
                    i = (ret_idx + 1).max(i + 1);
                    continue;
                }
                let then_end = else_idx.unwrap_or(stop).min(stop).max(i + 1);
                // A then-arm whose LAST block jumps unconditionally BACK to at-or-before this
                // test is the latch of a loop the detectors could not take: they ask for a
                // single-block header, and a short-circuited condition is computed across
                // several. The latch has no rendering of its own, so the loop vanishes silently.
                // Mark it instead, and let the emitter turn the pair into a `while` once it has
                // folded the condition into one expression -- where it cannot, the mark is
                // dropped and this stays the `if` it is today.
                let latch_back = (then_end > i + 1
                    && self.is_backward_jump(then_end - 1)
                    && self.g.blocks[then_end - 1]
                        .succs
                        .first()
                        .is_some_and(|&s| s <= b.start_dw))
                .then_some(then_end - 1);
                // The latch block is NOT excluded: its jump is only its terminator, and the
                // statements before it are the body's last ones.
                let then_end_body = then_end;
                let _ = writeln!(out, "{ind}if ({cond})");
                let _ = writeln!(out, "{ind}{{");
                let then_body_at = out.len();
                if let Some(t) = then_idx {
                    if t > i && t <= then_end_body {
                        self.emit_range(t, then_end_body, depth + 1, out);
                    }
                }
                if latch_back.is_some() {
                    let _ = writeln!(out, "{ind}    {LOOP_BACK_EDGE}");
                }
                // Whether the arm we just WROTE ends in a return. The bytecode saying the branch
                // returns is not enough: a return this renderer cannot express — a void one, a
                // handle one, an RVO one — leaves the arm empty, and dropping the `else` would
                // then let what follows run on the path that was supposed to have left.
                let then_arm_returns = out[then_body_at..]
                    .lines()
                    .rev()
                    .find(|line| !line.trim().is_empty())
                    .is_some_and(|line| line.trim_start().starts_with("return"));
                let _ = writeln!(out, "{ind}}}");
                next = then_end;
                if let Some(ei) = else_idx {
                    // batch-36: inside a recursed loop body, a guard whose THEN branch exits the
                    // loop (its last block `JMP`s to the break/continue offset or to a bare-RET
                    // row = a `return`) has NO else — the taken target (`ei`) is the loop-body
                    // CONTINUATION, not an else clause. Emitting `else { <continuation> }` would
                    // wrongly nest the rest of the body. Only skip when in loop scope AND the
                    // then-block's JMP is such an exit; the non-loop diamond path is unchanged.
                    let then_exits_loop = self.loop_scope.is_some_and(|ls| {
                        ei > 0
                            && self.jump_op(ei - 1) == "JMP"
                            && self.g.blocks[ei - 1].succs.first().is_some_and(|&t| {
                                t == ls.break_off || t == ls.continue_off || self.is_bare_ret_off(t)
                            })
                    });
                    // Outside a loop the same fact holds on its own: a then-arm whose
                    // terminator jumps to the function's bare `RET` RETURNS, so there is no join
                    // and there is no `else` — the rest of the function is sequential. Writing
                    // one costs a `JMP` to a join vanilla never had (measured: 301 functions
                    // carry exactly that extra jump).
                    let then_returns = ei > 0
                        && self.jump_op(ei - 1) == "JMP"
                        && self.g.blocks[ei - 1]
                            .succs
                            .first()
                            .is_some_and(|&t| self.is_bare_ret_off(t));
                    // A test INSIDE the then-arm that fails to the same place this one does makes
                    // that place a shared TAIL, not an else arm: the source nested two `if`s and
                    // wrote the tail once behind them. Rendered as an `else`, the middle path — the
                    // outer test true, the inner false — runs nothing at all, and rendered as
                    // `A && B` it costs the carrier the compiler builds for a real `&&` (which
                    // vanilla always has where the source wrote one). Let it fall through.
                    let shares_the_tail = taken.is_some_and(|t| {
                        (then_idx.unwrap_or(i + 1)..then_end).any(|b| {
                            b < self.g.blocks.len()
                                && is_cond_op(self.jump_op(b))
                                && self.g.blocks[b].succs.first().copied() == Some(t)
                        })
                    });
                    let after_idx = self.g.blocks[ei - 1]
                        .succs
                        .first()
                        .and_then(|o| self.idx_of.get(o).copied())
                        .unwrap_or(stop)
                        .min(stop);
                    // ... unless vanilla WROTE the `else` after all. A `return` that is the last
                    // statement of the function's outermost block compiles to nothing but the
                    // `RET`; the same `return` one block deep compiles to a jump to the epilogue.
                    // So an else region whose last block `JMP`s to the bare `RET` row is one
                    // vanilla nested — flattening it drops that jump (measured: 94 functions,
                    // nearly all generated dialog `Act_Implementation`).
                    let else_tail_returns_from_a_block = after_idx > ei
                        && self.jump_op(after_idx - 1) == "JMP"
                        && self.g.blocks[after_idx - 1]
                            .succs
                            .first()
                            .is_some_and(|&t| self.is_bare_ret_off(t));
                    if ei >= then_end
                        && ei > 0
                        && self.jump_op(ei - 1) == "JMP"
                        && !then_exits_loop
                        && !shares_the_tail
                        && !(then_returns && then_arm_returns && !else_tail_returns_from_a_block)
                    {
                        if after_idx > ei {
                            let _ = writeln!(out, "{ind}else");
                            let _ = writeln!(out, "{ind}{{");
                            self.emit_range(ei, after_idx, depth + 1, out);
                            let _ = writeln!(out, "{ind}}}");
                            next = after_idx;
                            // Both arms left through a `return`, so the row they jump to is the
                            // function's epilogue and not a statement of its own: rendering it
                            // would put a `return;` after the `else` that no path can reach, and
                            // this compiler treats unreachable code as an error.
                            if then_returns
                                && then_arm_returns
                                && after_idx + 1 >= stop
                                && self.is_bare_ret_off(self.g.blocks[after_idx].start_dw)
                            {
                                next = stop;
                            }
                        }
                    }
                }
                // batch-27: a guard block's leftover operand stack survives into the JOIN
                // block's initial stack — ONLY when the construct is provably the null-check
                // Cast diamond (see diamond_join). Everything else keeps drop-at-boundary.
                if !leftover.is_empty() {
                    if let Some(j) = self.diamond_join(i, Some(next), &leftover, &cmp) {
                        self.carry = Some((j, leftover));
                    }
                }
            } else if self.suppress_ref_ret_row(i) {
                // batch-35a: the shared bare `RET wN` row of a ref-returning function whose
                // EVERY predecessor block already rendered its own `return <chain>;` (the
                // cross-block reference carry). Its scan-back value is a garbage int slot
                // ("Not a valid reference"); emit nothing (both arms returned) rather than a
                // wrong `return local_1;`. Same shape as the switch RET-row dead-code skip.
                next = i + 1;
            } else {
                // (linear fallthrough-carry is out of scope — the plain arm's leftover is dropped.)
                let ret_ref_tail = self.ctx.ret_is_ref() && self.flows_to_bare_ret(i);
                let (mut stmts, _, _) =
                    block_stmts_in(self.ctx, b.instr_lo, b.instr_hi, init, ret_ref_tail);
                let plain_return = self.plain_return_exit_stmt(i).map(|exit| {
                    let exit = fold_return_into_store(&mut stmts, exit);
                    self.constant_return_exit(i, exit, &stmts)
                });
                for s in &stmts {
                    let _ = writeln!(out, "{ind}{s}");
                }
                // Precedence: a switch case region (`exit_join`) is always the tighter scope when
                // set (its join is nearer than the enclosing loop's continue/break), so consult it
                // first; only fall to the loop-exit hook when no switch exit applies. batch-36:
                // `loop_exit_stmt` renders a bare `JMP` to the loop break/continue offset (or to a
                // bare-RET row inside the body) as `break;`/`continue;`/`return ...;`.
                if let Some(x) = self.region_exit_stmt(i) {
                    let _ = writeln!(out, "{ind}{}", self.constant_return_exit(i, x, &stmts));
                } else if let Some(x) = self.loop_exit_stmt(i) {
                    let _ = writeln!(out, "{ind}{}", self.constant_return_exit(i, x, &stmts));
                } else if let Some(x) = plain_return {
                    let _ = writeln!(out, "{ind}{x}");
                }
                next = i + 1;
            }

            i = next.max(prev + 1);
        }
    }

    /// Emit a linear run of blocks [i, end) as statements (loop body); the last block's
    /// trailing comparison/jump is dropped when `skip_term_cond`.
    ///
    /// batch-33e: the diamond carry works here too. Loop bodies flatten their diamonds
    /// (arms emitted sequentially), so a guard block's leftover operand stack used to die
    /// at every block boundary — an arg pushed BEFORE an in-loop Cast diamond never reached
    /// its consumer in the join (ANotifySpellCategoryActor::EndPlay rendered
    /// `RemoveTag()` 0-arg inside its m_Targets iterator loop). The SAME diamond_join proof
    /// applies unchanged: D10's dual-sim guarantees each arm emits identical statements
    /// with the carry as initial stack and returns it verbatim — so threading the carry
    /// through the flattened arm blocks up to the join is exactly the runtime stack on
    /// BOTH paths. `next` is None (no emission adjacency in linear mode); instead the
    /// join must lie inside this linear range.
    fn emit_linear(&mut self, i: usize, end: usize, depth: usize, out: &mut String, _skip: bool) {
        let ind = "    ".repeat(depth);
        let mut carry: Vec<Arg> = Vec::new();
        let mut carry_until: Option<usize> = None;
        for bi in i..end {
            let b = &self.g.blocks[bi];
            let init = match carry_until {
                Some(j) if bi <= j => std::mem::take(&mut carry),
                _ => Vec::new(),
            };
            let (mut stmts, cmp, leftover) =
                block_stmts_in(self.ctx, b.instr_lo, b.instr_hi, init, false);
            let plain_return = self.plain_return_exit_stmt(bi).map(|exit| {
                let exit = fold_return_into_store(&mut stmts, exit);
                self.constant_return_exit(bi, exit, &stmts)
            });
            for s in &stmts {
                let _ = writeln!(out, "{ind}{s}");
            }
            if let Some(x) = self.region_exit_stmt(bi) {
                let _ = writeln!(out, "{ind}{}", self.constant_return_exit(bi, x, &stmts));
            } else if let Some(x) = plain_return {
                let _ = writeln!(out, "{ind}{x}");
            }
            match carry_until {
                // still strictly inside the proven diamond: D10 returned the carry verbatim.
                Some(j) if bi < j => carry = leftover,
                _ => {
                    carry_until = None;
                    if !leftover.is_empty() {
                        if let Some(j) = self.diamond_join(bi, None, &leftover, &cmp) {
                            if j > bi && j < end {
                                carry = leftover;
                                carry_until = Some(j);
                            }
                        }
                    }
                }
            }
        }
    }

    /// Inside a recovered switch's case region: if block `bi` ends with an unconditional
    /// `JMP` that LEAVES the region, render the source-level exit statement — `break;` to
    /// the JOIN, or a synthesized `return ...;` when the jump goes to a bare `RET` row
    /// (the compiled form of `return <expr>;` from inside a case). Returns None outside
    /// switch emission or for any other terminator (statu quo: the JMP is just a block end).
    fn region_exit_stmt(&self, bi: usize) -> Option<String> {
        let join = self.exit_join?;
        let b = &self.g.blocks[bi];
        if self.ctx.instrs[b.instr_hi - 1].op.name != "JMP" {
            return None;
        }
        let t = *b.succs.first()?;
        if t == join {
            return Some(if self.exit_rvo_return && self.exit_join_is_ret {
                // batch-43 (Fix 2): struct-RVO return — the payload is in the `__return` out-slot
                // (a register scan-back is empty). Emit `return __return;`; the emission-site fold
                // (try_emit_switch) collapses the region's trailing `__return = <val>;` store +
                // this line into `return <val>;` to match vanilla's per-case value return.
                "return __return;".into()
            } else if self.exit_join_is_ret {
                self.ctx.return_stmt(scan_back_retval_floor(
                    self.ctx,
                    b.instr_hi - 1,
                    self.exit_scan_floor,
                ))
            } else {
                "break;".into()
            });
        }
        // A backward edge to the enclosing loop's *proven* continue target is an early switch
        // exit, not the switch JOIN. Keep this narrower than `break_off`: `break;` written inside
        // a switch is lexically a switch break and cannot represent an enclosing-loop break.
        if self
            .loop_scope
            .is_some_and(|ls| t == ls.continue_off && ls.continue_off < self.g.blocks[bi].start_dw)
        {
            return Some("continue;".into());
        }
        if self.exit_mixed_rvo_ret_rows_ok
            && self.is_bare_ret_off(t)
            && self.proves_mixed_rvo_exit(bi, t)
        {
            // The validating pass proved this edge's own block writes the hidden RVO slot before
            // any cleanup and the jump. Keep the store and cleanup in order, then return its
            // copied value; folding to the original source local could use it after destruction.
            return Some("return __return;".into());
        }
        if self.exit_ret_rows_ok && self.is_bare_ret_off(t) {
            return Some(self.ctx.return_stmt(scan_back_retval_floor(
                self.ctx,
                b.instr_hi - 1,
                self.exit_scan_floor,
            )));
        }
        None
    }

    /// Prove one mixed-RVO early-return edge locally. This deliberately does not accept a store
    /// in a predecessor: every `JMP -> bare RET` must carry its own clean out-slot write so no CFG
    /// dominance guess can turn an uninitialised `__return` into a source return.
    fn proves_mixed_rvo_exit(&self, bi: usize, target: usize) -> bool {
        if !self.ctx.ret_via_rvo() || !self.is_bare_ret_off(target) {
            return false;
        }
        let b = &self.g.blocks[bi];
        if self.ctx.instrs[b.instr_hi - 1].op.name != "JMP"
            || b.succs.first().copied() != Some(target)
        {
            return false;
        }
        let (stmts, _) = block_stmts(self.ctx, b.instr_lo, b.instr_hi);
        single_clean_rvo_store(&stmts).is_some()
    }

    /// The compiler's bool-return normalization. `return <expr>;` out of a bool function does not
    /// copy the expression straight into the value register: it branches on it and materializes a
    /// literal 1 or 0 into a slot, copies THAT, and both arms end at one shared bare `RET`.
    /// Written back arm by arm the returned expression is lost (`local_N = 1;` in one arm and a
    /// literal `return false;` at the end), so recognize the shape and give the return its
    /// condition back. `Some((true, ret))`: the THEN arm carries the 1, so the source is
    /// `return <cond>;`; `Some((false, ret))`: the arms are the other way round.
    fn bool_return_diamond(
        &self,
        then_idx: Option<usize>,
        else_idx: Option<usize>,
    ) -> Option<(bool, usize)> {
        if self
            .ctx
            .ret_ty
            .map(|t| t.base_name(self.ctx.refs))
            .as_deref()
            != Some("bool")
        {
            return None;
        }
        let (then_slot, then_val, ret_off) = self.bool_return_arm(then_idx?, true)?;
        let (else_slot, else_val, else_ret) = self.bool_return_arm(else_idx?, false)?;
        if then_slot != else_slot || ret_off != else_ret || then_val == else_val {
            return None;
        }
        Some((then_val == 1, *self.idx_of.get(&ret_off)?))
    }

    /// One arm of [`Self::bool_return_diamond`]: `SetV1 wN, K` + `CpyVtoR{1,4} wN`, then either a
    /// `JMP` into the shared bare `RET` row (the arm that has to jump over the other) or a plain
    /// fall-through into it. Returns the slot, the literal and the `RET` row's offset.
    fn bool_return_arm(&self, bi: usize, jumps: bool) -> Option<(u16, u32, usize)> {
        let b = &self.g.blocks[bi];
        if b.instr_hi - b.instr_lo != if jumps { 3 } else { 2 } {
            return None;
        }
        let set = &self.ctx.instrs[b.instr_lo];
        let copy = &self.ctx.instrs[b.instr_lo + 1];
        if set.op.name != "SetV1" || !matches!(copy.op.name, "CpyVtoR1" | "CpyVtoR4") {
            return None;
        }
        let slot = set.words.first().copied()?;
        if copy.words.first().copied()? != slot {
            return None;
        }
        let value = set.dwords.first().copied()?;
        if value > 1 || (jumps && self.ctx.instrs[b.instr_hi - 1].op.name != "JMP") {
            return None;
        }
        let target = *b.succs.first()?;
        self.is_bare_ret_off(target)
            .then_some((slot, value, target))
    }

    /// A block that copies its value into the value register and then jumps to the function's
    /// single bare `RET` row IS a `return <expr>;`. Rendered without it the copy is dropped, the
    /// store before it is left behind and the shared `RET` returns whatever the OTHER path put
    /// in that slot. This is the plain-block counterpart of [`Self::region_exit_stmt`] and
    /// [`Self::loop_exit_stmt`], and is consulted only after both of them.
    fn plain_return_exit_stmt(&self, bi: usize) -> Option<String> {
        if self.ctx.ret_via_rvo() || self.ctx.ret_is_ref() {
            return None;
        }
        let b = &self.g.blocks[bi];
        if b.instr_hi == b.instr_lo || self.ctx.instrs[b.instr_hi - 1].op.name != "JMP" {
            return None;
        }
        // A VOID function's early return carries no value: the whole of it is the jump to the
        // shared exit row. Rendered as nothing, the branch it sits in looks like it falls
        // through — which is not a byte difference but a WRONG program, and it is what made
        // `if (x == nullptr) { … }` run the null-dereferencing tail behind it.
        if self.ctx.ret_ty.map(|t| t.token) == Some(0x52) {
            return b
                .succs
                .first()
                .is_some_and(|t| self.is_bare_ret_off(*t))
                .then(|| "return;".to_owned());
        }
        if b.instr_hi - b.instr_lo < 2 {
            return None;
        }
        // `LOADOBJ` is how a HANDLE return puts its value in place, exactly as `CpyVtoR*` does
        // for a scalar — and `scan_back_retval_floor` already reads it. Without it a handle
        // function's early return rendered as nothing, the same silence the void case had.
        if !matches!(
            self.ctx.instrs[b.instr_hi - 2].op.name,
            "CpyVtoR1" | "CpyVtoR4" | "CpyVtoR8" | "LOADOBJ"
        ) {
            return None;
        }
        if !b.succs.first().is_some_and(|t| self.is_bare_ret_off(*t)) {
            return None;
        }
        Some(self.ctx.return_stmt(scan_back_retval_floor(
            self.ctx,
            b.instr_hi - 1,
            b.instr_lo,
        )))
    }

    /// The condition under which the test FALLS THROUGH — the one an `if` is written with. A
    /// bool slot tested for zero reads as itself; everything else is the taken condition negated.
    fn fall_condition(&self, cmp: &Option<Cmp>, jop: &str) -> String {
        cmp.as_ref()
            .and_then(|c| c.expr.as_deref())
            .and_then(|expr| {
                let slot = expr
                    .strip_prefix("local_")
                    .and_then(|digits| digits.parse::<i32>().ok());
                (matches!(jop, "JZ" | "JLowZ")
                    && slot.and_then(|slot| self.ctx.slot_type(slot)).as_deref() == Some("bool"))
                .then(|| expr.to_string())
            })
            .unwrap_or_else(|| negate(&branch_cond(cmp, jop)))
    }

    /// `return local_N;` where a `SetV1` wrote the slot the instruction before the return read
    /// it returns THAT CONSTANT. `SetV*` registers a constant rather than rendering a statement,
    /// so where no store was rendered for the store fold to take, the return kept the slot — and
    /// the slot still carried the condition just tested. `if (!ok) { return false; }` came back
    /// as `return <the condition>`, which is `true` there: not a byte difference, a wrong
    /// program. Only applies when the fold left a bare slot behind.
    fn constant_return_exit(&self, bi: usize, exit: String, stmts: &[String]) -> String {
        let Some(name) = exit
            .strip_prefix("return ")
            .and_then(|v| v.strip_suffix(';'))
            .filter(|v| v.starts_with("local_") && !v.contains(['.', '(', ' ', '[']))
        else {
            return exit;
        };
        if self.ctx.ret_ty.map(|t| t.token) != Some(0x41) {
            return exit;
        }
        let b = &self.g.blocks[bi];
        if b.instr_hi < b.instr_lo + 3 {
            return exit;
        }
        let read = b.instr_hi - 2;
        let slot = self.ctx.instrs[read].words.first().copied().map(s16);
        if slot.map(|s| self.ctx.slot_name(s)).as_deref() != Some(name) {
            return exit;
        }
        // The constant is not always the instruction before the read: a return inside a scope
        // that owns a temporary has that temporary's destructor between the two. Scan back to the
        // start of the block over the ops that cannot write this slot, and stop at anything else.
        let mut wrote = None;
        for at in (b.instr_lo..read).rev() {
            let ins = &self.ctx.instrs[at];
            match ins.op.name {
                "SetV1" if ins.words.first().copied().map(s16) == slot => {
                    wrote = Some(ins);
                    break;
                }
                "PSF" | "CALLSYS" | "SUSPEND" => {}
                "FreeNullV8" if ins.words.first().copied().map(s16) != slot => {}
                _ => break,
            }
        }
        let Some(prev) = wrote else { return exit };
        // A rendered store of the SAME constant already carries it; a rendered store of anything
        // else carries the condition that was tested, which is not what this return returns.
        let bits = prev.dwords.first().copied().unwrap_or(0);
        let literal = self.ctx.return_stmt(Some((bits as i32).to_string()));
        let rendered_constant = literal
            .strip_prefix("return ")
            .and_then(|v| v.strip_suffix(';'))
            .map(|v| format!("{name} = {v};"))
            .unwrap_or_default();
        if stmts.iter().any(|s| s.trim() == rendered_constant) {
            return exit;
        }
        literal
    }

    /// The block at dword offset `off` is a bare `RET` row (exactly one instruction).
    fn is_bare_ret_off(&self, off: usize) -> bool {
        self.idx_of.get(&off).is_some_and(|&bi| {
            let b = &self.g.blocks[bi];
            b.instr_hi - b.instr_lo == 1 && self.ctx.instrs[b.instr_lo].op.name == "RET"
        })
    }

    /// batch-36 (Stage A) — mirrors [`Self::region_exit_stmt`] for the loop body. Inside a
    /// recursed loop body (`loop_scope` set), a block ending in a bare unconditional `JMP` to
    /// the loop `break_off`/`continue_off` renders `break;`/`continue;`; a `JMP` to a bare `RET`
    /// row (a `return <expr>;` from inside the loop, e.g. GetFreeArm) renders the synthesized
    /// return. Returns None outside loop emission or for any other terminator (status quo: the
    /// JMP is just a block end that the diamond machinery / plain arm drops).
    fn loop_exit_stmt(&self, bi: usize) -> Option<String> {
        let ls = self.loop_scope?;
        let b = &self.g.blocks[bi];
        if self.ctx.instrs[b.instr_hi - 1].op.name != "JMP" {
            return None;
        }
        let t = *b.succs.first()?;
        if t == ls.break_off {
            return Some("break;".into());
        }
        if t == ls.continue_off {
            return Some("continue;".into());
        }
        // an in-body `return <expr>;` compiled as a JMP to the function's shared bare RET row —
        // recover the returned value from the block's trailing register write (scan floored to
        // this block so no value leaks in from a preceding block).
        if self.is_bare_ret_off(t) {
            return Some(self.ctx.return_stmt(scan_back_retval_floor(
                self.ctx,
                b.instr_hi - 1,
                b.instr_lo,
            )));
        }
        None
    }

    /// batch-36 (Stage A) — a conditional jump inside a recursed loop body whose TAKEN target is
    /// the loop break/continue offset. Returns `(cond, keyword)` where `cond` fires the keyword on
    /// the taken edge and the fall edge (block `bi+1`) continues the body — rendered as
    /// `if (cond) { break;/continue; }`. `None` when not inside a loop, not a 2-succ conditional,
    /// or the taken target is not exactly the break/continue offset (a genuine inner diamond is
    /// left to the `is_cond` arm). The fall successor MUST be the physically next block so the
    /// remaining body flows straight on.
    fn loop_break_continue(&self, bi: usize) -> Option<(String, &'static str)> {
        let ls = self.loop_scope?;
        let b = &self.g.blocks[bi];
        let jop = self.jump_op(bi);
        if !matches!(
            jop,
            "JZ" | "JNZ" | "JS" | "JNS" | "JP" | "JNP" | "JLowZ" | "JLowNZ"
        ) {
            return None;
        }
        if b.succs.len() != 2 {
            return None;
        }
        let taken = *b.succs.first()?;
        let fall = *b.succs.get(1)?;
        // the fall edge must be the immediately-following block (straight-line body remainder)
        if self.idx_of.get(&fall).copied() != Some(bi + 1) {
            return None;
        }
        let kw = if taken == ls.break_off {
            "break;"
        } else if taken == ls.continue_off {
            "continue;"
        } else {
            return None;
        };
        let cmp = block_stmts(self.ctx, b.instr_lo, b.instr_hi).1;
        // the keyword fires on the TAKEN edge, so the rendered condition is the jump's own sense
        // (not negated — negation is for the fall-through-then pattern of `is_cond`).
        let cond = branch_cond(&cmp, jop);
        Some((cond, kw))
    }

    /// batch-36 Gate 1 — true iff the loop body `[lo, hi)` (block indices) contains a GENUINE
    /// inner conditional: a 2-successor conditional jump (not the latch) that either forks
    /// forward inside the body (a real `if`) or exits to break/continue/return. A body with no
    /// such branch is straight-line (or a diamond-carry body) and keeps the exact status-quo
    /// `emit_linear` call byte-for-byte — this preserves the clean loops and the diamond-carry
    /// loops. Cheap linear scan.
    fn body_has_inner_branch(&self, lo: usize, hi: usize, ls: LoopScope) -> bool {
        for bi in lo..hi {
            let b = &self.g.blocks[bi];
            let jop = self.jump_op(bi);
            if !matches!(
                jop,
                "JZ" | "JNZ" | "JS" | "JNS" | "JP" | "JNP" | "JLowZ" | "JLowNZ"
            ) {
                continue;
            }
            if b.succs.len() != 2 {
                continue;
            }
            // an inner forward diamond (both succs strictly inside the body), OR a conditional
            // exit to break/continue/an in-body return
            let forward_inner = b
                .succs
                .iter()
                .all(|&s| self.idx_of.get(&s).is_some_and(|&si| si > bi && si <= hi));
            let exits = b
                .succs
                .iter()
                .any(|&s| s == ls.break_off || s == ls.continue_off || self.is_bare_ret_off(s));
            if forward_inner || exits {
                return true;
            }
        }
        false
    }

    /// batch-36 Gate 2 — non-emitting dry run over the loop body `[lo, hi)`: assert that EVERY
    /// block's terminator resolves to a recognized shape so the recursive emit can never produce
    /// wrong control flow. On ANY anomaly return false ⇒ the loop falls back to the exact
    /// status-quo `emit_linear` (a dropped-but-compiling body, never a wrong one). Recognized
    /// per block:
    ///   * plain fallthrough (no jump) to the next in-body block;
    ///   * `RET` of its own;
    ///   * an unconditional `JMP` to break/continue/an in-body bare-RET row (an exit keyword or
    ///     return), or to a forward in-body block (an `if`/`else` skip);
    ///   * a 2-succ conditional whose BOTH targets are in-body-forward (a diamond) OR whose taken
    ///     is break/continue/return and whose fall is the next in-body block (a guard);
    ///   * a recognized nested switch or nested loop (dry-run detection).
    /// Any edge that leaves `[header, break_off]` other than via continue/break/return, an
    /// irreducible edge, a `JMPP` we cannot map, or a conditional we cannot classify ⇒ false.
    fn loop_body_recoverable(&mut self, lo: usize, hi: usize, ls: LoopScope) -> bool {
        let header_off = self.g.blocks[lo].start_dw;
        let mut bi = lo;
        while bi < hi {
            // a nested switch consumes its own multi-block region — trust its self-validating
            // dry run and skip past it (it internally bounds itself by `hi`).
            if let Some(end) = self.switch_span(bi, hi) {
                if end <= bi || end > hi {
                    return false;
                }
                bi = end;
                continue;
            }
            // a nested loop: its latch back-edges to an in-body header; recurse the recognizer
            // over its own body, then skip past its latch.
            if let Some(inner) = self.loop_latch(bi, hi) {
                let inner_off = self.g.blocks[inner].start_dw;
                let inner_break = self.g.blocks[inner]
                    .succs
                    .iter()
                    .copied()
                    .find(|&s| s > inner_off);
                match inner_break {
                    Some(ib) => {
                        let inner_ls = LoopScope {
                            continue_off: inner_off,
                            break_off: ib,
                        };
                        // the inner loop's exit must stay inside our body or be our own exit.
                        let ib_in_body = self
                            .idx_of
                            .get(&ib)
                            .copied()
                            .is_some_and(|x| x >= lo && x <= hi);
                        if !self.loop_edge_ok(ib, lo, hi, ls) && !ib_in_body {
                            return false;
                        }
                        // the inner body's own dry run must see the inner loop_scope (so a switch
                        // nested in the inner loop resolves against the inner continue/break).
                        let saved = self.loop_scope;
                        self.loop_scope = Some(inner_ls);
                        let ok = self.loop_body_recoverable(bi, inner, inner_ls);
                        self.loop_scope = saved;
                        if !ok {
                            return false;
                        }
                        bi = inner + 1;
                        continue;
                    }
                    None => return false,
                }
            }
            let succs = self.g.blocks[bi].succs.clone();
            let jop = self.jump_op(bi);
            match jop {
                "RET" => {}
                "JMP" => {
                    let t = match succs.first() {
                        Some(&t) => t,
                        None => return false,
                    };
                    if !self.loop_edge_ok(t, lo, hi, ls) {
                        return false;
                    }
                }
                "JMPP" => return false, // a nested JMPP not recognized as a switch — cannot map
                j if matches!(
                    j,
                    "JZ" | "JNZ" | "JS" | "JNS" | "JP" | "JNP" | "JLowZ" | "JLowNZ"
                ) =>
                {
                    if succs.len() != 2 {
                        return false;
                    }
                    let taken = succs[0];
                    let fall = succs[1];
                    // a backward conditional inside the body that is NOT the recognized inner
                    // loop latch (handled above) is irreducible — bail.
                    if taken <= header_off && taken != ls.continue_off {
                        return false;
                    }
                    // fall must always continue in-body (the physically next block)
                    if self.idx_of.get(&fall).copied() != Some(bi + 1) {
                        return false;
                    }
                    // taken: an in-body forward diamond, an exit keyword, or an in-body return
                    let taken_ok = self.loop_edge_ok(taken, lo, hi, ls)
                        || self
                            .idx_of
                            .get(&taken)
                            .copied()
                            .is_some_and(|ti| ti > bi && ti <= hi);
                    if !taken_ok {
                        return false;
                    }
                }
                _ => {
                    // plain fallthrough terminator: the single successor (if any) must continue
                    // in-body (into the next block or the latch/continue).
                    if let Some(&s) = succs.first() {
                        if succs.len() != 1 || !self.loop_edge_ok(s, lo, hi, ls) {
                            return false;
                        }
                    }
                }
            }
            bi += 1;
        }
        true
    }

    /// An in-body edge target `s` is legal iff it lands on a block inside `[lo, hi]`, OR is a
    /// recognized exit (loop break/continue offset, or an in-body `return` via a bare-RET row).
    fn loop_edge_ok(&self, s: usize, lo: usize, hi: usize, ls: LoopScope) -> bool {
        if s == ls.break_off || s == ls.continue_off || self.is_bare_ret_off(s) {
            return true;
        }
        self.idx_of
            .get(&s)
            .copied()
            .is_some_and(|si| si >= lo && si <= hi)
    }

    /// batch-36 — non-emitting probe: if a recognized compiler `switch` idiom starts at block
    /// `bi` and is fully self-validating within `[bi, cap)`, return the block index just past its
    /// JOIN; otherwise None. Used by [`Self::loop_body_recoverable`] to skip a nested switch
    /// during the dry run. Implemented by a throwaway emit into a scratch buffer under the
    /// current `loop_scope` (the same context the real emit will use), reusing the exact
    /// `try_emit_switch` validation so the dry run and the real run agree.
    fn switch_span(&mut self, bi: usize, cap: usize) -> Option<usize> {
        let saved_carry = self.carry.take();
        let mut scratch = String::new();
        let r = self.try_emit_switch(bi, cap, 0, &mut scratch);
        self.carry = saved_carry; // discard any carry the dry run produced
        r
    }

    /// batch-35a (cross-block reference carry): block `bi` reaches the function's shared bare
    /// `RET` row via its SOLE successor — either an unconditional `JMP` to it, or a fallthrough
    /// into it (a block ending in a value-producing call, e.g. the trailing opIndex chain of a
    /// ref-returning `FindRoleGroup` predecessor). Requires exactly one successor (no
    /// conditional fork) and that the block does NOT already end in `RET` (the RET arm handles
    /// its own block). Used only when `ctx.ret_is_ref()`; the block's trailing by-reference
    /// pending then renders as `return <chain>;` (see `block_stmts_in`'s `ret_ref_tail`).
    fn flows_to_bare_ret(&self, bi: usize) -> bool {
        let b = &self.g.blocks[bi];
        if self.ctx.instrs[b.instr_hi - 1].op.name == "RET" {
            return false;
        }
        b.succs.len() == 1 && self.is_bare_ret_off(b.succs[0]) && self.block_tail_is_ref_call(bi)
    }

    /// batch-35a: the block's last value-producing instruction is a call whose cache return
    /// DataType `is_reference` — i.e. the block ends by leaving a by-reference lvalue in the
    /// register (`... .RoleGroups.opIndex(i)` -> FCombatRoleGroup&), the exact producer the
    /// reference carry turns into `return <chain>;`. Trailing pure housekeeping (JMP, PopPtr,
    /// FreeNullV8, ...) is skipped. A block NOT ending in a ref-returning call (a value store,
    /// a void call, a bare fallthrough) fails — so a legitimate register-value return through
    /// the shared bare RET row is never suppressed. Bails (false) on any non-call tail.
    fn block_tail_is_ref_call(&self, bi: usize) -> bool {
        let b = &self.g.blocks[bi];
        for j in (b.instr_lo..b.instr_hi).rev() {
            let ins = &self.ctx.instrs[j];
            match ins.op.name {
                // pure control/stack housekeeping that can trail a value-producing call
                "JMP" | "PopPtr" | "PshRPtr" | "SwapPtr" | "ClrHi" | "ClrVPtr" | "FreeNullV8"
                | "FREE" | "CHKREF" | "ChkRefS" | "ChkNullV" | "ChkNullS" | "SUSPEND"
                | "SaveReturnValue" | "ResolveObjectPtr" => continue,
                "CALLSYS" | "Thiscall1" => {
                    let ptr = ins.qwords.first().copied().unwrap_or(0) as i64;
                    return self
                        .ctx
                        .refs
                        .func_ret_by_ptr(ptr)
                        .map(|d| d.is_reference)
                        .unwrap_or(false);
                }
                "CALL" | "CALLINTF" | "CALLBND" => {
                    let id = ins.dwords.first().copied().unwrap_or(0) as i32;
                    return self
                        .ctx
                        .refs
                        .func_ret_by_id(id)
                        .map(|d| d.is_reference)
                        .unwrap_or(false);
                }
                _ => return false,
            }
        }
        false
    }

    /// batch-35a: block `bi` is the shared bare `RET` row of a ref-returning function whose
    /// EVERY predecessor is a `flows_to_bare_ret` block — i.e. each predecessor renders its own
    /// `return <chain>;` via the reference carry, so this row's own `return <scan-back>;` (a
    /// garbage int slot -> "Not a valid reference") is dead. Emit nothing. Gated hard: at least
    /// one predecessor must exist, the function must return by reference, and NO predecessor may
    /// be a normal fall-in that DIDN'T get the carry (that would drop a legitimate return).
    fn suppress_ref_ret_row(&self, bi: usize) -> bool {
        if !self.ctx.ret_is_ref() {
            return false;
        }
        let off = self.g.blocks[bi].start_dw;
        if !self.is_bare_ret_off(off) {
            return false;
        }
        let mut preds = 0usize;
        for (pi, pb) in self.g.blocks.iter().enumerate() {
            if pb.succs.contains(&off) {
                preds += 1;
                if !self.flows_to_bare_ret(pi) {
                    return false;
                }
            }
        }
        preds > 0
    }

    /// Detect and emit the Hazelight compiler's `switch` lowering rooted at block `i`
    /// (see `work/reversing/gore-as/specs/illegal-op-round2.md` Part B). The 5-part idiom:
    ///
    /// ```text
    /// [ ...stmts ; CMPIi wV,hi ; JP DEF ]      block i    (range guard, hi = lo+N-1)
    /// [ CMPIi wV,lo ; JS DEF ]                 block i+1  (range guard)
    /// [ SUBIi wS,wV,lo ; JMPP wS,N-1 ]         block i+2  (normalize + dispatch)
    /// [ N dispatch rows: JMP tK trampolines; the LAST row may be the final case inlined ]
    /// case regions in offset order ... [ DEF region ] JOIN
    /// ```
    ///
    /// DEF handling: `ThrowException`-only DEF = compiler trap for a default-less switch ->
    /// emit NO default clause (recompiling regenerates the trap); DEF == JOIN -> no default;
    /// DEF sharing a case entry -> stacked `default:` label; else a real `default:` region.
    /// Shared case targets render stacked `case a: case b:` labels; empty cases (entry ==
    /// a JMP-to-JOIN thunk) render `break;` only.
    ///
    /// Returns the next block index after the construct, or None on ANY deviation from the
    /// idiom — the caller then falls through to the existing arms and the `// JMPP` marker
    /// keeps the function safely stubbed (never wrong control flow).
    fn try_emit_switch(
        &mut self,
        i: usize,
        stop: usize,
        depth: usize,
        out: &mut String,
    ) -> Option<usize> {
        let ctx = self.ctx;
        let g = self.g;
        let blocks = &g.blocks;
        // batch-36 (Stage B): when this switch is emitted inside a recursed loop body, a case/DEF
        // region that exits by falling into the loop-continue point (the increment/latch = `stop`)
        // or by jumping to the loop break offset is a legal switch exit (`break;` out of the
        // switch → the loop continues), not the "escapes the region" bail. `loop_scope` is set by
        // the loop_latch arm before both the Gate-2 dry run and the real emit, so the two agree.
        let lscope = self.loop_scope;
        if stop > blocks.len() || i + 3 > stop || i + 2 >= blocks.len() {
            return None;
        }
        // cheap pre-probe before anything costly
        if self.jump_op(i) != "JP" || self.jump_op(i + 1) != "JS" || self.jump_op(i + 2) != "JMPP" {
            return None;
        }
        // A bottom-test loop headed at this very block would be LOST if the switch were
        // emitted in its place (the latch back-edge has no rendering); bail to the loop
        // arm — its linear body hits the JMPP marker and the function stays stubbed.
        if self.loop_latch(i, stop).is_some() {
            return None;
        }
        let b0 = &blocks[i];
        let b1 = &blocks[i + 1];
        let b2 = &blocks[i + 2];
        if b0.instr_hi - b0.instr_lo < 2
            || b1.instr_hi - b1.instr_lo != 2
            || b2.instr_hi - b2.instr_lo != 2
        {
            return None;
        }
        let g_hi = &ctx.instrs[b0.instr_hi - 2];
        let g_lo = &ctx.instrs[b1.instr_lo];
        let sub = &ctx.instrs[b2.instr_lo];
        let jmpp = &ctx.instrs[b2.instr_hi - 1];
        if g_hi.op.name != "CMPIi" || g_lo.op.name != "CMPIi" || sub.op.name != "SUBIi" {
            return None;
        }
        let wv = s16(g_hi.words.first().copied()?);
        let hi_c = g_hi.dwords.first().copied()? as i32;
        let lo_c = g_lo.dwords.first().copied()? as i32;
        if s16(g_lo.words.first().copied()?) != wv {
            return None;
        }
        // SUBIi wS, wV, lo — selector normalization (emitted even for lo == 0); wS is dead
        // once the switch is recovered, and never rendered (blocks i+1/i+2 emit no stmts).
        let ws = s16(sub.words.first().copied()?);
        if s16(sub.words.get(1).copied()?) != wv || sub.dwords.first().copied()? as i32 != lo_c {
            return None;
        }
        if s16(jmpp.words.first().copied()?) != ws {
            return None;
        }
        let n = jmpp.dwords.first().copied()? as usize + 1;
        if n < 2 || hi_c != lo_c + n as i32 - 1 {
            return None;
        }
        // guard edges: both guards jump to the same DEF; fallthroughs chain b0 -> b1 -> b2
        if b0.succs.len() != 2 || b1.succs.len() != 2 {
            return None;
        }
        let def_off = b0.succs[0];
        if b0.succs[1] != b1.start_dw || b1.succs[0] != def_off || b1.succs[1] != b2.start_dw {
            return None;
        }
        if b2.succs.len() != n {
            return None; // cfg.rs could not verify the dispatch-row shape
        }
        // dispatch rows -> case entry offsets (row N-1 may BE the last case body, inlined)
        let mut targets: Vec<usize> = Vec::with_capacity(n);
        let mut inline_last = false;
        for (k, &row) in b2.succs.iter().enumerate() {
            if row != jmpp.offset_dw + 2 + 2 * k {
                return None;
            }
            let rb = &blocks[*self.idx_of.get(&row)?];
            if rb.instr_hi - rb.instr_lo == 1 && ctx.instrs[rb.instr_lo].op.name == "JMP" {
                targets.push(*rb.succs.first()?);
            } else if k == n - 1 {
                targets.push(row);
                inline_last = true;
            } else {
                return None;
            }
        }
        // region boundaries: unique case entries + DEF, ascending; the first must start
        // immediately after the dispatch rows (no unreachable gap)
        let mut bounds: Vec<usize> = targets.clone();
        bounds.push(def_off);
        bounds.sort_unstable();
        bounds.dedup();
        let first_body = if inline_last {
            b2.succs[n - 1]
        } else {
            b2.succs[n - 1] + 2
        };
        if bounds[0] != first_body {
            return None;
        }
        let first_region_idx = i + 3 + n - usize::from(inline_last);
        if self.idx_of.get(&bounds[0]).copied() != Some(first_region_idx) {
            return None;
        }
        // JOIN inference: every region between consecutive boundaries must END by leaving —
        // JMP to a common exit beyond the last boundary (the JOIN), JMP to a bare RET row
        // (a per-case `return`), a RET of its own, or a fallthrough into the JOIN itself.
        let t_last = *bounds.last()?;
        // register-based return (value/object register): a struct-by-value return travels
        // through the hidden RVO slot instead, so `JMP -> RET row` can't be synthesized.
        // Decided from the return TYPE, not `rvo_off`: the param-map heuristic misclassifies
        // ENUM returns as RVO (token 5, see spec A4), yet enums return in the value register
        // — `ctx.rvo_off` would wrongly bail every enum-returning switch (GetNodeStatus,
        // the UCBT_*::Tick family).
        let register_based = match ctx.ret_ty {
            None => true,
            Some(t) => {
                t.token != 5
                    || t.is_object_handle
                    || t.is_reference
                    || is_enum_name(&t.base_name(ctx.refs))
            }
        };
        // batch-43 (Fix 2, switch-rvo-return.md §4.2): a struct-by-value RVO return switch
        // (`!register_based` AND a genuine RVO out-slot, NOT enum). Its per-case returns travel
        // through the `__return` out-slot, so `JMP -> bare RET` cannot be synthesized from the
        // register — the status quo bails the whole switch (leaving the function stubbed).
        // Under this mode we RELAX the two `!register_based` bare-RET bails, but ONLY after a
        // per-region `__return`-write proof (gate G-RVO, below) confirms EVERY region writes the
        // out-slot before its exit; any unproven region bails the whole switch. Never emit a
        // switch that silently returns a DEFAULT struct.
        let rvo_return_mode = !register_based && ctx.ret_via_rvo() && ctx.rvo_off.is_some();
        let non_void = ctx.ret_ty.map(|t| t.token != 0x52).unwrap_or(false);
        let mut join_cands: Vec<usize> = Vec::new();
        let mut ret_rows: Vec<usize> = Vec::new();
        let mut fall_pend: Vec<usize> = Vec::new();
        // A backward edge to the exact enclosing-loop continue target is a proven early exit.
        // It is never a JOIN candidate. A forward continue point can retain the older, ordinary
        // `switch break -> loop increment/test` JOIN interpretation. `break_off` is deliberately
        // excluded: a source `break;` inside the switch cannot exit the enclosing loop.
        let early_continue = lscope
            .filter(|ls| ls.continue_off < b0.start_dw)
            .map(|ls| ls.continue_off);
        let is_forward_loop_join = |x: usize| -> bool {
            lscope.is_some_and(|ls| x == ls.continue_off && ls.continue_off >= b0.start_dw)
        };
        let is_loop_break = |x: usize| -> bool { lscope.is_some_and(|ls| x == ls.break_off) };
        for w in bounds.windows(2) {
            let last_bi = self.idx_of.get(&w[1]).copied()? - 1;
            let lb = &blocks[last_bi];
            match ctx.instrs[lb.instr_hi - 1].op.name {
                "JMP" => {
                    let x = lb.succs.first().copied()?;
                    if is_loop_break(x) {
                        return None;
                    } else if Some(x) == early_continue {
                        // Exact backward loop-continue trampoline: a case early-exit, not JOIN.
                    } else if is_forward_loop_join(x) {
                        // Ordinary switch break immediately before the loop increment/test.
                        join_cands.push(x);
                    } else if self.is_bare_ret_off(x) {
                        // batch-43 (Fix 2): a struct-RVO switch's per-case exit ALSO jumps to a
                        // bare RET (GetDebugColor: every case writes `__return = FColor::X` then
                        // `JMP -> RET`). Allow it under `rvo_return_mode` — the per-region
                        // out-slot-write proof (G-RVO) gates the actual recovery below; here we
                        // only let the join be recognized so validation can proceed.
                        if !register_based && !rvo_return_mode {
                            return None;
                        }
                        ret_rows.push(x);
                    } else if x >= t_last {
                        join_cands.push(x);
                    } else {
                        return None; // cross/backward jump — not a shared exit
                    }
                }
                "RET" => {}
                name if is_cond_op(name) => return None,
                // falls into the next boundary: legal into the JOIN, or (in a loop) into the
                // loop-continue point which IS `stop` == the next boundary past the switch.
                _ => fall_pend.push(w[1]),
            }
        }
        join_cands.sort_unstable();
        join_cands.dedup();
        ret_rows.sort_unstable();
        ret_rows.dedup();
        // batch-36 (Stage B): inside a loop, the natural JOIN of a switch whose non-returning
        // regions fall through / break to the loop-continue is that continue offset (== `stop`,
        // the loop increment/latch). Prefer it so the returning cases stay `return`s and the DEF
        // falls into the increment as a `break;`. Only when NO explicit forward join exists.
        let loop_join = lscope.and_then(|ls| {
            let cont_idx = self.idx_of.get(&ls.continue_off).copied();
            if ls.continue_off >= b0.start_dw
                && cont_idx == Some(stop)
                && join_cands.iter().all(|&j| is_forward_loop_join(j))
            {
                Some(ls.continue_off)
            } else {
                None
            }
        });
        let join_off = match (join_cands.as_slice(), ret_rows.as_slice()) {
            _ if loop_join.is_some() && join_cands.iter().all(|&j| is_forward_loop_join(j)) => {
                loop_join.unwrap()
            }
            ([j], _) => *j,
            // every region returns: the shared bare RET row is the join/epilogue
            ([], [r]) => *r,
            _ => return None,
        };
        if fall_pend.iter().any(|&nb| nb != join_off) {
            return None;
        }
        if join_off < t_last || targets.contains(&join_off) {
            return None;
        }
        let join_is_ret = self.is_bare_ret_off(join_off);
        // batch-43 (Fix 2): the RVO-return recovery fires ONLY for the true out-slot shape — a
        // struct-by-value switch whose per-case regions write `__return` and JMP to the shared
        // BARE RET row (GetDebugColor). A struct-RVO switch whose join is a REAL block (cases
        // write a normal local `local_N` then `break;`, and the function returns `local_N` AFTER
        // the switch — GetSFXTagByLevel) recovers through the NORMAL `break;`-to-join path and
        // must NOT enter RVO-return mode (its cases write no `__return`, so the G-RVO proof would
        // wrongly bail the whole switch to a stub). Gate the proof + fold on `join_is_ret`.
        let rvo_ret_switch = rvo_return_mode && join_is_ret;
        if join_is_ret && !register_based && !rvo_ret_switch {
            return None; // per-case RVO-struct returns are not recoverable from the register
        }
        let join_idx = self.idx_of.get(&join_off).copied()?;
        // the construct may extend past `stop` only through its JOIN (e.g. the switch is
        // the tail of an if-branch and its breaks target the post-if continuation)
        let switch_end = join_idx.min(stop);
        if switch_end <= first_region_idx {
            return None;
        }
        // ---- regions: enumerate + validate (any anomaly bails BEFORE any emission) ----
        struct Region {
            off: usize,
            start: usize,
            end: usize,
            is_def: bool,
            trap: bool,
            append_break: bool,
        }
        let mut regions: Vec<Region> = Vec::new();
        let mut has_mixed_rvo_ret_rows = false;
        for (k, &b) in bounds.iter().enumerate() {
            if b == join_off {
                continue; // DEF == JOIN: no default clause, nothing to emit
            }
            let start = self.idx_of.get(&b).copied()?;
            let end = match bounds.get(k + 1) {
                Some(nb) => self.idx_of.get(nb).copied()?.min(switch_end),
                None => switch_end,
            };
            if start >= end || start < first_region_idx || end > switch_end {
                return None;
            }
            let end_off = blocks[end].start_dw;
            let mut trap_ops = true;
            let mut has_cond_join = false;
            for bi2 in start..end {
                let bb = &blocks[bi2];
                let tname = ctx.instrs[bb.instr_hi - 1].op.name;
                for k2 in bb.instr_lo..bb.instr_hi {
                    if !matches!(
                        ctx.instrs[k2].op.name,
                        "ThrowException" | "SUSPEND" | "JitEntry"
                    ) {
                        trap_ops = false;
                    }
                }
                let last_of_region = bi2 + 1 == end;
                let uncond = tname == "JMP";
                // A normal-JOIN switch in an RVO-returning function may still return early from
                // individual case paths. Prove each such `JMP -> bare RET` from the terminating
                // block's own clean out-slot store; an unproved edge falls through to the
                // existing escape rejection below and atomically keeps the function stubbed.
                let mixed_rvo_ret_exit = uncond
                    && rvo_return_mode
                    && !rvo_ret_switch
                    && bb.succs.first().copied().is_some_and(|s| {
                        self.is_bare_ret_off(s) && self.proves_mixed_rvo_exit(bi2, s)
                    });
                if mixed_rvo_ret_exit {
                    has_mixed_rvo_ret_rows = true;
                }
                for &s in &bb.succs {
                    if s >= b && s < end_off {
                        continue; // in-region (incl. internal loops' back edges)
                    }
                    if is_loop_break(s) {
                        return None;
                    }
                    if uncond && Some(s) == early_continue {
                        continue; // exact proven enclosing-loop `continue;`
                    }
                    if s == join_off {
                        if uncond || (last_of_region && end == join_idx) {
                            continue; // break/return exit, or the last region falling into JOIN
                        }
                        // conditional TAKEN edge to the JOIN (`if (x) <skip rest of case>`):
                        // the is_cond arm renders it as the inverted `if (!x) { rest }`,
                        // which is correct ONLY when the region abuts the JOIN so both
                        // paths hit the appended `break;` (never in RETURN mode — the
                        // skipped path's register value would be unrecoverable).
                        if is_cond_op(tname)
                            && !join_is_ret
                            && end == join_idx
                            && bb.succs.first() == Some(&s)
                        {
                            has_cond_join = true;
                            continue;
                        }
                    }
                    if uncond
                        && (register_based || rvo_ret_switch || mixed_rvo_ret_exit)
                        && self.is_bare_ret_off(s)
                    {
                        continue; // per-case `return` exit (register value or RVO out-slot)
                    }
                    return None; // escapes the region (incl. cond jumps to an exit)
                }
                // a return-exit must have a recoverable value INSIDE this region. For a
                // register-based return that value is a `CpyVtoR*`/`LOADOBJ` scan-back; for a
                // struct-RVO return (rvo_ret_switch) the value travels through `__return` and is
                // proven separately by G-RVO (the scratch-emit proof below), so the register
                // scan-back is DELIBERATELY not required here (it is always empty for an RVO struct).
                if uncond && non_void && !rvo_ret_switch {
                    if let Some(&s) = bb.succs.first() {
                        let ret_exit = (s == join_off && join_is_ret)
                            || (s != join_off
                                && !(s >= b && s < end_off)
                                && self.is_bare_ret_off(s));
                        if ret_exit
                            && !mixed_rvo_ret_exit
                            && scan_back_retval_floor(ctx, bb.instr_hi - 1, blocks[start].instr_lo)
                                .is_none()
                        {
                            return None;
                        }
                    }
                }
            }
            // region's last block must actually LEAVE the construct. `has_cond_join`
            // means SOME path skips to the appended `break;` after the region body, so
            // one must be appended even when the body's own last statement already exits.
            let lb = &blocks[end - 1];
            let lt = ctx.instrs[lb.instr_hi - 1].op.name;
            let append_break;
            if lt == "JMP" {
                let x = lb.succs.first().copied()?;
                if is_loop_break(x) {
                    return None;
                }
                let mixed_rvo_ret_exit = rvo_return_mode
                    && !rvo_ret_switch
                    && self.is_bare_ret_off(x)
                    && self.proves_mixed_rvo_exit(end - 1, x);
                let exits = Some(x) == early_continue
                    || x == join_off
                    || ((register_based || mixed_rvo_ret_exit)
                        && !(x >= b && x < end_off)
                        && self.is_bare_ret_off(x));
                if !exits {
                    return None;
                }
                append_break = has_cond_join; // the exit hook renders break;/return at the JMP
            } else if lt == "RET" {
                append_break = has_cond_join;
            } else if is_cond_op(lt) {
                return None;
            } else {
                // plain fallthrough: only into a physically adjacent JOIN
                if end != join_idx {
                    return None;
                }
                append_break = true;
            }
            let is_def = b == def_off;
            let trap = trap_ops && end - start == 1;
            if trap && (!is_def || targets.contains(&b)) {
                return None; // a case entry can never be the compiler trap
            }
            regions.push(Region {
                off: b,
                start,
                end,
                is_def,
                trap,
                append_break,
            });
        }
        // no OUTSIDE block may enter the construct anywhere but its head
        let span_lo = b0.start_dw;
        let span_hi = blocks[switch_end].start_dw;
        for (bi2, bb) in blocks.iter().enumerate() {
            if bi2 >= i && bi2 < switch_end {
                continue;
            }
            for &s in &bb.succs {
                if s > span_lo && s < span_hi {
                    return None;
                }
            }
        }
        // batch-43 (Fix 2, gate G-RVO): a struct-RVO-return switch may recover ONLY when EVERY
        // non-trap region PROVABLY writes the `__return` out-slot before its exit. Prove it by a
        // non-emitting scratch pass: emit each region with `exit_rvo_return` set, then require
        // `fold_rvo_return` to succeed (its `__return = <val>;` + `return __return;` fold IS the
        // proof — a resolved out-slot store). ANY region without such a store bails the WHOLE
        // switch to the stub (never emit a case that silently returns a default struct). This is
        // the LAST bail point; the real emission below is byte-for-byte identical, so the proof
        // and the emit agree. Runs only for the bare-RET-join RVO shape (`rvo_ret_switch`).
        if rvo_ret_switch {
            let saved = (
                self.exit_join,
                self.exit_join_is_ret,
                self.exit_ret_rows_ok,
                self.exit_rvo_return,
                self.exit_mixed_rvo_ret_rows_ok,
                self.exit_scan_floor,
            );
            let mut all_ok = true;
            ctx.rvo_switch_region.set(true); // scope the `$beh0(__return,src)` statement-emit
            for r in &regions {
                if r.trap {
                    continue;
                }
                let mut scratch = String::new();
                self.exit_join = Some(join_off);
                self.exit_join_is_ret = join_is_ret;
                self.exit_ret_rows_ok = register_based;
                self.exit_rvo_return = true;
                self.exit_mixed_rvo_ret_rows_ok = false;
                self.exit_scan_floor = blocks[r.start].instr_lo;
                self.emit_range(r.start, r.end, depth + 1, &mut scratch);
                (
                    self.exit_join,
                    self.exit_join_is_ret,
                    self.exit_ret_rows_ok,
                    self.exit_rvo_return,
                    self.exit_mixed_rvo_ret_rows_ok,
                    self.exit_scan_floor,
                ) = saved;
                // a region that FALLS THROUGH into the join (the default, whose destructor +
                // shared RET follow) still writes `__return` inside its body but ends without the
                // synthetic `return __return;` — accept it iff its body carries a resolved
                // `__return = <val>;` store; the emission handles its fallthrough return normally.
                let has_synth_return = scratch.lines().any(|l| l.trim() == "return __return;");
                let ok = if has_synth_return {
                    fold_rvo_return(&scratch).is_some()
                } else {
                    // fallthrough region (last, into the epilogue): require a resolved out-slot store
                    scratch
                        .lines()
                        .rev()
                        .find_map(|l| {
                            let t = l.trim();
                            t.strip_prefix("__return = ")
                                .and_then(|s| s.strip_suffix(';'))
                        })
                        .is_some_and(|val| {
                            !val.is_empty()
                                && !val.contains('\u{1}')
                                && !val.contains('\u{2}')
                                && val != UNRESOLVED
                                && !val.contains(RVODEF)
                        })
                };
                if !ok {
                    all_ok = false;
                    break;
                }
            }
            ctx.rvo_switch_region.set(false);
            if !all_ok {
                return None;
            }
        }
        // ---- emission (validated: no bail past this point) ----
        let ind = "    ".repeat(depth);
        let (stmts, _) = block_stmts(ctx, b0.instr_lo, b0.instr_hi);
        for s in &stmts {
            let _ = writeln!(out, "{ind}{s}");
        }
        let sel_raw = ctx.slot_name(wv);
        // an enum selector needs the explicit int() (mirrors the CMPIi arm: AS has no
        // implicit enum<->int, and the case labels are int literals)
        let sel = if ctx
            .slot_type(wv)
            .as_deref()
            .map(is_enum_name)
            .unwrap_or(false)
        {
            format!("int({sel_raw})")
        } else {
            sel_raw
        };
        let _ = writeln!(out, "{ind}switch ({sel})");
        let _ = writeln!(out, "{ind}{{");
        let saved = (
            self.exit_join,
            self.exit_join_is_ret,
            self.exit_ret_rows_ok,
            self.exit_rvo_return,
            self.exit_mixed_rvo_ret_rows_ok,
            self.exit_scan_floor,
        );
        for r in &regions {
            if r.trap {
                continue; // trap DEF = source had NO default; recompiling regenerates it
            }
            for (k, &tg) in targets.iter().enumerate() {
                if tg == r.off {
                    let _ = writeln!(out, "{ind}case {}:", lo_c + k as i32);
                }
            }
            if r.is_def {
                let _ = writeln!(out, "{ind}default:");
            }
            // batch-24d: the in-game compiler rejects "Variables cannot be declared in
            // switch cases, except inside statement blocks". Declarations are INTRODUCED
            // into case bodies only later, by the emit-side decl-init rewrites (auto
            // iterator decls, executor decl-inits, __na temp-splits) — the structurer
            // cannot know which cases will end up carrying one, so brace EVERY case body
            // unconditionally (a statement block is always-legal AS; stacked case/default
            // labels stay outside the braces, the body and its trailing break;/return
            // exits go inside).
            let _ = writeln!(out, "{ind}{{");
            self.exit_join = Some(join_off);
            self.exit_join_is_ret = join_is_ret;
            self.exit_ret_rows_ok = register_based;
            self.exit_rvo_return = rvo_ret_switch;
            self.exit_mixed_rvo_ret_rows_ok = has_mixed_rvo_ret_rows;
            self.exit_scan_floor = blocks[r.start].instr_lo;
            if rvo_ret_switch {
                // batch-43 (Fix 2): emit to scratch, then fold the region's trailing
                // `__return = <val>;` + `return __return;` into `return <val>;` (G-RVO already
                // proved every region has such a store, so the fold cannot silently drop a
                // value). A fallthrough region (default, whose body writes `__return` but exits
                // by falling into the shared epilogue) has no synthetic `return __return;` — its
                // scratch has no fold site, so it is written verbatim and the trailing epilogue's
                // shared `return __return;` (see below) covers it.
                let mut scratch = String::new();
                ctx.rvo_switch_region.set(true); // scope the `$beh0(__return,src)` statement-emit
                self.emit_range(r.start, r.end, depth + 1, &mut scratch);
                ctx.rvo_switch_region.set(false);
                let folded = fold_rvo_return(&scratch).unwrap_or(scratch);
                out.push_str(&folded);
            } else {
                self.emit_range(r.start, r.end, depth + 1, out);
            }
            (
                self.exit_join,
                self.exit_join_is_ret,
                self.exit_ret_rows_ok,
                self.exit_rvo_return,
                self.exit_mixed_rvo_ret_rows_ok,
                self.exit_scan_floor,
            ) = saved;
            if r.append_break {
                let _ = writeln!(out, "{ind}    break;");
            }
            let _ = writeln!(out, "{ind}}}");
        }
        let _ = writeln!(out, "{ind}}}");
        // batch-30b (C9 'Unreachable code', specs/batch29-errortail.md §9): when the JOIN is
        // the shared bare RET row, a REAL `default:` region was emitted, and every region
        // leaves by RETURNING (terminator RET, or JMP rendered as `return ...;` by the exit
        // hook — never an appended `break;`), control cannot fall out of the switch. Emitting
        // the RET row after it is dead code the compiler flags ("Unreachable code" [W],
        // a module-killer under warnings-as-errors) — skip the row. Conservative gates:
        // a trap DEF (no `default:` emitted) keeps the row (a non-matching selector falls
        // through in the recompiled source), as does any external jump to the row (another
        // path may rely on its emission).
        if join_is_ret && switch_end == join_idx && join_idx < stop {
            let every_region_returns = regions.iter().all(|r| {
                !r.trap
                    && !r.append_break
                    && matches!(
                        ctx.instrs[blocks[r.end - 1].instr_hi - 1].op.name,
                        "JMP" | "RET"
                    )
            });
            let has_real_default = regions.iter().any(|r| r.is_def && !r.trap);
            let externally_referenced = blocks
                .iter()
                .enumerate()
                .any(|(bi2, bb)| (bi2 < i || bi2 >= switch_end) && bb.succs.contains(&join_off));
            if every_region_returns && has_real_default && !externally_referenced {
                return Some(join_idx + 1);
            }
        }
        Some(switch_end)
    }

    fn is_cond(&self, bi: usize) -> bool {
        matches!(
            self.jump_op(bi),
            "JZ" | "JNZ" | "JS" | "JNS" | "JP" | "JNP" | "JLowZ" | "JLowNZ"
        ) && self.g.blocks[bi].succs.len() == 2
            // forward only (backward = loop latch, handled elsewhere)
            && self.g.blocks[bi].succs.iter().all(|&s| s > self.g.blocks[bi].start_dw)
    }

    /// batch-27 (Cast-diamond carry, design `specs/batch26-castdiamond.md` §2.3): decide
    /// whether cond block `i` — whose `block_stmts_in` left `l` on the operand stack — is the
    /// GUARD of exactly the null-check Cast diamond
    /// `if (x != nullptr) { y = Cast<T>(x); } else { <housekeeping> }`, so `l` may be carried
    /// into the JOIN block's initial stack. Returns the join's block index, or None on ANY
    /// deviation (bail-by-default: a false negative costs nothing — status-quo drop).
    ///
    /// Stage 1 gates (D1-D11): JZ-on-CmpPtrNull guard; then-arm = exactly one block
    /// whitelisted to the lowered `Cast<T>` shape (TYPEID/PSF/PshVPtr/CALLSYS-opCast/JMP);
    /// else-arm absent or one housekeeping block; join sole-entry and == the `next` the
    /// is_cond arm just computed; every carried entry pushed by a plain push opcode
    /// (`carryable`); dual simulation proves the arms' emission + net stack effect are
    /// independent of the carried entries; the join's first call is a CALLSYS/Thiscall1 with
    /// TRUSTED arity (split path, never take-all). Stage 2 (own harness batch) relaxes the
    /// consumer gate to CALL/CALLINTF script consumers.
    fn diamond_join(
        &self,
        i: usize,
        next: Option<usize>,
        l: &[Arg],
        cmp: &Option<Cmp>,
    ) -> Option<usize> {
        let ctx = self.ctx;
        let blocks = &self.g.blocks;
        let b = &blocks[i];
        // D9 carryability: every carried entry originates from a plain slot/const/global push
        // (never a pending-call-result PshRPtr — carrying one would reorder side effects).
        if !l.iter().all(|a| a.carryable) {
            return None;
        }
        // D1 guard shape: `JZ` over a bare `CmpPtrNull` (`x == nullptr`, no T*-op, no expr)
        // — the classic Cast diamond. batch-33d: `JLowZ` (bool-register test, e.g. an
        // `IsValid(...)`-guarded assignment diamond) is also accepted; such guards can never
        // be the classic Cast shape, so they always take the RELAXED arm rules below.
        let jop = self.jump_op(i);
        if jop != "JZ" && jop != "JLowZ" {
            return None;
        }
        let c = cmp.as_ref()?;
        if jop == "JZ" && (c.b != "nullptr" || c.op.is_some() || c.expr.is_some()) {
            return None;
        }
        // D2 then-arm entry: the fallthrough successor is the physically-next block.
        let fall = *b.succs.get(1)?;
        if self.idx_of.get(&fall).copied() != Some(i + 1) {
            return None;
        }
        // D3 then-arm exit: a single block, JMP-terminated, sole successor = the join.
        let t = blocks.get(i + 1)?;
        if ctx.instrs[t.instr_hi - 1].op.name != "JMP" || t.succs.len() != 1 {
            return None;
        }
        let join_off = t.succs[0];
        // D4 else-arm: absent (guard's taken edge goes straight to the join), or exactly one
        // non-cond block (fallthrough or JMP) whose sole successor is the join.
        let taken = *b.succs.first()?;
        let else_idx = if taken == join_off {
            None
        } else {
            if self.idx_of.get(&taken).copied() != Some(i + 2) {
                return None;
            }
            let e = blocks.get(i + 2)?;
            if is_cond_op(ctx.instrs[e.instr_hi - 1].op.name) || e.succs.as_slice() != [join_off] {
                return None;
            }
            Some(i + 2)
        };
        // D5 join index: must be exactly the `next` the is_cond arm just emitted (creation and
        // consumption are then adjacent iterations of the same emit_range loop, or — via the
        // sole-entry proof below — the next emission of block j in an enclosing range).
        // batch-33e: `next` is None when called from emit_linear (loop bodies) — there the
        // caller checks the join lies inside the linear range instead; every other proof
        // (topology D2-D4/D6, dual-sim D10, consumer D11/D12) is graph/simulation-based and
        // emission-mode-independent.
        let j = *self.idx_of.get(&join_off)?;
        if let Some(nx) = next {
            if j != nx {
                return None;
            }
        }
        // D6 sole-entry edges (precedent: try_emit_switch's outside-entry scan): the only edge
        // into the then-arm (and else-arm, if present) comes from the guard; the only edges
        // into the join come from the diamond's arm set.
        let then_off = t.start_dw;
        let else_off = else_idx.map(|e| blocks[e].start_dw);
        for (bi, bb) in blocks.iter().enumerate() {
            for &s in &bb.succs {
                if s == then_off && bi != i {
                    return None;
                }
                if Some(s) == else_off && bi != i {
                    return None;
                }
                if s == join_off {
                    let legal = match else_idx {
                        None => bi == i || bi == i + 1,
                        Some(e) => bi == i + 1 || bi == e,
                    };
                    if !legal {
                        return None;
                    }
                }
            }
        }
        // D7 then-arm content whitelist (stage-1 belt): the CLASSIC arm is exactly the
        // lowered `Cast<T>` shape — one CALLSYS resolving to `opCast` plus one TYPEID.
        // batch-33d: any other content no longer bails outright but falls to the RELAXED
        // guarded-assignment rules below (SayVoicelineWithContext lost its Context/Loudness
        // args and BroadcastPerceptionEventInRadius its Affected arg to the old bail: the
        // guard is an `IsValid(...)`/null test and the then-arm a `local = <calls>;`
        // assignment, not a Cast).
        let (mut ncast, mut ntypeid) = (0usize, 0usize);
        let mut classic = jop == "JZ";
        for k in t.instr_lo..t.instr_hi {
            let ins = &ctx.instrs[k];
            match ins.op.name {
                "SUSPEND" | "JitEntry" | "PSF" | "PshVPtr" | "JMP" => {}
                "TYPEID" => ntypeid += 1,
                "CALLSYS" => {
                    let ptr = ins.qwords.first().copied().unwrap_or(0) as i64;
                    if ctx.refs.func_by_ptr(ptr) != Some("opCast") {
                        classic = false;
                    } else {
                        ncast += 1;
                    }
                }
                _ => classic = false,
            }
        }
        if classic && (ncast != 1 || ntypeid != 1) {
            classic = false;
        }
        if !classic {
            // batch-33d RELAXED guarded-assignment diamond. The D10 dual-sim below remains
            // the authoritative stack-semantics gate (arms provably net-zero, emission
            // unchanged, carry returned verbatim); these rules close the REORDERING hole
            // the classic D7 whitelist closed structurally — the carried renders are
            // re-evaluated at the join, AFTER the arm's statements run:
            //  (R1) every carried entry renders as a bare slot/param identifier or an
            //       integer constant — no member chains (an arm call could mutate the
            //       object behind `this.m_X`), no container reads (`reeval`, see Arg).
            //  (R2) both arms contain only whitelisted ops, and every slot-writing op's
            //       destination differs from every carried identifier — so the value a
            //       carried name renders to at the join equals the value the bytecode
            //       pushed before the guard.
            let bare_ident = |s: &str| {
                let b = s.as_bytes();
                !s.is_empty()
                    && (b[0].is_ascii_alphabetic() || b[0] == b'_')
                    && b.iter().all(|c| c.is_ascii_alphanumeric() || *c == b'_')
            };
            let int_const = |a: &Arg| {
                a.cbits.is_some()
                    || (!a.s.is_empty()
                        && a.s
                            .trim_start_matches('-')
                            .bytes()
                            .all(|b| b.is_ascii_digit()))
            };
            for a in l {
                if a.reeval || !(bare_ident(&a.s) || int_const(a)) {
                    return None;
                }
            }
            let carried: Vec<&str> = l.iter().map(|a| a.s.as_str()).collect();
            for arm in std::iter::once(i + 1).chain(else_idx) {
                let ab = &blocks[arm];
                for k in ab.instr_lo..ab.instr_hi {
                    let ins = &ctx.instrs[k];
                    match ins.op.name {
                        // non-writing ops: pushes, member-address/deref, register loads,
                        // calls (their arg consumption is proven balanced by D10).
                        "SUSPEND" | "JitEntry" | "JMP" | "PSF" | "PshVPtr" | "PshV4" | "PshV8"
                        | "PshC4" | "PshC8" | "PshNull" | "PGA" | "PshGPtr" | "ADDSi"
                        | "RDSPtr" | "TYPEID" | "CALL" | "CALLINTF" | "CALLBND" | "CALLSYS"
                        | "Thiscall1" | "LoadThisR" | "LoadVObjR" | "LoadRObjR" => {}
                        // slot-writing ops: legal unless the destination is carried.
                        "STOREOBJ" | "FreeNullV8" | "ClrVPtr" | "FREE" | "RDR1" | "RDR2"
                        | "RDR4" | "RDR8" | "CpyRtoV4" | "CpyRtoV8" | "SetV1" | "SetV4"
                        | "SetV8" | "CpyVtoV4" | "CpyVtoV8" => {
                            let dst = ctx.slot_name(s16(ins.words.first().copied().unwrap_or(0)));
                            if carried.iter().any(|c| *c == dst) {
                                return None;
                            }
                        }
                        // anything else (register-indirect writes WRTV*/PopRPtr, arithmetic,
                        // control flow) -> bail: destination untrackable.
                        _ => return None,
                    }
                }
            }
        }
        // D8 else-arm content whitelist: housekeeping only.
        if let Some(e) = else_idx {
            let eb = &blocks[e];
            for k in eb.instr_lo..eb.instr_hi {
                if !matches!(
                    ctx.instrs[k].op.name,
                    "SUSPEND" | "JitEntry" | "ClrVPtr" | "FREE" | "FreeNullV8" | "JMP"
                ) {
                    return None;
                }
            }
        }
        // D10 dual-simulation safety gate (authoritative — subsumes D7/D8 semantically, both
        // kept for belt-and-suspenders): each arm must be stack-net-zero on its own, emit the
        // IDENTICAL statements with and without the carried entries, and return the carried
        // entries verbatim. Then on BOTH runtime paths the stack at the join equals `l`, and
        // the arm's actual emission (done with an empty init) is unchanged.
        for arm in std::iter::once(i + 1).chain(else_idx) {
            let ab = &blocks[arm];
            let (s0, _, r0) = block_stmts_in(ctx, ab.instr_lo, ab.instr_hi, Vec::new(), false);
            let (s1, _, r1) = block_stmts_in(ctx, ab.instr_lo, ab.instr_hi, l.to_vec(), false);
            if !r0.is_empty() || s1 != s0 || r1.as_slice() != l {
                return None;
            }
        }
        // D11 consumer gate: the join's FIRST call-class instruction must have a TRUSTED
        // arity, so the split path, not take-all, consumes the carry.
        //   - stage 1 (batch-27): CALLSYS/Thiscall1 — Binds native arity, or the cache
        //     FunctionReference param count (the same fallback the CALLSYS arm's split uses).
        //   - stage 2 (batch-31a, spec batch31-nomatch-illegalop §1.1 N2a): CALL/CALLINTF/
        //     CALLBND — script functions always carry a FunctionReference param list, so the
        //     by-id count has the same trust level as the CALLSYS by-ptr fallback (it is what
        //     the CALL arm's EDIT B-PRIME split already uses). Note the join's first call need
        //     NOT itself be the carry's consumer (Proof D — ANotifySpellCategoryActor::
        //     OnBeginOverlap: the carried entry sits at the stack BOTTOM and a LATER CALLSYS
        //     in the same join consumes it); trusting the first call's split arithmetic is
        //     what keeps the carried entry in place for that later consumer.
        //   - CallPtr keeps the bail (no id, no trusted arity). No call at all -> bail
        //     (no proven consumer).
        let jb = &blocks[j];
        let mut consumer = false;
        for k in jb.instr_lo..jb.instr_hi {
            let ins = &ctx.instrs[k];
            match ins.op.name {
                "CALLSYS" | "Thiscall1" => {
                    let ptr = ins.qwords.first().copied().unwrap_or(0) as i64;
                    let f = ctx.refs.func_by_ptr(ptr).unwrap_or("");
                    let na = ctx.refs.native_arity_by_ptr(ptr, f);
                    if na
                        .or_else(|| ctx.refs.func_params_by_ptr(ptr).map(|p| p.len()))
                        .is_none()
                    {
                        return None;
                    }
                    consumer = true;
                    break;
                }
                "CALL" | "CALLINTF" | "CALLBND" => {
                    let id = ins.dwords.first().copied().unwrap_or(0) as i32;
                    if ctx.refs.func_params_by_id(id).is_none() {
                        return None;
                    }
                    consumer = true;
                    break;
                }
                "CallPtr" => return None,
                _ => {}
            }
        }
        if !consumer {
            return None;
        }
        // D12 (batch-27 addition beyond the spec, raw-gate belt): consumer simulation.
        // Pre-run the JOIN block with and without the carried entries (pure function — the
        // real emission is exactly the with-init run). Bail when carrying
        //   (a) introduces an ARGMISMATCH sentinel the empty-init run does not have (the
        //       carried window is misaligned for this consumer — e.g. the big-operand-window
        //       MagicScript::GetSingleActorTargetFromCamera* family — and emitting it would
        //       force-stub the WHOLE function), or
        //   (b) changes the join's statement COUNT (the carry must only let existing
        //       statements gain args, never create/destroy statements — a spurious
        //       consumption by a non-call opcode, or a statement dropped as unresolved).
        // False negatives cost nothing: status-quo drop-at-boundary.
        // batch-31a note: with stage-2 CALL/CALLINTF consumers the WITH-init run may now
        // RESOLVE a sentinel the empty-init run HAS (a previously-missing arg recovered by
        // the carry) — that direction stays legal; only a NEW sentinel bails.
        let has_amm = |v: &[String]| v.iter().any(|s| s.contains('\u{2}'));
        let (j0, _, _) = block_stmts_in(ctx, jb.instr_lo, jb.instr_hi, Vec::new(), false);
        let (j1, _, _) = block_stmts_in(ctx, jb.instr_lo, jb.instr_hi, l.to_vec(), false);
        if j1.len() != j0.len() || (has_amm(&j1) && !has_amm(&j0)) {
            return None;
        }
        Some(j)
    }

    fn header_slot_value(&self, state: &HeaderState, slot: i32) -> HeaderValue {
        state.slots.get(&slot).cloned().unwrap_or_else(|| {
            let ty = self.ctx.slot_type(slot);
            let boolish = ty.as_deref() == Some("bool");
            HeaderValue {
                expr: HeaderExpr::Atom(self.ctx.slot_name(slot)),
                boolish,
                full_bool: false,
                ty,
            }
        })
    }

    fn header_truth(&self, value: HeaderValue) -> Option<HeaderExpr> {
        header_truth_expr(value)
    }

    fn header_relation(&self, state: &HeaderState, rel: HeaderRel) -> Option<HeaderExpr> {
        let (a, b) = state.cmp.clone()?;
        Some(HeaderExpr::Cmp(Box::new(a), rel, Box::new(b)))
    }

    fn header_taken_condition(&self, state: &HeaderState, jump: &str) -> Option<HeaderExpr> {
        if matches!(jump, "JLowZ" | "JLowNZ") {
            return header_bool_jump_taken(state.value_reg.clone()?, jump);
        }
        let rel = match jump {
            "JZ" => HeaderRel::Eq,
            "JNZ" => HeaderRel::Ne,
            "JS" => HeaderRel::Lt,
            "JNS" => HeaderRel::Ge,
            "JP" => HeaderRel::Gt,
            "JNP" => HeaderRel::Le,
            _ => return None,
        };
        self.header_relation(state, rel)
            .or_else(|| header_bool_jump_taken(state.value_reg.clone()?, jump))
    }

    /// Symbolically execute one compiler-materialized predicate block. The whitelist contains
    /// only local/register copies, constants, numeric casts and comparisons. Any call, member
    /// access, write, allocation, suspension or unmodelled opcode rejects the entire loop proof.
    fn exec_header_block(&self, bi: usize, state: &mut HeaderState) -> Option<()> {
        let b = &self.g.blocks[bi];
        for k in b.instr_lo..b.instr_hi {
            let ins = &self.ctx.instrs[k];
            let n = ins.op.name;
            if k + 1 == b.instr_hi && (n == "JMP" || is_cond_op(n)) {
                continue;
            }
            match n {
                "SetV1" | "SetV2" | "SetV4" => {
                    let slot = ins.words.first().copied().map(s16)?;
                    let ty = header_set_slot_type(
                        self.ctx.slot_type(slot),
                        self.ctx.float_slots.contains(&slot),
                        n,
                    );
                    let value = header_set_value(n, *ins.dwords.first()? as u64, ty)?;
                    state.slots.insert(slot, value);
                }
                "SetV8" => {
                    let slot = ins.words.first().copied().map(s16)?;
                    let ty = header_set_slot_type(
                        self.ctx.slot_type(slot),
                        self.ctx.float_slots.contains(&slot),
                        n,
                    );
                    let value = header_set_value(n, *ins.qwords.first()?, ty)?;
                    state.slots.insert(slot, value);
                }
                "CpyVtoV4" | "CpyVtoV8" => {
                    let dst = ins.words.first().copied().map(s16)?;
                    let src = ins.words.get(1).copied().map(s16)?;
                    let value = header_copy_value(
                        self.header_slot_value(state, src),
                        self.ctx.slot_type(dst),
                        if n == "CpyVtoV4" { 32 } else { 64 },
                    )?;
                    state.slots.insert(dst, value);
                }
                "CpyRtoV4" | "CpyRtoV8" => {
                    let dst = ins.words.first().copied().map(s16)?;
                    let value = header_copy_value(
                        state.value_reg.clone()?,
                        self.ctx.slot_type(dst),
                        if n == "CpyRtoV4" { 32 } else { 64 },
                    )?;
                    state.slots.insert(dst, value);
                }
                "CpyVtoR1" | "CpyVtoR4" | "CpyVtoR8" => {
                    let src = ins.words.first().copied().map(s16)?;
                    let mut value = self.header_slot_value(state, src);
                    if n == "CpyVtoR1" {
                        value.full_bool = false;
                    }
                    state.value_reg = Some(value);
                    state.cmp = None;
                }
                "CMPIi" | "CMPIu" => {
                    let src = ins.words.first().copied().map(s16)?;
                    let value = self.header_slot_value(state, src);
                    if !header_is_integral_type(value.ty.as_deref()) {
                        return None;
                    }
                    let (target, rhs) = if n == "CMPIu" {
                        (
                            "uint",
                            HeaderExpr::Cast(
                                "uint",
                                Box::new(HeaderExpr::UInt(*ins.dwords.first()? as u64)),
                            ),
                        )
                    } else {
                        ("int", HeaderExpr::Int(*ins.dwords.first()? as i32 as i64))
                    };
                    let lhs = if value.ty.as_deref() == Some(target) {
                        value.expr
                    } else {
                        HeaderExpr::Cast(target, Box::new(value.expr))
                    };
                    state.cmp = Some((lhs, rhs));
                    state.value_reg = None;
                }
                "CMPi" | "CMPu" | "CMPi64" | "CMPu64" => {
                    let a = ins.words.first().copied().map(s16)?;
                    let b = ins.words.get(1).copied().map(s16)?;
                    let a = self.header_slot_value(state, a);
                    let b = self.header_slot_value(state, b);
                    if !header_is_integral_type(a.ty.as_deref())
                        || !header_is_integral_type(b.ty.as_deref())
                    {
                        return None;
                    }
                    let target = match n {
                        "CMPi" => "int",
                        "CMPu" => "uint",
                        "CMPi64" => "int64",
                        _ => "uint64",
                    };
                    let coerce = |v: HeaderValue| {
                        if v.ty.as_deref() == Some(target) {
                            v.expr
                        } else {
                            HeaderExpr::Cast(target, Box::new(v.expr))
                        }
                    };
                    state.cmp = Some((coerce(a), coerce(b)));
                    state.value_reg = None;
                }
                "TZ" | "TNZ" | "TS" | "TNS" | "TP" | "TNP" => {
                    let rel = match n {
                        "TZ" => HeaderRel::Eq,
                        "TNZ" => HeaderRel::Ne,
                        "TS" => HeaderRel::Lt,
                        "TNS" => HeaderRel::Ge,
                        "TP" => HeaderRel::Gt,
                        _ => HeaderRel::Le,
                    };
                    state.value_reg = Some(HeaderValue {
                        expr: self.header_relation(state, rel)?,
                        boolish: true,
                        full_bool: true,
                        ty: Some("bool".into()),
                    });
                    state.cmp = None;
                }
                "NOT" => {
                    let slot = ins.words.first().copied().map(s16)?;
                    let value = self.header_truth(self.header_slot_value(state, slot))?;
                    state.slots.insert(
                        slot,
                        HeaderValue {
                            expr: header_not(value),
                            boolish: true,
                            full_bool: false,
                            ty: Some("bool".into()),
                        },
                    );
                }
                n if is_numeric_cast(n) => {
                    let dst = ins.words.first().copied().map(s16)?;
                    let src = ins.words.get(1).copied().map(s16)?;
                    let source = self.header_slot_value(state, src);
                    let source = header_numeric_cast_operand(n, source)?;
                    let dst_ty = self.ctx.slot_type(dst);
                    let cast = header_numeric_cast_target(n, dst_ty.as_deref())?;
                    state.slots.insert(
                        dst,
                        HeaderValue {
                            expr: HeaderExpr::Cast(cast, Box::new(source)),
                            boolish: false,
                            full_bool: false,
                            ty: Some(cast.into()),
                        },
                    );
                }
                // An unconditional terminator was skipped above. No other instruction is safe
                // merely because it happens not to emit source text.
                _ => return None,
            }
        }
        Some(())
    }

    fn compound_header_written_slots(
        &self,
        head: usize,
        body: usize,
    ) -> Option<std::collections::BTreeSet<i32>> {
        let mut written = std::collections::BTreeSet::new();
        for bi in head..body {
            let b = &self.g.blocks[bi];
            for ins in &self.ctx.instrs[b.instr_lo..b.instr_hi] {
                for &word in &ins.words {
                    let slot = s16(word);
                    if explicit_slot_access(ins, slot).1 {
                        if slot <= 0 {
                            return None;
                        }
                        written.insert(slot);
                    }
                }
            }
        }
        Some(written)
    }

    /// Every header-written temporary that is read in the header must be defined earlier on that
    /// exact path. This prevents an elided assignment on one iteration/path from becoming a
    /// carried value consumed on the next iteration after the latch returns to `head`.
    fn compound_header_writes_are_path_local(
        &self,
        head: usize,
        body: usize,
        written: &std::collections::BTreeSet<i32>,
    ) -> bool {
        let mut work = vec![(head, std::collections::BTreeSet::new())];
        let mut paths = 0usize;
        while let Some((bi, mut assigned)) = work.pop() {
            paths += 1;
            if paths > COMPOUND_HEADER_MAX_PATHS * COMPOUND_HEADER_MAX_BLOCKS {
                return false;
            }
            let block = &self.g.blocks[bi];
            for ins in &self.ctx.instrs[block.instr_lo..block.instr_hi] {
                for &slot in written {
                    let (reads, writes) = explicit_slot_access(ins, slot);
                    if reads && !assigned.contains(&slot) {
                        return false;
                    }
                    if writes {
                        assigned.insert(slot);
                    }
                }
            }
            for &succ in &block.succs {
                let Some(next) = self.idx_of.get(&succ).copied() else {
                    return false;
                };
                if next >= head && next < body {
                    if next <= bi {
                        return false;
                    }
                    work.push((next, assigned.clone()));
                }
            }
        }
        true
    }

    /// Prove that every primitive temporary written by the elided header DAG is dead once control
    /// reaches either the loop body or loop exit. A path is safe only when the slot is overwritten
    /// before its first explicit read/escape, reaches a return without a read, or returns to the
    /// proven header (whose own read-before-write safety is checked separately).
    fn compound_header_writes_are_dead(
        &self,
        head: usize,
        body: usize,
        exit: usize,
        written: &std::collections::BTreeSet<i32>,
    ) -> bool {
        for &slot in written {
            let mut work = vec![body, exit];
            let mut seen = std::collections::HashSet::new();
            while let Some(bi) = work.pop() {
                if bi == head || !seen.insert(bi) {
                    continue;
                }
                let Some(block) = self.g.blocks.get(bi) else {
                    return false;
                };
                let mut killed = false;
                for ins in &self.ctx.instrs[block.instr_lo..block.instr_hi] {
                    let (reads, writes) = explicit_slot_access(ins, slot);
                    if reads {
                        return false;
                    }
                    if writes {
                        killed = true;
                        break;
                    }
                }
                if killed {
                    continue;
                }
                let term = self.ctx.instrs[block.instr_hi - 1].op.name;
                if block.succs.is_empty() {
                    if !matches!(term, "RET" | "ThrowException") {
                        return false;
                    }
                    continue;
                }
                for &succ in &block.succs {
                    let Some(next) = self.idx_of.get(&succ).copied() else {
                        return false;
                    };
                    work.push(next);
                }
            }
        }
        true
    }

    /// Recover the condition of a bounded, acyclic materialized header `[head, body)`. Every
    /// path must terminate at exactly `body_off` or `exit_off`; path-local values are retained so
    /// a compiler temporary assigned on both arms can feed the final low-register test.
    fn symbolic_header_condition(
        &self,
        head: usize,
        body: usize,
        body_off: usize,
        exit_off: usize,
    ) -> Option<String> {
        let mut work = vec![(head, HeaderState::default(), HeaderExpr::Bool(true))];
        let mut body_paths = Vec::new();
        let mut exit_paths = Vec::new();
        let mut path_count = 0usize;
        while let Some((bi, mut state, path)) = work.pop() {
            if bi < head || bi >= body {
                return None;
            }
            self.exec_header_block(bi, &mut state)?;
            let b = &self.g.blocks[bi];
            let term = self.jump_op(bi);
            let mut push_edge =
                |target: usize, state: HeaderState, condition: HeaderExpr| -> Option<()> {
                    path_count += 1;
                    if path_count > COMPOUND_HEADER_MAX_PATHS
                        || condition.nodes() > COMPOUND_HEADER_MAX_EXPR_NODES
                    {
                        return None;
                    }
                    if target == body_off {
                        body_paths.push(condition);
                    } else if target == exit_off {
                        exit_paths.push(condition);
                    } else {
                        let target_idx = self.idx_of.get(&target).copied()?;
                        if target_idx <= bi || target_idx < head || target_idx >= body {
                            return None;
                        }
                        work.push((target_idx, state, condition));
                    }
                    Some(())
                };
            if is_cond_op(term) {
                if b.succs.len() != 2 {
                    return None;
                }
                let taken = self.header_taken_condition(&state, term)?;
                let fall = header_not(taken.clone());
                push_edge(
                    b.succs[0],
                    state.clone(),
                    header_and(vec![path.clone(), taken]),
                )?;
                push_edge(b.succs[1], state, header_and(vec![path, fall]))?;
            } else {
                if b.succs.len() != 1 {
                    return None;
                }
                push_edge(b.succs[0], state, path)?;
            }
        }
        if body_paths.is_empty() || exit_paths.is_empty() {
            return None;
        }
        let condition = header_or(body_paths);
        if matches!(condition, HeaderExpr::Bool(_))
            || condition.nodes() > COMPOUND_HEADER_MAX_EXPR_NODES
        {
            return None;
        }
        let rendered = condition.render();
        (rendered.len() <= 1024).then_some(rendered)
    }

    /// Detect a compound, side-effect-free top-test header whose only back-edge is an exact
    /// unconditional jump to the header. Unlike a conventional latch-at-the-end loop, the
    /// back-edge may live inside a switch case and code for another case/default may be laid out
    /// after it; consequently the source loop body is `[body, exit)`, not `[body, latch)`.
    fn compound_latch_loop(
        &self,
        i: usize,
        stop: usize,
    ) -> Option<(usize, usize, String, usize, usize)> {
        if i >= stop || self.loop_latch(i, stop).is_some() || self.top_test_while(i, stop).is_some()
        {
            return None;
        }
        let header_off = self.g.blocks[i].start_dw;
        let mut latch = None;
        for bi in (i + 1)..stop {
            if self.is_backward_jump(bi)
                && self.g.blocks[bi].succs.first().copied() == Some(header_off)
                && latch.replace(bi).is_some()
            {
                return None;
            }
        }
        let latch = latch?;

        for body in (i + 2)..=(i + COMPOUND_HEADER_MAX_BLOCKS).min(latch) {
            let terminal = &self.g.blocks[body - 1];
            if !self.is_cond(body - 1) || terminal.succs.len() != 2 {
                continue;
            }
            let body_off = self.g.blocks[body].start_dw;
            let exit_off = if terminal.succs[0] == body_off {
                terminal.succs[1]
            } else if terminal.succs[1] == body_off {
                terminal.succs[0]
            } else {
                continue;
            };
            let Some(exit_idx) = self.idx_of.get(&exit_off).copied() else {
                continue;
            };
            if exit_idx <= latch || exit_idx > stop {
                continue;
            }

            // The header is a forward-only DAG with one terminal body edge and one terminal
            // exit edge. No earlier header block may escape or enter the body directly.
            let mut header_ok = true;
            for bi in i..body {
                for &s in &self.g.blocks[bi].succs {
                    let internal = self
                        .idx_of
                        .get(&s)
                        .copied()
                        .is_some_and(|si| si > bi && si >= i && si < body);
                    let terminal_edge = bi + 1 == body && (s == body_off || s == exit_off);
                    if !internal && !terminal_edge {
                        header_ok = false;
                    }
                }
            }
            if !header_ok {
                continue;
            }

            // Single-entry/dominance: nothing outside `[i, exit)` may enter its interior; the
            // only external entry is permitted at the header itself. Within the span, the sole
            // non-forward edge is the proven latch -> header back-edge.
            let mut shape_ok = true;
            for (src, block) in self.g.blocks.iter().enumerate() {
                for &s in &block.succs {
                    let Some(target) = self.idx_of.get(&s).copied() else {
                        continue;
                    };
                    if target >= i
                        && target < exit_idx
                        && !(src >= i && src < exit_idx)
                        && (target != i || src >= i)
                    {
                        shape_ok = false;
                    }
                    if src >= i
                        && src < exit_idx
                        && target >= i
                        && target < exit_idx
                        && target <= src
                        && (src != latch || target != i || self.jump_op(src) != "JMP")
                    {
                        shape_ok = false;
                    }
                }
            }
            if !shape_ok {
                continue;
            }

            // Every physical block owned by the loop must be reachable from the one header
            // entry. This is the explicit dominance/reducibility proof, including switch rows,
            // default code laid out after the back-edge, and the latch itself.
            let mut seen = std::collections::HashSet::new();
            let mut work = vec![i];
            while let Some(bi) = work.pop() {
                if !seen.insert(bi) {
                    continue;
                }
                for &s in &self.g.blocks[bi].succs {
                    if let Some(ti) = self.idx_of.get(&s).copied() {
                        if ti >= i && ti < exit_idx && !(bi == latch && ti == i) {
                            work.push(ti);
                        }
                    }
                }
            }
            if !(i..exit_idx).all(|bi| seen.contains(&bi)) {
                continue;
            }
            let Some(written) = self.compound_header_written_slots(i, body) else {
                continue;
            };
            if !self.compound_header_writes_are_path_local(i, body, &written)
                || !self.compound_header_writes_are_dead(i, body, exit_idx, &written)
            {
                continue;
            }

            let cond = self.symbolic_header_condition(i, body, body_off, exit_off)?;
            return Some((body, exit_idx, cond, header_off, exit_off));
        }
        None
    }

    fn compound_latch_loop_recoverable(
        &mut self,
        i: usize,
        stop: usize,
    ) -> Option<(usize, usize, String, usize, usize)> {
        let (body, exit, cond, continue_off, break_off) = self.compound_latch_loop(i, stop)?;
        let ls = LoopScope {
            continue_off,
            break_off,
        };
        if !self.body_has_inner_branch(body, exit, ls) {
            return None;
        }
        let saved = self.loop_scope;
        self.loop_scope = Some(ls);
        let ok = self.loop_body_recoverable(body, exit, ls);
        self.loop_scope = saved;
        ok.then_some((body, exit, cond, continue_off, break_off))
    }

    /// If block `i` begins a bottom-test loop within [.., stop), return the latch block
    /// index (the block whose conditional jump targets back to `i` or earlier in the body).
    fn loop_latch(&self, i: usize, stop: usize) -> Option<usize> {
        let header_off = self.g.blocks[i].start_dw;
        for bi in i..stop {
            let b = &self.g.blocks[bi];
            for &s in &b.succs {
                if s <= header_off && self.is_backward_cond(bi) {
                    return Some(bi);
                }
            }
        }
        None
    }

    /// Detect a top-test loop headed at block `i`:
    /// `header: <cmp> Jcc exit; body...; JMP header`. Returns (body_end_idx, condition).
    fn top_test_while(&self, i: usize, stop: usize) -> Option<(usize, String)> {
        if !self.is_cond(i) {
            return None;
        }
        let b = &self.g.blocks[i];
        let taken = *b.succs.first()?;
        let fall = *b.succs.get(1)?;
        let taken_idx = *self.idx_of.get(&taken)?;
        let fall_idx = *self.idx_of.get(&fall)?;
        if fall_idx != i + 1 || taken_idx <= i || taken_idx > stop {
            return None;
        }
        let prev = taken_idx.checked_sub(1)?;
        if prev <= i || self.jump_op(prev) != "JMP" {
            return None;
        }
        // last body block must JMP back to the header's start offset
        if self.g.blocks[prev].succs.first().copied() != Some(b.start_dw) {
            return None;
        }
        let (stmts, cmp) = block_stmts(self.ctx, b.instr_lo, b.instr_hi);
        let cond = negate(&branch_cond(&cmp, self.jump_op(i)));
        // Whatever the header computes IS part of the condition — it is re-evaluated on every
        // iteration, so it can be neither hoisted in front of the loop nor dropped. Dropped is
        // what used to happen, and it leaves the test reading a slot nothing ever fills: a
        // `while (!this.bDone)` came out as `bool local_1 = false; while (local_1)`, a loop that
        // never runs. Fold the header's single store into the test where its shape allows it;
        // anything else keeps the old rendering rather than risking a worse one.
        loop_diag("top-test", stmts.as_slice(), &cond);
        let cond = fold_loop_header_store(stmts.as_slice(), &cond).unwrap_or(cond);
        Some((taken_idx, cond))
    }

    /// batch-44b (E2, specs/loop-body-cfg-ext.md §2.2): the `it.CanProceed` top-test condition of
    /// a test-first foreach whose latch/test block is `test_idx`. The iterator test lowers as
    /// `LoadVObjR wIt, ?, <CanProceed-off>; RDR1 wS; CpyVtoR1 wS; JLowNZ/JLowZ →body`. Render the
    /// condition DIRECTLY from the member load (`local_It.CanProceed`) instead of the
    /// bottom-test's uninitialized slot read (`local_S != 0`), oriented so the loop CONTINUES on
    /// the back-edge sense. Returns None on ANY deviation from the exact iterator idiom (bail-safe:
    /// the caller then keeps the current bottom-test emission).
    fn foreach_test_cond(&self, test_idx: usize) -> Option<String> {
        let b = &self.g.blocks[test_idx];
        let jop = self.jump_op(test_idx);
        // the test terminator must be the conditional back-edge itself
        if !matches!(jop, "JLowNZ" | "JLowZ") || !self.is_backward_cond(test_idx) {
            return None;
        }
        // find the member load feeding the test register: LoadVObjR wIt, ?, off -> a `CanProceed`
        // field on the iterator. Require the field name to be exactly `CanProceed` — the iterator
        // termination predicate — so no other member-tested loop can match.
        let mut recv_field: Option<String> = None;
        for j in b.instr_lo..b.instr_hi {
            let ins = &self.ctx.instrs[j];
            if ins.op.name == "LoadVObjR" {
                let obj = self
                    .ctx
                    .slot_name(ins.words.first().copied().map(s16).unwrap_or(0));
                let off = ins.words.get(1).copied().unwrap_or(0) as i32;
                let tid = ins.dwords.first().copied().unwrap_or(0) as i32;
                if let Some(field) = self.ctx.refs.member(tid, off) {
                    if field == "CanProceed" {
                        recv_field = Some(format!("{obj}.{field}"));
                    }
                }
            }
        }
        let expr = recv_field?;
        // back-edge (loop body) is taken when the iterator can proceed. JLowNZ → body when the
        // test is non-zero (true): loop continues while `expr`. JLowZ → body when zero (a NOT'd
        // form): loop continues while `!expr`.
        Some(match jop {
            "JLowNZ" => expr,
            _ => format!("!({expr})"),
        })
    }

    /// batch-44d: the iterator SLOT of a test-first foreach — the `wIt` operand of the test
    /// block's `LoadVObjR wIt, ?, CanProceed`. Used to capture the entry block's `Iterator()`
    /// result into a decl-init (`auto local_It = coll.Iterator();`) — the iterator local has NO
    /// default constructor, so a bare hoisted `TMapIterator local_It;` fails to compile.
    fn foreach_iter_slot(&self, test_idx: usize) -> Option<i32> {
        let b = &self.g.blocks[test_idx];
        for j in b.instr_lo..b.instr_hi {
            let ins = &self.ctx.instrs[j];
            if ins.op.name == "LoadVObjR" {
                let off = ins.words.get(1).copied().unwrap_or(0) as i32;
                let tid = ins.dwords.first().copied().unwrap_or(0) as i32;
                if self.ctx.refs.member(tid, off) == Some("CanProceed") {
                    return Some(ins.words.first().copied().map(s16).unwrap_or(0));
                }
            }
        }
        None
    }

    /// batch-44b (E2, specs/loop-body-cfg-ext.md §2.2): detect a TEST-FIRST `foreach` whose ENTRY
    /// block `e` is a bare `JMP →T` into the CanProceed test block `T` (the loop is entered at the
    /// test, guarding the first iteration). The loop IS already detected by `loop_latch` (firing at
    /// `e+1`, latch=`T`), but the current emission renders it as a BOTTOM-test `while (slot != 0)`
    /// reading an uninitialized test slot — an invalid `.as` that collapses. When this exact
    /// signature holds, the caller renders the correct top-test `while (it.CanProceed) { body }`.
    ///
    /// Signature (ALL required; any deviation ⇒ None ⇒ status-quo bottom-test):
    ///   1. block `e` ends in a bare `JMP →jt` with a SINGLE successor (no fall-through);
    ///   2. `T = idx_of[jt]`, `e < T < stop`, and `T` is the CanProceed test block
    ///      (`is_backward_cond`, terminator `JLowNZ`/`JLowZ`, `foreach_test_cond` recognizes it);
    ///   3. `T`'s back-edge target is exactly `blocks[e+1].start_dw` — the body head is `e+1`;
    ///   4. the body `[e+1, T)` is Gate-2 recoverable under `loop_scope = {continue: T, break:
    ///      T's forward exit}`.
    /// Returns `(test_idx, cond, body_head, break_off)`.
    fn test_first_foreach(
        &mut self,
        e: usize,
        stop: usize,
    ) -> Option<(usize, String, usize, usize)> {
        // 1. entry block `e` must be a bare single-successor JMP.
        if self.jump_op(e) != "JMP" {
            return None;
        }
        let eb = &self.g.blocks[e];
        if eb.succs.len() != 1 {
            return None;
        }
        let jt = eb.succs[0];
        let test_idx = *self.idx_of.get(&jt)?;
        if test_idx <= e || test_idx >= stop {
            return None;
        }
        // 2. `T` is the CanProceed test with a recognized condition.
        let cond = self.foreach_test_cond(test_idx)?;
        // 3. `T`'s back-edge target is the body head = block e+1.
        let tb = &self.g.blocks[test_idx];
        let test_off = tb.start_dw;
        let back = *tb.succs.iter().find(|&&s| s <= test_off)?;
        let body_head = e + 1;
        if body_head >= test_idx || self.g.blocks[body_head].start_dw != back {
            return None;
        }
        // break_off = the test's forward (non-back-edge) successor (the loop exit).
        let break_off = *tb.succs.iter().find(|&&s| s > test_off)?;
        // 4. Gate 2 — the body must fully structure under the loop scope. (No Gate 1: a foreach
        //    body is legitimately straight-line; Gate 2 alone proves every edge is recoverable.)
        let ls = LoopScope {
            continue_off: test_off,
            break_off,
        };
        let saved = self.loop_scope;
        self.loop_scope = Some(ls);
        let ok = self.loop_body_recoverable(body_head, test_idx, ls);
        self.loop_scope = saved;
        if !ok {
            return None;
        }
        Some((test_idx, cond, body_head, break_off))
    }

    fn is_backward_cond(&self, bi: usize) -> bool {
        let b = &self.g.blocks[bi];
        matches!(
            self.jump_op(bi),
            "JS" | "JNS" | "JP" | "JNP" | "JZ" | "JNZ" | "JLowZ" | "JLowNZ"
        ) && b.succs.iter().any(|&s| s <= b.start_dw)
    }

    /// batch-36 Stage C — block `bi` ends in an UNCONDITIONAL `JMP` back to an earlier-or-equal
    /// offset (an uncond-JMP loop latch: `while(true)`/mid-test loops that `loop_latch`
    /// (cond-only) and `top_test_while` (strict block-before-taken rule) both miss).
    fn is_backward_jump(&self, bi: usize) -> bool {
        let b = &self.g.blocks[bi];
        self.jump_op(bi) == "JMP" && b.succs.first().is_some_and(|&s| s <= b.start_dw)
    }

    /// batch-36 Stage C (conservative first cut, spec §2.3) — detect a mid-test loop whose latch
    /// is an UNCONDITIONAL `JMP` back to the header block `i`, in the CLEAN top-test form only:
    /// block `i` is itself a single-block conditional (`is_cond`) that tests the loop condition,
    /// one edge continues the body (`i+1`), the other is the loop EXIT (`break_off`, forward past
    /// the latch), and the last body block JMPs unconditionally back to `i`. Returns
    /// `(latch_idx, cond, continue_off, break_off)`. Bails (None) on any compound-header /
    /// mid-body-test / multi-exit shape — those stay on their status-quo emission (a stub or
    /// flat chain), never a guessed `while(true)`.
    fn uncond_latch_loop(&self, i: usize, stop: usize) -> Option<(usize, String, usize, usize)> {
        // never steal a block a conditional latch or top-test already claims
        if self.loop_latch(i, stop).is_some() || self.top_test_while(i, stop).is_some() {
            return None;
        }
        let header_off = self.g.blocks[i].start_dw;
        // the header block must be a single 2-succ conditional = the loop test
        if !self.is_cond(i) {
            return None;
        }
        let b = &self.g.blocks[i];
        let taken = *b.succs.first()?;
        let fall = *b.succs.get(1)?;
        // find the uncond-JMP latch: a block in (i, stop) whose JMP targets exactly `header_off`
        let mut latch = None;
        for bi in (i + 1)..stop {
            if self.is_backward_jump(bi) && self.g.blocks[bi].succs.first() == Some(&header_off) {
                if latch.is_some() {
                    return None; // more than one back-edge to the header — not the clean shape
                }
                latch = Some(bi);
            }
        }
        let latch = latch?;
        let latch_off = self.g.blocks[latch].start_dw;
        // exactly one of {taken, fall} continues the body (== i+1 offset), the other is the exit
        let fall_idx = *self.idx_of.get(&fall)?;
        let taken_idx = *self.idx_of.get(&taken)?;
        // body must be the fall-through run [i+1, latch); the exit is the other edge, and it must
        // lie strictly after the latch (a genuine loop exit, not an in-body forward jump)
        let (body_continue_idx, exit_off) = if fall_idx == i + 1 {
            (fall_idx, taken)
        } else if taken_idx == i + 1 {
            (taken_idx, fall)
        } else {
            return None;
        };
        let _ = body_continue_idx;
        let exit_idx = *self.idx_of.get(&exit_off)?;
        // the exit must be OUTSIDE the loop span (past the latch) — else it is an inner branch,
        // and this is not a clean top-test loop.
        if exit_idx <= latch || exit_off <= latch_off {
            return None;
        }
        // no OTHER back-edge inside the body may target an offset <= header (irreducible / a
        // second loop we are not modeling) — keep this to the single-latch reducible shape.
        for bi in (i + 1)..latch {
            let bb = &self.g.blocks[bi];
            if bb.succs.iter().any(|&s| s <= header_off) {
                return None;
            }
        }
        // condition renders so the loop CONTINUES on the fall edge (body) and EXITS on the other:
        // if fall is the body, the taken edge is the exit → `while (!taken_cond)`; if taken is the
        // body, `while (taken_cond)`. Mirror `top_test_while` (which always has fall = body).
        let cmp = block_stmts(self.ctx, b.instr_lo, b.instr_hi).1;
        let raw = branch_cond(&cmp, self.jump_op(i));
        let cond = if fall_idx == i + 1 { negate(&raw) } else { raw };
        Some((latch, cond, header_off, exit_off))
    }

    /// batch-36 Stage C — [`Self::uncond_latch_loop`] PLUS the two body gates, run with the loop
    /// scope active so the Gate-2 dry run's nested-switch probe agrees with the real emit. Returns
    /// the loop tuple ONLY when the body has a genuine inner branch (Gate 1) AND fully structures
    /// (Gate 2). If either gate fails, returns None so the caller falls through to the `is_cond`
    /// arm — a newly-detected loop must NOT emit a linearized/lossy body (that would be a QUALITY
    /// regression vs the status-quo `if` the is_cond arm produces, and risks mis-binding an inner
    /// break/continue). This is the "a newly-detected loop only wins when fully recoverable" rule.
    fn uncond_latch_loop_recoverable(
        &mut self,
        i: usize,
        stop: usize,
    ) -> Option<(usize, String, usize, usize)> {
        let (latch, cond, cont_off, brk_off) = self.uncond_latch_loop(i, stop)?;
        let ls = LoopScope {
            continue_off: cont_off,
            break_off: brk_off,
        };
        if !self.body_has_inner_branch(i + 1, latch, ls) {
            return None;
        }
        let saved = self.loop_scope;
        self.loop_scope = Some(ls);
        let ok = self.loop_body_recoverable(i + 1, latch, ls);
        self.loop_scope = saved;
        if ok {
            Some((latch, cond, cont_off, brk_off))
        } else {
            None
        }
    }
}

fn bin_op(name: &str) -> Option<&'static str> {
    Some(match name {
        "ADDi" | "ADDi64" | "ADDf" | "ADDd" => "+",
        "SUBi" | "SUBi64" | "SUBf" | "SUBd" => "-",
        "MULi" | "MULi64" | "MULf" | "MULd" => "*",
        "DIVi" | "DIVi64" | "DIVf" | "DIVd" | "DIVu" | "DIVu64" => "/",
        "MODi" | "MODi64" | "MODf" | "MODd" | "MODu" | "MODu64" => "%",
        "BAND" | "BAND64" => "&",
        "BOR" | "BOR64" => "|",
        "BXOR" | "BXOR64" => "^",
        "BSLL" | "BSLL64" => "<<",
        // arithmetic (BSRA) and logical (BSRL) right shift both render `>>`; AngelScript
        // re-derives the variant from the operand's signedness (matches the linear decompiler).
        "BSRA" | "BSRA64" | "BSRL" | "BSRL64" => ">>",
        _ => return None,
    })
}

fn iconst_op(name: &str) -> Option<&'static str> {
    Some(match name {
        "ADDIi" | "ADDIf" => "+",
        "SUBIi" | "SUBIf" => "-",
        "MULIi" | "MULIf" => "*",
        _ => return None,
    })
}

fn operand_str(ins: &Instr) -> String {
    let mut p = Vec::new();
    for w in &ins.words {
        p.push(format!("w{}", *w as i16));
    }
    for d in &ins.dwords {
        p.push(format!("0x{d:x}"));
    }
    for q in &ins.qwords {
        p.push(format!("0x{q:x}"));
    }
    p.join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone)]
    struct TestInsn {
        label: Option<&'static str>,
        name: &'static str,
        words: Vec<u16>,
        dwords: Vec<u32>,
        target: Option<&'static str>,
    }

    #[derive(Clone)]
    struct CompoundFixture {
        instrs: Vec<Instr>,
        labels: HashMap<&'static str, usize>,
    }

    #[derive(Default)]
    struct TestAssembler {
        next_label: Option<&'static str>,
        ops: Vec<TestInsn>,
    }

    impl TestAssembler {
        fn label(&mut self, label: &'static str) {
            assert!(self.next_label.replace(label).is_none());
        }

        fn op(&mut self, name: &'static str, words: &[u16], dwords: &[u32]) {
            self.ops.push(TestInsn {
                label: self.next_label.take(),
                name,
                words: words.to_vec(),
                dwords: dwords.to_vec(),
                target: None,
            });
        }

        fn jump(&mut self, name: &'static str, target: &'static str) {
            self.ops.push(TestInsn {
                label: self.next_label.take(),
                name,
                words: Vec::new(),
                dwords: vec![0],
                target: Some(target),
            });
        }

        fn finish(self) -> CompoundFixture {
            let op_info = |name: &str| {
                crate::cache::isa::OPCODES
                    .iter()
                    .find(|op| op.name == name)
                    .unwrap_or_else(|| panic!("test opcode {name}"))
            };
            let mut labels = HashMap::new();
            let mut offset = 0usize;
            for op in &self.ops {
                if let Some(label) = op.label {
                    assert!(labels.insert(label, offset).is_none());
                }
                offset += op_info(op.name).size_dwords as usize;
            }
            assert!(self.next_label.is_none());

            let mut instrs = Vec::new();
            offset = 0;
            for mut spec in self.ops {
                if let Some(target) = spec.target {
                    let rel = labels[&target] as i64 - offset as i64 - 2;
                    spec.dwords[0] = rel as i32 as u32;
                }
                let op = op_info(spec.name);
                instrs.push(Instr {
                    offset_dw: offset,
                    op,
                    words: spec.words,
                    dwords: spec.dwords,
                    qwords: Vec::new(),
                });
                offset += op.size_dwords as usize;
            }
            CompoundFixture { instrs, labels }
        }
    }

    fn compound_switch_fixture() -> CompoundFixture {
        let mut a = TestAssembler::default();
        a.label("preheader");
        a.op("SetV4", &[4], &[0]);

        // Materialized `(local_4 == 0 || local_4 == 1)` header.
        a.label("head");
        a.op("iTOf", &[11, 4], &[]);
        a.op("fTOi", &[12, 11], &[]);
        a.op("CMPIi", &[12], &[0]);
        a.jump("JNZ", "head_else");
        a.label("head_true");
        a.op("SetV1", &[7], &[1]);
        a.jump("JMP", "head_test");
        a.label("head_else");
        a.op("iTOf", &[11, 4], &[]);
        a.op("fTOi", &[12, 11], &[]);
        a.op("CMPIi", &[12], &[1]);
        a.label("head_tz");
        a.op("TZ", &[], &[]);
        a.label("head_else_store");
        a.op("CpyRtoV4", &[7], &[]);
        a.label("head_test");
        a.op("CpyVtoR1", &[7], &[]);
        a.label("head_branch");
        a.jump("JLowZ", "loop_exit");

        // Standard guarded JMPP dispatch: cases 0/1 share a body; case 2 is inline.
        a.label("switch");
        a.op("CMPIi", &[4], &[2]);
        a.jump("JP", "default");
        a.op("CMPIi", &[4], &[0]);
        a.jump("JS", "default");
        a.op("SUBIi", &[6, 4], &[0]);
        a.op("JMPP", &[6], &[2]);
        a.jump("JMP", "shared_case");
        a.jump("JMP", "shared_case");
        a.label("case_two");
        a.op("SetV1", &[5], &[2]);
        a.op("CpyVtoR4", &[5], &[]);
        a.label("case_two_return");
        a.jump("JMP", "shared_ret");

        a.label("shared_case");
        a.op("CMPIi", &[4], &[0]);
        a.jump("JNZ", "continue_trampoline");
        a.label("case_return");
        a.op("SetV1", &[5], &[1]);
        a.op("CpyVtoR4", &[5], &[]);
        a.label("case_return_jump");
        a.jump("JMP", "shared_ret");
        a.label("continue_trampoline");
        a.jump("JMP", "head");

        // Physical default code follows the backward trampoline but still belongs to switch.
        a.label("default");
        a.op("SetV1", &[5], &[1]);
        a.op("CpyVtoR4", &[5], &[]);
        a.jump("JMP", "shared_ret");

        a.label("loop_exit");
        a.op("SetV1", &[5], &[1]);
        a.op("CpyVtoR4", &[5], &[]);
        a.label("shared_ret");
        a.op("RET", &[0], &[]);

        // Reachable only as independent post-RET blocks. They provide mutation targets for the
        // outside-entry and ambiguous-join negative regressions without shifting the fixture.
        a.label("outside_jump");
        a.jump("JMP", "shared_ret");
        a.label("alt_ret");
        a.op("RET", &[0], &[]);
        a.finish()
    }

    fn retarget(fixture: &mut CompoundFixture, source: &'static str, target: &'static str) {
        let source = fixture.labels[&source];
        let target = fixture.labels[&target];
        let ins = fixture
            .instrs
            .iter_mut()
            .find(|ins| ins.offset_dw == source)
            .expect("source instruction");
        assert!(matches!(ins.op.name, "JMP" | "JZ" | "JNZ"));
        ins.dwords[0] = (target as i64 - source as i64 - 2) as i32 as u32;
    }

    fn replace_same_width(
        fixture: &mut CompoundFixture,
        label: &'static str,
        name: &'static str,
        words: &[u16],
        dwords: &[u32],
    ) {
        let offset = fixture.labels[&label];
        let ins = fixture
            .instrs
            .iter_mut()
            .find(|ins| ins.offset_dw == offset)
            .expect("instruction to replace");
        let op = crate::cache::isa::OPCODES
            .iter()
            .find(|op| op.name == name)
            .expect("replacement opcode");
        assert_eq!(op.size_dwords, ins.op.size_dwords);
        ins.op = op;
        ins.words = words.to_vec();
        ins.dwords = dwords.to_vec();
        ins.qwords.clear();
    }

    fn replace_jump_opcode_same_target(
        fixture: &mut CompoundFixture,
        label: &'static str,
        name: &'static str,
    ) {
        let offset = fixture.labels[&label];
        let ins = fixture
            .instrs
            .iter_mut()
            .find(|ins| ins.offset_dw == offset)
            .expect("jump instruction to replace");
        let op = crate::cache::isa::OPCODES
            .iter()
            .find(|op| op.name == name)
            .expect("replacement jump opcode");
        assert!(is_cond_op(ins.op.name) && is_cond_op(op.name));
        assert_eq!(op.size_dwords, ins.op.size_dwords);
        ins.op = op;
    }

    fn render_fixture_range(
        fixture: &CompoundFixture,
        range: Option<(&'static str, &'static str, LoopScope)>,
    ) -> String {
        let f = FuncCode {
            func: "Synthetic::CompoundLoopSwitch".into(),
            is_method: false,
            param_names: Vec::new(),
            param_types: Vec::new(),
            ret: DataType {
                token: 0x44,
                ..Default::default()
            },
            bytecode: Vec::new(),
        };
        let refs = RefResolver::default();
        let local_types =
            HashMap::from([(4, "EHeaderStatus".to_string()), (7, "bool".to_string())]);
        let g = cfg::build(&fixture.instrs);
        let idx_of: HashMap<usize, usize> = g
            .blocks
            .iter()
            .enumerate()
            .map(|(i, b)| (b.start_dw, i))
            .collect();
        let ctx = Ctx {
            f: &f,
            refs: &refs,
            instrs: &fixture.instrs,
            super_ctor: None,
            ret_ty: Some(&f.ret),
            fields: None,
            param_types: None,
            class_name: None,
            local_types: Some(&local_types),
            float_slots: std::collections::HashSet::new(),
            param_off_map: HashMap::new(),
            rvo_off: None,
            keep_ints: None,
            rvo_switch_region: std::cell::Cell::new(false),
        };
        let mut st = Structurer {
            ctx: &ctx,
            g: &g,
            idx_of: &idx_of,
            exit_join: None,
            exit_join_is_ret: false,
            exit_ret_rows_ok: false,
            exit_rvo_return: false,
            exit_mixed_rvo_ret_rows_ok: false,
            exit_scan_floor: 0,
            carry: None,
            loop_scope: None,
        };
        let (start, stop) = if let Some((start, stop, scope)) = range {
            st.loop_scope = Some(scope);
            (
                idx_of[&fixture.labels[start]],
                idx_of[&fixture.labels[stop]],
            )
        } else {
            (0, g.blocks.len())
        };
        let mut out = String::new();
        st.emit_range(start, stop, 0, &mut out);
        out
    }

    fn assert_jmpp_rejected(fixture: &CompoundFixture, why: &str) {
        let src = render_fixture_range(fixture, None);
        assert!(
            src.contains("// JMPP"),
            "unsafe compound loop accepted ({why}):\n{src}"
        );
    }

    #[test]
    fn compound_header_switch_continue_recovers_synthetically() {
        let fixture = compound_switch_fixture();
        let src = render_fixture_range(&fixture, None);
        assert!(src.contains("while ("), "loop missing:\n{src}");
        assert!(
            src.contains("int(float32(int(local_4))) == 0"),
            "enum cast / numeric-cast chain was lost:\n{src}"
        );
        assert!(
            src.contains("int(float32(int(local_4))) == 1"),
            "second OR term missing:\n{src}"
        );
        assert!(
            src.contains("switch (int(local_4))"),
            "enum switch cast missing:\n{src}"
        );
        assert!(src.contains("continue;"), "continue missing:\n{src}");
        assert!(!src.contains("// JMPP"), "JMPP marker remains:\n{src}");
    }

    #[test]
    fn compound_header_rejects_second_or_wrong_backedge() {
        let mut second = compound_switch_fixture();
        retarget(&mut second, "case_return_jump", "head");
        assert_jmpp_rejected(&second, "second backedge");

        let mut wrong = compound_switch_fixture();
        retarget(&mut wrong, "continue_trampoline", "head_else");
        assert_jmpp_rejected(&wrong, "wrong backward target");
    }

    #[test]
    fn compound_header_rejects_side_effect_or_nonboolean_materialization() {
        let mut side_effect = compound_switch_fixture();
        replace_same_width(&mut side_effect, "head_tz", "INCi", &[], &[]);
        assert_jmpp_rejected(&side_effect, "side-effecting header");

        let mut non_bool = compound_switch_fixture();
        replace_same_width(&mut non_bool, "head_tz", "CpyVtoR4", &[4], &[]);
        assert_jmpp_rejected(&non_bool, "non-boolean materialized value");
    }

    #[test]
    fn compound_header_rejects_a_live_elided_temporary() {
        let mut live = compound_switch_fixture();
        replace_same_width(&mut live, "loop_exit", "CMPIi", &[7], &[0]);
        assert_jmpp_rejected(&live, "header temporary read on loop exit");
    }

    #[test]
    fn compound_header_rejects_cross_iteration_partial_materialization() {
        let mut partial = compound_switch_fixture();
        replace_same_width(&mut partial, "head_else_store", "CpyRtoV4", &[10], &[]);
        assert_jmpp_rejected(&partial, "header temp carried from a prior iteration");
    }

    #[test]
    fn compound_header_rejects_non_low_jump_after_narrow_bool_copy() {
        let mut jnp = compound_switch_fixture();
        replace_jump_opcode_same_target(&mut jnp, "head_branch", "JNP");
        assert_jmpp_rejected(&jnp, "JNP after CpyVtoR1 has unproved upper bytes");

        let mut jns = compound_switch_fixture();
        replace_jump_opcode_same_target(&mut jns, "head_branch", "JNS");
        assert_jmpp_rejected(&jns, "JNS after CpyVtoR1 has unproved upper bytes");
    }

    #[test]
    fn canonical_full_bool_models_all_non_low_jump_senses() {
        let canonical = HeaderValue {
            expr: HeaderExpr::Atom("b".into()),
            boolish: true,
            full_bool: true,
            ty: Some("bool".into()),
        };
        assert_eq!(
            header_bool_jump_taken(canonical.clone(), "JNP")
                .unwrap()
                .render(),
            "!(b)"
        );
        assert!(matches!(
            header_bool_jump_taken(canonical.clone(), "JNS"),
            Some(HeaderExpr::Bool(true))
        ));
        let narrow = HeaderValue {
            full_bool: false,
            ..canonical
        };
        assert!(header_bool_jump_taken(narrow, "JNP").is_none());
    }

    #[test]
    fn not_is_an_explicit_header_slot_read_and_write() {
        let op = crate::cache::isa::OPCODES
            .iter()
            .find(|op| op.name == "NOT")
            .unwrap();
        let ins = Instr {
            offset_dw: 0,
            op,
            words: vec![7],
            dwords: Vec::new(),
            qwords: Vec::new(),
        };
        assert_eq!(explicit_slot_access(&ins, 7), (true, true));
    }

    #[test]
    fn ambiguous_numeric_cast_targets_require_exact_slot_signedness() {
        assert_eq!(
            header_numeric_cast_target("iTOb", Some("uint8")),
            Some("uint8")
        );
        assert_eq!(
            header_numeric_cast_target("iTOb", Some("int8")),
            Some("int8")
        );
        assert_eq!(
            header_numeric_cast_target("uTOi64", Some("uint64")),
            Some("uint64")
        );
        assert_eq!(
            header_numeric_cast_target("iTOi64", Some("int64")),
            Some("int64")
        );
        assert_eq!(header_numeric_cast_target("iTOb", None), None);
        assert_eq!(header_numeric_cast_target("uTOi64", Some("int")), None);
        assert_eq!(header_numeric_cast_source("iTOf"), Some("int"));
        assert_eq!(header_numeric_cast_source("ubTOi"), Some("uint8"));

        let enum_value = || HeaderValue {
            expr: HeaderExpr::Atom("status".into()),
            boolish: false,
            full_bool: false,
            ty: Some("EStatus".into()),
        };
        assert_eq!(
            HeaderExpr::Cast(
                "int",
                Box::new(header_numeric_cast_operand("sbTOi", enum_value()).unwrap())
            )
            .render(),
            "int(status)"
        );
        assert_eq!(
            HeaderExpr::Cast(
                "float32",
                Box::new(header_numeric_cast_operand("iTOf", enum_value()).unwrap())
            )
            .render(),
            "float32(int(status))"
        );
    }

    #[test]
    fn header_copy_uses_destination_signedness_before_following_casts() {
        let unsigned = HeaderValue {
            expr: HeaderExpr::UInt(u32::MAX as u64),
            boolish: false,
            full_bool: false,
            ty: Some("uint".into()),
        };
        let signed = header_copy_value(unsigned, Some("int".into()), 32)
            .expect("same-width uint-to-int copy");
        assert_eq!(signed.ty.as_deref(), Some("int"));
        assert_eq!(signed.expr.render(), "int(4294967295)");

        // Model iTOf -> fTOi after the copy. The first cast must consume signed -1, not the
        // original uint 4294967295; retaining the source slot type would change the condition.
        let as_float = HeaderExpr::Cast("float32", Box::new(signed.expr));
        let back_to_int = HeaderExpr::Cast("int", Box::new(as_float));
        let compare = HeaderExpr::Cmp(
            Box::new(back_to_int),
            HeaderRel::Eq,
            Box::new(HeaderExpr::Int(-1)),
        );
        assert_eq!(compare.render(), "int(float32(int(4294967295))) == -1");

        let float_bits = HeaderValue {
            expr: HeaderExpr::Real("1.0f".into()),
            boolish: false,
            full_bool: false,
            ty: Some("float32".into()),
        };
        assert!(header_copy_value(float_bits.clone(), Some("uint".into()), 32).is_none());
        assert!(header_copy_value(float_bits.clone(), None, 32).is_none());
        assert!(header_copy_value(float_bits, Some("float32".into()), 64).is_none());
    }

    #[test]
    fn header_constants_preserve_ieee_bits_and_unsigned_high_bits() {
        let value = |op, raw, ty: &str| {
            header_set_value(op, raw, Some(ty.into()))
                .expect("supported typed SetV constant")
                .expr
                .render()
        };
        assert_eq!(value("SetV4", 1.0f32.to_bits() as u64, "float32"), "1.0f");
        assert_eq!(value("SetV8", 1.0f64.to_bits(), "float"), "1.0");
        assert_eq!(value("SetV4", u32::MAX as u64, "uint"), "4294967295");
        assert_eq!(value("SetV8", u64::MAX, "uint64"), "18446744073709551615");
        assert!(header_set_value("SetV4", 0, Some("float".into())).is_none());
        assert!(header_set_value("SetV8", 0, Some("float32".into())).is_none());
        assert!(header_set_value("SetV1", 0xff, Some("EByteEnum".into())).is_none());
        assert!(
            header_set_value("SetV4", f32::NAN.to_bits() as u64, Some("float32".into())).is_none()
        );
        assert!(header_set_value("SetV8", f64::INFINITY.to_bits(), Some("float".into())).is_none());

        let inferred = header_set_slot_type(None, true, "SetV4");
        assert_eq!(inferred.as_deref(), Some("float32"));
        assert_eq!(
            header_set_value("SetV4", 1.0f32.to_bits() as u64, inferred)
                .unwrap()
                .expr
                .render(),
            "1.0f"
        );

        // CMPIu uses the same unsigned expression form, then makes its 32-bit VM domain explicit.
        let cmpiu_rhs = HeaderExpr::Cast("uint", Box::new(HeaderExpr::UInt(u32::MAX as u64)));
        assert_eq!(cmpiu_rhs.render(), "uint(4294967295)");
    }

    #[test]
    fn integer_header_comparisons_reject_known_non_integer_sources() {
        for ty in [
            "bool",
            "int8",
            "uint8",
            "int16",
            "uint16",
            "int",
            "uint",
            "int64",
            "uint64",
            "EHeaderStatus",
        ] {
            assert!(header_is_integral_type(Some(ty)), "rejected {ty}");
        }
        assert!(header_is_integral_type(None));
        for ty in ["float32", "float", "UObject", "FVector"] {
            assert!(!header_is_integral_type(Some(ty)), "accepted {ty}");
        }
    }

    #[test]
    fn compound_loop_rejects_outside_entry_into_header_or_switch_tail() {
        let mut header_entry = compound_switch_fixture();
        retarget(&mut header_entry, "outside_jump", "head_test");
        assert_jmpp_rejected(&header_entry, "outside header entry");

        let mut default_entry = compound_switch_fixture();
        retarget(&mut default_entry, "outside_jump", "default");
        assert_jmpp_rejected(&default_entry, "outside default/tail entry");
    }

    #[test]
    fn switch_rejects_loop_break_and_ambiguous_return_join() {
        let mut break_target = compound_switch_fixture();
        retarget(&mut break_target, "continue_trampoline", "loop_exit");
        let scope = LoopScope {
            continue_off: break_target.labels["head"],
            break_off: break_target.labels["loop_exit"],
        };
        let body = render_fixture_range(&break_target, Some(("switch", "loop_exit", scope)));
        assert!(
            body.contains("// JMPP"),
            "loop break was misrendered as switch break:\n{body}"
        );

        let mut ambiguous = compound_switch_fixture();
        retarget(&mut ambiguous, "case_two_return", "outside_jump");
        retarget(&mut ambiguous, "continue_trampoline", "loop_exit");
        let scope = LoopScope {
            continue_off: ambiguous.labels["head"],
            break_off: ambiguous.labels["alt_ret"],
        };
        let body = render_fixture_range(&ambiguous, Some(("switch", "loop_exit", scope)));
        assert!(
            body.contains("// JMPP"),
            "switch with ambiguous forward joins was accepted:\n{body}"
        );
    }

    #[test]
    fn member_ref_push_uses_native_enum_only_as_a_precise_value_fallback() {
        assert_eq!(
            member_ref_push_type(
                Some("ERelationship"),
                Some("EOtherEnum"),
                Some("FContainingStruct")
            ),
            Some("ERelationship".into())
        );
        assert_eq!(
            member_ref_push_type(None, Some("ERelationship"), Some("FContainingStruct")),
            Some("ERelationship".into())
        );
    }

    #[test]
    fn member_ref_push_keeps_owner_fallback_without_a_value_type_witness() {
        assert_eq!(
            member_ref_push_type(None, None, Some("FContainingStruct")),
            Some("FContainingStruct".into())
        );
        assert_eq!(member_ref_push_type(None, None, None), None);
    }

    #[test]
    fn same_type_psf_copy_requires_all_type_witnesses() {
        let dst = Arg::psf("local_28".into(), Some("FTransform".into()));
        let src = Arg::psf("local_52".into(), Some("FTransform".into()));
        assert!(is_proven_same_type_psf_copy(
            &dst,
            std::slice::from_ref(&src),
            Some("FTransform"),
            Some(1)
        ));

        let wrong_src = Arg::psf("local_52".into(), Some("FVector".into()));
        assert!(!is_proven_same_type_psf_copy(
            &dst,
            &[wrong_src],
            Some("FTransform"),
            Some(1)
        ));
        assert!(!is_proven_same_type_psf_copy(&dst, &[src], None, Some(1)));
    }

    #[test]
    fn thiscall1_physical_frame_preserves_deferred_outer_args() {
        // Synthetic form of:
        //   n"CanToast", this, 0, this.Transitions, Thiscall1 Last,
        //   <Last result>.Condition, CALLSYS BindUFunction
        // `Last()` renders with zero args, but Thiscall1 physically consumes the pushed zero.
        let opcode = crate::cache::isa::op_info(200).expect("Thiscall1 opcode");
        assert_eq!(opcode.name, "Thiscall1");
        assert_eq!(opcode.stack_inc, -3); // one dword argument + two-dword receiver
        assert_eq!(call_frame_arity("Thiscall1", Some(0)), Some(1));
        assert_eq!(call_frame_arity("CALLSYS", Some(0)), Some(0));

        let mut stack = vec![
            Arg::typed(r#"n"CanToast""#.into(), Some("FName".into())).carry(),
            Arg::typed("this".into(), Some("UObject".into())).carry(),
            Arg::iconst("0".into(), ConstBits::W4(0)).carry(),
            Arg::typed("this.Transitions".into(), Some("TArray".into())).carry(),
        ];

        let inner_need = call_frame_arity("Thiscall1", Some(0)).unwrap() + 1; // receiver
        let inner = take_call_frame(&mut stack, Some(inner_need));
        assert_eq!(
            inner.iter().map(|a| a.s.as_str()).collect::<Vec<_>>(),
            ["0", "this.Transitions"]
        );
        assert_eq!(
            stack.iter().map(|a| a.s.as_str()).collect::<Vec<_>>(),
            [r#"n"CanToast""#, "this"]
        );

        stack.push(Arg::typed(
            "this.Transitions.Last().Condition".into(),
            Some("FInteractionAnimTransitionCondition".into()),
        ));
        let outer = take_call_frame(&mut stack, Some(3));
        assert!(stack.is_empty());
        assert_eq!(
            outer.iter().map(|a| a.s.as_str()).collect::<Vec<_>>(),
            [
                r#"n"CanToast""#,
                "this",
                "this.Transitions.Last().Condition"
            ]
        );
    }

    #[test]
    fn mixed_rvo_exit_proof_accepts_one_clean_store_before_cleanup() {
        let stmts = vec![
            "local_16 = MakeResult();".to_string(),
            "__return = local_16;".to_string(),
            "local_24.Destruct();".to_string(),
        ];
        assert_eq!(single_clean_rvo_store(&stmts), Some("local_16"));
    }

    #[test]
    fn mixed_rvo_exit_proof_rejects_missing_unresolved_or_ambiguous_store() {
        let missing = vec!["local_24.Destruct();".to_string()];
        assert_eq!(single_clean_rvo_store(&missing), None);

        let unresolved = vec![format!("__return = {UNRESOLVED};")];
        assert_eq!(single_clean_rvo_store(&unresolved), None);

        let duplicate = vec![
            "__return = local_16;".to_string(),
            "__return = local_32;".to_string(),
        ];
        assert_eq!(single_clean_rvo_store(&duplicate), None);
    }
}
