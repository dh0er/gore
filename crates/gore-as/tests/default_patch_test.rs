use std::collections::{BTreeMap, BTreeSet};
use std::io::Read;
use std::path::Path;

use gore_as::cache::{
    header::{CacheHeader, CACHE_MAGIC},
    types::DataType,
    walk_modules::{
        collect_function_bytecode_spans, collect_function_bytecodes, FuncCode, FuncCodeKind,
    },
    wire::WireError,
};
use sha2::{Digest, Sha256};

const SAMPLES: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../work/reversing/gore-as/samples"
);

fn push_i32(bytes: &mut Vec<u8>, value: i32) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn push_u32(bytes: &mut Vec<u8>, value: u32) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn push_i64(bytes: &mut Vec<u8>, value: i64) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn push_sia(bytes: &mut Vec<u8>, value: &str) {
    push_i32(
        bytes,
        i32::try_from(value.len()).expect("fixture SIA length fits i32"),
    );
    if !value.is_empty() {
        bytes.extend_from_slice(value.as_bytes());
        bytes.push(0);
    }
}

fn push_fstring(bytes: &mut Vec<u8>, value: &str) {
    push_i32(
        bytes,
        i32::try_from(value.len() + 1).expect("fixture FString length fits i32"),
    );
    bytes.extend_from_slice(value.as_bytes());
    bytes.push(0);
}

fn push_data_type(bytes: &mut Vec<u8>, token: i32) {
    push_data_type_ref(bytes, 0, token);
}

fn push_data_type_ref(bytes: &mut Vec<u8>, type_info: i64, token: i32) {
    for _ in 0..6 {
        push_i32(bytes, 0); // six serialized bool flags
    }
    push_i64(bytes, type_info); // TypeInfo.OldReference
    push_i32(bytes, token);
}

fn push_function(bytes: &mut Vec<u8>, name: &str, traits: i32, bytecode: &[i32]) {
    push_sia(bytes, name);
    push_sia(bytes, ""); // Namespace
    push_data_type(bytes, 0x52); // void return
    push_i32(bytes, 0); // ParameterTypes count
    push_i32(bytes, 0); // ParameterNames count
    push_i32(bytes, 0); // ParameterFlags count
    push_i32(bytes, 0); // ParameterDefaultArgs count
    push_i32(bytes, traits);
    push_i32(
        bytes,
        i32::try_from(bytecode.len()).expect("fixture bytecode count fits i32"),
    );
    for &word in bytecode {
        push_i32(bytes, word);
    }
    push_i32(bytes, 0); // ByteCodeReferences count
    push_i32(bytes, 0); // VariableSpace
    push_i32(bytes, 0); // ObjVariableTypes count
    push_i32(bytes, 0); // ObjVariablePos count
    push_i32(bytes, 0); // ObjVariablesOnHeap
    push_i32(bytes, 0); // VariableInfoProgramPos count
    push_i32(bytes, 0); // VariableInfoOffset count
    push_i32(bytes, 0); // VariableInfoOption count
    push_i32(bytes, 0); // StackNeeded
    push_i32(bytes, 0); // Id
    push_i32(bytes, 0); // DeclaredAt
    push_i32(bytes, 0); // LineNumbers count
    push_i32(bytes, 0); // bIsUFunction
}

fn opcode_word(opcode: u8, word: u16) -> i32 {
    (u32::from(opcode) | (u32::from(word) << 16)) as i32
}

/// One script class with a directly patchable int default and caller-selected MethodTable.
fn class_method_table_cache(method_table: &[i32]) -> Vec<u8> {
    class_method_table_cache_with_value(method_table, 0x44, 4)
}

#[derive(Clone, Copy)]
struct ScriptEnumFixture<'a> {
    name: &'a str,
    namespace: &'a str,
    type_ref_module: &'a str,
    type_ref_namespace: &'a str,
    has_subtypes: bool,
}

fn class_method_table_cache_with_value(
    method_table: &[i32],
    field_type_token: i32,
    value: i32,
) -> Vec<u8> {
    class_method_table_cache_with_value_and_enum(method_table, field_type_token, value, None)
}

