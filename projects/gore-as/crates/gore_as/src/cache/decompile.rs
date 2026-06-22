//! Decompiler (D3): asBC instructions -> readable AngelScript-ish source.
//!
//! AngelScript is a register + local-variable machine (see
//! `work/reversing/gore-as/findings/decompile-*.md`): values live in
//! frame-relative dword SLOTS (`l_fp - operand`), one valueRegister, and a
//! reference register (for member access). This module simulates that to rebuild
//! expressions, then emits statements. It is best-effort and *annotates* opcodes
//! it doesn't model rather than failing — output is for reading, not recompiling.
//!
//! Current scope: straight-line expression bodies (assignments, member access,
//! arithmetic, returns, simple calls). Control-flow structuring (if/while/for)
//! is a follow-up (D3.3); branches are emitted as labelled gotos for now.

use std::collections::HashMap;
use std::fmt::Write as _;

use super::disasm::disassemble;
use super::refs::RefResolver;
use super::walk_modules::FuncCode;

/// AngelScript value-pointer size in dwords on x64 (AS_PTR_SIZE).
const AS_PTR_SIZE: i32 = 2;

#[derive(Clone)]
enum Expr {
    This,
    Var(String),
    Member(Box<Expr>, String),
    Const(i64),
    Bin(&'static str, Box<Expr>, Box<Expr>),
    Unary(&'static str, Box<Expr>),
    Call(String, Vec<Expr>),
}

impl Expr {
    fn render(&self) -> String {
        match self {
            Expr::This => "this".into(),
            Expr::Var(s) => s.clone(),
            Expr::Member(o, f) => format!("{}.{}", o.render(), f),
            Expr::Const(c) => c.to_string(),
            Expr::Bin(op, a, b) => format!("({} {} {})", a.render(), op, b.render()),
            Expr::Unary(op, a) => format!("{}{}", op, a.render()),
            Expr::Call(n, args) => {
                let a: Vec<String> = args.iter().map(|e| e.render()).collect();
                format!("{}({})", n, a.join(", "))
            }
        }
    }
}

/// Decompile one function body to a string (signature + statements).
pub fn decompile_function(f: &FuncCode, refs: &RefResolver) -> String {
    let instrs = match disassemble(&f.bytecode) {
        Ok(i) => i,
        Err(e) => return format!("// {} — disasm error: {e}\n", f.func),
    };

    let mut slots: HashMap<i32, Expr> = HashMap::new();
    let mut value_reg: Option<Expr> = None;
    let mut ref_reg: Option<Expr> = None;
    let mut arg_stack: Vec<Expr> = Vec::new();
    let mut body = String::new();

    let slot_name = |off: i32| -> Expr {
        if off > 0 {
            return Expr::Var(format!("local_{off}"));
        }
        // Methods carry an implicit `this` at slot 0 with params at -AS_PTR_SIZE, -AS_PTR_SIZE-1, …
        // Free functions have no `this`: params start at slot 0 (0, -1, -2, …). See structure.rs.
        let idx = if f.is_method {
            if off == 0 {
                return Expr::This;
            }
            (-off - AS_PTR_SIZE) as usize
        } else {
            (-off) as usize
        };
        match f.param_names.get(idx) {
            Some(n) if !n.is_empty() => Expr::Var(n.clone()),
            _ => Expr::Var(format!("arg{idx}")),
        }
    };
    let get = |slots: &HashMap<i32, Expr>, off: i32| -> Expr {
        slots.get(&off).cloned().unwrap_or_else(|| slot_name(off))
    };
    // signed 16-bit operand -> slot offset
    let s16 = |w: u16| w as i16 as i32;

    for ins in &instrs {
        let name = ins.op.name;
        match name {
            // --- member reference ---
            "LoadThisR" => {
                // W=byte offset, DW=type-id
                let off = ins.words.first().copied().unwrap_or(0) as i32;
                let tid = ins.dwords.first().copied().unwrap_or(0) as i32;
                let field = refs
                    .member(tid, off)
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| format!("field_0x{off:x}"));
                ref_reg = Some(Expr::Member(Box::new(Expr::This), field));
            }
            "LoadRObjR" => {
                // wW=object slot, W=offset, DW=type-id
                let obj = get(&slots, s16(ins.words.first().copied().unwrap_or(0)));
                let off = ins.words.get(1).copied().unwrap_or(0) as i32;
                let tid = ins.dwords.first().copied().unwrap_or(0) as i32;
                let field = refs
                    .member(tid, off)
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| format!("field_0x{off:x}"));
                ref_reg = Some(Expr::Member(Box::new(obj), field));
            }
            // --- deref reference register into a slot ---
            n if n.starts_with("RDR") => {
                let dst = s16(ins.words.first().copied().unwrap_or(0));
                if let Some(r) = &ref_reg {
                    slots.insert(dst, r.clone());
                }
            }
            // --- value-register copies ---
            n if n.starts_with("CpyVtoR") => {
                let src = s16(ins.words.first().copied().unwrap_or(0));
                value_reg = Some(get(&slots, src));
            }
            n if n.starts_with("CpyRtoV") => {
                let dst = s16(ins.words.first().copied().unwrap_or(0));
                if let Some(v) = &value_reg {
                    slots.insert(dst, v.clone());
                }
            }
            // --- constants ---
            // SetV8 is `wW_QW_ARG`: its 64-bit immediate lives in qwords, not dwords.
            "SetV8" => {
                let dst = s16(ins.words.first().copied().unwrap_or(0));
                let c = ins.qwords.first().copied().unwrap_or(0) as i64;
                slots.insert(dst, Expr::Const(c));
            }
            n if n.starts_with("SetV") => {
                let dst = s16(ins.words.first().copied().unwrap_or(0));
                let c = ins.dwords.first().copied().unwrap_or(0) as i32 as i64;
                slots.insert(dst, Expr::Const(c));
            }
            "PshC4" => arg_stack.push(Expr::Const(
                ins.dwords.first().copied().unwrap_or(0) as i32 as i64,
            )),
            "PshV4" | "PshV8" | "PSF" | "PshVPtr" => {
                arg_stack.push(get(&slots, s16(ins.words.first().copied().unwrap_or(0))))
            }
            // --- binary arithmetic (dst, s1, s2) ---
            _ if bin_op(name).is_some() => {
                let op = bin_op(name).unwrap();
                let dst = s16(ins.words.first().copied().unwrap_or(0));
                let a = get(&slots, s16(ins.words.get(1).copied().unwrap_or(0)));
                let b = get(&slots, s16(ins.words.get(2).copied().unwrap_or(0)));
                slots.insert(dst, Expr::Bin(op, Box::new(a), Box::new(b)));
            }
            // --- unary negation (rW) ---
            "NEGi" | "NEGf" | "NEGd" => {
                let s = s16(ins.words.first().copied().unwrap_or(0));
                let e = get(&slots, s);
                slots.insert(s, Expr::Unary("-", Box::new(e)));
            }
            // --- calls ---
            "CALL" | "CALLINTF" | "CALLBND" => {
                let id = ins.dwords.first().copied().unwrap_or(0) as i32;
                let fname = refs.func_by_id(id).unwrap_or("func?").to_string();
                let args = std::mem::take(&mut arg_stack);
                value_reg = Some(Expr::Call(fname, args));
            }
            "CALLSYS" | "Thiscall1" => {
                let ptr = ins.qwords.first().copied().unwrap_or(0) as i64;
                let fname = refs.func_by_ptr(ptr).unwrap_or("syscall?").to_string();
                let args = std::mem::take(&mut arg_stack);
                value_reg = Some(Expr::Call(fname, args));
            }
            // Indirect function-pointer call: `CallPtr` is rW_ARG, so its callee is the slot
            // value in `words` (not a qword pointer) — render that expression as the callee.
            "CallPtr" => {
                let slot = s16(ins.words.first().copied().unwrap_or(0));
                let callee = get(&slots, slot).render();
                let args = std::mem::take(&mut arg_stack);
                value_reg = Some(Expr::Call(callee, args));
            }
            // --- returns ---
            "RET" => {
                match &value_reg {
                    Some(v) => {
                        let _ = writeln!(body, "    return {};", v.render());
                    }
                    None => {
                        let _ = writeln!(body, "    return;");
                    }
                }
            }
            // --- ignore frame/flow housekeeping silently ---
            "SUSPEND" | "JitEntry" | "PopPtr" | "ClrHi" | "SwapPtr" => {}
            // --- everything else: annotate (don't fail) ---
            _ => {
                let _ = writeln!(body, "    // {} {}", name, operand_str(ins));
            }
        }
    }

    let params: Vec<String> = f
        .param_names
        .iter()
        .enumerate()
        .map(|(i, n)| if n.is_empty() { format!("arg{i}") } else { n.clone() })
        .collect();
    format!("// {}\nfunction({})\n{{\n{}}}\n", f.func, params.join(", "), body)
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
        "BSRL" | "BSRL64" | "BSRA" | "BSRA64" => ">>",
        _ => return None,
    })
}

fn operand_str(ins: &super::disasm::Instr) -> String {
    let mut parts = Vec::new();
    for w in &ins.words {
        parts.push(format!("w{}", *w as i16));
    }
    for d in &ins.dwords {
        parts.push(format!("0x{d:x}"));
    }
    for q in &ins.qwords {
        parts.push(format!("0x{q:x}"));
    }
    parts.join(", ")
}
