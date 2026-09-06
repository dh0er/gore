use gore_as::cache::emit_all::PreparedEmit;
use gore_as::cache::model::{Class, Field, Func, Module, Param};
use gore_as::cache::refs::RefResolver;
use gore_as::cache::types::DataType;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::io::Read;
use std::path::{Path, PathBuf};

fn primitive(token: i32) -> DataType {
    DataType {
        token,
        ..DataType::default()
    }
}

fn function(name: &str, namespace: &str, params: Vec<Param>) -> Func {
    Func {
        name: name.into(),
        namespace: namespace.into(),
        param_defaults: Vec::new(),
        ret: primitive(0x52),
        params,
        bytecode: Vec::new(),
        variable_space: 0,
        obj_locals: Vec::new(),
        is_ufunction: false,
        traits: 0,
    }
}

fn parameter(mut ty: DataType, flags: i32) -> Param {
    if flags != 0 {
        ty.is_reference = true;
    }
    Param {
        name: "Value".into(),
        ty,
        flags,
    }
}

fn module(name: &str, functions: Vec<Func>, classes: Vec<Class>) -> Module {
    Module {
        name: name.into(),
        file: format!("{name}.as"),
        functions,
        classes,
        enums: Vec::new(),
        globals: Vec::new(),
    }
}

#[test]
fn collision_plan_matches_emitted_functions_and_reference_modifiers() {
    let get = || function("Get", "", vec![parameter(primitive(0x44), 0)]);
    let ref_only = |flags| function("RefOnly", "", vec![parameter(primitive(0x44), flags)]);
    let mirror = || function("Mirror", "Owner", Vec::new());
    let owner = || Class {
        name: "Owner".into(),
        namespace: String::new(),
        super_class: None,
        fields: Vec::new(),
        methods: vec![function("Mirror", "Owner", Vec::new())],
        ctors: Vec::new(),
        flags: 0,
    };
    let modules = vec![
        module(
            "A",
            vec![
                get(),
                ref_only(1),
                function("StaticClass", "", Vec::new()),
                mirror(),
            ],
            vec![owner()],
        ),
        module(
            "B",
            vec![
                get(),
                ref_only(2),
                function("StaticClass", "", Vec::new()),
                mirror(),
            ],
            vec![owner()],
        ),
    ];
    let mut refs = RefResolver::default();

    let prepared = PreparedEmit::new(&modules, &mut refs, None).unwrap();
    let a = prepared.emit_module(0).unwrap();
    let b = prepared.emit_module(1).unwrap();

    // `Get(int)` is a genuine emitted free function despite its factory-like name.
    assert!(a.contains("Get_g0("));
    assert!(b.contains("Get_g1("));
    // AngelScript overload identity includes reference direction, so these do not collide.
    assert!(a.contains("RefOnly("));
    assert!(b.contains("RefOnly("));
    assert!(!a.contains("RefOnly_g0("));
    assert!(!b.contains("RefOnly_g1("));
    // Exact generated/class-mirror records are not emitted and cannot create collisions.
    assert!(!a.contains("StaticClass_g0("));
    assert!(!a.contains("Mirror_g0("));
}

#[test]
fn distinct_real_world_namespaces_do_not_create_free_function_collisions() {
    let context_module = |module_name: &str, namespace: &str| {
        module(
            module_name,
            vec![
                function(
                    "FetchContext",
                    namespace,
                    vec![parameter(primitive(0x44), 0)],
                ),
                function(
                    "GetTuning",
                    namespace,
                    vec![parameter(primitive(0x44), 0)],
                ),
            ],
            Vec::new(),
        )
    };
    let modules = vec![
        context_module("Theft", "TheftContext"),
        context_module("Pickpocket", "PickpocketContext"),
        context_module("Creeping", "CreepingContext"),
    ];
    let mut refs = RefResolver::default();
    let prepared = PreparedEmit::new(&modules, &mut refs, None).unwrap();

    for (index, namespace) in
        ["TheftContext", "PickpocketContext", "CreepingContext"]
            .into_iter()
            .enumerate()
    {
        let emitted = prepared.emit_module(index).unwrap();
        assert!(emitted.contains(&format!("namespace {namespace}")));
        assert!(emitted.contains("FetchContext("));
        assert!(emitted.contains("GetTuning("));
        assert!(!emitted.contains("FetchContext_g"));
        assert!(!emitted.contains("GetTuning_g"));
    }

    prepared
        .prepare_overlay(
            "add",
            "AssessmentBits",
            "void Check() { TheftContext::FetchContext(1); PickpocketContext::GetTuning(1); CreepingContext::FetchContext(1); }",
        )
        .unwrap();
}

