use gore_as::cache::refs::RefResolver;

const SAMPLES: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../work/reversing/gore-as/samples"
);

fn read_sample(name: &str) -> Option<Vec<u8>> {
    std::fs::read(format!("{SAMPLES}/{name}")).ok()
}

#[test]
fn resolves_richtest_member_and_type() {
    let Some(b) = read_sample("PrecompiledScript.richtest.Cache") else {
        eprintln!("skip: richtest sample not present");
        return;
    };
    let r = RefResolver::build(&b).expect("build resolver");
    // method1's LoadThisR: type-id 0x8003366 (GoreTestClass), byte offset 40 -> field1.
    assert_eq!(
        r.member(0x8003366, 40),
        Some("field1"),
        "member resolves to field1"
    );
    // the GoreTestClass type-id resolves to its name.
    assert_eq!(
        r.type_by_id(0x8003366),
        Some("GoreTestClass"),
        "type-id -> GoreTestClass"
    );
}

/// Resolver must parse the real cache's tables without desync (env-gated).
#[test]
fn builds_on_real_cache() {
    let Ok(path) = std::env::var("GORE_AS_REAL_CACHE") else {
        eprintln!("skip: set GORE_AS_REAL_CACHE");
        return;
    };
    let b = std::fs::read(&path).expect("read real cache");
    let r = RefResolver::build(&b).expect("build resolver on real cache");
    // Spot-check: some well-known engine type id should resolve. Just assert it built
    // and a few member lookups are queryable (non-panicking).
    let _ = r.type_by_id(0);
    eprintln!("resolver built on real cache OK");
}
