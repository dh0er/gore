use std::collections::{HashMap, HashSet};

use super::*;
use crate::cache::header::CACHE_MAGIC;

const SCRIPT_OBJECT_KIND: u32 = 0x0800_0000;
const APP_OBJECT_KIND: u32 = 0x0400_0000;

#[derive(Default)]
struct TailRows {
    types: Vec<Vec<u8>>,
    type_ids: Vec<Vec<u8>>,
    funcs: Vec<Vec<u8>>,
    func_ids: Vec<Vec<u8>>,
    globals: Vec<Vec<u8>>,
    static_names: Vec<Vec<u8>>,
    properties: Vec<Vec<u8>>,
}

fn sia(value: &str) -> Vec<u8> {
    if value.is_empty() {
        return 0i32.to_le_bytes().to_vec();
    }
    let mut out = (value.len() as i32).to_le_bytes().to_vec();
    out.extend_from_slice(value.as_bytes());
    out.push(0);
    out
}

fn fstring(value: &str) -> Vec<u8> {
    let mut out = ((value.len() + 1) as i32).to_le_bytes().to_vec();
    out.extend_from_slice(value.as_bytes());
    out.push(0);
    out
}

fn datatype(type_ptr: i64, token: i32) -> Vec<u8> {
    let mut out = vec![0u8; 6 * 4];
    out.extend_from_slice(&type_ptr.to_le_bytes());
    out.extend_from_slice(&token.to_le_bytes());
    out
}

fn type_row(key: i64, name: &str, module: &str) -> Vec<u8> {
    let mut out = key.to_le_bytes().to_vec();
    out.extend_from_slice(&sia(name));
    out.extend_from_slice(&sia(module));
    out.extend_from_slice(&sia(""));
    out.extend_from_slice(&0i32.to_le_bytes()); // subtypes
    out
}

fn id_row(id: i32, ptr: i64) -> Vec<u8> {
    let mut out = id.to_le_bytes().to_vec();
    out.extend_from_slice(&ptr.to_le_bytes());
    out
}

fn string_global_row(key: i64, value: &str) -> Vec<u8> {
    let mut out = key.to_le_bytes().to_vec();
    out.extend_from_slice(&sia(value));
    out.extend_from_slice(&sia("")); // string literal lookup ignores Module
    out.extend_from_slice(&sia(""));
    out.extend_from_slice(&1i32.to_le_bytes());
    out
}

fn nonstring_global_row(key: i64, name: &str, module: &str) -> Vec<u8> {
    let mut out = key.to_le_bytes().to_vec();
    out.extend_from_slice(&sia(name));
    out.extend_from_slice(&sia(module));
    out.extend_from_slice(&sia(""));
    out.extend_from_slice(&0i32.to_le_bytes());
    out
}

fn property_row(type_id: i32, member_offset: i32, name: &str) -> Vec<u8> {
    let key = ((type_id as u32 as u64) << 1) | ((member_offset as u32 as u64) << 33) | 1;
    let mut out = (key as i64).to_le_bytes().to_vec();
    out.extend_from_slice(&sia(name));
    out.extend_from_slice(&type_id.to_le_bytes());
    out
}

fn module_global_record(name: &str) -> Vec<u8> {
    let mut out = sia(name);
    out.extend_from_slice(&sia(""));
    out.extend_from_slice(&datatype(0, 0x52));
    out.extend_from_slice(&1i32.to_le_bytes()); // default initialization, no payload
    out
}

fn function_tail_row(key: i64, name: &str, module: &str, params: &[i64]) -> Vec<u8> {
    let mut out = key.to_le_bytes().to_vec();
    out.extend_from_slice(&sia(name));
    out.extend_from_slice(&sia(module));
    out.extend_from_slice(&sia(""));
    out.extend_from_slice(&0i32.to_le_bytes()); // const
    out.extend_from_slice(&0i32.to_le_bytes()); // imported
    out.extend_from_slice(&0i32.to_le_bytes()); // method
    out.extend_from_slice(&0i64.to_le_bytes()); // owner
    out.extend_from_slice(&(params.len() as i32).to_le_bytes());
    for &param in params {
        out.extend_from_slice(&datatype(param, 5));
    }
    out.extend_from_slice(&datatype(0, 0x52)); // void return
    out
}

fn function_record(name: &str, params: &[i64], runtime_id: i32) -> Vec<u8> {
    function_record_with_code(name, params, runtime_id, &[10])
}

fn function_record_with_code(name: &str, params: &[i64], runtime_id: i32, code: &[i32]) -> Vec<u8> {
    let mut out = sia(name);
    out.extend_from_slice(&sia(""));
    out.extend_from_slice(&datatype(0, 0x52)); // void return
    out.extend_from_slice(&(params.len() as i32).to_le_bytes());
    for &param in params {
        out.extend_from_slice(&datatype(param, 5));
    }
    out.extend_from_slice(&(params.len() as i32).to_le_bytes());
    for index in 0..params.len() {
        out.extend_from_slice(&sia(&format!("parameter{index}")));
    }
    out.extend_from_slice(&(params.len() as i32).to_le_bytes());
    for _ in params {
        out.extend_from_slice(&0i32.to_le_bytes());
    }
    out.extend_from_slice(&(params.len() as i32).to_le_bytes());
    for _ in params {
        out.extend_from_slice(&sia(""));
    }
    out.extend_from_slice(&0i32.to_le_bytes()); // traits
    out.extend_from_slice(&(code.len() as i32).to_le_bytes());
    for dword in code {
        out.extend_from_slice(&dword.to_le_bytes());
    }
    out.extend_from_slice(&0i32.to_le_bytes()); // bytecode refs
    out.extend_from_slice(&0i32.to_le_bytes()); // variable space
    out.extend_from_slice(&0i32.to_le_bytes()); // object variable types
    out.extend_from_slice(&0i32.to_le_bytes()); // object variable positions
    out.extend_from_slice(&0i32.to_le_bytes()); // object vars on heap
    out.extend_from_slice(&0i32.to_le_bytes()); // var-info program positions
    out.extend_from_slice(&0i32.to_le_bytes()); // var-info offsets
    out.extend_from_slice(&0i32.to_le_bytes()); // var-info options
    out.extend_from_slice(&0i32.to_le_bytes()); // stack needed
    out.extend_from_slice(&runtime_id.to_le_bytes());
    out.extend_from_slice(&0i32.to_le_bytes()); // declared at
    out.extend_from_slice(&0i32.to_le_bytes()); // line numbers
    out.extend_from_slice(&0i32.to_le_bytes()); // no Unreal metadata
    out
}

fn class_record(name: &str) -> Vec<u8> {
    let mut out = sia(name);
    out.extend_from_slice(&sia(""));
    out.extend_from_slice(&0i32.to_le_bytes()); // flags
    out.extend_from_slice(&0i32.to_le_bytes()); // properties
    out.extend_from_slice(&0i32.to_le_bytes()); // methods
    out.extend_from_slice(&0i32.to_le_bytes()); // method table
    out.extend_from_slice(&0i64.to_le_bytes()); // derived from
    out.extend_from_slice(&0i64.to_le_bytes()); // shadow type
    out.extend_from_slice(&0i32.to_le_bytes()); // constructors
    out.extend_from_slice(&0i32.to_le_bytes()); // factory refs
    out.extend_from_slice(&7i32.to_le_bytes()); // behavior refs
    for _ in 0..7 {
        out.extend_from_slice(&0i64.to_le_bytes());
    }
    out.extend_from_slice(&0i32.to_le_bytes()); // behavior functions
    out.extend_from_slice(&0i32.to_le_bytes()); // behavior types
    out.extend_from_slice(&0i32.to_le_bytes()); // no Unreal metadata
    out
}

fn class_record_with_factory_ref(name: &str, function_id: i64) -> Vec<u8> {
    let mut out = sia(name);
    out.extend_from_slice(&sia(""));
    out.extend_from_slice(&0i32.to_le_bytes()); // flags
    out.extend_from_slice(&0i32.to_le_bytes()); // properties
    out.extend_from_slice(&0i32.to_le_bytes()); // methods
    out.extend_from_slice(&0i32.to_le_bytes()); // method table
    out.extend_from_slice(&0i64.to_le_bytes()); // derived from
    out.extend_from_slice(&0i64.to_le_bytes()); // shadow type
    out.extend_from_slice(&0i32.to_le_bytes()); // constructors
    out.extend_from_slice(&1i32.to_le_bytes()); // factory refs
    out.extend_from_slice(&function_id.to_le_bytes());
    out.extend_from_slice(&7i32.to_le_bytes()); // behavior refs
    for _ in 0..7 {
        out.extend_from_slice(&0i64.to_le_bytes());
    }
    out.extend_from_slice(&0i32.to_le_bytes()); // behavior functions
    out.extend_from_slice(&0i32.to_le_bytes()); // behavior types
    out.extend_from_slice(&0i32.to_le_bytes()); // no Unreal metadata
    out
}

