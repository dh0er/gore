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
fn execute_core(command: &str, payload: Value) -> Value {
    gore_save::api::execute(&gore_save::api::Request {
        command: command.into(),
        payload,
    })
    .unwrap()
}
fn run(home: &Path, args: &[&str]) -> Value {
    run_from(home, None, args)
}

fn run_from(home: &Path, directory: Option<&Path>, args: &[&str]) -> Value {
    let mut command = Command::cargo_bin("gore").unwrap();
    command
        .env("LOCALAPPDATA", home)
        .env("APPDATA", home)
        .env("XDG_DATA_HOME", home)
        .env("GORE_DISABLE_GAME_AUTODETECT", "1")
        .args(["save"])
        .args(args)
        .arg("--json");
    if let Some(directory) = directory {
        command.current_dir(directory);
    }
    let output = command.assert().success().get_output().stdout.clone();
    let value: Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(value["ok"], true, "{value}");
    value["data"].clone()
}

fn run_failure(home: &Path, args: &[&str]) -> Value {
    let output = Command::cargo_bin("gore")
        .unwrap()
        .env("LOCALAPPDATA", home)
        .env("APPDATA", home)
        .env("XDG_DATA_HOME", home)
        .env("GORE_DISABLE_GAME_AUTODETECT", "1")
        .arg("save")
        .args(args)
        .arg("--json")
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let value: Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(value["ok"], false, "{value}");
    value["error"].clone()
}

// A strictly typed profile registry, including the difficulty and slot arrays
// used by the native reset, detach and delete operations.
fn profile_fixture(preset: &str) -> Vec<u8> {
    fn string(value: &str) -> Vec<u8> {
        let mut bytes = ((value.len() + 1) as i32).to_le_bytes().to_vec();
        bytes.extend_from_slice(value.as_bytes());
        bytes.push(0);
        bytes
    }
    fn property(name: &str, kind: &str, descriptor: &[u8], value: &[u8]) -> Vec<u8> {
        let mut bytes = string(name);
        bytes.extend(string(kind));
        bytes.extend(descriptor);
        bytes.extend(0u32.to_le_bytes());
        bytes.extend((value.len() as u32).to_le_bytes());
        bytes.push(0);
        bytes.extend(value);
        bytes
    }
    fn structure(name: &str) -> Vec<u8> {
        let mut bytes = 1u32.to_le_bytes().to_vec();
        bytes.extend(string(name));
        bytes.extend(1u32.to_le_bytes());
        bytes.extend(string("/Script/G1R"));
        bytes
    }
    fn slots(name: &str, populated: bool) -> Vec<u8> {
        let mut descriptor = 1u32.to_le_bytes().to_vec();
        descriptor.extend(string("StrProperty"));
        let mut value = u32::from(populated).to_le_bytes().to_vec();
        if populated {
            value.extend(string("G1R-001"));
        }
        property(name, "ArrayProperty", &descriptor, &value)
    }
    let mut public = property("m_SlotName", "StrProperty", &[], &string("G1R-001"));
    public.extend(property(
        "m_PlayerSaveName",
        "StrProperty",
        &[],
        &string("Profile test"),
    ));
    public.extend(property(
        "m_ProfileId",
        "IntProperty",
        &[],
        &0i32.to_le_bytes(),
    ));
    public.extend(string("None"));
    let mut map = 0u32.to_le_bytes().to_vec();
    map.extend(1u32.to_le_bytes());
    map.extend(string("G1R-001"));
    map.extend(public);
    let mut map_descriptor = 2u32.to_le_bytes().to_vec();
    map_descriptor.extend(string("StrProperty"));
    map_descriptor.extend(0u32.to_le_bytes());
    map_descriptor.extend(string("StructProperty"));
    map_descriptor.extend(structure("SaveGamePublicData"));
    let mut profile = property("m_ProfileName", "StrProperty", &[], &string("Profile0"));
    profile.extend(property(
        "m_ProfileId",
        "IntProperty",
        &[],
        &0i32.to_le_bytes(),
    ));
    for (name, class) in [
        ("m_difficultyPreset", "DifficultyPreset"),
        ("m_customCombatSettings", "CombatDifficultySettings"),
        ("m_customResourcesSettings", "ResourcesDifficultySettings"),
        (
            "m_customProgressionSettings",
            "ProgressionDifficultySettings",
        ),
    ] {
        profile.extend(property(
            name,
            "ObjectProperty",
            &[],
            &string(&format!("/Script/Angelscript.{class}_{preset}")),
        ));
    }
    for (name, populated) in [
        ("m_SavedSlotsNames", true),
        ("m_QuickSaveName", false),
        ("m_AutoSaveName", false),
    ] {
        profile.extend(slots(name, populated));
    }
    profile.extend(string("None"));
    let mut profiles = 1u32.to_le_bytes().to_vec();
    profiles.extend(profile);
    let mut array_descriptor = 1u32.to_le_bytes().to_vec();
    array_descriptor.extend(string("StructProperty"));
    array_descriptor.extend(structure("ProfileEntry"));
    let mut bytes = b"GVAS".to_vec();
    bytes.extend([0u8; 24]);
    bytes.extend(string("/Script/G1R.PersistentDataList"));
    bytes.push(0);
    bytes.extend(slots("m_SavedGamesNames", true));
    bytes.extend(property(
        "m_SavedGamesPublicData",
        "MapProperty",
        &map_descriptor,
        &map,
    ));
    bytes.extend(property(
        "m_Profiles",
        "ArrayProperty",
        &array_descriptor,
        &profiles,
    ));
    bytes.extend(string("None"));
    bytes.extend(0u32.to_le_bytes());
    bytes
}

