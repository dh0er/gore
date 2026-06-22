use gore_as::cache::decompile::decompile_function;
use gore_as::cache::disasm::{disassemble, listing};
use gore_as::cache::refs::RefResolver;
use gore_as::cache::walk_modules::collect_function_bytecodes;
use gore_as::cache::cfg;

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

#[test]
fn dump_branchtest() {
    let Some(b) = read_sample("PrecompiledScript.branchtest.Cache") else {
        eprintln!("skip: branchtest sample not present");
        return;
    };
    let refs = RefResolver::build(&b).expect("resolver");
    let funcs = collect_function_bytecodes(&b).expect("collect");
    let f = funcs.iter().find(|f| f.func.contains("GoreBranchTest")).expect("GoreBranchTest");
    let instrs = disassemble(&f.bytecode).unwrap();
    eprintln!("=== DISASM {} ===\n{}", f.func, listing(&instrs));
    let g = cfg::build(&instrs);
    eprintln!("=== CFG: {} blocks (back_edge={}) ===", g.blocks.len(), g.has_back_edge());
    for bb in &g.blocks {
        eprintln!("  block @{} instrs[{}..{}] -> {:?}", bb.start_dw, bb.instr_lo, bb.instr_hi, bb.succs);
    }
    eprintln!("=== DECOMPILE (linear) ===\n{}", decompile_function(f, &refs));
    eprintln!("=== DECOMPILE (structured) ===\n{}", gore_as::cache::structure::decompile(f, &refs));
}

#[test]
fn structures_branchtest() {
    let Some(b) = read_sample("PrecompiledScript.branchtest.Cache") else {
        return;
    };
    let refs = RefResolver::build(&b).unwrap();
    let funcs = collect_function_bytecodes(&b).unwrap();
    let f = funcs.iter().find(|f| f.func.contains("GoreBranchTest")).unwrap();
    let src = gore_as::cache::structure::decompile(f, &refs);
    // source: for(i<n) sum+=i; if(sum>100) sum=100 else sum+1; while(sum>0) sum-=5; return sum
    assert!(src.contains("while (local_3 < n)"), "for-loop:\n{src}");
    assert!(src.contains("if (local_1 > 100)"), "if cond:\n{src}");
    assert!(src.contains("else"), "else branch:\n{src}");
    assert!(src.contains("while (local_1 > 0)"), "while-loop:\n{src}");
    assert!(src.contains("return local_1;"), "return:\n{src}");
}

/// Structured decompile must not panic or hang across many real functions (loop guard).
#[test]
fn structured_real_cache_no_panic() {
    let Ok(path) = std::env::var("GORE_AS_REAL_CACHE") else {
        eprintln!("skip: set GORE_AS_REAL_CACHE");
        return;
    };
    let b = std::fs::read(&path).unwrap();
    let refs = RefResolver::build(&b).unwrap();
    let funcs = collect_function_bytecodes(&b).unwrap();
    let mut chars = 0usize;
    for f in funcs.iter().take(20000) {
        chars += gore_as::cache::structure::decompile(f, &refs).len();
    }
    eprintln!("structured 20000 funcs, {chars} chars total");
}

/// Tally opcode frequency across all real function bytecode (to prioritize op coverage).
#[test]
fn opcode_frequency() {
    let Ok(path) = std::env::var("GORE_AS_REAL_CACHE") else {
        eprintln!("skip: set GORE_AS_REAL_CACHE");
        return;
    };
    let b = std::fs::read(&path).unwrap();
    let funcs = collect_function_bytecodes(&b).unwrap();
    let mut freq: std::collections::HashMap<&'static str, u64> = std::collections::HashMap::new();
    for f in &funcs {
        if let Ok(ins) = gore_as::cache::disasm::disassemble(&f.bytecode) {
            for i in &ins {
                *freq.entry(i.op.name).or_insert(0) += 1;
            }
        }
    }
    let mut v: Vec<_> = freq.into_iter().collect();
    v.sort_by(|a, b| b.1.cmp(&a.1));
    eprintln!("=== top 45 opcodes by frequency ===");
    for (name, n) in v.iter().take(45) {
        eprintln!("{n:>9}  {name}");
    }
}

/// Measure body-recovery rate across ALL real functions: a body is "clean" if it has no
/// raw `// OPCODE` annotation / disasm error (i.e. fully decompiled, no stub needed).
#[test]
fn body_recovery_rate() {
    let Ok(path) = std::env::var("GORE_AS_REAL_CACHE") else {
        eprintln!("skip: set GORE_AS_REAL_CACHE");
        return;
    };
    let b = std::fs::read(&path).unwrap();
    let refs = RefResolver::build(&b).unwrap();
    let funcs = collect_function_bytecodes(&b).unwrap();
    let total = funcs.len();
    let mut clean = 0usize;
    for f in &funcs {
        let body = gore_as::cache::structure::body_statements(f, &refs, 1);
        let stub = body.lines().any(|l| {
            let t = l.trim_start();
            t.starts_with("// ") && {
                let w = t.trim_start_matches("// ").split_whitespace().next().unwrap_or("");
                w.chars().next().map(|c| c.is_ascii_uppercase()).unwrap_or(false)
                    && w.chars().all(|c| c.is_ascii_alphanumeric())
            }
        });
        if !stub {
            clean += 1;
        }
    }
    eprintln!("body recovery: {clean}/{total} functions fully decompiled ({}%)", clean * 100 / total.max(1));
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