fn class_method_table_cache_with_value_and_enum(
    method_table: &[i32],
    field_type_token: i32,
    value: i32,
    script_enum: Option<ScriptEnumFixture<'_>>,
) -> Vec<u8> {
    const OWNER_TYPE_ID: i32 = 7;
    const OWNER_TYPE_PTR: i64 = 100;
    const ENUM_TYPE_PTR: i64 = 200;
    const MEMBER_OFFSET: u16 = 0;

    let mut bytes = vec![0; 16]; // build GUID
    push_u32(&mut bytes, CACHE_MAGIC);
    push_u32(&mut bytes, 1); // Modules count

    push_fstring(&mut bytes, "_fixture"); // TMap key
    push_sia(&mut bytes, "_fixture"); // ModuleName
    push_i32(&mut bytes, 0); // Functions count
    push_i32(&mut bytes, 1); // Classes count

    push_sia(&mut bytes, "FixtureClass");
    push_sia(&mut bytes, ""); // Namespace
    push_i32(&mut bytes, 0); // Flags
    push_i32(&mut bytes, 1); // Properties count
    push_sia(&mut bytes, "Value");
    if script_enum.is_some() {
        push_data_type_ref(&mut bytes, ENUM_TYPE_PTR, field_type_token);
    } else {
        push_data_type(&mut bytes, field_type_token);
    }
    push_i32(&mut bytes, 0); // bIsPrivate
    push_i32(&mut bytes, 0); // bIsProtected
    push_i32(&mut bytes, 0); // bIsUProperty
    push_i32(&mut bytes, 1); // Methods count
    push_function(
        &mut bytes,
        "__InitDefaults",
        0,
        &[
            opcode_word(77, 1), // SetV4 slot 1
            value,
            opcode_word(178, MEMBER_OFFSET), // LoadThisR member offset
            OWNER_TYPE_ID,
            opcode_word(90, 1), // WRTV4 slot 1
            opcode_word(10, 0), // RET
        ],
    );
    push_i32(
        &mut bytes,
        i32::try_from(method_table.len()).expect("fixture MethodTable length fits i32"),
    );
    for &entry in method_table {
        push_i32(&mut bytes, entry);
    }
    push_i64(&mut bytes, 0); // DerivedFrom
    push_i64(&mut bytes, 0); // ShadowType
    push_i32(&mut bytes, 0); // Constructors count
    push_i32(&mut bytes, 0); // FactoryRefs count
    push_i32(&mut bytes, 0); // BehaviorRefs count
    push_i32(&mut bytes, 0); // BehaviorFunctions count
    push_i32(&mut bytes, 0); // BehaviorFunctionTypes count
    push_i32(&mut bytes, 0); // bHasClassData

    push_i32(&mut bytes, i32::from(script_enum.is_some())); // Enums count
    if let Some(script_enum) = script_enum {
        push_sia(&mut bytes, script_enum.name);
        push_sia(&mut bytes, script_enum.namespace);
        push_i32(&mut bytes, 2); // Enum.Names count
        push_sia(&mut bytes, "Idle");
        push_sia(&mut bytes, "Active");
        push_i32(&mut bytes, 2); // Enum.Values count
        push_i32(&mut bytes, 0);
        push_i32(&mut bytes, 1);
    }
    push_i32(&mut bytes, 0); // GlobalVariables count
    push_i32(&mut bytes, 0); // FunctionImports count
    push_i64(&mut bytes, 0); // CodeHash
    push_i32(&mut bytes, 0); // ImportedModules count
    push_sia(&mut bytes, ""); // StaticsClassName
    push_i32(&mut bytes, 0); // DeclaredEvents count
    push_i32(&mut bytes, 0); // DeclaredDelegates count
    push_sia(&mut bytes, "Fixture.as");
    push_i32(&mut bytes, 0); // PostInitFunctions count

    push_i32(&mut bytes, 1 + i32::from(script_enum.is_some())); // TypeReferences
    push_i64(&mut bytes, OWNER_TYPE_PTR);
    push_sia(&mut bytes, "FixtureClass");
    push_sia(&mut bytes, "_fixture");
    push_sia(&mut bytes, "");
    push_i32(&mut bytes, 0); // TypeRef.SubTypes
    if let Some(script_enum) = script_enum {
        push_i64(&mut bytes, ENUM_TYPE_PTR);
        push_sia(&mut bytes, script_enum.name);
        push_sia(&mut bytes, script_enum.type_ref_module);
        push_sia(&mut bytes, script_enum.type_ref_namespace);
        push_i32(&mut bytes, i32::from(script_enum.has_subtypes));
        if script_enum.has_subtypes {
            push_data_type(&mut bytes, 0x44);
        }
    }
    push_i32(&mut bytes, 1); // TypeIdReferenceToPointer
    push_i32(&mut bytes, OWNER_TYPE_ID);
    push_i64(&mut bytes, OWNER_TYPE_PTR);
    push_i32(&mut bytes, 0); // FunctionReferences
    push_i32(&mut bytes, 0); // FunctionIdReferenceToPointer
    push_i32(&mut bytes, 0); // GlobalReferences
    push_i32(&mut bytes, 0); // StaticNames
    push_i32(&mut bytes, 1); // PropertyReferences
    let member_key = (i64::from(OWNER_TYPE_ID) << 1) | (i64::from(MEMBER_OFFSET) << 33) | 1;
    push_i64(&mut bytes, member_key);
    push_sia(&mut bytes, "Value");
    push_i32(&mut bytes, OWNER_TYPE_ID); // OldTypeId

    bytes
}

/// Canonical one-module/one-function cache built independently of the walker.
fn canonical_one_function_cache() -> Vec<u8> {
    let mut bytes = vec![0; 16]; // build GUID
    push_u32(&mut bytes, CACHE_MAGIC);
    push_u32(&mut bytes, 1); // Modules count

    push_fstring(&mut bytes, "_fixture"); // TMap key
    push_sia(&mut bytes, "_fixture"); // ModuleName
    push_i32(&mut bytes, 1); // Functions count

    push_sia(&mut bytes, "PatchMe");
    push_sia(&mut bytes, ""); // Namespace
    push_data_type(&mut bytes, 0x52); // void return
    push_i32(&mut bytes, 1); // ParameterTypes count
    push_data_type(&mut bytes, 0x44); // int parameter
    push_i32(&mut bytes, 1); // ParameterNames count
    push_sia(&mut bytes, "value");
    push_i32(&mut bytes, 1); // ParameterFlags count
    push_i32(&mut bytes, 0);
    push_i32(&mut bytes, 1); // ParameterDefaultArgs count
    push_sia(&mut bytes, "");
    push_i32(&mut bytes, 7); // FunctionTraits
    push_i32(&mut bytes, 3); // ByteCode count; payload starts at canonical offset 0xb8
    for word in [0x1122_3344, -7, 0x5566_7788] {
        push_i32(&mut bytes, word);
    }
    push_i32(&mut bytes, 0); // ByteCodeReferences count
    push_i32(&mut bytes, 12); // VariableSpace
    push_i32(&mut bytes, 0); // ObjVariableTypes count
    push_i32(&mut bytes, 0); // ObjVariablePos count
    push_i32(&mut bytes, 0); // ObjVariablesOnHeap
    push_i32(&mut bytes, 0); // VariableInfoProgramPos count
    push_i32(&mut bytes, 0); // VariableInfoOffset count
    push_i32(&mut bytes, 0); // VariableInfoOption count
    push_i32(&mut bytes, 3); // StackNeeded
    push_i32(&mut bytes, 42); // Id
    push_i32(&mut bytes, 17); // DeclaredAt
    push_i32(&mut bytes, 0); // LineNumbers count
    push_i32(&mut bytes, 0); // bIsUFunction

    push_i32(&mut bytes, 0); // Classes count
    push_i32(&mut bytes, 0); // Enums count
    push_i32(&mut bytes, 0); // GlobalVariables count
    push_i32(&mut bytes, 0); // FunctionImports count
    push_i64(&mut bytes, 0x0102_0304_0506_0708); // CodeHash
    push_i32(&mut bytes, 0); // ImportedModules count
    push_sia(&mut bytes, ""); // StaticsClassName
    push_i32(&mut bytes, 0); // DeclaredEvents count
    push_i32(&mut bytes, 0); // DeclaredDelegates count
    push_sia(&mut bytes, "Fixture.as");
    push_i32(&mut bytes, 0); // PostInitFunctions count

    for _ in 0..7 {
        push_i32(&mut bytes, 0); // seven empty global tail tables
    }
    bytes
}

