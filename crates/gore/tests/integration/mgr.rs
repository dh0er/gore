//! Integration tests for the `gore mgr` multi-mod manager CLI namespace.
//!
//! The non-ignored end-to-end walks the whole loadout lifecycle WITHOUT a real
//! game deploy: import → list → enable → disable → order → analyze → status →
//! reset. Bundles are built through the real `gore mod build` path. A real
//! game-deploy apply (which needs a decodable `.lcache` fixture) is exercised by
//! the `#[ignore]`d `mgr_apply_on_temp_game` below.

use assert_cmd::Command;
use predicates::prelude::PredicateBooleanExt; // `.not()` on predicates
use std::path::{Path, PathBuf};
use tempfile::TempDir;

/// Write a BuildSpec JSON that produces a goremod bundle whose ONLY component is
/// a loc patch editing `loc_id` (german) → `value`. Two such bundles editing the
/// same `loc_id` share a loc target, so `analyze` reports a conflict.
fn write_loc_spec(dir: &Path, name: &str, loc_id: &str, value: &str) -> PathBuf {
    let spec = serde_json::json!({
        "meta": { "name": name, "version": "1.0", "author": "test" },
        "loc_edits": { loc_id: { "german": value } }
    });
    let path = dir.join(format!("{name}.spec.json"));
    std::fs::write(&path, serde_json::to_vec_pretty(&spec).unwrap()).unwrap();
    path
}

/// Build a bundle from `spec` into `<out>/<name>` via `gore mod build`, returning
/// the bundle dir path.
fn build_bundle(out: &Path, name: &str, spec: &Path) -> PathBuf {
    Command::cargo_bin("gore")
        .unwrap()
        .args(["mod", "build", "--spec", spec.to_str().unwrap(), "-o", out.to_str().unwrap()])
        .assert()
        .success();
    out.join(name)
}

