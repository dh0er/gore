//! `FAngelscriptPrecompiledDataType` (36 B) + rendering to a recompilable AS type.
//!
//! Layout (per `work/reversing/gore-as/findings/recompile-types.md`):
//! 6×bool(4B each) = [bIsReference, bIsObjectConst, bIsObjectHandle, bIsConstHandle
//! (=isReadOnly), bIsAuto, bIfHandleThenConst]; int64 TypeInfo.OldReference; int32 TokenType.
//! Object/class/array types carry TokenType == ttIdentifier(5) and the name comes from
//! `TypeReferences[TypeInfo]`; primitives carry a keyword token. `floatIsFloat64` in this
//! build → 0x51 renders `float`, 0x50 `float32`.

use super::refs::RefResolver;
use super::wire::{Cursor, WireError};

pub const DATA_TYPE_SIZE: usize = 36;

#[derive(Debug, Clone, Default)]
pub struct DataType {
    pub is_reference: bool,
    pub is_object_const: bool,
    pub is_object_handle: bool,
    pub is_read_only: bool, // bIsConstHandle: the value/handle itself is const
    pub is_auto: bool,
    pub if_handle_then_const: bool,
    pub type_info: i64,
    pub token: i32,
}

impl DataType {
    /// Read a 36-byte DataType from the cursor.
    pub fn read(c: &mut Cursor) -> Result<DataType, WireError> {
        Ok(DataType {
            is_reference: c.read_bool4()?,
            is_object_const: c.read_bool4()?,
            is_object_handle: c.read_bool4()?,
            is_read_only: c.read_bool4()?,
            is_auto: c.read_bool4()?,
            if_handle_then_const: c.read_bool4()?,
            type_info: c.read_i64()?,
            token: c.read_i32()?,
        })
    }

    /// Render to a recompilable AngelScript type string (without the param `&in/&out`,
    /// which the caller adds from ParameterFlags).
    pub fn render(&self, refs: &RefResolver) -> String {
        let mut base = if self.token == 5 {
            if self.is_auto {
                "auto".to_string()
            } else {
                refs.type_by_ptr(self.type_info)
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "auto".to_string())
            }
        } else {
            token_keyword(self.token).to_string()
        };
        if self.is_object_handle {
            base.push('@');
        }
        // AngelScript splits const into two flags; emit at most ONE leading `const`.
        if self.is_read_only || self.is_object_const {
            base = format!("const {base}");
        }
        base
    }

    /// Base type keyword/name without any leading `const` (for value lookups).
    pub fn base_name(&self, refs: &RefResolver) -> String {
        self.render(refs).trim_start_matches("const ").trim_end_matches('@').to_string()
    }
}

/// AngelScript keyword for a primitive `eTokenType` value (see recompile-types.md §1).
pub fn token_keyword(token: i32) -> &'static str {
    match token {
        0x41 => "bool",
        0x44 => "int",
        0x45 => "int8",
        0x46 => "int16",
        0x47 => "int64",
        0x4B => "uint",
        0x4C => "uint8",
        0x4D => "uint16",
        0x4E => "uint64",
        0x50 => "float32",
        0x51 => "float",
        0x52 => "void",
        0x5E => "double",
        0x3B => "?",
        _ => "auto",
    }
}
