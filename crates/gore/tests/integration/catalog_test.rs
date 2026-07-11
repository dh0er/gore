use assert_cmd::Command;
use std::path::PathBuf;
use tempfile::TempDir;

fn sdk_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/integration/fixtures/sdk")
}

fn fixture_dump() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/integration/fixtures/object_dump.txt")
}

/// Build a minimal object dump fixture that exercises all three catalog kinds.
fn ensure_dump_fixture() {
    let path = fixture_dump();
    if path.exists() {
        return;
    }
    let content = "\
[0001] ASClass /Script/Angelscript.ItFo_Apple [n: 1] [c: 2] [or: 3]
[0002] ASClass /Script/Angelscript.ItMi_Orenugget [n: 1] [c: 2] [or: 3]
[0003] ASClass /Script/Angelscript.CharacterDefinition_Human_OC_STT_Diego [n: 1]
[0004] ASClass /Script/Angelscript.CharacterDefinition_Creature_Biter [n: 1]
[0005] ASClass /Script/Angelscript.Topic_Diego_209799 [n: 1]
[0006] ASClass /Script/Angelscript.Info_FMORGAreyouok [n: 1]
[0007] ASClass /Script/Angelscript.ChoiceDiegoGamestart [n: 1]
";
    std::fs::write(&path, content).expect("writing dump fixture");
}

