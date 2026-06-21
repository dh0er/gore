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
use super::types::DataType;
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
    /// GlobalReferences with bIsString=true: the Name is the literal string text.
    global_is_string: std::collections::HashSet<i64>,
    /// FunctionReferences with bIsMethod=true (receiver split for calls).
    func_is_method: std::collections::HashSet<i64>,
    /// Template type SubTypes per type ptr (e.g. TSubclassOf -> [UObject]).
    type_subtypes: HashMap<i64, Vec<DataType>>,
    /// FunctionReferences parameter DataTypes (for arg-type-driven casts at call sites).
    func_params: HashMap<i64, Vec<DataType>>,
    /// FunctionReferences return DataType.
    func_ret: HashMap<i64, DataType>,
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
            let nsub = c.read_count("TypeRef.SubTypes")?;
            if nsub > 0 {
                let mut subs = Vec::with_capacity(nsub);
                for _ in 0..nsub {
                    subs.push(DataType::read(&mut c)?);
                }
                r.type_subtypes.insert(key, subs);
            }
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
            c.skip(4)?; // bIsConst
            c.skip(4)?; // bIsImportedDecl
            let is_method = c.read_bool4()?;
            c.skip(8)?; // ObjectType ptr
            let nparams = c.read_count("FuncRef.Params")?;
            let mut params = Vec::with_capacity(nparams);
            for _ in 0..nparams {
                params.push(DataType::read(&mut c)?);
            }
            let ret = DataType::read(&mut c)?; // ReturnType
            if is_method {
                r.func_is_method.insert(key);
            }
            if !params.is_empty() {
                r.func_params.insert(key, params);
            }
            r.func_ret.insert(key, ret);
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
            let is_string = c.read_bool4()?;
            if is_string {
                r.global_is_string.insert(key);
            }
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
    /// Template SubTypes for a type ptr (e.g. TSubclassOf -> [UObject]).
    pub fn type_subtypes(&self, ptr: i64) -> Option<&[DataType]> {
        self.type_subtypes.get(&ptr).map(|v| v.as_slice())
    }
    pub fn func_by_ptr(&self, ptr: i64) -> Option<&str> {
        self.func_by_ptr.get(&ptr).map(|s| s.as_str())
    }
    /// Parameter DataTypes for a function by ptr (excludes the receiver).
    pub fn func_params_by_ptr(&self, ptr: i64) -> Option<&[DataType]> {
        self.func_params.get(&ptr).map(|v| v.as_slice())
    }
    /// Parameter DataTypes for a function by id.
    pub fn func_params_by_id(&self, id: i32) -> Option<&[DataType]> {
        self.funcid_to_ptr.get(&id).and_then(|p| self.func_params.get(p)).map(|v| v.as_slice())
    }
    /// Return DataType for a function by ptr.
    pub fn func_ret_by_ptr(&self, ptr: i64) -> Option<&DataType> {
        self.func_ret.get(&ptr)
    }
    /// Return DataType for a function by id.
    pub fn func_ret_by_id(&self, id: i32) -> Option<&DataType> {
        self.funcid_to_ptr.get(&id).and_then(|p| self.func_ret.get(p))
    }
    pub fn global_by_ptr(&self, ptr: i64) -> Option<&str> {
        self.global_by_ptr.get(&ptr).map(|s| s.as_str())
    }
    /// True if the global at `ptr` is actually a string literal (Name = the text).
    pub fn global_is_string(&self, ptr: i64) -> bool {
        self.global_is_string.contains(&ptr)
    }
    /// True if the function (by ptr) is a method (receiver split for calls).
    pub fn is_method_by_ptr(&self, ptr: i64) -> bool {
        self.func_is_method.contains(&ptr)
    }
    /// True if the function (by id) is a method.
    pub fn is_method_by_id(&self, id: i32) -> bool {
        self.funcid_to_ptr
            .get(&id)
            .map(|p| self.func_is_method.contains(p))
            .unwrap_or(false)
    }
    /// Member name from a containing type-id + byte offset.
    pub fn member(&self, type_id: i32, offset: i32) -> Option<&str> {
        let key = ((type_id as i64) << 1) | ((offset as i64) << 33) | 1;
        self.prop_by_key.get(&key).map(|s| s.as_str())
    }
}
