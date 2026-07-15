use gore_as::compile::{compile_module, CompileError, CompileOpts};
use std::path::{Path, PathBuf};

// A fake regen that just copies a fixture "regen" cache into place and returns it.
fn fake_regen_ok(fixture: PathBuf) -> impl Fn(&Path, &Path) -> Result<PathBuf, String> {
    move |_game_dir: &Path, _src_dir: &Path| Ok(fixture.clone())
}

#[test]
fn compile_errors_when_source_missing() {
    let tmp = std::env::temp_dir().join("gore-as-compile-missing");
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    let opts = CompileOpts {
        game_dir: tmp.clone(),
        op: "add".into(),
        module_name: "M".into(),
        rel_path: "M.as".into(),
        as_path: tmp.join("does-not-exist.as"),
        source_override: None,
        work_dir: tmp.clone(),
        allow_new_symbols: false,
        base_override: None,
        binds_override: None,
    };
    let err = compile_module(&opts, fake_regen_ok(tmp.join("regen.cache"))).unwrap_err();
    assert!(matches!(err, CompileError::Io(_)));
}
