//! Integration tests for `gore config`. Each run isolates the shared dir by
//! pointing LOCALAPPDATA/APPDATA (Windows), HOME (macOS), and XDG_DATA_HOME/HOME
//! (Linux) at a TempDir, so tests never touch the real user config.

use assert_cmd::Command;
use std::path::Path;
use tempfile::TempDir;

/// A `gore` command with the shared dir redirected into `home`.
fn gore(home: &Path) -> Command {
    let mut cmd = Command::cargo_bin("gore").unwrap();
    cmd.env("LOCALAPPDATA", home)
        .env("APPDATA", home)
        .env("XDG_DATA_HOME", home)
        .env("HOME", home);
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