/// Import `bundle` into `library`, registering it in `loadout`; return its library id
/// (parsed from the "imported <id> ..." line so later subcommands can target it).
fn import(library: &Path, loadout: &Path, bundle: &Path) -> String {
    let out = Command::cargo_bin("gore")
        .unwrap()
        .args([
            "mgr",
            "import",
            bundle.to_str().unwrap(),
            "--library",
            library.to_str().unwrap(),
            "--loadout",
            loadout.to_str().unwrap(),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(out).unwrap();
    // Line shape: "imported <id> (<name>) [<Kind>]".
    let line = text.lines().find(|l| l.starts_with("imported ")).expect("import line");
    line.split_whitespace().nth(1).expect("id token").to_string()
}

/// The whole loadout lifecycle with NO real game deploy. This is the always-run gate.
#[test]
fn mgr_import_list_enable_disable_order_analyze_status_reset() {
    let tmp = TempDir::new().unwrap();
    let lib = tmp.path().join("library");
    let loadout = tmp.path().join("loadout.json");
    let built = tmp.path().join("built");

    // Two bundles both editing the SAME loc id → a conflict once both are enabled.
    let spec_a = write_loc_spec(tmp.path(), "AlphaMod", "itfo_cheese", "Gouda");
    let spec_b = write_loc_spec(tmp.path(), "BravoMod", "itfo_cheese", "Brie");
    let bundle_a = build_bundle(&built, "AlphaMod", &spec_a);
    let bundle_b = build_bundle(&built, "BravoMod", &spec_b);

    let id_a = import(&lib, &loadout, &bundle_a);
    let id_b = import(&lib, &loadout, &bundle_b);
    assert_ne!(id_a, id_b, "distinct bundles must get distinct ids");

    // list: both present, both disabled ([ ]) by default, names shown.
    Command::cargo_bin("gore")
        .unwrap()
        .args([
            "mgr", "list", "--library", lib.to_str().unwrap(), "--loadout", loadout.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains(&id_a))
        .stdout(predicates::str::contains(&id_b))
        .stdout(predicates::str::contains("AlphaMod"))
        .stdout(predicates::str::contains("[ ]"));

    // enable both.
    for id in [&id_a, &id_b] {
        Command::cargo_bin("gore")
            .unwrap()
            .args(["mgr", "enable", id, "--loadout", loadout.to_str().unwrap()])
            .assert()
            .success()
            .stdout(predicates::str::contains("enabled"));
    }

    // list now shows an enabled marker.
    Command::cargo_bin("gore")
        .unwrap()
        .args(["mgr", "list", "--library", lib.to_str().unwrap(), "--loadout", loadout.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicates::str::contains("[x]"));

    // analyze: both enabled + same loc id → a conflict line naming both ids and the winner.
    Command::cargo_bin("gore")
        .unwrap()
        .args([
            "mgr", "analyze", "--library", lib.to_str().unwrap(), "--loadout", loadout.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains("itfo_cheese|german"))
        .stdout(predicates::str::contains(&id_a))
        .stdout(predicates::str::contains(&id_b))
        .stdout(predicates::str::contains("winner"));

    // disable one → analyze reports no conflict (only one editor left).
    Command::cargo_bin("gore")
        .unwrap()
        .args(["mgr", "disable", &id_b, "--loadout", loadout.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicates::str::contains("disabled"));

    Command::cargo_bin("gore")
        .unwrap()
        .args([
            "mgr", "analyze", "--library", lib.to_str().unwrap(), "--loadout", loadout.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains("no conflicts"));

    // order: move id_b to position 0; the reported order lists it first.
    Command::cargo_bin("gore")
        .unwrap()
        .args(["mgr", "order", &id_b, "0", "--loadout", loadout.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicates::str::contains(&id_b))
        .stdout(predicates::str::contains("position 0"));

    // status on an empty game tree: nothing was ever deployed here.
    let game = tmp.path().join("game");
    std::fs::create_dir_all(&game).unwrap();
    Command::cargo_bin("gore")
        .unwrap()
        .args(["mgr", "status", "--game", game.to_str().unwrap(), "--loadout", loadout.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicates::str::contains("nothing deployed"));

    // reset on a game that has nothing deployed runs clean and says so.
    Command::cargo_bin("gore")
        .unwrap()
        .args(["mgr", "reset", "--game", game.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicates::str::contains("nothing was deployed"));

    // remove a mod: library entry gone, dropped from the loadout.
    Command::cargo_bin("gore")
        .unwrap()
        .args([
            "mgr", "remove", &id_a, "--library", lib.to_str().unwrap(), "--loadout", loadout.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains("true"));
    assert!(!lib.join(&id_a).exists(), "removed entry dir must be gone");

    Command::cargo_bin("gore")
        .unwrap()
        .args(["mgr", "list", "--library", lib.to_str().unwrap(), "--loadout", loadout.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicates::str::contains(&id_b))
        .stdout(predicates::str::contains(&id_a).not());
}

/// Enabling / ordering an id that isn't in the loadout is a clean error, not a panic.
#[test]
fn mgr_enable_unknown_id_errors() {
    let tmp = TempDir::new().unwrap();
    let loadout = tmp.path().join("loadout.json");
    Command::cargo_bin("gore")
        .unwrap()
        .args(["mgr", "enable", "does-not-exist", "--loadout", loadout.to_str().unwrap()])
        .assert()
        .failure()
        .stderr(predicates::str::contains("does-not-exist"));
}

/// Full apply against a temp game tree with a real, decodable `.lcache`. `#[ignore]`d
/// because it builds an encrypted localization fixture (the same AES-256-ECB shape
/// gore-loc/gore-mod use); run explicitly with `cargo test -p gore -- --ignored`.
#[test]
#[ignore]
fn mgr_apply_on_temp_game() {
    use aes::cipher::{generic_array::GenericArray, BlockEncrypt, KeyInit};
    use aes::Aes256;

    // The exact 32 ASCII key gore-loc uses so the fixture is a real decodable .lcache.
    const KEY: &[u8; 32] = b"8f93ff6fa254d9c536ad88c1ff1d812b";

    fn fstr(s: &str) -> Vec<u8> {
        // ASCII path only (fixture values are ASCII): i32 byte count, utf8+NUL.
        let mut raw = s.as_bytes().to_vec();
        raw.push(0);
        let mut out = (raw.len() as i32).to_le_bytes().to_vec();
        out.extend_from_slice(&raw);
        out
    }

    fn build_lcache(records: &[(&str, &str)]) -> Vec<u8> {
        let mut plain = Vec::new();
        plain.push(0u8); // prefix
        plain.extend_from_slice(&(b"LCACHE".len() as i32).to_le_bytes());
        plain.extend_from_slice(b"LCACHE");
        plain.extend_from_slice(&1i32.to_le_bytes()); // lang_count
        plain.extend_from_slice(&fstr("german"));
        plain.extend_from_slice(&(records.len() as i32).to_le_bytes()); // group_count
        for (key, val) in records {
            plain.extend_from_slice(&fstr(key));
            plain.extend_from_slice(&1i32.to_le_bytes());
            plain.extend_from_slice(&fstr("german"));
            plain.extend_from_slice(&fstr(val));
            plain.extend_from_slice(&fstr("")); // meta record: empty key
            plain.extend_from_slice(&0i32.to_le_bytes()); // no pairs
        }
        let pad = (16 - (plain.len() % 16)) % 16;
        plain.extend(std::iter::repeat(0u8).take(pad));
        let cipher = Aes256::new(GenericArray::from_slice(KEY));
        let mut ct = plain;
        for chunk in ct.chunks_mut(16) {
            cipher.encrypt_block(GenericArray::from_mut_slice(chunk));
        }
        ct
    }

    let tmp = TempDir::new().unwrap();
    let lib = tmp.path().join("library");
    let loadout = tmp.path().join("loadout.json");
    let built = tmp.path().join("built");

    // A minimal game tree with a pristine .lcache to patch against.
    let game = tmp.path().join("game");
    for p in [
        "G1R/Binaries/Win64/ue4ss/Mods",
        "G1R/Content/FMOD/Desktop",
        "G1R/Content/Paks/~mods",
        "G1R/Script",
        "G1R/Story/Cache",
    ] {
        std::fs::create_dir_all(game.join(p)).unwrap();
    }
    let lcache = game.join("G1R/Story/Cache/AlkimiaLocalization_0.lcache");
    std::fs::write(&lcache, build_lcache(&[("itfo_cheese", "Cheese"), ("itfo_apple", "Apple")])).unwrap();

    // One loc-editing bundle → import → enable.
    let spec = write_loc_spec(tmp.path(), "PatchMod", "itfo_cheese", "Gouda");
    let bundle = build_bundle(&built, "PatchMod", &spec);
    let id = import(&lib, &loadout, &bundle);
    Command::cargo_bin("gore")
        .unwrap()
        .args(["mgr", "enable", &id, "--loadout", loadout.to_str().unwrap()])
        .assert()
        .success();

    // apply: the enabled loadout deploys against the game.
    Command::cargo_bin("gore")
        .unwrap()
        .args([
            "mgr", "apply",
            "--game", game.to_str().unwrap(),
            "--library", lib.to_str().unwrap(),
            "--loadout", loadout.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains("PatchMod"));

    // The live .lcache now decodes with the patched value.
    let live = std::fs::read(&lcache).unwrap();
    let lc = gore_loc::loc::Lcache::decode(&live).unwrap();
    assert_eq!(
        lc.export(false).get("itfo_cheese").and_then(|m| m.get("german")).map(String::as_str),
        Some("Gouda"),
        "apply must patch the loc value"
    );

    // status now reports in-sync for the same target. Pass --library so the fingerprint check
    // reads the SAME library apply recorded against (its default would be the shared per-user one).
    Command::cargo_bin("gore")
        .unwrap()
        .args([
            "mgr", "status",
            "--game", game.to_str().unwrap(),
            "--library", lib.to_str().unwrap(),
            "--loadout", loadout.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains("in sync"));

    // reset restores pristine and reports it undeployed something.
    Command::cargo_bin("gore")
        .unwrap()
        .args(["mgr", "reset", "--game", game.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicates::str::contains("undeployed"));
    let restored = std::fs::read(&lcache).unwrap();
    let lc2 = gore_loc::loc::Lcache::decode(&restored).unwrap();
    assert_eq!(
        lc2.export(false).get("itfo_cheese").and_then(|m| m.get("german")).map(String::as_str),
        Some("Cheese"),
        "reset must restore the pristine value"
    );
}
