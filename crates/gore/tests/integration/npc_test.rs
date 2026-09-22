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

fn routine_workspace(legacy: bool) -> TempDir {
    let dir = TempDir::new().unwrap();
    let routine = if legacy {
        "UDailyRoutine_MY_NPC_Start()"
    } else {
        "nullptr"
    };
    let mut source =
        "// keep my custom code\nclass UCharacterDefinition_Human_MY_NPC {}\n".to_string();
    if legacy {
        source.push_str("class UDailyRoutine_MY_NPC_Start : UAIState_DailyRoutine_Human\n{\n    default Schedule(0, 0, UAIState_Stand(), n\"FP_XT_WAIT_OUTSIDE\", 1000.0f, TSubclassOf<UNavArea>(nullptr), nullptr);\n    default TeleportToCurrentTaskWhen = EDailyRoutineTeleportMode::WhenOutOfBounds;\n}\n");
    }
    std::fs::write(dir.path().join("MY_NPC.as"), source.replace('\n', "\r\n")).unwrap();
    std::fs::write(dir.path().join("Level.as"), format!("// leave this alone\r\n    this.SpawnAIAgent(TSubclassOf<USpawnAIAgentDefinition>(USpawnAIAgentDefinition_MY_NPC::StaticClass()), {routine});\r\n// unrelated footer\r\n")).unwrap();
    std::fs::write(dir.path().join("gore-npc-edit.json"), serde_json::to_vec(&serde_json::json!({
        "operation":"new", "npc_id":"MY_NPC", "derived_from":"OC_STT_Diego",
        "modules":[
            {"module":"AI.MY_NPC","relative_path":"AI/MY_NPC.as","source_file":"MY_NPC.as","pristine_file":null,"op":"add"},
            {"module":"Level","relative_path":"Level.as","source_file":"Level.as","pristine_file":"pristine/Level.as","op":"edit"}
        ],
        "world_points":["UWP_TEST"],"level_module":"Level","cache_sha256":"a".repeat(64),"modular_visuals":false
    })).unwrap()).unwrap();
    dir
}

fn routine_set(
    dir: &TempDir,
    time: &str,
    activity: &str,
    spot: &str,
) -> assert_cmd::assert::Assert {
    gore()
        .args(["npc", "routine", "set"])
        .arg(dir.path())
        .args(["--time", time, "--activity", activity, "--spot", spot])
        .assert()
}

fn routine_show(dir: &TempDir) -> serde_json::Value {
    let output = gore()
        .args(["npc", "routine", "show"])
        .arg(dir.path())
        .arg("--json")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    serde_json::from_slice(&output).unwrap()
}

#[test]
fn routine_upserts_sorts_wires_spawn_and_preserves_unrelated_source() {
    let dir = routine_workspace(false);
    // Preserve manual code with LF even when the generated file started as CRLF.
    let source_path = dir.path().join("MY_NPC.as");
    let before = std::fs::read_to_string(&source_path).unwrap() + "// mixed ending\n";
    std::fs::write(&source_path, &before).unwrap();
    routine_set(&dir, "18:00", "drink", "FP_XT_WAIT_OUTSIDE").success();
    routine_set(&dir, "08:00", "read", "FP_NavigationSupport393").success();
    routine_set(&dir, "18:00", "stand", "FP_XT_WAIT_OUTSIDE").success();
    let shown = routine_show(&dir);
    assert_eq!(shown["activation_function"], "GoreApplyRoutine_MY_NPC");
    assert_eq!(shown["phases"].as_array().unwrap().len(), 2);
    assert_eq!(shown["phases"][0]["time"], "08:00");
    assert_eq!(shown["phases"][1]["activity"], "stand");
    let source = std::fs::read_to_string(dir.path().join("MY_NPC.as")).unwrap();
    assert!(source.starts_with(&before));
    assert!(source
        .starts_with("// keep my custom code\r\nclass UCharacterDefinition_Human_MY_NPC {}\r\n"));
    assert!(source.contains("EDailyRoutineTeleportMode::Never"));
    assert!(!source.contains("UAIState_GoreRoutine_MY_NPC_Drink"));
    let level = std::fs::read_to_string(dir.path().join("Level.as")).unwrap();
    assert_eq!(level, "// leave this alone\r\n    this.SpawnAIAgent(TSubclassOf<USpawnAIAgentDefinition>(USpawnAIAgentDefinition_MY_NPC::StaticClass()), UDailyRoutine_MY_NPC_Start());\r\n// unrelated footer\r\n");
}

