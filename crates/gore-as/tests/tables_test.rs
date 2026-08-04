use gore_as::cache::tables::parse_tail_tables;
use gore_as::cache::walk_modules::module_region_end;

const SAMPLES: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../work/reversing/gore-as/samples"
);

fn read_sample(name: &str) -> Option<Vec<u8>> {
    std::fs::read(format!("{SAMPLES}/{name}")).ok()
}

#[test]
fn richtest_tail_tables_parse_to_eof() {
    let Some(b) = read_sample("PrecompiledScript.richtest.Cache") else {
        eprintln!("skip: richtest sample not present");
        return;
    };
    let tail = module_region_end(&b).unwrap();
    assert_eq!(tail, 0x753);
    let tt = parse_tail_tables(&b, tail).expect("parse tail tables");
    let counts: Vec<u32> = tt.tables.iter().map(|t| t.count).collect();
    assert_eq!(counts, vec![5, 5, 4, 4, 1, 0, 2], "per-table counts");
    assert_eq!(tt.end, b.len(), "tables consume exactly to EOF");
    assert_eq!(tt.end, 0xad6);
}

/// Decisive: parse the REAL cache's 7 tail tables and confirm they consume exactly to EOF.
/// This validates the value-struct layouts against ~36 MB of real table data (hundreds of
/// thousands of entries) — far stronger than the 12-entry richtest. Set GORE_AS_REAL_CACHE.
#[test]
fn real_cache_tail_tables_parse_to_eof() {
    let Ok(path) = std::env::var("GORE_AS_REAL_CACHE") else {
        eprintln!("skip: set GORE_AS_REAL_CACHE");
        return;
    };
    let b = std::fs::read(&path).expect("read real cache");
    let tail = module_region_end(&b).expect("walk modules");
    let tt = parse_tail_tables(&b, tail).expect("parse real tail tables");
    let counts: Vec<u32> = tt.tables.iter().map(|t| t.count).collect();
    eprintln!(
        "real tail tables: counts={counts:?}, end={:#x}, eof={:#x}",
        tt.end,
        b.len()
    );
    assert_eq!(
        tt.end,
        b.len(),
        "real tail tables must consume exactly to EOF"
    );
}

#[test]
fn minimal_tail_tables_all_empty() {
    let Some(b) = read_sample("PrecompiledScript.minimal-1fn.Cache") else {
        return;
    };
    let tail = module_region_end(&b).unwrap();
    let tt = parse_tail_tables(&b, tail).expect("parse empty tail");
    assert!(tt.tables.iter().all(|t| t.count == 0), "all 7 tables empty");
    assert_eq!(tt.end, b.len());
}

#[test]
fn truncated_huge_tail_count_fails_before_allocation() {
    let bytes = 50_000_000i32.to_le_bytes();
    let error = parse_tail_tables(&bytes, 0).unwrap_err();
    assert!(
        error.to_string().contains("unexpected end of data"),
        "{error}"
    );
}
