//! Contract and workflow checks through the shipped command-line interface.
use assert_cmd::Command;
use serde_json::{json, Value};
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
