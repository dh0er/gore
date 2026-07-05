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
    /// batch-27 (Cast-diamond carry): pushed by a plain slot/const/global push opcode in
    /// `block_stmts` — safe to carry across a recognized Cast diamond; never set for
    /// pending-call-result pushes (`PshRPtr`) or synthetic pushes.
    carryable: bool,
}
impl Arg {
    fn int(s: String) -> Arg {
        Arg { s, is_int: true, ..Default::default() }
    }
    fn iconst(s: String, cbits: ConstBits) -> Arg {
        Arg { s, is_int: true, cbits: Some(cbits), ..Default::default() }
    }
    fn obj(s: String) -> Arg {
        Arg { s, ..Default::default() }
    }
    fn typed(s: String, ty: Option<String>) -> Arg {
        Arg { s, ty, ..Default::default() }
    }
    /// A `PSF`-pushed slot address (out / RVO / in-place-ctor receiver), carrying the slot's
    /// recovered type so the construct can render `slot = <ty>(args)`.
    fn psf(s: String, ty: Option<String>) -> Arg {
        Arg { s, ty, is_psf: true, ..Default::default() }
    }
    /// A synthesized implicit `__WorldContext` marker (see `is_ctx`).
    fn ctx() -> Arg {
        Arg { is_ctx: true, ..Default::default() }
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
pub fn body_statements_ctor(f: &FuncCode, refs: &RefResolver, depth: usize, super_ctor: Option<&str>, ret_ty: Option<&DataType>, fields: Option<&HashMap<String, String>>, param_types: Option<&[String]>, class_name: Option<&str>, local_types: Option<&HashMap<i32, String>>, hints: Option<&ArgSlotHints>) -> String {
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
    let ctx = Ctx { f, refs, instrs: &instrs, super_ctor, ret_ty, fields, param_types, class_name, local_types, float_slots, param_off_map, rvo_off, keep_ints };
    let idx_of: HashMap<usize, usize> =
        g.blocks.iter().enumerate().map(|(i, b)| (b.start_dw, i)).collect();
    let mut body = String::new();
    let mut st = Structurer { ctx: &ctx, g: &g, idx_of: &idx_of, exit_join: None, exit_join_is_ret: false, exit_ret_rows_ok: false, exit_scan_floor: 0, carry: None };
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
                    && lhs.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'.');
                return plain.then_some(lhs);
            }
            _ => {}
        }
    }
    None
}

/// Decompile a function to a self-contained `function(...) { ... }` (readable, not recompilable).
pub fn decompile(f: &FuncCode, refs: &RefResolver) -> String {
    let params: Vec<String> = f
        .param_names
        .iter()
        .enumerate()
        .map(|(i, n)| if n.is_empty() { format!("arg{i}") } else { n.clone() })
        .collect();
    let body = body_statements(f, refs, 1)
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
}

