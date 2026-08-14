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

fn minimal_cache_header(guid: [u8; 16], module_count: u32) -> Vec<u8> {
    let mut bytes = guid.to_vec();
    bytes.extend_from_slice(&gore_as::cache::header::CACHE_MAGIC.to_le_bytes());
    bytes.extend_from_slice(&module_count.to_le_bytes());
    bytes
}

fn expected_default_evidence(cache: &[u8]) -> (&'static str, &'static str, &'static str) {
    match format!("{:x}", Sha256::digest(cache)).as_str() {
        "1018f1cfe6b99a650eecb33afb96752d691d2088ead27808971b812f04ecb4c2" => (
            gore_generation::ROW_G1R_1_0_3.native_ancestry_profile_id,
            gore_generation::ROW_G1R_1_0_3.gameplay_tag_float32_map_proof_id,
            "d02d0b0a7bd68cdae2d2e04b530fa959a94c2270cf178d406f64c474f1840312",
        ),
        "757d8624f0c7480f63cc14a1ba2d7e43f461a529064b0c0cfbf523a54639e385" => (
            gore_generation::ROW_G1R_24169431.native_ancestry_profile_id,
            gore_generation::ROW_G1R_24169431.gameplay_tag_float32_map_proof_id,
            "7b6864cf0e12a886b80b1ad574bb08a42c0afe0d6ae6831fd441a90dcefb304c",
        ),
        other => panic!("configured cache is not an exact supported pristine generation: {other}"),
    }
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

/// A file that is not a module cache, shaped like the one people actually hit: `Binds.Cache` sits
/// beside the real cache, carries no `CACHE_MAGIC` at 0x10, and holds the ASCII of an embedded
/// script path where a Modules walk expects an `FString` length — `b"/Scr"` little-endian is the
/// 1919111983 out of the original report.
fn not_a_module_cache() -> Vec<u8> {
    let mut bytes = vec![0u8; 16];
    bytes.extend_from_slice(&0x0072_6579u32.to_le_bytes()); // 0x10: not CACHE_MAGIC
    bytes.extend_from_slice(&0x0000_7fffu32.to_le_bytes()); // 0x14: a count nothing backs
    bytes.extend_from_slice(b"/Script/Engine.Actor"); // 0x18: the FString length that lied
    bytes
}

#[test]
fn every_module_cache_subcommand_rejects_a_file_that_is_not_a_module_cache() {
    // `Binds.Cache` sits beside the real cache and is the file people point `as` at by mistake.
    // Every structural walker skips the outer header and re-reads the module count from 0x14, so
    // `decompile` started a Modules walk at 0x18, read the ASCII of an embedded script path as an
    // FString length, and blamed the container: `resolver: unexpected end of data at pos 28: needed
    // 1919111983 more bytes`. The decoy below reproduces exactly that read. `decode-header` had
    // parsed the magic all along but named neither the file nor a code, so it goes through the same
    // gate rather than standing apart from it. The two patch arms are absent only because they
    // validate their selector file first — they reach this gate through the same two helpers.
    let dir = tempfile::tempdir().unwrap();
    let decoy_path = dir.path().join("Binds.Cache");
    std::fs::write(&decoy_path, not_a_module_cache()).unwrap();
    let decoy = decoy_path.to_str().unwrap();
    let out_path = dir.path().join("never-written.Cache");
    let out = out_path.to_str().unwrap();
    let outdir_path = dir.path().join("never-emitted");
    let outdir = outdir_path.to_str().unwrap();

    for args in [
        &["as", "decode-header", decoy][..],
        &["as", "info", decoy][..],
        &["as", "walk", decoy][..],
        &["as", "decompile", decoy][..],
        &["as", "disasm", decoy][..],
        &["as", "emit", decoy][..],
        &["as", "emit-all", decoy, outdir][..],
        &["as", "static-names", decoy][..],
        &["as", "replace", decoy, decoy, "SomeModule", "-o", out][..],
        &["as", "splice", decoy, decoy, "-o", out][..],
        &["as", "extract", decoy, "SomeModule", "-o", out][..],
        &["as", "extract-remap", decoy, "SomeModule", decoy, "-o", out][..],
        &["as", "bytediff", decoy, decoy][..],
        &["as", "default-sites", decoy][..],
        &["as", "tag-map-sites", decoy][..],
    ] {
        let assertion = Command::cargo_bin("gore").unwrap().args(args).assert().failure();
        let stderr = String::from_utf8_lossy(&assertion.get_output().stderr).into_owned();
        let invocation = args.join(" ");
        assert!(
            stderr.contains("bad cache magic"),
            "`gore {invocation}` must name the format mismatch, got: {stderr}"
        );
        assert!(
            stderr.contains(decoy),
            "`gore {invocation}` must name the offending file, got: {stderr}"
        );
        assert!(
            !stderr.contains("unexpected end of data"),
            "`gore {invocation}` still blames the container walk: {stderr}"
        );
    }
    // Every `-o` arm above was handed the same destination, so this is what its name claims: a
    // refusal that has already opened, created or truncated the output leaves the file behind.
    assert!(
        !out_path.exists(),
        "a refused input must leave the -o path uncreated"
    );
    assert!(
        !outdir_path.exists(),
        "emit-all must refuse before it creates an output tree"
    );
}

#[test]
fn splice_and_replace_refuse_a_mini_from_another_cache_before_writing_output() {
    let dir = tempfile::tempdir().unwrap();
    let base_path = dir.path().join("base.Cache");
    let mini_path = dir.path().join("foreign-mini.Cache");
    let out_path = dir.path().join("never-written.Cache");

    let mut base = minimal_cache_header([0x11; 16], 0);
    base.extend_from_slice(&[0u8; 28]); // seven empty tail tables
    std::fs::write(&base_path, base).unwrap();
    // The GUID mismatch is checked before the declared module body is parsed.
    std::fs::write(&mini_path, minimal_cache_header([0x22; 16], 1)).unwrap();

    for args in [
        vec![
            "as",
            "splice",
            base_path.to_str().unwrap(),
            mini_path.to_str().unwrap(),
            "-o",
            out_path.to_str().unwrap(),
        ],
        vec![
            "as",
            "replace",
            base_path.to_str().unwrap(),
            mini_path.to_str().unwrap(),
            "SomeModule",
            "-o",
            out_path.to_str().unwrap(),
        ],
    ] {
        Command::cargo_bin("gore")
            .unwrap()
            .args(args)
            .assert()
            .failure()
            .stderr(contains("does not match target base GUID"));
        assert!(
            !out_path.exists(),
            "a foreign mini must be refused before the output path is created"
        );
    }
}

#[test]
fn catalog_knowledge_rejects_a_script_cache_that_is_not_a_module_cache() {
    // The same walkers, reached from another command family: `--script-cache` hands a user-chosen
    // path to the knowledge-caption extractor, which runs `parse_modules` and `RefResolver::build`
    // itself. Pointed at `Binds.Cache` it produced the identical invented length, worded as
    // `extracting knowledge captions from '…': unexpected end of data at pos 28: needed 1919111983
    // more bytes`. The case lives beside the `as` arms because the check it reaches is theirs.
    let dir = tempfile::tempdir().unwrap();
    let dump_path = dir.path().join("UE4SS_ObjectDump.txt");
    std::fs::write(
        &dump_path,
        "[0001] ASClass /Script/Angelscript.Topic_Diego_209799 [n: 1]\n",
    )
    .unwrap();
    let decoy_path = dir.path().join("Binds.Cache");
    std::fs::write(&decoy_path, not_a_module_cache()).unwrap();
    let decoy = decoy_path.to_str().unwrap();
    let out_path = dir.path().join("never-written.json");

    let assertion = Command::cargo_bin("gore")
        .unwrap()
        .args([
            "catalog",
            "--kind",
            "knowledge",
            dump_path.to_str().unwrap(),
            "--script-cache",
            decoy,
            "-o",
            out_path.to_str().unwrap(),
        ])
        .assert()
        .failure();
    let stderr = String::from_utf8_lossy(&assertion.get_output().stderr).into_owned();
    assert!(
        stderr.contains("bad cache magic"),
        "`gore catalog --kind knowledge --script-cache` must name the format mismatch, got: {stderr}"
    );
    assert!(
        stderr.contains(decoy),
        "`gore catalog --kind knowledge --script-cache` must name the offending file, got: {stderr}"
    );
    assert!(
        !stderr.contains("unexpected end of data"),
        "`gore catalog --kind knowledge --script-cache` still blames the container walk: {stderr}"
    );
    assert!(
        !out_path.exists(),
        "a refused script cache must leave the catalog output uncreated"
    );
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
fn configured_hotfix_24169431_cli_loads_only_the_exact_profile_pair() {
    let Some(game) = std::env::var_os("GORE_AS_HOTFIX_24169431_GAME") else {
        eprintln!("skip: set GORE_AS_HOTFIX_24169431_GAME");
        return;
    };
    let cache = std::path::PathBuf::from(game).join("G1R/Script/PrecompiledScript_Shipping.Cache");
    let assertion = Command::cargo_bin("gore")
        .unwrap()
        .env_remove("GORE_AS_BINDS")
        .env_remove("GORE_AS_USMAP")
        .args([
            "as",
            "default-sites",
            cache.to_str().unwrap(),
            "--field",
            "m_Weight",
            "--json",
        ])
        .assert()
        .success()
        .stderr(contains(
            gore_generation::ROW_G1R_24169431.native_ancestry_profile_id,
        ));
    let document: Value = serde_json::from_slice(&assertion.get_output().stdout)
        .expect("BuildID-24169431 default-sites JSON");
    assert_eq!(document["site_count"], 109);
    assert_eq!(document["stats"]["unresolved_fields"], 0);
    for site in document["sites"].as_array().expect("weight sites") {
        assert_eq!(site["selector"]["field_owner"], "UItemDefinition");
        assert_eq!(site["selector"]["field"], "m_Weight");
        assert_eq!(site["selector"]["value_type"], "float32");
        assert_eq!(
            site["selector"]["ancestry_profile"],
            gore_generation::ROW_G1R_24169431.native_ancestry_profile_id
        );
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
    let (expected_profile, _, _) = expected_default_evidence(&original);
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
    assert_eq!(site["selector"]["ancestry_profile"], expected_profile);

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
    assert_eq!(receipt["selector"]["ancestry_profile"], expected_profile);

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
    let (expected_profile, expected_map_proof, expected_context_sha256) =
        expected_default_evidence(&original);
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
    assert_eq!(site["selector"]["map_proof_id"], expected_map_proof);
    assert_eq!(site["selector"]["ancestry_profile"], expected_profile);
    assert_eq!(
        site["provenance"]["context_sha256"],
        expected_context_sha256
    );
    assert!(site["provenance"]["function"]
        .as_str()
        .unwrap()
        .ends_with("UItMw_1H_Sword_Old_01::__InitDefaults"));
    assert_eq!(
        site["provenance"]["field_schema_proof_id"],
        expected_map_proof
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
    for field in ["map_proof_id", "ancestry_profile"] {
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
            .stderr(contains("is not one exact sealed generation pair"));
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

// ---------------------------------------------------------------------------------------------
// `gore as qualify`
//
// The command that is meant to make the next Steam patch a data change. What these cases are for
// is the one input a green seal cannot vouch for: the USMAP is generated by UE4SS on the user's
// machine, not shipped, so a dump from the previous build passes its own hash check while
// describing the wrong game. Every case below is about refusing that rather than resolving it.
// ---------------------------------------------------------------------------------------------

/// A game tree with the three sealed files where every other command looks for them, plus
/// `usmaps` reflection dumps under `ue4ss/`.
struct QualifyFixture {
    _temp: tempfile::TempDir,
    game: std::path::PathBuf,
    executable: Vec<u8>,
    cache: Vec<u8>,
    binds: Vec<u8>,
}

fn qualify_fixture(usmaps: usize) -> QualifyFixture {
    let temp = tempfile::TempDir::new().unwrap();
    let game = temp.path().join("Gothic 1 Remake");
    let win64 = game.join("G1R/Binaries/Win64");
    let script = game.join("G1R/Script");
    let dumps = win64.join("ue4ss");
    std::fs::create_dir_all(&dumps).unwrap();
    std::fs::create_dir_all(&script).unwrap();

    let cache = std::fs::read(fixture_path()).unwrap();
    std::fs::write(script.join("PrecompiledScript_Shipping.Cache"), &cache).unwrap();
    let binds = b"fixture Binds.Cache, deliberately not a parseable bind database".to_vec();
    std::fs::write(script.join("Binds.Cache"), &binds).unwrap();

    // The executable has to name the classes the dumps declare, because that naming is the only
    // file-only tie between a reflection dump and the build it claims to describe, and it is what
    // the command decides on. A fixture whose executable named none of them would exercise the
    // "no dump fits" path in every case and never reach the interesting ones.
    let mut executable = b"G1R-Win64-Shipping fixture; not a portable executable\0".to_vec();
    for index in 0..usmaps {
        let path = dumps.join(format!("G1R-5.4.3-{index}-fixture.usmap"));
        gore_asset::test_fixture::write_valid_usmap(&path).unwrap();
        if index == 0 {
            let schemas =
                gore_asset::SchemaDb::from_usmap(&std::fs::read(&path).unwrap()).unwrap();
            for record in schemas.schemas() {
                executable.extend_from_slice(record.name.as_bytes());
                executable.push(0);
            }
        }
    }
    std::fs::write(win64.join("G1R-Win64-Shipping.exe"), &executable).unwrap();
    QualifyFixture {
        _temp: temp,
        game,
        executable,
        cache,
        binds,
    }
}

fn qualify(fixture: &QualifyFixture) -> Command {
    let mut command = Command::cargo_bin("gore").unwrap();
    command
        .env_remove("GORE_AS_BINDS")
        .env_remove("GORE_AS_USMAP")
        .env("GORE_DISABLE_GAME_AUTODETECT", "1")
        .args(["as", "qualify", "--game"])
        .arg(&fixture.game)
        .arg("--json");
    command
}

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[test]
fn qualify_refuses_when_no_reflection_dump_can_be_tied_to_the_executable() {
    // The failure this rules out is a qualification that shrugs. Without a dump there is no class
    // graph, no ancestry and no profile ID, and the three generations sealed before this command
    // existed were qualified by a person who had to remember that. A warning here reads as
    // "mostly fine", and mostly fine is what puts a wrong digest in the table.
    let fixture = qualify_fixture(0);
    let assertion = qualify(&fixture)
        .assert()
        .failure()
        .stderr(contains("AS_QUALIFY_USMAP"))
        .stderr(contains("re-dump"));

    // The refusal still emits its document: the three file seals are the cheapest inventory a
    // maintainer has and they are correct whatever the dump situation turned out to be.
    let document: Value = serde_json::from_slice(&assertion.get_output().stdout)
        .expect("a refusing qualify run still emits its document");
    assert_eq!(document["format"], "gore-as-qualify-v1");
    assert_eq!(
        document["inputs"]["executable"]["sha256"],
        sha256_hex(&fixture.executable)
    );
    assert_eq!(
        document["inputs"]["executable"]["byte_len"],
        fixture.executable.len() as u64
    );
    assert_eq!(
        document["inputs"]["shipping_cache"]["sha256"],
        sha256_hex(&fixture.cache)
    );
    assert_eq!(
        document["inputs"]["binds_cache"]["sha256"],
        sha256_hex(&fixture.binds)
    );
    assert!(document["usmap_selection"]["sealed"].is_null());
    assert_eq!(document["complete"], false);
}

#[test]
fn qualify_refuses_to_choose_between_two_dumps_that_fit_this_build_equally_well() {
    // The whole reason the command exists. Two dumps sit side by side in `ue4ss/` after an update —
    // one re-dumped against this build, one left over — and the stale one still hashes to its own
    // sealed value and parses perfectly. Picking either is picking at random, and the wrong pick
    // produces a profile that is internally consistent over the previous game's class graph.
    let fixture = qualify_fixture(2);
    let assertion = qualify(&fixture)
        .assert()
        .failure()
        .stderr(contains("refusing to choose"))
        .stderr(contains("--usmap"));

    let document: Value = serde_json::from_slice(&assertion.get_output().stdout).expect("document");
    assert!(document["usmap_selection"]["sealed"].is_null());
    assert_eq!(
        document["usmap_selection"]["examined"]
            .as_array()
            .expect("both dumps are reported, not only the one that lost")
            .len(),
        2
    );
}

#[test]
fn qualify_names_every_row_value_it_could_not_derive_and_refuses_to_render_it() {
    // A draft that quietly omitted a value would be the most dangerous thing this command could
    // produce: twenty-four fields of the right types compile whatever they say, and a zero digest
    // reads as a measurement. So an underived value renders as something that cannot be pasted,
    // and the document names it twice — in `row.missing`, and in `unavailable` with the reason.
    let fixture = qualify_fixture(1);
    let assertion = qualify(&fixture).assert();
    let document: Value = serde_json::from_slice(&assertion.get_output().stdout).expect("document");

    assert_eq!(
        document["usmap_selection"]["sealed"]["sha256"]
            .as_str()
            .map(str::len),
        Some(64)
    );
    let missing: Vec<&str> = document["row"]["missing"]
        .as_array()
        .expect("missing fields")
        .iter()
        .map(|value| value.as_str().expect("a field name"))
        .collect();
    for field in [
        "script_cache_mutation_stable_sha256",
        "binds_field_map_sha256",
        "binds_class_path_map_sha256",
        "resolved_class_profile_sha256",
        "native_ancestry_profile_id",
    ] {
        assert!(
            missing.contains(&field),
            "{field} is not reported as missing"
        );
    }
    let literal = document["row"]["literal"].as_str().expect("a row literal");
    assert!(
        literal.contains(gore_generation::qualify::UNDERIVED),
        "an underived value must render as something that does not compile"
    );
    assert!(
        !literal.contains("sha256: hex(\"0000000000000000"),
        "an underived digest must never render as zeroes"
    );
    assert!(
        !document["unavailable"]
            .as_array()
            .expect("unavailable")
            .is_empty(),
        "every value the command could not reach has to carry the reason it could not"
    );
    assert!(
        document["still_to_do"]
            .as_array()
            .expect("still to do")
            .iter()
            .any(|step| step.as_str().unwrap_or_default().contains("lib.rs")),
        "qualifying is not admitting: the row still has to be added by a person"
    );
}

/// Reads the installed game, writes nothing to it, and needs the real `.usmap` a UE4SS run leaves
/// beside the executable. Set `GORE_AS_QUALIFY_GAME` to point somewhere other than the configured
/// install.
#[test]
#[ignore = "local real-game proof; reads the installed game and never writes to it"]
fn qualify_reproduces_the_sealed_values_of_the_generation_it_is_run_against() {
    // The case that makes the command trustworthy rather than merely careful. Everything above
    // proves `qualify` refuses correctly; this proves it *derives* correctly, by re-deriving an
    // audited generation from the installed bytes and comparing against the row that generation is
    // already sealed as. A derivation that is subtly wrong produces a seal that agrees with itself
    // and describes nothing, and this is the only place that difference is visible.
    let mut command = Command::cargo_bin("gore").unwrap();
    command
        .env_remove("GORE_AS_BINDS")
        .env_remove("GORE_AS_USMAP");
    if let Some(game) = std::env::var_os("GORE_AS_QUALIFY_GAME") {
        command.arg("--game").arg(game);
    }
    let assertion = command.args(["as", "qualify", "--json"]).assert().success();
    let document: Value = serde_json::from_slice(&assertion.get_output().stdout).expect("document");

    let executable = document["inputs"]["executable"]["sha256"]
        .as_str()
        .expect("an executable seal");
    let row = gore_generation::rows()
        .iter()
        .find(|row| gore_generation::qualify::hex_lower(&row.executable.sha256) == executable)
        .expect("the installed game is one of the audited generations");
    assert_eq!(document["already_audited_as"], row.id);

    let field = |name: &str| -> String {
        document["row"]["fields"]
            .as_array()
            .expect("row fields")
            .iter()
            .find(|entry| entry["field"] == name)
            .and_then(|entry| entry["value"].as_str())
            .unwrap_or_else(|| panic!("{name} was not derived"))
            .to_owned()
    };
    assert_eq!(
        field("usmap"),
        format!(
            "{} bytes / sha256 {}",
            row.usmap.byte_len,
            gore_generation::qualify::hex_lower(&row.usmap.sha256)
        ),
        "the dump this run sealed is not the dump the row is sealed against"
    );
    assert_eq!(
        field("script_cache_guid"),
        gore_generation::qualify::hex_lower(&row.script_cache_guid)
    );
    assert_eq!(
        field("usmap_class_graph_sha256"),
        gore_generation::qualify::hex_lower(&row.usmap_class_graph_sha256),
        "the class-graph digest derived here disagrees with the one the admission gate checks"
    );
    assert_eq!(
        field("gameplay_tag_float32_map_profile_sha256"),
        gore_generation::qualify::hex_lower(&row.gameplay_tag_float32_map_profile_sha256)
    );
    assert_eq!(
        field("native_ancestry_profile_id"),
        row.native_ancestry_profile_id
    );
    assert_eq!(
        field("gameplay_tag_float32_map_proof_id"),
        row.gameplay_tag_float32_map_proof_id
    );
    assert_eq!(
        field("scalar_default_operand_count"),
        row.scalar_default_operand_count.to_string()
    );
    assert_eq!(
        document["qualification"]["class_count"],
        serde_json::json!(qualified_number(row.id, "class_count"))
    );
    assert_eq!(
        document["qualification"]["gameplay_tag_float32_map_field_count"],
        serde_json::json!(qualified_number(
            row.id,
            "gameplay_tag_float32_map_field_count"
        ))
    );
    assert_eq!(
        document["qualification"]["unresolved_fields_with_ancestry"],
        0
    );
    assert!(
        document["curated_records"]["all_reproduce"]
            .as_bool()
            .unwrap_or(false),
        "every curated module must still emit the source the story catalog seals"
    );

    // Step 7 of `docs/reference/game-updates.md`, reproduced rather than remembered. The counts
    // this run measures against the generation before it have to be the ones the artifact records,
    // and none of them may have fallen — a digest is silent about a parser that dropped rows.
    let previous = gore_generation::rows()
        .iter()
        .position(|candidate| candidate.id == row.id)
        .and_then(|index| index.checked_sub(1))
        .map(|index| gore_generation::rows()[index].id);
    assert_eq!(document["compared_against"], serde_json::json!(previous));
    let counts = document["counts"].as_array().expect("counts");
    let count = |name: &str| -> &Value {
        counts
            .iter()
            .find(|count| count["name"] == name)
            .unwrap_or_else(|| panic!("{name} was not counted"))
    };
    assert_eq!(
        count("scalar default windows")["observed"],
        serde_json::json!(row.scalar_default_operand_count)
    );
    assert_eq!(
        count("bridged classes")["observed"],
        serde_json::json!(qualified_number(row.id, "class_count"))
    );
    assert_eq!(
        count("USMAP tag-map field declarations")["observed"],
        serde_json::json!(qualified_number(
            row.id,
            "gameplay_tag_float32_map_field_count"
        ))
    );
    for entry in counts {
        assert_eq!(
            entry["fell"],
            serde_json::json!(false),
            "{} fell against the previous generation",
            entry["name"]
        );
    }
}

fn qualified_number(row_id: &str, key: &str) -> u64 {
    let (_, artifact) = gore_generation::QUALIFICATION_ARTIFACTS
        .iter()
        .find(|(id, _)| *id == row_id)
        .expect("every row carries a committed qualification artifact");
    serde_json::from_str::<Value>(artifact).expect("artifact JSON")[key]
        .as_u64()
        .unwrap_or_else(|| panic!("{key} is not a number in {row_id}'s artifact"))
}
