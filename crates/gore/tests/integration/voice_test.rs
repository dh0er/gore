use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

use assert_cmd::Command;
use predicates::str::contains;
use tempfile::TempDir;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

fn vorbis_ogg(sample_rate: u32) -> Vec<u8> {
    let mut packet = Vec::with_capacity(30);
    packet.extend_from_slice(b"\x01vorbis");
    packet.extend_from_slice(&0u32.to_le_bytes());
    packet.push(1);
    packet.extend_from_slice(&sample_rate.to_le_bytes());
    packet.extend_from_slice(&0i32.to_le_bytes());
    packet.extend_from_slice(&0i32.to_le_bytes());
    packet.extend_from_slice(&0i32.to_le_bytes());
    packet.push(0x86);
    packet.push(1);

    let mut page = Vec::with_capacity(28 + packet.len());
    page.extend_from_slice(b"OggS");
    page.push(0);
    page.push(0x02 | 0x04);
    page.extend_from_slice(&0u64.to_le_bytes());
    page.extend_from_slice(&7u32.to_le_bytes());
    page.extend_from_slice(&0u32.to_le_bytes());
    page.extend_from_slice(&0u32.to_le_bytes());
    page.push(1);
    page.push(packet.len() as u8);
    page.extend_from_slice(&packet);
    let checksum = ogg_crc(&page);
    page[22..26].copy_from_slice(&checksum.to_le_bytes());
    page
}

fn ogg_crc(page: &[u8]) -> u32 {
    let mut crc = 0u32;
    for (index, byte) in page.iter().copied().enumerate() {
        let byte = if (22..26).contains(&index) { 0 } else { byte };
        crc ^= u32::from(byte) << 24;
        for _ in 0..8 {
            crc = if crc & 0x8000_0000 != 0 {
                (crc << 1) ^ 0x04c1_1db7
            } else {
                crc << 1
            };
        }
    }
    crc
}

fn make_archive(path: &Path, entries: &[(&str, &[u8])]) {
    let file = File::create(path).unwrap();
    let mut writer = ZipWriter::new(file);
    for (name, bytes) in entries {
        writer
            .start_file(
                *name,
                SimpleFileOptions::default().compression_method(CompressionMethod::Stored),
            )
            .unwrap();
        writer.write_all(bytes).unwrap();
    }
    writer.finish().unwrap();
}

fn read_entry(path: &Path, name: &str) -> Vec<u8> {
    let mut archive = ZipArchive::new(File::open(path).unwrap()).unwrap();
    let mut entry = archive.by_name(name).unwrap();
    let mut bytes = Vec::new();
    entry.read_to_end(&mut bytes).unwrap();
    bytes
}

#[test]
fn list_json_is_machine_readable_and_index_alias_works() {
    let temp = TempDir::new().unwrap();
    let archive = temp.path().join("voices.zip");
    let ogg = vorbis_ogg(44_100);
    make_archive(
        &archive,
        &[("NPC/Line.ogg", &ogg), ("manifest.txt", b"meta")],
    );

    let output = Command::cargo_bin("gore")
        .unwrap()
        .args([
            "voice",
            "list",
            "--archive",
            archive.to_str().unwrap(),
            "--json",
        ])
        .output()
        .unwrap();
    assert!(output.status.success(), "{:?}", output);
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["entry_count"], 2);
    assert_eq!(value["entries"][0]["path"], "NPC/Line.ogg");
    assert_eq!(value["entries"][0]["compression"], "Stored");

    Command::cargo_bin("gore")
        .unwrap()
        .args(["voice", "index", "--archive", archive.to_str().unwrap()])
        .assert()
        .success()
        .stdout(contains("Voice archive:"))
        .stdout(contains("NPC/Line.ogg"));
}