fn assert_data_type_eq(actual: &DataType, expected: &DataType, context: &str) {
    assert_eq!(
        (
            actual.is_reference,
            actual.is_object_const,
            actual.is_object_handle,
            actual.is_read_only,
            actual.is_auto,
            actual.if_handle_then_const,
            actual.type_info,
            actual.token,
        ),
        (
            expected.is_reference,
            expected.is_object_const,
            expected.is_object_handle,
            expected.is_read_only,
            expected.is_auto,
            expected.if_handle_then_const,
            expected.type_info,
            expected.token,
        ),
        "data type mismatch for {context}"
    );
}

fn assert_func_eq(actual: &FuncCode, expected: &FuncCode, index: usize) {
    assert_eq!(actual.func, expected.func, "function name at index {index}");
    assert_eq!(
        actual.is_method, expected.is_method,
        "method flag for {}",
        actual.func
    );
    assert_eq!(
        actual.param_names, expected.param_names,
        "parameter names for {}",
        actual.func
    );
    assert_eq!(
        actual.param_types.len(),
        expected.param_types.len(),
        "parameter count for {}",
        actual.func
    );
    for (param_index, (actual_type, expected_type)) in actual
        .param_types
        .iter()
        .zip(&expected.param_types)
        .enumerate()
    {
        assert_data_type_eq(
            actual_type,
            expected_type,
            &format!("{} parameter {param_index}", actual.func),
        );
    }
    assert_data_type_eq(
        &actual.ret,
        &expected.ret,
        &format!("{} return", actual.func),
    );
    assert_eq!(
        actual.bytecode, expected.bytecode,
        "bytecode for {}",
        actual.func
    );
}

fn assert_spans_match_legacy(bytes: &[u8], label: &str) {
    let legacy = collect_function_bytecodes(bytes).expect("legacy bytecode walk");
    let spans = collect_function_bytecode_spans(bytes).expect("spanned bytecode walk");

    assert_eq!(
        spans.len(),
        legacy.len(),
        "function count mismatch for {label}"
    );
    for (index, (span, expected)) in spans.iter().zip(&legacy).enumerate() {
        assert_func_eq(&span.code, expected, index);

        let count_offset = span
            .bytecode_offset
            .checked_sub(4)
            .expect("bytecode payload follows its four-byte count");
        let count_end = count_offset + 4;
        assert!(
            count_end <= bytes.len(),
            "{} bytecode count is outside {label}",
            span.code.func
        );
        let serialized_count = i32::from_le_bytes(
            bytes[count_offset..count_end]
                .try_into()
                .expect("four-byte count"),
        );
        assert_eq!(
            serialized_count,
            i32::try_from(span.code.bytecode.len()).expect("cache bytecode count fits i32"),
            "serialized bytecode count for {} in {label}",
            span.code.func
        );

        let byte_len = span
            .code
            .bytecode
            .len()
            .checked_mul(4)
            .expect("bytecode byte length");
        let bytecode_end = span
            .bytecode_offset
            .checked_add(byte_len)
            .expect("bytecode end offset");
        assert!(
            bytecode_end <= bytes.len(),
            "{} bytecode span {:#x}..{bytecode_end:#x} exceeds {label} length {:#x}",
            span.code.func,
            span.bytecode_offset,
            bytes.len()
        );

        let serialized_words: Vec<i32> = bytes[span.bytecode_offset..bytecode_end]
            .chunks_exact(4)
            .map(|word| i32::from_le_bytes(word.try_into().expect("four-byte word")))
            .collect();
        assert_eq!(
            serialized_words, span.code.bytecode,
            "absolute bytecode span for {} in {label}",
            span.code.func
        );
    }
}

