//! Contract and workflow checks through the shipped command-line interface.
use assert_cmd::Command;
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
};

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../gore-save/assets/start_saves/resources_gothic.sav")
}
fn run(home: &Path, args: &[&str]) -> Value {
    let output = Command::cargo_bin("gore")
        .unwrap()
        .env("LOCALAPPDATA", home)
        .env("APPDATA", home)
        .env("XDG_DATA_HOME", home)
        .env("GORE_DISABLE_GAME_AUTODETECT", "1")
        .args(["save"])
        .args(args)
        .arg("--json")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let value: Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(value["ok"], true, "{value}");
    value["data"].clone()
}
#[test]
fn every_editor_feature_maps_to_existing_cli_and_mcp_leaves() {
    let parity = gore_save::api::editor_parity();
    let rows = parity["features"].as_array().unwrap();
    assert_eq!(rows.len(), 64);
    let group = gore_mcp::spec::group("gore_save").unwrap();
    for row in rows {
        for path in row["cliPaths"].as_array().unwrap() {
            let path = path.as_str().unwrap();
            assert!(
                group
                    .commands
                    .iter()
                    .any(|command| group.command_path(command.sub).join(" ") == path),
                "{}: missing {path}",
                row["id"]
            );
        }
    }
    let caps = run(
        tempfile::tempdir().unwrap().path(),
        &["core", "capabilities"],
    );
    assert_eq!(caps["editorParity"], parity);
    let source = include_str!("../../../gore-save/src/lib.rs");
    for command in gore_save::api::COMMANDS {
        assert!(
            source.contains(&format!("\"{command}\"")),
            "unbound core command {command}"
        );
    }
    for edit in gore_save::api::EDITS {
        assert!(
            source.contains(&format!("\"{edit}\"")),
            "unbound core edit {edit}"
        );
    }
}

#[test]
fn corrupt_ui_preferences_do_not_block_commands_or_a_full_reset() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path();
    let path = home.join("gore/gore-save/ui_settings.json");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    for bytes in [b"{broken".as_slice(), b"null".as_slice()] {
        fs::write(&path, bytes).unwrap();
        run(home, &["about"]);
        run(home, &["settings", "reset", "--scope", "ui", "--dry-run"]);
        assert_eq!(fs::read(&path).unwrap(), bytes);
        let reset = run(home, &["settings", "reset", "--scope", "ui"]);
        assert_eq!(reset["settings"], json!({}));
        assert_eq!(
            serde_json::from_slice::<Value>(&fs::read(&path).unwrap()).unwrap(),
            json!({})
        );
    }
}

#[test]
fn import_discovers_the_destination_without_using_the_external_source_parent() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path().join("home");
    let destination = home.join("G1R").join("Saved").join("SaveGames");
    let downloads = temp.path().join("downloads");
    fs::create_dir_all(&destination).unwrap();
    fs::create_dir(&downloads).unwrap();
    let source = downloads.join("G1R-010.sav");
    fs::copy(fixture(), &source).unwrap();
    let source_profile = downloads.join("PersistentDataList.sav");
    fs::write(&source_profile, b"external profile must not be touched").unwrap();
    let output = Command::cargo_bin("gore")
        .unwrap()
        .env("LOCALAPPDATA", &home)
        .env("APPDATA", &home)
        .env("XDG_DATA_HOME", &home)
        .args([
            "save",
            "import",
            source.to_str().unwrap(),
            "--profile",
            "0",
            "--json",
        ])
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let result: Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(result["ok"], false);
    assert!(
        result["error"]["message"]
            .as_str()
            .unwrap()
            .contains(destination.join("PersistentDataList.sav").to_str().unwrap()),
        "{result}"
    );
    assert_eq!(
        fs::read(&source_profile).unwrap(),
        b"external profile must not be touched"
    );
    assert_eq!(fs::read(&source).unwrap(), fs::read(fixture()).unwrap());
    assert_eq!(fs::read_dir(&destination).unwrap().count(), 0);
}