#[test]
fn knowledge_filters_select_all_core_pages_before_pagination() {
    use gore_save::codec_backend::{CodecBackend, KrakenBackend};

    fn string(value: &str) -> Vec<u8> {
        let mut bytes = ((value.len() + 1) as i32).to_le_bytes().to_vec();
        bytes.extend_from_slice(value.as_bytes());
        bytes.push(0);
        bytes
    }
    fn property(name: &str, kind: &str, descriptor: &[u8], value: &[u8]) -> Vec<u8> {
        let mut bytes = string(name);
        bytes.extend(string(kind));
        bytes.extend(descriptor);
        bytes.extend(0u32.to_le_bytes());
        bytes.extend((value.len() as u32).to_le_bytes());
        bytes.push(0);
        bytes.extend(value);
        bytes
    }
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path();
    let save = home.join("G1R-001.sav");
    let mut entries = (0..1001)
        .map(|i| format!("Voiceline_Page_{i:04}"))
        .collect::<Vec<_>>();
    entries.extend((0..120).map(|i| format!("ChoicePage{i:04}")));
    entries.push("Choice62749".into());
    let mut set = 0u32.to_le_bytes().to_vec();
    set.extend((entries.len() as u32).to_le_bytes());
    for entry in &entries {
        set.extend(string(entry));
    }
    let mut set_descriptor = 1u32.to_le_bytes().to_vec();
    set_descriptor.extend(string("NameProperty"));
    let mut map = 0u32.to_le_bytes().to_vec();
    map.extend(1u32.to_le_bytes());
    map.extend(string("Hero"));
    map.extend(property("Knowledge", "SetProperty", &set_descriptor, &set));
    map.extend(string("None"));
    let mut map_descriptor = 2u32.to_le_bytes().to_vec();
    map_descriptor.extend(string("NameProperty"));
    map_descriptor.extend(0u32.to_le_bytes());
    map_descriptor.extend(string("StructProperty"));
    map_descriptor.extend(1u32.to_le_bytes());
    map_descriptor.extend(string("KnowledgeSet"));
    map_descriptor.extend(1u32.to_le_bytes());
    map_descriptor.extend(string("/Script/G1R"));
    let mut private = string("/Script/Angelscript.GothicFinalDataGame");
    private.push(0);
    private.extend(property(
        "CharacterKnowledgeByUniqueName",
        "MapProperty",
        &map_descriptor,
        &map,
    ));
    private.extend(string("None"));
    private.extend(0u32.to_le_bytes());
    gore_save::properties::parse_private_root(&private).unwrap();
    let compressed = KrakenBackend.compress(&private, 4).unwrap();
    let mut stream = (private.len() as u64).to_le_bytes().to_vec();
    stream.extend(string("Oodle"));
    stream.extend(0x9E2A83C1u32.to_le_bytes());
    stream.extend(0x22222222u32.to_le_bytes());
    stream.extend((private.len() as u64).to_le_bytes());
    stream.push(2);
    for _ in 0..2 {
        stream.extend((compressed.len() as u64).to_le_bytes());
        stream.extend((private.len() as u64).to_le_bytes());
    }
    stream.extend(compressed);
    let reference = fs::read(fixture()).unwrap();
    let public_size = u32::from_le_bytes(reference[9..13].try_into().unwrap()) as usize;
    let mut bytes = reference[..13 + public_size].to_vec();
    let body_size = (bytes.len() + stream.len()) as u32;
    bytes[5..9].copy_from_slice(&body_size.to_le_bytes());
    bytes.extend(stream);
    bytes.extend(0u32.to_le_bytes());
    fs::write(&save, &bytes).unwrap();
    let catalog = home.join("gore/loc_catalog.json");
    fs::create_dir_all(catalog.parent().unwrap()).unwrap();
    fs::write(
        &catalog,
        json!({"text_andre_20220118_145939":{"english":"Needle knowledge on a later page"}})
            .to_string(),
    )
    .unwrap();
    let catalog_bytes = fs::read(&catalog).unwrap();
    let save_arg = save.to_str().unwrap();
    let all = run(
        home,
        &[
            "knowledge",
            "list",
            save_arg,
            "--character",
            "Hero",
            "--all",
        ],
    );
    assert_eq!(all["total"], entries.len());
    let expected = all["entries"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|entry| entry["category"] == "choice")
        .cloned()
        .collect::<Vec<_>>();
    assert_eq!(expected.len(), 121);
    let common = [
        "knowledge",
        "list",
        save_arg,
        "--character",
        "Hero",
        "--category",
        "CHOICE",
    ];
    let default = run(home, &common);
    assert_eq!(default["entries"], json!(&expected[..100]));
    assert_eq!(default["total"], 121);
    assert_eq!(default["count"], 100);
    assert_eq!(default["limit"], 100);
    let mut args = common.to_vec();
    args.extend(["--offset", "1", "--limit", "2"]);
    let page = run(home, &args);
    assert_eq!(page["entries"], json!(&expected[1..3]));
    assert_eq!(page["total"], 121);
    assert_eq!(page["count"], 2);
    assert_eq!(page["offset"], 1);
    assert_eq!(page["limit"], 2);
    args.push("--all");
    let rest = run(home, &args);
    assert_eq!(rest["entries"], json!(&expected[1..]));
    assert_eq!(rest["total"], 121);
    assert_eq!(rest["count"], 120);
    assert_eq!(rest["limit"], 120);
    let mut exact = common.to_vec();
    exact.extend(["--id", "Choice62749"]);
    let last = run(home, &exact);
    assert_eq!(last["entries"], json!([expected.last().unwrap()]));
    assert_eq!(last["total"], 1);
    assert_eq!(last["count"], 1);
    let localized = run(
        home,
        &[
            "knowledge",
            "list",
            save_arg,
            "--character",
            "Hero",
            "--query",
            "needle knowledge",
        ],
    );
    assert_eq!(localized["entries"], last["entries"]);
    assert_eq!(localized["total"], 1);
    assert_eq!(localized["count"], 1);
    let payload_file = home.join("knowledge-query.json");
    fs::write(
        &payload_file,
        json!({"character":"Hero","query":"needle knowledge"}).to_string(),
    )
    .unwrap();
    let from_payload = run(
        home,
        &[
            "knowledge",
            "list",
            save_arg,
            "--payload-file",
            payload_file.to_str().unwrap(),
        ],
    );
    assert_eq!(from_payload["entries"], localized["entries"]);
    assert_eq!(from_payload["total"], 1);
    assert_eq!(fs::read(&catalog).unwrap(), catalog_bytes);
    assert_eq!(fs::read(&save).unwrap(), bytes);
    assert!(!home.join("goresave_backups").exists());
}

#[test]
fn inventory_reset_uses_the_selected_saves_profile_difficulty() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path();
    let configured = home.join("configured");
    let external = home.join("external");
    for (directory, preset) in [(&configured, "Easy"), (&external, "Hard")] {
        fs::create_dir(directory).unwrap();
        fs::copy(fixture(), directory.join("G1R-001.sav")).unwrap();
        fs::write(
            directory.join("PersistentDataList.sav"),
            profile_fixture(preset),
        )
        .unwrap();
    }
    run(
        home,
        &[
            "settings",
            "set",
            "--key",
            "saveDir",
            "--value",
            configured.to_str().unwrap(),
        ],
    );
    let save = external.join("G1R-001.sav");
    let save_arg = save.to_str().unwrap();
    let original_hash = gore_save::api::file_sha1(&save).unwrap();
    let foreign_save_hash = gore_save::api::file_sha1(&configured.join("G1R-001.sav")).unwrap();
    let foreign_profile_hash =
        gore_save::api::file_sha1(&configured.join("PersistentDataList.sav")).unwrap();
    let profile_hash = gore_save::api::file_sha1(&external.join("PersistentDataList.sav")).unwrap();
    assert_eq!(
        run(home, &["difficulty", "show", save_arg, "--profile", "0"])["difficultyPreset"],
        "/Script/Angelscript.DifficultyPreset_Hard"
    );
    let draft = home.join("reset.json");
    run(
        home,
        &[
            "inventory",
            "reset",
            save_arg,
            "--draft",
            draft.to_str().unwrap(),
        ],
    );
    let staged = run(home, &["draft", "show", draft.to_str().unwrap()]);
    assert_eq!(staged["edits"][0]["value"]["resourcesLevel"], "Hard");
    let override_draft = home.join("override.json");
    run(
        home,
        &[
            "inventory",
            "reset",
            save_arg,
            "--resources-level",
            "Novice",
            "--draft",
            override_draft.to_str().unwrap(),
        ],
    );
    assert_eq!(
        run(home, &["draft", "show", override_draft.to_str().unwrap()])["edits"][0]["value"]["resourcesLevel"],
        "Novice"
    );
    assert_eq!(gore_save::api::file_sha1(&save).unwrap(), original_hash);
    run(home, &["inventory", "reset", save_arg, "--dry-run"]);
    assert_eq!(gore_save::api::file_sha1(&save).unwrap(), original_hash);
    run(home, &["draft", "apply", draft.to_str().unwrap()]);
    let hard = fixture().with_file_name("resources_hard.sav");
    assert_eq!(
        run(home, &["inventory", "list", save_arg, "--all"])["items"],
        run(
            home,
            &["inventory", "list", hard.to_str().unwrap(), "--all"]
        )["items"]
    );
    assert_eq!(
        gore_save::api::file_sha1(&configured.join("G1R-001.sav")).unwrap(),
        foreign_save_hash
    );
    assert_eq!(
        gore_save::api::file_sha1(&configured.join("PersistentDataList.sav")).unwrap(),
        foreign_profile_hash
    );
    assert_eq!(
        gore_save::api::file_sha1(&external.join("PersistentDataList.sav")).unwrap(),
        profile_hash
    );
    assert!(!configured.join("goresave_backups").exists());
}