#[test]
fn canonical_function_reports_exact_metadata_and_payload_offset() {
    let bytes = canonical_one_function_cache();
    assert_eq!(bytes.len(), 0x14f, "canonical fixture byte length");

    let spans = collect_function_bytecode_spans(&bytes).expect("walk canonical cache");
    assert_eq!(spans.len(), 1);
    let span = &spans[0];
    assert_eq!(span.code.func, "_fixture::PatchMe");
    assert!(!span.code.is_method);
    assert_eq!(span.kind, FuncCodeKind::FreeFunction);
    assert_eq!(span.function_traits, 7);
    assert!(!span.method_table_valid);
    assert!(!span.in_method_table);
    assert_eq!(span.code.param_names, ["value"]);
    assert_eq!(span.code.param_types.len(), 1);
    assert_data_type_eq(
        &span.code.param_types[0],
        &DataType {
            token: 0x44,
            ..DataType::default()
        },
        "canonical int parameter",
    );
    assert_data_type_eq(
        &span.code.ret,
        &DataType {
            token: 0x52,
            ..DataType::default()
        },
        "canonical void return",
    );
    assert_eq!(span.code.bytecode, [0x1122_3344, -7, 0x5566_7788]);

    assert_eq!(span.bytecode_offset, 0xb8, "canonical ByteCode[0]");
    assert_eq!(
        i32::from_le_bytes(bytes[0xb4..0xb8].try_into().unwrap()),
        3,
        "serialized ByteCode count immediately precedes the payload"
    );
    assert_eq!(
        &bytes[0xb8..0xc4],
        &[
            0x44, 0x33, 0x22, 0x11, // 0x11223344
            0xf9, 0xff, 0xff, 0xff, // -7
            0x88, 0x77, 0x66, 0x55, // 0x55667788
        ]
    );
}

#[test]
fn malformed_or_duplicate_method_tables_invalidate_every_class_method() {
    for (method_table, valid) in [
        (vec![-1, 0, -1], true),
        (vec![0, 1], false),
        (vec![0, -2], false),
        (vec![0, 0], false),
    ] {
        let bytes = class_method_table_cache(&method_table);
        let spans = collect_function_bytecode_spans(&bytes).expect("walk class fixture");
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].kind, FuncCodeKind::ClassMethod);
        assert!(spans[0].in_method_table);
        assert_eq!(
            spans[0].method_table_valid, valid,
            "MethodTable {method_table:?}"
        );

        let report = gore_as::cache::default_patch::default_sites(&bytes, None)
            .expect("inspect class fixture");
        assert_eq!(report.stats.init_functions, usize::from(valid));
        assert_eq!(report.sites.len(), usize::from(valid));
    }
}

#[test]
fn noncanonical_void_return_metadata_rejects_initializer_fixture() {
    for (label, mutate) in [
        ("reference flag", (0usize, 1i64)),
        ("nonzero type info", (24usize, 7i64)),
    ] {
        let mut bytes = class_method_table_cache(&[0]);
        let name = b"__InitDefaults\0";
        let name_start = bytes
            .windows(name.len())
            .position(|window| window == name)
            .expect("fixture initializer name");
        let return_type_start = name_start + name.len() + 4; // empty namespace SIA
        let (relative_offset, value) = mutate;
        if relative_offset == 24 {
            bytes[return_type_start + relative_offset..return_type_start + relative_offset + 8]
                .copy_from_slice(&value.to_le_bytes());
        } else {
            bytes[return_type_start + relative_offset..return_type_start + relative_offset + 4]
                .copy_from_slice(&(value as i32).to_le_bytes());
        }

        let report = gore_as::cache::default_patch::default_sites(&bytes, None)
            .unwrap_or_else(|error| panic!("{label}: {error}"));
        assert_eq!(report.stats.init_functions, 0, "{label}");
        assert!(report.sites.is_empty(), "{label}");
    }
}

#[test]
fn stale_equal_expected_and_replacement_reports_cas_mismatch_first() {
    let bytes = class_method_table_cache(&[0]);
    let report =
        gore_as::cache::default_patch::default_sites(&bytes, None).expect("inspect class fixture");
    let site = report.sites.first().expect("direct default site");
    assert_eq!(site.selector.field_owner, "FixtureClass");
    let stale = 99u32.to_le_bytes();

    let error =
        gore_as::cache::default_patch::patch_default(&bytes, None, &site.selector, &stale, &stale)
            .unwrap_err();
    assert!(matches!(
        error,
        gore_as::cache::default_patch::DefaultPatchError::CasMismatch {
            expected,
            actual,
        } if expected == "63000000" && actual == "04000000"
    ));

    let error = gore_as::cache::default_patch::patch_default(
        &bytes,
        None,
        &site.selector,
        &site.expected,
        &site.expected,
    )
    .unwrap_err();
    assert!(matches!(
        error,
        gore_as::cache::default_patch::DefaultPatchError::NoChange
    ));
}

#[test]
fn stale_selector_cannot_cross_value_type_drift_with_identical_cas_bytes() {
    let int_cache = class_method_table_cache_with_value(&[0], 0x44, 0);
    let float_cache = class_method_table_cache_with_value(&[0], 0x50, 0);
    let int_report = gore_as::cache::default_patch::default_sites(&int_cache, None).unwrap();
    let float_report = gore_as::cache::default_patch::default_sites(&float_cache, None).unwrap();
    let int_site = int_report.sites.first().unwrap();
    let float_site = float_report.sites.first().unwrap();

    assert_eq!(int_site.selector.value_type, "int");
    assert_eq!(float_site.selector.value_type, "float32");
    assert_ne!(int_site.selector, float_site.selector);
    assert_eq!(int_site.expected, [0, 0, 0, 0]);
    assert_eq!(float_site.expected, [0, 0, 0, 0]);

    let error = gore_as::cache::default_patch::patch_default(
        &float_cache,
        None,
        &int_site.selector,
        &[0, 0, 0, 0],
        &[1, 0, 0, 0],
    )
    .unwrap_err();
    assert!(matches!(
        error,
        gore_as::cache::default_patch::DefaultPatchError::SelectorNotFound
    ));
}

fn script_enum_cache(
    namespace: &str,
    type_ref_module: &str,
    type_ref_namespace: &str,
    has_subtypes: bool,
) -> Vec<u8> {
    class_method_table_cache_with_value_and_enum(
        &[0],
        5,
        0,
        Some(ScriptEnumFixture {
            name: "StatusKind",
            namespace,
            type_ref_module,
            type_ref_namespace,
            has_subtypes,
        }),
    )
}

