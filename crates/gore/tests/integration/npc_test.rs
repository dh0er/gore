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
