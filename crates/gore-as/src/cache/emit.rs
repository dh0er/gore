//! Recompilable `.as` emitter: a parsed [`model::Module`] -> AngelScript source the
//! GAME compiler accepts (per `work/reversing/gore-as/findings/recompile-*.md`).
//!
//! Rules: flat top-level file (module name is the file PATH, not a namespace, so no
//! wrapper); no `import` (automaticImports=1); `class X : Super`; UFUNCTION()/UPROPERTY()
//! only when the stored flag is set; skip generator-synthesized symbols (StaticClass,
//! the class-name ctor wrapper). Function bodies come from the structured decompiler with
//! hoisted local declarations; bodies the decompiler can't recover fall back to a
//! signature-matched STUB so the module still compiles.

use std::collections::BTreeMap;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fmt::Write as _;
use std::sync::OnceLock;

/// Optional list (one `Class::method` or free `function` per line) of functions to force
/// into the stub fallback — for the handful the decompiler can't recover correctly that the
/// in-game compile feedback flagged (engine-object arg mismatches, float-overload ambiguity).
/// Path comes from `GORE_AS_STUBLIST`; absent => empty (no forced stubs).
fn force_stub_set() -> &'static HashSet<String> {
    static L: OnceLock<HashSet<String>> = OnceLock::new();
    L.get_or_init(|| {
        std::env::var("GORE_AS_STUBLIST")
            .ok()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .map(|s| s.lines().map(|l| l.trim().to_string()).filter(|l| !l.is_empty()).collect())
            .unwrap_or_default()
    })
}

use super::disasm::disassemble;
use super::model::{Class, Func, Module};
use super::refs::RefResolver;
use super::structure::{body_statements_ctor, RVODEF};
use super::types::token_keyword;
use super::walk_modules::FuncCode;

/// Emit a whole module as recompilable AngelScript.
pub fn emit_module(m: &Module, refs: &RefResolver) -> String {
    let mut s = String::new();
    let _ = writeln!(s, "// gore-as decompiled module: {} ({})", m.name, m.file);
    let _ = writeln!(s, "// NOTE: local names + string literals are not stored in the cache.\n");

    for e in &m.enums {
        let _ = writeln!(s, "enum {}", e.name);
        let _ = writeln!(s, "{{");
        let mut expect = 0i32;
        for (name, val) in &e.entries {
            if *val == expect {
                let _ = writeln!(s, "    {name},");
            } else {
                let _ = writeln!(s, "    {name} = {val},");
            }
            expect = val + 1;
        }
        let _ = writeln!(s, "}}\n");
    }

    for g in &m.globals {
        if g.name.starts_with("__") {
            continue; // generator-synthesized (e.g. __StaticType_X)
        }
        let base = g.ty.base_name(refs);
        if !is_primitive(&base) && !is_enum(&base) {
            // AngelScript globals MUST be const, but an FName/F-struct can't take `= 0`.
            // FName constants are almost always named after their value -> `n"Name"`; other
            // structs get a default-constructed const (their real value isn't recoverable).
            if base == "FName" {
                let _ = writeln!(s, "const FName {0} = n\"{0}\";", g.name);
            } else {
                let _ = writeln!(s, "const {base} {} = {base}();", g.name);
            }
            continue;
        }
        // a primitive/enum const; None = runtime init we didn't recover, use a stub value.
        let inner = match g.value {
            Some(v) => render_const(&base, v),
            None => default_for(&base),
        };
        // AngelScript rejects implicit int->enum, so an enum const must be cast: `EType(1)`.
        if is_enum(&base) {
            let _ = writeln!(s, "const {base} {} = {base}({inner});", g.name);
        } else {
            let _ = writeln!(s, "const {base} {} = {inner};", g.name);
        }
    }
    if !m.globals.is_empty() {
        s.push('\n');
    }

    let class_names: HashSet<&str> = m.classes.iter().map(|c| c.name.as_str()).collect();
    // class name -> its member (method/ctor) names, so a module-level function is only treated
    // as an already-emitted class member when it actually names one — not merely because its
    // namespace happens to match a class.
    let class_members: HashMap<&str, HashSet<&str>> = m
        .classes
        .iter()
        .map(|c| {
            let members: HashSet<&str> = c
                .methods
                .iter()
                .chain(c.ctors.iter())
                .map(|f| f.name.as_str())
                .collect();
            (c.name.as_str(), members)
        })
        .collect();

    for c in &m.classes {
        emit_class(&mut s, c, refs);
    }

    // free functions = module.functions that aren't generator-synthesized accessors
    let mut seen_free: HashSet<String> = HashSet::new();
    for f in &m.functions {
        if is_generated(f, &class_names, &class_members) || is_generated_spawn(f, refs) {
            continue;
        }
        if !seen_free.insert(format!("{}({})", f.name, render_params(f, refs))) {
            continue; // duplicate signature -> "function ... already exists"
        }
        emit_function(&mut s, f, refs, false, false, 0);
    }
    s
}

