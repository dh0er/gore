//! Structured module model parsed from the cache, for the recompilable emitter.
//!
//! Captures the full `FAngelscriptPrecompiledModule` tree (functions with typed
//! signatures, classes with field types + methods, enums, globals) per
//! `work/reversing/gore-as/findings/container-splice.md` §9 + `recompile-*.md`.
//! Unlike `walk_modules` (which skips types for the fast splice path), this captures
//! everything the emitter needs.

use std::collections::HashMap;

use super::header::CacheHeader;
use super::types::{DataType, DATA_TYPE_SIZE};
use super::wire::{Cursor, WireError};

/// AngelScript value-pointer size in dwords on x64 (`AS_PTR_SIZE`). Every handle/reference
/// and every 64-bit scalar occupies this many frame slots; mirrors `isa.rs`/`decompile.rs`.
pub const AS_PTR_SIZE: i32 = 2;

/// Number of frame dword slots a parameter of this type occupies (`GetSizeOnStackDWords`):
/// pointer-sized for every UObject/AActor handle, every `&`-reference and 64-bit scalar;
/// 1 for ordinary 32-bit-or-smaller value primitives/enums. Struct-by-value (rare — the
/// engine usually passes structs by reference) has no registered dword-size table here, so it
/// defaults to `AS_PTR_SIZE`, the safe conservative width.
pub fn slot_width_dwords(p: &DataType) -> i32 {
    if p.is_object_handle || p.is_reference {
        return AS_PTR_SIZE;
    }
    match p.token {
        // 64-bit scalars occupy 2 dwords: int64 / uint64 / double, AND `float` (0x51) because
        // this build is `floatIsFloat64` (see types.rs / render_const) — its `float` is 64-bit.
        // `float32` (0x50) is the genuine 32-bit type (width 1).
        0x47 | 0x4E | 0x5E | 0x51 => AS_PTR_SIZE,
        5 => AS_PTR_SIZE, // struct-by-value (no size table -> conservative)
        _ => 1,           // int/uint/float32/bool/int8..16/enum by value
    }
}

/// True when the return type is an F-struct returned BY VALUE — which inserts a hidden RVO
/// out-pointer slot (one `AS_PTR_SIZE`) between `this` and the first real parameter.
/// UObject/AActor handles (`is_object_handle`) return in the value register, NOT via an RVO
/// slot, so they are excluded.
pub fn returns_struct_by_value(ret: &DataType) -> bool {
    ret.token == 5 && !ret.is_object_handle && !ret.is_reference
}