#[test]
fn same_module_namespaced_free_function_signatures_remain_distinct() {
    let modules = vec![module(
        "Namespaced",
        vec![
            function("Same", "Alpha", vec![parameter(primitive(0x44), 0)]),
            function("Same", "Beta", vec![parameter(primitive(0x44), 0)]),
        ],
        Vec::new(),
    )];
    let mut refs = RefResolver::default();
    let prepared = PreparedEmit::new(&modules, &mut refs, None).unwrap();

    let emitted = prepared.emit_module(0).unwrap();
    assert!(emitted.contains("namespace Alpha"));
    assert!(emitted.contains("namespace Beta"));
    assert_eq!(emitted.matches("void Same(int Value)").count(), 2);

    let output = tempfile::tempdir().unwrap();
    let stats = prepared.emit_tree(output.path()).unwrap();
    assert_eq!(stats.functions, 2);
}

#[test]
fn same_namespace_free_function_calls_remain_fail_closed() {
    let shared = || {
        function(
            "FetchContext",
            "SharedContext",
            vec![parameter(primitive(0x44), 0)],
        )
    };
    let modules = vec![
        module("First", vec![shared()], Vec::new()),
        module("Second", vec![shared()], Vec::new()),
    ];
    let mut refs = RefResolver::default();
    let prepared = PreparedEmit::new(&modules, &mut refs, None).unwrap();

    assert!(prepared
        .prepare_overlay(
            "add",
            "Caller",
            "void Check() { SharedContext::FetchContext(1); }",
        )
        .unwrap_err()
        .contains("collision-ambiguous"));
}

#[test]
fn collision_targets_are_unique_and_overlay_calls_fail_closed() {
    let get = || function("Get", "", vec![parameter(primitive(0x44), 0)]);
    let get_pair = || {
        function(
            "Get",
            "",
            vec![parameter(primitive(0x44), 0), parameter(primitive(0x44), 0)],
        )
    };
    let modules = vec![
        module("A", vec![get()], Vec::new()),
        module("B", vec![get()], Vec::new()),
        module("UniqueArity", vec![get_pair()], Vec::new()),
        module(
            "Reserved",
            vec![function("Get_g0", "", Vec::new())],
            Vec::new(),
        ),
    ];
    let mut refs = RefResolver::default();
    let prepared = PreparedEmit::new(&modules, &mut refs, None).unwrap();
    assert!(prepared.emit_module(0).unwrap().contains("Get_g0_r1("));
    assert!(prepared.emit_module(1).unwrap().contains("Get_g1("));

    let edited = prepared
        .prepare_overlay("edit", "A", "void Get(int Value)\n{\n}\n")
        .unwrap();
    assert!(edited.contains("void Get_g0_r1(int Value)"));

    let ambiguous_edit = "void Get(int Value)\n{\n}\nvoid Caller()\n{\n    Get(1);\n}\n";
    assert!(prepared
        .prepare_overlay("edit", "A", ambiguous_edit)
        .unwrap_err()
        .contains("collision-ambiguous"));
    assert!(prepared
        .prepare_overlay("add", "NewModule", "void Caller() { Get(1); }")
        .unwrap_err()
        .contains("collision-ambiguous"));
    assert!(prepared
        .prepare_overlay("add", "NewModule", "void Caller() { Namespace::Get(1); }")
        .unwrap_err()
        .contains("collision-ambiguous"));
    assert!(prepared
        .prepare_overlay(
            "add",
            "NewModule",
            "void Caller() { Namespace::Get(1, 2); }"
        )
        .unwrap_err()
        .contains("collision-ambiguous"));
    assert!(prepared
        .prepare_overlay("add", "NewModule", "void Caller() { ::Get(1); }")
        .unwrap_err()
        .contains("collision-ambiguous"));
    assert!(prepared
        .prepare_overlay(
            "add",
            "NewModule",
            "void Caller() { ::Get(\"comma, stays\", Nested(1, 2)); }"
        )
        .is_ok());
    assert!(prepared
        .prepare_overlay("add", "NewModule", "int Caller() { return ::Get(1, 2); }")
        .is_ok());
    assert!(prepared
        .prepare_overlay(
            "add",
            "NewModule",
            "void Caller() { ::Get(TMap<FString, int>()); }"
        )
        .unwrap_err()
        .contains("collision-ambiguous"));
    assert!(prepared
        .prepare_overlay(
            "add",
            "NewModule",
            "void Caller() { Callback@ Cb = @::Get; }"
        )
        .unwrap_err()
        .contains("collision-ambiguous"));
    let global_edit = "void Get(int Value)\n{\n}\nvoid Caller()\n{\n    ::Get(1);\n}\n";
    assert!(prepared
        .prepare_overlay("edit", "A", global_edit)
        .unwrap_err()
        .contains("collision-ambiguous"));
    let safe_global_edit = "void Get(int Value)\n{\n}\nvoid Caller()\n{\n    ::Get(1, 2);\n}\n";
    assert!(prepared
        .prepare_overlay("edit", "A", safe_global_edit)
        .is_ok());
    // Explicit member calls are not global free-function references and remain legal.
    assert!(prepared
        .prepare_overlay(
            "add",
            "NewModule",
            "class ObjectType { void Get() {} }\nvoid Caller() { Object.Get(); }"
        )
        .is_ok());
}

