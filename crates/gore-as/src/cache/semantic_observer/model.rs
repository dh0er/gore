use std::mem::size_of;

use super::SemanticObserverError;
use crate::cache::wire::Cursor;

pub(super) const MAX_CACHE_BYTES: usize = 512 * 1024 * 1024;
const MAX_DECODED_HEAP_BYTES: usize = 1024 * 1024 * 1024;

#[derive(Debug, Clone)]
pub(super) struct DataType {
    pub flags: [bool; 6],
    pub type_info: i64,
    pub token: i32,
}

#[derive(Debug, Clone)]
pub(super) struct Function {
    pub name: String,
    pub namespace: String,
    pub return_type: DataType,
    pub parameter_types: Vec<DataType>,
    pub parameter_names: Vec<String>,
    pub parameter_flags: Vec<i32>,
    pub parameter_default_args: Vec<String>,
    pub traits: i32,
    pub bytecode: Vec<i32>,
    pub bytecode_references: Vec<i32>,
    pub variable_space: i32,
    pub object_variable_types: Vec<i64>,
    pub object_variable_positions: Vec<i32>,
    pub object_variables_on_heap: i32,
    pub var_info_program_positions: Vec<i32>,
    pub var_info_offsets: Vec<i32>,
    pub var_info_options: Vec<i32>,
    pub stack_needed: i32,
    pub id: u32,
    pub declared_at: i32,
    pub line_numbers: Vec<i32>,
    pub unreal: Option<UnrealFunction>,
}

#[derive(Debug, Clone)]
pub(super) struct UnrealFunction {
    pub unreal_name: String,
    pub metadata_specifiers: Vec<String>,
    pub metadata_values: Vec<String>,
    pub flags: [bool; 18],
}

#[derive(Debug, Clone)]
pub(super) struct Property {
    pub name: String,
    pub data_type: DataType,
    pub is_private: bool,
    pub is_protected: bool,
    pub unreal: Option<UnrealProperty>,
}

#[derive(Debug, Clone)]
pub(super) struct UnrealProperty {
    pub metadata_specifiers: Vec<String>,
    pub metadata_values: Vec<String>,
    pub flags_before_replication: [bool; 9],
    pub replicated: bool,
    pub skip_replication: bool,
    pub skip_serialization: bool,
    pub save_game: bool,
    pub replication: Option<(i32, bool)>,
    pub config: bool,
    pub interp: bool,
    pub asset_registry_searchable: bool,
}

#[derive(Debug, Clone)]
pub(super) struct Class {
    pub name: String,
    pub namespace: String,
    pub flags: i32,
    pub properties: Vec<Property>,
    pub methods: Vec<Function>,
    pub method_table: Vec<i32>,
    pub derived_from: i64,
    pub shadow_type: i64,
    pub constructors: Vec<Function>,
    pub factory_references: Vec<i64>,
    pub behaviour_references: Vec<i64>,
    pub behaviour_functions: Vec<Function>,
    pub behaviour_function_types: Vec<i32>,
    pub preprocessor: Option<PreprocessorClass>,
}

#[derive(Debug, Clone)]
pub(super) struct PreprocessorClass {
    pub super_class: String,
    pub code_super_class: String,
    pub flags: [bool; 7],
    pub config_name: String,
    pub static_class_global_variable_name: String,
    pub placeable: bool,
    pub metadata_specifiers: Vec<String>,
    pub metadata_values: Vec<String>,
    pub compose_onto_class_name: String,
}

#[derive(Debug, Clone)]
pub(super) struct Enum {
    pub name: String,
    pub namespace: String,
    pub names: Vec<String>,
    pub values: Vec<i32>,
}

#[derive(Debug, Clone)]
pub(super) struct Global {
    pub name: String,
    pub namespace: String,
    pub data_type: DataType,
    pub initializer: GlobalInitializer,
}

#[derive(Debug, Clone)]
pub(super) enum GlobalInitializer {
    Default,
    PureConstant(u64),
    Function { present: bool, function: Function },
}

