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
}
impl Arg {
    fn int(s: String) -> Arg {
        Arg { s, is_int: true, ty: None, cbits: None }
    }
    fn iconst(s: String, cbits: ConstBits) -> Arg {
        Arg { s, is_int: true, ty: None, cbits: Some(cbits) }
    }
    fn obj(s: String) -> Arg {
        Arg { s, is_int: false, ty: None, cbits: None }
    }
    fn typed(s: String, ty: Option<String>) -> Arg {
        Arg { s, is_int: false, ty, cbits: None }
    }
}

const AS_PTR_SIZE: i32 = 2;
/// Placeholder for a value the decompiler couldn't resolve (e.g. PshRPtr with no live
/// register). Statements that would emit it are dropped rather than producing bad source.
const UNRESOLVED: &str = "\u{1}unresolved";
/// Sentinel marking a call whose recovered arg count disagrees with the callee signature;
/// its presence in a body forces the stub fallback. Distinct from UNRESOLVED so it is NOT
/// stripped by the unresolved-statement retain (it must survive to reach the emitter).
const ARGMISMATCH: &str = "\u{2}argmismatch";
/// An ARGMISMATCH sentinel tagged with a short cause code (for stub-reason aggregation).
fn amm(code: &str) -> String {
    format!("\u{2}{code}")
}

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
    let ctx = Ctx { f, refs, instrs: &instrs, super_ctor, ret_ty, fields, param_types, class_name, local_types };
    let idx_of: HashMap<usize, usize> =
        g.blocks.iter().enumerate().map(|(i, b)| (b.start_dw, i)).collect();
    let mut body = String::new();
    let mut st = Structurer { ctx: &ctx, g: &g, idx_of: &idx_of };
    st.emit_range(0, g.blocks.len(), depth, &mut body);
    body
}