/// Collect slots used as an operand of a float/double arithmetic or compare op. Every word
/// operand of those ops is a float/double value, so a constant feeding such a slot is float.
/// Additionally, a slot copied into the VALUE REGISTER (`CpyVtoR4`/`CpyVtoR8`) in a function
/// whose return type is the matching-width float family IS the float return payload, so its
/// `SetV*` constants are IEEE-754 bits too (e.g. the per-case `SetV8 w4, 0xc04b...` returns
/// in `GetScanSweepAngleDeg` are -55.0, not -4590434657685733376).
fn float_operand_slots(instrs: &[Instr], ret_ty: Option<&DataType>) -> std::collections::HashSet<i32> {
    let is_float_op = |n: &str| {
        matches!(n,
            "ADDf" | "SUBf" | "MULf" | "DIVf" | "MODf" | "NEGf" | "IncVf" | "DecVf"
            | "ADDIf" | "SUBIf" | "MULIf" | "CMPf" | "CMPIf"
            | "ADDd" | "SUBd" | "MULd" | "DIVd" | "MODd" | "NEGd" | "CMPd")
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
                let declared_bool = v
                    .strip_prefix("local_")
                    .and_then(|d| d.parse::<i32>().ok())
                    .and_then(|n| self.slot_type(n))
                    .as_deref()
                    == Some("bool");
                let v = match self.ret_ty {
                    Some(rt) if looks_int(&v) && !declared_bool => {
                        let tn = if rt.token == 0x41 { "bool".to_string() } else { rt.base_name(self.refs) };
                        cast_to_typename(&v, &tn).unwrap_or(v)
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
                let v = match (self.ret_ty, v.strip_prefix("local_").and_then(|d| d.parse::<i32>().ok())) {
                    (Some(rt), Some(slot)) => {
                        let rh = rt.base_name(self.refs);
                        match self.slot_type(slot) {
                            Some(st) if provably_derived(&rh, &st, self.refs) => {
                                format!("Cast<{rh}>({v})")
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
            && !self.ret_ty.map(|t| is_enum_name(&t.base_name(self.refs))).unwrap_or(true)
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

/// Conditional-jump opcode (mirrors `cfg::is_cond_jump`, which is private to that module).
fn is_cond_op(n: &str) -> bool {
    matches!(n, "JZ" | "JNZ" | "JS" | "JNS" | "JP" | "JNP" | "JLowZ" | "JLowNZ")
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
    matches!(n,
        "iTOf" | "fTOi" | "uTOf" | "fTOu" | "iTOd" | "dTOi" | "uTOd" | "dTOu"
        | "fTOd" | "dTOf" | "iTOb" | "iTOw" | "sbTOi" | "swTOi" | "ubTOi" | "uwTOi"
        | "i64TOi" | "iTOi64" | "uTOi64" | "i64TOf" | "fTOi64" | "i64TOd" | "dTOi64"
        | "u64TOf" | "fTOu64" | "u64TOd" | "dTOu64")
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
            (false, false, true) => format!("{:?}", f64::MAX),
            (false, true, true) => format!("{:?}", f64::MIN),
            (false, false, false) => format!("{:?}f", f32::MAX),
            (false, true, false) => format!("{:?}f", f32::MIN),
        };
    }
    if double { format!("{v:?}") } else { format!("{:?}f", v as f32) }
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
#[allow(clippy::too_many_arguments)]
fn build_call(stack: &mut Vec<Arg>, f: &str, is_method: bool, super_ctor: Option<&str>, params: Option<&[DataType]>, native_arity: Option<usize>, trusted_arity: Option<usize>, target_owner: Option<&str>, cur_class: Option<&str>, non_virtual: bool, ret_ty: Option<&str>, ret_is_ref: bool, global_shadowed: bool, refs: &RefResolver) -> Option<String> {
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
    fn tyhead(s: &str) -> &str { s.split('<').next().unwrap_or(s) }
    let rvo_slot = !ret_is_ref
        && ret_ty.map(|t| matches!(tyhead(t).bytes().next(), Some(b'F') | Some(b'T'))).unwrap_or(false)
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
    let collected: Vec<Arg> = match need {
        Some(k) if stack.len() > k => {
            // Split off this call's own operands (top `k`); the deeper entries belong to an
            // ENCLOSING call. Drop STRANDED slot-sourced ints (SetV*+PshV4 temporaries) left over
            // from an unmodeled push — preserving them pollutes the enclosing call's arg list and
            // force-stubs it (regression). BUT keep PshC4/PshC8 LITERAL consts (cbits set): those
            // are almost always real pending args of the enclosing/chained call — a float Distance,
            // an int MinCount, a builder-chain weight/cooldown — that the enclosing call needs
            // (proven: IsCloseToCharacter(a, b, 699.0) lost its 699.0 to this drop).
            let own = stack.split_off(stack.len() - k);
            stack.retain(|x| !x.is_int || x.cbits.is_some() || x.keep);
            own
        }
        _ => std::mem::take(stack),
    };
    // Implicit `__WorldContext` markers occupied a stack slot so the split arithmetic matched the
    // native's real frame (whose declared arity counts the hidden WorldContextObject param). They
    // are NOT source args — the UE-AngelScript compiler auto-injects the world context — so strip
    // them from the rendered args and lower the effective arity by the count removed. Without this,
    // dropping them at push time made the split one entry too deep and stole a neighbouring call's
    // arg (GetNPCState-shifts-args family).
    let ctx_count = collected.iter().filter(|x| x.is_ctx).count();
    let mut a: Vec<Arg> = collected.into_iter()
        .filter(|x| !x.is_ctx && !x.s.is_empty() && x.s != UNRESOLVED)
        .collect();
    // Effective arity: the in-game compile validates against the shipped Binds.Cache, so its native
    // arity is authoritative — prefer it over the script FunctionReferences param count. Falls back.
    // Subtract any stripped implicit-context markers (they inflate both the frame and the arity).
    let arity = native_arity.or_else(|| params.map(|p| p.len())).map(|n| n.saturating_sub(ctx_count));
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
        fn head(s: &str) -> &str { s.split('<').next().unwrap_or(s) }
        let is_operator = assign_op(f).is_some() || binop_method(f).is_some();
        if let Some(rh) = ret_ty.map(head).filter(|_| !is_operator && !ret_is_ref) {
            if matches!(rh.bytes().next(), Some(b'F') | Some(b'T') | Some(b'E')) {
                // the RVO out-slot = a PSF arg whose type head equals the return-type head.
                // batch-29c (3a, specs/batch29-errortail.md): the ABI pushes
                // [args..., dest, recv], so after the recv pop the dest is the LAST entry —
                // probe with rposition. The bottom-up probe stole a same-headed struct ARG
                // (FVector::RotateAngleAxis's Axis) and slid the real dest into the arg list
                // (`local_54 = local_6.RotateAngleAxis(local_48, local_62);` with w48 the
                // true dest). Single-PSF cases (the 495-site Iterator/GetActorLocation
                // population) pick the same entry — no regression surface.
                if let Some(pos) = a.iter().rposition(|x| x.is_psf
                    && x.ty.as_deref().map(head) == Some(rh)) {
                    let out = a.remove(pos).s;
                    if let Some(w) = arity {
                        let w = w.min(a.len());
                        if a.len() > w { a.drain(..a.len() - w); }
                    }
                    maybe_reverse_args(&mut a, params, refs);
                    // Include the receiver — this is a METHOD RVO struct-return (has_recv popped
                    // `recv`); omitting it emitted `out = Iterator()` (495×) / `out =
                    // GetActorLocation()` instead of `out = recv.Iterator()` -> "No matching
                    // signatures". `this`-receivers render `this.Method()` (legal, matches the
                    // normal method-render path below).
                    return Some(format!("{out} = {}.{f}({})", wrap_uobject_recv(&recv, target_owner, refs), render_args(&a, params, refs)));
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
        if is_operator && rvo_slot
            && a.last()
                .map(|x| x.is_psf && x.ty.as_deref().map(head) == ret_ty.map(head))
                .unwrap_or(false)
        {
            let dest = a.pop().unwrap();
            if let Some(w) = arity {
                let w = w.min(a.len());
                if a.len() > w { a.drain(..a.len() - w); }
            }
            match (assign_op(f), binop_method(f), a.first()) {
                (Some(op), _, Some(rhs)) => {
                    // compound/plain assign: the result lives in the RECEIVER; `dest` is the
                    // dead return-value temp. Mirror the existing arm's copyctor stub gate.
                    if op == "=" && recv.s == "this" {
                        return Some(amm("copyctor"));
                    }
                    let r = params.and_then(|p| p.first()).map(|pt| cast_arg(rhs, pt, refs))
                        .unwrap_or_else(|| rhs.s.clone());
                    return Some(format!("{} {op} {}", recv.s, r));
                }
                (None, Some(op), Some(rhs)) => {
                    // pure binop: the DEST receives the result.
                    let r = params.and_then(|p| p.first()).map(|pt| cast_arg(rhs, pt, refs))
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
            return Some(format!("super({})", render_args(&a, params, refs)));
        }
        // BUG (a) — SUPER-CALL: a NON-VIRTUAL (`CALL`) dispatch on `this` to a method owned by a
        // STRICT ANCESTOR of the current class is a `Super::method()` call, not `this.method()`
        // (a genuine virtual self-call compiles to CALLINTF, never a CALL to the base func-id).
        if non_virtual && recv.s == "this" {
            if let (Some(owner), Some(cur)) = (target_owner, cur_class) {
                if owner != cur && refs.is_subclass(cur, owner) {
                    maybe_reverse_args(&mut a, params, refs); // super calls are reverse-pushed too
                    return Some(format!("Super::{f}({})", render_args(&a, params, refs)));
                }
            }
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
                    let r = params.and_then(|p| p.first()).map(|pt| cast_arg(rhs, pt, refs))
                        .unwrap_or_else(|| rhs.s.clone());
                    return Some(format!("{} {op} {}", recv.s, r));
                }
                None => return None, // unresolved RHS -> skip rather than emit `x = <bad>`
            }
        }
        if let Some(op) = binop_method(f) {
            if let Some(rhs) = a.first() {
                let r = params.and_then(|p| p.first()).map(|pt| cast_arg(rhs, pt, refs))
                    .unwrap_or_else(|| rhs.s.clone());
                return Some(format!("({} {op} {})", recv.s, r));
            }
        }
        maybe_reverse_args(&mut a, params, refs);
        cast_container_args(f, recv.ty.as_deref(), &mut a);
        Some(format!("{}.{f}({})", wrap_uobject_recv(&recv, target_owner, refs), render_args(&a, params, refs)))
    } else {
        if refs.is_type_name(f) {
            // EDIT C: a value-type factory call (FName/E*/T* — NOT U*/A*, which use ALLOC) whose
            // result is built into the return register and re-pushed by a following `PshRPtr` as
            // an ENCLOSING call's arg. Dropping it (return None) loses that arg → the consuming
            // call renders 0-arg (`this.GetCharacter()`). Instead render it as `T(args)` so it
            // FLOWS into `pending` and PshRPtr recovers it (`this.GetCharacter(FName(...))`).
            // Restores ARITY; the literal may be a name-table index (acceptable per spec). Gate
            // strictly: value type AND non-empty args (a 0-arg in-place default ctor stays None).
            let is_value = matches!(f.bytes().next(), Some(b'F') | Some(b'E') | Some(b'T'));
            if is_value && !a.is_empty() {
                maybe_reverse_args(&mut a, params, refs);
                return Some(format!("{f}({})", render_args(&a, params, refs)));
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
            let owner = target_owner
                .filter(|o| o.starts_with('U'))
                .or_else(|| ret_ty
                    .map(|t| t.split('<').next().unwrap_or(t))
                    .filter(|t| t.starts_with('U') && refs.is_type_name(t)));
            if let Some(owner) = owner {
                maybe_reverse_args(&mut a, params, refs);
                return Some(format!("{owner}::{f}({})", render_args(&a, params, refs)));
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
        let f: std::borrow::Cow<str> =
            if global_shadowed { format!("::{f}").into() } else { f.into() };
        // Free-call RVO struct-return (mirror of the method Fix-b3 arm): a free/static function
        // returning a struct BY VALUE pushes a hidden PSF out-slot. Recover `out = f(args)`
        // instead of leaking the out-slot as a leading arg (GotoPosition/Say/GiveItemTo/
        // FInGameTime::Now). Same data-driven gate as Fix-b3: a PSF arg whose type head equals the
        // return-type head; a call without a genuine out-slot can't match. Gated on !ret_is_ref
        // (batch-20 Class D): a BY-REFERENCE return has no out-slot, and the probe would steal a
        // same-typed by-ref struct arg.
        if let Some(rh) = ret_ty.map(tyhead).filter(|_| !ret_is_ref) {
            if matches!(rh.bytes().next(), Some(b'F') | Some(b'T') | Some(b'E')) {
                if let Some(pos) = a.iter().position(|x| x.is_psf
                    && x.ty.as_deref().map(tyhead) == Some(rh)) {
                    let out = a.remove(pos).s;
                    if let Some(w) = arity {
                        let w = w.min(a.len());
                        if a.len() > w { a.drain(..a.len() - w); }
                    }
                    maybe_reverse_args(&mut a, params, refs);
                    return Some(format!("{out} = {f}({})", render_args(&a, params, refs)));
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
        Some(format!("{f}({})", render_args(&a, params, refs)))
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
    let Some(rt) = recv.ty.as_deref() else { return recv.s.clone() };
    if recv.s == "this" || !BASE_RECV.contains(&rt) {
        return recv.s.clone();
    }
    let Some(o) = owner else { return recv.s.clone() };
    if o == rt
        || !matches!(o.bytes().next(), Some(b'U') | Some(b'A'))
        || engine_ancestors(rt).contains(&o)
        || refs.is_subclass(rt, o)
    {
        return recv.s.clone();
    }
    format!("Cast<{o}>({})", recv.s)
}

/// Count DEFINITE type mismatches when pairing args[i] with params[i] (mirrors `cast_arg`'s
/// "this arg can't possibly match" rule). A value-type (F/E/T) head-mismatch or an object
/// arg that is a known non-subclass of a known-script param both count; everything else
/// (unknown types, int->primitive casts, engine upcasts) is treated as a possible match so
/// the score never penalizes a legitimately-ordered call.
fn arg_mismatch_count(a: &[Arg], params: &[DataType], refs: &RefResolver) -> usize {
    let head = |s: &str| s.split('<').next().unwrap_or(s).to_string();
    let is_value = |s: &str| matches!(s.bytes().next(), Some(b'F') | Some(b'E') | Some(b'T'));
    let is_obj = |s: &str| matches!(s.bytes().next(), Some(b'U') | Some(b'A'));
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
            if cast_to_typename("0", &pt.base_name(refs)).is_none() {
                n += 1;
            }
            continue;
        }
        let Some(at) = &arg.ty else { continue };
        let (ph, ah) = (head(&pt.base_name(refs)), head(at));
        if is_value(&ph) && is_value(&ah) && ph != ah {
            n += 1;
        } else if is_obj(&ph) && is_obj(&ah) && ah != ph
            && refs.is_script_class(&ah) && refs.is_script_class(&ph)
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
fn cast_container_args(method: &str, recv_ty: Option<&str>, a: &mut [Arg]) {
    let Some(t) = recv_ty else { return };
    let t = t.trim_start_matches("const ");
    let Some((head, rest)) = t.split_once('<') else { return };
    let Some(inner) = rest.strip_suffix('>') else { return };
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
        ("TArray" | "TSet", "Add" | "AddUnique" | "Remove" | "RemoveSingle" | "Contains", 1) => &subs[..1],
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
        if let Some(c) = cast_to_typename(&arg.s, want) {
            arg.s = c;
            arg.is_int = false;
            arg.cbits = None;
            arg.ty = Some((*want).to_string());
        }
    }
}

/// Render args joined by ", ", casting each int arg to the callee's expected param type.
fn render_args(a: &[Arg], params: Option<&[DataType]>, refs: &RefResolver) -> String {
    a.iter()
        .enumerate()
        .map(|(i, arg)| match params.and_then(|p| p.get(i)) {
            Some(pt) => cast_arg(arg, pt, refs),
            None => arg.s.clone(),
        })
        .collect::<Vec<_>>()
        .join(", ")
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
                let is_value = |s: &str| matches!(s.bytes().next(), Some(b'F') | Some(b'E') | Some(b'T'));
                let is_obj = |s: &str| matches!(s.bytes().next(), Some(b'U') | Some(b'A'));
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
                if is_obj(&ph) && is_obj(&ah) && ah != ph
                    && refs.is_script_class(&ah) && refs.is_script_class(&ph)
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
                    && matches!(pt.base_name(refs).as_str(),
                        "int" | "int8" | "int16" | "int64" | "uint" | "uint8" | "uint16" | "uint64")
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
            0x50 => return fmt_float(cb, false),        // float32 -> `Nf` literal
            0x51 | 0x5E => return fmt_float(cb, true),  // float (64-bit here) / double -> plain
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
        _ => {}
    }
    if pt.token == 5 {
        // object/enum identifier type: UE enums are `E<Upper>...`; cast int -> enum
        let base = pt.base_name(refs);
        if let Some(c) = cast_to_typename(&arg.s, &base) {
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
fn downcast(rhs: String, src_ty: Option<String>, src_const: bool, dst_ty: Option<&String>, refs: &RefResolver) -> String {
    let is_obj = |s: &str| s.starts_with('U') || s.starts_with('A');
    match (src_ty, dst_ty) {
        (Some(s), Some(d)) if is_obj(&s) && is_obj(d) && s != *d => {
            // An upcast (src derives from dst) is implicit in AngelScript — wrapping it in
            // `Cast<Base>(derived)` can fail in-game compile. Only emit Cast for an actual
            // downcast / unrelated covariant-erased type.
            if refs.is_subclass(&s, d) {
                rhs
            } else {
                format!("Cast<{d}>({rhs})")
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
fn field_assign_rhs(rhs: &str, tyname: &str) -> String {
    if let Some(c) = cast_to_typename(rhs, tyname) {
        return c; // bool / enum
    }
    match tyname {
        "int" | "uint" | "int8" | "int16" | "int64" | "uint8" | "uint16" | "uint64"
        | "float" | "float32" | "double" | "?" => rhs.to_string(),
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
        s.split('<').next().unwrap_or(s).trim_start_matches("const ")
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
pub(crate) fn is_enum_name(tyname: &str) -> bool {
    let b = tyname.trim_start_matches("const ").as_bytes();
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
fn cast_to_typename(rhs: &str, tyname: &str) -> Option<String> {
    if tyname == "bool" {
        return Some(format!("({rhs} != 0)"));
    }
    let b = tyname.as_bytes();
    if b.len() >= 2 && b[0] == b'E' && b[1].is_ascii_uppercase() {
        return Some(format!("{tyname}({rhs})"));
    }
    None
}

/// True if a rendered operand is an integer slot/constant (safe to cast to bool/enum).
/// Excludes already-typed operands (params, fields, calls) so we never double-cast.
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

/// Emit `dst = rhs;` if a result is available (object store).
fn flush_store(out: &mut Vec<String>, dst: String, rhs: Option<String>) {
    if let Some(r) = rhs {
        out.push(format!("{dst} = {r};"));
    }
}

/// Escape a string literal for AS source.
fn esc(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n").replace('\r', "\\r")
}

/// Decompile one block's instruction range into statements; also return the
/// pending comparison (operands of the last CMP*) for condition recovery.
fn block_stmts(ctx: &Ctx, lo: usize, hi: usize) -> (Vec<String>, Option<Cmp>) {
    let (out, cmp, _) = block_stmts_in(ctx, lo, hi, Vec::new());
    (out, cmp)
}

/// [`block_stmts`] with an explicit INITIAL operand stack and the block's LEFTOVER stack in
/// the return (batch-27 Cast-diamond carry): the carried entries occupy the DEEPEST positions,
/// below everything the block pushes — exactly the runtime layout — and the leftover is
/// returned verbatim after the final flush (the UNRESOLVED retain applies to statements only).
fn block_stmts_in(ctx: &Ctx, lo: usize, hi: usize, init: Vec<Arg>) -> (Vec<String>, Option<Cmp>, Vec<Arg>) {
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
    // batch-25a (G2): ENUM value type of a native struct's field behind ref_reg, from the
    // in-crate native-field table (`refs::native_field_type`). Consumed ONLY by the WRTV1
    // guard so the render becomes `field = EEnum(slot)` instead of the bool-wrap.
    let mut ref_reg_nfty: Option<String> = None;
    let mut set_consts: HashMap<i32, ConstBits> = HashMap::new(); // last SetV* constant per slot
    // Slots whose current value came from a MEMBER READ (RDR* after a member-ref load) — real
    // data values, kept by the nested-call retain (see Arg.keep). Invalidated on overwrite.
    let mut member_read_slots: std::collections::HashSet<i32> = std::collections::HashSet::new();
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

    let insns = &ctx.instrs[lo..hi];
    for k in 0..insns.len() {
        let ins = &insns[k];
        let n = ins.op.name;
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
                } else if matches!(ctx.slot_type(off).as_deref(), Some("float" | "float32" | "double")) {
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
                    stack.push(Arg { s: name(off), is_int: true, keep: true, ..Default::default() }.carry());
                } else if ctx.keep_ints.is_some_and(|s| s.contains(&off)) {
                    // batch-25g (spec family G kin): the emit-side pairing proved every value
                    // push of this slot feeds a KNOWN int-family parameter — a real arg
                    // (Math::Min's second operand below a nested call), not a stranded SetV
                    // temporary; the nested-call retain must keep it.
                    stack.push(Arg { s: name(off), is_int: true, keep: true, ..Default::default() }.carry());
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
                    } else if pending_is_static_name {
                        // batch-31b (N2b): the pending is the PURE `n"..."`/`FName(...)`
                        // literal of the __STATIC_NAME idiom — no side effect can be
                        // reordered by carrying it across a cast diamond, so it may pass
                        // the D9 carryability gate like any plain const push.
                        stack.push(Arg::typed(p, pending_ty.take()).carry());
                    } else {
                        stack.push(Arg::typed(p, pending_ty.take()));
                    }
                } else {
                    let (s, ty) = match value_reg.take() {
                        Some(v) => (v, None),
                        None => (ref_reg.clone().unwrap_or_else(|| UNRESOLVED.into()), ref_reg_ty.clone()),
                    };
                    stack.push(Arg::typed(s, ty));
                }
            }
            "PGA" | "PshGPtr" | "PshG4" => {
                let ptr = ins.qwords.first().copied().unwrap_or(0) as i64;
                if ctx.refs.global_is_string(ptr) {
                    stack.push(Arg::obj(format!("\"{}\"", esc(ctx.refs.global_by_ptr(ptr).unwrap_or("")))).carry());
                } else {
                    let nm = ctx.refs.global_by_ptr(ptr).unwrap_or("global?");
                    if let Some(cls) = nm.strip_prefix("__StaticType_") {
                        // generator class-pointer global -> the real UClass accessor
                        stack.push(Arg::obj(format!("{cls}::StaticClass()")).carry());
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
                let field = ctx.refs.member(tid, off).map(|s| s.to_string()).unwrap_or_else(|| format!("field_0x{off:x}"));
                // field VALUE type — from the enclosing class map, else (batch-21 Class B) the
                // injected per-class field maps keyed by the ADDSi type-id's class name, which
                // resolve FOREIGN script-class/struct members correctly (`SaveState.WeatherModifiers`
                // -> `TMap<EWeather, float32>`). `member_type(tid,off)` stays unused here: it
                // returns the OWNER struct (PropertyReferences.OldTypeId = the CONTAINING type),
                // NOT the field's value type; using it poisons foreign member reads (e.g.
                // `HitResult.BoneName` typed as `FHitResult` instead of `FName`) -> false
                // value-head mismatch -> spurious argtype stub. None = unknown = conservative match.
                let fty = ctx.fields.and_then(|m| m.get(&field)).cloned().or_else(|| {
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
                });
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
                }
            }
            "RDSPtr" => {} // deref in place: no change to the rendered name
            // ---- member access (Idiom B: register) ----
            "LoadThisR" => {
                let off = ins.words.first().copied().unwrap_or(0) as i32;
                let tid = ins.dwords.first().copied().unwrap_or(0) as i32;
                let field = ctx.refs.member(tid, off).map(|s| s.to_string()).unwrap_or_else(|| format!("field_0x{off:x}"));
                // The class field-type map holds the real field VALUE type; member_type()
                // resolves PropertyReferences OldTypeId which is the OWNER class, not the
                // field type — so prefer the map and only fall back to member_type.
                // batch-30c: before that owner-name fallback, try the FLOAT-FAMILY-gated
                // precise sources (cross-module script field maps + the in-crate native
                // float rows) so a float64 member read into an int slot takes the RDR8
                // int(...) wrap instead of the bare precision-warning render. Gated to
                // float names only: a broad flip (e.g. foreign bool fields going bare
                // where the unknowable int(...) wrap compiles today) would regress.
                ref_reg_ty = ctx.fields.and_then(|m| m.get(&field)).cloned()
                    .or_else(|| float_field_type(ctx.refs, tid, &field))
                    .or_else(|| ctx.refs.member_type(tid, off).map(|s| s.to_string()));
                ref_reg_nfty = ctx.refs.type_by_id(tid)
                    .and_then(|cls| ctx.refs.native_field_type(cls, &field))
                    .filter(|t| is_enum_name(t))
                    .map(|s| s.to_string());
                // LoadThisR loads from slot 0. In a METHOD that is `this`; in a FREE (mixin)
                // function slot 0 is parameter 0, so hardcoding `this.` emits an undeclared base
                // -> "'field' is not a member of 'Unknown'". slot_name(0) renders both correctly.
                ref_reg = Some(format!("{}.{field}", name(0)));
            }
            "LoadRObjR" | "LoadVObjR" => {
                let obj = name(w(ins, 0));
                let off = ins.words.get(1).copied().unwrap_or(0) as i32;
                let tid = ins.dwords.first().copied().unwrap_or(0) as i32;
                let field = ctx.refs.member(tid, off).map(|s| s.to_string()).unwrap_or_else(|| format!("field_0x{off:x}"));
                // batch-30c: float-family-gated precise resolution first (see LoadThisR) —
                // e.g. `local = Start.Z;` (FVector, double) into an int slot must know the
                // source is float so the RDR8 wrap fires.
                ref_reg_ty = float_field_type(ctx.refs, tid, &field)
                    .or_else(|| ctx.refs.member_type(tid, off).map(|s| s.to_string())); // foreign object field
                ref_reg_nfty = ctx.refs.type_by_id(tid)
                    .and_then(|cls| ctx.refs.native_field_type(cls, &field))
                    .filter(|t| is_enum_name(t))
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
                            matches!(t.bytes().next(), Some(b'U') | Some(b'A') | Some(b'F') | Some(b'T'))
                                && t.as_bytes().get(1).map(|c| c.is_ascii_uppercase()).unwrap_or(false)
                        }
                    };
                    let float_src = matches!(
                        pending_ty.as_deref().map(|t| t.trim_start_matches("const ")),
                        Some("float" | "float32" | "double")
                    );
                    // batch-30c: RDR8 joins the wrap when the source is KNOWN float-family —
                    // a float64 read into an int slot warns (module-killer); int64 reads
                    // (the reason RDR8 was excluded) stay bare via !float_src.
                    let rhs = if dst_is_int && (unknowable || float_src) && (n != "RDR8" || float_src) {
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
                            matches!(t.bytes().next(), Some(b'U') | Some(b'A') | Some(b'F') | Some(b'T'))
                                && t.as_bytes().get(1).map(|c| c.is_ascii_uppercase()).unwrap_or(false)
                        }
                    };
                    // A KNOWN float-family member read into an int-declared slot keeps the
                    // int(...) wrap: warnings are errors in the game compile, and the bare read
                    // is "Implicit conversion from float to integer loses precision". (Before
                    // batch-21's inherited-fields map these reads were mostly `None`-typed and
                    // took the unknowable wrap; a KNOWN bool/int stays bare — proven clean.)
                    let float_src = matches!(
                        ref_reg_ty.as_deref().map(|t| t.trim_start_matches("const ")),
                        Some("float" | "float32" | "double")
                    );
                    // batch-30c: RDR8 joins the wrap for KNOWN float-family members (the
                    // float64 member -> int slot precision-warning residue: foreign script
                    // config floats, FVector.Z/FRotator.Yaw); int64 member reads stay bare.
                    let rhs = if dst_is_int && (unknowable || float_src) && (n != "RDR8" || float_src) {
                        format!("int({r})")
                    } else {
                        enum_to_int(r.clone(), ref_reg_ty.as_deref(), dst_is_int)
                    };
                    out.push(format!("{} = {rhs};", name(dst_slot)));
                    member_read_slots.insert(dst_slot); // real data value, not a SetV temporary
                }
            }
            _ if n.starts_with("WRTV") => {
                flush!();
                if let Some(r) = &ref_reg {
                    let slot = w(ins, 0);
                    let raw = name(slot);
                    // a constant slot stored into a float/double field carries IEEE-754 bits,
                    // not an int — decode it; else apply the bool/enum/incompatible cast.
                    let mut rhs = match ref_reg_ty.as_deref() {
                        Some("float32") => float_lit(&set_consts, slot, false).unwrap_or(raw.clone()),
                        Some("float") | Some("double") => float_lit(&set_consts, slot, true).unwrap_or(raw.clone()),
                        Some(t) if looks_int(&raw) => field_assign_rhs(&raw, t),
                        _ => raw.clone(),
                    };
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
                        && !ctx.slot_type(slot).as_deref().map(is_enum_name).unwrap_or(false)
                    {
                        if let Some(ety) = ref_reg_nfty.as_deref() {
                            if let Some(c) = cast_to_typename(&raw, ety) {
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
                    if n == "WRTV1"
                        && ref_reg_ty.as_deref() != Some("bool")
                        && !ref_reg_ty.as_deref().map(is_enum_name).unwrap_or(false)
                        && rhs == raw
                        && rhs != UNRESOLVED
                        && ctx.slot_type(slot).as_deref() != Some("bool")
                        && !ctx.slot_type(slot).as_deref().map(is_enum_name).unwrap_or(false)
                    {
                        rhs = format!("({rhs} != 0)");
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
                let rhs = if ctx.float_slots.contains(&w(ins, 0)) {
                    fmt_float(ConstBits::W4(bits), false)
                } else if ctx.slot_type(w(ins, 0)).as_deref().is_some_and(is_enum_name) {
                    // batch-31c (N3 Fix 2): an ENUM-typed slot (out-param slot typing)
                    // written a raw ordinal needs the explicit conversion — AS has no
                    // implicit int->enum (`EInventoryTypes local_7 = 0;` fails).
                    format!("{}({})", ctx.slot_type(w(ins, 0)).unwrap(), bits as i32)
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
                    && matches!(ctx.slot_type(src).as_deref(), Some("float" | "float32" | "double"))
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
                out.push(format!("{} = {} {} {};", name(w(ins, 0)), name(w(ins, 1)), bin_op(n).unwrap(), name(w(ins, 2))));
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
                out.push(format!("{} = {} {} {};", name(w(ins, 0)), name(w(ins, 1)), iconst_op(n).unwrap(), c));
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
            // carry a slot operand. Render as a compound assignment on the recovered member expr.
            "INCi" | "INCi64" | "INCi16" | "INCi8" => {
                flush!();
                if let Some(r) = &ref_reg { out.push(format!("{0} = {0} + 1;", r)); }
            }
            "DECi" | "DECi64" | "DECi16" | "DECi8" => {
                flush!();
                if let Some(r) = &ref_reg { out.push(format!("{0} = {0} - 1;", r)); }
            }
            "NEGi" | "NEGf" | "NEGd" => {
                flush!();
                out.push(format!("{0} = -{0};", name(w(ins, 0))));
            }
            // asBC NOT (opcode 6) is the boolean logical invert. The slot is held as `int` (and
            // is often also written with integer values like `= 1`), and AngelScript rejects `!`
            // on int ("Illegal operation on this datatype"); render an int-safe toggle instead.
            "NOT" => {
                flush!();
                let s = name(w(ins, 0));
                out.push(format!("{s} = int({s} == 0);"));
            }
            // ---- comparisons ----
            "CMPi" | "CMPu" | "CMPf" | "CMPd" | "CMPi64" | "CMPu64" => {
                test_after_call = true;
                cmp = Some(Cmp { a: name(w(ins, 0)), b: name(w(ins, 1)), ..Default::default() });
            }
            "CMPIi" | "CMPIu" => {
                test_after_call = true;
                let c = ins.dwords.first().copied().unwrap_or(0) as i32;
                let s = w(ins, 0);
                // an enum compared to an int literal needs explicit int(enum) — AngelScript has
                // no implicit enum<->int (e.g. `if (_AlternativeState != 0)`).
                let a = if ctx.slot_type(s).as_deref().map(is_enum_name).unwrap_or(false) {
                    format!("int({})", name(s))
                } else { name(s) };
                cmp = Some(Cmp { a, b: c.to_string(), ..Default::default() });
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
                cmp = Some(Cmp { a: name(w(ins, 0)), b: "nullptr".into(), ..Default::default() });
            }
            // a test op turns the CMP register into a bool; it carries the real relational
            // operator (the jump only carries the true/false sense).
            "TZ" => { if let Some(c) = &mut cmp { c.op = Some("=="); } }
            "TNZ" => { if let Some(c) = &mut cmp { c.op = Some("!="); } }
            "TS" => { if let Some(c) = &mut cmp { c.op = Some("<"); } }
            "TNS" => { if let Some(c) = &mut cmp { c.op = Some(">="); } }
            "TP" => { if let Some(c) = &mut cmp { c.op = Some(">"); } }
            "TNP" => { if let Some(c) = &mut cmp { c.op = Some("<="); } }
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
                pending = if f == "StaticClass" {
                    // Fix b1 — StaticClass takes 0 operands; the stack holds the ENCLOSING call's
                    // already-pushed args. Do NOT clear it (clearing destroys those args).
                    pending_ty = None;
                    pending_const = false;
                    pending_is_ref = false;
                    // The class is the StaticClass func's NAMESPACE last-segment (objtype is
                    // NULL for StaticClass; the target class lives in the namespace), not the
                    // calling class — `local = UFoo::StaticClass()` from inside UBar must say UFoo.
                    let cls = ctx.refs.staticclass_class_by_id(id)
                        .or_else(|| ctx.refs.func_owner_by_id(id))
                        .or(ctx.class_name).unwrap_or("UObject");
                    Some(format!("{cls}::StaticClass()"))
                } else {
                    pending_ty = ctx.refs.func_ret_by_id(id).map(|d| d.base_name(ctx.refs));
                    // CALL-by-id = SCRIPT function: its authoritative signature is the module-region
                    // one WE emit (GetG1R renders `UStoryG1R GetG1R()`, no const), while the
                    // tail-table entry spuriously carries bIsObjectConst (2757 GetG1R stores compile
                    // CLEAN in the batch-19 capture). Never const-wrap script-call results.
                    pending_const = false;
                    let na = ctx.refs.native_arity_by_id(id, &orig);
                    // SCRIPT call by id: the cache FunctionReference param count is authoritative
                    // (only NATIVE param lists undercount), so trust it for the EDIT B-PRIME split.
                    let trusted = ctx.refs.func_params_by_id(id).map(|p| p.len());
                    let owner = ctx.refs.func_owner_by_id(id);
                    let ret_is_ref = ctx.refs.func_ret_by_id(id).map(|d| d.is_reference).unwrap_or(false);
                    pending_is_ref = ret_is_ref; // batch-29a: RDR may consume this call's result
                    // batch-30c: a ref-returning call clobbers the VM value register — the
                    // same register member loads fill — so a stale member `ref_reg` can no
                    // longer be what a later RDR reads (the 29a gate-rest: GetResult()
                    // discarded + a stale member read consumed in its place).
                    if ret_is_ref {
                        ref_reg = None;
                        ref_reg_ty = None;
                        ref_reg_nfty = None;
                    }
                    // batch-24b shadow gate: a free SCRIPT global (no owner, global namespace)
                    // rendered inside a class method is shadowed by any same-named member in
                    // the class's (native or script) ancestry. Member-name existence (T3
                    // method names, script method decls, Binds when loaded) over-approximates
                    // "such a member exists somewhere" — `::`-qualifying a non-shadowed global
                    // resolves identically, so false positives are harmless; no name sources
                    // -> false (status quo).
                    let global_shadowed = !ctx.refs.is_method_by_id(id)
                        && owner.is_none()
                        && ctx.class_name.is_some()
                        && ctx.refs.func_ns_by_id(id).map_or(true, |ns| ns.is_empty())
                        && ctx.refs.member_name_exists(&f);
                    build_call(&mut stack, &f, ctx.refs.is_method_by_id(id), ctx.super_ctor, ctx.refs.func_params_by_id(id), na, trusted, owner, ctx.class_name, n == "CALL", pending_ty.as_deref(), ret_is_ref, global_shadowed, ctx.refs)
                };
                pending_is_static_name = false;
            }
            "CALLSYS" | "Thiscall1" => {
                test_after_call = false;
                // Fix b2 — flush a pending statement-position call result before this call begins
                // (e.g. MakeRequirement's result must be emitted before `Add(...)` overwrites it).
                flush_b2!();
                let ptr = ins.qwords.first().copied().unwrap_or(0) as i64;
                let f = ctx.refs.func_by_ptr(ptr).unwrap_or("syscall?").to_string();
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
                                format!("Cast<{ty}>({src})")
                            }
                            _ => src,
                        };
                        out.push(format!("{dst} = {rhs};"));
                    }
                    continue;
                }
                if f == "$beh0" {
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
                        if recv.s == "__return" && !recv.is_psf
                            && ctx.refs.func_params_by_ptr(ptr).map(|p| p.len()) == Some(1)
                            && stack.len() >= 2
                        {
                            let src = stack[stack.len() - 2].clone();
                            if !src.s.is_empty() && src.s != UNRESOLVED && !src.s.starts_with('\u{2}') {
                                stack.truncate(stack.len() - 2);
                                ret_val = Some(src.s);
                                continue;
                            }
                        }
                        // Gate (a): receiver is a PSF'd slot with a known VALUE/struct/template
                        // type (F*/T*/E*). Never a `$`/`?` placeholder, never an object (U*/A*):
                        // object construction uses ALLOC, not this in-place behaviour.
                        let is_value = recv.ty.as_deref()
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
                                .filter(|x| !x.s.is_empty() && x.s != UNRESOLVED).collect();
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
                            // Gate (b): no arg is itself a PSF slot — that is a copy/convert ctor
                            // whose true source is an unrecovered pending call result; rendering it
                            // as `T(&slot)` would be wrong, so drop (prior behaviour).
                            let any_psf_arg = args.iter().any(|a| a.is_psf);
                            // Gate (c): arg count matches the ctor's declared param count (no
                            // spurious leftover operands on the stack).
                            let count_ok = params.map(|p| p.len() == args.len()).unwrap_or(false);
                            if !args.is_empty() && !any_psf_arg && count_ok {
                                let rendered = render_args(&args, params, ctx.refs);
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
                    }
                    // not a recoverable in-place construct -> fall through to the generic `$` drop.
                }
                pending = if f == "StaticClass" {
                    // Fix b1 — do NOT clear the stack; StaticClass takes 0 operands and the entries
                    // present belong to an ENCLOSING call.
                    pending_ty = None;
                    pending_const = false;
                    pending_is_ref = false;
                    let cls = ctx.refs.staticclass_class_by_ptr(ptr)
                        .or_else(|| ctx.refs.func_owner_by_ptr(ptr))
                        .or(ctx.class_name).unwrap_or("UObject");
                    Some(format!("{cls}::StaticClass()"))
                } else {
                    pending_ty = ctx.refs.func_ret_by_ptr(ptr).map(|d| d.base_name(ctx.refs));
                    pending_const = ctx.refs.func_ret_by_ptr(ptr)
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
                    let trusted = na.or_else(|| ctx.refs.func_params_by_ptr(ptr).map(|p| p.len()));
                    // target_owner (T3 ObjectType) was only wired for CALL/CALLINTF; pass it for
                    // native calls too so the UObject-receiver Cast wrap (batch19 class 1) and the
                    // generated-accessor qualification see the owning class of CALLSYS methods.
                    let ret_is_ref = ctx.refs.func_ret_by_ptr(ptr).map(|d| d.is_reference).unwrap_or(false);
                    pending_is_ref = ret_is_ref; // batch-29a: RDR may consume this call's result
                    // batch-30c: mirror of the by-id arm — a ref-returning native call
                    // clobbers the value register; invalidate a stale member ref_reg.
                    if ret_is_ref {
                        ref_reg = None;
                        ref_reg_ty = None;
                        ref_reg_nfty = None;
                    }
                    build_call(&mut stack, &qualified, ctx.refs.is_method_by_ptr(ptr), ctx.super_ctor, ctx.refs.func_params_by_ptr(ptr), na, trusted, ctx.refs.func_owner_by_ptr(ptr), ctx.class_name, false, pending_ty.as_deref(), ret_is_ref, false, ctx.refs)
                };
                // batch-31b: tag a resolved static-name FName literal (see the flag's doc).
                // build_call returns the literal only for the accessor name; a failed gate
                // (non-constant Id operand) falls to the `$`-drop and returns None.
                pending_is_static_name = f == "__STATIC_NAME" && pending.is_some();
            }
            "CallPtr" => {
                let f = name(w(ins, 0));
                pending_ty = None;
                pending_const = false;
                pending_is_ref = false;
                pending_is_static_name = false;
                pending = build_call(&mut stack, &f, false, ctx.super_ctor, None, None, None, None, ctx.class_name, false, None, false, false, ctx.refs);
            }
            // ---- object construction ----
            "ALLOC" => {
                let tptr = ins.qwords.first().copied().unwrap_or(0) as i64;
                let ty = ctx.refs.type_by_ptr(tptr).unwrap_or("Object").to_string();
                let args: Vec<String> = std::mem::take(&mut stack).into_iter().filter(|a| !a.s.is_empty()).map(|a| a.s).collect();
                pending_ty = Some(ty.clone());
                pending_const = false;
                pending_is_ref = false;
                pending_is_static_name = false;
                pending = Some(format!("{ty}({})", args.join(", ")));
            }
            // ---- result capture ----
            "STOREOBJ" => {
                let slot = w(ins, 0);
                let rhs = match pending.take() {
                    Some(p) => Some(downcast(p, pending_ty.take(), std::mem::take(&mut pending_const), ctx.local_types.and_then(|m| m.get(&slot)), ctx.refs)),
                    None => obj_reg.take(),
                };
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
                let foldable = pending.as_deref().map(|p| assign_lhs(p).is_none()).unwrap_or(true)
                    && match (pending_ty.as_deref(), ctx.ret_ty.map(|t| t.base_name(ctx.refs))) {
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
                    && pending.as_deref().map(|p| assign_lhs(p).is_none()).unwrap_or(false);
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
                        (pending_ty.as_deref() != Some("void")).then(|| pending.take()).flatten()
                    })
                } else {
                    None
                };
                flush!();
                if non_void && v.is_none() && !ctx.ret_via_rvo() {
                    v = scan_back_retval(ctx, lo + k);
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
                    let ok = !top.s.is_empty()
                        && top.s != dst
                        && (top.s.starts_with("local_") || top.s.starts_with("Cast<"));
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
                            Some(dt) if top
                                .ty
                                .as_deref()
                                .map(|st| provably_derived(&dt, st, ctx.refs))
                                .unwrap_or(false) =>
                            {
                                format!("Cast<{dt}>({})", top.s)
                            }
                            _ => top.s.clone(),
                        };
                        out.push(format!("{dst} = {rhs};"));
                    }
                }
            }
            // REFCPY (NO_ARG): a pure stack handle-copy with no destination slot operand — just
            // balance the operand stack (the dominant phantom-arg cause); the value is re-read
            // where it is actually used.
            "REFCPY" => { stack.pop(); }
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
                let src = if ctx.slot_type(w(ins, 1)).as_deref().map(is_enum_name).unwrap_or(false) {
                    format!("int({src})")
                } else { src };
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
            "CmpPtr" => cmp = Some(Cmp { a: name(w(ins, 0)), b: name(w(ins, 1)), ..Default::default() }),
            "OBJTYPE" => stack.push(Arg::obj("objtype".into())), // +2: RTTI objtype ptr
            "STR" => stack.push(Arg::obj("\"\"".into())),         // +3: string-constant push
            "PshListElmnt" => stack.push(Arg::int(name(w(ins, 0)))), // +2: list element
            "COPY" => { stack.pop(); }                           // -2: pop the source ptr
            // asBC_CopyScript (QW_ARG = object-type ptr; stack -2): a script value-type / struct
            // copy = the source-level assignment `dest = src;`. Per asBC_COPY (as_context.cpp) the
            // DESTINATION pointer is popped FIRST (it is the stack TOP), the SOURCE is the next
            // entry below it. The compiler pushes SRC first then DEST, so DEST ends up on top —
            // e.g. `PSF <localSrc>; PshVPtr this; ADDSi <member>; CopyScript` is `this.member =
            // localSrc;`, and the RVO struct-return `PSF <local>; PshVPtr <retSlot>; CopyScript`
            // is `<retSlot> = <local>;`. (Earlier this arm had src/dst swapped, which emitted
            // every struct copy/member-init/RVO-return BACKWARDS — `local = this.member` and
            // `local = retSlot`.) Both operands arrive as fully-rendered member/local exprs;
            // dropping it left the destination (member or RVO temp) unwritten -> garbage/null.
            "CopyScript" => {
                let dst = stack.pop();
                let src = stack.pop();
                if let (Some(dst), Some(src)) = (dst, src) {
                    if !src.s.is_empty() && src.s != UNRESOLVED {
                        // RVO struct-return: a copy whose DEST is the hidden return slot is the
                        // function's `return <src>;` — capture it as the return value (the slot
                        // itself has no source name) instead of emitting `__return = <src>;`.
                        if dst.s == "__return" {
                            ret_val = Some(src.s);
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
                            stack.push(Arg::typed(format!("Cast<{ty}>({})", src.s), Some(ty)));
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
            // ---- pure VM housekeeping / flow: ignore ----
            // NOTE: `ThrowException` (throw) and `JMPP` (jump-table/switch) are NOT housekeeping
            // — they're unmodeled control transfers, and `cfg.rs` leaves JMPP successors unknown.
            // They fall through to the `// opcode` marker below so `stub_reason` stubs the body
            // rather than emitting recompilable source with the throw/switch silently dropped.
            "SUSPEND" | "JitEntry" | "PopPtr" | "SwapPtr" | "ClrHi" | "ClrVPtr"
            | "FREE" | "FinConstruct" | "CHKREF" | "ChkRefS" | "ChkNullV" | "ChkNullS"
            | "DestructScript" | "SaveReturnValue" | "ResolveObjectPtr" | "FreeNullV8" | "GETOBJ"
            | "GETOBJREF" | "GETREF"
            | "JMP" => {}
            // CopyScript performs a script-value copy (an assignment), not housekeeping — it's
            // unmodeled, so fall through to the marker + stub rather than dropping the copy.
            // (FinConstruct/DestructScript stay ignored: implicit AS construct/destruct that
            // appear in nearly every function — emitting/stubbing them would be wrong.)
            _ => {
                flush!();
                out.push(format!("// {} {}", n, operand_str(ins)));
            }
        }
    }
    flush!();
    out.retain(|s| !s.contains(UNRESOLVED)); // drop statements with an unresolved value
    // no binary comparison but a bool value was tested -> use it as the branch condition so
    // the jump renders `if (cond != 0)` instead of `if (? != ?)`.
    if cmp.is_none() {
        if let Some((c, is_bool)) = cond.take() {
            if !c.is_empty() && c != UNRESOLVED {
                cmp = Some(Cmp { expr: Some(c), expr_bool: is_bool, ..Default::default() });
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
    // cheap structural negation for the common relational forms
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
    format!("!({cond})")
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
    /// Instruction-index floor for the synthesized-return value scan (the current case
    /// region's first instruction — a value must not leak in from a preceding region).
    exit_scan_floor: usize,
    /// batch-27: (join block index, operand stack surviving a recognized Cast diamond).
    /// Created at the end of the is_cond arm ONLY when [`Self::diamond_join`] proves the
    /// construct is exactly the null-check Cast diamond; consumed exactly once (`take()`)
    /// by the very next loop iteration of `emit_range`.
    carry: Option<(usize, Vec<Arg>)>,
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
            } else if let Some(latch) = self.loop_latch(i, stop) {
                let lcmp = block_stmts(self.ctx, self.g.blocks[latch].instr_lo, self.g.blocks[latch].instr_hi).1;
                let cond = branch_cond(&lcmp, self.jump_op(latch));
                let _ = writeln!(out, "{ind}while ({cond})");
                let _ = writeln!(out, "{ind}{{");
                self.emit_linear(i, latch + 1, depth + 1, out, true);
                let _ = writeln!(out, "{ind}}}");
                next = latch + 1;
            } else if self.is_cond(i) {
                let (stmts, cmp, leftover) = block_stmts_in(self.ctx, b.instr_lo, b.instr_hi, init);
                for s in &stmts {
                    let _ = writeln!(out, "{ind}{s}");
                }
                let jop = self.jump_op(i);
                let fall = b.succs.get(1).copied();
                let taken = b.succs.first().copied();
                let then_idx = fall.and_then(|o| self.idx_of.get(&o).copied());
                let else_idx = taken.and_then(|o| self.idx_of.get(&o).copied());
                let cond = negate(&branch_cond(&cmp, jop));
                let then_end = else_idx.unwrap_or(stop).min(stop).max(i + 1);
                let _ = writeln!(out, "{ind}if ({cond})");
                let _ = writeln!(out, "{ind}{{");
                if let Some(t) = then_idx {
                    if t > i && t <= then_end {
                        self.emit_range(t, then_end, depth + 1, out);
                    }
                }
                let _ = writeln!(out, "{ind}}}");
                next = then_end;
                if let Some(ei) = else_idx {
                    if ei >= then_end && ei > 0 && self.jump_op(ei - 1) == "JMP" {
                        let after_idx = self.g.blocks[ei - 1]
                            .succs
                            .first()
                            .and_then(|o| self.idx_of.get(o).copied())
                            .unwrap_or(stop)
                            .min(stop);
                        if after_idx > ei {
                            let _ = writeln!(out, "{ind}else");
                            let _ = writeln!(out, "{ind}{{");
                            self.emit_range(ei, after_idx, depth + 1, out);
                            let _ = writeln!(out, "{ind}}}");
                            next = after_idx;
                        }
                    }
                }
                // batch-27: a guard block's leftover operand stack survives into the JOIN
                // block's initial stack — ONLY when the construct is provably the null-check
                // Cast diamond (see diamond_join). Everything else keeps drop-at-boundary.
                if !leftover.is_empty() {
                    if let Some(j) = self.diamond_join(i, next, &leftover, &cmp) {
                        self.carry = Some((j, leftover));
                    }
                }
            } else {
                // (linear fallthrough-carry is out of scope — the plain arm's leftover is dropped.)
                let (stmts, _, _) = block_stmts_in(self.ctx, b.instr_lo, b.instr_hi, init);
                for s in &stmts {
                    let _ = writeln!(out, "{ind}{s}");
                }
                if let Some(x) = self.region_exit_stmt(i) {
                    let _ = writeln!(out, "{ind}{x}");
                }
                next = i + 1;
            }

            i = next.max(prev + 1);
        }
    }

    /// Emit a linear run of blocks [i, end) as statements (loop body); the last block's
    /// trailing comparison/jump is dropped when `skip_term_cond`.
    fn emit_linear(&mut self, i: usize, end: usize, depth: usize, out: &mut String, _skip: bool) {
        let ind = "    ".repeat(depth);
        for bi in i..end {
            let b = &self.g.blocks[bi];
            let (stmts, _) = block_stmts(self.ctx, b.instr_lo, b.instr_hi);
            for s in &stmts {
                let _ = writeln!(out, "{ind}{s}");
            }
            if let Some(x) = self.region_exit_stmt(bi) {
                let _ = writeln!(out, "{ind}{x}");
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
            return Some(if self.exit_join_is_ret {
                self.ctx.return_stmt(scan_back_retval_floor(self.ctx, b.instr_hi - 1, self.exit_scan_floor))
            } else {
                "break;".into()
            });
        }
        if self.exit_ret_rows_ok && self.is_bare_ret_off(t) {
            return Some(self.ctx.return_stmt(scan_back_retval_floor(self.ctx, b.instr_hi - 1, self.exit_scan_floor)));
        }
        None
    }

    /// The block at dword offset `off` is a bare `RET` row (exactly one instruction).
    fn is_bare_ret_off(&self, off: usize) -> bool {
        self.idx_of.get(&off).is_some_and(|&bi| {
            let b = &self.g.blocks[bi];
            b.instr_hi - b.instr_lo == 1 && self.ctx.instrs[b.instr_lo].op.name == "RET"
        })
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
    fn try_emit_switch(&mut self, i: usize, stop: usize, depth: usize, out: &mut String) -> Option<usize> {
        let ctx = self.ctx;
        let g = self.g;
        let blocks = &g.blocks;
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
        if b0.instr_hi - b0.instr_lo < 2 || b1.instr_hi - b1.instr_lo != 2 || b2.instr_hi - b2.instr_lo != 2 {
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
        let first_body = if inline_last { b2.succs[n - 1] } else { b2.succs[n - 1] + 2 };
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
        let non_void = ctx.ret_ty.map(|t| t.token != 0x52).unwrap_or(false);
        let mut join_cands: Vec<usize> = Vec::new();
        let mut ret_rows: Vec<usize> = Vec::new();
        let mut fall_pend: Vec<usize> = Vec::new();
        for w in bounds.windows(2) {
            let last_bi = self.idx_of.get(&w[1]).copied()? - 1;
            let lb = &blocks[last_bi];
            match ctx.instrs[lb.instr_hi - 1].op.name {
                "JMP" => {
                    let x = lb.succs.first().copied()?;
                    if self.is_bare_ret_off(x) {
                        if !register_based {
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
                _ => fall_pend.push(w[1]), // falls into next boundary: legal only into JOIN
            }
        }
        join_cands.sort_unstable();
        join_cands.dedup();
        ret_rows.sort_unstable();
        ret_rows.dedup();
        let join_off = match (join_cands.as_slice(), ret_rows.as_slice()) {
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
        if join_is_ret && !register_based {
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
                    if !matches!(ctx.instrs[k2].op.name, "ThrowException" | "SUSPEND" | "JitEntry") {
                        trap_ops = false;
                    }
                }
                let last_of_region = bi2 + 1 == end;
                let uncond = tname == "JMP";
                for &s in &bb.succs {
                    if s >= b && s < end_off {
                        continue; // in-region (incl. internal loops' back edges)
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
                    if uncond && register_based && self.is_bare_ret_off(s) {
                        continue; // per-case `return` exit
                    }
                    return None; // escapes the region (incl. cond jumps to an exit)
                }
                // a return-exit must have a recoverable value INSIDE this region
                if uncond && non_void {
                    if let Some(&s) = bb.succs.first() {
                        let ret_exit = (s == join_off && join_is_ret)
                            || (s != join_off && !(s >= b && s < end_off) && self.is_bare_ret_off(s));
                        if ret_exit
                            && scan_back_retval_floor(ctx, bb.instr_hi - 1, blocks[start].instr_lo).is_none()
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
                let exits = x == join_off
                    || (register_based && !(x >= b && x < end_off) && self.is_bare_ret_off(x));
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
            regions.push(Region { off: b, start, end, is_def, trap, append_break });
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
        // ---- emission (validated: no bail past this point) ----
        let ind = "    ".repeat(depth);
        let (stmts, _) = block_stmts(ctx, b0.instr_lo, b0.instr_hi);
        for s in &stmts {
            let _ = writeln!(out, "{ind}{s}");
        }
        let sel_raw = ctx.slot_name(wv);
        // an enum selector needs the explicit int() (mirrors the CMPIi arm: AS has no
        // implicit enum<->int, and the case labels are int literals)
        let sel = if ctx.slot_type(wv).as_deref().map(is_enum_name).unwrap_or(false) {
            format!("int({sel_raw})")
        } else {
            sel_raw
        };
        let _ = writeln!(out, "{ind}switch ({sel})");
        let _ = writeln!(out, "{ind}{{");
        let saved = (self.exit_join, self.exit_join_is_ret, self.exit_ret_rows_ok, self.exit_scan_floor);
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
            self.exit_scan_floor = blocks[r.start].instr_lo;
            self.emit_range(r.start, r.end, depth + 1, out);
            (self.exit_join, self.exit_join_is_ret, self.exit_ret_rows_ok, self.exit_scan_floor) = saved;
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
            let externally_referenced = blocks.iter().enumerate().any(|(bi2, bb)| {
                (bi2 < i || bi2 >= switch_end) && bb.succs.contains(&join_off)
            });
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
    fn diamond_join(&self, i: usize, next: usize, l: &[Arg], cmp: &Option<Cmp>) -> Option<usize> {
        let ctx = self.ctx;
        let blocks = &self.g.blocks;
        let b = &blocks[i];
        // D9 carryability: every carried entry originates from a plain slot/const/global push
        // (never a pending-call-result PshRPtr — carrying one would reorder side effects).
        if !l.iter().all(|a| a.carryable) {
            return None;
        }
        // D1 guard shape: `JZ` over a bare `CmpPtrNull` (`x == nullptr`, no T*-op, no expr).
        if self.jump_op(i) != "JZ" {
            return None;
        }
        let c = cmp.as_ref()?;
        if c.b != "nullptr" || c.op.is_some() || c.expr.is_some() {
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
        let j = *self.idx_of.get(&join_off)?;
        if j != next {
            return None;
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
        // D7 then-arm content whitelist (stage-1 belt): only the lowered `Cast<T>` shape, with
        // exactly one CALLSYS (resolving to `opCast`) and exactly one TYPEID.
        let (mut ncast, mut ntypeid) = (0usize, 0usize);
        for k in t.instr_lo..t.instr_hi {
            let ins = &ctx.instrs[k];
            match ins.op.name {
                "SUSPEND" | "JitEntry" | "PSF" | "PshVPtr" | "JMP" => {}
                "TYPEID" => ntypeid += 1,
                "CALLSYS" => {
                    let ptr = ins.qwords.first().copied().unwrap_or(0) as i64;
                    if ctx.refs.func_by_ptr(ptr) != Some("opCast") {
                        return None;
                    }
                    ncast += 1;
                }
                _ => return None,
            }
        }
        if ncast != 1 || ntypeid != 1 {
            return None;
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
            let (s0, _, r0) = block_stmts_in(ctx, ab.instr_lo, ab.instr_hi, Vec::new());
            let (s1, _, r1) = block_stmts_in(ctx, ab.instr_lo, ab.instr_hi, l.to_vec());
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
                    if na.or_else(|| ctx.refs.func_params_by_ptr(ptr).map(|p| p.len())).is_none() {
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
        let (j0, _, _) = block_stmts_in(ctx, jb.instr_lo, jb.instr_hi, Vec::new());
        let (j1, _, _) = block_stmts_in(ctx, jb.instr_lo, jb.instr_hi, l.to_vec());
        if j1.len() != j0.len() || (has_amm(&j1) && !has_amm(&j0)) {
            return None;
        }
        Some(j)
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
        let cmp = block_stmts(self.ctx, b.instr_lo, b.instr_hi).1;
        let cond = negate(&branch_cond(&cmp, self.jump_op(i)));
        Some((taken_idx, cond))
    }

    fn is_backward_cond(&self, bi: usize) -> bool {
        let b = &self.g.blocks[bi];
        matches!(
            self.jump_op(bi),
            "JS" | "JNS" | "JP" | "JNP" | "JZ" | "JNZ" | "JLowZ" | "JLowNZ"
        ) && b.succs.iter().any(|&s| s <= b.start_dw)
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
