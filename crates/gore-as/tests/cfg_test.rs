use gore_as::cache::cfg;
use gore_as::cache::disasm::disassemble;
use gore_as::cache::walk_modules::collect_function_bytecodes;

const SAMPLES: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../work/reversing/gore-as/samples"
);

fn read_sample(name: &str) -> Option<Vec<u8>> {
    std::fs::read(format!("{SAMPLES}/{name}")).ok()
}

#[test]
fn method1_is_single_block_no_loop() {
    let Some(b) = read_sample("PrecompiledScript.richtest.Cache") else {
        return;
    };
    let funcs = collect_function_bytecodes(&b).unwrap();
    let m = funcs
        .iter()
        .find(|f| f.func.ends_with("::method1"))
        .unwrap();
    let instrs = disassemble(&m.bytecode).unwrap();
    let g = cfg::build(&instrs);
    assert_eq!(g.blocks.len(), 1, "straight-line method1 = 1 block");
    assert!(!g.has_back_edge(), "no loop");
}

/// LevelFormula contains a for-loop -> the CFG must have a back-edge. (env-gated)
#[test]
fn levelformula_has_loop() {
    let Ok(path) = std::env::var("GORE_AS_REAL_CACHE") else {
        eprintln!("skip: set GORE_AS_REAL_CACHE");
        return;
    };
    let b = std::fs::read(&path).unwrap();
    let funcs = collect_function_bytecodes(&b).unwrap();
    let f = funcs
        .iter()
        .find(|f| f.func.contains("LevelFormula") && f.bytecode.len() > 20)
        .expect("a LevelFormula function");
    let instrs = disassemble(&f.bytecode).unwrap();
    let g = cfg::build(&instrs);
    eprintln!(
        "{}: {} blocks, back_edge={}",
        f.func,
        g.blocks.len(),
        g.has_back_edge()
    );
    assert!(g.blocks.len() > 1, "multi-block");
    assert!(g.has_back_edge(), "for-loop -> back-edge");
}
