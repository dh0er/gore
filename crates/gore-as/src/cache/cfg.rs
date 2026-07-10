//! Control-flow graph (D3.1): split a function's instructions into basic blocks
//! and compute successor edges from jumps.
//!
//! Jump encoding (see `work/reversing/gore-as/findings/decompile-controlflow.md`):
//! the operand is a signed int32 in DWORDS, relative to the dword AFTER the jump
//! (`l_bc += DW + 2`, the jump being 2 dwords). So target_dw = jump_dw + 2 + off.
//! Conditional jumps (JZ/JNZ/JS/JNS/JP/JNP/JLowZ/JLowNZ) fall through on no-jump;
//! `JMP` is unconditional; `RET` ends a block with no successor; `JMPP` is a jump
//! table (successors unknown here — left empty, annotated by the caller).

use std::collections::BTreeSet;

use super::disasm::Instr;

/// JMPP dispatch-row shape check (Hazelight `rW_DW_ARG` form, 2 dwords): the DW operand is
/// `N-1` (max selector value); execution lands on row `jmpp_dw + 2 + 2*value`. The compiler
/// emits rows 0..N-2 as `JMP` trampolines; row N-1 is either a `JMP` trampoline or the START
/// of the last case body inlined in place (it is the highest row, so an inline body cannot
/// overlap another row). Returns the N row start offsets when the shape holds, else None
/// (leaving the JMPP successor-less, exactly the prior behavior).
fn jmpp_rows(instrs: &[Instr], idx: usize) -> Option<Vec<usize>> {
    let ins = &instrs[idx];
    let n = (*ins.dwords.first()? as usize).checked_add(1)?;
    let mut rows = Vec::with_capacity(n);
    for k in 0..n {
        let row = ins.offset_dw + 2 + 2 * k;
        let ri = instrs.get(idx + 1 + k)?;
        if ri.offset_dw != row {
            return None; // rows must be adjacent 2-dword instructions
        }
        if k + 1 < n && ri.op.name != "JMP" {
            return None; // all but the last row must be trampolines
        }
        rows.push(row);
    }
    Some(rows)
}

#[derive(Debug, Clone)]
pub struct BasicBlock {
    /// Dword offset where the block starts.
    pub start_dw: usize,
    /// Index range `[lo, hi)` into the instruction list.
    pub instr_lo: usize,
    pub instr_hi: usize,
    /// Successor block start-dword offsets.
    pub succs: Vec<usize>,
}

#[derive(Debug, Clone)]
pub struct Cfg {
    pub blocks: Vec<BasicBlock>,
}

fn is_uncond_jump(name: &str) -> bool {
    name == "JMP"
}
fn is_cond_jump(name: &str) -> bool {
    matches!(
        name,
        "JZ" | "JNZ" | "JS" | "JNS" | "JP" | "JNP" | "JLowZ" | "JLowNZ"
    )
}
fn is_return(name: &str) -> bool {
    name == "RET"
}

/// Jump target dword for a jump instruction (signed dword operand, rel to dword after).
fn jump_target(ins: &Instr) -> Option<usize> {
    let off = *ins.dwords.first()? as i32;
    let t = ins.offset_dw as i64 + 2 + off as i64;
    if t < 0 {
        None
    } else {
        Some(t as usize)
    }
}

/// Build the CFG from a function's decoded instructions.
pub fn build(instrs: &[Instr]) -> Cfg {
    if instrs.is_empty() {
        return Cfg { blocks: Vec::new() };
    }
    // index instrs by their dword offset
    let off_to_idx: std::collections::HashMap<usize, usize> =
        instrs.iter().enumerate().map(|(i, x)| (x.offset_dw, i)).collect();

    // 1) leaders
    let mut leaders: BTreeSet<usize> = BTreeSet::new();
    leaders.insert(instrs[0].offset_dw);
    for (i, ins) in instrs.iter().enumerate() {
        let n = ins.op.name;
        if is_uncond_jump(n) || is_cond_jump(n) {
            // Only honour a jump target that lands on a decoded instruction boundary;
            // a malformed operand pointing mid-instruction would later panic the
            // `off_to_idx[&start]` lookup. Skip it (the block just loses that edge).
            if let Some(t) = jump_target(ins) {
                if off_to_idx.contains_key(&t) {
                    leaders.insert(t);
                }
            }
            // instruction after the jump is a leader (fallthrough / dead)
            if let Some(next) = instrs.get(i + 1) {
                leaders.insert(next.offset_dw);
            }
        } else if is_return(n) {
            if let Some(next) = instrs.get(i + 1) {
                leaders.insert(next.offset_dw);
            }
        } else if n == "JMPP" && jmpp_rows(instrs, i).is_some() {
            // dispatch row 0 starts right after the JMPP; rows 1.. become leaders via the
            // trampoline JMPs' own next-instruction rule, and row targets via the JMP rule.
            if let Some(next) = instrs.get(i + 1) {
                leaders.insert(next.offset_dw);
            }
        }
    }

    // 2) blocks: from each leader to the next leader
    let leader_vec: Vec<usize> = leaders.iter().copied().collect();
    let mut blocks = Vec::new();
    for (li, &start) in leader_vec.iter().enumerate() {
        let lo = off_to_idx[&start];
        let hi = leader_vec
            .get(li + 1)
            .map(|&next| off_to_idx[&next])
            .unwrap_or(instrs.len());
        if lo >= hi {
            continue;
        }
        // successors from the LAST instruction of the block
        let last = &instrs[hi - 1];
        let n = last.op.name;
        let mut succs = Vec::new();
        if n == "JMPP" {
            // Verified switch-dispatch shape: successors = the N dispatch-row start offsets
            // (in selector-value order). Unverified shape: none (prior behavior — the
            // structurer's marker/stub path still catches the uncovered transfer).
            if let Some(rows) = jmpp_rows(instrs, hi - 1) {
                succs = rows.into_iter().filter(|r| off_to_idx.contains_key(r)).collect();
            }
        } else if is_return(n) {
            // none
        } else if is_uncond_jump(n) {
            if let Some(t) = jump_target(last).filter(|t| off_to_idx.contains_key(t)) {
                succs.push(t);
            }
        } else if is_cond_jump(n) {
            if let Some(t) = jump_target(last).filter(|t| off_to_idx.contains_key(t)) {
                succs.push(t);
            }
            if let Some(&fall) = leader_vec.get(li + 1) {
                succs.push(fall);
            }
        } else if let Some(&fall) = leader_vec.get(li + 1) {
            succs.push(fall);
        }
        blocks.push(BasicBlock { start_dw: start, instr_lo: lo, instr_hi: hi, succs });
    }
    Cfg { blocks }
}

impl Cfg {
    /// True if any edge is a back-edge (target <= source start) — i.e. a loop exists.
    pub fn has_back_edge(&self) -> bool {
        for b in &self.blocks {
            for &s in &b.succs {
                if s <= b.start_dw {
                    return true;
                }
            }
        }
        false
    }
}