#[test]
fn detach_and_delete_use_the_supplied_saves_registry_including_missing_saves() {
    for (operation, missing) in [("detach", false), ("detach", true), ("delete", false)] {
        let temp = tempfile::tempdir().unwrap();
        let home = temp.path();
        let configured = home.join("configured");
        let external = home.join("external");
        for (directory, preset) in [(&configured, "Easy"), (&external, "Hard")] {
            fs::create_dir(directory).unwrap();
            fs::copy(fixture(), directory.join("G1R-001.sav")).unwrap();
            fs::write(
                directory.join("PersistentDataList.sav"),
                profile_fixture(preset),
            )
            .unwrap();
        }
        run(
            home,
            &[
                "settings",
                "set",
                "--key",
                "saveDir",
                "--value",
                configured.to_str().unwrap(),
            ],
        );
        let save = external.join("G1R-001.sav");
        let own_save_hash = gore_save::api::file_sha1(&save).unwrap();
        if missing {
            fs::remove_file(&save).unwrap();
        }
        let profile = external.join("PersistentDataList.sav");
        let profile_hash = gore_save::api::file_sha1(&profile).unwrap();
        let foreign_save_hash = gore_save::api::file_sha1(&configured.join("G1R-001.sav")).unwrap();
        let foreign_profile_hash =
            gore_save::api::file_sha1(&configured.join("PersistentDataList.sav")).unwrap();
        let mut args = if operation == "detach" {
            vec!["profile", "detach"]
        } else {
            vec!["delete"]
        };
        args.extend([save.to_str().unwrap(), "--profile", "0"]);
        let mut conflicting = args.clone();
        conflicting.extend(["--root", configured.to_str().unwrap()]);
        let error = run_failure(home, &conflicting);
        assert!(
            error["message"]
                .as_str()
                .unwrap()
                .contains("save's own directory")
        );
        let mut preview = args.clone();
        preview.push("--dry-run");
        assert_eq!(run(home, &preview)["validated"], true);
        assert_eq!(gore_save::api::file_sha1(&profile).unwrap(), profile_hash);
        assert_eq!(save.exists(), !missing);
        if !missing {
            assert_eq!(gore_save::api::file_sha1(&save).unwrap(), own_save_hash);
        }
        assert!(!configured.join("goresave_backups").exists());
        assert!(!external.join("goresave_backups").exists());
        let slot_only = run(
            home,
            &[
                "profile",
                "detach",
                "--slot",
                "G1R-001",
                "--profile",
                "0",
                "--dry-run",
            ],
        );
        assert_eq!(
            Path::new(slot_only["request"]["persistentPath"].as_str().unwrap())
                .canonicalize()
                .unwrap(),
            configured
                .join("PersistentDataList.sav")
                .canonicalize()
                .unwrap()
        );
        let save_index = if operation == "detach" { 2 } else { 1 };
        args[save_index] = "G1R-001.sav";
        run_from(home, Some(&external), &args);
        assert_eq!(save.exists(), operation == "detach" && !missing);
        if operation == "detach" && !missing {
            assert_eq!(gore_save::api::file_sha1(&save).unwrap(), own_save_hash);
        }
        assert_ne!(gore_save::api::file_sha1(&profile).unwrap(), profile_hash);
        let listing = run(
            home,
            &["profiles", "list", "--root", external.to_str().unwrap()],
        );
        assert_eq!(listing["profiles"][0]["savedSlots"], json!([]));
        assert_eq!(
            gore_save::api::file_sha1(&configured.join("G1R-001.sav")).unwrap(),
            foreign_save_hash
        );
        assert_eq!(
            gore_save::api::file_sha1(&configured.join("PersistentDataList.sav")).unwrap(),
            foreign_profile_hash
        );
        assert!(!configured.join("goresave_backups").exists());
    }
}

#[test]
fn adjacent_legacy_backup_dry_runs_validate_without_changing_live_files() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path();
    let save = home.join("G1R-001.sav");
    let backup = home.join("G1R-001.sav.bak.1000");
    fs::copy(fixture(), &save).unwrap();
    fs::copy(fixture(), &backup).unwrap();
    let save_arg = save.to_str().unwrap();
    let backup_arg = backup.to_str().unwrap();
    run(
        home,
        &[
            "backups", "rename", save_arg, "--backup", backup_arg, "--name", "Keep",
        ],
    );
    let labels = home.join("goresave_backups/backup_names.json");
    let label_bytes = fs::read(&labels).unwrap();
    let save_hash = gore_save::api::file_sha1(&save).unwrap();
    let backup_hash = gore_save::api::file_sha1(&backup).unwrap();
    for operation in ["restore", "delete", "rename"] {
        let mut args = vec![
            "backups",
            operation,
            save_arg,
            "--backup",
            backup_arg,
            "--dry-run",
        ];
        if operation == "rename" {
            args.extend(["--name", "Changed"]);
        }
        let result = run(home, &args);
        assert_eq!(result["validated"], true, "{result}");
        assert_eq!(fs::read(&labels).unwrap(), label_bytes);
        assert_eq!(gore_save::api::file_sha1(&save).unwrap(), save_hash);
        assert_eq!(gore_save::api::file_sha1(&backup).unwrap(), backup_hash);
    }
    assert_eq!(
        run(home, &["backups", "list", save_arg])["backups"][0]["name"],
        "Keep"
    );
}