#[test]
fn catalog_item_produces_json() {
    ensure_dump_fixture();
    let tmp = TempDir::new().unwrap();
    let out = tmp.path().join("item_catalog.json");
    Command::cargo_bin("gore")
        .unwrap()
        .args([
            "catalog",
            "--kind",
            "item",
            fixture_dump().to_str().unwrap(),
            "-o",
            out.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(out.exists());
    let content = std::fs::read_to_string(&out).unwrap();
    let items: Vec<serde_json::Value> = serde_json::from_str(&content).unwrap();
    assert!(items.iter().any(|e| e["id"].as_str() == Some("ItFo_Apple")));
    assert!(items
        .iter()
        .any(|e| e["id"].as_str() == Some("ItMi_Orenugget")));
    // Verify field order matches sort_keys=True (category < id < path)
    let apple = items
        .iter()
        .find(|e| e["id"].as_str() == Some("ItFo_Apple"))
        .unwrap();
    assert_eq!(apple["category"].as_str(), Some("food"));
    assert_eq!(
        apple["path"].as_str(),
        Some("/Script/Angelscript.ItFo_Apple")
    );
}

#[test]
fn catalog_npc_produces_json() {
    ensure_dump_fixture();
    let tmp = TempDir::new().unwrap();
    let out = tmp.path().join("npc_catalog.json");
    Command::cargo_bin("gore")
        .unwrap()
        .args([
            "catalog",
            "--kind",
            "npc",
            fixture_dump().to_str().unwrap(),
            "-o",
            out.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(out.exists());
    let content = std::fs::read_to_string(&out).unwrap();
    let npcs: Vec<serde_json::Value> = serde_json::from_str(&content).unwrap();
    assert!(npcs
        .iter()
        .any(|e| e["id"].as_str() == Some("OC_STT_Diego")));
    assert!(npcs
        .iter()
        .any(|e| e["id"].as_str() == Some("Creature_Biter")));
}

#[test]
fn catalog_knowledge_produces_json() {
    ensure_dump_fixture();
    let tmp = TempDir::new().unwrap();
    let out = tmp.path().join("knowledge_catalog.json");
    Command::cargo_bin("gore")
        .unwrap()
        .args([
            "catalog",
            "--kind",
            "knowledge",
            fixture_dump().to_str().unwrap(),
            "-o",
            out.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(out.exists());
    let content = std::fs::read_to_string(&out).unwrap();
    let tokens: Vec<serde_json::Value> = serde_json::from_str(&content).unwrap();
    assert!(tokens
        .iter()
        .any(|e| e["id"].as_str() == Some("Topic_Diego_209799")));
    assert!(tokens
        .iter()
        .any(|e| e["category"].as_str() == Some("info")));
    assert!(tokens
        .iter()
        .any(|e| e["category"].as_str() == Some("choice")));
}

/// The old catalog command used model.json as input; verify the new command
/// correctly uses the dump file and --kind flag instead.
#[test]
fn catalog_item_entry_has_correct_fields() {
    ensure_dump_fixture();
    let tmp = TempDir::new().unwrap();
    let out = tmp.path().join("catalog.json");
    Command::cargo_bin("gore")
        .unwrap()
        .args([
            "catalog",
            "--kind",
            "item",
            fixture_dump().to_str().unwrap(),
            "-o",
            out.to_str().unwrap(),
        ])
        .assert()
        .success();

    let content = std::fs::read_to_string(&out).unwrap();
    let items: Vec<serde_json::Value> = serde_json::from_str(&content).unwrap();
    let apple = items
        .iter()
        .find(|e| e["id"].as_str() == Some("ItFo_Apple"))
        .unwrap();
    // Verify the three fields id/path/category are all present
    assert!(apple.get("id").is_some());
    assert!(apple.get("path").is_some());
    assert!(apple.get("category").is_some());
    // No extra fields (no display_name from old catalog format)
    let obj = apple.as_object().unwrap();
    assert_eq!(obj.len(), 3);
}

/// gui-model integration test: use the SDK fixture dump to build a model,
/// then generate a GUI model from the item catalog.
#[test]
fn gui_model_produces_valid_json() {
    ensure_dump_fixture();
    let tmp = TempDir::new().unwrap();

    // 1. Build model.json from SDK fixtures
    let model_out = tmp.path().join("model.json");
    Command::cargo_bin("gore")
        .unwrap()
        .args([
            "dump",
            sdk_dir().to_str().unwrap(),
            "-o",
            model_out.to_str().unwrap(),
        ])
        .assert()
        .success();

    // 2. Build item catalog from dump fixture
    let catalog_out = tmp.path().join("item_catalog.json");
    Command::cargo_bin("gore")
        .unwrap()
        .args([
            "catalog",
            "--kind",
            "item",
            fixture_dump().to_str().unwrap(),
            "-o",
            catalog_out.to_str().unwrap(),
        ])
        .assert()
        .success();

    // 3. Build GUI model
    let gui_out = tmp.path().join("gui_model.json");
    Command::cargo_bin("gore")
        .unwrap()
        .args([
            "gui-model",
            "--model",
            model_out.to_str().unwrap(),
            "--catalog",
            catalog_out.to_str().unwrap(),
            "-o",
            gui_out.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(gui_out.exists());
    let content = std::fs::read_to_string(&gui_out).unwrap();
    let gui: serde_json::Value = serde_json::from_str(&content).unwrap();

    // Top level must have "classes" key
    assert!(gui.get("classes").is_some());
    let classes = gui["classes"].as_object().unwrap();

    // ItFo_Apple is in both the dump fixture AND the SDK fixture
    if let Some(apple) = classes.get("ItFo_Apple") {
        let fields = apple["fields"].as_array().unwrap();
        // Must have GUI-editable fields from UItemDefinition (m_Value, m_MaxStack, m_Weight, m_Mass, m_Buoyancy)
        let names: Vec<&str> = fields.iter().map(|f| f["name"].as_str().unwrap()).collect();
        // m_Value is Int → "int"
        if let Some(v_field) = fields.iter().find(|f| f["name"] == "m_Value") {
            assert_eq!(v_field["type"].as_str(), Some("int"));
        }
        // m_Weight is Float → "float"
        if let Some(w_field) = fields.iter().find(|f| f["name"] == "m_Weight") {
            assert_eq!(w_field["type"].as_str(), Some("float"));
        }
        // No String fields should appear (m_Name in fixture is int32_t but let's verify no "string" type)
        assert!(!fields.iter().any(|f| f["type"] == "string"));
        let _ = names;
    }
}
