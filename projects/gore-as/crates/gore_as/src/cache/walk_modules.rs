//! Streaming walker over the `Modules` array of a precompiled cache.
//!
//! Locates `TAIL_OFF` — the byte offset where the 7 global tail tables begin
//! (i.e. the end of the last module). This is the splice insertion point.
//!
//! Field layout per `work/reversing/gore-as/findings/container-splice.md` §1-§3, §9
//! (byte-exact validated against the 314 B and 2774 B Rosetta samples). All `bool`s
//! are 4 bytes; `DataType` is a fixed 36 bytes.

use super::header::CacheHeader;
use super::wire::{Cursor, WireError};

/// `FAngelscriptPrecompiledDataType` = 6×bool(24) + int64 TypeInfo.Old(8) + int32 Token(4).
const DATA_TYPE_SIZE: usize = 36;

/// Parse the cache header + all `Modules`, returning `TAIL_OFF` (offset of the first
/// global tail table = end of the last module).
pub fn module_region_end(bytes: &[u8]) -> Result<usize, WireError> {
    let mut c = Cursor::at(bytes, CacheHeader::SIZE); // skip FGuid+magic+count (0x18)
    // Re-read the count from its known offset (0x14) rather than trusting header parse.
    let count = u32::from_le_bytes(bytes[0x14..0x18].try_into().unwrap()) as usize;
    for _ in 0..count {
        // Modules is TMap<FString key, FAngelscriptPrecompiledModule value>.
        c.read_fstring()?; // key (UE FString)
        read_module(&mut c)?;
    }
    Ok(c.pos())
}

/// Number of modules declared in the header (@0x14).
pub fn module_count(bytes: &[u8]) -> u32 {
    u32::from_le_bytes(bytes[0x14..0x18].try_into().unwrap())
}

/// Collect every module's name (the `Modules` TMap key) in order.
pub fn module_names(bytes: &[u8]) -> Result<Vec<String>, WireError> {
    let mut c = Cursor::at(bytes, CacheHeader::SIZE);
    let count = module_count(bytes) as usize;
    let mut names = Vec::with_capacity(count);
    for _ in 0..count {
        names.push(c.read_fstring()?); // key
        read_module(&mut c)?;
    }
    Ok(names)
}

/// A function's compiled bytecode, captured during a walk.
#[derive(Debug, Clone)]
pub struct FuncCode {
    /// `module::func` or `module.Class::method`.
    pub func: String,
    /// Raw `TArray<int32>` bytecode (the asBC dword stream).
    pub bytecode: Vec<i32>,
}

/// Walk all modules and collect every function's bytecode (free functions, class
/// methods/constructors/behaviors, and global-var init functions).
pub fn collect_function_bytecodes(bytes: &[u8]) -> Result<Vec<FuncCode>, WireError> {
    let mut c = Cursor::at(bytes, CacheHeader::SIZE);
    let count = module_count(bytes) as usize;
    let mut out = Vec::new();
    for _ in 0..count {
        c.read_fstring()?; // key
        read_module_c(&mut c, &mut out)?;
    }
    Ok(out)
}

