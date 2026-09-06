//! Structured module model parsed from the cache, for the recompilable emitter.
//!
//! Captures the full `FAngelscriptPrecompiledModule` tree (functions with typed
//! signatures, classes with field types + methods, enums, globals) per
//! `work/reversing/gore-as/findings/container-splice.md` §9 + `recompile-*.md`.
//! Unlike `walk_modules` (which skips types for the fast splice path), this captures
//! everything the emitter needs.

use std::collections::HashMap;

use super::header::CacheHeader;
use super::refs::RefResolver;
use super::types::{DataType, DATA_TYPE_SIZE};
use super::wire::{Cursor, WireError};

// Exact conservative wire minima. These include every fixed field and the four-byte count/string
// header for every variable-width member, but no optional payload. Proving them against the bytes
// that remain keeps attacker-controlled counts from driving allocations or loops after a desync.
const MIN_MODULE_ENTRY_BYTES: usize = 60;
const MIN_FUNCTION_BYTES: usize = 120;
const MIN_CLASS_BYTES: usize = 64;
const MIN_PROPERTY_BYTES: usize = 52;
const MIN_ENUM_BYTES: usize = 16;
const MIN_GLOBAL_BYTES: usize = 48;
const MIN_IMPORT_BYTES: usize = 60;

/// AngelScript value-pointer size in dwords on x64 (`AS_PTR_SIZE`). Every handle/reference
/// and every 64-bit scalar occupies this many frame slots; mirrors `isa.rs`/`decompile.rs`.
pub const AS_PTR_SIZE: i32 = 2;

/// Number of frame dword slots a parameter of this type occupies (`GetSizeOnStackDWords`):
/// pointer-sized for every UObject/AActor handle, every `&`-reference and 64-bit scalar;
/// 1 for ordinary 32-bit-or-smaller value primitives/enums. A genuine struct passed BY VALUE
/// (rare — the engine usually passes structs by reference) has no registered dword-size table
/// here, so it defaults to `AS_PTR_SIZE`, the safe conservative width.
///
/// `refs` lets a `token == 5` value type be resolved to its name so an ENUM (`E`-prefixed,
/// UE/codebase convention; 32-bit underlying storage) is correctly sized at 1 dword rather than
/// the conservative struct width 2. Without this, a function with an enum-by-value parameter
/// FOLLOWING a wider param (handle/ref/struct) mis-maps every later param's frame offset (the
/// cumulative cursor over-counts), which is what stubbed `MakeRequirement` (4 EQuestState params
/// after a `TSubclassOf&` ref). Passing `None` keeps the conservative width-2 fallback.
pub fn slot_width_dwords(p: &DataType, refs: Option<&RefResolver>) -> i32 {
    if p.is_object_handle || p.is_reference {
        return AS_PTR_SIZE;
    }
    match p.token {
        // 64-bit scalars occupy 2 dwords: int64 / uint64 / double, AND `float` (0x51) because
        // this build is `floatIsFloat64` (see types.rs / render_const) — its `float` is 64-bit.
        // `float32` (0x50) is the genuine 32-bit type (width 1).
        0x47 | 0x4E | 0x5E | 0x51 => AS_PTR_SIZE,
        5 => {
            // token 5 is the catch-all "identifier" token shared by enums, value structs,
            // templates and objects. An enum's underlying storage is a 32-bit int -> 1 dword;
            // only a genuine value STRUCT (F*/T* by value) needs the conservative width 2. Use
            // the resolved type name (E-prefix = enum, the established convention used by the
            // value-type checks in structure.rs/cast_arg) to tell them apart.
            if refs
                .and_then(|r| r.type_by_ptr(p.type_info))
                .map(is_enum_name)
                .unwrap_or(false)
            {
                1
            } else {
                AS_PTR_SIZE // struct-by-value (no size table -> conservative)
            }
        }
        _ => 1, // int/uint/float32/bool/int8..16/enum by value
    }
}

/// True for an enum type NAME by the codebase/UE `E`-prefix convention (e.g. `EQuestState`).
/// Enums are 32-bit value types (1 frame dword), unlike `F`/`T` value structs (conservative 2).
fn is_enum_name(name: &str) -> bool {
    let mut b = name.bytes();
    matches!(b.next(), Some(b'E')) && matches!(b.next(), Some(c) if c.is_ascii_uppercase())
}