#[test]
fn mixed_case_pending_placement_note_survives_a_later_move_without_stay() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path();
    let save = home.join("G1R-001.sav");
    fs::copy(fixture(), &save).unwrap();
    let save_arg = save.to_str().unwrap();
    let actor = "OC_STT_Diego-WP_EZ_START_DIEGO_SPAWN";
    let original = run(home, &["position", "show", save_arg, "--actor", actor]);
    let x = original["pose"]["location"]["x"].as_f64().unwrap();
    let draft = home.join("mixed-case.json");
    let draft_arg = draft.to_str().unwrap();
    run(home, &["draft", "create", draft_arg, "--target", save_arg]);
    run(
        home,
        &[
            "position",
            "set",
            save_arg,
            "--actor",
            actor,
            "--x",
            &(x + 100.0).to_string(),
            "--stay",
            "--draft",
            draft_arg,
        ],
    );
    let mut staged: Value = serde_json::from_slice(&fs::read(&draft).unwrap()).unwrap();
    let id = staged["placementNotes"][0]["npc"]
        .as_str()
        .unwrap()
        .to_owned();
    let first_note = staged["placementNotes"][0]["note"].clone();
    staged["placementNotes"][0]["npc"] = json!(id.to_uppercase());
    fs::write(&draft, serde_json::to_vec(&staged).unwrap()).unwrap();
    run(
        home,
        &[
            "position",
            "set",
            save_arg,
            "--actor",
            &id.to_lowercase(),
            "--x",
            &(x + 200.0).to_string(),
            "--draft",
            draft_arg,
        ],
    );
    let staged = run(home, &["draft", "show", draft_arg]);
    assert_eq!(staged["placementNotes"].as_array().unwrap().len(), 1);
    assert_eq!(staged["placementNotes"][0]["npc"], id);
    assert_eq!(
        staged["placementNotes"][0]["note"]["original_location"],
        first_note["original_location"]
    );
    assert_eq!(
        staged["placementNotes"][0]["note"]["original_routine_class"],
        first_note["original_routine_class"]
    );
    assert_eq!(
        staged["placementNotes"][0]["note"]["written_location"][0],
        x + 200.0
    );
    run(home, &["draft", "apply", draft_arg]);
    let pinned = run(
        home,
        &["position", "pin-status", save_arg, "--actor", actor],
    );
    assert_eq!(pinned["routineClass"], pinned["inertRoutineClass"]);
    assert_eq!(pinned["undo"]["restorable"], true);
    assert_eq!(
        pinned["undo"]["originalLocation"],
        original["pose"]["location"]
    );
}

#[test]
fn repinning_after_staged_undo_keeps_the_latest_routine_and_undo_note() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path();
    let save = home.join("G1R-001.sav");
    fs::copy(fixture(), &save).unwrap();
    let save_arg = save.to_str().unwrap();
    let actor = "OC_STT_Diego-WP_EZ_START_DIEGO_SPAWN";
    let original = run(home, &["position", "show", save_arg, "--actor", actor]);
    let first_x = (original["pose"]["location"]["x"].as_f64().unwrap() + 100.0).to_string();
    let second_x = (original["pose"]["location"]["x"].as_f64().unwrap() + 200.0).to_string();
    run(
        home,
        &[
            "position", "set", save_arg, "--actor", actor, "--x", &first_x, "--stay",
        ],
    );
    let draft = home.join("placement.json");
    let draft_arg = draft.to_str().unwrap();
    run(home, &["draft", "create", draft_arg, "--target", save_arg]);
    run(
        home,
        &[
            "position", "undo", save_arg, "--actor", actor, "--draft", draft_arg,
        ],
    );
    assert_eq!(
        run(home, &["draft", "show", draft_arg])["clearPlacementNotes"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    run(
        home,
        &[
            "position", "set", save_arg, "--actor", actor, "--x", &second_x, "--stay", "--draft",
            draft_arg,
        ],
    );
    let staged = run(home, &["draft", "show", draft_arg]);
    assert!(staged.get("clearPlacementNotes").is_none());
    assert_eq!(staged["placementNotes"].as_array().unwrap().len(), 1);
    run(home, &["draft", "apply", draft_arg]);
    let pinned = run(
        home,
        &["position", "pin-status", save_arg, "--actor", actor],
    );
    assert_eq!(pinned["routineClass"], pinned["inertRoutineClass"]);
    assert_eq!(pinned["undo"]["restorable"], true);
    assert_eq!(pinned["undo"]["routineRestorable"], true);
    assert_eq!(
        pinned["undo"]["originalLocation"],
        original["pose"]["location"]
    );
    run(home, &["position", "undo", save_arg, "--actor", actor]);
    let restored = run(home, &["position", "show", save_arg, "--actor", actor]);
    assert_eq!(restored["pose"], original["pose"]);
    assert_eq!(restored["routineClass"], original["routineClass"]);
    assert!(restored["undo"].is_null());
}

#[test]
fn backup_companions_are_listed_only_when_requested_without_modifying_files() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path();
    let save = home.join("G1R-001.sav");
    fs::copy(fixture(), &save).unwrap();
    let original_sha1 = gore_save::api::file_sha1(&save).unwrap();
    let directory = home.join("goresave_backups");
    fs::create_dir(&directory).unwrap();
    let backup = directory.join("G1R-001.sav.bak.1000");
    fs::copy(&save, &backup).unwrap();
    let companion = directory.join("PersistentDataList.sav.bak.1000");
    fs::write(
        &companion,
        b"damaged profile backups are still listed for inspection",
    )
    .unwrap();
    let save_arg = save.to_str().unwrap();
    let plain = run(home, &["backups", "list", save_arg]);
    assert!(plain.get("companionBackups").is_none());
    assert_eq!(plain["backups"].as_array().unwrap().len(), 1);
    let detailed = run(home, &["backups", "list", save_arg, "--include-companions"]);
    assert_eq!(detailed["backups"], plain["backups"]);
    let companions = detailed["companionBackups"].as_array().unwrap();
    assert_eq!(companions.len(), 1);
    assert_eq!(companions[0]["path"], json!(companion));
    assert_eq!(companions[0]["scope"], "persistent_data_list");
    assert_eq!(gore_save::api::file_sha1(&save).unwrap(), original_sha1);
    assert_eq!(gore_save::api::file_sha1(&backup).unwrap(), original_sha1);
    assert_eq!(
        fs::read(&companion).unwrap(),
        b"damaged profile backups are still listed for inspection"
    );
    assert_eq!(fs::read_dir(&directory).unwrap().count(), 2);
}

