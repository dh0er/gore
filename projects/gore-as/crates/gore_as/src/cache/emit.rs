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
use std::collections::HashSet;
use std::fmt::Write as _;

use super::disasm::disassemble;
use super::model::{Class, Func, Module};
use super::refs::RefResolver;
use super::structure::body_statements;
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
        match g.value {
            Some(v) => {
                let _ = writeln!(s, "const {base} {} = {};", g.name, render_const(&base, v));
            }
            None => {
                // a non-const global with a runtime init we didn't recover — emit a const stub
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
        let ty = f.ty.render(refs);
        if f.is_uproperty {
            let _ = writeln!(s, "    UPROPERTY()");
        }
        let _ = writeln!(s, "    {ty} {};", f.name);
    }
    if !c.fields.is_empty() {
        s.push('\n');
    }
    for ctor in &c.ctors {
        emit_function(s, ctor, refs, true, true, 1);
    }
    for m in &c.methods {
        emit_function(s, m, refs, true, false, 1);
    }
    let _ = writeln!(s, "}}\n");
}

fn emit_function(s: &mut String, f: &Func, refs: &RefResolver, is_method: bool, is_ctor: bool, depth: usize) {
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
    let body = body_statements(&fc, refs, depth + 1);

    if body_is_recoverable(&body) {
        // hoist local declarations
        for (slot, ty) in infer_locals(f, refs) {
            let _ = writeln!(s, "{ind}    {ty} local_{slot};");
        }
        s.push_str(&body);
    } else {
        // stub fallback so the module still compiles
        let _ = writeln!(s, "{ind}    // body not fully recovered — stub", );
        if ret != "void" {
            let _ = writeln!(s, "{ind}    {ret} __r; return __r;");
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

/// A body is "recoverable" if it has no raw opcode annotations or disasm errors.
fn body_is_recoverable(body: &str) -> bool {
    !body.lines().any(|l| {
        let t = l.trim_start();
        t.starts_with("// ") && (t.contains("disasm error") || is_opcode_comment(t))
    })
}

fn is_opcode_comment(t: &str) -> bool {
    // structure.rs annotates uncovered ops as "// <Mnemonic> ..."
    t.strip_prefix("// ")
        .and_then(|r| r.split_whitespace().next())
        .map(|w| w.chars().next().map(|c| c.is_ascii_uppercase()).unwrap_or(false)
            && w.chars().all(|c| c.is_ascii_alphanumeric()))
        .unwrap_or(false)
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
