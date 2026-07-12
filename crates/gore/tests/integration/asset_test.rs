use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use predicates::prelude::*;
use retoc::legacy_asset::{
    EPackageFlags, FLegacyPackageFileSummary, FLegacyPackageHeader, FObjectExport, FObjectImport,
};
use retoc::logging::Log;
use retoc::version::EngineVersion;
use retoc::zen::FPackageIndex;
use retoc::{EIoChunkType, FIoChunkId, FIoContainerId, FPackageId};
use serde_json::Value;
use sha2::{Digest, Sha256};
use tempfile::TempDir;

const EXPORT_BYTES: [u8; 3] = [0x00, 0x03, 0x01];

struct Fixture {
    uasset: PathBuf,
    uexp: PathBuf,
    usmap: PathBuf,
    original_uasset: Vec<u8>,
    original_uexp: Vec<u8>,
}

#[test]
fn asset_help_exposes_offline_extract_inspect_patch_and_pack() {
    Command::cargo_bin("gore")
        .unwrap()
        .args(["asset", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("extract"))
        .stdout(predicate::str::contains("inspect"))
        .stdout(predicate::str::contains("patch-fixed"))
        .stdout(predicate::str::contains("pack"));
}

#[test]
fn extract_and_pack_require_unambiguous_safe_arguments() {
    Command::cargo_bin("gore")
        .unwrap()
        .args(["asset", "extract"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("--game"))
        .stderr(predicate::str::contains("--asset"))
        .stderr(predicate::str::contains("--out"));

    Command::cargo_bin("gore")
        .unwrap()
        .args(["asset", "pack"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("--game"))
        .stderr(predicate::str::contains("--uasset"))
        .stderr(predicate::str::contains("--patch-receipt"))
        .stderr(predicate::str::contains("--asset"))
        .stderr(predicate::str::contains("--name"))
        .stderr(predicate::str::contains("--out"));
}

#[test]
fn extract_and_pack_reject_existing_output_before_writing() {
    let temp = TempDir::new().unwrap();
    let game = temp.path().join("Game");
    fs::create_dir_all(game.join("G1R")).unwrap();
    let out = temp.path().join("Existing");
    fs::create_dir(&out).unwrap();
    let sentinel = out.join("keep.txt");
    fs::write(&sentinel, b"untouched").unwrap();

    Command::cargo_bin("gore")
        .unwrap()
        .args(["asset", "extract", "--game"])
        .arg(&game)
        .args(["--asset", "/Game/Test/DA_Fixture", "--out"])
        .arg(&out)
        .arg("--json")
        .assert()
        .failure()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains("ASSET_EXTRACT_OUTPUT"));
    assert_eq!(fs::read(&sentinel).unwrap(), b"untouched");

    Command::cargo_bin("gore")
        .unwrap()
        .args(["asset", "pack", "--game"])
        .arg(&game)
        .arg("--uasset")
        .arg(temp.path().join("missing.uasset"))
        .arg("--patch-receipt")
        .arg(temp.path().join("missing-patch-receipt.json"))
        .args([
            "--asset",
            "/Game/Test/DA_Fixture",
            "--name",
            "zzz_Test_P",
            "--out",
        ])
        .arg(&out)
        .arg("--json")
        .assert()
        .failure()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains("ASSET_PACK_OUTPUT"));
    assert_eq!(fs::read(&sentinel).unwrap(), b"untouched");
}

#[test]
fn extract_rejects_virtual_path_traversal_with_clean_stdout() {
    let temp = TempDir::new().unwrap();
    Command::cargo_bin("gore")
        .unwrap()
        .args(["asset", "extract", "--game"])
        .arg(temp.path())
        .args(["--asset", "/Game/Test/../DA_Fixture", "--out"])
        .arg(temp.path().join("out"))
        .arg("--json")
        .assert()
        .failure()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains("ASSET_EXTRACT_ASSET"));
}

#[test]
fn extract_rejects_reserved_or_overdeep_virtual_paths_without_staging() {
    let temp = TempDir::new().unwrap();
    let overdeep = format!(
        "/Game/{}/Leaf",
        std::iter::repeat_n("Segment", 32)
            .collect::<Vec<_>>()
            .join("/")
    );
    for asset in ["/Game/CON/DA_Fixture".to_owned(), overdeep] {
        let out = temp.path().join("must-not-exist");
        Command::cargo_bin("gore")
            .unwrap()
            .args(["asset", "extract", "--game"])
            .arg(temp.path())
            .args(["--asset", &asset, "--out"])
            .arg(&out)
            .arg("--json")
            .assert()
            .failure()
            .stdout(predicate::str::is_empty())
            .stderr(predicate::str::contains("ASSET_EXTRACT_ASSET"));
        assert!(!out.exists());
    }
    let leftovers: Vec<_> = fs::read_dir(temp.path())
        .unwrap()
        .filter_map(Result::ok)
        .filter(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .starts_with(".gore-asset-")
        })
        .collect();
    assert!(leftovers.is_empty());
}

#[test]
fn conversion_failures_leave_no_output_or_staging_directory() {
    let temp = TempDir::new().unwrap();
    let game = temp.path().join("FakeGame");
    let paks = game.join("G1R/Content/Paks");
    let mappings = game.join("G1R/Binaries/Win64/ue4ss");
    fs::create_dir_all(&paks).unwrap();
    fs::create_dir_all(&mappings).unwrap();
    fs::write(paks.join("G1R-Windows.utoc"), b"not an iostore").unwrap();
    fs::write(paks.join("G1R-Windows.ucas"), b"not an iostore").unwrap();
    fs::write(paks.join("global.utoc"), b"not an iostore").unwrap();
    fs::write(paks.join("global.ucas"), b"not an iostore").unwrap();
    fs::write(mappings.join("fixture.usmap"), b"not a usmap").unwrap();

    let extract_out = temp.path().join("extract-must-not-exist");
    Command::cargo_bin("gore")
        .unwrap()
        .args(["asset", "extract", "--game"])
        .arg(&game)
        .args(["--asset", "/Game/Test/DA_Fixture", "--out"])
        .arg(&extract_out)
        .arg("--json")
        .assert()
        .failure()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains("ASSET_EXTRACT_CONVERT"));
    assert!(!extract_out.exists());

    let fixture = write_fixture(temp.path());
    let pack_out = temp.path().join("pack-must-not-exist");
    let patch_receipt =
        write_synthetic_patch_receipt(&fixture.uasset, &fixture.usmap, "/Game/Test/DA_Fixture");
    Command::cargo_bin("gore")
        .unwrap()
        .args(["asset", "pack", "--game"])
        .arg(&game)
        .arg("--uasset")
        .arg(&fixture.uasset)
        .arg("--patch-receipt")
        .arg(&patch_receipt)
        .args([
            "--asset",
            "/Game/Test/DA_Fixture",
            "--name",
            "zzz_Fixture_P",
            "--out",
        ])
        .arg(&pack_out)
        .arg("--json")
        .assert()
        .failure()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains("ASSET_PACK_GENERATION"));
    assert!(!pack_out.exists());
    let leftovers: Vec<_> = fs::read_dir(temp.path())
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .filter(|name| name.to_string_lossy().starts_with(".gore-asset-"))
        .collect();
    assert!(leftovers.is_empty(), "staging leftovers: {leftovers:?}");
}

