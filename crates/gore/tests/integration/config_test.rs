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
    gore(tmp.path()).args(["config", "set", "game-path", "X"]).assert().success();
    gore(tmp.path()).args(["config", "unset", "game-path"]).assert().success();
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
