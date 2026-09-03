use gore_as::cache::splice::{
    extract_module, replace_module, splice, splice_auto, splice_case_a,
    validate_standalone_script_cache, SequentialMiniGuard, SpliceError,
};
use gore_as::cache::tables::parse_tail_tables;
use gore_as::cache::walk_modules::{module_count, module_names, module_ranges, module_region_end};

const SAMPLES: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../work/reversing/gore-as/samples"
);

fn read_sample(name: &str) -> Option<Vec<u8>> {
    std::fs::read(format!("{SAMPLES}/{name}")).ok()
}

const MINI: &str = "PrecompiledScript.minimal-1fn.Cache"; // 1 fn, empty tail
const RICH: &str = "PrecompiledScript.richtest.Cache"; // class-bearing, non-empty tail

fn fstring(value: &str) -> Vec<u8> {
    let mut out = ((value.len() + 1) as i32).to_le_bytes().to_vec();
    out.extend_from_slice(value.as_bytes());
    out.push(0);
    out
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

fn empty_module_entry(outer_name: &str, inner_name: &str) -> Vec<u8> {
    let mut out = fstring(outer_name);
    out.extend_from_slice(&sia(inner_name));
    out.extend_from_slice(&[0; 5 * 4]); // functions/classes/enums/globals/imports
    out.extend_from_slice(&0i64.to_le_bytes()); // CodeHash
    out.extend_from_slice(&0u32.to_le_bytes()); // ImportedModules
    out.extend_from_slice(&sia("")); // StaticsClassName
    out.extend_from_slice(&0u32.to_le_bytes()); // DeclaredEvents
    out.extend_from_slice(&0u32.to_le_bytes()); // DeclaredDelegates
    out.extend_from_slice(&sia("")); // ScriptRelativeFilename
    out.extend_from_slice(&0u32.to_le_bytes()); // PostInitFunctions
    out
}

fn empty_modules_cache(modules: &[(&str, &str)]) -> Vec<u8> {
    let mut out = [0x11; 16].to_vec();
    out.extend_from_slice(&gore_as::cache::header::CACHE_MAGIC.to_le_bytes());
    out.extend_from_slice(&(modules.len() as u32).to_le_bytes());
    for &(outer_name, inner_name) in modules {
        out.extend_from_slice(&empty_module_entry(outer_name, inner_name));
    }
    out.extend_from_slice(&[0; 7 * 4]); // empty tail tables
    out
}

fn empty_module_cache(module: &str) -> Vec<u8> {
    empty_modules_cache(&[(module, module)])
}

fn standalone_cache_with_tail_keys(rows: &[(usize, i64)]) -> Vec<u8> {
    let mut cache = empty_module_cache("FullReplacement");
    cache.truncate(cache.len() - 7 * 4);
    for table in 0..7 {
        let keys = rows
            .iter()
            .filter_map(|&(candidate, key)| (candidate == table).then_some(key))
            .collect::<Vec<_>>();
        cache.extend_from_slice(&(keys.len() as i32).to_le_bytes());
        for key in keys {
            match table {
                0 => {
                    cache.extend_from_slice(&key.to_le_bytes());
                    cache.extend_from_slice(&sia("Type"));
                    cache.extend_from_slice(&sia("FullReplacement"));
                    cache.extend_from_slice(&sia(""));
                    cache.extend_from_slice(&0i32.to_le_bytes());
                }
                1 | 3 => {
                    cache.extend_from_slice(&(key as i32).to_le_bytes());
                    cache.extend_from_slice(&0x4000i64.to_le_bytes());
                }
                2 => {
                    cache.extend_from_slice(&key.to_le_bytes());
                    cache.extend_from_slice(&sia("Function"));
                    cache.extend_from_slice(&sia("FullReplacement"));
                    cache.extend_from_slice(&sia(""));
                    cache.extend_from_slice(&[0; 3 * 4]);
                    cache.extend_from_slice(&0i64.to_le_bytes());
                    cache.extend_from_slice(&0i32.to_le_bytes());
                    cache.extend_from_slice(&[0; 36]);
                }
                4 => {
                    cache.extend_from_slice(&key.to_le_bytes());
                    cache.extend_from_slice(&sia("Global"));
                    cache.extend_from_slice(&sia("FullReplacement"));
                    cache.extend_from_slice(&sia(""));
                    cache.extend_from_slice(&0i32.to_le_bytes());
                }
                5 => unreachable!("StaticNames is intentionally not keyed"),
                6 => {
                    cache.extend_from_slice(&key.to_le_bytes());
                    cache.extend_from_slice(&sia("Property"));
                    cache.extend_from_slice(&0i32.to_le_bytes());
                }
                _ => unreachable!(),
            }
        }
    }
    cache
}

#[test]
fn impossible_module_count_is_refused_before_capacity_allocation() {
    let mut malformed = [0x11; 16].to_vec();
    malformed.extend_from_slice(&gore_as::cache::header::CACHE_MAGIC.to_le_bytes());
    malformed.extend_from_slice(&u32::MAX.to_le_bytes());
    let mini = empty_module_cache("Mini");

    assert!(module_region_end(&malformed).is_err());
    assert!(module_names(&malformed).is_err());
    assert!(module_ranges(&malformed).is_err());
    assert!(extract_module(&malformed, "Missing").is_err());
    assert!(splice(&malformed, &mini).is_err());
    assert!(replace_module(&malformed, &mini, "Missing").is_err());
}

#[test]
fn sequential_guard_rejects_bad_cache_magic_without_poisoning_retry() {
    let base = empty_module_cache("Base");
    let mut bad_base = base.clone();
    bad_base[0x10..0x14].copy_from_slice(&0xdead_beefu32.to_le_bytes());
    assert!(matches!(
        SequentialMiniGuard::new(&bad_base),
        Err(SpliceError::Header(_))
    ));

    let mut guard = SequentialMiniGuard::new(&base).unwrap();
    let good_mini = empty_module_cache("Mini");
    let mut bad_mini = good_mini.clone();
    bad_mini[0x10..0x14].copy_from_slice(&0xdead_beefu32.to_le_bytes());
    assert!(matches!(
        guard.compose_add(&base, &bad_mini),
        Err(SpliceError::Header(_))
    ));
    guard
        .compose_add(&base, &good_mini)
        .expect("a bad-magic refusal must not commit guard history");
}

#[test]
fn standalone_script_cache_validation_is_structural_guid_agnostic_and_non_mutating() {
    let mut cache = empty_module_cache("FullReplacement");
    cache[..16].copy_from_slice(&[0xa5; 16]);
    let original = cache.clone();

    validate_standalone_script_cache(&cache)
        .expect("a complete replacement owns its GUID and valid wire container");
    assert_eq!(
        cache, original,
        "structural validation must not rewrite bytes"
    );
}

#[test]
fn standalone_script_cache_validation_rejects_bad_container_shapes() {
    let short = vec![0u8; gore_as::cache::header::CacheHeader::SIZE - 1];
    assert!(matches!(
        validate_standalone_script_cache(&short),
        Err(SpliceError::Header(_))
    ));

    let mut wrong_magic = empty_module_cache("FullReplacement");
    wrong_magic[0x10..0x14].copy_from_slice(&0xdead_beefu32.to_le_bytes());
    assert!(matches!(
        validate_standalone_script_cache(&wrong_magic),
        Err(SpliceError::Header(_))
    ));

    let mut truncated = empty_module_cache("FullReplacement");
    truncated.pop();
    assert!(matches!(
        validate_standalone_script_cache(&truncated),
        Err(SpliceError::Wire(_))
    ));

    let mut impossible_count = empty_module_cache("FullReplacement");
    impossible_count[0x14..0x18].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(matches!(
        validate_standalone_script_cache(&impossible_count),
        Err(SpliceError::Wire(_))
    ));

    let mut trailing = empty_module_cache("FullReplacement");
    trailing.push(0);
    assert!(matches!(
        validate_standalone_script_cache(&trailing),
        Err(SpliceError::Wire(_))
    ));

    let duplicate_key = empty_modules_cache(&[("Dup", "InnerA"), ("Dup", "InnerB")]);
    assert!(matches!(
        validate_standalone_script_cache(&duplicate_key),
        Err(SpliceError::ModuleKeyCollision(ref name)) if name == "Dup"
    ));
}

#[test]
fn standalone_script_cache_validation_rejects_tail_map_key_collisions() {
    for table in [0usize, 1, 2, 3, 4, 6] {
        let error = validate_standalone_script_cache(&standalone_cache_with_tail_keys(&[
            (table, 0x41),
            (table, 0x41),
        ]))
        .expect_err("duplicate TMap keys are malformed in a standalone cache too");
        assert!(
            matches!(error, SpliceError::KeyCollision { table: got, key: 0x41 } if got == table),
            "unexpected table-{table} error: {error:?}"
        );
    }

    for table in [2usize, 4, 6] {
        let error = validate_standalone_script_cache(&standalone_cache_with_tail_keys(&[
            (0, 0x52),
            (table, 0x52),
        ]))
        .expect_err("T1/T3/T5/T7 share one runtime key domain");
        assert!(
            matches!(error, SpliceError::KeyCollision { table: got, key: 0x52 } if got == table),
            "unexpected shared-domain table-{table} error: {error:?}"
        );
    }
}

#[test]
fn duplicate_outer_module_keys_are_rejected_before_ambiguous_composition() {
    let malformed = empty_modules_cache(&[("Dup", "InnerA"), ("Dup", "InnerB")]);
    let mini = empty_module_cache("Added");

    let guard_error = SequentialMiniGuard::new(&malformed)
        .expect_err("Unreal's Modules TMap cannot represent duplicate outer keys");
    assert!(
        matches!(guard_error, SpliceError::InnerNameCollision(ref name) if name == "Dup"),
        "unexpected guard error: {guard_error:?}"
    );

    let splice_error = splice(&malformed, &mini)
        .expect_err("a low-level splice must not publish a duplicate-key Modules TMap");
    assert!(
        matches!(splice_error, SpliceError::InnerNameCollision(ref name) if name == "Dup"),
        "unexpected splice error: {splice_error:?}"
    );

    let replace_error = replace_module(&malformed, &mini, "Dup")
        .expect_err("replace must not silently choose the first duplicate target key");
    assert!(
        matches!(replace_error, SpliceError::AmbiguousTarget(ref name) if name == "Dup"),
        "unexpected replace error: {replace_error:?}"
    );

    let corrected = empty_modules_cache(&[("OuterA", "InnerA"), ("OuterB", "InnerB")]);
    SequentialMiniGuard::new(&corrected).expect("distinct outer and inner names remain valid");
}

#[test]
fn splices_primitive_module_into_base() {
    let (Some(base), Some(mini)) = (read_sample(RICH), read_sample(MINI)) else {
        eprintln!("skip: samples not present (gitignored scratch)");
        return;
    };
    let base_tail = module_region_end(&base).unwrap();
    let mini_tail = module_region_end(&mini).unwrap();

    let out = splice(&base, &mini).expect("splice ok");

    assert_eq!(module_count(&out), 2, "module count bumped");
    let names = module_names(&out).unwrap();
    assert_eq!(
        names,
        vec!["_gore_richtest".to_string(), "_gore_bakemarker".to_string()]
    );

    // Output re-walks cleanly and the inserted module sits before the preserved tail.
    let out_tail = module_region_end(&out).unwrap();
    let mod_len = mini_tail - 0x18;
    assert_eq!(
        out_tail,
        base_tail + mod_len,
        "tail moved by inserted module length"
    );
    assert_eq!(
        &out[out_tail..],
        &base[base_tail..],
        "global tail tables preserved verbatim"
    );
}

#[test]
fn case_a_merges_tables() {
    // base = minimal (empty tables), mini = richtest (class-bearing, non-empty tables).
    let (Some(base), Some(mini)) = (read_sample(MINI), read_sample(RICH)) else {
        eprintln!("skip: samples not present");
        return;
    };
    let out = splice_case_a(&base, &mini).expect("case-a splice");
    assert_eq!(module_count(&out), 2, "module count bumped");
    assert_eq!(
        module_names(&out).unwrap(),
        vec!["_gore_bakemarker".to_string(), "_gore_richtest".to_string()]
    );

    // Output re-walks and its merged tail tables parse to EOF.
    let out_tail = module_region_end(&out).unwrap();
    let tt = parse_tail_tables(&out, out_tail).expect("parse merged tables");
    assert_eq!(tt.end, out.len(), "merged tables consume to EOF");
    // base tables were empty, so merged counts == mini's counts.
    let counts: Vec<u32> = tt.tables.iter().map(|t| t.count).collect();
    assert_eq!(counts, vec![5, 5, 4, 4, 1, 0, 2]);
}

#[test]
fn rejects_name_collision() {
    let Some(mini) = read_sample(MINI) else {
        return;
    };
    let err = splice(&mini, &mini).unwrap_err();
    assert!(matches!(err, SpliceError::NameCollision(_)), "got {err:?}");
}

#[test]
fn rejects_mini_with_global_refs() {
    let (Some(base), Some(rich)) = (read_sample(MINI), read_sample(RICH)) else {
        return;
    };
    // richtest as the MINI has non-empty global tables (class-bearing) -> rejected.
    let err = splice(&base, &rich).unwrap_err();
    assert!(
        matches!(err, SpliceError::MiniHasGlobalRefs(_)),
        "got {err:?}"
    );
}

const BRANCH: &str = "PrecompiledScript.branchtest.Cache"; // single-module branchtest

#[test]
fn replace_module_swaps_module_in_place() {
    // Build a 2-module base: richtest base + minimal appended.
    let (Some(rich), Some(minimal), Some(branch)) =
        (read_sample(RICH), read_sample(MINI), read_sample(BRANCH))
    else {
        eprintln!("skip: samples not present (gitignored scratch)");
        return;
    };
    let base = splice_auto(&rich, &minimal).expect("build 2-module base");
    assert_eq!(module_count(&base), 2, "base has 2 modules");
    assert_eq!(
        module_names(&base).unwrap(),
        vec!["_gore_richtest".to_string(), "_gore_bakemarker".to_string()]
    );

    // Replace the minimal module (_gore_bakemarker) with the branchtest module.
    let out = replace_module(&base, &branch, "_gore_bakemarker").expect("replace ok");

    // Module count is UNCHANGED (still 2).
    assert_eq!(module_count(&out), 2, "module count unchanged");

    // Names: richtest + branchtest present, bakemarker gone.
    let names = module_names(&out).unwrap();
    assert!(
        names.contains(&"_gore_richtest".to_string()),
        "richtest kept: {names:?}"
    );
    assert!(
        names.contains(&"_gore_branchtest".to_string()),
        "branchtest added: {names:?}"
    );
    assert!(
        !names.contains(&"_gore_bakemarker".to_string()),
        "bakemarker replaced: {names:?}"
    );

    // Output re-walks and its merged tail tables parse to EOF.
    let out_tail = module_region_end(&out).expect("re-walk replaced cache");
    let tt = parse_tail_tables(&out, out_tail).expect("parse merged tables");
    assert_eq!(tt.end, out.len(), "tail tables consume to EOF");
}

#[test]
fn replace_module_rejects_missing_name() {
    let (Some(rich), Some(branch)) = (read_sample(RICH), read_sample(BRANCH)) else {
        return;
    };
    let err = replace_module(&rich, &branch, "_does_not_exist").unwrap_err();
    assert!(matches!(err, SpliceError::NameNotFound(_)), "got {err:?}");
}

/// Splice the primitive mini into a real cache and verify the result re-walks
/// to the combined dynamic module count. Set GORE_AS_REAL_CACHE to run.
#[test]
fn splice_into_real_cache() {
    let Ok(path) = std::env::var("GORE_AS_REAL_CACHE") else {
        eprintln!("skip: set GORE_AS_REAL_CACHE");
        return;
    };
    let Some(mini) = read_sample(MINI) else {
        eprintln!("skip: minimal sample not present");
        return;
    };
    let base = std::fs::read(&path).expect("read real cache");
    let expected_modules = module_count(&base) + module_count(&mini);
    let out = splice(&base, &mini).expect("splice into real cache");
    assert_eq!(module_count(&out), expected_modules);
    let tail = module_region_end(&out).expect("re-walk spliced real cache");
    assert!(tail < out.len());
    assert_eq!(
        out.len(),
        base.len() + (module_region_end(&mini).unwrap() - 0x18)
    );
    eprintln!(
        "spliced real cache: {expected_modules} modules, new TAIL_OFF={tail:#x}, size {}->{}",
        base.len(),
        out.len()
    );
}

// ── multi-module minis ───────────────────────────────────────────────────────────────────────────

/// A referenceless mini carrying `modules` plus `static_names` (T6 is unkeyed and needs no
/// declaration authority, so it is the one non-empty tail a synthetic mini can carry safely).
fn multi_module_mini(modules: &[(&str, &str)], static_names: &[&str]) -> Vec<u8> {
    let mut out = empty_modules_cache(modules);
    out.truncate(out.len() - 7 * 4);
    for table in 0..7 {
        if table == 5 {
            out.extend_from_slice(&(static_names.len() as u32).to_le_bytes());
            for name in static_names {
                out.extend_from_slice(&sia(name));
            }
        } else {
            out.extend_from_slice(&0u32.to_le_bytes());
        }
    }
    out
}

#[test]
fn splice_appends_every_module_of_a_referenceless_multi_module_mini() {
    let base = empty_modules_cache(&[("Base.One", "Base.One"), ("Base.Two", "Base.Two")]);
    let mini = empty_modules_cache(&[("Mod.A", "Mod.A"), ("Mod.B", "Mod.B")]);

    let out = splice(&base, &mini).expect("two referenceless modules splice");

    assert_eq!(module_count(&out), 4);
    assert_eq!(
        module_names(&out).unwrap(),
        ["Base.One", "Base.Two", "Mod.A", "Mod.B"]
    );
    let tail = module_region_end(&out).unwrap();
    assert_eq!(&out[tail..], &[0u8; 7 * 4], "empty tail tables preserved");
}

#[test]
fn splice_auto_merges_the_tail_of_a_multi_module_mini_once() {
    let base = multi_module_mini(&[("Base.One", "Base.One")], &["BaseName"]);
    let mini = multi_module_mini(&[("Mod.A", "Mod.A"), ("Mod.B", "Mod.B")], &["NameA", "NameB"]);

    let out = splice_auto(&base, &mini).expect("case-a multi-module splice");

    assert_eq!(module_names(&out).unwrap(), ["Base.One", "Mod.A", "Mod.B"]);
    let tail = module_region_end(&out).unwrap();
    let tables = parse_tail_tables(&out, tail).unwrap();
    assert_eq!(tables.end, out.len());
    assert_eq!(tables.tables[5].count, 3, "StaticNames appended once for the whole mini");
}

#[test]
fn splice_rejects_a_multi_module_mini_when_any_module_already_exists() {
    let base = empty_modules_cache(&[("Base.One", "Base.One"), ("Mod.B", "Mod.B")]);
    let mini = empty_modules_cache(&[("Mod.A", "Mod.A"), ("Mod.B", "Mod.B")]);

    let err = splice(&base, &mini).unwrap_err();
    assert!(
        matches!(&err, SpliceError::NameCollision(name) if name == "Mod.B"),
        "got {err:?}"
    );
}

#[test]
fn splice_rejects_a_mini_without_modules() {
    let base = empty_modules_cache(&[("Base.One", "Base.One")]);
    let mini = empty_modules_cache(&[]);

    let err = splice(&base, &mini).unwrap_err();
    assert!(matches!(err, SpliceError::MiniEmpty), "got {err:?}");
    let err = splice_auto(&base, &mini).unwrap_err();
    assert!(matches!(err, SpliceError::MiniEmpty), "got {err:?}");
}

#[test]
fn replace_module_keeps_requiring_a_single_module_mini() {
    let base = empty_modules_cache(&[("Base.One", "Base.One")]);
    let mini = empty_modules_cache(&[("Mod.A", "Mod.A"), ("Mod.B", "Mod.B")]);

    let err = replace_module(&base, &mini, "Base.One").unwrap_err();
    assert!(matches!(err, SpliceError::MiniNotSingle(2)), "got {err:?}");
}

#[test]
fn sequential_guard_composes_a_multi_module_mini_as_one_add() {
    let base = multi_module_mini(&[("Base.One", "Base.One")], &["BaseName"]);
    let mut mini = multi_module_mini(&[("Mod.A", "Mod.A"), ("Mod.B", "Mod.B")], &["NameA"]);
    mini[..16].copy_from_slice(&base[..16]); // generation-bound to the base GUID

    let mut guard = SequentialMiniGuard::new(&base).unwrap();
    let out = guard.compose_add(&base, &mini).expect("guarded multi-module add");

    assert_eq!(module_names(&out).unwrap(), ["Base.One", "Mod.A", "Mod.B"]);
    let tail = module_region_end(&out).unwrap();
    let tables = parse_tail_tables(&out, tail).unwrap();
    assert_eq!(tables.tables[5].count, 2);
}

#[test]
fn upsert_replaces_existing_modules_in_place_and_appends_new_ones() {
    use gore_as::cache::splice::upsert_modules;
    let base = multi_module_mini(
        &[("Base.One", "Base.One"), ("Shared.Edit", "Shared.Edit"), ("Base.Two", "Base.Two")],
        &["BaseName"],
    );
    let mini = multi_module_mini(
        &[("Shared.Edit", "Shared.Edit.v2"), ("Mod.New", "Mod.New")],
        &["NameNew"],
    );

    let out = upsert_modules(&base, &mini).expect("upsert");

    assert_eq!(module_count(&out), 4);
    assert_eq!(
        module_names(&out).unwrap(),
        ["Base.One", "Shared.Edit", "Base.Two", "Mod.New"]
    );
    // The edited entry now carries the mini's bytes (its inner name changed to `.v2`).
    let ranges = module_ranges(&out).unwrap();
    let (_, start, end) = &ranges[1];
    assert!(out[*start..*end]
        .windows(b"Shared.Edit.v2".len())
        .any(|w| w == b"Shared.Edit.v2"));
    let tail = module_region_end(&out).unwrap();
    let tables = parse_tail_tables(&out, tail).unwrap();
    assert_eq!(tables.end, out.len());
    assert_eq!(tables.tables[5].count, 2);
}

#[test]
fn extract_modules_keeps_cache_order_and_rejects_bad_requests() {
    use gore_as::cache::splice::extract_modules;
    let cache = multi_module_mini(
        &[("Mod.A", "Mod.A"), ("Mod.B", "Mod.B"), ("Mod.C", "Mod.C")],
        &["Name"],
    );

    let mini = extract_modules(&cache, &["Mod.C", "Mod.A"]).expect("extract two");
    assert_eq!(module_names(&mini).unwrap(), ["Mod.A", "Mod.C"]);
    let tail = module_region_end(&mini).unwrap();
    assert_eq!(&mini[tail..], &cache[module_region_end(&cache).unwrap()..]);

    assert!(matches!(
        extract_modules(&cache, &["Mod.A", "Mod.A"]).unwrap_err(),
        SpliceError::AmbiguousTarget(_)
    ));
    assert!(matches!(
        extract_modules(&cache, &["Mod.X"]).unwrap_err(),
        SpliceError::NameNotFound(_)
    ));
    assert!(matches!(
        extract_modules(&cache, &[]).unwrap_err(),
        SpliceError::MiniEmpty
    ));
}