#[test]
fn hero_transform_raw_collisions_reject_core_writes_and_keep_staged_drafts() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path();
    let save = home.join("G1R-001.sav");
    fs::copy(fixture(), &save).unwrap();
    let save_arg = save.to_str().unwrap();
    let inspection = execute_core("inspect_save", json!({"path":save,"includePrivate":true}));
    let original = &inspection["private"]["player"]["transform"];
    let properties = execute_core(
        "search_typed_properties",
        json!({"path":save,"query":"m_SavedPlayers","includeNodes":true,"limit":1000}),
    );
    let draft = home.join("position.json");
    let draft_arg = draft.to_str().unwrap();
    let x = (original["location"]["x"].as_f64().unwrap() + 10.0).to_string();
    run(
        home,
        &["position", "set", save_arg, "--x", &x, "--draft", draft_arg],
    );
    let staged = run(home, &["draft", "show", draft_arg]);
    let transform = staged["edits"][0].clone();
    let draft_bytes = fs::read(&draft).unwrap();
    let save_hash = gore_save::api::file_sha1(&save).unwrap();
    for (leaf, value) in [
        (
            "m_Location",
            json!({"x":original["location"]["x"].as_f64().unwrap()+20.0,"y":original["location"]["y"],"z":original["location"]["z"]}),
        ),
        (
            "m_Rotation",
            json!({"pitch":original["rotation"]["pitch"],"yaw":original["rotation"]["yaw"].as_f64().unwrap()+20.0,"roll":original["rotation"]["roll"]}),
        ),
    ] {
        let rows: Vec<_> = properties["results"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|row| {
                row["path"]
                    .as_array()
                    .is_some_and(|path| path.last().and_then(Value::as_str) == Some(leaf))
            })
            .collect();
        assert_eq!(rows.len(), 1);
        let path = rows[0]["path"].clone();
        let raw = json!({"path":"private.typed.setValue","value":{"path":path,"value":value}});
        // Each edit is valid by itself; only their overlap causes the refusal.
        execute_core(
            "apply_edits",
            json!({"path":save,"edits":[raw.clone()],"dryRun":true}),
        );
        for edits in [
            vec![transform.clone(), raw.clone()],
            vec![raw.clone(), transform.clone()],
        ] {
            for command in ["write_save", "apply_edits"] {
                let error = gore_save::api::execute(&gore_save::api::Request {
                    command: command.into(),
                    payload: json!({"path":save,"edits":edits,"backup":true}),
                })
                .unwrap_err();
                assert!(
                    matches!(
                        error,
                        gore_save::CoreError::UnsupportedEdit(_)
                            | gore_save::CoreError::PlanConflict {
                                kind: "property",
                                ..
                            }
                    ),
                    "{error}"
                );
                assert_eq!(gore_save::api::file_sha1(&save).unwrap(), save_hash);
                assert!(!home.join("goresave_backups").exists());
            }
        }
        let path_file = home.join("path.json");
        fs::write(&path_file, serde_json::to_vec(&path).unwrap()).unwrap();
        let value_arg = value.to_string();
        run(
            home,
            &[
                "data",
                "set",
                save_arg,
                "--path-file",
                path_file.to_str().unwrap(),
                "--value-json",
                &value_arg,
                "--draft",
                draft_arg,
            ],
        );
        let conflicting_draft = fs::read(&draft).unwrap();
        for operation in ["validate", "apply"] {
            let error = run_failure(home, &["draft", operation, draft_arg]);
            assert!(
                error["message"]
                    .as_str()
                    .unwrap()
                    .contains("pending edit conflict"),
                "{error}"
            );
            assert_eq!(fs::read(&draft).unwrap(), conflicting_draft);
        }
        assert_eq!(gore_save::api::file_sha1(&save).unwrap(), save_hash);
        assert!(!home.join("goresave_backups").exists());
        fs::write(&draft, &draft_bytes).unwrap();
    }
}

#[test]
fn inventory_reset_and_skill_unlearning_refuse_raw_changes_they_would_discard() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path();
    let save = home.join("G1R-001.sav");
    fs::copy(fixture(), &save).unwrap();
    let actor = "OC_STT_Diego-WP_EZ_START_DIEGO_SPAWN";
    let inventory = execute_core(
        "search_typed_properties",
        json!({"path":save,"query":format!("InventoryByGlobalId {actor}"),"includeNodes":true,"limit":1000}),
    );
    let count = inventory["results"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| {
            row["path"]
                .as_array()
                .unwrap()
                .last()
                .and_then(Value::as_str)
                == Some("m_ItemCount")
        })
        .unwrap();
    execute_core(
        "write_save",
        json!({"path":save,"backup":false,"edits":[{"path":"private.skills.set","value":{"actor":"Hero","base":"Hunting_Scutes","tier":"Trained"}}]}),
    );
    let effects = execute_core(
        "search_typed_properties",
        json!({"path":save,"query":"Hero ActiveEffects","includeNodes":true,"limit":1000}),
    );
    let rows = effects["results"].as_array().unwrap();
    let definition = rows
        .iter()
        .find(|row| {
            row["path"]
                .as_array()
                .unwrap()
                .last()
                .and_then(Value::as_str)
                == Some("Def")
                && row["value"]
                    .as_str()
                    .is_some_and(|value| value.ends_with("GE_Skill_Hunting_Scutes_Trained"))
        })
        .expect("learned effect is present");
    let mut duration_path = definition["path"].as_array().unwrap().clone();
    *duration_path.last_mut().unwrap() = json!("Duration");
    let duration = rows
        .iter()
        .find(|row| row["path"] == json!(duration_path))
        .unwrap();
    let source_hash = gore_save::api::file_sha1(&save).unwrap();
    for (structured, raw, kind) in [
        (
            json!({"path":"private.inventory.reset","value":{"actorId":actor,"resourcesLevel":"gothic"}}),
            json!({"path":"private.typed.setValue","value":{"path":count["path"],"value":count["editValue"].as_i64().unwrap()+7}}),
            "inventoryReset",
        ),
        (
            json!({"path":"private.skills.set","value":{"actor":"Hero","base":"Hunting_Scutes","tier":"Untrained"}}),
            json!({"path":"private.typed.setValue","value":{"path":duration_path,"value":duration["editValue"].as_f64().unwrap()+7.0}}),
            "skillsEffect",
        ),
    ] {
        for edit in [&structured, &raw] {
            execute_core(
                "apply_edits",
                json!({"path":save,"edits":[edit],"dryRun":true}),
            );
        }
        for edits in [vec![structured.clone(), raw.clone()], vec![raw, structured]] {
            for command in ["write_save", "apply_edits"] {
                let error = gore_save::api::execute(&gore_save::api::Request {
                    command: command.into(),
                    payload: json!({"path":save,"edits":edits,"backup":true}),
                })
                .unwrap_err();
                if command == "apply_edits" {
                    assert!(
                        matches!(error,gore_save::CoreError::PlanConflict {kind:found,..} if found==kind)
                    );
                } else {
                    assert!(matches!(error, gore_save::CoreError::UnsupportedEdit(_)));
                }
                assert_eq!(gore_save::api::file_sha1(&save).unwrap(), source_hash);
                assert!(!home.join("goresave_backups").exists());
            }
        }
    }
}