#[test]
fn script_enum_kind_requires_exact_type_info_module_and_namespace() {
    let exact = script_enum_cache("State", "_fixture", "State", false);
    let report = gore_as::cache::default_patch::default_sites(&exact, None).unwrap();
    let site = report.sites.first().expect("exact script enum site");
    assert_eq!(site.selector.field, "Value");
    assert!(site.selector.value_type.starts_with("script-enum:"));
    assert!(site.selector.value_type.ends_with(":StatusKind"));

    let replacement = 1u32.to_le_bytes();
    let patched = gore_as::cache::default_patch::patch_default(
        &exact,
        None,
        &site.selector,
        &site.expected,
        &replacement,
    )
    .expect("patch exact script enum");
    assert_eq!(patched.after.expected, replacement);

    let wrong_namespace = script_enum_cache("State", "_fixture", "OtherState", false);
    let report = gore_as::cache::default_patch::default_sites(&wrong_namespace, None).unwrap();
    assert!(report.sites.is_empty());
    assert_eq!(report.stats.unsupported_types, 1);

    let wrong_module = script_enum_cache("State", "other_module", "State", false);
    let report = gore_as::cache::default_patch::default_sites(&wrong_module, None).unwrap();
    assert!(report.sites.is_empty());
    assert_eq!(report.stats.unsupported_types, 1);

    let templated = script_enum_cache("State", "_fixture", "State", true);
    let report = gore_as::cache::default_patch::default_sites(&templated, None).unwrap();
    assert!(report.sites.is_empty());
    assert_eq!(report.stats.unsupported_types, 1);
}

#[test]
fn stale_selector_cannot_cross_script_enum_identity_drift() {
    let first = script_enum_cache("FirstState", "_fixture", "FirstState", false);
    let second = script_enum_cache("SecondState", "_fixture", "SecondState", false);
    let first_site = gore_as::cache::default_patch::default_sites(&first, None)
        .unwrap()
        .sites
        .into_iter()
        .next()
        .unwrap();
    let second_site = gore_as::cache::default_patch::default_sites(&second, None)
        .unwrap()
        .sites
        .into_iter()
        .next()
        .unwrap();
    assert_eq!(first_site.expected, second_site.expected);
    assert_ne!(first_site.selector, second_site.selector);

    let error = gore_as::cache::default_patch::patch_default(
        &second,
        None,
        &first_site.selector,
        &first_site.expected,
        &1u32.to_le_bytes(),
    )
    .unwrap_err();
    assert!(matches!(
        error,
        gore_as::cache::default_patch::DefaultPatchError::SelectorNotFound
    ));
}

#[test]
fn span_walk_rejects_inputs_shorter_than_the_cache_header() {
    for len in [0, 1, CacheHeader::SIZE - 1] {
        let bytes = vec![0; len];
        assert_eq!(
            collect_function_bytecode_spans(&bytes).unwrap_err(),
            WireError::Eof {
                pos: 0,
                need: CacheHeader::SIZE,
                have: len,
            }
        );
    }
}

#[test]
fn default_inspection_rejects_duplicate_semantic_tail_keys() {
    let mut bytes = canonical_one_function_cache();
    bytes.truncate(bytes.len() - 7 * 4);
    push_i32(&mut bytes, 0); // TypeReferences
    push_i32(&mut bytes, 2); // TypeIdReferenceToPointer
    for pointer in [11i64, 22] {
        push_i32(&mut bytes, 7); // duplicated type id
        push_i64(&mut bytes, pointer);
    }
    for _ in 0..5 {
        push_i32(&mut bytes, 0);
    }
    let error = gore_as::cache::default_patch::default_sites(&bytes, None).unwrap_err();
    assert!(matches!(
        error,
        gore_as::cache::default_patch::DefaultSiteError::DuplicateTailKey {
            table: "TypeIdReferenceToPointer",
            key: 7,
        }
    ));
}

#[test]
fn span_walk_rejects_impossible_counts_before_count_backed_growth() {
    let mut too_many_modules = vec![0; CacheHeader::SIZE];
    too_many_modules[0x10..0x14].copy_from_slice(&CACHE_MAGIC.to_le_bytes());
    too_many_modules[0x14..0x18].copy_from_slice(&10_000_000u32.to_le_bytes());
    assert_eq!(
        collect_function_bytecode_spans(&too_many_modules).unwrap_err(),
        WireError::Eof {
            pos: CacheHeader::SIZE,
            need: 10_000_000 * 60,
            have: 0,
        }
    );

    let mut too_many_parameter_types = canonical_one_function_cache();
    too_many_parameter_types[0x6a..0x6e].copy_from_slice(&50_000_000i32.to_le_bytes());
    assert_eq!(
        collect_function_bytecode_spans(&too_many_parameter_types).unwrap_err(),
        WireError::Eof {
            pos: 0x6e,
            need: 50_000_000 * 36,
            have: too_many_parameter_types.len() - 0x6e,
        }
    );

    let mut too_many_bytecode_words = canonical_one_function_cache();
    too_many_bytecode_words[0xb4..0xb8].copy_from_slice(&50_000_000i32.to_le_bytes());
    assert_eq!(
        collect_function_bytecode_spans(&too_many_bytecode_words).unwrap_err(),
        WireError::Eof {
            pos: 0xb8,
            need: 50_000_000 * 4,
            have: too_many_bytecode_words.len() - 0xb8,
        }
    );
}

#[test]
fn sample_cache_spans_match_legacy_walk_and_payload_bounds() {
    let mut found = false;
    for name in [
        "PrecompiledScript.minimal-1fn.Cache",
        "PrecompiledScript.richtest.Cache",
    ] {
        let path = Path::new(SAMPLES).join(name);
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        found = true;
        assert_spans_match_legacy(&bytes, name);
    }
    if !found {
        eprintln!("skip: bytecode cache samples are not present");
    }
}

