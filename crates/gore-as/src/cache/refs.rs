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
    /// PropertyReferences OldTypeId per member key (for field-assignment casts).
    prop_type_id: HashMap<i64, i32>,
    typeid_to_ptr: HashMap<i32, i64>,
    funcid_to_ptr: HashMap<i32, i64>,
    /// GlobalReferences with bIsString=true: the Name is the literal string text.
    global_is_string: std::collections::HashSet<i64>,
    /// FunctionReferences with bIsMethod=true (receiver split for calls).
    func_is_method: std::collections::HashSet<i64>,
    /// Template type SubTypes per type ptr (e.g. TSubclassOf -> [UObject]).
    type_subtypes: HashMap<i64, Vec<DataType>>,
    /// Set of all type names (to recognise constructor calls).
    type_names: std::collections::HashSet<String>,
    /// Namespace per global ref ptr (e.g. an `FColor::Red` constant's `FColor`).
    global_ns: HashMap<i64, String>,
    /// Script class -> its super class name (injected from the parsed modules after build).
    /// Lets call sites distinguish a legal upcast from an unrelated-object arg.
    class_super: HashMap<String, String>,
    /// FunctionReferences parameter DataTypes (for arg-type-driven casts at call sites).
    func_params: HashMap<i64, Vec<DataType>>,
    /// FunctionReferences return DataType.
    func_ret: HashMap<i64, DataType>,
    /// FunctionReferences owning class name (from the ObjectType ptr) — disambiguates native
    /// method overloads when looking up arity in the Binds.Cache native API.
    func_owner: HashMap<i64, String>,
    /// FunctionReferences namespace (e.g. `Gameplay`, `Math`, `System`) for free/static native
    /// functions — a call must be qualified `Namespace::func(...)` or the global-scope lookup
    /// fails with "No matching signatures". Empty for un-namespaced globals and for methods.
    func_ns: HashMap<i64, String>,
    /// Native AngelScript API arities parsed from Binds.Cache (fallback arity for native calls
    /// whose param count isn't carried in the script FunctionReferences).
    native: Option<super::binds::NativeApi>,
    /// StaticNames tail table (table 5): the `n"..."` FName-literal pool. The synthesized
    /// accessor `const FName& __STATIC_NAME(int Id)` (decl string in the shipping exe) returns
    /// `FAngelscriptManager::StaticNames[Id]`, which PrepareToFinalizePrecompiledModules
    /// populates from THIS table — so the bytecode's int operand indexes it directly.
    static_names: Vec<String>,
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
            r.type_names.insert(name.clone());
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
            let ns = c.read_sia()?; // Namespace
            c.skip(4)?; // bIsConst
            c.skip(4)?; // bIsImportedDecl
            let is_method = c.read_bool4()?;
            let objtype = c.read_i64()?; // ObjectType ptr (owning class)
            let nparams = c.read_count("FuncRef.Params")?;
            let mut params = Vec::with_capacity(nparams);
            for _ in 0..nparams {
                params.push(DataType::read(&mut c)?);
            }
            let ret = DataType::read(&mut c)?; // ReturnType
            if is_method {
                r.func_is_method.insert(key);
            }
            // Always record params — even an empty list — so the call-site arg-count check
            // can stub a zero-param method that was decompiled with phantom args.
            r.func_params.insert(key, params);
            r.func_ret.insert(key, ret);
            if let Some(cls) = r.type_by_ptr.get(&objtype) {
                r.func_owner.insert(key, cls.clone());
            }
            // Only a non-method (free/static) function needs namespace qualification; a method is
            // rendered via its receiver. Record the namespace so the call site can prefix it.
            if !is_method && !ns.is_empty() {
                r.func_ns.insert(key, ns);
            }
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
            let ns = c.read_sia()?; // Namespace
            let is_string = c.read_bool4()?;
            if is_string {
                r.global_is_string.insert(key);
            }
            if !ns.is_empty() {
                r.global_ns.insert(key, ns);
            }
            r.global_by_ptr.insert(key, name);
        }
        // T6 StaticNames: TArray<SIA> — the FName-literal pool `__STATIC_NAME(Id)` indexes.
        let n_static = c.read_count("StaticNames")?;
        r.static_names.reserve_exact(n_static);
        for _ in 0..n_static {
            r.static_names.push(c.read_sia()?);
        }
        // T7 PropertyReferences: int64 key + (Name, int32 OldTypeId)
        for _ in 0..c.read_count("PropertyReferences")? {
            let key = c.read_i64()?;
            let name = c.read_sia()?;
            let old_type_id = c.read_i32()?; // OldTypeId
            r.prop_by_key.insert(key, name);
            r.prop_type_id.insert(key, old_type_id);
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
    /// Type name by id WITH template subtypes composed
    /// (`TArrayConstIterator<AGothicCharacter>`), mirroring `DataType::render` — `type_by_id`
    /// returns the bare head, which is a template-arity error when used as a declaration.
    /// Falls back to the bare name when the T1 entry records no subtypes.
    pub fn type_by_id_composed(&self, id: i32) -> Option<String> {
        let ptr = self.typeid_to_ptr.get(&id)?;
        let base = self.type_by_ptr.get(ptr)?;
        match self.type_subtypes.get(ptr) {
            Some(subs) if !subs.is_empty() => {
                let inner: Vec<String> = subs.iter().map(|s| s.base_name(self)).collect();
                Some(format!("{base}<{}>", inner.join(", ")))
            }
            _ => Some(base.clone()),
        }
    }
    pub fn func_by_id(&self, id: i32) -> Option<&str> {
        self.funcid_to_ptr
            .get(&id)
            .and_then(|p| self.func_by_ptr.get(p))
            .map(|s| s.as_str())
    }
    /// Owning class name of a function by ptr (the ObjectType the method belongs to). Used to
    /// qualify `Class::StaticClass()` with the TARGET class, not the calling class.
    pub fn func_owner_by_ptr(&self, ptr: i64) -> Option<&str> {
        self.func_owner.get(&ptr).map(|s| s.as_str())
    }
    /// Owning class name of a function by id.
    pub fn func_owner_by_id(&self, id: i32) -> Option<&str> {
        self.funcid_to_ptr.get(&id).and_then(|p| self.func_owner.get(p)).map(|s| s.as_str())
    }
    pub fn type_by_ptr(&self, ptr: i64) -> Option<&str> {
        self.type_by_ptr.get(&ptr).map(|s| s.as_str())
    }
    /// True if `name` is a known type (so a call to it is a constructor, not a method).
    pub fn is_type_name(&self, name: &str) -> bool {
        self.type_names.contains(name)
    }

    /// Inject the script-class hierarchy (class name -> super name) from parsed modules.
    pub fn set_class_hierarchy(&mut self, supers: HashMap<String, String>) {
        self.class_super = supers;
    }
    /// True if `name` is a class DEFINED in a script module (vs an engine/native type).
    pub fn is_script_class(&self, name: &str) -> bool {
        self.class_super.contains_key(name)
    }
    /// True if `sub` is `sup` or transitively derives from it (within the script hierarchy).
    pub fn is_subclass(&self, sub: &str, sup: &str) -> bool {
        if sub == sup {
            return true;
        }
        let mut cur = sub;
        for _ in 0..64 {
            // bound the walk against cycles
            match self.class_super.get(cur) {
                Some(s) if s == sup => return true,
                Some(s) => cur = s,
                None => return false,
            }
        }
        false
    }
    /// Template SubTypes for a type ptr (e.g. TSubclassOf -> [UObject]).
    pub fn type_subtypes(&self, ptr: i64) -> Option<&[DataType]> {
        self.type_subtypes.get(&ptr).map(|v| v.as_slice())
    }
    pub fn func_by_ptr(&self, ptr: i64) -> Option<&str> {
        self.func_by_ptr.get(&ptr).map(|s| s.as_str())
    }
    /// Namespace (`Gameplay`, `Math`, ...) for a free/static native function by ptr, if any.
    pub fn func_ns_by_ptr(&self, ptr: i64) -> Option<&str> {
        self.func_ns.get(&ptr).map(|s| s.as_str())
    }
    /// Namespace for a free/static native function by id, if any.
    pub fn func_ns_by_id(&self, id: i32) -> Option<&str> {
        self.funcid_to_ptr.get(&id).and_then(|p| self.func_ns.get(p)).map(|s| s.as_str())
    }
    /// Target class of a `StaticClass` call: StaticClass is a namespaced free fn whose
    /// Namespace IS the (fully-qualified) target class — the LAST `::` segment is the class
    /// name (objtype is NULL for StaticClass, so func_owner can't carry it).
    pub fn staticclass_class_by_id(&self, id: i32) -> Option<&str> {
        self.func_ns_by_id(id).map(|ns| ns.rsplit("::").next().unwrap_or(ns))
    }
    pub fn staticclass_class_by_ptr(&self, ptr: i64) -> Option<&str> {
        self.func_ns_by_ptr(ptr).map(|ns| ns.rsplit("::").next().unwrap_or(ns))
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

    /// Attach the Binds.Cache native API (for arity fallback on native method calls).
    pub fn set_native_api(&mut self, api: super::binds::NativeApi) {
        self.native = Some(api);
    }
    /// Best-known native arity for a call by function ptr: prefer the exact (owning class,
    /// name) match, else the unambiguous by-name arity. None if no native data / ambiguous.
    pub fn native_arity_by_ptr(&self, ptr: i64, name: &str) -> Option<usize> {
        let n = self.native.as_ref()?;
        if let Some(cls) = self.func_owner.get(&ptr) {
            if let Some(a) = n.arity(cls, name) {
                return Some(a);
            }
        }
        n.arity_by_name(name)
    }
    /// Best-known native arity for a call by function id.
    pub fn native_arity_by_id(&self, id: i32, name: &str) -> Option<usize> {
        match self.funcid_to_ptr.get(&id) {
            Some(&ptr) => self.native_arity_by_ptr(ptr, name),
            None => self.native.as_ref()?.arity_by_name(name),
        }
    }
    pub fn global_by_ptr(&self, ptr: i64) -> Option<&str> {
        self.global_by_ptr.get(&ptr).map(|s| s.as_str())
    }
    /// Namespace for a global ref ptr (empty/absent -> None).
    pub fn global_ns(&self, ptr: i64) -> Option<&str> {
        self.global_ns.get(&ptr).map(|s| s.as_str())
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
    /// FName-literal text for a `__STATIC_NAME(Id)` index into the StaticNames tail table.
    /// None if out of range (e.g. a mini-cache with empty tail tables).
    pub fn static_name(&self, id: i64) -> Option<&str> {
        usize::try_from(id).ok().and_then(|i| self.static_names.get(i)).map(|s| s.as_str())
    }
    /// Number of StaticNames entries (debug aid).
    pub fn static_name_count(&self) -> usize {
        self.static_names.len()
    }
    /// Member name from a containing type-id + byte offset.
    pub fn member(&self, type_id: i32, offset: i32) -> Option<&str> {
        let key = ((type_id as i64) << 1) | ((offset as i64) << 33) | 1;
        self.prop_by_key.get(&key).map(|s| s.as_str())
    }
    /// Member's type NAME (e.g. `bool`, `ECrimeDurationType`) from type-id + byte offset,
    /// resolved via its PropertyReferences OldTypeId. Used to cast field-assignment RHS.
    pub fn member_type(&self, type_id: i32, offset: i32) -> Option<&str> {
        let key = ((type_id as i64) << 1) | ((offset as i64) << 33) | 1;
        self.prop_type_id.get(&key).and_then(|id| self.type_by_id(*id))
    }
    /// [`Self::member_type`], composed variant (declaring class INCLUDING template subtypes,
    /// e.g. `TArrayConstIterator<AGothicCharacter>` instead of the bare head). Used where the
    /// name becomes a slot DECLARATION (is-not-a-member.md §2.1/§2.2); the bare variant stays
    /// for the head-comparing cast paths in structure.rs.
    pub fn member_type_composed(&self, type_id: i32, offset: i32) -> Option<String> {
        let key = ((type_id as i64) << 1) | ((offset as i64) << 33) | 1;
        self.prop_type_id.get(&key).and_then(|id| self.type_by_id_composed(*id))
    }
}