fn append_rows(out: &mut Vec<u8>, rows: &[Vec<u8>]) {
    out.extend_from_slice(&(rows.len() as u32).to_le_bytes());
    for row in rows {
        out.extend_from_slice(row);
    }
}

fn cache(module: &str, functions: &[Vec<u8>], classes: &[Vec<u8>], rows: TailRows) -> Vec<u8> {
    cache_with_module_globals(module, functions, classes, &[], rows)
}

fn cache_with_module_globals(
    module: &str,
    functions: &[Vec<u8>],
    classes: &[Vec<u8>],
    module_globals: &[Vec<u8>],
    rows: TailRows,
) -> Vec<u8> {
    cache_with_module_identity(module, module, functions, classes, module_globals, rows)
}

fn cache_with_module_identity(
    outer_module: &str,
    runtime_module: &str,
    functions: &[Vec<u8>],
    classes: &[Vec<u8>],
    module_globals: &[Vec<u8>],
    rows: TailRows,
) -> Vec<u8> {
    let mut out = vec![0u8; 16];
    out.extend_from_slice(&CACHE_MAGIC.to_le_bytes());
    out.extend_from_slice(&1u32.to_le_bytes());
    out.extend_from_slice(&fstring(outer_module));
    out.extend_from_slice(&sia(runtime_module));
    out.extend_from_slice(&(functions.len() as i32).to_le_bytes());
    for function in functions {
        out.extend_from_slice(function);
    }
    out.extend_from_slice(&(classes.len() as i32).to_le_bytes());
    for class in classes {
        out.extend_from_slice(class);
    }
    out.extend_from_slice(&0i32.to_le_bytes()); // enums
    out.extend_from_slice(&(module_globals.len() as i32).to_le_bytes());
    for global in module_globals {
        out.extend_from_slice(global);
    }
    out.extend_from_slice(&0i32.to_le_bytes()); // imports
    out.extend_from_slice(&0i64.to_le_bytes()); // code hash
    out.extend_from_slice(&0i32.to_le_bytes()); // imported modules
    out.extend_from_slice(&sia("")); // statics class
    out.extend_from_slice(&0i32.to_le_bytes()); // events
    out.extend_from_slice(&0i32.to_le_bytes()); // delegates
    out.extend_from_slice(&sia(&format!("{outer_module}.as")));
    out.extend_from_slice(&0i32.to_le_bytes()); // post-init functions
    append_rows(&mut out, &rows.types);
    append_rows(&mut out, &rows.type_ids);
    append_rows(&mut out, &rows.funcs);
    append_rows(&mut out, &rows.func_ids);
    append_rows(&mut out, &rows.globals);
    append_rows(&mut out, &rows.static_names);
    append_rows(&mut out, &rows.properties);
    out
}

fn empty_base() -> Vec<u8> {
    cache("Pristine", &[], &[], TailRows::default())
}

#[derive(Clone)]
struct SymbolMiniSpec<'a> {
    module: &'a str,
    type_name: &'a str,
    function_name: &'a str,
    type_ptr: i64,
    function_ptr: i64,
    type_id: i32,
    function_id: i32,
    runtime_id: i32,
}

fn symbol_mini(spec: &SymbolMiniSpec<'_>) -> Vec<u8> {
    symbol_mini_with_code(spec, &[10])
}

fn symbol_mini_with_code(spec: &SymbolMiniSpec<'_>, code: &[i32]) -> Vec<u8> {
    cache(
        spec.module,
        &[function_record_with_code(
            spec.function_name,
            &[],
            spec.runtime_id,
            code,
        )],
        &[class_record(spec.type_name)],
        TailRows {
            types: vec![type_row(spec.type_ptr, spec.type_name, spec.module)],
            type_ids: vec![id_row(spec.type_id, spec.type_ptr)],
            funcs: vec![function_tail_row(
                spec.function_ptr,
                spec.function_name,
                spec.module,
                &[],
            )],
            func_ids: vec![id_row(spec.function_id, spec.function_ptr)],
            ..TailRows::default()
        },
    )
}

fn string_global_mini(
    module: &str,
    function_name: &str,
    function_ptr: i64,
    function_id: i32,
    global_keys: &[i64],
    value: &str,
) -> Vec<u8> {
    let mut code = Vec::new();
    for &key in global_keys {
        code.push(1); // PshGPtr
        code.push(key as u32 as i32);
        code.push((key as u64 >> 32) as u32 as i32);
    }
    code.push(10); // RET
    cache(
        module,
        &[function_record_with_code(
            function_name,
            &[],
            function_id,
            &code,
        )],
        &[],
        TailRows {
            funcs: vec![function_tail_row(function_ptr, function_name, module, &[])],
            func_ids: vec![id_row(function_id, function_ptr)],
            globals: global_keys
                .iter()
                .map(|&key| string_global_row(key, value))
                .collect(),
            ..TailRows::default()
        },
    )
}

fn analyzed_identities(base: &AllowNewBaseContext, mini: &[u8]) -> NovelIdentitySet {
    preflight_mini_module_work(mini).unwrap();
    let analyzed = analyze_new_symbol_mini(mini, base).unwrap();
    novel_identity_set(
        &analyzed.plan,
        &analyzed.meta,
        &analyzed.regen,
        &analyzed.spans,
    )
    .unwrap()
}

#[derive(Debug, PartialEq, Eq)]
struct AssignedIds {
    type_ptr: i64,
    type_id: i32,
    function_ptr: i64,
    function_id: i32,
}

fn assigned_ids(mini: &[u8]) -> AssignedIds {
    let meta = TailMetadata::build(mini).unwrap();
    assert_eq!(meta.types.len(), 1);
    assert_eq!(meta.type_ids.len(), 1);
    assert_eq!(meta.funcs.len(), 1);
    assert_eq!(meta.func_ids.len(), 1);
    AssignedIds {
        type_ptr: meta.types[0].key,
        type_id: meta.type_ids[0].id,
        function_ptr: meta.funcs[0].key,
        function_id: meta.func_ids[0].id,
    }
}

fn module_function_ids(mini: &[u8]) -> Vec<i32> {
    collect_module_spans(mini).unwrap().function_ids
}

fn global_ptr_operands(mini: &[u8]) -> Vec<i64> {
    let spans = collect_module_spans(mini).unwrap();
    let mut pointers = Vec::new();
    for span in spans.code {
        let code: Vec<i32> = (0..span.count)
            .map(|index| {
                let off = span.data_off + index * 4;
                i32::from_le_bytes(mini[off..off + 4].try_into().unwrap())
            })
            .collect();
        for instruction in disassemble(&code).unwrap() {
            if instruction.op.name == "PshGPtr" {
                pointers.push(read_qw(&code, instruction.offset_dw + 1));
            }
        }
    }
    pointers
}

fn small_domains() -> CanonicalAllocationDomains {
    CanonicalAllocationDomains {
        pointer_slot_high: 63,
        type_sequence_high: 15,
        function_id_high: 4,
    }
}

#[test]
fn exact_base_keys_are_preserved_even_when_portable_identity_has_aliases() {
    const TYPE_KEY: i64 = 0x10_100;
    const FUNC_KEY: i64 = 0x20_100;
    const GLOBAL_KEY: i64 = 0x30_100;
    let base = cache(
        "Pristine",
        &[],
        &[],
        TailRows {
            types: vec![
                type_row(TYPE_KEY, "AliasType", ""),
                type_row(TYPE_KEY + 8, "AliasType", ""),
            ],
            funcs: vec![
                function_tail_row(FUNC_KEY, "AliasFunction", "Pristine", &[]),
                function_tail_row(FUNC_KEY + 8, "AliasFunction", "Pristine", &[]),
            ],
            globals: vec![
                nonstring_global_row(GLOBAL_KEY, "AliasGlobal", "Pristine"),
                nonstring_global_row(GLOBAL_KEY + 8, "AliasGlobal", "Pristine"),
            ],
            ..TailRows::default()
        },
    );
    let regen = cache(
        "Prepared",
        &[],
        &[],
        TailRows {
            types: vec![type_row(TYPE_KEY, "AliasType", "")],
            funcs: vec![function_tail_row(
                FUNC_KEY,
                "AliasFunction",
                "Pristine",
                &[],
            )],
            globals: vec![nonstring_global_row(GLOBAL_KEY, "AliasGlobal", "Pristine")],
            ..TailRows::default()
        },
    );
    let base_syms = SymTables::build(&base).unwrap();
    let regen_syms = SymTables::build(&regen).unwrap();
    let summaries = SymbolIdentitySummaries {
        types: IdentityReverseSummary::build(&base_syms.type_ident_of_ptr).unwrap(),
        functions: IdentityReverseSummary::build(&base_syms.func_ident_of_ptr).unwrap(),
        globals: IdentityReverseSummary::build(&base_syms.global_ident_of_ptr).unwrap(),
    };
    let mut comparisons = IdentityComparisonBudget::new(base.len() + regen.len());
    let mut plan = NewSymbolPlan::default();
    declare_type(
        &mut plan,
        TYPE_KEY,
        "test",
        &regen_syms,
        &base_syms,
        &summaries,
        &mut comparisons,
    )
    .unwrap();
    declare_func(
        &mut plan,
        FUNC_KEY,
        "test",
        &regen_syms,
        &base_syms,
        &summaries,
        &mut comparisons,
    )
    .unwrap();
    declare_global(
        &mut plan,
        GLOBAL_KEY,
        "test",
        &regen_syms,
        &base_syms,
        &summaries,
        &mut comparisons,
    )
    .unwrap();
    assert_eq!(plan.type_ptrs.get(&TYPE_KEY), Some(&TYPE_KEY));
    assert_eq!(plan.func_ptrs.get(&FUNC_KEY), Some(&FUNC_KEY));
    assert_eq!(plan.global_ptrs.get(&GLOBAL_KEY), Some(&GLOBAL_KEY));
}

