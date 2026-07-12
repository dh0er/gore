use assert_cmd::Command;
use predicates::str::contains;
use serde_json::Value;
use sha2::{Digest, Sha256};

fn fixture_path() -> String {
    concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/cache_head_8k.bin"
    )
    .to_string()
}

#[test]
fn decode_header_prints_values() {
    Command::cargo_bin("gore")
        .unwrap()
        .args(["as", "decode-header", &fixture_path()])
        .assert()
        .success()
        .stdout(contains("magic      : 0x9e377abe"))
        .stdout(contains("type_count : 7264"))
        .stdout(contains("d54f0ffb10c1054b99f11446a43ed5dc"));
}

#[test]
fn cli_command_graph_has_stack_headroom_for_version_and_help() {
    Command::cargo_bin("gore")
        .unwrap()
        .arg("--version")
        .assert()
        .success()
        .stdout(contains(format!("gore {}", env!("CARGO_PKG_VERSION"))));

    for (args, usage, requires_tag_evidence) in [
        (
            &["as", "tag-map-sites", "--help"][..],
            "tag-map-sites [OPTIONS] <CACHE>",
            true,
        ),
        (
            &["as", "patch-tag-map", "--help"][..],
            "patch-tag-map [OPTIONS]",
            true,
        ),
        (
            &["as", "decode-header", "--help"][..],
            "decode-header <FILE>",
            false,
        ),
    ] {
        let assertion = Command::cargo_bin("gore")
            .unwrap()
            .args(args)
            .assert()
            .success()
            .stdout(contains(usage));
        if requires_tag_evidence {
            assertion
                .stdout(contains("Binds.Cache"))
                .stdout(contains("GORE_AS_BINDS"))
                .stdout(contains("GORE_AS_USMAP"))
                .stdout(contains("fails closed"));
        }
    }
}