fn direction_collision_modules() -> Vec<Module> {
    let overloads = || {
        let mut const_int = primitive(0x44);
        const_int.is_read_only = true;
        vec![
            function("Shared", "", vec![parameter(const_int, 1)]),
            function("Shared", "", vec![parameter(primitive(0x44), 2)]),
        ]
    };
    vec![
        module("DirectionA", overloads(), Vec::new()),
        module("DirectionB", overloads(), Vec::new()),
    ]
}

#[test]
fn edit_overlay_requires_the_exact_collision_bound_overload_set() {
    let modules = direction_collision_modules();
    let mut refs = RefResolver::default();
    let prepared = PreparedEmit::new(&modules, &mut refs, None).unwrap();
    let target = "Shared_g0";

    let complete = r#"
void Shared(const int &in Value /* trivia */) {}
void Shared(int &out Value = 0) {}
"#;
    let rewritten = prepared
        .prepare_overlay("edit", "DirectionA", complete)
        .unwrap();
    assert_eq!(rewritten.matches(&format!("void {target}(")).count(), 2);

    let invalid_sources = [
        // Missing one direction overload.
        "void Shared(const int &in Value) {}".to_owned(),
        // Duplicate direction cannot stand in for the other overload.
        "void Shared(const int &in A) {}\nvoid Shared(const int &in B) {}".to_owned(),
        // Changed type.
        "void Shared(const int &in A) {}\nvoid Shared(float &out B) {}".to_owned(),
        // An already-renamed dummy declaration cannot satisfy a different required signature.
        format!("void {target}(const int &in A) {{}}\nvoid {target}(float &out B) {{}}"),
        // Additional collision-bound overloads are not part of the base module contract.
        "void Shared(const int &in A) {}\nvoid Shared(int &out B) {}\nvoid Shared(float C) {}"
            .to_owned(),
    ];
    for invalid in &invalid_sources {
        assert!(
            prepared
                .prepare_overlay("edit", "DirectionA", invalid)
                .is_err(),
            "invalid overload set was accepted:\n{invalid}"
        );
    }

    // A current `as emit` file already carries target declarations and remains valid input.
    let prepared_source = complete.replace("Shared", target);
    prepared
        .prepare_overlay("edit", "DirectionA", &prepared_source)
        .unwrap();
}

#[test]
fn overlay_scanner_is_comment_safe_and_understands_class_members_and_handles() {
    let mut modules = direction_collision_modules();
    modules[0].classes.push(Class {
        name: "VanillaClass".into(),
        namespace: String::new(),
        super_class: None,
        fields: Vec::new(),
        methods: vec![function("Shared", "VanillaClass", Vec::new())],
        ctors: Vec::new(),
        flags: 0,
    });
    let mut refs = RefResolver::default();
    let prepared = PreparedEmit::new(&modules, &mut refs, None).unwrap();
    let legal = r#"
class C
{
    void Shared() {}
    void Run()
    {
        this./* receiver trivia */Shared();
        super /* before dot */ . /* after dot */ Shared();
        Object /* before dot */ . /* after dot */ Shared();
        FString Literal = "Shared() @Shared";
        // Shared(); @Shared
    }
}
"#;
    prepared.prepare_overlay("add", "NewModule", legal).unwrap();

    let local = "class C { void Shared() {} void Run() { Shared(); } }";
    assert!(prepared
        .prepare_overlay("add", "NewModule", local)
        .unwrap_err()
        .contains("collision-ambiguous"));
    let struct_local = "struct S { void Shared() {} void Run() { Shared(); } }";
    assert!(prepared
        .prepare_overlay("add", "NewModule", struct_local)
        .unwrap_err()
        .contains("collision-ambiguous"));
    let removed_vanilla_method = "class VanillaClass { void Run() { Shared(); } }";
    assert!(prepared
        .prepare_overlay("add", "NewModule", removed_vanilla_method)
        .unwrap_err()
        .contains("collision-ambiguous"));
    let handle = "class C { void Run() { Callback@ Value = @ /* trivia */ Shared; } }";
    assert!(prepared
        .prepare_overlay("add", "NewModule", handle)
        .unwrap_err()
        .contains("collision-ambiguous"));
    let namespace = "void Run() { Namespace /* trivia */ :: Shared(); }";
    assert!(prepared
        .prepare_overlay("add", "NewModule", namespace)
        .unwrap_err()
        .contains("collision-ambiguous"));
}

