use gore_as::cache::disasm::disassemble;
use gore_as::cache::walk_modules::collect_function_bytecodes;

const SAMPLES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../work/reversing/gore-as/samples");

fn read_sample(name: &str) -> Option<Vec<u8>> {
    std::fs::read(format!("{SAMPLES}/{name}")).ok()
}

#[test]
fn disassembles_richtest_method1() {
    let Some(b) = read_sample("PrecompiledScript.richtest.Cache") else {
        eprintln!("skip: richtest sample not present");
        return;
    };
    let funcs = collect_function_bytecodes(&b).expect("collect");
    let m = funcs
        .iter()
        .find(|f| f.func.ends_with("::method1"))
        .expect("method1 found among collected functions");
    assert!(!m.bytecode.is_empty(), "method1 has bytecode");
    let instrs = disassemble(&m.bytecode).expect("disasm method1");
    assert!(!instrs.is_empty());
    // The walk consumed the whole bytecode array exactly.
    let last = instrs.last().unwrap();
    assert_eq!(
        last.offset_dw + last.op.size_dwords as usize,
        m.bytecode.len(),
        "disasm consumes the whole bytecode array"
    );
    // method1 = `return a + field1;` -> must contain an integer ADD and a RET.
    assert!(instrs.iter().any(|x| x.op.name == "RET"), "has RET");
    assert!(
        instrs.iter().any(|x| x.op.name.starts_with("ADDi")),
        "has integer ADD (a + field1): {}",
        gore_as::cache::disasm::listing(&instrs)
    );
    eprintln!(
        "{} -> {} instrs:\n{}",
        m.func,
        instrs.len(),
        gore_as::cache::disasm::listing(&instrs)
    );
}

#[test]
fn all_richtest_functions_disassemble_clean() {
    let Some(b) = read_sample("PrecompiledScript.richtest.Cache") else {
        return;
    };
    let funcs = collect_function_bytecodes(&b).expect("collect");
    assert!(!funcs.is_empty());
    for f in &funcs {
        let r = disassemble(&f.bytecode);
        assert!(r.is_ok(), "disasm failed for {}: {:?}", f.func, r.err());
    }
}

/// Strong ISA validation: disassemble every function in the real 122 MB cache and
/// report the clean rate. Wrong opcode sizes would desync -> Truncated/Unknown.
/// Set GORE_AS_REAL_CACHE to run.
#[test]
fn real_cache_functions_disassemble() {
    let Ok(path) = std::env::var("GORE_AS_REAL_CACHE") else {
        eprintln!("skip: set GORE_AS_REAL_CACHE");
        return;
    };
    let b = std::fs::read(&path).expect("read real cache");
    let funcs = collect_function_bytecodes(&b).expect("collect real");
    let total = funcs.len();
    let mut ok = 0usize;
    let mut errs: Vec<String> = Vec::new();
    for f in &funcs {
        match disassemble(&f.bytecode) {
            Ok(_) => ok += 1,
            Err(e) => {
                if errs.len() < 10 {
                    errs.push(format!("{}: {e}", f.func));
                }
            }
        }
    }
    eprintln!("real cache: {ok}/{total} functions disassembled clean");
    for e in &errs {
        eprintln!("  ERR {e}");
    }
    // Expect the vast majority to disassemble; a perfect ISA gives 100%.
    assert!(ok * 100 / total.max(1) >= 99, "clean rate too low: {ok}/{total}");
}
