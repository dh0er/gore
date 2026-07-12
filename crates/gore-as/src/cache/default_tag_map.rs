//! Raw discovery of the generated GameplayTag-to-float32 map-default bytecode shape.
//!
//! This module deliberately performs no semantic resolution and no mutation. A returned window
//! is only raw evidence: later code must still prove the initializer, target class ancestry,
//! declaring field and container schema, GameplayTag global, and exact `TMap::Add` signature
//! before it may expose a selector or change any bytes.

use sha2::{Digest, Sha256};

use super::disasm::{disassemble, DisasmError, Instr};

/// Exact encoded size of the only admitted raw window.
pub const RAW_TAG_MAP_WINDOW_DWORDS: usize = 12;

/// One exact, contiguous raw candidate:
///
/// `SetV4 value, immediate; PSF value; PshGPtr tag; PshVPtr this;`
/// `ADDSi member_offset, owner_type_id; CALLSYS callee`
///
/// Offsets, ids, and pointers are provenance only. None of them is a semantic selector.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawTagMapWindow {
    pub instruction_index: usize,
    pub instruction_offset_dw: usize,
    /// Dword containing the complete four-byte `SetV4` immediate.
    pub operand_offset_dw: usize,
    pub value_slot: u16,
    pub expected: [u8; 4],
    pub owner_type_id: i32,
    pub member_offset: i32,
    pub tag_global_ptr: i64,
    pub callee_func_ptr: i64,
    /// SHA-256 of all six instructions with only the four-byte immediate zeroed.
    pub context_sha256: String,
}

/// Disassemble one function and return only exact raw GameplayTag-map candidates.
pub fn scan_raw_tag_map_windows(bytecode: &[i32]) -> Result<Vec<RawTagMapWindow>, DisasmError> {
    let instrs = disassemble(bytecode)?;
    Ok(raw_tag_map_windows(bytecode, &instrs))
}

/// Match instructions produced from this same bytecode against the exact six-opcode wire shape.
/// Kept private so callers cannot pair an unrelated instruction list with different raw bytes.
fn raw_tag_map_windows(bytecode: &[i32], instrs: &[Instr]) -> Vec<RawTagMapWindow> {
    let mut result = Vec::new();
    for (instruction_index, six) in instrs.windows(6).enumerate() {
        let [set, value_address, tag, receiver, member, call] = six else {
            unreachable!("windows(6) always yields six instructions")
        };
        if set.op.name != "SetV4"
            || value_address.op.name != "PSF"
            || tag.op.name != "PshGPtr"
            || receiver.op.name != "PshVPtr"
            || member.op.name != "ADDSi"
            || call.op.name != "CALLSYS"
        {
            continue;
        }

        let (
            [value_slot],
            [immediate],
            [address_slot],
            [tag_global_ptr],
            [receiver_slot],
            [member_offset],
            [owner_type_id],
            [callee_func_ptr],
        ) = (
            set.words.as_slice(),
            set.dwords.as_slice(),
            value_address.words.as_slice(),
            tag.qwords.as_slice(),
            receiver.words.as_slice(),
            member.words.as_slice(),
            member.dwords.as_slice(),
            call.qwords.as_slice(),
        )
        else {
            continue;
        };
        if value_slot != address_slot || *receiver_slot != 0 {
            continue;
        }

        // Require the exact instruction sizes and contiguity, not merely six matching decoded
        // names. This also seals the relative immediate position used by a later CAS patcher.
        let Some(psf_offset) = set.offset_dw.checked_add(2) else {
            continue;
        };
        let Some(tag_offset) = set.offset_dw.checked_add(3) else {
            continue;
        };
        let Some(receiver_offset) = set.offset_dw.checked_add(6) else {
            continue;
        };
        let Some(member_offset_dw) = set.offset_dw.checked_add(7) else {
            continue;
        };
        let Some(call_offset) = set.offset_dw.checked_add(9) else {
            continue;
        };
        let Some(window_end) = set.offset_dw.checked_add(RAW_TAG_MAP_WINDOW_DWORDS) else {
            continue;
        };
        if value_address.offset_dw != psf_offset
            || tag.offset_dw != tag_offset
            || receiver.offset_dw != receiver_offset
            || member.offset_dw != member_offset_dw
            || call.offset_dw != call_offset
            || call.offset_dw.checked_add(call.op.size_dwords as usize) != Some(window_end)
        {
            continue;
        }

        let Some(operand_offset_dw) = set.offset_dw.checked_add(1) else {
            continue;
        };
        let Some(context_sha256) = context_hash(bytecode, set.offset_dw, operand_offset_dw) else {
            continue;
        };
        result.push(RawTagMapWindow {
            instruction_index,
            instruction_offset_dw: set.offset_dw,
            operand_offset_dw,
            value_slot: *value_slot,
            expected: immediate.to_le_bytes(),
            owner_type_id: *owner_type_id as i32,
            member_offset: i32::from(*member_offset),
            tag_global_ptr: *tag_global_ptr as i64,
            callee_func_ptr: *callee_func_ptr as i64,
            context_sha256,
        });
    }
    result
}

