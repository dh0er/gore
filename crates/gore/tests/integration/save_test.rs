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
fn item_icon_fixture(home: &Path) -> PathBuf {
    use image::ImageEncoder;
    let generation = home.join(format!("item-icons-v1-{}", "a".repeat(64)));
    fs::create_dir_all(generation.join("images")).unwrap();
    let rgba = [255, 0, 0, 255];
    let mut png = Vec::new();
    image::codecs::png::PngEncoder::new(&mut png)
        .write_image(&rgba, 1, 1, image::ExtendedColorType::Rgba8)
        .unwrap();
    fs::write(generation.join("images/one.png"), &png).unwrap();
    let manifest = generation.join("manifest.json");
    fs::write(&manifest, serde_json::to_vec(&json!({
        "schema":1,"buildId":"fixture","itemCount":1,"items":{"ItMi_One":"images/one.png"},
        "files":{"images/one.png":{"width":1,"height":1,"byteLength":png.len(),"decodedByteLength":4,
            "pngBlake3":blake3::hash(&png).to_hex().to_string(),"rgbaBlake3":blake3::hash(&rgba).to_hex().to_string()}}
    })).unwrap()).unwrap();
    gore_tex::item_icons::verified_item_icon_manifest(&manifest).unwrap();
    manifest
}

#[test]
fn asset_release_releases_durable_leases_across_cli_processes() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path();
    let manifest = item_icon_fixture(home);
    let original = fs::read(&manifest).unwrap();
    let image = manifest.parent().unwrap().join("images/one.png");
    let original_image = fs::read(&image).unwrap();
    let entries = || {
        let mut names = fs::read_dir(home)
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect::<Vec<_>>();
        names.sort();
        names
    };
    let initial_entries = entries();
    let args = [
        "assets",
        "release",
        "--manifest",
        manifest.to_str().unwrap(),
    ];
    let mut dry = args.to_vec();
    dry.push("--dry-run");
    assert_eq!(run(home, &dry)["wouldRelease"], false);
    assert_eq!(
        entries(),
        initial_entries,
        "a preview must not create lock files"
    );
    let foreign = home.join("unrelated");
    fs::create_dir(&foreign).unwrap();
    let foreign_manifest = foreign.join("manifest.json");
    fs::write(&foreign_manifest, b"{}").unwrap();
    let invalid = [manifest.with_file_name("image.png"), foreign_manifest];
    let before_invalid = entries();
    for path in &invalid {
        for preview in [false, true] {
            let mut args = vec!["assets", "release", "--manifest", path.to_str().unwrap()];
            if preview {
                args.push("--dry-run");
            }
            run_failure(home, &args);
            assert_eq!(entries(), before_invalid);
        }
    }
    for _ in 0..2 {
        gore_tex::item_icons::retain_item_icon_cache_for_cli(&manifest).unwrap();
    }
    assert!(!gore_tex::item_icons::release_item_icon_cache(&manifest).unwrap());
    let leases = || {
        fs::read_dir(home)
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| entry.file_name().to_string_lossy().contains(".cli-lease-"))
            .count()
    };
    assert_eq!(leases(), 2);
    let retained_entries = entries();
    let preview = run(home, &dry);
    assert_eq!(preview["dryRun"], true);
    assert_eq!(preview["wouldRelease"], true);
    assert_eq!(entries(), retained_entries);
    assert_eq!(leases(), 2);
    assert_eq!(
        run_from(
            home,
            Some(manifest.parent().unwrap()),
            &["assets", "release", "--manifest", "manifest.json"]
        )["released"],
        true
    );
    assert_eq!(leases(), 1);
    assert_eq!(run(home, &args)["released"], true);
    assert_eq!(leases(), 0);
    assert_eq!(run(home, &dry)["wouldRelease"], false);
    assert_eq!(run(home, &args)["released"], false);
    assert_eq!(fs::read(&manifest).unwrap(), original);
    assert_eq!(fs::read(&image).unwrap(), original_image);
    gore_tex::item_icons::retain_item_icon_cache_for_cli(&manifest).unwrap();
    fs::remove_dir_all(manifest.parent().unwrap()).unwrap();
    assert_eq!(run(home, &dry)["wouldRelease"], true);
    assert!(!manifest.parent().unwrap().exists());
    assert_eq!(leases(), 1);
    assert_eq!(run(home, &args)["released"], true);
    assert_eq!(leases(), 0);
    assert_eq!(run(home, &args)["released"], false);
}

#[test]
fn export_previews_reject_missing_output_parents_without_creating_them() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path();
    let save = home.join("G1R-001.sav");
    fs::copy(fixture(), &save).unwrap();
    let hash = gore_save::api::file_sha1(&save).unwrap();
    let screenshot = home.join("Profile_0_Screenshots.sav");
    let screenshot_bytes = screenshot_fixture("G1R-001", &[0xff, 0xd8, 0xff, 0xd9]);
    fs::write(&screenshot, &screenshot_bytes).unwrap();
    let game = home.join("game");
    let portraits = game.join("G1R/Story/Conversation/images/Glossary/Locations");
    fs::create_dir_all(&portraits).unwrap();
    let artwork = portraits.join("T_GlossaryImage_AbandonedMine_S.png");
    fs::write(&artwork, b"original artwork").unwrap();
    let manifest = item_icon_fixture(home);
    let out = home.join("missing/child/export");
    let workflows = [
        vec!["screenshot", "export", save.to_str().unwrap()],
        vec![
            "assets",
            "export",
            "--kind",
            "portraits",
            "--game",
            game.to_str().unwrap(),
            "--id",
            "Document_Glossary_AbandonedMine",
        ],
        vec![
            "assets",
            "export",
            "--manifest",
            manifest.to_str().unwrap(),
            "--id",
            "ItMi_One",
        ],
        vec!["report", save.to_str().unwrap()],
    ];
    for mut args in workflows {
        args.extend(["--out", out.to_str().unwrap()]);
        for dry in [false, true] {
            let mut invocation = args.clone();
            if dry {
                invocation.push("--dry-run");
            }
            run_failure(home, &invocation);
            assert!(!home.join("missing").exists());
        }
    }
    let preview = run_from(
        home,
        Some(home),
        &[
            "screenshot",
            "export",
            "G1R-001.sav",
            "--out",
            "fresh.jpg",
            "--dry-run",
        ],
    );
    assert_eq!(preview["dryRun"], true);
    assert!(!home.join("fresh.jpg").exists());
    assert_eq!(gore_save::api::file_sha1(&save).unwrap(), hash);
    assert_eq!(fs::read(screenshot).unwrap(), screenshot_bytes);
    assert_eq!(fs::read(artwork).unwrap(), b"original artwork");
    assert!(!home.join("goresave_backups").exists());
}

fn execute_core(command: &str, payload: Value) -> Value {
    gore_save::api::execute(&gore_save::api::Request {
        command: command.into(),
        payload,
    })
    .unwrap()
}

fn nested_container_fixture(save: &Path) {
    nested_container_fixture_with_strings(save, false);
}

fn nested_container_fixture_with_strings(save: &Path, include_strings: bool) {
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
    let mut set_type = 1u32.to_le_bytes().to_vec();
    set_type.extend(string("NameProperty"));
    let mut set = 0u32.to_le_bytes().to_vec();
    set.extend(1u32.to_le_bytes());
    set.extend(string("ChoiceA"));
    let mut notes_type = 1u32.to_le_bytes().to_vec();
    notes_type.extend(string("StrProperty"));
    let mut notes = 2u32.to_le_bytes().to_vec();
    notes.extend(string("A"));
    notes.extend(string("B"));
    let mut event = property("Knowledge", "SetProperty", &set_type, &set);
    event.extend(property("Notes", "ArrayProperty", &notes_type, &notes));
    event.extend(string("None"));
    let mut events = 2u32.to_le_bytes().to_vec();
    events.extend(&event);
    events.extend(event);
    let mut events_type = 1u32.to_le_bytes().to_vec();
    events_type.extend(string("StructProperty"));
    events_type.extend(1u32.to_le_bytes());
    events_type.extend(string("EventData"));
    events_type.extend(1u32.to_le_bytes());
    events_type.extend(string("/Script/G1R"));
    let mut private = string("/Script/Angelscript.GothicFinalDataGame");
    private.push(0);
    private.extend(property("Events", "ArrayProperty", &events_type, &events));
    private.extend(property("Other", "SetProperty", &set_type, &set));
    if include_strings {
        let mut strings_type = 1u32.to_le_bytes().to_vec();
        strings_type.extend(string("StrProperty"));
        private.extend(property("Strings", "SetProperty", &strings_type, &set));
    }
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
    let size = (bytes.len() + stream.len()) as u32;
    bytes[5..9].copy_from_slice(&size.to_le_bytes());
    bytes.extend(stream);
    bytes.extend(0u32.to_le_bytes());
    fs::write(save, bytes).unwrap();
}

#[test]
fn set_case_conflicts_follow_name_and_string_element_descriptors() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path();
    let save = home.join("G1R-001.sav");
    nested_container_fixture_with_strings(&save, true);
    let original = fs::read(&save).unwrap();
    let hash = gore_save::api::file_sha1(&save).unwrap();
    let draft = home.join("case.json");
    let request = home.join("request.json");
    for (first_op, first_value, second_op, second_value) in [
        (
            "private.typed.setAdd",
            "ChoiceB",
            "private.typed.setRemove",
            "CHOICEb",
        ),
        (
            "private.typed.setRemove",
            "choiceA",
            "private.typed.setAdd",
            "CHOICEa",
        ),
        (
            "private.typed.setAdd",
            "ChoiceB",
            "private.typed.setAdd",
            "CHOICEb",
        ),
        (
            "private.typed.setRemove",
            "ChoiceA",
            "private.typed.setRemove",
            "choicea",
        ),
    ] {
        let edits = vec![
            json!({"path":first_op,"value":{"path":["Events","[01]","Knowledge"],"value":first_value}}),
            json!({"path":second_op,"value":{"path":["Events","[1]","Knowledge"],"value":second_value}}),
        ];
        execute_core(
            "apply_edits",
            json!({"path":save,"edits":[edits[0]],"dryRun":true}),
        );
        for command in ["plan_edits", "write_save", "apply_edits"] {
            for dry in [false, true] {
                let payload = json!({"path":save,"edits":edits,"dryRun":dry,"backup":true});
                let error = gore_save::api::execute(&gore_save::api::Request {
                    command: command.into(),
                    payload: payload.clone(),
                })
                .unwrap_err();
                assert!(
                    matches!(
                        error,
                        gore_save::CoreError::PlanConflict {
                            kind: "property",
                            ..
                        } | gore_save::CoreError::UnsupportedEdit(_)
                    ),
                    "{error}"
                );
                if command == "apply_edits" {
                    fs::write(
                        &request,
                        serde_json::to_vec(&json!({"command":command,"payload":payload})).unwrap(),
                    )
                    .unwrap();
                    let error = run_failure(
                        home,
                        &["core", "exec", "--request-file", request.to_str().unwrap()],
                    );
                    assert_eq!(error["code"], "PLAN_CONFLICT", "{error}");
                }
                assert_eq!(fs::read(&save).unwrap(), original);
                assert!(!home.join("goresave_backups").exists());
            }
        }
        let pending = serde_json::to_vec(&json!({"format":"gore.save.draft.v1","path":save.canonicalize().unwrap(),"expectedSha1":hash,"edits":edits})).unwrap();
        fs::write(&draft, &pending).unwrap();
        for flags in [vec!["validate"], vec!["apply"], vec!["apply", "--dry-run"]] {
            let mut args = vec!["draft", flags[0], draft.to_str().unwrap()];
            args.extend_from_slice(&flags[1..]);
            let error = run_failure(home, &args);
            assert!(
                error.to_string().contains("pending edit conflict"),
                "{error}"
            );
            assert_eq!(fs::read(&draft).unwrap(), pending);
            assert_eq!(fs::read(&save).unwrap(), original);
            assert!(!home.join("goresave_backups").exists());
        }
    }
    let add = json!({"path":"private.typed.setAdd","value":{"path":["Strings"],"value":"choicea"}});
    let remove =
        json!({"path":"private.typed.setRemove","value":{"path":["Strings"],"value":"ChoiceA"}});
    for edits in [vec![add.clone(), remove.clone()], vec![remove, add]] {
        // A source-less planner cannot establish whether these are FNames.
        assert!(gore_save::workflow::plan(&edits).is_err());
        assert_eq!(
            execute_core("plan_edits", json!({"path":save,"edits":edits}))["groups"],
            json!([[0, 1]])
        );
        execute_core(
            "apply_edits",
            json!({"path":save,"edits":edits,"dryRun":true}),
        );
        assert_eq!(fs::read(&save).unwrap(), original);
        for command in ["write_save", "apply_edits"] {
            fs::write(&save, &original).unwrap();
            let result = execute_core(command, json!({"path":save,"edits":edits,"backup":false}));
            if command == "apply_edits" {
                assert_eq!(result["complete"], true);
                assert_eq!(result["committed"], json!([0, 1]));
            } else {
                assert_eq!(result["editsApplied"], 2);
            }
            let inspected =
                execute_core("inspect_save", json!({"path":save,"includePrivate":true}));
            assert!(
                inspected["private"]["strings"]
                    .as_array()
                    .unwrap()
                    .contains(&json!("choicea"))
            );
            let data = execute_core(
                "search_typed_properties",
                json!({"path":save,"query":"Strings","includeNodes":true}),
            );
            let row = data["results"]
                .as_array()
                .unwrap()
                .iter()
                .find(|row| row["path"] == json!(["Strings"]))
                .unwrap();
            assert_eq!(row["childCount"], 1);
            assert!(!home.join("goresave_backups").exists());
        }
        fs::write(&save, &original).unwrap();
        let pending = serde_json::to_vec(&json!({"format":"gore.save.draft.v1","path":save.canonicalize().unwrap(),"expectedSha1":hash,"edits":edits,"backup":false})).unwrap();
        fs::write(&draft, &pending).unwrap();
        run(home, &["draft", "validate", draft.to_str().unwrap()]);
        assert_eq!(fs::read(&draft).unwrap(), pending);
        assert_eq!(fs::read(&save).unwrap(), original);
        assert_eq!(
            run(home, &["draft", "apply", draft.to_str().unwrap()])["complete"],
            true
        );
        fs::write(&save, &original).unwrap();
    }

    for command in ["write_save", "apply_edits"] {
        fs::write(&save, &original).unwrap();
        for (operation, expected_count) in
            [("private.typed.setAdd", 3), ("private.typed.setRemove", 1)]
        {
            let edits = ["ChoiceB", "choiceb"]
                .map(|value| json!({"path":operation,"value":{"path":["Strings"],"value":value}}));
            let before = fs::read(&save).unwrap();
            assert_eq!(
                execute_core("plan_edits", json!({"path":save,"edits":edits}))["groups"],
                json!([[0, 1]])
            );
            execute_core(
                "apply_edits",
                json!({"path":save,"edits":edits,"dryRun":true}),
            );
            assert_eq!(fs::read(&save).unwrap(), before);
            execute_core(command, json!({"path":save,"edits":edits,"backup":false}));
            let data = execute_core(
                "search_typed_properties",
                json!({"path":save,"query":"Strings","includeNodes":true}),
            );
            let row = data["results"]
                .as_array()
                .unwrap()
                .iter()
                .find(|row| row["path"] == json!(["Strings"]))
                .unwrap();
            assert_eq!(row["childCount"], expected_count);
            assert!(!home.join("goresave_backups").exists());
        }
    }
}