/// The AngelScript-UE binding auto-generates factory free functions for every actor/component
/// class. The cache also carries them as module functions, so emitting them duplicates the native
/// binding ("a function with the same name and parameters already exists" — un-stubbable, the
/// declaration itself collides). Skip the exact generated shapes:
///   - actor:     `<Actor> Spawn(const FVector&, const FRotator&, const FName&, bool, ULevel)`
///   - component: `<Comp> Get|GetOrCreate|Create(const AActor, const FName&)`
fn is_generated_spawn(f: &Func, refs: &RefResolver) -> bool {
    if !f.ret.is_object_handle {
        return false;
    }
    let p0 = f.params.first().map(|p| p.ty.base_name(refs));
    let p0 = p0.as_deref();
    if f.name == "Spawn" && f.params.len() == 5 && p0 == Some("FVector") {
        return true;
    }
    if matches!(f.name.as_str(), "Get" | "GetOrCreate" | "Create")
        && f.params.len() == 2
        && p0 == Some("AActor")
        && f.params.get(1).map(|p| p.ty.base_name(refs)).as_deref() == Some("FName")
    {
        return true;
    }
    // subsystem/singleton accessor: `<Subsystem> Get()` / `GetG1R()` (0 params, handle return).
    if matches!(f.name.as_str(), "Get" | "GetG1R") && f.params.is_empty() {
        return true;
    }
    false
}

fn emit_class(s: &mut String, c: &Class, refs: &RefResolver) {
    match &c.super_class {
        Some(sup) if !sup.is_empty() => {
            let _ = writeln!(s, "class {} : {}", c.name, sup);
        }
        _ => {
            let _ = writeln!(s, "class {}", c.name);
        }
    }
    let _ = writeln!(s, "{{");
    for f in &c.fields {
        // Drop a leading `const`: UE-AS UPROPERTY members aren't const-assignable, yet the
        // generated constructor assigns them — keeping `const` causes "Cannot assign" errors.
        let ty = f.ty.render(refs);
        let ty = ty.strip_prefix("const ").unwrap_or(&ty);
        if f.is_uproperty {
            let _ = writeln!(s, "    UPROPERTY()");
        }
        let _ = writeln!(s, "    {ty} {};", f.name);
    }
    if !c.fields.is_empty() {
        s.push('\n');
    }
    let super_name = c.super_class.as_deref().filter(|s| !s.is_empty());
    // field name -> base type name, so the decompiler can cast `this.field = <int>` assignments.
    let field_types: HashMap<String, String> =
        c.fields.iter().map(|f| (f.name.clone(), f.ty.base_name(refs))).collect();
    for ctor in &c.ctors {
        emit_function_ctor(s, ctor, refs, true, true, 1, super_name, Some(&field_types), Some(&c.name));
    }
    // Dedup methods by name+parameters: the cache can carry two entries that render to the same
    // signature (e.g. a const- and non-const-return overload that collapse once the meaningless
    // return `const` is stripped), which AngelScript rejects as "a function with the same name
    // and parameters already exists".
    let mut seen_sigs: HashSet<String> = HashSet::new();
    for m in &c.methods {
        // `__InitDefaults` (and other `__`-prefixed generator methods) set the CDO defaults
        // via raw `__StaticType_*` symbols and untyped literals we can't reconstruct offline;
        // they are auto-generated boilerplate, not hand-written script — skip them so the
        // class compiles. (Runtime UPROPERTY defaults are lost; real script logic is intact.)
        if m.name.starts_with("__") {
            continue;
        }
        if !seen_sigs.insert(format!("{}({})", m.name, render_params(m, refs))) {
            continue; // duplicate signature
        }
        emit_function_ctor(s, m, refs, true, false, 1, None, Some(&field_types), Some(&c.name));
    }
    let _ = writeln!(s, "}}\n");
}

fn emit_function(s: &mut String, f: &Func, refs: &RefResolver, is_method: bool, is_ctor: bool, depth: usize) {
    emit_function_ctor(s, f, refs, is_method, is_ctor, depth, None, None, None);
}