#[test]
fn slot_only_delete_reaches_the_same_guarded_transaction_as_a_positional_save() {
    let temp = tempfile::tempdir().unwrap();
    let save = temp.path().join("G1R-001.sav");
    fs::copy(fixture(), &save).unwrap();
    let before = fs::read(&save).unwrap();
    let mut errors = Vec::new();
    for positional in [false, true] {
        let mut command = Command::cargo_bin("gore").unwrap();
        command
            .env("LOCALAPPDATA", temp.path())
            .env("APPDATA", temp.path())
            .env("XDG_DATA_HOME", temp.path())
            .args(["save", "delete"]);
        if positional {
            command.arg(&save);
        }
        let output = command
            .arg("--root")
            .arg(temp.path())
            .args(["--slot", "G1R-001", "--profile", "0", "--json"])
            .assert()
            .failure()
            .get_output()
            .stdout
            .clone();
        let result: Value = serde_json::from_slice(&output).unwrap();
        errors.push(result["error"]["message"].as_str().unwrap().to_owned());
        assert_eq!(fs::read(&save).unwrap(), before);
    }
    assert_eq!(errors[0], errors[1]);
    assert!(
        errors[0].contains("PersistentDataList.sav was not found"),
        "{}",
        errors[0]
    );
}

#[test]
fn localization_find_matches_identifiers_regardless_of_case() {
    let temp = tempfile::tempdir().unwrap();
    let catalog = temp.path().join("gore").join("loc_catalog.json");
    fs::create_dir_all(catalog.parent().unwrap()).unwrap();
    fs::write(
        &catalog,
        serde_json::to_vec(&json!({
            "Document_Glossary_Bloodfly":{"en":"No identifier here"},
            "Other":{"en":"A different text"}
        }))
        .unwrap(),
    )
    .unwrap();
    for query in [
        "Document_Glossary_Bloodfly",
        "document_glossary_bloodfly",
        "BLOODFLY",
    ] {
        let data = run(temp.path(), &["localization", "find", "--query", query]);
        assert_eq!(data["entries"].as_array().unwrap().len(), 1);
        assert_eq!(data["entries"][0]["id"], "Document_Glossary_Bloodfly");
    }
}

#[test]
fn skill_story_event_selectors_and_attribute_offsets_select_the_requested_records() {
    let home = tempfile::tempdir().unwrap();
    let save = fixture();
    let save = save.to_str().unwrap();
    let skills = run(home.path(), &["skills", "list", save, "--all"]);
    let selected = skills["skills"].as_array().unwrap().last().unwrap();
    let base = selected["base"].as_str().unwrap();
    for option in ["--skill", "--id"] {
        let shown = run(
            home.path(),
            &[
                "skills", "show", save, option, base, "--offset", "100000", "--limit", "1",
            ],
        );
        assert_eq!(shown["base"], selected["base"]);
        assert_eq!(shown["current"], selected["current"]);
    }
    let story = run(
        home.path(),
        &["story", "list", save, "--all", "--include-unset"],
    );
    let selected = story["entries"].as_array().unwrap().last().unwrap();
    let shown = run(
        home.path(),
        &[
            "story",
            "show",
            save,
            "--id",
            selected["id"].as_str().unwrap(),
            "--offset",
            "100000",
            "--limit",
            "1",
        ],
    );
    assert_eq!(shown["id"], selected["id"]);
    assert_eq!(shown["stored"], selected["stored"]);
    assert_eq!(shown["rawValue"], selected["rawValue"]);

    let attributes = run(
        home.path(),
        &["attributes", "list", save, "--limit", "1000"],
    );
    let rows = attributes["results"].as_array().unwrap();
    assert!(rows.len() > 12);
    let page = run(
        home.path(),
        &["attributes", "list", save, "--offset", "10", "--limit", "2"],
    );
    assert_eq!(page["results"], json!(&rows[10..12]));
    assert_eq!(page["total"], attributes["total"]);

    let events = run(home.path(), &["events", "list", save, "--all"]);
    let selected = events["events"].as_array().unwrap().last().unwrap();
    let shown = run(
        home.path(),
        &[
            "events",
            "show",
            save,
            "--index",
            &selected["index"].to_string(),
            "--offset",
            "100000",
            "--limit",
            "1",
        ],
    );
    assert_eq!(shown["index"], selected["index"]);
    assert_eq!(shown["payload"], selected["payload"]);
    assert_eq!(shown["arrayPath"], events["arrayPath"]);
}