fn read_function_c(c: &mut Cursor, scope: &str, out: &mut Vec<FuncCode>) -> Result<(), WireError> {
    let name = c.read_sia()?;
    c.read_sia()?; // Namespace
    read_data_type(c)?; // ReturnType
    c.skip_tarray_fixed(DATA_TYPE_SIZE, "ParameterTypes")?;
    c.skip_tarray_sia("ParameterNames")?;
    c.skip_tarray_fixed(4, "ParameterFlags")?;
    c.skip_tarray_sia("ParameterDefaultArgs")?;
    c.skip(4)?; // FunctionTraits
    let bytecode = c.read_tarray_i32("ByteCode")?;
    c.skip_tarray_fixed(4, "ByteCodeReferences")?;
    c.skip(4)?; // VariableSpace
    c.skip_tarray_fixed(8, "ObjVariableTypes")?;
    c.skip_tarray_fixed(4, "ObjVariablePos")?;
    c.skip(4)?; // ObjVariablesOnHeap
    c.skip_tarray_fixed(4, "VariableInfoProgramPos")?;
    c.skip_tarray_fixed(4, "VariableInfoOffset")?;
    c.skip_tarray_fixed(4, "VariableInfoOption")?;
    c.skip(4)?; // StackNeeded
    c.skip(4)?; // Id
    c.skip(4)?; // DeclaredAt
    c.skip_tarray_fixed(4, "LineNumbers")?;
    if c.read_bool4()? {
        c.read_sia()?; // UnrealFunctionName
        c.skip_tarray_sia("UF.MetaSpec")?;
        c.skip_tarray_sia("UF.MetaValues")?;
        c.skip(18 * 4)?;
    }
    out.push(FuncCode {
        func: format!("{scope}::{name}"),
        bytecode,
    });
    Ok(())
}

fn read_class_c(c: &mut Cursor, module: &str, out: &mut Vec<FuncCode>) -> Result<(), WireError> {
    let class_name = c.read_sia()?;
    c.read_sia()?; // Namespace
    c.skip(4)?; // Flags
    let scope = format!("{module}.{class_name}");
    let nprops = c.read_count("Class.Properties")?;
    for _ in 0..nprops {
        read_property(c)?;
    }
    let nmethods = c.read_count("Class.Methods")?;
    for _ in 0..nmethods {
        read_function_c(c, &scope, out)?;
    }
    c.skip_tarray_fixed(4, "Class.MethodTable")?;
    c.skip(8)?; // DerivedFrom
    c.skip(8)?; // ShadowType
    let nctors = c.read_count("Class.Constructors")?;
    for _ in 0..nctors {
        read_function_c(c, &scope, out)?;
    }
    c.skip_tarray_fixed(8, "Class.FactoryRefs")?;
    c.skip_tarray_fixed(8, "Class.BehaviorRefs")?;
    let nbehav = c.read_count("Class.BehaviorFunctions")?;
    for _ in 0..nbehav {
        read_function_c(c, &scope, out)?;
    }
    c.skip_tarray_fixed(4, "Class.BehaviorFunctionTypes")?;
    if c.read_bool4()? {
        c.read_sia()?; // SuperClass
        c.read_sia()?; // CodeSuperClass
        c.skip(8 * 4)?;
        c.read_sia()?; // StaticClassGVName
        c.skip(4)?; // bPlaceable
        c.skip_tarray_sia("Class.MetaSpec")?;
        c.skip_tarray_sia("Class.MetaValues")?;
        c.read_sia()?; // ComposeOntoClassName
    }
    Ok(())
}

fn read_global_c(c: &mut Cursor, module: &str, out: &mut Vec<FuncCode>) -> Result<(), WireError> {
    let name = c.read_sia()?;
    c.read_sia()?; // Namespace
    read_data_type(c)?; // Type
    if !c.read_bool4()? {
        // !bIsDefaultInit
        if c.read_bool4()? {
            c.skip(8)?; // PureConstantValue
        } else if c.read_bool4()? {
            // bHasInitFunction
            read_function_c(c, &format!("{module}.<glob:{name}>"), out)?;
        }
    }
    Ok(())
}