#[allow(clippy::too_many_arguments)]
fn emit_function_ctor(s: &mut String, f: &Func, refs: &RefResolver, is_method: bool, is_ctor: bool, depth: usize, super_ctor: Option<&str>, fields: Option<&HashMap<String, String>>, class_name: Option<&str>) {
    let ind = "    ".repeat(depth);
    // Strip a leading `const` from the return type: a return-by-value `const` is meaningless in
    // AngelScript, and the cache sets the const flag inconsistently between a base method and its
    // override -> "must have the same return type as in the base class". Stripping makes them match.
    let ret = f.ret.render(refs).trim_start_matches("const ").to_string();
    let params = render_params(f, refs);
    if f.is_ufunction {
        let _ = writeln!(s, "{ind}UFUNCTION()");
    }
    if is_ctor {
        let _ = writeln!(s, "{ind}{}({params})", f.name); // constructors have no return type
    } else {
        let _ = writeln!(s, "{ind}{ret} {}({params})", f.name);
    }
    let _ = writeln!(s, "{ind}{{");

    let fc = FuncCode {
        func: f.name.clone(),
        is_method,
        param_names: f.params.iter().map(|p| p.name.clone()).collect(),
        param_types: f.params.iter().map(|p| p.ty.clone()).collect(),
        ret: f.ret.clone(),
        bytecode: f.bytecode.clone(),
    };
    let param_types: Vec<String> = f.params.iter().map(|p| p.ty.base_name(refs)).collect();
    // object-local slot -> type name, so the decompiler can insert downcasts on stores.
    let mut local_types: HashMap<i32, String> = f.obj_locals.iter().map(|(slot, tinfo)| {
        let ty = super::types::DataType { token: 5, type_info: *tinfo, is_object_handle: true, ..Default::default() }.base_name(refs);
        (*slot, ty)
    }).collect();
    // consumer-side override: never-written arg slots get the type their callee expects (fixes
    // mis-typed default/optional-arg slots — FName->UAIState_DailyRoutine, TSubclassOf<X>->X).
    let slot_overrides = infer_slot_types(f, refs);
    for (slot, ty) in &slot_overrides {
        local_types.insert(*slot, ty.clone());
    }
    let body = body_statements_ctor(&fc, refs, depth + 1, super_ctor, Some(&f.ret), fields, Some(&param_types), class_name, Some(&local_types));
    // hoist every referenced local; infer_locals types what it can, the rest default to `int`
    // (a wrong type just becomes a compile error the in-game loop force-stubs, rather than the
    // whole function stubbing on an undeclared identifier).
    let used = used_locals(&body);
    let mut locals = infer_locals(f, refs);
    for &n in &used {
        locals.entry(n).or_insert_with(|| "int".to_string());
    }
    // declare never-written consumer-typed slots with their inferred type (not the cache's wrong one)
    for (slot, ty) in &slot_overrides {
        if used.contains(slot) {
            // Never let a consumer-derived `?` (the AngelScript template type, e.g. an opCast
            // out-param slot) clobber a concrete type already inferred for the slot — declaring
            // `? local_N;` is a syntax error that stubs the whole function. Keep the concrete
            // type (e.g. the opCast retype) when the override is the unusable `?`.
            if ty == "?" && locals.get(slot).map(|t| t != "?").unwrap_or(false) {
                continue;
            }
            locals.insert(*slot, ty.clone());
        }
    }
    // Drop locals never referenced in the body: `obj_locals` includes profiling temporaries like
    // FScopeCycleCounter / FStatID that the body never uses, and they have no default constructor,
    // so declaring an unused one fails ("No default constructor"). An unused declaration is dead.
    locals.retain(|slot, _| used.contains(slot));
    // arg slots the bytecode reads beyond the declared parameter list (the signature parse
    // undercounts some value-type / defaulted params). Declare them as `int` locals so the
    // body compiles instead of stubbing wholesale; a wrong type the in-game loop force-stubs.
    let mut oor_args: Vec<i32> =
        used_idents(&body, "arg").into_iter().filter(|&n| n as usize >= f.params.len()).collect();
    oor_args.sort_unstable();
    // §3.3 safety net: an unmapped `argN` (signature undercount / RVO-return slot) declared as
    // `int` breaks any member/operator use on it ("Illegal operation on 'int'"). Type it from
    // its CONSUMER instead — the RHS of `argN = <expr>` (a field/local/param whose type we know)
    // — so the declaration is member-compatible. Falls back to `int` when nothing is recoverable.
    let oor_arg_types = infer_oor_arg_types(&body, &oor_args, fields, &locals, &param_types);

    // force-stub functions the in-game compile flagged as unrecoverable (by Class::method).
    let qid = match class_name {
        Some(c) => format!("{c}::{}", f.name),
        None => f.name.clone(),
    };
    let reason = if force_stub_set().contains(&qid) || force_stub_set().contains(&f.name) {
        Some("forced".to_string())
    } else {
        stub_reason(&body, &locals, f.params.len(), ret == "void")
    };

    if reason.is_none() {
        // hoist local declarations; primitives must be initialized (AngelScript errors on
        // "may not be initialized"), objects/structs/handles default-construct themselves.
        for (slot, ty) in &locals {
            if is_primitive(ty) {
                let _ = writeln!(s, "{ind}    {ty} local_{slot} = {};", default_for(ty));
            } else {
                let _ = writeln!(s, "{ind}    {ty} local_{slot};");
            }
        }
        for n in &oor_args {
            match oor_arg_types.get(n) {
                Some(ty) if is_primitive(ty) => {
                    let _ = writeln!(s, "{ind}    {ty} arg{n} = {};", default_for(ty));
                }
                Some(ty) => {
                    // object/struct/handle local: default-constructs itself (no initializer).
                    let _ = writeln!(s, "{ind}    {ty} arg{n};");
                }
                None => {
                    let _ = writeln!(s, "{ind}    int arg{n} = 0;");
                }
            }
        }
        // RVODEF marks a return whose value couldn't be recovered: substitute a type-correct
        // default. A handle return defaults to `nullptr` (no local needed — and it sidesteps
        // "no default constructor" for engine object types); everything else uses a default
        // local `{ret} __r;` (works for primitives, enums and default-constructible structs).
        if body.contains(RVODEF) {
            // Object/AActor handles have no default constructor, so `{ret} __r;` fails to
            // compile — return `nullptr` directly (this build's null-handle literal, matching
            // PshNull/CmpPtrNull). `render` strips `@`, so detect handles via the DataType flag.
            if f.ret.is_object_handle {
                s.push_str(&body.replace(RVODEF, "nullptr"));
            } else {
                let _ = writeln!(s, "{ind}    {ret} __r;");
                s.push_str(&body.replace(RVODEF, "__r"));
            }
        } else {
            s.push_str(&body);
        }
    } else {
        // stub fallback so the module still compiles (reason recorded for aggregation)
        let _ = writeln!(s, "{ind}    // body not fully recovered — stub [{}]", reason.unwrap());
        // constructors must NOT return a value; everything else returns a default. An object
        // handle return defaults to `nullptr` (no default-constructor for engine object types;
        // `nullptr` is this build's null-handle literal — `null` parses as undeclared).
        if !is_ctor && ret != "void" {
            if f.ret.is_object_handle {
                let _ = writeln!(s, "{ind}    return nullptr;");
            } else {
                let _ = writeln!(s, "{ind}    {ret} __r; return __r;");
            }
        }
    }
    let _ = writeln!(s, "{ind}}}");
}