#[test]
fn set_element_conflicts_preserve_drafts_and_allow_independent_changes() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path();
    let save = home.join("G1R-001.sav");
    nested_container_fixture(&save);
    let original = fs::read(&save).unwrap();
    let hash = gore_save::api::file_sha1(&save).unwrap();
    let draft = home.join("opposing.json");
    let request = home.join("request.json");
    let path_file = home.join("set-path.json");
    fs::write(&path_file, br#"["Events","[01]","Knowledge"]"#).unwrap();
    for (first_op, second_op, value) in [
        ("private.typed.setAdd", "private.typed.setRemove", "ChoiceB"),
        ("private.typed.setRemove", "private.typed.setAdd", "ChoiceA"),
        ("private.typed.setAdd", "private.typed.setAdd", "ChoiceB"),
        (
            "private.typed.setRemove",
            "private.typed.setRemove",
            "ChoiceA",
        ),
    ] {
        let first =
            json!({"path":first_op,"value":{"path":["Events","[01]","Knowledge"],"value":value}});
        let second =
            json!({"path":second_op,"value":{"path":["Events","[1]","Knowledge"],"value":value}});
        execute_core(
            "apply_edits",
            json!({"path":save,"edits":[first.clone()],"dryRun":true}),
        );
        let edits = vec![first.clone(), second];
        for command in ["plan_edits", "write_save", "apply_edits"] {
            for dry in [false, true] {
                let payload = json!({"path":save,"edits":edits,"dryRun":dry,"backup":true});
                let error = gore_save::api::execute(&gore_save::api::Request {
                    command: command.into(),
                    payload: payload.clone(),
                })
                .unwrap_err();
                assert!(
                    matches!(
                        error,
                        gore_save::CoreError::PlanConflict {
                            kind: "property",
                            ..
                        } | gore_save::CoreError::UnsupportedEdit(_)
                    ),
                    "{error}"
                );
                if command == "apply_edits" {
                    fs::write(
                        &request,
                        serde_json::to_vec(&json!({"command":command,"payload":payload})).unwrap(),
                    )
                    .unwrap();
                    let error = run_failure(
                        home,
                        &["core", "exec", "--request-file", request.to_str().unwrap()],
                    );
                    assert_eq!(error["code"], "PLAN_CONFLICT", "{error}");
                }
                assert_eq!(fs::read(&save).unwrap(), original);
                assert!(!home.join("goresave_backups").exists());
            }
        }
        let pending = serde_json::to_vec(&json!({"format":"gore.save.draft.v1","path":save.canonicalize().unwrap(),"expectedSha1":hash,"edits":edits})).unwrap();
        fs::write(&draft, &pending).unwrap();
        for flags in [vec!["validate"], vec!["apply"], vec!["apply", "--dry-run"]] {
            let mut args = vec!["draft", flags[0], draft.to_str().unwrap()];
            args.extend_from_slice(&flags[1..]);
            let error = run_failure(home, &args);
            assert!(
                error.to_string().contains("pending edit conflict"),
                "{error}"
            );
            assert_eq!(fs::read(&draft).unwrap(), pending);
            assert_eq!(fs::read(&save).unwrap(), original);
            assert!(!home.join("goresave_backups").exists());
        }
        fs::remove_file(&draft).unwrap();
        let value_arg = serde_json::to_string(value).unwrap();
        run(
            home,
            &[
                "data",
                if first_op == "private.typed.setAdd" {
                    "set-add"
                } else {
                    "set-remove"
                },
                save.to_str().unwrap(),
                "--path-file",
                path_file.to_str().unwrap(),
                "--value-json",
                &value_arg,
                "--draft",
                draft.to_str().unwrap(),
            ],
        );
        run(
            home,
            &[
                "data",
                if second_op == "private.typed.setAdd" {
                    "set-add"
                } else {
                    "set-remove"
                },
                save.to_str().unwrap(),
                "--path-file",
                path_file.to_str().unwrap(),
                "--value-json",
                &value_arg,
                "--draft",
                draft.to_str().unwrap(),
            ],
        );
        let opposed = fs::read(&draft).unwrap();
        let pending: Value = serde_json::from_slice(&opposed).unwrap();
        assert_eq!(pending["edits"].as_array().unwrap().len(), 2);
        for flags in [vec!["validate"], vec!["apply"], vec!["apply", "--dry-run"]] {
            let mut args = vec!["draft", flags[0], draft.to_str().unwrap()];
            args.extend_from_slice(&flags[1..]);
            let error = run_failure(home, &args);
            assert!(
                error.to_string().contains("pending edit conflict"),
                "{error}"
            );
            assert_eq!(fs::read(&draft).unwrap(), opposed);
        }
        assert_eq!(fs::read(&save).unwrap(), original);
        assert!(!home.join("goresave_backups").exists());
    }
    let result = execute_core(
        "apply_edits",
        json!({"path":save,"backup":false,"edits":[
            {"path":"private.typed.setAdd","value":{"path":["Events","[1]","Knowledge"],"value":"ChoiceB"}},
            {"path":"private.typed.setRemove","value":{"path":["Events","[1]","Knowledge"],"value":"ChoiceA"}}
        ]}),
    );
    assert_eq!(result["complete"], true, "{result}");
    assert_eq!(result["committed"], json!([0, 1]));
    let inspected = execute_core("inspect_save", json!({"path":save,"includePrivate":true}));
    assert!(
        inspected["private"]["strings"]
            .as_array()
            .unwrap()
            .contains(&json!("ChoiceB"))
    );
    let properties = execute_core(
        "search_typed_properties",
        json!({"path":save,"query":"Knowledge","includeNodes":true}),
    );
    let selected = properties["results"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["path"] == json!(["Events", "[1]", "Knowledge"]))
        .unwrap();
    assert_eq!(selected["childCount"], 1);
    assert!(!home.join("goresave_backups").exists());
}

#[test]
fn structural_array_conflicts_preserve_nested_container_edits_in_drafts() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path();
    let save = home.join("G1R-001.sav");
    nested_container_fixture(&save);
    let hash = gore_save::api::file_sha1(&save).unwrap();
    let draft = home.join("nested.json");
    run(
        home,
        &[
            "draft",
            "create",
            draft.to_str().unwrap(),
            "--target",
            save.to_str().unwrap(),
        ],
    );
    let template: Value = serde_json::from_slice(&fs::read(&draft).unwrap()).unwrap();
    for parent in ["private.typed.arrayRemove", "private.typed.arrayDuplicate"] {
        let structural = json!({"path":parent,"value":{"path":["Events"],"index":1}});
        execute_core(
            "apply_edits",
            json!({"path":save,"edits":[structural.clone()],"dryRun":true}),
        );
        for (operation, field, value) in [
            ("private.typed.setAdd", "Knowledge", json!("ChoiceB")),
            ("private.typed.setRemove", "Knowledge", json!("ChoiceA")),
            ("private.typed.arrayRemove", "Notes", Value::Null),
            ("private.typed.arrayDuplicate", "Notes", Value::Null),
        ] {
            let descendant = json!({"path":operation,"value":{"path":["Events","[01]",field],"value":value,"index":0}});
            execute_core(
                "apply_edits",
                json!({"path":save,"edits":[descendant.clone()],"dryRun":true}),
            );
            let mut single = template.clone();
            single["edits"] = json!([descendant.clone()]);
            fs::write(&draft, serde_json::to_vec(&single).unwrap()).unwrap();
            run(home, &["draft", "validate", draft.to_str().unwrap()]);
            for edits in [
                vec![structural.clone(), descendant.clone()],
                vec![descendant.clone(), structural.clone()],
            ] {
                for dry in [false, true] {
                    let error = gore_save::api::execute(&gore_save::api::Request {
                        command: "apply_edits".into(),
                        payload: json!({"path":save,"edits":edits,"backup":true,"dryRun":dry}),
                    })
                    .unwrap_err();
                    assert!(
                        matches!(
                            error,
                            gore_save::CoreError::PlanConflict {
                                kind: "structuralValue",
                                ..
                            }
                        ),
                        "{error}"
                    );
                }
                let mut staged = template.clone();
                staged["edits"] = json!(edits);
                let bytes = serde_json::to_vec(&staged).unwrap();
                fs::write(&draft, &bytes).unwrap();
                for command in ["validate", "apply"] {
                    run_failure(home, &["draft", command, draft.to_str().unwrap()]);
                    assert_eq!(fs::read(&draft).unwrap(), bytes);
                    assert_eq!(gore_save::api::file_sha1(&save).unwrap(), hash);
                    assert!(!home.join("goresave_backups").exists());
                }
            }
        }
    }
    let committed = execute_core(
        "apply_edits",
        json!({"path":save,"backup":false,"edits":[
            {"path":"private.typed.setAdd","value":{"path":["Other"],"value":"UnrelatedChoice"}},
            {"path":"private.typed.arrayRemove","value":{"path":["Events"],"index":1}}
        ]}),
    );
    assert_eq!(committed["committed"], json!([0, 1]));
    let inspected = execute_core("inspect_save", json!({"path":save,"includePrivate":true}));
    assert!(
        inspected["private"]["strings"]
            .as_array()
            .unwrap()
            .contains(&json!("UnrelatedChoice"))
    );
    let properties = execute_core(
        "search_typed_properties",
        json!({"path":save,"query":"Events","includeNodes":true}),
    );
    let events = properties["results"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["path"] == json!(["Events"]))
        .unwrap();
    assert_eq!(events["childCount"], 1);
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
    profile_fixture_with_metadata(preset, None)
}

fn profile_fixture_with_metadata(preset: &str, metadata: Option<(i32, f64)>) -> Vec<u8> {
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
    if let Some((chapter, played)) = metadata {
        public.extend(property(
            "m_ChapterID",
            "IntProperty",
            &[],
            &chapter.to_le_bytes(),
        ));
        public.extend(property(
            "m_TimePlayed",
            "DoubleProperty",
            &[],
            &played.to_le_bytes(),
        ));
    }
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

fn screenshot_fixture(slot: &str, jpeg: &[u8]) -> Vec<u8> {
    fn string(value: &str) -> Vec<u8> {
        let mut bytes = ((value.len() + 1) as i32).to_le_bytes().to_vec();
        bytes.extend_from_slice(value.as_bytes());
        bytes.push(0);
        bytes
    }
    let mut payload = Vec::new();
    for value in [
        "m_Screenshots",
        "MapProperty",
        "StrProperty",
        "ArrayProperty",
        "ByteProperty",
        slot,
    ] {
        payload.extend(string(value));
    }
    payload.extend_from_slice(jpeg);
    payload.extend(string("None"));
    let mut screenshot_bytes = b"GSAV".to_vec();
    screenshot_bytes.push(2);
    screenshot_bytes.extend(((13 + payload.len()) as u32).to_le_bytes());
    screenshot_bytes.extend(0u32.to_le_bytes());
    screenshot_bytes.extend(payload);
    screenshot_bytes
}

#[test]
fn portrait_exports_refuse_source_artwork_aliases_and_copy_other_outputs() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path();
    let game = home.join("game");
    let root = game.join("G1R/Story/Conversation/images/Glossary/Locations");
    fs::create_dir_all(&root).unwrap();
    let small = root.join("T_GlossaryImage_AbandonedMine_S.png");
    let medium = root.join("T_GlossaryImage_AbandonedMine_M.png");
    let image = b"\x89PNG\r\n\x1a\noriginal game artwork";
    fs::write(&small, image).unwrap();
    fs::write(&medium, image).unwrap();
    for source in [&small, &medium] {
        let hardlink = home.join(if source == &small {
            "small-link.png"
        } else {
            "medium-link.png"
        });
        fs::hard_link(source, &hardlink).unwrap();
        let mut aliases = vec![source.clone(), hardlink];
        #[cfg(unix)]
        {
            let link = source.with_extension("symlink");
            std::os::unix::fs::symlink(source, &link).unwrap();
            aliases.push(link);
        }
        #[cfg(windows)]
        aliases.push(source.with_extension("PNG"));
        let mut base = vec![
            "assets",
            "export",
            "--kind",
            "portraits",
            "--game",
            game.to_str().unwrap(),
            "--id",
            "Document_Glossary_AbandonedMine",
        ];
        if source == &medium {
            base.push("--details");
        }
        for out in &aliases {
            for dry_run in [false, true] {
                let mut args = base.clone();
                args.extend(["--out", out.to_str().unwrap()]);
                if dry_run {
                    args.push("--dry-run");
                }
                let error = run_failure(home, &args);
                assert!(
                    error
                        .to_string()
                        .contains("must not refer to the source artwork"),
                    "{error}"
                );
                assert_eq!(fs::read(source).unwrap(), image);
                assert_eq!(fs::read(out).unwrap(), image);
            }
        }
        let out = home.join(if source == &small {
            "small-export.png"
        } else {
            "medium-export.png"
        });
        let mut args = base.clone();
        args.extend(["--out", out.to_str().unwrap()]);
        let mut dry = args.clone();
        dry.push("--dry-run");
        run(home, &dry);
        assert!(!out.exists());
        run(home, &args);
        assert_eq!(fs::read(&out).unwrap(), image);
        let previous = b"a much longer previous output to prove the copied file is fully truncated";
        fs::write(&out, previous).unwrap();
        run(home, &dry);
        assert_eq!(fs::read(&out).unwrap(), previous);
        run(home, &args);
        assert_eq!(fs::read(&out).unwrap(), image);
    }
    assert_eq!(fs::read(&small).unwrap(), image);
    assert_eq!(fs::read(&medium).unwrap(), image);
    assert!(!home.join("goresave_backups").exists());
}

#[test]
fn draft_staging_keeps_distinct_inventory_id_counts_and_applies_both() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path();
    let save = home.join("G1R-001.sav");
    let draft = home.join("draft.json");
    let payload = home.join("payload.json");
    fs::copy(fixture(), &save).unwrap();
    let original = fs::read(&save).unwrap();
    let count = |id, count| json!({"path":"private.inventory.setItemCount","value":{"id":id,"count":count}});
    run(
        home,
        &[
            "draft",
            "create",
            draft.to_str().unwrap(),
            "--target",
            save.to_str().unwrap(),
        ],
    );
    let stage = |edit, dry| {
        fs::write(
            &payload,
            serde_json::to_vec(&json!({"path":save,"edits":[edit]})).unwrap(),
        )
        .unwrap();
        let mut args = vec![
            "draft",
            "stage",
            draft.to_str().unwrap(),
            "--payload-file",
            payload.to_str().unwrap(),
        ];
        if dry {
            args.push("--dry-run");
        }
        run(home, &args)
    };
    assert_eq!(
        stage(count("ItWr_Scroll_Letter_01", 11), false)["pending"],
        1
    );
    let before = fs::read(&draft).unwrap();
    assert_eq!(stage(count("ItMs_Glossary", 22), true)["pending"], 2);
    assert_eq!(fs::read(&draft).unwrap(), before);
    assert_eq!(stage(count("ItMs_Glossary", 22), false)["pending"], 2);
    let staged = stage(count("ItMs_Glossary", 33), false);
    assert_eq!(staged["pending"], 2);
    assert_eq!(
        staged["data"]["edits"][0],
        count("ItWr_Scroll_Letter_01", 11)
    );
    assert_eq!(staged["data"]["edits"][1], count("ItMs_Glossary", 33));
    run(home, &["draft", "validate", draft.to_str().unwrap()]);
    assert_eq!(fs::read(&save).unwrap(), original);
    assert!(!home.join("goresave_backups").exists());
    assert_eq!(
        run(home, &["draft", "apply", draft.to_str().unwrap()])["complete"],
        true
    );
    let inventory = run(
        home,
        &["inventory", "list", save.to_str().unwrap(), "--all"],
    );
    for (id, count) in [("ItWr_Scroll_Letter_01", 11), ("ItMs_Glossary", 33)] {
        assert_eq!(
            inventory["items"]
                .as_array()
                .unwrap()
                .iter()
                .find(|item| item["id"] == id)
                .unwrap()["count"],
            count
        );
    }
    assert_eq!(
        run(home, &["draft", "show", draft.to_str().unwrap()])["edits"],
        json!([])
    );
}

#[test]
fn npc_revival_and_health_edits_are_rejected_without_consuming_the_draft() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path();
    let save = home.join("G1R-001.sav");
    fs::copy(fixture(), &save).unwrap();
    let hash = gore_save::api::file_sha1(&save).unwrap();
    let actor = "OC_STT_Diego-WP_EZ_START_DIEGO_SPAWN";
    let rows = run(
        home,
        &[
            "attributes",
            "show",
            save.to_str().unwrap(),
            "--actor",
            actor,
            "--attribute",
            "Health",
        ],
    );
    let health = &rows["attributes"][0];
    let revive = json!({"path":"private.npc.revive","value":{"id":actor}});
    execute_core(
        "apply_edits",
        json!({"path":save,"edits":[revive.clone()],"dryRun":true}),
    );
    let file = home.join("health-revive.json");
    for (field, flag) in [("basePath", "--base"), ("currentPath", "--current")] {
        let raw =
            json!({"path":"private.typed.setValue","value":{"path":health[field],"value":1.0}});
        execute_core(
            "apply_edits",
            json!({"path":save,"edits":[raw.clone()],"dryRun":true}),
        );
        for revive_first in [false, true] {
            let edits = if revive_first {
                vec![revive.clone(), raw.clone()]
            } else {
                vec![raw.clone(), revive.clone()]
            };
            for command in ["write_save", "apply_edits"] {
                for dry in [false, true] {
                    let error = gore_save::api::execute(&gore_save::api::Request {
                        command: command.into(),
                        payload: json!({"path":save,"edits":edits,"dryRun":dry,"backup":true}),
                    })
                    .unwrap_err();
                    match command {
                        "apply_edits" => assert!(
                            matches!(
                                error,
                                gore_save::CoreError::PlanConflict {
                                    kind: "property",
                                    ..
                                }
                            ),
                            "{error}"
                        ),
                        _ => assert!(error.to_string().contains("rewrites as a whole"), "{error}"),
                    }
                }
            }
            let health_args = [
                "attributes",
                "set",
                save.to_str().unwrap(),
                "--actor",
                actor,
                "--attribute",
                "Health",
                flag,
                "1",
                "--draft",
                file.to_str().unwrap(),
            ];
            let revive_args = [
                "npc",
                "revive",
                save.to_str().unwrap(),
                "--actor",
                actor,
                "--draft",
                file.to_str().unwrap(),
            ];
            if revive_first {
                run(home, &revive_args);
                run(home, &health_args);
            } else {
                run(home, &health_args);
                run(home, &revive_args);
            }
            let pending = fs::read(&file).unwrap();
            for verb in ["validate", "apply"] {
                run_failure(home, &["draft", verb, file.to_str().unwrap()]);
                assert_eq!(fs::read(&file).unwrap(), pending);
                assert_eq!(gore_save::api::file_sha1(&save).unwrap(), hash);
                assert!(!home.join("goresave_backups").exists());
            }
            fs::remove_file(&file).unwrap();
        }
    }
}

