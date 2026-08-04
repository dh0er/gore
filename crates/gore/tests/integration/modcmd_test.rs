//! Integration tests for `gore mod build`, driving the real binary.
//!
//! Only this tier can prove the thing that actually broke: a spec whose assets sit beside it,
//! built from a working directory that is NOT the spec's directory. A unit test passes the base
//! in explicitly, so it cannot tell a cwd-relative implementation apart from a spec-relative one.
//! An agent driving the MCP server is in exactly this position — the server spawns `gore` without
//! choosing a working directory, so the child inherits one the user never picked and cannot see.
//!
//! The five environment variables mirror `config_test.rs`, so a developer's real installation is
//! never consulted.

use assert_cmd::Command;
use std::path::Path;
use tempfile::TempDir;

fn gore(home: &Path) -> Command {
    let mut cmd = Command::cargo_bin("gore").unwrap();
    cmd.env("LOCALAPPDATA", home)
        .env("APPDATA", home)
        .env("XDG_DATA_HOME", home)
        .env("HOME", home)
        .env("GORE_DISABLE_GAME_AUTODETECT", "1");
    cmd
}

/// Write `spec.json` plus its `click.wav` into a fresh subdirectory and return it.
fn spec_dir_with_asset(root: &Path, wav: Option<&[u8]>) -> std::path::PathBuf {
    let dir = root.join("authoring");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("spec.json"),
        br#"{
  "meta": { "name": "MyMod", "version": "1.0.0", "author": "tester" },
  "audio": [ { "bank": "SFX.bank", "sample": "Click", "wav_path": "click.wav" } ]
}
"#,
    )
    .unwrap();
    if let Some(bytes) = wav {
        std::fs::write(dir.join("click.wav"), bytes).unwrap();
    }
    dir
}

/// The same spec, with the audio bank field written the way a user who copied it out of a file
/// picker writes it. Everything else about it is valid.
fn spec_dir_with_bank(root: &Path, bank: &str) -> std::path::PathBuf {
    let dir = root.join("authoring");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("spec.json"),
        format!(
            r#"{{
  "meta": {{ "name": "MyMod", "version": "1.0.0", "author": "tester" }},
  "audio": [ {{ "bank": "{bank}", "sample": "Click", "wav_path": "click.wav" }} ]
}}
"#
        ),
    )
    .unwrap();
    std::fs::write(dir.join("click.wav"), b"WAV-BYTES").unwrap();
    dir
}

#[test]
fn a_bank_written_as_a_full_path_fails_the_build_instead_of_the_deploy() {
    // What this rules out: `build` printing "built bundle: … (4 components, 9 files)" for a spec
    // the deploy planner will always refuse. That ordering cost a real session four calls — the
    // deploy, the edit, the removal of the half-useful bundle, the rebuild — and the refusal it
    // eventually printed said only "unsafe bank name", which reads as a security verdict rather
    // than as "this field takes a bare file name".
    let tmp = TempDir::new().unwrap();
    let spec_dir = spec_dir_with_bank(
        tmp.path(),
        r"D:\\SteamLibrary\\steamapps\\common\\Gothic 1 Remake\\G1R\\Content\\FMOD\\Desktop\\Music.bank",
    );
    let out = tmp.path().join("out");

    let output = gore(tmp.path())
        .arg("mod")
        .arg("build")
        .arg("--spec")
        .arg(spec_dir.join("spec.json"))
        .arg("-o")
        .arg(&out)
        .assert()
        .failure()
        .get_output()
        .stderr
        .clone();
    let stderr = String::from_utf8_lossy(&output);

    assert!(
        stderr.contains("Music.bank"),
        "the failure must quote the value it refused: {stderr}"
    );
    assert!(
        stderr.contains("G1R/Content/FMOD/Desktop") && stderr.contains("SFX.bank"),
        "the failure must name the constraint and a spelling that works: {stderr}"
    );
    assert!(
        !out.exists(),
        "a refused spec must leave no bundle behind for a deploy to be tried against"
    );
}

#[test]
fn a_spec_relative_asset_is_found_from_an_unrelated_working_directory() {
    let tmp = TempDir::new().unwrap();
    let spec_dir = spec_dir_with_asset(tmp.path(), Some(b"WAV-BYTES"));
    let elsewhere = tmp.path().join("cwd");
    let out = tmp.path().join("out");
    std::fs::create_dir_all(&elsewhere).unwrap();

    gore(tmp.path())
        .current_dir(&elsewhere)
        .arg("mod")
        .arg("build")
        .arg("--spec")
        .arg(spec_dir.join("spec.json"))
        .arg("-o")
        .arg(&out)
        .assert()
        .success();

    assert_eq!(
        std::fs::read(out.join("MyMod/audio/0_SFX_bank__Click.wav")).unwrap(),
        b"WAV-BYTES",
        "the asset beside the spec must be the asset that lands in the bundle"
    );
}

#[test]
fn a_missing_asset_names_the_spec_and_the_path_that_was_actually_opened() {
    // Before spec-relative resolution the message was a bare `click.wav` — which was neither the
    // path that was opened nor a hint about where the tool had looked.
    let tmp = TempDir::new().unwrap();
    let spec_dir = spec_dir_with_asset(tmp.path(), None);
    let elsewhere = tmp.path().join("cwd");
    std::fs::create_dir_all(&elsewhere).unwrap();

    let output = gore(tmp.path())
        .current_dir(&elsewhere)
        .arg("mod")
        .arg("build")
        .arg("--spec")
        .arg(spec_dir.join("spec.json"))
        .arg("-o")
        .arg(tmp.path().join("out"))
        .assert()
        .failure()
        .get_output()
        .stderr
        .clone();
    let stderr = String::from_utf8_lossy(&output).replace('\\', "/");

    let spec_path = spec_dir
        .join("spec.json")
        .display()
        .to_string()
        .replace('\\', "/");
    let resolved = spec_dir
        .join("click.wav")
        .display()
        .to_string()
        .replace('\\', "/");
    assert!(
        stderr.contains(&spec_path),
        "the failure must name the spec it came from: {stderr}"
    );
    assert!(
        stderr.contains(&resolved),
        "the failure must name the resolved asset path: {stderr}"
    );
    assert!(
        stderr.contains("audio[0]"),
        "the failure must name the section and index: {stderr}"
    );
}
