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
use super::walk_modules::FuncCode;

const AS_PTR_SIZE: i32 = 2;

/// Decompile a function to structured AngelScript-ish source.
pub fn decompile(f: &FuncCode, refs: &RefResolver) -> String {
    let instrs = match disassemble(&f.bytecode) {
        Ok(i) => i,
        Err(e) => return format!("// {} — disasm error: {e}\n", f.func),
    };
    let g = cfg::build(&instrs);
    let ctx = Ctx { f, refs, instrs: &instrs };

    // index blocks by start offset, in order
    let order: Vec<usize> = g.blocks.iter().map(|b| b.start_dw).collect();
    let idx_of: HashMap<usize, usize> = order.iter().enumerate().map(|(i, &o)| (o, i)).collect();

    let mut body = String::new();
    let mut st = Structurer { ctx: &ctx, g: &g, idx_of: &idx_of };
    st.emit_range(0, g.blocks.len(), 1, &mut body);

    let params: Vec<String> = f
        .param_names
        .iter()
        .enumerate()
        .map(|(i, n)| if n.is_empty() { format!("arg{i}") } else { n.clone() })
        .collect();
    format!("// {}\nfunction({})\n{{\n{}}}\n", f.func, params.join(", "), body)
}

struct Ctx<'a> {
    f: &'a FuncCode,
    refs: &'a RefResolver,
    instrs: &'a [Instr],
}

impl Ctx<'_> {
    fn slot_name(&self, off: i32) -> String {
        if self.f.is_method {
            if off == 0 {
                return "this".into();
            }
            if off < 0 {
                let idx = (-off - AS_PTR_SIZE) as usize;
                if let Some(n) = self.f.param_names.get(idx) {
                    if !n.is_empty() {
                        return n.clone();
                    }
                }
            }
        } else if off <= 0 {
            let idx = (-off) as usize;
            if let Some(n) = self.f.param_names.get(idx) {
                if !n.is_empty() {
                    return n.clone();
                }
            }
        }
        if off > 0 {
            format!("local_{off}")
        } else {
            format!("arg_{}", -off)
        }
    }
}

fn s16(w: u16) -> i32 {
    w as i16 as i32
}

/// A recovered comparison (from CMP* operands), pending a conditional jump.
#[derive(Clone)]
struct Cmp {
    a: String,
    b: String,
}

/// Decompile one block's instruction range into statements; also return the
/// pending comparison (operands of the last CMP*) for condition recovery.
fn block_stmts(ctx: &Ctx, lo: usize, hi: usize) -> (Vec<String>, Option<Cmp>) {
    let mut out = Vec::new();
    let mut cmp: Option<Cmp> = None;
    let mut ref_reg: Option<String> = None;
    let mut ret_val: Option<String> = None;
    let name = |off: i32| ctx.slot_name(off);
    let w = |ins: &Instr, i: usize| s16(ins.words.get(i).copied().unwrap_or(0));

    for ins in &ctx.instrs[lo..hi] {
        let n = ins.op.name;
        match n {
            "SetV4" | "SetV8" | "SetV1" => {
                let c = ins.dwords.first().copied().unwrap_or(0) as i32;
                out.push(format!("{} = {};", name(w(ins, 0)), c));
            }
            "LoadThisR" => {
                let off = ins.words.first().copied().unwrap_or(0) as i32;
                let tid = ins.dwords.first().copied().unwrap_or(0) as i32;
                let field = ctx.refs.member(tid, off).map(|s| s.to_string()).unwrap_or_else(|| format!("field_0x{off:x}"));
                ref_reg = Some(format!("this.{field}"));
            }
            "LoadRObjR" => {
                let obj = name(w(ins, 0));
                let off = ins.words.get(1).copied().unwrap_or(0) as i32;
                let tid = ins.dwords.first().copied().unwrap_or(0) as i32;
                let field = ctx.refs.member(tid, off).map(|s| s.to_string()).unwrap_or_else(|| format!("field_0x{off:x}"));
                ref_reg = Some(format!("{obj}.{field}"));
            }
            _ if n.starts_with("RDR") => {
                if let Some(r) = &ref_reg {
                    out.push(format!("{} = {};", name(w(ins, 0)), r));
                }
            }
            _ if n.starts_with("WRTV") => {
                if let Some(r) = &ref_reg {
                    out.push(format!("{} = {};", r, name(w(ins, 0))));
                }
            }
            // binary: dst, s1, s2
            _ if bin_op(n).is_some() && ins.words.len() >= 3 => {
                out.push(format!(
                    "{} = {} {} {};",
                    name(w(ins, 0)),
                    name(w(ins, 1)),
                    bin_op(n).unwrap(),
                    name(w(ins, 2))
                ));
            }
            // inline-const binary: dst, src, const
            _ if iconst_op(n).is_some() => {
                let c = ins.dwords.first().copied().unwrap_or(0) as i32;
                out.push(format!(
                    "{} = {} {} {};",
                    name(w(ins, 0)),
                    name(w(ins, 1)),
                    iconst_op(n).unwrap(),
                    c
                ));
            }
            "IncVi" | "IncVf" => out.push(format!("{0} = {0} + 1;", name(w(ins, 0)))),
            "DecVi" | "DecVf" => out.push(format!("{0} = {0} - 1;", name(w(ins, 0)))),
            "NEGi" | "NEGf" | "NEGd" => out.push(format!("{0} = -{0};", name(w(ins, 0)))),
            "CMPi" | "CMPu" | "CMPf" | "CMPd" | "CMPi64" | "CMPu64" => {
                cmp = Some(Cmp { a: name(w(ins, 0)), b: name(w(ins, 1)) });
            }
            "CMPIi" | "CMPIf" | "CMPIu" => {
                let c = ins.dwords.first().copied().unwrap_or(0) as i32;
                cmp = Some(Cmp { a: name(w(ins, 0)), b: c.to_string() });
            }
            "CpyVtoR4" | "CpyVtoR8" => ret_val = Some(name(w(ins, 0))),
            "CALL" | "CALLINTF" | "CALLBND" => {
                let id = ins.dwords.first().copied().unwrap_or(0) as i32;
                let fname = ctx.refs.func_by_id(id).unwrap_or("func?");
                out.push(format!("{}(...);", fname));
            }
            "CALLSYS" | "Thiscall1" | "CallPtr" => {
                let ptr = ins.qwords.first().copied().unwrap_or(0) as i64;
                let fname = ctx.refs.func_by_ptr(ptr).unwrap_or("syscall?");
                out.push(format!("{}(...);", fname));
            }
            // housekeeping / flow — ignore (jumps handled by the structurer)
            "SUSPEND" | "JitEntry" | "PopPtr" | "SwapPtr" | "ClrHi" | "JMP" | "JZ" | "JNZ"
            | "JS" | "JNS" | "JP" | "JNP" | "JLowZ" | "JLowNZ" | "JMPP" => {}
            "RET" => {
                if let Some(v) = &ret_val {
                    out.push(format!("return {};", v));
                } else {
                    out.push("return;".into());
                }
            }
            _ => out.push(format!("// {} {}", n, operand_str(ins))),
        }
    }
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