#[test]
fn screenshot_export_rejects_invalid_sources_without_creating_or_truncating_output() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path();
    let save = home.join("G1R-001.sav");
    let profile = home.join("PersistentDataList.sav");
    let profile_bytes = profile_fixture("Gothic");
    fs::write(&profile, &profile_bytes).unwrap();
    let sidecar = home.join("Profile_0_Screenshots.sav");
    let jpeg = [0xff, 0xd8, 0xaa, 0xbb, 0xff, 0xd9];
    let screenshot_bytes = screenshot_fixture("G1R-001", &jpeg);
    fs::write(&sidecar, &screenshot_bytes).unwrap();
    let valid = fs::read(fixture()).unwrap();
    let claim = home.join("G1R-001.sav.assign-final-goresave-1-2-3");
    fs::write(&claim, &valid).unwrap();
    let output = home.join("existing.jpg");
    let fresh = home.join("fresh.jpg");
    fs::write(&output, b"previous export").unwrap();
    for bytes in [
        b"not a save".to_vec(),
        b"GSAV".to_vec(),
        profile_bytes.clone(),
        valid[..20].to_vec(),
    ] {
        fs::write(&save, &bytes).unwrap();
        for out in [&output, &fresh] {
            for dry in [false, true] {
                let mut args = vec![
                    "screenshot",
                    "export",
                    save.to_str().unwrap(),
                    "--out",
                    out.to_str().unwrap(),
                ];
                if dry {
                    args.push("--dry-run");
                }
                run_failure(home, &args);
                assert_eq!(fs::read(&save).unwrap(), bytes);
                assert_eq!(fs::read(&profile).unwrap(), profile_bytes);
                assert_eq!(fs::read(&sidecar).unwrap(), screenshot_bytes);
                assert_eq!(fs::read(&claim).unwrap(), valid);
                assert_eq!(fs::read(&output).unwrap(), b"previous export");
                assert!(!fresh.exists());
                assert!(!home.join("goresave_backups").exists());
            }
        }
    }
    fs::remove_file(&save).unwrap();
    fs::create_dir(&save).unwrap();
    run_failure(
        home,
        &[
            "screenshot",
            "export",
            save.to_str().unwrap(),
            "--out",
            output.to_str().unwrap(),
        ],
    );
    assert_eq!(fs::read(&output).unwrap(), b"previous export");
    fs::remove_dir(&save).unwrap();
    fs::write(&save, &valid).unwrap();
    run(
        home,
        &[
            "screenshot",
            "export",
            save.to_str().unwrap(),
            "--out",
            fresh.to_str().unwrap(),
            "--dry-run",
        ],
    );
    assert!(!fresh.exists());
    run(
        home,
        &[
            "screenshot",
            "export",
            save.to_str().unwrap(),
            "--out",
            output.to_str().unwrap(),
        ],
    );
    assert_eq!(fs::read(&output).unwrap(), jpeg);
    assert_eq!(fs::read(&save).unwrap(), valid);
    assert_eq!(fs::read(&claim).unwrap(), valid);
    assert_eq!(fs::read(&profile).unwrap(), profile_bytes);
    assert_eq!(fs::read(&sidecar).unwrap(), screenshot_bytes);
    assert!(!home.join("goresave_backups").exists());
}

#[test]
fn reports_embed_readonly_screenshot_sidecars_without_repairing_save_state() {
    use base64::Engine;
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path();
    let save = home.join("G1R-001.sav");
    fs::copy(fixture(), &save).unwrap();
    let save_arg = save.to_str().unwrap();
    let hash = gore_save::api::file_sha1(&save).unwrap();
    let profile = home.join("PersistentDataList.sav");
    let profile_bytes = profile_fixture("Gothic");
    fs::write(&profile, &profile_bytes).unwrap();
    let claim = home.join("G1R-001.sav.assign-final-goresave-1-2-3");
    fs::copy(&save, &claim).unwrap();
    let unrelated = home.join("G1R-999.sav");
    fs::create_dir(&unrelated).unwrap();
    let jpeg = [0xff, 0xd8, 0xaa, 0xbb, 0xff, 0xd9];
    let screenshot_bytes = screenshot_fixture("G1R-001", &jpeg);
    let sidecar = home.join("Profile_0_Screenshots.sav");
    fs::write(&sidecar, &screenshot_bytes).unwrap();
    let foreign_sidecar = home.join("Profile_1_Screenshots.sav");
    let foreign_bytes = screenshot_fixture("G1R-001", &[0xff, 0xd8, 0x01, 0x02, 0xff, 0xd9]);
    fs::write(&foreign_sidecar, &foreign_bytes).unwrap();
    let encoded = base64::engine::general_purpose::STANDARD.encode(jpeg);
    let overview = run(home, &["overview", save_arg]);
    assert_eq!(
        overview["inspection"]["screenshot"]["mimeType"],
        "image/jpeg"
    );
    assert_eq!(overview["inspection"]["screenshot"]["bytesBase64"], encoded);
    let report = home.join("report.html");
    fs::write(&report, b"existing report").unwrap();
    for dry in [true, false] {
        let mut args = vec!["report", save_arg, "--out", report.to_str().unwrap()];
        if dry {
            args.push("--dry-run");
        }
        run(home, &args);
        if dry {
            assert_eq!(fs::read(&report).unwrap(), b"existing report");
        } else {
            let html = fs::read_to_string(&report).unwrap();
            assert!(html.contains(&format!(
                "alt=\"Save screenshot\" src=\"data:image/jpeg;base64,{encoded}\""
            )));
            assert_eq!(html.matches("data:image/jpeg;base64,").count(), 1);
        }
        assert_eq!(gore_save::api::file_sha1(&save).unwrap(), hash);
        assert_eq!(gore_save::api::file_sha1(&claim).unwrap(), hash);
        assert_eq!(fs::read(&profile).unwrap(), profile_bytes);
        assert_eq!(fs::read(&sidecar).unwrap(), screenshot_bytes);
        assert_eq!(fs::read(&foreign_sidecar).unwrap(), foreign_bytes);
        assert!(unrelated.is_dir());
        assert!(!home.join("goresave_backups").exists());
    }
    fs::write(&sidecar, b"unavailable optional screenshot").unwrap();
    run(
        home,
        &["report", save_arg, "--out", report.to_str().unwrap()],
    );
    assert!(
        !fs::read_to_string(&report)
            .unwrap()
            .contains("alt=\"Save screenshot\"")
    );
    assert_eq!(gore_save::api::file_sha1(&save).unwrap(), hash);
    assert_eq!(gore_save::api::file_sha1(&claim).unwrap(), hash);
    assert_eq!(fs::read(&profile).unwrap(), profile_bytes);
    assert_eq!(
        fs::read(&sidecar).unwrap(),
        b"unavailable optional screenshot"
    );
    assert_eq!(fs::read(&foreign_sidecar).unwrap(), foreign_bytes);
    assert!(!home.join("goresave_backups").exists());
}

#[test]
fn screenshot_export_reads_only_thumbnail_sidecars_and_preserves_recovery_claims() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path();
    let save = home.join("G1R-001.sav");
    fs::copy(fixture(), &save).unwrap();
    let before = fs::read(&save).unwrap();
    let profile = home.join("PersistentDataList.sav");
    let profile_bytes = profile_fixture("Gothic");
    fs::write(&profile, &profile_bytes).unwrap();
    let claim = home.join("G1R-001.sav.assign-final-goresave-1-2-3");
    fs::write(&claim, &before).unwrap();
    // A whole-directory scan would try to read this unrelated slot and fail.
    fs::create_dir(home.join("G1R-999.sav")).unwrap();
    let jpeg = [0xff, 0xd8, 0xaa, 0xbb, 0xff, 0xd9];
    let screenshot_bytes = screenshot_fixture("G1R-001", &jpeg);
    let sidecar = home.join("Profile_0_Screenshots.sav");
    fs::write(&sidecar, &screenshot_bytes).unwrap();
    for dry_run in [true, false] {
        let out = home.join(if dry_run { "dry.jpg" } else { "live.jpg" });
        let mut args = vec![
            "screenshot",
            "export",
            save.to_str().unwrap(),
            "--out",
            out.to_str().unwrap(),
        ];
        if dry_run {
            args.push("--dry-run");
        }
        let data = run(home, &args);
        assert_eq!(data["byteLength"], jpeg.len());
        assert_eq!(data["mimeType"], "image/jpeg");
        if dry_run {
            assert!(!out.exists());
        } else {
            assert_eq!(fs::read(out).unwrap(), jpeg);
        }
        assert_eq!(fs::read(&save).unwrap(), before);
        assert_eq!(fs::read(&claim).unwrap(), before);
        assert_eq!(fs::read(&profile).unwrap(), profile_bytes);
        assert_eq!(fs::read(&sidecar).unwrap(), screenshot_bytes);
        assert!(home.join("G1R-999.sav").is_dir());
        assert!(!home.join("goresave_backups").exists());
    }
}

#[test]
fn screenshot_and_report_exports_refuse_source_file_aliases_before_truncating() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path();
    let save = home.join("G1R-001.sav");
    fs::copy(fixture(), &save).unwrap();
    let before = fs::read(&save).unwrap();
    let profile = home.join("PersistentDataList.sav");
    let profile_bytes = profile_fixture("Gothic");
    fs::write(&profile, &profile_bytes).unwrap();
    let jpeg = [0xff, 0xd8, 0xaa, 0xbb, 0xff, 0xd9];
    let screenshot_bytes = screenshot_fixture("G1R-001", &jpeg);
    let screenshot = home.join("Profile_0_Screenshots.sav");
    fs::write(&screenshot, &screenshot_bytes).unwrap();
    let hardlink = home.join("save-hardlink.jpg");
    fs::hard_link(&save, &hardlink).unwrap();
    fs::create_dir(home.join("nested")).unwrap();
    let mut aliases = vec![save.clone(), home.join("nested/../G1R-001.sav"), hardlink];
    #[cfg(unix)]
    {
        let link = home.join("save-symlink.jpg");
        std::os::unix::fs::symlink(&save, &link).unwrap();
        aliases.push(link);
    }
    #[cfg(windows)]
    aliases.push(home.join("G1R-001.SAV"));
    for command in [vec!["screenshot", "export"], vec!["report"]] {
        for out in &aliases {
            for dry_run in [false, true] {
                let mut args = command.clone();
                args.extend([save.to_str().unwrap(), "--out", out.to_str().unwrap()]);
                if dry_run {
                    args.push("--dry-run");
                }
                let error = run_failure(home, &args);
                assert!(
                    error
                        .to_string()
                        .contains("must not refer to the source save"),
                    "{error}"
                );
                assert_eq!(fs::read(&save).unwrap(), before);
                assert_eq!(fs::read(out).unwrap(), before);
                assert_eq!(fs::read(&profile).unwrap(), profile_bytes);
                assert_eq!(fs::read(&screenshot).unwrap(), screenshot_bytes);
                assert!(!home.join("goresave_backups").exists());
            }
        }
    }
    let out = home.join("screenshot.jpg");
    let out_arg = out.to_str().unwrap();
    let save_arg = save.to_str().unwrap();
    run(
        home,
        &[
            "screenshot",
            "export",
            save_arg,
            "--out",
            out_arg,
            "--dry-run",
        ],
    );
    assert!(!out.exists());
    let exported = run(home, &["screenshot", "export", save_arg, "--out", out_arg]);
    assert_eq!(exported["byteLength"], jpeg.len());
    assert_eq!(exported["mimeType"], "image/jpeg");
    assert_eq!(fs::read(&out).unwrap(), jpeg);
    fs::write(&out, b"a longer previous export that must be truncated").unwrap();
    run(home, &["screenshot", "export", save_arg, "--out", out_arg]);
    assert_eq!(fs::read(&out).unwrap(), jpeg);
    let report = home.join("report.html");
    fs::write(&report, b"previous report").unwrap();
    run(
        home,
        &[
            "report",
            save_arg,
            "--out",
            report.to_str().unwrap(),
            "--dry-run",
        ],
    );
    assert_eq!(fs::read(&report).unwrap(), b"previous report");
    run(
        home,
        &["report", save_arg, "--out", report.to_str().unwrap()],
    );
    assert!(fs::read_to_string(&report).unwrap().contains("<table>"));
    assert_eq!(fs::read(&save).unwrap(), before);
    assert_eq!(fs::read(&profile).unwrap(), profile_bytes);
    assert_eq!(fs::read(&screenshot).unwrap(), screenshot_bytes);
    assert!(!home.join("goresave_backups").exists());
}

#[test]
fn draft_remove_requires_an_operation_and_preserves_pending_placement_actions() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path();
    let save = home.join("G1R-001.sav");
    let profile = home.join("PersistentDataList.sav");
    let draft = home.join("pending.json");
    fs::copy(fixture(), &save).unwrap();
    fs::write(&profile, b"original profile snapshot").unwrap();
    let save_arg = save.to_str().unwrap();
    let draft_arg = draft.to_str().unwrap();
    run(
        home,
        &[
            "position",
            "set",
            save_arg,
            "--actor",
            "OC_STT_Diego-WP_EZ_START_DIEGO_SPAWN",
            "--x",
            "100",
            "--stay",
            "--draft",
            draft_arg,
        ],
    );
    run(
        home,
        &[
            "rename",
            save_arg,
            "--name",
            "Pending name",
            "--draft",
            draft_arg,
        ],
    );
    let before = fs::read(&draft).unwrap();
    let pending: Value = serde_json::from_slice(&before).unwrap();
    assert_eq!(pending["placementNotes"].as_array().unwrap().len(), 1);
    assert_eq!(pending["syncPersistentDataList"], true);
    let save_before = fs::read(&save).unwrap();
    for mode in [vec![], vec!["--dry-run"], vec!["--operation", "99999"]] {
        let mut args = vec!["draft", "remove", draft_arg];
        args.extend(mode);
        let error = run_failure(home, &args);
        assert!(
            error.to_string().contains("--operation required")
                || error.to_string().contains("operation out of range"),
            "{error}"
        );
        assert_eq!(fs::read(&draft).unwrap(), before);
    }
    let index = pending["edits"]
        .as_array()
        .unwrap()
        .iter()
        .position(|edit| edit["path"] == "public.m_PlayerSaveName")
        .unwrap()
        .to_string();
    let simulated = run(
        home,
        &[
            "draft",
            "remove",
            draft_arg,
            "--operation",
            &index,
            "--dry-run",
        ],
    );
    assert_eq!(
        simulated["edits"].as_array().unwrap().len(),
        pending["edits"].as_array().unwrap().len() - 1
    );
    assert_eq!(simulated["placementNotes"], pending["placementNotes"]);
    assert_eq!(fs::read(&draft).unwrap(), before);
    let removed = run(home, &["draft", "remove", draft_arg, "--operation", &index]);
    assert_eq!(removed, simulated);
    let remaining = fs::read(&draft).unwrap();
    let reset = run(home, &["draft", "reset", draft_arg, "--dry-run"]);
    assert_eq!(reset["edits"], json!([]));
    assert!(reset.get("placementNotes").is_none());
    assert_eq!(fs::read(&draft).unwrap(), remaining);
    assert_eq!(
        run(home, &["draft", "reset", draft_arg])["edits"],
        json!([])
    );
    assert_eq!(fs::read(save).unwrap(), save_before);
    assert_eq!(fs::read(profile).unwrap(), b"original profile snapshot");
    assert!(!home.join("goresave_backups").exists());
}

#[test]
fn all_pages_report_the_count_of_the_collected_rows() {
    let temp = tempfile::tempdir().unwrap();
    let save = temp.path().join("G1R-001.sav");
    fs::copy(fixture(), &save).unwrap();
    let before = fs::read(&save).unwrap();
    let save_arg = save.to_str().unwrap();
    let npcs = run(
        temp.path(),
        &["npc", "list", save_arg, "--all", "--limit", "1000"],
    );
    assert!(npcs["npcs"].as_array().unwrap().len() > 1000);
    assert_eq!(npcs["count"], npcs["npcs"].as_array().unwrap().len());
    assert_eq!(npcs["count"], npcs["total"]);
    let rest = run(
        temp.path(),
        &[
            "npc", "list", save_arg, "--all", "--limit", "1000", "--offset", "1000",
        ],
    );
    assert_eq!(
        rest["npcs"],
        json!(&npcs["npcs"].as_array().unwrap()[1000..])
    );
    assert_eq!(rest["count"], npcs["count"].as_u64().unwrap() - 1000);
    assert_eq!(rest["limit"], rest["count"]);
    assert_eq!(rest["offset"], 1000);
    let empty = run(
        temp.path(),
        &["npc", "list", save_arg, "--all", "--offset", "99999"],
    );
    assert_eq!(empty["count"], 0);
    assert_eq!(empty["limit"], 0);
    assert_eq!(empty["npcs"], json!([]));
    let data = run(
        temp.path(),
        &[
            "data", "search", save_arg, "--query", "m_Game", "--all", "--limit", "2",
        ],
    );
    assert!(data["results"].as_array().unwrap().len() > 2);
    assert_eq!(data["count"], data["results"].as_array().unwrap().len());
    assert_eq!(data["limit"], data["count"]);
    assert_eq!(data["total"], data["count"]);
    assert_eq!(data["offset"], 0);
    assert_eq!(fs::read(save).unwrap(), before);
    assert!(!temp.path().join("goresave_backups").exists());
}

