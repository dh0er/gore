use gore_as::cache::splice::{splice, splice_case_a, SpliceError};
use gore_as::cache::tables::parse_tail_tables;
use gore_as::cache::walk_modules::{module_count, module_names, module_region_end};

const SAMPLES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../work/reversing/gore-as/samples");

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
    assert_eq!(names, vec!["_gore_richtest".to_string(), "_gore_bakemarker".to_string()]);

    // Output re-walks cleanly and the inserted module sits before the preserved tail.
    let out_tail = module_region_end(&out).unwrap();
    let mod_len = mini_tail - 0x18;
    assert_eq!(out_tail, base_tail + mod_len, "tail moved by inserted module length");
    assert_eq!(&out[out_tail..], &base[base_tail..], "global tail tables preserved verbatim");
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
    let Some(mini) = read_sample(MINI) else { return };
    let err = splice(&mini, &mini).unwrap_err();
    assert!(matches!(err, SpliceError::NameCollision(_)), "got {err:?}");
}

#[test]
fn rejects_mini_with_global_refs() {
    let (Some(base), Some(rich)) = (read_sample(MINI), read_sample(RICH)) else { return };
    // richtest as the MINI has non-empty global tables (class-bearing) -> rejected.
    let err = splice(&base, &rich).unwrap_err();
    assert!(matches!(err, SpliceError::MiniHasGlobalRefs(_)), "got {err:?}");
}

/// Splice the primitive mini into the REAL 122 MB cache and verify the result re-walks
/// to 7265 modules. Set GORE_AS_REAL_CACHE to run.
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
    let out = splice(&base, &mini).expect("splice into real cache");
    assert_eq!(module_count(&out), 7265);
    let tail = module_region_end(&out).expect("re-walk spliced real cache");
    assert!(tail < out.len());
    assert_eq!(out.len(), base.len() + (module_region_end(&mini).unwrap() - 0x18));
    eprintln!("spliced real cache: 7265 modules, new TAIL_OFF={tail:#x}, size {}->{}", base.len(), out.len());
}
