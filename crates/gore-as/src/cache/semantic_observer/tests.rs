use std::collections::HashMap;

use super::*;
use crate::cache::isa::OPCODES;

#[derive(Default)]
struct Writer {
    bytes: Vec<u8>,
    marks: HashMap<&'static str, usize>,
}

impl Writer {
    fn mark(&mut self, name: &'static str) {
        assert!(self.marks.insert(name, self.bytes.len()).is_none());
    }
    fn i32(&mut self, value: i32) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }
    fn u32(&mut self, value: u32) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }
    fn i64(&mut self, value: i64) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }
    fn u64(&mut self, value: u64) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }
    fn bool4(&mut self, value: bool) {
        self.i32(i32::from(value));
    }
    fn sia(&mut self, value: &str) {
        self.i32(value.len() as i32);
        if !value.is_empty() {
            self.bytes.extend_from_slice(value.as_bytes());
            self.bytes.push(0);
        }
    }
    fn marked_sia(&mut self, mark: &'static str, value: &str) {
        self.i32(value.len() as i32);
        assert!(!value.is_empty());
        self.mark(mark);
        self.bytes.extend_from_slice(value.as_bytes());
        self.bytes.push(0);
    }
    fn fstring(&mut self, value: &str) {
        self.i32(value.len() as i32 + 1);
        self.bytes.extend_from_slice(value.as_bytes());
        self.bytes.push(0);
    }
    fn i32s(&mut self, values: &[i32]) {
        self.i32(values.len() as i32);
        for &value in values {
            self.i32(value);
        }
    }
    fn i64s(&mut self, values: &[i64]) {
        self.i32(values.len() as i32);
        for &value in values {
            self.i64(value);
        }
    }
    fn strings(&mut self, values: &[&str]) {
        self.i32(values.len() as i32);
        for value in values {
            self.sia(value);
        }
    }
}

#[derive(Clone, Copy)]
struct FixtureShape {
    pointer_delta: i64,
    id_delta: i32,
    member_offset: i16,
    guid_byte: u8,
    extra_function_id_alias: bool,
    legacy_bytecode_reference: bool,
    unresolved_bytecode_type: bool,
    include_reserved_fork_opcodes: bool,
    include_resolve_object_ptr: bool,
}

impl Default for FixtureShape {
    fn default() -> Self {
        Self {
            pointer_delta: 0,
            id_delta: 0,
            member_offset: 2,
            guid_byte: 0x11,
            extra_function_id_alias: false,
            legacy_bytecode_reference: false,
            unresolved_bytecode_type: false,
            include_reserved_fork_opcodes: true,
            include_resolve_object_ptr: true,
        }
    }
}

struct Fixture {
    bytes: Vec<u8>,
    marks: HashMap<&'static str, usize>,
}