#[test]
fn trader_stock_set_requires_an_explicit_count_without_writing_or_staging() {
    let temp = tempfile::tempdir().unwrap();
    let save = temp.path().join("G1R-001.sav");
    fs::copy(fixture(), &save).unwrap();
    let before = fs::read(&save).unwrap();
    let save = save.to_str().unwrap();
    let draft = temp.path().join("pending.json");
    let missing_draft = temp.path().join("missing.json");
    run(
        temp.path(),
        &[
            "rename",
            save,
            "--name",
            "Pending name",
            "--draft",
            draft.to_str().unwrap(),
        ],
    );
    let pending = fs::read(&draft).unwrap();
    for map in ["current", "default"] {
        for mode in [
            vec![],
            vec!["--dry-run"],
            vec!["--draft", draft.to_str().unwrap()],
            vec!["--draft", missing_draft.to_str().unwrap()],
        ] {
            let mut args = vec![
                "traders",
                "stock",
                "set",
                save,
                "--index",
                "1",
                "--map",
                map,
                "--item",
                "ItMi_Orenugget",
            ];
            args.extend(mode);
            let error = run_failure(temp.path(), &args);
            assert!(error.to_string().contains("--count required"), "{error}");
            assert_eq!(fs::read(save).unwrap(), before);
            assert_eq!(fs::read(&draft).unwrap(), pending);
            assert!(!missing_draft.exists());
            assert!(!temp.path().join("goresave_backups").exists());
        }
    }
    let args = [
        "traders",
        "stock",
        "set",
        save,
        "--index",
        "1",
        "--item",
        "ItMi_Orenugget",
        "--count",
        "13",
    ];
    let mut simulated = args.to_vec();
    simulated.push("--dry-run");
    run(temp.path(), &simulated);
    assert_eq!(fs::read(save).unwrap(), before);
    assert!(!temp.path().join("goresave_backups").exists());
    run(temp.path(), &args);
    let trader = run(temp.path(), &["traders", "show", save, "--index", "1"]);
    assert_eq!(trader["ore"], 13);
    assert_eq!(
        trader["defaultItems"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["id"] == "ItMi_Orenugget")
            .unwrap()["count"],
        18
    );
    run(
        temp.path(),
        &[
            "traders",
            "stock",
            "add",
            save,
            "--index",
            "1",
            "--item",
            "ItFo_Loaf",
        ],
    );
    let trader = run(temp.path(), &["traders", "show", save, "--index", "1"]);
    assert_eq!(
        trader["items"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["id"] == "ItFo_Loaf")
            .unwrap()["count"],
        1
    );
    assert_eq!(fs::read(draft).unwrap(), pending);
}

#[test]
fn character_roles_include_every_catalog_assignment_before_pagination() {
    let temp = tempfile::tempdir().unwrap();
    let save = temp.path().join("G1R-001.sav");
    fs::copy(fixture(), &save).unwrap();
    let before = fs::read(&save).unwrap();
    let save = save.to_str().unwrap();
    let all = run(temp.path(), &["characters", "list", save, "--all"]);
    let rows = all["characters"].as_array().unwrap();
    let armorers = run(
        temp.path(),
        &["characters", "list", save, "--role", "armorer", "--all"],
    );
    let selected = armorers["characters"].as_array().unwrap();
    let expected_names = std::collections::BTreeSet::from([
        "nc_org_wolf_855",
        "nc_sld_torlof_737",
        "oc_stt_fisk_311",
        "ocr_grd_stone_219",
        "sc_gur_baalnamib_1204",
        "sc_nov_darrion_1312",
        "sc_tpl_gornatoth_1402",
    ]);
    assert_eq!(selected.len(), 7);
    assert_eq!(armorers["total"], 7);
    assert_eq!(
        selected
            .iter()
            .map(|row| row["uniqueName"].as_str().unwrap().to_lowercase())
            .collect::<std::collections::BTreeSet<_>>(),
        expected_names.into_iter().map(String::from).collect()
    );
    let wolf = selected
        .iter()
        .find(|row| row["uniqueName"] == "NC_ORG_Wolf_855")
        .unwrap();
    assert_eq!(
        wolf["roles"],
        json!(["armorer", "dead", "portrait", "teacher", "trader"])
    );
    assert_eq!(wolf["teacher"], true);
    assert_eq!(wolf["isDead"], false);
    assert!(
        rows.iter()
            .filter(|row| row["uniqueName"] == "Wolf")
            .all(|row| row["roles"] == json!([]))
    );
    assert_eq!(
        run(
            temp.path(),
            &["characters", "show", save, "--id", "NC_ORG_Wolf_855"]
        )["roles"],
        wolf["roles"]
    );
    let page = run(
        temp.path(),
        &[
            "characters",
            "list",
            save,
            "--role",
            "armorer",
            "--offset",
            "2",
            "--limit",
            "2",
        ],
    );
    assert_eq!(page["total"], 7);
    assert_eq!(page["count"], 2);
    assert_eq!(page["offset"], 2);
    assert_eq!(page["characters"], json!(selected[2..4]));
    for role in ["dead", "portrait", "teacher", "trader"] {
        let filtered = run(
            temp.path(),
            &["characters", "list", save, "--role", role, "--all"],
        );
        let expected: Vec<_> = rows
            .iter()
            .filter(|row| match role {
                "teacher" => row["teacher"] == true,
                "trader" => row["isTrader"] == true,
                _ => row["roles"].as_array().unwrap().contains(&json!(role)),
            })
            .cloned()
            .collect();
        assert!(!expected.is_empty());
        assert_eq!(filtered["characters"], json!(expected));
    }
    assert_eq!(fs::read(save).unwrap(), before);
    assert!(!temp.path().join("goresave_backups").exists());
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
fn dictionary_catalog_queries_and_pages_work_through_the_cli() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path();
    for domain in [
        "story-semantics",
        "glossary-text",
        "portraits",
        "hero-attributes",
    ] {
        let all = run(home, &["catalog", "list", "--kind", domain, "--all"]);
        let entries = all["entries"].as_array().unwrap();
        assert!(entries.len() > 1, "{domain}");
        assert_eq!(all["total"], entries.len());
        let page = run(
            home,
            &[
                "catalog", "list", "--kind", domain, "--offset", "1", "--limit", "1",
            ],
        );
        assert_eq!(page["entries"], json!([entries[1].clone()]));
        assert_eq!(page["total"], entries.len());
        assert_eq!(page["count"], 1);
        assert_eq!(page["offset"], 1);
        let empty = run(
            home,
            &[
                "catalog",
                "search",
                "--kind",
                domain,
                "--query",
                "PR123_NO_SUCH_CATALOG_ENTRY_987654",
                "--limit",
                "1",
            ],
        );
        assert_eq!(empty["entries"], json!([]), "{domain}");
        assert_eq!(empty["total"], 0, "{domain}");
        let id = entries[0]["id"].as_str().unwrap();
        let selected = run(home, &["catalog", "list", "--kind", domain, "--id", id]);
        let expected = entries
            .iter()
            .filter(|entry| {
                entry["id"]
                    .as_str()
                    .is_some_and(|value| value.eq_ignore_ascii_case(id))
            })
            .cloned()
            .collect::<Vec<_>>();
        assert_eq!(selected["entries"], json!(expected));
        assert_eq!(selected["total"], expected.len());
    }
}

#[test]
fn draft_output_overrides_preserve_source_and_validate_before_publication() {
    for stored_output in [false, true] {
        let temp = tempfile::tempdir().unwrap();
        let home = temp.path();
        let source = home.join("G1R-001.sav");
        fs::copy(fixture(), &source).unwrap();
        let before = fs::read(&source).unwrap();
        let draft = home.join("draft.json");
        let staged_output = home.join("staged.sav");
        let output = home.join("override.sav");
        let mut args = vec![
            "attributes",
            "set",
            source.to_str().unwrap(),
            "--attribute",
            "Strength",
            "--current",
            "41",
            "--draft",
            draft.to_str().unwrap(),
        ];
        if stored_output {
            args.extend(["--out", staged_output.to_str().unwrap()]);
        }
        run(home, &args);
        let draft_before = fs::read(&draft).unwrap();
        for invalid in [
            source.clone(),
            draft.clone(),
            home.join("missing/output.sav"),
        ] {
            for mode in ["validate", "apply"] {
                run_failure(
                    home,
                    &[
                        "draft",
                        mode,
                        draft.to_str().unwrap(),
                        "--out",
                        invalid.to_str().unwrap(),
                    ],
                );
                assert_eq!(fs::read(&source).unwrap(), before);
                assert_eq!(fs::read(&draft).unwrap(), draft_before);
                assert!(!staged_output.exists());
                assert!(!home.join("goresave_backups").exists());
            }
        }
        run(
            home,
            &[
                "draft",
                "validate",
                draft.to_str().unwrap(),
                "--out",
                output.to_str().unwrap(),
            ],
        );
        run(
            home,
            &[
                "draft",
                "apply",
                draft.to_str().unwrap(),
                "--out",
                output.to_str().unwrap(),
                "--dry-run",
            ],
        );
        assert_eq!(fs::read(&source).unwrap(), before);
        assert_eq!(fs::read(&draft).unwrap(), draft_before);
        assert!(!output.exists());
        assert!(!staged_output.exists());
        let result = run(
            home,
            &[
                "draft",
                "apply",
                draft.to_str().unwrap(),
                "--out",
                output.to_str().unwrap(),
            ],
        );
        assert_eq!(result["complete"], true);
        assert_eq!(result["draftUpdated"], true);
        let exported = execute_core("inspect_save", json!({"path":output,"includePrivate":true}));
        let attributes = &exported["private"]["player"]["attributes"];
        assert_eq!(
            attributes
                .as_array()
                .unwrap()
                .iter()
                .find(|row| row["id"] == "Strength")
                .unwrap()["currentValue"],
            41.0
        );
        assert_eq!(fs::read(&source).unwrap(), before);
        assert!(!staged_output.exists());
        let saved_draft = run(home, &["draft", "show", draft.to_str().unwrap()]);
        assert_eq!(saved_draft["path"], json!(output.canonicalize().unwrap()));
        assert_eq!(
            saved_draft["expectedSha1"],
            gore_save::api::file_sha1(&output).unwrap()
        );
        assert_eq!(saved_draft["edits"], json!([]));
        assert!(saved_draft["outputPath"].is_null());
    }
}

#[test]
fn edited_save_previews_validate_output_parents_and_preserve_drafts() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path();
    let save = home.join("G1R-001.sav");
    fs::copy(fixture(), &save).unwrap();
    let before = fs::read(&save).unwrap();
    let profile = home.join("PersistentDataList.sav");
    let profile_bytes = profile_fixture("Gothic");
    fs::write(&profile, &profile_bytes).unwrap();
    let file_parent = home.join("file-parent");
    fs::write(&file_parent, b"parent bytes").unwrap();
    let draft = home.join("export.json");
    for out in [
        home.join("missing/child.sav"),
        home.join("missing/nested/child.sav"),
        file_parent.join("child.sav"),
        home.to_path_buf(),
    ] {
        for command in ["apply_edits", "write_save"] {
            for dry in [false, true] {
                assert!(
                    gore_save::api::execute(&gore_save::api::Request {
                        command: command.into(),
                        payload: json!({"path":save,"outputPath":out,"dryRun":dry,
                            "edits":[{"path":"public.m_PlayerSaveName","value":"Export name"}]}),
                    })
                    .is_err(),
                    "{command} dry={dry} out={out:?}"
                );
            }
        }
        for mode in [
            vec![],
            vec!["--dry-run"],
            vec!["--draft", draft.to_str().unwrap()],
            vec!["--draft", draft.to_str().unwrap(), "--dry-run"],
        ] {
            let mut args = vec![
                "rename",
                save.to_str().unwrap(),
                "--name",
                "Export name",
                "--out",
                out.to_str().unwrap(),
            ];
            args.extend(mode);
            run_failure(home, &args);
            assert!(!draft.exists());
            assert!(!home.join("missing").exists());
            assert!(!home.join("goresave_backups").exists());
            assert_eq!(fs::read(&save).unwrap(), before);
            assert_eq!(fs::read(&profile).unwrap(), profile_bytes);
            assert_eq!(fs::read(&file_parent).unwrap(), b"parent bytes");
        }
    }

    let output_parent = home.join("existing-parent");
    fs::create_dir(&output_parent).unwrap();
    let output = output_parent.join("export.sav");
    run(
        home,
        &[
            "rename",
            save.to_str().unwrap(),
            "--name",
            "Export name",
            "--out",
            output.to_str().unwrap(),
            "--draft",
            draft.to_str().unwrap(),
        ],
    );
    let draft_bytes = fs::read(&draft).unwrap();
    assert!(!output.exists());
    fs::remove_dir(&output_parent).unwrap();
    for args in [
        vec!["draft", "validate", draft.to_str().unwrap()],
        vec!["draft", "apply", draft.to_str().unwrap()],
        vec!["draft", "apply", draft.to_str().unwrap(), "--dry-run"],
    ] {
        let error = run_failure(home, &args);
        assert!(error.to_string().contains("outputPath parent"), "{error}");
        assert_eq!(fs::read(&draft).unwrap(), draft_bytes);
        assert!(!output_parent.exists());
        assert!(!home.join("goresave_backups").exists());
        assert_eq!(fs::read(&save).unwrap(), before);
        assert_eq!(fs::read(&profile).unwrap(), profile_bytes);
    }
    fs::create_dir(&output_parent).unwrap();
    run(home, &["draft", "validate", draft.to_str().unwrap()]);
    assert_eq!(fs::read(&draft).unwrap(), draft_bytes);
    assert!(!output.exists());
    run(home, &["draft", "apply", draft.to_str().unwrap()]);
    assert_eq!(
        run(home, &["inspect", output.to_str().unwrap()])["public"]["playerSaveName"],
        "Export name"
    );
    run_from(
        home,
        Some(home),
        &[
            "rename",
            "G1R-001.sav",
            "--name",
            "Relative export",
            "--out",
            "relative.sav",
            "--dry-run",
        ],
    );
    assert!(!home.join("relative.sav").exists());
    run_from(
        home,
        Some(home),
        &[
            "rename",
            "G1R-001.sav",
            "--name",
            "Relative export",
            "--out",
            "relative.sav",
        ],
    );
    assert_eq!(
        run(
            home,
            &["inspect", home.join("relative.sav").to_str().unwrap()]
        )["public"]["playerSaveName"],
        "Relative export"
    );
    assert_eq!(fs::read(&save).unwrap(), before);
    assert_eq!(fs::read(&profile).unwrap(), profile_bytes);
    assert!(!home.join("goresave_backups").exists());
}

#[test]
fn edited_save_exports_refuse_source_aliases_without_writing_or_staging() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path();
    let save = home.join("G1R-001.sav");
    fs::copy(fixture(), &save).unwrap();
    let before = fs::read(&save).unwrap();
    let profile = home.join("PersistentDataList.sav");
    let profile_bytes = profile_fixture("Gothic");
    fs::write(&profile, &profile_bytes).unwrap();
    let hardlink = home.join("alias.sav");
    fs::hard_link(&save, &hardlink).unwrap();
    let mut aliases = vec![save.clone(), home.join("./G1R-001.sav"), hardlink];
    #[cfg(unix)]
    {
        let link = home.join("symlink.sav");
        std::os::unix::fs::symlink(&save, &link).unwrap();
        aliases.push(link);
    }
    #[cfg(windows)]
    aliases.push(home.join("G1R-001.SAV"));
    let draft = home.join("export.json");
    for out in &aliases {
        for mode in [
            vec![],
            vec!["--dry-run"],
            vec!["--draft", draft.to_str().unwrap()],
        ] {
            let mut args = vec![
                "rename",
                save.to_str().unwrap(),
                "--name",
                "Export name",
                "--out",
                out.to_str().unwrap(),
            ];
            args.extend(mode);
            let error = run_failure(home, &args);
            assert!(
                error
                    .to_string()
                    .contains("must not refer to the source save"),
                "{error}"
            );
            assert_eq!(fs::read(&save).unwrap(), before);
            assert_eq!(fs::read(out).unwrap(), before);
            assert_eq!(fs::read(&profile).unwrap(), profile_bytes);
            assert!(!draft.exists());
            assert!(!home.join("goresave_backups").exists());
        }
    }
    let out = home.join("export.sav");
    run(
        home,
        &[
            "rename",
            save.to_str().unwrap(),
            "--name",
            "Export name",
            "--out",
            out.to_str().unwrap(),
            "--dry-run",
        ],
    );
    assert!(!out.exists());
    run(
        home,
        &[
            "rename",
            save.to_str().unwrap(),
            "--name",
            "Export name",
            "--out",
            out.to_str().unwrap(),
        ],
    );
    assert_eq!(
        run(home, &["inspect", out.to_str().unwrap()])["public"]["playerSaveName"],
        "Export name"
    );
    assert_eq!(fs::read(save).unwrap(), before);
    assert_eq!(fs::read(profile).unwrap(), profile_bytes);
    assert!(!home.join("goresave_backups").exists());
}

