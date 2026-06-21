//! Reference resolver: map bytecode operands (engine type-ids, function-ids,
//! global/object pointers, member keys) to NAMES, using the 7 global tail tables.
//!
//! Resolution chains (per `work/reversing/gore-as/findings/decompile-refs.md`):
//! - type-id (DW)   -> TypeIdReferenceToPointer[id]      -> TypeReferences[ptr].Name
//! - func-id (DW)   -> FunctionIdReferenceToPointer[id]  -> FunctionReferences[ptr].Name
//! - type-ptr (QW)  -> TypeReferences[ptr].Name
//! - func-ptr (QW)  -> FunctionReferences[ptr].Name
//! - global-ptr(QW) -> GlobalReferences[ptr].Name
//! - member         -> PropertyReferences[(typeId<<1)|(offset<<33)|1].Name

use std::collections::HashMap;

use super::header::CacheHeader;
use super::walk_modules::module_region_end;
use super::wire::{Cursor, WireError};

const DATA_TYPE_SIZE: usize = 36;

/// Resolved-name lookup built from a cache's tail tables.
#[derive(Debug, Default)]
pub struct RefResolver {
    type_by_ptr: HashMap<i64, String>,
    func_by_ptr: HashMap<i64, String>,
    global_by_ptr: HashMap<i64, String>,
    prop_by_key: HashMap<i64, String>,
    typeid_to_ptr: HashMap<i32, i64>,
    funcid_to_ptr: HashMap<i32, i64>,
}

impl RefResolver {
    /// Parse a cache's 7 tail tables into name lookups.
    pub fn build(bytes: &[u8]) -> Result<Self, WireError> {
        let tail = module_region_end(bytes)?;
        let mut c = Cursor::at(bytes, tail);
        let mut r = RefResolver::default();

        // T1 TypeReferences: int64 key + (Name, Module, Namespace, TArray<DataType>)
        for _ in 0..c.read_count("TypeReferences")? {
            let key = c.read_i64()?;
            let name = c.read_sia()?;
            c.read_sia()?; // Module
            c.read_sia()?; // Namespace
            c.skip_tarray_fixed(DATA_TYPE_SIZE, "TypeRef.SubTypes")?;
            r.type_by_ptr.insert(key, name);
        }
        // T2 TypeIdReferenceToPointer: int32 id -> int64 ptr
        for _ in 0..c.read_count("TypeIdRef")? {
            let id = c.read_i32()?;
            let ptr = c.read_i64()?;
            r.typeid_to_ptr.insert(id, ptr);
        }
        // T3 FunctionReferences: int64 key + (Name, Module, Namespace, 3 bool, int64, params, ret)
        for _ in 0..c.read_count("FunctionReferences")? {
            let key = c.read_i64()?;
            let name = c.read_sia()?;
            c.read_sia()?; // Module
            c.read_sia()?; // Namespace
            c.skip(4 * 3)?; // bIsConst/bIsImportedDecl/bIsMethod
            c.skip(8)?; // ObjectType ptr
            c.skip_tarray_fixed(DATA_TYPE_SIZE, "FuncRef.Params")?;
            c.skip(DATA_TYPE_SIZE)?; // ReturnType
            r.func_by_ptr.insert(key, name);
        }
        // T4 FunctionIdReferenceToPointer: int32 id -> int64 ptr
        for _ in 0..c.read_count("FuncIdRef")? {
            let id = c.read_i32()?;
            let ptr = c.read_i64()?;
            r.funcid_to_ptr.insert(id, ptr);
        }
        // T5 GlobalReferences: int64 key + (Name, Module, Namespace, int32 bIsString)
        for _ in 0..c.read_count("GlobalReferences")? {
            let key = c.read_i64()?;
            let name = c.read_sia()?;
            c.read_sia()?; // Module
            c.read_sia()?; // Namespace
            c.skip(4)?; // bIsString
            r.global_by_ptr.insert(key, name);
        }
        // T6 StaticNames: TArray<SIA>
        for _ in 0..c.read_count("StaticNames")? {
            c.read_sia()?;
        }
        // T7 PropertyReferences: int64 key + (Name, int32 OldTypeId)
        for _ in 0..c.read_count("PropertyReferences")? {
            let key = c.read_i64()?;
            let name = c.read_sia()?;
            c.skip(4)?; // OldTypeId
            r.prop_by_key.insert(key, name);
        }
        let _ = CacheHeader::SIZE; // (header parsed elsewhere)
        Ok(r)
    }

    pub fn type_by_id(&self, id: i32) -> Option<&str> {
        self.typeid_to_ptr
            .get(&id)
            .and_then(|p| self.type_by_ptr.get(p))
            .map(|s| s.as_str())
    }
    pub fn func_by_id(&self, id: i32) -> Option<&str> {
        self.funcid_to_ptr
            .get(&id)
            .and_then(|p| self.func_by_ptr.get(p))
            .map(|s| s.as_str())
    }
    pub fn type_by_ptr(&self, ptr: i64) -> Option<&str> {
        self.type_by_ptr.get(&ptr).map(|s| s.as_str())
    }
    pub fn func_by_ptr(&self, ptr: i64) -> Option<&str> {
        self.func_by_ptr.get(&ptr).map(|s| s.as_str())
    }
    pub fn global_by_ptr(&self, ptr: i64) -> Option<&str> {
        self.global_by_ptr.get(&ptr).map(|s| s.as_str())
    }
    /// Member name from a containing type-id + byte offset.
    pub fn member(&self, type_id: i32, offset: i32) -> Option<&str> {
        let key = ((type_id as i64) << 1) | ((offset as i64) << 33) | 1;
        self.prop_by_key.get(&key).map(|s| s.as_str())
    }
}