#[test]
fn public_emit_all_api_prepares_collision_declarations_without_a_prior_side_call() {
    let modules = direction_collision_modules();
    let mut single_refs = RefResolver::default();
    let prepared = PreparedEmit::new(&modules, &mut single_refs, None).unwrap();
    let single_a = prepared.emit_module(0).unwrap();
    let single_b = prepared.emit_module(1).unwrap();
    drop(prepared);
    let mut refs = RefResolver::default();
    let output = std::env::temp_dir().join(format!("gore-as-public-emit-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&output);

    gore_as::cache::emit_all::emit_all_tree(&modules, &mut refs, None, &output).unwrap();

    let a = std::fs::read_to_string(output.join("DirectionA.as")).unwrap();
    let b = std::fs::read_to_string(output.join("DirectionB.as")).unwrap();
    assert_eq!(single_a, a);
    assert_eq!(single_b, b);
    assert_eq!(a.matches("Shared_g0(").count(), 2);
    assert_eq!(b.matches("Shared_g1(").count(), 2);
    assert!(!a.contains("void Shared("));
    assert!(!b.contains("void Shared("));
    std::fs::remove_dir_all(output).unwrap();
}

#[test]
fn partial_overlap_renames_the_complete_same_name_family() {
    let shared = |token| function("Shared", "", vec![parameter(primitive(token), 0)]);
    let modules = vec![
        module("A", vec![shared(0x44), shared(0x51)], Vec::new()),
        module("B", vec![shared(0x44), shared(0x41)], Vec::new()),
    ];
    let mut refs = RefResolver::default();
    let prepared = PreparedEmit::new(&modules, &mut refs, None).unwrap();
    let a = prepared.emit_module(0).unwrap();
    let b = prepared.emit_module(1).unwrap();

    assert_eq!(a.matches("Shared_g0(").count(), 2, "{a}");
    assert_eq!(b.matches("Shared_g1(").count(), 2, "{b}");
    assert!(!a.contains(" Shared("), "{a}");
    assert!(!b.contains(" Shared("), "{b}");
}

fn push_sia(output: &mut Vec<u8>, value: &str) {
    output.extend_from_slice(&(value.len() as i32).to_le_bytes());
    if value.is_empty() {
        return;
    }
    output.extend_from_slice(value.as_bytes());
    output.push(0);
}

fn push_void_type(output: &mut Vec<u8>) {
    for _ in 0..6 {
        output.extend_from_slice(&0i32.to_le_bytes());
    }
    output.extend_from_slice(&0i64.to_le_bytes());
    output.extend_from_slice(&0x52i32.to_le_bytes());
}

fn resolver_with_shared_ids(entries: &[(i32, i64, &str)]) -> RefResolver {
    let mut bytes = vec![0u8; 16];
    bytes.extend_from_slice(&gore_as::cache::header::CACHE_MAGIC.to_le_bytes());
    bytes.extend_from_slice(&0u32.to_le_bytes()); // no module-region records
    bytes.extend_from_slice(&0i32.to_le_bytes()); // type references
    bytes.extend_from_slice(&0i32.to_le_bytes()); // type-id references
    bytes.extend_from_slice(&(entries.len() as i32).to_le_bytes());
    for (_, pointer, declaring_module) in entries {
        bytes.extend_from_slice(&pointer.to_le_bytes());
        push_sia(&mut bytes, "Shared");
        push_sia(&mut bytes, declaring_module);
        push_sia(&mut bytes, "");
        bytes.extend_from_slice(&0i32.to_le_bytes()); // const
        bytes.extend_from_slice(&0i32.to_le_bytes()); // imported
        bytes.extend_from_slice(&0i32.to_le_bytes()); // method
        bytes.extend_from_slice(&0i64.to_le_bytes()); // owning type
        bytes.extend_from_slice(&0i32.to_le_bytes()); // params
        push_void_type(&mut bytes);
    }
    bytes.extend_from_slice(&(entries.len() as i32).to_le_bytes());
    for (id, pointer, _) in entries {
        bytes.extend_from_slice(&id.to_le_bytes());
        bytes.extend_from_slice(&pointer.to_le_bytes());
    }
    for _ in 0..3 {
        bytes.extend_from_slice(&0i32.to_le_bytes());
    }
    RefResolver::build(&bytes).unwrap()
}

#[test]
fn partial_overlap_installs_cross_module_id_renames_for_every_family_member() {
    let shared = |token| function("Shared", "", vec![parameter(primitive(token), 0)]);
    let modules = vec![
        module("A", vec![shared(0x44), shared(0x51)], Vec::new()),
        module("B", vec![shared(0x44), shared(0x41)], Vec::new()),
    ];
    let entries = [
        (101, 1001, "A"),
        (102, 1002, "A"),
        (201, 2001, "B"),
        (202, 2002, "B"),
    ];
    let mut refs = resolver_with_shared_ids(&entries);
    let prepared = PreparedEmit::new(&modules, &mut refs, None).unwrap();
    drop(prepared);

    assert_eq!(refs.renamed_free_fn_by_id(101), Some("Shared_g0"));
    assert_eq!(refs.renamed_free_fn_by_id(102), Some("Shared_g0"));
    assert_eq!(refs.renamed_free_fn_by_id(201), Some("Shared_g1"));
    assert_eq!(refs.renamed_free_fn_by_id(202), Some("Shared_g1"));
}

#[test]
fn module_layout_aliases_and_unsafe_paths_fail_before_output_creation() {
    let mut empty_derived = module("A", Vec::new(), Vec::new());
    empty_derived.file.clear();
    let mut aliases_derived = module("B", Vec::new(), Vec::new());
    aliases_derived.file = "a.AS".into();

    let mut path_a = module("PathA", Vec::new(), Vec::new());
    path_a.file = "Dir\\Foo.as".into();
    let mut path_b = module("PathB", Vec::new(), Vec::new());
    path_b.file = "dir/./foo.AS".into();

    let mut unsafe_module = module("Unsafe", Vec::new(), Vec::new());
    unsafe_module.file = "../escape.as".into();
    let mut illegal_character = module("IllegalCharacter", Vec::new(), Vec::new());
    illegal_character.file = "New?.as".into();
    let mut prefix_file = module("PrefixFile", Vec::new(), Vec::new());
    prefix_file.file = "Foo.as".into();
    let mut prefix_child = module("PrefixChild", Vec::new(), Vec::new());
    prefix_child.file = "foo.AS/Bar.as".into();
    let cases = [
        vec![
            module("Alpha", Vec::new(), Vec::new()),
            module("alpha", Vec::new(), Vec::new()),
        ],
        vec![path_a, path_b],
        vec![empty_derived, aliases_derived],
        vec![unsafe_module],
        vec![illegal_character],
        vec![prefix_file, prefix_child],
    ];

    for (case, modules) in cases.into_iter().enumerate() {
        let output = std::env::temp_dir().join(format!(
            "gore-as-invalid-layout-{}-{case}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&output);
        let mut refs = RefResolver::default();
        let error = gore_as::cache::emit_all::emit_all_tree(&modules, &mut refs, None, &output)
            .unwrap_err();
        assert!(error.to_string().contains("invalid module layout"));
        assert!(!output.exists(), "invalid layout created {output:?}");
    }
}

#[test]
fn full_tree_resolver_preparation_includes_classes_fields_and_unreferenced_methods() {
    let int_type = DataType {
        token: 0x44,
        ..DataType::default()
    };
    let method = Func {
        name: "ShadowedName".into(),
        namespace: String::new(),
        param_defaults: Vec::new(),
        ret: DataType {
            token: 0x52,
            ..DataType::default()
        },
        params: Vec::new(),
        bytecode: Vec::new(),
        variable_space: 0,
        obj_locals: Vec::new(),
        is_ufunction: false,
        traits: 0,
    };
    let modules = vec![Module {
        name: "ResolverFixture".into(),
        file: "ResolverFixture.as".into(),
        functions: Vec::new(),
        classes: vec![
            Class {
                name: "UBaseFixture".into(),
                namespace: String::new(),
                super_class: None,
                fields: vec![Field {
                    name: "Count".into(),
                    ty: int_type,
                    is_uproperty: false,
                }],
                methods: vec![method],
                ctors: Vec::new(),
                flags: 0,
            },
            Class {
                name: "UChildFixture".into(),
                namespace: String::new(),
                super_class: Some("UBaseFixture".into()),
                fields: Vec::new(),
                methods: Vec::new(),
                ctors: Vec::new(),
                flags: 0,
            },
        ],
        enums: Vec::new(),
        globals: Vec::new(),
    }];
    let mut refs = RefResolver::default();

    let prepared = PreparedEmit::new(&modules, &mut refs, None).unwrap();
    drop(prepared);

    assert!(refs.is_script_class("UBaseFixture"));
    assert_eq!(refs.class_super_of("UChildFixture"), Some("UBaseFixture"));
    assert_eq!(
        refs.field_type_by_class("UChildFixture", "Count"),
        Some("int")
    );
    assert!(refs.member_name_exists("ShadowedName"));
}

fn tree_manifest(root: &Path) -> BTreeMap<PathBuf, (u64, [u8; 32])> {
    fn visit(root: &Path, current: &Path, out: &mut BTreeMap<PathBuf, (u64, [u8; 32])>) {
        for entry in std::fs::read_dir(current).expect("read manifest directory") {
            let entry = entry.expect("read manifest entry");
            let path = entry.path();
            let ty = entry.file_type().expect("read manifest entry type");
            if ty.is_dir() {
                visit(root, &path, out);
            } else if ty.is_file() {
                let mut file = std::fs::File::open(&path).expect("open manifest file");
                let mut hash = Sha256::new();
                let mut buffer = [0_u8; 64 * 1024];
                let mut length = 0_u64;
                loop {
                    let read = file.read(&mut buffer).expect("hash manifest file");
                    if read == 0 {
                        break;
                    }
                    hash.update(&buffer[..read]);
                    length += read as u64;
                }
                out.insert(
                    path.strip_prefix(root)
                        .expect("manifest relative path")
                        .into(),
                    (length, hash.finalize().into()),
                );
            } else {
                panic!(
                    "manifest tree contains a non-file entry: {}",
                    path.display()
                );
            }
        }
    }

    let mut manifest = BTreeMap::new();
    visit(root, root, &mut manifest);
    manifest
}

/// Expensive real-cache qualification. Set both variables to prove the shared preparation path
/// reproduces an independently game-compiled source tree byte-for-byte.
#[test]
#[ignore = "requires GORE_AS_REAL_CACHE and GORE_AS_QUALIFIED_TREE"]
fn configured_real_cache_emit_matches_qualified_tree() {
    let cache = PathBuf::from(
        std::env::var_os("GORE_AS_REAL_CACHE").expect("set GORE_AS_REAL_CACHE for qualification"),
    );
    let expected = PathBuf::from(
        std::env::var_os("GORE_AS_QUALIFIED_TREE")
            .expect("set GORE_AS_QUALIFIED_TREE for qualification"),
    );
    let output = std::env::temp_dir().join(format!(
        "gore-as-qualified-emit-{}-{}",
        std::process::id(),
        std::thread::current().name().unwrap_or("test")
    ));
    let _ = std::fs::remove_dir_all(&output);

    let bytes = std::fs::read(&cache).expect("read configured real cache");
    let mut refs = RefResolver::build(&bytes).expect("build real resolver");
    let modules = gore_as::cache::model::parse_modules(&bytes).expect("parse real modules");
    let native = std::env::var_os("GORE_AS_BINDS")
        .map(PathBuf::from)
        .or_else(|| cache.parent().map(|parent| parent.join("Binds.Cache")))
        .filter(|path| path.is_file())
        .and_then(|path| gore_as::cache::binds::NativeApi::load(&path));
    gore_as::cache::emit_all::emit_all_tree(&modules, &mut refs, native, &output)
        .expect("emit configured real tree");

    let actual_manifest = tree_manifest(&output);
    let expected_manifest = tree_manifest(&expected);
    std::fs::remove_dir_all(&output).expect("clean qualification tree");
    assert_eq!(actual_manifest, expected_manifest);
}
