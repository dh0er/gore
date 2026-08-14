use gore_as::cache::disasm::disassemble;
use gore_as::cache::header::CACHE_MAGIC;
use gore_as::cache::refs::RefResolver;
use gore_as::cache::remap::{
    remap_module_to_base, remap_module_to_base_with_options, RemapError, RemapOptions,
};
use gore_as::cache::splice::{
    extract_module, remap_module_to_base_with_loadout_plan, replace_module, splice, splice_auto,
    LoadoutScriptIdPlanBuilder, SequentialMiniGuard, SpliceError,
};
use gore_as::cache::tables::parse_tail_tables;
use gore_as::cache::walk_modules::{
    collect_function_bytecodes, module_count, module_names, module_region_end,
};
use gore_as::cache::wire::WireError;

const MODULE: &str = "EditedModule";

const BASE_TYPE_PTR: i64 = 0x1111;
const REGEN_TYPE_PTR: i64 = 0x2111;
const NEW_TYPE_PTR: i64 = BASE_TYPE_PTR; // deliberate base collision
const BASE_TYPE_ID: i32 = 0x0800_0100;
const REGEN_TYPE_ID: i32 = 0x0800_0200;
const NEW_TYPE_ID: i32 = BASE_TYPE_ID; // deliberate base collision

const BASE_FUNC_PTR: i64 = 0x1222;
const REGEN_FUNC_PTR: i64 = 0x2222;
const NEW_FUNC_PTR: i64 = BASE_FUNC_PTR; // deliberate base collision
const BASE_FUNC_ID: i32 = 10;
const REGEN_FUNC_ID: i32 = 20;
const NEW_FUNC_ID: i32 = BASE_FUNC_ID; // deliberate base collision

const BASE_STATIC_FUNC_PTR: i64 = 0x1333;
const REGEN_STATIC_FUNC_PTR: i64 = 0x2333;
const BASE_STATIC_FUNC_ID: i32 = 11;
const REGEN_STATIC_FUNC_ID: i32 = 21;

const BASE_GLOBAL_PTR: i64 = 0x1444;
const REGEN_GLOBAL_PTR: i64 = 0x2444;
const NEW_GLOBAL_PTR: i64 = BASE_GLOBAL_PTR; // deliberate base collision

#[derive(Default)]
struct Tables {
    types: Vec<Vec<u8>>,
    type_ids: Vec<Vec<u8>>,
    funcs: Vec<Vec<u8>>,
    func_ids: Vec<Vec<u8>>,
    globals: Vec<Vec<u8>>,
    static_names: Vec<Vec<u8>>,
    properties: Vec<Vec<u8>>,
}

fn sia(s: &str) -> Vec<u8> {
    if s.is_empty() {
        return 0i32.to_le_bytes().to_vec();
    }
    let mut out = (s.len() as i32).to_le_bytes().to_vec();
    out.extend_from_slice(s.as_bytes());
    out.push(0);
    out
}

fn fstring(s: &str) -> Vec<u8> {
    let mut out = ((s.len() + 1) as i32).to_le_bytes().to_vec();
    out.extend_from_slice(s.as_bytes());
    out.push(0);
    out
}

fn datatype(type_ptr: i64, token: i32) -> Vec<u8> {
    datatype_flags(type_ptr, token, [false; 6])
}

fn datatype_flags(type_ptr: i64, token: i32, flags: [bool; 6]) -> Vec<u8> {
    let mut out = Vec::with_capacity(36);
    for flag in flags {
        out.extend_from_slice(&i32::from(flag).to_le_bytes());
    }
    out.extend_from_slice(&type_ptr.to_le_bytes());
    out.extend_from_slice(&token.to_le_bytes());
    out
}

fn type_row(key: i64, name: &str, module: &str, subtypes: &[i64]) -> Vec<u8> {
    type_row_ns(key, name, module, "", subtypes)
}

fn type_row_ns(key: i64, name: &str, module: &str, namespace: &str, subtypes: &[i64]) -> Vec<u8> {
    let datatypes = subtypes
        .iter()
        .map(|&ptr| datatype(ptr, 5))
        .collect::<Vec<_>>();
    type_row_datatypes(key, name, module, namespace, &datatypes)
}

fn type_row_datatypes(
    key: i64,
    name: &str,
    module: &str,
    namespace: &str,
    subtypes: &[Vec<u8>],
) -> Vec<u8> {
    let mut out = key.to_le_bytes().to_vec();
    out.extend_from_slice(&sia(name));
    out.extend_from_slice(&sia(module));
    out.extend_from_slice(&sia(namespace));
    out.extend_from_slice(&(subtypes.len() as i32).to_le_bytes());
    for datatype in subtypes {
        out.extend_from_slice(datatype);
    }
    out
}

fn id_row(id: i32, ptr: i64) -> Vec<u8> {
    let mut out = id.to_le_bytes().to_vec();
    out.extend_from_slice(&ptr.to_le_bytes());
    out
}

fn func_row(key: i64, name: &str, module: &str, owner: i64, params: &[i64], ret: i64) -> Vec<u8> {
    func_row_ns(key, name, module, "", owner, params, ret)
}

fn func_row_ns(
    key: i64,
    name: &str,
    module: &str,
    namespace: &str,
    owner: i64,
    params: &[i64],
    ret: i64,
) -> Vec<u8> {
    func_row_flags(
        key,
        name,
        module,
        namespace,
        false,
        false,
        owner != 0,
        owner,
        params,
        ret,
    )
}

fn func_row_flags(
    key: i64,
    name: &str,
    module: &str,
    namespace: &str,
    is_const: bool,
    is_imported: bool,
    is_method: bool,
    owner: i64,
    params: &[i64],
    ret: i64,
) -> Vec<u8> {
    let params = params
        .iter()
        .map(|&ptr| datatype(ptr, 5))
        .collect::<Vec<_>>();
    let ret = if ret == 0 {
        datatype(0, 0x52)
    } else {
        datatype(ret, 5)
    };
    func_row_datatypes(
        key,
        name,
        module,
        namespace,
        is_const,
        is_imported,
        is_method,
        owner,
        &params,
        &ret,
    )
}

fn func_row_datatypes(
    key: i64,
    name: &str,
    module: &str,
    namespace: &str,
    is_const: bool,
    is_imported: bool,
    is_method: bool,
    owner: i64,
    params: &[Vec<u8>],
    ret: &[u8],
) -> Vec<u8> {
    let mut out = key.to_le_bytes().to_vec();
    out.extend_from_slice(&sia(name));
    out.extend_from_slice(&sia(module));
    out.extend_from_slice(&sia(namespace));
    out.extend_from_slice(&i32::from(is_const).to_le_bytes());
    out.extend_from_slice(&i32::from(is_imported).to_le_bytes());
    out.extend_from_slice(&i32::from(is_method).to_le_bytes());
    out.extend_from_slice(&owner.to_le_bytes());
    out.extend_from_slice(&(params.len() as i32).to_le_bytes());
    for datatype in params {
        out.extend_from_slice(datatype);
    }
    out.extend_from_slice(ret);
    out
}

fn global_row(key: i64, name: &str, module: &str) -> Vec<u8> {
    global_row_ns(key, name, module, "", false)
}

fn global_row_ns(key: i64, name: &str, module: &str, namespace: &str, is_string: bool) -> Vec<u8> {
    let mut out = key.to_le_bytes().to_vec();
    out.extend_from_slice(&sia(name));
    out.extend_from_slice(&sia(module));
    out.extend_from_slice(&sia(namespace));
    out.extend_from_slice(&i32::from(is_string).to_le_bytes());
    out
}

fn property_key(type_id: i32, offset: i32) -> i64 {
    ((type_id as i64) << 1) | ((offset as i64) << 33) | 1
}

fn property_row(type_id: i32, offset: i32, name: &str) -> Vec<u8> {
    let mut out = property_key(type_id, offset).to_le_bytes().to_vec();
    out.extend_from_slice(&sia(name));
    out.extend_from_slice(&type_id.to_le_bytes());
    out
}

const DEFAULT_MODULE_FUNCTION_ID: i32 = 0x1234_5678;

fn function(bytecode: &[i32], id: i32) -> Vec<u8> {
    function_with_signature("Edited", "", &[], &datatype(0, 0x52), false, bytecode, id)
}

fn function_with_signature(
    name: &str,
    namespace: &str,
    parameter_types: &[Vec<u8>],
    return_type: &[u8],
    is_const: bool,
    bytecode: &[i32],
    id: i32,
) -> Vec<u8> {
    let mut out = sia(name);
    out.extend_from_slice(&sia(namespace));
    out.extend_from_slice(return_type);
    out.extend_from_slice(&(parameter_types.len() as i32).to_le_bytes());
    for parameter_type in parameter_types {
        out.extend_from_slice(parameter_type);
    }
    out.extend_from_slice(&(parameter_types.len() as i32).to_le_bytes());
    for index in 0..parameter_types.len() {
        out.extend_from_slice(&sia(&format!("parameter{index}")));
    }
    out.extend_from_slice(&(parameter_types.len() as i32).to_le_bytes());
    for _ in parameter_types {
        out.extend_from_slice(&0i32.to_le_bytes());
    }
    out.extend_from_slice(&(parameter_types.len() as i32).to_le_bytes());
    for _ in parameter_types {
        out.extend_from_slice(&sia(""));
    }
    out.extend_from_slice(&(if is_const { 4i32 } else { 0 }).to_le_bytes());
    out.extend_from_slice(&(bytecode.len() as i32).to_le_bytes());
    for &dw in bytecode {
        out.extend_from_slice(&dw.to_le_bytes());
    }
    out.extend_from_slice(&0i32.to_le_bytes()); // bytecode refs
    out.extend_from_slice(&0i32.to_le_bytes()); // variable space
    out.extend_from_slice(&0i32.to_le_bytes()); // object variable types
    out.extend_from_slice(&0i32.to_le_bytes()); // object variable positions
    out.extend_from_slice(&0i32.to_le_bytes()); // object vars on heap
    out.extend_from_slice(&0i32.to_le_bytes()); // var info program pos
    out.extend_from_slice(&0i32.to_le_bytes()); // var info offset
    out.extend_from_slice(&0i32.to_le_bytes()); // var info option
    out.extend_from_slice(&0i32.to_le_bytes()); // stack needed
    out.extend_from_slice(&id.to_le_bytes()); // runtime Function.Id (not T4)
    out.extend_from_slice(&0i32.to_le_bytes()); // declared at
    out.extend_from_slice(&0i32.to_le_bytes()); // line numbers
    out.extend_from_slice(&0i32.to_le_bytes()); // is UFunction
    out
}

#[derive(Clone, Copy, Default)]
struct FunctionShape {
    parameter_types: usize,
    parameter_names: usize,
    parameter_flags: usize,
    parameter_defaults: usize,
    variable_space: i32,
    object_types: usize,
    object_positions: usize,
    serialized_object_positions: Option<i32>,
    object_position_values: &'static [i32],
    object_heap_mask: i32,
    variable_program_positions: usize,
    serialized_variable_program_positions: Option<i32>,
    variable_program_values: &'static [i32],
    variable_offsets: usize,
    variable_offset_values: &'static [i32],
    variable_options: usize,
    variable_option_values: &'static [i32],
    stack_needed: i32,
    unreal_metadata: Option<(usize, usize)>,
}

fn shaped_function(id: i32, shape: FunctionShape) -> Vec<u8> {
    let mut out = sia("Shaped");
    out.extend_from_slice(&sia(""));
    out.extend_from_slice(&datatype(0, 0x52));
    out.extend_from_slice(&(shape.parameter_types as i32).to_le_bytes());
    for _ in 0..shape.parameter_types {
        out.extend_from_slice(&datatype(0, 0x52));
    }
    out.extend_from_slice(&(shape.parameter_names as i32).to_le_bytes());
    for _ in 0..shape.parameter_names {
        out.extend_from_slice(&sia("parameter"));
    }
    out.extend_from_slice(&(shape.parameter_flags as i32).to_le_bytes());
    for _ in 0..shape.parameter_flags {
        out.extend_from_slice(&0i32.to_le_bytes());
    }
    out.extend_from_slice(&(shape.parameter_defaults as i32).to_le_bytes());
    for _ in 0..shape.parameter_defaults {
        out.extend_from_slice(&sia(""));
    }
    out.extend_from_slice(&0i32.to_le_bytes()); // traits
    out.extend_from_slice(&1i32.to_le_bytes()); // bytecode
    out.extend_from_slice(&10i32.to_le_bytes()); // RET
    out.extend_from_slice(&0i32.to_le_bytes()); // bytecode refs
    out.extend_from_slice(&shape.variable_space.to_le_bytes());
    out.extend_from_slice(&(shape.object_types as i32).to_le_bytes());
    for _ in 0..shape.object_types {
        out.extend_from_slice(&0i64.to_le_bytes());
    }
    out.extend_from_slice(
        &shape
            .serialized_object_positions
            .unwrap_or(shape.object_positions as i32)
            .to_le_bytes(),
    );
    for index in 0..shape.object_positions {
        out.extend_from_slice(
            &shape
                .object_position_values
                .get(index)
                .copied()
                .unwrap_or(0)
                .to_le_bytes(),
        );
    }
    out.extend_from_slice(&shape.object_heap_mask.to_le_bytes());
    for (count, serialized_count, values) in [
        (
            shape.variable_program_positions,
            shape.serialized_variable_program_positions,
            shape.variable_program_values,
        ),
        (shape.variable_offsets, None, shape.variable_offset_values),
        (shape.variable_options, None, shape.variable_option_values),
    ] {
        out.extend_from_slice(&serialized_count.unwrap_or(count as i32).to_le_bytes());
        for index in 0..count {
            out.extend_from_slice(&values.get(index).copied().unwrap_or(0).to_le_bytes());
        }
    }
    out.extend_from_slice(&shape.stack_needed.to_le_bytes());
    out.extend_from_slice(&id.to_le_bytes());
    out.extend_from_slice(&0i32.to_le_bytes()); // declared at
    out.extend_from_slice(&0i32.to_le_bytes()); // line numbers
    out.extend_from_slice(&i32::from(shape.unreal_metadata.is_some()).to_le_bytes());
    if let Some((specs, values)) = shape.unreal_metadata {
        out.extend_from_slice(&sia("Shaped"));
        out.extend_from_slice(&(specs as i32).to_le_bytes());
        for _ in 0..specs {
            out.extend_from_slice(&sia("Spec"));
        }
        out.extend_from_slice(&(values as i32).to_le_bytes());
        for _ in 0..values {
            out.extend_from_slice(&sia("Value"));
        }
        out.extend_from_slice(&[0u8; 18 * 4]);
    }
    out
}

fn property_with_metadata(specs: usize, values: usize) -> Vec<u8> {
    let mut out = sia("Property");
    out.extend_from_slice(&datatype(0, 0x52));
    out.extend_from_slice(&0i32.to_le_bytes()); // private
    out.extend_from_slice(&0i32.to_le_bytes()); // protected
    out.extend_from_slice(&1i32.to_le_bytes()); // has Unreal property data
    out.extend_from_slice(&(specs as i32).to_le_bytes());
    for _ in 0..specs {
        out.extend_from_slice(&sia("Spec"));
    }
    out.extend_from_slice(&(values as i32).to_le_bytes());
    for _ in 0..values {
        out.extend_from_slice(&sia("Value"));
    }
    out.extend_from_slice(&[0u8; 9 * 4]);
    out.extend_from_slice(&0i32.to_le_bytes()); // replicated
    out.extend_from_slice(&[0u8; 6 * 4]);
    out
}

fn property_record_named(name: &str) -> Vec<u8> {
    let mut out = sia(name);
    out.extend_from_slice(&datatype(0, 0x52));
    out.extend_from_slice(&0i32.to_le_bytes()); // private
    out.extend_from_slice(&0i32.to_le_bytes()); // protected
    out.extend_from_slice(&0i32.to_le_bytes()); // no Unreal property data
    out
}

fn structural_class_record(
    properties: &[Vec<u8>],
    methods: &[Vec<u8>],
    method_table: &[i32],
    behavior_refs: &[i64],
    behavior_functions: &[Vec<u8>],
    behavior_types: &[i32],
    metadata: Option<(usize, usize)>,
) -> Vec<u8> {
    structural_class_record_full(
        properties,
        methods,
        method_table,
        &[],
        &[],
        behavior_refs,
        behavior_functions,
        behavior_types,
        metadata,
    )
}

fn structural_class_record_full(
    properties: &[Vec<u8>],
    methods: &[Vec<u8>],
    method_table: &[i32],
    constructors: &[Vec<u8>],
    factory_refs: &[i64],
    behavior_refs: &[i64],
    behavior_functions: &[Vec<u8>],
    behavior_types: &[i32],
    metadata: Option<(usize, usize)>,
) -> Vec<u8> {
    structural_class_record_full_named(
        "StructuralClass",
        "",
        properties,
        methods,
        method_table,
        constructors,
        factory_refs,
        behavior_refs,
        behavior_functions,
        behavior_types,
        metadata,
    )
}

fn structural_class_record_full_named(
    name: &str,
    namespace: &str,
    properties: &[Vec<u8>],
    methods: &[Vec<u8>],
    method_table: &[i32],
    constructors: &[Vec<u8>],
    factory_refs: &[i64],
    behavior_refs: &[i64],
    behavior_functions: &[Vec<u8>],
    behavior_types: &[i32],
    metadata: Option<(usize, usize)>,
) -> Vec<u8> {
    let mut out = sia(name);
    out.extend_from_slice(&sia(namespace));
    out.extend_from_slice(&0i32.to_le_bytes()); // flags
    out.extend_from_slice(&(properties.len() as i32).to_le_bytes());
    for property in properties {
        out.extend_from_slice(property);
    }
    out.extend_from_slice(&(methods.len() as i32).to_le_bytes());
    for method in methods {
        out.extend_from_slice(method);
    }
    out.extend_from_slice(&(method_table.len() as i32).to_le_bytes());
    for index in method_table {
        out.extend_from_slice(&index.to_le_bytes());
    }
    out.extend_from_slice(&0i64.to_le_bytes()); // derived from
    out.extend_from_slice(&0i64.to_le_bytes()); // shadow type
    out.extend_from_slice(&(constructors.len() as i32).to_le_bytes());
    for constructor in constructors {
        out.extend_from_slice(constructor);
    }
    out.extend_from_slice(&(factory_refs.len() as i32).to_le_bytes());
    for reference in factory_refs {
        out.extend_from_slice(&reference.to_le_bytes());
    }
    out.extend_from_slice(&(behavior_refs.len() as i32).to_le_bytes());
    for reference in behavior_refs {
        out.extend_from_slice(&reference.to_le_bytes());
    }
    out.extend_from_slice(&(behavior_functions.len() as i32).to_le_bytes());
    for function in behavior_functions {
        out.extend_from_slice(function);
    }
    out.extend_from_slice(&(behavior_types.len() as i32).to_le_bytes());
    for behavior_type in behavior_types {
        out.extend_from_slice(&behavior_type.to_le_bytes());
    }
    out.extend_from_slice(&i32::from(metadata.is_some()).to_le_bytes());
    if let Some((specs, values)) = metadata {
        out.extend_from_slice(&sia("Super"));
        out.extend_from_slice(&sia("CodeSuper"));
        out.extend_from_slice(&[0u8; 8 * 4]);
        out.extend_from_slice(&sia("StaticClass"));
        out.extend_from_slice(&0i32.to_le_bytes()); // placeable
        out.extend_from_slice(&(specs as i32).to_le_bytes());
        for _ in 0..specs {
            out.extend_from_slice(&sia("Spec"));
        }
        out.extend_from_slice(&(values as i32).to_le_bytes());
        for _ in 0..values {
            out.extend_from_slice(&sia("Value"));
        }
        out.extend_from_slice(&sia("")); // compose onto
    }
    out
}

fn enum_record(names: usize, values: usize) -> Vec<u8> {
    enum_record_named("StructuralEnum", "", names, values)
}

fn enum_record_named(name: &str, namespace: &str, names: usize, values: usize) -> Vec<u8> {
    let mut out = sia(name);
    out.extend_from_slice(&sia(namespace));
    out.extend_from_slice(&(names as i32).to_le_bytes());
    for _ in 0..names {
        out.extend_from_slice(&sia("Entry"));
    }
    out.extend_from_slice(&(values as i32).to_le_bytes());
    for _ in 0..values {
        out.extend_from_slice(&0i32.to_le_bytes());
    }
    out
}

fn import_record(parameter_types: usize, flags: usize, defaults: usize) -> Vec<u8> {
    import_record_named(
        "ImportedModule",
        "ImportedFunction",
        "",
        parameter_types,
        flags,
        defaults,
    )
}

fn import_record_named(
    imported_from_module: &str,
    name: &str,
    namespace: &str,
    parameter_types: usize,
    flags: usize,
    defaults: usize,
) -> Vec<u8> {
    let mut out = sia(imported_from_module);
    out.extend_from_slice(&sia(name));
    out.extend_from_slice(&sia(namespace));
    out.extend_from_slice(&(parameter_types as i32).to_le_bytes());
    for _ in 0..parameter_types {
        out.extend_from_slice(&datatype(0, 0x52));
    }
    out.extend_from_slice(&(flags as i32).to_le_bytes());
    for _ in 0..flags {
        out.extend_from_slice(&0i32.to_le_bytes());
    }
    out.extend_from_slice(&(defaults as i32).to_le_bytes());
    for _ in 0..defaults {
        out.extend_from_slice(&sia(""));
    }
    out.extend_from_slice(&datatype(0, 0x52));
    out
}

fn global_init_record(init_function: &[u8]) -> Vec<u8> {
    let mut out = sia("InitializedGlobal");
    out.extend_from_slice(&sia(""));
    out.extend_from_slice(&datatype(0, 0x52));
    out.extend_from_slice(&0i32.to_le_bytes()); // not default-init
    out.extend_from_slice(&0i32.to_le_bytes()); // not a pure constant
    out.extend_from_slice(&1i32.to_le_bytes()); // has init function
    out.extend_from_slice(init_function);
    out
}

fn global_record_named(name: &str, namespace: &str) -> Vec<u8> {
    let mut out = sia(name);
    out.extend_from_slice(&sia(namespace));
    out.extend_from_slice(&datatype(0, 0x52));
    out.extend_from_slice(&1i32.to_le_bytes()); // default-init: no payload follows
    out
}

fn class_record() -> Vec<u8> {
    let mut out = sia("EditedClass");
    out.extend_from_slice(&sia(""));
    out.extend_from_slice(&0i32.to_le_bytes()); // flags
    out.extend_from_slice(&0i32.to_le_bytes()); // properties
    out.extend_from_slice(&0i32.to_le_bytes()); // methods
    out.extend_from_slice(&2i32.to_le_bytes()); // MethodTable: local Methods[] indices
    out.extend_from_slice(&(-1i32).to_le_bytes());
    out.extend_from_slice(&(-1i32).to_le_bytes());
    out.extend_from_slice(&REGEN_TYPE_PTR.to_le_bytes()); // DerivedFrom: existing type
    out.extend_from_slice(&NEW_TYPE_PTR.to_le_bytes()); // ShadowType: new type
    out.extend_from_slice(&0i32.to_le_bytes()); // constructors
    out.extend_from_slice(&2i32.to_le_bytes()); // FactoryRefs (T4 ids)
    out.extend_from_slice(&(REGEN_FUNC_ID as i64).to_le_bytes());
    out.extend_from_slice(&(NEW_FUNC_ID as i64).to_le_bytes());
    out.extend_from_slice(&7i32.to_le_bytes()); // BehaviorRefs (T4 ids)
    out.extend_from_slice(&(REGEN_FUNC_ID as i64).to_le_bytes());
    out.extend_from_slice(&(NEW_FUNC_ID as i64).to_le_bytes());
    for _ in 0..5 {
        out.extend_from_slice(&0i64.to_le_bytes());
    }
    out.extend_from_slice(&0i32.to_le_bytes()); // behavior functions
    out.extend_from_slice(&0i32.to_le_bytes()); // behavior function types
    out.extend_from_slice(&0i32.to_le_bytes()); // has Unreal class data
    out
}

fn class_record_with_embedded_refs(derived_from: i64, factory_ref: i64) -> Vec<u8> {
    let mut out = sia("EditedClass");
    out.extend_from_slice(&sia(""));
    out.extend_from_slice(&0i32.to_le_bytes()); // flags
    out.extend_from_slice(&0i32.to_le_bytes()); // properties
    out.extend_from_slice(&0i32.to_le_bytes()); // methods
    out.extend_from_slice(&0i32.to_le_bytes()); // method table
    out.extend_from_slice(&derived_from.to_le_bytes());
    out.extend_from_slice(&0i64.to_le_bytes()); // shadow type
    out.extend_from_slice(&0i32.to_le_bytes()); // constructors
    out.extend_from_slice(&1i32.to_le_bytes()); // factory refs
    out.extend_from_slice(&factory_ref.to_le_bytes());
    out.extend_from_slice(&7i32.to_le_bytes()); // behavior refs
    for _ in 0..7 {
        out.extend_from_slice(&0i64.to_le_bytes());
    }
    out.extend_from_slice(&0i32.to_le_bytes()); // behavior functions
    out.extend_from_slice(&0i32.to_le_bytes()); // behavior function types
    out.extend_from_slice(&0i32.to_le_bytes()); // has Unreal class data
    out
}

fn module_value_with_records(
    functions: &[Vec<u8>],
    classes: &[Vec<u8>],
    enums: &[Vec<u8>],
    globals: &[Vec<u8>],
    imports: &[Vec<u8>],
) -> Vec<u8> {
    module_value_with_name_and_records(MODULE, functions, classes, enums, globals, imports)
}

fn module_value_with_name_and_records(
    module_name: &str,
    functions: &[Vec<u8>],
    classes: &[Vec<u8>],
    enums: &[Vec<u8>],
    globals: &[Vec<u8>],
    imports: &[Vec<u8>],
) -> Vec<u8> {
    let mut out = sia(module_name);
    for records in [functions, classes, enums, globals, imports] {
        out.extend_from_slice(&(records.len() as i32).to_le_bytes());
        for record in records {
            out.extend_from_slice(record);
        }
    }
    out.extend_from_slice(&0i64.to_le_bytes()); // code hash
    out.extend_from_slice(&0i32.to_le_bytes()); // imported modules
    out.extend_from_slice(&sia("")); // statics class
    out.extend_from_slice(&0i32.to_le_bytes()); // events
    out.extend_from_slice(&0i32.to_le_bytes()); // delegates
    out.extend_from_slice(&sia("EditedModule.as"));
    out.extend_from_slice(&0i32.to_le_bytes()); // post-init functions
    out
}

fn append_table(out: &mut Vec<u8>, rows: &[Vec<u8>]) {
    out.extend_from_slice(&(rows.len() as u32).to_le_bytes());
    for row in rows {
        out.extend_from_slice(row);
    }
}

