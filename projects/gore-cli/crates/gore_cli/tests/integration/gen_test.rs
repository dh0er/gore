use assert_cmd::Command;
use std::path::PathBuf;
use tempfile::TempDir;

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/integration/fixtures")
}

fn sdk_dir() -> PathBuf {
    fixtures().join("sdk")
}

#[test]
fn gen_creates_mod_dir_and_main_lua() {
    let tmp = TempDir::new().unwrap();
    let overrides = fixtures().join("overrides.toml");

    Command::cargo_bin("gore-cli")
        .unwrap()
        .args(["gen", overrides.to_str().unwrap(), "-o", tmp.path().to_str().unwrap()])
        .assert()
        .success();

    let mod_dir = tmp.path().join("IntegrationTestMod");
    assert!(mod_dir.is_dir(), "mod directory must be created");
    assert!(mod_dir.join("enabled.txt").exists(), "enabled.txt must exist");

    let main_lua = mod_dir.join("Scripts").join("main.lua");
    assert!(main_lua.exists(), "Scripts/main.lua must exist");
    let content = std::fs::read_to_string(&main_lua).unwrap();
    // The CDO path is built at runtime from the per-override module (default
    // Angelscript), so assert the runtime template + the module field.
    assert!(content.contains(r#"".Default__" .. o.class"#));
    assert!(content.contains(r#"module="Angelscript""#));
    assert!(content.contains("ItFo_Apple"));
    assert!(content.contains("m_Value"));
    assert!(content.contains("500"));
}

#[test]
fn gen_round_trip_lua_shape() {
    let tmp = TempDir::new().unwrap();
    let overrides = fixtures().join("overrides.toml");

    Command::cargo_bin("gore-cli")
        .unwrap()
        .args(["gen", overrides.to_str().unwrap(), "-o", tmp.path().to_str().unwrap()])
        .assert()
        .success();

    let lua = std::fs::read_to_string(
        tmp.path().join("IntegrationTestMod/Scripts/main.lua")
    ).unwrap();
    // Both overrides present
    assert!(lua.contains("m_Value"));
    assert!(lua.contains("m_Weight"));
    assert!(lua.contains("0.1") || lua.contains("0.1f"));
    // Log format
    assert!(lua.contains("IntegrationTestMod"));
    assert!(lua.contains("tostring(before)") || lua.contains("before"));
}

#[test]
fn gen_with_valid_model_succeeds() {
    let tmp = TempDir::new().unwrap();
    let model = tmp.path().join("model.json");
    Command::cargo_bin("gore-cli")
        .unwrap()
        .args(["dump", sdk_dir().to_str().unwrap(), "-o", model.to_str().unwrap()])
        .assert()
        .success();

    Command::cargo_bin("gore-cli")
        .unwrap()
        .args([
            "gen", fixtures().join("overrides.toml").to_str().unwrap(),
            "-o", tmp.path().to_str().unwrap(),
            "--model", model.to_str().unwrap(),
        ])
        .assert()
        .success();
}

#[test]
fn gen_with_unknown_class_fails_validation() {
    let tmp = TempDir::new().unwrap();
    let model = tmp.path().join("model.json");
    Command::cargo_bin("gore-cli")
        .unwrap()
        .args(["dump", sdk_dir().to_str().unwrap(), "-o", model.to_str().unwrap()])
        .assert()
        .success();

    // Write a bad overrides toml with a nonexistent class
    let bad_toml = tmp.path().join("bad_overrides.toml");
    std::fs::write(&bad_toml,
        "[meta]\nname = \"BadMod\"\ndelay_ms = 0\n\n[[override]]\nclass = \"NonExistentClass\"\nfield = \"m_Value\"\nvalue_int = 1\n"
    ).unwrap();

    Command::cargo_bin("gore-cli")
        .unwrap()
        .args([
            "gen", bad_toml.to_str().unwrap(),
            "-o", tmp.path().join("mods").to_str().unwrap(),
            "--model", model.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(predicates::str::contains("NonExistentClass"));
}