#[test]
fn extract_and_pack_refuse_active_subdirectory_containers_and_extension_aliases() {
    let temp = TempDir::new().unwrap();
    let game = temp.path().join("Game");
    let paks = game.join("G1R/Content/Paks");
    let mods = paks.join("~mods");
    fs::create_dir_all(&mods).unwrap();
    for extension in ["utoc", "ucas", "pak"] {
        fs::write(mods.join(format!("zzz_Override_P.{extension}")), b"active").unwrap();
    }

    let extract_out = temp.path().join("extract-must-not-exist");
    Command::cargo_bin("gore")
        .unwrap()
        .args(["asset", "extract", "--game"])
        .arg(&game)
        .args(["--asset", "/Game/Test/DA_Fixture", "--out"])
        .arg(&extract_out)
        .arg("--json")
        .assert()
        .failure()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains("active game-mountable container"))
        .stderr(predicate::str::contains("undeploy every mod"))
        .stderr(predicate::str::contains("ASSET_EXTRACT_CONVERT").not());
    assert!(!extract_out.exists());

    let pack_out = temp.path().join("pack-must-not-exist");
    Command::cargo_bin("gore")
        .unwrap()
        .args(["asset", "pack", "--game"])
        .arg(&game)
        .args([
            "--uasset",
            "missing.uasset",
            "--patch-receipt",
            "missing.json",
        ])
        .args([
            "--asset",
            "/Game/Test/DA_Fixture",
            "--name",
            "zzz_Fixture_P",
            "--out",
        ])
        .arg(&pack_out)
        .arg("--json")
        .assert()
        .failure()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains("active game-mountable container"))
        .stderr(predicate::str::contains("ASSET_PACK_GENERATION").not());
    assert!(!pack_out.exists());

    fs::remove_dir_all(&mods).unwrap();
    fs::write(paks.join("CaseAlias.UTOC"), b"alias").unwrap();
    let alias_out = temp.path().join("alias-must-not-exist");
    Command::cargo_bin("gore")
        .unwrap()
        .args(["asset", "extract", "--game"])
        .arg(&game)
        .args(["--asset", "/Game/Test/DA_Fixture", "--out"])
        .arg(&alias_out)
        .arg("--json")
        .assert()
        .failure()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains(
            "noncanonical IoStore extension casing",
        ))
        .stderr(predicate::str::contains(".UTOC"))
        .stderr(predicate::str::contains(".utoc"));
    assert!(!alias_out.exists());
    assert!(fs::read_dir(temp.path())
        .unwrap()
        .filter_map(Result::ok)
        .all(|entry| !entry
            .file_name()
            .to_string_lossy()
            .starts_with(".gore-asset-")));
}

#[test]
fn pack_rejects_patch_pair_mismatch_before_generation_probe_or_staging() {
    let temp = TempDir::new().unwrap();
    let game = temp.path().join("Game");
    let paks = game.join("G1R/Content/Paks");
    fs::create_dir_all(&paks).unwrap();
    fs::write(paks.join("global.utoc"), b"sealed global toc").unwrap();
    fs::write(paks.join("global.ucas"), b"sealed global cas").unwrap();
    let fixture = write_fixture(temp.path());
    let receipt =
        write_synthetic_patch_receipt(&fixture.uasset, &fixture.usmap, "/Game/Test/DA_Fixture");
    let mut changed_uexp = fs::read(&fixture.uexp).unwrap();
    changed_uexp[0] ^= 1;
    fs::write(&fixture.uexp, changed_uexp).unwrap();

    let out = temp.path().join("must-not-exist");
    Command::cargo_bin("gore")
        .unwrap()
        .args(["asset", "pack", "--game"])
        .arg(&game)
        .arg("--uasset")
        .arg(&fixture.uasset)
        .arg("--patch-receipt")
        .arg(&receipt)
        .args([
            "--asset",
            "/Game/Test/DA_Fixture",
            "--name",
            "zzz_Fixture_P",
            "--out",
        ])
        .arg(&out)
        .arg("--json")
        .assert()
        .failure()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains("ASSET_GENERATION_MISMATCH"));
    assert!(!out.exists());
    let leftovers: Vec<_> = fs::read_dir(temp.path())
        .unwrap()
        .filter_map(Result::ok)
        .filter(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .starts_with(".gore-asset-")
        })
        .collect();
    assert!(leftovers.is_empty());
}

#[test]
fn patch_fixed_requires_snapshot_selector_expected_replacement_and_output() {
    Command::cargo_bin("gore")
        .unwrap()
        .args([
            "asset",
            "patch-fixed",
            "--uasset",
            "input.uasset",
            "--usmap",
            "mappings.usmap",
            "--extract-receipt",
            "gore-asset-extract.json",
        ])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("--selector"))
        .stderr(predicate::str::contains("--expected-hex"))
        .stderr(predicate::str::contains("--replacement-hex"))
        .stderr(predicate::str::contains("--out"));
}

#[test]
fn inspect_runtime_error_keeps_stdout_empty_and_uses_stable_prefix() {
    let temp = TempDir::new().unwrap();
    let missing_uasset = temp.path().join("missing.uasset");
    let missing_usmap = temp.path().join("missing.usmap");

    Command::cargo_bin("gore")
        .unwrap()
        .args(["asset", "inspect", "--uasset"])
        .arg(&missing_uasset)
        .arg("--usmap")
        .arg(&missing_usmap)
        .arg("--json")
        .assert()
        .failure()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains("ASSET_USMAP"));
}

#[test]
fn inspect_and_patch_fixed_reject_uasset_above_64_mib() {
    let temp = TempDir::new().unwrap();
    let fixture = write_fixture(temp.path());
    let report = inspect_json(&fixture.uasset, &fixture.usmap);
    let selector = temp.path().join("selector-limit.json");
    fs::write(
        &selector,
        serde_json::to_vec_pretty(&report["exports"][0]["leaves"][0]).unwrap(),
    )
    .unwrap();
    let receipt =
        write_synthetic_extract_receipt(&fixture.uasset, &fixture.usmap, "/Game/AssetCliFixture");

    let oversized = temp.path().join("Oversized.uasset");
    let file = fs::File::create(&oversized).unwrap();
    file.set_len(64 * 1024 * 1024 + 1).unwrap();
    drop(file);
    fs::copy(&fixture.uexp, oversized.with_extension("uexp")).unwrap();

    Command::cargo_bin("gore")
        .unwrap()
        .args(["asset", "inspect", "--uasset"])
        .arg(&oversized)
        .arg("--usmap")
        .arg(&fixture.usmap)
        .arg("--json")
        .assert()
        .failure()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains("ASSET_INPUT"));

    let output = temp.path().join("MustNotExist.uasset");
    patch_command_with_receipt(
        &oversized,
        &fixture.usmap,
        &receipt,
        &selector,
        "01",
        "00",
        &output,
    )
    .assert()
    .failure()
    .stdout(predicate::str::is_empty())
    .stderr(predicate::str::contains("ASSET_INPUT"));
    assert!(!output.exists());
    assert!(!output.with_extension("uexp").exists());
}

#[test]
fn patch_fixed_rejects_receipt_pair_or_usmap_generation_mismatch() {
    let temp = TempDir::new().unwrap();
    let fixture = write_fixture(temp.path());
    let report = inspect_json(&fixture.uasset, &fixture.usmap);
    let selector = temp.path().join("selector-provenance.json");
    fs::write(
        &selector,
        serde_json::to_vec_pretty(&report["exports"][0]["leaves"][0]).unwrap(),
    )
    .unwrap();
    let receipt =
        write_synthetic_extract_receipt(&fixture.uasset, &fixture.usmap, "/Game/AssetCliFixture");

    let wrong_usmap = temp.path().join("wrong.usmap");
    let mut wrong_usmap_bytes = fs::read(&fixture.usmap).unwrap();
    wrong_usmap_bytes.push(0);
    fs::write(&wrong_usmap, wrong_usmap_bytes).unwrap();
    let usmap_output = temp.path().join("WrongUsmapMustNotExist.uasset");
    patch_command_with_receipt(
        &fixture.uasset,
        &wrong_usmap,
        &receipt,
        &selector,
        "01",
        "00",
        &usmap_output,
    )
    .assert()
    .failure()
    .stdout(predicate::str::is_empty())
    .stderr(predicate::str::contains("ASSET_GENERATION_MISMATCH"));
    assert!(!usmap_output.exists());

    let wrong_pair = temp.path().join("WrongPair.uasset");
    fs::copy(&fixture.uasset, &wrong_pair).unwrap();
    let mut wrong_uexp = fs::read(&fixture.uexp).unwrap();
    wrong_uexp[0] ^= 1;
    fs::write(wrong_pair.with_extension("uexp"), wrong_uexp).unwrap();
    let pair_output = temp.path().join("WrongPairMustNotExist.uasset");
    patch_command_with_receipt(
        &wrong_pair,
        &fixture.usmap,
        &receipt,
        &selector,
        "01",
        "00",
        &pair_output,
    )
    .assert()
    .failure()
    .stdout(predicate::str::is_empty())
    .stderr(predicate::str::contains("ASSET_GENERATION_MISMATCH"));
    assert!(!pair_output.exists());
}