#[test]
fn extract_rejects_ambiguous_basename_and_exact_path_does_not_clobber() {
    let temp = TempDir::new().unwrap();
    let archive = temp.path().join("voices.zip");
    let output = temp.path().join("extracted");
    let first = vorbis_ogg(22_050);
    let second = vorbis_ogg(48_000);
    make_archive(
        &archive,
        &[("NPC/A/Line.ogg", &first), ("NPC/B/LINE.OGG", &second)],
    );

    Command::cargo_bin("gore")
        .unwrap()
        .args([
            "voice",
            "extract",
            "--archive",
            archive.to_str().unwrap(),
            "--basename",
            "line.ogg",
            "--out",
            output.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(contains("ambiguous"))
        .stderr(contains("NPC/A/Line.ogg"));

    let extracted = output.join("NPC/A/Line.ogg");
    Command::cargo_bin("gore")
        .unwrap()
        .args([
            "voice",
            "extract",
            "--archive",
            archive.to_str().unwrap(),
            "--path",
            "NPC/A/Line.ogg",
            "--out",
            output.to_str().unwrap(),
        ])
        .assert()
        .success();
    assert_eq!(std::fs::read(&extracted).unwrap(), first);

    Command::cargo_bin("gore")
        .unwrap()
        .args([
            "voice",
            "extract",
            "--archive",
            archive.to_str().unwrap(),
            "--path",
            "NPC/A/Line.ogg",
            "--out",
            output.to_str().unwrap(),
        ])
        .assert()
        .failure();
    assert_eq!(std::fs::read(extracted).unwrap(), first);
}

#[test]
fn add_and_replace_write_verified_new_archives_only() {
    let temp = TempDir::new().unwrap();
    let input = temp.path().join("input.zip");
    let added_archive = temp.path().join("added.zip");
    let replaced_archive = temp.path().join("replaced.zip");
    let original = vorbis_ogg(22_050);
    let added = vorbis_ogg(44_100);
    let replacement = vorbis_ogg(48_000);
    let added_ogg = temp.path().join("added.ogg");
    let replacement_ogg = temp.path().join("replacement.ogg");
    make_archive(&input, &[("NPC/Old.ogg", &original)]);
    std::fs::write(&added_ogg, &added).unwrap();
    std::fs::write(&replacement_ogg, &replacement).unwrap();
    let pristine_input = std::fs::read(&input).unwrap();

    Command::cargo_bin("gore")
        .unwrap()
        .args([
            "voice",
            "add",
            "--archive",
            input.to_str().unwrap(),
            "--path",
            "NPC/New.ogg",
            "--ogg",
            added_ogg.to_str().unwrap(),
            "--out",
            added_archive.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(contains("Added NPC/New.ogg"));
    assert_eq!(std::fs::read(&input).unwrap(), pristine_input);
    assert_eq!(read_entry(&added_archive, "NPC/New.ogg"), added);

    Command::cargo_bin("gore")
        .unwrap()
        .args([
            "voice",
            "add",
            "--archive",
            input.to_str().unwrap(),
            "--path",
            "NPC/Other.ogg",
            "--ogg",
            added_ogg.to_str().unwrap(),
            "--out",
            added_archive.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(contains("output already exists"));

    Command::cargo_bin("gore")
        .unwrap()
        .args([
            "voice",
            "replace",
            "--archive",
            added_archive.to_str().unwrap(),
            "--basename",
            "old.ogg",
            "--ogg",
            replacement_ogg.to_str().unwrap(),
            "--out",
            replaced_archive.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(contains("Replaced NPC/Old.ogg"));
    assert_eq!(read_entry(&replaced_archive, "NPC/Old.ogg"), replacement);
    assert_eq!(read_entry(&replaced_archive, "NPC/New.ogg"), added);
}

#[test]
fn invalid_ogg_error_is_preserved_and_no_output_is_published() {
    let temp = TempDir::new().unwrap();
    let input = temp.path().join("input.zip");
    let invalid = temp.path().join("invalid.ogg");
    let output = temp.path().join("output.zip");
    let original = vorbis_ogg(22_050);
    make_archive(&input, &[("NPC/Old.ogg", &original)]);
    std::fs::write(&invalid, b"not an Ogg stream").unwrap();

    Command::cargo_bin("gore")
        .unwrap()
        .args([
            "voice",
            "add",
            "--archive",
            input.to_str().unwrap(),
            "--path",
            "NPC/New.ogg",
            "--ogg",
            invalid.to_str().unwrap(),
            "--out",
            output.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(contains("truncated Ogg page at byte 0"));
    assert!(!output.exists());
}

#[test]
fn apply_manifest_mixes_edits_in_one_pass_and_preserves_batch_order() {
    let temp = TempDir::new().unwrap();
    let input = temp.path().join("input.zip");
    let output = temp.path().join("output.zip");
    let manifest = temp.path().join("voice-manifest.json");
    let files = temp.path().join("files");
    std::fs::create_dir(&files).unwrap();

    let old_one = vorbis_ogg(11_025);
    let old_two = vorbis_ogg(16_000);
    let new_one = vorbis_ogg(22_050);
    let new_two = vorbis_ogg(24_000);
    let added_first = vorbis_ogg(44_100);
    let added_second = vorbis_ogg(48_000);
    make_archive(
        &input,
        &[
            ("NPC/One.ogg", &old_one),
            ("manifest.txt", b"untouched"),
            ("NPC/Two.ogg", &old_two),
        ],
    );
    std::fs::write(files.join("new-one.ogg"), &new_one).unwrap();
    std::fs::write(files.join("new-two.ogg"), &new_two).unwrap();
    std::fs::write(files.join("added-first.ogg"), &added_first).unwrap();
    std::fs::write(files.join("added-second.ogg"), &added_second).unwrap();
    std::fs::write(
        &manifest,
        serde_json::to_vec_pretty(&serde_json::json!({
            "format": 1,
            "edits": [
                {"op": "add", "path": "Added/First.ogg", "ogg": "files/added-first.ogg"},
                {"op": "replace", "path": "NPC/Two.ogg", "ogg": "files/new-two.ogg"},
                {"op": "add", "path": "Added/Second.ogg", "ogg": "files/added-second.ogg"},
                {"op": "replace", "path": "NPC/One.ogg", "ogg": "files/new-one.ogg"}
            ]
        }))
        .unwrap(),
    )
    .unwrap();
    let pristine = std::fs::read(&input).unwrap();

    let command = Command::cargo_bin("gore")
        .unwrap()
        .args([
            "voice",
            "apply",
            "--archive",
            input.to_str().unwrap(),
            "--manifest",
            manifest.to_str().unwrap(),
            "--out",
            output.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(command.status.success(), "{:?}", command);
    assert_eq!(std::fs::read(&input).unwrap(), pristine);
    let stdout = String::from_utf8(command.stdout).unwrap();
    assert!(stdout.contains("Applied 4 voice edit(s) in one pass"));
    let report_positions = [
        "Added Added/First.ogg",
        "Replaced NPC/Two.ogg",
        "Added Added/Second.ogg",
        "Replaced NPC/One.ogg",
    ]
    .map(|needle| stdout.find(needle).unwrap());
    assert!(report_positions.windows(2).all(|pair| pair[0] < pair[1]));

    let mut rewritten = ZipArchive::new(File::open(&output).unwrap()).unwrap();
    let names = (0..rewritten.len())
        .map(|index| rewritten.by_index(index).unwrap().name().to_owned())
        .collect::<Vec<_>>();
    assert_eq!(
        names,
        [
            "NPC/One.ogg",
            "manifest.txt",
            "NPC/Two.ogg",
            "Added/First.ogg",
            "Added/Second.ogg"
        ]
    );
    drop(rewritten);
    assert_eq!(read_entry(&output, "NPC/One.ogg"), new_one);
    assert_eq!(read_entry(&output, "NPC/Two.ogg"), new_two);
    assert_eq!(read_entry(&output, "Added/First.ogg"), added_first);
    assert_eq!(read_entry(&output, "Added/Second.ogg"), added_second);

    let published = std::fs::read(&output).unwrap();
    Command::cargo_bin("gore")
        .unwrap()
        .args([
            "voice",
            "apply-manifest",
            "--archive",
            input.to_str().unwrap(),
            "--manifest",
            manifest.to_str().unwrap(),
            "--out",
            output.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(contains("output already exists"));
    assert_eq!(std::fs::read(&output).unwrap(), published);
}

#[test]
fn apply_manifest_validates_later_ogg_before_publishing_any_output() {
    let temp = TempDir::new().unwrap();
    let input = temp.path().join("input.zip");
    let output = temp.path().join("output.zip");
    let manifest = temp.path().join("manifest.json");
    let valid = vorbis_ogg(44_100);
    make_archive(&input, &[("NPC/Old.ogg", &vorbis_ogg(22_050))]);
    std::fs::write(temp.path().join("valid.ogg"), &valid).unwrap();
    std::fs::write(temp.path().join("invalid.ogg"), b"not Ogg").unwrap();
    std::fs::write(
        &manifest,
        serde_json::to_vec(&serde_json::json!({
            "format": 1,
            "edits": [
                {"op": "add", "path": "Added/Valid.ogg", "ogg": "valid.ogg"},
                {"op": "replace", "path": "NPC/Old.ogg", "ogg": "invalid.ogg"}
            ]
        }))
        .unwrap(),
    )
    .unwrap();

    Command::cargo_bin("gore")
        .unwrap()
        .args([
            "voice",
            "apply-manifest",
            "--archive",
            input.to_str().unwrap(),
            "--manifest",
            manifest.to_str().unwrap(),
            "--out",
            output.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(contains("voice manifest edit #2"))
        .stderr(contains("truncated Ogg page at byte 0"));
    assert!(!output.exists());
}

#[test]
fn apply_manifest_rejects_traversal_and_duplicate_targets_before_writing() {
    let temp = TempDir::new().unwrap();
    let bundle = temp.path().join("bundle");
    std::fs::create_dir(&bundle).unwrap();
    let input = temp.path().join("input.zip");
    let original = vorbis_ogg(22_050);
    make_archive(&input, &[("NPC/Old.ogg", &original)]);
    std::fs::write(temp.path().join("outside.ogg"), vorbis_ogg(44_100)).unwrap();

    let traversal_manifest = bundle.join("traversal.json");
    let traversal_output = temp.path().join("traversal-output.zip");
    std::fs::write(
        &traversal_manifest,
        serde_json::to_vec(&serde_json::json!({
            "format": 1,
            "edits": [{"op": "add", "path": "Added/Outside.ogg", "ogg": "../outside.ogg"}]
        }))
        .unwrap(),
    )
    .unwrap();
    Command::cargo_bin("gore")
        .unwrap()
        .args([
            "voice",
            "apply-manifest",
            "--archive",
            input.to_str().unwrap(),
            "--manifest",
            traversal_manifest.to_str().unwrap(),
            "--out",
            traversal_output.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(contains("'..' components are forbidden"));
    assert!(!traversal_output.exists());

    let duplicate_manifest = bundle.join("duplicate.json");
    let duplicate_output = temp.path().join("duplicate-output.zip");
    std::fs::write(
        &duplicate_manifest,
        serde_json::to_vec(&serde_json::json!({
            "format": 1,
            "edits": [
                {"op": "add", "path": "Added/Same.ogg", "ogg": "missing-first.ogg"},
                {"op": "replace", "path": "added/same.OGG", "ogg": "missing-second.ogg"}
            ]
        }))
        .unwrap(),
    )
    .unwrap();
    Command::cargo_bin("gore")
        .unwrap()
        .args([
            "voice",
            "apply-manifest",
            "--archive",
            input.to_str().unwrap(),
            "--manifest",
            duplicate_manifest.to_str().unwrap(),
            "--out",
            duplicate_output.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(contains("same case-insensitive archive path"));
    assert!(!duplicate_output.exists());
}

#[test]
fn apply_manifest_rejects_unknown_format_unknown_op_and_empty_batch() {
    let temp = TempDir::new().unwrap();
    let input = temp.path().join("input.zip");
    make_archive(&input, &[("NPC/Old.ogg", &vorbis_ogg(22_050))]);
    let cases = [
        (
            "format.json",
            serde_json::json!({"format": 2, "edits": []}),
            "unsupported voice manifest format 2",
        ),
        (
            "empty.json",
            serde_json::json!({"format": 1, "edits": []}),
            "voice manifest contains no edits",
        ),
        (
            "operation.json",
            serde_json::json!({
                "format": 1,
                "edits": [{"op": "delete", "path": "NPC/Old.ogg", "ogg": "old.ogg"}]
            }),
            "unknown variant `delete`",
        ),
    ];

    for (index, (name, value, message)) in cases.into_iter().enumerate() {
        let manifest = temp.path().join(name);
        let output = temp.path().join(format!("schema-output-{index}.zip"));
        std::fs::write(&manifest, serde_json::to_vec(&value).unwrap()).unwrap();
        Command::cargo_bin("gore")
            .unwrap()
            .args([
                "voice",
                "apply-manifest",
                "--archive",
                input.to_str().unwrap(),
                "--manifest",
                manifest.to_str().unwrap(),
                "--out",
                output.to_str().unwrap(),
            ])
            .assert()
            .failure()
            .stderr(contains(message));
        assert!(!output.exists());
    }
}

#[cfg(any(unix, windows))]
#[test]
fn apply_manifest_rejects_symlinked_ogg_input() {
    let temp = TempDir::new().unwrap();
    let input = temp.path().join("input.zip");
    let output = temp.path().join("output.zip");
    let manifest = temp.path().join("manifest.json");
    let real = temp.path().join("real.ogg");
    let link = temp.path().join("linked.ogg");
    make_archive(&input, &[("NPC/Old.ogg", &vorbis_ogg(22_050))]);
    std::fs::write(&real, vorbis_ogg(44_100)).unwrap();
    if let Err(error) = create_file_symlink(&real, &link) {
        if error.kind() == std::io::ErrorKind::PermissionDenied {
            eprintln!("skipping symlink test because this Windows account cannot create links");
            return;
        }
        panic!("creating test symlink failed: {error}");
    }
    std::fs::write(
        &manifest,
        serde_json::to_vec(&serde_json::json!({
            "format": 1,
            "edits": [{"op": "add", "path": "Added/Linked.ogg", "ogg": "linked.ogg"}]
        }))
        .unwrap(),
    )
    .unwrap();

    Command::cargo_bin("gore")
        .unwrap()
        .args([
            "voice",
            "apply-manifest",
            "--archive",
            input.to_str().unwrap(),
            "--manifest",
            manifest.to_str().unwrap(),
            "--out",
            output.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(contains("symbolic link or reparse point"));
    assert!(!output.exists());
}

#[cfg(unix)]
fn create_file_symlink(target: &Path, link: &Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(target, link)
}

#[cfg(windows)]
fn create_file_symlink(target: &Path, link: &Path) -> std::io::Result<()> {
    std::os::windows::fs::symlink_file(target, link)
}
