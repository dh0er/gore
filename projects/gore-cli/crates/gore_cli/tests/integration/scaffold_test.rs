use assert_cmd::Command;
use tempfile::TempDir;

#[test]
fn scaffold_creates_mod_structure() {
    let tmp = TempDir::new().unwrap();

    Command::cargo_bin("gore-cli")
        .unwrap()
        .args(["scaffold", "MyBalanceMod", "-o", tmp.path().to_str().unwrap()])
        .assert()
        .success();

    let mod_dir = tmp.path().join("MyBalanceMod");
    assert!(mod_dir.is_dir(), "mod directory must exist");
    assert!(mod_dir.join("enabled.txt").exists(), "enabled.txt must exist");

    let scripts_dir = mod_dir.join("Scripts");
    assert!(scripts_dir.is_dir(), "Scripts/ dir must exist");

    let main_lua = scripts_dir.join("main.lua");
    assert!(main_lua.exists(), "main.lua must exist");

    let content = std::fs::read_to_string(&main_lua).unwrap();
    assert!(content.contains("StaticFindObject"), "must include CDO pattern comment");
    assert!(content.contains("Default__"), "must mention Default__ prefix");
    assert!(content.contains("MyBalanceMod"), "must include mod name");
}

#[test]
fn scaffold_main_lua_wires_gorelib_loader() {
    let dir = TempDir::new().unwrap();
    Command::cargo_bin("gore-cli")
        .unwrap()
        .args(["scaffold", "MyMod", "-o", dir.path().to_str().unwrap()])
        .assert()
        .success();
    let main = std::fs::read_to_string(dir.path().join("MyMod/Scripts/main.lua")).unwrap();
    assert!(
        main.contains(r#"require("gorelib")"#) || main.contains(r#"require, "gorelib")"#),
        "should require gorelib"
    );
    assert!(main.contains("loadfile"), "should have a loadfile fallback");
}

#[test]
fn scaffold_enabled_txt_is_empty() {
    let tmp = TempDir::new().unwrap();
    Command::cargo_bin("gore-cli")
        .unwrap()
        .args(["scaffold", "EmptyMod", "-o", tmp.path().to_str().unwrap()])
        .assert()
        .success();
    let content = std::fs::read_to_string(tmp.path().join("EmptyMod/enabled.txt")).unwrap();
    assert!(content.is_empty() || content.trim().is_empty());
}