#[test]
fn inspect_and_patch_fixed_bool_is_copy_on_write_and_drift_safe() {
    let temp = TempDir::new().unwrap();
    let fixture = write_fixture(temp.path());
    let source_sidecars = [
        (fixture.uasset.with_extension("ubulk"), b"bulk".as_slice()),
        (
            fixture.uasset.with_extension("uptnl"),
            b"optional bulk".as_slice(),
        ),
        (
            fixture.uasset.with_file_name("Fixture.m.ubulk"),
            b"memory mapped bulk".as_slice(),
        ),
    ];
    for (path, bytes) in &source_sidecars {
        fs::write(path, bytes).unwrap();
    }

    let source_report = inspect_json(&fixture.uasset, &fixture.usmap);
    assert_eq!(source_report["status"], "walked");
    assert_eq!(source_report["summary"]["exports"], 1);
    assert_eq!(source_report["summary"]["walked_exports"], 1);
    assert_eq!(source_report["summary"]["editable_leaves"], 1);
    assert_eq!(source_report["exports"].as_array().unwrap().len(), 1);
    assert_eq!(
        source_report["exports"][0]["class_path"],
        "/Script/Test.Fixture"
    );
    assert_eq!(source_report["exports"][0]["status"], "walked");
    assert_eq!(
        source_report["exports"][0]["leaves"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    let source_leaf = &source_report["exports"][0]["leaves"][0];
    assert_eq!(source_leaf["editable"], true);
    assert_eq!(source_leaf["selector"]["kind"], "bool");
    assert_eq!(source_leaf["selector"]["expected_hex"], "01");

    let selector_path = temp.path().join("bool-selector.json");
    fs::write(
        &selector_path,
        // Save the natural inspect leaf wrapper, not only its nested selector.
        // `patch-fixed` promises that this exact public output shape is input.
        serde_json::to_vec_pretty(source_leaf).unwrap(),
    )
    .unwrap();
    let bare_selector_path = temp.path().join("bool-bare-selector.json");
    fs::write(
        &bare_selector_path,
        serde_json::to_vec_pretty(&source_leaf["selector"]).unwrap(),
    )
    .unwrap();
    let descriptor_path = temp.path().join("bool-descriptor.json");
    fs::write(
        &descriptor_path,
        serde_json::to_vec_pretty(&serde_json::json!({
            "selector": source_leaf["selector"],
            "editable": true,
        }))
        .unwrap(),
    )
    .unwrap();

    let output_uasset = temp.path().join("Patched.uasset");
    let output_uexp = output_uasset.with_extension("uexp");
    let patch_assert = patch_command(
        &fixture.uasset,
        &fixture.usmap,
        &selector_path,
        "01",
        "00",
        &output_uasset,
    )
    .assert()
    .success()
    .stderr(predicate::str::is_empty());
    let patch_report: Value = serde_json::from_slice(&patch_assert.get_output().stdout).unwrap();
    assert_eq!(patch_report["status"], "patched");
    assert_eq!(patch_report["expected_hex"], "01");
    assert_eq!(patch_report["replacement_hex"], "00");
    assert_eq!(patch_report["input_selector"]["expected_hex"], "01");
    assert_eq!(patch_report["output_requires_reinspect"], true);

    assert_eq!(fs::read(&fixture.uasset).unwrap(), fixture.original_uasset);
    assert_eq!(fs::read(&fixture.uexp).unwrap(), fixture.original_uexp);
    assert!(output_uasset.is_file());
    assert!(output_uexp.is_file());
    for ((source, expected), target) in source_sidecars.iter().zip([
        output_uasset.with_extension("ubulk"),
        output_uasset.with_extension("uptnl"),
        output_uasset.with_file_name("Patched.m.ubulk"),
    ]) {
        assert_eq!(fs::read(source).unwrap(), *expected);
        assert_eq!(fs::read(target).unwrap(), *expected);
    }
    assert_eq!(patch_report["format"], "gore.asset.patch-fixed.v2");
    let sidecars = patch_report["output_sidecars"].as_array().unwrap();
    assert_eq!(sidecars.len(), 3);
    assert_eq!(sidecars[0]["role"], "BulkData");
    assert_eq!(sidecars[0]["file_name"], "Patched.ubulk");
    assert_eq!(sidecars[1]["role"], "OptionalBulkData");
    assert_eq!(sidecars[1]["file_name"], "Patched.uptnl");
    assert_eq!(sidecars[2]["role"], "MemoryMappedBulkData");
    assert_eq!(sidecars[2]["file_name"], "Patched.m.ubulk");

    let patched_uasset = fs::read(&output_uasset).unwrap();
    let patched_uexp = fs::read(&output_uexp).unwrap();
    assert_eq!(patched_uasset, fixture.original_uasset);
    assert_eq!(patched_uexp.len(), fixture.original_uexp.len());
    let changed_offsets: Vec<_> = fixture
        .original_uexp
        .iter()
        .zip(&patched_uexp)
        .enumerate()
        .filter_map(|(offset, (before, after))| (before != after).then_some(offset))
        .collect();
    assert_eq!(changed_offsets, vec![2]);
    assert_eq!(fixture.original_uexp[2], 0x01);
    assert_eq!(patched_uexp[2], 0x00);

    let patched_report = inspect_json(&output_uasset, &fixture.usmap);
    assert_eq!(
        patched_report["exports"][0]["leaves"][0]["selector"]["expected_hex"],
        "00"
    );

    let stale_target = temp.path().join("StaleMustNotExist.uasset");
    patch_command(
        &output_uasset,
        &fixture.usmap,
        &bare_selector_path,
        "01",
        "00",
        &stale_target,
    )
    .assert()
    .failure()
    .stdout(predicate::str::is_empty())
    .stderr(predicate::str::contains("ASSET_SELECTOR"));
    assert!(!stale_target.exists());
    assert!(!stale_target.with_extension("uexp").exists());
    assert_eq!(fs::read(&output_uasset).unwrap(), patched_uasset);
    assert_eq!(fs::read(&output_uexp).unwrap(), patched_uexp);

    patch_command(
        &fixture.uasset,
        &fixture.usmap,
        &descriptor_path,
        "01",
        "00",
        &fixture.uasset,
    )
    .assert()
    .failure()
    .stdout(predicate::str::is_empty())
    .stderr(predicate::str::contains("ASSET_OUTPUT"));
    assert_eq!(fs::read(&fixture.uasset).unwrap(), fixture.original_uasset);
    assert_eq!(fs::read(&fixture.uexp).unwrap(), fixture.original_uexp);

    let existing_uasset = temp.path().join("Existing.uasset");
    let existing_uexp = existing_uasset.with_extension("uexp");
    let existing_uasset_sentinel = b"existing header";
    let existing_uexp_sentinel = b"existing exports";
    fs::write(&existing_uasset, existing_uasset_sentinel).unwrap();
    fs::write(&existing_uexp, existing_uexp_sentinel).unwrap();
    patch_command(
        &fixture.uasset,
        &fixture.usmap,
        &selector_path,
        "01",
        "00",
        &existing_uasset,
    )
    .assert()
    .failure()
    .stdout(predicate::str::is_empty())
    .stderr(predicate::str::contains("ASSET_OUTPUT"));
    assert_eq!(
        fs::read(&existing_uasset).unwrap(),
        existing_uasset_sentinel
    );
    assert_eq!(fs::read(&existing_uexp).unwrap(), existing_uexp_sentinel);

    let receipt_collision = temp.path().join("ReceiptCollision.uasset");
    let receipt_collision_path = temp.path().join("ReceiptCollision.gore-asset-patch.json");
    fs::write(&receipt_collision_path, b"existing receipt").unwrap();
    patch_command(
        &fixture.uasset,
        &fixture.usmap,
        &selector_path,
        "01",
        "00",
        &receipt_collision,
    )
    .assert()
    .failure()
    .stdout(predicate::str::is_empty())
    .stderr(predicate::str::contains("ASSET_PATCH_RECEIPT"));
    assert!(!receipt_collision.exists());
    assert!(!receipt_collision.with_extension("uexp").exists());
    assert_eq!(
        fs::read(receipt_collision_path).unwrap(),
        b"existing receipt"
    );

    let sidecar_collision = temp.path().join("SidecarCollision.uasset");
    let sidecar_collision_path = sidecar_collision.with_extension("uptnl");
    fs::write(&sidecar_collision_path, b"existing sidecar").unwrap();
    patch_command(
        &fixture.uasset,
        &fixture.usmap,
        &selector_path,
        "01",
        "00",
        &sidecar_collision,
    )
    .assert()
    .failure()
    .stdout(predicate::str::is_empty())
    .stderr(predicate::str::contains("ASSET_OUTPUT"));
    assert!(!sidecar_collision.exists());
    assert!(!sidecar_collision.with_extension("uexp").exists());
    assert!(!temp
        .path()
        .join("SidecarCollision.gore-asset-patch.json")
        .exists());
    assert_eq!(
        fs::read(&sidecar_collision_path).unwrap(),
        b"existing sidecar"
    );
}

#[test]
fn patch_fixed_rejects_extracted_sidecar_drift_before_any_output() {
    let temp = TempDir::new().unwrap();
    let fixture = write_fixture(temp.path());
    let source_sidecar = fixture.uasset.with_extension("ubulk");
    fs::write(&source_sidecar, b"sealed bulk").unwrap();
    let receipt =
        write_synthetic_extract_receipt(&fixture.uasset, &fixture.usmap, "/Game/AssetCliFixture");
    fs::write(&source_sidecar, b"mutated bulk").unwrap();

    let report = inspect_json(&fixture.uasset, &fixture.usmap);
    let selector = temp.path().join("sidecar-drift-selector.json");
    fs::write(
        &selector,
        serde_json::to_vec_pretty(&report["exports"][0]["leaves"][0]).unwrap(),
    )
    .unwrap();
    let output = temp.path().join("MustNotExist.uasset");
    patch_command_with_receipt(
        &fixture.uasset,
        &fixture.usmap,
        &receipt,
        &selector,
        "01",
        "00",
        &output,
    )
    .assert()
    .failure()
    .stdout(predicate::str::is_empty())
    .stderr(predicate::str::contains("ASSET_GENERATION_MISMATCH"));
    for path in [
        output.clone(),
        output.with_extension("uexp"),
        output.with_extension("ubulk"),
        output.with_extension("uptnl"),
        output.with_file_name("MustNotExist.m.ubulk"),
        temp.path().join("MustNotExist.gore-asset-patch.json"),
    ] {
        assert!(
            !path.exists(),
            "unexpected partial output: {}",
            path.display()
        );
    }
}

#[test]
fn pack_rejects_missing_mutated_or_extra_patched_sidecars_before_staging() {
    const ASSET: &str = "/Game/Test/DA_Fixture";
    let temp = TempDir::new().unwrap();
    let game = temp.path().join("Game");
    let paks = game.join("G1R/Content/Paks");
    fs::create_dir_all(&paks).unwrap();
    fs::write(paks.join("global.utoc"), b"global toc").unwrap();
    fs::write(paks.join("global.ucas"), b"global cas").unwrap();

    let missing_dir = temp.path().join("missing");
    fs::create_dir(&missing_dir).unwrap();
    let missing = write_fixture(&missing_dir);
    fs::write(missing.uasset.with_extension("ubulk"), b"bulk").unwrap();
    let missing_receipt = write_synthetic_patch_receipt(&missing.uasset, &missing.usmap, ASSET);
    fs::remove_file(missing.uasset.with_extension("ubulk")).unwrap();
    let missing_out = temp.path().join("missing-out");
    pack_command(
        &game,
        &missing.uasset,
        &missing_receipt,
        ASSET,
        &missing_out,
    )
    .assert()
    .failure()
    .stdout(predicate::str::is_empty())
    .stderr(predicate::str::contains(
        "required optional sidecar is missing",
    ));
    assert!(!missing_out.exists());

    let mutated_dir = temp.path().join("mutated");
    fs::create_dir(&mutated_dir).unwrap();
    let mutated = write_fixture(&mutated_dir);
    fs::write(mutated.uasset.with_extension("uptnl"), b"optional").unwrap();
    let mutated_receipt = write_synthetic_patch_receipt(&mutated.uasset, &mutated.usmap, ASSET);
    fs::write(mutated.uasset.with_extension("uptnl"), b"changed optional").unwrap();
    let mutated_out = temp.path().join("mutated-out");
    pack_command(
        &game,
        &mutated.uasset,
        &mutated_receipt,
        ASSET,
        &mutated_out,
    )
    .assert()
    .failure()
    .stdout(predicate::str::is_empty())
    .stderr(predicate::str::contains("sidecar content differs"));
    assert!(!mutated_out.exists());

    let extra_dir = temp.path().join("extra");
    fs::create_dir(&extra_dir).unwrap();
    let extra = write_fixture(&extra_dir);
    let extra_receipt = write_synthetic_patch_receipt(&extra.uasset, &extra.usmap, ASSET);
    fs::write(
        extra.uasset.with_file_name("Fixture.m.ubulk"),
        b"unexpected memory mapped bulk",
    )
    .unwrap();
    let extra_out = temp.path().join("extra-out");
    pack_command(&game, &extra.uasset, &extra_receipt, ASSET, &extra_out)
        .assert()
        .failure()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains("unexpected optional sidecar"));
    assert!(!extra_out.exists());

    assert!(fs::read_dir(temp.path())
        .unwrap()
        .filter_map(Result::ok)
        .all(|entry| !entry
            .file_name()
            .to_string_lossy()
            .starts_with(".gore-asset-")));
}

#[test]
fn pack_deserializes_and_cross_checks_the_hash_bound_extract_receipt() {
    const ASSET: &str = "/Game/Test/DA_Fixture";
    let temp = TempDir::new().unwrap();
    let game = temp.path().join("Game");
    let paks = game.join("G1R/Content/Paks");
    fs::create_dir_all(&paks).unwrap();
    fs::write(paks.join("global.utoc"), b"global toc").unwrap();
    fs::write(paks.join("global.ucas"), b"global cas").unwrap();

    for case in [
        "empty-object",
        "asset-mismatch",
        "embedded-components-mismatch",
        "source-global-mismatch",
        "source-consumed-mismatch",
        "source-game-root-mismatch",
        "source-noncanonical-path",
        "source-relocated-path",
        "source-unknown-field",
    ] {
        let directory = temp.path().join(case);
        fs::create_dir(&directory).unwrap();
        let fixture = write_fixture(&directory);
        let patch_receipt = write_synthetic_patch_receipt(&fixture.uasset, &fixture.usmap, ASSET);
        let mut patch_json: Value =
            serde_json::from_slice(&fs::read(&patch_receipt).unwrap()).unwrap();
        let extract_path = PathBuf::from(
            patch_json["provenance"]["extract_receipt"]["path"]
                .as_str()
                .unwrap(),
        );
        if case == "embedded-components-mismatch" {
            patch_json["provenance"]["extract_components"][0]["sha256"] =
                Value::String("ff".repeat(32));
        } else {
            let mut extract_json: Value =
                serde_json::from_slice(&fs::read(&extract_path).unwrap()).unwrap();
            match case {
                "empty-object" => extract_json = serde_json::json!({}),
                "asset-mismatch" => {
                    extract_json["asset"] = Value::String("/Game/Test/Different".to_owned());
                }
                "source-global-mismatch" => {
                    extract_json["source"]["global_script_store"]["ucas"]["sha256"] =
                        Value::String("aa".repeat(32));
                }
                "source-consumed-mismatch" => {
                    extract_json["source"]["consumed_chunks"][0]["blake3"] =
                        Value::String("bb".repeat(32));
                }
                "source-game-root-mismatch" => {
                    extract_json["source"]["game_root"] =
                        Value::String(directory.join("OtherGame").display().to_string());
                }
                "source-noncanonical-path" => {
                    let original = PathBuf::from(
                        extract_json["source"]["global_script_store"]["utoc"]["path"]
                            .as_str()
                            .unwrap(),
                    );
                    extract_json["source"]["global_script_store"]["utoc"]["path"] = Value::String(
                        original
                            .parent()
                            .unwrap()
                            .join("Alias")
                            .join("..")
                            .join(original.file_name().unwrap())
                            .display()
                            .to_string(),
                    );
                }
                "source-relocated-path" => {
                    extract_json["source"]["global_script_store"]["utoc"]["path"] = Value::String(
                        directory
                            .join("Relocated")
                            .join("global.utoc")
                            .display()
                            .to_string(),
                    );
                }
                "source-unknown-field" => {
                    extract_json["source"]["unexpected"] = Value::Bool(true);
                }
                _ => unreachable!(),
            }
            let extract_bytes = serde_json::to_vec_pretty(&extract_json).unwrap();
            fs::write(&extract_path, &extract_bytes).unwrap();
            patch_json["provenance"]["extract_receipt"]["length"] =
                Value::from(extract_bytes.len());
            patch_json["provenance"]["extract_receipt"]["sha256"] =
                Value::String(sha256_hex(&extract_bytes));
        }
        fs::write(
            &patch_receipt,
            serde_json::to_vec_pretty(&patch_json).unwrap(),
        )
        .unwrap();

        let output = temp.path().join(format!("{case}-out"));
        let expected_code = if case == "embedded-components-mismatch" {
            "ASSET_PATCH_RECEIPT"
        } else {
            "ASSET_EXTRACT_RECEIPT"
        };
        pack_command(&game, &fixture.uasset, &patch_receipt, ASSET, &output)
            .assert()
            .failure()
            .stdout(predicate::str::is_empty())
            .stderr(predicate::str::contains(expected_code))
            .stderr(predicate::str::contains("ASSET_PACK_GENERATION").not());
        assert!(!output.exists());
    }
    assert!(fs::read_dir(temp.path())
        .unwrap()
        .filter_map(Result::ok)
        .all(|entry| !entry
            .file_name()
            .to_string_lossy()
            .starts_with(".gore-asset-")));
}

#[test]
fn pack_rejects_every_formerly_ignored_patch_v2_proof() {
    const ASSET: &str = "/Game/Test/DA_Fixture";
    let temp = TempDir::new().unwrap();
    let game = temp.path().join("Game");
    let paks = game.join("G1R/Content/Paks");
    fs::create_dir_all(&paks).unwrap();
    fs::write(paks.join("global.utoc"), b"global toc").unwrap();
    fs::write(paks.join("global.ucas"), b"global cas").unwrap();

    for case in [
        "unknown-top-level",
        "status",
        "selector-usmap",
        "reinspect-flag",
        "expected-proof",
        "replacement-proof",
        "patch-before",
        "output-uasset",
    ] {
        let directory = temp.path().join(case);
        fs::create_dir(&directory).unwrap();
        let fixture = write_fixture(&directory);
        let patch_receipt = write_synthetic_patch_receipt(&fixture.uasset, &fixture.usmap, ASSET);
        let mut patch_json: Value =
            serde_json::from_slice(&fs::read(&patch_receipt).unwrap()).unwrap();
        match case {
            "unknown-top-level" => {
                patch_json
                    .as_object_mut()
                    .unwrap()
                    .insert("unexpected".to_owned(), Value::Bool(true));
            }
            "status" => patch_json["status"] = Value::String("complete".to_owned()),
            "selector-usmap" => {
                patch_json["input_selector"]["usmap_sha256"] = Value::String("aa".repeat(32));
            }
            "reinspect-flag" => patch_json["output_requires_reinspect"] = Value::Bool(false),
            "expected-proof" => {
                let bytes = patch_json["expected_hex"].as_str().unwrap().len() / 2;
                patch_json["expected_hex"] = Value::String("ff".repeat(bytes));
            }
            "replacement-proof" => {
                let bytes = patch_json["replacement_hex"].as_str().unwrap().len() / 2;
                patch_json["replacement_hex"] = Value::String("00".repeat(bytes));
            }
            "patch-before" => {
                patch_json["patch"]["before"]["uasset_sha256"] = Value::String("bb".repeat(32));
            }
            "output-uasset" => {
                patch_json["output"]["uasset"]["sha256"] = Value::String("cc".repeat(32));
            }
            _ => unreachable!(),
        }
        fs::write(
            &patch_receipt,
            serde_json::to_vec_pretty(&patch_json).unwrap(),
        )
        .unwrap();

        let output = temp.path().join(format!("{case}-out"));
        pack_command(&game, &fixture.uasset, &patch_receipt, ASSET, &output)
            .assert()
            .failure()
            .stdout(predicate::str::is_empty())
            .stderr(predicate::str::contains("ASSET_PATCH_RECEIPT"))
            .stderr(predicate::str::contains("ASSET_PACK_GENERATION").not());
        assert!(!output.exists());
    }
    assert!(fs::read_dir(temp.path())
        .unwrap()
        .filter_map(Result::ok)
        .all(|entry| !entry
            .file_name()
            .to_string_lossy()
            .starts_with(".gore-asset-")));
}

#[test]
#[ignore = "local real-game proof; scans the installed IoStore and never deploys"]
fn real_wolf_extract_inspect_patch_pack_and_reopen_offline() {
    const ASSET: &str = "/Game/Blueprints/TrackingSystem/FootstepsPresets/DA_WolfFootsteps";
    let game = std::env::var_os("GORE_REAL_GAME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"D:\SteamLibrary\steamapps\common\Gothic 1 Remake"));
    assert!(game.join("G1R/Content/Paks/G1R-Windows.utoc").is_file());
    let temp = TempDir::new().unwrap();
    let extracted = temp.path().join("extracted");

    let extract = Command::cargo_bin("gore")
        .unwrap()
        .args(["asset", "extract", "--game"])
        .arg(&game)
        .args(["--asset", ASSET, "--out"])
        .arg(&extracted)
        .arg("--json")
        .assert()
        .success()
        .stderr(predicate::str::is_empty());
    let extract_receipt: Value = serde_json::from_slice(&extract.get_output().stdout).unwrap();
    assert_eq!(extract_receipt["format"], "gore.asset.extract.v2");
    assert_eq!(extract_receipt["asset"], ASSET);
    assert_eq!(extract_receipt["deployed"], false);
    assert_eq!(
        extract_receipt["source"]["composite_store_anchor"]["ucas"]["content_hash_omitted"],
        true
    );
    assert!(extract_receipt["source"]["composite_store_anchor"]["ucas"]["sha256"].is_null());
    assert!(
        extract_receipt["source"]["composite_store_anchor"]["ucas"]["length"]
            .as_u64()
            .unwrap()
            > 1_000_000_000
    );
    let consumed = extract_receipt["source"]["consumed_chunks"]
        .as_array()
        .unwrap();
    assert!(!consumed.is_empty());
    assert!(consumed.iter().all(|chunk| {
        chunk["blake3"]
            .as_str()
            .is_some_and(|hash| hash.len() == 64)
            && chunk["source_utoc"].as_str().is_some()
    }));
    assert!(consumed
        .iter()
        .any(|chunk| chunk["chunk_type"] == "ContainerHeader"));
    assert!(consumed
        .iter()
        .any(|chunk| chunk["chunk_type"] == "ExportBundleData"));
    assert!(!extract_receipt["source"]["source_container_tocs"]
        .as_array()
        .unwrap()
        .is_empty());
    assert!(extracted.join("gore-asset-extract.json").is_file());
    let extract_receipt_path = extracted.join("gore-asset-extract.json");
    let usmap = extracted.join("gore-generation.usmap");
    assert!(usmap.is_file());
    assert_eq!(
        fs::read(&usmap).unwrap(),
        fs::read(gore_tex::paths::usmap(&game).unwrap()).unwrap()
    );

    let source_uasset = extracted.join("DA_WolfFootsteps.uasset");
    let source_uexp = source_uasset.with_extension("uexp");
    let source_uasset_bytes = fs::read(&source_uasset).unwrap();
    let source_uexp_bytes = fs::read(&source_uexp).unwrap();
    let report = inspect_json(&source_uasset, &usmap);
    let leaf = report["exports"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|export| export["leaves"].as_array().unwrap())
        .find(|leaf| leaf["editable"] == true && leaf["selector"]["kind"] == "vector4_f64x4")
        .expect("real Wolf fixture should expose its proven Vector4 leaf");
    let expected = leaf["selector"]["expected_hex"].as_str().unwrap();
    let first_byte = u8::from_str_radix(&expected[..2], 16).unwrap() ^ 1;
    let replacement = format!("{first_byte:02x}{}", &expected[2..]);
    let selector = temp.path().join("selector.json");
    fs::write(&selector, serde_json::to_vec_pretty(leaf).unwrap()).unwrap();

    let patched_dir = temp.path().join("patched");
    fs::create_dir(&patched_dir).unwrap();
    let patched_uasset = patched_dir.join("DA_WolfFootsteps.uasset");
    patch_command_with_receipt(
        &source_uasset,
        &usmap,
        &extract_receipt_path,
        &selector,
        expected,
        &replacement,
        &patched_uasset,
    )
    .assert()
    .success()
    .stderr(predicate::str::is_empty());
    let patch_receipt = patched_dir.join("DA_WolfFootsteps.gore-asset-patch.json");
    assert!(patch_receipt.is_file());
    assert_eq!(fs::read(&source_uasset).unwrap(), source_uasset_bytes);
    assert_eq!(fs::read(&source_uexp).unwrap(), source_uexp_bytes);
    assert_eq!(
        inspect_json(&patched_uasset, &usmap)["exports"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|export| export["leaves"].as_array().unwrap())
            .find(|candidate| candidate["semantic_path"] == leaf["semantic_path"])
            .unwrap()["selector"]["expected_hex"],
        replacement
    );

    let packed = temp.path().join("packed");
    let pack = Command::cargo_bin("gore")
        .unwrap()
        .args(["asset", "pack", "--game"])
        .arg(&game)
        .arg("--uasset")
        .arg(&patched_uasset)
        .arg("--patch-receipt")
        .arg(&patch_receipt)
        .args(["--asset", ASSET, "--name", "zzz_GoreWolfProof_P", "--out"])
        .arg(&packed)
        .arg("--json")
        .assert()
        .success()
        .stderr(predicate::str::is_empty());
    let pack_receipt: Value = serde_json::from_slice(&pack.get_output().stdout).unwrap();
    assert_eq!(pack_receipt["format"], "gore.asset.pack.v2");
    assert_eq!(pack_receipt["generation_bound"], true);
    assert_eq!(pack_receipt["output"]["reopened_packages"][0], ASSET);
    assert_eq!(pack_receipt["deployed"], false);
    assert!(pack_receipt["source"]["consumed_chunks"]
        .as_array()
        .unwrap()
        .iter()
        .any(|chunk| chunk["chunk_type"] == "ContainerHeader"));
    assert_eq!(pack_receipt["output"]["strict_reopen"]["bulk_chunks"], 0);
    assert_eq!(
        pack_receipt["output"]["strict_reopen"]["optional_bulk_chunks"],
        0
    );
    assert_eq!(
        pack_receipt["output"]["strict_reopen"]["memory_mapped_bulk_chunks"],
        0
    );
    assert!(
        pack_receipt["source"]["global_script_store"]["utoc"]["sha256"]
            .as_str()
            .is_some()
    );
    let utoc = packed.join("zzz_GoreWolfProof_P.utoc");
    assert_eq!(gore_tex::container::list_packages(&utoc).unwrap(), [ASSET]);
    assert!(packed.join("zzz_GoreWolfProof_P.ucas").is_file());
    assert!(packed.join("zzz_GoreWolfProof_P.pak").is_file());
    assert!(packed.join("gore-asset-pack.json").is_file());
}

fn inspect_json(uasset: &Path, usmap: &Path) -> Value {
    let assert = Command::cargo_bin("gore")
        .unwrap()
        .args(["asset", "inspect", "--uasset"])
        .arg(uasset)
        .arg("--usmap")
        .arg(usmap)
        .arg("--json")
        .assert()
        .success()
        .stderr(predicate::str::is_empty());
    serde_json::from_slice(&assert.get_output().stdout).unwrap()
}

fn patch_command(
    uasset: &Path,
    usmap: &Path,
    selector: &Path,
    expected_hex: &str,
    replacement_hex: &str,
    output: &Path,
) -> Command {
    let receipt = write_synthetic_extract_receipt(uasset, usmap, "/Game/AssetCliFixture");
    patch_command_with_receipt(
        uasset,
        usmap,
        &receipt,
        selector,
        expected_hex,
        replacement_hex,
        output,
    )
}

fn patch_command_with_receipt(
    uasset: &Path,
    usmap: &Path,
    extract_receipt: &Path,
    selector: &Path,
    expected_hex: &str,
    replacement_hex: &str,
    output: &Path,
) -> Command {
    let mut command = Command::cargo_bin("gore").unwrap();
    command
        .args(["asset", "patch-fixed", "--uasset"])
        .arg(uasset)
        .arg("--usmap")
        .arg(usmap)
        .arg("--extract-receipt")
        .arg(extract_receipt)
        .arg("--selector")
        .arg(selector)
        .arg("--expected-hex")
        .arg(expected_hex)
        .arg("--replacement-hex")
        .arg(replacement_hex)
        .arg("--out")
        .arg(output)
        .arg("--json");
    command
}

fn pack_command(
    game: &Path,
    uasset: &Path,
    patch_receipt: &Path,
    asset: &str,
    output: &Path,
) -> Command {
    let mut command = Command::cargo_bin("gore").unwrap();
    command
        .args(["asset", "pack", "--game"])
        .arg(game)
        .arg("--uasset")
        .arg(uasset)
        .arg("--patch-receipt")
        .arg(patch_receipt)
        .args(["--asset", asset, "--name", "zzz_Fixture_P", "--out"])
        .arg(output)
        .arg("--json");
    command
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn synthetic_file_anchor(path: &Path, bytes: &[u8]) -> Value {
    serde_json::json!({
        "file_name": path.file_name().unwrap().to_string_lossy(),
        "length": bytes.len(),
        "sha256": sha256_hex(bytes),
    })
}

fn synthetic_chunk_id(asset: &str, chunk_type: EIoChunkType) -> String {
    let package = FPackageId(FIoContainerId::from_name(asset).0);
    FIoChunkId::from_package_id(package, 0, chunk_type)
        .with_version(EngineVersion::UE5_4.toc_version())
        .get_raw()
        .id
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn write_synthetic_extract_receipt(uasset: &Path, usmap: &Path, asset: &str) -> PathBuf {
    let uasset_bytes = fs::read(uasset).unwrap();
    let uexp = uasset.with_extension("uexp");
    let uexp_bytes = fs::read(&uexp).unwrap();
    let usmap_bytes = fs::read(usmap).unwrap();
    let directory = uasset.parent().unwrap();
    let receipt_path = directory.join("gore-asset-extract.json");
    let synthetic_game = directory.join("SyntheticGame");
    let synthetic_paks = synthetic_game.join("G1R").join("Content").join("Paks");
    let synthetic_ue4ss = synthetic_game
        .join("G1R")
        .join("Binaries")
        .join("Win64")
        .join("ue4ss");
    let dummy_utoc_path = synthetic_paks.join("G1R-Windows.utoc");
    let dummy_ucas_path = synthetic_paks.join("G1R-Windows.ucas");
    let global_utoc_path = synthetic_paks.join("global.utoc");
    let global_ucas_path = synthetic_paks.join("global.ucas");
    let source_usmap_path = synthetic_ue4ss.join(usmap.file_name().unwrap());
    let usmap_anchor = synthetic_file_anchor(usmap, &usmap_bytes);
    let dummy_utoc = serde_json::json!({
        "file_name": "G1R-Windows.utoc",
        "length": 1,
        "sha256": "00".repeat(32),
    });
    let global_utoc = serde_json::json!({
        "file_name": "global.utoc",
        "length": 1,
        "sha256": "77".repeat(32),
    });
    let mut target_chunks = vec![
        serde_json::json!({
            "chunk_id": synthetic_chunk_id(asset, EIoChunkType::ExportBundleData),
            "chunk_type": "ExportBundleData",
            "winner_utoc": dummy_utoc,
            "length": 1,
            "blake3": "22".repeat(32),
            "toc_hash": "22".repeat(20),
            "toc_hash_bytes": 20,
        }),
        serde_json::json!({
            "chunk_id": "01".repeat(12),
            "chunk_type": "ContainerHeader",
            "winner_utoc": dummy_utoc,
            "length": 1,
            "blake3": "33".repeat(32),
            "toc_hash": "33".repeat(20),
            "toc_hash_bytes": 20,
        }),
    ];
    let mut components = vec![
        serde_json::json!({
            "relative_path": uasset.file_name().unwrap().to_string_lossy(),
            "length": uasset_bytes.len(),
            "sha256": sha256_hex(&uasset_bytes),
        }),
        serde_json::json!({
            "relative_path": uexp.file_name().unwrap().to_string_lossy(),
            "length": uexp_bytes.len(),
            "sha256": sha256_hex(&uexp_bytes),
        }),
        serde_json::json!({
            "relative_path": "gore-generation.usmap",
            "length": usmap_bytes.len(),
            "sha256": sha256_hex(&usmap_bytes),
        }),
    ];
    let stem = uasset.file_stem().unwrap().to_string_lossy();
    for (role, suffix, chunk_type, fill) in [
        ("BulkData", "ubulk", EIoChunkType::BulkData, "44"),
        (
            "OptionalBulkData",
            "uptnl",
            EIoChunkType::OptionalBulkData,
            "55",
        ),
        (
            "MemoryMappedBulkData",
            "m.ubulk",
            EIoChunkType::MemoryMappedBulkData,
            "66",
        ),
    ] {
        let path = uasset.with_file_name(format!("{stem}.{suffix}"));
        if !path.is_file() {
            continue;
        }
        let bytes = fs::read(&path).unwrap();
        components.push(serde_json::json!({
            "relative_path": path.file_name().unwrap().to_string_lossy(),
            "length": bytes.len(),
            "sha256": sha256_hex(&bytes),
        }));
        target_chunks.push(serde_json::json!({
            "chunk_id": synthetic_chunk_id(asset, chunk_type),
            "chunk_type": role,
            "winner_utoc": dummy_utoc,
            "length": bytes.len(),
            "blake3": fill.repeat(32),
            "toc_hash": fill.repeat(20),
            "toc_hash_bytes": 20,
        }));
    }
    target_chunks.sort_by(|left, right| {
        left["chunk_id"]
            .as_str()
            .unwrap()
            .cmp(right["chunk_id"].as_str().unwrap())
    });
    let consumed_chunks: Vec<_> = target_chunks
        .iter()
        .map(|chunk| {
            serde_json::json!({
                "chunk_id": chunk["chunk_id"],
                "chunk_type": chunk["chunk_type"],
                "source_utoc": dummy_utoc_path,
                "length": chunk["length"],
                "blake3": chunk["blake3"],
                "toc_hash": chunk["toc_hash"],
                "toc_hash_bytes": chunk["toc_hash_bytes"],
            })
        })
        .collect();
    let generation = serde_json::json!({
        "format": "gore.asset.generation.v1",
        "asset": asset,
        "usmap": usmap_anchor,
        "main_utoc": dummy_utoc,
        "global_utoc": global_utoc,
        "global_ucas": {
            "file_name": "global.ucas",
            "length": 1,
            "sha256": "11".repeat(32),
        },
        "container_set": [dummy_utoc, global_utoc],
        "target_chunks": target_chunks,
    });
    let receipt = serde_json::json!({
        "format": "gore.asset.extract.v2",
        "status": "extracted",
        "asset": asset,
        "generation": generation,
        "source": {
            "game_root": synthetic_game.display().to_string(),
            "composite_store_anchor": {
                "utoc": {
                    "path": dummy_utoc_path.display().to_string(),
                    "length": 1,
                    "sha256": "00".repeat(32),
                },
                "ucas": {
                    "path": dummy_ucas_path.display().to_string(),
                    "length": 1,
                    "modified_stamp": "synthetic",
                    "platform_identity": "synthetic-file-identity",
                    "sha256": null,
                    "verification": "identity_length_mtime_point_check",
                    "content_hash_omitted": true,
                    "limitation": "the large UCAS payload is not content-hashed; file identity, length, and modification stamp are held and point-rechecked before publication",
                },
                "role": "environment anchor only; consumed_chunks is the authoritative content binding",
            },
            "consumed_chunks": consumed_chunks,
            "source_container_tocs": [
                {
                    "path": dummy_utoc_path.display().to_string(),
                    "length": 1,
                    "sha256": "00".repeat(32),
                },
                {
                    "path": global_utoc_path.display().to_string(),
                    "length": 1,
                    "sha256": "77".repeat(32),
                },
            ],
            "content_binding": "each consumed decompressed chunk was verified against its winning container's TOC BLAKE3 hash and cached for all conversion reads",
            "usmap": {
                "source": {
                    "path": source_usmap_path.display().to_string(),
                    "length": usmap_bytes.len(),
                    "sha256": sha256_hex(&usmap_bytes),
                },
                "copied_relative_path": "gore-generation.usmap",
                "copy": components[2],
            },
            "global_script_store": {
                "utoc": {
                    "path": global_utoc_path.display().to_string(),
                    "length": 1,
                    "sha256": "77".repeat(32),
                },
                "ucas": {
                    "path": global_ucas_path.display().to_string(),
                    "length": 1,
                    "sha256": "11".repeat(32),
                },
            },
        },
        "package_seal": {
            "uasset_sha256": sha256_hex(&uasset_bytes),
            "uexp_sha256": sha256_hex(&uexp_bytes),
        },
        "output": {
            "root": uasset.parent().unwrap().display().to_string(),
            "receipt": "gore-asset-extract.json",
            "components": components,
        },
        "deployed": false,
    });
    fs::write(&receipt_path, serde_json::to_vec_pretty(&receipt).unwrap()).unwrap();
    receipt_path
}

fn write_synthetic_patch_receipt(uasset: &Path, usmap: &Path, asset: &str) -> PathBuf {
    let extract = write_synthetic_extract_receipt(uasset, usmap, asset);
    let extract_bytes = fs::read(&extract).unwrap();
    let extract_json: Value = serde_json::from_slice(&extract_bytes).unwrap();
    let uasset_bytes = fs::read(uasset).unwrap();
    let uexp_bytes = fs::read(uasset.with_extension("uexp")).unwrap();
    let pair = serde_json::json!({
        "uasset_sha256": sha256_hex(&uasset_bytes),
        "uexp_sha256": sha256_hex(&uexp_bytes),
    });
    let stem = uasset.file_stem().unwrap().to_string_lossy();
    let path = uasset
        .parent()
        .unwrap()
        .join(format!("{stem}.gore-asset-patch.json"));
    let output_sidecars: Vec<_> = extract_json["output"]["components"]
        .as_array()
        .unwrap()
        .iter()
        .skip(3)
        .map(|component| {
            let file_name = component["relative_path"].as_str().unwrap();
            let role = if file_name == format!("{stem}.ubulk") {
                "BulkData"
            } else if file_name == format!("{stem}.uptnl") {
                "OptionalBulkData"
            } else {
                assert_eq!(file_name, format!("{stem}.m.ubulk"));
                "MemoryMappedBulkData"
            };
            serde_json::json!({
                "role": role,
                "file_name": file_name,
                "length": component["length"],
                "sha256": component["sha256"],
            })
        })
        .collect();
    let inspect = inspect_json(uasset, usmap);
    let selector = inspect["exports"][0]["leaves"][0]["selector"].clone();
    let expected_hex = selector["expected_hex"].as_str().unwrap();
    let patch_length = expected_hex.len() / 2;
    let receipt = serde_json::json!({
        "format": "gore.asset.patch-fixed.v2",
        "status": "patched",
        "asset": asset,
        "generation_bound": true,
        "provenance": {
            "extract_receipt": {
                "path": extract.display().to_string(),
                "length": extract_bytes.len(),
                "sha256": sha256_hex(&extract_bytes),
            },
            "generation": extract_json["generation"],
            "usmap": {
                "file_name": "gore-generation.usmap",
                "length": extract_json["output"]["components"][2]["length"],
                "sha256": extract_json["output"]["components"][2]["sha256"],
            },
            "extract_components": extract_json["output"]["components"],
            "extracted_sidecars": output_sidecars,
        },
        "input_package_seal": pair,
        "output_package_seal": pair,
        "output_sidecars": output_sidecars,
        "input_selector": selector,
        "output_requires_reinspect": true,
        "expected_hex": expected_hex,
        "replacement_hex": expected_hex,
        "patch": {
            "before": pair,
            "after": pair,
            "export_index": selector["export_index"],
            "component": selector["component"],
            "absolute_offset": 2,
            "length": patch_length,
            "kind": selector["kind"],
        },
        "output": {
            "uasset": {
                "path": uasset.display().to_string(),
                "length": uasset_bytes.len(),
                "sha256": sha256_hex(&uasset_bytes),
            },
            "uexp": {
                "path": uasset.with_extension("uexp").display().to_string(),
                "length": uexp_bytes.len(),
                "sha256": sha256_hex(&uexp_bytes),
            },
            "sidecars": output_sidecars,
            "receipt": path.display().to_string(),
        },
    });
    fs::write(&path, serde_json::to_vec_pretty(&receipt).unwrap()).unwrap();
    path
}

fn write_fixture(directory: &Path) -> Fixture {
    let mut package = FLegacyPackageHeader::default();
    package.summary.versioning_info.package_file_version =
        EngineVersion::UE5_4.package_file_version();
    package.summary.versioning_info.is_unversioned = true;
    package.summary.package_name = "/Game/AssetCliFixture".to_owned();
    package.summary.package_flags = EPackageFlags::Cooked as u32
        | EPackageFlags::FilterEditorOnly as u32
        | EPackageFlags::UsesUnversionedProperties as u32;

    let class_index = add_imported_class(&mut package, "/Script/Test", "Fixture");
    let object_name = package.name_map.store("AssetCliFixture");
    package.exports.push(FObjectExport {
        class_index,
        object_name,
        serial_offset: 0,
        serial_size: EXPORT_BYTES.len() as i64,
        ..FObjectExport::default()
    });

    let mut serialized_header = Cursor::new(Vec::new());
    package
        .serialize(&mut serialized_header, None, &Log::no_log())
        .unwrap();
    let original_uasset = serialized_header.into_inner();
    let mut original_uexp = EXPORT_BYTES.to_vec();
    original_uexp.extend_from_slice(&FLegacyPackageFileSummary::PACKAGE_FILE_TAG.to_le_bytes());

    let uasset = directory.join("Fixture.uasset");
    let uexp = uasset.with_extension("uexp");
    fs::write(&uasset, &original_uasset).unwrap();
    fs::write(&uexp, &original_uexp).unwrap();

    let mapping = usmap::Usmap {
        enums: Vec::new(),
        structs: vec![usmap::Struct {
            name: "Fixture".to_owned(),
            super_struct: None,
            properties: vec![usmap::Property {
                name: "Enabled".to_owned(),
                array_dim: 1,
                index: 0,
                inner: usmap::PropertyInner::Bool,
            }],
        }],
        cext: None,
        ppth: Some(usmap::ExtPpth {
            version: 0,
            enums: Vec::new(),
            structs: vec!["/Script/Test".to_owned()],
        }),
        eatr: Some(usmap::ExtEatr {
            version: 0,
            enum_flags: Vec::new(),
            struct_flags: vec![usmap::StructFlags {
                type_: usmap::FlagsType::Class,
                value: 0,
                prop_flags: Vec::new(),
            }],
        }),
        envp: None,
    };
    let mut raw_usmap = Vec::new();
    mapping.write(&mut raw_usmap).unwrap();
    let usmap = directory.join("Fixture.usmap");
    fs::write(&usmap, raw_usmap).unwrap();

    Fixture {
        uasset,
        uexp,
        usmap,
        original_uasset,
        original_uexp,
    }
}

fn add_imported_class(
    package: &mut FLegacyPackageHeader,
    module: &str,
    class: &str,
) -> FPackageIndex {
    let core_uobject = package.name_map.store("/Script/CoreUObject");
    let package_class = package.name_map.store("Package");
    let class_class = package.name_map.store("Class");
    let module_name = package.name_map.store(module);
    let class_name = package.name_map.store(class);

    let module_index = package.imports.len();
    package.imports.push(FObjectImport {
        class_package: core_uobject,
        class_name: package_class,
        object_name: module_name,
        ..FObjectImport::default()
    });
    let class_index = package.imports.len();
    package.imports.push(FObjectImport {
        class_package: core_uobject,
        class_name: class_class,
        outer_index: FPackageIndex::create_import(module_index as u32),
        object_name: class_name,
        ..FObjectImport::default()
    });
    FPackageIndex::create_import(class_index as u32)
}
