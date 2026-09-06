//! `gore npc` gegen das gebaute Binary.
//!
//! Alles hier kommt ohne Spielinstallation aus: geprüft wird der eingebettete Katalog, und dass
//! die Kommandos, die eine Installation brauchen, ohne sie sauber scheitern statt zu raten.

use assert_cmd::Command;
use predicates::str::contains;
use tempfile::TempDir;

fn gore() -> Command {
    Command::cargo_bin("gore").expect("built binary")
}

#[test]
fn list_finds_diego_by_substring() {
    gore()
        .args(["npc", "list", "diego"])
        .assert()
        .success()
        .stdout(contains("OC_STT_Diego"))
        .stdout(contains("2 of 2 shown"));
}

#[test]
fn list_narrows_by_category() {
    gore()
        .args(["npc", "list", "--category", "creature", "--max", "3"])
        .assert()
        .success()
        .stdout(contains("creature"));
}

#[test]
fn list_json_reports_matched_and_listed_apart() {
    let out = gore()
        .args(["npc", "list", "--json", "--max", "1", "diego"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let doc: serde_json::Value = serde_json::from_slice(&out).expect("json");
    assert_eq!(doc["matched"], 2);
    assert_eq!(doc["listed"], 1);
    assert_eq!(doc["npcs"].as_array().expect("npcs array").len(), 1);
}

#[test]
fn a_filter_that_matches_nothing_says_so_instead_of_failing() {
    gore()
        .args(["npc", "list", "zzzznosuchnpc"])
        .assert()
        .success()
        .stdout(contains("0 of 0 shown"));
}

#[test]
fn list_without_a_filter_reaches_the_whole_bundled_catalog() {
    let out = gore()
        .args(["npc", "list", "--json", "--max", "1"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let doc: serde_json::Value = serde_json::from_slice(&out).expect("json");
    assert_eq!(doc["matched"], 1095);
}

#[test]
fn show_refuses_a_game_path_that_holds_no_script_cache() {
    let tmp = TempDir::new().expect("temp dir");
    gore()
        .args(["npc", "show", "OC_STT_Diego", "--game"])
        .arg(tmp.path())
        .assert()
        .failure()
        .stderr(contains("no script cache at"));
}

#[test]
fn sites_refuses_a_game_path_that_holds_no_script_cache() {
    let tmp = TempDir::new().expect("temp dir");
    gore()
        .args(["npc", "sites", "--game"])
        .arg(tmp.path())
        .assert()
        .failure()
        .stderr(contains("no script cache at"));
}

#[test]
fn stage_binds_single_module_commands_to_the_selected_and_configured_cache() {
    let tmp = TempDir::new().unwrap();
    let game = tmp.path().join("game");
    let cache = gore_mod::resolve_game_paths(&game).script_cache;
    std::fs::create_dir_all(cache.parent().unwrap()).unwrap();
    let selected = tmp.path().join("selected.Cache");
    std::fs::write(&selected, b"checkout cache").unwrap();
    let digest: String = gore_as::cache::faithfulness::cache_seal(b"checkout cache")
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();

    // Isolate configuration and disable installation discovery as in config_test.
    let command = || {
        let mut cmd = gore();
        cmd.env("LOCALAPPDATA", tmp.path())
            .env("APPDATA", tmp.path())
            .env("XDG_DATA_HOME", tmp.path())
            .env("HOME", tmp.path())
            .env("GORE_DISABLE_GAME_AUTODETECT", "1");
        cmd
    };
    command()
        .args(["config", "set", "game-path"])
        .arg(&game)
        .assert()
        .success();

    for operation in ["checkout", "suppress"] {
        let dir = tmp.path().join(operation);
        std::fs::create_dir(&dir).unwrap();
        let manifest = serde_json::json!({
            "operation": operation,
            "npc_id": "OC_STT_Diego",
            "modules": [{
                "module": "LevelScripts.XardasTower_AI",
                "relative_path": "LevelScripts/XardasTower_AI.as",
                "source_file": "XardasTower_AI.as",
                "op": "edit"
            }],
            "world_points": [],
            "level_module": "LevelScripts.XardasTower_AI",
            "cache_sha256": digest,
            "modular_visuals": false
        });
        std::fs::write(
            dir.join("gore-npc-edit.json"),
            serde_json::to_vec(&manifest).unwrap(),
        )
        .unwrap();
        let stage = || {
            let mut cmd = command();
            cmd.args(["npc", "stage"])
                .arg(&dir)
                .args(["--mod-name", "ToughDiego", "--cache"])
                .arg(&selected);
            cmd
        };

        // Missing or unreadable source must fail before any staging artifacts appear.
        std::fs::write(&cache, b"checkout cache").unwrap();
        let source = dir.join("XardasTower_AI.as");
        let source_error = format!("reading {}", source.display());
        stage()
            .assert()
            .failure()
            .stderr(contains(source_error.clone()));
        std::fs::create_dir(&source).unwrap();
        stage().assert().failure().stderr(contains(source_error));
        std::fs::remove_dir(&source).unwrap();
        assert!(!dir.join("spec.json").exists());
        assert!(!tmp.path().join(format!("{operation}.work")).exists());
        std::fs::write(&source, "// authored source\n").unwrap();

        std::fs::write(&cache, b"different installed cache").unwrap();
        stage().assert().failure().stderr(contains("not the cache"));
        assert!(!dir.join("spec.json").exists());
        assert!(!tmp.path().join(format!("{operation}.work")).exists());

        std::fs::write(&cache, b"checkout cache").unwrap();
        std::fs::write(&selected, b"different selected cache").unwrap();
        stage().assert().failure().stderr(contains("not the cache"));

        std::fs::write(&selected, b"checkout cache").unwrap();
        stage()
            .assert()
            .success()
            .stdout(contains("gore as compile-module"))
            .stdout(contains(format!("--game \"{}\"", game.display())));
        assert!(dir.join("spec.json").is_file());
    }
}