/// True when the return type is an F-struct returned BY VALUE — which inserts a hidden RVO
/// out-pointer slot (one `AS_PTR_SIZE`) between `this` and the first real parameter.
/// UObject/AActor handles (`is_object_handle`) return in the value register, NOT via an RVO
/// slot, so they are excluded.
///
/// batch-29c (A4, specs/batch29-errortail.md §1.1): ENUMS are also token 5 but return in the
/// VALUE REGISTER — no hidden RVO slot exists (the callee-side probes in structure.rs/emit.rs
/// already exclude `E*` heads; this predicate was the last admitter). Counting an enum return
/// as by-value-struct mapped a REAL param onto the phantom RVO slot (`ERelationship __return;
/// ... UpdateCrimeSeverityTowardsPlayer(__return, AI);` — 11 [E] lines, 6 fns), so enum-named
/// returns are excluded here, at the map-building choke point.
pub fn returns_struct_by_value(ret: &DataType, refs: &RefResolver) -> bool {
    ret.token == 5
        && !ret.is_object_handle
        && !ret.is_reference
        && !is_enum_name(&ret.base_name(refs))
}

/// Build the AS_PTR_SIZE-aware map from a frame offset (signed dword slot, negative below the
/// frame pointer) to the 0-based parameter index. Each parameter consumes its real slot width,
/// so param *i* lives at a cumulative offset, NOT at `-i`.
///
/// `rvo` controls whether the hidden by-value-return RVO out-pointer slot is reserved before
/// the first param (callers self-correct on the observed offsets — see structure.rs/decompile.rs).
pub fn param_slot_map(
    params: &[DataType],
    is_method: bool,
    rvo: bool,
    refs: Option<&RefResolver>,
) -> HashMap<i32, usize> {
    let mut map = HashMap::new();
    // Cursor start: free fn -> param 0 at off 0; method -> first param after `this` (-AS_PTR_SIZE).
    let mut off: i32 = if is_method { -AS_PTR_SIZE } else { 0 };
    if rvo {
        off -= AS_PTR_SIZE; // skip the hidden RVO out-pointer slot
    }
    for (i, p) in params.iter().enumerate() {
        map.insert(off, i);
        off -= slot_width_dwords(p, refs);
    }
    map
}

#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub ty: DataType,
    pub flags: i32,
}

#[derive(Debug, Clone)]
pub struct Func {
    pub name: String,
    /// Source text of the trailing parameter default arguments, in declaration order.
    pub param_defaults: Vec<String>,
    pub namespace: String,
    pub ret: DataType,
    pub params: Vec<Param>,
    pub bytecode: Vec<i32>,
    /// Local stack-frame size in dwords; inactive function records may store a negative sentinel.
    pub variable_space: i32,
    /// (slot offset, type-ptr) for object-typed locals.
    pub obj_locals: Vec<(i32, i64)>,
    pub is_ufunction: bool,
    /// asSFunctionTraits bitfield (asTRAIT_CONST=4, asTRAIT_FINAL=32, generated=0x40000, ...).
    pub traits: i32,
}

impl Func {
    /// The vanilla method was declared `const` (asTRAIT_CONST). Dropping it on re-emit makes
    /// every call through a `const` object handle fail with "Non-const method call on
    /// read-only object reference", so the emitter must render it back.
    pub fn is_const_method(&self) -> bool {
        self.traits & 4 != 0
    }
}

#[derive(Debug, Clone)]
pub struct Field {
    pub name: String,
    pub ty: DataType,
    pub is_uproperty: bool,
}

#[derive(Debug, Clone)]
pub struct Class {
    pub name: String,
    /// Declaring AngelScript namespace, empty at global scope. Part of the class's identity in
    /// the cache's TypeReferences table, so a recompile that drops it produces a DIFFERENT
    /// symbol that no longer matches the base cache.
    pub namespace: String,
    pub super_class: Option<String>,
    pub fields: Vec<Field>,
    pub methods: Vec<Func>,
    pub ctors: Vec<Func>,
    /// asCObjectType flags (asOBJ_* bitfield) from the cache Class record.
    pub flags: u32,
}

#[derive(Debug, Clone)]
pub struct EnumDef {
    pub name: String,
    pub namespace: String,
    pub entries: Vec<(String, i32)>,
}