#[test]
fn inventory_container_and_slot_selectors_resolve_the_stack_before_editing() {
    let home = tempfile::tempdir().unwrap();
    let save = home.path().join("G1R-001.sav");
    fs::copy(fixture(), &save).unwrap();
    let save = save.to_str().unwrap();
    let characters = run(home.path(), &["characters", "list", save, "--all"]);
    let npc = characters["characters"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| {
            row["uniqueName"]
                .as_str()
                .is_some_and(|name| name.contains("Diego"))
        })
        .unwrap()["globalId"]
        .as_str()
        .unwrap();
    for actor in ["hero", npc] {
        let inventory = run(
            home.path(),
            &["inventory", "list", save, "--actor", actor, "--all"],
        );
        let selected = inventory["items"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["removable"] == true && row["slotId"].is_i64() && row["count"].is_i64())
            .unwrap();
        let slot = selected["slotId"].to_string();
        let container = selected["containerType"].as_str().unwrap();
        let next = (selected["count"].as_i64().unwrap() + 1).to_string();
        let before = fs::read(save).unwrap();
        run(
            home.path(),
            &[
                "inventory",
                "set-count",
                save,
                "--actor",
                actor,
                "--container",
                container,
                "--slot",
                &slot,
                "--count",
                &next,
                "--dry-run",
            ],
        );
        assert_eq!(fs::read(save).unwrap(), before);
        run(
            home.path(),
            &[
                "inventory",
                "set-count",
                save,
                "--actor",
                actor,
                "--container",
                container,
                "--slot",
                &slot,
                "--count",
                &next,
            ],
        );
        let stack = run(
            home.path(),
            &[
                "inventory",
                "list",
                save,
                "--actor",
                actor,
                "--container",
                container,
                "--slot",
                &slot,
                "--all",
            ],
        );
        assert_eq!(stack["items"][0]["path"], selected["path"]);
        assert_eq!(
            stack["items"][0]["count"].as_i64(),
            next.parse::<i64>().ok()
        );
        let before = fs::read(save).unwrap();
        run(
            home.path(),
            &[
                "inventory",
                "remove",
                save,
                "--actor",
                actor,
                "--container",
                container,
                "--slot",
                &slot,
                "--dry-run",
            ],
        );
        assert_eq!(fs::read(save).unwrap(), before);
        run(
            home.path(),
            &[
                "inventory",
                "remove",
                save,
                "--actor",
                actor,
                "--container",
                container,
                "--slot",
                &slot,
            ],
        );
        let remaining = run(
            home.path(),
            &[
                "inventory",
                "list",
                save,
                "--actor",
                actor,
                "--container",
                container,
                "--slot",
                &slot,
                "--all",
            ],
        );
        assert!(remaining["items"].as_array().unwrap().is_empty());
    }
}

