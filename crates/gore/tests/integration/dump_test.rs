use assert_cmd::Command;
use std::path::PathBuf;
use tempfile::TempDir;

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/integration/fixtures")
}

#[test]
fn dump_produces_model_json() {
    let tmp = TempDir::new().unwrap();
    let out = tmp.path().join("model.json");
    let sdk_dir = fixtures_dir().join("sdk");

    Command::cargo_bin("gore")
        .unwrap()
        .args(["dump", sdk_dir.to_str().unwrap(), "-o", out.to_str().unwrap()])
        .assert()
        .success();

    assert!(out.exists(), "model.json must be created");
    let content = std::fs::read_to_string(&out).unwrap();
    let value: serde_json::Value = serde_json::from_str(&content).unwrap();
    // Should contain at least the 4 classes from the snippet fixture
    let classes = value["classes"].as_array().unwrap();
    assert!(classes.len() >= 4, "expected >=4 classes, got {}", classes.len());
    let names: Vec<&str> = classes.iter()
        .filter_map(|c| c["name"].as_str())
        .collect();
    assert!(names.contains(&"UItemDefinition"), "must contain UItemDefinition");
    assert!(names.contains(&"ItFo_Apple"), "must contain ItFo_Apple");
}

#[test]
fn dump_fails_on_missing_dir() {
    let tmp = TempDir::new().unwrap();
    Command::cargo_bin("gore")
        .unwrap()
        .args(["dump", "/no/such/dir", "-o", tmp.path().join("out.json").to_str().unwrap()])
        .assert()
        .failure();
}