#[test]
fn reverse_id_aliases_preserve_exact_t2_and_nonzero_t4_operands() {
    const TYPE_PTR: i64 = 0x40_100;
    const FUNC_PTR: i64 = 0x50_100;
    const TYPE_ID: i32 = (SCRIPT_OBJECT_KIND | 500) as i32;
    const TYPE_ALIAS: i32 = (SCRIPT_OBJECT_KIND | 501) as i32;
    const TYPE_REPACKED: i32 = (SCRIPT_OBJECT_KIND | 502) as i32;
    const WRONG_KIND_ALIAS: i32 = (APP_OBJECT_KIND | 499) as i32;
    const FUNC_ID: i32 = 600;
    let base = cache(
        "Pristine",
        &[],
        &[],
        TailRows {
            types: vec![type_row(TYPE_PTR, "AliasType", "")],
            type_ids: vec![
                id_row(TYPE_ID, TYPE_PTR),
                id_row(TYPE_ALIAS, TYPE_PTR),
                id_row(WRONG_KIND_ALIAS, TYPE_PTR),
            ],
            funcs: vec![function_tail_row(
                FUNC_PTR,
                "AliasFunction",
                "Pristine",
                &[],
            )],
            func_ids: vec![id_row(FUNC_ID, FUNC_PTR), id_row(0, FUNC_PTR)],
            ..TailRows::default()
        },
    );
    let regen = cache(
        "Prepared",
        &[],
        &[],
        TailRows {
            types: vec![type_row(TYPE_PTR + 8, "AliasType", "")],
            type_ids: vec![
                id_row(TYPE_ID, TYPE_PTR + 8),
                id_row(TYPE_REPACKED, TYPE_PTR + 8),
            ],
            funcs: vec![function_tail_row(
                FUNC_PTR + 8,
                "AliasFunction",
                "Pristine",
                &[],
            )],
            func_ids: vec![id_row(FUNC_ID, FUNC_PTR + 8)],
            ..TailRows::default()
        },
    );
    let mut code = vec![76, TYPE_ID, 76, TYPE_REPACKED, 9, FUNC_ID, 10];
    remap_bytecode(
        &mut code,
        &SymTables::build(&regen).unwrap(),
        &SymTables::build(&base).unwrap(),
    )
    .unwrap();
    assert_eq!(code, [76, TYPE_ID, 76, TYPE_ID, 9, FUNC_ID, 10]);
}

#[test]
fn nonzero_function_reference_never_falls_back_to_a_zero_only_t4_alias() {
    const BASE_PTR: i64 = 0x51_100;
    const REGEN_PTR: i64 = 0x51_200;
    const REGEN_ID: i32 = 601;
    let base = cache(
        "Pristine",
        &[],
        &[],
        TailRows {
            funcs: vec![function_tail_row(
                BASE_PTR,
                "ZeroOnlyAlias",
                "Pristine",
                &[],
            )],
            func_ids: vec![id_row(0, BASE_PTR)],
            ..TailRows::default()
        },
    );
    let regen = cache(
        "Prepared",
        &[],
        &[],
        TailRows {
            funcs: vec![function_tail_row(
                REGEN_PTR,
                "ZeroOnlyAlias",
                "Pristine",
                &[],
            )],
            func_ids: vec![id_row(REGEN_ID, REGEN_PTR)],
            ..TailRows::default()
        },
    );
    let mut code = vec![9, REGEN_ID, 10];
    assert!(matches!(
        remap_bytecode(
            &mut code,
            &SymTables::build(&regen).unwrap(),
            &SymTables::build(&base).unwrap(),
        ),
        Err(RemapError::Unresolved {
            kind: "function-id(no base id)",
            ..
        })
    ));
    assert_eq!(code, [9, REGEN_ID, 10]);
}

#[test]
fn pristine_property_lookup_uses_semantic_index_without_alias_bucket_scan() {
    const TYPE_PTR: i64 = 0x60_100;
    const TYPE_ID: i32 = (APP_OBJECT_KIND | 610) as i32;
    let base = cache(
        "Pristine",
        &[],
        &[],
        TailRows {
            types: vec![
                type_row(TYPE_PTR, "EngineAlias", ""),
                type_row(TYPE_PTR + 8, "EngineAlias", ""),
            ],
            type_ids: vec![id_row(TYPE_ID, TYPE_PTR)],
            properties: vec![property_row(TYPE_ID, 24, "Health")],
            ..TailRows::default()
        },
    );
    let meta = TailMetadata::build(&base).unwrap();
    let mut syms = SymTables::build(&base).unwrap();
    let mut comparisons = IdentityComparisonBudget::new(base.len() + syms.identity_bytes);
    let collected =
        collect_declaration_inventory(&base, &syms, None, None, &meta, &mut comparisons).unwrap();
    let authority =
        PristineDeclarationAuthority::build(&meta, &syms, collected.declarations, &mut comparisons)
            .unwrap();

    // The semantic property index is complete now; the large pointer-alias reverse bucket is no
    // longer part of the lookup path.
    syms.type_ptr_of_id.clear();
    let prepared = cache(
        "Prepared",
        &[],
        &[],
        TailRows {
            properties: vec![property_row(TYPE_ID, 24, "Health")],
            ..TailRows::default()
        },
    );
    let prepared_meta = TailMetadata::build(&prepared).unwrap();
    let prepared_syms = SymTables::build(&prepared).unwrap();
    assert!(has_pristine_property_identity(
        &prepared_meta.properties[0],
        &prepared_syms,
        &syms,
        &authority,
    ));
}

fn collision_pair(base: &[u8]) -> (Vec<u8>, Vec<u8>) {
    let builder = LoadoutScriptIdPlanBuilder::new_with_config(
        base,
        PRODUCTION_LOADOUT_PLAN_LIMITS,
        small_domains(),
    )
    .unwrap();
    let mut buckets: HashMap<(u64, u64), Vec<u8>> = HashMap::new();
    for index in 0..=32 {
        let module = format!("CollisionMod{index}");
        let type_name = format!("CollisionType{index}");
        let function_name = format!("CollisionFunction{index}");
        let mini = symbol_mini(&SymbolMiniSpec {
            module: &module,
            type_name: &type_name,
            function_name: &function_name,
            type_ptr: 0x1000,
            function_ptr: 0x2000,
            type_id: (SCRIPT_OBJECT_KIND | 12) as i32,
            function_id: 1,
            runtime_id: 10_000 + index,
        });
        let identities = analyzed_identities(&builder.base, &mini);
        let (type_identity, _) = identities.type_ids.first_key_value().unwrap();
        let function_identity = identities.function_ids.first().unwrap();
        let bucket = (
            type_sequence_start(type_identity, small_domains().type_sequence_high),
            function_id_start(function_identity, small_domains().function_id_high),
        );
        if let Some(prior) = buckets.insert(bucket, mini.clone()) {
            return (prior, mini);
        }
    }
    panic!("pigeonhole collision was not found")
}

