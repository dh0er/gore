use gore_as::cache::decompile::decompile_function;
use gore_as::cache::refs::RefResolver;
use gore_as::cache::walk_modules::collect_function_bytecodes;

const SAMPLES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../work/reversing/gore-as/samples");

fn read_sample(name: &str) -> Option<Vec<u8>> {
    std::fs::read(format!("{SAMPLES}/{name}")).ok()
}

#[test]
fn decompiles_richtest_method1() {
    let Some(b) = read_sample("PrecompiledScript.richtest.Cache") else {
        eprintln!("skip: richtest sample not present");
        return;
    };
    let refs = RefResolver::build(&b).expect("resolver");
    let funcs = collect_function_bytecodes(&b).expect("collect");
    let m = funcs.iter().find(|f| f.func.ends_with("::method1")).expect("method1");
    let src = decompile_function(m, &refs);
    eprintln!("{src}");
    // source was: int method1(int a, float b) { return a + field1; }
    assert!(src.contains("return"), "has return");
    assert!(src.contains("field1"), "resolves member field1");
    assert!(src.contains("a + this.field1") || src.contains("(a + this.field1)"), "reconstructs a + this.field1:\n{src}");
}

/// Decompile a couple of real modules (env-gated) and print them; must not panic.
#[test]
fn decompiles_real_targets() {
    let Ok(path) = std::env::var("GORE_AS_REAL_CACHE") else {
        eprintln!("skip: set GORE_AS_REAL_CACHE");
        return;
    };
    let b = std::fs::read(&path).expect("read real cache");
    let refs = RefResolver::build(&b).expect("resolver");
    let funcs = collect_function_bytecodes(&b).expect("collect");
    for needle in ["LevelFormula", "GE_Kill"] {
        if let Some(f) = funcs.iter().find(|f| f.func.contains(needle)) {
            eprintln!("==== {} ====\n{}", f.func, decompile_function(f, &refs));
        }
    }
    // decompiling every function must not panic
    for f in funcs.iter().take(2000) {
        let _ = decompile_function(f, &refs);
    }
}
