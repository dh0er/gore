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
use serde_json::Value;
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
fn asset_help_exposes_only_inspect_and_single_fixed_patch() {
    Command::cargo_bin("gore")
        .unwrap()
        .args(["asset", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("inspect"))
        .stdout(predicate::str::contains("patch-fixed"));
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
fn inspect_and_patch_fixed_bool_is_copy_on_write_and_drift_safe() {
    let temp = TempDir::new().unwrap();
    let fixture = write_fixture(temp.path());

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
    let mut command = Command::cargo_bin("gore").unwrap();
    command
        .args(["asset", "patch-fixed", "--uasset"])
        .arg(uasset)
        .arg("--usmap")
        .arg(usmap)
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
