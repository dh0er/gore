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
#[derive(Clone)]
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
}
impl Arg {
    fn int(s: String) -> Arg {
        Arg { s, is_int: true, ty: None, cbits: None, is_psf: false }
    }
    fn iconst(s: String, cbits: ConstBits) -> Arg {
        Arg { s, is_int: true, ty: None, cbits: Some(cbits), is_psf: false }
    }
    fn obj(s: String) -> Arg {
        Arg { s, is_int: false, ty: None, cbits: None, is_psf: false }
    }
    fn typed(s: String, ty: Option<String>) -> Arg {
        Arg { s, is_int: false, ty, cbits: None, is_psf: false }
    }
    /// A `PSF`-pushed slot address (out / RVO / in-place-ctor receiver), carrying the slot's
    /// recovered type so the construct can render `slot = <ty>(args)`.
    fn psf(s: String, ty: Option<String>) -> Arg {
        Arg { s, is_int: false, ty, cbits: None, is_psf: true }
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

/// Structured statement body for a function (no signature wrapper), indented at `depth`.
/// Returns an error annotation string on disasm failure (never panics).
pub fn body_statements(f: &FuncCode, refs: &RefResolver, depth: usize) -> String {
    body_statements_ctor(f, refs, depth, None, None, None, None, None, None)
}

/// Like [`body_statements`], but with class context for type-aware casts:
/// - `super_ctor`: super class name, so a call to its ctor on `this` -> `super(...)`.
/// - `ret_ty`: the function's return type, so `return <int>` casts to `bool`/enum.
/// - `fields`: the owning class's field name -> base type name, so a `this.field = <int>`
///   assignment casts the RHS to a `bool`/enum field.
#[allow(clippy::too_many_arguments)]
pub fn body_statements_ctor(f: &FuncCode, refs: &RefResolver, depth: usize, super_ctor: Option<&str>, ret_ty: Option<&DataType>, fields: Option<&HashMap<String, String>>, param_types: Option<&[String]>, class_name: Option<&str>, local_types: Option<&HashMap<i32, String>>) -> String {
    let instrs = match disassemble(&f.bytecode) {
        Ok(i) => i,
        Err(e) => return format!("{}// disasm error: {e}\n", "    ".repeat(depth)),
    };
    let g = cfg::build(&instrs);
    let float_slots = float_operand_slots(&instrs);
    // AS_PTR_SIZE-aware frame-offset -> param-index map (2-dword handles/refs + hidden RVO slot),
    // self-correcting on observed offsets. Built once per function; consulted by slot_name/slot_type.
    let (param_off_map, rvo_off) = super::decompile::build_param_off_map_rvo(f, &instrs, refs);
    let ctx = Ctx { f, refs, instrs: &instrs, super_ctor, ret_ty, fields, param_types, class_name, local_types, float_slots, param_off_map, rvo_off };
    let idx_of: HashMap<usize, usize> =
        g.blocks.iter().enumerate().map(|(i, b)| (b.start_dw, i)).collect();
    let mut body = String::new();
    let mut st = Structurer { ctx: &ctx, g: &g, idx_of: &idx_of };
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

/// Decompile a function to a self-contained `function(...) { ... }` (readable, not recompilable).
pub fn decompile(f: &FuncCode, refs: &RefResolver) -> String {
    let params: Vec<String> = f
        .param_names
        .iter()
        .enumerate()
        .map(|(i, n)| if n.is_empty() { format!("arg{i}") } else { n.clone() })
        .collect();
    let body = body_statements(f, refs, 1).replace(RVODEF, "/* unrecovered */ {}");
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
}

/// Collect slots used as an operand of a float/double arithmetic or compare op. Every word
/// operand of those ops is a float/double value, so a constant feeding such a slot is float.
fn float_operand_slots(instrs: &[Instr]) -> std::collections::HashSet<i32> {
    let is_float_op = |n: &str| {
        matches!(n,
            "ADDf" | "SUBf" | "MULf" | "DIVf" | "MODf" | "NEGf" | "IncVf" | "DecVf"
            | "ADDIf" | "SUBIf" | "MULIf" | "CMPf" | "CMPIf"
            | "ADDd" | "SUBd" | "MULd" | "DIVd" | "MODd" | "NEGd" | "CMPd")
    };
    let mut slots = std::collections::HashSet::new();
    for ins in instrs {
        if is_float_op(ins.op.name) {
            for &wd in &ins.words {
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
    for i in (0..before).rev() {
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
        _ => return None,
    })
}

/// The raw bits of a `SetV*` constant written to a slot — so a later store into a float/
/// double field can reinterpret them as the real IEEE-754 value instead of an int literal.
#[derive(Clone, Copy)]
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
fn build_call(stack: &mut Vec<Arg>, f: &str, is_method: bool, super_ctor: Option<&str>, params: Option<&[DataType]>, native_arity: Option<usize>, trusted_arity: Option<usize>, target_owner: Option<&str>, cur_class: Option<&str>, non_virtual: bool, ret_ty: Option<&str>, refs: &RefResolver) -> Option<String> {
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
    let rvo_slot = is_method && ret_ty
        .map(|t| matches!(t.split('<').next().unwrap_or(t).bytes().next(), Some(b'F') | Some(b'T')))
        .unwrap_or(false);
    let need = trusted_arity.map(|n| n + is_method as usize + rvo_slot as usize);
    let mut a: Vec<Arg> = match need {
        Some(k) if stack.len() > k => {
            // Split off this call's own operands (top `k`); the deeper entries belong to an
            // ENCLOSING call. BUT only PRESERVE deeper entries that are plausibly real enclosing
            // args (typed locals/globals/PSF slots) — a stranded plain int/const literal left over
            // from an unmodeled push is dead and, if preserved, pollutes the enclosing call's arg
            // list and force-stubs it (regression). Drop such dead leading constants here, which is
            // exactly what the old whole-stack `mem::take` did for them.
            let own = stack.split_off(stack.len() - k);
            stack.retain(|x| !x.is_int);
            own
        }
        _ => std::mem::take(stack),
    }
        .into_iter()
        .filter(|x| !x.s.is_empty() && x.s != UNRESOLVED)
        .collect();
    // Effective arity: the in-game compile validates against the shipped Binds.Cache, so its native
    // arity is authoritative — prefer it over the script FunctionReferences param count. Falls back.
    let arity = native_arity.or_else(|| params.map(|p| p.len()));
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
        if let Some(rh) = ret_ty.map(head).filter(|_| !is_operator) {
            if matches!(rh.bytes().next(), Some(b'F') | Some(b'T') | Some(b'E')) {
                // the RVO out-slot = a PSF arg whose type head equals the return-type head
                if let Some(pos) = a.iter().position(|x| x.is_psf
                    && x.ty.as_deref().map(head) == Some(rh)) {
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
            return Some(format!("super({})", render_args(&a, params, refs)));
        }
        // BUG (a) — SUPER-CALL: a NON-VIRTUAL (`CALL`) dispatch on `this` to a method owned by a
        // STRICT ANCESTOR of the current class is a `Super::method()` call, not `this.method()`
        // (a genuine virtual self-call compiles to CALLINTF, never a CALL to the base func-id).
        if non_virtual && recv.s == "this" {
            if let (Some(owner), Some(cur)) = (target_owner, cur_class) {
                if owner != cur && refs.is_subclass(cur, owner) {
                    return Some(format!("Super::{f}({})", render_args(&a, params, refs)));
                }
            }
        }
        // a call whose name is a type = an in-place constructor (member struct default ctor) —
        // implicit in AS source, emit nothing.
        if refs.is_type_name(f) {
            return None;
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
        Some(format!("{}.{f}({})", recv.s, render_args(&a, params, refs)))
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
    let fwd = arg_mismatch_count(a, params, refs);
    if fwd == 0 {
        return; // already matches -> leave untouched
    }
    let mut rev = a.clone();
    rev.reverse();
    if arg_mismatch_count(&rev, params, refs) < fwd {
        a.reverse();
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
fn downcast(rhs: String, src_ty: Option<String>, dst_ty: Option<&String>, refs: &RefResolver) -> String {
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

/// True if `tyname` is a UE enum type (`E<Upper>...`) — same shape `cast_to_typename` keys on.
fn is_enum_name(tyname: &str) -> bool {
    let b = tyname.as_bytes();
    b.len() >= 2 && b[0] == b'E' && b[1].is_ascii_uppercase()
}

/// Wrap an enum-typed RHS being stored into an INT slot as `int(expr)`. AngelScript has no
/// implicit enum->int conversion, so an enum field-read / enum-returning call stored into an
/// `int` local fails to compile. Only fires when the value is a known enum AND the dest is an
/// int slot (so enum->enum and enum->enum-param copies stay bare).
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
    let mut out = Vec::new();
    let mut cmp: Option<Cmp> = None;
    let mut cond: Option<(String, bool)> = None; // (bool value tested by a jump, is-bool-typed)
    let mut stack: Vec<Arg> = Vec::new(); // pushed pointer/value expressions
    let mut value_reg: Option<String> = None;
    let mut obj_reg: Option<String> = None;
    let mut ref_reg: Option<String> = None; // Idiom-B member address
    let mut ref_reg_ty: Option<String> = None; // field type name behind ref_reg (for casts)
    let mut set_consts: HashMap<i32, ConstBits> = HashMap::new(); // last SetV* constant per slot
    let mut pending: Option<String> = None; // unconsumed call/ctor result
    let mut pending_ty: Option<String> = None; // recovered type of `pending` (call return type)
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
            }
        }
        match n {
            // ---- pushes ----
            "PshC4" => {
                let b = ins.dwords.first().copied().unwrap_or(0);
                stack.push(Arg::iconst((b as i32).to_string(), ConstBits::W4(b)));
            }
            "PshC8" => {
                let b = ins.qwords.first().copied().unwrap_or(0);
                stack.push(Arg::iconst((b as i64).to_string(), ConstBits::W8(b)));
            }
            "PshV4" | "PshV8" => stack.push(Arg::int(name(w(ins, 0)))),
            "PshVPtr" => stack.push(Arg::typed(name(w(ins, 0)), ctx.slot_type(w(ins, 0)))),
            "PSF" => {
                // &local, unless it's the destination of a following ALLOC
                if insns.get(k + 1).map(|i| i.op.name) != Some("ALLOC") {
                    // &local at the AS source level is implicit (param decides &in/&out) — no `&`.
                    // Tag is_psf so a following `$beh0` construct can recover `slot = T(args)`.
                    stack.push(Arg::psf(name(w(ins, 0)), ctx.slot_type(w(ins, 0))));
                }
                // else: this PSF is the destination local for the following ALLOC; don't push.
            }
            "PshRPtr" => {
                // The value register holds a just-completed call's return value; PshRPtr pushes it
                // back onto the operand stack as the NEXT call's argument (e.g. the receiver/arg of
                // a chained call). Prefer that live call result over the stale member-ref register.
                if let Some(p) = pending.take() {
                    stack.push(Arg::typed(p, pending_ty.take()));
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
                    stack.push(Arg::obj(format!("\"{}\"", esc(ctx.refs.global_by_ptr(ptr).unwrap_or("")))));
                } else {
                    let nm = ctx.refs.global_by_ptr(ptr).unwrap_or("global?");
                    if let Some(cls) = nm.strip_prefix("__StaticType_") {
                        // generator class-pointer global -> the real UClass accessor
                        stack.push(Arg::obj(format!("{cls}::StaticClass()")));
                    } else if nm.starts_with("__") {
                        // other implicit generator global (e.g. __WorldContext) — not a
                        // source-level identifier and not a real arg; drop it.
                    } else if let Some(ns) = ctx.refs.global_ns(ptr) {
                        stack.push(Arg::obj(format!("{ns}::{nm}"))); // e.g. `FColor::Red`
                    } else {
                        stack.push(Arg::obj(nm.to_string()));
                    }
                }
            }
            "PshNull" => stack.push(Arg::obj("nullptr".into())),
            "VAR" => stack.push(Arg::int(name(w(ins, 0)))),
            "FuncPtr" => stack.push(Arg::obj("funcptr".into())),
            // ---- member access (Idiom A: rewrite top of stack in place) ----
            "ADDSi" => {
                let off = ins.words.first().copied().unwrap_or(0) as i32;
                let tid = ins.dwords.first().copied().unwrap_or(0) as i32;
                let field = ctx.refs.member(tid, off).map(|s| s.to_string()).unwrap_or_else(|| format!("field_0x{off:x}"));
                // field VALUE type from the class map (member_type gives the owner class).
                let fty = ctx.fields.and_then(|m| m.get(&field)).cloned()
                    .or_else(|| ctx.refs.member_type(tid, off).map(|s| s.to_string()));
                if let Some(top) = stack.last_mut() {
                    top.s = format!("{}.{field}", top.s);
                    top.is_int = false; // now a member access, not a bare int slot
                    top.ty = fty;
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
                ref_reg_ty = ctx.fields.and_then(|m| m.get(&field)).cloned()
                    .or_else(|| ctx.refs.member_type(tid, off).map(|s| s.to_string()));
                ref_reg = Some(format!("this.{field}"));
            }
            "LoadRObjR" | "LoadVObjR" => {
                let obj = name(w(ins, 0));
                let off = ins.words.get(1).copied().unwrap_or(0) as i32;
                let tid = ins.dwords.first().copied().unwrap_or(0) as i32;
                let field = ctx.refs.member(tid, off).map(|s| s.to_string()).unwrap_or_else(|| format!("field_0x{off:x}"));
                ref_reg_ty = ctx.refs.member_type(tid, off).map(|s| s.to_string()); // foreign object field
                ref_reg = Some(format!("{obj}.{field}"));
            }
            _ if n.starts_with("RDR") => {
                flush!();
                if let Some(r) = &ref_reg {
                    let dst_slot = w(ins, 0);
                    let dst_is_int = dst_slot > 0 && ctx.slot_type(dst_slot).is_none();
                    let rhs = enum_to_int(r.clone(), ref_reg_ty.as_deref(), dst_is_int);
                    out.push(format!("{} = {rhs};", name(dst_slot)));
                }
            }
            _ if n.starts_with("WRTV") => {
                flush!();
                if let Some(r) = &ref_reg {
                    let rhs = name(w(ins, 0));
                    // a constant slot stored into a float/double field carries IEEE-754 bits,
                    // not an int — decode it; else apply the bool/enum/incompatible cast.
                    let rhs = match ref_reg_ty.as_deref() {
                        Some("float32") => float_lit(&set_consts, w(ins, 0), false).unwrap_or(rhs),
                        Some("float") | Some("double") => float_lit(&set_consts, w(ins, 0), true).unwrap_or(rhs),
                        Some(t) if looks_int(&rhs) => field_assign_rhs(&rhs, t),
                        _ => rhs,
                    };
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
                } else {
                    (bits as i32).to_string()
                };
                out.push(format!("{} = {};", name(w(ins, 0)), rhs));
            }
            "CpyVtoV4" | "CpyVtoV8" => {
                flush!();
                out.push(format!("{} = {};", name(w(ins, 0)), name(w(ins, 1))));
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
                cmp = Some(Cmp { a: name(w(ins, 0)), b: name(w(ins, 1)), ..Default::default() });
            }
            "CMPIi" | "CMPIu" => {
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
                let bits = ins.dwords.first().copied().unwrap_or(0);
                cmp = Some(Cmp {
                    a: name(w(ins, 0)),
                    b: fmt_float(ConstBits::W4(bits), false),
                    ..Default::default()
                });
            }
            "CmpPtrNull" => cmp = Some(Cmp { a: name(w(ins, 0)), b: "nullptr".into(), ..Default::default() }),
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
                // Fix b2 — FLUSH a pending statement-position call result before this call starts,
                // so a chained statement call (e.g. MakeRequirement then Add) isn't silently
                // overwritten. Drops sentinel/unresolved pendings (see flush_b2 doc).
                flush_b2!();
                let id = ins.dwords.first().copied().unwrap_or(0) as i32;
                let f = ctx.refs.func_by_id(id).unwrap_or("func?").to_string();
                pending = if f == "StaticClass" {
                    // Fix b1 — StaticClass takes 0 operands; the stack holds the ENCLOSING call's
                    // already-pushed args. Do NOT clear it (clearing destroys those args).
                    pending_ty = None;
                    // The class is the StaticClass func's NAMESPACE last-segment (objtype is
                    // NULL for StaticClass; the target class lives in the namespace), not the
                    // calling class — `local = UFoo::StaticClass()` from inside UBar must say UFoo.
                    let cls = ctx.refs.staticclass_class_by_id(id)
                        .or_else(|| ctx.refs.func_owner_by_id(id))
                        .or(ctx.class_name).unwrap_or("UObject");
                    Some(format!("{cls}::StaticClass()"))
                } else {
                    pending_ty = ctx.refs.func_ret_by_id(id).map(|d| d.base_name(ctx.refs));
                    let na = ctx.refs.native_arity_by_id(id, &f);
                    // SCRIPT call by id: the cache FunctionReference param count is authoritative
                    // (only NATIVE param lists undercount), so trust it for the EDIT B-PRIME split.
                    let trusted = ctx.refs.func_params_by_id(id).map(|p| p.len());
                    let owner = ctx.refs.func_owner_by_id(id);
                    build_call(&mut stack, &f, ctx.refs.is_method_by_id(id), ctx.super_ctor, ctx.refs.func_params_by_id(id), na, trusted, owner, ctx.class_name, n == "CALL", pending_ty.as_deref(), ctx.refs)
                };
            }
            "CALLSYS" | "Thiscall1" => {
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
                            let args: Vec<Arg> = match params.map(|p| p.len()) {
                                Some(k) if stack.len() > k => stack.split_off(stack.len() - k),
                                _ => std::mem::take(&mut stack),
                            }
                                .into_iter()
                                .filter(|x| !x.s.is_empty() && x.s != UNRESOLVED).collect();
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
                    let cls = ctx.refs.staticclass_class_by_ptr(ptr)
                        .or_else(|| ctx.refs.func_owner_by_ptr(ptr))
                        .or(ctx.class_name).unwrap_or("UObject");
                    Some(format!("{cls}::StaticClass()"))
                } else {
                    pending_ty = ctx.refs.func_ret_by_ptr(ptr).map(|d| d.base_name(ctx.refs));
                    let na = ctx.refs.native_arity_by_ptr(ptr, &f);
                    // A free/static native function in a namespace (Gameplay, Math, System, ...)
                    // must be called qualified `Namespace::func(...)` or the global lookup fails
                    // with "No matching signatures". (Methods carry no namespace -> rendered via
                    // their receiver, unchanged. Arity lookup still uses the bare name + owner.)
                    let qualified = match ctx.refs.func_ns_by_ptr(ptr) {
                        Some(ns) => format!("{ns}::{f}"),
                        None => f.clone(),
                    };
                    // NATIVE call by ptr: the cache param list undercounts, so trust ONLY the
                    // Binds native arity (`na`) for the EDIT B-PRIME split; None falls back to
                    // take-all (byte-identical to today when Binds is absent).
                    build_call(&mut stack, &qualified, ctx.refs.is_method_by_ptr(ptr), ctx.super_ctor, ctx.refs.func_params_by_ptr(ptr), na, na, None, ctx.class_name, false, pending_ty.as_deref(), ctx.refs)
                };
            }
            "CallPtr" => {
                let f = name(w(ins, 0));
                pending_ty = None;
                pending = build_call(&mut stack, &f, false, ctx.super_ctor, None, None, None, None, ctx.class_name, false, None, ctx.refs);
            }
            // ---- object construction ----
            "ALLOC" => {
                let tptr = ins.qwords.first().copied().unwrap_or(0) as i64;
                let ty = ctx.refs.type_by_ptr(tptr).unwrap_or("Object").to_string();
                let args: Vec<String> = std::mem::take(&mut stack).into_iter().filter(|a| !a.s.is_empty()).map(|a| a.s).collect();
                pending_ty = Some(ty.clone());
                pending = Some(format!("{ty}({})", args.join(", ")));
            }
            // ---- result capture ----
            "STOREOBJ" => {
                let slot = w(ins, 0);
                let rhs = match pending.take() {
                    Some(p) => Some(downcast(p, pending_ty.take(), ctx.local_types.and_then(|m| m.get(&slot)), ctx.refs)),
                    None => obj_reg.take(),
                };
                flush_store(&mut out, name(slot), rhs);
            }
            "CpyRtoV4" | "CpyRtoV8" => {
                if let Some(p) = pending.take() {
                    // an enum-returning call stored into an int slot needs an explicit int(...)
                    let dst_slot = w(ins, 0);
                    let dst_is_int = dst_slot > 0 && ctx.slot_type(dst_slot).is_none();
                    let rhs = enum_to_int(p, pending_ty.as_deref(), dst_is_int);
                    out.push(format!("{} = {rhs};", name(dst_slot)));
                }
                pending_ty = None;
            }
            "LOADOBJ" => obj_reg = Some(name(w(ins, 0))),
            "CpyVtoR4" | "CpyVtoR8" => {
                ret_val = pending.take().or_else(|| Some(name(w(ins, 0))));
            }
            "CpyVtoR1" => {
                // a bool moved into the test register: feeds either RET or a conditional jump.
                // A call result is already bool-typed; a slot holds the bool as an int.
                let is_bool = pending.is_some() && pending_ty.as_deref() == Some("bool");
                let v = pending.take().unwrap_or_else(|| name(w(ins, 0)));
                cond = Some((v.clone(), is_bool));
                ret_val = Some(v);
            }
            "RET" => {
                let non_void = ctx.ret_ty.map(|t| t.token != 0x52).unwrap_or(false);
                // a value only belongs in a non-void return; for void, `ret_val` may hold a
                // condition value (CpyVtoR1) that must NOT become `return x;`.
                // Capture the return value BEFORE flush!: a directly-returned call/ctor result
                // lives in `pending` (e.g. `CALL`/`ALLOC` then `RET`), and flush! would emit it
                // as a standalone statement, leaving the return a default (RVODEF).
                let mut v = if non_void {
                    ret_val.take().or_else(|| obj_reg.take()).or_else(|| pending.take())
                } else {
                    None
                };
                flush!();
                if non_void && v.is_none() {
                    v = scan_back_retval(ctx, lo + k);
                }
                match v {
                    Some(v) => {
                        // RVO: a non-trivial return is built by writing the hidden return slot
                        // then returning it -> the decompiler renders `return slot = <value>;`,
                        // which is a syntax error (aborts the whole module's parse). The
                        // assignment RHS is the actual returned value.
                        let v = strip_return_assign(&v).to_string();
                        let v = match ctx.ret_ty {
                            Some(rt) if looks_int(&v) => {
                                let tn = if rt.token == 0x41 { "bool".to_string() } else { rt.base_name(ctx.refs) };
                                cast_to_typename(&v, &tn).unwrap_or(v)
                            }
                            _ => v,
                        };
                        out.push(format!("return {v};"));
                    }
                    // non-void with no recovered value: keep the recovered body, return a
                    // default (the emitter fills RVODEF with a type-correct default value).
                    None if non_void => out.push(format!("return {RVODEF};")),
                    None => out.push("return;".into()),
                }
            }
            // Idiom-A member store: an ADDSi chain builds `this.a.b` on the stack top, then
            // PopRPtr moves that address into the reference register for the following WRTV.
            // (Ignoring it left the member expression on the stack -> phantom call args.)
            "PopRPtr" => {
                if let Some(top) = stack.pop() {
                    ref_reg = Some(top.s);
                    ref_reg_ty = top.ty;
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
                        out.push(format!("{dst} = {};", top.s));
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
    (out, cmp)
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

            if let Some((body_end, cond)) = self.top_test_while(i, stop) {
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
                let (stmts, cmp) = block_stmts(self.ctx, b.instr_lo, b.instr_hi);
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
            } else {
                let (stmts, _) = block_stmts(self.ctx, b.instr_lo, b.instr_hi);
                for s in &stmts {
                    let _ = writeln!(out, "{ind}{s}");
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
        }
    }

    fn is_cond(&self, bi: usize) -> bool {
        matches!(
            self.jump_op(bi),
            "JZ" | "JNZ" | "JS" | "JNS" | "JP" | "JNP" | "JLowZ" | "JLowNZ"
        ) && self.g.blocks[bi].succs.len() == 2
            // forward only (backward = loop latch, handled elsewhere)
            && self.g.blocks[bi].succs.iter().all(|&s| s > self.g.blocks[bi].start_dw)
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
