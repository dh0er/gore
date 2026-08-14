//! The 7 GLOBAL tail tables that follow the module list (case-a merge support).
//!
//! Layout per `work/reversing/gore-as/findings/case-a-tables-and-exec.md`:
//! ```text
//! 0 TypeReferences              TMap<int64, FAngelscriptTypeReference>
//! 1 TypeIdReferenceToPointer    TMap<int32, int64>
//! 2 FunctionReferences          TMap<int64, FAngelscriptFunctionReference>
//! 3 FunctionIdReferenceToPointer TMap<int32, int64>
//! 4 GlobalReferences            TMap<int64, FAngelscriptGlobalReference>
//! 5 StaticNames                 TArray<FStringInArchive>
//! 6 PropertyReferences          TMap<int64, FAngelscriptPropertyReference>
//! ```

use super::wire::{Cursor, WireError};

const DATA_TYPE_SIZE: usize = 36;
pub const N_TABLES: usize = 7;

/// One parsed tail table: its entry byte range (excluding the count prefix) + int64 keys.
#[derive(Debug, Clone)]
pub struct TableSpan {
    pub count: u32,
    /// Offset just after the `int32 count`.
    pub entries_start: usize,
    /// Offset just after the last entry.
    pub entries_end: usize,
    /// Entry keys (int64 for tables 0/2/4/6; the int32 key widened for tables 1/3; empty for table 5).
    pub keys: Vec<i64>,
    /// Byte offset where each entry begins (parallel to `keys`). Entry `i` spans
    /// `entry_starts[i] .. entry_starts[i+1]` (or `entries_end` for the last) — lets a merge
    /// drop a single variable-width colliding entry. Empty for table 5 (StaticNames).
    pub entry_starts: Vec<usize>,
}

#[derive(Debug, Clone)]
pub struct TailTables {
    pub start: usize,
    pub end: usize,
    pub tables: Vec<TableSpan>, // length N_TABLES
}

fn read_type_reference(c: &mut Cursor) -> Result<(), WireError> {
    c.read_sia()?; // Name
    c.read_sia()?; // Module
    c.read_sia()?; // Namespace
    c.skip_tarray_fixed(DATA_TYPE_SIZE, "TypeRef.SubTypes")?;
    Ok(())
}

fn read_function_reference(c: &mut Cursor) -> Result<(), WireError> {
    c.read_sia()?; // Name
    c.read_sia()?; // Module
    c.read_sia()?; // Namespace
    c.skip(4 * 3)?; // bIsConst, bIsImportedDecl, bIsMethod
    c.skip(8)?; // ObjectType.OldReference
    c.skip_tarray_fixed(DATA_TYPE_SIZE, "FuncRef.ParameterTypes")?;
    c.skip(DATA_TYPE_SIZE)?; // ReturnType (single inline DataType)
    Ok(())
}

fn read_global_reference(c: &mut Cursor) -> Result<(), WireError> {
    // The game writes only string-literal names with AssignAsUTF8. The discriminator follows the
    // three strings, so retain the validated bytes until it is available instead of guessing from
    // the payload.
    let name_pos = c.pos();
    let name = c.read_sia_bytes()?; // Name
    c.read_sia()?; // Module
    c.read_sia()?; // Namespace
    if c.read_bool4()? {
        name.decode_utf8(name_pos)?;
    }
    Ok(())
}

fn read_property_reference(c: &mut Cursor) -> Result<(), WireError> {
    c.read_sia()?; // Name
    c.skip(4)?; // OldTypeId
    Ok(())
}

/// Read an `int64`-keyed TMap; `read_value` consumes one value.
fn read_i64_map(
    c: &mut Cursor,
    field: &'static str,
    minimum_value_bytes: usize,
    mut read_value: impl FnMut(&mut Cursor) -> Result<(), WireError>,
) -> Result<TableSpan, WireError> {
    let count = c.read_count(field)?;
    c.ensure_minimum_remaining(count, 8 + minimum_value_bytes, field)?;
    let entries_start = c.pos();
    let mut keys = Vec::new();
    let mut entry_starts = Vec::new();
    for _ in 0..count {
        entry_starts.push(c.pos());
        keys.push(c.read_i64()?);
        read_value(c)?;
    }
    Ok(TableSpan {
        count: count as u32,
        entries_start,
        entries_end: c.pos(),
        keys,
        entry_starts,
    })
}