#[test]
fn difficulty_dry_runs_remap_nested_profile_paths_and_preserve_live_files() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path();
    let save = home.join("G1R-001.sav");
    fs::copy(fixture(), &save).unwrap();
    let before = fs::read(&save).unwrap();
    let profile = home.join("PersistentDataList.sav");
    let profile_bytes = profile_fixture("Gothic");
    fs::write(&profile, &profile_bytes).unwrap();
    for directory in [None, Some(home)] {
        let input = if directory.is_some() {
            "G1R-001.sav"
        } else {
            save.to_str().unwrap()
        };
        let preview = run_from(
            home,
            directory,
            &[
                "difficulty",
                "set",
                input,
                "--profile",
                "0",
                "--preset",
                "Hard",
                "--dry-run",
            ],
        );
        assert_eq!(preview["validated"], true);
        assert_eq!(preview["request"]["profile"]["profileId"], 0);
        assert_eq!(preview["request"]["difficulty"]["preset"], "Hard");
        assert_eq!(fs::read(&profile).unwrap(), profile_bytes);
        assert_eq!(fs::read(&save).unwrap(), before);
        assert!(!home.join("goresave_backups").exists());
        assert_eq!(
            run(
                home,
                &[
                    "difficulty",
                    "show",
                    save.to_str().unwrap(),
                    "--profile",
                    "0"
                ]
            )["difficultyPreset"],
            "/Script/Angelscript.DifficultyPreset_Gothic"
        );
    }
    run(
        home,
        &[
            "difficulty",
            "set",
            save.to_str().unwrap(),
            "--profile",
            "0",
            "--preset",
            "Hard",
        ],
    );
    assert_ne!(fs::read(&profile).unwrap(), profile_bytes);
    assert_eq!(fs::read(&save).unwrap(), before);
    assert_eq!(
        run(
            home,
            &[
                "difficulty",
                "show",
                save.to_str().unwrap(),
                "--profile",
                "0"
            ]
        )["difficultyPreset"],
        "/Script/Angelscript.DifficultyPreset_Hard"
    );
    assert!(home.join("goresave_backups").exists());
}

#[test]
fn administrative_dry_runs_keep_slot_values_when_launched_from_the_save_directory() {
    for operation in [vec!["profile", "detach"], vec!["delete"]] {
        let temp = tempfile::tempdir().unwrap();
        let home = temp.path();
        let save = home.join("G1R-001.sav");
        fs::copy(fixture(), &save).unwrap();
        let before = fs::read(&save).unwrap();
        let profile = home.join("PersistentDataList.sav");
        let profile_bytes = profile_fixture("Gothic");
        fs::write(&profile, &profile_bytes).unwrap();
        let mut args = operation.clone();
        args.extend(["G1R-001.sav", "--profile", "0"]);
        let mut dry = args.clone();
        dry.push("--dry-run");
        let preview = run_from(home, Some(home), &dry);
        assert_eq!(preview["validated"], true);
        assert_eq!(preview["request"]["slot"], "G1R-001");
        assert_eq!(preview["simulation"]["slot"], "G1R-001");
        assert_eq!(fs::read(&save).unwrap(), before);
        assert_eq!(fs::read(&profile).unwrap(), profile_bytes);
        assert!(!home.join("goresave_backups").exists());
        let live = run_from(home, Some(home), &args);
        assert_eq!(live["slot"], "G1R-001");
        assert_ne!(fs::read(&profile).unwrap(), profile_bytes);
        assert_eq!(save.exists(), operation[0] != "delete");
    }
}

#[test]
fn refresh_marks_only_changed_profile_dependencies_as_stale() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path();
    let save = home.join("G1R-001.sav");
    fs::copy(fixture(), &save).unwrap();
    let hash = gore_save::api::file_sha1(&save).unwrap();
    let profile = home.join("PersistentDataList.sav");
    let profile_bytes = profile_fixture("Hard");
    fs::write(&profile, &profile_bytes).unwrap();
    let renamed = home.join("rename.json");
    let derived = home.join("derived.json");
    let explicit = home.join("explicit.json");
    for (file, mut args) in [
        (
            &renamed,
            vec!["rename", save.to_str().unwrap(), "--name", "Staged name"],
        ),
        (&derived, vec!["inventory", "reset", save.to_str().unwrap()]),
        (
            &explicit,
            vec![
                "inventory",
                "reset",
                save.to_str().unwrap(),
                "--resources-level",
                "Novice",
            ],
        ),
    ] {
        args.extend(["--draft", file.to_str().unwrap()]);
        run(home, &args);
    }
    let snapshots = [&renamed, &derived, &explicit].map(|file| (file, fs::read(file).unwrap()));
    let check = |file: &Path, stale: bool| {
        for verb in ["refresh", "list"] {
            let result = run(
                home,
                &[
                    verb,
                    "--root",
                    home.to_str().unwrap(),
                    "--draft",
                    file.to_str().unwrap(),
                ],
            );
            assert_eq!(result["draft"]["stale"], stale, "{verb}: {file:?}");
            assert_eq!(result["draft"]["pending"], 1);
            assert_eq!(result["draft"]["preserved"], true);
        }
    };
    for (file, _) in &snapshots {
        check(file, false);
    }
    let changed = profile_fixture("Easy");
    fs::write(&profile, &changed).unwrap();
    for (file, expected) in [(&renamed, true), (&derived, true), (&explicit, false)] {
        check(file, expected);
        if expected {
            let error = run_failure(home, &["draft", "validate", file.to_str().unwrap()]);
            assert!(error.to_string().contains("profile changed"), "{error}");
        } else {
            run(home, &["draft", "validate", file.to_str().unwrap()]);
        }
    }
    assert_eq!(fs::read(&profile).unwrap(), changed);
    fs::write(&profile, &profile_bytes).unwrap();
    for (file, _) in &snapshots {
        check(file, false);
    }
    fs::remove_file(&profile).unwrap();
    for (file, expected) in [(&renamed, true), (&derived, true), (&explicit, false)] {
        check(file, expected);
    }
    assert!(!profile.exists());
    for (file, bytes) in snapshots {
        assert_eq!(fs::read(file).unwrap(), bytes);
    }
    assert_eq!(gore_save::api::file_sha1(&save).unwrap(), hash);
    assert!(!home.join("goresave_backups").exists());
}

#[test]
fn profile_derived_reset_drafts_reject_profile_changes_and_preserve_pending_edits() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path();
    let save = home.join("G1R-001.sav");
    fs::copy(fixture(), &save).unwrap();
    let before = fs::read(&save).unwrap();
    let profile = home.join("PersistentDataList.sav");
    fs::write(&profile, profile_fixture("Hard")).unwrap();
    let draft = home.join("derived.json");
    run(
        home,
        &[
            "inventory",
            "reset",
            save.to_str().unwrap(),
            "--draft",
            draft.to_str().unwrap(),
        ],
    );
    let pending = fs::read(&draft).unwrap();
    let data: Value = serde_json::from_slice(&pending).unwrap();
    assert_eq!(data["edits"][0]["value"]["resourcesLevel"], "Hard");
    assert_eq!(
        data["edits"][0]["value"]["expectedPersistentSha1"],
        gore_save::api::file_sha1(&profile).unwrap()
    );
    let explicit = home.join("explicit.json");
    run(
        home,
        &[
            "inventory",
            "reset",
            save.to_str().unwrap(),
            "--resources-level",
            "Novice",
            "--draft",
            explicit.to_str().unwrap(),
        ],
    );
    let changed = profile_fixture("Easy");
    fs::write(&profile, &changed).unwrap();
    for verb in ["validate", "apply"] {
        let error = run_failure(home, &["draft", verb, draft.to_str().unwrap()]);
        assert!(error.to_string().contains("profile changed"), "{error}");
        assert_eq!(fs::read(&draft).unwrap(), pending);
        assert_eq!(fs::read(&save).unwrap(), before);
        assert_eq!(fs::read(&profile).unwrap(), changed);
        assert!(!home.join("goresave_backups").exists());
    }
    run(home, &["draft", "validate", explicit.to_str().unwrap()]);
    run(home, &["draft", "apply", explicit.to_str().unwrap()]);
    let novice = fixture().with_file_name("resources_novice.sav");
    assert_eq!(
        run(
            home,
            &["inventory", "list", save.to_str().unwrap(), "--all"]
        )["items"],
        run(
            home,
            &["inventory", "list", novice.to_str().unwrap(), "--all"]
        )["items"]
    );
    assert_eq!(fs::read(&profile).unwrap(), changed);
    assert_eq!(fs::read(&draft).unwrap(), pending);

    let combined = home.join("combined");
    fs::create_dir(&combined).unwrap();
    let source = combined.join("G1R-001.sav");
    fs::copy(fixture(), &source).unwrap();
    let source_profile = combined.join("PersistentDataList.sav");
    fs::write(&source_profile, profile_fixture("Hard")).unwrap();
    let combined_draft = home.join("combined.json");
    run(
        home,
        &[
            "rename",
            source.to_str().unwrap(),
            "--name",
            "Combined rename",
            "--draft",
            combined_draft.to_str().unwrap(),
        ],
    );
    run(
        home,
        &[
            "inventory",
            "reset",
            source.to_str().unwrap(),
            "--draft",
            combined_draft.to_str().unwrap(),
        ],
    );
    let applied = run(home, &["draft", "apply", combined_draft.to_str().unwrap()]);
    assert_eq!(applied["complete"], true);
    assert_eq!(applied["committed"], json!([0, 1]));
    assert_eq!(
        run(home, &["inspect", source.to_str().unwrap()])["persistent"]["playerSaveName"],
        "Combined rename"
    );
    let source_bytes = fs::read(&source).unwrap();
    let profile_bytes = fs::read(&source_profile).unwrap();
    let export_dir = home.join("exports");
    fs::create_dir(&export_dir).unwrap();
    let exported = export_dir.join("G1R-001.sav");
    let export_draft = home.join("combined-export.json");
    run(
        home,
        &[
            "rename",
            source.to_str().unwrap(),
            "--name",
            "Exported combined",
            "--out",
            exported.to_str().unwrap(),
            "--draft",
            export_draft.to_str().unwrap(),
        ],
    );
    run(
        home,
        &[
            "inventory",
            "reset",
            source.to_str().unwrap(),
            "--draft",
            export_draft.to_str().unwrap(),
        ],
    );
    assert_eq!(
        run(home, &["draft", "apply", export_draft.to_str().unwrap()])["complete"],
        true
    );
    assert_eq!(
        run(home, &["inspect", exported.to_str().unwrap()])["public"]["playerSaveName"],
        "Exported combined"
    );
    assert_eq!(fs::read(&source).unwrap(), source_bytes);
    assert_eq!(fs::read(&source_profile).unwrap(), profile_bytes);
    assert!(!export_dir.join("PersistentDataList.sav").exists());
    assert!(!export_dir.join("goresave_backups").exists());
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
fn staging_npc_pose_back_to_live_replaces_pending_location_and_rotation() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path();
    let save = home.join("G1R-001.sav");
    fs::copy(fixture(), &save).unwrap();
    let save_arg = save.to_str().unwrap();
    let actor = "OC_STT_Diego-WP_EZ_START_DIEGO_SPAWN";
    let original = run(home, &["position", "show", save_arg, "--actor", actor]);
    let original_bytes = fs::read(&save).unwrap();
    let x = original["pose"]["location"]["x"].as_f64().unwrap();
    let yaw = original["pose"]["rotation"]["yaw"].as_f64().unwrap();
    let draft = home.join("reverted-pose.json");
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
            "--yaw",
            &(yaw + 15.0).to_string(),
            "--draft",
            draft_arg,
        ],
    );
    let before_preview = fs::read(&draft).unwrap();
    run(
        home,
        &[
            "position",
            "set",
            save_arg,
            "--actor",
            actor,
            "--x",
            &x.to_string(),
            "--draft",
            draft_arg,
            "--dry-run",
        ],
    );
    assert_eq!(fs::read(&draft).unwrap(), before_preview);
    for (flag, value) in [("--x", x), ("--yaw", yaw)] {
        run(
            home,
            &[
                "position",
                "set",
                save_arg,
                "--actor",
                actor,
                flag,
                &value.to_string(),
                "--draft",
                draft_arg,
            ],
        );
    }
    let staged = run(home, &["draft", "show", draft_arg]);
    for (leaf, path) in [("location", "locationPath"), ("rotation", "rotationPath")] {
        let edit = staged["edits"]
            .as_array()
            .unwrap()
            .iter()
            .find(|edit| edit["value"]["path"] == original["pose"][path])
            .unwrap();
        assert_eq!(edit["value"]["value"], original["pose"][leaf]);
    }
    assert_eq!(fs::read(&save).unwrap(), original_bytes);
    assert_eq!(run(home, &["draft", "apply", draft_arg])["complete"], true);
    let applied = run(home, &["position", "show", save_arg, "--actor", actor]);
    assert_eq!(applied["pose"], original["pose"]);
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
fn repeated_player_fields_refuse_publication_and_allow_disjoint_components() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path();
    let save = home.join("G1R-001.sav");
    fs::copy(fixture(), &save).unwrap();
    let original = fs::read(&save).unwrap();
    let hash = gore_save::api::file_sha1(&save).unwrap();
    let draft = home.join("overlap.json");
    let request = home.join("request.json");
    let base =
        json!({"path":"private.player.setAttribute","value":{"id":"Strength","baseValue":20}});
    let current =
        json!({"path":"private.player.setAttribute","value":{"id":"Strength","currentValue":21}});
    let location =
        json!({"path":"private.player.setTransform","value":{"location":{"x":1,"y":2,"z":3}}});
    let rotation = json!({"path":"private.player.setTransform","value":{"rotation":{"pitch":4,"yaw":5,"roll":6}}});
    let pairs = [
        (
            json!({"path":"private.player.setPlayerName","value":"First player"}),
            json!({"path":"private.player.setPlayerName","value":{"name":"Second player"}}),
        ),
        (
            json!({"path":"private.profile.setProfileName","value":"First profile"}),
            json!({"path":"private.profile.setProfileName","value":{"name":"Second profile"}}),
        ),
        (
            base.clone(),
            json!({"path":"private.player.setAttribute","value":{"id":"Strength","value":22}}),
        ),
        (
            current.clone(),
            json!({"path":"private.player.setAttribute","value":{"id":"Strength","currentValue":23}}),
        ),
        (
            location.clone(),
            json!({"path":"private.player.setTransform","value":{"location":{"x":7,"y":8,"z":9}}}),
        ),
        (
            rotation.clone(),
            json!({"path":"private.player.setTransform","value":{"location":{"x":7,"y":8,"z":9},"rotation":{"pitch":10,"yaw":11,"roll":12}}}),
        ),
    ];
    for (first, second) in pairs {
        // Start-save resources omit private user names. Valid name writes and
        // duplicate refusal use the core's existing synthetic name fixtures.
        if first["path"] != "private.player.setPlayerName"
            && first["path"] != "private.profile.setProfileName"
        {
            for edit in [&first, &second] {
                execute_core(
                    "apply_edits",
                    json!({"path":save,"edits":[edit],"dryRun":true}),
                );
                assert_eq!(fs::read(&save).unwrap(), original);
            }
        }
        for edits in [vec![first.clone(), second.clone()], vec![second, first]] {
            for command in ["plan_edits", "write_save", "apply_edits"] {
                for dry in [false, true] {
                    let payload = json!({"path":save,"edits":edits,"dryRun":dry,"backup":true});
                    let error = gore_save::api::execute(&gore_save::api::Request {
                        command: command.into(),
                        payload: payload.clone(),
                    })
                    .unwrap_err();
                    assert!(
                        matches!(
                            error,
                            gore_save::CoreError::PlanConflict {
                                kind: "property",
                                ..
                            } | gore_save::CoreError::UnsupportedEdit(_)
                        ),
                        "{error}"
                    );
                    if command == "apply_edits" {
                        fs::write(
                            &request,
                            serde_json::to_vec(&json!({"command":command,"payload":payload}))
                                .unwrap(),
                        )
                        .unwrap();
                        let error = run_failure(
                            home,
                            &["core", "exec", "--request-file", request.to_str().unwrap()],
                        );
                        assert_eq!(error["code"], "PLAN_CONFLICT", "{error}");
                    }
                    assert_eq!(fs::read(&save).unwrap(), original);
                    assert!(!home.join("goresave_backups").exists());
                }
            }
            let pending = serde_json::to_vec(&json!({"format":"gore.save.draft.v1","path":save.canonicalize().unwrap(),"expectedSha1":hash,"edits":edits})).unwrap();
            fs::write(&draft, &pending).unwrap();
            for flags in [vec!["validate"], vec!["apply"], vec!["apply", "--dry-run"]] {
                let mut args = vec!["draft", flags[0], draft.to_str().unwrap()];
                args.extend_from_slice(&flags[1..]);
                let error = run_failure(home, &args);
                assert!(
                    error.to_string().contains("pending edit conflict"),
                    "{error}"
                );
                assert_eq!(fs::read(&draft).unwrap(), pending);
                assert_eq!(fs::read(&save).unwrap(), original);
                assert!(!home.join("goresave_backups").exists());
            }
        }
    }
    let result = execute_core(
        "apply_edits",
        json!({"path":save,"edits":[base,current,location,rotation],"backup":false}),
    );
    assert_eq!(result["complete"], true, "{result}");
    assert_eq!(result["committed"], json!([0, 1, 2, 3]));
    let inspect = execute_core("inspect_save", json!({"path":save,"includePrivate":true}));
    let attributes = inspect["private"]["player"]["attributes"]
        .as_array()
        .unwrap();
    let strength = attributes
        .iter()
        .find(|row| row["id"] == "Strength")
        .unwrap();
    assert_eq!(strength["baseValue"], 20.0);
    assert_eq!(strength["currentValue"], 21.0);
    let transform = &inspect["private"]["player"]["transform"];
    assert_eq!(transform["location"], json!({"x":1.0,"y":2.0,"z":3.0}));
    assert_eq!(
        transform["rotation"],
        json!({"pitch":4.0,"yaw":5.0,"roll":6.0})
    );
    assert!(!home.join("goresave_backups").exists());
}