#[test]
fn routine_legacy_migration_keeps_midnight_and_removal_extends_other_phase() {
    let dir = routine_workspace(true);
    assert_eq!(routine_show(&dir)["managed"], false);
    routine_set(&dir, "12:00", "drink", "FP_NavigationSupport393").success();
    let shown = routine_show(&dir);
    assert_eq!(shown["phases"][0]["time"], "00:00");
    assert_eq!(shown["phases"][0]["spot"], "FP_XT_WAIT_OUTSIDE");
    gore()
        .args(["npc", "routine", "remove"])
        .arg(dir.path())
        .args(["--time", "00:00"])
        .assert()
        .success();
    assert_eq!(routine_show(&dir)["phases"].as_array().unwrap().len(), 1);
    let before = std::fs::read(dir.path().join("MY_NPC.as")).unwrap();
    for time in ["12:00", "01:00"] {
        gore()
            .args(["npc", "routine", "remove"])
            .arg(dir.path())
            .args(["--time", time])
            .assert()
            .failure();
        assert_eq!(std::fs::read(dir.path().join("MY_NPC.as")).unwrap(), before);
    }
}

#[test]
fn routine_invalid_input_never_changes_either_file() {
    let dir = routine_workspace(false);
    let source = std::fs::read(dir.path().join("MY_NPC.as")).unwrap();
    let level = std::fs::read(dir.path().join("Level.as")).unwrap();
    routine_set(&dir, "24:00", "stand", "FP_XT_WAIT_OUTSIDE").failure();
    routine_set(&dir, "9:00", "stand", "FP_XT_WAIT_OUTSIDE").failure();
    routine_set(&dir, "09:00", "stand", "NO_SUCH_ROUTINE_SPOT")
        .failure()
        .stderr(contains("unknown routine spot"));
    assert_eq!(std::fs::read(dir.path().join("MY_NPC.as")).unwrap(), source);
    assert_eq!(std::fs::read(dir.path().join("Level.as")).unwrap(), level);
}

#[test]
fn routine_manual_block_and_spawn_changes_are_not_overwritten() {
    let dir = routine_workspace(false);
    routine_set(&dir, "08:00", "read", "FP_XT_WAIT_OUTSIDE").success();
    let path = dir.path().join("MY_NPC.as");
    let source = std::fs::read_to_string(&path).unwrap();
    let modified = source.replace("this.WaitSeconds(2.0f)", "this.WaitSeconds(4.0f)");
    assert_ne!(modified, source);
    std::fs::write(&path, &modified).unwrap();
    routine_set(&dir, "12:00", "stand", "FP_XT_WAIT_OUTSIDE")
        .failure()
        .stderr(contains("manually changed"));
    assert_eq!(std::fs::read_to_string(&path).unwrap(), modified);
    std::fs::write(&path, &source).unwrap();
    let level = dir.path().join("Level.as");
    let modified = std::fs::read_to_string(&level)
        .unwrap()
        .replace("UDailyRoutine_MY_NPC_Start()", "MyHandwrittenRoutine()");
    std::fs::write(&level, &modified).unwrap();
    routine_set(&dir, "12:00", "stand", "FP_XT_WAIT_OUTSIDE")
        .failure()
        .stderr(contains("spawn line was manually changed"));
    assert_eq!(std::fs::read_to_string(&path).unwrap(), source);
    assert_eq!(std::fs::read_to_string(&level).unwrap(), modified);
}

#[test]
fn routine_spots_direct_activity_is_offline_and_filters_before_truncation() {
    let output = gore()
        .args([
            "npc",
            "routine",
            "spots",
            "--activity",
            "read",
            "--prefix",
            "FP_XT",
            "--max",
            "1",
            "--json",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let doc: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert!(doc["matched_count"].as_u64().unwrap() > 1);
    assert_eq!(doc["listed_count"], 1);
    assert_eq!(doc["truncated"], true);
    assert!(doc["spots"][0]["name"]
        .as_str()
        .unwrap()
        .starts_with("FP_XT"));
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
                "pristine_file": "pristine/XardasTower_AI.as",
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
        let pristine = if operation == "checkout" {
            "class UTest : UObject\n{\n    default Value = 1;\n}\n"
        } else {
            "class UWP : UWorldPointScript\n{\n    this.SpawnAIAgent(USpawnAIAgentDefinition_OC_STT_Diego::StaticClass());\n}\n"
        };
        let edited = if operation == "checkout" {
            pristine.replace("Value = 1", "Value = 2")
        } else {
            pristine.replace(
                "    this.SpawnAIAgent(USpawnAIAgentDefinition_OC_STT_Diego::StaticClass());\n",
                "",
            )
        };
        std::fs::create_dir(dir.join("pristine")).unwrap();
        std::fs::write(dir.join("pristine/XardasTower_AI.as"), pristine).unwrap();
        std::fs::write(&source, &edited).unwrap();

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
        if operation == "checkout" {
            std::fs::write(&source, edited.replace("class UTest", "class UOther")).unwrap();
            stage()
                .assert()
                .failure()
                .stderr(contains("workspace source failed the NPC guards"));
        }
    }
}