fn render_params(f: &Func, refs: &RefResolver) -> String {
    f.params
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let ty = p.ty.render(refs);
            // ParameterFlags: 1=in (asTM_INREF), 2=out, 3=inout (asTM_*); &-ref when reference
            let amp = if p.ty.is_reference {
                match p.flags & 3 {
                    2 => "&out ",
                    3 => "&inout ",
                    _ => "&in ",
                }
            } else {
                ""
            };
            let nm = if p.name.is_empty() { format!("arg{i}") } else { p.name.clone() };
            format!("{ty} {amp}{nm}")
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// A body is "recoverable" only if it has NO recovery gaps: the decompiler emits a
/// `// <mnemonic> ...` comment for every op it can't lower, an unresolved-operand
/// placeholder `?` (e.g. `if (? != ?)`) when a comparison/operand couldn't be recovered,
/// and may reference a `local_N` that wasn't inferred. Any of these is a syntax/semantic
/// error that aborts the module's parse, so such a function falls back to a clean stub.
/// Returns `None` if the body is recoverable, else `Some(reason)` for the first gap found —
/// the reason string is emitted in the stub comment so the stub causes can be aggregated.
fn stub_reason(body: &str, locals: &BTreeMap<i32, String>, param_count: usize, ret_is_void: bool) -> Option<String> {
    // an ARGMISMATCH sentinel (\u{2}<code>) — extract its cause code for aggregation.
    if let Some(i) = body.find('\u{2}') {
        let code: String = body[i + 1..].chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '_').collect();
        return Some(format!("argmismatch:{}", if code.is_empty() { "?" } else { &code }));
    }
    for l in body.lines() {
        let t = l.trim_start();
        if let Some(rest) = t.strip_prefix("// ") {
            // A structurer bailout (control-flow guard hit) leaves the body truncated, so it
            // MUST still stub — but it's not missing bytecode, so report a distinct cause
            // rather than mislabeling it `opcode-uncovered`.
            let reason = if rest.starts_with("disasm error") {
                "disasm-error"
            } else if rest.starts_with("<structurer bailout>") {
                "structurer-bailout"
            } else {
                "opcode-uncovered"
            };
            return Some(reason.into());
        }
        if t.contains("(? ") || t.contains(" ?)") || t.contains(" ? ") {
            return Some("unresolved-operand".into());
        }
    }
    if !used_locals(body).iter().all(|n| locals.contains_key(n)) {
        return Some("undeclared-local".into());
    }
    let _ = param_count; // out-of-range arg slots are hoisted as locals, not stubbed
    if !ret_is_void && !body.contains("return ") {
        return Some("no-return".into());
    }
    None
}