#[test]
fn incremental_plan_separates_colliding_t2_and_t4_starts() {
    let base = empty_base();
    let (first, second) = collision_pair(&base);
    let base_context = build_allow_new_base_context(&base).unwrap();
    let first_identities = analyzed_identities(&base_context, &first);
    let second_identities = analyzed_identities(&base_context, &second);
    let (first_type, _) = first_identities.type_ids.first_key_value().unwrap();
    let (second_type, _) = second_identities.type_ids.first_key_value().unwrap();
    assert_eq!(
        type_sequence_start(first_type, small_domains().type_sequence_high),
        type_sequence_start(second_type, small_domains().type_sequence_high)
    );
    assert_eq!(
        function_id_start(
            first_identities.function_ids.first().unwrap(),
            small_domains().function_id_high,
        ),
        function_id_start(
            second_identities.function_ids.first().unwrap(),
            small_domains().function_id_high,
        )
    );

    let mut builder = LoadoutScriptIdPlanBuilder::new_with_config(
        &base,
        PRODUCTION_LOADOUT_PLAN_LIMITS,
        small_domains(),
    )
    .unwrap();
    builder.inspect(&first).unwrap();
    builder.inspect(&second).unwrap();
    let plan = builder.finish().unwrap();
    let first_out = remap_module_to_base_with_loadout_plan(&first, &base, &plan)
        .unwrap()
        .0;
    let second_out = remap_module_to_base_with_loadout_plan(&second, &base, &plan)
        .unwrap()
        .0;
    let first_ids = assigned_ids(&first_out);
    let second_ids = assigned_ids(&second_out);
    assert_ne!(first_ids.type_ptr, second_ids.type_ptr);
    assert_ne!(first_ids.function_ptr, second_ids.function_ptr);
    assert_ne!(first_ids.type_id, second_ids.type_id);
    assert_ne!(first_ids.function_id, second_ids.function_id);
}

#[test]
fn builder_and_strict_validator_share_one_pristine_base_context() {
    let base = empty_base();
    let builder = LoadoutScriptIdPlanBuilder::new(&base).unwrap();
    assert!(Arc::ptr_eq(&builder.base, &builder.effective_base.base));
    let plan = builder.finish().unwrap();
    assert!(Arc::ptr_eq(&plan.base, &plan.effective_base.base));
}

#[test]
fn identity_union_deduplicates_and_ignores_inspection_order_and_raw_packaging() {
    let base = empty_base();
    let shared = SymbolMiniSpec {
        module: "SharedMod",
        type_name: "SharedType",
        function_name: "SharedFunction",
        type_ptr: 0x1110,
        function_ptr: 0x2110,
        type_id: (SCRIPT_OBJECT_KIND | 100) as i32,
        function_id: 100,
        runtime_id: 300,
    };
    let shared_repacked = SymbolMiniSpec {
        type_ptr: 0x7770,
        function_ptr: 0x8880,
        type_id: (SCRIPT_OBJECT_KIND | 777) as i32,
        function_id: 777,
        runtime_id: 301,
        ..shared.clone()
    };
    let other = SymbolMiniSpec {
        module: "OtherMod",
        type_name: "OtherType",
        function_name: "OtherFunction",
        type_ptr: 0x3110,
        function_ptr: 0x4110,
        type_id: (SCRIPT_OBJECT_KIND | 200) as i32,
        function_id: 200,
        runtime_id: 400,
    };
    let shared = symbol_mini(&shared);
    let shared_repacked = symbol_mini(&shared_repacked);
    let other = symbol_mini(&other);

    let mut forward = LoadoutScriptIdPlanBuilder::new(&base).unwrap();
    forward.inspect(&shared).unwrap();
    forward.inspect(&other).unwrap();
    let forward = forward.finish().unwrap();

    let mut reversed = LoadoutScriptIdPlanBuilder::new(&base).unwrap();
    reversed.inspect(&other).unwrap();
    reversed.inspect(&shared_repacked).unwrap();
    let reversed = reversed.finish().unwrap();
    assert_eq!(forward.pointer_assignments, reversed.pointer_assignments);
    assert_eq!(forward.type_id_assignments, reversed.type_id_assignments);
    assert_eq!(
        forward.function_id_assignments,
        reversed.function_id_assignments
    );

    let mut deduplicated = LoadoutScriptIdPlanBuilder::new(&base).unwrap();
    deduplicated.inspect(&shared).unwrap();
    deduplicated.inspect(&shared_repacked).unwrap();
    let deduplicated = deduplicated.finish().unwrap();
    assert_eq!(deduplicated.pointer_assignments.len(), 2);
    assert_eq!(deduplicated.type_id_assignments.len(), 1);
    assert_eq!(deduplicated.function_id_assignments.len(), 1);
    let shared_out = remap_module_to_base_with_loadout_plan(&shared, &base, &deduplicated)
        .unwrap()
        .0;
    let repacked_out =
        remap_module_to_base_with_loadout_plan(&shared_repacked, &base, &deduplicated)
            .unwrap()
            .0;
    assert_eq!(assigned_ids(&shared_out), assigned_ids(&repacked_out));
    assert_eq!(
        module_function_ids(&shared_out),
        module_function_ids(&repacked_out)
    );
}

#[test]
fn loadout_rekeys_colliding_module_function_ids_before_sequential_composition() {
    let base = empty_base();
    let first = symbol_mini(&SymbolMiniSpec {
        module: "RuntimeIdA",
        type_name: "RuntimeTypeA",
        function_name: "RuntimeFunctionA",
        type_ptr: 0x11_100,
        function_ptr: 0x11_200,
        type_id: (SCRIPT_OBJECT_KIND | 170) as i32,
        function_id: 170,
        runtime_id: 777,
    });
    let second = symbol_mini(&SymbolMiniSpec {
        module: "RuntimeIdB",
        type_name: "RuntimeTypeB",
        function_name: "RuntimeFunctionB",
        type_ptr: 0x22_100,
        function_ptr: 0x22_200,
        type_id: (SCRIPT_OBJECT_KIND | 171) as i32,
        function_id: 171,
        runtime_id: 777,
    });

    let mut forward = LoadoutScriptIdPlanBuilder::new(&base).unwrap();
    forward.inspect(&first).unwrap();
    forward.inspect(&second).unwrap();
    let forward = forward.finish().unwrap();
    let first_out = remap_module_to_base_with_loadout_plan(&first, &base, &forward)
        .unwrap()
        .0;
    let second_out = remap_module_to_base_with_loadout_plan(&second, &base, &forward)
        .unwrap()
        .0;

    let first_runtime = module_function_ids(&first_out);
    let second_runtime = module_function_ids(&second_out);
    assert_ne!(first_runtime, second_runtime);

    let mut reversed = LoadoutScriptIdPlanBuilder::new(&base).unwrap();
    reversed.inspect(&second).unwrap();
    reversed.inspect(&first).unwrap();
    let reversed = reversed.finish().unwrap();
    assert_eq!(
        forward.module_function_id_assignments,
        reversed.module_function_id_assignments
    );

    let mut guard = crate::cache::splice::SequentialMiniGuard::new(&base).unwrap();
    let running = guard.compose_add(&base, &first_out).unwrap();
    guard
        .compose_add(&running, &second_out)
        .expect("loadout-planned Function.Id values must compose without collision");
}

