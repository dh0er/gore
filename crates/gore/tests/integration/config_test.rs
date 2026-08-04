//! Integration tests for `gore config`. Each run isolates the shared dir by
//! pointing LOCALAPPDATA/APPDATA (Windows), HOME (macOS), and XDG_DATA_HOME/HOME
//! (Linux) at a TempDir, so tests never touch the real user config.

use assert_cmd::Command;
use predicates::prelude::PredicateBooleanExt; // `.not()`
use std::path::Path;
use tempfile::TempDir;

/// A `gore` command with the shared dir redirected into `home` and Steam
/// auto-detect disabled, so resolution depends only on explicit/configured paths.
fn gore(home: &Path) -> Command {
    let mut cmd = Command::cargo_bin("gore").unwrap();
    cmd.env("LOCALAPPDATA", home)
        .env("APPDATA", home)
        .env("XDG_DATA_HOME", home)
        .env("HOME", home)
        .env("GORE_DISABLE_GAME_AUTODETECT", "1");
    cmd
}

#[test]
fn set_then_get_round_trips_game_path() {
    let tmp = TempDir::new().unwrap();
    gore(tmp.path())
        .args(["config", "set", "game-path", "D:/Games/G1R"])
        .assert()
        .success();
    gore(tmp.path())
        .args(["config", "get", "game-path"])
        .assert()
        .success()
        .stdout(predicates::str::contains("D:/Games/G1R"));
}

#[test]
fn config_path_prints_config_json_under_gore() {
    let tmp = TempDir::new().unwrap();
    gore(tmp.path())
        .args(["config", "path"])
        .assert()
        .success()
        .stdout(predicates::str::contains("config.json"));
}

#[test]
fn unset_clears_the_value() {
    let tmp = TempDir::new().unwrap();
    gore(tmp.path())
        .args(["config", "set", "game-path", "X"])
        .assert()
        .success();
    gore(tmp.path())
        .args(["config", "unset", "game-path"])
        .assert()
        .success();
    gore(tmp.path())
        .args(["config", "get", "game-path"])
        .assert()
        .failure(); // unset key exits non-zero
}

#[test]
fn mgr_status_uses_configured_game_path() {
    let tmp = TempDir::new().unwrap();
    // A fake game root so normalize_root resolves and the command reaches its
    // own logic instead of erroring on an unresolved path.
    let game = tmp.path().join("Game");
    std::fs::create_dir_all(game.join("G1R")).unwrap();
    gore(tmp.path())
        .args(["config", "set", "game-path", game.to_str().unwrap()])
        .assert()
        .success();
    // `mgr status` with NO --game must not fail with the "no game path" error.
    gore(tmp.path())
        .args(["mgr", "status"])
        .assert()
        .stderr(predicates::str::contains("no game path set").not());
}

#[test]
fn mgr_status_errors_helpfully_when_unset() {
    let tmp = TempDir::new().unwrap();
    // No config, and auto-detect disabled -> nothing resolves.
    gore(tmp.path())
        .args(["mgr", "status"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("no game path set"));
}

#[test]
fn loc_extract_honors_disabled_autodetect() {
    let tmp = TempDir::new().unwrap();
    // No configured game-path and autodetect disabled (the `gore` helper sets
    // GORE_DISABLE_GAME_AUTODETECT=1): `loc extract` must NOT fall through to a
    // Steam scan and reach an install the caller excluded. It must fail cleanly
    // with the not-found message. (On a machine WITH the game installed, an
    // ungated Steam fallback would instead succeed — so asserting `.failure()`
    // is a real guard on the seam.)
    gore(tmp.path())
        .args(["loc", "extract", "-y"])
        .assert()
        .failure()
        .stderr(predicates::str::contains(
            "no AlkimiaLocalization .lcache found",
        ));
}

#[test]
fn loc_export_and_import_find_the_lcache_the_way_loc_extract_does() {
    // Before this, `--lcache` was required on export and import and optional only on extract.
    // A session that reached for `export` therefore had to know that the cache is called
    // `AlkimiaLocalization_00000000.lcache` and lives under `G1R\Story\Cache` — a path no
    // command prints and the guide spelled wrong. It cost three calls to find by hand.
    //
    // With autodetect disabled and no configured game path there is nothing to find, so the
    // proof that the flag is optional is WHICH failure comes back: the resolver's, not clap's
    // "the following required arguments were not provided".
    let tmp = TempDir::new().unwrap();
    let edits = tmp.path().join("edits.json");
    std::fs::write(&edits, b"{}").unwrap();

    for args in [
        vec!["loc", "export", "-o", "loc.json"],
        vec!["loc", "import", "--edits", edits.to_str().unwrap()],
    ] {
        gore(tmp.path())
            .current_dir(tmp.path())
            .args(&args)
            .assert()
            .failure()
            .stderr(predicates::str::contains(
                "no AlkimiaLocalization .lcache found",
            ));
    }
}

#[test]
fn set_stores_relative_game_path_as_absolute() {
    let tmp = TempDir::new().unwrap();
    // `set game-path .` run from cwd=tmp must persist an ABSOLUTE path, not the
    // literal ".", so a later command run from any other directory resolves the
    // same install rather than a stray ./G1R relative to its own cwd.
    gore(tmp.path())
        .current_dir(tmp.path())
        .args(["config", "set", "game-path", "."])
        .assert()
        .success();
    let out = gore(tmp.path())
        .args(["config", "get", "game-path"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let got = String::from_utf8(out).unwrap();
    assert!(
        std::path::Path::new(got.trim()).is_absolute(),
        "stored game-path is not absolute: {got:?}"
    );
}
