use assert_cmd::Command;
use std::fs;
use tempfile::tempdir;

#[test]
fn deploy_shared_copies_tree_into_mods_shared() {
    let src = tempdir().unwrap();
    // a fake lua/shared/ tree
    fs::create_dir_all(src.path().join("gore-lua")).unwrap();
    fs::write(src.path().join("gore-lua/gore-lua.lua"), "return {}\n").unwrap();

    let game = tempdir().unwrap();
    let mods = game.path().join("ue4ss/Mods");
    fs::create_dir_all(&mods).unwrap();

    Command::cargo_bin("gore")
        .unwrap()
        .args([
            "deploy-shared",
            "--src",
            src.path().to_str().unwrap(),
            "--game",
            game.path().to_str().unwrap(),
        ])
        .assert()
        .success();

    let dest = mods.join("shared/gore-lua/gore-lua.lua");
    assert!(dest.exists(), "gore-lua.lua should be copied to Mods/shared/");
    assert_eq!(fs::read_to_string(dest).unwrap(), "return {}\n");
}