#[test]
fn builder_limits_are_checked_before_atomic_commit_and_allow_retry() {
    let base = empty_base();
    let first_spec = SymbolMiniSpec {
        module: "LimitA",
        type_name: "LimitTypeA",
        function_name: "LimitFunctionA",
        type_ptr: 0x1010,
        function_ptr: 0x2020,
        type_id: (SCRIPT_OBJECT_KIND | 20) as i32,
        function_id: 20,
        runtime_id: 20,
    };
    let duplicate_spec = SymbolMiniSpec {
        type_ptr: 0x3030,
        function_ptr: 0x4040,
        type_id: (SCRIPT_OBJECT_KIND | 30) as i32,
        function_id: 30,
        runtime_id: 30,
        ..first_spec.clone()
    };
    let second_spec = SymbolMiniSpec {
        module: "LimitB",
        type_name: "LimitTypeB",
        function_name: "LimitFunctionB",
        type_ptr: 0x5050,
        function_ptr: 0x6060,
        type_id: (SCRIPT_OBJECT_KIND | 40) as i32,
        function_id: 40,
        runtime_id: 40,
    };
    let first = symbol_mini(&first_spec);
    let duplicate = symbol_mini(&duplicate_spec);
    let second = symbol_mini(&second_spec);
    let context = build_allow_new_base_context(&base).unwrap();
    let first_identities = analyzed_identities(&context, &first);
    let second_identities = analyzed_identities(&context, &second);
    let (_, first_bytes) = first_identities
        .additional_usage(&NovelIdentitySet::default(), PRODUCTION_LOADOUT_PLAN_LIMITS)
        .unwrap();
    let mut combined = first_identities.clone();
    second_identities.clone().merge_into(&mut combined);
    let combined_entries = combined.pointers.len()
        + combined.type_ids.len()
        + combined.function_ids.len()
        + combined.module_function_ids.len();
    let (_, combined_bytes) = combined
        .additional_usage(&NovelIdentitySet::default(), PRODUCTION_LOADOUT_PLAN_LIMITS)
        .unwrap();

    let mut assignment_limited = LoadoutScriptIdPlanBuilder::new_with_config(
        &base,
        LoadoutPlanLimits {
            max_minis: 2,
            max_assignments: combined_entries - 1,
            max_identity_bytes: usize::MAX,
        },
        PRODUCTION_ALLOCATION_DOMAINS,
    )
    .unwrap();
    assignment_limited.inspect(&first).unwrap();
    assert!(matches!(
        assignment_limited.inspect(&second),
        Err(RemapError::LoadoutPlanResourceLimit {
            resource: "novel assignments",
            actual,
            limit,
        }) if actual == combined_entries && limit + 1 == actual
    ));
    assert_eq!(assignment_limited.inspected_minis.len(), 1);
    assignment_limited.inspect(&duplicate).unwrap();
    assert_eq!(assignment_limited.assignment_entries, 5);

    let mut byte_limited = LoadoutScriptIdPlanBuilder::new_with_config(
        &base,
        LoadoutPlanLimits {
            max_minis: 2,
            max_assignments: usize::MAX,
            max_identity_bytes: combined_bytes - 1,
        },
        PRODUCTION_ALLOCATION_DOMAINS,
    )
    .unwrap();
    byte_limited.inspect(&first).unwrap();
    assert_eq!(byte_limited.identity_bytes, first_bytes);
    assert!(matches!(
        byte_limited.inspect(&second),
        Err(RemapError::LoadoutPlanResourceLimit {
            resource: "identity bytes",
            actual,
            limit,
        }) if actual == combined_bytes && limit + 1 == actual
    ));
    assert_eq!(byte_limited.inspected_minis.len(), 1);
    byte_limited.inspect(&duplicate).unwrap();

    let mut mini_limited = LoadoutScriptIdPlanBuilder::new_with_config(
        &base,
        LoadoutPlanLimits {
            max_minis: 2,
            max_assignments: usize::MAX,
            max_identity_bytes: usize::MAX,
        },
        PRODUCTION_ALLOCATION_DOMAINS,
    )
    .unwrap();
    mini_limited.inspect(&first).unwrap();
    mini_limited.inspect(&duplicate).unwrap();
    assert!(matches!(
        mini_limited.inspect(&second),
        Err(RemapError::LoadoutPlanResourceLimit {
            resource: "inspected minis",
            actual: 3,
            limit: 2,
        })
    ));
    assert_eq!(mini_limited.inspected_minis.len(), 2);

    let mut duplicate_limited = LoadoutScriptIdPlanBuilder::new(&base).unwrap();
    for _ in 0..MAX_LOADOUT_PLAN_MINIS {
        duplicate_limited.inspect(&first).unwrap();
    }
    assert_eq!(duplicate_limited.inspected_count, MAX_LOADOUT_PLAN_MINIS);
    assert_eq!(duplicate_limited.inspected_minis.len(), 1);
    assert!(matches!(
        duplicate_limited.inspect(&first),
        Err(RemapError::LoadoutPlanResourceLimit {
            resource: "inspected minis",
            actual,
            limit: MAX_LOADOUT_PLAN_MINIS,
        }) if actual == MAX_LOADOUT_PLAN_MINIS + 1
    ));
    assert_eq!(duplicate_limited.inspected_count, MAX_LOADOUT_PLAN_MINIS);
    assert_eq!(duplicate_limited.inspected_minis.len(), 1);
}

#[test]
fn header_sha_and_identity_bindings_reject_changes_before_output() {
    let base = empty_base();
    let spec = SymbolMiniSpec {
        module: "BoundMod",
        type_name: "BoundType",
        function_name: "BoundFunction",
        type_ptr: 0x1110,
        function_ptr: 0x2220,
        type_id: (SCRIPT_OBJECT_KIND | 50) as i32,
        function_id: 50,
        runtime_id: 50,
    };
    let mini = symbol_mini(&spec);

    let mut builder = LoadoutScriptIdPlanBuilder::new(&base).unwrap();
    let mut wrong_magic = mini.clone();
    wrong_magic[16..20].copy_from_slice(&0u32.to_le_bytes());
    assert!(matches!(
        builder.inspect(&wrong_magic),
        Err(RemapError::LoadoutPlanInvalidHeader {
            artifact: "mini",
            ..
        })
    ));
    let mut wrong_guid = mini.clone();
    wrong_guid[0] = 1;
    assert!(matches!(
        builder.inspect(&wrong_guid),
        Err(RemapError::LoadoutPlanGuidMismatch { .. })
    ));
    assert!(builder.inspected_minis.is_empty());
    builder.inspect(&mini).unwrap(); // corrected retry succeeds
    let mut plan = builder.finish().unwrap();

    let mut changed = mini.clone();
    let position = changed
        .windows(spec.type_name.len())
        .position(|window| window == spec.type_name.as_bytes())
        .unwrap();
    changed[position] ^= 1;
    let unchanged = changed.clone();
    assert!(matches!(
        remap_module_to_base_with_loadout_plan(&changed, &base, &plan),
        Err(RemapError::LoadoutPlanMiniNotInspected)
    ));
    assert_eq!(changed, unchanged);

    let mut wrong_base = base.clone();
    wrong_base[0] = 2;
    assert!(matches!(
        remap_module_to_base_with_loadout_plan(&mini, &wrong_base, &plan),
        Err(RemapError::LoadoutPlanBaseMismatch)
    ));

    *plan.inspected_minis.get_mut(&sha256_bytes(&mini)).unwrap() = [0xff; 32];
    assert!(matches!(
        remap_module_to_base_with_loadout_plan(&mini, &base, &plan),
        Err(RemapError::LoadoutPlanIdentityMismatch)
    ));
}

#[test]
fn same_type_identity_with_different_object_kind_is_rejected_atomically() {
    let base = empty_base();
    let first_spec = SymbolMiniSpec {
        module: "KindMod",
        type_name: "KindType",
        function_name: "KindFunction",
        type_ptr: 0x1110,
        function_ptr: 0x2220,
        type_id: (SCRIPT_OBJECT_KIND | 60) as i32,
        function_id: 60,
        runtime_id: 60,
    };
    let conflicting_spec = SymbolMiniSpec {
        type_ptr: 0x3330,
        function_ptr: 0x4440,
        type_id: (APP_OBJECT_KIND | 70) as i32,
        function_id: 70,
        runtime_id: 70,
        ..first_spec.clone()
    };
    let corrected_spec = SymbolMiniSpec {
        type_id: (SCRIPT_OBJECT_KIND | 70) as i32,
        ..conflicting_spec.clone()
    };
    let first = symbol_mini(&first_spec);
    let conflicting = symbol_mini(&conflicting_spec);
    let corrected = symbol_mini(&corrected_spec);

    let mut builder = LoadoutScriptIdPlanBuilder::new(&base).unwrap();
    builder.inspect(&first).unwrap();
    assert!(matches!(
        builder.inspect(&conflicting),
        Err(RemapError::LoadoutPlanTypeKindConflict {
            first: SCRIPT_OBJECT_KIND,
            second: APP_OBJECT_KIND,
            ..
        })
    ));
    assert_eq!(builder.inspected_minis.len(), 1);
    builder.inspect(&corrected).unwrap();
    let plan = builder.finish().unwrap();
    assert_eq!(plan.type_id_assignments.len(), 1);
}

#[test]
fn planned_second_pass_resolves_pristine_signature_type_without_local_t1_row() {
    const BASE_TYPE_PTR: i64 = 0x5510;
    const BASE_TYPE_ID: i32 = (SCRIPT_OBJECT_KIND | 80) as i32;
    let base = cache(
        "Pristine",
        &[],
        &[class_record("PristineType")],
        TailRows {
            types: vec![type_row(BASE_TYPE_PTR, "PristineType", "Pristine")],
            type_ids: vec![id_row(BASE_TYPE_ID, BASE_TYPE_PTR)],
            ..TailRows::default()
        },
    );
    let mini = cache(
        "DependencyMod",
        &[function_record("UsesPristine", &[BASE_TYPE_PTR], 90)],
        &[],
        TailRows {
            funcs: vec![function_tail_row(
                0x9910,
                "UsesPristine",
                "DependencyMod",
                &[BASE_TYPE_PTR],
            )],
            func_ids: vec![id_row(90, 0x9910)],
            ..TailRows::default()
        },
    );

    let mut builder = LoadoutScriptIdPlanBuilder::new(&base).unwrap();
    builder.inspect(&mini).unwrap();
    let plan = builder.finish().unwrap();
    let output = remap_module_to_base_with_loadout_plan(&mini, &base, &plan)
        .unwrap()
        .0;
    let meta = TailMetadata::build(&output).unwrap();
    assert!(meta.types.is_empty(), "pristine T1 must remain omitted");
    assert_eq!(meta.funcs.len(), 1);
    assert_eq!(meta.funcs[0].type_deps[0].ptr, BASE_TYPE_PTR);
}

