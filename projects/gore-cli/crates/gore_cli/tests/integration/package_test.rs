use assert_cmd::Command;
use std::path::PathBuf;
use tempfile::TempDir;
use zip::ZipArchive;

fn make_test_mod(tmp: &TempDir) -> PathBuf {
    let mod_dir = tmp.path().join("TestMod");
    std::fs::create_dir_all(mod_dir.join("Scripts")).unwrap();
    std::fs::write(mod_dir.join("enabled.txt"), "").unwrap();
    std::fs::write(mod_dir.join("Scripts/main.lua"), "-- test\napply()\n").unwrap();
    mod_dir
}

#[test]
fn package_creates_zip() {
    let tmp = TempDir::new().unwrap();
    let mod_dir = make_test_mod(&tmp);
    let out_zip = tmp.path().join("TestMod.zip");

    Command::cargo_bin("gore-cli")
        .unwrap()
        .args(["package", mod_dir.to_str().unwrap(), "-o", out_zip.to_str().unwrap()])
        .assert()
        .success();

    assert!(out_zip.exists(), "zip file must be created");
}

#[test]
fn package_zip_contains_required_files() {
    let tmp = TempDir::new().unwrap();
    let mod_dir = make_test_mod(&tmp);
    let out_zip = tmp.path().join("out.zip");

    Command::cargo_bin("gore-cli")
        .unwrap()
        .args(["package", mod_dir.to_str().unwrap(), "-o", out_zip.to_str().unwrap()])
        .assert()
        .success();

    let f = std::fs::File::open(&out_zip).unwrap();
    let mut zip = ZipArchive::new(f).unwrap();
    let names: Vec<String> = (0..zip.len())
        .map(|i| zip.by_index(i).unwrap().name().to_string())
        .collect();
    assert!(
        names.iter().any(|n| n.ends_with("enabled.txt")),
        "zip must contain enabled.txt; got: {names:?}"
    );
    assert!(
        names.iter().any(|n| n.ends_with("main.lua")),
        "zip must contain Scripts/main.lua; got: {names:?}"
    );
}

#[test]
fn package_fails_if_enabled_txt_missing() {
    let tmp = TempDir::new().unwrap();
    let mod_dir = tmp.path().join("BadMod");
    std::fs::create_dir_all(mod_dir.join("Scripts")).unwrap();
    // No enabled.txt, no main.lua — validation must reject
    Command::cargo_bin("gore-cli")
        .unwrap()
        .args(["package", mod_dir.to_str().unwrap(), "-o", tmp.path().join("bad.zip").to_str().unwrap()])
        .assert()
        .failure()
        .stderr(predicates::str::contains("enabled.txt"));
}

#[test]
fn package_fails_if_main_lua_missing() {
    let tmp = TempDir::new().unwrap();
    let mod_dir = tmp.path().join("NoLuaMod");
    std::fs::create_dir_all(mod_dir.join("Scripts")).unwrap();
    std::fs::write(mod_dir.join("enabled.txt"), "").unwrap();
    // Scripts/main.lua is missing
    Command::cargo_bin("gore-cli")
        .unwrap()
        .args(["package", mod_dir.to_str().unwrap(), "-o", tmp.path().join("nolua.zip").to_str().unwrap()])
        .assert()
        .failure()
        .stderr(predicates::str::contains("main.lua"));
}
