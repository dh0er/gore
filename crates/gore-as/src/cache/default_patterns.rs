//! Cache-only canonical initializer predicates and direct scalar-default bytecode patterns.
//!
//! This layer contains no schema, ancestry, selector, or mutation authority. Both inspection and
//! fingerprinting consume the same exact metadata and range matcher from here.

use sha2::{Digest, Sha256};

use super::disasm::Instr;
use super::types::DataType;
use super::walk_modules::{FuncCodeKind, FuncCodeSpan};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DefaultPattern {
    SetV1LoadThisWrtV1,
    SetV2LoadThisWrtV2,
    SetV4LoadThisWrtV4,
    SetV8LoadThisWrtV8,
}

impl DefaultPattern {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SetV1LoadThisWrtV1 => "set_v1_load_this_wrt_v1",
            Self::SetV2LoadThisWrtV2 => "set_v2_load_this_wrt_v2",
            Self::SetV4LoadThisWrtV4 => "set_v4_load_this_wrt_v4",
            Self::SetV8LoadThisWrtV8 => "set_v8_load_this_wrt_v8",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "set_v1_load_this_wrt_v1" => Some(Self::SetV1LoadThisWrtV1),
            "set_v2_load_this_wrt_v2" => Some(Self::SetV2LoadThisWrtV2),
            "set_v4_load_this_wrt_v4" => Some(Self::SetV4LoadThisWrtV4),
            "set_v8_load_this_wrt_v8" => Some(Self::SetV8LoadThisWrtV8),
            _ => None,
        }
    }

    pub const fn value_width(self) -> usize {
        match self {
            Self::SetV1LoadThisWrtV1 => 1,
            Self::SetV2LoadThisWrtV2 => 2,
            Self::SetV4LoadThisWrtV4 => 4,
            Self::SetV8LoadThisWrtV8 => 8,
        }
    }

    /// The complete serialized immediate width. SetV1/SetV2 still carry a full dword.
    pub const fn operand_width(self) -> usize {
        match self {
            Self::SetV8LoadThisWrtV8 => 8,
            _ => 4,
        }
    }

    pub(crate) const fn set_name(self) -> &'static str {
        match self {
            Self::SetV1LoadThisWrtV1 => "SetV1",
            Self::SetV2LoadThisWrtV2 => "SetV2",
            Self::SetV4LoadThisWrtV4 => "SetV4",
            Self::SetV8LoadThisWrtV8 => "SetV8",
        }
    }

    const fn write_name(self) -> &'static str {
        match self {
            Self::SetV1LoadThisWrtV1 => "WRTV1",
            Self::SetV2LoadThisWrtV2 => "WRTV2",
            Self::SetV4LoadThisWrtV4 => "WRTV4",
            Self::SetV8LoadThisWrtV8 => "WRTV8",
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct DirectDefaultWindow {
    pub(crate) pattern: DefaultPattern,
    pub(crate) instruction_index: usize,
    pub(crate) instruction_offset_dw: usize,
    pub(crate) operand_offset_dw: usize,
    pub(crate) owner_type_id: i32,
    pub(crate) member_offset: i32,
    pub(crate) context_sha256: String,
}

pub(crate) fn is_canonical_initializer_metadata(span: &FuncCodeSpan) -> bool {
    span.kind == FuncCodeKind::ClassMethod
        && span.method_table_valid
        && span.in_method_table
        && is_initializer_traits(span.function_traits)
        && span.code.is_method
        && span.code.func.ends_with("::__InitDefaults")
        && span.code.param_types.is_empty()
        && is_plain_void(&span.code.ret)
}

pub(crate) const fn is_initializer_traits(traits: i32) -> bool {
    matches!(traits, 0 | 0x20)
}

pub(crate) fn is_plain_void(value: &DataType) -> bool {
    !value.is_reference
        && !value.is_object_const
        && !value.is_object_handle
        && !value.is_read_only
        && !value.is_auto
        && !value.if_handle_then_const
        && value.type_info == 0
        && value.token == 0x52
}

pub(crate) fn is_branch(name: &str) -> bool {
    matches!(
        name,
        "JMP" | "JZ" | "JNZ" | "JS" | "JNS" | "JP" | "JNP" | "JMPP" | "JLowZ" | "JLowNZ"
    )
}

pub(crate) fn is_reachable_linear_initializer(instrs: &[Instr]) -> bool {
    if instrs
        .iter()
        .any(|instruction| is_branch(instruction.op.name))
    {
        return false;
    }
    let Some((last, prefix)) = instrs.split_last() else {
        return false;
    };
    last.op.name == "RET"
        && prefix
            .iter()
            .all(|instruction| !matches!(instruction.op.name, "RET" | "ThrowException"))
}

pub(crate) fn direct_default_windows(
    bytecode: &[i32],
    instrs: &[Instr],
) -> Vec<DirectDefaultWindow> {
    let mut result = Vec::new();
    for (instruction_index, triple) in instrs.windows(3).enumerate() {
        let set = &triple[0];
        let load = &triple[1];
        let write = &triple[2];
        let Some(pattern) = pattern_for_set(set.op.name) else {
            continue;
        };
        if load.op.name != "LoadThisR" || write.op.name != pattern.write_name() {
            continue;
        }
        let (Some(&set_slot), Some(&write_slot)) = (set.words.first(), write.words.first()) else {
            continue;
        };
        if set_slot != write_slot {
            continue;
        }
        let (Some(&member_offset), Some(&owner_type_id)) =
            (load.words.first(), load.dwords.first())
        else {
            continue;
        };
        let operand_offset_dw = set.offset_dw + 1;
        let Some(context_sha256) = context_hash(bytecode, triple, operand_offset_dw, pattern)
        else {
            continue;
        };
        result.push(DirectDefaultWindow {
            pattern,
            instruction_index,
            instruction_offset_dw: set.offset_dw,
            operand_offset_dw,
            owner_type_id: owner_type_id as i32,
            member_offset: member_offset as i32,
            context_sha256,
        });
    }
    result
}

pub(crate) fn immediate_bytes(bytecode: &[i32], offset_dw: usize, width: usize) -> Option<Vec<u8>> {
    if !width.is_multiple_of(4) {
        return None;
    }
    let words = bytecode.get(offset_dw..offset_dw.checked_add(width / 4)?)?;
    bytecode_words(words)
}

fn pattern_for_set(name: &str) -> Option<DefaultPattern> {
    match name {
        "SetV1" => Some(DefaultPattern::SetV1LoadThisWrtV1),
        "SetV2" => Some(DefaultPattern::SetV2LoadThisWrtV2),
        "SetV4" => Some(DefaultPattern::SetV4LoadThisWrtV4),
        "SetV8" => Some(DefaultPattern::SetV8LoadThisWrtV8),
        _ => None,
    }
}

fn context_hash(
    bytecode: &[i32],
    triple: &[Instr],
    operand_offset_dw: usize,
    pattern: DefaultPattern,
) -> Option<String> {
    let start = triple.first()?.offset_dw;
    let last = triple.last()?;
    let end = last.offset_dw.checked_add(last.op.size_dwords as usize)?;
    let mut bytes = bytecode_words(bytecode.get(start..end)?)?;
    let relative = operand_offset_dw.checked_sub(start)?.checked_mul(4)?;
    let value_end = relative.checked_add(pattern.operand_width())?;
    bytes.get_mut(relative..value_end)?.fill(0);
    Some(hex_sha256(&bytes))
}

fn bytecode_words(words: &[i32]) -> Option<Vec<u8>> {
    let capacity = words.len().checked_mul(4)?;
    let mut bytes = Vec::with_capacity(capacity);
    for word in words {
        bytes.extend_from_slice(&word.to_le_bytes());
    }
    Some(bytes)
}

fn hex_sha256(data: &[u8]) -> String {
    let digest = Sha256::digest(data);
    let mut output = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write as _;
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}