#[test]
fn plan_union_does_not_authorize_a_dependency_declared_only_by_another_mini() {
    const PROVIDER_TYPE_PTR: i64 = 0x7710;
    const CONSUMER_FUNC_PTR: i64 = 0x8810;
    let base = empty_base();
    let provider = symbol_mini(&SymbolMiniSpec {
        module: "ProviderMod",
        type_name: "ProviderType",
        function_name: "ProviderFunction",
        type_ptr: PROVIDER_TYPE_PTR,
        function_ptr: 0x7720,
        type_id: (SCRIPT_OBJECT_KIND | 100) as i32,
        function_id: 100,
        runtime_id: 100,
    });
    let consumer = cache(
        "ConsumerMod",
        &[function_record(
            "ConsumesProvider",
            &[PROVIDER_TYPE_PTR],
            101,
        )],
        &[],
        TailRows {
            funcs: vec![function_tail_row(
                CONSUMER_FUNC_PTR,
                "ConsumesProvider",
                "ConsumerMod",
                &[PROVIDER_TYPE_PTR],
            )],
            func_ids: vec![id_row(101, CONSUMER_FUNC_PTR)],
            ..TailRows::default()
        },
    );

    let mut builder = LoadoutScriptIdPlanBuilder::new(&base).unwrap();
    builder.inspect(&provider).unwrap();
    let inventory_before = builder.inventory.clone();
    let entries_before = builder.assignment_entries;
    let bytes_before = builder.identity_bytes;
    assert!(matches!(
        builder.inspect(&consumer),
        Err(RemapError::Wire(WireError::InvalidDataType {
            key: CONSUMER_FUNC_PTR,
            detail: "identifier requires a concrete TypeReference",
        }))
    ));
    assert_eq!(builder.inspected_count, 1);
    assert_eq!(builder.inspected_minis.len(), 1);
    assert_eq!(builder.inventory, inventory_before);
    assert_eq!(builder.assignment_entries, entries_before);
    assert_eq!(builder.identity_bytes, bytes_before);
}

#[test]
fn unknown_executable_ids_fail_before_builder_commit_and_allow_retry() {
    const UNKNOWN_FUNC_ID: i32 = 0x12_345;
    const UNKNOWN_TYPE_ID: i32 = (SCRIPT_OBJECT_KIND | 0x12_345) as i32;
    let base = empty_base();
    let spec = SymbolMiniSpec {
        module: "StrictRefs",
        type_name: "StrictType",
        function_name: "StrictFunction",
        type_ptr: 0x9110,
        function_ptr: 0x9220,
        type_id: (SCRIPT_OBJECT_KIND | 110) as i32,
        function_id: 110,
        runtime_id: 110,
    };
    let unknown_call = symbol_mini_with_code(&spec, &[9, UNKNOWN_FUNC_ID, 10]);
    let unknown_type_id = symbol_mini_with_code(&spec, &[76, UNKNOWN_TYPE_ID, 10]);
    let corrected = symbol_mini(&spec);
    let mut builder = LoadoutScriptIdPlanBuilder::new(&base).unwrap();

    assert!(matches!(
        builder.inspect(&unknown_call),
        Err(RemapError::UnresolvedEffectiveReference {
            kind: "function id",
            op: "CALL",
            key,
        }) if key == i64::from(UNKNOWN_FUNC_ID)
    ));
    assert_eq!(builder.inspected_count, 0);
    assert!(builder.inspected_minis.is_empty());
    assert!(builder.inventory.pointers.is_empty());

    assert!(matches!(
        builder.inspect(&unknown_type_id),
        Err(RemapError::UnresolvedEffectiveReference {
            kind: "type id",
            op: "TYPEID",
            key,
        }) if key == i64::from(UNKNOWN_TYPE_ID)
    ));
    assert_eq!(builder.inspected_count, 0);
    assert!(builder.inspected_minis.is_empty());
    assert!(builder.inventory.pointers.is_empty());

    builder.inspect(&corrected).unwrap();
    assert_eq!(builder.inspected_count, 1);
    assert_eq!(builder.inspected_minis.len(), 1);
}

#[test]
fn duplicate_portable_identity_under_two_raw_keys_is_rejected_before_emit() {
    let base = empty_base();
    let duplicate_types = cache(
        "AliasTypes",
        &[],
        &[class_record("AliasType")],
        TailRows {
            types: vec![
                type_row(0xc110, "AliasType", "AliasTypes"),
                type_row(0xc220, "AliasType", "AliasTypes"),
            ],
            type_ids: vec![
                id_row((SCRIPT_OBJECT_KIND | 130) as i32, 0xc110),
                id_row((SCRIPT_OBJECT_KIND | 131) as i32, 0xc220),
            ],
            ..TailRows::default()
        },
    );
    let duplicate_functions = cache(
        "AliasFunctions",
        &[function_record("AliasFunction", &[], 132)],
        &[],
        TailRows {
            funcs: vec![
                function_tail_row(0xd110, "AliasFunction", "AliasFunctions", &[]),
                function_tail_row(0xd220, "AliasFunction", "AliasFunctions", &[]),
            ],
            func_ids: vec![id_row(132, 0xd110), id_row(133, 0xd220)],
            ..TailRows::default()
        },
    );
    let duplicate_globals = cache_with_module_globals(
        "AliasGlobals",
        &[],
        &[],
        &[module_global_record("AliasGlobal")],
        TailRows {
            globals: vec![
                nonstring_global_row(0xe110, "AliasGlobal", "AliasGlobals"),
                nonstring_global_row(0xe220, "AliasGlobal", "AliasGlobals"),
            ],
            ..TailRows::default()
        },
    );

    for (mini, table) in [
        (duplicate_types, 0),
        (duplicate_functions, 2),
        (duplicate_globals, 4),
    ] {
        let mut builder = LoadoutScriptIdPlanBuilder::new(&base).unwrap();
        assert!(matches!(
            builder.inspect(&mini),
            Err(RemapError::InvalidTailRow {
                table: actual_table,
                kind: "symbol identity",
                ..
            }) if actual_table == table
        ));
        assert_eq!(builder.inspected_count, 0);
        assert!(builder.inspected_minis.is_empty());
        assert!(builder.inventory.pointers.is_empty());
    }
}

#[test]
fn string_global_aliases_share_one_canonical_row_within_and_across_minis() {
    let base = empty_base();
    let aliases = string_global_mini(
        "StringAliases",
        "UsesAliases",
        0xf110,
        140,
        &[0xf210, 0xf220],
        "None",
    );
    let repacked = string_global_mini(
        "StringRepacked",
        "UsesRepackedAlias",
        0xf330,
        141,
        &[0xf440],
        "None",
    );
    let mut builder = LoadoutScriptIdPlanBuilder::new(&base).unwrap();
    builder.inspect(&aliases).unwrap();
    builder.inspect(&repacked).unwrap();
    let plan = builder.finish().unwrap();
    let aliases_output = remap_module_to_base_with_loadout_plan(&aliases, &base, &plan)
        .unwrap()
        .0;
    let repacked_output = remap_module_to_base_with_loadout_plan(&repacked, &base, &plan)
        .unwrap()
        .0;

    let aliases_meta = TailMetadata::build(&aliases_output).unwrap();
    let repacked_meta = TailMetadata::build(&repacked_output).unwrap();
    assert_eq!(aliases_meta.globals.len(), 1);
    assert_eq!(repacked_meta.globals.len(), 1);
    let aliases_row = &aliases_meta.globals[0];
    let repacked_row = &repacked_meta.globals[0];
    assert!(aliases_row.is_string && repacked_row.is_string);
    assert_eq!(aliases_row.name, "None");
    assert_eq!(aliases_row.module, "");
    assert_eq!(aliases_row.namespace, "");
    assert_eq!(aliases_row.key, repacked_row.key);
    assert_eq!(
        &aliases_output[aliases_row.start..aliases_row.end],
        &repacked_output[repacked_row.start..repacked_row.end]
    );
    assert_eq!(
        global_ptr_operands(&aliases_output),
        [aliases_row.key, aliases_row.key]
    );
    assert_eq!(global_ptr_operands(&repacked_output), [aliases_row.key]);

    let reference_base = EffectiveReferenceBase::build(&base).unwrap();
    let mut reference_state = EffectiveReferenceState::default();
    let first_contribution = reference_base
        .validate(&reference_state, &aliases_output)
        .unwrap();
    reference_state.record(first_contribution);
    reference_base
        .validate(&reference_state, &repacked_output)
        .expect("canonical string alias must remain valid after the first mini");

    let ordinary = remap_module_allow_new(&aliases, &base).unwrap().0;
    let ordinary_meta = TailMetadata::build(&ordinary).unwrap();
    assert_eq!(ordinary_meta.globals.len(), 1);
    assert_eq!(global_ptr_operands(&ordinary).len(), 2);
    assert_eq!(
        global_ptr_operands(&ordinary)[0],
        global_ptr_operands(&ordinary)[1]
    );
}