#[test]
fn slot_checks_and_repairs_cover_other_npcs_regardless_of_the_actor_selector() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path();
    let save = home.join("G1R-001.sav");
    fs::copy(fixture(), &save).unwrap();
    let save_arg = save.to_str().unwrap();
    let actor = "OC_STT_Diego-WP_EZ_START_DIEGO_SPAWN";
    execute_core(
        "write_save",
        json!({"path":save,"backup":false,"edits":[{"path":"private.inventory.addItem","value":{"actorId":actor,"path":"/Script/Angelscript.ItMi_Orenugget","count":7}}]}),
    );
    let properties = execute_core(
        "search_typed_properties",
        json!({"path":save,"query":"m_Slots m_Id","includeNodes":true,"limit":1000}),
    );
    let path = properties["results"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| {
            row["path"].as_array().is_some_and(|path| {
                !path.iter().any(|p| p == "m_SavedPlayers")
                    && path.iter().any(|p| p == "m_Slots")
                    && path.last().and_then(Value::as_str) == Some("m_Id")
                    && !path
                        .iter()
                        .filter_map(Value::as_str)
                        .any(|p| p.to_lowercase().contains("diego"))
            })
        })
        .expect("fixture has another NPC's inventory slot")["path"]
        .clone();
    execute_core(
        "write_save",
        json!({"path":save,"backup":false,"edits":[{"path":"private.typed.setValue","value":{"path":path,"value":9999}}]}),
    );
    let damaged_hash = gore_save::api::file_sha1(&save).unwrap();
    let global = run(home, &["inventory", "check-slots", save_arg]);
    assert!(global["slotIntegrity"]["misalignedSlots"].as_u64().unwrap() > 0);
    let npc_inventory = run(
        home,
        &["inventory", "list", save_arg, "--actor", actor, "--all"],
    );
    let added = npc_inventory["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["id"] == "ItMi_Orenugget" && row["count"] == 7)
        .expect("NPC contains the added item");
    let slot = added["slotId"].as_i64().unwrap().to_string();
    for selectors in [
        vec![],
        vec!["--container", "MainContainer", "--slot", slot.as_str()],
    ] {
        let mut list_args = vec!["inventory", "list", save_arg, "--actor", actor, "--all"];
        list_args.extend_from_slice(&selectors);
        let listed = run(home, &list_args);
        assert!(!listed["items"].as_array().unwrap().is_empty());
        list_args[1] = "check-slots";
        let checked = run(home, &list_args);
        assert_eq!(checked["items"], listed["items"]);
        assert_eq!(checked["id"], listed["id"]);
        assert_eq!(checked["slotIntegrity"], global["slotIntegrity"]);
    }
    let unknown_actor = "unresolved actor is irrelevant to a global repair";
    run_failure(
        home,
        &[
            "inventory",
            "check-slots",
            save_arg,
            "--actor",
            unknown_actor,
        ],
    );
    for selected in [actor, unknown_actor] {
        run(
            home,
            &[
                "inventory",
                "repair-slots",
                save_arg,
                "--actor",
                selected,
                "--dry-run",
            ],
        );
        assert_eq!(gore_save::api::file_sha1(&save).unwrap(), damaged_hash);
        assert!(!home.join("goresave_backups").exists());
    }
    run(
        home,
        &["inventory", "repair-slots", save_arg, "--actor", actor],
    );
    let repaired = run(
        home,
        &["inventory", "check-slots", save_arg, "--actor", actor],
    );
    assert_eq!(repaired["slotIntegrity"]["misalignedSlots"], 0);
    assert_eq!(repaired["slotIntegrity"]["containers"], 0);
    assert_ne!(gore_save::api::file_sha1(&save).unwrap(), damaged_hash);
}

#[test]
fn attribute_writes_resolve_exact_sets_and_reject_ambiguous_hero_and_npc_targets() {
    for actor in ["hero", "OC_STT_Diego-WP_EZ_START_DIEGO_SPAWN"] {
        let temp = tempfile::tempdir().unwrap();
        let home = temp.path();
        let save = home.join("G1R-001.sav");
        fs::copy(fixture(), &save).unwrap();
        let save_arg = save.to_str().unwrap();
        let key = if actor == "hero" {
            "results"
        } else {
            "attributes"
        };
        let show = [
            "attributes",
            "show",
            save_arg,
            "--actor",
            actor,
            "--attribute",
            "RecoveryRatePerHourOfSleep",
            "--all",
        ];
        let before = run(home, &show);
        let full_class = "/Script/G1R.AttributeSet_Mana";
        let rows = before[key].as_array().unwrap();
        assert!(rows.iter().any(|row| row["setClass"] == full_class));
        assert!(rows.iter().any(|row| row["setClass"] != full_class));
        let mut expected_edits = Vec::new();
        let mut expected_rows = rows.clone();
        for row in &mut expected_rows {
            if row["setClass"] != full_class {
                continue;
            }
            if actor == "hero" {
                let value = match row["path"]
                    .as_array()
                    .unwrap()
                    .last()
                    .and_then(Value::as_str)
                {
                    Some("BaseValue") => 0.5,
                    Some("CurrentValue") => 0.75,
                    _ => continue,
                };
                expected_edits.push(json!({"path":"private.typed.setValue","value":{"path":row["path"],"value":value}}));
                row["value"] = json!(value.to_string());
            } else {
                for (field, path, value) in
                    [("base", "basePath", 0.5), ("current", "currentPath", 0.75)]
                {
                    expected_edits.push(json!({"path":"private.typed.setValue","value":{"path":row[path],"value":value}}));
                    row[field] = json!(value);
                }
            }
        }
        assert_eq!(expected_edits.len(), 2);
        let draft = home.join("attribute.json");
        let draft_arg = draft.to_str().unwrap();
        run(home, &["draft", "create", draft_arg, "--target", save_arg]);
        let original_draft = fs::read(&draft).unwrap();
        let original_hash = gore_save::api::file_sha1(&save).unwrap();
        let set = [
            "attributes",
            "set",
            save_arg,
            "--actor",
            actor,
            "--attribute",
            "RecoveryRatePerHourOfSleep",
            "--base",
            "0.5",
            "--current",
            "0.75",
        ];
        for selector in [None, Some("Mana")] {
            let mut args = set.to_vec();
            args.extend(["--draft", draft_arg]);
            if let Some(selector) = selector {
                args.extend(["--set-class", selector]);
            }
            let error = run_failure(home, &args);
            assert!(
                error["message"]
                    .as_str()
                    .unwrap()
                    .contains("resolve uniquely"),
                "{error}"
            );
            assert_eq!(fs::read(&draft).unwrap(), original_draft);
            assert_eq!(gore_save::api::file_sha1(&save).unwrap(), original_hash);
            assert!(!home.join("goresave_backups").exists());
        }
        let mut preview = set.to_vec();
        preview.extend(["--set-class", full_class, "--dry-run"]);
        run(home, &preview);
        assert_eq!(gore_save::api::file_sha1(&save).unwrap(), original_hash);
        assert_eq!(fs::read(&draft).unwrap(), original_draft);
        assert!(!home.join("goresave_backups").exists());
        for selector in ["AttributeSet_Mana", full_class] {
            let mut args = set.to_vec();
            args.extend(["--set-class", selector, "--draft", draft_arg]);
            run(home, &args);
            let staged = run(home, &["draft", "show", draft_arg]);
            assert_eq!(staged["edits"], json!(expected_edits));
            assert_eq!(gore_save::api::file_sha1(&save).unwrap(), original_hash);
        }
        run(home, &["draft", "apply", draft_arg]);
        assert_eq!(run(home, &show)[key], json!(expected_rows));
    }
}

#[test]
fn attribute_set_selectors_disambiguate_hero_and_npc_reads_before_pagination() {
    let home = tempfile::tempdir().unwrap();
    let save = fixture();
    let save_arg = save.to_str().unwrap();
    for actor in ["hero", "OC_STT_Diego-WP_EZ_START_DIEGO_SPAWN"] {
        let key = if actor == "hero" {
            "results"
        } else {
            "attributes"
        };
        let base = [
            "attributes",
            "show",
            save_arg,
            "--actor",
            actor,
            "--attribute",
            "RecoveryRatePerHourOfSleep",
            "--all",
        ];
        let all = run(home.path(), &base);
        let rows = all[key].as_array().unwrap();
        let full_class = "/Script/G1R.AttributeSet_Health";
        let expected: Vec<_> = rows
            .iter()
            .filter(|row| row["setClass"] == full_class)
            .cloned()
            .collect();
        assert!(!expected.is_empty());
        assert!(expected.len() < rows.len());
        for selector in [full_class, "AttributeSet_Health"] {
            let mut args = base.to_vec();
            args.extend(["--set-class", selector]);
            let shown = run(home.path(), &args);
            assert_eq!(shown[key], json!(expected));
            assert_eq!(shown["total"], expected.len());
        }
        let offset = if expected.len() > 1 { 1 } else { 0 };
        let offset_arg = offset.to_string();
        let selected = run(
            home.path(),
            &[
                "attributes",
                "list",
                save_arg,
                "--actor",
                actor,
                "--attribute",
                "RecoveryRatePerHourOfSleep",
                "--set-class",
                "AttributeSet_Health",
                "--offset",
                &offset_arg,
                "--limit",
                "1",
            ],
        );
        assert_eq!(selected[key], json!([expected[offset]]));
        assert_eq!(selected["total"], expected.len());
    }
}