#[test]
fn duplicate_public_renames_preserve_save_profile_and_manual_drafts() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path();
    let save = home.join("G1R-001.sav");
    fs::copy(fixture(), &save).unwrap();
    let profile = home.join("PersistentDataList.sav");
    let profile_bytes = profile_fixture("Gothic");
    fs::write(&profile, &profile_bytes).unwrap();
    let hash = gore_save::api::file_sha1(&save).unwrap();
    let unchanged = || {
        assert_eq!(gore_save::api::file_sha1(&save).unwrap(), hash);
        assert_eq!(fs::read(&profile).unwrap(), profile_bytes);
        assert!(!home.join("goresave_backups").exists());
    };
    let first = json!({"path":"public.m_PlayerSaveName","value":"First name"});
    let second = json!({"path":"public.m_PlayerSaveName","value":"Second name"});
    for edit in [&first, &second] {
        execute_core(
            "apply_edits",
            json!({"path":save,"edits":[edit],"syncPersistentDataList":true,"dryRun":true}),
        );
        unchanged();
    }
    let draft = home.join("duplicate.json");
    let request = home.join("request.json");
    for edits in [
        vec![first.clone(), second.clone()],
        vec![second.clone(), first.clone()],
        vec![first.clone(), first],
    ] {
        for command in ["plan_edits", "write_save", "apply_edits"] {
            for dry in [false, true] {
                let payload = json!({"path":save,"expectedSha1":hash,"edits":edits,"syncPersistentDataList":true,"dryRun":dry,"backup":true});
                let error = gore_save::api::execute(&gore_save::api::Request {
                    command: command.into(),
                    payload: payload.clone(),
                })
                .unwrap_err();
                assert!(
                    matches!(error,gore_save::CoreError::PlanConflict {kind:"property",ref path} if path=="public › m_PlayerSaveName"),
                    "{error}"
                );
                unchanged();
                if command == "apply_edits" {
                    fs::write(
                        &request,
                        serde_json::to_vec(&json!({"command":command,"payload":payload})).unwrap(),
                    )
                    .unwrap();
                    let error = run_failure(
                        home,
                        &["core", "exec", "--request-file", request.to_str().unwrap()],
                    );
                    assert_eq!(error["code"], "PLAN_CONFLICT", "{error}");
                    unchanged();
                }
            }
        }
        let pending = serde_json::to_vec(&json!({"format":"gore.save.draft.v1","path":save.canonicalize().unwrap(),"expectedSha1":hash,"edits":edits,"syncPersistentDataList":true})).unwrap();
        fs::write(&draft, &pending).unwrap();
        for flags in [vec!["validate"], vec!["apply"], vec!["apply", "--dry-run"]] {
            let mut args = vec!["draft", flags[0], draft.to_str().unwrap()];
            args.extend_from_slice(&flags[1..]);
            let error = run_failure(home, &args);
            assert!(
                error.to_string().contains("pending edit conflict"),
                "{error}"
            );
            assert_eq!(fs::read(&draft).unwrap(), pending);
            unchanged();
        }
    }
    fs::remove_file(&draft).unwrap();
    for name in ["First staged name", "Final staged name"] {
        run(
            home,
            &[
                "rename",
                save.to_str().unwrap(),
                "--name",
                name,
                "--draft",
                draft.to_str().unwrap(),
            ],
        );
        unchanged();
    }
    let pending: Value = serde_json::from_slice(&fs::read(&draft).unwrap()).unwrap();
    assert_eq!(
        pending["edits"],
        json!([{"path":"public.m_PlayerSaveName","value":"Final staged name"}])
    );
    run(home, &["draft", "validate", draft.to_str().unwrap()]);
    unchanged();
    let result = run(home, &["draft", "apply", draft.to_str().unwrap()]);
    assert_eq!(result["complete"], true, "{result}");
    assert_ne!(gore_save::api::file_sha1(&save).unwrap(), hash);
    assert_ne!(fs::read(&profile).unwrap(), profile_bytes);
}

#[test]
fn repeated_structured_targets_refuse_manual_drafts_and_raw_core_requests() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path();
    let save = home.join("G1R-001.sav");
    fs::copy(fixture(), &save).unwrap();
    let characters = execute_core("private.characters.list", json!({"path":save}));
    let rows = characters["characters"].as_array().unwrap();
    let actor = |name: &str| {
        rows.iter()
            .find(|row| {
                row["uniqueName"]
                    .as_str()
                    .is_some_and(|value| value.contains(name))
            })
            .unwrap()["globalId"]
            .as_str()
            .unwrap()
    };
    let diego = actor("Diego");
    let buster = actor("Buster");
    let first =
        json!({"path":"private.npc.setRelationship","value":{"id":diego,"relationship":"friend"}});
    let second =
        json!({"path":"private.npc.setRelationship","value":{"id":diego,"relationship":"enemy"}});
    for edit in [&first, &second] {
        execute_core(
            "apply_edits",
            json!({"path":save,"edits":[edit],"dryRun":true}),
        );
    }
    let hash = gore_save::api::file_sha1(&save).unwrap();
    let draft = home.join("repeated.json");
    let request = home.join("request.json");
    for edits in [
        vec![first.clone(), second.clone()],
        vec![second.clone(), first.clone()],
    ] {
        for command in ["plan_edits", "write_save", "apply_edits"] {
            for dry in [true, false] {
                let error = gore_save::api::execute(&gore_save::api::Request {
                    command: command.into(),
                    payload: json!({"path":save,"edits":edits,"dryRun":dry,"backup":true}),
                })
                .unwrap_err();
                assert!(
                    matches!(
                        error,
                        gore_save::CoreError::PlanConflict {
                            kind: "property",
                            ..
                        } | gore_save::CoreError::UnsupportedEdit(_)
                    ),
                    "{error}"
                );
                assert_eq!(gore_save::api::file_sha1(&save).unwrap(), hash);
                assert!(!home.join("goresave_backups").exists());
            }
        }
        let pending = serde_json::to_vec(&json!({"format":"gore.save.draft.v1","path":save.canonicalize().unwrap(),"expectedSha1":hash,"edits":edits})).unwrap();
        fs::write(&draft, &pending).unwrap();
        for flags in [vec!["validate"], vec!["apply"], vec!["apply", "--dry-run"]] {
            let mut args = vec!["draft", flags[0], draft.to_str().unwrap()];
            args.extend_from_slice(&flags[1..]);
            let error = run_failure(home, &args);
            assert!(
                error.to_string().contains("pending edit conflict"),
                "{error}"
            );
            assert_eq!(fs::read(&draft).unwrap(), pending);
            assert_eq!(gore_save::api::file_sha1(&save).unwrap(), hash);
            assert!(!home.join("goresave_backups").exists());
        }
        for dry in [true, false] {
            fs::write(&request,serde_json::to_vec(&json!({"command":"apply_edits","payload":{"path":save,"edits":edits,"dryRun":dry,"backup":true}})).unwrap()).unwrap();
            let error = run_failure(
                home,
                &["core", "exec", "--request-file", request.to_str().unwrap()],
            );
            assert!(
                error.to_string().contains("pending edit conflict"),
                "{error}"
            );
            assert_eq!(fs::read(&draft).unwrap(), pending);
            assert_eq!(gore_save::api::file_sha1(&save).unwrap(), hash);
            assert!(!home.join("goresave_backups").exists());
        }
    }
    let other =
        json!({"path":"private.npc.setRelationship","value":{"id":buster,"relationship":"enemy"}});
    let result = execute_core(
        "apply_edits",
        json!({"path":save,"edits":[first,other],"backup":true}),
    );
    assert_eq!(result["complete"], true);
    assert_eq!(result["committed"], json!([0, 1]));
    assert_ne!(gore_save::api::file_sha1(&save).unwrap(), hash);
}

#[test]
fn inventory_counts_and_removals_of_the_same_stack_fail_before_publication() {
    for actor in ["hero", "OC_STT_Diego-WP_EZ_START_DIEGO_SPAWN"] {
        let temp = tempfile::tempdir().unwrap();
        let home = temp.path();
        let save = home.join("G1R-001.sav");
        fs::copy(fixture(), &save).unwrap();
        let save_arg = save.to_str().unwrap();
        let inventory = run(
            home,
            &["inventory", "list", save_arg, "--actor", actor, "--all"],
        );
        let items = inventory["items"].as_array().unwrap();
        let selected = items
            .iter()
            .find(|row| {
                row["removable"] == true
                    && row["slotId"].is_i64()
                    && row["count"].as_i64().is_some_and(|n| n > 0)
            })
            .unwrap();
        let mut value = json!({"path":selected["path"],"containerType":selected["containerType"],"slotId":selected["slotId"]});
        if actor != "hero" {
            value["actorId"] = json!(actor);
        }
        let remove = json!({"path":"private.inventory.removeItem","value":value});
        value["count"] = json!(selected["count"].as_i64().unwrap() + 7);
        let count = json!({"path":"private.inventory.setItemCount","value":value});
        for edit in [&count, &remove] {
            execute_core(
                "apply_edits",
                json!({"path":save,"edits":[edit],"dryRun":true}),
            );
        }
        let hash = gore_save::api::file_sha1(&save).unwrap();
        for edits in [
            vec![count.clone(), remove.clone()],
            vec![remove.clone(), count.clone()],
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
                                kind: "inventorySlot",
                                ..
                            }
                    ),
                    "{error}"
                );
                assert_eq!(gore_save::api::file_sha1(&save).unwrap(), hash);
                assert!(!home.join("goresave_backups").exists());
            }
            let draft = home.join("inventory.json");
            fs::write(&draft,serde_json::to_vec(&json!({"format":"gore.save.draft.v1","path":save,"expectedSha1":hash,"edits":edits})).unwrap()).unwrap();
            let pending = fs::read(&draft).unwrap();
            for command in ["validate", "apply"] {
                let error = run_failure(home, &["draft", command, draft.to_str().unwrap()]);
                assert!(
                    error.to_string().contains("pending edit conflict"),
                    "{error}"
                );
                assert_eq!(fs::read(&draft).unwrap(), pending);
                assert_eq!(gore_save::api::file_sha1(&save).unwrap(), hash);
                assert!(!home.join("goresave_backups").exists());
            }
        }
        // Both operations still commit when they address separate stacks.
        let other = items
            .iter()
            .find(|row| {
                row["removable"] == true
                    && row["slotId"].is_i64()
                    && row["path"] != selected["path"]
            })
            .unwrap();
        let mut remove_other = remove.clone();
        remove_other["value"]["path"] = other["path"].clone();
        remove_other["value"]["slotId"] = other["slotId"].clone();
        remove_other["value"]["containerType"] = other["containerType"].clone();
        let result = execute_core(
            "apply_edits",
            json!({"path":save,"edits":[count.clone(),remove_other],"backup":false}),
        );
        assert_eq!(result["complete"], true);
        assert_eq!(result["committed"], json!([0, 1]));
        let after = run(
            home,
            &["inventory", "list", save_arg, "--actor", actor, "--all"],
        );
        let rows = after["items"].as_array().unwrap();
        let kept = rows
            .iter()
            .find(|row| {
                row["path"] == selected["path"]
                    && row["slotId"] == selected["slotId"]
                    && row["containerType"] == selected["containerType"]
            })
            .unwrap();
        assert_eq!(kept["count"], count["value"]["count"]);
        assert!(!rows.iter().any(|row| row["path"] == other["path"]
            && row["slotId"] == other["slotId"]
            && row["containerType"] == other["containerType"]));
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
fn hero_position_show_returns_the_transform_and_rejects_npc_pin_status() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path();
    let save = home.join("G1R-001.sav");
    fs::copy(fixture(), &save).unwrap();
    let before = fs::read(&save).unwrap();
    let save_arg = save.to_str().unwrap();
    let inspected = execute_core("inspect_save", json!({"path":save,"includePrivate":true}));
    let expected = &inspected["private"]["player"]["transform"];
    assert!(expected["location"]["x"].is_number());
    assert!(expected["rotation"].is_object());
    for actor in [vec![], vec!["--actor", "HERO"]] {
        let mut args = vec!["position", "show", save_arg];
        args.extend(actor.clone());
        let shown = run(home, &args);
        assert_eq!(&shown, expected);
        assert!(shown.get("private").is_none());
        args[1] = "pin-status";
        let error = run_failure(home, &args);
        assert!(error.to_string().contains("requires an NPC"), "{error}");
    }
    let x = (expected["location"]["x"].as_f64().unwrap() + 7.0).to_string();
    run(home, &["position", "set", save_arg, "--x", &x, "--dry-run"]);
    assert_eq!(run(home, &["position", "show", save_arg]), *expected);
    let npc = "OC_STT_Diego-WP_EZ_START_DIEGO_SPAWN";
    let shown = run(home, &["position", "show", save_arg, "--actor", npc]);
    assert!(shown["pose"]["location"].is_object());
    assert_eq!(
        run(home, &["position", "pin-status", save_arg, "--actor", npc]),
        shown
    );
    assert_eq!(fs::read(save).unwrap(), before);
    assert!(!home.join("goresave_backups").exists());
}

#[test]
fn library_add_accepts_only_inspectable_gsav_files_before_updating_settings() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path();
    let valid = home.join("external.backup");
    fs::copy(fixture(), &valid).unwrap();
    let bytes = fs::read(&valid).unwrap();
    let note = home.join("note.sav");
    fs::write(&note, b"this is not a save").unwrap();
    let truncated = home.join("truncated.sav");
    fs::write(&truncated, &bytes[..16]).unwrap();
    let profile = home.join("profile.sav");
    let profile_bytes = profile_fixture("Gothic");
    fs::write(&profile, &profile_bytes).unwrap();
    let missing = home.join("absent/directory/missing.sav");
    let settings = home.join("gore/gore-save/settings.json");
    let bad_paths: [&Path; 5] = [&note, &truncated, &profile, &missing, home];
    for initialized in [false, true] {
        if initialized {
            let preview = run(
                home,
                &["library", "add", valid.to_str().unwrap(), "--dry-run"],
            );
            assert_eq!(preview["externalSavePaths"].as_array().unwrap().len(), 1);
            assert!(!settings.exists());
            let added = run(home, &["library", "add", valid.to_str().unwrap()]);
            let path = added["externalSavePaths"][0].as_str().unwrap();
            assert_eq!(
                Path::new(path).canonicalize().unwrap(),
                valid.canonicalize().unwrap()
            );
        }
        let snapshot = fs::read(&settings).ok();
        for invalid in bad_paths {
            for dry_run in [false, true] {
                let mut args = vec!["library", "add", invalid.to_str().unwrap()];
                if dry_run {
                    args.push("--dry-run");
                }
                run_failure(home, &args);
                assert_eq!(fs::read(&settings).ok(), snapshot);
            }
        }
        run_failure(home, &["library", "add", "missing.sav"]);
        assert_eq!(fs::read(&settings).ok(), snapshot);
    }
    let before = fs::read(&settings).unwrap();
    let duplicate = run_from(home, Some(home), &["library", "add", "external.backup"]);
    assert_eq!(duplicate["externalSavePaths"].as_array().unwrap().len(), 1);
    assert_eq!(fs::read(&settings).unwrap(), before);
    let root = home.join("game-saves");
    fs::create_dir(&root).unwrap();
    let listed = run(home, &["list", "--root", root.to_str().unwrap()]);
    let rows = listed["saves"].as_array().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["external"], true);
    assert_eq!(rows[0]["format"], "GSAV");
    assert_ne!(rows[0]["available"], false);
    assert_eq!(fs::read(valid).unwrap(), bytes);
    assert_eq!(fs::read(note).unwrap(), b"this is not a save");
    assert_eq!(fs::read(profile).unwrap(), profile_bytes);
    assert_eq!(fs::read(&settings).unwrap(), before);
    assert!(!home.join("goresave_backups").exists());
}