#[test]
fn string_global_matching_large_base_alias_bucket_uses_the_smallest_key() {
    const FIRST_BASE_KEY: i64 = 0x1010;
    const BASE_ALIAS_COUNT: i64 = 2_500; // above the measured Shipping maximum of 2,442
    let base_aliases: Vec<Vec<u8>> = (0..BASE_ALIAS_COUNT)
        .rev()
        .map(|index| string_global_row(FIRST_BASE_KEY + index * 8, "SharedLiteral"))
        .collect();
    // A real Shipping cache has a large module body behind this alias bucket. Keep the synthetic
    // source/identity ratio representative so this lookup test does not trip the separate
    // small-source identity-amplification guard first.
    let source_padding = "x".repeat(256 * 1024);
    let base = cache(
        "Pristine",
        &[],
        &[],
        TailRows {
            globals: base_aliases,
            static_names: vec![sia(&source_padding)],
            ..TailRows::default()
        },
    );
    let mini = string_global_mini(
        "LiteralConsumer",
        "UsesBaseLiteral",
        0x3030,
        150,
        &[0x4040],
        "SharedLiteral",
    );

    let ordinary = remap_module_allow_new(&mini, &base).unwrap().0;
    assert_eq!(global_ptr_operands(&ordinary), [FIRST_BASE_KEY]);
    assert!(TailMetadata::build(&ordinary).unwrap().globals.is_empty());

    let mut builder = LoadoutScriptIdPlanBuilder::new(&base).unwrap();
    builder.inspect(&mini).unwrap();
    let plan = builder.finish().unwrap();
    let planned = remap_module_to_base_with_loadout_plan(&mini, &base, &plan)
        .unwrap()
        .0;
    assert_eq!(global_ptr_operands(&planned), [FIRST_BASE_KEY]);
    assert!(TailMetadata::build(&planned).unwrap().globals.is_empty());
}

#[test]
fn effective_reference_state_rejects_global_alias_from_a_prior_mini_atomically() {
    let base = empty_base();
    let first = cache_with_module_globals(
        "SharedGlobalModule",
        &[],
        &[],
        &[module_global_record("SharedGlobal")],
        TailRows {
            globals: vec![nonstring_global_row(
                0xe310,
                "SharedGlobal",
                "SharedGlobalModule",
            )],
            ..TailRows::default()
        },
    );
    let second = cache_with_module_globals(
        "SharedGlobalModule",
        &[],
        &[],
        &[module_global_record("SharedGlobal")],
        TailRows {
            globals: vec![nonstring_global_row(
                0xe320,
                "SharedGlobal",
                "SharedGlobalModule",
            )],
            ..TailRows::default()
        },
    );
    let reference_base = EffectiveReferenceBase::build(&base).unwrap();
    let mut state = EffectiveReferenceState::default();
    let first_contribution = reference_base.validate(&state, &first).unwrap();
    state.record(first_contribution);
    let state_before = state.clone();
    assert!(matches!(
        reference_base.validate(&state, &second),
        Err(RemapError::InvalidTailRow {
            table: 4,
            kind: "symbol identity",
            ..
        })
    ));
    assert_eq!(
        state.accepted_global_identities,
        state_before.accepted_global_identities
    );
    assert_eq!(
        state.accepted_identity_bytes,
        state_before.accepted_identity_bytes
    );
    assert_eq!(
        state.accepted_source_bytes,
        state_before.accepted_source_bytes
    );
}

#[test]
fn prepared_base_only_func_id_preserves_factory_ref_and_static_name() {
    const BASE_FUNC_PTR: i64 = 0xa110;
    const BASE_FUNC_ID: i32 = 120;
    const MINI_TYPE_PTR: i64 = 0xb110;
    const MINI_TYPE_ID: i32 = (SCRIPT_OBJECT_KIND | 121) as i32;
    const MINI_FUNC_PTR: i64 = 0xb220;
    const MINI_FUNC_ID: i32 = 121;
    let base = cache(
        "Pristine",
        &[function_record("__STATIC_NAME", &[], BASE_FUNC_ID)],
        &[],
        TailRows {
            funcs: vec![function_tail_row(
                BASE_FUNC_PTR,
                "__STATIC_NAME",
                "Pristine",
                &[],
            )],
            func_ids: vec![id_row(BASE_FUNC_ID, BASE_FUNC_PTR)],
            ..TailRows::default()
        },
    );
    let mini = cache(
        "PreparedConsumer",
        &[function_record_with_code(
            "UsesStaticName",
            &[],
            MINI_FUNC_ID,
            &[2, 0, 9, BASE_FUNC_ID, 10],
        )],
        &[class_record_with_factory_ref(
            "PreparedClass",
            i64::from(BASE_FUNC_ID),
        )],
        TailRows {
            types: vec![type_row(MINI_TYPE_PTR, "PreparedClass", "PreparedConsumer")],
            type_ids: vec![id_row(MINI_TYPE_ID, MINI_TYPE_PTR)],
            funcs: vec![function_tail_row(
                MINI_FUNC_PTR,
                "UsesStaticName",
                "PreparedConsumer",
                &[],
            )],
            func_ids: vec![id_row(MINI_FUNC_ID, MINI_FUNC_PTR)],
            static_names: vec![sia("FreshStaticName")],
            ..TailRows::default()
        },
    );

    let mut builder = LoadoutScriptIdPlanBuilder::new(&base).unwrap();
    builder.inspect(&mini).unwrap();
    let plan = builder.finish().unwrap();
    let output = remap_module_to_base_with_loadout_plan(&mini, &base, &plan)
        .unwrap()
        .0;
    let meta = TailMetadata::build(&output).unwrap();
    assert_eq!(meta.static_names.len(), 1);
    assert_eq!(meta.static_names[0].name, "FreshStaticName");
    let spans = collect_module_spans(&output).unwrap();
    let embedded_func_ids: Vec<i64> = spans
        .embeds
        .iter()
        .filter(|embed| matches!(embed.kind, EmbedKind::FuncId))
        .map(|embed| {
            i64::from_le_bytes(
                output[embed.byte_off..embed.byte_off + 8]
                    .try_into()
                    .unwrap(),
            )
        })
        .filter(|&id| id != 0)
        .collect();
    assert_eq!(embedded_func_ids, [i64::from(BASE_FUNC_ID)]);
}

#[test]
fn loadout_second_pass_accepts_prepared_absolute_static_name_index() {
    const BASE_FUNC_PTR: i64 = 0xc110;
    const BASE_FUNC_ID: i32 = 130;
    const MINI_FUNC_PTR: i64 = 0xc220;
    const MINI_FUNC_ID: i32 = 131;
    let base = cache(
        "Pristine",
        &[function_record("__STATIC_NAME", &[], BASE_FUNC_ID)],
        &[],
        TailRows {
            funcs: vec![function_tail_row(
                BASE_FUNC_PTR,
                "__STATIC_NAME",
                "Pristine",
                &[],
            )],
            func_ids: vec![id_row(BASE_FUNC_ID, BASE_FUNC_PTR)],
            static_names: vec![sia("BaseName0"), sia("BaseName1")],
            ..TailRows::default()
        },
    );
    // A compile-module artifact has already been remapped against `base`: its first compact T6
    // row is addressed at the absolute index immediately after the two pristine rows.
    let prepared = cache(
        "PreparedStaticConsumer",
        &[function_record_with_code(
            "UsesPreparedStaticName",
            &[],
            MINI_FUNC_ID,
            &[2, 2, 9, BASE_FUNC_ID, 10],
        )],
        &[],
        TailRows {
            funcs: vec![function_tail_row(
                MINI_FUNC_PTR,
                "UsesPreparedStaticName",
                "PreparedStaticConsumer",
                &[],
            )],
            func_ids: vec![id_row(MINI_FUNC_ID, MINI_FUNC_PTR)],
            static_names: vec![sia("PreparedName")],
            ..TailRows::default()
        },
    );

    let mut builder = LoadoutScriptIdPlanBuilder::new(&base).unwrap();
    builder.inspect(&prepared).unwrap();
    let plan = builder.finish().unwrap();
    let output = remap_module_to_base_with_loadout_plan(&prepared, &base, &plan)
        .unwrap()
        .0;

    let meta = TailMetadata::build(&output).unwrap();
    assert_eq!(meta.static_names.len(), 1);
    assert_eq!(meta.static_names[0].name, "PreparedName");
    let spans = collect_module_spans(&output).unwrap();
    assert_eq!(spans.code.len(), 1);
    let span = &spans.code[0];
    let code: Vec<i32> = (0..span.count)
        .map(|index| {
            let offset = span.data_off + index * 4;
            i32::from_le_bytes(output[offset..offset + 4].try_into().unwrap())
        })
        .collect();
    assert_eq!(code, [2, 2, 9, BASE_FUNC_ID, 10]);
}