#[test]
fn library_removes_missing_absolute_and_relative_paths_without_requiring_their_directory() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path();
    let directory = home.join("external");
    fs::create_dir(&directory).unwrap();
    let file = directory.join("old.sav");
    fs::write(&file, b"external library reference").unwrap();
    let path = file.to_str().unwrap();
    run(home, &["library", "add", path]);
    run(home, &["library", "hide", path]);
    fs::remove_file(&file).unwrap();
    fs::remove_dir(&directory).unwrap();
    let settings_path = home.join("gore/gore-save/settings.json");
    let before = fs::read(&settings_path).unwrap();
    let preview = run(home, &["library", "remove", path, "--dry-run"]);
    assert_eq!(preview["externalSavePaths"], json!([]));
    assert_eq!(fs::read(&settings_path).unwrap(), before);
    let preview = run_from(
        home,
        Some(home),
        &["library", "unhide", "external/old.sav", "--dry-run"],
    );
    assert_eq!(preview["hiddenOtherSavePaths"], json!([]));
    assert_eq!(fs::read(&settings_path).unwrap(), before);
    assert_eq!(
        run(home, &["library", "remove", path])["externalSavePaths"],
        json!([])
    );
    let removed = run_from(home, Some(home), &["library", "unhide", "external/old.sav"]);
    assert_eq!(removed["externalSavePaths"], json!([]));
    assert_eq!(removed["hiddenOtherSavePaths"], json!([]));
    assert!(!directory.exists());
}

#[test]
fn profile_assignment_uses_the_explicit_saves_parent_and_rejects_a_foreign_root() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path();
    let configured = home.join("configured");
    let external = home.join("external");
    fs::create_dir(&configured).unwrap();
    fs::create_dir(&external).unwrap();
    let save = external.join("G1R-001.sav");
    fs::copy(fixture(), &save).unwrap();
    let profile = configured.join("PersistentDataList.sav");
    fs::write(&profile, b"foreign profile must remain untouched").unwrap();
    run(
        home,
        &[
            "settings",
            "set",
            "--key",
            "saveDir",
            "--value",
            configured.to_str().unwrap(),
        ],
    );
    let save_sha1 = gore_save::api::file_sha1(&save).unwrap();
    let expected = external
        .canonicalize()
        .unwrap()
        .join("PersistentDataList.sav");
    for flags in [
        vec![],
        vec!["--dry-run"],
        vec!["--root", external.to_str().unwrap()],
    ] {
        let mut args = vec![
            "profile",
            "assign",
            save.to_str().unwrap(),
            "--profile",
            "0",
        ];
        let dry_run = flags.contains(&"--dry-run");
        args.extend(flags);
        let error = run_failure(home, &args);
        let message = error["message"].as_str().unwrap();
        assert!(
            message.contains("PersistentDataList.sav was not found"),
            "{error}"
        );
        if !dry_run {
            assert!(message.contains(expected.to_str().unwrap()), "{error}");
        }
    }
    let error = run_failure(
        home,
        &[
            "profile",
            "assign",
            save.to_str().unwrap(),
            "--profile",
            "0",
            "--root",
            configured.to_str().unwrap(),
        ],
    );
    assert!(
        error["message"]
            .as_str()
            .unwrap()
            .contains("use save import")
    );
    assert_eq!(gore_save::api::file_sha1(&save).unwrap(), save_sha1);
    assert_eq!(
        fs::read(&profile).unwrap(),
        b"foreign profile must remain untouched"
    );
    assert!(!configured.join("G1R-001.sav").exists());
    assert!(!configured.join("goresave_backups").exists());
    assert!(!external.join("goresave_backups").exists());
}

#[test]
fn npc_restores_require_the_original_pinned_routine_and_preserve_its_note() {
    let temp = tempfile::tempdir().unwrap();
    let save = temp.path().join("G1R-001.sav");
    fs::copy(fixture(), &save).unwrap();
    let save_arg = save.to_str().unwrap();
    let home = temp.path();
    let actor = "OC_STT_Diego-WP_EZ_START_DIEGO_SPAWN";
    let original = run(home, &["position", "show", save_arg, "--actor", actor]);
    let x = (original["pose"]["location"]["x"].as_f64().unwrap() + 100.0).to_string();
    run(
        home,
        &[
            "position", "set", save_arg, "--actor", actor, "--x", &x, "--stay",
        ],
    );
    let notes = gore_save::placement::read_notes(&save);
    let (id, valid_note) = notes.iter().next().unwrap();
    assert!(
        valid_note
            .original_routine_class
            .as_ref()
            .is_some_and(|class| !class.is_empty())
    );
    let pinned_sha1 = gore_save::api::file_sha1(&save).unwrap();
    let pinned = run(home, &["position", "show", save_arg, "--actor", actor]);
    let draft = home.join("restore-draft.json");
    run(
        home,
        &[
            "draft",
            "create",
            draft.to_str().unwrap(),
            "--target",
            save_arg,
        ],
    );
    let draft_bytes = fs::read(&draft).unwrap();
    let backup_count = fs::read_dir(home.join("goresave_backups")).unwrap().count();
    for missing in [None, Some(String::new())] {
        gore_save::placement::mutate_notes(&save, |notes| {
            notes.get_mut(id).unwrap().original_routine_class = missing.clone();
        })
        .unwrap();
        let note_path = gore_save::placement::notes_path(&save);
        let note_bytes = fs::read(&note_path).unwrap();
        let status = run(
            home,
            &["position", "pin-status", save_arg, "--actor", actor],
        );
        assert_eq!(status["undo"]["restorable"], true);
        assert_eq!(status["undo"]["routineRestorable"], true);
        for operation in ["resume-routine", "undo"] {
            for flags in [
                vec![],
                vec!["--dry-run"],
                vec!["--draft", draft.to_str().unwrap()],
            ] {
                let mut args = vec!["position", operation, save_arg, "--actor", actor];
                args.extend(flags);
                let error = run_failure(home, &args);
                assert!(
                    error["message"]
                        .as_str()
                        .unwrap()
                        .contains("original NPC routine is unavailable")
                );
                assert_eq!(gore_save::api::file_sha1(&save).unwrap(), pinned_sha1);
                assert_eq!(fs::read(&note_path).unwrap(), note_bytes);
                assert_eq!(fs::read(&draft).unwrap(), draft_bytes);
                assert_eq!(
                    fs::read_dir(home.join("goresave_backups")).unwrap().count(),
                    backup_count
                );
            }
        }
    }
    gore_save::placement::record(&save, &[(id.clone(), valid_note.clone())]).unwrap();
    run(
        home,
        &["position", "resume-routine", save_arg, "--actor", actor],
    );
    let resumed = run(home, &["position", "show", save_arg, "--actor", actor]);
    assert_eq!(resumed["routineClass"], original["routineClass"]);
    assert_eq!(resumed["pose"], pinned["pose"]);
    assert!(resumed["undo"].is_null());
    // A note for a move that left the routine alone needs only a pose restore.
    let mut move_note = valid_note.clone();
    move_note.original_routine_class = None;
    move_note.written_routine_class = None;
    gore_save::placement::record(&save, &[(id.clone(), move_note)]).unwrap();
    run(home, &["position", "undo", save_arg, "--actor", actor]);
    let restored = run(home, &["position", "show", save_arg, "--actor", actor]);
    assert_eq!(restored["pose"], original["pose"]);
    assert_eq!(restored["routineClass"], original["routineClass"]);
    assert!(restored["undo"].is_null());
}