/// Read a `TMap<int32,int64>` (id -> ptr).
fn read_id_ptr_map(c: &mut Cursor, field: &'static str) -> Result<TableSpan, WireError> {
    let count = c.read_count(field)?;
    c.ensure_minimum_remaining(count, 12, field)?;
    let entries_start = c.pos();
    let mut keys = Vec::new();
    let mut entry_starts = Vec::new();
    for _ in 0..count {
        entry_starts.push(c.pos());
        keys.push(c.read_i32()? as i64);
        c.skip(8)?; // value (int64 ptr)
    }
    Ok(TableSpan {
        count: count as u32,
        entries_start,
        entries_end: c.pos(),
        keys,
        entry_starts,
    })
}

/// Read `StaticNames TArray<FStringInArchive>`.
fn read_static_names(c: &mut Cursor) -> Result<TableSpan, WireError> {
    let count = c.read_count("StaticNames")?;
    c.ensure_minimum_remaining(count, 4, "StaticNames")?;
    let entries_start = c.pos();
    for _ in 0..count {
        c.read_sia()?;
    }
    Ok(TableSpan {
        count: count as u32,
        entries_start,
        entries_end: c.pos(),
        keys: Vec::new(),
        entry_starts: Vec::new(),
    })
}

/// Parse the 7 tail tables starting at `start` (= TAIL_OFF).
pub fn parse_tail_tables(bytes: &[u8], start: usize) -> Result<TailTables, WireError> {
    let mut c = Cursor::at(bytes, start);
    let mut tables = Vec::with_capacity(N_TABLES);
    tables.push(read_i64_map(
        &mut c,
        "TypeReferences",
        16,
        read_type_reference,
    )?);
    tables.push(read_id_ptr_map(&mut c, "TypeIdReferenceToPointer")?);
    tables.push(read_i64_map(
        &mut c,
        "FunctionReferences",
        72,
        read_function_reference,
    )?);
    tables.push(read_id_ptr_map(&mut c, "FunctionIdReferenceToPointer")?);
    tables.push(read_i64_map(
        &mut c,
        "GlobalReferences",
        16,
        read_global_reference,
    )?);
    tables.push(read_static_names(&mut c)?);
    tables.push(read_i64_map(
        &mut c,
        "PropertyReferences",
        8,
        read_property_reference,
    )?);
    Ok(TailTables {
        start,
        end: c.pos(),
        tables,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn push_sia_bytes(out: &mut Vec<u8>, value: &[u8]) {
        if value.is_empty() {
            out.extend_from_slice(&0i32.to_le_bytes());
        } else {
            out.extend_from_slice(&(value.len() as i32).to_le_bytes());
            out.extend_from_slice(value);
            out.push(0);
        }
    }

    fn tail_with_global_name(name: &[u8], is_string: i32) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&0i32.to_le_bytes()); // TypeReferences
        out.extend_from_slice(&0i32.to_le_bytes()); // TypeIdReferenceToPointer
        out.extend_from_slice(&0i32.to_le_bytes()); // FunctionReferences
        out.extend_from_slice(&0i32.to_le_bytes()); // FunctionIdReferenceToPointer
        out.extend_from_slice(&1i32.to_le_bytes()); // GlobalReferences
        out.extend_from_slice(&0x3000i64.to_le_bytes());
        push_sia_bytes(&mut out, name);
        push_sia_bytes(&mut out, b""); // Module
        push_sia_bytes(&mut out, b""); // Namespace
        out.extend_from_slice(&is_string.to_le_bytes());
        out.extend_from_slice(&0i32.to_le_bytes()); // StaticNames
        out.extend_from_slice(&0i32.to_le_bytes()); // PropertyReferences
        out
    }

    #[test]
    fn global_name_encoding_is_selected_only_after_b_is_string() {
        let utf8 = tail_with_global_name("Grüße 世界".as_bytes(), 1);
        assert_eq!(parse_tail_tables(&utf8, 0).unwrap().end, utf8.len());

        let ansi = tail_with_global_name(&[0xff], 0);
        assert_eq!(parse_tail_tables(&ansi, 0).unwrap().end, ansi.len());

        let invalid_utf8 = tail_with_global_name(&[0xff], 1);
        assert!(matches!(
            parse_tail_tables(&invalid_utf8, 0),
            Err(WireError::InvalidSia {
                detail: "script string literal is not valid UTF-8",
                ..
            })
        ));
    }

    #[test]
    fn global_string_discriminator_must_be_a_canonical_archive_bool() {
        assert!(matches!(
            parse_tail_tables(&tail_with_global_name(b"literal", 2), 0),
            Err(WireError::BadLen {
                field: "bool",
                len: 2,
                ..
            })
        ));
    }
}