/// Indices of every `<prefix>N` identifier in a body, at an identifier boundary
/// (so `arg` does not match inside `Target`/`FArg`, and the trailing char isn't alnum).
fn used_idents(body: &str, prefix: &str) -> HashSet<i32> {
    let mut out = HashSet::new();
    let b = body.as_bytes();
    let pl = prefix.len();
    let is_ident = |c: u8| c.is_ascii_alphanumeric() || c == b'_';
    let mut i = 0;
    while i + pl < b.len() {
        if &b[i..i + pl] == prefix.as_bytes()
            && (i == 0 || !is_ident(b[i - 1]))
        {
            let mut j = i + pl;
            let start = j;
            while j < b.len() && b[j].is_ascii_digit() { j += 1; }
            if j > start && (j >= b.len() || !is_ident(b[j])) {
                if let Ok(n) = body[start..j].parse::<i32>() { out.insert(n); }
                i = j; continue;
            }
        }
        i += 1;
    }
    out
}

/// Slot indices of every `local_N` identifier referenced in a body.
fn used_locals(body: &str) -> HashSet<i32> {
    let mut out = HashSet::new();
    let b = body.as_bytes();
    let needle = b"local_";
    let mut i = 0;
    while i + needle.len() < b.len() {
        if &b[i..i + needle.len()] == needle {
            let mut j = i + needle.len();
            let start = j;
            while j < b.len() && b[j].is_ascii_digit() {
                j += 1;
            }
            if j > start {
                if let Ok(n) = body[start..j].parse::<i32>() {
                    out.insert(n);
                }
            }
            i = j;
        } else {
            i += 1;
        }
    }
    out
}