pub(crate) struct SyntheticObserverQualificationFixtureV1 {
    pub semantic_mutations: Vec<(&'static str, Vec<u8>)>,
    pub unresolved_runtime_reference: Vec<u8>,
    pub legacy_bytecode_references: Vec<u8>,
}

fn op(name: &str) -> i32 {
    OPCODES
        .iter()
        .find(|value| value.name == name)
        .unwrap()
        .opcode as i32
}

fn qword(code: &mut Vec<i32>, opcode: &str, value: i64) {
    code.push(op(opcode));
    code.push(value as u32 as i32);
    code.push((value as u64 >> 32) as u32 as i32);
}

fn data_type(writer: &mut Writer, type_ptr: i64, token: i32) {
    for _ in 0..6 {
        writer.bool4(false);
    }
    writer.i64(type_ptr);
    writer.i32(token);
}

fn function(
    writer: &mut Writer,
    name: &str,
    id: u32,
    type_ptr: i64,
    code: &[i32],
    rich_unreal: bool,
    legacy_bytecode_reference: bool,
) {
    writer.sia(name);
    writer.sia("FixtureNS");
    data_type(writer, 0, 0x52);
    writer.i32(1); // parameter types
    data_type(writer, type_ptr, 5);
    writer.strings(&["Arg"]);
    writer.i32s(&[1]);
    writer.strings(&["DefaultArg"]);
    if name == "Probe" {
        writer.mark("function_traits");
    }
    writer.i32(0x1234);
    writer.i32(code.len() as i32);
    for (index, &word) in code.iter().enumerate() {
        if name == "Probe" && index == 0 {
            writer.mark("bytecode_opcode");
        }
        if name == "Probe" && index + 1 == code.len() {
            writer.mark("bytecode_literal");
        }
        writer.i32(word);
    }
    if legacy_bytecode_reference {
        writer.i32s(&[7]);
    } else {
        writer.i32(0);
    }
    if name == "Probe" {
        writer.mark("variable_space");
    }
    writer.i32(4);
    writer.i64s(&[type_ptr]);
    writer.i32(1);
    if name == "Probe" {
        writer.mark("object_variable_position");
    }
    writer.i32(1);
    if name == "Probe" {
        writer.mark("object_variables_on_heap");
    }
    writer.i32(1);
    writer.i32s(&[0, code.len() as i32]);
    writer.i32s(&[1, 1]);
    writer.i32(2);
    if name == "Probe" {
        writer.mark("var_info_option");
    }
    writer.i32(0);
    writer.i32(1);
    if name == "Probe" {
        writer.mark("stack_needed");
    }
    writer.i32(8);
    writer.u32(id);
    if name == "Probe" {
        writer.mark("declared_at");
    }
    writer.i32(7);
    writer.i32(4);
    writer.i32(0);
    if name == "Probe" {
        writer.mark("line_number");
    }
    writer.i32(10);
    writer.i32(code.len() as i32);
    writer.i32(11);
    writer.bool4(rich_unreal);
    if rich_unreal {
        writer.sia("K2_Probe");
        writer.i32(1);
        writer.marked_sia("ufunction_metadata", "UFMeta");
        writer.strings(&["UFValue"]);
        for index in 0..18 {
            writer.bool4(index % 2 == 0);
        }
    }
}

fn property(writer: &mut Writer, type_ptr: i64) {
    property_named(writer, type_ptr, "Property", true, true, true);
}

fn property_named(
    writer: &mut Writer,
    type_ptr: i64,
    name: &str,
    unreal_property: bool,
    transient: bool,
    mark_metadata: bool,
) {
    writer.sia(name);
    data_type(writer, type_ptr, 5);
    writer.bool4(true);
    writer.bool4(false);
    writer.bool4(unreal_property);
    if !unreal_property {
        return;
    }
    writer.i32(1);
    if mark_metadata {
        writer.marked_sia("property_metadata", "PMeta");
    } else {
        writer.sia("PMeta");
    }
    writer.strings(&["PValue"]);
    for index in 0..9 {
        writer.bool4(if index == 8 {
            transient
        } else {
            index % 2 == 0
        });
    }
    writer.bool4(true); // replicated
    writer.bool4(false);
    writer.bool4(true);
    writer.bool4(true);
    writer.i32(3);
    writer.bool4(true);
    writer.bool4(true);
    writer.bool4(false);
    writer.bool4(true);
}

fn build_fixture(shape: FixtureShape) -> Fixture {
    build_fixture_with_module(shape, "FixtureModule")
}

fn build_fixture_with_module(shape: FixtureShape, module_name: &str) -> Fixture {
    let type_ptr = 0x1000 + shape.pointer_delta;
    let function_ptr = 0x2000 + shape.pointer_delta;
    let global_ptr = 0x3000 + shape.pointer_delta;
    let type_id = 0x0800_000c_i32 + shape.id_delta;
    let function_id = 77 + shape.id_delta;
    let bytecode_type_ptr = if shape.unresolved_bytecode_type {
        type_ptr + 0x999
    } else {
        type_ptr
    };
    let mut code = Vec::new();
    qword(&mut code, "PshGPtr", global_ptr);
    qword(&mut code, "CALLSYS", function_ptr);
    code.extend([op("TYPEID"), type_id]);
    code.extend([
        op("ADDSi") | ((shape.member_offset as u16 as i32) << 16),
        type_id,
    ]);
    code.extend([op("CALL"), function_id]);
    qword(&mut code, "OBJTYPE", bytecode_type_ptr);
    code.extend([op("COPY") | (8 << 16), type_id]);
    code.push(op("ALLOC"));
    code.push(type_ptr as u32 as i32);
    code.push((type_ptr as u64 >> 32) as u32 as i32);
    code.push(function_id);
    qword(&mut code, "FinConstruct", type_ptr);
    qword(&mut code, "CopyScript", type_ptr);
    if shape.include_resolve_object_ptr {
        code.push(op("ResolveObjectPtr"));
    }
    for opcode in ["FreeNullV8", "CpyVtoR1", "CmpPtrNull", "ThrowException"] {
        code.push(op(opcode));
    }
    if shape.include_reserved_fork_opcodes {
        qword(&mut code, "DestructScript", type_ptr);
        for opcode in ["TrackRef", "UntrackRef", "ValidateRef", "SaveReturnValue"] {
            code.push(op(opcode));
        }
    }
    code.extend([op("PshC4"), 123]);

    let mut writer = Writer::default();
    writer
        .bytes
        .extend(std::iter::repeat(shape.guid_byte).take(16));
    writer.i32(0x1234_5678);
    writer.i32(1);
    writer.fstring(module_name);
    writer.sia(module_name);
    writer.i32(1);
    function(
        &mut writer,
        "Probe",
        (1 + shape.id_delta) as u32,
        type_ptr,
        &code,
        true,
        shape.legacy_bytecode_reference,
    );

    let generated_class_case = module_name == "GeneratedClass";
    writer.i32(if generated_class_case { 2 } else { 1 }); // classes
    writer.sia(if generated_class_case {
        "UQualificationObject"
    } else {
        "FixtureClass"
    });
    writer.sia("FixtureNS");
    writer.i32(0x42);
    writer.i32(if generated_class_case { 2 } else { 1 });
    if generated_class_case {
        property_named(&mut writer, type_ptr, "ImplicitObject", false, false, false);
        property_named(&mut writer, type_ptr, "ImplicitScalar", false, false, false);
    } else {
        property(&mut writer, type_ptr);
    }
    writer.i32(1);
    function(
        &mut writer,
        "Method",
        (2 + shape.id_delta) as u32,
        type_ptr,
        &[],
        false,
        false,
    );
    writer.i32s(&[0]);
    writer.i64(type_ptr);
    writer.i64(0);
    writer.i32(1);
    function(
        &mut writer,
        "Ctor",
        (3 + shape.id_delta) as u32,
        type_ptr,
        &[],
        false,
        false,
    );
    writer.i64s(&[function_id as i64]);
    writer.i64s(&[function_id as i64, 0, 0, 0, 0, 0, 0]);
    writer.i32(1);
    function(
        &mut writer,
        "Dtor",
        (4 + shape.id_delta) as u32,
        type_ptr,
        &[],
        false,
        false,
    );
    writer.i32(1);
    writer.mark("behaviour_function_type");
    writer.i32(4);
    writer.bool4(true);
    writer.sia("Base");
    writer.sia("CodeBase");
    for index in 0..7 {
        writer.bool4(index % 2 == 0);
    }
    writer.sia("Game");
    writer.sia("GFixtureClass");
    writer.bool4(true);
    writer.i32(1);
    writer.marked_sia("class_metadata", "CMeta");
    writer.strings(&["CValue"]);
    writer.sia("Composition");

    if generated_class_case {
        writer.sia("FQualificationStruct");
        writer.sia("");
        writer.i32(0x42);
        writer.i32(2);
        property_named(&mut writer, type_ptr, "ImplicitObject", true, false, false);
        property_named(&mut writer, type_ptr, "ImplicitScalar", true, false, false);
        writer.i32(0); // methods
        writer.i32(0); // method table
        writer.i64(0); // derived from
        writer.i64(0); // shadow type
        writer.i32(0); // constructors
        writer.i32(0); // factory references
        writer.i32(0); // behaviour references
        writer.i32(0); // behaviour functions
        writer.i32(0); // behaviour function types
        writer.bool4(true);
        writer.sia("");
        writer.sia("");
        for _ in 0..7 {
            writer.bool4(false);
        }
        writer.sia("");
        writer.sia("");
        writer.bool4(true);
        writer.i32(0);
        writer.i32(0);
        writer.sia("");
    }

    writer.i32(1); // enums
    writer.sia("FixtureEnum");
    writer.sia("FixtureNS");
    writer.strings(&["First", "Second"]);
    writer.i32(2);
    writer.i32(1);
    writer.mark("enum_value");
    writer.i32(2);

    writer.i32(3); // globals
    writer.sia("Initialized");
    writer.sia("FixtureNS");
    data_type(&mut writer, type_ptr, 5);
    writer.bool4(false);
    writer.bool4(false);
    writer.bool4(true);
    function(
        &mut writer,
        "Init",
        (5 + shape.id_delta) as u32,
        type_ptr,
        &[],
        false,
        false,
    );
    writer.sia("Constant");
    writer.sia("FixtureNS");
    data_type(&mut writer, 0, 0x44);
    writer.bool4(false);
    writer.bool4(true);
    writer.mark("global_constant");
    writer.u64(42);
    writer.sia("Defaulted");
    writer.sia("FixtureNS");
    data_type(&mut writer, 0, 0x41);
    writer.bool4(true);

    writer.i32(1); // imports
    writer.sia("OtherModule");
    writer.i32("Imported".len() as i32);
    writer.mark("import_name");
    writer.bytes.extend_from_slice(b"Imported");
    writer.bytes.push(0);
    writer.sia("FixtureNS");
    writer.i32(1);
    data_type(&mut writer, type_ptr, 5);
    writer.i32s(&[1]);
    writer.strings(&["Default"]);
    data_type(&mut writer, type_ptr, 5);

    writer.mark("module_code_hash");
    writer.i64(0x1122_3344);
    writer.strings(&["OtherModule"]);
    writer.sia("FixtureStatics");
    writer.strings(&["Event"]);
    writer.strings(&["Delegate"]);
    writer.sia("Scripts/Fixture.as");
    writer.strings(&["PostInit"]);

    writer.i32(1); // T1
    writer.i64(type_ptr);
    writer.i32("FixtureClass".len() as i32);
    writer.mark("tail_t1_name");
    writer.bytes.extend_from_slice(b"FixtureClass");
    writer.bytes.push(0);
    writer.sia(module_name);
    writer.sia("FixtureNS");
    writer.i32(0);

    writer.i32(1); // T2
    writer.i32(type_id);
    writer.i64(type_ptr);

    writer.i32(1); // T3
    writer.i64(function_ptr);
    writer.sia("NativeCall");
    writer.sia("");
    writer.sia("FixtureNS");
    writer.mark("tail_t3_const");
    writer.bool4(false);
    writer.bool4(false);
    writer.bool4(true);
    writer.i64(type_ptr);
    writer.i32(0);
    data_type(&mut writer, 0, 0x52);

    writer.i32(if shape.extra_function_id_alias { 2 } else { 1 }); // T4
    writer.i32(function_id);
    writer.i64(function_ptr);
    if shape.extra_function_id_alias {
        writer.mark("tail_t4_alias_id");
        writer.i32(function_id + 1000);
        writer.i64(function_ptr);
    }

    writer.i32(2); // T5
    writer.i64(global_ptr);
    writer.i32("GlobalValue".len() as i32);
    writer.mark("tail_t5_name");
    writer.bytes.extend_from_slice(b"GlobalValue");
    writer.bytes.push(0);
    writer.sia(module_name);
    writer.sia("FixtureNS");
    writer.bool4(false);
    writer.i64(global_ptr + 1);
    writer.i32("Grüße_日本".len() as i32);
    writer.mark("tail_t5_string_name");
    writer.bytes.extend_from_slice("Grüße_日本".as_bytes());
    writer.bytes.push(0);
    writer.sia("");
    writer.sia("");
    writer.bool4(true);

    writer.i32(1); // T6
    writer.i32("StaticName".len() as i32);
    writer.mark("tail_t6_name");
    writer.bytes.extend_from_slice(b"StaticName");
    writer.bytes.push(0);

    let property_key =
        ((type_id as u32 as u64) << 1) | ((shape.member_offset as u32 as u64) << 33) | 1;
    writer.i32(1); // T7
    writer.i64(property_key as i64);
    writer.i32("Member".len() as i32);
    writer.mark("tail_t7_name");
    writer.bytes.extend_from_slice(b"Member");
    writer.bytes.push(0);
    writer.i32(type_id);

    Fixture {
        bytes: writer.bytes,
        marks: writer.marks,
    }
}

fn digest(bytes: &[u8]) -> [u8; 32] {
    *observe_whole_cache_semantics_v1(bytes, None)
        .unwrap()
        .sha256()
}

fn mutate_mark(fixture: &Fixture, mark: &'static str) -> Vec<u8> {
    let mut bytes = fixture.bytes.clone();
    bytes[fixture.marks[mark]] ^= 1;
    bytes
}

pub(crate) fn synthetic_observer_qualification_fixture_v1(
) -> SyntheticObserverQualificationFixtureV1 {
    let qualification_shape = FixtureShape {
        include_reserved_fork_opcodes: false,
        include_resolve_object_ptr: false,
        ..FixtureShape::default()
    };
    let fixture = build_fixture_with_module(qualification_shape, "CompleteModel");
    let mut semantic_mutations = [
        "function_traits",
        "bytecode_literal",
        "variable_space",
        "object_variable_position",
        "object_variables_on_heap",
        "var_info_option",
        "stack_needed",
        "declared_at",
        "line_number",
        "ufunction_metadata",
        "property_metadata",
        "class_metadata",
        "behaviour_function_type",
        "enum_value",
        "global_constant",
        "import_name",
        "module_code_hash",
        "tail_t1_name",
        "tail_t3_const",
        "tail_t5_name",
        "tail_t5_string_name",
        "tail_t6_name",
        "tail_t7_name",
    ]
    .into_iter()
    .map(|mark| (mark, mutate_mark(&fixture, mark)))
    .collect::<Vec<_>>();
    let mut t2_kind = qualification_shape;
    t2_kind.id_delta = 0x0400_0000;
    semantic_mutations.push((
        "tail_t2_kind",
        build_fixture_with_module(t2_kind, "CompleteModel").bytes,
    ));
    let mut t4_alias = qualification_shape;
    t4_alias.extra_function_id_alias = true;
    semantic_mutations.push((
        "tail_t4_alias",
        build_fixture_with_module(t4_alias, "CompleteModel").bytes,
    ));

    let mut unresolved = qualification_shape;
    unresolved.unresolved_bytecode_type = true;
    let mut legacy = qualification_shape;
    legacy.legacy_bytecode_reference = true;
    SyntheticObserverQualificationFixtureV1 {
        semantic_mutations,
        unresolved_runtime_reference: build_fixture_with_module(unresolved, "CompleteModel").bytes,
        legacy_bytecode_references: build_fixture_with_module(legacy, "CompleteModel").bytes,
    }
}

pub(crate) fn synthetic_observer_qualification_cache_for_module_v1(module_name: &str) -> Vec<u8> {
    build_fixture_with_module(
        FixtureShape {
            include_reserved_fork_opcodes: false,
            include_resolve_object_ptr: false,
            ..FixtureShape::default()
        },
        module_name,
    )
    .bytes
}

#[test]
fn whole_model_groups_and_every_tail_table_are_digest_visible() {
    let fixture = build_fixture(FixtureShape::default());
    let observation = observe_whole_cache_semantics_v1(&fixture.bytes, None).unwrap();
    let baseline = *observation.sha256();
    for opcode in 201..=212 {
        assert!(
            observation.opcode_count(opcode).unwrap() > 0,
            "opcode {opcode}"
        );
    }
    assert_eq!(observation.tail_table_counts(), &[1, 1, 1, 1, 2, 1, 1]);
    assert_eq!(observation.class_count(), 1);
    assert_eq!(observation.behaviour_function_count(), 1);
    assert_eq!(observation.property_count(), 1);
    assert_eq!(observation.global_count(), 3);
    assert_eq!(observation.initializer_function_count(), 1);
    assert_eq!(observation.string_global_reference_count(), 1);
    for mark in [
        "function_traits",
        "bytecode_literal",
        "variable_space",
        "object_variable_position",
        "object_variables_on_heap",
        "var_info_option",
        "stack_needed",
        "declared_at",
        "line_number",
        "ufunction_metadata",
        "property_metadata",
        "class_metadata",
        "behaviour_function_type",
        "enum_value",
        "global_constant",
        "import_name",
        "module_code_hash",
        "tail_t1_name",
        "tail_t3_const",
        "tail_t5_name",
        "tail_t5_string_name",
        "tail_t6_name",
        "tail_t7_name",
    ] {
        assert_ne!(digest(&mutate_mark(&fixture, mark)), baseline, "{mark}");
    }
    let mut t2_kind = FixtureShape::default();
    t2_kind.id_delta = 0x0400_0000;
    assert_ne!(
        digest(&build_fixture(t2_kind).bytes),
        baseline,
        "T2 object kind"
    );
    let mut t4 = FixtureShape::default();
    t4.extra_function_id_alias = true;
    assert_ne!(digest(&build_fixture(t4).bytes), baseline, "T4 row");
}

#[test]
fn pointer_id_property_offset_and_guid_drift_normalize_without_alignment_loss() {
    let baseline = digest(&build_fixture(FixtureShape::default()).bytes);
    let drifted = FixtureShape {
        pointer_delta: 0x5555,
        id_delta: 123,
        member_offset: 19,
        guid_byte: 0xee,
        ..FixtureShape::default()
    };
    assert_eq!(digest(&build_fixture(drifted).bytes), baseline);
}

#[test]
fn unresolved_legacy_and_invoke_contracts_fail_closed() {
    let mut unresolved = FixtureShape::default();
    unresolved.unresolved_bytecode_type = true;
    assert!(matches!(
        observe_whole_cache_semantics_v1(&build_fixture(unresolved).bytes, None),
        Err(SemanticObserverError::UnresolvedReference { .. })
    ));
    let mut legacy = FixtureShape::default();
    legacy.legacy_bytecode_reference = true;
    assert!(matches!(
        observe_whole_cache_semantics_v1(&build_fixture(legacy).bytes, None),
        Err(SemanticObserverError::UnsupportedByteCodeReferences { .. })
    ));

    let fixture = build_fixture(FixtureShape::default());
    let mut bad_opcode = fixture.bytes.clone();
    bad_opcode[fixture.marks["bytecode_opcode"]] = 0xff;
    assert!(matches!(
        observe_whole_cache_semantics_v1(&bad_opcode, None),
        Err(SemanticObserverError::InvalidBytecode { .. })
    ));
    let mut negative_module_count = fixture.bytes.clone();
    negative_module_count[20..24].copy_from_slice(&(-1i32).to_le_bytes());
    assert!(matches!(
        observe_whole_cache_semantics_v1(&negative_module_count, None),
        Err(SemanticObserverError::Wire(_))
    ));
    let mut trailing = fixture.bytes.clone();
    trailing.push(0);
    assert!(matches!(
        observe_whole_cache_semantics_v1(&trailing, None),
        Err(SemanticObserverError::TrailingBytes { .. })
    ));

    let mut alias_shape = FixtureShape::default();
    alias_shape.extra_function_id_alias = true;
    let alias = build_fixture(alias_shape);
    let mut duplicate_id = alias.bytes.clone();
    duplicate_id[alias.marks["tail_t4_alias_id"]..alias.marks["tail_t4_alias_id"] + 4]
        .copy_from_slice(&(77i32).to_le_bytes());
    assert!(matches!(
        observe_whole_cache_semantics_v1(&duplicate_id, None),
        Err(SemanticObserverError::DuplicateKey { .. })
    ));

    let cache = fixture;
    let first = CanonicalInvokeReturnV1::new(
        "Fixture::Result",
        CanonicalInvokeValueV1::Record(vec![
            (
                "b".into(),
                CanonicalInvokeValueV1::F64Bits(1.5f64.to_bits()),
            ),
            ("a".into(), CanonicalInvokeValueV1::I64(7)),
        ]),
    );
    let reordered = CanonicalInvokeReturnV1::new(
        "Fixture::Result",
        CanonicalInvokeValueV1::Record(vec![
            ("a".into(), CanonicalInvokeValueV1::I64(7)),
            (
                "b".into(),
                CanonicalInvokeValueV1::F64Bits(1.5f64.to_bits()),
            ),
        ]),
    );
    assert_eq!(
        observe_whole_cache_semantics_v1(&cache.bytes, Some(&first))
            .unwrap()
            .sha256(),
        observe_whole_cache_semantics_v1(&cache.bytes, Some(&reordered))
            .unwrap()
            .sha256(),
    );
    assert_ne!(
        observe_whole_cache_semantics_v1(&cache.bytes, Some(&first))
            .unwrap()
            .sha256(),
        observe_whole_cache_semantics_v1(&cache.bytes, None)
            .unwrap()
            .sha256(),
    );
    let duplicate = CanonicalInvokeReturnV1::new(
        "Fixture::Result",
        CanonicalInvokeValueV1::Record(vec![
            ("x".into(), CanonicalInvokeValueV1::Null),
            ("x".into(), CanonicalInvokeValueV1::Bool(false)),
        ]),
    );
    assert!(matches!(
        observe_whole_cache_semantics_v1(&cache.bytes, Some(&duplicate)),
        Err(SemanticObserverError::InvalidInvoke(_))
    ));
}

#[test]
fn configured_full_cache_sample_is_observable_when_present() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../work/probe/asghan-miniquest/public-v1/sandbox-game/G1R/Script/",
        "PrecompiledScript_Shipping.Cache"
    );
    let Ok(bytes) = std::fs::read(path) else {
        eprintln!("[skip] offline full-cache sample is absent: {path}");
        return;
    };
    let observation = observe_whole_cache_semantics_v1(&bytes, None)
        .unwrap_or_else(|error| panic!("observe offline full-cache sample: {error}"));
    assert!(observation.module_count() > 0);
    assert!(observation.function_count() > 0);
    assert_eq!(observation.sha256().len(), 32);
}