#[derive(Debug, Clone)]
pub struct Global {
    pub name: String,
    /// The global's initializer function, when it has one. Its bytecode carries the value a
    /// non-primitive global is built from — an `FName` global's real name literal, say, which is
    /// otherwise unrecoverable from the declaration alone.
    pub init: Option<Func>,
    /// Declaring AngelScript namespace, empty at global scope. Reference sites render a
    /// namespaced global as `Ns::Name`, so the declaration has to reopen that namespace or the
    /// compiler rejects the reference with "Unknown scope".
    pub namespace: String,
    pub ty: DataType,
    pub value: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct Module {
    pub name: String,
    pub file: String,
    pub functions: Vec<Func>,
    pub classes: Vec<Class>,
    pub enums: Vec<EnumDef>,
    pub globals: Vec<Global>,
}

fn bounded_count(
    c: &mut Cursor<'_>,
    field: &'static str,
    minimum_element_bytes: usize,
) -> Result<usize, WireError> {
    let count = c.read_count(field)?;
    c.ensure_minimum_remaining(count, minimum_element_bytes, field)?;
    Ok(count)
}

fn skip_tarray_fixed_checked(
    c: &mut Cursor<'_>,
    element_bytes: usize,
    field: &'static str,
) -> Result<(), WireError> {
    let count = bounded_count(c, field, element_bytes)?;
    // `bounded_count` proved this product does not overflow.
    c.skip(count * element_bytes)
}

fn skip_tarray_sia_checked(c: &mut Cursor<'_>, field: &'static str) -> Result<(), WireError> {
    let count = bounded_count(c, field, 4)?;
    for _ in 0..count {
        c.read_sia()?;
    }
    Ok(())
}

fn read_tarray_i32_checked(c: &mut Cursor<'_>, field: &'static str) -> Result<Vec<i32>, WireError> {
    let count = bounded_count(c, field, 4)?;
    let mut values = Vec::with_capacity(count);
    for _ in 0..count {
        values.push(c.read_i32()?);
    }
    Ok(values)
}

fn read_tarray_sia_checked(
    c: &mut Cursor<'_>,
    field: &'static str,
) -> Result<Vec<String>, WireError> {
    let count = bounded_count(c, field, 4)?;
    let mut values = Vec::with_capacity(count);
    for _ in 0..count {
        values.push(c.read_sia()?);
    }
    Ok(values)
}

pub fn parse_modules(bytes: &[u8]) -> Result<Vec<Module>, WireError> {
    if bytes.len() < CacheHeader::SIZE {
        return Err(WireError::Eof {
            pos: 0,
            need: CacheHeader::SIZE,
            have: bytes.len(),
        });
    }
    let mut c = Cursor::at(bytes, CacheHeader::SIZE);
    let count = u32::from_le_bytes(bytes[0x14..0x18].try_into().unwrap()) as usize;
    c.ensure_minimum_remaining(count, MIN_MODULE_ENTRY_BYTES, "Modules")?;
    let mut out = Vec::with_capacity(count);
    for _ in 0..count {
        c.read_fstring()?; // TMap key
        out.push(read_module(&mut c)?);
    }
    Ok(out)
}

fn read_function(c: &mut Cursor) -> Result<Func, WireError> {
    let name = c.read_sia()?;
    let namespace = c.read_sia()?;
    let ret = DataType::read(c)?;
    let nptypes = bounded_count(c, "ParameterTypes", DATA_TYPE_SIZE)?;
    let mut ptypes = Vec::with_capacity(nptypes);
    for _ in 0..nptypes {
        ptypes.push(DataType::read(c)?);
    }
    let pnames = read_tarray_sia_checked(c, "ParameterNames")?;
    let nflags = bounded_count(c, "ParameterFlags", 4)?;
    let mut pflags = Vec::with_capacity(nflags);
    for _ in 0..nflags {
        pflags.push(c.read_i32()?);
    }
    // Source text of each parameter's default argument, right-aligned with the LAST
    // parameters (AngelScript only allows trailing defaults). A call that omits them renders
    // differently from one that spells them out, and the two compile into different symbols.
    let param_defaults = read_tarray_sia_checked(c, "ParameterDefaultArgs")?;
    let traits = c.read_i32()?; // FunctionTraits (asSFunctionTraits bitfield)
    let bytecode = read_tarray_i32_checked(c, "ByteCode")?;
    skip_tarray_fixed_checked(c, 4, "ByteCodeReferences")?;
    let variable_space = c.read_i32()?;
    // ObjVariableTypes: TArray<int64 ref>; ObjVariablePos: TArray<int32>
    let nobj = bounded_count(c, "ObjVariableTypes", 8)?;
    let mut obj_types = Vec::with_capacity(nobj);
    for _ in 0..nobj {
        obj_types.push(c.read_i64()?);
    }
    let nobjpos = bounded_count(c, "ObjVariablePos", 4)?;
    let mut obj_pos = Vec::with_capacity(nobjpos);
    for _ in 0..nobjpos {
        obj_pos.push(c.read_i32()?);
    }
    c.skip(4)?; // ObjVariablesOnHeap
    skip_tarray_fixed_checked(c, 4, "VarInfoProgramPos")?;
    skip_tarray_fixed_checked(c, 4, "VarInfoOffset")?;
    skip_tarray_fixed_checked(c, 4, "VarInfoOption")?;
    c.skip(4)?; // StackNeeded
    c.skip(4)?; // Id
    c.skip(4)?; // DeclaredAt
    skip_tarray_fixed_checked(c, 4, "LineNumbers")?;
    let is_ufunction = c.read_bool4()?;
    if is_ufunction {
        c.read_sia()?; // UnrealFunctionName
        skip_tarray_sia_checked(c, "UF.MetaSpec")?;
        skip_tarray_sia_checked(c, "UF.MetaValues")?;
        c.skip(18 * 4)?;
    }
    // build params (zip names/types/flags by index)
    let mut params = Vec::with_capacity(ptypes.len());
    for (i, ty) in ptypes.into_iter().enumerate() {
        params.push(Param {
            name: pnames.get(i).cloned().unwrap_or_default(),
            ty,
            flags: pflags.get(i).copied().unwrap_or(0),
        });
    }
    let obj_locals = obj_pos.into_iter().zip(obj_types).collect();
    Ok(Func {
        name,
        namespace,
        ret,
        params,
        param_defaults,
        bytecode,
        variable_space,
        obj_locals,
        is_ufunction,
        traits,
    })
}

fn read_property(c: &mut Cursor) -> Result<Field, WireError> {
    let name = c.read_sia()?;
    let ty = DataType::read(c)?;
    c.skip(4)?; // bIsPrivate
    c.skip(4)?; // bIsProtected
    let is_uproperty = c.read_bool4()?;
    if is_uproperty {
        skip_tarray_sia_checked(c, "UP.MetaSpec")?;
        skip_tarray_sia_checked(c, "UP.MetaValues")?;
        c.skip(9 * 4)?;
        let replicated = c.read_bool4()?;
        c.skip(4)?; // bSkipReplication
        c.skip(4)?; // bSkipSerialization
        c.skip(4)?; // bSaveGame
        if replicated {
            c.skip(4)?; // ReplicationCondition
            c.skip(4)?; // bRepNotify
        }
        c.skip(4)?; // bConfig
        c.skip(4)?; // bInterp
        c.skip(4)?; // bAssetRegistrySearchable
    }
    Ok(Field {
        name,
        ty,
        is_uproperty,
    })
}

fn read_class(c: &mut Cursor) -> Result<Class, WireError> {
    let name = c.read_sia()?;
    let namespace = c.read_sia()?;
    let flags = c.read_i32()? as u32; // asCObjectType Flags (asOBJ_* bitfield)
    let nprops = bounded_count(c, "Class.Properties", MIN_PROPERTY_BYTES)?;
    let mut fields = Vec::with_capacity(nprops);
    for _ in 0..nprops {
        fields.push(read_property(c)?);
    }
    let nmethods = bounded_count(c, "Class.Methods", MIN_FUNCTION_BYTES)?;
    let mut methods = Vec::with_capacity(nmethods);
    for _ in 0..nmethods {
        methods.push(read_function(c)?);
    }
    skip_tarray_fixed_checked(c, 4, "Class.MethodTable")?;
    c.skip(8)?; // DerivedFrom
    c.skip(8)?; // ShadowType
    let nctors = bounded_count(c, "Class.Constructors", MIN_FUNCTION_BYTES)?;
    let mut ctors = Vec::with_capacity(nctors);
    for _ in 0..nctors {
        ctors.push(read_function(c)?);
    }
    skip_tarray_fixed_checked(c, 8, "Class.FactoryRefs")?;
    skip_tarray_fixed_checked(c, 8, "Class.BehaviorRefs")?;
    let nbehav = bounded_count(c, "Class.BehaviorFunctions", MIN_FUNCTION_BYTES)?;
    for _ in 0..nbehav {
        read_function(c)?;
    }
    skip_tarray_fixed_checked(c, 4, "Class.BehaviorFunctionTypes")?;
    let mut super_class = None;
    if c.read_bool4()? {
        super_class = Some(c.read_sia()?); // SuperClass
        c.read_sia()?; // CodeSuperClass
                       // bSuperIsCodeClass + six serialized class flags. ConfigName follows
                       // as a variable-width FStringInArchive; treating an empty ConfigName's
                       // four-byte length as an eighth bool desynchronizes any non-empty one.
        for _ in 0..7 {
            c.read_bool4()?;
        }
        c.read_sia()?; // ConfigName
        c.read_sia()?; // StaticClassGVName
        c.read_bool4()?; // bPlaceable
        skip_tarray_sia_checked(c, "Class.MetaSpec")?;
        skip_tarray_sia_checked(c, "Class.MetaValues")?;
        c.read_sia()?; // ComposeOntoClassName
    }
    Ok(Class {
        name,
        namespace,
        super_class,
        fields,
        methods,
        ctors,
        flags,
    })
}

fn read_enum(c: &mut Cursor) -> Result<EnumDef, WireError> {
    let name = c.read_sia()?;
    let namespace = c.read_sia()?;
    let names = read_tarray_sia_checked(c, "Enum.Names")?;
    let nvals = bounded_count(c, "Enum.Values", 4)?;
    let mut vals = Vec::with_capacity(nvals);
    for _ in 0..nvals {
        vals.push(c.read_i32()?);
    }
    let entries = names.into_iter().zip(vals).collect();
    Ok(EnumDef {
        name,
        namespace,
        entries,
    })
}

fn read_global(c: &mut Cursor) -> Result<Global, WireError> {
    let name = c.read_sia()?;
    let namespace = c.read_sia()?;
    let ty = DataType::read(c)?;
    let mut value = None;
    let mut init = None;
    if !c.read_bool4()? {
        // !bIsDefaultInit
        if c.read_bool4()? {
            value = Some(c.read_u64()?); // PureConstantValue
        } else {
            let has_init = c.read_bool4()?; // bHasInitFunction
            // The fork archives InitFunc unconditionally in this branch, including the default
            // function object when the flag is false. Read it either way to keep the stream in
            // step, and keep it only when the flag says it is a real initializer.
            let func = read_function(c)?;
            if has_init {
                init = Some(func);
            }
        }
    }
    Ok(Global {
        name,
        namespace,
        ty,
        value,
        init,
    })
}

fn read_function_import(c: &mut Cursor) -> Result<(), WireError> {
    c.read_sia()?; // ImportedFromModule
    c.read_sia()?; // Name
    c.read_sia()?; // Namespace
    skip_tarray_fixed_checked(c, DATA_TYPE_SIZE, "Import.ParameterTypes")?;
    skip_tarray_fixed_checked(c, 4, "Import.ParameterFlags")?;
    skip_tarray_sia_checked(c, "Import.ParameterDefaultArgs")?;
    c.skip(DATA_TYPE_SIZE)?; // ReturnType
    Ok(())
}

fn read_module(c: &mut Cursor) -> Result<Module, WireError> {
    let name = c.read_sia()?;
    let nfns = bounded_count(c, "Module.Functions", MIN_FUNCTION_BYTES)?;
    let mut functions = Vec::with_capacity(nfns);
    for _ in 0..nfns {
        functions.push(read_function(c)?);
    }
    let nclasses = bounded_count(c, "Module.Classes", MIN_CLASS_BYTES)?;
    let mut classes = Vec::with_capacity(nclasses);
    for _ in 0..nclasses {
        classes.push(read_class(c)?);
    }
    let nenums = bounded_count(c, "Module.Enums", MIN_ENUM_BYTES)?;
    let mut enums = Vec::with_capacity(nenums);
    for _ in 0..nenums {
        enums.push(read_enum(c)?);
    }
    let nglobals = bounded_count(c, "Module.GlobalVariables", MIN_GLOBAL_BYTES)?;
    let mut globals = Vec::with_capacity(nglobals);
    for _ in 0..nglobals {
        globals.push(read_global(c)?);
    }
    let nimports = bounded_count(c, "Module.FunctionImports", MIN_IMPORT_BYTES)?;
    for _ in 0..nimports {
        read_function_import(c)?;
    }
    c.skip(8)?; // CodeHash
    skip_tarray_sia_checked(c, "Module.ImportedModules")?;
    c.read_sia()?; // StaticsClassName
    skip_tarray_sia_checked(c, "Module.DeclaredEvents")?;
    skip_tarray_sia_checked(c, "Module.DeclaredDelegates")?;
    let file = c.read_sia()?; // ScriptRelativeFilename
    skip_tarray_sia_checked(c, "Module.PostInitFunctions")?;
    Ok(Module {
        name,
        file,
        functions,
        classes,
        enums,
        globals,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const HUGE_COUNT: i32 = 50_000_000;

    fn cache_with_module_count(count: u32) -> Vec<u8> {
        let mut bytes = vec![0; CacheHeader::SIZE];
        bytes[0x14..0x18].copy_from_slice(&count.to_le_bytes());
        bytes
    }

    fn push_i32(bytes: &mut Vec<u8>, value: i32) {
        bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn push_i64(bytes: &mut Vec<u8>, value: i64) {
        bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn push_sia(bytes: &mut Vec<u8>, value: &str) {
        push_i32(bytes, value.len() as i32);
        if !value.is_empty() {
            bytes.extend_from_slice(value.as_bytes());
            bytes.push(0);
        }
    }

    fn push_data_type(bytes: &mut Vec<u8>) {
        for _ in 0..6 {
            push_i32(bytes, 0);
        }
        push_i64(bytes, 0);
        push_i32(bytes, 0x44);
    }

    fn push_default_function(bytes: &mut Vec<u8>) {
        push_sia(bytes, ""); // FunctionName
        push_sia(bytes, ""); // Namespace
        push_data_type(bytes);
        for _ in 0..4 {
            push_i32(bytes, 0); // parameter arrays
        }
        push_i32(bytes, 0); // FunctionTraits
        push_i32(bytes, 0); // ByteCode
        push_i32(bytes, 0); // ByteCodeReferences
        push_i32(bytes, -1); // VariableSpace
        push_i32(bytes, 0); // ObjVariableTypes
        push_i32(bytes, 0); // ObjVariablePos
        push_i32(bytes, -1); // ObjVariablesOnHeap
        for _ in 0..3 {
            push_i32(bytes, 0); // variable info arrays
        }
        push_i32(bytes, -1); // StackNeeded
        push_i32(bytes, 0); // Id
        push_i32(bytes, 0); // DeclaredAt
        push_i32(bytes, 0); // LineNumbers
        push_i32(bytes, 0); // bIsUFunction
    }

    fn cache_with_nonempty_config_and_inactive_global_initializer() -> Vec<u8> {
        let mut bytes = cache_with_module_count(1);
        push_i32(&mut bytes, 0); // empty Modules TMap key
        push_sia(&mut bytes, "M");
        push_i32(&mut bytes, 0); // Functions
        push_i32(&mut bytes, 1); // Classes
        push_sia(&mut bytes, "UFixture");
        push_sia(&mut bytes, ""); // Namespace
        push_i32(&mut bytes, 0); // Flags
        for _ in 0..2 {
            push_i32(&mut bytes, 0); // Properties, Methods
        }
        push_i32(&mut bytes, 0); // MethodTable
        push_i64(&mut bytes, 0); // DerivedFrom
        push_i64(&mut bytes, 0); // ShadowType
        for _ in 0..5 {
            push_i32(&mut bytes, 0); // Constructors through BehaviorFunctionTypes
        }
        push_i32(&mut bytes, 1); // bIsInPreprocessor
        push_sia(&mut bytes, "UObject");
        push_sia(&mut bytes, "/Script/CoreUObject.Object");
        for index in 0..7 {
            push_i32(&mut bytes, i32::from(index == 0));
        }
        push_sia(&mut bytes, "Game"); // ConfigName
        push_sia(&mut bytes, "FixtureClass");
        push_i32(&mut bytes, 1); // bPlaceable
        push_i32(&mut bytes, 0); // MetaSpec
        push_i32(&mut bytes, 0); // MetaValues
        push_sia(&mut bytes, ""); // ComposeOntoClassName
        push_i32(&mut bytes, 0); // Enums
        push_i32(&mut bytes, 1); // GlobalVariables
        push_sia(&mut bytes, "Inactive");
        push_sia(&mut bytes, ""); // Namespace
        push_data_type(&mut bytes);
        push_i32(&mut bytes, 0); // bIsDefaultInit
        push_i32(&mut bytes, 0); // bIsPureConstant
        push_i32(&mut bytes, 0); // bHasInitFunction
        push_default_function(&mut bytes); // still serialized by the fork
        push_i32(&mut bytes, 0); // FunctionImports
        push_i64(&mut bytes, 0); // CodeHash
        push_i32(&mut bytes, 0); // ImportedModules
        push_sia(&mut bytes, ""); // StaticsClassName
        push_i32(&mut bytes, 0); // DeclaredEvents
        push_i32(&mut bytes, 0); // DeclaredDelegates
        push_sia(&mut bytes, "M.as");
        push_i32(&mut bytes, 0); // PostInitFunctions
        bytes
    }

    #[test]
    fn rejects_unbacked_header_module_count_before_allocating() {
        let bytes = cache_with_module_count(u32::MAX);
        let error = parse_modules(&bytes).unwrap_err();

        match error {
            WireError::Eof {
                pos, need, have, ..
            } => {
                assert_eq!(pos, CacheHeader::SIZE);
                assert_eq!(need, u32::MAX as usize * MIN_MODULE_ENTRY_BYTES);
                assert_eq!(have, 0);
            }
            other => panic!("expected module-count EOF, got {other:?}"),
        }
    }

    #[test]
    fn rejects_unbacked_function_count_before_allocating() {
        let mut bytes = cache_with_module_count(1);
        push_i32(&mut bytes, 0); // empty TMap key
        push_i32(&mut bytes, 0); // empty module name
        push_i32(&mut bytes, HUGE_COUNT); // Module.Functions
        bytes.resize(CacheHeader::SIZE + MIN_MODULE_ENTRY_BYTES, 0);

        let error = parse_modules(&bytes).unwrap_err();
        assert!(matches!(
            error,
            WireError::Eof {
                pos: 36,
                need: 6_000_000_000,
                have: 48,
            }
        ));
    }

    #[test]
    fn rejects_unbacked_variable_string_count_before_allocating() {
        let mut bytes = cache_with_module_count(1);
        push_i32(&mut bytes, 0); // empty TMap key
        push_i32(&mut bytes, 0); // empty module name
        push_i32(&mut bytes, 1); // Module.Functions
        push_i32(&mut bytes, 0); // Function.Name
        push_i32(&mut bytes, 0); // Function.Namespace
        bytes.extend_from_slice(&[0; DATA_TYPE_SIZE]); // Function.ReturnType
        push_i32(&mut bytes, 0); // ParameterTypes
        push_i32(&mut bytes, HUGE_COUNT); // ParameterNames
        bytes.resize(36 + MIN_FUNCTION_BYTES, 0);

        let error = parse_modules(&bytes).unwrap_err();
        assert!(matches!(
            error,
            WireError::Eof {
                pos: 88,
                need: 200_000_000,
                have: 68,
            }
        ));
    }

    #[test]
    fn exact_class_and_global_conditionals_do_not_desynchronize_module_walkers() {
        let bytes = cache_with_nonempty_config_and_inactive_global_initializer();
        let modules = parse_modules(&bytes).expect("full conditional fixture must parse");
        assert_eq!(modules.len(), 1);
        assert_eq!(modules[0].classes[0].name, "UFixture");
        assert_eq!(modules[0].globals[0].name, "Inactive");
        assert_eq!(modules[0].file, "M.as");
        assert_eq!(
            super::super::walk_modules::module_region_end(&bytes).unwrap(),
            bytes.len()
        );
    }
}