#[test]
fn library_removes_missing_absolute_and_relative_paths_without_requiring_their_directory() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path().join("Long library root with spaces");
    fs::create_dir(&home).unwrap();
    let directory = home.join("external/nested");
    fs::create_dir_all(&directory).unwrap();
    let file = directory.join("old.sav");
    fs::copy(fixture(), &file).unwrap();
    #[cfg(unix)]
    let alias = {
        let alias = temp.path().join("library-alias");
        std::os::unix::fs::symlink(&home, &alias).unwrap();
        alias
    };
    #[cfg(windows)]
    let alias = {
        use std::os::windows::ffi::{OsStrExt, OsStringExt};
        let input = home
            .as_os_str()
            .encode_wide()
            .chain(Some(0))
            .collect::<Vec<_>>();
        let mut output = vec![0u16; 32768];
        let length = unsafe {
            windows_sys::Win32::Storage::FileSystem::GetShortPathNameW(
                input.as_ptr(),
                output.as_mut_ptr(),
                output.len() as u32,
            )
        };
        assert!(
            length > 0 && (length as usize) < output.len(),
            "{}",
            std::io::Error::last_os_error()
        );
        PathBuf::from(std::ffi::OsString::from_wide(&output[..length as usize]))
    };
    assert_eq!(alias.canonicalize().unwrap(), home.canonicalize().unwrap());
    let selected = alias.join("external/nested/old.sav");
    let path = selected.to_str().unwrap();
    run(&home, &["library", "add", path]);
    run(&home, &["library", "hide", path]);
    fs::remove_file(&file).unwrap();
    fs::remove_dir_all(home.join("external")).unwrap();
    let settings_path = home.join("gore/gore-save/settings.json");
    let before = fs::read(&settings_path).unwrap();
    let preview = run(&home, &["library", "remove", path, "--dry-run"]);
    assert_eq!(preview["externalSavePaths"], json!([]));
    assert_eq!(fs::read(&settings_path).unwrap(), before);
    let preview = run_from(
        &home,
        Some(&alias),
        &["library", "unhide", "external/nested/old.sav", "--dry-run"],
    );
    assert_eq!(preview["hiddenOtherSavePaths"], json!([]));
    assert_eq!(fs::read(&settings_path).unwrap(), before);
    assert_eq!(
        run(&home, &["library", "remove", path])["externalSavePaths"],
        json!([])
    );
    let removed = run_from(
        &home,
        Some(&alias),
        &["library", "unhide", "external/nested/old.sav"],
    );
    assert_eq!(removed["externalSavePaths"], json!([]));
    assert_eq!(removed["hiddenOtherSavePaths"], json!([]));
    assert!(!home.join("external").exists());
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
fn lock_reads_follow_saved_names_and_include_uncatalogued_unlocks() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path();
    let save = home.join("G1R-001.sav");
    fs::copy(fixture(), &save).unwrap();
    for name in [
        "AMR_Storage_Room_Door",
        "AbandonedMine_Fence_Door",
        "Modded_Lock_CLI_Test",
    ] {
        run(
            home,
            &["locks", "unlock", save.to_str().unwrap(), "--lock", name],
        );
        let shown = run(
            home,
            &["locks", "show", save.to_str().unwrap(), "--lock", name],
        );
        assert_eq!(shown["total"], 1, "{shown}");
        assert_eq!(shown["locks"][0]["unlocked"], true, "{name}");
        assert!(
            run(
                home,
                &[
                    "locks",
                    "list",
                    save.to_str().unwrap(),
                    "--state",
                    "unlocked",
                    "--all"
                ]
            )["locks"]
                .as_array()
                .unwrap()
                .iter()
                .any(|r| r["n"] == name)
        );
        run(
            home,
            &["locks", "lock", save.to_str().unwrap(), "--lock", name],
        );
        let shown = run(
            home,
            &["locks", "show", save.to_str().unwrap(), "--lock", name],
        );
        if name == "Modded_Lock_CLI_Test" {
            assert_eq!(shown["total"], 0);
        } else {
            assert_eq!(shown["locks"][0]["unlocked"], false);
        }
    }
}

#[test]
fn exact_data_reads_return_only_the_requested_native_path() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path();
    let save = home.join("G1R-001.sav");
    fs::copy(fixture(), &save).unwrap();
    let property = json!(["m_GenericData", "{GameTime}", "CurrentTime", "TotalSeconds"]);
    let path = home.join("path.json");
    fs::write(&path, serde_json::to_vec(&property).unwrap()).unwrap();
    run(
        home,
        &[
            "data",
            "set",
            save.to_str().unwrap(),
            "--path-file",
            path.to_str().unwrap(),
            "--value-json",
            "123456.0",
        ],
    );
    let before = fs::read(&save).unwrap();
    let output = Command::cargo_bin("gore")
        .unwrap()
        .timeout(std::time::Duration::from_secs(60))
        .env("LOCALAPPDATA", home)
        .env("APPDATA", home)
        .env("XDG_DATA_HOME", home)
        .env("GORE_DISABLE_GAME_AUTODETECT", "1")
        .args([
            "save",
            "data",
            "show",
            save.to_str().unwrap(),
            "--path-file",
            path.to_str().unwrap(),
            "--json",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let data: Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(data["ok"], true);
    let data = &data["data"];
    assert_eq!(data["total"], 1);
    assert_eq!(data["results"][0]["path"], property);
    assert_eq!(data["results"][0]["editValue"], 123456.0);
    assert_eq!(fs::read(&save).unwrap(), before);
}

#[test]
fn exact_npc_reads_filter_before_pagination() {
    let temp = tempfile::tempdir().unwrap();
    let save = fixture();
    let actor = "Goblin_Black-WP_EF_GOBLINCAVE_GOBLIN_SPAWN_08-2";
    for command in [vec!["npc", "show"], vec!["npc", "relationship", "show"]] {
        let mut args = command;
        args.extend([save.to_str().unwrap(), "--actor", actor]);
        let data = run(temp.path(), &args);
        assert_eq!(data["total"], 1, "{data}");
        assert_eq!(data["npcs"][0]["id"], actor);
        args.extend(["--offset", "1", "--limit", "1"]);
        let empty = run(temp.path(), &args);
        assert_eq!(empty["total"], 1);
        assert_eq!(empty["npcs"], json!([]));
    }
}

#[test]
fn pending_recovery_blocks_profile_writes_without_changing_any_files() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path();
    let save = home.join("G1R-001.sav");
    let profile = home.join("PersistentDataList.sav");
    fs::copy(fixture(), &save).unwrap();
    fs::write(&profile, profile_fixture("Gothic")).unwrap();
    run(home, &["delete", save.to_str().unwrap(), "--profile", "0"]);
    let recovery = run(
        home,
        &["recovery", "show", "--root", home.to_str().unwrap()],
    );
    let before = fs::read(&profile).unwrap();
    let manifest = home.join("goresave_backups").join(format!(
        ".delete-recovery.{}.json",
        Path::new(recovery["backupPath"].as_str().unwrap())
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
    ));
    let record = fs::read(&manifest).unwrap();
    let other = home.join("G1R-002.sav");
    fs::copy(fixture(), &other).unwrap();
    let draft = home.join("pending.json");
    run(
        home,
        &[
            "draft",
            "create",
            draft.to_str().unwrap(),
            "--target",
            other.to_str().unwrap(),
        ],
    );
    run(
        home,
        &[
            "rename",
            other.to_str().unwrap(),
            "--name",
            "Pending name",
            "--draft",
            draft.to_str().unwrap(),
        ],
    );
    let other_before = fs::read(&other).unwrap();
    let draft_before = fs::read(&draft).unwrap();
    for base in [
        vec![
            "difficulty",
            "set",
            "--root",
            home.to_str().unwrap(),
            "--profile",
            "0",
            "--preset",
            "Hard",
        ],
        vec![
            "profile",
            "assign",
            other.to_str().unwrap(),
            "--profile",
            "0",
        ],
        vec![
            "profile",
            "detach",
            other.to_str().unwrap(),
            "--profile",
            "0",
        ],
        vec!["rename", other.to_str().unwrap(), "--name", "Blocked name"],
        vec!["draft", "validate", draft.to_str().unwrap()],
        vec!["draft", "apply", draft.to_str().unwrap()],
        vec![
            "backups",
            "restore",
            "--target",
            save.to_str().unwrap(),
            "--backup",
            recovery["backupPath"].as_str().unwrap(),
        ],
        vec![
            "backups",
            "delete",
            "--target",
            save.to_str().unwrap(),
            "--backup",
            recovery["backupPath"].as_str().unwrap(),
            "--yes",
        ],
    ] {
        for dry in [false, true] {
            let mut args = base.clone();
            if dry {
                args.push("--dry-run");
            }
            let error = run_failure(home, &args);
            assert!(
                error.to_string().contains("pending deleted-save recovery"),
                "{error}"
            );
            assert_eq!(fs::read(&profile).unwrap(), before);
            assert_eq!(fs::read(&manifest).unwrap(), record);
            assert_eq!(fs::read(&other).unwrap(), other_before);
            assert_eq!(fs::read(&draft).unwrap(), draft_before);
            assert_eq!(
                run(
                    home,
                    &["recovery", "show", "--root", home.to_str().unwrap()]
                ),
                recovery
            );
        }
    }
}

#[test]
fn recovery_dismiss_clears_only_matching_editor_tokens_and_accepts_invalid_manifests() {
    for scenario in [
        "valid",
        "malformed",
        "malformed-no-token",
        "malformed-incomplete-token",
        "stale",
        "other-token",
    ] {
        let temp = tempfile::tempdir().unwrap();
        let home = temp.path();
        let save = home.join("G1R-001.sav");
        let profile = home.join("PersistentDataList.sav");
        fs::copy(fixture(), &save).unwrap();
        fs::write(&profile, profile_fixture("Gothic")).unwrap();
        run(home, &["delete", save.to_str().unwrap(), "--profile", "0"]);
        let recovery = run(
            home,
            &["recovery", "show", "--root", home.to_str().unwrap()],
        );
        let backup = Path::new(recovery["backupPath"].as_str().unwrap());
        let manifest = backup.parent().unwrap().join(format!(
            ".delete-recovery.{}.json",
            backup.file_name().unwrap().to_str().unwrap()
        ));
        let mut token = recovery.clone();
        token["message"] = json!("Recovery available");
        if scenario == "other-token" {
            token["backupPath"] = json!(home.join("goresave_backups/G1R-002.sav.bak.0"));
            token["targetPath"] = json!(home.join("G1R-002.sav"));
        }
        run(
            home,
            &[
                "settings",
                "set",
                "--scope",
                "editor",
                "--key",
                "saveDir",
                "--value",
                home.to_str().unwrap(),
            ],
        );
        if scenario == "malformed-incomplete-token" {
            token = json!({"targetPath":save,"backupPath":backup});
        }
        if scenario != "malformed-no-token" {
            run(
                home,
                &[
                    "settings",
                    "set",
                    "--scope",
                    "editor",
                    "--key",
                    "deletedSaveRecovery",
                    "--value-json",
                    &token.to_string(),
                ],
            );
        }
        let settings = run(home, &["settings", "show", "--scope", "editor"]);
        let settings_path = Path::new(settings["path"].as_str().unwrap());
        if scenario.starts_with("malformed") {
            fs::write(&manifest, b"broken JSON").unwrap();
        }
        if scenario == "stale" {
            fs::write(&profile, profile_fixture("Hard")).unwrap();
        }
        let profile_before = fs::read(&profile).unwrap();
        let manifest_before = fs::read(&manifest).unwrap();
        let settings_before = fs::read(settings_path).unwrap();
        let backup_before = fs::read(backup).unwrap();
        if scenario == "malformed-incomplete-token" {
            run(
                home,
                &[
                    "difficulty",
                    "set",
                    "--root",
                    home.to_str().unwrap(),
                    "--profile",
                    "0",
                    "--preset",
                    "Hard",
                    "--dry-run",
                ],
            );
            assert_eq!(fs::read(&profile).unwrap(), profile_before);
        }
        if matches!(scenario, "malformed" | "stale") {
            let error = run_failure(
                home,
                &[
                    "difficulty",
                    "set",
                    "--root",
                    home.to_str().unwrap(),
                    "--profile",
                    "0",
                    "--preset",
                    "Hard",
                ],
            );
            assert!(
                error.to_string().contains("pending deleted-save recovery"),
                "{error}"
            );
            assert_eq!(fs::read(&profile).unwrap(), profile_before);
        }
        let args = [
            "recovery",
            "dismiss",
            "--root",
            home.to_str().unwrap(),
            "--backup",
            backup.to_str().unwrap(),
        ];
        let mut dry = args.to_vec();
        dry.push("--dry-run");
        let mut wrong = args.to_vec();
        let wrong_target = home.join("G1R-003.sav");
        wrong.extend(["--target", wrong_target.to_str().unwrap()]);
        run_failure(home, &wrong);
        assert_eq!(fs::read(&manifest).unwrap(), manifest_before);
        assert_eq!(fs::read(settings_path).unwrap(), settings_before);
        assert_eq!(run(home, &dry)["validated"], true);
        assert_eq!(fs::read(&manifest).unwrap(), manifest_before);
        assert_eq!(fs::read(settings_path).unwrap(), settings_before);
        assert_eq!(run(home, &args)["dismissed"], true);
        assert!(!manifest.exists());
        assert!(!save.exists());
        assert_eq!(fs::read(backup).unwrap(), backup_before);
        assert_eq!(fs::read(&profile).unwrap(), profile_before);
        let current = run(home, &["settings", "show", "--scope", "editor"]);
        if scenario == "other-token" {
            assert_eq!(current["settings"]["deletedSaveRecovery"], token);
        } else {
            assert!(current["settings"].get("deletedSaveRecovery").is_none());
            run(
                home,
                &[
                    "difficulty",
                    "set",
                    "--root",
                    home.to_str().unwrap(),
                    "--profile",
                    "0",
                    "--preset",
                    "Hard",
                    "--dry-run",
                ],
            );
            assert_eq!(fs::read(&profile).unwrap(), profile_before);
        }
    }
}

#[test]
fn recovery_previews_preserve_manifest_hashes_when_run_from_the_save_directory() {
    for operation in ["restore", "dismiss"] {
        let temp = tempfile::tempdir().unwrap();
        let home = temp.path();
        let save = home.join("G1R-001.sav");
        fs::copy(fixture(), &save).unwrap();
        let original_save = fs::read(&save).unwrap();
        let profile = home.join("PersistentDataList.sav");
        let original_profile = profile_fixture("Recovery");
        fs::write(&profile, &original_profile).unwrap();
        run_from(
            home,
            Some(home),
            &["delete", "G1R-001.sav", "--profile", "0"],
        );
        assert!(!save.exists());
        let deleted_profile = fs::read(&profile).unwrap();
        let backups = home.join("goresave_backups");
        let snapshots = fs::read_dir(&backups)
            .unwrap()
            .map(|entry| {
                let path = entry.unwrap().path();
                let bytes = fs::read(&path).unwrap();
                (path, bytes)
            })
            .collect::<Vec<_>>();
        assert!(snapshots.iter().any(|(path, _)| {
            path.extension()
                .is_some_and(|extension| extension == "json")
        }));
        let repair = run_from(
            home,
            Some(home),
            &[
                "recovery",
                "repair",
                "--root",
                home.to_str().unwrap(),
                "--dry-run",
            ],
        );
        assert_eq!(
            repair["simulation"]["deletedSaveRecovery"]["persistentPostDeleteSha1"],
            gore_save::api::file_sha1(&profile).unwrap(),
            "repair={repair:#}, manifests={:?}",
            snapshots
                .iter()
                .filter(|(path, _)| path
                    .extension()
                    .is_some_and(|extension| extension == "json"))
                .map(|(_, bytes)| String::from_utf8_lossy(bytes))
                .collect::<Vec<_>>()
        );
        let recovery =
            run_from(home, Some(home), &["recovery", "list", "--root", "."])["recoveries"][0]
                .clone();
        let relative_backup = Path::new("goresave_backups").join(
            Path::new(recovery["backupPath"].as_str().unwrap())
                .file_name()
                .unwrap(),
        );
        let relative_backup = relative_backup.to_str().unwrap();
        assert_eq!(
            run_from(
                home,
                Some(home),
                &[
                    "recovery",
                    "show",
                    "--root",
                    ".",
                    "--backup",
                    relative_backup
                ]
            ),
            recovery
        );
        for relative in [false, true] {
            let backup_arg = if relative {
                relative_backup
            } else {
                recovery["backupPath"].as_str().unwrap()
            };
            let root_arg = if relative {
                "."
            } else {
                home.to_str().unwrap()
            };
            let preview = run_from(
                home,
                Some(home),
                &[
                    "recovery",
                    operation,
                    "--root",
                    root_arg,
                    "--backup",
                    backup_arg,
                    "--dry-run",
                ],
            );
            assert_eq!(preview["validated"], true);
            assert_eq!(
                preview["request"]["expectedPersistentSha1"],
                gore_save::api::file_sha1(&profile).unwrap()
            );
            if operation == "dismiss" {
                assert_eq!(preview["simulation"]["dismissed"], true);
            } else {
                assert_eq!(preview["simulation"]["bytesChanged"], true);
            }
            assert!(!save.exists());
            assert_eq!(fs::read(&profile).unwrap(), deleted_profile);
            assert_eq!(fs::read_dir(&backups).unwrap().count(), snapshots.len());
            for (path, bytes) in &snapshots {
                assert_eq!(fs::read(path).unwrap(), *bytes);
            }
        }
        let result = run_from(
            home,
            Some(home),
            &[
                "recovery",
                operation,
                "--root",
                ".",
                "--backup",
                relative_backup,
            ],
        );
        if operation == "restore" {
            assert_eq!(result["bytesChanged"], true);
            assert_eq!(fs::read(&save).unwrap(), original_save);
            assert_eq!(fs::read(&profile).unwrap(), original_profile);
        } else {
            assert_eq!(result["dismissed"], true);
            assert!(!save.exists());
            assert_eq!(fs::read(&profile).unwrap(), deleted_profile);
        }
        for (path, bytes) in snapshots {
            if path
                .extension()
                .is_some_and(|extension| extension == "json")
            {
                assert!(!path.exists());
            } else {
                assert_eq!(fs::read(path).unwrap(), bytes);
            }
        }
    }
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
    let relative_backup = Path::new("goresave_backups").join(
        Path::new(records[0]["backupPath"].as_str().unwrap())
            .file_name()
            .unwrap(),
    );
    assert_eq!(
        run_from(
            root,
            Some(root),
            &[
                "recovery",
                "show",
                "--root",
                ".",
                "--backup",
                relative_backup.to_str().unwrap()
            ]
        ),
        records[0]
    );
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
fn glossary_roles_follow_live_segment_unlocks_and_discovery() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path();
    let save = home.join("G1R-001.sav");
    fs::copy(fixture(), &save).unwrap();
    let save_arg = save.to_str().unwrap();
    let catalog = run(home, &["catalog", "list", "--kind", "glossary", "--all"]);
    let entry = catalog["entries"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| {
            entry["segments"].as_array().unwrap().iter().any(|segment| {
                segment["roles"]
                    .as_array()
                    .unwrap()
                    .contains(&json!("teacher"))
            })
        })
        .unwrap();
    let segment = entry["segments"]
        .as_array()
        .unwrap()
        .iter()
        .find(|segment| {
            segment["roles"]
                .as_array()
                .unwrap()
                .contains(&json!("teacher"))
        })
        .unwrap();
    let id = entry["id"].as_str().unwrap();
    let document = entry["documentClass"].as_str().unwrap();
    let segment = segment["class"].as_str().unwrap();
    let listed = |state| {
        run(
            home,
            &[
                "glossary", "list", save_arg, "--id", id, "--state", state, "--role", "teacher",
                "--all",
            ],
        )
    };
    assert_eq!(listed("unlocked")["total"], 0);
    let original = fs::read(&save).unwrap();
    run(
        home,
        &[
            "glossary",
            "segment",
            "unlock",
            save_arg,
            "--document",
            document,
            "--segment",
            segment,
            "--dry-run",
        ],
    );
    assert_eq!(fs::read(&save).unwrap(), original);
    assert!(!home.join("goresave_backups").exists());
    run(
        home,
        &[
            "glossary",
            "segment",
            "unlock",
            save_arg,
            "--document",
            document,
            "--segment",
            segment,
        ],
    );
    let unlocked = listed("unlocked");
    assert_eq!(unlocked["total"], 1, "{unlocked}");
    let rows = unlocked["categories"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|category| category["entries"].as_array().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["id"], id);
    assert_eq!(rows[0]["unlocked"], true);
    assert!(
        rows[0]["roles"]
            .as_array()
            .unwrap()
            .contains(&json!("teacher"))
    );
    run(
        home,
        &[
            "glossary",
            "segment",
            "lock",
            save_arg,
            "--document",
            document,
            "--segment",
            segment,
        ],
    );
    assert_eq!(listed("unlocked")["total"], 0);
}

