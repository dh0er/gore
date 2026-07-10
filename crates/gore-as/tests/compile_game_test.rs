// Real compile via the installed game. Run with:
//   GORE_TEST_GAME="C:/.../Gothic 1 Remake" cargo test -p gore-as -- --ignored real_compile_add
#[test]
#[ignore]
fn real_compile_add() {
    let Ok(game) = std::env::var("GORE_TEST_GAME") else { return; };
    let game = std::path::PathBuf::from(game);
    let work = std::env::temp_dir().join("gore-as-real-compile");
    let _ = std::fs::remove_dir_all(&work);
    std::fs::create_dir_all(&work).unwrap();
    // A trivial primitive-only module.
    let as_path = work.join("GoreHello.as");
    std::fs::write(&as_path, "int GoreHello(){ return 42; }").unwrap();
    let opts = gore_as::compile::CompileOpts {
        game_dir: game,
        op: "add".into(),
        module_name: "GoreHello".into(),
        rel_path: "GoreHello.as".into(),
        as_path,
        work_dir: work.clone(),
        base_override: None,
    };
    let out = gore_as::compile::compile_module(&opts, gore_as::compile::game_run_regen).unwrap();
    assert!(out.mini_path.exists());
    let mini = std::fs::read(&out.mini_path).unwrap();
    assert_eq!(gore_as::cache::walk_modules::module_count(&mini), 1);
}
