//! Disassembler: asBC bytecode (a function's `int32` dword stream) -> instruction list.
//!
//! Decodes per the ISA in [`super::isa`]. Each instruction starts on a dword
//! boundary: byte 0 of dword 0 = opcode; 16-bit word args sit in word slots
//! (d0 high, d1 low, d1 high); 32-bit args at dword 1/2; 64-bit args span two
//! dwords. Layout per `BcType` (see `isa.rs` field-offset docs / bytecode-isa.md).

use super::isa::{BcType, OpInfo, OPCODES};

/// Maximum number of decoded instructions in one function.
///
/// The 2026-08-14 Shipping-cache census found a real maximum of 389,358
/// instructions (852,642 dwords) in
/// `Map.MainMap.WorldPointManagerConfig_MainMap.UWorldpointManagerConfig_MainMap::__InitDefaults`.
/// This limit therefore retains about 2.69x instruction-count headroom.
pub const MAX_DISASSEMBLED_INSTRUCTIONS: usize = 1_048_576;

/// Maximum conservatively projected heap use for one decoded function.
///
/// On the Win64 target each [`Instr`] occupies 88 bytes. The projection also
/// charges 64 bytes for every non-empty operand `Vec`, covering its small
/// payload and allocator overhead. The largest function in the audited
/// Shipping cache projects to 58,114,576 bytes (55.42 MiB), leaving about
/// 2.31x headroom below this limit.
pub const MAX_DISASSEMBLY_PROJECTED_HEAP_BYTES: usize = 128 * 1024 * 1024;

const PROJECTED_NONEMPTY_OPERAND_VEC_BYTES: usize = 64;
const INSTRUCTION_RESOURCE: &str = "instructions";
const PROJECTED_HEAP_RESOURCE: &str = "projected heap bytes";

#[derive(Debug, Clone, Copy)]
struct DisasmLimits {
    max_instructions: usize,
    max_projected_heap_bytes: usize,
}

const PRODUCTION_LIMITS: DisasmLimits = DisasmLimits {
    max_instructions: MAX_DISASSEMBLED_INSTRUCTIONS,
    max_projected_heap_bytes: MAX_DISASSEMBLY_PROJECTED_HEAP_BYTES,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DisasmPreflight {
    instruction_count: usize,
    projected_heap_bytes: usize,
}

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
    UnknownOpcode {
        offset_dw: usize,
        opcode: u8,
    },
    Truncated {
        offset_dw: usize,
        need: usize,
        have: usize,
    },
    ResourceLimit {
        resource: &'static str,
        offset_dw: usize,
        actual: usize,
        limit: usize,
    },
    AllocationFailed {
        requested_instructions: usize,
    },
}

impl std::fmt::Display for DisasmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DisasmError::UnknownOpcode { offset_dw, opcode } => {
                write!(f, "unknown opcode {opcode} at dword {offset_dw}")
            }
            DisasmError::Truncated {
                offset_dw,
                need,
                have,
            } => {
                write!(
                    f,
                    "truncated instruction at dword {offset_dw}: need {need} dwords, have {have}"
                )
            }
            DisasmError::ResourceLimit {
                resource,
                offset_dw,
                actual,
                limit,
            } => write!(
                f,
                "disassembly resource limit exceeded for {resource} at dword {offset_dw}: {actual} > {limit}"
            ),
            DisasmError::AllocationFailed {
                requested_instructions,
            } => write!(
                f,
                "failed to allocate disassembly output for {requested_instructions} instructions"
            ),
        }
    }
}

impl std::error::Error for DisasmError {}

const fn nonempty_operand_vec_count(fmt: BcType) -> usize {
    use BcType::*;
    match fmt {
        INFO | NO_ARG => 0,
        rW_DW_ARG | wW_DW_ARG | wW_QW_ARG | wW_rW_DW_ARG | QW_DW_ARG | rW_QW_ARG | W_DW_ARG
        | rW_W_DW_ARG | rW_DW_DW_ARG => 2,
        W_ARG | wW_ARG | DW_ARG | QW_ARG | DW_DW_ARG | wW_rW_rW_ARG | wW_rW_ARG | rW_ARG
        | rW_rW_ARG | wW_W_ARG | W_rW_ARG => 1,
    }
}

const fn projected_instruction_heap_bytes(fmt: BcType) -> usize {
    std::mem::size_of::<Instr>()
        + nonempty_operand_vec_count(fmt) * PROJECTED_NONEMPTY_OPERAND_VEC_BYTES
}

/// Validate and size a disassembly without allocating instruction or operand
/// storage. Malformed-bytecode errors deliberately win over a resource error
/// for the same instruction, preserving the decoder's diagnostic order.
fn preflight_disassembly(
    bytecode: &[i32],
    limits: DisasmLimits,
) -> Result<DisasmPreflight, DisasmError> {
    let mut instruction_count = 0usize;
    let mut projected_heap_bytes = 0usize;
    let mut i = 0usize;
    while i < bytecode.len() {
        let opcode = (bytecode[i] as u32 & 0xFF) as u8;
        let op = OPCODES
            .get(opcode as usize)
            .ok_or(DisasmError::UnknownOpcode {
                offset_dw: i,
                opcode,
            })?;
        let size = op.size_dwords as usize;
        let have = bytecode.len() - i;
        if size == 0 || size > have {
            return Err(DisasmError::Truncated {
                offset_dw: i,
                need: size.max(1),
                have,
            });
        }

        let next_instruction_count = instruction_count.saturating_add(1);
        if next_instruction_count > limits.max_instructions {
            return Err(DisasmError::ResourceLimit {
                resource: INSTRUCTION_RESOURCE,
                offset_dw: i,
                actual: next_instruction_count,
                limit: limits.max_instructions,
            });
        }

        let next_projected_heap_bytes =
            projected_heap_bytes.saturating_add(projected_instruction_heap_bytes(op.fmt));
        if next_projected_heap_bytes > limits.max_projected_heap_bytes {
            return Err(DisasmError::ResourceLimit {
                resource: PROJECTED_HEAP_RESOURCE,
                offset_dw: i,
                actual: next_projected_heap_bytes,
                limit: limits.max_projected_heap_bytes,
            });
        }

        instruction_count = next_instruction_count;
        projected_heap_bytes = next_projected_heap_bytes;
        i += size;
    }

    Ok(DisasmPreflight {
        instruction_count,
        projected_heap_bytes,
    })
}