#[test]
fn relative_statistics_preserve_profile_chapter_and_playtime() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path();
    let save = home.join("G1R-001.sav");
    fs::copy(fixture(), &save).unwrap();
    let profile = home.join("PersistentDataList.sav");
    let profile_bytes = profile_fixture_with_metadata("Statistics", Some((3, 12345.0)));
    fs::write(&profile, &profile_bytes).unwrap();
    let original = fs::read(&save).unwrap();
    let absolute = run(home, &["statistics", save.to_str().unwrap()]);
    assert_eq!(absolute["statistics"]["progress"]["chapter"], 3);
    assert_eq!(
        absolute["statistics"]["progress"]["playedSeconds"].as_f64(),
        Some(12345.0)
    );
    for name in ["G1R-001.sav", "./G1R-001.sav"] {
        let relative = run_from(home, Some(home), &["statistics", name]);
        assert_eq!(
            relative["statistics"]["progress"],
            absolute["statistics"]["progress"]
        );
    }
    assert_eq!(fs::read(&save).unwrap(), original);
    assert_eq!(fs::read(&profile).unwrap(), profile_bytes);
    assert!(!home.join("goresave_backups").exists());
}

#[test]
fn corrupt_ui_preferences_do_not_block_commands_or_a_full_reset() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path();
    let path = home.join("gore/gore-save/ui_settings.json");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let save = home.join("G1R-001.sav");
    fs::copy(fixture(), &save).unwrap();
    let original = fs::read(&save).unwrap();
    let report = home.join("report.html");
    for bytes in [b"{broken".as_slice(), b"null".as_slice()] {
        fs::write(&path, bytes).unwrap();
        run(home, &["about"]);
        fs::write(&report, b"existing report").unwrap();
        run(
            home,
            &[
                "report",
                save.to_str().unwrap(),
                "--out",
                report.to_str().unwrap(),
                "--dry-run",
            ],
        );
        assert_eq!(fs::read(&report).unwrap(), b"existing report");
        assert_eq!(fs::read(&path).unwrap(), bytes);
        run(
            home,
            &[
                "report",
                save.to_str().unwrap(),
                "--out",
                report.to_str().unwrap(),
            ],
        );
        assert!(
            fs::read_to_string(&report)
                .unwrap()
                .contains("id=\"statistics\"")
        );
        assert_eq!(fs::read(&path).unwrap(), bytes);
        assert_eq!(fs::read(&save).unwrap(), original);
        assert!(!home.join("goresave_backups").exists());
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
fn import_previews_copy_requested_gsav_sources_with_arbitrary_filenames() {
    for name in [
        "export.backup",
        "without-extension",
        "G1R-010.SAV",
        "nested/export.backup",
    ] {
        let temp = tempfile::tempdir().unwrap();
        let home = temp.path();
        let destination = home.join("saves");
        let source_root = if name.starts_with("nested/") {
            destination.clone()
        } else {
            home.join("downloads")
        };
        fs::create_dir_all(&destination).unwrap();
        let source = source_root.join(name);
        fs::create_dir_all(source.parent().unwrap()).unwrap();
        fs::copy(fixture(), &source).unwrap();
        let source_hash = gore_save::api::file_sha1(&source).unwrap();
        let profile = destination.join("PersistentDataList.sav");
        let profile_bytes = profile_fixture("Gothic");
        fs::write(&profile, &profile_bytes).unwrap();
        for relative in [false, true] {
            let input = if relative {
                source.file_name().unwrap().to_str().unwrap()
            } else {
                source.to_str().unwrap()
            };
            let result = run_from(
                home,
                relative.then(|| source.parent().unwrap()),
                &[
                    "import",
                    input,
                    "--root",
                    destination.to_str().unwrap(),
                    "--profile",
                    "0",
                    "--dry-run",
                ],
            );
            assert_eq!(result["validated"], true);
            assert_eq!(gore_save::api::file_sha1(&source).unwrap(), source_hash);
            assert_eq!(fs::read(&profile).unwrap(), profile_bytes);
            assert!(!destination.join("G1R-002.sav").exists());
            assert!(!destination.join("G1R-010.sav").exists());
            assert!(!destination.join("goresave_backups").exists());
            assert!(!source_root.join("goresave_backups").exists());
        }
        run(
            home,
            &[
                "import",
                source.to_str().unwrap(),
                "--root",
                destination.to_str().unwrap(),
                "--profile",
                "0",
            ],
        );
        let imported = destination.join(if name == "G1R-010.SAV" {
            "G1R-010.sav"
        } else {
            "G1R-002.sav"
        });
        assert!(imported.is_file(), "{imported:?}");
        assert_eq!(
            run(home, &["inspect", imported.to_str().unwrap()])["format"],
            "GSAV"
        );
        assert_eq!(gore_save::api::file_sha1(&source).unwrap(), source_hash);
    }
}

#[cfg(unix)]
#[test]
fn administrative_previews_ignore_unrelated_symlinks_without_changing_live_files() {
    use std::os::unix::fs::symlink;
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path();
    let save = home.join("G1R-001.sav");
    fs::copy(fixture(), &save).unwrap();
    let hash = gore_save::api::file_sha1(&save).unwrap();
    let profile = home.join("PersistentDataList.sav");
    let profile_bytes = profile_fixture("Gothic");
    fs::write(&profile, &profile_bytes).unwrap();
    let external = tempfile::tempdir().unwrap();
    let outside = external.path().join("outside.txt");
    fs::write(&outside, b"outside bytes").unwrap();
    let link = home.join("shortcut");
    symlink(&outside, &link).unwrap();
    symlink(external.path(), home.join("directory-shortcut")).unwrap();
    for args in [
        vec![
            "profile",
            "assign",
            save.to_str().unwrap(),
            "--profile",
            "0",
            "--dry-run",
        ],
        vec![
            "difficulty",
            "set",
            save.to_str().unwrap(),
            "--profile",
            "0",
            "--preset",
            "Hard",
            "--dry-run",
        ],
        vec![
            "delete",
            save.to_str().unwrap(),
            "--profile",
            "0",
            "--yes",
            "--dry-run",
        ],
        vec![
            "recovery",
            "repair",
            "--root",
            home.to_str().unwrap(),
            "--dry-run",
        ],
    ] {
        run(home, &args);
        assert_eq!(gore_save::api::file_sha1(&save).unwrap(), hash);
        assert_eq!(fs::read(&profile).unwrap(), profile_bytes);
        assert_eq!(fs::read(&outside).unwrap(), b"outside bytes");
        assert!(fs::symlink_metadata(&link).unwrap().is_symlink());
        assert!(!home.join("goresave_backups").exists());
    }
    run(
        home,
        &[
            "difficulty",
            "set",
            save.to_str().unwrap(),
            "--profile",
            "0",
            "--preset",
            "Hard",
        ],
    );
    assert_eq!(gore_save::api::file_sha1(&save).unwrap(), hash);
    assert_ne!(fs::read(&profile).unwrap(), profile_bytes);
    assert_eq!(fs::read(&outside).unwrap(), b"outside bytes");
    assert!(fs::symlink_metadata(&link).unwrap().is_symlink());
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
fn inventory_item_only_removals_resolve_unique_stacks_and_preserve_ambiguous_drafts() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path();
    let save = home.join("G1R-001.sav");
    fs::copy(fixture(), &save).unwrap();
    let save_arg = save.to_str().unwrap();
    let actor = "OC_STT_Diego-WP_EZ_START_DIEGO_SPAWN";
    let original = run(
        home,
        &["inventory", "list", save_arg, "--actor", actor, "--all"],
    );
    let items = original["items"].as_array().unwrap();
    let duplicate = items
        .iter()
        .find(|row| {
            row["removable"] == true
                && items
                    .iter()
                    .filter(|other| other["path"] == row["path"])
                    .count()
                    > 1
        })
        .unwrap();
    let pouch = items
        .iter()
        .find(|row| row["containerType"] == "Pouch")
        .unwrap();
    let melee = items
        .iter()
        .find(|row| row["containerType"] == "MeleeSlot")
        .unwrap();
    assert_eq!(
        items
            .iter()
            .filter(|row| row["path"] == pouch["path"])
            .count(),
        1
    );
    assert_eq!(
        items
            .iter()
            .filter(|row| row["path"] == melee["path"])
            .count(),
        1
    );
    let hash = gore_save::api::file_sha1(&save).unwrap();
    let draft = home.join("inventory.json");
    let draft_arg = draft.to_str().unwrap();
    let empty_draft = serde_json::to_vec(&json!({
        "format":"gore.save.draft.v1","path":save.canonicalize().unwrap(),
        "expectedSha1":hash,"edits":[]
    }))
    .unwrap();
    fs::write(&draft, &empty_draft).unwrap();
    for item in [
        duplicate["id"].as_str().unwrap(),
        duplicate["path"].as_str().unwrap(),
        "/Script/Angelscript.ItWr_Scroll_Letter_01",
        "Pouch",
    ] {
        for flags in [
            vec![],
            vec!["--dry-run"],
            vec!["--draft", draft_arg],
            vec!["--draft", draft_arg, "--dry-run"],
        ] {
            let mut args = vec![
                "inventory",
                "remove",
                save_arg,
                "--actor",
                actor,
                "--item",
                item,
                "--offset",
                "100000",
                "--limit",
                "1",
                "--query",
                "unmatched",
            ];
            args.extend(flags);
            let error = run_failure(home, &args);
            assert!(error.to_string().contains("exactly one stack"), "{error}");
            assert_eq!(gore_save::api::file_sha1(&save).unwrap(), hash);
            assert_eq!(fs::read(&draft).unwrap(), empty_draft);
            assert!(!home.join("goresave_backups").exists());
        }
    }
    let no_selector = run_failure(home, &["inventory", "remove", save_arg, "--actor", actor]);
    assert!(
        no_selector
            .to_string()
            .contains("--item or a container/slot selector required")
    );
    let lower_id = pouch["id"].as_str().unwrap().to_ascii_lowercase();
    for (option, item) in [
        ("--item", pouch["id"].as_str().unwrap()),
        ("--item", pouch["path"].as_str().unwrap()),
        ("--item", lower_id.as_str()),
        ("--id", pouch["id"].as_str().unwrap()),
    ] {
        let preview = run(
            home,
            &[
                "inventory",
                "remove",
                save_arg,
                "--actor",
                actor,
                option,
                item,
                "--draft",
                draft_arg,
                "--dry-run",
                "--offset",
                "100000",
                "--limit",
                "1",
            ],
        );
        let value = &preview["data"]["edits"][0]["value"];
        assert_eq!(value["path"], pouch["path"]);
        assert_eq!(value["slotId"], pouch["slotId"]);
        assert_eq!(value["containerType"], "Pouch");
        assert!(value["actorId"].is_string());
        assert_eq!(gore_save::api::file_sha1(&save).unwrap(), hash);
        assert_eq!(fs::read(&draft).unwrap(), empty_draft);
        assert!(!home.join("goresave_backups").exists());
    }
    run(
        home,
        &[
            "inventory",
            "remove",
            save_arg,
            "--actor",
            actor,
            "--item",
            pouch["id"].as_str().unwrap(),
            "--dry-run",
        ],
    );
    assert_eq!(gore_save::api::file_sha1(&save).unwrap(), hash);
    assert!(!home.join("goresave_backups").exists());
    run(
        home,
        &[
            "inventory",
            "remove",
            save_arg,
            "--actor",
            actor,
            "--item",
            pouch["id"].as_str().unwrap(),
            "--draft",
            draft_arg,
        ],
    );
    assert_eq!(gore_save::api::file_sha1(&save).unwrap(), hash);
    run(home, &["draft", "validate", draft_arg]);
    run(home, &["draft", "apply", draft_arg]);
    let after_pouch = run(
        home,
        &["inventory", "list", save_arg, "--actor", actor, "--all"],
    );
    let expected = items
        .iter()
        .filter(|row| row["path"] != pouch["path"])
        .cloned()
        .collect::<Vec<_>>();
    assert_eq!(after_pouch["items"], json!(expected));
    run(
        home,
        &[
            "inventory",
            "remove",
            save_arg,
            "--actor",
            actor,
            "--item",
            melee["path"].as_str().unwrap(),
        ],
    );
    let after_melee = run(
        home,
        &["inventory", "list", save_arg, "--actor", actor, "--all"],
    );
    let expected = expected
        .into_iter()
        .filter(|row| row["path"] != melee["path"])
        .collect::<Vec<_>>();
    assert_eq!(after_melee["items"], json!(expected));

    let hero = run(home, &["inventory", "list", save_arg, "--all"]);
    let hero_items = hero["items"].as_array().unwrap();
    let selected = &hero_items[0];
    assert_eq!(
        hero_items
            .iter()
            .filter(|row| row["path"] == selected["path"])
            .count(),
        1
    );
    let hash = gore_save::api::file_sha1(&save).unwrap();
    run(
        home,
        &[
            "inventory",
            "remove",
            save_arg,
            "--item",
            selected["id"].as_str().unwrap(),
            "--dry-run",
        ],
    );
    assert_eq!(gore_save::api::file_sha1(&save).unwrap(), hash);
    run(
        home,
        &[
            "inventory",
            "remove",
            save_arg,
            "--item",
            selected["id"].as_str().unwrap(),
        ],
    );
    let remaining = run(home, &["inventory", "list", save_arg, "--all"]);
    let expected = hero_items
        .iter()
        .filter(|row| row["path"] != selected["path"])
        .cloned()
        .collect::<Vec<_>>();
    assert_eq!(remaining["items"], json!(expected));
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
        let mut args = vec![
            domain,
            "list",
            save,
            "--all",
            "--limit",
            if domain == "glossary" { "7" } else { "100" },
        ];
        if domain == "glossary" {
            args.push("--include-unset");
        }
        let data = run(&home, &args);
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