#[test]
fn configured_real_cache_default_patch_is_copy_on_write_and_noclobber() {
    let Some(cache) = std::env::var_os("GORE_AS_DEFAULT_CACHE") else {
        eprintln!("skip: set GORE_AS_DEFAULT_CACHE");
        return;
    };
    let cache = std::path::PathBuf::from(cache);
    let binds = cache.parent().unwrap().join("Binds.Cache");
    let inspect = Command::cargo_bin("gore")
        .unwrap()
        .args([
            "as",
            "default-sites",
            cache.to_str().unwrap(),
            "--class",
            "UItFo_Apple",
            "--field",
            "m_Value",
            "--json",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let document: Value = serde_json::from_slice(&inspect).expect("default-sites JSON");
    assert_eq!(document["site_count"], 1);
    let site = &document["sites"][0];
    assert_eq!(site["display_value"], "4");
    assert_eq!(site["expected_hex"], "04000000");
    assert_eq!(site["selector"]["field_owner"], "UItemDefinition");
    assert_eq!(site["selector"]["value_type"], "int");
    assert_eq!(site["selector"]["ancestry_profile"], Value::Null);

    let dir = tempfile::tempdir().unwrap();
    let selector = dir.path().join("site.json");
    std::fs::write(
        &selector,
        serde_json::to_vec_pretty(&site["selector"]).unwrap(),
    )
    .unwrap();
    let missing_owner_selector = dir.path().join("site-missing-owner.json");
    let mut missing_owner = site["selector"].clone();
    missing_owner.as_object_mut().unwrap().remove("field_owner");
    std::fs::write(
        &missing_owner_selector,
        serde_json::to_vec_pretty(&missing_owner).unwrap(),
    )
    .unwrap();
    let rejected_output = dir.path().join("rejected.Cache");
    Command::cargo_bin("gore")
        .unwrap()
        .args([
            "as",
            "patch-default",
            cache.to_str().unwrap(),
            "--selector",
            missing_owner_selector.to_str().unwrap(),
            "--expected-hex",
            "04000000",
            "--replacement-hex",
            "05000000",
            "--out",
            rejected_output.to_str().unwrap(),
            "--json",
        ])
        .assert()
        .failure()
        .stderr(contains("missing field `field_owner`"));
    assert!(!rejected_output.exists());

    let missing_type_selector = dir.path().join("site-missing-type.json");
    let mut missing_type = site["selector"].clone();
    missing_type.as_object_mut().unwrap().remove("value_type");
    std::fs::write(
        &missing_type_selector,
        serde_json::to_vec_pretty(&missing_type).unwrap(),
    )
    .unwrap();
    Command::cargo_bin("gore")
        .unwrap()
        .args([
            "as",
            "patch-default",
            cache.to_str().unwrap(),
            "--selector",
            missing_type_selector.to_str().unwrap(),
            "--expected-hex",
            "04000000",
            "--replacement-hex",
            "05000000",
            "--out",
            rejected_output.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(contains("missing field `value_type`"));
    assert!(!rejected_output.exists());

    let output = dir.path().join("patched.Cache");
    let patch = Command::cargo_bin("gore")
        .unwrap()
        .args([
            "as",
            "patch-default",
            cache.to_str().unwrap(),
            "--selector",
            selector.to_str().unwrap(),
            "--expected-hex",
            "04000000",
            "--replacement-hex",
            "05000000",
            "--out",
            output.to_str().unwrap(),
            "--json",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let receipt: Value = serde_json::from_slice(&patch).expect("patch JSON");
    assert_eq!(receipt["status"], "patched");
    assert_eq!(receipt["selector"]["field_owner"], "UItemDefinition");
    assert_eq!(receipt["selector"]["value_type"], "int");
    assert_eq!(receipt["expected_hex"], "04000000");
    assert_eq!(receipt["replacement_hex"], "05000000");
    let first_output = std::fs::read(&output).unwrap();
    assert_eq!(
        receipt["output"]["length"].as_u64().unwrap(),
        first_output.len() as u64
    );
    assert_eq!(
        receipt["output"]["sha256"],
        format!("{:x}", Sha256::digest(&first_output))
    );
    assert_eq!(
        first_output.len() as u64,
        std::fs::metadata(&cache).unwrap().len()
    );

    Command::cargo_bin("gore")
        .unwrap()
        .args([
            "as",
            "patch-default",
            cache.to_str().unwrap(),
            "--selector",
            selector.to_str().unwrap(),
            "--expected-hex",
            "04000000",
            "--replacement-hex",
            "05000000",
            "--out",
            output.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(contains("without clobbering"));
    assert_eq!(std::fs::read(&output).unwrap(), first_output);

    let reinspection = Command::cargo_bin("gore")
        .unwrap()
        .env("GORE_AS_BINDS", &binds)
        .args([
            "as",
            "default-sites",
            output.to_str().unwrap(),
            "--class",
            "UItFo_Apple",
            "--field",
            "m_Value",
            "--json",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let reinspection: Value = serde_json::from_slice(&reinspection).unwrap();
    assert_eq!(reinspection["sites"][0]["display_value"], "5");
    assert_eq!(reinspection["sites"][0]["expected_hex"], "05000000");
}

#[test]
fn configured_real_cache_native_ancestry_patch_is_rediscovered_and_fail_closed() {
    let Some(cache) = std::env::var_os("GORE_AS_DEFAULT_CACHE") else {
        eprintln!("skip: set GORE_AS_DEFAULT_CACHE");
        return;
    };
    let cache = std::path::PathBuf::from(cache);
    let original = std::fs::read(&cache).unwrap();
    let original_sha = Sha256::digest(&original);
    let binds = cache.parent().unwrap().join("Binds.Cache");
    let usmap = std::env::var_os("GORE_AS_USMAP")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            let directory = cache
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .join("Binaries/Win64/ue4ss");
            let mut maps: Vec<_> = std::fs::read_dir(directory)
                .unwrap()
                .map(|entry| entry.unwrap().path())
                .filter(|path| {
                    path.extension()
                        .and_then(|extension| extension.to_str())
                        .is_some_and(|extension| extension.eq_ignore_ascii_case("usmap"))
                })
                .collect();
            maps.sort();
            assert_eq!(maps.len(), 1, "configured Shipping test needs one USMAP");
            maps.remove(0)
        });

    // No path or filename is supplied: the CLI must discover by layout, then accept only the
    // exact sealed contents.
    let inspect = Command::cargo_bin("gore")
        .unwrap()
        .env_remove("GORE_AS_BINDS")
        .env_remove("GORE_AS_USMAP")
        .args([
            "as",
            "default-sites",
            cache.to_str().unwrap(),
            "--class",
            "UItMw_1H_Sword_Old_01",
            "--field",
            "m_Value",
            "--json",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let document: Value = serde_json::from_slice(&inspect).unwrap();
    assert_eq!(document["site_count"], 1);
    assert_eq!(document["stats"]["unresolved_fields"], 0);
    let site = &document["sites"][0];
    assert_eq!(site["selector"]["field_owner"], "UItemDefinition");
    assert_eq!(site["selector"]["field"], "m_Value");
    assert_eq!(site["selector"]["value_type"], "int");
    assert_eq!(site["expected_hex"], "0a000000");
    assert_eq!(site["display_value"], "10");
    assert_eq!(
        site["selector"]["ancestry_profile"],
        gore_as::cache::default_ancestry::DEFAULT_NATIVE_ANCESTRY_PROFILE_ID
    );

    let directory = tempfile::tempdir().unwrap();
    let selector = directory.path().join("sword-value.selector.json");
    std::fs::write(
        &selector,
        serde_json::to_vec_pretty(&site["selector"]).unwrap(),
    )
    .unwrap();

    // A mismatched USMAP must degrade to scalar-only evidence. The valid native selector then
    // becomes unresolvable and no output may appear.
    let invalid_usmap = directory.path().join("unknown.usmap");
    std::fs::write(&invalid_usmap, b"not a sealed usmap").unwrap();
    let rejected = directory.path().join("rejected.Cache");
    Command::cargo_bin("gore")
        .unwrap()
        .env("GORE_AS_USMAP", &invalid_usmap)
        .args([
            "as",
            "patch-default",
            cache.to_str().unwrap(),
            "--selector",
            selector.to_str().unwrap(),
            "--expected-hex",
            "0a000000",
            "--replacement-hex",
            "0b000000",
            "--out",
            rejected.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(contains("selector was not found"));
    assert!(!rejected.exists());

    let output = directory.path().join("SwordValue11.Cache");
    let patch = Command::cargo_bin("gore")
        .unwrap()
        .env("GORE_AS_BINDS", &binds)
        .env("GORE_AS_USMAP", &usmap)
        .args([
            "as",
            "patch-default",
            cache.to_str().unwrap(),
            "--selector",
            selector.to_str().unwrap(),
            "--expected-hex",
            "0a000000",
            "--replacement-hex",
            "0b000000",
            "--out",
            output.to_str().unwrap(),
            "--json",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let receipt: Value = serde_json::from_slice(&patch).unwrap();
    assert_eq!(receipt["status"], "patched");
    assert_eq!(receipt["expected_hex"], "0a000000");
    assert_eq!(receipt["replacement_hex"], "0b000000");
    assert_eq!(
        receipt["selector"]["ancestry_profile"],
        gore_as::cache::default_ancestry::DEFAULT_NATIVE_ANCESTRY_PROFILE_ID
    );

    let patched = std::fs::read(&output).unwrap();
    assert_eq!(patched.len(), original.len());
    assert_eq!(Sha256::digest(std::fs::read(&cache).unwrap()), original_sha);
    let offset = receipt["provenance"]["operand_offset"].as_u64().unwrap() as usize;
    let changed: Vec<_> = original
        .iter()
        .zip(&patched)
        .enumerate()
        .filter_map(|(index, (before, after))| (before != after).then_some(index))
        .collect();
    assert!(!changed.is_empty());
    assert!(changed
        .iter()
        .all(|index| (offset..offset + 4).contains(index)));

    let reinspection = Command::cargo_bin("gore")
        .unwrap()
        .env("GORE_AS_BINDS", &binds)
        .env("GORE_AS_USMAP", &usmap)
        .args([
            "as",
            "default-sites",
            output.to_str().unwrap(),
            "--class",
            "UItMw_1H_Sword_Old_01",
            "--field",
            "m_Value",
            "--json",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let reinspection: Value = serde_json::from_slice(&reinspection).unwrap();
    assert_eq!(reinspection["site_count"], 1);
    assert_eq!(reinspection["sites"][0]["display_value"], "11");
    assert_eq!(reinspection["sites"][0]["expected_hex"], "0b000000");
    assert_eq!(reinspection["sites"][0]["selector"], site["selector"]);

    Command::cargo_bin("gore")
        .unwrap()
        .args([
            "as",
            "patch-default",
            cache.to_str().unwrap(),
            "--selector",
            selector.to_str().unwrap(),
            "--expected-hex",
            "0a000000",
            "--replacement-hex",
            "0b000000",
            "--out",
            output.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(contains("without clobbering"));
    assert_eq!(std::fs::read(&output).unwrap(), patched);
}

#[test]
fn configured_shipping_tag_map_patch_is_operand_only_rediscovered_and_fail_closed() {
    let Some(cache) = std::env::var_os("GORE_AS_DEFAULT_CACHE") else {
        eprintln!("skip: set GORE_AS_DEFAULT_CACHE");
        return;
    };
    let cache = std::path::PathBuf::from(cache);
    let original = std::fs::read(&cache).unwrap();
    let original_sha = Sha256::digest(&original);
    let binds = cache.parent().unwrap().join("Binds.Cache");
    let usmap = std::env::var_os("GORE_AS_USMAP")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            let directory = cache
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .join("Binaries/Win64/ue4ss");
            let mut maps: Vec<_> = std::fs::read_dir(directory)
                .unwrap()
                .map(|entry| entry.unwrap().path())
                .filter(|path| {
                    path.extension()
                        .and_then(|extension| extension.to_str())
                        .is_some_and(|extension| extension.eq_ignore_ascii_case("usmap"))
                })
                .collect();
            maps.sort();
            assert_eq!(maps.len(), 1, "configured Shipping test needs one USMAP");
            maps.remove(0)
        });

    // Exercise layout-based Binds/USMAP discovery first. Tag-map commands must not silently
    // degrade to scalar-only evidence.
    let inspect = Command::cargo_bin("gore")
        .unwrap()
        .env_remove("GORE_AS_BINDS")
        .env_remove("GORE_AS_USMAP")
        .args([
            "as",
            "tag-map-sites",
            cache.to_str().unwrap(),
            "--module",
            "Items.GenericItems.WeaponsOneHandedGeneric",
            "--class",
            "UItMw_1H_Sword_Old_01",
            "--field",
            "m_DamageBase",
            "--tag",
            "Item_Damage_Physical_Edge",
            "--json",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let document: Value = serde_json::from_slice(&inspect).expect("tag-map-sites JSON");
    let binds_bytes = std::fs::read(&binds).unwrap();
    let usmap_bytes = std::fs::read(&usmap).unwrap();
    assert_eq!(document["format"], "gore-as-tag-map-sites-v1");
    assert_eq!(document["site_count"], 1);
    assert_eq!(document["cache"]["path"], cache.display().to_string());
    assert_eq!(document["cache"]["length"], original.len() as u64);
    assert_eq!(document["cache"]["sha256"], format!("{original_sha:x}"));
    assert_eq!(document["binds"]["path"], binds.display().to_string());
    assert_eq!(document["binds"]["length"], binds_bytes.len() as u64);
    assert_eq!(
        document["binds"]["sha256"],
        format!("{:x}", Sha256::digest(&binds_bytes))
    );
    assert_eq!(document["usmap"]["path"], usmap.display().to_string());
    assert_eq!(document["usmap"]["length"], usmap_bytes.len() as u64);
    assert_eq!(
        document["usmap"]["sha256"],
        format!("{:x}", Sha256::digest(&usmap_bytes))
    );

    let site = &document["sites"][0];
    assert_eq!(site["display_value"], "10");
    assert_eq!(site["expected_hex"], "00002041");
    assert_eq!(
        site["selector"]["module"],
        "Items.GenericItems.WeaponsOneHandedGeneric"
    );
    assert_eq!(site["selector"]["class"], "UItMw_1H_Sword_Old_01");
    assert_eq!(site["selector"]["field_owner"], "UWeaponDefinition");
    assert_eq!(site["selector"]["field"], "m_DamageBase");
    assert_eq!(site["selector"]["tag_module"], "");
    assert_eq!(site["selector"]["tag_namespace"], "GameplayTag");
    assert_eq!(site["selector"]["tag"], "Item_Damage_Physical_Edge");
    assert_eq!(site["selector"]["tag_is_string"], false);
    assert_eq!(site["selector"]["value_type"], "float32");
    assert_eq!(
        site["selector"]["map_proof_id"],
        gore_as::cache::default_ancestry::DEFAULT_GAMEPLAY_TAG_FLOAT32_MAP_PROOF_ID
    );
    assert_eq!(
        site["selector"]["ancestry_profile"],
        gore_as::cache::default_ancestry::DEFAULT_NATIVE_ANCESTRY_PROFILE_ID
    );
    assert_eq!(
        site["provenance"]["context_sha256"],
        "d02d0b0a7bd68cdae2d2e04b530fa959a94c2270cf178d406f64c474f1840312"
    );
    assert!(site["provenance"]["function"]
        .as_str()
        .unwrap()
        .ends_with("UItMw_1H_Sword_Old_01::__InitDefaults"));
    assert_eq!(
        site["provenance"]["field_schema_proof_id"],
        gore_as::cache::default_ancestry::DEFAULT_GAMEPLAY_TAG_FLOAT32_MAP_PROOF_ID
    );
    assert_eq!(site["provenance"]["length"], 4);

    let directory = tempfile::tempdir().unwrap();
    let selector = directory.path().join("sword-edge.selector.json");
    std::fs::write(
        &selector,
        serde_json::to_vec_pretty(&site["selector"]).unwrap(),
    )
    .unwrap();

    // Proof/profile constants are selector input and are rejected before any output is created.
    for (field, error_fragment) in [
        ("map_proof_id", "map_proof_id does not match"),
        ("ancestry_profile", "ancestry_profile does not match"),
    ] {
        let mut stale = site["selector"].clone();
        stale[field] = Value::String("sha256:stale".into());
        let stale_selector = directory.path().join(format!("stale-{field}.json"));
        std::fs::write(&stale_selector, serde_json::to_vec_pretty(&stale).unwrap()).unwrap();
        let rejected = directory.path().join(format!("stale-{field}.Cache"));
        Command::cargo_bin("gore")
            .unwrap()
            .args([
                "as",
                "patch-tag-map",
                cache.to_str().unwrap(),
                "--selector",
                stale_selector.to_str().unwrap(),
                "--expected-hex",
                "00002041",
                "--replacement-hex",
                "00003041",
                "--out",
                rejected.to_str().unwrap(),
                "--json",
            ])
            .assert()
            .failure()
            .stdout(predicates::str::is_empty())
            .stderr(contains(error_fragment));
        assert!(!rejected.exists());
    }

    // A present but mismatched mapping file is a hard failure for tag maps, not an ancestry
    // fallback. The output path must remain absent.
    let invalid_usmap = directory.path().join("unknown.usmap");
    std::fs::write(&invalid_usmap, b"not a sealed usmap").unwrap();
    let rejected = directory.path().join("invalid-usmap.Cache");
    Command::cargo_bin("gore")
        .unwrap()
        .env("GORE_AS_BINDS", &binds)
        .env("GORE_AS_USMAP", &invalid_usmap)
        .args([
            "as",
            "patch-tag-map",
            cache.to_str().unwrap(),
            "--selector",
            selector.to_str().unwrap(),
            "--expected-hex",
            "00002041",
            "--replacement-hex",
            "00003041",
            "--out",
            rejected.to_str().unwrap(),
            "--json",
        ])
        .assert()
        .failure()
        .stdout(predicates::str::is_empty())
        .stderr(contains("sealed cache/Binds/USMAP evidence is required"));
    assert!(!rejected.exists());

    let output = directory.path().join("SwordEdge11.Cache");
    let patch = Command::cargo_bin("gore")
        .unwrap()
        .env_remove("GORE_AS_BINDS")
        .env_remove("GORE_AS_USMAP")
        .args([
            "as",
            "patch-tag-map",
            cache.to_str().unwrap(),
            "--selector",
            selector.to_str().unwrap(),
            "--expected-hex",
            "00002041",
            "--replacement-hex",
            "00003041",
            "--out",
            output.to_str().unwrap(),
            "--json",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let receipt: Value = serde_json::from_slice(&patch).expect("patch-tag-map JSON");
    assert_eq!(receipt["format"], "gore-as-tag-map-patch-v1");
    assert_eq!(receipt["status"], "patched");
    assert_eq!(receipt["selector"], site["selector"]);
    assert_eq!(receipt["expected_hex"], "00002041");
    assert_eq!(receipt["replacement_hex"], "00003041");
    assert_eq!(receipt["input"], document["cache"]);
    assert_eq!(receipt["cache_guid"], document["cache_guid"]);
    assert_eq!(receipt["fingerprint"], document["fingerprint"]);
    assert_eq!(receipt["binds"], document["binds"]);
    assert_eq!(receipt["usmap"], document["usmap"]);
    assert_eq!(
        receipt["provenance"]["context_sha256"],
        site["provenance"]["context_sha256"]
    );

    let patched = std::fs::read(&output).unwrap();
    assert_eq!(patched.len(), original.len());
    assert_eq!(Sha256::digest(std::fs::read(&cache).unwrap()), original_sha);
    assert_eq!(
        receipt["output"]["sha256"],
        format!("{:x}", Sha256::digest(&patched))
    );
    assert_eq!(receipt["output"]["length"], patched.len() as u64);
    assert_eq!(receipt["output"]["path"], output.display().to_string());
    let offset = receipt["provenance"]["operand_offset"].as_u64().unwrap() as usize;
    assert_eq!(&original[offset..offset + 4], &[0x00, 0x00, 0x20, 0x41]);
    assert_eq!(&patched[offset..offset + 4], &[0x00, 0x00, 0x30, 0x41]);
    assert_eq!(&original[..offset], &patched[..offset]);
    assert_eq!(&original[offset + 4..], &patched[offset + 4..]);

    let reinspection = Command::cargo_bin("gore")
        .unwrap()
        .env("GORE_AS_BINDS", &binds)
        .env("GORE_AS_USMAP", &usmap)
        .args([
            "as",
            "tag-map-sites",
            output.to_str().unwrap(),
            "--module",
            "Items.GenericItems.WeaponsOneHandedGeneric",
            "--class",
            "UItMw_1H_Sword_Old_01",
            "--field",
            "m_DamageBase",
            "--tag",
            "Item_Damage_Physical_Edge",
            "--json",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let reinspection: Value = serde_json::from_slice(&reinspection).unwrap();
    assert_eq!(reinspection["site_count"], 1);
    assert_eq!(reinspection["sites"][0]["display_value"], "11");
    assert_eq!(reinspection["sites"][0]["expected_hex"], "00003041");
    assert_eq!(reinspection["sites"][0]["selector"], site["selector"]);
    assert_eq!(
        reinspection["sites"][0]["provenance"],
        receipt["provenance"]
    );
    assert_eq!(reinspection["fingerprint"], document["fingerprint"]);

    // The old expected bytes are now stale on the patched cache. A new target must not appear.
    let stale_output = directory.path().join("stale-cas.Cache");
    Command::cargo_bin("gore")
        .unwrap()
        .env("GORE_AS_BINDS", &binds)
        .env("GORE_AS_USMAP", &usmap)
        .args([
            "as",
            "patch-tag-map",
            output.to_str().unwrap(),
            "--selector",
            selector.to_str().unwrap(),
            "--expected-hex",
            "00002041",
            "--replacement-hex",
            "00004041",
            "--out",
            stale_output.to_str().unwrap(),
            "--json",
        ])
        .assert()
        .failure()
        .stdout(predicates::str::is_empty())
        .stderr(contains("expected operand drifted"));
    assert!(!stale_output.exists());

    // No-clobber is checked before mutation work and preserves the already verified artifact.
    Command::cargo_bin("gore")
        .unwrap()
        .args([
            "as",
            "patch-tag-map",
            cache.to_str().unwrap(),
            "--selector",
            selector.to_str().unwrap(),
            "--expected-hex",
            "00002041",
            "--replacement-hex",
            "00003041",
            "--out",
            output.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stdout(predicates::str::is_empty())
        .stderr(contains("without clobbering"));
    assert_eq!(std::fs::read(&output).unwrap(), patched);
}
