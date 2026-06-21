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
            if let Some(t) = jump_target(ins) {
                leaders.insert(t);
            }
            // instruction after the jump is a leader (fallthrough / dead)
            if let Some(next) = instrs.get(i + 1) {
                leaders.insert(next.offset_dw);
            }
        } else if is_return(n) {
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
        if is_return(n) || n == "JMPP" {
            // RET: none; JMPP: table targets unknown here
        } else if is_uncond_jump(n) {
            if let Some(t) = jump_target(last) {
                succs.push(t);
            }
        } else if is_cond_jump(n) {
            if let Some(t) = jump_target(last) {
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