fn read_module_c(c: &mut Cursor, out: &mut Vec<FuncCode>) -> Result<(), WireError> {
    let module = c.read_sia()?;
    let nfns = c.read_count("Module.Functions")?;
    for _ in 0..nfns {
        read_function_c(c, &module, out)?;
    }
    let nclasses = c.read_count("Module.Classes")?;
    for _ in 0..nclasses {
        read_class_c(c, &module, out)?;
    }
    let nenums = c.read_count("Module.Enums")?;
    for _ in 0..nenums {
        read_enum(c)?;
    }
    let nglobals = c.read_count("Module.GlobalVariables")?;
    for _ in 0..nglobals {
        read_global_c(c, &module, out)?;
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
    c.read_sia()?; // ScriptRelativeFilename
    c.skip_tarray_sia("Module.PostInitFunctions")?;
    Ok(())
}

fn read_data_type(c: &mut Cursor) -> Result<(), WireError> {
    c.skip(DATA_TYPE_SIZE)
}

fn read_function(c: &mut Cursor) -> Result<(), WireError> {
    c.read_sia()?; // FunctionName
    c.read_sia()?; // Namespace
    read_data_type(c)?; // ReturnType
    c.skip_tarray_fixed(DATA_TYPE_SIZE, "ParameterTypes")?;
    c.skip_tarray_sia("ParameterNames")?;
    c.skip_tarray_fixed(4, "ParameterFlags")?;
    c.skip_tarray_sia("ParameterDefaultArgs")?;
    c.skip(4)?; // FunctionTraits
    c.skip_tarray_fixed(4, "ByteCode")?;
    c.skip_tarray_fixed(4, "ByteCodeReferences")?;
    c.skip(4)?; // VariableSpace
    c.skip_tarray_fixed(8, "ObjVariableTypes")?; // TArray<int64 Reference>
    c.skip_tarray_fixed(4, "ObjVariablePos")?;
    c.skip(4)?; // ObjVariablesOnHeap
    c.skip_tarray_fixed(4, "VariableInfoProgramPos")?;
    c.skip_tarray_fixed(4, "VariableInfoOffset")?;
    c.skip_tarray_fixed(4, "VariableInfoOption")?;
    c.skip(4)?; // StackNeeded
    c.skip(4)?; // Id (uint32)
    c.skip(4)?; // DeclaredAt
    c.skip_tarray_fixed(4, "LineNumbers")?;
    let is_ufunction = c.read_bool4()?;
    if is_ufunction {
        // HYPOTHESIS (container-splice.md §9.0): not present in either Rosetta sample.
        c.read_sia()?; // UnrealFunctionName
        c.skip_tarray_sia("UF.MetaSpec")?;
        c.skip_tarray_sia("UF.MetaValues")?;
        c.skip(18 * 4)?; // 18 × bool
    }
    Ok(())
}

fn read_property(c: &mut Cursor) -> Result<(), WireError> {
    c.read_sia()?; // Name
    read_data_type(c)?; // Type
    c.skip(4)?; // bIsPrivate
    c.skip(4)?; // bIsProtected
    let is_unreal = c.read_bool4()?;
    if is_unreal {
        // HYPOTHESIS (§9.4): UPROPERTY tail, not present in the richtest sample.
        c.skip_tarray_sia("UP.MetaSpec")?;
        c.skip_tarray_sia("UP.MetaValues")?;
        c.skip(9 * 4)?; // 9 bools (bBlueprintReadable..bTransient)
        let replicated = c.read_bool4()?;
        c.skip(4)?; // bSkipReplication
        c.skip(4)?; // bSkipSerialization
        c.skip(4)?; // bSaveGame
        if replicated {
            c.skip(4)?; // ReplicationCondition (int32)
            c.skip(4)?; // bRepNotify
        }
        c.skip(4)?; // bConfig
        c.skip(4)?; // bInterp
        c.skip(4)?; // bAssetRegistrySearchable
    }
    Ok(())
}

fn read_class(c: &mut Cursor) -> Result<(), WireError> {
    c.read_sia()?; // ClassName
    c.read_sia()?; // Namespace
    c.skip(4)?; // Flags (int32)
    let nprops = c.read_count("Class.Properties")?;
    for _ in 0..nprops {
        read_property(c)?;
    }
    let nmethods = c.read_count("Class.Methods")?;
    for _ in 0..nmethods {
        read_function(c)?;
    }
    c.skip_tarray_fixed(4, "Class.MethodTable")?;
    c.skip(8)?; // DerivedFrom (Reference int64)
    c.skip(8)?; // ShadowType (Reference int64)
    let nctors = c.read_count("Class.Constructors")?;
    for _ in 0..nctors {
        read_function(c)?;
    }
    c.skip_tarray_fixed(8, "Class.FactoryRefs")?; // TArray<int64>
    c.skip_tarray_fixed(8, "Class.BehaviorRefs")?; // TArray<int64>
    let nbehav = c.read_count("Class.BehaviorFunctions")?;
    for _ in 0..nbehav {
        read_function(c)?;
    }
    c.skip_tarray_fixed(4, "Class.BehaviorFunctionTypes")?;
    let in_preprocessor = c.read_bool4()?;
    if in_preprocessor {
        c.read_sia()?; // SuperClass
        c.read_sia()?; // CodeSuperClass
        c.skip(8 * 4)?; // 8 × int32 (class bools)
        c.read_sia()?; // StaticClassGVName
        c.skip(4)?; // bPlaceable
        c.skip_tarray_sia("Class.MetaSpec")?;
        c.skip_tarray_sia("Class.MetaValues")?;
        c.read_sia()?; // ComposeOntoClassName
    }
    Ok(())
}

fn read_enum(c: &mut Cursor) -> Result<(), WireError> {
    c.read_sia()?; // Name
    c.read_sia()?; // Namespace
    c.skip_tarray_sia("Enum.Names")?;
    c.skip_tarray_fixed(4, "Enum.Values")?;
    Ok(())
}

fn read_global(c: &mut Cursor) -> Result<(), WireError> {
    c.read_sia()?; // Name
    c.read_sia()?; // Namespace
    read_data_type(c)?; // Type
    let default_init = c.read_bool4()?;
    if !default_init {
        let pure_const = c.read_bool4()?;
        if pure_const {
            c.skip(8)?; // PureConstantValue (uint64)
        } else {
            let has_init = c.read_bool4()?;
            if has_init {
                read_function(c)?; // InitFunc
            }
        }
    }
    Ok(())
}

fn read_function_import(c: &mut Cursor) -> Result<(), WireError> {
    c.read_sia()?; // ImportedFromModule
    // FAngelscriptPrecompiledFunctionSignature
    c.read_sia()?; // Name
    c.read_sia()?; // Namespace
    c.skip_tarray_fixed(DATA_TYPE_SIZE, "Import.ParameterTypes")?;
    c.skip_tarray_fixed(4, "Import.ParameterFlags")?;
    c.skip_tarray_sia("Import.ParameterDefaultArgs")?;
    read_data_type(c)?; // ReturnType
    Ok(())
}

fn read_module(c: &mut Cursor) -> Result<(), WireError> {
    c.read_sia()?; // ModuleName
    let nfns = c.read_count("Module.Functions")?;
    for _ in 0..nfns {
        read_function(c)?;
    }
    let nclasses = c.read_count("Module.Classes")?;
    for _ in 0..nclasses {
        read_class(c)?;
    }
    let nenums = c.read_count("Module.Enums")?;
    for _ in 0..nenums {
        read_enum(c)?;
    }
    let nglobals = c.read_count("Module.GlobalVariables")?;
    for _ in 0..nglobals {
        read_global(c)?;
    }
    let nimports = c.read_count("Module.FunctionImports")?;
    for _ in 0..nimports {
        read_function_import(c)?;
    }
    c.skip(8)?; // CodeHash (int64)
    c.skip_tarray_sia("Module.ImportedModules")?;
    c.read_sia()?; // StaticsClassName
    c.skip_tarray_sia("Module.DeclaredEvents")?;
    c.skip_tarray_sia("Module.DeclaredDelegates")?;
    c.read_sia()?; // ScriptRelativeFilename
    c.skip_tarray_sia("Module.PostInitFunctions")?;
    Ok(())
}
