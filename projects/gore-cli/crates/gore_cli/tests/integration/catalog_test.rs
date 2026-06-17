use assert_cmd::Command;
use std::path::PathBuf;
use tempfile::TempDir;

fn sdk_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/integration/fixtures/sdk")
}

#[test]
fn catalog_produces_json() {
    let tmp = TempDir::new().unwrap();
    let model = tmp.path().join("model.json");
    Command::cargo_bin("gore-cli")
        .unwrap()
        .args(["dump", sdk_dir().to_str().unwrap(), "-o", model.to_str().unwrap()])
        .assert()
        .success();

    let catalog_out = tmp.path().join("item_catalog.json");
    Command::cargo_bin("gore-cli")
        .unwrap()
        .args(["catalog", model.to_str().unwrap(), "-o", catalog_out.to_str().unwrap()])
        .assert()
        .success();

    assert!(catalog_out.exists());
    let content = std::fs::read_to_string(&catalog_out).unwrap();
    let items: serde_json::Value = serde_json::from_str(&content).unwrap();
    let arr = items.as_array().unwrap();
    // The fixture has ItFo_Apple — must appear in catalog
    assert!(arr.iter().any(|e| e["id"].as_str() == Some("ItFo_Apple")));
    // UItemDefinition is not an item instance — must NOT appear
    assert!(!arr.iter().any(|e| e["id"].as_str() == Some("UItemDefinition")));
}

#[test]
fn catalog_entry_has_category() {
    let tmp = TempDir::new().unwrap();
    let model = tmp.path().join("model.json");
    Command::cargo_bin("gore-cli")
        .unwrap()
        .args(["dump", sdk_dir().to_str().unwrap(), "-o", model.to_str().unwrap()])
        .assert()
        .success();

    let catalog_out = tmp.path().join("catalog.json");
    Command::cargo_bin("gore-cli")
        .unwrap()
        .args(["catalog", model.to_str().unwrap(), "-o", catalog_out.to_str().unwrap()])
        .assert()
        .success();

    let content = std::fs::read_to_string(&catalog_out).unwrap();
    let items: Vec<serde_json::Value> = serde_json::from_str(&content).unwrap();
    let apple = items.iter().find(|e| e["id"].as_str() == Some("ItFo_Apple")).unwrap();
    assert_eq!(apple["category"].as_str(), Some("Food"));
}