/// Consumer-side slot typing: a slot that is NEVER written but is passed as a call argument takes
/// the type that call's parameter expects (e.g. an optional/default-arg slot the cache mis-typed —
/// FName where UAIState_DailyRoutine is wanted, or TSubclassOf<X> where X is wanted). Returns an
/// override `slot -> type` ONLY for never-written slots with a single consistent consumer object
/// type, so it can never clobber a real producer type. Pairs args from the stack TOP (robust to the
/// cache counting the implicit `this` in a method's param list).
fn infer_slot_types(f: &Func, refs: &RefResolver) -> HashMap<i32, String> {
    let instrs = match disassemble(&f.bytecode) {
        Ok(i) => i,
        Err(_) => return HashMap::new(),
    };
    let w0 = |ins: &super::disasm::Instr| ins.words.first().map(|w| *w as i16 as i32);
    let writes = |op: &str| {
        op.starts_with("SetV") || op.starts_with("CpyVtoV") || op.starts_with("CpyRtoV")
            || op.starts_with("RDR") || op.contains("TO") || op == "STOREOBJ" || op == "PopRPtr"
            || op.starts_with("ADD") || op.starts_with("SUB") || op.starts_with("MUL")
            || op.starts_with("DIV") || op.starts_with("MOD") || op.starts_with("NEG")
            || op.starts_with("Inc") || op.starts_with("Dec") || op == "NOT"
            || op.starts_with("B") || op == "ALLOC"
    };
    let mut written: HashSet<i32> = HashSet::new();
    for ins in &instrs {
        if writes(ins.op.name) {
            if let Some(d) = w0(ins) {
                if d > 0 {
                    written.insert(d);
                }
            }
        }
    }
    let mut ostack: Vec<Option<i32>> = Vec::new();
    let mut cand: HashMap<i32, Option<String>> = HashMap::new(); // slot -> Some(type) | None(conflict)
    let mut pair = |ostack: &mut Vec<Option<i32>>, params: Option<&[super::types::DataType]>, is_method: bool, cand: &mut HashMap<i32, Option<String>>| {
        let Some(params) = params else { ostack.clear(); return; };
        let total = if is_method { params.len() + 1 } else { params.len() };
        let take = total.min(ostack.len());
        let popped = ostack.split_off(ostack.len() - take);
        // method: top popped entry is the receiver -> drop it; the rest are the user args
        let args = if is_method && !popped.is_empty() { &popped[..popped.len() - 1] } else { &popped[..] };
        // pair from the TOP: last arg <-> last param (so a leading `this` param, if the cache
        // counts it, never shifts the user-arg pairing).
        for (i, slot) in args.iter().rev().enumerate() {
            if let Some(s) = slot {
                if let Some(pt) = params.len().checked_sub(1 + i).and_then(|j| params.get(j)) {
                    let ty = pt.base_name(refs);
                    match cand.get(s) {
                        None => { cand.insert(*s, Some(ty)); }
                        Some(Some(prev)) if *prev != ty => { cand.insert(*s, None); }
                        _ => {}
                    }
                }
            }
        }
    };
    for ins in &instrs {
        match ins.op.name {
            "PshVPtr" | "PshV4" | "PshV8" | "PSF" => {
                let s = w0(ins).unwrap_or(0);
                ostack.push(if s > 0 { Some(s) } else { None });
            }
            "PshC4" | "PshC8" | "PshNull" | "PGA" | "PshGPtr" | "PshG4" | "PshRPtr" | "STR"
            | "TYPEID" | "OBJTYPE" | "PshListElmnt" => ostack.push(None),
            "CALL" | "CALLINTF" | "CALLBND" => {
                let id = ins.dwords.first().copied().unwrap_or(0) as i32;
                pair(&mut ostack, refs.func_params_by_id(id), refs.is_method_by_id(id), &mut cand);
            }
            "CALLSYS" | "Thiscall1" => {
                let ptr = ins.qwords.first().copied().unwrap_or(0) as i64;
                pair(&mut ostack, refs.func_params_by_ptr(ptr), refs.is_method_by_ptr(ptr), &mut cand);
            }
            _ => {}
        }
    }
    cand.into_iter()
        .filter_map(|(slot, ty)| match ty {
            Some(t) if !written.contains(&slot) && !is_primitive(t.trim_end_matches('@')) && t != "void" && !t.is_empty() => Some((slot, t)),
            _ => None,
        })
        .collect()
}

/// §3.3 consumer-driven typing of out-of-range `argN` slots. Scans the body for the RHS of an
/// `argN = <expr>` assignment (including `return argN = <expr>;`) and resolves `<expr>`'s type
/// from the maps we already have: `this.<field>` -> field type, `local_M` -> local type,
/// `<param>` -> param type. A type that supports member access makes `argN.Member` legal where a
/// bare `int` would not. Anything unresolved is left out (declared `int`, as before — no regression).
fn infer_oor_arg_types(
    body: &str,
    oor_args: &[i32],
    fields: Option<&HashMap<String, String>>,
    locals: &BTreeMap<i32, String>,
    param_types: &[String],
) -> HashMap<i32, String> {
    let mut out: HashMap<i32, String> = HashMap::new();
    if oor_args.is_empty() {
        return out;
    }
    // a primitive/enum int-ish RHS isn't worth retyping (int default already works); only adopt a
    // type that is NOT a bare primitive (i.e. a struct/handle/array the member access needs).
    let adopt = |out: &mut HashMap<i32, String>, n: i32, ty: String| {
        if !ty.is_empty() && !is_primitive(&ty) {
            out.entry(n).or_insert(ty);
        }
    };
    for line in body.lines() {
        let t = line.trim();
        let t = t.strip_prefix("return ").unwrap_or(t);
        // parse `argN = RHS;`
        let Some(rest) = t.strip_prefix("arg") else { continue };
        let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        let Ok(n) = digits.parse::<i32>() else { continue };
        if !oor_args.contains(&n) {
            continue;
        }
        let after = rest[digits.len()..].trim_start();
        let Some(rhs) = after.strip_prefix("= ") else { continue };
        let rhs = rhs.trim().trim_end_matches(';').trim();
        // `this.<field>` (single member hop) -> the field's type.
        if let Some(field) = rhs.strip_prefix("this.") {
            if !field.contains('.') && !field.contains('(') {
                if let Some(ty) = fields.and_then(|m| m.get(field)) {
                    adopt(&mut out, n, ty.clone());
                    continue;
                }
            }
        }
        // `local_M` -> that local's inferred type.
        if let Some(m) = rhs.strip_prefix("local_") {
            if let Ok(slot) = m.parse::<i32>() {
                if let Some(ty) = locals.get(&slot) {
                    adopt(&mut out, n, ty.clone());
                    continue;
                }
            }
        }
        // a bare parameter name -> its declared type (param_types is index-aligned to params,
        // but we only have names in the body; skip — names aren't carried here). Left for future.
        let _ = param_types;
    }
    out
}