#[derive(Debug, Clone)]
pub(super) struct FunctionSignature {
    pub name: String,
    pub namespace: String,
    pub parameter_types: Vec<DataType>,
    pub parameter_flags: Vec<i32>,
    pub parameter_default_args: Vec<String>,
    pub return_type: DataType,
}

#[derive(Debug, Clone)]
pub(super) struct FunctionImport {
    pub imported_from_module: String,
    pub signature: FunctionSignature,
}

#[derive(Debug, Clone)]
pub(super) struct Module {
    pub map_key: String,
    pub name: String,
    pub functions: Vec<Function>,
    pub classes: Vec<Class>,
    pub enums: Vec<Enum>,
    pub globals: Vec<Global>,
    pub function_imports: Vec<FunctionImport>,
    pub code_hash: i64,
    pub imported_modules: Vec<String>,
    pub statics_class_name: String,
    pub declared_events: Vec<String>,
    pub declared_delegates: Vec<String>,
    pub script_relative_filename: String,
    pub post_init_functions: Vec<String>,
}

#[derive(Debug, Clone)]
pub(super) struct TypeReference {
    pub raw_key: i64,
    pub name: String,
    pub module: String,
    pub namespace: String,
    pub sub_types: Vec<DataType>,
}

#[derive(Debug, Clone)]
pub(super) struct FunctionReference {
    pub raw_key: i64,
    pub name: String,
    pub module: String,
    pub namespace: String,
    pub is_const: bool,
    pub is_imported_decl: bool,
    pub is_method: bool,
    pub object_type: i64,
    pub parameter_types: Vec<DataType>,
    pub return_type: DataType,
}

#[derive(Debug, Clone)]
pub(super) struct GlobalReference {
    pub raw_key: i64,
    pub name: String,
    pub module: String,
    pub namespace: String,
    pub is_string: bool,
}

#[derive(Debug, Clone)]
pub(super) struct PropertyReference {
    pub raw_key: i64,
    pub name: String,
    pub old_type_id: i32,
}

#[derive(Debug, Clone)]
pub(super) struct Cache {
    pub build_identifier: i32,
    pub modules: Vec<Module>,
    pub type_references: Vec<TypeReference>,
    pub type_ids: Vec<(i32, i64)>,
    pub function_references: Vec<FunctionReference>,
    pub function_ids: Vec<(i32, i64)>,
    pub global_references: Vec<GlobalReference>,
    pub static_names: Vec<String>,
    pub property_references: Vec<PropertyReference>,
}

struct Budget {
    heap: usize,
}

impl Budget {
    fn new() -> Self {
        Self { heap: 0 }
    }

    fn charge(
        &mut self,
        bytes: usize,
        resource: &'static str,
    ) -> Result<(), SemanticObserverError> {
        self.heap = self
            .heap
            .checked_add(bytes)
            .ok_or(SemanticObserverError::ResourceLimit {
                resource,
                actual: usize::MAX,
                limit: MAX_DECODED_HEAP_BYTES,
            })?;
        if self.heap > MAX_DECODED_HEAP_BYTES {
            return Err(SemanticObserverError::ResourceLimit {
                resource,
                actual: self.heap,
                limit: MAX_DECODED_HEAP_BYTES,
            });
        }
        Ok(())
    }
}

fn read_string(
    cursor: &mut Cursor<'_>,
    budget: &mut Budget,
) -> Result<String, SemanticObserverError> {
    let value = cursor.read_sia()?;
    budget.charge(value.len(), "decoded strings")?;
    Ok(value)
}

fn read_strings(
    cursor: &mut Cursor<'_>,
    budget: &mut Budget,
    field: &'static str,
) -> Result<Vec<String>, SemanticObserverError> {
    read_vec(cursor, budget, field, 4, |c, b| read_string(c, b))
}