#[test]
fn inventory_slot_only_selectors_keep_the_resolved_hero_container() {
    let home = tempfile::tempdir().unwrap();
    let save = home.path().join("G1R-001.sav");
    fs::copy(fixture(), &save).unwrap();
    // Seed the empty ArmorSlot in the bundled game-start save. Its id 0 also
    // exists in MainContainer, whose non-lootable marker is hidden from lists.
    let armor = "/Script/Angelscript.Ore_Armor_H";
    let prefix = json!([
        "m_GenericData",
        "{PlayersSavedData}",
        "m_SavedPlayers",
        "[0]",
        "m_Inventory",
        "m_Values",
        "Items",
        "[3]",
        "m_Slots",
        "[0]",
        "m_SlotData"
    ]);
    let edits = [
        ("m_ItemDefinition", json!(armor)),
        ("m_ItemCount", json!(1)),
    ]
    .into_iter()
    .map(|(member, value)| {
        let mut path = prefix.as_array().unwrap().clone();
        path.push(json!(member));
        json!({"path":"private.typed.setValue","value":{"path":path,"value":value}})
    })
    .collect::<Vec<_>>();
    gore_save::api::execute(&gore_save::api::Request {
        command: "write_save".into(),
        payload: json!({"path":save,"edits":edits}),
    })
    .unwrap();
    let save = save.to_str().unwrap();
    let original = run(home.path(), &["inventory", "list", save, "--all"]);
    let main = |inventory: &Value| {
        inventory["items"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|row| row["containerType"] == "MainContainer")
            .cloned()
            .collect::<Vec<_>>()
    };
    let original_main = main(&original);
    let selected = run(
        home.path(),
        &["inventory", "list", save, "--slot", "0", "--all"],
    );
    assert_eq!(selected["items"].as_array().unwrap().len(), 1);
    assert_eq!(selected["items"][0]["containerType"], "ArmorSlot");
    assert_eq!(selected["items"][0]["equipped"], true);
    let before = fs::read(save).unwrap();
    run(
        home.path(),
        &[
            "inventory",
            "set-count",
            save,
            "--slot",
            "0",
            "--count",
            "2",
            "--dry-run",
        ],
    );
    assert_eq!(fs::read(save).unwrap(), before);
    run(
        home.path(),
        &[
            "inventory",
            "set-count",
            save,
            "--slot",
            "0",
            "--count",
            "2",
        ],
    );
    let selected = run(
        home.path(),
        &["inventory", "list", save, "--slot", "0", "--all"],
    );
    assert_eq!(selected["items"][0]["count"], 2);
    run(
        home.path(),
        &[
            "inventory",
            "set-count",
            save,
            "--item",
            armor,
            "--slot",
            "0",
            "--count",
            "3",
        ],
    );
    let inventory = run(home.path(), &["inventory", "list", save, "--all"]);
    assert_eq!(main(&inventory), original_main);
    let selected = inventory["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["containerType"] == "ArmorSlot")
        .unwrap();
    assert_eq!(selected["count"], 3);
    run(home.path(), &["inventory", "remove", save, "--slot", "0"]);
    let inventory = run(home.path(), &["inventory", "list", save, "--all"]);
    assert_eq!(main(&inventory), original_main);
    assert!(
        inventory["items"]
            .as_array()
            .unwrap()
            .iter()
            .all(|row| row["containerType"] != "ArmorSlot")
    );
}

#[test]
fn domain_writes_resolve_aliases_and_find_states_before_applying_the_new_filter() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path().join("home");
    fs::create_dir(&home).unwrap();
    let save = fixture();
    let save = save.to_str().unwrap();
    for domain in ["quests", "tutorials", "glossary"] {
        let data = run(
            &home,
            &[
                domain,
                "list",
                save,
                "--all",
                "--limit",
                if domain == "glossary" { "7" } else { "100" },
            ],
        );
        if domain == "glossary" {
            let returned = data["categories"]
                .as_array()
                .unwrap()
                .iter()
                .map(|category| category["entries"].as_array().unwrap().len())
                .sum::<usize>();
            assert_eq!(Some(returned as u64), data["total"].as_u64());
            assert_eq!(data["count"].as_u64(), Some(returned as u64));
        }
        let row = if domain == "glossary" {
            &data["categories"][0]["entries"][0]
        } else {
            &data["quests"][0]
        };
        assert_ne!(row["currentState"], "EQuestState::Succeeded");
        let id = row["id"].as_str().unwrap();
        let selected = if domain == "glossary" {
            data["categories"]
                .as_array()
                .unwrap()
                .iter()
                .flat_map(|category| category["entries"].as_array().unwrap())
                .last()
                .unwrap()
        } else {
            data["quests"].as_array().unwrap().last().unwrap()
        };
        for (option, value) in [
            ("--id", selected["id"].as_str().unwrap()),
            ("--entry", selected["id"].as_str().unwrap()),
            (
                "--document",
                selected[if domain == "glossary" {
                    "documentClass"
                } else {
                    "questClass"
                }]
                .as_str()
                .unwrap(),
            ),
        ] {
            let shown = run(
                &home,
                &[
                    domain, "show", save, option, value, "--limit", "1", "--offset", "100000",
                ],
            );
            assert_eq!(shown["id"], selected["id"]);
            assert_eq!(shown["statePath"], selected["statePath"]);
        }
        run(
            &home,
            &[
                domain,
                "set-state",
                save,
                "--id",
                id,
                "--state",
                "EQuestState::Succeeded",
                "--dry-run",
            ],
        );
    }
    let draft = temp.path().join("aliases.json");
    let draft = draft.to_str().unwrap();
    run(
        &home,
        &["knowledge", "create-character", save, "--draft", draft],
    );
    assert_eq!(
        run(&home, &["draft", "show", draft])["edits"][0]["value"]["value"],
        "Hero"
    );
    let characters = run(&home, &["characters", "list", save, "--all"]);
    let npc = characters["characters"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| {
            r["uniqueName"]
                .as_str()
                .is_some_and(|n| n.contains("Diego"))
                && r["globalId"].is_string()
        })
        .unwrap();
    let alias = npc["uniqueName"].as_str().unwrap();
    let id = npc["globalId"].as_str().unwrap();
    for command in [vec!["npc", "show"], vec!["npc", "relationship", "show"]] {
        let mut args = command;
        args.extend([save, "--actor", alias]);
        let rows = run(&home, &args);
        assert_eq!(rows["npcs"][0]["id"], id);
    }
    run(
        &home,
        &[
            "knowledge",
            "create-character",
            save,
            "--actor",
            id,
            "--draft",
            draft,
        ],
    );
    assert_eq!(
        run(&home, &["draft", "show", draft])["edits"][1]["value"]["value"],
        alias
    );
}