/// Infer (slot, type) for primitive + object locals to hoist as declarations.
fn infer_locals(f: &Func, refs: &RefResolver) -> BTreeMap<i32, String> {
    let mut locals: BTreeMap<i32, String> = BTreeMap::new();
    let obj: BTreeMap<i32, i64> = f.obj_locals.iter().copied().collect();
    let instrs = match disassemble(&f.bytecode) {
        Ok(i) => i,
        Err(_) => return locals,
    };
    for ins in &instrs {
        let n = ins.op.name;
        // destination slot is the first word for writing ops
        let dst = ins.words.first().map(|w| *w as i16 as i32);
        let Some(dst) = dst else { continue };
        if dst <= 0 {
            continue; // params / this, not a local
        }
        let ty = if let Some(p) = obj.get(&dst) {
            // reuse DataType::render (template subtypes + @ only on ref types U*/A*)
            super::types::DataType {
                token: 5,
                type_info: *p,
                is_object_handle: true,
                ..Default::default()
            }
            .render(refs)
        } else if writes_float(n) {
            "float".to_string()
        } else if writes_double(n) {
            "double".to_string()
        } else if writes_int64(n) {
            "int64".to_string()
        } else if writes_int(n) {
            "int".to_string()
        } else {
            continue;
        };
        // A slot first written by an ambiguous constant (SetV4/SetV8 -> int/int64) may later
        // be written by a typed float/double op; let the wider/typed write refine the guess
        // instead of locking in the first. Object types (from `obj`) are never downgraded.
        let rank = |s: &str| match s {
            "int" => 1,
            "int64" => 2,
            "float" | "double" => 3,
            _ => 0, // object/other
        };
        match locals.get(&dst) {
            None => {
                locals.insert(dst, ty);
            }
            Some(prev) if rank(prev) > 0 && rank(&ty) > rank(prev) => {
                locals.insert(dst, ty);
            }
            _ => {}
        }
    }
    // opCast retype: a script-handle downcast `T@ dst = Cast<T>(src)` lowers to
    // `TYPEID <tid> ; PSF <dst> ; PshVPtr <src> ; CALLSYS opCast`, and the cache types the
    // out-slot `dst` as the AngelScript `?` template type. Declaring `? local_N;` is a syntax
    // error ("Expected expression value, instead found '?'") that stubs the whole function.
    // Retype `dst` to the cast's resolved target T (from the preceding TYPEID) so it declares
    // as e.g. `UGothicFinalDataGame local_N;` and the recovered `local_N = Cast<T>(src);`
    // type-checks. This is the declaration-side counterpart of the structure.rs opCast recovery.
    {
        let mut last_tid: Option<i32> = None;
        let mut last_psf: Option<i32> = None;
        for ins in &instrs {
            match ins.op.name {
                "TYPEID" => {
                    last_tid = ins.dwords.first().map(|d| *d as i32);
                    last_psf = None;
                }
                "PSF" => {
                    // first PSF after a TYPEID is the opCast out-slot destination
                    if last_tid.is_some() {
                        last_psf = ins.words.first().map(|w| *w as i16 as i32);
                    }
                }
                "CALLSYS" | "Thiscall1" => {
                    let ptr = ins.qwords.first().copied().unwrap_or(0) as i64;
                    if refs.func_by_ptr(ptr) == Some("opCast") {
                        if let (Some(tid), Some(dst)) = (last_tid, last_psf) {
                            if dst > 0 {
                                if let Some(t) = super::structure::resolve_cast_typeid(refs, tid) {
                                    if t.starts_with('U') || t.starts_with('A') {
                                        locals.insert(dst, t);
                                    }
                                }
                            }
                        }
                    }
                    last_tid = None;
                    last_psf = None;
                }
                _ => {}
            }
        }
    }
    let _ = token_keyword; // keep import used if obj path elided
    locals
}