/// Build the AS_PTR_SIZE-aware map from a frame offset (signed dword slot, negative below the
/// frame pointer) to the 0-based parameter index. Each parameter consumes its real slot width,
/// so param *i* lives at a cumulative offset, NOT at `-i`.
///
/// `rvo` controls whether the hidden by-value-return RVO out-pointer slot is reserved before
/// the first param (callers self-correct on the observed offsets — see structure.rs/decompile.rs).
pub fn param_slot_map(params: &[DataType], is_method: bool, rvo: bool) -> HashMap<i32, usize> {
    let mut map = HashMap::new();
    // Cursor start: free fn -> param 0 at off 0; method -> first param after `this` (-AS_PTR_SIZE).
    let mut off: i32 = if is_method { -AS_PTR_SIZE } else { 0 };
    if rvo {
        off -= AS_PTR_SIZE; // skip the hidden RVO out-pointer slot
    }
    for (i, p) in params.iter().enumerate() {
        map.insert(off, i);
        off -= slot_width_dwords(p);
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
    pub namespace: String,
    pub ret: DataType,
    pub params: Vec<Param>,
    pub bytecode: Vec<i32>,
    /// (slot offset, type-ptr) for object-typed locals.
    pub obj_locals: Vec<(i32, i64)>,
    pub is_ufunction: bool,
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
    pub super_class: Option<String>,
    pub fields: Vec<Field>,
    pub methods: Vec<Func>,
    pub ctors: Vec<Func>,
}

#[derive(Debug, Clone)]
pub struct EnumDef {
    pub name: String,
    pub entries: Vec<(String, i32)>,
}

#[derive(Debug, Clone)]
pub struct Global {
    pub name: String,
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
    let nptypes = c.read_count("ParameterTypes")?;
    let mut ptypes = Vec::with_capacity(nptypes);
    for _ in 0..nptypes {
        ptypes.push(DataType::read(c)?);
    }
    let pnames = c.read_tarray_sia("ParameterNames")?;
    let nflags = c.read_count("ParameterFlags")?;
    let mut pflags = Vec::with_capacity(nflags);
    for _ in 0..nflags {
        pflags.push(c.read_i32()?);
    }
    c.skip_tarray_sia("ParameterDefaultArgs")?;
    c.skip(4)?; // FunctionTraits
    let bytecode = c.read_tarray_i32("ByteCode")?;
    c.skip_tarray_fixed(4, "ByteCodeReferences")?;
    c.skip(4)?; // VariableSpace
    // ObjVariableTypes: TArray<int64 ref>; ObjVariablePos: TArray<int32>
    let nobj = c.read_count("ObjVariableTypes")?;
    let mut obj_types = Vec::with_capacity(nobj);
    for _ in 0..nobj {
        obj_types.push(c.read_i64()?);
    }
    let nobjpos = c.read_count("ObjVariablePos")?;
    let mut obj_pos = Vec::with_capacity(nobjpos);
    for _ in 0..nobjpos {
        obj_pos.push(c.read_i32()?);
    }
    c.skip(4)?; // ObjVariablesOnHeap
    c.skip_tarray_fixed(4, "VarInfoProgramPos")?;
    c.skip_tarray_fixed(4, "VarInfoOffset")?;
    c.skip_tarray_fixed(4, "VarInfoOption")?;
    c.skip(4)?; // StackNeeded
    c.skip(4)?; // Id
    c.skip(4)?; // DeclaredAt
    c.skip_tarray_fixed(4, "LineNumbers")?;
    let is_ufunction = c.read_bool4()?;
    if is_ufunction {
        c.read_sia()?; // UnrealFunctionName
        c.skip_tarray_sia("UF.MetaSpec")?;
        c.skip_tarray_sia("UF.MetaValues")?;
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
    Ok(Func { name, namespace, ret, params, bytecode, obj_locals, is_ufunction })
}

fn read_property(c: &mut Cursor) -> Result<Field, WireError> {
    let name = c.read_sia()?;
    let ty = DataType::read(c)?;
    c.skip(4)?; // bIsPrivate
    c.skip(4)?; // bIsProtected
    let is_uproperty = c.read_bool4()?;
    if is_uproperty {
        c.skip_tarray_sia("UP.MetaSpec")?;
        c.skip_tarray_sia("UP.MetaValues")?;
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
    Ok(Field { name, ty, is_uproperty })
}

fn read_class(c: &mut Cursor) -> Result<Class, WireError> {
    let name = c.read_sia()?;
    c.read_sia()?; // Namespace
    c.skip(4)?; // Flags
    let nprops = c.read_count("Class.Properties")?;
    let mut fields = Vec::with_capacity(nprops);
    for _ in 0..nprops {
        fields.push(read_property(c)?);
    }
    let nmethods = c.read_count("Class.Methods")?;
    let mut methods = Vec::with_capacity(nmethods);
    for _ in 0..nmethods {
        methods.push(read_function(c)?);
    }
    c.skip_tarray_fixed(4, "Class.MethodTable")?;
    c.skip(8)?; // DerivedFrom
    c.skip(8)?; // ShadowType
    let nctors = c.read_count("Class.Constructors")?;
    let mut ctors = Vec::with_capacity(nctors);
    for _ in 0..nctors {
        ctors.push(read_function(c)?);
    }
    c.skip_tarray_fixed(8, "Class.FactoryRefs")?;
    c.skip_tarray_fixed(8, "Class.BehaviorRefs")?;
    let nbehav = c.read_count("Class.BehaviorFunctions")?;
    for _ in 0..nbehav {
        read_function(c)?;
    }
    c.skip_tarray_fixed(4, "Class.BehaviorFunctionTypes")?;
    let mut super_class = None;
    if c.read_bool4()? {
        super_class = Some(c.read_sia()?); // SuperClass
        c.read_sia()?; // CodeSuperClass
        c.skip(8 * 4)?;
        c.read_sia()?; // StaticClassGVName
        c.skip(4)?; // bPlaceable
        c.skip_tarray_sia("Class.MetaSpec")?;
        c.skip_tarray_sia("Class.MetaValues")?;
        c.read_sia()?; // ComposeOntoClassName
    }
    Ok(Class { name, super_class, fields, methods, ctors })
}

fn read_enum(c: &mut Cursor) -> Result<EnumDef, WireError> {
    let name = c.read_sia()?;
    c.read_sia()?; // Namespace
    let names = c.read_tarray_sia("Enum.Names")?;
    let nvals = c.read_count("Enum.Values")?;
    let mut vals = Vec::with_capacity(nvals);
    for _ in 0..nvals {
        vals.push(c.read_i32()?);
    }
    let entries = names.into_iter().zip(vals).collect();
    Ok(EnumDef { name, entries })
}

fn read_global(c: &mut Cursor) -> Result<Global, WireError> {
    let name = c.read_sia()?;
    c.read_sia()?; // Namespace
    let ty = DataType::read(c)?;
    let mut value = None;
    if !c.read_bool4()? {
        // !bIsDefaultInit
        if c.read_bool4()? {
            value = Some(c.read_u64()?); // PureConstantValue
        } else if c.read_bool4()? {
            read_function(c)?; // InitFunc (ignored for emit)
        }
    }
    Ok(Global { name, ty, value })
}

fn read_function_import(c: &mut Cursor) -> Result<(), WireError> {
    c.read_sia()?; // ImportedFromModule
    c.read_sia()?; // Name
    c.read_sia()?; // Namespace
    c.skip_tarray_fixed(DATA_TYPE_SIZE, "Import.ParameterTypes")?;
    c.skip_tarray_fixed(4, "Import.ParameterFlags")?;
    c.skip_tarray_sia("Import.ParameterDefaultArgs")?;
    c.skip(DATA_TYPE_SIZE)?; // ReturnType
    Ok(())
}

fn read_module(c: &mut Cursor) -> Result<Module, WireError> {
    let name = c.read_sia()?;
    let nfns = c.read_count("Module.Functions")?;
    let mut functions = Vec::with_capacity(nfns);
    for _ in 0..nfns {
        functions.push(read_function(c)?);
    }
    let nclasses = c.read_count("Module.Classes")?;
    let mut classes = Vec::with_capacity(nclasses);
    for _ in 0..nclasses {
        classes.push(read_class(c)?);
    }
    let nenums = c.read_count("Module.Enums")?;
    let mut enums = Vec::with_capacity(nenums);
    for _ in 0..nenums {
        enums.push(read_enum(c)?);
    }
    let nglobals = c.read_count("Module.GlobalVariables")?;
    let mut globals = Vec::with_capacity(nglobals);
    for _ in 0..nglobals {
        globals.push(read_global(c)?);
    }
    let nimports = c.read_count("Module.FunctionImports")?;
    for _ in 0..nimports {
        read_function_import(c)?;
    }
    c.skip(8)?; // CodeHash
    c.skip_tarray_sia("Module.ImportedModules")?;
    c.read_sia()?; // StaticsClassName
    c.skip_tarray_sia("Module.DeclaredEvents")?;
    c.skip_tarray_sia("Module.DeclaredDelegates")?;
    let file = c.read_sia()?; // ScriptRelativeFilename
    c.skip_tarray_sia("Module.PostInitFunctions")?;
    Ok(Module { name, file, functions, classes, enums, globals })
}