/// Decode a function's bytecode into a flat instruction list.
pub fn disassemble(bytecode: &[i32]) -> Result<Vec<Instr>, DisasmError> {
    use BcType::*;
    let preflight = preflight_disassembly(bytecode, PRODUCTION_LIMITS)?;
    let mut out = Vec::new();
    out.try_reserve_exact(preflight.instruction_count)
        .map_err(|_| DisasmError::AllocationFailed {
            requested_instructions: preflight.instruction_count,
        })?;
    let mut i = 0usize;
    while i < bytecode.len() {
        let opcode = (bytecode[i] as u32 & 0xFF) as u8;
        let op = OPCODES
            .get(opcode as usize)
            .ok_or(DisasmError::UnknownOpcode {
                offset_dw: i,
                opcode,
            })?;
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
        out.push(Instr {
            offset_dw: i,
            op,
            words,
            dwords,
            qwords,
        });
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

#[cfg(test)]
mod tests {
    use super::*;

    fn limits(max_instructions: usize, max_projected_heap_bytes: usize) -> DisasmLimits {
        DisasmLimits {
            max_instructions,
            max_projected_heap_bytes,
        }
    }

    #[test]
    fn preflight_counts_zero_one_and_two_operand_vecs_at_exact_boundaries() {
        let zero_vec_bytes = std::mem::size_of::<Instr>();
        let one_vec_bytes = zero_vec_bytes + PROJECTED_NONEMPTY_OPERAND_VEC_BYTES;
        let two_vec_bytes = zero_vec_bytes + 2 * PROJECTED_NONEMPTY_OPERAND_VEC_BYTES;

        let assert_projection = |bytecode: &[i32], projected_heap_bytes: usize| {
            assert_eq!(
                preflight_disassembly(bytecode, limits(1, projected_heap_bytes)),
                Ok(DisasmPreflight {
                    instruction_count: 1,
                    projected_heap_bytes,
                })
            );

            let error =
                preflight_disassembly(bytecode, limits(1, projected_heap_bytes - 1)).unwrap_err();
            assert_eq!(
                error,
                DisasmError::ResourceLimit {
                    resource: PROJECTED_HEAP_RESOURCE,
                    offset_dw: 0,
                    actual: projected_heap_bytes,
                    limit: projected_heap_bytes - 1,
                }
            );
            assert_eq!(
                error.to_string(),
                format!(
                    "disassembly resource limit exceeded for projected heap bytes at dword 0: {projected_heap_bytes} > {}",
                    projected_heap_bytes - 1
                )
            );
        };

        // PopPtr has no operands, PSF one non-empty operand Vec, and LdGRdR4 two.
        assert_projection(&[0], zero_vec_bytes);
        assert_projection(&[4], one_vec_bytes);
        assert_projection(&[8, 0, 0], two_vec_bytes);

        let bytecode = [0, 4, 8, 0, 0];
        assert_eq!(
            preflight_disassembly(&bytecode, limits(2, usize::MAX)),
            Err(DisasmError::ResourceLimit {
                resource: INSTRUCTION_RESOURCE,
                offset_dw: 2,
                actual: 3,
                limit: 2,
            })
        );
    }

    #[test]
    fn malformed_instruction_diagnostics_precede_resource_limits() {
        let no_resources = limits(0, 0);
        assert_eq!(
            preflight_disassembly(&[255], no_resources),
            Err(DisasmError::UnknownOpcode {
                offset_dw: 0,
                opcode: 255,
            })
        );
        assert_eq!(
            preflight_disassembly(&[1], no_resources),
            Err(DisasmError::Truncated {
                offset_dw: 0,
                need: 3,
                have: 1,
            })
        );

        let one_instruction = limits(1, usize::MAX);
        assert_eq!(
            preflight_disassembly(&[0, 255], one_instruction),
            Err(DisasmError::UnknownOpcode {
                offset_dw: 1,
                opcode: 255,
            })
        );
        assert_eq!(
            preflight_disassembly(&[0, 1], one_instruction),
            Err(DisasmError::Truncated {
                offset_dw: 1,
                need: 3,
                have: 1,
            })
        );
    }

    #[test]
    fn public_disassemble_refuses_max_plus_one_compact_instructions() {
        let bytecode = vec![0; MAX_DISASSEMBLED_INSTRUCTIONS + 1];
        let error = disassemble(&bytecode).unwrap_err();
        assert_eq!(
            error,
            DisasmError::ResourceLimit {
                resource: INSTRUCTION_RESOURCE,
                offset_dw: MAX_DISASSEMBLED_INSTRUCTIONS,
                actual: MAX_DISASSEMBLED_INSTRUCTIONS + 1,
                limit: MAX_DISASSEMBLED_INSTRUCTIONS,
            }
        );
        assert_eq!(
            error.to_string(),
            "disassembly resource limit exceeded for instructions at dword 1048576: 1048577 > 1048576"
        );
    }
}
