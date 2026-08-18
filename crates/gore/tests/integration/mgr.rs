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

/// Write a BuildSpec for one additive `pak_files` payload. This stays entirely inside the test's
/// temporary tree; the resulting manager apply only writes to a synthetic game root.
fn write_pak_file_spec(dir: &Path, name: &str, game_path: &str, value: &[u8]) -> PathBuf {
    let payload = dir.join(format!("{name}.bin"));
    std::fs::write(&payload, value).unwrap();
    let spec = serde_json::json!({
        "meta": { "name": name, "version": "2.0", "author": "test" },
        "pak_files": [{
            "game_path": game_path,
            "source_path": payload.display().to_string(),
        }],
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
        .args([
            "mod",
            "build",
            "--spec",
            spec.to_str().unwrap(),
            "-o",
            out.to_str().unwrap(),
        ])
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
    let line = text
        .lines()
        .find(|l| l.starts_with("imported "))
        .expect("import line");
    line.split_whitespace()
        .nth(1)
        .expect("id token")
        .to_string()
}

#[test]
fn mgr_import_keeps_prefix_and_reports_structured_success_outcome() {
    let tmp = TempDir::new().unwrap();
    let library = tmp.path().join("library");
    let loadout = tmp.path().join("loadout.json");
    let a_dir = tmp.path().join("a");
    let b_dir = tmp.path().join("b");
    std::fs::create_dir(&a_dir).unwrap();
    std::fs::create_dir(&b_dir).unwrap();
    let a = a_dir.join("same_P.pak");
    let b = b_dir.join("same_P.pak");
    std::fs::write(&a, b"opaque bytes").unwrap();
    std::fs::write(&b, b"opaque bytes").unwrap();

    let run = |source: &Path| {
        let output = Command::cargo_bin("gore")
            .unwrap()
            .args([
                "mgr",
                "import",
                source.to_str().unwrap(),
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
        String::from_utf8(output).unwrap()
    };
    let id = |text: &str| {
        text.lines()
            .find(|line| line.starts_with("imported "))
            .unwrap()
            .split_whitespace()
            .nth(1)
            .unwrap()
            .to_owned()
    };

    let created = run(&a);
    assert!(created.starts_with("imported "), "{created}");
    assert!(created.contains("disposition=created"), "{created}");
    assert!(created.contains("matched_by=none"), "{created}");

    let unchanged = run(&a);
    assert_eq!(id(&unchanged), id(&created));
    assert!(unchanged.contains("disposition=unchanged"), "{unchanged}");
    assert!(unchanged.contains("matched_by=source"), "{unchanged}");

    let moved = run(&b);
    assert_eq!(id(&moved), id(&created));
    assert!(moved.contains("disposition=updated"), "{moved}");
    assert!(moved.contains("matched_by=content"), "{moved}");
}

#[test]
fn mgr_accepts_child_loadout_path_relative_to_the_process_cwd() {
    let tmp = TempDir::new().unwrap();
    Command::cargo_bin("gore")
        .unwrap()
        .current_dir(tmp.path())
        .args([
            "mgr",
            "list",
            "--library",
            "library",
            "--loadout",
            "loadout.json",
        ])
        .assert()
        .success();
    assert!(tmp.path().join("library").is_dir());
}

#[test]
fn mgr_rejects_unpaired_loadout_override_before_reconciliation() {
    let tmp = TempDir::new().unwrap();
    let data = tmp.path().join("data");
    let default_library = data.join("gore/mod-manager/library");
    std::fs::create_dir_all(&default_library).unwrap();
    let loadout = tmp.path().join("custom-loadout.json");
    let bytes = br#"{"format":1,"entries":[{"id":"custom","enabled":true}]}"#;
    std::fs::write(&loadout, bytes).unwrap();

    for args in [
        vec!["mgr", "enable", "custom", "--loadout"],
        vec!["mgr", "disable", "custom", "--loadout"],
        vec!["mgr", "order", "custom", "0", "--loadout"],
    ] {
        Command::cargo_bin("gore")
            .unwrap()
            .env("LOCALAPPDATA", &data)
            .env("APPDATA", &data)
            .env("XDG_DATA_HOME", &data)
            .args(args)
            .arg(&loadout)
            .assert()
            .failure()
            .stderr(predicates::str::contains(
                "--library and --loadout overrides must be supplied together",
            ));
        assert_eq!(std::fs::read(&loadout).unwrap(), bytes);
        assert_eq!(std::fs::read_dir(&default_library).unwrap().count(), 0);
    }
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
            "mgr",
            "list",
            "--library",
            lib.to_str().unwrap(),
            "--loadout",
            loadout.to_str().unwrap(),
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
            .args([
                "mgr",
                "enable",
                id,
                "--library",
                lib.to_str().unwrap(),
                "--loadout",
                loadout.to_str().unwrap(),
            ])
            .assert()
            .success()
            .stdout(predicates::str::contains("enabled"));
    }

    // list now shows an enabled marker.
    Command::cargo_bin("gore")
        .unwrap()
        .args([
            "mgr",
            "list",
            "--library",
            lib.to_str().unwrap(),
            "--loadout",
            loadout.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains("[x]"));

    // analyze: both enabled + same loc id → a conflict line naming both ids and the winner.
    Command::cargo_bin("gore")
        .unwrap()
        .args([
            "mgr",
            "analyze",
            "--library",
            lib.to_str().unwrap(),
            "--loadout",
            loadout.to_str().unwrap(),
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
        .args([
            "mgr",
            "disable",
            &id_b,
            "--library",
            lib.to_str().unwrap(),
            "--loadout",
            loadout.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains("disabled"));

    Command::cargo_bin("gore")
        .unwrap()
        .args([
            "mgr",
            "analyze",
            "--library",
            lib.to_str().unwrap(),
            "--loadout",
            loadout.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains("no conflicts"));

    // order: move id_b to position 0; the reported order lists it first.
    Command::cargo_bin("gore")
        .unwrap()
        .args([
            "mgr",
            "order",
            &id_b,
            "0",
            "--library",
            lib.to_str().unwrap(),
            "--loadout",
            loadout.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains(&id_b))
        .stdout(predicates::str::contains("position 0"));

    // status on an empty game tree: nothing was ever deployed here.
    let game = tmp.path().join("game");
    std::fs::create_dir_all(&game).unwrap();
    Command::cargo_bin("gore")
        .unwrap()
        .args([
            "mgr",
            "status",
            "--game",
            game.to_str().unwrap(),
            "--library",
            lib.to_str().unwrap(),
            "--loadout",
            loadout.to_str().unwrap(),
        ])
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
            "mgr",
            "remove",
            &id_a,
            "--library",
            lib.to_str().unwrap(),
            "--loadout",
            loadout.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains("true"));
    assert!(!lib.join(&id_a).exists(), "removed entry dir must be gone");

    Command::cargo_bin("gore")
        .unwrap()
        .args([
            "mgr",
            "list",
            "--library",
            lib.to_str().unwrap(),
            "--loadout",
            loadout.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains(&id_b))
        .stdout(predicates::str::contains(&id_a).not());
}

/// Enabling / ordering an id that isn't in the loadout is a clean error, not a panic.
#[test]
fn mgr_enable_unknown_id_errors() {
    let tmp = TempDir::new().unwrap();
    let library = tmp.path().join("library");
    let loadout = tmp.path().join("loadout.json");
    Command::cargo_bin("gore")
        .unwrap()
        .args([
            "mgr",
            "enable",
            "does-not-exist",
            "--library",
            library.to_str().unwrap(),
            "--loadout",
            loadout.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(predicates::str::contains("does-not-exist"));
}

/// Real CLI path for format-2 additive file mods: build -> manager import/loadout -> analyze ->
/// apply -> reorder -> reapply -> reset. All writes land in a temporary synthetic game tree. The
/// archive assertions prove deterministic manager slot/filesystem behavior, not Unreal mount
/// priority, gameplay, installation support, or runtime behavior.
#[test]
fn mgr_pak_file_patch_reorders_and_resets_in_a_temp_game() {
    let tmp = TempDir::new().unwrap();
    let lib = tmp.path().join("library");
    let loadout = tmp.path().join("loadout.json");
    let built = tmp.path().join("built");
    let game = tmp.path().join("game");
    let mods = game.join("G1R/Content/Paks/~mods");
    std::fs::create_dir_all(&mods).unwrap();
    let unrelated = mods.join("user-owned.pak");
    std::fs::write(&unrelated, b"KEEP").unwrap();

    let target = "G1R/Content/Slate/Cursors/Normal/Normal.PNG";
    let alpha_spec = write_pak_file_spec(tmp.path(), "AlphaPak", target, b"ALPHA-CURSOR");
    let bravo_spec = write_pak_file_spec(tmp.path(), "BravoPak", target, b"BRAVO-CURSOR");
    let alpha_bundle = build_bundle(&built, "AlphaPak", &alpha_spec);
    let bravo_bundle = build_bundle(&built, "BravoPak", &bravo_spec);
    let alpha = import(&lib, &loadout, &alpha_bundle);
    let bravo = import(&lib, &loadout, &bravo_bundle);

    for id in [&alpha, &bravo] {
        Command::cargo_bin("gore")
            .unwrap()
            .args([
                "mgr",
                "enable",
                id,
                "--library",
                lib.to_str().unwrap(),
                "--loadout",
                loadout.to_str().unwrap(),
            ])
            .assert()
            .success();
    }
    Command::cargo_bin("gore")
        .unwrap()
        .args([
            "mgr",
            "analyze",
            "--library",
            lib.to_str().unwrap(),
            "--loadout",
            loadout.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains(target.to_ascii_lowercase()))
        .stdout(predicates::str::contains(&alpha))
        .stdout(predicates::str::contains(&bravo))
        .stdout(predicates::str::contains("winner"));

    let apply = || {
        Command::cargo_bin("gore")
            .unwrap()
            .args([
                "mgr",
                "apply",
                "--game",
                game.to_str().unwrap(),
                "--library",
                lib.to_str().unwrap(),
                "--loadout",
                loadout.to_str().unwrap(),
            ])
            .assert()
            .success()
            .stdout(predicates::str::contains("applied 2 mod(s)"));
    };
    let manager_paks = || {
        let mut paths = std::fs::read_dir(&mods)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| {
                        name.starts_with("zzz_gm")
                            && name.contains("_files_")
                            && name.ends_with("_P.pak")
                    })
            })
            .collect::<Vec<_>>();
        paths.sort();
        paths
    };
    let find_slot = |paths: &[PathBuf], slot: usize, id: &str| {
        let prefix = format!("zzz_gm{slot:03}_{id}_");
        paths
            .iter()
            .find(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.starts_with(&prefix))
            })
            .cloned()
            .unwrap_or_else(|| panic!("missing {prefix:?} in {paths:?}"))
    };
    let patch_priority = |path: &Path| {
        path.file_stem()
            .and_then(|name| name.to_str())
            .and_then(|name| name.strip_suffix("_P"))
            .and_then(|name| name.rsplit_once('_'))
            .and_then(|(_, priority)| priority.parse::<usize>().ok())
            .unwrap_or_else(|| panic!("missing numeric patch priority in {}", path.display()))
    };

    apply();
    let first = manager_paks();
    assert_eq!(first.len(), 2);
    let alpha_slot_0 = find_slot(&first, 0, &alpha);
    let bravo_slot_1 = find_slot(&first, 1, &bravo);
    assert_eq!(patch_priority(&alpha_slot_0), 1);
    assert_eq!(patch_priority(&bravo_slot_1), 2);
    let alpha_archive = std::fs::read(&alpha_slot_0).unwrap();
    let bravo_archive = std::fs::read(&bravo_slot_1).unwrap();
    assert_ne!(alpha_archive, bravo_archive);
    for pak in [&alpha_slot_0, &bravo_slot_1] {
        assert_eq!(
            gore_tex::container::list_pak_files(pak).unwrap(),
            vec![target.to_string()]
        );
    }

    Command::cargo_bin("gore")
        .unwrap()
        .args([
            "mgr",
            "order",
            &bravo,
            "0",
            "--library",
            lib.to_str().unwrap(),
            "--loadout",
            loadout.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains("position 0"));
    apply();
    let reordered = manager_paks();
    assert_eq!(reordered.len(), 2);
    let bravo_slot_0 = find_slot(&reordered, 0, &bravo);
    let alpha_slot_1 = find_slot(&reordered, 1, &alpha);
    assert_eq!(patch_priority(&bravo_slot_0), 1);
    assert_eq!(patch_priority(&alpha_slot_1), 2);
    assert_eq!(std::fs::read(&bravo_slot_0).unwrap(), bravo_archive);
    assert_eq!(std::fs::read(&alpha_slot_1).unwrap(), alpha_archive);
    assert!(!alpha_slot_0.exists(), "old alpha slot must be removed");
    assert!(!bravo_slot_1.exists(), "old bravo slot must be removed");

    Command::cargo_bin("gore")
        .unwrap()
        .args(["mgr", "reset", "--game", game.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicates::str::contains("undeployed"));
    assert!(manager_paks().is_empty());
    assert_eq!(std::fs::read(&unrelated).unwrap(), b"KEEP");
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
        plain.extend(std::iter::repeat_n(0u8, pad));
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
    std::fs::write(
        &lcache,
        build_lcache(&[("itfo_cheese", "Cheese"), ("itfo_apple", "Apple")]),
    )
    .unwrap();

    // One loc-editing bundle → import → enable.
    let spec = write_loc_spec(tmp.path(), "PatchMod", "itfo_cheese", "Gouda");
    let bundle = build_bundle(&built, "PatchMod", &spec);
    let id = import(&lib, &loadout, &bundle);
    Command::cargo_bin("gore")
        .unwrap()
        .args([
            "mgr",
            "enable",
            &id,
            "--library",
            lib.to_str().unwrap(),
            "--loadout",
            loadout.to_str().unwrap(),
        ])
        .assert()
        .success();

    // apply: the enabled loadout deploys against the game.
    Command::cargo_bin("gore")
        .unwrap()
        .args([
            "mgr",
            "apply",
            "--game",
            game.to_str().unwrap(),
            "--library",
            lib.to_str().unwrap(),
            "--loadout",
            loadout.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains("PatchMod"));

    // The live .lcache now decodes with the patched value.
    let live = std::fs::read(&lcache).unwrap();
    let lc = gore_loc::loc::Lcache::decode(&live).unwrap();
    assert_eq!(
        lc.export(false)
            .get("itfo_cheese")
            .and_then(|m| m.get("german"))
            .map(String::as_str),
        Some("Gouda"),
        "apply must patch the loc value"
    );

    // status now reports in-sync for the same target. Pass --library so the fingerprint check
    // reads the SAME library apply recorded against (its default would be the shared per-user one).
    Command::cargo_bin("gore")
        .unwrap()
        .args([
            "mgr",
            "status",
            "--game",
            game.to_str().unwrap(),
            "--library",
            lib.to_str().unwrap(),
            "--loadout",
            loadout.to_str().unwrap(),
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
        lc2.export(false)
            .get("itfo_cheese")
            .and_then(|m| m.get("german"))
            .map(String::as_str),
        Some("Cheese"),
        "reset must restore the pristine value"
    );
}