#[test]
fn configured_real_cache_spans_match_legacy_walk_and_payload_bounds() {
    let Some(path) =
        std::env::var_os("GORE_AS_REAL_CACHE").or_else(|| std::env::var_os("GORE_AS_CACHE"))
    else {
        eprintln!("skip: set GORE_AS_REAL_CACHE (or GORE_AS_CACHE)");
        return;
    };
    let bytes = std::fs::read(&path).expect("read configured real cache");
    assert_spans_match_legacy(&bytes, &path.to_string_lossy());
}

#[test]
fn configured_real_cache_exposes_known_direct_default_sites() {
    let Some(path) = std::env::var_os("GORE_AS_DEFAULT_CACHE") else {
        eprintln!("skip: set GORE_AS_DEFAULT_CACHE");
        return;
    };
    let path = std::path::PathBuf::from(path);
    let bytes = std::fs::read(&path).expect("read configured real cache");
    let refs = gore_as::cache::refs::RefResolver::build(&bytes).expect("build refs");
    let owner = refs
        .type_by_id(0x0400_121d)
        .expect("known item owner")
        .to_owned();
    let value = refs
        .member(0x0400_121d, 128)
        .expect("known item value field")
        .to_owned();
    let stack = refs
        .member(0x0400_121d, 132)
        .expect("known item max-stack field")
        .to_owned();
    let binds_path = path.parent().unwrap().join("Binds.Cache");
    let native = gore_as::cache::binds::NativeApi::load(&binds_path).expect("load sibling binds");
    let cache_guid = CacheHeader::parse(&bytes)
        .expect("parse configured header")
        .hash;
    assert_eq!(owner, "UItemDefinition");
    assert_eq!(value, "m_Value");
    assert_eq!(stack, "m_MaxStack");
    assert_eq!(
        native.verified_default_field_type(&cache_guid, &owner, &value),
        Some("int")
    );
    assert_eq!(
        native.verified_default_field_type(&cache_guid, &owner, &stack),
        Some("int")
    );

    let report = gore_as::cache::default_patch::default_sites(&bytes, Some(native))
        .expect("inspect default sites");
    let all_spans = collect_function_bytecode_spans(&bytes).unwrap();
    let mut initializer_shapes = std::collections::BTreeMap::new();
    for span in all_spans
        .iter()
        .filter(|span| span.code.func.ends_with("::__InitDefaults"))
    {
        *initializer_shapes
            .entry(format!(
                "{:?}/table={}/traits={:#x}/params={}/ret={:#x}",
                span.kind,
                span.in_method_table,
                span.function_traits,
                span.code.param_types.len(),
                span.code.ret.token
            ))
            .or_insert(0usize) += 1;
    }
    assert_eq!(initializer_shapes.len(), 2, "{initializer_shapes:#?}");
    assert_eq!(
        initializer_shapes.get("ClassMethod/table=true/traits=0x0/params=0/ret=0x52"),
        Some(&3_248)
    );
    assert_eq!(
        initializer_shapes.get("ClassMethod/table=true/traits=0x20/params=0/ret=0x52"),
        Some(&26_703)
    );
    let apple_span = all_spans
        .into_iter()
        .find(|span| span.code.func.ends_with("UItFo_Apple::__InitDefaults"))
        .expect("Apple initializer span");
    assert_eq!(apple_span.kind, FuncCodeKind::ClassMethod);
    assert_eq!(apple_span.function_traits, 0x20);
    assert!(apple_span.method_table_valid);
    assert!(apple_span.in_method_table);
    assert!(apple_span.code.param_types.is_empty());
    assert_eq!(apple_span.code.ret.token, 0x52);
    let apple: Vec<_> = report
        .sites
        .iter()
        .filter(|site| site.selector.class == "UItFo_Apple")
        .map(|site| (site.selector.field.as_str(), site.display_value.as_str()))
        .collect();
    assert!(apple.contains(&("m_Value", "4")), "{apple:?}");
    assert!(apple.contains(&("m_MaxStack", "99")), "{apple:?}");
    assert_eq!(report.stats.init_functions, 29_951);
    assert_eq!(report.stats.branched_init_functions, 1);
    assert_eq!(report.stats.direct_windows, 26_339);
    // Native inheritance beyond the first native base is not present in the script cache.
    // Those windows remain visible in `direct_windows` but are deliberately not editable until
    // a separately sealed native-ancestry profile can prove target -> declaring owner.
    assert_eq!(report.stats.unresolved_fields, 5_197);
    assert_eq!(report.stats.unresolved_types, 1);
    // Ten formerly ambiguous groups are wholly inside the unproven native-grandparent set.
    assert_eq!(report.stats.ambiguous_fields, 1);
    // Sword derives from the unparsed native `USword1H`; its owners UItemDefinition and
    // UWeaponDefinition are native grandparents, so both formerly known raw windows now fail the
    // required ancestry proof until a sealed native hierarchy profile is supplied.
    assert!(!report
        .sites
        .iter()
        .any(|site| site.selector.class == "UItMw_1H_Sword_Old_01"));
    assert!(!report
        .sites
        .iter()
        .any(|site| site.selector.class == "UArmor_OC_EBR_Gomez_100"));
    assert!(report.sites.iter().any(|site| {
        site.selector.field == "MaximumRangedAttackHorizontalDistance"
            && site.display_value == "1500"
            && site.encoding == gore_as::cache::default_patch::RawEncoding::LeU64
    }));

    let value_site = report
        .sites
        .iter()
        .find(|site| site.selector.class == "UItFo_Apple" && site.selector.field == "m_Value")
        .expect("Apple m_Value site");
    assert_eq!(value_site.selector.field_owner, "UItemDefinition");
    let replacement = 5u32.to_le_bytes();
    let native = gore_as::cache::binds::NativeApi::load(&binds_path).expect("reload sibling binds");
    let patched = gore_as::cache::default_patch::patch_default(
        &bytes,
        Some(native),
        &value_site.selector,
        &value_site.expected,
        &replacement,
    )
    .expect("patch Apple m_Value in a copy");
    assert_eq!(patched.bytes.len(), bytes.len());
    assert_eq!(patched.after.display_value, "5");
    assert_eq!(patched.after.expected, replacement);
    assert_eq!(
        bytes[value_site.operand_offset..value_site.operand_offset + 4],
        [4, 0, 0, 0]
    );
    assert_eq!(
        patched.bytes[value_site.operand_offset..value_site.operand_offset + 4],
        replacement
    );
}

