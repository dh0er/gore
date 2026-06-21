use gore_as::cache::walk_modules::{module_count, module_region_end};

const SAMPLES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../work/reversing/gore-as/samples");

fn read_sample(name: &str) -> Option<Vec<u8>> {
    std::fs::read(format!("{SAMPLES}/{name}")).ok()
}

#[test]
fn minimal_sample_tail_at_0x11e() {
    let Some(b) = read_sample("PrecompiledScript.minimal-1fn.Cache") else {
        eprintln!("skip: minimal sample not present (gitignored scratch)");
        return;
    };
    assert_eq!(module_count(&b), 1);
    let tail = module_region_end(&b).expect("walk minimal");
    assert_eq!(tail, 0x11e, "minimal TAIL_OFF");
    assert!(b[tail..].iter().all(|&x| x == 0), "minimal tail = 7 empty tables");
    assert_eq!(b[tail..].len(), 28);
}

#[test]
fn richtest_sample_tail_at_0x753() {
    let Some(b) = read_sample("PrecompiledScript.richtest.Cache") else {
        eprintln!("skip: richtest sample not present (gitignored scratch)");
        return;
    };
    assert_eq!(module_count(&b), 1);
    let tail = module_region_end(&b).expect("walk richtest");
    assert_eq!(tail, 0x753, "richtest TAIL_OFF");
    assert!(tail < b.len(), "tail before EOF");
    // richtest's global tables are non-empty (class-bearing module).
    assert!(b[tail..].iter().any(|&x| x != 0), "richtest tail non-empty");
}

/// Decisive correctness gate: walk all 7264 modules of the real shipped cache.
/// Set `GORE_AS_REAL_CACHE` to its path to run. Asserts the walk lands at a
/// plausible TAIL_OFF (strictly inside the file) and the first global table count
/// is sane (a desync would blow past EOF or read a garbage count).
#[test]
fn real_cache_walk_reaches_tail() {
    let Ok(path) = std::env::var("GORE_AS_REAL_CACHE") else {
        eprintln!("skip: set GORE_AS_REAL_CACHE to run the real-cache walk");
        return;
    };
    let b = std::fs::read(&path).expect("read real cache");
    let n = module_count(&b);
    assert_eq!(n, 7264, "expected 7264 modules");
    let tail = module_region_end(&b).expect("walk real cache to TAIL_OFF");
    assert!(tail < b.len(), "TAIL_OFF {tail:#x} must be < EOF {:#x}", b.len());
    // First global table (TypeReferences) count must be plausible.
    let first_tbl = u32::from_le_bytes(b[tail..tail + 4].try_into().unwrap());
    assert!(first_tbl < 50_000_000, "implausible TypeReferences count {first_tbl} => desync");
    eprintln!("real cache: 7264 modules, TAIL_OFF={tail:#x}, EOF={:#x}, first table count={first_tbl}", b.len());
}