#[test]
fn interface_and_game_text_languages_follow_shared_preferences_independently() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path();
    let save = fixture();
    let save = save.to_str().unwrap();
    let initial = run(home, &["quests", "list", save]);
    let class = initial["quests"][0]["questClass"].as_str().unwrap();
    let body = class
        .rsplit('.')
        .next()
        .unwrap()
        .strip_prefix("Quest_")
        .unwrap()
        .to_lowercase();
    let key = format!("quest-{body}-name");
    let cache = home.join("gore/loc_catalog.json");
    fs::create_dir_all(cache.parent().unwrap()).unwrap();
    fs::write(
        cache,
        json!({key:{"english":"English quest","german":"Deutsche Quest","japanese":"日本語","brazilian":"Missão brasileira"}})
            .to_string(),
    )
    .unwrap();
    let set = |key: &str, code: &str| {
        run(
            home,
            &[
                "settings", "set", "--scope", "ui", "--key", key, "--value", code,
            ],
        )
    };
    set("appLocale", "uk");
    assert_eq!(
        run(home, &["settings", "show", "--scope", "ui"])["settings"]["gameTextLocale"],
        "en"
    );
    set("gameTextLocale", "de");
    assert_eq!(
        run(home, &["quests", "list", save])["quests"][0]["label"],
        "Deutsche Quest"
    );
    assert_eq!(
        run(home, &["quests", "list", save, "--lang", "cs"])["quests"][0]["label"],
        "English quest"
    );
    assert_eq!(
        run(
            home,
            &["quests", "list", save, "--lang", "cs", "--game-lang", "ja"]
        )["quests"][0]["label"],
        "日本語"
    );
    let ui = run(
        home,
        &[
            "catalog", "search", "--kind", "ui-texts", "--id", "language",
        ],
    );
    let arb: Value = serde_json::from_slice(
        &fs::read(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../apps/save-editor/lib/l10n/app_uk.arb"),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(ui["entries"][0]["text"], arb["language"]);
    set("appLocale", "zh-Hant");
    assert_eq!(
        run(home, &["settings", "show", "--scope", "ui"])["settings"]["gameTextLocale"],
        "zh-Hans"
    );
    assert!(run(home, &["licenses"])["fonts"]["NotoSerifTC"].is_string());
    set("appLocale", "pt-BR");
    assert_eq!(
        run(home, &["quests", "list", save])["quests"][0]["label"],
        "Missão brasileira"
    );
    set("gameTextLocale", "pt-BR");
    assert_eq!(
        run(home, &["settings", "show", "--scope", "ui"])["settings"]["gameTextLocale"],
        "pt-BR"
    );
}

#[test]
fn overview_uses_the_selected_actors_inventory_and_skills() {
    let home = tempfile::tempdir().unwrap();
    let save = fixture();
    let save = save.to_str().unwrap();
    for actor in ["hero", "OC_STT_Diego-WP_EZ_START_DIEGO_SPAWN"] {
        let inventory = run(
            home.path(),
            &["inventory", "list", save, "--actor", actor, "--all"],
        );
        let skills = run(
            home.path(),
            &["skills", "list", save, "--actor", actor, "--all"],
        );
        let overview = run(home.path(), &["overview", save, "--actor", actor]);
        assert_eq!(overview["inventory"]["items"], inventory["items"]);
        assert!(overview["inventory"].get("public").is_none());
        assert_eq!(overview["skills"]["skills"], skills["skills"]);
        assert!(
            !overview["inventory"]["items"]
                .as_array()
                .unwrap()
                .is_empty()
        );
    }
}

#[test]
fn removing_a_draft_rename_releases_the_profile_guard_for_remaining_save_edits() {
    let temp = tempfile::tempdir().unwrap();
    let save = temp.path().join("G1R-001.sav");
    let profile = temp.path().join("PersistentDataList.sav");
    let draft = temp.path().join("draft.json");
    fs::copy(fixture(), &save).unwrap();
    fs::write(&profile, b"profile snapshot").unwrap();
    let save = save.to_str().unwrap();
    let draft = draft.to_str().unwrap();
    run(
        temp.path(),
        &["rename", save, "--name", "Pending rename", "--draft", draft],
    );
    run(
        temp.path(),
        &[
            "attributes",
            "set",
            save,
            "--attribute",
            "Health",
            "--base",
            "300",
            "--draft",
            draft,
        ],
    );
    assert_eq!(
        run(temp.path(), &["draft", "show", draft])["syncPersistentDataList"],
        true
    );
    let before = fs::read(draft).unwrap();
    let simulated = run(
        temp.path(),
        &["draft", "remove", draft, "--operation", "0", "--dry-run"],
    );
    assert!(simulated.get("syncPersistentDataList").is_none());
    assert_eq!(fs::read(draft).unwrap(), before);

    // Removing an unrelated edit keeps the rename's profile guard.
    let unrelated = run(
        temp.path(),
        &["draft", "remove", draft, "--operation", "1", "--dry-run"],
    );
    assert_eq!(unrelated["syncPersistentDataList"], true);
    let removed = run(temp.path(), &["draft", "remove", draft, "--operation", "0"]);
    assert!(removed.get("syncPersistentDataList").is_none());
    assert_eq!(removed["edits"].as_array().unwrap().len(), 1);
    fs::write(&profile, b"changed profile difficulty").unwrap();
    run(temp.path(), &["draft", "validate", draft]);
    let applied = run(temp.path(), &["draft", "apply", draft]);
    assert_eq!(applied["complete"], true);
    assert_eq!(applied["committed"], json!([0]));
    assert_eq!(fs::read(profile).unwrap(), b"changed profile difficulty");
    assert_eq!(
        run(temp.path(), &["draft", "show", draft])["edits"],
        json!([])
    );
    let health = run(
        temp.path(),
        &["attributes", "show", save, "--attribute", "Health"],
    );
    let base = health["results"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["path"].as_array().unwrap().last() == Some(&json!("BaseValue")))
        .unwrap();
    assert_eq!(
        base["value"].as_str().unwrap().parse::<f64>().unwrap(),
        300.0
    );
}

#[test]
fn native_reads_reports_and_drafts_work_without_an_editor_process() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path().join("home");
    fs::create_dir(&home).unwrap();
    let save = temp.path().join("G1R-001.sav");
    fs::copy(fixture(), &save).unwrap();
    let save = save.to_str().unwrap();
    let before = fs::read(save).unwrap();
    for command in [
        vec!["inspect"],
        vec!["characters", "list"],
        vec!["attributes", "list"],
        vec!["skills", "list"],
        vec!["inventory", "list"],
        vec!["position", "show"],
        vec!["time", "show"],
        vec!["quests", "list"],
        vec!["tutorials", "list"],
        vec!["glossary", "list"],
        vec!["story", "list"],
        vec!["knowledge", "list"],
        vec!["events", "list"],
        vec!["factions", "list"],
        vec!["locks", "list"],
        vec!["traders", "list"],
        vec!["codec", "status"],
        vec!["data", "search"],
    ] {
        let mut argv = command;
        argv.push(save);
        run(&home, &argv);
    }
    assert_eq!(before, fs::read(save).unwrap());
    assert!(!temp.path().join("goresave_backups").exists());
    run(
        &home,
        &[
            "settings",
            "set",
            "--scope",
            "ui",
            "--key",
            "uiFontFamily",
            "--value",
            "system",
        ],
    );
    let out = temp.path().join("report.html");
    run(&home, &["report", save, "--out", out.to_str().unwrap()]);
    let html = fs::read_to_string(&out).unwrap();
    assert!(html.contains("<table>"));
    assert!(html.contains("id=\"statistics\""));
    let draft = temp.path().join("draft.json");
    let draft = draft.to_str().unwrap();
    run(&home, &["draft", "create", draft, "--target", save]);
    run(
        &home,
        &["rename", save, "--name", "Draft name", "--draft", draft],
    );
    run(
        &home,
        &["rename", save, "--name", "Final name", "--draft", draft],
    );
    let staged = run(&home, &["draft", "show", draft]);
    assert_eq!(staged["edits"].as_array().unwrap().len(), 1);
    run(&home, &["draft", "validate", draft]);
    assert_eq!(before, fs::read(save).unwrap());
    let result = run(&home, &["draft", "apply", draft]);
    assert_eq!(result["committed"], json!([0]));
    assert_eq!(run(&home, &["draft", "show", draft])["edits"], json!([]));
    assert_ne!(before, fs::read(save).unwrap());
    let backups = run(&home, &["backups", "list", save]);
    assert_eq!(backups["backups"].as_array().unwrap().len(), 1);
    let backup = backups["backups"][0]["path"].as_str().unwrap();
    let current = fs::read(save).unwrap();
    run(
        &home,
        &["backups", "restore", save, "--backup", backup, "--dry-run"],
    );
    assert_eq!(current, fs::read(save).unwrap());
    run(&home, &["backups", "restore", save, "--backup", backup]);
    assert_eq!(before, fs::read(save).unwrap());
}
