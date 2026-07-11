use gore_as::cache::disasm::disassemble;
use gore_as::cache::header::CACHE_MAGIC;
use gore_as::cache::refs::RefResolver;
use gore_as::cache::remap::{
    remap_module_to_base, remap_module_to_base_with_options, RemapError, RemapOptions,
};
use gore_as::cache::splice::{
    extract_module, replace_module, splice_auto, SequentialMiniGuard, SpliceError,
};
use gore_as::cache::tables::parse_tail_tables;
use gore_as::cache::walk_modules::{
    collect_function_bytecodes, module_count, module_names, module_region_end,
};

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
    let mut out = vec![0u8; 24]; // six serialized bools
    out.extend_from_slice(&type_ptr.to_le_bytes());
    out.extend_from_slice(&token.to_le_bytes());
    out
}

fn type_row(key: i64, name: &str, module: &str, subtypes: &[i64]) -> Vec<u8> {
    type_row_ns(key, name, module, "", subtypes)
}

fn type_row_ns(key: i64, name: &str, module: &str, namespace: &str, subtypes: &[i64]) -> Vec<u8> {
    let mut out = key.to_le_bytes().to_vec();
    out.extend_from_slice(&sia(name));
    out.extend_from_slice(&sia(module));
    out.extend_from_slice(&sia(namespace));
    out.extend_from_slice(&(subtypes.len() as i32).to_le_bytes());
    for &ptr in subtypes {
        out.extend_from_slice(&datatype(ptr, 5));
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
    let mut out = key.to_le_bytes().to_vec();
    out.extend_from_slice(&sia(name));
    out.extend_from_slice(&sia(module));
    out.extend_from_slice(&sia(namespace));
    out.extend_from_slice(&0i32.to_le_bytes()); // const
    out.extend_from_slice(&0i32.to_le_bytes()); // imported
    let is_method = i32::from(owner != 0);
    out.extend_from_slice(&is_method.to_le_bytes());
    out.extend_from_slice(&owner.to_le_bytes());
    out.extend_from_slice(&(params.len() as i32).to_le_bytes());
    for &ptr in params {
        out.extend_from_slice(&datatype(ptr, 5));
    }
    out.extend_from_slice(&if ret == 0 {
        datatype(0, 0x40)
    } else {
        datatype(ret, 5)
    });
    out
}

fn global_row(key: i64, name: &str, module: &str) -> Vec<u8> {
    let mut out = key.to_le_bytes().to_vec();
    out.extend_from_slice(&sia(name));
    out.extend_from_slice(&sia(module));
    out.extend_from_slice(&sia(""));
    out.extend_from_slice(&0i32.to_le_bytes());
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

fn function(bytecode: &[i32]) -> Vec<u8> {
    let mut out = sia("Edited");
    out.extend_from_slice(&sia(""));
    out.extend_from_slice(&datatype(0, 0x40)); // void return
    out.extend_from_slice(&0i32.to_le_bytes()); // parameter types
    out.extend_from_slice(&0i32.to_le_bytes()); // parameter names
    out.extend_from_slice(&0i32.to_le_bytes()); // parameter flags
    out.extend_from_slice(&0i32.to_le_bytes()); // default args
    out.extend_from_slice(&0i32.to_le_bytes()); // traits
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
    out.extend_from_slice(&0x1234_5678i32.to_le_bytes()); // function-local hash/id (not T4)
    out.extend_from_slice(&0i32.to_le_bytes()); // declared at
    out.extend_from_slice(&0i32.to_le_bytes()); // line numbers
    out.extend_from_slice(&0i32.to_le_bytes()); // is UFunction
    out
}

fn class_record() -> Vec<u8> {
    let mut out = sia("EditedClass");
    out.extend_from_slice(&sia(""));
    out.extend_from_slice(&0i32.to_le_bytes()); // flags
    out.extend_from_slice(&0i32.to_le_bytes()); // properties
    out.extend_from_slice(&0i32.to_le_bytes()); // methods
    out.extend_from_slice(&2i32.to_le_bytes()); // MethodTable: local Methods[] indices
    out.extend_from_slice(&7i32.to_le_bytes());
    out.extend_from_slice(&(-1i32).to_le_bytes());
    out.extend_from_slice(&REGEN_TYPE_PTR.to_le_bytes()); // DerivedFrom: existing type
    out.extend_from_slice(&NEW_TYPE_PTR.to_le_bytes()); // ShadowType: new type
    out.extend_from_slice(&0i32.to_le_bytes()); // constructors
    out.extend_from_slice(&2i32.to_le_bytes()); // FactoryRefs (T4 ids)
    out.extend_from_slice(&(REGEN_FUNC_ID as i64).to_le_bytes());
    out.extend_from_slice(&(NEW_FUNC_ID as i64).to_le_bytes());
    out.extend_from_slice(&2i32.to_le_bytes()); // BehaviorRefs (T4 ids)
    out.extend_from_slice(&(REGEN_FUNC_ID as i64).to_le_bytes());
    out.extend_from_slice(&(NEW_FUNC_ID as i64).to_le_bytes());
    out.extend_from_slice(&0i32.to_le_bytes()); // behavior functions
    out.extend_from_slice(&0i32.to_le_bytes()); // behavior function types
    out.extend_from_slice(&0i32.to_le_bytes()); // has Unreal class data
    out
}

fn module_value(bytecode: &[i32], class: Option<&[u8]>) -> Vec<u8> {
    let mut out = sia(MODULE);
    out.extend_from_slice(&1i32.to_le_bytes());
    out.extend_from_slice(&function(bytecode));
    out.extend_from_slice(&(class.is_some() as i32).to_le_bytes());
    if let Some(class) = class {
        out.extend_from_slice(class);
    }
    for _ in 0..3 {
        out.extend_from_slice(&0i32.to_le_bytes()); // enums/globals/imports
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

fn cache_with_class(bytecode: &[i32], tables: Tables, class: Option<&[u8]>) -> Vec<u8> {
    let mut out = vec![0u8; 16];
    out.extend_from_slice(&CACHE_MAGIC.to_le_bytes());
    out.extend_from_slice(&1u32.to_le_bytes());
    out.extend_from_slice(&fstring(MODULE));
    out.extend_from_slice(&module_value(bytecode, class));
    append_table(&mut out, &tables.types);
    append_table(&mut out, &tables.type_ids);
    append_table(&mut out, &tables.funcs);
    append_table(&mut out, &tables.func_ids);
    append_table(&mut out, &tables.globals);
    append_table(&mut out, &tables.static_names);
    append_table(&mut out, &tables.properties);
    out
}

fn cache(bytecode: &[i32], tables: Tables) -> Vec<u8> {
    cache_with_class(bytecode, tables, None)
}

fn qw_op(opcode: i32, value: i64, out: &mut Vec<i32>) {
    out.push(opcode);
    out.push(value as u64 as u32 as i32);
    out.push(((value as u64) >> 32) as u32 as i32);
}

fn base_cache() -> Vec<u8> {
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
    cache(&[10], tables) // RET
}

fn regen_tables(existing_property: &str) -> Tables {
    Tables {
        types: vec![
            type_row(REGEN_TYPE_PTR, "ExistingType", MODULE, &[]),
            // NewType depends on ExistingType; the carried T1 row must rewrite this dependency.
            type_row(NEW_TYPE_PTR, "NewType", MODULE, &[REGEN_TYPE_PTR]),
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

fn regen_cache_with_existing_property(existing_property: &str) -> Vec<u8> {
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
    cache_with_class(&code, regen_tables(existing_property), Some(&class))
}

fn regen_cache() -> Vec<u8> {
    regen_cache_with_existing_property("ExistingField")
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
        regen_tables("ExistingField"),
    )
}

fn keyed_mini(table: usize, key: i64, label: &str) -> Vec<u8> {
    keyed_mini_with_value_delta(table, key, label, 0)
}

fn keyed_mini_with_value_delta(table: usize, key: i64, label: &str, delta: i64) -> Vec<u8> {
    let mut tables = Tables::default();
    match table {
        0 => tables.types.push(type_row(key, label, MODULE, &[])),
        1 => tables
            .type_ids
            .push(id_row(key as i32, key + 0x1000 + delta)),
        2 => tables.funcs.push(func_row(key, label, MODULE, 0, &[], 0)),
        3 => tables
            .func_ids
            .push(id_row(key as i32, key + 0x2000 + delta)),
        4 => tables.globals.push(global_row(key, label, MODULE)),
        6 => tables.properties.push(property_row(key as i32, 4, label)),
        _ => unreachable!(),
    }
    cache(&[10], tables)
}

#[test]
fn sequential_guard_rejects_collisions_in_every_keyed_tail_table() {
    for table in [0usize, 1, 2, 3, 4, 6] {
        let base = cache(&[10], Tables::default());
        let mut guard = SequentialMiniGuard::new(&base).unwrap();
        guard
            .check_and_record(&keyed_mini(table, 0x55, "First"))
            .unwrap();
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
        let mini = keyed_mini(table, 0x66, "SharedSymbol");
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
    let mut base = base_cache();
    replace_ascii_same_len(&mut base, "EditedModule", "PristineBase");
    let regen_a = regen_cache();
    let mut regen_b = regen_cache();
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
    let prepared_a = guard.check_and_record(&mini_a).unwrap();
    let prepared_b = guard.check_and_record(&mini_b).unwrap();
    let after_a = splice_auto(&base, &prepared_a).unwrap();
    let combined = splice_auto(&after_a, &prepared_b).unwrap();
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
fn flagged_typeid_alone_declares_and_carries_external_new_type() {
    const PTR: i64 = 0x7777;
    const ID: i32 = 0x0800_7777;
    let base = cache(&[10], Tables::default());
    let regen = cache(
        &[76, ID | 0x4000_0000, 10],
        Tables {
            types: vec![type_row(PTR, "ExternalNewType", "ExternalModule", &[])],
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
    let regen = cache(
        &code,
        Tables {
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
        },
    );

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
    assert_eq!(counts.embed_type_ptr, 2, "DerivedFrom + ShadowType");
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
    type_ref_sequence.extend_from_slice(&7i32.to_le_bytes());
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
    let occurrences = module_bytes
        .windows(function_ref_sequence.len())
        .filter(|w| *w == function_ref_sequence)
        .count();
    assert_eq!(occurrences, 2, "FactoryRefs and BehaviorRefs both remapped");

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
    let fixture_first = fixture_guard.check_and_record(&fixture_minis[0]).unwrap();
    let fixture_second = fixture_guard.check_and_record(&fixture_minis[1]).unwrap();
    let fixture_combined = splice_auto(
        &splice_auto(&base, &fixture_first).unwrap(),
        &fixture_second,
    )
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
    let mut guard = SequentialMiniGuard::new(&base).unwrap();
    let first = guard.check_and_record(&minis[0]).unwrap();
    let second = guard.check_and_record(&minis[1]).unwrap();
    let combined = splice_auto(&splice_auto(&base, &first).unwrap(), &second).unwrap();
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
}