#[test]
fn configured_real_default_ancestry_profile_is_sealed() {
    let Some(path) = std::env::var_os("GORE_AS_DEFAULT_CACHE") else {
        eprintln!("skip: set GORE_AS_DEFAULT_CACHE");
        return;
    };
    let path = std::path::PathBuf::from(path);
    let bytes = std::fs::read(&path).expect("read configured real cache");
    let semantic_sha = gore_as::cache::default_patch::default_profile_cache_sha256(&bytes)
        .expect("compute default-profile cache identity");
    eprintln!(
        "default-profile-cache-sha256={}",
        gore_as::cache::default_patch::encode_hex(&semantic_sha)
    );
    let binds = gore_as::cache::binds::NativeApi::load(
        &path.parent().expect("Script directory").join("Binds.Cache"),
    )
    .expect("load sibling Binds.Cache");
    let usmap = std::env::var_os("GORE_AS_USMAP")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            path.parent()
                .expect("Script directory")
                .parent()
                .expect("G1R directory")
                .join("Binaries/Win64/ue4ss/G1R-5.4.3-168781-272ce2f8.usmap")
        });
    let usmap_bytes = std::fs::read(&usmap).expect("read configured USMAP");
    let schemas = gore_asset::SchemaDb::from_usmap(&usmap_bytes).expect("parse configured USMAP");
    let profile = gore_as::cache::default_ancestry::DefaultNativeAncestry::from_schema_db(
        &binds, &bytes, &schemas,
    )
    .expect("build sealed ancestry profile");
    assert_eq!(profile.class_count(), 6_572);
    assert!(
        gore_as::cache::default_ancestry::is_supported_gameplay_tag_float32_proof_pair(
            profile.profile_id(),
            profile.gameplay_tag_float32_map_proof_id(),
        )
    );
}