fn read_vec<T>(
    cursor: &mut Cursor<'_>,
    budget: &mut Budget,
    field: &'static str,
    minimum_wire_bytes: usize,
    mut read: impl FnMut(&mut Cursor<'_>, &mut Budget) -> Result<T, SemanticObserverError>,
) -> Result<Vec<T>, SemanticObserverError> {
    let count = cursor.read_count(field)?;
    cursor.ensure_minimum_remaining(count, minimum_wire_bytes, field)?;
    budget.charge(
        count
            .checked_mul(size_of::<T>())
            .ok_or(SemanticObserverError::ResourceLimit {
                resource: field,
                actual: usize::MAX,
                limit: MAX_DECODED_HEAP_BYTES,
            })?,
        field,
    )?;
    let mut values = Vec::new();
    values
        .try_reserve_exact(count)
        .map_err(|_| SemanticObserverError::AllocationFailed { resource: field })?;
    for _ in 0..count {
        values.push(read(cursor, budget)?);
    }
    Ok(values)
}

fn read_i32s(
    cursor: &mut Cursor<'_>,
    budget: &mut Budget,
    field: &'static str,
) -> Result<Vec<i32>, SemanticObserverError> {
    read_vec(cursor, budget, field, 4, |c, _| Ok(c.read_i32()?))
}

fn read_i64s(
    cursor: &mut Cursor<'_>,
    budget: &mut Budget,
    field: &'static str,
) -> Result<Vec<i64>, SemanticObserverError> {
    read_vec(cursor, budget, field, 8, |c, _| Ok(c.read_i64()?))
}

fn read_data_type(cursor: &mut Cursor<'_>) -> Result<DataType, SemanticObserverError> {
    Ok(DataType {
        flags: [
            cursor.read_bool4()?,
            cursor.read_bool4()?,
            cursor.read_bool4()?,
            cursor.read_bool4()?,
            cursor.read_bool4()?,
            cursor.read_bool4()?,
        ],
        type_info: cursor.read_i64()?,
        token: cursor.read_i32()?,
    })
}

fn read_data_types(
    cursor: &mut Cursor<'_>,
    budget: &mut Budget,
    field: &'static str,
) -> Result<Vec<DataType>, SemanticObserverError> {
    read_vec(cursor, budget, field, 36, |c, _| read_data_type(c))
}

fn read_function(
    cursor: &mut Cursor<'_>,
    budget: &mut Budget,
) -> Result<Function, SemanticObserverError> {
    let name = read_string(cursor, budget)?;
    let namespace = read_string(cursor, budget)?;
    let return_type = read_data_type(cursor)?;
    let parameter_types = read_data_types(cursor, budget, "Function.ParameterTypes")?;
    let parameter_names = read_strings(cursor, budget, "Function.ParameterNames")?;
    let parameter_flags = read_i32s(cursor, budget, "Function.ParameterFlags")?;
    let parameter_default_args = read_strings(cursor, budget, "Function.ParameterDefaultArgs")?;
    let traits = cursor.read_i32()?;
    let bytecode = read_i32s(cursor, budget, "Function.ByteCode")?;
    let bytecode_references = read_i32s(cursor, budget, "Function.ByteCodeReferences")?;
    let variable_space = cursor.read_i32()?;
    let object_variable_types = read_i64s(cursor, budget, "Function.ObjVariableTypes")?;
    let object_variable_positions = read_i32s(cursor, budget, "Function.ObjVariablePos")?;
    let object_variables_on_heap = cursor.read_i32()?;
    let var_info_program_positions = read_i32s(cursor, budget, "Function.VarInfoProgramPos")?;
    let var_info_offsets = read_i32s(cursor, budget, "Function.VarInfoOffset")?;
    let var_info_options = read_i32s(cursor, budget, "Function.VarInfoOption")?;
    let stack_needed = cursor.read_i32()?;
    let id = cursor.read_u32()?;
    let declared_at = cursor.read_i32()?;
    let line_numbers = read_i32s(cursor, budget, "Function.LineNumbers")?;
    let unreal = if cursor.read_bool4()? {
        Some(UnrealFunction {
            unreal_name: read_string(cursor, budget)?,
            metadata_specifiers: read_strings(cursor, budget, "Function.MetaSpec")?,
            metadata_values: read_strings(cursor, budget, "Function.MetaValues")?,
            flags: [
                cursor.read_bool4()?,
                cursor.read_bool4()?,
                cursor.read_bool4()?,
                cursor.read_bool4()?,
                cursor.read_bool4()?,
                cursor.read_bool4()?,
                cursor.read_bool4()?,
                cursor.read_bool4()?,
                cursor.read_bool4()?,
                cursor.read_bool4()?,
                cursor.read_bool4()?,
                cursor.read_bool4()?,
                cursor.read_bool4()?,
                cursor.read_bool4()?,
                cursor.read_bool4()?,
                cursor.read_bool4()?,
                cursor.read_bool4()?,
                cursor.read_bool4()?,
            ],
        })
    } else {
        None
    };
    Ok(Function {
        name,
        namespace,
        return_type,
        parameter_types,
        parameter_names,
        parameter_flags,
        parameter_default_args,
        traits,
        bytecode,
        bytecode_references,
        variable_space,
        object_variable_types,
        object_variable_positions,
        object_variables_on_heap,
        var_info_program_positions,
        var_info_offsets,
        var_info_options,
        stack_needed,
        id,
        declared_at,
        line_numbers,
        unreal,
    })
}

fn read_property(
    cursor: &mut Cursor<'_>,
    budget: &mut Budget,
) -> Result<Property, SemanticObserverError> {
    let name = read_string(cursor, budget)?;
    let data_type = read_data_type(cursor)?;
    let is_private = cursor.read_bool4()?;
    let is_protected = cursor.read_bool4()?;
    let unreal = if cursor.read_bool4()? {
        let metadata_specifiers = read_strings(cursor, budget, "Property.MetaSpec")?;
        let metadata_values = read_strings(cursor, budget, "Property.MetaValues")?;
        let flags_before_replication = [
            cursor.read_bool4()?,
            cursor.read_bool4()?,
            cursor.read_bool4()?,
            cursor.read_bool4()?,
            cursor.read_bool4()?,
            cursor.read_bool4()?,
            cursor.read_bool4()?,
            cursor.read_bool4()?,
            cursor.read_bool4()?,
        ];
        let replicated = cursor.read_bool4()?;
        let skip_replication = cursor.read_bool4()?;
        let skip_serialization = cursor.read_bool4()?;
        let save_game = cursor.read_bool4()?;
        let replication = replicated
            .then(|| Ok::<_, SemanticObserverError>((cursor.read_i32()?, cursor.read_bool4()?)))
            .transpose()?;
        Some(UnrealProperty {
            metadata_specifiers,
            metadata_values,
            flags_before_replication,
            replicated,
            skip_replication,
            skip_serialization,
            save_game,
            replication,
            config: cursor.read_bool4()?,
            interp: cursor.read_bool4()?,
            asset_registry_searchable: cursor.read_bool4()?,
        })
    } else {
        None
    };
    Ok(Property {
        name,
        data_type,
        is_private,
        is_protected,
        unreal,
    })
}

fn read_class(
    cursor: &mut Cursor<'_>,
    budget: &mut Budget,
) -> Result<Class, SemanticObserverError> {
    let name = read_string(cursor, budget)?;
    let namespace = read_string(cursor, budget)?;
    let flags = cursor.read_i32()?;
    let properties = read_vec(cursor, budget, "Class.Properties", 52, read_property)?;
    let methods = read_vec(cursor, budget, "Class.Methods", 120, read_function)?;
    let method_table = read_i32s(cursor, budget, "Class.MethodTable")?;
    let derived_from = cursor.read_i64()?;
    let shadow_type = cursor.read_i64()?;
    let constructors = read_vec(cursor, budget, "Class.Constructors", 120, read_function)?;
    let factory_references = read_i64s(cursor, budget, "Class.FactoryRefs")?;
    let behaviour_references = read_i64s(cursor, budget, "Class.BehaviorRefs")?;
    let behaviour_functions = read_vec(
        cursor,
        budget,
        "Class.BehaviorFunctions",
        120,
        read_function,
    )?;
    let behaviour_function_types = read_i32s(cursor, budget, "Class.BehaviorFunctionTypes")?;
    let preprocessor = if cursor.read_bool4()? {
        Some(PreprocessorClass {
            super_class: read_string(cursor, budget)?,
            code_super_class: read_string(cursor, budget)?,
            flags: [
                cursor.read_bool4()?,
                cursor.read_bool4()?,
                cursor.read_bool4()?,
                cursor.read_bool4()?,
                cursor.read_bool4()?,
                cursor.read_bool4()?,
                cursor.read_bool4()?,
            ],
            config_name: read_string(cursor, budget)?,
            static_class_global_variable_name: read_string(cursor, budget)?,
            placeable: cursor.read_bool4()?,
            metadata_specifiers: read_strings(cursor, budget, "Class.MetaSpec")?,
            metadata_values: read_strings(cursor, budget, "Class.MetaValues")?,
            compose_onto_class_name: read_string(cursor, budget)?,
        })
    } else {
        None
    };
    Ok(Class {
        name,
        namespace,
        flags,
        properties,
        methods,
        method_table,
        derived_from,
        shadow_type,
        constructors,
        factory_references,
        behaviour_references,
        behaviour_functions,
        behaviour_function_types,
        preprocessor,
    })
}

fn read_enum(cursor: &mut Cursor<'_>, budget: &mut Budget) -> Result<Enum, SemanticObserverError> {
    Ok(Enum {
        name: read_string(cursor, budget)?,
        namespace: read_string(cursor, budget)?,
        names: read_strings(cursor, budget, "Enum.EnumNames")?,
        values: read_i32s(cursor, budget, "Enum.EnumValues")?,
    })
}

fn read_global(
    cursor: &mut Cursor<'_>,
    budget: &mut Budget,
) -> Result<Global, SemanticObserverError> {
    let name = read_string(cursor, budget)?;
    let namespace = read_string(cursor, budget)?;
    let data_type = read_data_type(cursor)?;
    let initializer = if cursor.read_bool4()? {
        GlobalInitializer::Default
    } else if cursor.read_bool4()? {
        GlobalInitializer::PureConstant(cursor.read_u64()?)
    } else {
        GlobalInitializer::Function {
            present: cursor.read_bool4()?,
            function: read_function(cursor, budget)?,
        }
    };
    Ok(Global {
        name,
        namespace,
        data_type,
        initializer,
    })
}

fn read_signature(
    cursor: &mut Cursor<'_>,
    budget: &mut Budget,
) -> Result<FunctionSignature, SemanticObserverError> {
    Ok(FunctionSignature {
        name: read_string(cursor, budget)?,
        namespace: read_string(cursor, budget)?,
        parameter_types: read_data_types(cursor, budget, "FunctionSignature.ParameterTypes")?,
        parameter_flags: read_i32s(cursor, budget, "FunctionSignature.ParameterFlags")?,
        parameter_default_args: read_strings(
            cursor,
            budget,
            "FunctionSignature.ParameterDefaultArgs",
        )?,
        return_type: read_data_type(cursor)?,
    })
}

fn read_import(
    cursor: &mut Cursor<'_>,
    budget: &mut Budget,
) -> Result<FunctionImport, SemanticObserverError> {
    Ok(FunctionImport {
        imported_from_module: read_string(cursor, budget)?,
        signature: read_signature(cursor, budget)?,
    })
}

fn read_module(
    cursor: &mut Cursor<'_>,
    budget: &mut Budget,
    map_key: String,
) -> Result<Module, SemanticObserverError> {
    Ok(Module {
        map_key,
        name: read_string(cursor, budget)?,
        functions: read_vec(cursor, budget, "Module.Functions", 120, read_function)?,
        classes: read_vec(cursor, budget, "Module.Classes", 64, read_class)?,
        enums: read_vec(cursor, budget, "Module.Enums", 16, read_enum)?,
        globals: read_vec(cursor, budget, "Module.GlobalVariables", 48, read_global)?,
        function_imports: read_vec(cursor, budget, "Module.FunctionImports", 60, read_import)?,
        code_hash: cursor.read_i64()?,
        imported_modules: read_strings(cursor, budget, "Module.ImportedModules")?,
        statics_class_name: read_string(cursor, budget)?,
        declared_events: read_strings(cursor, budget, "Module.DeclaredEvents")?,
        declared_delegates: read_strings(cursor, budget, "Module.DeclaredDelegates")?,
        script_relative_filename: read_string(cursor, budget)?,
        post_init_functions: read_strings(cursor, budget, "Module.PostInitFunctions")?,
    })
}

pub(super) fn decode(bytes: &[u8]) -> Result<Cache, SemanticObserverError> {
    if bytes.len() > MAX_CACHE_BYTES {
        return Err(SemanticObserverError::ResourceLimit {
            resource: "cache bytes",
            actual: bytes.len(),
            limit: MAX_CACHE_BYTES,
        });
    }
    let mut cursor = Cursor::new(bytes);
    let mut budget = Budget::new();
    cursor.skip(16)?; // DataGuid: deliberately normalized by the semantic layer.
    let build_identifier = cursor.read_i32()?;

    let module_count = cursor.read_count("Modules")?;
    cursor.ensure_minimum_remaining(module_count, 60, "Modules")?;
    budget.charge(module_count * size_of::<Module>(), "Modules")?;
    let mut modules = Vec::new();
    modules.try_reserve_exact(module_count).map_err(|_| {
        SemanticObserverError::AllocationFailed {
            resource: "Modules",
        }
    })?;
    for _ in 0..module_count {
        let map_key = cursor.read_fstring()?;
        budget.charge(map_key.len(), "module map keys")?;
        modules.push(read_module(&mut cursor, &mut budget, map_key)?);
    }

    let type_references = read_vec(&mut cursor, &mut budget, "TypeReferences", 24, |c, b| {
        Ok(TypeReference {
            raw_key: c.read_i64()?,
            name: read_string(c, b)?,
            module: read_string(c, b)?,
            namespace: read_string(c, b)?,
            sub_types: read_data_types(c, b, "TypeReference.SubTypes")?,
        })
    })?;
    let type_ids = read_vec(
        &mut cursor,
        &mut budget,
        "TypeIdReferenceToPointer",
        12,
        |c, _| Ok((c.read_i32()?, c.read_i64()?)),
    )?;
    let function_references = read_vec(
        &mut cursor,
        &mut budget,
        "FunctionReferences",
        80,
        |c, b| {
            Ok(FunctionReference {
                raw_key: c.read_i64()?,
                name: read_string(c, b)?,
                module: read_string(c, b)?,
                namespace: read_string(c, b)?,
                is_const: c.read_bool4()?,
                is_imported_decl: c.read_bool4()?,
                is_method: c.read_bool4()?,
                object_type: c.read_i64()?,
                parameter_types: read_data_types(c, b, "FunctionReference.ParameterTypes")?,
                return_type: read_data_type(c)?,
            })
        },
    )?;
    let function_ids = read_vec(
        &mut cursor,
        &mut budget,
        "FunctionIdReferenceToPointer",
        12,
        |c, _| Ok((c.read_i32()?, c.read_i64()?)),
    )?;
    let global_references = read_vec(&mut cursor, &mut budget, "GlobalReferences", 24, |c, b| {
        let raw_key = c.read_i64()?;
        let name_position = c.pos();
        let name_bytes = c.read_sia_bytes()?;
        let module = read_string(c, b)?;
        let namespace = read_string(c, b)?;
        let is_string = c.read_bool4()?;
        let name = if is_string {
            name_bytes.decode_utf8(name_position)?
        } else {
            name_bytes.decode_ansi()
        };
        b.charge(name.len(), "GlobalReference.Name")?;
        Ok(GlobalReference {
            raw_key,
            name,
            module,
            namespace,
            is_string,
        })
    })?;
    let static_names = read_strings(&mut cursor, &mut budget, "StaticNames")?;
    let property_references = read_vec(
        &mut cursor,
        &mut budget,
        "PropertyReferences",
        16,
        |c, b| {
            Ok(PropertyReference {
                raw_key: c.read_i64()?,
                name: read_string(c, b)?,
                old_type_id: c.read_i32()?,
            })
        },
    )?;
    if cursor.remaining() != 0 {
        return Err(SemanticObserverError::TrailingBytes {
            offset: cursor.pos(),
            remaining: cursor.remaining(),
        });
    }
    Ok(Cache {
        build_identifier,
        modules,
        type_references,
        type_ids,
        function_references,
        function_ids,
        global_references,
        static_names,
        property_references,
    })
}