fn context_hash(bytecode: &[i32], start_dw: usize, operand_offset_dw: usize) -> Option<String> {
    let end_dw = start_dw.checked_add(RAW_TAG_MAP_WINDOW_DWORDS)?;
    let words = bytecode.get(start_dw..end_dw)?;
    let mut bytes = Vec::with_capacity(RAW_TAG_MAP_WINDOW_DWORDS * 4);
    for word in words {
        bytes.extend_from_slice(&word.to_le_bytes());
    }
    let relative = operand_offset_dw.checked_sub(start_dw)?.checked_mul(4)?;
    bytes.get_mut(relative..relative.checked_add(4)?)?.fill(0);
    Some(encode_hex(&Sha256::digest(bytes)))
}

fn encode_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut result = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        result.push(HEX[(byte >> 4) as usize] as char);
        result.push(HEX[(byte & 0x0f) as usize] as char);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    const TAG_PTR: u64 = 0x024b_dd09_8bc8;
    const CALLEE_PTR: u64 = 0x024b_ef73_8200;
    const OWNER_TYPE_ID: u32 = 0x0400_1269;

    fn op(opcode: u8, word: u16) -> i32 {
        (u32::from(opcode) | (u32::from(word) << 16)) as i32
    }

    fn qword_op(opcode: u8, value: u64, out: &mut Vec<i32>) {
        out.extend([
            i32::from(opcode),
            value as u32 as i32,
            (value >> 32) as u32 as i32,
        ]);
    }

    fn exact_window() -> Vec<i32> {
        let mut code = vec![op(77, 2), 0x4120_0000]; // SetV4 w2, 10.0f
        code.push(op(4, 2)); // PSF w2
        qword_op(1, TAG_PTR, &mut code); // PshGPtr tag
        code.push(op(48, 0)); // PshVPtr this
        code.extend([op(79, 800), OWNER_TYPE_ID as i32]); // ADDSi field, owner
        qword_op(61, CALLEE_PTR, &mut code); // CALLSYS callee
        code
    }

    #[test]
    fn finds_exact_sword_shaped_raw_window() {
        let windows = scan_raw_tag_map_windows(&exact_window()).unwrap();
        assert_eq!(windows.len(), 1);
        let window = &windows[0];
        assert_eq!(window.instruction_index, 0);
        assert_eq!(window.instruction_offset_dw, 0);
        assert_eq!(window.operand_offset_dw, 1);
        assert_eq!(window.value_slot, 2);
        assert_eq!(window.expected, 10.0f32.to_le_bytes());
        assert_eq!(window.owner_type_id, OWNER_TYPE_ID as i32);
        assert_eq!(window.member_offset, 800);
        assert_eq!(window.tag_global_ptr, TAG_PTR as i64);
        assert_eq!(window.callee_func_ptr, CALLEE_PTR as i64);
        assert_eq!(
            window.context_sha256,
            "d02d0b0a7bd68cdae2d2e04b530fa959a94c2270cf178d406f64c474f1840312"
        );
    }

    #[test]
    fn preserves_arbitrary_raw_ids_without_claiming_semantics() {
        let mut code = exact_window();
        code[4] = 0x5566_7788;
        code[5] = 0x1122_3344;
        code[8] = 0x7654_3210;
        code[10] = 0x0bad_f00d;
        code[11] = 0x0123_4567;
        let window = scan_raw_tag_map_windows(&code).unwrap().pop().unwrap();
        assert_eq!(window.tag_global_ptr as u64, 0x1122_3344_5566_7788);
        assert_eq!(window.owner_type_id as u32, 0x7654_3210);
        assert_eq!(window.callee_func_ptr as u64, 0x0123_4567_0bad_f00d);
    }

    #[test]
    fn rejects_slot_receiver_opcode_and_contiguity_drift() {
        let mut cases = Vec::new();

        let mut slot_mismatch = exact_window();
        slot_mismatch[2] = op(4, 3);
        cases.push(("slot mismatch", slot_mismatch));

        let mut non_this_receiver = exact_window();
        non_this_receiver[6] = op(48, 1);
        cases.push(("non-this receiver", non_this_receiver));

        let mut non_global_tag = exact_window();
        non_global_tag[3] = i32::from(7); // PshG4, same qword wire width
        cases.push(("non-pointer tag push", non_global_tag));

        let mut non_callsys = exact_window();
        non_callsys[9] = i32::from(200); // Thiscall1, same qword wire width
        cases.push(("non-CALLSYS callee", non_callsys));

        let mut interrupted = exact_window();
        interrupted.insert(2, 0); // PopPtr between SetV4 and PSF
        cases.push(("interrupted window", interrupted));

        for (name, code) in cases {
            assert!(
                scan_raw_tag_map_windows(&code).unwrap().is_empty(),
                "{name} must fail closed"
            );
        }
    }

    #[test]
    fn rejects_truncated_bytecode_before_scanning() {
        let mut code = exact_window();
        code.pop();
        assert!(matches!(
            scan_raw_tag_map_windows(&code),
            Err(DisasmError::Truncated { .. })
        ));
    }
}