#[test]
fn configured_hotfix_24169431_profile_and_item_field_matrix_are_exact() {
    let Some(game) = std::env::var_os("GORE_AS_HOTFIX_24169431_GAME") else {
        eprintln!("skip: set GORE_AS_HOTFIX_24169431_GAME");
        return;
    };
    let game = std::path::PathBuf::from(game);
    let exe = game.join("G1R/Binaries/Win64/G1R-Win64-Shipping.exe");
    let cache_path = game.join("G1R/Script/PrecompiledScript_Shipping.Cache");
    let binds_path = game.join("G1R/Script/Binds.Cache");
    let usmap = std::env::var_os("GORE_AS_HOTFIX_24169431_USMAP")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            let directory = game.join("G1R/Binaries/Win64/ue4ss");
            let mut candidates: Vec<_> = std::fs::read_dir(&directory)
                .expect("read configured hotfix ue4ss directory")
                .map(|entry| entry.expect("read ue4ss entry").path())
                .filter(|path| {
                    path.extension()
                        .and_then(|extension| extension.to_str())
                        .is_some_and(|extension| extension.eq_ignore_ascii_case("usmap"))
                })
                .collect();
            candidates.sort();
            assert_eq!(
                candidates.len(),
                1,
                "hotfix qualification requires one USMAP"
            );
            candidates.remove(0)
        });

    fn stream_seal(path: &Path) -> (u64, String) {
        let mut file = std::fs::File::open(path).expect("open sealed generation file");
        let length = file.metadata().expect("read sealed file metadata").len();
        let mut hash = Sha256::new();
        let mut buffer = vec![0u8; 1024 * 1024];
        loop {
            let read = file.read(&mut buffer).expect("hash sealed generation file");
            if read == 0 {
                break;
            }
            hash.update(&buffer[..read]);
        }
        (
            length,
            gore_as::cache::default_patch::encode_hex(&hash.finalize()),
        )
    }

    assert_eq!(
        stream_seal(&exe),
        (
            171_704_320,
            "b52cd0453ad03987b833f7f26d09a2075109f18d653b8d4ff95271c857139e5d".into()
        )
    );
    assert_eq!(
        stream_seal(&cache_path),
        (
            123_394_250,
            "757d8624f0c7480f63cc14a1ba2d7e43f461a529064b0c0cfbf523a54639e385".into()
        )
    );
    assert_eq!(
        stream_seal(&binds_path),
        (
            5_903_938,
            "46e6629ad5cacc112b9922d48a1aa948f40572d7285705b981c3eca3dc615fea".into()
        )
    );
    assert_eq!(
        stream_seal(&usmap),
        (
            2_516_955,
            "73558c36895cd1b0f0fd1b3cb44305b240f8dbb93730ad03c88d7b8478b7ffca".into()
        )
    );

    let cache = std::fs::read(&cache_path).expect("read hotfix Shipping cache");
    assert_eq!(
        gore_as::cache::default_patch::encode_hex(&Sha256::digest(&cache)),
        "757d8624f0c7480f63cc14a1ba2d7e43f461a529064b0c0cfbf523a54639e385"
    );
    assert_eq!(
        CacheHeader::parse(&cache)
            .expect("parse hotfix header")
            .hash,
        [
            0x43, 0x52, 0x1b, 0x38, 0x49, 0x7e, 0x98, 0x4f, 0x8a, 0xbb, 0xc0, 0x35, 0xeb, 0x4c,
            0xb1, 0xd7,
        ]
    );
    let binds_bytes = std::fs::read(&binds_path).expect("read hotfix Binds");
    assert_eq!(
        gore_as::cache::default_patch::encode_hex(&Sha256::digest(&binds_bytes)),
        "46e6629ad5cacc112b9922d48a1aa948f40572d7285705b981c3eca3dc615fea"
    );
    let binds = gore_as::cache::binds::NativeApi::from_bytes(&binds_bytes)
        .expect("parse sealed hotfix Binds");
    let usmap_bytes = std::fs::read(&usmap).expect("read hotfix USMAP");
    assert_eq!(
        gore_as::cache::default_patch::encode_hex(&Sha256::digest(&usmap_bytes)),
        "73558c36895cd1b0f0fd1b3cb44305b240f8dbb93730ad03c88d7b8478b7ffca"
    );
    let schemas =
        gore_asset::SchemaDb::from_usmap(&usmap_bytes).expect("parse sealed hotfix USMAP");
    let profile = gore_as::cache::default_ancestry::DefaultNativeAncestry::from_schema_db(
        &binds, &cache, &schemas,
    )
    .expect("derive exact BuildID-24169431 profile");
    assert_eq!(profile.class_count(), 6_572);
    assert_eq!(
        profile.profile_id(),
        gore_as::cache::default_ancestry::HOTFIX_24169431_NATIVE_ANCESTRY_PROFILE_ID
    );
    assert_eq!(
        profile.gameplay_tag_float32_map_proof_id(),
        gore_as::cache::default_ancestry::HOTFIX_24169431_GAMEPLAY_TAG_FLOAT32_MAP_PROOF_ID
    );

    let tag_report = gore_as::cache::native_tag_map::inspect_native_tag_maps(&cache, &profile)
        .expect("inspect sealed hotfix tag maps");
    assert_eq!(tag_report.site_count(), 1_432);
    assert_eq!(
        tag_report.ancestry_profile_id(),
        gore_as::cache::default_ancestry::HOTFIX_24169431_NATIVE_ANCESTRY_PROFILE_ID
    );
    assert_eq!(
        tag_report.map_proof_id(),
        gore_as::cache::default_ancestry::HOTFIX_24169431_GAMEPLAY_TAG_FLOAT32_MAP_PROOF_ID
    );

    let report = gore_as::cache::default_patch::default_sites_with_native_ancestry(
        &cache,
        Some(
            gore_as::cache::binds::NativeApi::from_bytes(&binds_bytes)
                .expect("reparse sealed hotfix Binds"),
        ),
        Some(profile),
    )
    .expect("inspect hotfix defaults with sealed ancestry");
    assert_eq!(report.stats.unresolved_fields, 0);

    let wanted_types = BTreeMap::from([
        ("m_Value", "int"),
        ("m_MaxStack", "int"),
        ("m_Weight", "float32"),
        ("m_Mass", "float32"),
    ]);
    let expected_counts = BTreeMap::from([
        ("m_Value", 906usize),
        ("m_MaxStack", 641usize),
        ("m_Weight", 109usize),
        ("m_Mass", 2usize),
    ]);
    let expected_native_counts = BTreeMap::from([
        ("m_Value", 583usize),
        ("m_MaxStack", 317usize),
        ("m_Weight", 109usize),
        ("m_Mass", 2usize),
    ]);
    let mut counts = BTreeMap::new();
    let mut native_counts = BTreeMap::new();
    let mut target_fields: BTreeMap<(String, String), BTreeSet<String>> = BTreeMap::new();
    for site in report
        .sites
        .iter()
        .filter(|site| wanted_types.contains_key(site.selector.field.as_str()))
    {
        let field = site.selector.field.as_str();
        assert_eq!(site.selector.field_owner, "UItemDefinition", "{field}");
        assert_eq!(
            site.selector.value_type,
            *wanted_types.get(field).expect("known requested field"),
            "{field}"
        );
        *counts.entry(field).or_insert(0usize) += 1;
        if let Some(ancestry_profile) = site.selector.ancestry_profile.as_deref() {
            assert_eq!(
                ancestry_profile,
                gore_as::cache::default_ancestry::HOTFIX_24169431_NATIVE_ANCESTRY_PROFILE_ID,
                "{field}"
            );
            *native_counts.entry(field).or_insert(0usize) += 1;
        }
        assert!(
            target_fields
                .entry((site.selector.module.clone(), site.selector.class.clone()))
                .or_default()
                .insert(site.selector.field.clone()),
            "duplicate qualified target field: {}.{}.{}",
            site.selector.module,
            site.selector.class,
            site.selector.field
        );
    }
    assert_eq!(counts, expected_counts);
    assert_eq!(native_counts, expected_native_counts);
    assert_eq!(target_fields.len(), 918);

    let mut combinations: BTreeMap<Vec<String>, usize> = BTreeMap::new();
    for fields in target_fields.values() {
        *combinations
            .entry(fields.iter().cloned().collect())
            .or_default() += 1;
    }
    assert_eq!(
        combinations,
        BTreeMap::from([
            (vec!["m_Mass".into()], 2usize),
            (vec!["m_MaxStack".into()], 10usize),
            (vec!["m_MaxStack".into(), "m_Value".into()], 631usize),
            (vec!["m_Value".into()], 166usize),
            (vec!["m_Value".into(), "m_Weight".into()], 109usize),
        ])
    );
}