fn test_sia_at(bytes: &[u8], mut pos: usize) -> Option<(String, usize)> {
    let len = i32::from_le_bytes(bytes.get(pos..pos + 4)?.try_into().ok()?);
    pos += 4;
    if len == 0 {
        return Some((String::new(), pos));
    }
    let len = usize::try_from(len).ok()?;
    let raw = bytes.get(pos..pos.checked_add(len + 1)?)?;
    if raw.last() != Some(&0) || raw[..len].contains(&0) {
        return None;
    }
    Some((
        String::from_utf8_lossy(&raw[..len]).into_owned(),
        pos + len + 1,
    ))
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct SyntheticT3Signature {
    name: String,
    module: String,
    namespace: String,
    is_const: bool,
    is_imported: bool,
    is_method: bool,
    owner: i64,
    parameter_types: Vec<Vec<u8>>,
    return_type: Vec<u8>,
}

fn test_t3_signature(row: &[u8]) -> Option<SyntheticT3Signature> {
    let (name, pos) = test_sia_at(row, 8)?;
    let (module, pos) = test_sia_at(row, pos)?;
    let (namespace, mut pos) = test_sia_at(row, pos)?;
    let read_bool = |pos: &mut usize| {
        let value = i32::from_le_bytes(row.get(*pos..*pos + 4)?.try_into().ok()?);
        *pos += 4;
        match value {
            0 => Some(false),
            1 => Some(true),
            _ => None,
        }
    };
    let is_const = read_bool(&mut pos)?;
    let is_imported = read_bool(&mut pos)?;
    let is_method = read_bool(&mut pos)?;
    let owner = i64::from_le_bytes(row.get(pos..pos + 8)?.try_into().ok()?);
    pos += 8;
    let parameter_count = i32::from_le_bytes(row.get(pos..pos + 4)?.try_into().ok()?);
    pos += 4;
    let parameter_count = usize::try_from(parameter_count).ok()?;
    let mut parameter_types = Vec::with_capacity(parameter_count);
    for _ in 0..parameter_count {
        let end = pos.checked_add(36)?;
        parameter_types.push(row.get(pos..end)?.to_vec());
        pos = end;
    }
    let end = pos.checked_add(36)?;
    let return_type = row.get(pos..end)?.to_vec();
    (end == row.len()).then_some(SyntheticT3Signature {
        name,
        module,
        namespace,
        is_const,
        is_imported,
        is_method,
        owner,
        parameter_types,
        return_type,
    })
}

fn default_module_function_declares(signature: &SyntheticT3Signature, module_name: &str) -> bool {
    signature.name == "Edited"
        && signature.module == module_name
        && signature.namespace.is_empty()
        && !signature.is_const
        && !signature.is_imported
        && !signature.is_method
        && signature.owner == 0
        && signature.parameter_types.is_empty()
        && signature.return_type == datatype(0, 0x52)
}

#[derive(Default)]
struct SyntheticDeclarations {
    functions: Vec<Vec<u8>>,
    classes: Vec<Vec<u8>>,
    globals: Vec<Vec<u8>>,
    imports: Vec<Vec<u8>>,
}

fn synthetic_property_names_for_owner(tables: &Tables, owner_ptr: i64) -> Vec<String> {
    let owner_ids = tables
        .type_ids
        .iter()
        .filter_map(|row| {
            let id = i32::from_le_bytes(row.get(..4)?.try_into().ok()?);
            let ptr = i64::from_le_bytes(row.get(4..12)?.try_into().ok()?);
            (ptr == owner_ptr).then_some(id)
        })
        .collect::<std::collections::HashSet<_>>();
    let mut names = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for row in &tables.properties {
        let Some((name, pos)) = test_sia_at(row, 8) else {
            continue;
        };
        let Some(old_type_id) = row
            .get(pos..pos + 4)
            .and_then(|bytes| bytes.try_into().ok())
            .map(i32::from_le_bytes)
        else {
            continue;
        };
        if owner_ids.contains(&old_type_id) && seen.insert(name.clone()) {
            names.push(name);
        }
    }
    names
}

fn synthetic_declarations_for_tables(
    module_name: &str,
    tables: &Tables,
    function_id_seed: i32,
) -> SyntheticDeclarations {
    let signatures = tables
        .funcs
        .iter()
        .filter_map(|row| test_t3_signature(row))
        .collect::<Vec<_>>();
    let mut functions = Vec::new();
    let mut methods_by_owner: std::collections::HashMap<i64, Vec<Vec<u8>>> =
        std::collections::HashMap::new();
    let mut seen_functions = std::collections::HashSet::new();
    for signature in &signatures {
        // Imported rows need FunctionImports authority and native/template methods need pristine
        // engine authority. The generic cache fixture must not invent either.
        if signature.is_imported || !seen_functions.insert(signature.clone()) {
            continue;
        }
        let id = function_id_seed
            .wrapping_neg()
            .wrapping_sub(i32::try_from(seen_functions.len()).unwrap());
        let declaration = function_with_signature(
            &signature.name,
            &signature.namespace,
            &signature.parameter_types,
            &signature.return_type,
            signature.is_const,
            &[10],
            id,
        );
        if signature.is_method {
            methods_by_owner
                .entry(signature.owner)
                .or_default()
                .push(declaration);
        } else if signature.module == module_name
            && !default_module_function_declares(signature, module_name)
        {
            functions.push(declaration);
        }
    }

    let mut classes = Vec::new();
    let mut seen_types = std::collections::HashSet::new();
    for row in &tables.types {
        let Some(key) = row
            .get(..8)
            .and_then(|bytes| bytes.try_into().ok())
            .map(i64::from_le_bytes)
        else {
            continue;
        };
        let Some((name, pos)) = test_sia_at(row, 8) else {
            continue;
        };
        let Some((module, pos)) = test_sia_at(row, pos) else {
            continue;
        };
        let Some((namespace, pos)) = test_sia_at(row, pos) else {
            continue;
        };
        let Some(subtypes) = row
            .get(pos..pos + 4)
            .and_then(|bytes| bytes.try_into().ok())
            .map(i32::from_le_bytes)
        else {
            continue;
        };
        if subtypes == 0
            && module == module_name
            && seen_types.insert((name.clone(), namespace.clone()))
        {
            let methods = methods_by_owner.remove(&key).unwrap_or_default();
            let method_table = (0..methods.len())
                .map(|index| i32::try_from(index).unwrap())
                .collect::<Vec<_>>();
            let properties = synthetic_property_names_for_owner(tables, key)
                .into_iter()
                .map(|name| property_record_named(&name))
                .collect::<Vec<_>>();
            classes.push(structural_class_record_full_named(
                &name,
                &namespace,
                &properties,
                &methods,
                &method_table,
                &[],
                &[],
                &[0; 7],
                &[],
                &[],
                None,
            ));
        }
    }

    let mut globals = Vec::new();
    let mut seen_globals = std::collections::HashSet::new();
    for row in &tables.globals {
        let Some((name, pos)) = test_sia_at(row, 8) else {
            continue;
        };
        let Some((module, pos)) = test_sia_at(row, pos) else {
            continue;
        };
        let Some((namespace, pos)) = test_sia_at(row, pos) else {
            continue;
        };
        let Some(is_string) = row
            .get(pos..pos + 4)
            .and_then(|bytes| bytes.try_into().ok())
            .map(i32::from_le_bytes)
        else {
            continue;
        };
        if is_string == 0
            && module == module_name
            && seen_globals.insert((name.clone(), namespace.clone()))
        {
            globals.push(global_record_named(&name, &namespace));
        }
    }
    SyntheticDeclarations {
        functions,
        classes,
        globals,
        imports: Vec::new(),
    }
}

fn cache_from_module_value(module: &[u8], tables: Tables) -> Vec<u8> {
    cache_from_named_module_value(MODULE, module, tables)
}

fn cache_from_named_module_value(module_name: &str, module: &[u8], tables: Tables) -> Vec<u8> {
    let mut out = vec![0u8; 16];
    out.extend_from_slice(&CACHE_MAGIC.to_le_bytes());
    out.extend_from_slice(&1u32.to_le_bytes());
    out.extend_from_slice(&fstring(module_name));
    out.extend_from_slice(module);
    append_table(&mut out, &tables.types);
    append_table(&mut out, &tables.type_ids);
    append_table(&mut out, &tables.funcs);
    append_table(&mut out, &tables.func_ids);
    append_table(&mut out, &tables.globals);
    append_table(&mut out, &tables.static_names);
    append_table(&mut out, &tables.properties);
    out
}

fn cache_with_class_and_function_id(
    bytecode: &[i32],
    tables: Tables,
    class: Option<&[u8]>,
    function_id: i32,
) -> Vec<u8> {
    let mut declarations = synthetic_declarations_for_tables(MODULE, &tables, function_id);
    if let Some(class) = class {
        declarations.classes.push(class.to_vec());
    }
    let mut functions = vec![function(bytecode, function_id)];
    functions.extend(declarations.functions);
    let module = module_value_with_name_and_records(
        MODULE,
        &functions,
        &declarations.classes,
        &[],
        &declarations.globals,
        &declarations.imports,
    );
    cache_from_module_value(&module, tables)
}

fn cache_with_class(bytecode: &[i32], tables: Tables, class: Option<&[u8]>) -> Vec<u8> {
    cache_with_class_and_function_id(bytecode, tables, class, DEFAULT_MODULE_FUNCTION_ID)
}

fn cache_with_function_id(bytecode: &[i32], tables: Tables, function_id: i32) -> Vec<u8> {
    cache_with_class_and_function_id(bytecode, tables, None, function_id)
}

fn cache_with_records(
    functions: &[Vec<u8>],
    classes: &[Vec<u8>],
    enums: &[Vec<u8>],
    imports: &[Vec<u8>],
) -> Vec<u8> {
    cache_with_all_records(functions, classes, enums, &[], imports)
}

fn cache_with_all_records(
    functions: &[Vec<u8>],
    classes: &[Vec<u8>],
    enums: &[Vec<u8>],
    globals: &[Vec<u8>],
    imports: &[Vec<u8>],
) -> Vec<u8> {
    cache_from_module_value(
        &module_value_with_records(functions, classes, enums, globals, imports),
        Tables::default(),
    )
}

fn cache(bytecode: &[i32], tables: Tables) -> Vec<u8> {
    cache_with_class(bytecode, tables, None)
}

fn cache_with_named_module(module_name: &str, bytecode: &[i32], tables: Tables) -> Vec<u8> {
    cache_with_module_key_name_and_function_id(
        module_name,
        module_name,
        bytecode,
        tables,
        DEFAULT_MODULE_FUNCTION_ID,
    )
}

fn cache_with_module_key_and_name(
    module_key: &str,
    module_name: &str,
    bytecode: &[i32],
    tables: Tables,
) -> Vec<u8> {
    cache_with_module_key_name_and_function_id(
        module_key,
        module_name,
        bytecode,
        tables,
        DEFAULT_MODULE_FUNCTION_ID,
    )
}

fn cache_with_module_key_name_and_function_id(
    module_key: &str,
    module_name: &str,
    bytecode: &[i32],
    tables: Tables,
    function_id: i32,
) -> Vec<u8> {
    let declarations = synthetic_declarations_for_tables(module_name, &tables, function_id);
    let mut functions = vec![function(bytecode, function_id)];
    functions.extend(declarations.functions);
    let module = module_value_with_name_and_records(
        module_name,
        &functions,
        &declarations.classes,
        &[],
        &declarations.globals,
        &declarations.imports,
    );
    cache_from_named_module_value(module_key, &module, tables)
}

fn cache_with_module_key_name_and_records(
    module_key: &str,
    module_name: &str,
    functions: &[Vec<u8>],
    classes: &[Vec<u8>],
    enums: &[Vec<u8>],
    globals: &[Vec<u8>],
    tables: Tables,
) -> Vec<u8> {
    let module =
        module_value_with_name_and_records(module_name, functions, classes, enums, globals, &[]);
    cache_from_named_module_value(module_key, &module, tables)
}

fn qw_op(opcode: i32, value: i64, out: &mut Vec<i32>) {
    out.push(opcode);
    out.push(value as u64 as u32 as i32);
    out.push(((value as u64) >> 32) as u32 as i32);
}

fn base_cache_with_function_id(function_id: i32) -> Vec<u8> {
    let tables = Tables {
        types: vec![type_row(BASE_TYPE_PTR, "ExistingType", MODULE, &[])],
        type_ids: vec![id_row(BASE_TYPE_ID, BASE_TYPE_PTR)],
        funcs: vec![
            func_row(BASE_FUNC_PTR, "ExistingFn", MODULE, BASE_TYPE_PTR, &[], 0),
            func_row(BASE_STATIC_FUNC_PTR, "__STATIC_NAME", "", 0, &[], 0),
        ],
        func_ids: vec![
            id_row(BASE_FUNC_ID, BASE_FUNC_PTR),
            id_row(BASE_STATIC_FUNC_ID, BASE_STATIC_FUNC_PTR),
        ],
        globals: vec![global_row(BASE_GLOBAL_PTR, "ExistingGlobal", MODULE)],
        static_names: vec![sia("VanillaName"), sia("SharedName")],
        properties: vec![property_row(BASE_TYPE_ID, 4, "ExistingField")],
    };
    cache_with_function_id(&[10], tables, function_id) // RET
}

fn base_cache() -> Vec<u8> {
    base_cache_with_function_id(DEFAULT_MODULE_FUNCTION_ID)
}

fn regen_tables(existing_property: &str) -> Tables {
    Tables {
        types: vec![
            type_row(REGEN_TYPE_PTR, "ExistingType", MODULE, &[]),
            type_row(NEW_TYPE_PTR, "NewType", MODULE, &[]),
        ],
        type_ids: vec![
            id_row(REGEN_TYPE_ID, REGEN_TYPE_PTR),
            id_row(NEW_TYPE_ID, NEW_TYPE_PTR),
        ],
        funcs: vec![
            func_row(REGEN_FUNC_PTR, "ExistingFn", MODULE, REGEN_TYPE_PTR, &[], 0),
            func_row(REGEN_STATIC_FUNC_PTR, "__STATIC_NAME", "", 0, &[], 0),
            // NewFn's owner/return are NewType; its param is ExistingType.
            func_row(
                NEW_FUNC_PTR,
                "NewFn",
                MODULE,
                NEW_TYPE_PTR,
                &[REGEN_TYPE_PTR],
                NEW_TYPE_PTR,
            ),
        ],
        func_ids: vec![
            id_row(REGEN_FUNC_ID, REGEN_FUNC_PTR),
            id_row(REGEN_STATIC_FUNC_ID, REGEN_STATIC_FUNC_PTR),
            id_row(NEW_FUNC_ID, NEW_FUNC_PTR),
        ],
        globals: vec![
            global_row(REGEN_GLOBAL_PTR, "ExistingGlobal", MODULE),
            global_row(NEW_GLOBAL_PTR, "NewGlobal", MODULE),
        ],
        static_names: vec![sia("SharedName"), sia("BrandNew")],
        properties: vec![
            property_row(REGEN_TYPE_ID, 4, existing_property),
            property_row(NEW_TYPE_ID, 8, "NewField"),
        ],
    }
}

fn regen_cache_with_existing_property_and_function_id(
    existing_property: &str,
    function_id: i32,
) -> Vec<u8> {
    let mut code = Vec::new();
    qw_op(75, NEW_TYPE_PTR, &mut code); // OBJTYPE new type ptr
    code.extend([76, NEW_TYPE_ID | 0x6000_0000]); // flagged TYPEID for new type
    code.extend([76, REGEN_TYPE_ID | 0x4000_0000]); // flagged TYPEID for existing type
    code.extend([79 | (8 << 16), NEW_TYPE_ID]); // ADDSi offset=8, core new type id
    qw_op(61, REGEN_FUNC_PTR, &mut code); // existing CALLSYS -> vanilla key
    qw_op(61, NEW_FUNC_PTR, &mut code); // new CALLSYS -> re-keyed key
    code.extend([9, NEW_FUNC_ID]); // CALL new function id -> re-keyed id
    qw_op(1, NEW_GLOBAL_PTR, &mut code); // PshGPtr new global -> re-keyed key
    code.push(60); // STR regen StaticNames[0] = SharedName -> base index 1
    code.extend([2, 1]); // PshC4 regen StaticNames[1] = BrandNew -> appended index 2
    qw_op(61, REGEN_STATIC_FUNC_PTR, &mut code); // __STATIC_NAME
    code.push(10); // RET
    let class = class_record();
    cache_with_class_and_function_id(
        &code,
        regen_tables(existing_property),
        Some(&class),
        function_id,
    )
}

fn regen_cache_with_existing_property(existing_property: &str) -> Vec<u8> {
    regen_cache_with_existing_property_and_function_id(
        existing_property,
        DEFAULT_MODULE_FUNCTION_ID,
    )
}

fn regen_cache() -> Vec<u8> {
    regen_cache_with_existing_property("ExistingField")
}

fn regen_cache_with_function_id(function_id: i32) -> Vec<u8> {
    regen_cache_with_existing_property_and_function_id("ExistingField", function_id)
}

fn embedded_function_id_caches(base_id: i32, regen_id: i32, embedded: i64) -> (Vec<u8>, Vec<u8>) {
    const FUNCTION_MODULE: &str = "SharedFunctionModule";
    let base = cache(
        &[10],
        Tables {
            funcs: vec![func_row(
                BASE_FUNC_PTR,
                "EmbeddedFn",
                FUNCTION_MODULE,
                0,
                &[],
                0,
            )],
            func_ids: vec![id_row(base_id, BASE_FUNC_PTR)],
            ..Tables::default()
        },
    );
    let class = class_record_with_embedded_refs(0, embedded);
    let regen = cache_with_class(
        &[10],
        Tables {
            funcs: vec![func_row(
                REGEN_FUNC_PTR,
                "EmbeddedFn",
                FUNCTION_MODULE,
                0,
                &[],
                0,
            )],
            func_ids: vec![id_row(regen_id, REGEN_FUNC_PTR)],
            ..Tables::default()
        },
        Some(&class),
    );
    (base, regen)
}

fn duplicate_tail_key_tables(table: usize, conflicting: bool) -> Tables {
    const TYPE_PTR: i64 = 0x7c00;
    const OTHER_PTR: i64 = 0x7c08;
    const TYPE_ID: i32 = 0x0800_7c00;
    const FUNC_PTR: i64 = 0x7d00;
    const OTHER_FUNC_PTR: i64 = 0x7d08;
    const FUNC_ID: i32 = 0x0001_7d00;
    const GLOBAL_PTR: i64 = 0x7e00;
    match table {
        0 => {
            let first = type_row(TYPE_PTR, "DuplicateType", MODULE, &[]);
            Tables {
                types: vec![
                    first.clone(),
                    if conflicting {
                        type_row(TYPE_PTR, "ConflictingType", MODULE, &[])
                    } else {
                        first
                    },
                ],
                type_ids: vec![id_row(TYPE_ID, TYPE_PTR)],
                ..Tables::default()
            }
        }
        1 => Tables {
            types: vec![type_row(TYPE_PTR, "DuplicateTypeId", MODULE, &[])],
            type_ids: vec![
                id_row(TYPE_ID, TYPE_PTR),
                id_row(TYPE_ID, if conflicting { OTHER_PTR } else { TYPE_PTR }),
            ],
            ..Tables::default()
        },
        2 => {
            let first = func_row(FUNC_PTR, "Edited", MODULE, 0, &[], 0);
            Tables {
                funcs: vec![
                    first.clone(),
                    if conflicting {
                        func_row(FUNC_PTR, "ConflictingFunction", MODULE, 0, &[], 0)
                    } else {
                        first
                    },
                ],
                func_ids: vec![id_row(FUNC_ID, FUNC_PTR)],
                ..Tables::default()
            }
        }
        3 => Tables {
            funcs: vec![func_row(FUNC_PTR, "Edited", MODULE, 0, &[], 0)],
            func_ids: vec![
                id_row(FUNC_ID, FUNC_PTR),
                id_row(
                    FUNC_ID,
                    if conflicting {
                        OTHER_FUNC_PTR
                    } else {
                        FUNC_PTR
                    },
                ),
            ],
            ..Tables::default()
        },
        4 => {
            let first = global_row(GLOBAL_PTR, "DuplicateGlobal", MODULE);
            Tables {
                globals: vec![
                    first.clone(),
                    if conflicting {
                        global_row(GLOBAL_PTR, "ConflictingGlobal", MODULE)
                    } else {
                        first
                    },
                ],
                ..Tables::default()
            }
        }
        _ => unreachable!(),
    }
}

fn contains_single_factory_ref(cache: &[u8], id: i64) -> bool {
    let mut needle = 1i32.to_le_bytes().to_vec();
    needle.extend_from_slice(&id.to_le_bytes());
    needle.extend_from_slice(&7i32.to_le_bytes());
    let module_end = module_region_end(cache).unwrap();
    cache[..module_end]
        .windows(needle.len())
        .any(|window| window == needle)
}

fn replace_ascii_same_len(bytes: &mut [u8], from: &str, to: &str) {
    assert_eq!(from.len(), to.len());
    let mut replaced = 0;
    let mut offset = 0;
    while let Some(rel) = bytes[offset..]
        .windows(from.len())
        .position(|window| window == from.as_bytes())
    {
        let start = offset + rel;
        bytes[start..start + from.len()].copy_from_slice(to.as_bytes());
        offset = start + from.len();
        replaced += 1;
    }
    assert!(replaced > 0, "fixture did not contain {from:?}");
}

fn regen_existing_flagged_typeid_cache() -> Vec<u8> {
    cache(
        &[76, REGEN_TYPE_ID | 0x6000_0000, 10],
        Tables {
            types: vec![type_row(REGEN_TYPE_PTR, "ExistingType", MODULE, &[])],
            type_ids: vec![id_row(REGEN_TYPE_ID, REGEN_TYPE_PTR)],
            ..Tables::default()
        },
    )
}

fn keyed_mini(table: usize, key: i64, label: &str) -> Vec<u8> {
    keyed_mini_with_value_delta(table, key, label, 0)
}

fn keyed_mini_with_value_delta(table: usize, key: i64, label: &str, delta: i64) -> Vec<u8> {
    let mut tables = Tables::default();
    match table {
        0 => {
            tables.types.push(type_row(key, label, MODULE, &[]));
            tables.type_ids.push(id_row((key as i32).max(12), key));
        }
        1 => {
            let dependency = key + 0x1000 + delta;
            tables
                .types
                .push(type_row(dependency, "TypeIdDependency", MODULE, &[]));
            tables.type_ids.push(id_row(key as i32, dependency));
        }
        2 => {
            tables.funcs.push(func_row(key, label, MODULE, 0, &[], 0));
            tables.func_ids.push(id_row((key as i32).max(1), key));
        }
        3 => {
            let dependency = key + 0x2000 + delta;
            tables
                .funcs
                .push(func_row(dependency, "FuncIdDependency", MODULE, 0, &[], 0));
            tables.func_ids.push(id_row(key as i32, dependency));
        }
        4 => tables.globals.push(global_row(key, label, MODULE)),
        6 => {
            let dependency = key + 0x3000;
            tables
                .types
                .push(type_row(dependency, "PropertyOwner", MODULE, &[]));
            tables.type_ids.push(id_row(key as i32, dependency));
            tables.properties.push(property_row(key as i32, 4, label));
        }
        _ => unreachable!(),
    }
    cache(&[10], tables)
}

fn cache_with_function_return_datatype(type_ptr: i64, token: i32) -> Vec<u8> {
    cache_with_function_return_datatype_flags(type_ptr, token, [false; 6])
}

fn cache_with_function_return_datatype_flags(
    type_ptr: i64,
    token: i32,
    flags: [bool; 6],
) -> Vec<u8> {
    let mut mini = cache(&[10], Tables::default());
    let original = datatype(0, 0x52);
    let replacement = datatype_flags(type_ptr, token, flags);
    let start = mini
        .windows(original.len())
        .position(|window| window == original.as_slice())
        .expect("minimal function return DataType");
    mini[start..start + replacement.len()].copy_from_slice(&replacement);
    mini
}

fn assert_guard_rejects_reference(base: &[u8], mini: &[u8], case: &str) {
    let mut guard = SequentialMiniGuard::new(base).unwrap();
    let error = match guard.check_and_record(mini) {
        Ok(_) => panic!("{case}: invalid reference unexpectedly passed"),
        Err(error) => error,
    };
    assert!(
        matches!(error, SpliceError::MiniReference(_)),
        "{case}: unexpected error: {error:?}"
    );
}

fn assert_guard_and_allow_new_reject_declaration(
    base: &[u8],
    mini: &[u8],
    table: usize,
    row_key: i64,
    case: &str,
) {
    let guard_error = match SequentialMiniGuard::new(base)
        .unwrap()
        .check_and_record(mini)
    {
        Ok(_) => panic!("{case}: guard accepted a tail row without a declaration"),
        Err(error) => error,
    };
    assert!(
        matches!(
            guard_error,
            SpliceError::MiniReference(RemapError::InvalidTailRow {
                table: actual_table,
                row_key: actual_key,
                kind,
                ..
            }) if actual_table == table
                && actual_key == row_key
                && kind == "declaration membership"
        ),
        "{case}: unexpected guard error: {guard_error:?}"
    );

    let remap_error = match remap_module_to_base_with_options(
        mini,
        base,
        RemapOptions {
            allow_new_symbols: true,
        },
    ) {
        Ok(_) => panic!("{case}: allow-new accepted a tail row without a declaration"),
        Err(error) => error,
    };
    assert!(
        matches!(
            remap_error,
            RemapError::InvalidTailRow {
                table: actual_table,
                row_key: actual_key,
                kind,
                ..
            } if actual_table == table
                && actual_key == row_key
                && kind == "declaration membership"
        ),
        "{case}: unexpected allow-new error: {remap_error:?}"
    );
}

fn type_symbol_mini(ptr: i64, id: i32, name: &str, subtypes: &[i64]) -> Vec<u8> {
    cache(
        &[10],
        Tables {
            types: vec![type_row(ptr, name, MODULE, subtypes)],
            type_ids: vec![id_row(id, ptr)],
            ..Tables::default()
        },
    )
}

fn function_symbol_mini(ptr: i64, id: i32, name: &str, owner: i64, params: &[i64]) -> Vec<u8> {
    cache(
        &[10],
        Tables {
            funcs: vec![func_row(ptr, name, MODULE, owner, params, 0)],
            func_ids: vec![id_row(id, ptr)],
            ..Tables::default()
        },
    )
}

#[test]
fn sequential_guard_rejects_a_mini_from_another_cache_guid() {
    let mut base = base_cache();
    base[..16].copy_from_slice(&[0x11; 16]);
    let mut mini = cache(&[10], Tables::default());
    mini[..16].copy_from_slice(&[0x22; 16]);

    let mut guard = SequentialMiniGuard::new(&base).unwrap();
    let error = guard.check_and_record(&mini).unwrap_err();
    match error {
        SpliceError::MiniGuidMismatch {
            base: actual_base,
            mini: actual_mini,
        } => {
            assert_eq!(actual_base, [0x11; 16]);
            assert_eq!(actual_mini, [0x22; 16]);
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn sequential_guard_rejects_stale_callsys_even_when_guid_matches() {
    let base = base_cache();
    let stale_key = 0x7fff_1234_5678i64;
    let mut code = Vec::new();
    qw_op(61, stale_key, &mut code);
    code.push(10);
    let mini = cache(&code, Tables::default());

    let mut guard = SequentialMiniGuard::new(&base).unwrap();
    let error = guard.check_and_record(&mini).unwrap_err();
    assert!(matches!(
        error,
        SpliceError::MiniReference(RemapError::UnresolvedEffectiveReference {
            kind: "function pointer",
            op: "CALLSYS",
            key
        }) if key == stale_key
    ));
}

#[test]
fn sequential_guard_rejects_an_unresolved_tail_row_dependency() {
    let base = base_cache();
    let stale_type_ptr = 0x7fff_3456_789ai64;
    let mini = cache(
        &[10],
        Tables {
            type_ids: vec![id_row(0x0800_6abc, stale_type_ptr)],
            ..Tables::default()
        },
    );

    let mut guard = SequentialMiniGuard::new(&base).unwrap();
    let error = guard.check_and_record(&mini).unwrap_err();
    assert!(
        matches!(
            error,
            SpliceError::MiniReference(RemapError::UnresolvedTailDependency {
                table: 1,
                kind: "type pointer",
                dependency,
                ..
            }) if dependency == stale_type_ptr
        ),
        "unexpected error: {error:?}"
    );
}

#[test]
fn sequential_guard_rejects_a_null_type_pointer_in_t2() {
    let base = base_cache();
    let object_id = 0x0800_4321;
    let mini = cache(
        &[76, object_id, 10],
        Tables {
            type_ids: vec![id_row(object_id, 0)],
            ..Tables::default()
        },
    );

    let mut guard = SequentialMiniGuard::new(&base).unwrap();
    let error = guard.check_and_record(&mini).unwrap_err();
    assert!(matches!(
        error,
        SpliceError::MiniReference(RemapError::UnresolvedTailDependency {
            table: 1,
            dependency: 0,
            ..
        })
    ));
}

#[test]
fn sequential_guard_rejects_a_large_unknown_embedded_function_id() {
    let base = base_cache();
    let stale_id = 0x1234_5678i64;
    let class = class_record_with_embedded_refs(0, stale_id);
    let mini = cache_with_class(&[10], Tables::default(), Some(&class));

    let mut guard = SequentialMiniGuard::new(&base).unwrap();
    let error = guard.check_and_record(&mini).unwrap_err();
    assert!(matches!(
        error,
        SpliceError::MiniReference(RemapError::UnresolvedEffectiveReference {
            kind: "embedded function id",
            op: "Factory/BehaviorRefs",
            key
        }) if key == stale_id
    ));
}

#[test]
fn sequential_guard_rejects_a_mini_row_that_retargets_a_base_key() {
    let base = base_cache();
    let mini = cache(
        &[10],
        Tables {
            funcs: vec![func_row(
                BASE_FUNC_PTR,
                "DifferentIdentity",
                MODULE,
                BASE_TYPE_PTR,
                &[],
                0,
            )],
            ..Tables::default()
        },
    );

    let mut guard = SequentialMiniGuard::new(&base).unwrap();
    let error = guard.check_and_record(&mini).unwrap_err();
    assert!(matches!(
        error,
        SpliceError::KeyCollision {
            table: 2,
            key: BASE_FUNC_PTR
        }
    ));
}

#[test]
fn sequential_guard_rejects_a_missing_function_id_operand() {
    let base = base_cache();
    let missing_id = 0x12345;
    let mini = cache(&[9, missing_id, 10], Tables::default());

    let mut guard = SequentialMiniGuard::new(&base).unwrap();
    let error = guard.check_and_record(&mini).unwrap_err();
    assert!(matches!(
        error,
        SpliceError::MiniReference(RemapError::UnresolvedEffectiveReference {
            kind: "function id",
            op: "CALL",
            key
        }) if key == i64::from(missing_id)
    ));
}

#[test]
fn sequential_guard_rejects_an_unmapped_runtime_object_type_id() {
    let base = base_cache();
    let missing_object_id = 0x0800_4321;
    let mini = cache(&[76, missing_object_id, 10], Tables::default());

    let mut guard = SequentialMiniGuard::new(&base).unwrap();
    let error = guard.check_and_record(&mini).unwrap_err();
    assert!(matches!(
        error,
        SpliceError::MiniReference(RemapError::UnresolvedEffectiveReference {
            kind: "type id",
            op: "TYPEID",
            key
        }) if key == i64::from(missing_object_id)
    ));
}

#[test]
fn sequential_guard_rejects_invalid_type_id_operand_shapes() {
    let base = base_cache();
    let cases = [
        ("unknown mask-clear core 12", 12),
        ("qualifier on primitive core", 1 | 0x4000_0000),
        (
            "unsupported high-bit qualifier",
            (BASE_TYPE_ID as u32 | 0x8000_0000) as i32,
        ),
    ];

    for (case, raw_id) in cases {
        let mini = cache(&[76, raw_id, 10], Tables::default());
        assert_guard_rejects_reference(&base, &mini, case);
    }
}

#[test]
fn sequential_guard_rejects_zero_old_reference_rows() {
    let base = base_cache();
    let cases = [
        (
            "T1 zero OldReference",
            cache(
                &[10],
                Tables {
                    types: vec![type_row(0, "ZeroType", MODULE, &[])],
                    ..Tables::default()
                },
            ),
        ),
        (
            "T3 zero OldReference",
            cache(
                &[10],
                Tables {
                    funcs: vec![func_row(0, "ZeroFunction", MODULE, 0, &[], 0)],
                    ..Tables::default()
                },
            ),
        ),
        (
            "T5 zero OldReference",
            cache(
                &[10],
                Tables {
                    globals: vec![global_row(0, "ZeroGlobal", MODULE)],
                    ..Tables::default()
                },
            ),
        ),
    ];

    for (case, mini) in cases {
        assert_guard_rejects_reference(&base, &mini, case);
    }
}

#[test]
fn sequential_guard_rejects_reserved_or_qualified_type_id_rows() {
    let base = base_cache();
    let reserved = cache(
        &[10],
        Tables {
            type_ids: vec![id_row(11, BASE_TYPE_PTR)],
            ..Tables::default()
        },
    );
    assert_guard_rejects_reference(&base, &reserved, "reserved primitive T2 id");

    let cases = [
        (
            "qualified T2 id",
            (0x0800_4321u32 | 0x2000_0000) as i32,
            0x7001,
        ),
        (
            "negative T2 id",
            (0x0800_4322u32 | 0x8000_0000) as i32,
            0x7002,
        ),
        (
            "combined handle-qualified T2 id",
            0x0800_4323 | 0x6000_0000,
            0x7003,
        ),
        (
            "object-kind T2 id with reserved primitive sequence",
            0x0800_0001,
            0x7004,
        ),
    ];
    for (case, id, ptr) in cases {
        let mini = cache(
            &[10],
            Tables {
                types: vec![type_row(ptr, case, MODULE, &[])],
                type_ids: vec![id_row(id, ptr)],
                ..Tables::default()
            },
        );
        assert_guard_rejects_reference(&base, &mini, case);
    }
}

#[test]
fn sequential_guard_normalizes_runtime_ignored_datatype_flags_in_t1_and_t3() {
    const LEAF: i64 = 0x7f00;
    const BASE_TYPE: i64 = 0x7f08;
    const MINI_TYPE: i64 = 0x7f10;
    const BASE_FUNC: i64 = 0x7f18;
    const MINI_FUNC: i64 = 0x7f20;
    let cases = [
        (
            "primitive ignored flags",
            datatype_flags(0, 0x4c, [false; 6]),
            datatype_flags(0, 0x4c, [false, false, true, true, false, true]),
        ),
        (
            "non-handle identifier const-handle flag",
            datatype_flags(LEAF, 5, [false; 6]),
            datatype_flags(LEAF, 5, [false, false, false, true, false, false]),
        ),
        (
            "identifier if-handle-then-const flag",
            datatype_flags(LEAF, 5, [false, false, true, false, false, false]),
            datatype_flags(LEAF, 5, [false, false, true, false, false, true]),
        ),
    ];

    for (case, base_datatype, mini_datatype) in cases {
        let base = cache(
            &[10],
            Tables {
                types: vec![
                    type_row(LEAF, "FlagLeaf", MODULE, &[]),
                    type_row_datatypes(
                        BASE_TYPE,
                        "FlaggedWrapper",
                        MODULE,
                        "",
                        &[base_datatype.clone()],
                    ),
                ],
                type_ids: vec![id_row(0x0800_7f00, LEAF), id_row(0x0800_7f08, BASE_TYPE)],
                ..Tables::default()
            },
        );
        let mini = cache(
            &[10],
            Tables {
                types: vec![type_row_datatypes(
                    MINI_TYPE,
                    "FlaggedWrapper",
                    MODULE,
                    "",
                    &[mini_datatype.clone()],
                )],
                type_ids: vec![id_row(0x0800_7f10, MINI_TYPE)],
                ..Tables::default()
            },
        );
        let error = SequentialMiniGuard::new(&base)
            .unwrap()
            .check_and_record(&mini)
            .expect_err(case);
        assert!(
            matches!(
                error,
                SpliceError::MiniReference(RemapError::InvalidTailRow {
                    table: 0,
                    kind: "symbol identity",
                    ..
                })
            ),
            "{case}: unexpected T1 error: {error:?}"
        );

        let return_type = datatype(0, 0x52);
        let base = cache(
            &[10],
            Tables {
                types: vec![type_row(LEAF, "FlagLeaf", MODULE, &[])],
                type_ids: vec![id_row(0x0800_7f00, LEAF)],
                funcs: vec![func_row_datatypes(
                    BASE_FUNC,
                    "FlaggedFunction",
                    MODULE,
                    "",
                    false,
                    false,
                    false,
                    0,
                    &[base_datatype],
                    &return_type,
                )],
                func_ids: vec![id_row(0x7f18, BASE_FUNC)],
                ..Tables::default()
            },
        );
        let mini = cache(
            &[10],
            Tables {
                funcs: vec![func_row_datatypes(
                    MINI_FUNC,
                    "FlaggedFunction",
                    MODULE,
                    "",
                    false,
                    false,
                    false,
                    0,
                    &[mini_datatype],
                    &return_type,
                )],
                func_ids: vec![id_row(0x7f20, MINI_FUNC)],
                ..Tables::default()
            },
        );
        let error = SequentialMiniGuard::new(&base)
            .unwrap()
            .check_and_record(&mini)
            .expect_err(case);
        assert!(
            matches!(
                error,
                SpliceError::MiniReference(RemapError::InvalidTailRow {
                    table: 2,
                    kind: "symbol identity",
                    ..
                })
            ),
            "{case}: unexpected T3 error: {error:?}"
        );
    }
}

#[test]
fn sequential_guard_rejects_noncanonical_auto_and_zero_token_datatypes_in_t1_and_t3() {
    let cases = [
        (
            "auto primitive",
            datatype_flags(0, 0x52, [false, false, false, false, true, false]),
        ),
        ("zero token", datatype_flags(0, 0, [false; 6])),
    ];

    for (ordinal, (case, datatype_bytes)) in cases.into_iter().enumerate() {
        let type_ptr = 0x7f40 + ordinal as i64 * 8;
        let mini = cache(
            &[10],
            Tables {
                types: vec![type_row_datatypes(
                    type_ptr,
                    case,
                    MODULE,
                    "",
                    &[datatype_bytes.clone()],
                )],
                type_ids: vec![id_row(0x0800_7f40 + ordinal as i32, type_ptr)],
                ..Tables::default()
            },
        );
        let error = SequentialMiniGuard::new(&cache(&[10], Tables::default()))
            .unwrap()
            .check_and_record(&mini)
            .expect_err(case);
        assert!(
            matches!(
                error,
                SpliceError::MiniReference(RemapError::Wire(WireError::InvalidDataType { .. }))
            ),
            "{case}: unexpected T1 error: {error:?}"
        );

        let func_ptr = 0x7f60 + ordinal as i64 * 8;
        let mini = cache(
            &[10],
            Tables {
                funcs: vec![func_row_datatypes(
                    func_ptr,
                    case,
                    MODULE,
                    "",
                    false,
                    false,
                    false,
                    0,
                    &[datatype_bytes],
                    &datatype(0, 0x52),
                )],
                func_ids: vec![id_row(0x7f60 + ordinal as i32, func_ptr)],
                ..Tables::default()
            },
        );
        let error = SequentialMiniGuard::new(&cache(&[10], Tables::default()))
            .unwrap()
            .check_and_record(&mini)
            .expect_err(case);
        assert!(
            matches!(
                error,
                SpliceError::MiniReference(RemapError::Wire(WireError::InvalidDataType { .. }))
            ),
            "{case}: unexpected T3 error: {error:?}"
        );
    }
}

#[test]
fn sequential_guard_rejects_malformed_sia_encodings() {
    let malformed = [
        (
            "negative/UTF-16 length is not supported by FStringInArchive",
            (-1i32).to_le_bytes().to_vec(),
        ),
        (
            "embedded NUL",
            [3i32.to_le_bytes().as_slice(), b"A\0B\0"].concat(),
        ),
        (
            "missing trailing NUL",
            [3i32.to_le_bytes().as_slice(), b"ABCX"].concat(),
        ),
    ];

    for (detail, encoded_name) in malformed {
        let mini = cache(
            &[10],
            Tables {
                static_names: vec![encoded_name],
                ..Tables::default()
            },
        );
        let error = SequentialMiniGuard::new(&cache(&[10], Tables::default()))
            .unwrap()
            .check_and_record(&mini)
            .expect_err(detail);
        assert!(
            matches!(
                error,
                SpliceError::Wire(WireError::InvalidSia {
                    detail: actual,
                    ..
                }) if actual == detail
            ),
            "{detail}: unexpected error: {error:?}"
        );
    }
}

#[test]
fn sequential_guard_length_frames_identity_fields_containing_the_display_separator() {
    let mini = cache(
        &[10],
        Tables {
            types: vec![
                type_row_ns(0x7f80, "X\u{1f}Y", MODULE, "N", &[]),
                type_row_ns(0x7f88, "Y", MODULE, "N\u{1f}X", &[]),
            ],
            type_ids: vec![id_row(0x0800_7f80, 0x7f80), id_row(0x0800_7f88, 0x7f88)],
            ..Tables::default()
        },
    );
    SequentialMiniGuard::new(&cache(&[10], Tables::default()))
        .unwrap()
        .check_and_record(&mini)
        .expect("length framing must keep delimiter-containing field tuples distinct");
}

#[test]
fn sequential_guard_allows_zero_ctor_only_for_native_alloc() {
    let alloc_mini = |ptr, id, name| {
        let mut code = Vec::new();
        qw_op(64, ptr, &mut code);
        code.extend([0, 10]);
        cache(
            &code,
            Tables {
                types: vec![type_row(ptr, name, MODULE, &[])],
                type_ids: vec![id_row(id, ptr)],
                ..Tables::default()
            },
        )
    };
    for (case, ptr, id) in [
        ("APPOBJECT", 0x7fa0, 0x0400_7fa0),
        ("TEMPLATE", 0x7fa8, 0x1000_7fa8),
    ] {
        SequentialMiniGuard::new(&cache(&[10], Tables::default()))
            .unwrap()
            .check_and_record(&alloc_mini(ptr, id, case))
            .unwrap_or_else(|error| panic!("{case} permits ALLOC's null ctor sentinel: {error:?}"));
    }

    for (case, ptr, id) in [
        ("ENUM/mask0", 0x7fb0, 0x0000_7fb0),
        ("SCRIPTOBJECT", 0x7fb8, 0x0800_7fb8),
    ] {
        let error = SequentialMiniGuard::new(&cache(&[10], Tables::default()))
            .unwrap()
            .check_and_record(&alloc_mini(ptr, id, case))
            .unwrap_err();
        assert!(
            matches!(
                error,
                SpliceError::MiniReference(RemapError::UnresolvedEffectiveReference {
                    kind: "function id",
                    op: "ALLOC",
                    key: 0,
                })
            ),
            "{case}: unexpected error: {error:?}"
        );
    }

    let call_mini = cache(&[9, 0, 10], Tables::default());
    let error = SequentialMiniGuard::new(&cache(&[10], Tables::default()))
        .unwrap()
        .check_and_record(&call_mini)
        .expect_err("CALL never treats zero as a native-constructor sentinel");
    assert!(matches!(
        error,
        SpliceError::MiniReference(RemapError::UnresolvedEffectiveReference {
            kind: "function id",
            op: "CALL",
            key: 0,
        })
    ));
}

#[test]
fn allow_new_type_id_allocator_skips_reserved_low_sequences() {
    const RAW_PTR: i64 = 0x7fb0;
    const RAW_ID: i32 = 0x0800_7fb0;
    // The canonical leaf identity for this name hashes to T2 sequence 4 with the fixed kind-3
    // FNV allocator. It must be advanced past AngelScript's primitive range to sequence 12.
    let regen = cache(
        &[76, RAW_ID, 10],
        Tables {
            types: vec![type_row(RAW_PTR, "LowSeq4793533", MODULE, &[])],
            type_ids: vec![id_row(RAW_ID, RAW_PTR)],
            ..Tables::default()
        },
    );
    let (mini, counts) = remap_module_to_base_with_options(
        &regen,
        &cache(&[10], Tables::default()),
        RemapOptions {
            allow_new_symbols: true,
        },
    )
    .unwrap();
    assert_eq!(counts.type_id, 1);
    let tail = parse_tail_tables(&mini, module_region_end(&mini).unwrap()).unwrap();
    assert_eq!(tail.tables[1].keys, vec![0x0800_000c]);
}

#[test]
fn allow_new_rejects_ambiguous_t2_and_t4_rows_for_one_new_pointer() {
    const TYPE_PTR: i64 = 0x7fc0;
    const TYPE_ID_A: i32 = 0x0800_7fc0;
    const TYPE_ID_B: i32 = 0x0800_7fc1;
    const FUNC_PTR: i64 = 0x7fd0;
    const FUNC_ID_A: i32 = 0x0001_7fd0;
    const FUNC_ID_B: i32 = 0x0001_7fd1;
    let base = cache(&[10], Tables::default());

    let duplicate_reverse_type = cache(
        &[76, TYPE_ID_A, 10],
        Tables {
            types: vec![type_row(TYPE_PTR, "ReverseType", MODULE, &[])],
            type_ids: vec![id_row(TYPE_ID_A, TYPE_PTR), id_row(TYPE_ID_B, TYPE_PTR)],
            ..Tables::default()
        },
    );
    let type_error = remap_module_to_base_with_options(
        &duplicate_reverse_type,
        &base,
        RemapOptions {
            allow_new_symbols: true,
        },
    )
    .expect_err("one new T1 pointer cannot be registered by two T2 ids");
    assert!(matches!(
        type_error,
        RemapError::InvalidTailRow {
            table: 1,
            kind: "reverse type-id mapping",
            ..
        }
    ));

    let duplicate_reverse_function = cache(
        &[10],
        Tables {
            funcs: vec![func_row(FUNC_PTR, "Edited", MODULE, 0, &[], 0)],
            func_ids: vec![id_row(FUNC_ID_A, FUNC_PTR), id_row(FUNC_ID_B, FUNC_PTR)],
            ..Tables::default()
        },
    );
    let function_error = remap_module_to_base_with_options(
        &duplicate_reverse_function,
        &base,
        RemapOptions {
            allow_new_symbols: true,
        },
    )
    .expect_err("one new T3 pointer cannot be registered by two T4 ids");
    assert!(matches!(
        function_error,
        RemapError::InvalidTailRow {
            table: 3,
            kind: "reverse function-id mapping",
            ..
        }
    ));
}

#[test]
fn sequential_guard_accepts_one_zero_t4_id_but_rejects_its_aliases() {
    let base = cache(&[10], Tables::default());
    let ptr = 0x7100;
    let row = func_row(ptr, "ZeroIdFunction", MODULE, 0, &[], 0);
    let unique = cache(
        &[10],
        Tables {
            funcs: vec![row.clone()],
            func_ids: vec![id_row(0, ptr)],
            ..Tables::default()
        },
    );
    SequentialMiniGuard::new(&base)
        .unwrap()
        .check_and_record(&unique)
        .expect("T4 id zero is a legal unique FunctionIdReferenceToPointer key");

    let alias = cache(
        &[10],
        Tables {
            funcs: vec![row.clone()],
            func_ids: vec![id_row(0, ptr), id_row(1, ptr)],
            ..Tables::default()
        },
    );
    assert_guard_rejects_reference(&base, &alias, "T4 id zero may not alias another id");

    let duplicate_key = cache(
        &[10],
        Tables {
            funcs: vec![
                row,
                func_row(ptr + 1, "OtherZeroIdFunction", MODULE, 0, &[], 0),
            ],
            func_ids: vec![id_row(0, ptr), id_row(0, ptr + 1)],
            ..Tables::default()
        },
    );
    let error = SequentialMiniGuard::new(&base)
        .unwrap()
        .check_and_record(&duplicate_key)
        .unwrap_err();
    assert!(matches!(
        error,
        SpliceError::SequentialKeyCollision { table: 3, key: 0 }
    ));
}

#[test]
fn t4_zero_row_does_not_resolve_raw_zero_function_id_operand() {
    let ptr = 0x7110;
    let row = func_row(ptr, "ZeroMappedFunction", "ExternalModule", 0, &[], 0);
    let known_module_base = cache_with_named_module(
        "ExternalModule",
        &[10],
        Tables {
            funcs: vec![row.clone()],
            func_ids: vec![id_row(0, ptr)],
            ..Tables::default()
        },
    );
    let mini = cache(
        &[2, 0, 9, 0, 10], // PshC4 0 + CALL 0 must not look like __STATIC_NAME.
        Tables {
            funcs: vec![row],
            func_ids: vec![id_row(0, ptr)],
            static_names: vec![sia("ZeroMustNotBeAStaticAccessor")],
            ..Tables::default()
        },
    );
    let error = SequentialMiniGuard::new(&known_module_base)
        .unwrap()
        .check_and_record(&mini)
        .unwrap_err();
    assert!(matches!(
        error,
        SpliceError::MiniReference(RemapError::UnresolvedEffectiveReference {
            kind: "function id",
            op: "CALL",
            key: 0,
        })
    ));

    let (remapped, counts) = remap_module_to_base_with_options(
        &mini,
        &known_module_base,
        RemapOptions {
            allow_new_symbols: true,
        },
    )
    .expect("allow-new must preserve raw CALL 0 without declaring its T4[0] row");
    assert_eq!(counts.func_id, 0);
    let instructions = disassemble(&collect_function_bytecodes(&remapped).unwrap()[0].bytecode)
        .expect("remapped zero-sentinel bytecode");
    let call = instructions
        .iter()
        .find(|instruction| instruction.op.name == "CALL")
        .unwrap();
    assert_eq!(call.dwords[0], 0);
    let psh = instructions
        .iter()
        .find(|instruction| instruction.op.name == "PshC4")
        .unwrap();
    assert_eq!(psh.dwords[0], 0);
    let tail = parse_tail_tables(&remapped, module_region_end(&remapped).unwrap()).unwrap();
    assert_eq!(tail.tables[2].count, 0);
    assert_eq!(tail.tables[3].count, 0);
    assert_eq!(tail.tables[5].count, 0);

    let base_ptr = 0x7118;
    let strict_base = cache_with_named_module(
        "ExternalModule",
        &[10],
        Tables {
            funcs: vec![func_row(
                base_ptr,
                "ZeroMappedFunction",
                "ExternalModule",
                0,
                &[],
                0,
            )],
            func_ids: vec![id_row(0x7118, base_ptr)],
            ..Tables::default()
        },
    );
    let (strict, strict_counts) = remap_module_to_base(&mini, &strict_base)
        .expect("strict remap must not resolve raw CALL 0 through regen T4[0]");
    assert_eq!(strict_counts.func_id, 0);
    let strict_instructions =
        disassemble(&collect_function_bytecodes(&strict).unwrap()[0].bytecode).unwrap();
    assert_eq!(
        strict_instructions
            .iter()
            .find(|instruction| instruction.op.name == "CALL")
            .unwrap()
            .dwords[0],
        0
    );
    assert_eq!(
        strict_instructions
            .iter()
            .find(|instruction| instruction.op.name == "PshC4")
            .unwrap()
            .dwords[0],
        0
    );
}

#[test]
fn sequential_guard_rejects_t1_key_in_the_t7_derived_domain() {
    let type_id = 0x0800_7120;
    let colliding_key = property_key(type_id, 4);
    let mini = cache(
        &[10],
        Tables {
            types: vec![type_row(colliding_key, "PropertyDomainType", MODULE, &[])],
            type_ids: vec![id_row(type_id, colliding_key)],
            properties: vec![property_row(type_id, 4, "CollidingProperty")],
            ..Tables::default()
        },
    );

    let error = SequentialMiniGuard::new(&cache(&[10], Tables::default()))
        .unwrap()
        .check_and_record(&mini)
        .expect_err("T1 OldReference keys must never occupy the derived T7 key domain");
    assert!(matches!(
        error,
        SpliceError::SequentialKeyCollision { table: 6, key } if key == colliding_key
    ));
}

#[test]
fn allow_new_synthetic_old_references_remain_eight_byte_aligned() {
    let (mini, _) = remap_module_to_base_with_options(
        &regen_cache(),
        &base_cache(),
        RemapOptions {
            allow_new_symbols: true,
        },
    )
    .unwrap();
    let tail = parse_tail_tables(&mini, module_region_end(&mini).unwrap()).unwrap();
    for table in [0usize, 2, 4] {
        for &key in &tail.tables[table].keys {
            assert_eq!(
                key & 7,
                0,
                "synthetic OldReference {key:#x} in T{} must preserve Win64 low3=0 alignment",
                table + 1
            );
        }
    }
}

#[test]
fn sequential_guard_rejects_inconsistent_property_key_fields() {
    let base = base_cache();
    let mut row = property_row(BASE_TYPE_ID, 4, "InconsistentProperty");
    let inconsistent_key = property_key(BASE_TYPE_ID, 4) ^ 2;
    row[..8].copy_from_slice(&inconsistent_key.to_le_bytes());
    let mini = cache(
        &[10],
        Tables {
            properties: vec![row],
            ..Tables::default()
        },
    );
    assert_guard_rejects_reference(&base, &mini, "inconsistent T7 key");
}

#[test]
fn sequential_guard_rejects_high_dword_function_id_alias() {
    let base = base_cache();
    let alias = (1i64 << 32) | i64::from(BASE_FUNC_ID);
    let class = class_record_with_embedded_refs(0, alias);
    let mini = cache_with_class(&[10], Tables::default(), Some(&class));
    assert_guard_rejects_reference(&base, &mini, "high-dword embedded function-id alias");
}

#[test]
fn strict_remap_sign_extends_positive_embedded_id_mapped_to_negative() {
    const REGEN_ID: i32 = 0x7fff_0123;
    const BASE_ID: i32 = 0x8000_1234u32 as i32;
    let (base, regen) = embedded_function_id_caches(BASE_ID, REGEN_ID, i64::from(REGEN_ID));

    let (mini, counts) = remap_module_to_base(&regen, &base).expect("strict embedded-id remap");
    assert_eq!(counts.embed_func_id, 1);
    assert!(contains_single_factory_ref(&mini, i64::from(BASE_ID)));
    assert!(
        !contains_single_factory_ref(&mini, i64::from(BASE_ID as u32)),
        "a negative i32 must not be serialized as a zero-extended int64"
    );
    SequentialMiniGuard::new(&base)
        .unwrap()
        .check_and_record(&mini)
        .expect("the canonically sign-extended strict mini passes admission");
}

#[test]
fn allow_new_remap_clears_negative_high_dword_when_mapped_id_is_positive() {
    const REGEN_ID: i32 = 0x8000_1234u32 as i32;
    const BASE_ID: i32 = 0x7fff_0123;
    let (base, regen) = embedded_function_id_caches(BASE_ID, REGEN_ID, i64::from(REGEN_ID));

    let (mini, counts) = remap_module_to_base_with_options(
        &regen,
        &base,
        RemapOptions {
            allow_new_symbols: true,
        },
    )
    .expect("allow-new embedded-id remap");
    assert_eq!(counts.embed_func_id, 1);
    assert!(contains_single_factory_ref(&mini, i64::from(BASE_ID)));
    let stale_negative_high_dword = (0xffff_ffff_0000_0000u64 | u64::from(BASE_ID as u32)) as i64;
    assert!(
        !contains_single_factory_ref(&mini, stale_negative_high_dword),
        "the regen slot's negative high dword must not survive a positive mapping"
    );
    SequentialMiniGuard::new(&base)
        .unwrap()
        .check_and_record(&mini)
        .expect("the canonical allow-new mini passes admission");
}

#[test]
fn remappers_reject_noncanonical_high_dword_embedded_function_ids() {
    const REGEN_ID: i32 = 0x1234_567;
    const BASE_ID: i32 = 0x2345_678;
    let malformed = (1i64 << 32) | i64::from(REGEN_ID);

    for allow_new_symbols in [false, true] {
        let (base, regen) = embedded_function_id_caches(BASE_ID, REGEN_ID, malformed);
        let error = if allow_new_symbols {
            remap_module_to_base_with_options(
                &regen,
                &base,
                RemapOptions {
                    allow_new_symbols: true,
                },
            )
            .unwrap_err()
        } else {
            remap_module_to_base(&regen, &base).unwrap_err()
        };
        assert!(
            matches!(
                error,
                RemapError::UnresolvedEffectiveReference {
                    kind: "embedded function id",
                    op: "Factory/BehaviorRefs",
                    key,
                } if key == malformed
            ),
            "allow_new_symbols={allow_new_symbols}: {error:?}"
        );
    }
}

#[test]
fn allow_new_rejects_identical_and_conflicting_duplicate_t1_through_t5_keys() {
    let base = cache(&[10], Tables::default());
    let expected_fields = [
        "duplicate TypeReferences key",
        "duplicate TypeIdReferenceToPointer key",
        "duplicate FunctionReferences key",
        "duplicate FunctionIdReferenceToPointer key",
        "duplicate GlobalReferences key",
    ];

    for (table, expected_field) in expected_fields.into_iter().enumerate() {
        for conflicting in [false, true] {
            let regen = cache(&[10], duplicate_tail_key_tables(table, conflicting));
            let error = remap_module_to_base_with_options(
                &regen,
                &base,
                RemapOptions {
                    allow_new_symbols: true,
                },
            )
            .expect_err("a public allow-new remap must never emit duplicate keyed rows");
            assert!(
                matches!(
                    error,
                    RemapError::Wire(WireError::BadLen { field, .. })
                        if field == expected_field
                ),
                "T{} conflicting={conflicting}: {error:?}",
                table + 1
            );
        }
    }
}

#[test]
fn sequential_guard_accepts_zero_embedded_function_sentinel() {
    let base = base_cache();
    let class = class_record_with_embedded_refs(0, 0);
    let mini = cache_with_class(&[10], Tables::default(), Some(&class));
    let prepared = SequentialMiniGuard::new(&base)
        .unwrap()
        .check_and_record(&mini)
        .expect("zero is the Factory/BehaviorRefs sentinel");
    assert_eq!(prepared, mini);
}

#[test]
fn sequential_guard_rejects_invalid_module_record_datatypes() {
    let base = base_cache();
    let cases = [
        (
            "identifier DataType with null pointer",
            cache_with_function_return_datatype(0, 5),
        ),
        (
            "primitive DataType with non-null pointer",
            cache_with_function_return_datatype(BASE_TYPE_PTR, 0x52),
        ),
        (
            "auto DataType with a primitive token",
            cache_with_function_return_datatype_flags(
                0,
                0x52,
                [false, false, false, false, true, false],
            ),
        ),
        (
            "DataType with token zero",
            cache_with_function_return_datatype(0, 0),
        ),
    ];
    for (case, mini) in cases {
        assert_guard_rejects_reference(&base, &mini, case);
    }
}

#[test]
fn sequential_guard_rejects_flagged_type_from_an_unknown_external_module() {
    const PTR: i64 = 0x71a0;
    const ID: i32 = 0x0800_71a0;
    let base = cache(&[10], Tables::default());
    let mini = cache(
        &[76, ID | 0x4000_0000, 10],
        Tables {
            types: vec![type_row(PTR, "UnknownExternalType", "ExternalModule", &[])],
            type_ids: vec![id_row(ID, PTR)],
            ..Tables::default()
        },
    );

    assert_guard_rejects_reference(
        &base,
        &mini,
        "flagged TYPEID cannot claim a module absent from both base and current mini",
    );
    remap_module_to_base_with_options(
        &mini,
        &base,
        RemapOptions {
            allow_new_symbols: true,
        },
    )
    .expect_err("allow-new must reject a selected T1 row from an unknown external module");
}

#[test]
fn sequential_guard_accepts_type_modules_owned_by_the_current_mini_or_base() {
    const PTR: i64 = 0x71a8;
    const ID: i32 = 0x0800_71a8;
    let tables = || Tables {
        types: vec![type_row(PTR, "KnownExternalType", "ExternalModule", &[])],
        type_ids: vec![id_row(ID, PTR)],
        ..Tables::default()
    };

    let base = cache(&[10], Tables::default());
    let current_module =
        cache_with_named_module("ExternalModule", &[76, ID | 0x4000_0000, 10], tables());
    SequentialMiniGuard::new(&base)
        .unwrap()
        .check_and_record(&current_module)
        .expect("a T1 module may name the current mini module");
    remap_module_to_base_with_options(
        &current_module,
        &base,
        RemapOptions {
            allow_new_symbols: true,
        },
    )
    .expect("allow-new may carry a T1 row owned by the current mini module");

    let base_with_external_module = cache_with_named_module("ExternalModule", &[10], tables());
    let different_target_module = cache(&[76, ID | 0x4000_0000, 10], tables());
    SequentialMiniGuard::new(&base_with_external_module)
        .unwrap()
        .check_and_record(&different_target_module)
        .expect("a T1 module may name a base module even when the mini targets another module");
    remap_module_to_base_with_options(
        &different_target_module,
        &base_with_external_module,
        RemapOptions {
            allow_new_symbols: true,
        },
    )
    .expect("allow-new may carry a T1 row owned by a base module");
}

#[test]
fn allow_new_seeds_target_symbols_from_inner_module_name_only() {
    const PTR: i64 = 0x71ac;
    const ID: i32 = 0x0800_71ac;
    let base = cache_with_named_module("PristineBase", &[10], Tables::default());
    let regen = cache_with_module_key_and_name(
        "OuterAlias",
        "InnerReal",
        &[10],
        Tables {
            types: vec![type_row(PTR, "IrrelevantOuterAliasType", "OuterAlias", &[])],
            type_ids: vec![id_row(ID, PTR)],
            ..Tables::default()
        },
    );

    let (mini, counts) = remap_module_to_base_with_options(
        &regen,
        &base,
        RemapOptions {
            allow_new_symbols: true,
        },
    )
    .expect("an outer TMap alias must not make its unrelated symbol row part of the target");
    assert_eq!(counts.total(), 0);
    let tables = parse_tail_tables(&mini, module_region_end(&mini).unwrap()).unwrap();
    assert_eq!(
        tables
            .tables
            .iter()
            .map(|table| table.count)
            .collect::<Vec<_>>(),
        vec![0, 0, 0, 0, 0, 0, 0],
        "only the inner ModuleName may seed target-module symbol retention"
    );
}

#[test]
fn sequential_guard_requires_modules_for_global_imported_and_t5_rows() {
    let base = cache(&[10], Tables::default());
    let mut global_function_code = Vec::new();
    qw_op(61, 0x71b0, &mut global_function_code);
    global_function_code.push(10);
    let mut imported_function_code = Vec::new();
    qw_op(61, 0x71b8, &mut imported_function_code);
    imported_function_code.push(10);
    let mut global_code = Vec::new();
    qw_op(1, 0x71c0, &mut global_code);
    global_code.push(10);
    let cases = [
        (
            "global T3",
            cache(
                &global_function_code,
                Tables {
                    funcs: vec![func_row_flags(
                        0x71b0,
                        "MissingModuleGlobalFunction",
                        "MissingGlobalModule",
                        "",
                        false,
                        false,
                        false,
                        0,
                        &[],
                        0,
                    )],
                    func_ids: vec![id_row(0x71b0, 0x71b0)],
                    ..Tables::default()
                },
            ),
        ),
        (
            "imported T3",
            cache(
                &imported_function_code,
                Tables {
                    funcs: vec![func_row_flags(
                        0x71b8,
                        "MissingModuleImportedFunction",
                        "MissingImportedModule",
                        "",
                        false,
                        true,
                        false,
                        0,
                        &[],
                        0,
                    )],
                    func_ids: vec![id_row(0x71b8, 0x71b8)],
                    ..Tables::default()
                },
            ),
        ),
        (
            "nonempty-module T5",
            cache(
                &global_code,
                Tables {
                    globals: vec![global_row(
                        0x71c0,
                        "MissingModuleGlobal",
                        "MissingGlobalModule",
                    )],
                    ..Tables::default()
                },
            ),
        ),
    ];

    for (case, mini) in cases {
        assert_guard_rejects_reference(&base, &mini, case);
        let result = remap_module_to_base_with_options(
            &mini,
            &base,
            RemapOptions {
                allow_new_symbols: true,
            },
        );
        assert!(
            result.is_err(),
            "allow-new accepted {case} from an unknown module"
        );
    }
}

#[test]
fn sequential_guard_rejects_referenced_imported_t3_with_an_empty_module() {
    const PTR: i64 = 0x71d8;
    const ID: i32 = 0x71d8;
    let base = cache(&[10], Tables::default());
    let mut code = Vec::new();
    qw_op(61, PTR, &mut code);
    code.push(10);
    let mini = cache(
        &code,
        Tables {
            funcs: vec![func_row_flags(
                PTR,
                "EmptyModuleImportedFunction",
                "",
                "",
                false,
                true,
                false,
                0,
                &[],
                0,
            )],
            func_ids: vec![id_row(ID, PTR)],
            ..Tables::default()
        },
    );

    let guard_error = SequentialMiniGuard::new(&base)
        .unwrap()
        .check_and_record(&mini)
        .expect_err("an imported declaration needs exact import membership");
    assert!(matches!(
        guard_error,
        SpliceError::MiniReference(RemapError::InvalidTailRow {
            table: 2,
            row_key: PTR,
            kind: "declaration membership",
            ..
        })
    ));

    let remap_error = remap_module_to_base_with_options(
        &mini,
        &base,
        RemapOptions {
            allow_new_symbols: true,
        },
    )
    .expect_err("allow-new must not carry an imported declaration without import membership");
    assert!(matches!(
        remap_error,
        RemapError::InvalidTailRow {
            table: 2,
            row_key: PTR,
            kind: "declaration membership",
            ..
        }
    ));
}

#[test]
fn sequential_guard_does_not_treat_a_prior_mini_as_module_authority() {
    const PTR: i64 = 0x71e0;
    const ID: i32 = 0x0800_71e0;
    let rows = || Tables {
        types: vec![type_row(PTR, "PriorMiniType", "MiniA", &[])],
        type_ids: vec![id_row(ID, PTR)],
        ..Tables::default()
    };
    let mini_a = cache_with_named_module("MiniA", &[10], rows());
    let mini_b = cache_with_named_module("MiniB", &[76, ID | 0x4000_0000, 10], rows());

    let mut guard = SequentialMiniGuard::new(&cache(&[10], Tables::default())).unwrap();
    guard
        .check_and_record(&mini_a)
        .expect("MiniA is authoritative for its own novel T1/T2 rows");
    let error = guard
        .check_and_record(&mini_b)
        .expect_err("an exact prior-mini row is novelty history, not authority for MiniB");
    assert!(matches!(
        error,
        SpliceError::MiniReference(RemapError::InvalidTailRow {
            table: 0,
            row_key: PTR,
            kind: "declaration membership",
            ..
        })
    ));
}

#[test]
fn sequential_guard_rejects_leaf_t1_without_a_matching_class_or_enum() {
    const PTR: i64 = 0x71e8;
    const ID: i32 = 0x0800_71e8;
    let base = cache_with_named_module("PristineBase", &[10], Tables::default());
    let mini = cache_with_module_key_name_and_records(
        "OuterA",
        "ModuleA",
        &[function(&[76, ID, 10], 0x0500_0101)],
        &[],
        &[],
        &[],
        Tables {
            types: vec![type_row_ns(PTR, "BogusType", "ModuleA", "Types", &[])],
            type_ids: vec![id_row(ID, PTR)],
            ..Tables::default()
        },
    );

    assert_guard_and_allow_new_reject_declaration(
        &base,
        &mini,
        0,
        PTR,
        "leaf T1 names no serialized class or enum",
    );
}

#[test]
fn sequential_guard_accepts_t1_matching_current_or_base_class_and_enum() {
    const CLASS_PTR: i64 = 0x71f0;
    const CLASS_ID: i32 = 0x0800_71f0;
    const ENUM_PTR: i64 = 0x71f8;
    const ENUM_ID: i32 = 0x0800_71f8;
    let class = structural_class_record_full_named(
        "OwnedClass",
        "Types",
        &[],
        &[],
        &[],
        &[],
        &[],
        &[0; 7],
        &[],
        &[],
        None,
    );
    let enum_value = enum_record_named("OwnedEnum", "Types", 0, 0);
    let tables = || Tables {
        types: vec![
            type_row_ns(CLASS_PTR, "OwnedClass", "ModuleA", "Types", &[]),
            type_row_ns(ENUM_PTR, "OwnedEnum", "ModuleA", "Types", &[]),
        ],
        type_ids: vec![id_row(CLASS_ID, CLASS_PTR), id_row(ENUM_ID, ENUM_PTR)],
        ..Tables::default()
    };
    let code = [76, CLASS_ID, 76, ENUM_ID, 10];

    let current = cache_with_module_key_name_and_records(
        "OuterA",
        "ModuleA",
        &[function(&code, 0x0500_0102)],
        &[class.clone()],
        &[enum_value.clone()],
        &[],
        tables(),
    );
    let pristine = cache_with_named_module("PristineBase", &[10], Tables::default());
    SequentialMiniGuard::new(&pristine)
        .unwrap()
        .check_and_record(&current)
        .expect("current class and enum declarations authorize matching leaf T1 rows");
    remap_module_to_base_with_options(
        &current,
        &pristine,
        RemapOptions {
            allow_new_symbols: true,
        },
    )
    .expect("allow-new accepts current class and enum declaration membership");

    let base = cache_with_module_key_name_and_records(
        "BaseOuterA",
        "ModuleA",
        &[],
        &[class],
        &[enum_value],
        &[],
        Tables::default(),
    );
    let from_base = cache_with_module_key_name_and_records(
        "OuterB",
        "ModuleB",
        &[function(&code, 0x0500_0103)],
        &[],
        &[],
        &[],
        tables(),
    );
    SequentialMiniGuard::new(&base)
        .unwrap()
        .check_and_record(&from_base)
        .expect("base class and enum declarations authorize matching leaf T1 rows");
    remap_module_to_base_with_options(
        &from_base,
        &base,
        RemapOptions {
            allow_new_symbols: true,
        },
    )
    .expect("allow-new accepts base class and enum declaration membership");
}

#[test]
fn sequential_guard_rejects_wrong_namespace_t1_declaration() {
    const WRONG_PTR: i64 = 0x7200;
    const WRONG_ID: i32 = 0x0800_7200;
    let class = structural_class_record_full_named(
        "NamespaceType",
        "Right::Namespace",
        &[],
        &[],
        &[],
        &[],
        &[],
        &[0; 7],
        &[],
        &[],
        None,
    );
    let wrong_namespace = cache_with_module_key_name_and_records(
        "OuterA",
        "ModuleA",
        &[function(&[76, WRONG_ID, 10], 0x0500_0104)],
        &[class],
        &[],
        &[],
        Tables {
            types: vec![type_row_ns(
                WRONG_PTR,
                "NamespaceType",
                "ModuleA",
                "Wrong::Namespace",
                &[],
            )],
            type_ids: vec![id_row(WRONG_ID, WRONG_PTR)],
            ..Tables::default()
        },
    );
    let pristine = cache_with_named_module("PristineBase", &[10], Tables::default());
    assert_guard_and_allow_new_reject_declaration(
        &pristine,
        &wrong_namespace,
        0,
        WRONG_PTR,
        "T1 namespace differs from its current class declaration",
    );
}

#[test]
fn sequential_guard_rejects_duplicate_runtime_declarations_atomically() {
    let base = cache_with_named_module("PristineBase", &[10], Tables::default());
    let class = structural_class_record_full_named(
        "DuplicateType",
        "Duplicate::Namespace",
        &[],
        &[],
        &[],
        &[],
        &[],
        &[0; 7],
        &[],
        &[],
        None,
    );
    let enum_value = enum_record_named("DuplicateType", "Duplicate::Namespace", 0, 0);
    let duplicate_type = cache_with_module_key_name_and_records(
        "OuterType",
        "ModuleType",
        &[],
        &[class.clone()],
        &[enum_value],
        &[],
        Tables::default(),
    );
    let corrected_type = cache_with_module_key_name_and_records(
        "OuterType",
        "ModuleType",
        &[],
        &[class],
        &[],
        &[],
        Tables::default(),
    );

    let duplicate_function = cache_with_module_key_name_and_records(
        "OuterFunction",
        "ModuleFunction",
        &[function(&[10], 0x0500_0201), function(&[10], 0x0500_0202)],
        &[],
        &[],
        &[],
        Tables::default(),
    );
    let corrected_function = cache_with_module_key_name_and_records(
        "OuterFunction",
        "ModuleFunction",
        &[function(&[10], 0x0500_0201)],
        &[],
        &[],
        &[],
        Tables::default(),
    );

    let global = global_record_named("DuplicateGlobal", "Duplicate::Namespace");
    let duplicate_global = cache_with_module_key_name_and_records(
        "OuterGlobal",
        "ModuleGlobal",
        &[],
        &[],
        &[],
        &[global.clone(), global.clone()],
        Tables::default(),
    );
    let corrected_global = cache_with_module_key_name_and_records(
        "OuterGlobal",
        "ModuleGlobal",
        &[],
        &[],
        &[],
        &[global],
        Tables::default(),
    );

    let property = property_record_named("DuplicateProperty");
    let duplicate_property_class = structural_class_record_full_named(
        "PropertyOwner",
        "",
        &[property.clone(), property.clone()],
        &[],
        &[],
        &[],
        &[],
        &[0; 7],
        &[],
        &[],
        None,
    );
    let corrected_property_class = structural_class_record_full_named(
        "PropertyOwner",
        "",
        &[property],
        &[],
        &[],
        &[],
        &[],
        &[0; 7],
        &[],
        &[],
        None,
    );
    let duplicate_property = cache_with_module_key_name_and_records(
        "OuterProperty",
        "ModuleProperty",
        &[],
        &[duplicate_property_class],
        &[],
        &[],
        Tables::default(),
    );
    let corrected_property = cache_with_module_key_name_and_records(
        "OuterProperty",
        "ModuleProperty",
        &[],
        &[corrected_property_class],
        &[],
        &[],
        Tables::default(),
    );

    for (field, malformed, corrected) in [
        ("duplicate type declaration", duplicate_type, corrected_type),
        (
            "duplicate function declaration",
            duplicate_function,
            corrected_function,
        ),
        (
            "duplicate global declaration",
            duplicate_global,
            corrected_global,
        ),
        (
            "duplicate property declaration",
            duplicate_property,
            corrected_property,
        ),
    ] {
        let mut guard = SequentialMiniGuard::new(&base).unwrap();
        let error = guard
            .check_and_record(&malformed)
            .expect_err("duplicate runtime declaration must be refused");
        assert!(
            matches!(
                error,
                SpliceError::MiniReference(RemapError::Wire(WireError::BadLen {
                    field: actual,
                    ..
                })) if actual == field
            ),
            "{field}: unexpected error: {error:?}"
        );
        guard
            .check_and_record(&corrected)
            .expect("duplicate-declaration refusal must not commit guard history");
    }
}

#[test]
fn sequential_guard_rejects_prior_only_t1_declaration() {
    const PRIOR_PTR: i64 = 0x7208;
    const PRIOR_ID: i32 = 0x0800_7208;
    let base = cache_with_module_key_name_and_records(
        "BaseOuterA",
        "ModuleA",
        &[],
        &[],
        &[],
        &[],
        Tables::default(),
    );
    let prior_class = structural_class_record_full_named(
        "PriorOnlyType",
        "Types",
        &[],
        &[],
        &[],
        &[],
        &[],
        &[0; 7],
        &[],
        &[],
        None,
    );
    let prior = cache_with_module_key_name_and_records(
        "PriorOuter",
        "ModuleA",
        &[],
        &[prior_class],
        &[],
        &[],
        Tables::default(),
    );
    let later = cache_with_module_key_name_and_records(
        "OuterB",
        "ModuleB",
        &[function(&[76, PRIOR_ID, 10], 0x0500_0105)],
        &[],
        &[],
        &[],
        Tables {
            types: vec![type_row_ns(
                PRIOR_PTR,
                "PriorOnlyType",
                "ModuleA",
                "Types",
                &[],
            )],
            type_ids: vec![id_row(PRIOR_ID, PRIOR_PTR)],
            ..Tables::default()
        },
    );
    let mut guard = SequentialMiniGuard::new(&base).unwrap();
    guard
        .check_and_record(&prior)
        .expect("prior declaration carrier is valid in isolation");
    let error = guard
        .check_and_record(&later)
        .expect_err("a prior mini declaration is not authority for a later T1 row");
    assert!(
        matches!(
            error,
            SpliceError::MiniReference(RemapError::InvalidTailRow {
                table: 0,
                row_key: PRIOR_PTR,
                kind,
                ..
            }) if kind == "declaration membership"
        ),
        "unexpected prior-only T1 error: {error:?}"
    );
}

#[test]
fn sequential_guard_rejects_nonstring_t5_without_a_matching_global_declaration() {
    const PTR: i64 = 0x7210;
    let mut code = Vec::new();
    qw_op(1, PTR, &mut code);
    code.push(10);
    let base = cache_with_named_module("PristineBase", &[10], Tables::default());
    let mini = cache_with_module_key_name_and_records(
        "OuterA",
        "ModuleA",
        &[function(&code, 0x0500_0106)],
        &[],
        &[],
        &[],
        Tables {
            globals: vec![global_row_ns(
                PTR,
                "MissingGlobal",
                "ModuleA",
                "Globals",
                false,
            )],
            ..Tables::default()
        },
    );

    assert_guard_and_allow_new_reject_declaration(
        &base,
        &mini,
        4,
        PTR,
        "non-string T5 names no serialized global variable",
    );
}

#[test]
fn sequential_guard_accepts_t5_matching_current_or_base_global() {
    const PTR: i64 = 0x7218;
    let mut code = Vec::new();
    qw_op(1, PTR, &mut code);
    code.push(10);
    let global = global_record_named("OwnedGlobal", "Globals");
    let tables = || Tables {
        globals: vec![global_row_ns(
            PTR,
            "OwnedGlobal",
            "ModuleA",
            "Globals",
            false,
        )],
        ..Tables::default()
    };

    let pristine = cache_with_named_module("PristineBase", &[10], Tables::default());
    let current = cache_with_module_key_name_and_records(
        "OuterA",
        "ModuleA",
        &[function(&code, 0x0500_0107)],
        &[],
        &[],
        &[global.clone()],
        tables(),
    );
    SequentialMiniGuard::new(&pristine)
        .unwrap()
        .check_and_record(&current)
        .expect("current global declaration authorizes its matching T5 row");
    remap_module_to_base_with_options(
        &current,
        &pristine,
        RemapOptions {
            allow_new_symbols: true,
        },
    )
    .expect("allow-new accepts current global declaration membership");

    let base = cache_with_module_key_name_and_records(
        "BaseOuterA",
        "ModuleA",
        &[],
        &[],
        &[],
        &[global],
        Tables::default(),
    );
    let from_base = cache_with_module_key_name_and_records(
        "OuterB",
        "ModuleB",
        &[function(&code, 0x0500_0108)],
        &[],
        &[],
        &[],
        tables(),
    );
    SequentialMiniGuard::new(&base)
        .unwrap()
        .check_and_record(&from_base)
        .expect("base global declaration authorizes its matching T5 row");
    remap_module_to_base_with_options(
        &from_base,
        &base,
        RemapOptions {
            allow_new_symbols: true,
        },
    )
    .expect("allow-new accepts base global declaration membership");
}

#[test]
fn sequential_guard_rejects_prior_only_t5_global_declaration() {
    const PTR: i64 = 0x7220;
    let base = cache_with_module_key_name_and_records(
        "BaseOuterA",
        "ModuleA",
        &[],
        &[],
        &[],
        &[],
        Tables::default(),
    );
    let prior = cache_with_module_key_name_and_records(
        "PriorOuter",
        "ModuleA",
        &[],
        &[],
        &[],
        &[global_record_named("PriorOnlyGlobal", "Globals")],
        Tables::default(),
    );
    let mut code = Vec::new();
    qw_op(1, PTR, &mut code);
    code.push(10);
    let later = cache_with_module_key_name_and_records(
        "OuterB",
        "ModuleB",
        &[function(&code, 0x0500_0109)],
        &[],
        &[],
        &[],
        Tables {
            globals: vec![global_row_ns(
                PTR,
                "PriorOnlyGlobal",
                "ModuleA",
                "Globals",
                false,
            )],
            ..Tables::default()
        },
    );

    let mut guard = SequentialMiniGuard::new(&base).unwrap();
    guard
        .check_and_record(&prior)
        .expect("prior global carrier is valid in isolation");
    let error = guard
        .check_and_record(&later)
        .expect_err("a prior mini global declaration is not authority for a later T5 row");
    assert!(
        matches!(
            error,
            SpliceError::MiniReference(RemapError::InvalidTailRow {
                table: 4,
                row_key: PTR,
                kind,
                ..
            }) if kind == "declaration membership"
        ),
        "unexpected prior-only T5 error: {error:?}"
    );
}

#[test]
fn sequential_guard_rejects_imported_t3_without_a_matching_function_import() {
    const PTR: i64 = 0x7228;
    const ID: i32 = 0x0500_0110;
    let mut code = Vec::new();
    qw_op(61, PTR, &mut code);
    code.push(10);
    let base = cache_with_named_module("PristineBase", &[10], Tables::default());
    let mini = cache_with_module_key_name_and_records(
        "OuterA",
        "ModuleA",
        &[function(&code, 0x0500_0111)],
        &[],
        &[],
        &[],
        Tables {
            funcs: vec![func_row_flags(
                PTR,
                "ImportedNoBinding",
                "ModuleA",
                "Imports",
                false,
                true,
                false,
                0,
                &[],
                0,
            )],
            func_ids: vec![id_row(ID, PTR)],
            ..Tables::default()
        },
    );

    assert_guard_and_allow_new_reject_declaration(
        &base,
        &mini,
        2,
        PTR,
        "imported T3 has a current module but no matching FunctionImport",
    );
}

#[test]
fn sequential_guard_rejects_empty_module_nondeclaration_t3_and_t5_rows() {
    const FUNC_PTR: i64 = 0x7238;
    const FUNC_ID: i32 = 0x0500_0114;
    const GLOBAL_PTR: i64 = 0x7240;
    let base = cache_with_named_module("PristineBase", &[10], Tables::default());

    let mut call_code = Vec::new();
    qw_op(61, FUNC_PTR, &mut call_code);
    call_code.push(10);
    let function_mini = cache_with_module_key_name_and_records(
        "OuterA",
        "ModuleA",
        &[function(&call_code, 0x0500_0115)],
        &[],
        &[],
        &[],
        Tables {
            funcs: vec![func_row_flags(
                FUNC_PTR,
                "EngineGlobalMissing",
                "",
                "Globals",
                false,
                false,
                false,
                0,
                &[],
                0,
            )],
            func_ids: vec![id_row(FUNC_ID, FUNC_PTR)],
            ..Tables::default()
        },
    );
    assert_guard_and_allow_new_reject_declaration(
        &base,
        &function_mini,
        2,
        FUNC_PTR,
        "empty-module ordinary T3 has no serialized global function declaration",
    );

    let mut global_code = Vec::new();
    qw_op(1, GLOBAL_PTR, &mut global_code);
    global_code.push(10);
    let global_mini = cache_with_module_key_name_and_records(
        "OuterB",
        "ModuleB",
        &[function(&global_code, 0x0500_0116)],
        &[],
        &[],
        &[],
        Tables {
            globals: vec![global_row_ns(
                GLOBAL_PTR,
                "EngineGlobalMissing",
                "",
                "Globals",
                false,
            )],
            ..Tables::default()
        },
    );
    assert_guard_and_allow_new_reject_declaration(
        &base,
        &global_mini,
        4,
        GLOBAL_PTR,
        "empty-module non-string T5 has no serialized global variable declaration",
    );
}

#[test]
fn sequential_guard_accepts_empty_module_string_t5_and_exact_pristine_rows() {
    const STRING_PTR: i64 = 0x7248;
    let base = cache_with_named_module("PristineBase", &[10], Tables::default());
    let mut string_code = Vec::new();
    qw_op(1, STRING_PTR, &mut string_code);
    string_code.push(10);
    let string_mini = cache_with_module_key_name_and_records(
        "OuterA",
        "ModuleA",
        &[function(&string_code, 0x0500_0117)],
        &[],
        &[],
        &[],
        Tables {
            globals: vec![global_row_ns(STRING_PTR, "literal value", "", "", true)],
            ..Tables::default()
        },
    );
    SequentialMiniGuard::new(&base)
        .unwrap()
        .check_and_record(&string_mini)
        .expect("string T5 rows are literal values and need no declaration membership");
    remap_module_to_base_with_options(
        &string_mini,
        &base,
        RemapOptions {
            allow_new_symbols: true,
        },
    )
    .expect("allow-new accepts an empty-module string T5 literal");

    const FUNC_PTR: i64 = 0x7250;
    const FUNC_ID: i32 = 0x0500_0118;
    const GLOBAL_PTR: i64 = 0x7258;
    let repeated_tables = || Tables {
        funcs: vec![func_row_flags(
            FUNC_PTR,
            "PristineEngineGlobal",
            "",
            "Globals",
            false,
            false,
            false,
            0,
            &[],
            0,
        )],
        func_ids: vec![id_row(FUNC_ID, FUNC_PTR)],
        globals: vec![global_row_ns(
            GLOBAL_PTR,
            "PristineEngineGlobal",
            "",
            "Globals",
            false,
        )],
        ..Tables::default()
    };
    let base = cache_with_named_module("PristineBase", &[10], repeated_tables());
    let mut code = Vec::new();
    qw_op(61, FUNC_PTR, &mut code);
    qw_op(1, GLOBAL_PTR, &mut code);
    code.push(10);
    let repeat = cache_with_module_key_name_and_records(
        "OuterB",
        "ModuleB",
        &[function(&code, 0x0500_0119)],
        &[],
        &[],
        &[],
        repeated_tables(),
    );
    SequentialMiniGuard::new(&base)
        .unwrap()
        .check_and_record(&repeat)
        .expect("exact pristine T3/T4/T5 repeats are grandfathered without declarations");
    remap_module_to_base_with_options(
        &repeat,
        &base,
        RemapOptions {
            allow_new_symbols: true,
        },
    )
    .expect("allow-new resolves exact empty-module rows back to the pristine generation");
}

#[test]
fn sequential_guard_rejects_empty_module_leaf_and_unauthorized_template_sentinel_t1() {
    const EMPTY_PTR: i64 = 0x7260;
    const EMPTY_ID: i32 = 0x0800_7260;
    const SENTINEL_PTR: i64 = 0x7268;
    const SENTINEL_ID: i32 = 0x0800_7268;
    let base = cache_with_named_module("PristineBase", &[10], Tables::default());

    let empty_leaf = cache_with_module_key_name_and_records(
        "OuterA",
        "ModuleA",
        &[function(&[76, EMPTY_ID, 10], 0x0500_011a)],
        &[],
        &[],
        &[],
        Tables {
            types: vec![type_row_ns(
                EMPTY_PTR,
                "EngineTypeMissing",
                "",
                "Types",
                &[],
            )],
            type_ids: vec![id_row(EMPTY_ID, EMPTY_PTR)],
            ..Tables::default()
        },
    );
    assert_guard_and_allow_new_reject_declaration(
        &base,
        &empty_leaf,
        0,
        EMPTY_PTR,
        "empty-module leaf T1 has no class or enum declaration",
    );

    let sentinel = cache_with_module_key_name_and_records(
        "OuterB",
        "ModuleB",
        &[function(&[76, SENTINEL_ID, 10], 0x0500_011b)],
        &[],
        &[],
        &[],
        Tables {
            types: vec![type_row_ns(
                SENTINEL_PTR,
                "T",
                "$__T__",
                "TemplateParameters",
                &[],
            )],
            type_ids: vec![id_row(SENTINEL_ID, SENTINEL_PTR)],
            ..Tables::default()
        },
    );
    assert_guard_and_allow_new_reject_declaration(
        &base,
        &sentinel,
        0,
        SENTINEL_PTR,
        "$__T__ sentinel has no exact pristine authority",
    );
}

#[test]
fn sequential_guard_accepts_exact_pristine_template_sentinel_t1() {
    const PTR: i64 = 0x7270;
    const ID: i32 = 0x0800_7270;
    let tables = || Tables {
        types: vec![type_row_ns(PTR, "T", "$__T__", "TemplateParameters", &[])],
        type_ids: vec![id_row(ID, PTR)],
        ..Tables::default()
    };
    let base = cache_with_named_module("PristineBase", &[10], tables());
    let repeat = cache_with_module_key_name_and_records(
        "OuterA",
        "ModuleA",
        &[function(&[76, ID, 10], 0x0500_011c)],
        &[],
        &[],
        &[],
        tables(),
    );

    SequentialMiniGuard::new(&base)
        .unwrap()
        .check_and_record(&repeat)
        .expect("an exact pristine-authorized $__T__ sentinel remains valid");
    remap_module_to_base_with_options(
        &repeat,
        &base,
        RemapOptions {
            allow_new_symbols: true,
        },
    )
    .expect("allow-new resolves an exact pristine $__T__ sentinel");
}

#[test]
fn sequential_guard_rejects_template_instance_without_a_pristine_template_base() {
    const FOO_PTR: i64 = 0x7278;
    const FOO_ID: i32 = 0x0800_7278;
    const BOGUS_PTR: i64 = 0x7280;
    const BOGUS_ID: i32 = 0x1000_7280;
    let foo_class = structural_class_record_full_named(
        "Foo",
        "Types",
        &[],
        &[],
        &[],
        &[],
        &[],
        &[0; 7],
        &[],
        &[],
        None,
    );
    let base = cache_with_module_key_name_and_records(
        "BaseOuter",
        "TemplateBase",
        &[],
        &[foo_class],
        &[],
        &[],
        Tables {
            types: vec![type_row_ns(FOO_PTR, "Foo", "TemplateBase", "Types", &[])],
            type_ids: vec![id_row(FOO_ID, FOO_PTR)],
            ..Tables::default()
        },
    );
    let mini = cache_with_module_key_name_and_records(
        "OuterA",
        "ModuleA",
        &[function(&[76, BOGUS_ID, 10], 0x0500_011d)],
        &[],
        &[],
        &[],
        Tables {
            types: vec![
                type_row_ns(FOO_PTR, "Foo", "TemplateBase", "Types", &[]),
                type_row_ns(
                    BOGUS_PTR,
                    "BogusTemplate",
                    "IgnoredModule",
                    "Templates",
                    &[FOO_PTR],
                ),
            ],
            type_ids: vec![id_row(FOO_ID, FOO_PTR), id_row(BOGUS_ID, BOGUS_PTR)],
            ..Tables::default()
        },
    );

    assert_guard_and_allow_new_reject_declaration(
        &base,
        &mini,
        0,
        BOGUS_PTR,
        "template Name/Namespace/arity has no pristine T1 template instance",
    );
}

#[test]
fn sequential_guard_rejects_template_instance_with_only_wrong_arity_authority() {
    const FOO_PTR: i64 = 0x72e8;
    const FOO_ID: i32 = 0x0800_72e8;
    const BASE_BOX_PTR: i64 = 0x72f0;
    const BASE_BOX_ID: i32 = 0x1000_72f0;
    const TWO_ARG_BOX_PTR: i64 = 0x72f8;
    const TWO_ARG_BOX_ID: i32 = 0x1000_72f8;
    let foo = structural_class_record_full_named(
        "Foo",
        "Types",
        &[],
        &[],
        &[],
        &[],
        &[],
        &[0; 7],
        &[],
        &[],
        None,
    );
    let base = cache_with_module_key_name_and_records(
        "BaseOuter",
        "TemplateBase",
        &[],
        &[foo],
        &[],
        &[],
        Tables {
            types: vec![
                type_row_ns(FOO_PTR, "Foo", "TemplateBase", "Types", &[]),
                type_row_ns(
                    BASE_BOX_PTR,
                    "Box",
                    "IgnoredModule",
                    "Templates",
                    &[FOO_PTR],
                ),
            ],
            type_ids: vec![id_row(FOO_ID, FOO_PTR), id_row(BASE_BOX_ID, BASE_BOX_PTR)],
            ..Tables::default()
        },
    );
    let mini = cache_with_module_key_name_and_records(
        "OuterA",
        "ModuleA",
        &[function(&[76, TWO_ARG_BOX_ID, 10], 0x0500_0125)],
        &[],
        &[],
        &[],
        Tables {
            types: vec![
                type_row_ns(FOO_PTR, "Foo", "TemplateBase", "Types", &[]),
                type_row_ns(
                    TWO_ARG_BOX_PTR,
                    "Box",
                    "AnotherIgnoredModule",
                    "Templates",
                    &[FOO_PTR, FOO_PTR],
                ),
            ],
            type_ids: vec![
                id_row(FOO_ID, FOO_PTR),
                id_row(TWO_ARG_BOX_ID, TWO_ARG_BOX_PTR),
            ],
            ..Tables::default()
        },
    );

    assert_guard_and_allow_new_reject_declaration(
        &base,
        &mini,
        0,
        TWO_ARG_BOX_PTR,
        "pristine Box<T> must not authorize novel Box<T,U>",
    );
}

#[test]
fn sequential_guard_accepts_new_template_combination_from_pristine_template_authority() {
    const FOO_PTR: i64 = 0x7288;
    const FOO_ID: i32 = 0x0800_7288;
    const BASE_BOX_PTR: i64 = 0x7290;
    const BASE_BOX_ID: i32 = 0x1000_7290;
    const BAR_PTR: i64 = 0x7298;
    const BAR_ID: i32 = 0x0800_7298;
    const NEW_BOX_PTR: i64 = 0x72a0;
    const NEW_BOX_ID: i32 = 0x1000_72a0;
    let foo_class = structural_class_record_full_named(
        "Foo",
        "Types",
        &[],
        &[],
        &[],
        &[],
        &[],
        &[0; 7],
        &[],
        &[],
        None,
    );
    let base = cache_with_module_key_name_and_records(
        "BaseOuter",
        "TemplateBase",
        &[],
        &[foo_class],
        &[],
        &[],
        Tables {
            types: vec![
                type_row_ns(FOO_PTR, "Foo", "TemplateBase", "Types", &[]),
                type_row_ns(
                    BASE_BOX_PTR,
                    "Box",
                    "PristineIgnoredModule",
                    "Templates",
                    &[FOO_PTR],
                ),
            ],
            type_ids: vec![id_row(FOO_ID, FOO_PTR), id_row(BASE_BOX_ID, BASE_BOX_PTR)],
            ..Tables::default()
        },
    );
    let bar_class = structural_class_record_full_named(
        "Bar",
        "Types",
        &[],
        &[],
        &[],
        &[],
        &[],
        &[0; 7],
        &[],
        &[],
        None,
    );
    let mini = cache_with_module_key_name_and_records(
        "OuterA",
        "ModuleA",
        &[function(&[76, NEW_BOX_ID, 10], 0x0500_011e)],
        &[bar_class],
        &[],
        &[],
        Tables {
            types: vec![
                type_row_ns(BAR_PTR, "Bar", "ModuleA", "Types", &[]),
                type_row_ns(
                    NEW_BOX_PTR,
                    "Box",
                    "CurrentIgnoredModule",
                    "Templates",
                    &[BAR_PTR],
                ),
            ],
            type_ids: vec![id_row(BAR_ID, BAR_PTR), id_row(NEW_BOX_ID, NEW_BOX_PTR)],
            ..Tables::default()
        },
    );

    SequentialMiniGuard::new(&base)
        .unwrap()
        .check_and_record(&mini)
        .expect("pristine Box<Foo> authorizes novel Box<Bar> by Name/Namespace/arity");
    remap_module_to_base_with_options(
        &mini,
        &base,
        RemapOptions {
            allow_new_symbols: true,
        },
    )
    .expect("allow-new accepts a template combination backed by a pristine template base");
}

#[test]
fn sequential_guard_rejects_script_t7_name_without_a_matching_current_property() {
    const OWNER_PTR: i64 = 0x72a8;
    const OWNER_ID: i32 = 0x0800_72a8;
    const OFFSET: i32 = 4;
    let owner = structural_class_record_full_named(
        "Owner",
        "Types",
        &[property_record_named("Good")],
        &[],
        &[],
        &[],
        &[],
        &[0; 7],
        &[],
        &[],
        None,
    );
    let mini = cache_with_module_key_name_and_records(
        "OuterA",
        "ModuleA",
        &[function(&[79 | (OFFSET << 16), OWNER_ID, 10], 0x0500_011f)],
        &[owner],
        &[],
        &[],
        Tables {
            types: vec![type_row_ns(OWNER_PTR, "Owner", "ModuleA", "Types", &[])],
            type_ids: vec![id_row(OWNER_ID, OWNER_PTR)],
            properties: vec![property_row(OWNER_ID, OFFSET, "Bogus")],
            ..Tables::default()
        },
    );
    let base = cache_with_named_module("PristineBase", &[10], Tables::default());

    assert_guard_and_allow_new_reject_declaration(
        &base,
        &mini,
        6,
        property_key(OWNER_ID, OFFSET),
        "script T7 name differs from the owner's direct current property",
    );
}

#[test]
fn sequential_guard_accepts_script_t7_matching_current_or_base_property() {
    const CURRENT_PTR: i64 = 0x72b0;
    const CURRENT_ID: i32 = 0x0800_72b0;
    const BASE_PTR: i64 = 0x72b8;
    const BASE_ID: i32 = 0x0800_72b8;
    const OFFSET: i32 = 4;
    let owner_class = |name: &str| {
        structural_class_record_full_named(
            name,
            "Types",
            &[property_record_named("Good")],
            &[],
            &[],
            &[],
            &[],
            &[0; 7],
            &[],
            &[],
            None,
        )
    };

    let current = cache_with_module_key_name_and_records(
        "OuterA",
        "ModuleA",
        &[function(
            &[79 | (OFFSET << 16), CURRENT_ID, 10],
            0x0500_0120,
        )],
        &[owner_class("CurrentOwner")],
        &[],
        &[],
        Tables {
            types: vec![type_row_ns(
                CURRENT_PTR,
                "CurrentOwner",
                "ModuleA",
                "Types",
                &[],
            )],
            type_ids: vec![id_row(CURRENT_ID, CURRENT_PTR)],
            properties: vec![property_row(CURRENT_ID, OFFSET, "Good")],
            ..Tables::default()
        },
    );
    let pristine = cache_with_named_module("PristineBase", &[10], Tables::default());
    SequentialMiniGuard::new(&pristine)
        .unwrap()
        .check_and_record(&current)
        .expect("a script T7 may match a direct property in the current mini");
    remap_module_to_base_with_options(
        &current,
        &pristine,
        RemapOptions {
            allow_new_symbols: true,
        },
    )
    .expect("allow-new accepts a script T7 backed by a current property");

    let base = cache_with_module_key_name_and_records(
        "BaseOuter",
        "ModuleA",
        &[],
        &[owner_class("BaseOwner")],
        &[],
        &[],
        Tables {
            types: vec![type_row_ns(BASE_PTR, "BaseOwner", "ModuleA", "Types", &[])],
            type_ids: vec![id_row(BASE_ID, BASE_PTR)],
            ..Tables::default()
        },
    );
    let from_base = cache_with_module_key_name_and_records(
        "OuterB",
        "ModuleB",
        &[function(&[79 | (OFFSET << 16), BASE_ID, 10], 0x0500_0121)],
        &[],
        &[],
        &[],
        Tables {
            properties: vec![property_row(BASE_ID, OFFSET, "Good")],
            ..Tables::default()
        },
    );
    SequentialMiniGuard::new(&base)
        .unwrap()
        .check_and_record(&from_base)
        .expect("a script T7 may match a direct property in the pristine base");
    remap_module_to_base_with_options(
        &from_base,
        &base,
        RemapOptions {
            allow_new_symbols: true,
        },
    )
    .expect("allow-new accepts a script T7 backed by a pristine property");
}

#[test]
fn allow_new_carries_only_used_t7_for_existing_non_target_script_owner() {
    const BASE_OWNER_PTR: i64 = 0x72ba_1000;
    const REGEN_OWNER_PTR: i64 = 0x72ba_2000;
    const BASE_OWNER_ID: i32 = 0x0800_72ba;
    const REGEN_OWNER_ID: i32 = 0x0800_73ba;
    const OFFSET: i32 = 12;
    const OWNER_MODULE: &str = "OwnerModule";
    const TARGET_MODULE: &str = "TargetModule";

    let owner = structural_class_record_full_named(
        "ExternalOwner",
        "Types",
        &[property_record_named("UsedField")],
        &[],
        &[],
        &[],
        &[],
        &[0; 7],
        &[],
        &[],
        None,
    );
    let base = cache_with_module_key_name_and_records(
        "OwnerOuter",
        OWNER_MODULE,
        &[],
        &[owner],
        &[],
        &[],
        Tables {
            types: vec![type_row_ns(
                BASE_OWNER_PTR,
                "ExternalOwner",
                OWNER_MODULE,
                "Types",
                &[],
            )],
            type_ids: vec![id_row(BASE_OWNER_ID, BASE_OWNER_PTR)],
            ..Tables::default()
        },
    );
    let regen = cache_with_module_key_name_and_records(
        "TargetOuter",
        TARGET_MODULE,
        &[function(
            &[79 | (OFFSET << 16), REGEN_OWNER_ID, 10],
            0x0500_0190,
        )],
        &[],
        &[],
        &[],
        Tables {
            types: vec![type_row_ns(
                REGEN_OWNER_PTR,
                "ExternalOwner",
                OWNER_MODULE,
                "Types",
                &[],
            )],
            type_ids: vec![id_row(REGEN_OWNER_ID, REGEN_OWNER_PTR)],
            properties: vec![
                property_row(REGEN_OWNER_ID, OFFSET, "UsedField"),
                property_row(REGEN_OWNER_ID, OFFSET + 4, "UnreferencedField"),
            ],
            ..Tables::default()
        },
    );

    let (mini, _) = remap_module_to_base_with_options(
        &regen,
        &base,
        RemapOptions {
            allow_new_symbols: true,
        },
    )
    .expect("the pristine class declaration authorizes its newly referenced property");
    let mini_tail = module_region_end(&mini).unwrap();
    let tables = parse_tail_tables(&mini, mini_tail).unwrap();
    assert_eq!(
        tables
            .tables
            .iter()
            .map(|table| table.count)
            .collect::<Vec<_>>(),
        vec![0, 0, 0, 0, 0, 0, 1],
        "only the concretely used property row is retained"
    );

    let composed = SequentialMiniGuard::new(&base)
        .unwrap()
        .compose_add(&base, &mini)
        .expect("the production composition path accepts the retained pristine-backed T7");
    let refs = RefResolver::build(&composed).unwrap();
    assert_eq!(refs.member(BASE_OWNER_ID, OFFSET), Some("UsedField"));
    let functions = collect_function_bytecodes(&composed).unwrap();
    let addsi = functions
        .iter()
        .flat_map(|function| disassemble(&function.bytecode).unwrap())
        .find(|instruction| instruction.op.name == "ADDSi")
        .unwrap();
    assert_eq!(addsi.dwords[0] as i32, BASE_OWNER_ID);
}

#[test]
fn sequential_guard_rejects_script_t7_with_only_prior_mini_property_authority() {
    const OWNER_PTR: i64 = 0x72c0;
    const OWNER_ID: i32 = 0x0800_72c0;
    const OFFSET: i32 = 4;
    let empty_owner = structural_class_record_full_named(
        "Owner",
        "Types",
        &[],
        &[],
        &[],
        &[],
        &[],
        &[0; 7],
        &[],
        &[],
        None,
    );
    let base = cache_with_module_key_name_and_records(
        "BaseOuter",
        "ModuleA",
        &[],
        &[empty_owner],
        &[],
        &[],
        Tables {
            types: vec![type_row_ns(OWNER_PTR, "Owner", "ModuleA", "Types", &[])],
            type_ids: vec![id_row(OWNER_ID, OWNER_PTR)],
            ..Tables::default()
        },
    );
    let prior_owner = structural_class_record_full_named(
        "Owner",
        "Types",
        &[property_record_named("PriorOnly")],
        &[],
        &[],
        &[],
        &[],
        &[0; 7],
        &[],
        &[],
        None,
    );
    let prior = cache_with_module_key_name_and_records(
        "PriorOuter",
        "ModuleA",
        &[],
        &[prior_owner],
        &[],
        &[],
        Tables::default(),
    );
    let later = cache_with_module_key_name_and_records(
        "OuterB",
        "ModuleB",
        &[function(&[79 | (OFFSET << 16), OWNER_ID, 10], 0x0500_0122)],
        &[],
        &[],
        &[],
        Tables {
            properties: vec![property_row(OWNER_ID, OFFSET, "PriorOnly")],
            ..Tables::default()
        },
    );

    let mut guard = SequentialMiniGuard::new(&base).unwrap();
    guard
        .check_and_record(&prior)
        .expect("a property-bearing prior module is structurally valid in isolation");
    let error = guard
        .check_and_record(&later)
        .expect_err("a prior mini property is not pristine declaration authority");
    assert!(
        matches!(
            error,
            SpliceError::MiniReference(RemapError::InvalidTailRow {
                table: 6,
                row_key,
                kind,
                ..
            }) if row_key == property_key(OWNER_ID, OFFSET)
                && kind == "declaration membership"
        ),
        "unexpected prior-only T7 error: {error:?}"
    );
}

#[test]
fn sequential_guard_requires_pristine_exact_t7_authority_for_native_and_template_owners() {
    const OFFSET: i32 = 4;
    for (name, id, function_id, tables) in [
        (
            "NativeOwner",
            0x0400_72c8,
            0x0500_0123,
            Tables {
                types: vec![type_row_ns(0x72c8, "NativeOwner", "", "Engine", &[])],
                type_ids: vec![id_row(0x0400_72c8, 0x72c8)],
                ..Tables::default()
            },
        ),
        (
            "TemplateOwner",
            0x1000_72d0,
            0x0500_0124,
            Tables {
                types: vec![
                    type_row_ns(0x72e8, "TemplateArg", "", "Engine", &[]),
                    type_row_ns(0x72d0, "TemplateOwner", "IgnoredModule", "Types", &[0x72e8]),
                ],
                type_ids: vec![id_row(0x0400_72e8, 0x72e8), id_row(0x1000_72d0, 0x72d0)],
                ..Tables::default()
            },
        ),
    ] {
        let base = cache_with_named_module("PristineBase", &[10], tables);
        let mini = cache_with_module_key_name_and_records(
            name,
            "ModuleA",
            &[function(&[79 | (OFFSET << 16), id, 10], function_id)],
            &[],
            &[],
            &[],
            Tables {
                properties: vec![property_row(id, OFFSET, "Good")],
                ..Tables::default()
            },
        );
        assert_guard_and_allow_new_reject_declaration(
            &base,
            &mini,
            6,
            property_key(id, OFFSET),
            &format!("{name} T7 is novel rather than an exact pristine row"),
        );
    }
}

#[test]
fn sequential_guard_compose_edit_rejects_deleting_class_referenced_by_retained_t1() {
    const OWNER_PTR: i64 = 0x72d8;
    const OWNER_ID: i32 = 0x0800_72d8;
    let owner = structural_class_record_full_named(
        "RetainedOwner",
        "Types",
        &[],
        &[],
        &[],
        &[],
        &[],
        &[0; 7],
        &[],
        &[],
        None,
    );
    let base = cache_with_module_key_name_and_records(
        "OuterA",
        "ModuleA",
        &[],
        &[owner.clone()],
        &[],
        &[],
        Tables {
            types: vec![type_row_ns(
                OWNER_PTR,
                "RetainedOwner",
                "ModuleA",
                "Types",
                &[],
            )],
            type_ids: vec![id_row(OWNER_ID, OWNER_PTR)],
            ..Tables::default()
        },
    );
    let deleted = cache_with_module_key_name_and_records(
        "OuterA",
        "ModuleA",
        &[],
        &[],
        &[],
        &[],
        Tables::default(),
    );
    let corrected = cache_with_module_key_name_and_records(
        "OuterA",
        "ModuleA",
        &[],
        &[owner],
        &[],
        &[],
        Tables::default(),
    );

    let mut guard = SequentialMiniGuard::new(&base).unwrap();
    let error = guard
        .compose_edit(&base, &deleted, "OuterA")
        .expect_err("the prospective output retains T1 but deletes its direct class");
    assert!(
        matches!(
            error,
            SpliceError::ComposedModule(RemapError::InvalidTailRow {
                table: 0,
                row_key: OWNER_PTR,
                kind,
                ..
            }) if kind == "declaration membership"
        ),
        "unexpected retained-T1 edit error: {error:?}"
    );
    guard
        .compose_edit(&base, &corrected, "OuterA")
        .expect("failed final-output validation must not poison a corrected class retry");
}

#[test]
fn sequential_guard_compose_edit_rejects_deleting_global_referenced_by_retained_t5() {
    const GLOBAL_PTR: i64 = 0x72e0;
    let global = global_record_named("RetainedGlobal", "Globals");
    let base = cache_with_module_key_name_and_records(
        "OuterA",
        "ModuleA",
        &[],
        &[],
        &[],
        &[global.clone()],
        Tables {
            globals: vec![global_row_ns(
                GLOBAL_PTR,
                "RetainedGlobal",
                "ModuleA",
                "Globals",
                false,
            )],
            ..Tables::default()
        },
    );
    let deleted = cache_with_module_key_name_and_records(
        "OuterA",
        "ModuleA",
        &[],
        &[],
        &[],
        &[],
        Tables::default(),
    );
    let corrected = cache_with_module_key_name_and_records(
        "OuterA",
        "ModuleA",
        &[],
        &[],
        &[],
        &[global],
        Tables::default(),
    );

    let mut guard = SequentialMiniGuard::new(&base).unwrap();
    let error = guard
        .compose_edit(&base, &deleted, "OuterA")
        .expect_err("the prospective output retains T5 but deletes its global declaration");
    assert!(
        matches!(
            error,
            SpliceError::ComposedModule(RemapError::InvalidTailRow {
                table: 4,
                row_key: GLOBAL_PTR,
                kind,
                ..
            }) if kind == "declaration membership"
        ),
        "unexpected retained-T5 edit error: {error:?}"
    );
    guard
        .compose_edit(&base, &corrected, "OuterA")
        .expect("failed final-output validation must not poison a corrected global retry");
}

#[test]
fn sequential_guard_resolves_method_module_ownership_through_its_owner_type() {
    const OWNER_PTR: i64 = 0x71c8;
    const METHOD_PTR: i64 = 0x71d0;
    let mini = cache(
        &[10],
        Tables {
            types: vec![type_row(OWNER_PTR, "CurrentModuleOwner", MODULE, &[])],
            type_ids: vec![id_row(0x0800_71c8, OWNER_PTR)],
            funcs: vec![func_row_flags(
                METHOD_PTR,
                "OwnerScopedMethod",
                "MissingButIgnoredMethodModule",
                "Missing::ButIgnoredMethodNamespace",
                false,
                false,
                true,
                OWNER_PTR,
                &[],
                0,
            )],
            func_ids: vec![id_row(0x71d0, METHOD_PTR)],
            ..Tables::default()
        },
    );

    SequentialMiniGuard::new(&cache(&[10], Tables::default()))
        .unwrap()
        .check_and_record(&mini)
        .expect("a method resolves through its owner type, not its ignored T3 Module field");
}

#[test]
fn sequential_guard_rejects_new_symbol_rows_without_reverse_ids() {
    let base = base_cache();
    let type_mini = cache(
        &[10],
        Tables {
            types: vec![type_row(0x7201, "UnmappedNewType", MODULE, &[])],
            ..Tables::default()
        },
    );
    assert_guard_rejects_reference(&base, &type_mini, "T1 row without reverse T2 id");

    let function_mini = cache(
        &[10],
        Tables {
            funcs: vec![func_row(0x7202, "UnmappedNewFunction", MODULE, 0, &[], 0)],
            ..Tables::default()
        },
    );
    assert_guard_rejects_reference(&base, &function_mini, "T3 row without reverse T4 id");
}

#[test]
fn sequential_guard_rejects_duplicate_reverse_id_aliases() {
    let base = base_cache();
    let type_ptr = 0x7301;
    let type_mini = cache(
        &[10],
        Tables {
            types: vec![type_row(type_ptr, "AliasedNewType", MODULE, &[])],
            type_ids: vec![id_row(0x0800_7301, type_ptr), id_row(0x0800_7302, type_ptr)],
            ..Tables::default()
        },
    );
    assert_guard_rejects_reference(&base, &type_mini, "two T2 ids alias one T1 ptr");

    let function_ptr = 0x7302;
    let function_mini = cache(
        &[10],
        Tables {
            funcs: vec![func_row(
                function_ptr,
                "AliasedNewFunction",
                MODULE,
                0,
                &[],
                0,
            )],
            func_ids: vec![id_row(0x7301, function_ptr), id_row(0x7302, function_ptr)],
            ..Tables::default()
        },
    );
    assert_guard_rejects_reference(&base, &function_mini, "two T4 ids alias one T3 ptr");
}

#[test]
fn sequential_guard_rejects_type_and_function_identity_aliases_against_base() {
    let base = base_cache();
    let type_alias = type_symbol_mini(0x7401, 0x0800_7401, "ExistingType", &[]);
    let type_error = SequentialMiniGuard::new(&base)
        .unwrap()
        .check_and_record(&type_alias)
        .unwrap_err();
    assert!(matches!(
        type_error,
        SpliceError::MiniReference(RemapError::InvalidTailRow {
            table: 0,
            row_key: 0x7401,
            kind: "symbol identity",
            ref detail,
        }) if detail.contains("0x1111")
    ));

    let function_alias = function_symbol_mini(0x7402, 0x7402, "ExistingFn", BASE_TYPE_PTR, &[]);
    let function_error = SequentialMiniGuard::new(&base)
        .unwrap()
        .check_and_record(&function_alias)
        .unwrap_err();
    assert!(matches!(
        function_error,
        SpliceError::MiniReference(RemapError::InvalidTailRow {
            table: 2,
            row_key: 0x7402,
            kind: "symbol identity",
            ref detail,
        }) if detail.contains("0x1222")
    ));
}

#[test]
fn sequential_guard_rejects_type_and_function_identity_aliases_within_one_mini() {
    let base = cache(&[10], Tables::default());
    let type_mini = cache(
        &[10],
        Tables {
            types: vec![
                type_row(0x7501, "RepeatedType", MODULE, &[]),
                type_row(0x7502, "RepeatedType", MODULE, &[]),
            ],
            type_ids: vec![id_row(0x0800_7501, 0x7501), id_row(0x0800_7502, 0x7502)],
            ..Tables::default()
        },
    );
    assert_guard_rejects_reference(&base, &type_mini, "two T1 keys share one identity");

    let function_mini = cache(
        &[10],
        Tables {
            funcs: vec![
                func_row(0x7503, "RepeatedFunction", MODULE, 0, &[], 0),
                func_row(0x7504, "RepeatedFunction", MODULE, 0, &[], 0),
            ],
            func_ids: vec![id_row(0x7503, 0x7503), id_row(0x7504, 0x7504)],
            ..Tables::default()
        },
    );
    assert_guard_rejects_reference(&base, &function_mini, "two T3 keys share one identity");
}

#[test]
fn sequential_guard_rejects_distinct_nonstring_t5_keys_with_shared_identity() {
    let base = cache(&[10], Tables::default());
    let mini = cache(
        &[10],
        Tables {
            globals: vec![
                global_row(0x7551, "LegitimateGlobalAlias", MODULE),
                global_row(0x7552, "LegitimateGlobalAlias", MODULE),
            ],
            ..Tables::default()
        },
    );

    assert_guard_rejects_reference(
        &base,
        &mini,
        "two non-string T5 keys share one portable identity",
    );
}

#[test]
fn sequential_guard_distinguishes_t3_const_and_imported_decl_flags() {
    let base = cache(&[10], Tables::default());
    let mini = cache(
        &[10],
        Tables {
            funcs: vec![
                func_row_flags(
                    0x7561,
                    "FlaggedFunction",
                    MODULE,
                    "",
                    false,
                    false,
                    false,
                    0,
                    &[],
                    0,
                ),
                func_row_flags(
                    0x7562,
                    "FlaggedFunction",
                    MODULE,
                    "",
                    true,
                    false,
                    false,
                    0,
                    &[],
                    0,
                ),
            ],
            func_ids: vec![id_row(0x7561, 0x7561), id_row(0x7562, 0x7562)],
            ..Tables::default()
        },
    );
    SequentialMiniGuard::new(&base)
        .unwrap()
        .check_and_record(&mini)
        .expect("const must distinguish T3 identity");
}

#[test]
fn sequential_guard_projects_method_module_and_namespace_out_of_t3_identity() {
    let owner = 0x7580;
    let base_func = 0x7588;
    let mini_func = 0x7590;
    let base = cache(
        &[10],
        Tables {
            types: vec![type_row(owner, "ProjectedOwner", "OwnerModule", &[])],
            type_ids: vec![id_row(0x0800_7580, owner)],
            funcs: vec![func_row_flags(
                base_func,
                "ProjectedMethod",
                "OriginalMethodModule",
                "Original::MethodNamespace",
                false,
                false,
                true,
                owner,
                &[],
                0,
            )],
            func_ids: vec![id_row(0x7588, base_func)],
            ..Tables::default()
        },
    );
    let mini = cache(
        &[10],
        Tables {
            funcs: vec![func_row_flags(
                mini_func,
                "ProjectedMethod",
                "DriftedMethodModule",
                "Drifted::MethodNamespace",
                false,
                false,
                true,
                owner,
                &[],
                0,
            )],
            func_ids: vec![id_row(0x7590, mini_func)],
            ..Tables::default()
        },
    );

    let error = SequentialMiniGuard::new(&base)
        .unwrap()
        .check_and_record(&mini)
        .expect_err("method lookup identity must ignore its own module and namespace drift");
    assert!(matches!(
        error,
        SpliceError::MiniReference(RemapError::InvalidTailRow {
            table: 2,
            kind: "symbol identity",
            ..
        })
    ));
}

#[test]
fn sequential_guard_projects_imported_namespace_out_and_rejects_method_owner_shape() {
    let drift_owner = 0x7598;
    let base_func = 0x75a0;
    let mini_func = 0x75a8;
    let base = cache(
        &[10],
        Tables {
            types: vec![type_row(
                drift_owner,
                "ImportedDriftOwner",
                "OwnerModule",
                &[],
            )],
            type_ids: vec![id_row(0x0800_7598, drift_owner)],
            funcs: vec![func_row_flags(
                base_func,
                "ProjectedImport",
                MODULE,
                "Original::ImportNamespace",
                false,
                true,
                false,
                0,
                &[],
                0,
            )],
            func_ids: vec![id_row(0x75a0, base_func)],
            ..Tables::default()
        },
    );
    let mini = cache(
        &[10],
        Tables {
            funcs: vec![func_row_flags(
                mini_func,
                "ProjectedImport",
                MODULE,
                "Drifted::ImportNamespace",
                false,
                true,
                false,
                0,
                &[],
                0,
            )],
            func_ids: vec![id_row(0x75a8, mini_func)],
            ..Tables::default()
        },
    );

    let error = SequentialMiniGuard::new(&base)
        .unwrap()
        .check_and_record(&mini)
        .expect_err("imported lookup identity must ignore namespace drift");
    assert!(matches!(
        error,
        SpliceError::MiniReference(RemapError::InvalidTailRow {
            table: 2,
            kind: "declaration membership",
            ..
        })
    ));

    for (case, is_method, owner, key) in [
        ("imported method bit", true, 0, 0x75b0),
        ("imported owner pointer", false, drift_owner, 0x75b8),
    ] {
        let malformed = cache(
            &[10],
            Tables {
                funcs: vec![func_row_flags(
                    key,
                    "ProjectedImport",
                    MODULE,
                    "Drifted::ImportNamespace",
                    false,
                    true,
                    is_method,
                    owner,
                    &[],
                    0,
                )],
                func_ids: vec![id_row(key as i32, key)],
                ..Tables::default()
            },
        );
        let error = SequentialMiniGuard::new(&base)
            .unwrap()
            .check_and_record(&malformed)
            .expect_err(case);
        assert!(
            matches!(
                error,
                SpliceError::MiniReference(RemapError::Wire(
                    WireError::InvalidFunctionReference { .. }
                ))
            ),
            "{case}: unexpected error: {error:?}"
        );
    }
}

#[test]
fn sequential_guard_rejects_t3_method_owner_disagreement_both_directions() {
    let owner = 0x7570;
    let base = cache(
        &[10],
        Tables {
            types: vec![type_row(owner, "Owner", MODULE, &[])],
            type_ids: vec![id_row(0x0800_7570, owner)],
            ..Tables::default()
        },
    );
    for (case, is_method, owner_ptr, key) in [
        ("non-method with owner", false, owner, 0x7571),
        ("method without owner", true, 0, 0x7572),
    ] {
        let mini = cache(
            &[10],
            Tables {
                funcs: vec![func_row_flags(
                    key,
                    "InvalidMethodShape",
                    MODULE,
                    "",
                    false,
                    false,
                    is_method,
                    owner_ptr,
                    &[],
                    0,
                )],
                func_ids: vec![id_row(key as i32, key)],
                ..Tables::default()
            },
        );
        assert_guard_rejects_reference(&base, &mini, case);
    }
}

#[test]
fn sequential_guard_rejects_type_and_function_identity_aliases_from_prior_minis() {
    let base = cache(&[10], Tables::default());

    let mut type_guard = SequentialMiniGuard::new(&base).unwrap();
    type_guard
        .check_and_record(&type_symbol_mini(0x7601, 0x0800_7601, "PriorType", &[]))
        .expect("first type identity is unique");
    let type_error = type_guard
        .check_and_record(&type_symbol_mini(0x7602, 0x0800_7602, "PriorType", &[]))
        .unwrap_err();
    assert!(matches!(type_error, SpliceError::MiniReference(_)));

    let mut function_guard = SequentialMiniGuard::new(&base).unwrap();
    function_guard
        .check_and_record(&function_symbol_mini(
            0x7603,
            0x7603,
            "PriorFunction",
            0,
            &[],
        ))
        .expect("first function identity is unique");
    let function_error = function_guard
        .check_and_record(&function_symbol_mini(
            0x7604,
            0x7604,
            "PriorFunction",
            0,
            &[],
        ))
        .unwrap_err();
    assert!(matches!(function_error, SpliceError::MiniReference(_)));
}

#[test]
fn sequential_guard_grandfathers_exact_key_repeats_from_ambiguous_base() {
    let repeated_type = type_row(0x7701, "AmbiguousType", MODULE, &[]);
    let repeated_function = func_row(0x7703, "AmbiguousFunction", MODULE, 0, &[], 0);
    let base = cache(
        &[10],
        Tables {
            types: vec![
                repeated_type.clone(),
                type_row(0x7702, "AmbiguousType", MODULE, &[]),
            ],
            type_ids: vec![id_row(0x0800_7701, 0x7701), id_row(0x0800_7702, 0x7702)],
            funcs: vec![
                repeated_function.clone(),
                func_row(0x7704, "AmbiguousFunction", MODULE, 0, &[], 0),
            ],
            func_ids: vec![id_row(0x7703, 0x7703), id_row(0x7704, 0x7704)],
            ..Tables::default()
        },
    );
    let mini = cache(
        &[10],
        Tables {
            types: vec![repeated_type],
            type_ids: vec![id_row(0x0800_7701, 0x7701)],
            funcs: vec![repeated_function],
            func_ids: vec![id_row(0x7703, 0x7703)],
            ..Tables::default()
        },
    );

    let prepared = SequentialMiniGuard::new(&base)
        .unwrap()
        .check_and_record(&mini)
        .expect("an exact known-key repeat must not reinterpret ambiguous base identity");
    assert_eq!(prepared, mini);
}

#[test]
fn sequential_guard_grandfathers_exact_id_repeats_from_aliased_base_maps() {
    let type_ptr = 0x7751;
    let function_ptr = 0x7752;
    let base = cache(
        &[10],
        Tables {
            types: vec![type_row(type_ptr, "HistoricallyAliasedType", MODULE, &[])],
            type_ids: vec![id_row(0x0800_7751, type_ptr), id_row(0x0800_7752, type_ptr)],
            funcs: vec![func_row(
                function_ptr,
                "HistoricallyAliasedFunction",
                MODULE,
                0,
                &[],
                0,
            )],
            func_ids: vec![id_row(0x7751, function_ptr), id_row(0x7752, function_ptr)],
            ..Tables::default()
        },
    );
    let exact_rows_only = cache(
        &[10],
        Tables {
            type_ids: vec![id_row(0x0800_7751, type_ptr)],
            func_ids: vec![id_row(0x7751, function_ptr)],
            ..Tables::default()
        },
    );

    SequentialMiniGuard::new(&base)
        .unwrap()
        .check_and_record(&exact_rows_only)
        .expect("exact existing T2/T4 id rows must grandfather historical base pointer aliases");

    for (table, new_id, mini, witness) in [
        (
            1,
            0x0800_7753,
            cache(
                &[10],
                Tables {
                    type_ids: vec![id_row(0x0800_7753, type_ptr)],
                    ..Tables::default()
                },
            ),
            "0x8007751",
        ),
        (
            3,
            0x7753,
            cache(
                &[10],
                Tables {
                    func_ids: vec![id_row(0x7753, function_ptr)],
                    ..Tables::default()
                },
            ),
            "0x7751",
        ),
    ] {
        let error = SequentialMiniGuard::new(&base)
            .unwrap()
            .check_and_record(&mini)
            .unwrap_err();
        assert!(
            matches!(
                error,
                SpliceError::MiniReference(RemapError::InvalidTailRow {
                    table: actual_table,
                    row_key,
                    kind: "pointer-to-id mapping",
                    ref detail,
                }) if actual_table == table && row_key == i64::from(new_id) && detail.contains(witness)
            ),
            "table {table} did not report deterministic lowest prior id: {error:?}"
        );
    }
}

#[test]
fn sequential_guard_identity_state_is_atomic_when_static_rebase_fails() {
    let base = cache(&[10], Tables::default());
    let bad = cache(
        &[(60u32 | (9 << 16)) as i32, 10],
        Tables {
            types: vec![type_row(0x7801, "RetryIdentity", MODULE, &[])],
            type_ids: vec![id_row(0x0800_7801, 0x7801)],
            static_names: vec![sia("RetryStatic")],
            ..Tables::default()
        },
    );
    let mut guard = SequentialMiniGuard::new(&base).unwrap();
    let error = guard.check_and_record(&bad).unwrap_err();
    assert!(matches!(error, SpliceError::StaticNameRebase(_)));

    let corrected = cache(
        &[60, 10],
        Tables {
            types: vec![type_row(0x7802, "RetryIdentity", MODULE, &[])],
            type_ids: vec![id_row(0x0800_7802, 0x7802)],
            static_names: vec![sia("RetryStatic")],
            ..Tables::default()
        },
    );
    guard
        .check_and_record(&corrected)
        .expect("failed rebase must not record the rejected identity under its first key");
}

#[test]
fn sequential_guard_rejects_prior_mini_type_and_function_id_pointer_aliases() {
    let base = cache(&[10], Tables::default());

    let type_row_bytes = type_row(0x7901, "PriorMappedType", MODULE, &[]);
    let mut type_guard = SequentialMiniGuard::new(&base).unwrap();
    type_guard
        .check_and_record(&cache(
            &[10],
            Tables {
                types: vec![type_row_bytes.clone()],
                type_ids: vec![id_row(0x0800_7901, 0x7901)],
                ..Tables::default()
            },
        ))
        .expect("first T2 mapping is unique");
    let type_alias = cache(
        &[10],
        Tables {
            types: vec![type_row_bytes],
            type_ids: vec![id_row(0x0800_7902, 0x7901)],
            ..Tables::default()
        },
    );
    let type_error = type_guard.check_and_record(&type_alias).unwrap_err();
    assert!(matches!(type_error, SpliceError::MiniReference(_)));

    let function_row_bytes = func_row(0x7903, "PriorMappedFunction", MODULE, 0, &[], 0);
    let mut function_guard = SequentialMiniGuard::new(&base).unwrap();
    function_guard
        .check_and_record(&cache(
            &[10],
            Tables {
                funcs: vec![function_row_bytes.clone()],
                func_ids: vec![id_row(0x7903, 0x7903)],
                ..Tables::default()
            },
        ))
        .expect("first T4 mapping is unique");
    let function_alias = cache(
        &[10],
        Tables {
            funcs: vec![function_row_bytes],
            func_ids: vec![id_row(0x7904, 0x7903)],
            ..Tables::default()
        },
    );
    let function_error = function_guard
        .check_and_record(&function_alias)
        .unwrap_err();
    assert!(matches!(function_error, SpliceError::MiniReference(_)));
}

#[test]
fn sequential_guard_uses_base_only_type_dependencies_for_alias_identity() {
    let dependency_ptr = 0x7a01;
    let base = cache(
        &[10],
        Tables {
            types: vec![
                type_row(dependency_ptr, "BaseDependency", MODULE, &[]),
                type_row(0x7a02, "DependentType", MODULE, &[dependency_ptr]),
            ],
            type_ids: vec![
                id_row(0x0800_7a01, dependency_ptr),
                id_row(0x0800_7a02, 0x7a02),
            ],
            funcs: vec![func_row(
                0x7a03,
                "DependentFunction",
                MODULE,
                0,
                &[dependency_ptr],
                0,
            )],
            func_ids: vec![id_row(0x7a03, 0x7a03)],
            ..Tables::default()
        },
    );

    let type_alias = type_symbol_mini(0x7a04, 0x0800_7a04, "DependentType", &[dependency_ptr]);
    assert_guard_rejects_reference(
        &base,
        &type_alias,
        "T1 identity must resolve an omitted base-only subtype",
    );

    let function_alias =
        function_symbol_mini(0x7a05, 0x7a05, "DependentFunction", 0, &[dependency_ptr]);
    assert_guard_rejects_reference(
        &base,
        &function_alias,
        "T3 identity must resolve an omitted base-only parameter type",
    );
}

#[test]
fn sequential_guard_distinguishes_recursively_nested_type_identities() {
    let base_foo = 0x7b01;
    let base_inner = 0x7b02;
    let base_outer = 0x7b03;
    let base = cache(
        &[10],
        Tables {
            types: vec![
                type_row_ns(base_foo, "Leaf", MODULE, "Foo", &[]),
                type_row_ns(base_inner, "Inner", "Templates", "", &[base_foo]),
                type_row_ns(base_outer, "Outer", "Templates", "", &[base_inner]),
            ],
            type_ids: vec![
                id_row(0x0800_7b01, base_foo),
                id_row(0x0800_7b02, base_inner),
                id_row(0x0800_7b03, base_outer),
            ],
            ..Tables::default()
        },
    );

    let mini_bar = 0x7b11;
    let mini_inner = 0x7b12;
    let mini_outer = 0x7b13;
    let mini = cache(
        &[10],
        Tables {
            types: vec![
                type_row_ns(mini_bar, "Leaf", MODULE, "Bar", &[]),
                type_row_ns(mini_inner, "Inner", "Templates", "", &[mini_bar]),
                type_row_ns(mini_outer, "Outer", "Templates", "", &[mini_inner]),
            ],
            type_ids: vec![
                id_row(0x0800_7b11, mini_bar),
                id_row(0x0800_7b12, mini_inner),
                id_row(0x0800_7b13, mini_outer),
            ],
            ..Tables::default()
        },
    );

    SequentialMiniGuard::new(&base)
        .unwrap()
        .check_and_record(&mini)
        .expect("Outer<Inner<Bar::Leaf>> must differ from Outer<Inner<Foo::Leaf>>");
}

#[test]
fn sequential_guard_projects_template_instance_module_out_of_t1_identity() {
    let leaf = 0x7bb0;
    let base_template = 0x7bb8;
    let mini_template = 0x7bc0;
    let base = cache(
        &[10],
        Tables {
            types: vec![
                type_row_ns(leaf, "TemplateLeaf", "LeafModule", "N", &[]),
                type_row_ns(
                    base_template,
                    "ProjectedTemplate",
                    "OriginalTemplateModule",
                    "N",
                    &[leaf],
                ),
            ],
            type_ids: vec![
                id_row(0x0800_7bb0, leaf),
                id_row(0x0800_7bb8, base_template),
            ],
            ..Tables::default()
        },
    );
    let mini = cache(
        &[10],
        Tables {
            types: vec![type_row_ns(
                mini_template,
                "ProjectedTemplate",
                "DriftedTemplateModule",
                "N",
                &[leaf],
            )],
            type_ids: vec![id_row(0x0800_7bc0, mini_template)],
            ..Tables::default()
        },
    );

    let error = SequentialMiniGuard::new(&base)
        .unwrap()
        .check_and_record(&mini)
        .expect_err("template-instance lookup ignores the declaring module");
    assert!(matches!(
        error,
        SpliceError::MiniReference(RemapError::InvalidTailRow {
            table: 0,
            kind: "symbol identity",
            ..
        })
    ));
}

#[test]
fn sequential_guard_rejects_noncanonical_template_sentinel_rows() {
    let leaf = 0x7bd0;
    for (case, key, namespace, subtypes) in [
        ("missing sentinel namespace", 0x7bd8, "", vec![]),
        (
            "sentinel carrying template arguments",
            0x7be0,
            "SentinelNamespace",
            vec![leaf],
        ),
    ] {
        let mut types = vec![type_row_ns(key, "Sentinel", "$__T__", namespace, &subtypes)];
        let mut type_ids = vec![id_row(0x0800_7bd8 + ((key - 0x7bd8) / 8) as i32, key)];
        if !subtypes.is_empty() {
            types.insert(0, type_row(leaf, "SentinelLeaf", "LeafModule", &[]));
            type_ids.insert(0, id_row(0x0800_7bd0, leaf));
        }
        let mini = cache(
            &[10],
            Tables {
                types,
                type_ids,
                ..Tables::default()
            },
        );
        let error = SequentialMiniGuard::new(&cache(&[10], Tables::default()))
            .unwrap()
            .check_and_record(&mini)
            .expect_err(case);
        assert!(
            matches!(
                error,
                SpliceError::MiniReference(RemapError::Wire(
                    WireError::InvalidTypeReference { .. }
                ))
            ),
            "{case}: unexpected error: {error:?}"
        );
    }
}

#[test]
fn sequential_guard_recursively_resolves_base_only_nested_type_fallback() {
    let leaf = 0x7c01;
    let inner = 0x7c02;
    let outer = 0x7c03;
    let base = cache(
        &[10],
        Tables {
            types: vec![
                type_row_ns(leaf, "Leaf", "FooModule", "Foo", &[]),
                type_row_ns(inner, "Inner", "Templates", "", &[leaf]),
                type_row_ns(outer, "Outer", "Templates", "", &[inner]),
            ],
            type_ids: vec![
                id_row(0x0800_7c01, leaf),
                id_row(0x0800_7c02, inner),
                id_row(0x0800_7c03, outer),
            ],
            ..Tables::default()
        },
    );
    let alias = cache(
        &[10],
        Tables {
            types: vec![type_row_ns(0x7c04, "Outer", "Templates", "", &[inner])],
            type_ids: vec![id_row(0x0800_7c04, 0x7c04)],
            ..Tables::default()
        },
    );
    assert_guard_rejects_reference(
        &base,
        &alias,
        "base-only Inner<Foo::Leaf> must contribute its recursive identity",
    );
}

#[test]
fn sequential_guard_rejects_cyclic_recursive_type_identity() {
    let key = 0x7d01;
    let mini = type_symbol_mini(key, 0x0800_7d01, "SelfCycle", &[key]);
    let error = SequentialMiniGuard::new(&cache(&[10], Tables::default()))
        .unwrap()
        .check_and_record(&mini)
        .unwrap_err();
    assert!(matches!(
        error,
        SpliceError::MiniReference(RemapError::Wire(WireError::CyclicTypeReference {
            key: actual,
        })) if actual == key
    ));
}

fn amplified_type_identity_tables(depth: usize, shared_parents: usize) -> Tables {
    let mut types = Vec::new();
    let mut type_ids = Vec::new();
    let first_key = 0x7e00i64;
    types.push(type_row_ns(first_key, "Leaf", "Amplifier", "N", &[]));
    type_ids.push(id_row(0x0800_7e00, first_key));
    let mut child = first_key;
    for level in 1..=depth {
        let key = first_key + level as i64;
        types.push(type_row_ns(
            key,
            &format!("Pair{level}"),
            "Amplifier",
            "N",
            &[child, child],
        ));
        type_ids.push(id_row(0x0800_7e00 + level as i32, key));
        child = key;
    }
    for parent in 0..shared_parents {
        let ordinal = depth + 1 + parent;
        let key = first_key + ordinal as i64;
        types.push(type_row_ns(
            key,
            &format!("SharedParent{parent}"),
            "Amplifier",
            "N",
            &[child],
        ));
        type_ids.push(id_row(0x0800_7e00 + ordinal as i32, key));
    }
    Tables {
        types,
        type_ids,
        ..Tables::default()
    }
}

#[test]
fn sequential_guard_rejects_per_identity_dag_amplification() {
    let mut tables = amplified_type_identity_tables(13, 0);
    // Keep the aggregate budget above the sum of the still-legal ancestor identities so this
    // fixture isolates the per-identity cap reached by the exponentially amplified final row.
    tables.static_names.push(sia(&"P".repeat(65_536)));
    let mini = cache(&[10], tables);
    let error = SequentialMiniGuard::new(&cache(&[10], Tables::default()))
        .unwrap()
        .check_and_record(&mini)
        .unwrap_err();
    assert!(
        matches!(
            error,
            SpliceError::MiniReference(RemapError::Wire(WireError::IdentityTooLarge {
                max: 65_536,
                ..
            }))
        ),
        "unexpected amplification refusal: {error:?}"
    );
}

#[test]
fn sequential_guard_rejects_aggregate_shared_child_identity_amplification() {
    let mini = cache(&[10], amplified_type_identity_tables(7, 12));
    let error = SequentialMiniGuard::new(&cache(&[10], Tables::default()))
        .unwrap()
        .check_and_record(&mini)
        .unwrap_err();
    assert!(
        matches!(
            error,
            SpliceError::MiniReference(RemapError::Wire(WireError::IdentityBudgetExceeded {
                max: 65_536
            }))
        ),
        "unexpected aggregate amplification refusal: {error:?}"
    );
}

#[test]
fn sequential_guard_enforces_identity_budget_across_split_t1_and_t3_minis() {
    const DEPTH: usize = 5;
    const MAX_MINIS: usize = 32;
    let child = 0x7e00 + DEPTH as i64;
    let mut base_tables = amplified_type_identity_tables(DEPTH, 0);
    for ordinal in 0..MAX_MINIS {
        let key = 0x8f00 + ordinal as i64 * 8;
        base_tables.types.push(type_row_ns(
            key,
            &format!("SplitBudgetType{ordinal}"),
            "PristineIgnoredModule",
            "N",
            &[0x7e00],
        ));
        base_tables
            .type_ids
            .push(id_row(0x1001_0000 + ordinal as i32, key));
    }
    let base = cache(&[10], base_tables);

    for domain in ["T1", "T3"] {
        let mut guard = SequentialMiniGuard::new(&base).unwrap();
        let mut accepted = 0usize;
        let mut refused = None;
        for ordinal in 0..MAX_MINIS {
            let key = 0x7f00 + ordinal as i64 * 8;
            let mini = if domain == "T1" {
                cache(
                    &[10],
                    Tables {
                        types: vec![type_row_ns(
                            key,
                            &format!("SplitBudgetType{ordinal}"),
                            MODULE,
                            "N",
                            &[child],
                        )],
                        type_ids: vec![id_row(0x1002_0000 + ordinal as i32, key)],
                        ..Tables::default()
                    },
                )
            } else {
                cache(
                    &[10],
                    Tables {
                        funcs: vec![func_row(
                            key,
                            &format!("SplitBudgetFunction{ordinal}"),
                            MODULE,
                            0,
                            &[child],
                            0,
                        )],
                        func_ids: vec![id_row(0x0001_0000 + ordinal as i32, key)],
                        ..Tables::default()
                    },
                )
            };
            match guard.check_and_record(&mini) {
                Ok(_) => accepted += 1,
                Err(error) => {
                    assert!(
                        matches!(
                            error,
                            SpliceError::MiniReference(RemapError::Wire(
                                WireError::IdentityBudgetExceeded { .. }
                            ))
                        ),
                        "{domain} split budget failed for another reason: {error:?}"
                    );
                    refused = Some(ordinal);
                    break;
                }
            }
        }
        assert!(
            accepted >= 2,
            "{domain} fixture crossed before proving split state"
        );
        assert!(
            refused.is_some(),
            "{domain} accepted all {MAX_MINIS} amplified minis by resetting the budget"
        );
    }
}

#[test]
fn sequential_guard_bounds_namespace_tolerant_t1_identity_comparisons() {
    let mut types = Vec::new();
    let mut type_ids = Vec::new();
    for ordinal in 0..160i32 {
        let key = 0x8800 + i64::from(ordinal) * 8;
        types.push(type_row_ns(
            key,
            "ComparisonBudgetType",
            MODULE,
            &format!("DistinctNamespace{ordinal:03}"),
            &[],
        ));
        type_ids.push(id_row(0x0802_0000 + ordinal, key));
    }
    let mini = cache(
        &[10],
        Tables {
            types,
            type_ids,
            ..Tables::default()
        },
    );

    let error = SequentialMiniGuard::new(&cache(&[10], Tables::default()))
        .unwrap()
        .check_and_record(&mini)
        .expect_err("pairwise-distinct namespaces must not permit quadratic T1 matching work");
    assert!(matches!(
        error,
        SpliceError::MiniReference(RemapError::Wire(
            WireError::IdentityComparisonBudgetExceeded { .. }
        ))
    ));
}

#[test]
fn sequential_guard_bounds_namespace_tolerant_t3_identity_comparisons() {
    let mut funcs = Vec::new();
    let mut func_ids = Vec::new();
    for ordinal in 0..160i32 {
        let key = 0x9800 + i64::from(ordinal) * 8;
        funcs.push(func_row_ns(
            key,
            "ComparisonBudgetFunction",
            MODULE,
            &format!("DistinctNamespace{ordinal:03}"),
            0,
            &[],
            0,
        ));
        func_ids.push(id_row(0x0002_0000 + ordinal, key));
    }
    let mini = cache(
        &[10],
        Tables {
            funcs,
            func_ids,
            ..Tables::default()
        },
    );

    let error = SequentialMiniGuard::new(&cache(&[10], Tables::default()))
        .unwrap()
        .check_and_record(&mini)
        .expect_err("pairwise-distinct namespaces must not permit quadratic T3 matching work");
    assert!(matches!(
        error,
        SpliceError::MiniReference(RemapError::Wire(
            WireError::IdentityComparisonBudgetExceeded { .. }
        ))
    ));
}

#[test]
fn allow_new_indexes_a_large_target_type_frontier_instead_of_rescanning_rows() {
    const ROWS: i32 = 12_000;
    let mut types = Vec::with_capacity(ROWS as usize);
    let mut type_ids = Vec::with_capacity(ROWS as usize);
    let mut classes = Vec::with_capacity(ROWS as usize);
    for ordinal in 0..ROWS {
        let ptr = 0x3000_0000 + i64::from(ordinal) * 8;
        let id = 0x0804_0000 + ordinal;
        let name = format!("PlannerType{ordinal:05}");
        types.push(type_row(ptr, &name, MODULE, &[]));
        type_ids.push(id_row(id, ptr));
        classes.push(structural_class_record_full_named(
            &name,
            "",
            &[],
            &[],
            &[],
            &[],
            &[],
            &[0; 7],
            &[],
            &[],
            None,
        ));
    }
    let regen = cache_with_module_key_name_and_records(
        "PlannerOuter",
        MODULE,
        &[function(&[10], 0x0500_0200)],
        &classes,
        &[],
        &[],
        Tables {
            types,
            type_ids,
            ..Tables::default()
        },
    );
    let base = cache_with_named_module("PlannerBase", &[10], Tables::default());

    let started = std::time::Instant::now();
    let (mini, _) = remap_module_to_base_with_options(
        &regen,
        &base,
        RemapOptions {
            allow_new_symbols: true,
        },
    )
    .expect("the indexed frontier admits every independently declared target type");
    assert!(
        started.elapsed() < std::time::Duration::from_secs(15),
        "12k target rows must not trigger quadratic dependency-row scans"
    );
    let tail = parse_tail_tables(&mini, module_region_end(&mini).unwrap()).unwrap();
    assert_eq!(tail.tables[0].count, ROWS as u32);
    assert_eq!(tail.tables[1].count, ROWS as u32);
}

#[test]
fn sequential_guard_indexes_many_property_and_native_alloc_sites_once() {
    const ROWS: i32 = 8_192;
    const OPS: usize = 16_384;
    const OWNER_PTR: i64 = 0x4100_0000;
    const OWNER_ID: i32 = 0x0805_0000;
    let mut base_types = Vec::with_capacity(ROWS as usize);
    let mut base_type_ids = Vec::with_capacity(ROWS as usize);
    for ordinal in 0..ROWS {
        let ptr = 0x4000_0000 + i64::from(ordinal) * 8;
        let id = 0x0404_0000 + ordinal;
        base_types.push(type_row(ptr, &format!("NativeType{ordinal:05}"), "", &[]));
        base_type_ids.push(id_row(id, ptr));
    }
    let native_ptr = 0x4000_0000 + i64::from(ROWS - 1) * 8;
    // Keep the source-proportional identity budget above this deliberately row-dense synthetic
    // base. Shipping has ample module bytes; this fixture otherwise unrealistically consists
    // almost entirely of tail identities.
    let base_code = vec![10; 500_000];
    let base = cache_with_named_module(
        "IndexedBase",
        &base_code,
        Tables {
            types: base_types,
            type_ids: base_type_ids,
            ..Tables::default()
        },
    );
    let mut properties = Vec::with_capacity(ROWS as usize);
    for ordinal in 0..ROWS {
        properties.push(property_row(
            OWNER_ID,
            ordinal * 4,
            &format!("Field{ordinal:05}"),
        ));
    }
    let member_offset = (ROWS - 1) * 4;
    let mut code = Vec::with_capacity(OPS * 6 + 1);
    for _ in 0..OPS {
        code.extend([79 | (member_offset << 16), OWNER_ID]);
        qw_op(64, native_ptr, &mut code);
        code.push(0);
    }
    code.push(10);
    let mini = cache(
        &code,
        Tables {
            types: vec![type_row(OWNER_PTR, "IndexedOwner", MODULE, &[])],
            type_ids: vec![id_row(OWNER_ID, OWNER_PTR)],
            properties,
            ..Tables::default()
        },
    );
    let mut guard = SequentialMiniGuard::new(&base).unwrap();

    let started = std::time::Instant::now();
    guard
        .check_and_record(&mini)
        .expect("indexed T7 membership and one native T2 summary validate all repeated sites");
    assert!(
        started.elapsed() < std::time::Duration::from_secs(15),
        "repeated property/ALLOC0 sites must not rescan T7/T2 or allocate per instruction"
    );
}

#[test]
fn sequential_guard_bounds_late_namespace_identity_comparison_work() {
    const ROWS: i32 = 48;
    const COMMON_PREFIX_BYTES: usize = 12 * 1024;
    let common = "N".repeat(COMMON_PREFIX_BYTES);
    let mut types = Vec::new();
    let mut type_ids = Vec::new();
    for ordinal in 0..ROWS {
        let key = 0xa800 + i64::from(ordinal) * 8;
        types.push(type_row_ns(
            key,
            "LateNamespaceWorkType",
            MODULE,
            &format!("{common}{ordinal:04}"),
            &[],
        ));
        type_ids.push(id_row(0x0803_0000 + ordinal, key));
    }
    let mini = cache(
        &[10],
        Tables {
            types,
            type_ids,
            ..Tables::default()
        },
    );

    let error = SequentialMiniGuard::new(&cache(&[10], Tables::default()))
        .unwrap()
        .check_and_record(&mini)
        .expect_err(
            "a below-count-budget bucket with long late-differing namespaces must have a byte-work budget",
        );
    assert!(
        matches!(
            error,
            SpliceError::MiniReference(RemapError::Wire(
                WireError::IdentityComparisonBudgetExceeded { .. }
            ))
        ),
        "unexpected identity comparison work refusal: {error:?}"
    );
}

fn composition_base(function_id: i32) -> Vec<u8> {
    let mut base = cache_with_function_id(&[10], Tables::default(), function_id);
    replace_ascii_same_len(&mut base, "EditedModule", "PristineBase");
    base
}

fn assert_composed_structure_error(mini: &[u8], field: &'static str) {
    let error = splice_auto(&composition_base(0x0100_0001), mini).unwrap_err();
    assert!(
        matches!(
            error,
            SpliceError::ComposedModule(RemapError::InvalidModuleStructure {
                field: actual,
                ..
            }) if actual == field
        ),
        "expected {field} structural refusal, got {error:?}"
    );
}

fn function_id_cache(ids: &[i32]) -> Vec<u8> {
    let functions = ids
        .iter()
        .map(|&id| function(&[10], id))
        .collect::<Vec<_>>();
    cache_with_records(&functions, &[], &[], &[])
}

#[test]
fn composed_cache_accepts_one_zero_but_rejects_duplicate_function_ids() {
    let with_zero = splice_auto(&composition_base(0x0100_0001), &function_id_cache(&[0]))
        .expect("Function.Id zero is a legal cache key when it remains unique");
    assert_eq!(module_count(&with_zero), 2);

    let duplicate_zero_error =
        splice_auto(&composition_base(0x0100_0001), &function_id_cache(&[0, 0])).unwrap_err();
    assert!(
        matches!(
            duplicate_zero_error,
            SpliceError::ComposedModule(RemapError::Wire(WireError::BadLen {
                field: "duplicate function declaration",
                ..
            }))
        ),
        "unexpected duplicate-zero refusal: {duplicate_zero_error:?}"
    );

    let duplicate = 0x0200_0001;
    let duplicate_error = splice_auto(
        &composition_base(0x0100_0001),
        &function_id_cache(&[duplicate, duplicate]),
    )
    .unwrap_err();
    assert!(
        matches!(
            duplicate_error,
            SpliceError::ComposedModule(RemapError::Wire(WireError::BadLen {
                field: "duplicate function declaration",
                ..
            }))
        ),
        "unexpected duplicate refusal: {duplicate_error:?}"
    );
}

#[test]
fn composed_cache_rejects_function_id_collision_across_added_modules() {
    let duplicate = 0x0200_0010;
    let error = splice_auto(
        &composition_base(duplicate),
        &function_id_cache(&[duplicate]),
    )
    .unwrap_err();
    assert!(matches!(
        error,
        SpliceError::ComposedModule(RemapError::FunctionIdCollision { id, .. })
            if id == duplicate
    ));
}

#[test]
fn composed_cache_accepts_unique_negative_function_id() {
    let combined = splice_auto(&composition_base(0x0200_0020), &function_id_cache(&[-7]))
        .expect("negative nonzero Function.Id values are valid runtime lookup keys");
    assert_eq!(module_count(&combined), 2);
}

#[test]
fn replace_may_reuse_target_function_id_but_not_an_untouched_modules_id() {
    let target_id = 0x0200_0030;
    let untouched_id = 0x0200_0031;
    let target = cache_with_function_id(&[10], Tables::default(), target_id);
    let mut untouched = cache_with_function_id(&[10], Tables::default(), untouched_id);
    replace_ascii_same_len(&mut untouched, "EditedModule", "SecondModule");
    let base = splice_auto(&target, &untouched).expect("two unique source modules");

    let replacement = cache_with_function_id(&[10], Tables::default(), target_id);
    replace_module(&base, &replacement, MODULE)
        .expect("replacement may retain the Function.Id owned by the removed target");

    let colliding = cache_with_function_id(&[10], Tables::default(), untouched_id);
    let error = replace_module(&base, &colliding, MODULE).unwrap_err();
    assert!(matches!(
        error,
        SpliceError::ComposedModule(RemapError::FunctionIdCollision { id, .. })
            if id == untouched_id
    ));
}

#[test]
fn composed_cache_scans_function_ids_at_every_runtime_record_location() {
    let seven_refs = [0i64; 7];
    let unique = structural_class_record_full(
        &[],
        &[function(&[10], 0x0250_0002)],
        &[0],
        &[function(&[10], 0x0250_0003)],
        &[],
        &seven_refs,
        &[function(&[10], 0x0250_0004)],
        &[2],
        None,
    );
    let unique_mini = cache_with_all_records(
        &[function(&[10], 0x0250_0001)],
        &[unique],
        &[],
        &[global_init_record(&function(&[10], 0x0250_0005))],
        &[],
    );
    splice_auto(&composition_base(0x0250_0000), &unique_mini)
        .expect("unique ids across module/method/ctor/behavior/global-init records");

    for location in ["method", "constructor", "behavior", "global init"] {
        let duplicate = 0x0250_0010;
        let method_id = if location == "method" {
            duplicate
        } else {
            0x0250_0012
        };
        let constructor_id = if location == "constructor" {
            duplicate
        } else {
            0x0250_0013
        };
        let behavior_id = if location == "behavior" {
            duplicate
        } else {
            0x0250_0014
        };
        let global_id = if location == "global init" {
            duplicate
        } else {
            0x0250_0015
        };
        let class = structural_class_record_full(
            &[],
            &[function(&[10], method_id)],
            &[0],
            &[function(&[10], constructor_id)],
            &[],
            &seven_refs,
            &[function(&[10], behavior_id)],
            &[2],
            None,
        );
        let mini = cache_with_all_records(
            &[function(&[10], duplicate)],
            &[class],
            &[],
            &[global_init_record(&function(&[10], global_id))],
            &[],
        );
        let error = splice_auto(&composition_base(0x0250_0000), &mini).unwrap_err();
        assert!(
            matches!(
                error,
                SpliceError::ComposedModule(RemapError::FunctionIdCollision { id, .. })
                    if id == duplicate
            ),
            "{location} Function.Id was not scanned: {error:?}"
        );
    }
}

#[test]
fn loadout_rekeys_function_ids_at_every_runtime_record_location() {
    let seven_refs = [0i64; 7];
    let make_mini = |outer: &str, inner: &str| {
        let class = structural_class_record_full(
            &[],
            &[function(&[10], 0x0260_0002)],
            &[0],
            &[function(&[10], 0x0260_0003)],
            &[],
            &seven_refs,
            &[function(&[10], 0x0260_0004)],
            &[2],
            None,
        );
        cache_with_module_key_name_and_records(
            outer,
            inner,
            &[function(&[10], 0x0260_0001)],
            &[class],
            &[],
            &[global_init_record(&function(&[10], 0x0260_0005))],
            Tables::default(),
        )
    };
    let base = composition_base(0x0260_0000);
    let first = make_mini("AllSitesA", "AllSitesA");
    let second = make_mini("AllSitesB", "AllSitesB");

    let mut builder = LoadoutScriptIdPlanBuilder::new(&base).unwrap();
    builder.inspect(&first).unwrap();
    builder.inspect(&second).unwrap();
    let plan = builder.finish().unwrap();
    let first = remap_module_to_base_with_loadout_plan(&first, &base, &plan).unwrap();
    let second = remap_module_to_base_with_loadout_plan(&second, &base, &plan).unwrap();

    let mut guard = SequentialMiniGuard::new(&base).unwrap();
    let running = guard.compose_add(&base, &first).unwrap();
    guard.compose_add(&running, &second).expect(
        "free, method, constructor, behavior, and global-init Function.Id sites must all rekey",
    );
}

#[test]
fn composed_cache_rejects_all_paired_count_mismatches() {
    let seven_refs = [0i64; 7];
    let parameter_name = cache_with_records(
        &[shaped_function(
            0x0300_0001,
            FunctionShape {
                parameter_types: 1,
                ..FunctionShape::default()
            },
        )],
        &[],
        &[],
        &[],
    );
    let parameter_flag = cache_with_records(
        &[shaped_function(
            0x0300_0002,
            FunctionShape {
                parameter_types: 1,
                parameter_names: 1,
                parameter_defaults: 1,
                ..FunctionShape::default()
            },
        )],
        &[],
        &[],
        &[],
    );
    let parameter_default = cache_with_records(
        &[shaped_function(
            0x0300_0003,
            FunctionShape {
                parameter_types: 1,
                parameter_names: 1,
                parameter_flags: 1,
                ..FunctionShape::default()
            },
        )],
        &[],
        &[],
        &[],
    );
    let object_variables = cache_with_records(
        &[shaped_function(
            0x0300_0004,
            FunctionShape {
                object_types: 1,
                ..FunctionShape::default()
            },
        )],
        &[],
        &[],
        &[],
    );
    let variable_offset = cache_with_records(
        &[shaped_function(
            0x0300_0005,
            FunctionShape {
                variable_program_positions: 1,
                variable_options: 1,
                ..FunctionShape::default()
            },
        )],
        &[],
        &[],
        &[],
    );
    let variable_option = cache_with_records(
        &[shaped_function(
            0x0300_0006,
            FunctionShape {
                variable_program_positions: 1,
                variable_offsets: 1,
                ..FunctionShape::default()
            },
        )],
        &[],
        &[],
        &[],
    );
    let function_metadata = cache_with_records(
        &[shaped_function(
            0x0300_0007,
            FunctionShape {
                unreal_metadata: Some((1, 0)),
                ..FunctionShape::default()
            },
        )],
        &[],
        &[],
        &[],
    );
    let behavior_function = shaped_function(0x0300_0009, FunctionShape::default());
    let class_behaviors =
        structural_class_record(&[], &[], &[], &seven_refs, &[behavior_function], &[], None);
    let class_behavior_counts = cache_with_records(
        &[function(&[10], 0x0300_0008)],
        &[class_behaviors],
        &[],
        &[],
    );
    let class_metadata =
        structural_class_record(&[], &[], &[], &seven_refs, &[], &[], Some((1, 0)));
    let class_metadata_counts =
        cache_with_records(&[function(&[10], 0x0300_000a)], &[class_metadata], &[], &[]);
    let property_metadata = structural_class_record(
        &[property_with_metadata(1, 0)],
        &[],
        &[],
        &seven_refs,
        &[],
        &[],
        None,
    );
    let property_metadata_counts = cache_with_records(
        &[function(&[10], 0x0300_000b)],
        &[property_metadata],
        &[],
        &[],
    );
    let enum_counts = cache_with_records(
        &[function(&[10], 0x0300_000c)],
        &[],
        &[enum_record(1, 0)],
        &[],
    );
    let import_flags = cache_with_records(
        &[function(&[10], 0x0300_000d)],
        &[],
        &[],
        &[import_record(1, 0, 1)],
    );
    let import_defaults = cache_with_records(
        &[function(&[10], 0x0300_000e)],
        &[],
        &[],
        &[import_record(1, 1, 0)],
    );

    let cases = [
        ("Function.Parameters", parameter_name),
        ("Function.Parameters", parameter_flag),
        ("Function.Parameters", parameter_default),
        ("Function.ObjectVariables", object_variables),
        ("Function.VariableInfo", variable_offset),
        ("Function.VariableInfo", variable_option),
        ("UFunction.Metadata", function_metadata),
        ("Class.Behaviors", class_behavior_counts),
        ("Class.Metadata", class_metadata_counts),
        ("UProperty.Metadata", property_metadata_counts),
        ("Enum.Entries", enum_counts),
        ("Import.Parameters", import_flags),
        ("Import.Parameters", import_defaults),
    ];
    for (field, mini) in cases {
        assert_composed_structure_error(&mini, field);
    }
}

#[test]
fn composed_module_parser_rejects_oversized_local_arrays_without_allocating_them() {
    const HUGE_COUNT: i32 = 50_000_000;
    let cases = [
        (
            "ObjVariablePos",
            0x0330_0010,
            FunctionShape {
                serialized_object_positions: Some(HUGE_COUNT),
                ..FunctionShape::default()
            },
        ),
        (
            "VariableInfoProgramPos",
            0x0330_0011,
            FunctionShape {
                serialized_variable_program_positions: Some(HUGE_COUNT),
                ..FunctionShape::default()
            },
        ),
    ];

    for (field, function_id, shape) in cases {
        let mini = cache_with_records(&[shaped_function(function_id, shape)], &[], &[], &[]);
        let error = match splice_auto(&composition_base(0x0330_0001), &mini) {
            Ok(_) => panic!("oversized {field} unexpectedly parsed"),
            Err(error) => error,
        };
        assert!(
            matches!(
                error,
                SpliceError::Wire(WireError::Eof { .. })
                    | SpliceError::Wire(WireError::BadLen { .. })
                    | SpliceError::ComposedModule(RemapError::Wire(WireError::Eof { .. }))
                    | SpliceError::ComposedModule(RemapError::Wire(WireError::BadLen { .. }))
            ),
            "oversized {field}: unexpected error: {error:?}"
        );
    }
}

#[test]
fn sequential_guard_preflights_tail_count_caps_and_records_failures_atomically() {
    const KEYED_LIMIT: u64 = 131_072;
    const STATIC_LIMIT: u64 = 65_536;
    const MINI_LIMIT: u64 = 256;
    let raw_count_mini = |table: usize, count: u32| {
        let mut mini = cache(&[10], Tables::default());
        let tail = module_region_end(&mini).unwrap();
        assert!(table <= 5);
        mini[tail + table * 4..tail + table * 4 + 4].copy_from_slice(&count.to_le_bytes());
        mini
    };

    let base = cache(&[10], Tables::default());
    let mut guard = SequentialMiniGuard::new(&base).unwrap();
    for (resource, actual, limit, mini) in [
        (
            "keyed row count",
            KEYED_LIMIT + 1,
            KEYED_LIMIT,
            raw_count_mini(0, (KEYED_LIMIT + 1) as u32),
        ),
        (
            "StaticNames count",
            STATIC_LIMIT + 1,
            STATIC_LIMIT,
            raw_count_mini(5, (STATIC_LIMIT + 1) as u32),
        ),
    ] {
        let error = match guard.check_and_record(&mini) {
            Ok(_) => panic!("oversized {resource} unexpectedly passed"),
            Err(error) => error,
        };
        assert!(matches!(
            error,
            SpliceError::SequentialLimitExceeded {
                resource: actual_resource,
                actual: actual_value,
                limit: actual_limit,
            } if actual_resource == resource && actual_value == actual && actual_limit == limit
        ));
    }

    let empty = cache(&[10], Tables::default());
    for accepted in 0..MINI_LIMIT {
        guard.check_and_record(&empty).unwrap_or_else(|error| {
            panic!(
                "failed preflight consumed usage; mini {} of {MINI_LIMIT} failed: {error:?}",
                accepted + 1
            )
        });
    }
    let error = guard
        .check_and_record(&empty)
        .expect_err("mini count limit+1 must fail");
    assert!(matches!(
        error,
        SpliceError::SequentialLimitExceeded {
            resource: "mini count",
            actual: 257,
            limit: 256,
        }
    ));
}

#[test]
fn sequential_guard_preflights_per_function_and_total_bytecode_caps_atomically() {
    const FUNCTION_LIMIT: usize = 4 * 1024 * 1024;
    const TOTAL_LIMIT: u64 = 16 * 1024 * 1024;
    let base = cache(&[10], Tables::default());

    let oversized_function = {
        let code = vec![10; FUNCTION_LIMIT + 1];
        cache(&code, Tables::default())
    };
    let mut guard = SequentialMiniGuard::new(&base).unwrap();
    let error = guard
        .check_and_record(&oversized_function)
        .expect_err("one function above the 4M-dword limit must fail in streaming preflight");
    assert!(matches!(
        error,
        SpliceError::SequentialLimitExceeded {
            resource: "mini function bytecode dwords",
            actual,
            limit: 4_194_304,
        } if actual == 4_194_305
    ));
    guard
        .check_and_record(&cache(&[10], Tables::default()))
        .expect("a failed per-function preflight must not consume sequential guard state");

    let oversized_total = {
        // Four functions exactly at the per-function boundary prove equality is admitted far
        // enough for the independent aggregate check to report total-limit+1.
        let boundary = vec![10; FUNCTION_LIMIT];
        let functions = [
            function(&boundary, 0x0600_0001),
            function(&boundary, 0x0600_0002),
            function(&boundary, 0x0600_0003),
            function(&boundary, 0x0600_0004),
            function(&[10], 0x0600_0005),
        ];
        let module = module_value_with_records(&functions, &[], &[], &[], &[]);
        cache_from_module_value(&module, Tables::default())
    };
    let mut guard = SequentialMiniGuard::new(&base).unwrap();
    let error = guard
        .check_and_record(&oversized_total)
        .expect_err("aggregate bytecode above 16M dwords must fail in streaming preflight");
    assert!(
        matches!(
            error,
            SpliceError::SequentialLimitExceeded {
                resource: "mini total bytecode dwords",
                actual,
                limit: TOTAL_LIMIT,
            } if actual == TOTAL_LIMIT + 1
        ),
        "unexpected aggregate bytecode error: {error:?}"
    );
    guard
        .check_and_record(&cache(&[10], Tables::default()))
        .expect("a failed aggregate preflight must not consume sequential guard state");
}

#[test]
fn composed_cache_validates_object_variables_on_heap_mask() {
    for (heap_mask, function_id) in [(0, 0x0340_0001), (1, 0x0340_0002)] {
        let mini = cache_with_records(
            &[shaped_function(
                function_id,
                FunctionShape {
                    variable_space: 4,
                    object_types: 1,
                    object_positions: 1,
                    object_position_values: &[4],
                    object_heap_mask: heap_mask,
                    stack_needed: 4,
                    ..FunctionShape::default()
                },
            )],
            &[],
            &[],
            &[],
        );
        splice_auto(&composition_base(0x0340_0000), &mini)
            .unwrap_or_else(|error| panic!("heap mask {heap_mask} must be valid: {error:?}"));
    }

    for (heap_mask, function_id) in [(-1, 0x0340_0003), (2, 0x0340_0004)] {
        let mini = cache_with_records(
            &[shaped_function(
                function_id,
                FunctionShape {
                    variable_space: 4,
                    object_types: 1,
                    object_positions: 1,
                    object_position_values: &[4],
                    object_heap_mask: heap_mask,
                    stack_needed: 4,
                    ..FunctionShape::default()
                },
            )],
            &[],
            &[],
            &[],
        );
        assert_composed_structure_error(&mini, "Function.ObjVariablesOnHeap");
    }
}

#[test]
fn composed_cache_validates_variable_space_object_positions_and_stack_needed() {
    for (case, function_id, shape) in [
        (
            "empty zero-sized frame",
            0x0340_0005,
            FunctionShape::default(),
        ),
        (
            "object at the lower frame bound",
            0x0340_0006,
            FunctionShape {
                variable_space: 4,
                object_types: 1,
                object_positions: 1,
                object_position_values: &[1],
                object_heap_mask: 1,
                stack_needed: 4,
                ..FunctionShape::default()
            },
        ),
    ] {
        let mini = cache_with_records(&[shaped_function(function_id, shape)], &[], &[], &[]);
        splice_auto(&composition_base(0x0340_0000), &mini)
            .unwrap_or_else(|error| panic!("{case} must be valid: {error:?}"));
    }

    let invalid = [
        (
            "negative VariableSpace",
            "Function.VariableSpace",
            0x0340_0007,
            FunctionShape {
                variable_space: -1,
                ..FunctionShape::default()
            },
        ),
        (
            "zero object position",
            "Function.ObjectVariables",
            0x0340_0008,
            FunctionShape {
                variable_space: 4,
                object_types: 1,
                object_positions: 1,
                object_position_values: &[0],
                stack_needed: 4,
                ..FunctionShape::default()
            },
        ),
        (
            "object position beyond VariableSpace",
            "Function.ObjectVariables",
            0x0340_0009,
            FunctionShape {
                variable_space: 4,
                object_types: 1,
                object_positions: 1,
                object_position_values: &[5],
                stack_needed: 4,
                ..FunctionShape::default()
            },
        ),
        (
            "StackNeeded below VariableSpace",
            "Function.StackNeeded",
            0x0340_000a,
            FunctionShape {
                variable_space: 4,
                stack_needed: 3,
                ..FunctionShape::default()
            },
        ),
    ];
    for (case, field, function_id, shape) in invalid {
        let mini = cache_with_records(&[shaped_function(function_id, shape)], &[], &[], &[]);
        let error = splice_auto(&composition_base(0x0340_0000), &mini).unwrap_err();
        assert!(
            matches!(
                error,
                SpliceError::ComposedModule(RemapError::InvalidModuleStructure {
                    field: actual,
                    ..
                }) if actual == field
            ),
            "{case}: expected {field}, got {error:?}"
        );
    }
}

#[test]
fn composed_cache_validates_variable_info_state_machine() {
    const OBJ_UNINIT: i32 = 0;
    const OBJ_INIT: i32 = 1;
    const BLOCK_BEGIN: i32 = 2;
    const BLOCK_END: i32 = 3;

    let balanced = cache_with_records(
        &[shaped_function(
            0x0340_0010,
            FunctionShape {
                variable_space: 4,
                object_types: 1,
                object_positions: 1,
                object_position_values: &[4],
                variable_program_positions: 4,
                variable_program_values: &[0, 0, 0, 1],
                variable_offsets: 4,
                variable_offset_values: &[0, 4, 4, 0],
                variable_options: 4,
                variable_option_values: &[BLOCK_BEGIN, OBJ_INIT, OBJ_UNINIT, BLOCK_END],
                stack_needed: 4,
                ..FunctionShape::default()
            },
        )],
        &[],
        &[],
        &[],
    );
    splice_auto(&composition_base(0x0340_0000), &balanced)
        .expect("balanced block and matching object INIT/UNINIT offsets are valid");

    let invalid = [
        (
            "option 4",
            0x0340_0011,
            FunctionShape {
                variable_space: 4,
                object_types: 1,
                object_positions: 1,
                object_position_values: &[4],
                variable_program_positions: 1,
                variable_program_values: &[0],
                variable_offsets: 1,
                variable_offset_values: &[4],
                variable_options: 1,
                variable_option_values: &[4],
                stack_needed: 4,
                ..FunctionShape::default()
            },
        ),
        (
            "UNINIT missing from object offsets",
            0x0340_0012,
            FunctionShape {
                variable_space: 4,
                object_types: 1,
                object_positions: 1,
                object_position_values: &[4],
                variable_program_positions: 1,
                variable_program_values: &[0],
                variable_offsets: 1,
                variable_offset_values: &[8],
                variable_options: 1,
                variable_option_values: &[OBJ_UNINIT],
                stack_needed: 4,
                ..FunctionShape::default()
            },
        ),
        (
            "INIT with empty object offsets",
            0x0340_0013,
            FunctionShape {
                variable_space: 4,
                variable_program_positions: 1,
                variable_program_values: &[0],
                variable_offsets: 1,
                variable_offset_values: &[4],
                variable_options: 1,
                variable_option_values: &[OBJ_INIT],
                stack_needed: 4,
                ..FunctionShape::default()
            },
        ),
        (
            "lone BLOCK_END",
            0x0340_0014,
            FunctionShape {
                variable_program_positions: 1,
                variable_program_values: &[0],
                variable_offsets: 1,
                variable_offset_values: &[0],
                variable_options: 1,
                variable_option_values: &[BLOCK_END],
                ..FunctionShape::default()
            },
        ),
        (
            "unsorted ProgramPos",
            0x0340_0015,
            FunctionShape {
                variable_program_positions: 2,
                variable_program_values: &[1, 0],
                variable_offsets: 2,
                variable_offset_values: &[0, 0],
                variable_options: 2,
                variable_option_values: &[BLOCK_BEGIN, BLOCK_END],
                ..FunctionShape::default()
            },
        ),
    ];
    for (case, function_id, shape) in invalid {
        let mini = cache_with_records(&[shaped_function(function_id, shape)], &[], &[], &[]);
        let error = splice_auto(&composition_base(0x0340_0000), &mini).unwrap_err();
        assert!(
            matches!(
                error,
                SpliceError::ComposedModule(RemapError::InvalidModuleStructure {
                    field: "Function.VariableInfo",
                    ..
                })
            ),
            "{case}: unexpected error: {error:?}"
        );
    }
}

#[test]
fn composed_cache_validates_behavior_function_type_tags() {
    let seven_refs = [0i64; 7];
    for (behavior_type, function_id, should_pass) in
        [(2, 0x0340_0021, true), (0, 0x0340_0022, false)]
    {
        let behavior = shaped_function(function_id + 1, FunctionShape::default());
        let class = structural_class_record(
            &[],
            &[],
            &[],
            &seven_refs,
            &[behavior],
            &[behavior_type],
            None,
        );
        let mini = cache_with_records(&[function(&[10], function_id)], &[class], &[], &[]);
        if should_pass {
            splice_auto(&composition_base(0x0340_0000), &mini)
                .expect("BehaviorFunctionTypes tag 2 is canonical");
        } else {
            assert_composed_structure_error(&mini, "Class.BehaviorFunctionTypes");
        }
    }

    let two_behaviors = structural_class_record(
        &[],
        &[],
        &[],
        &seven_refs,
        &[
            shaped_function(0x0340_0024, FunctionShape::default()),
            shaped_function(0x0340_0025, FunctionShape::default()),
        ],
        &[2, 2],
        None,
    );
    let mini = cache_with_records(&[function(&[10], 0x0340_0023)], &[two_behaviors], &[], &[]);
    assert_composed_structure_error(&mini, "Class.BehaviorFunctions");
}

#[test]
fn composed_cache_accepts_complete_structural_positive_matrix() {
    let shape = FunctionShape {
        parameter_types: 1,
        parameter_names: 1,
        parameter_flags: 1,
        parameter_defaults: 1,
        variable_space: 4,
        object_types: 1,
        object_positions: 1,
        object_position_values: &[4],
        variable_program_positions: 1,
        variable_program_values: &[0],
        variable_offsets: 1,
        variable_offset_values: &[4],
        variable_options: 1,
        variable_option_values: &[1],
        stack_needed: 4,
        unreal_metadata: Some((1, 1)),
        ..FunctionShape::default()
    };
    let behavior_function = shaped_function(0x0350_0003, FunctionShape::default());
    let class = structural_class_record_full(
        &[property_with_metadata(1, 1)],
        &[
            shaped_function(0x0350_0001, FunctionShape::default()),
            shaped_function(0x0350_0002, FunctionShape::default()),
        ],
        &[0, 1, -1],
        &[
            shaped_function(0x0350_0004, FunctionShape::default()),
            shaped_function(0x0350_0005, FunctionShape::default()),
        ],
        &[0],
        &[0; 7],
        &[behavior_function],
        &[2],
        Some((1, 1)),
    );
    let global = global_init_record(&shaped_function(0x0350_0006, FunctionShape::default()));
    let mini = cache_with_all_records(
        &[shaped_function(0, shape)],
        &[class],
        &[enum_record(1, 1)],
        &[global],
        &[import_record(1, 1, 1)],
    );

    let combined = splice_auto(&composition_base(0x0350_0007), &mini).expect(
        "equal paired arrays, canonical BehaviorRefs, sparse MethodTable and unequal ctor/factory counts are valid",
    );
    assert_eq!(module_count(&combined), 2);
}

#[test]
fn composed_cache_rejects_noncanonical_behavior_ref_count() {
    let class = structural_class_record(&[], &[], &[], &[0; 6], &[], &[], None);
    let mini = cache_with_records(&[function(&[10], 0x0400_0001)], &[class], &[], &[]);
    assert_composed_structure_error(&mini, "Class.BehaviorRefs");
}

#[test]
fn composed_cache_rejects_oob_and_duplicate_method_table_indices() {
    let seven_refs = [0i64; 7];
    let methods = [function(&[10], 0x0400_0011), function(&[10], 0x0400_0012)];
    for method_table in [vec![2], vec![0, 0]] {
        let class =
            structural_class_record(&[], &methods, &method_table, &seven_refs, &[], &[], None);
        let mini = cache_with_records(&[function(&[10], 0x0400_0010)], &[class], &[], &[]);
        assert_composed_structure_error(&mini, "Class.MethodTable");
    }
}

#[test]
fn strict_remap_stamps_target_guid_and_passes_the_sequential_guard() {
    let mut base = base_cache();
    base[..16].copy_from_slice(&[0x33; 16]);
    let mut regen = regen_existing_flagged_typeid_cache();
    regen[..16].copy_from_slice(&[0x44; 16]);

    let (mini, _) = remap_module_to_base(&regen, &base).unwrap();
    assert_eq!(&mini[..16], &base[..16]);
    SequentialMiniGuard::new(&base)
        .unwrap()
        .check_and_record(&mini)
        .expect("strict remapped refs all resolve through the target base");
}

#[test]
fn allow_new_remap_stamps_target_guid_and_passes_the_sequential_guard() {
    let mut base = base_cache();
    base[..16].copy_from_slice(&[0x55; 16]);
    let mut regen = regen_cache();
    regen[..16].copy_from_slice(&[0x66; 16]);

    let (mini, _) = remap_module_to_base_with_options(
        &regen,
        &base,
        RemapOptions {
            allow_new_symbols: true,
        },
    )
    .unwrap();
    assert_eq!(&mini[..16], &base[..16]);
    SequentialMiniGuard::new(&base)
        .unwrap()
        .check_and_record(&mini)
        .expect("allow-new rows and operands resolve through the effective base-plus-mini tables");
}

#[test]
fn extracted_referenceless_mini_from_the_same_base_passes() {
    let base = base_cache();
    let extracted = extract_module(&base, MODULE).unwrap();
    SequentialMiniGuard::new(&base)
        .unwrap()
        .check_and_record(&extracted)
        .expect("same-generation extraction carries complete effective reference tables");
}

#[test]
fn sequential_guard_rejects_collisions_in_every_keyed_tail_table() {
    for table in [0usize, 1, 2, 3, 4, 6] {
        let base = cache(&[10], Tables::default());
        let mut guard = SequentialMiniGuard::new(&base).unwrap();
        let first = keyed_mini(table, 0x55, "First");
        let first = if table == 6 {
            let owner = structural_class_record_full_named(
                "PropertyOwner",
                "",
                &[property_record_named("First")],
                &[],
                &[],
                &[],
                &[],
                &[0; 7],
                &[],
                &[],
                None,
            );
            let functions = [function(&[10], DEFAULT_MODULE_FUNCTION_ID)];
            let module =
                module_value_with_name_and_records(MODULE, &functions, &[owner], &[], &[], &[]);
            cache_from_module_value(
                &module,
                Tables {
                    types: vec![type_row(0x3055, "PropertyOwner", MODULE, &[])],
                    type_ids: vec![id_row(0x55, 0x3055)],
                    properties: vec![property_row(0x55, 4, "First")],
                    ..Tables::default()
                },
            )
        } else {
            first
        };
        guard.check_and_record(&first).unwrap();
        let err = guard
            .check_and_record(&keyed_mini_with_value_delta(table, 0x55, "Second", 1))
            .unwrap_err();
        let expected_key = if table == 6 {
            property_key(0x55, 4)
        } else {
            0x55
        };
        match err {
            SpliceError::SequentialKeyCollision { table: got, key } => {
                assert_eq!(got, table);
                assert_eq!(key, expected_key);
            }
            other => panic!("table {table}: {other:?}"),
        }
    }
}

#[test]
fn sequential_guard_accepts_exact_duplicate_symbol_rows() {
    let base = cache(&[10], Tables::default());
    for table in [0usize, 1, 2, 3, 4, 6] {
        let mut guard = SequentialMiniGuard::new(&base).unwrap();
        let mini = if table == 6 {
            let owner = structural_class_record_full_named(
                "PropertyOwner",
                "",
                &[property_record_named("SharedSymbol")],
                &[],
                &[],
                &[],
                &[],
                &[0; 7],
                &[],
                &[],
                None,
            );
            let functions = [function(&[10], DEFAULT_MODULE_FUNCTION_ID)];
            let module =
                module_value_with_name_and_records(MODULE, &functions, &[owner], &[], &[], &[]);
            cache_from_module_value(
                &module,
                Tables {
                    types: vec![type_row(0x3066, "PropertyOwner", MODULE, &[])],
                    type_ids: vec![id_row(0x66, 0x3066)],
                    properties: vec![property_row(0x66, 4, "SharedSymbol")],
                    ..Tables::default()
                },
            )
        } else {
            keyed_mini(table, 0x66, "SharedSymbol")
        };
        guard.check_and_record(&mini).unwrap();
        guard
            .check_and_record(&mini)
            .expect("byte-identical symbol row is safe to share");
    }
}

#[test]
fn sequential_guard_rejects_cross_table_oldreference_reuse() {
    let base = cache(&[10], Tables::default());
    let mut guard = SequentialMiniGuard::new(&base).unwrap();
    guard
        .check_and_record(&keyed_mini(0, 0x71, "NewType"))
        .unwrap();
    let err = guard
        .check_and_record(&keyed_mini(2, 0x71, "NewFunction"))
        .unwrap_err();
    assert!(matches!(
        err,
        SpliceError::SequentialKeyCollision {
            table: 2,
            key: 0x71
        }
    ));
}

#[test]
fn compose_add_rejects_running_bytes_from_another_guard_history() {
    const KEY: i64 = 0xb100;
    let base = composition_base(0x0500_0001);
    let mini_a = cache_with_module_key_name_and_function_id(
        "MiniA",
        "MiniA",
        &[10],
        Tables {
            types: vec![type_row(KEY, "RunningType", "MiniA", &[])],
            type_ids: vec![id_row(0x0800_b100, KEY)],
            ..Tables::default()
        },
        0x0500_0002,
    );
    let mut call_b = Vec::new();
    qw_op(61, KEY, &mut call_b);
    call_b.push(10);
    let mini_b = cache_with_module_key_name_and_function_id(
        "MiniB",
        "MiniB",
        &call_b,
        Tables {
            funcs: vec![func_row(KEY, "RunningFunction", "MiniB", 0, &[], 0)],
            func_ids: vec![id_row(0x0000_b100, KEY)],
            ..Tables::default()
        },
        0x0500_0003,
    );

    let mut guard_a = SequentialMiniGuard::new(&base).unwrap();
    let running_a = guard_a
        .compose_add(&base, &mini_a)
        .expect("Guard A may append MiniA to the pristine base");

    let mut fresh_guard = SequentialMiniGuard::new(&base).unwrap();
    let error = fresh_guard
        .compose_add(&running_a, &mini_b)
        .expect_err("a fresh guard must not stage MiniB against running bytes containing MiniA");
    assert!(
        matches!(error, SpliceError::RunningStateMismatch),
        "expected an exact running-state mismatch before staging, got {error:?}"
    );

    fresh_guard
        .compose_add(&base, &mini_b)
        .expect("running-state mismatch must leave fresh guard history uncommitted");
}

#[test]
fn compose_add_rejects_after_check_and_record_invalidates_running_state() {
    let base = composition_base(0x0500_0005);
    let mini_a = cache_with_module_key_name_and_function_id(
        "CheckedA",
        "CheckedA",
        &[10],
        Tables::default(),
        0x0500_0006,
    );
    let mini_b = cache_with_module_key_name_and_function_id(
        "ComposedB",
        "ComposedB",
        &[10],
        Tables::default(),
        0x0500_0007,
    );
    let corrected = cache_with_module_key_name_and_function_id(
        "CheckedRetry",
        "CheckedRetry",
        &[10],
        Tables::default(),
        0x0500_0008,
    );

    let mut guard = SequentialMiniGuard::new(&base).unwrap();
    guard
        .check_and_record(&mini_a)
        .expect("history-only validation of A succeeds");
    let error = guard
        .compose_add(&base, &mini_b)
        .expect_err("compose_add cannot reconstruct running bytes after check_and_record(A)");
    assert!(matches!(error, SpliceError::RunningStateMismatch));
    guard.check_and_record(&corrected).expect(
        "running-state mismatch occurs before staging and cannot poison validation history",
    );
}

#[test]
fn splice_and_guard_reject_duplicate_inner_module_names_atomically() {
    let base = cache_with_module_key_name_and_function_id(
        "OuterA",
        "InnerX",
        &[10],
        Tables::default(),
        0x0500_0011,
    );
    let colliding = cache_with_module_key_name_and_function_id(
        "OuterB",
        "InnerX",
        &[10],
        Tables::default(),
        0x0500_0012,
    );
    let corrected = cache_with_module_key_name_and_function_id(
        "OuterB",
        "InnerY",
        &[10],
        Tables::default(),
        0x0500_0012,
    );

    let splice_error = splice(&base, &colliding)
        .expect_err("distinct outer keys must not hide a duplicate inner ModuleName");
    assert!(matches!(
        splice_error,
        SpliceError::InnerNameCollision(ref name) if name == "InnerX"
    ));

    let mut guard = SequentialMiniGuard::new(&base).unwrap();
    let compose_error = guard
        .compose_add(&base, &colliding)
        .expect_err("compose_add must reject the same duplicate inner ModuleName");
    assert!(matches!(
        compose_error,
        SpliceError::InnerNameCollision(ref name) if name == "InnerX"
    ));

    let composed = guard
        .compose_add(&base, &corrected)
        .expect("inner-name collision must not commit staged guard history");
    assert_eq!(module_names(&composed).unwrap(), vec!["OuterA", "OuterB"]);
}

fn static_mini_str(name: &str, raw_index: u16) -> Vec<u8> {
    cache(
        &[(60u32 | (u32::from(raw_index) << 16)) as i32, 10],
        Tables {
            static_names: vec![sia(name)],
            ..Tables::default()
        },
    )
}

fn only_static_operand(mini: &[u8], op: &str) -> i64 {
    let functions = collect_function_bytecodes(mini).unwrap();
    let instrs = disassemble(&functions[0].bytecode).unwrap();
    let ins = instrs.iter().find(|ins| ins.op.name == op).unwrap();
    if op == "STR" {
        ins.words[0] as i64
    } else {
        ins.dwords[0] as i64
    }
}

#[test]
fn sequential_guard_rebases_two_static_name_minis_across_empty_mini_and_deduplicates() {
    let base = base_cache(); // two pristine names; independent minis both start private T6 at 2
    let mut guard = SequentialMiniGuard::new(&base).unwrap();

    let first = guard
        .check_and_record(&static_mini_str("FirstName", 2))
        .unwrap();
    assert_eq!(only_static_operand(&first, "STR"), 2);
    let first_tail = parse_tail_tables(&first, module_region_end(&first).unwrap()).unwrap();
    assert_eq!(first_tail.tables[5].count, 1);

    // A no-name mini neither shifts nor resets composition state.
    let empty = cache(&[10], Tables::default());
    let prepared_empty = guard.check_and_record(&empty).unwrap();
    assert_eq!(prepared_empty, empty);

    // Exercise the other observed encoding: PshC4 immediately feeding __STATIC_NAME.
    let mut psh_code = vec![2, 2];
    qw_op(61, BASE_STATIC_FUNC_PTR, &mut psh_code);
    psh_code.push(10);
    let second_raw = cache(
        &psh_code,
        Tables {
            static_names: vec![sia("SecondName")],
            ..Tables::default()
        },
    );
    let second = guard.check_and_record(&second_raw).unwrap();
    assert_eq!(only_static_operand(&second, "PshC4"), 3);
    let second_tail = parse_tail_tables(&second, module_region_end(&second).unwrap()).unwrap();
    assert_eq!(second_tail.tables[5].count, 1);

    // Text identity makes this duplicate safe to fold onto FirstName at index 2. The prepared
    // mini contributes no T6 row, rather than appending an unreachable duplicate.
    let duplicate = guard
        .check_and_record(&static_mini_str("FirstName", 2))
        .unwrap();
    assert_eq!(only_static_operand(&duplicate, "STR"), 2);
    let duplicate_tail =
        parse_tail_tables(&duplicate, module_region_end(&duplicate).unwrap()).unwrap();
    assert_eq!(duplicate_tail.tables[5].count, 0);
}

#[test]
fn sequential_guard_state_is_atomic_when_static_rebase_fails() {
    let base = base_cache();
    let mut guard = SequentialMiniGuard::new(&base).unwrap();
    let bad = cache(
        &[(60u32 | (9 << 16)) as i32, 10], // no mini-local T6 row can satisfy index 9
        Tables {
            types: vec![type_row(0x7777, "RetryType", MODULE, &[])],
            type_ids: vec![id_row(0x7777, 0x7777)],
            static_names: vec![sia("RetryName")],
            ..Tables::default()
        },
    );
    let err = guard.check_and_record(&bad).unwrap_err();
    assert!(matches!(
        err,
        SpliceError::StaticNameRebase(RemapError::MissingStaticName(9))
    ));

    // Both the keyed collision set and T6 contribution list must still be pristine after error.
    let corrected = cache(
        &[(60u32 | (2 << 16)) as i32, 10],
        Tables {
            types: vec![type_row(0x7777, "RetryType", MODULE, &[])],
            type_ids: vec![id_row(0x7777, 0x7777)],
            static_names: vec![sia("RetryName")],
            ..Tables::default()
        },
    );
    let prepared = guard.check_and_record(&corrected).unwrap();
    assert_eq!(only_static_operand(&prepared, "STR"), 2);
}

#[test]
fn sequential_guard_accepts_disjoint_and_empty_minis() {
    let base = cache(&[10], Tables::default());
    let mut guard = SequentialMiniGuard::new(&base).unwrap();
    guard
        .check_and_record(&keyed_mini(0, 0x61, "First"))
        .unwrap();
    guard
        .check_and_record(&cache(&[10], Tables::default()))
        .unwrap();
    guard
        .check_and_record(&keyed_mini(0, 0x62, "Second"))
        .unwrap();
}

#[test]
fn two_class_bearing_allow_new_minis_get_identity_stable_disjoint_keys_and_compose() {
    // All three caches deliberately reuse the same raw pointer/id values. Only portable symbol
    // identity differs, mirroring independent game-compiler runs that each allocate first-free
    // ids for their own additive class module.
    let mut base = base_cache_with_function_id(0x1100_0001);
    replace_ascii_same_len(&mut base, "EditedModule", "PristineBase");
    let regen_a = regen_cache_with_function_id(0x1100_0002);
    let mut regen_b = regen_cache_with_function_id(0x1100_0003);
    replace_ascii_same_len(&mut regen_b, "EditedModule", "SecondModule");

    let raw_a = parse_tail_tables(&regen_a, module_region_end(&regen_a).unwrap()).unwrap();
    let raw_b = parse_tail_tables(&regen_b, module_region_end(&regen_b).unwrap()).unwrap();
    for table in [0usize, 1, 2, 3, 4, 6] {
        assert_eq!(
            raw_a.tables[table].keys, raw_b.tables[table].keys,
            "fixture must reproduce raw table-{table} reuse"
        );
    }

    let options = RemapOptions {
        allow_new_symbols: true,
    };
    let (mini_a, _) = remap_module_to_base_with_options(&regen_a, &base, options).unwrap();
    let (mini_b, _) = remap_module_to_base_with_options(&regen_b, &base, options).unwrap();
    let (mini_a_again, _) = remap_module_to_base_with_options(&regen_a, &base, options).unwrap();
    assert_eq!(
        mini_a, mini_a_again,
        "identity allocation must be deterministic"
    );

    let tail_a = parse_tail_tables(&mini_a, module_region_end(&mini_a).unwrap()).unwrap();
    let tail_b = parse_tail_tables(&mini_b, module_region_end(&mini_b).unwrap()).unwrap();
    for table in [0usize, 1, 2, 3, 4, 6] {
        let a: std::collections::HashSet<_> = tail_a.tables[table].keys.iter().copied().collect();
        let b: std::collections::HashSet<_> = tail_b.tables[table].keys.iter().copied().collect();
        assert!(
            a.is_disjoint(&b),
            "different identities collided in remapped table {table}: {:?}",
            a.intersection(&b).collect::<Vec<_>>()
        );
    }

    let mut guard = SequentialMiniGuard::new(&base).unwrap();
    let after_a = guard.compose_add(&base, &mini_a).unwrap();
    let combined = guard.compose_add(&after_a, &mini_b).unwrap();
    assert_eq!(
        module_names(&combined).unwrap(),
        vec!["PristineBase", "EditedModule", "SecondModule"]
    );
    let tables = parse_tail_tables(&combined, module_region_end(&combined).unwrap()).unwrap();
    assert_eq!(tables.end, combined.len());
}

#[test]
fn strict_default_still_rejects_new_symbols() {
    let regen = regen_cache();
    let base = base_cache();
    let strict = remap_module_to_base(&regen, &base).unwrap_err();
    let default =
        remap_module_to_base_with_options(&regen, &base, RemapOptions::default()).unwrap_err();
    assert!(
        matches!(strict, RemapError::Unresolved { .. }),
        "got {strict:?}"
    );
    assert_eq!(strict.to_string(), default.to_string());
}

#[test]
fn strict_remap_maps_typeid_core_and_preserves_handle_flags() {
    let (mini, counts) =
        remap_module_to_base(&regen_existing_flagged_typeid_cache(), &base_cache()).unwrap();
    assert_eq!(counts.type_id, 1);
    let functions = collect_function_bytecodes(&mini).unwrap();
    let instrs = disassemble(&functions[0].bytecode).unwrap();
    let mapped = instrs
        .iter()
        .find(|i| i.op.name == "TYPEID")
        .unwrap()
        .dwords[0] as i32;
    assert_eq!(mapped, BASE_TYPE_ID | 0x6000_0000);
}

#[test]
fn flagged_typeid_alone_declares_and_carries_current_module_new_type() {
    const PTR: i64 = 0x7777;
    const ID: i32 = 0x0800_7777;
    let base = cache(&[10], Tables::default());
    let regen = cache(
        &[76, ID | 0x4000_0000, 10],
        Tables {
            types: vec![type_row(PTR, "ExternalNewType", MODULE, &[])],
            type_ids: vec![id_row(ID, PTR)],
            ..Tables::default()
        },
    );
    let (mini, counts) = remap_module_to_base_with_options(
        &regen,
        &base,
        RemapOptions {
            allow_new_symbols: true,
        },
    )
    .unwrap();
    assert_eq!(counts.type_id, 1);
    let tail = module_region_end(&mini).unwrap();
    let tables = parse_tail_tables(&mini, tail).unwrap();
    assert_eq!(
        tables.tables.iter().map(|t| t.count).collect::<Vec<_>>(),
        vec![1, 1, 0, 0, 0, 0, 0]
    );

    let replaced = replace_module(&base, &mini, MODULE).unwrap();
    let refs = RefResolver::build(&replaced).unwrap();
    let functions = collect_function_bytecodes(&replaced).unwrap();
    let instrs = disassemble(&functions[0].bytecode).unwrap();
    let mapped = instrs
        .iter()
        .find(|i| i.op.name == "TYPEID")
        .unwrap()
        .dwords[0] as i32;
    assert_eq!(mapped as u32 & !0x1fff_ffff, 0x4000_0000);
    assert_eq!(
        refs.type_by_id((mapped as u32 & 0x1fff_ffff) as i32),
        Some("ExternalNewType")
    );
}

#[test]
fn allow_new_reuses_unique_gap_a_rows_and_only_carries_probe_classes() {
    const BASE_QUEST_PTR: i64 = 0x31_001;
    const BASE_TOPIC_PTR: i64 = 0x31_002;
    const BASE_TOPIC_FUNC_PTR: i64 = 0x31_003;
    const BASE_QUEST_ID: i32 = 0x0803_1001;
    const BASE_TOPIC_ID: i32 = 0x0803_1002;
    const REGEN_QUEST_PTR: i64 = 0x41_001;
    const REGEN_TOPIC_PTR: i64 = 0x41_002;
    const REGEN_TOPIC_FUNC_PTR: i64 = 0x41_003;
    const REGEN_QUEST_ID: i32 = 0x0804_1001;
    const REGEN_TOPIC_ID: i32 = 0x0804_1002;
    const PROBE_QUEST_PTR: i64 = 0x41_101;
    const PROBE_CHOICE_PTR: i64 = 0x41_102;
    const PROBE_QUEST_ID: i32 = 0x0804_1101;
    const PROBE_CHOICE_ID: i32 = 0x0804_1102;

    let base = cache(
        &[10],
        Tables {
            types: vec![
                type_row_ns(BASE_QUEST_PTR, "UG1RQuest", MODULE, "G1R::UG1RQuest", &[]),
                type_row_ns(BASE_TOPIC_PTR, "UTopic", MODULE, "G1R::UTopic", &[]),
            ],
            type_ids: vec![
                id_row(BASE_QUEST_ID, BASE_QUEST_PTR),
                id_row(BASE_TOPIC_ID, BASE_TOPIC_PTR),
            ],
            funcs: vec![func_row_ns(
                BASE_TOPIC_FUNC_PTR,
                "CanUse",
                MODULE,
                "G1R::UTopic",
                BASE_TOPIC_PTR,
                &[],
                0,
            )],
            ..Tables::default()
        },
    );

    let mut code = Vec::new();
    qw_op(75, REGEN_QUEST_PTR, &mut code); // OBJTYPE: existing UG1RQuest
    qw_op(75, REGEN_TOPIC_PTR, &mut code); // OBJTYPE: existing UTopic
    qw_op(75, PROBE_QUEST_PTR, &mut code); // OBJTYPE: genuinely new probe quest
    qw_op(75, PROBE_CHOICE_PTR, &mut code); // OBJTYPE: genuinely new probe choice
    qw_op(61, REGEN_TOPIC_FUNC_PTR, &mut code); // CALLSYS: namespace-drifted method
    code.push(10); // RET
    let regen_tables = Tables {
        types: vec![
            // The emitter flattened the namespace blocks for these existing semantic rows.
            type_row(REGEN_QUEST_PTR, "UG1RQuest", MODULE, &[]),
            type_row(REGEN_TOPIC_PTR, "UTopic", MODULE, &[]),
            type_row(PROBE_QUEST_PTR, "UQuest_GORE_PROBE_HOMER_MINI", MODULE, &[]),
            type_row(
                PROBE_CHOICE_PTR,
                "UChoiceGOREProbeHomerMiniQuest",
                MODULE,
                &[],
            ),
        ],
        type_ids: vec![
            id_row(REGEN_QUEST_ID, REGEN_QUEST_PTR),
            id_row(REGEN_TOPIC_ID, REGEN_TOPIC_PTR),
            id_row(PROBE_QUEST_ID, PROBE_QUEST_PTR),
            id_row(PROBE_CHOICE_ID, PROBE_CHOICE_PTR),
        ],
        funcs: vec![func_row(
            REGEN_TOPIC_FUNC_PTR,
            "CanUse",
            MODULE,
            REGEN_TOPIC_PTR,
            &[],
            0,
        )],
        ..Tables::default()
    };
    let classes = [
        structural_class_record_full_named(
            "UG1RQuest",
            "G1R::UG1RQuest",
            &[],
            &[],
            &[],
            &[],
            &[],
            &[0; 7],
            &[],
            &[],
            None,
        ),
        structural_class_record_full_named(
            "UTopic",
            "G1R::UTopic",
            &[],
            &[],
            &[],
            &[],
            &[],
            &[0; 7],
            &[],
            &[],
            None,
        ),
        structural_class_record_full_named(
            "UQuest_GORE_PROBE_HOMER_MINI",
            "",
            &[],
            &[],
            &[],
            &[],
            &[],
            &[0; 7],
            &[],
            &[],
            None,
        ),
        structural_class_record_full_named(
            "UChoiceGOREProbeHomerMiniQuest",
            "",
            &[],
            &[],
            &[],
            &[],
            &[],
            &[0; 7],
            &[],
            &[],
            None,
        ),
    ];
    let functions = [function(&code, DEFAULT_MODULE_FUNCTION_ID)];
    let regen_module =
        module_value_with_name_and_records(MODULE, &functions, &classes, &[], &[], &[]);
    let regen = cache_from_module_value(&regen_module, regen_tables);

    let (mini, _) = remap_module_to_base_with_options(
        &regen,
        &base,
        RemapOptions {
            allow_new_symbols: true,
        },
    )
    .expect("unique GAP-A identities should map to the base");

    let mini_tail = module_region_end(&mini).unwrap();
    let mini_tables = parse_tail_tables(&mini, mini_tail).unwrap();
    assert_eq!(
        mini_tables
            .tables
            .iter()
            .map(|table| table.count)
            .collect::<Vec<_>>(),
        vec![2, 2, 0, 0, 0, 0, 0],
        "only the two genuinely new probe classes may be carried"
    );

    let replaced = replace_module(&base, &mini, MODULE).unwrap();
    let merged_tail = module_region_end(&replaced).unwrap();
    let merged_tables = parse_tail_tables(&replaced, merged_tail).unwrap();
    assert_eq!(
        merged_tables
            .tables
            .iter()
            .map(|table| table.count)
            .collect::<Vec<_>>(),
        vec![4, 4, 1, 0, 0, 0, 0],
        "UG1RQuest/UTopic rows must not be duplicated in the merged tail"
    );

    let refs = RefResolver::build(&replaced).unwrap();
    let functions = collect_function_bytecodes(&replaced).unwrap();
    let instrs = disassemble(&functions[0].bytecode).unwrap();
    let type_names = instrs
        .iter()
        .filter(|ins| ins.op.name == "OBJTYPE")
        .map(|ins| refs.type_by_ptr(ins.qwords[0] as i64).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        type_names,
        vec![
            "UG1RQuest",
            "UTopic",
            "UQuest_GORE_PROBE_HOMER_MINI",
            "UChoiceGOREProbeHomerMiniQuest",
        ]
    );
    let topic_call = instrs.iter().find(|ins| ins.op.name == "CALLSYS").unwrap();
    assert_eq!(topic_call.qwords[0] as i64, BASE_TOPIC_FUNC_PTR);
    assert_eq!(
        refs.func_by_ptr(topic_call.qwords[0] as i64),
        Some("CanUse")
    );
}

#[test]
fn allow_new_rejects_ambiguous_empty_namespace_oracle_match() {
    const FIRST_BASE_PTR: i64 = 0x51_001;
    const SECOND_BASE_PTR: i64 = 0x51_002;
    const REGEN_PTR: i64 = 0x61_001;

    let base = cache(
        &[10],
        Tables {
            types: vec![
                type_row_ns(FIRST_BASE_PTR, "UTopic", MODULE, "G1R::UTopic", &[]),
                type_row_ns(SECOND_BASE_PTR, "UTopic", MODULE, "Other::UTopic", &[]),
            ],
            ..Tables::default()
        },
    );
    let mut code = Vec::new();
    qw_op(75, REGEN_PTR, &mut code);
    code.push(10);
    let regen = cache(
        &code,
        Tables {
            types: vec![type_row(REGEN_PTR, "UTopic", MODULE, &[])],
            ..Tables::default()
        },
    );

    let err = remap_module_to_base_with_options(
        &regen,
        &base,
        RemapOptions {
            allow_new_symbols: true,
        },
    )
    .unwrap_err();
    assert!(
        matches!(
            err,
            RemapError::Ambiguous {
                kind: "type",
                n: 2,
                ..
            }
        ),
        "empty namespace must not bridge two distinct base namespaces: {err:?}"
    );
}

#[test]
fn allow_new_carries_minimal_rows_rekeys_collisions_and_replaces_cleanly() {
    let base = base_cache();
    let regen = regen_cache();
    let (mini, counts) = remap_module_to_base_with_options(
        &regen,
        &base,
        RemapOptions {
            allow_new_symbols: true,
        },
    )
    .expect("new-symbol remap");
    let (mini_again, _) = remap_module_to_base_with_options(
        &regen,
        &base,
        RemapOptions {
            allow_new_symbols: true,
        },
    )
    .expect("repeat new-symbol remap");
    assert_eq!(
        mini, mini_again,
        "collision re-keying must be deterministic"
    );

    assert!(
        counts.total() >= 8,
        "all typed refs should be processed: {counts:?}"
    );
    assert_eq!(
        counts.embed_type_ptr, 4,
        "DerivedFrom + ShadowType + NewFn parameter/return declarations"
    );
    assert_eq!(counts.embed_func_id, 4, "FactoryRefs + BehaviorRefs");
    let mini_tail = module_region_end(&mini).unwrap();
    let mini_tables = parse_tail_tables(&mini, mini_tail).unwrap();
    assert_eq!(mini_tables.end, mini.len());
    assert_eq!(
        mini_tables
            .tables
            .iter()
            .map(|t| t.count)
            .collect::<Vec<_>>(),
        vec![1, 1, 1, 1, 1, 1, 1],
        "only NewType/NewFn/NewGlobal/BrandNew/NewField rows are carried"
    );

    let replaced = replace_module(&base, &mini, MODULE).expect("replace remapped module");
    assert_eq!(module_count(&replaced), 1);
    let tail = module_region_end(&replaced).unwrap();
    let merged = parse_tail_tables(&replaced, tail).unwrap();
    assert_eq!(merged.end, replaced.len());
    assert_eq!(
        merged.tables.iter().map(|t| t.count).collect::<Vec<_>>(),
        vec![2, 2, 3, 3, 2, 3, 2]
    );

    let refs = RefResolver::build(&replaced).unwrap();
    let functions = collect_function_bytecodes(&replaced).unwrap();
    let code = &functions[0].bytecode;
    let instrs = disassemble(code).unwrap();

    let objtype = instrs
        .iter()
        .find(|i| i.op.name == "OBJTYPE")
        .unwrap()
        .qwords[0] as i64;
    assert_eq!(refs.type_by_ptr(objtype), Some("NewType"));
    assert_ne!(
        objtype, NEW_TYPE_PTR,
        "colliding OldReference must be re-keyed"
    );

    let type_ids: Vec<i32> = instrs
        .iter()
        .filter(|i| i.op.name == "TYPEID")
        .map(|i| i.dwords[0] as i32)
        .collect();
    assert_eq!(type_ids.len(), 2);
    let new_type_id = type_ids[0];
    assert_eq!(new_type_id as u32 & 0x6000_0000, 0x6000_0000);
    let new_type_core = (new_type_id as u32 & !0x6000_0000) as i32;
    assert_eq!(refs.type_by_id(new_type_core), Some("NewType"));
    assert_ne!(new_type_core, NEW_TYPE_ID);
    let existing_type_id = type_ids[1];
    assert_eq!(existing_type_id, BASE_TYPE_ID | 0x4000_0000);
    assert_eq!(
        refs.type_by_id((existing_type_id as u32 & !0x6000_0000) as i32),
        Some("ExistingType")
    );
    let member_type_id = instrs.iter().find(|i| i.op.name == "ADDSi").unwrap().dwords[0] as i32;
    assert_eq!(refs.type_by_id(member_type_id), Some("NewType"));
    assert_eq!(refs.member(member_type_id, 8), Some("NewField"));

    let callsys: Vec<(i64, &str)> = instrs
        .iter()
        .filter(|i| i.op.name == "CALLSYS")
        .map(|i| {
            let ptr = i.qwords[0] as i64;
            (ptr, refs.func_by_ptr(ptr).unwrap())
        })
        .collect();
    assert!(callsys
        .iter()
        .any(|&(ptr, name)| ptr == BASE_FUNC_PTR && name == "ExistingFn"));
    let new_call = callsys.iter().find(|(_, name)| *name == "NewFn").unwrap();
    assert_ne!(
        new_call.0, NEW_FUNC_PTR,
        "colliding function ptr must be re-keyed"
    );
    let call_id = instrs.iter().find(|i| i.op.name == "CALL").unwrap().dwords[0] as i32;
    assert_eq!(refs.func_by_id(call_id), Some("NewFn"));
    assert_ne!(
        call_id, NEW_FUNC_ID,
        "colliding function id must be re-keyed"
    );

    // Class-record refs use the same remap plan. MethodTable is deliberately NOT rewritten: its
    // values are local indices into Class.Methods[], not T4 ids.
    let module_bytes = &replaced[0x18..tail];
    let contains = |needle: &[u8]| module_bytes.windows(needle.len()).any(|w| w == needle);
    let mut type_ref_sequence = 2i32.to_le_bytes().to_vec(); // MethodTable count
    type_ref_sequence.extend_from_slice(&(-1i32).to_le_bytes());
    type_ref_sequence.extend_from_slice(&(-1i32).to_le_bytes());
    type_ref_sequence.extend_from_slice(&BASE_TYPE_PTR.to_le_bytes()); // DerivedFrom
    type_ref_sequence.extend_from_slice(&objtype.to_le_bytes()); // ShadowType
    assert!(
        contains(&type_ref_sequence),
        "MethodTable remains local while class type refs are remapped"
    );
    let mut function_ref_sequence = 2i32.to_le_bytes().to_vec();
    function_ref_sequence.extend_from_slice(&(BASE_FUNC_ID as i64).to_le_bytes());
    function_ref_sequence.extend_from_slice(&(call_id as i64).to_le_bytes());
    let factory_occurrences = module_bytes
        .windows(function_ref_sequence.len())
        .filter(|w| *w == function_ref_sequence)
        .count();
    assert_eq!(factory_occurrences, 1, "FactoryRefs remapped");
    let mut behavior_ref_sequence = 7i32.to_le_bytes().to_vec();
    behavior_ref_sequence.extend_from_slice(&(BASE_FUNC_ID as i64).to_le_bytes());
    behavior_ref_sequence.extend_from_slice(&(call_id as i64).to_le_bytes());
    assert!(
        contains(&behavior_ref_sequence),
        "BehaviorRefs retain their canonical count and remap their function ids"
    );

    let global = instrs
        .iter()
        .find(|i| i.op.name == "PshGPtr")
        .unwrap()
        .qwords[0] as i64;
    assert_eq!(refs.global_by_ptr(global), Some("NewGlobal"));
    assert_ne!(
        global, NEW_GLOBAL_PTR,
        "colliding global ptr must be re-keyed"
    );

    let str_index = instrs.iter().find(|i| i.op.name == "STR").unwrap().words[0] as i64;
    assert_eq!(refs.static_name(str_index), Some("SharedName"));
    let psh_index = instrs.iter().find(|i| i.op.name == "PshC4").unwrap().dwords[0] as i64;
    assert_eq!(refs.static_name(psh_index), Some("BrandNew"));
    assert_eq!(psh_index, 2, "new StaticName follows the two base entries");
}

#[test]
fn property_collision_fails_instead_of_overwriting_base_row() {
    let err = remap_module_to_base_with_options(
        &regen_cache_with_existing_property("DifferentField"),
        &base_cache(),
        RemapOptions {
            allow_new_symbols: true,
        },
    )
    .unwrap_err();
    assert!(
        matches!(err, RemapError::PropertyCollision { .. }),
        "got {err:?}"
    );
}

#[test]
fn real_viper_and_asghan_allow_new_regens_compose_when_fixtures_are_available() {
    let Ok(base_path) = std::env::var("GORE_AS_REAL_CACHE") else {
        eprintln!("skip: set GORE_AS_REAL_CACHE for Viper+Asghan fixture composition");
        return;
    };
    let workspace = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let fixtures = [
        (
            "GoreMods.Probe.ViperDialogFixture",
            workspace.join("work/probe/viper-dialog-fixture/regen.Cache"),
            workspace.join("work/probe/viper-dialog-fixture/ViperDialogFixture.mini.Cache"),
        ),
        (
            "GoreMods.Probe.AsghanMiniQuest",
            workspace.join("work/probe/asghan-miniquest/regen-asghan.Cache"),
            workspace.join("work/probe/asghan-miniquest/AsghanMiniQuest.mini.Cache"),
        ),
    ];
    if fixtures
        .iter()
        .any(|(_, regen, old_mini)| !regen.is_file() || !old_mini.is_file())
    {
        eprintln!("skip: Viper/Asghan regen fixtures are not present");
        return;
    }

    let base = std::fs::read(base_path).expect("read pristine real cache");
    // The checked-in fixture minis are the promoted deterministic artifacts regenerated after
    // the sequential T2/T4 collision fix (see the Viper validation report).  Prove those exact
    // bundle inputs still compose; an older version of this test incorrectly kept asserting that
    // they were the superseded pre-fix artifacts and therefore expected a historical collision.
    let fixture_minis: Vec<Vec<u8>> = fixtures
        .iter()
        .map(|(_, _, path)| std::fs::read(path).expect("read promoted fixture mini"))
        .collect();
    let mut fixture_guard = SequentialMiniGuard::new(&base).unwrap();
    let fixture_after_first = fixture_guard.compose_add(&base, &fixture_minis[0]).unwrap();
    let fixture_combined = fixture_guard
        .compose_add(&fixture_after_first, &fixture_minis[1])
        .unwrap();
    let fixture_names = module_names(&fixture_combined).unwrap();
    for (module, _, _) in &fixtures {
        assert!(
            fixture_names.iter().any(|name| name == module),
            "promoted fixture composition is missing {module}"
        );
    }

    let mut minis = Vec::new();
    for (module, regen_path, _) in &fixtures {
        let regen = std::fs::read(regen_path).expect("read full-tree regen fixture");
        let extracted = extract_module(&regen, module).expect("extract additive fixture module");
        drop(regen);
        let (mini, _) = remap_module_to_base_with_options(
            &extracted,
            &base,
            RemapOptions {
                allow_new_symbols: true,
            },
        )
        .expect("identity-remap real additive module");
        let tail = parse_tail_tables(&mini, module_region_end(&mini).unwrap()).unwrap();
        assert!(!tail.tables[1].keys.contains(&134_244_321));
        assert!(!tail.tables[3].keys.contains(&294_413));
        minis.push(mini);
    }

    // This exact boundary previously failed first at T2 id 134244321 and then at T4 id 294413.
    // Exercise the production composition path so every prospective output also passes the final
    // pristine-aware T1/T3/T5/T7 declaration closure before the guard commits either mini.
    let mut guard = SequentialMiniGuard::new(&base).unwrap();
    let after_first = guard.compose_add(&base, &minis[0]).unwrap();
    let combined = guard.compose_add(&after_first, &minis[1]).unwrap();
    let names = module_names(&combined).unwrap();
    for (module, _, _) in fixtures {
        assert!(names.iter().any(|name| name == module), "missing {module}");
    }
    let tables = parse_tail_tables(&combined, module_region_end(&combined).unwrap()).unwrap();
    assert_eq!(tables.end, combined.len());
    for table in [0usize, 1, 2, 3, 4, 6] {
        let unique: std::collections::HashSet<_> =
            tables.tables[table].keys.iter().copied().collect();
        assert_eq!(
            unique.len(),
            tables.tables[table].keys.len(),
            "duplicate key survived in final table {table}"
        );
    }
    RefResolver::build(&combined).expect("final real candidate reference tables resolve");
    let target_functions: Vec<_> = collect_function_bytecodes(&combined)
        .unwrap()
        .into_iter()
        .filter(|function| {
            function.func.contains("ViperDialogFixture")
                || function.func.contains("AsghanMiniQuest")
        })
        .collect();
    assert!(!target_functions.is_empty());
    for function in target_functions {
        disassemble(&function.bytecode)
            .unwrap_or_else(|error| panic!("{} no longer disassembles: {error}", function.func));
    }
}

#[test]
fn real_cache_tail_tables_roundtrip_when_configured() {
    let Ok(path) = std::env::var("GORE_AS_REAL_CACHE") else {
        eprintln!("skip: set GORE_AS_REAL_CACHE");
        return;
    };
    let bytes = std::fs::read(path).expect("read GORE_AS_REAL_CACHE");
    let tail = module_region_end(&bytes).expect("walk real modules");
    let tables = parse_tail_tables(&bytes, tail).expect("parse real tail tables");
    assert_eq!(tables.end, bytes.len());

    let mut rebuilt = Vec::with_capacity(bytes.len() - tail);
    for table in &tables.tables {
        rebuilt.extend_from_slice(&table.count.to_le_bytes());
        rebuilt.extend_from_slice(&bytes[table.entries_start..table.entries_end]);
    }
    assert_eq!(
        rebuilt,
        bytes[tail..],
        "raw table rows round-trip byte-for-byte"
    );
    SequentialMiniGuard::new(&bytes)
        .expect("real Shipping cache identities must fit the composed 4x/256 MiB guard budget");
}