#[test]
fn recovery_show_selects_one_record_without_changing_recovery_files() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let backups = root.join("goresave_backups");
    fs::create_dir(&backups).unwrap();
    let persistent = root.join("PersistentDataList.sav");
    fs::write(&persistent, b"guarded post-delete profile").unwrap();
    let profile_sha1 = gore_save::api::file_sha1(&persistent).unwrap();
    let mut records = Vec::new();
    let mut paths = vec![persistent.clone()];
    for (slot, epoch) in [(1, 1000), (2, 2000)] {
        let name = format!("G1R-{slot:03}.sav.bak.{epoch}");
        let backup = backups.join(&name);
        let paired = backups.join(format!("PersistentDataList.sav.bak.{epoch}"));
        fs::write(&backup, format!("deleted slot {slot}")).unwrap();
        fs::write(&paired, format!("original profile {slot}")).unwrap();
        let record = json!({
            "version":1,"createdEpoch":epoch,
            "targetPath":root.join(format!("G1R-{slot:03}.sav")),
            "backupPath":backup,"persistentPath":persistent,
            "persistentBackupPath":paired,"persistentPostDeleteSha1":profile_sha1,
            "deletedSaveSha1":gore_save::api::file_sha1(&backup).unwrap(),
            "deletedPersistentSha1":gore_save::api::file_sha1(&paired).unwrap()
        });
        let manifest = backups.join(format!(".delete-recovery.{name}.json"));
        fs::write(&manifest, serde_json::to_vec(&record).unwrap()).unwrap();
        paths.extend([backup, paired, manifest]);
        records.push(record);
    }
    let snapshots: Vec<_> = paths
        .iter()
        .map(|path| (path, fs::read(path).unwrap()))
        .collect();
    let root_arg = root.to_str().unwrap();
    let listed = run(root, &["recovery", "list", "--root", root_arg]);
    assert_eq!(listed["recoveries"], json!(records));
    let selected = run(
        root,
        &[
            "recovery",
            "show",
            "--root",
            root_arg,
            "--backup",
            records[0]["backupPath"].as_str().unwrap(),
        ],
    );
    assert_eq!(selected, records[0]);
    assert_eq!(
        run(root, &["recovery", "show", "--root", root_arg]),
        records[1]
    );
    let missing = backups.join("G1R-003.sav.bak.3000");
    let error = run_failure(
        root,
        &[
            "recovery",
            "show",
            "--root",
            root_arg,
            "--backup",
            missing.to_str().unwrap(),
        ],
    );
    assert!(
        error["message"]
            .as_str()
            .unwrap()
            .contains("no matching recovery")
    );
    for (path, bytes) in snapshots {
        assert_eq!(fs::read(path).unwrap(), bytes);
    }
    assert!(!root.join("G1R-001.sav").exists());
    assert!(!root.join("G1R-002.sav").exists());
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
fn character_show_resolves_hero_actor_aliases_and_ids_independently_of_pagination() {
    let home = tempfile::tempdir().unwrap();
    let save = fixture();
    let save = save.to_str().unwrap();
    let roster = run(home.path(), &["characters", "list", save, "--all"]);
    let rows = roster["characters"].as_array().unwrap();
    let hero = rows.iter().find(|row| row["globalId"] == "Hero").unwrap();
    let shown = run(
        home.path(),
        &[
            "characters",
            "show",
            save,
            "--offset",
            "100000",
            "--limit",
            "1",
        ],
    );
    assert_eq!(&shown, hero);
    let npc = rows
        .iter()
        .find(|row| {
            row["uniqueName"]
                .as_str()
                .is_some_and(|name| name.contains("Diego"))
                && rows
                    .iter()
                    .filter(|candidate| candidate["uniqueName"] == row["uniqueName"])
                    .count()
                    == 1
        })
        .unwrap();
    for (option, value) in [
        ("--actor", npc["uniqueName"].as_str().unwrap().to_string()),
        ("--actor", npc["globalId"].as_str().unwrap().to_lowercase()),
        ("--id", npc["globalId"].as_str().unwrap().to_uppercase()),
    ] {
        let shown = run(
            home.path(),
            &[
                "characters",
                "show",
                save,
                option,
                &value,
                "--offset",
                "100000",
                "--limit",
                "1",
            ],
        );
        assert_eq!(&shown, npc);
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
    let named = run(
        &home,
        &[
            "backups",
            "rename",
            save,
            "--backup",
            backup,
            "--name",
            "Keep this label",
        ],
    );
    assert_eq!(named["name"], "Keep this label");
    assert_eq!(
        run(&home, &["backups", "list", save])["backups"][0]["name"],
        "Keep this label"
    );
    let labels = temp.path().join("goresave_backups/backup_names.json");
    let label_bytes = fs::read(&labels).unwrap();
    let save_sha1 = gore_save::api::file_sha1(Path::new(save)).unwrap();
    let backup_sha1 = gore_save::api::file_sha1(Path::new(backup)).unwrap();
    for flags in [
        vec![],
        vec!["--name", "Other", "--clear-name"],
        vec!["--name", ""],
        vec!["--name", "  "],
    ] {
        let mut args = vec!["backups", "rename", save, "--backup", backup];
        args.extend(flags);
        let error = run_failure(&home, &args);
        assert!(error["message"].as_str().unwrap().contains("--clear-name"));
        assert_eq!(fs::read(&labels).unwrap(), label_bytes);
        assert_eq!(
            gore_save::api::file_sha1(Path::new(save)).unwrap(),
            save_sha1
        );
        assert_eq!(
            gore_save::api::file_sha1(Path::new(backup)).unwrap(),
            backup_sha1
        );
    }
    run(
        &home,
        &[
            "backups",
            "rename",
            save,
            "--backup",
            backup,
            "--clear-name",
            "--dry-run",
        ],
    );
    assert_eq!(fs::read(&labels).unwrap(), label_bytes);
    let cleared = run(
        &home,
        &[
            "backups",
            "rename",
            save,
            "--backup",
            backup,
            "--clear-name",
        ],
    );
    assert!(cleared["name"].is_null());
    assert!(run(&home, &["backups", "list", save])["backups"][0]["name"].is_null());
    let current = fs::read(save).unwrap();
    run(
        &home,
        &["backups", "restore", save, "--backup", backup, "--dry-run"],
    );
    assert_eq!(current, fs::read(save).unwrap());
    run(&home, &["backups", "restore", save, "--backup", backup]);
    assert_eq!(before, fs::read(save).unwrap());
}
