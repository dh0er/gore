//! Disassembler: asBC bytecode (a function's `int32` dword stream) -> instruction list.
//!
//! Decodes per the ISA in [`super::isa`]. Each instruction starts on a dword
//! boundary: byte 0 of dword 0 = opcode; 16-bit word args sit in word slots
//! (d0 high, d1 low, d1 high); 32-bit args at dword 1/2; 64-bit args span two
//! dwords. Layout per `BcType` (see `isa.rs` field-offset docs / bytecode-isa.md).

use super::isa::{BcType, OpInfo, OPCODES};

/// One decoded instruction. Operand values are collected positionally into
/// `words` (16-bit), `dwords` (32-bit), `qwords` (64-bit) in source order.
#[derive(Debug, Clone)]
pub struct Instr {
    /// Dword offset of this instruction within the function bytecode.
    pub offset_dw: usize,
    pub op: &'static OpInfo,
    pub words: Vec<u16>,
    pub dwords: Vec<u32>,
    pub qwords: Vec<u64>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum DisasmError {
    UnknownOpcode { offset_dw: usize, opcode: u8 },
    Truncated { offset_dw: usize, need: usize, have: usize },
}

impl std::fmt::Display for DisasmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DisasmError::UnknownOpcode { offset_dw, opcode } => {
                write!(f, "unknown opcode {opcode} at dword {offset_dw}")
            }
            DisasmError::Truncated { offset_dw, need, have } => {
                write!(f, "truncated instruction at dword {offset_dw}: need {need} dwords, have {have}")
            }
        }
    }
}

/// Decode a function's bytecode into a flat instruction list.
pub fn disassemble(bytecode: &[i32]) -> Result<Vec<Instr>, DisasmError> {
    use BcType::*;
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < bytecode.len() {
        let opcode = (bytecode[i] as u32 & 0xFF) as u8;
        let op = OPCODES
            .get(opcode as usize)
            .ok_or(DisasmError::UnknownOpcode { offset_dw: i, opcode })?;
        let size = op.size_dwords as usize;
        if size == 0 || i + size > bytecode.len() {
            return Err(DisasmError::Truncated {
                offset_dw: i,
                need: size.max(1),
                have: bytecode.len() - i,
            });
        }
        let g = |k: usize| bytecode[i + k] as u32;
        let w_hi = |dw: u32| ((dw >> 16) & 0xFFFF) as u16;
        let w_lo = |dw: u32| (dw & 0xFFFF) as u16;
        let qw = |lo: u32, hi: u32| lo as u64 | ((hi as u64) << 32);

        let mut words = Vec::new();
        let mut dwords = Vec::new();
        let mut qwords = Vec::new();
        match op.fmt {
            INFO | NO_ARG => {}
            W_ARG | wW_ARG | rW_ARG => words.push(w_hi(g(0))),
            DW_ARG => dwords.push(g(1)),
            rW_DW_ARG | wW_DW_ARG | W_DW_ARG => {
                words.push(w_hi(g(0)));
                dwords.push(g(1));
            }
            QW_ARG => qwords.push(qw(g(1), g(2))),
            DW_DW_ARG => {
                dwords.push(g(1));
                dwords.push(g(2));
            }
            wW_rW_rW_ARG => {
                words.push(w_hi(g(0)));
                words.push(w_lo(g(1)));
                words.push(w_hi(g(1)));
            }
            wW_QW_ARG | rW_QW_ARG => {
                words.push(w_hi(g(0)));
                qwords.push(qw(g(1), g(2)));
            }
            wW_rW_ARG | rW_rW_ARG | wW_W_ARG | W_rW_ARG => {
                words.push(w_hi(g(0)));
                words.push(w_lo(g(1)));
            }
            wW_rW_DW_ARG | rW_W_DW_ARG => {
                words.push(w_hi(g(0)));
                words.push(w_lo(g(1)));
                dwords.push(g(2));
            }
            QW_DW_ARG => {
                qwords.push(qw(g(1), g(2)));
                dwords.push(g(3));
            }
            rW_DW_DW_ARG => {
                words.push(w_hi(g(0)));
                dwords.push(g(1));
                dwords.push(g(2));
            }
        }
        out.push(Instr { offset_dw: i, op, words, dwords, qwords });
        i += size;
    }
    Ok(out)
}

/// A human-readable listing (one instruction per line).
pub fn listing(instrs: &[Instr]) -> String {
    let mut s = String::new();
    for ins in instrs {
        let mut args = Vec::new();
        for w in &ins.words {
            args.push(format!("w{w}"));
        }
        for d in &ins.dwords {
            args.push(format!("0x{d:x}"));
        }
        for q in &ins.qwords {
            args.push(format!("0x{q:x}"));
        }
        s.push_str(&format!(
            "  {:04}  {:<12} {}\n",
            ins.offset_dw,
            ins.op.name,
            args.join(", ")
        ));
    }
    s
}
