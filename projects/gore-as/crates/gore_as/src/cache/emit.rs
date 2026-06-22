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
        match g.value {
            Some(v) => {
                let _ = writeln!(s, "const {base} {} = {};", g.name, render_const(&base, v));
            }
            None => {
                // a primitive/enum const whose runtime init we didn't recover — stub value
                let _ = writeln!(s, "const {base} {} = {};", g.name, default_for(&base));
            }
        }
    }
    if !m.globals.is_empty() {
        s.push('\n');
    }

    let class_names: HashSet<&str> = m.classes.iter().map(|c| c.name.as_str()).collect();

    for c in &m.classes {
        emit_class(&mut s, c, refs);
    }

    // free functions = module.functions that aren't generator-synthesized accessors
    for f in &m.functions {
        if is_generated(f, &class_names) {
            continue;
        }
        emit_function(&mut s, f, refs, false, false, 0);
    }
    s
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
    for m in &c.methods {
        // `__InitDefaults` (and other `__`-prefixed generator methods) set the CDO defaults
        // via raw `__StaticType_*` symbols and untyped literals we can't reconstruct offline;
        // they are auto-generated boilerplate, not hand-written script — skip them so the
        // class compiles. (Runtime UPROPERTY defaults are lost; real script logic is intact.)
        if m.name.starts_with("__") {
            continue;
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
    let ret = f.ret.render(refs);
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
        bytecode: f.bytecode.clone(),
    };
    let param_types: Vec<String> = f.params.iter().map(|p| p.ty.base_name(refs)).collect();
    // object-local slot -> type name, so the decompiler can insert downcasts on stores.
    let local_types: HashMap<i32, String> = f.obj_locals.iter().map(|(slot, tinfo)| {
        let ty = super::types::DataType { token: 5, type_info: *tinfo, is_object_handle: true, ..Default::default() }.base_name(refs);
        (*slot, ty)
    }).collect();
    let body = body_statements_ctor(&fc, refs, depth + 1, super_ctor, Some(&f.ret), fields, Some(&param_types), class_name, Some(&local_types));
    // hoist every referenced local; infer_locals types what it can, the rest default to `int`
    // (a wrong type just becomes a compile error the in-game loop force-stubs, rather than the
    // whole function stubbing on an undeclared identifier).
    let mut locals = infer_locals(f, refs);
    for n in used_locals(&body) {
        locals.entry(n).or_insert_with(|| "int".to_string());
    }
    // arg slots the bytecode reads beyond the declared parameter list (the signature parse
    // undercounts some value-type / defaulted params). Declare them as `int` locals so the
    // body compiles instead of stubbing wholesale; a wrong type the in-game loop force-stubs.
    let mut oor_args: Vec<i32> =
        used_idents(&body, "arg").into_iter().filter(|&n| n as usize >= f.params.len()).collect();
    oor_args.sort_unstable();

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
            let _ = writeln!(s, "{ind}    int arg{n} = 0;");
        }
        // RVODEF marks a return whose value couldn't be recovered: substitute a type-correct
        // default. A handle return defaults to `null` (no local needed — and it sidesteps
        // "no default constructor" for engine object types); everything else uses a default
        // local `{ret} __r;` (works for primitives, enums and default-constructible structs).
        if body.contains(RVODEF) {
            // Object/AActor handles have no default constructor, so `{ret} __r;` fails to
            // compile — return `null` directly. `render` strips `@`, so detect handles via
            // the DataType flag, not the rendered string.
            if f.ret.is_object_handle {
                s.push_str(&body.replace(RVODEF, "null"));
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
        // handle return defaults to `null` (no default-constructor for engine object types).
        if !is_ctor && ret != "void" {
            if f.ret.is_object_handle {
                let _ = writeln!(s, "{ind}    return null;");
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
            return Some(if rest.starts_with("disasm error") { "disasm-error" } else { "opcode-uncovered" }.into());
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
        locals.entry(dst).or_insert(ty);
    }
    let _ = token_keyword; // keep import used if obj path elided
    locals
}

fn writes_int(n: &str) -> bool {
    matches!(n, "SetV4" | "SetV1" | "ADDi" | "SUBi" | "MULi" | "DIVi" | "MODi" | "IncVi" | "DecVi"
        | "NEGi" | "BAND" | "BOR" | "BXOR" | "BSLL" | "BSRA" | "ADDIi" | "SUBIi" | "MULIi"
        | "CpyVtoR4" | "RDR4" | "i2f" | "f2i" | "CpyRtoV4")
}
fn writes_float(n: &str) -> bool {
    matches!(n, "ADDf" | "SUBf" | "MULf" | "DIVf" | "MODf" | "NEGf" | "IncVf" | "DecVf"
        | "ADDIf" | "SUBIf" | "MULIf")
}
fn writes_double(n: &str) -> bool {
    matches!(n, "ADDd" | "SUBd" | "MULd" | "DIVd" | "MODd" | "NEGd")
}
fn writes_int64(n: &str) -> bool {
    matches!(n, "SetV8" | "ADDi64" | "SUBi64" | "MULi64" | "DIVi64")
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
        "float" => format!("{}f", f64::from_bits(v)),
        "double" => format!("{}", f64::from_bits(v)),
        "float32" => format!("{}f", f32::from_bits(v as u32)),
        "bool" => if v != 0 { "true".into() } else { "false".into() },
        _ => (v as i64).to_string(),
    }
}

/// Is this module-level function a generator-synthesized accessor (skip it)?
fn is_generated(f: &Func, class_names: &HashSet<&str>) -> bool {
    f.name == "StaticClass"
        || class_names.contains(f.name.as_str())
        || class_names.contains(f.namespace.as_str())
}
