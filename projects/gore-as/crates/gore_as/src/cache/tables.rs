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
    c.read_sia()?; // Name
    c.read_sia()?; // Module
    c.read_sia()?; // Namespace
    c.skip(4)?; // bIsString
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
    mut read_value: impl FnMut(&mut Cursor) -> Result<(), WireError>,
) -> Result<TableSpan, WireError> {
    let count = c.read_count(field)? as u32;
    let entries_start = c.pos();
    let mut keys = Vec::with_capacity(count as usize);
    let mut entry_starts = Vec::with_capacity(count as usize);
    for _ in 0..count {
        entry_starts.push(c.pos());
        keys.push(c.read_i64()?);
        read_value(c)?;
    }
    Ok(TableSpan {
        count,
        entries_start,
        entries_end: c.pos(),
        keys,
        entry_starts,
    })
}

/// Read a `TMap<int32,int64>` (id -> ptr).
fn read_id_ptr_map(c: &mut Cursor, field: &'static str) -> Result<TableSpan, WireError> {
    let count = c.read_count(field)? as u32;
    let entries_start = c.pos();
    let mut keys = Vec::with_capacity(count as usize);
    let mut entry_starts = Vec::with_capacity(count as usize);
    for _ in 0..count {
        entry_starts.push(c.pos());
        keys.push(c.read_i32()? as i64);
        c.skip(8)?; // value (int64 ptr)
    }
    Ok(TableSpan {
        count,
        entries_start,
        entries_end: c.pos(),
        keys,
        entry_starts,
    })
}

/// Read `StaticNames TArray<FStringInArchive>`.
fn read_static_names(c: &mut Cursor) -> Result<TableSpan, WireError> {
    let count = c.read_count("StaticNames")? as u32;
    let entries_start = c.pos();
    for _ in 0..count {
        c.read_sia()?;
    }
    Ok(TableSpan {
        count,
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
    tables.push(read_i64_map(&mut c, "TypeReferences", read_type_reference)?);
    tables.push(read_id_ptr_map(&mut c, "TypeIdReferenceToPointer")?);
    tables.push(read_i64_map(&mut c, "FunctionReferences", read_function_reference)?);
    tables.push(read_id_ptr_map(&mut c, "FunctionIdReferenceToPointer")?);
    tables.push(read_i64_map(&mut c, "GlobalReferences", read_global_reference)?);
    tables.push(read_static_names(&mut c)?);
    tables.push(read_i64_map(&mut c, "PropertyReferences", read_property_reference)?);
    Ok(TailTables {
        start,
        end: c.pos(),
        tables,
    })
}