/// Decompile a function to a self-contained `function(...) { ... }` (readable, not recompilable).
pub fn decompile(f: &FuncCode, refs: &RefResolver) -> String {
    let params: Vec<String> = f
        .param_names
        .iter()
        .enumerate()
        .map(|(i, n)| if n.is_empty() { format!("arg{i}") } else { n.clone() })
        .collect();
    format!(
        "// {}\nfunction({})\n{{\n{}}}\n",
        f.func,
        params.join(", "),
        body_statements(f, refs, 1)
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
}

impl Ctx<'_> {
    fn slot_name(&self, off: i32) -> String {
        if off > 0 {
            return format!("local_{off}");
        }
        if self.f.is_method {
            if off == 0 {
                return "this".into();
            }
            let idx = (-off - AS_PTR_SIZE) as usize;
            return self.param_or_arg(idx);
        }
        // free function: params at off 0, -1, -2, ...
        self.param_or_arg((-off) as usize)
    }

    /// Recovered base type name for a slot: object-local type for `off > 0`, parameter type
    /// for params; None for `this` / unknowns.
    fn slot_type(&self, off: i32) -> Option<String> {
        if off > 0 {
            return self.local_types.and_then(|m| m.get(&off)).cloned();
        }
        let idx = if self.f.is_method {
            (-off - AS_PTR_SIZE) as usize
        } else {
            (-off) as usize
        };
        self.param_types
            .and_then(|p| p.get(idx))
            .filter(|t| !t.is_empty())
            .cloned()
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

/// A primitive numeric-conversion opcode (`dst = (cast) src`); the cast is implicit in
/// type-erased AngelScript source, so we render the plain copy `dst = src`.
fn is_numeric_cast(n: &str) -> bool {
    matches!(n,
        "iTOf" | "fTOi" | "uTOf" | "fTOu" | "iTOd" | "dTOi" | "uTOd" | "dTOu"
        | "fTOd" | "dTOf" | "iTOb" | "iTOw" | "sbTOi" | "swTOi" | "ubTOi" | "uwTOi"
        | "i64TOi" | "iTOi64" | "uTOi64" | "i64TOf" | "fTOi64" | "i64TOd" | "dTOi64"
        | "u64TOf" | "fTOu64" | "u64TOd" | "dTOu64")
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
    if double { format!("{v:?}") } else { format!("{:?}f", v as f32) }
}

/// Render the constant last written to `slot` as a float/double literal. None if untracked.
fn float_lit(m: &HashMap<i32, ConstBits>, slot: i32, double: bool) -> Option<String> {
    Some(fmt_float(*m.get(&slot)?, double))
}

/// A recovered comparison (from CMP* operands), pending a conditional jump.
#[derive(Clone)]
struct Cmp {
    a: String,
    b: String,
}

/// Build a call expression from the pushed-arg stack. For a method, the receiver is the
/// last-pushed entry (top of stack); the rest are args in push order. Operator-overload
/// methods (opAssign/opAdd/opEquals/...) render as the source operator. Returns None for
/// compiler-generated behaviors ($behN construct/destruct) that have no source form.
fn build_call(stack: &mut Vec<Arg>, f: &str, is_method: bool, super_ctor: Option<&str>, params: Option<&[DataType]>, refs: &RefResolver) -> Option<String> {
    if f.starts_with('$') {
        stack.clear();
        return None; // generated construct/destruct behavior — no source statement
    }
    let mut a: Vec<Arg> = std::mem::take(stack)
        .into_iter()
        .filter(|x| !x.s.is_empty() && x.s != UNRESOLVED)
        .collect();
    // Receiver detection by COUNT, not the unreliable bIsMethod flag: AngelScript pushes
    // receiver + N args (N+1 entries) for a method, N for a free call, and the cache's param
    // count N is reliable. So a.len()==N+1 -> method (pop receiver), ==N -> free, else a genuine
    // recovery mismatch -> stub. (Eliminates most phantom-arg / dropped-receiver stubs.)
    let has_recv = match params.map(|p| p.len()) {
        Some(w) if a.len() == w + 1 => true,
        Some(w) if a.len() == w => false,
        Some(w) => return Some(amm(&format!("argcount_g{}_w{}", a.len(), w))),
        None => is_method && !a.is_empty(), // no signature: fall back to the flag
    };
    if has_recv {
        let recv = a.pop().unwrap();
        // super-class constructor on `this` -> `super(args)` (before is_type_name, since the
        // super name is itself a type name).
        if super_ctor == Some(f) && recv.s == "this" {
            return Some(format!("super({})", render_args(&a, params, refs)));
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
        Some(format!("{}.{f}({})", recv.s, render_args(&a, params, refs)))
    } else {
        if refs.is_type_name(f) {
            return None; // free-standing in-place constructor — implicit in AS source
        }
        // an operator-overload method with no stack receiver (its target was in the reference
        // register, e.g. a member opAssign) can't render as a free call — skip it rather than
        // emit `opAssign(...)`, which never resolves.
        if assign_op(f).is_some() || binop_method(f).is_some() {
            return None;
        }
        Some(format!("{f}({})", render_args(&a, params, refs)))
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
                if is_value(&ph) && is_value(&ah) && ph != ah {
                    return ARGMISMATCH.into();
                }
                // objects (U*/A*): an arg that isn't the param or a subclass of it is wrong —
                // but only when BOTH are known script classes (else an engine upcast we can't
                // verify; stay conservative and allow it).
                if is_obj(&ph) && is_obj(&ah) && ah != ph
                    && refs.is_script_class(&ah) && refs.is_script_class(&ph)
                    && !refs.is_subclass(&ah, &ph)
                {
                    return ARGMISMATCH.into();
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
        // recovery is wrong; mark the body for the stub fallback.
        return ARGMISMATCH.into();
    }
    arg.s.clone()
}

/// Wrap a call result in `Cast<DstType>(...)` when it's stored into an object local of a
/// different (derived) type — the cache erases the covariant return type of template getters
/// like `GetTypedOuter<T>`/`SpawnedStorage<T>` to the base, so AS rejects the implicit
/// downcast. Only applies between UObject/AActor types (`U*`/`A*`).
fn downcast(rhs: String, src_ty: Option<String>, dst_ty: Option<&String>, _refs: &RefResolver) -> String {
    let is_obj = |s: &str| s.starts_with('U') || s.starts_with('A');
    match (src_ty, dst_ty) {
        (Some(s), Some(d)) if is_obj(&s) && is_obj(d) && s != *d => format!("Cast<{d}>({rhs})"),
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
    let mut stack: Vec<Arg> = Vec::new(); // pushed pointer/value expressions
    let mut value_reg: Option<String> = None;
    let mut obj_reg: Option<String> = None;
    let mut ref_reg: Option<String> = None; // Idiom-B member address
    let mut ref_reg_ty: Option<String> = None; // field type name behind ref_reg (for casts)
    let mut set_consts: HashMap<i32, ConstBits> = HashMap::new(); // last SetV* constant per slot
    let mut pending: Option<String> = None; // unconsumed call/ctor result
    let mut pending_ty: Option<String> = None; // recovered type of `pending` (call return type)
    let mut ret_val: Option<String> = None;
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

    let insns = &ctx.instrs[lo..hi];
    for k in 0..insns.len() {
        let ins = &insns[k];
        let n = ins.op.name;
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
                    stack.push(Arg::typed(name(w(ins, 0)), ctx.slot_type(w(ins, 0))));
                }
                // else: this PSF is the destination local for the following ALLOC; don't push.
            }
            "PshRPtr" => {
                let (s, ty) = match value_reg.take() {
                    Some(v) => (v, None),
                    None => (ref_reg.clone().unwrap_or_else(|| UNRESOLVED.into()), ref_reg_ty.clone()),
                };
                stack.push(Arg::typed(s, ty));
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
                    out.push(format!("{} = {};", name(w(ins, 0)), r));
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
                out.push(format!("{} = {};", name(w(ins, 0)), bits as i64));
            }
            "SetV4" | "SetV1" => {
                flush!();
                let bits = ins.dwords.first().copied().unwrap_or(0);
                set_consts.insert(w(ins, 0), ConstBits::W4(bits));
                out.push(format!("{} = {};", name(w(ins, 0)), bits as i32));
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
                let c = ins.dwords.first().copied().unwrap_or(0) as i32;
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
            "NEGi" | "NEGf" | "NEGd" | "NOT" => {
                flush!();
                out.push(format!("{0} = -{0};", name(w(ins, 0))));
            }
            // ---- comparisons ----
            "CMPi" | "CMPu" | "CMPf" | "CMPd" | "CMPi64" | "CMPu64" => {
                cmp = Some(Cmp { a: name(w(ins, 0)), b: name(w(ins, 1)) });
            }
            "CMPIi" | "CMPIf" | "CMPIu" => {
                let c = ins.dwords.first().copied().unwrap_or(0) as i32;
                cmp = Some(Cmp { a: name(w(ins, 0)), b: c.to_string() });
            }
            "CmpPtrNull" => cmp = Some(Cmp { a: name(w(ins, 0)), b: "nullptr".into() }),
            // ---- calls ----
            "CALL" | "CALLINTF" | "CALLBND" => {
                let id = ins.dwords.first().copied().unwrap_or(0) as i32;
                let f = ctx.refs.func_by_id(id).unwrap_or("func?").to_string();
                pending = if f == "StaticClass" {
                    stack.clear();
                    pending_ty = None;
                    Some(format!("{}::StaticClass()", ctx.class_name.unwrap_or("UObject")))
                } else {
                    pending_ty = ctx.refs.func_ret_by_id(id).map(|d| d.base_name(ctx.refs));
                    build_call(&mut stack, &f, ctx.refs.is_method_by_id(id), ctx.super_ctor, ctx.refs.func_params_by_id(id), ctx.refs)
                };
            }
            "CALLSYS" | "Thiscall1" => {
                let ptr = ins.qwords.first().copied().unwrap_or(0) as i64;
                let f = ctx.refs.func_by_ptr(ptr).unwrap_or("syscall?").to_string();
                pending = if f == "StaticClass" {
                    stack.clear();
                    pending_ty = None;
                    Some(format!("{}::StaticClass()", ctx.class_name.unwrap_or("UObject")))
                } else {
                    pending_ty = ctx.refs.func_ret_by_ptr(ptr).map(|d| d.base_name(ctx.refs));
                    build_call(&mut stack, &f, ctx.refs.is_method_by_ptr(ptr), ctx.super_ctor, ctx.refs.func_params_by_ptr(ptr), ctx.refs)
                };
            }
            "CallPtr" => {
                let f = name(w(ins, 0));
                pending_ty = None;
                pending = build_call(&mut stack, &f, false, ctx.super_ctor, None, ctx.refs);
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
                    out.push(format!("{} = {};", name(w(ins, 0)), p));
                }
                pending_ty = None;
            }
            "LOADOBJ" => obj_reg = Some(name(w(ins, 0))),
            "CpyVtoR4" | "CpyVtoR8" | "CpyVtoR1" => {
                ret_val = pending.take().or_else(|| Some(name(w(ins, 0))));
            }
            "RET" => {
                flush!();
                if let Some(v) = ret_val.take().or_else(|| obj_reg.take()).or_else(|| pending.take())
                    .or_else(|| scan_back_retval(ctx, lo + k))
                {
                    // cast an int return value to the function's bool/enum return type
                    let v = match ctx.ret_ty {
                        Some(rt) if looks_int(&v) => {
                            let tn = if rt.token == 0x41 { "bool".to_string() } else { rt.base_name(ctx.refs) };
                            cast_to_typename(&v, &tn).unwrap_or(v)
                        }
                        _ => v,
                    };
                    out.push(format!("return {v};"));
                } else {
                    // bare `return;` in a non-void function = the return value wasn't
                    // recovered ("Must return a value"). Mark unreliable -> stub fallback.
                    let non_void = ctx.ret_ty.map(|t| t.token != 0x52).unwrap_or(false);
                    if non_void {
                        out.push(format!("return {ARGMISMATCH};"));
                    } else {
                        out.push("return;".into());
                    }
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
            // Handle-copy (asBC stack_inc -2): pop the source pointer to BALANCE the operand
            // stack (the dominant phantom-arg cause). We deliberately do NOT emit `slot = src`:
            // the source is often a const/derived handle whose direct assignment to the dest
            // local fails to compile, and the value is re-read where it's actually used.
            "RefCpyV" | "REFCPY" => { stack.pop(); }
            // The TYPEID push is the implicit type operand of the following opCast/cast syscall
            // (NOT counted in the cache param list) — drop it so the cast block stays balanced.
            "TYPEID" => {}
            // primitive numeric conversions (iTOf/fTOi/dTOf/sbTOi/...): `dst = src` (the cast is
            // implicit in type-erased AS source).
            n2 if is_numeric_cast(n2) => {
                flush!();
                out.push(format!("{} = {};", name(w(ins, 0)), name(w(ins, 1))));
            }
            "SetV2" => {
                flush!();
                let bits = ins.dwords.first().copied().unwrap_or(0);
                set_consts.insert(w(ins, 0), ConstBits::W4(bits));
                out.push(format!("{} = {};", name(w(ins, 0)), bits as i32));
            }
            "CmpPtr" => cmp = Some(Cmp { a: name(w(ins, 0)), b: name(w(ins, 1)) }),
            "OBJTYPE" => stack.push(Arg::obj("objtype".into())), // +2: RTTI objtype ptr
            "STR" => stack.push(Arg::obj("\"\"".into())),         // +3: string-constant push
            "PshListElmnt" => stack.push(Arg::int(name(w(ins, 0)))), // +2: list element
            "COPY" | "Cast" => { stack.pop(); }                  // -2: pop the source ptr
            // ---- pure VM housekeeping / flow: ignore ----
            "SUSPEND" | "JitEntry" | "PopPtr" | "SwapPtr" | "ClrHi" | "ClrVPtr"
            | "FREE" | "FinConstruct" | "CHKREF" | "ChkRefS" | "ChkNullV" | "ChkNullS"
            | "Destruct" | "SaveReturnValue" | "ResolveObjectPtr" | "FreeNullV8" | "GETOBJ"
            | "GETOBJREF" | "GETREF" | "CopyScript" | "ThrowException"
            | "JMP" | "JZ" | "JNZ" | "JS" | "JNS" | "JP" | "JNP" | "JLowZ" | "JLowNZ" | "JMPP" => {}
            _ => {
                flush!();
                out.push(format!("// {} {}", n, operand_str(ins)));
            }
        }
    }
    flush!();
    out.retain(|s| !s.contains(UNRESOLVED)); // drop statements with an unresolved value
    (out, cmp)
}

/// Condition rendered for the branch being TAKEN, given the CMP operands + jump op.
fn branch_cond(cmp: &Option<Cmp>, jump: &str) -> String {
    let (a, b) = match cmp {
        Some(c) => (c.a.clone(), c.b.clone()),
        None => ("?".into(), "?".into()),
    };
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
        "DIVi" | "DIVi64" | "DIVf" | "DIVd" => "/",
        "MODi" | "MODi64" | "MODf" | "MODd" => "%",
        "BAND" | "BAND64" => "&",
        "BOR" | "BOR64" => "|",
        "BXOR" | "BXOR64" => "^",
        "BSLL" | "BSLL64" => "<<",
        "BSRA" | "BSRA64" => ">>",
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