fn writes_int(n: &str) -> bool {
    matches!(n, "SetV4" | "SetV1" | "ADDi" | "SUBi" | "MULi" | "DIVi" | "MODi" | "IncVi" | "DecVi"
        | "NEGi" | "BAND" | "BOR" | "BXOR" | "BSLL" | "BSRA" | "ADDIi" | "SUBIi" | "MULIi"
        | "CpyVtoR4" | "RDR4" | "CpyRtoV4"
        // conversions whose RESULT is a 32-bit int/uint (*TO i/u/b/w)
        | "fTOi" | "fTOu" | "sbTOi" | "swTOi" | "ubTOi" | "uwTOi" | "dTOi" | "dTOu"
        | "iTOb" | "iTOw" | "i64TOi")
}
fn writes_float(n: &str) -> bool {
    matches!(n, "ADDf" | "SUBf" | "MULf" | "DIVf" | "MODf" | "NEGf" | "IncVf" | "DecVf"
        | "ADDIf" | "SUBIf" | "MULIf"
        // conversions whose RESULT is float (*TO f)
        | "iTOf" | "uTOf" | "dTOf" | "i64TOf" | "u64TOf")
}
fn writes_double(n: &str) -> bool {
    matches!(n, "ADDd" | "SUBd" | "MULd" | "DIVd" | "MODd" | "NEGd"
        // conversions whose RESULT is double (*TO d)
        | "iTOd" | "uTOd" | "fTOd" | "i64TOd" | "u64TOd")
}
fn writes_int64(n: &str) -> bool {
    matches!(n, "SetV8" | "ADDi64" | "SUBi64" | "MULi64" | "DIVi64"
        // conversions whose RESULT is a 64-bit int/uint (*TO i64/u64)
        | "uTOi64" | "iTOi64" | "fTOi64" | "dTOi64" | "fTOu64" | "dTOu64")
}

/// Heuristic: a UE enum type name (`E` + uppercase), which is int-castable like a primitive.
fn is_enum(ty: &str) -> bool {
    let b = ty.as_bytes();
    b.len() >= 2 && b[0] == b'E' && b[1].is_ascii_uppercase()
}

/// True for AngelScript primitive scalar types (need an explicit initializer).
fn is_primitive(ty: &str) -> bool {
    matches!(ty,
        "bool" | "int" | "int8" | "int16" | "int64" | "uint" | "uint8" | "uint16" | "uint64"
        | "float" | "float32" | "double")
}

/// A default initializer literal for a base type.
fn default_for(ty: &str) -> String {
    match ty {
        "float" | "double" | "float32" => "0.0".into(),
        "bool" => "false".into(),
        _ => "0".into(),
    }
}

/// Render a global's stored u64 constant per its rendered type.
fn render_const(ty: &str, v: u64) -> String {
    match ty {
        // This build is `floatIsFloat64` (types.rs): token 0x51 `float` is a 64-bit value (the
        // full u64), like `double` — render it WITHOUT the `f` suffix (which would round to
        // 32-bit / pick the wrong literal). Only token 0x50 `float32` is 32-bit and takes `f`.
        // Matches structure.rs's `fmt_float` (no suffix for 64-bit).
        "float" | "double" => format!("{}", f64::from_bits(v)),
        "float32" => format!("{}f", f32::from_bits(v as u32)),
        "bool" => if v != 0 { "true".into() } else { "false".into() },
        // Render integers per their actual width AND signedness: an unsigned type must not be
        // emitted negative (e.g. uint64 0xffff…ffff as -1), and signed types sign-extend from
        // their own width (the value lives in the low bits of the stored u64).
        "uint64" => v.to_string(),
        "uint" => (v as u32).to_string(),
        "uint16" => (v as u16).to_string(),
        "uint8" => (v as u8).to_string(),
        "int64" => (v as i64).to_string(),
        "int16" => (v as i16).to_string(),
        "int8" => (v as i8).to_string(),
        // "int" and any other int-like fallback: 32-bit signed.
        _ => (v as i32).to_string(),
    }
}

/// Is this module-level function a generator-synthesized accessor (skip it)?
fn is_generated(
    f: &Func,
    class_names: &HashSet<&str>,
    class_members: &HashMap<&str, HashSet<&str>>,
) -> bool {
    if f.name == "StaticClass" || class_names.contains(f.name.as_str()) {
        return true;
    }
    // A function whose namespace is a class is the already-emitted method ONLY if the class
    // actually declares a member with this name; a genuine free function that merely shares the
    // namespace is kept (previously it was silently dropped).
    class_members
        .get(f.namespace.as_str())
        .is_some_and(|members| members.contains(f.name.as_str()))
}
