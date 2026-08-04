use assert_cmd::Command;
use std::path::PathBuf;
use tempfile::TempDir;

fn make_model_json(tmp: &TempDir) -> PathBuf {
    // Build a small model.json by running dump first
    let sdk_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/integration/fixtures/sdk");
    let model_path = tmp.path().join("model.json");
    Command::cargo_bin("gore")
        .unwrap()
        .args([
            "dump",
            sdk_dir.to_str().unwrap(),
            "-o",
            model_path.to_str().unwrap(),
        ])
        .assert()
        .success();
    model_path
}

#[test]
fn stubs_creates_lua_files() {
    let tmp = TempDir::new().unwrap();
    let model = make_model_json(&tmp);
    let out_dir = tmp.path().join("stubs");

    Command::cargo_bin("gore")
        .unwrap()
        .args([
            "stubs",
            model.to_str().unwrap(),
            "-o",
            out_dir.to_str().unwrap(),
        ])
        .assert()
        .success();

    // At least one .lua file must exist
    let stubs: Vec<_> = std::fs::read_dir(&out_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|x| x == "lua").unwrap_or(false))
        .collect();
    assert!(!stubs.is_empty(), "expected .lua stub files");
}

#[test]
fn stubs_lua_file_contains_emmylua_annotations() {
    let tmp = TempDir::new().unwrap();
    let model = make_model_json(&tmp);
    let out_dir = tmp.path().join("stubs");

    Command::cargo_bin("gore")
        .unwrap()
        .args([
            "stubs",
            model.to_str().unwrap(),
            "-o",
            out_dir.to_str().unwrap(),
        ])
        .assert()
        .success();

    // Find UItemDefinition.lua or combined file
    let combined = out_dir.join("UItemDefinition.lua");
    let content = std::fs::read_to_string(&combined).expect("UItemDefinition.lua must exist");
    assert!(
        content.contains("---@class UItemDefinition"),
        "must have @class"
    );
    assert!(
        content.contains("---@field m_Value"),
        "must have m_Value field"
    );
    assert!(
        content.contains("---@field m_Weight"),
        "must have m_Weight field"
    );
}

#[test]
fn stubs_filter_limits_output() {
    let tmp = TempDir::new().unwrap();
    let model = make_model_json(&tmp);
    let out_dir = tmp.path().join("stubs_filtered");

    Command::cargo_bin("gore")
        .unwrap()
        .args([
            "stubs",
            model.to_str().unwrap(),
            "-o",
            out_dir.to_str().unwrap(),
            "--filter",
            "ItFo",
        ])
        .assert()
        .success();

    // Only ItFo_Apple.lua should exist; UItemDefinition.lua must not
    let files: Vec<_> = std::fs::read_dir(&out_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();
    assert!(
        files.iter().any(|f| f.starts_with("ItFo")),
        "ItFo class must be present"
    );
    assert!(
        !files.iter().any(|f| f.starts_with("UItem")),
        "UItemDefinition must be filtered out"
    );
}
