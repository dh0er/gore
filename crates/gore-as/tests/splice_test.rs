use gore_as::cache::splice::{replace_module, splice, splice_auto, splice_case_a, SpliceError};
use gore_as::cache::tables::parse_tail_tables;
use gore_as::cache::walk_modules::{module_count, module_names, module_region_end};

const SAMPLES: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../work/reversing/gore-as/samples"
);

fn read_sample(name: &str) -> Option<Vec<u8>> {
    std::fs::read(format!("{SAMPLES}/{name}")).ok()
}

const MINI: &str = "PrecompiledScript.minimal-1fn.Cache"; // 1 fn, empty tail
const RICH: &str = "PrecompiledScript.richtest.Cache"; // class-bearing, non-empty tail

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