#[test]
fn loadout_second_pass_rejects_missing_prepared_static_name_row() {
    const BASE_FUNC_PTR: i64 = 0xd110;
    const BASE_FUNC_ID: i32 = 140;
    const MINI_FUNC_PTR: i64 = 0xd220;
    const MINI_FUNC_ID: i32 = 141;
    let base = cache(
        "Pristine",
        &[function_record("__STATIC_NAME", &[], BASE_FUNC_ID)],
        &[],
        TailRows {
            funcs: vec![function_tail_row(
                BASE_FUNC_PTR,
                "__STATIC_NAME",
                "Pristine",
                &[],
            )],
            func_ids: vec![id_row(BASE_FUNC_ID, BASE_FUNC_PTR)],
            static_names: vec![sia("BaseName")],
            ..TailRows::default()
        },
    );
    let malformed = cache(
        "MalformedStaticConsumer",
        &[function_record_with_code(
            "UsesMissingStaticName",
            &[],
            MINI_FUNC_ID,
            &[2, 2, 9, BASE_FUNC_ID, 10],
        )],
        &[],
        TailRows {
            funcs: vec![function_tail_row(
                MINI_FUNC_PTR,
                "UsesMissingStaticName",
                "MalformedStaticConsumer",
                &[],
            )],
            func_ids: vec![id_row(MINI_FUNC_ID, MINI_FUNC_PTR)],
            // Its sole row lives at prepared absolute index 1, not the referenced gap at 2.
            static_names: vec![sia("OnlyPreparedName")],
            ..TailRows::default()
        },
    );

    let mut builder = LoadoutScriptIdPlanBuilder::new(&base).unwrap();
    builder.inspect(&malformed).unwrap();
    let plan = builder.finish().unwrap();
    assert!(matches!(
        remap_module_to_base_with_loadout_plan(&malformed, &base, &plan),
        Err(RemapError::MissingStaticName(2))
    ));
}

#[test]
fn successor_allocator_wraps_and_reports_small_domain_exhaustion() {
    let mut allocator = SuccessorAllocator::new(1, 3, [2, 3]);
    assert_eq!(allocator.allocate_from(2), Some(1));
    assert_eq!(allocator.allocate_from(1), None);

    let domains = CanonicalAllocationDomains {
        function_id_high: 2,
        ..small_domains()
    };
    let mut full = SuccessorAllocator::new(1, 2, [1, 2]);
    assert!(matches!(
        allocate_function_id("exhausted", &mut full, domains),
        Err(RemapError::KeySpaceExhausted {
            kind: "function-id"
        })
    ));
}

#[test]
fn selective_fullgraph_wakes_strict_consumer_by_inner_provider_identity() {
    use crate::cache::selective_fullgraph::{
        compose_selective_full_graph, SelectiveFullGraphChange,
    };
    use crate::cache::splice::{extract_module, splice_case_a, SequentialMiniGuard};

    const PROVIDER_OUTER: &str = "Z.ProviderOuter";
    const PROVIDER_RUNTIME: &str = "Runtime.Provider";
    const CONSUMER_OUTER: &str = "A.ConsumerOuter";
    const CONSUMER_RUNTIME: &str = "Runtime.Consumer";
    const PROVIDER_PTR: i64 = 0x7310;
    const PROVIDER_ID: i32 = 0x0800_7310;
    const CONSUMER_FUNC_PTR: i64 = 0x7420;
    const CONSUMER_FUNC_ID: i32 = 0x1742;

    let pristine = empty_base();
    let provider = cache_with_module_identity(
        PROVIDER_OUTER,
        PROVIDER_RUNTIME,
        &[],
        &[class_record("ProviderType")],
        &[],
        TailRows {
            types: vec![type_row(PROVIDER_PTR, "ProviderType", PROVIDER_RUNTIME)],
            type_ids: vec![id_row(PROVIDER_ID, PROVIDER_PTR)],
            ..TailRows::default()
        },
    );
    let consumer = cache_with_module_identity(
        CONSUMER_OUTER,
        CONSUMER_RUNTIME,
        &[function_record_with_code(
            "UseProvider",
            &[],
            0x0500_1742,
            &[76, PROVIDER_ID, 10],
        )],
        &[],
        &[],
        TailRows {
            funcs: vec![function_tail_row(
                CONSUMER_FUNC_PTR,
                "UseProvider",
                CONSUMER_RUNTIME,
                &[],
            )],
            func_ids: vec![id_row(CONSUMER_FUNC_ID, CONSUMER_FUNC_PTR)],
            ..TailRows::default()
        },
    );
    let full_graph = splice_case_a(&provider, &consumer).unwrap();
    let requested_modules = HashSet::from([PROVIDER_OUTER.to_owned(), CONSUMER_OUTER.to_owned()]);
    let dependency_index = RemapDependencyIndex::build(&full_graph, &requested_modules).unwrap();
    assert_eq!(
        dependency_index.providers_for_outer_module(PROVIDER_OUTER),
        [PROVIDER_RUNTIME]
    );

    let consumer_extracted = extract_module(&full_graph, CONSUMER_OUTER).unwrap();
    let strict_error = remap_module_to_base_with_options(
        &consumer_extracted,
        &pristine,
        RemapOptions {
            allow_new_symbols: false,
        },
    )
    .unwrap_err();
    assert!(matches!(
        strict_error,
        RemapError::Unresolved {
            kind: "type-id",
            op: "TYPEID",
            key: PROVIDER_PTR,
            ..
        }
    ));
    assert_eq!(
        dependency_index.retry_provider(&strict_error).as_deref(),
        Some(PROVIDER_RUNTIME)
    );

    let provider_extracted = extract_module(&full_graph, PROVIDER_OUTER).unwrap();
    let provider_mini = remap_module_to_base_with_options(
        &provider_extracted,
        &pristine,
        RemapOptions {
            allow_new_symbols: true,
        },
    )
    .unwrap()
    .0;
    let mut guard = SequentialMiniGuard::new(&pristine).unwrap();
    let running = guard.compose_add(&pristine, &provider_mini).unwrap();
    remap_module_to_base_with_options(
        &consumer_extracted,
        &running,
        RemapOptions {
            allow_new_symbols: false,
        },
    )
    .expect("strict consumer resolves after its runtime provider is composed");

    let output = compose_selective_full_graph(
        &pristine,
        &full_graph,
        vec![
            SelectiveFullGraphChange::add(CONSUMER_OUTER),
            SelectiveFullGraphChange::add(PROVIDER_OUTER),
        ],
    )
    .unwrap();
    assert_eq!(output.applied_modules, [PROVIDER_OUTER, CONSUMER_OUTER]);
}

#[test]
fn real_shipping_builds_complete_semantic_property_and_function_id_indexes() {
    let Some(path) = std::env::var_os("GORE_AS_REAL_CACHE") else {
        eprintln!("skip: set GORE_AS_REAL_CACHE");
        return;
    };
    let bytes = std::fs::read(path).expect("read real Shipping cache");
    let context = build_allow_new_base_context(&bytes).expect("index real Shipping cache");
    for row in &context.meta.properties {
        let Some(owner_ptr) = context.syms.typeid_to_ptr.get(&row.old_type_id) else {
            continue;
        };
        let Some(owner_identity) = context.syms.type_id_of_ptr.get(owner_ptr) else {
            continue;
        };
        assert!(context
            .declarations
            .properties
            .contains(&PristinePropertyIdentity {
                owner_identity: owner_identity.clone(),
                name: row.name.clone(),
                member_offset: row.member_offset,
            }));
    }
    assert!(
        !context.module_function_ids.is_empty(),
        "Shipping must expose module Function.Id assignments"
    );
    assert_eq!(
        context.module_function_ids.len(),
        context.occupied_module_function_ids.len(),
        "Shipping module Function.Id values must be globally unique"
    );
}
