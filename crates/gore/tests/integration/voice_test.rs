use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

use assert_cmd::Command;
use predicates::prelude::PredicateBooleanExt;
use predicates::str::contains;
use tempfile::TempDir;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

fn vorbis_ogg(sample_rate: u32) -> Vec<u8> {
    let mut data = include_bytes!("../../../gore-vo/testdata/tiny-vorbis.ogg").to_vec();
    let ident = data
        .windows(7)
        .position(|window| window == b"\x01vorbis")
        .expect("fixture has Vorbis identification");
    data[ident + 12..ident + 16].copy_from_slice(&sample_rate.to_le_bytes());

    let mut offset = 0usize;
    while offset < data.len() {
        let segment_count = usize::from(data[offset + 26]);
        let header_len = 27 + segment_count;
        let body_len = data[offset + 27..offset + header_len]
            .iter()
            .map(|value| usize::from(*value))
            .sum::<usize>();
        let page_len = header_len + body_len;
        data[offset + 22..offset + 26].fill(0);
        let checksum = ogg_crc(&data[offset..offset + page_len]);
        data[offset + 22..offset + 26].copy_from_slice(&checksum.to_le_bytes());
        offset += page_len;
    }
    data
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

fn make_archive_with_directories(path: &Path, directories: &[&str], entries: &[(&str, &[u8])]) {
    let file = File::create(path).unwrap();
    let mut writer = ZipWriter::new(file);
    for directory in directories {
        writer
            .add_directory(*directory, SimpleFileOptions::default())
            .unwrap();
    }
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

fn numbered_entry_names(prefix: &str, count: usize) -> Vec<String> {
    (0..count)
        .map(|index| format!("{prefix}{index:02}.ogg"))
        .collect()
}

fn stored_entries<'a>(names: &'a [String], payload: &'a [u8]) -> Vec<(&'a str, &'a [u8])> {
    names.iter().map(|name| (name.as_str(), payload)).collect()
}

fn list_json(archive: &Path, extra: &[&str]) -> serde_json::Value {
    let mut args = vec![
        "voice",
        "list",
        "--archive",
        archive.to_str().unwrap(),
        "--json",
    ];
    args.extend_from_slice(extra);
    let output = Command::cargo_bin("gore")
        .unwrap()
        .args(args)
        .output()
        .unwrap();
    assert!(output.status.success(), "{:?}", output);
    serde_json::from_slice(&output.stdout).unwrap()
}

fn list_stdout(archive: &Path, extra: &[&str]) -> String {
    let mut args = vec!["voice", "list", "--archive", archive.to_str().unwrap()];
    args.extend_from_slice(extra);
    let output = Command::cargo_bin("gore")
        .unwrap()
        .args(args)
        .output()
        .unwrap();
    assert!(output.status.success(), "{:?}", output);
    String::from_utf8(output.stdout).unwrap()
}

/// Collapse every run of whitespace, so a comparison against `--help` survives clap's own wrapping.
fn collapsed(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
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
fn list_says_how_many_entries_matched_when_it_stops_at_max() {
    // The shipped `foreign.zip` emitted 287,581 characters and the German archive is 42x that, so
    // the listing has to stop somewhere. A document that stopped without carrying its own counts
    // would read exactly like a complete one, and an agent would take the first page for the whole
    // archive and conclude a recording does not exist.
    let temp = TempDir::new().unwrap();
    let archive = temp.path().join("voices.zip");
    let names = numbered_entry_names("NPC/Line", 12);
    make_archive(&archive, &stored_entries(&names, b"audio"));

    let value = list_json(&archive, &["--max", "5"]);

    assert_eq!(value["entry_count"], 12);
    assert_eq!(value["matched_count"], 12);
    assert_eq!(value["listed_count"], 5);
    assert_eq!(value["truncated"], true);
    assert_eq!(value["entries"].as_array().unwrap().len(), 5);
    let notice = value["truncation_notice"].as_str().unwrap();
    assert!(
        notice.contains("12 entries matched") && notice.contains("first 5"),
        "the notice must name both numbers, got {notice:?}"
    );
}

#[test]
fn the_truncation_notice_never_hands_back_a_max_that_would_be_cut_off_in_transit() {
    // The notice used to end with `(--max 33073 lists them all)`. An agent through `gore mcp serve`
    // follows that literally, and 33,000 pretty-printed entries is an ~11 MB document against a
    // 256 KiB result budget: the cut lands inside `entries` and what arrives no longer parses --
    // the exact failure this bound exists to prevent, restated as advice. The remedy a notice names
    // has to be one that works.
    let temp = TempDir::new().unwrap();
    let archive = temp.path().join("voices.zip");
    let names = numbered_entry_names("NPC/Line", 12);
    make_archive(&archive, &stored_entries(&names, b"audio"));

    let value = list_json(&archive, &["--max", "5"]);
    let notice = value["truncation_notice"].as_str().unwrap();

    assert!(
        notice.contains("--filter"),
        "the notice must name the flag that narrows the query, got {notice:?}"
    );
    assert!(
        !notice.contains("--max 12"),
        "the notice must not spell out a --max that lists them all, got {notice:?}"
    );
}

#[test]
fn max_zero_lists_nothing_and_reports_only_the_counts() {
    // Many tools read 0 as "unlimited", so an agent asking for everything this way would get the
    // opposite. Which one it is now stands in the flag's own help, and this is the behaviour that
    // help describes: the counts still answer "how many match?" for the price of one call.
    let temp = TempDir::new().unwrap();
    let archive = temp.path().join("voices.zip");
    let names = numbered_entry_names("NPC/Line", 3);
    make_archive(&archive, &stored_entries(&names, b"audio"));

    let value = list_json(&archive, &["--max", "0"]);

    assert_eq!(value["entry_count"], 3);
    assert_eq!(value["matched_count"], 3);
    assert_eq!(value["listed_count"], 0);
    assert_eq!(value["truncated"], true);
    assert_eq!(value["complete"], false);
    assert!(value["entries"].as_array().unwrap().is_empty());
}

#[test]
fn list_prints_a_truncation_marker_the_mcp_guide_teaches_people_to_recognise() {
    // docs/guide/mcp.md tells every reader that output ending in `… [truncated]` means "narrow the
    // query with the command's own filter". That row was a dead end for `voice list`, which had
    // neither a marker nor a filter; both halves of the promise are asserted here.
    let temp = TempDir::new().unwrap();
    let archive = temp.path().join("voices.zip");
    let names = numbered_entry_names("NPC/Line", 12);
    make_archive(&archive, &stored_entries(&names, b"audio"));

    Command::cargo_bin("gore")
        .unwrap()
        .args([
            "voice",
            "list",
            "--archive",
            archive.to_str().unwrap(),
            "--max",
            "5",
        ])
        .assert()
        .success()
        .stdout(contains("… [truncated:"))
        .stdout(contains("--filter"));
}

#[test]
fn list_leaves_a_complete_result_unlabelled() {
    // A truncation notice on a listing that hid nothing is its own kind of lie: it would send a
    // caller looking for entries that are already in front of them.
    let temp = TempDir::new().unwrap();
    let archive = temp.path().join("voices.zip");
    let names = numbered_entry_names("NPC/Line", 3);
    make_archive(&archive, &stored_entries(&names, b"audio"));

    let value = list_json(&archive, &["--max", "100"]);

    assert_eq!(value["entry_count"], 3);
    assert_eq!(value["matched_count"], 3);
    assert_eq!(value["listed_count"], 3);
    assert_eq!(value["truncated"], false);
    assert_eq!(value["complete"], true);
    assert!(
        value.get("truncation_notice").is_none(),
        "a complete listing must carry no notice, got {value}"
    );
}

#[test]
fn list_filter_matches_regardless_of_case_and_the_counts_follow_it() {
    // Real archives hold `LINE_ONE.OGG` beside `line.ogg`, which is why `resolve` and `match-line`
    // both fold case. A case-sensitive filter here would answer "no such entry" when the truth is
    // "wrong case" -- a false negative dressed as a fact.
    let temp = TempDir::new().unwrap();
    let archive = temp.path().join("voices.zip");
    make_archive(
        &archive,
        &[
            ("NPC/other.ogg", b"other"),
            ("NPC/LINE_ONE.OGG", b"line one"),
        ],
    );

    let value = list_json(&archive, &["--filter", "line_one"]);

    assert_eq!(value["entry_count"], 2);
    assert_eq!(value["matched_count"], 1);
    assert_eq!(value["listed_count"], 1);
    assert_eq!(value["entries"][0]["path"], "NPC/LINE_ONE.OGG");
    // `index` stays the archive's own central-directory index. Renumbering it to the position in
    // a filtered listing would make `extract --path` and the ambiguity messages name slots that
    // do not exist in the file.
    assert_eq!(value["entries"][0]["index"], 1);
}

#[test]
fn list_filter_folds_case_the_way_the_basename_selector_does() {
    // `ArchiveIndex::resolve`, which `--filter` exists to feed, folds with `str::to_lowercase`.
    // Folding only ASCII here moved the false negative one code point up instead of removing it:
    // in a German archive -- the documented target -- `--filter MÜLLER` listed nothing about an
    // entry that `extract --basename` resolves in the same breath. Two folds, one query, and the
    // caller is told the recording does not exist.
    let temp = TempDir::new().unwrap();
    let archive = temp.path().join("voices.zip");
    let extracted = temp.path().join("extracted");
    let ogg = vorbis_ogg(44_100);
    make_archive(
        &archive,
        &[
            ("NPC/Quest/DIA_Müller_01.ogg", &ogg),
            ("NPC/Quest/DIA_Other.ogg", b"other"),
        ],
    );

    let value = list_json(&archive, &["--filter", "MÜLLER"]);

    assert_eq!(value["matched_count"], 1);
    assert_eq!(value["entries"][0]["path"], "NPC/Quest/DIA_Müller_01.ogg");

    Command::cargo_bin("gore")
        .unwrap()
        .args([
            "voice",
            "extract",
            "--archive",
            archive.to_str().unwrap(),
            "--basename",
            "DIA_MÜLLER_01.OGG",
            "--out",
            extracted.to_str().unwrap(),
        ])
        .assert()
        .success();
    assert_eq!(
        std::fs::read(extracted.join("NPC/Quest/DIA_Müller_01.ogg")).unwrap(),
        ogg,
        "the selector resolves what the filter must also list"
    );
}

#[test]
fn a_filter_that_matches_nothing_does_not_read_like_an_empty_archive() {
    // The human path printed the archive total, a column header and no rows -- which is exactly
    // what an empty archive prints. Only the JSON document said how many the filter kept, so
    // docs/guide/voice.md's "it always says what it left out" was half true.
    let temp = TempDir::new().unwrap();
    let archive = temp.path().join("voices.zip");
    let names = numbered_entry_names("NPC/Line", 3);
    make_archive(&archive, &stored_entries(&names, b"audio"));

    let empty = list_stdout(&archive, &["--filter", "zzz"]);
    assert!(
        empty.contains("3 entries, 0 matched --filter"),
        "a filter that kept nothing must say so, got {empty:?}"
    );

    let matched = list_stdout(&archive, &["--filter", "line0"]);
    assert!(
        matched.contains("3 entries, 3 matched --filter"),
        "the header must count what the filter kept, got {matched:?}"
    );

    let unfiltered = list_stdout(&archive, &[]);
    assert!(
        !unfiltered.contains("matched --filter"),
        "a listing with no filter must not report one, got {unfiltered:?}"
    );
}

#[test]
fn list_omits_directory_entries_but_says_how_many_it_dropped() {
    // A directory record carries no audio and gets an empty basename, so it is pure noise in a
    // voice listing. Dropping it silently would still hide structure from anyone auditing path
    // prefixes, so the count is always reported and `--directories` brings the records back.
    let temp = TempDir::new().unwrap();
    let archive = temp.path().join("voices.zip");
    make_archive_with_directories(
        &archive,
        &["NPC"],
        &[("NPC/One.ogg", b"one"), ("NPC/Two.ogg", b"two")],
    );

    let value = list_json(&archive, &[]);

    assert_eq!(value["entry_count"], 3);
    assert_eq!(value["directory_count"], 1);
    assert_eq!(value["matched_count"], 2);
    assert_eq!(value["listed_count"], 2);
    assert!(
        value["entries"]
            .as_array()
            .unwrap()
            .iter()
            .all(|entry| entry["basename"] != ""),
        "a dropped directory must not leave an empty basename behind, got {value}"
    );

    let with_directories = list_json(&archive, &["--directories"]);

    assert_eq!(with_directories["directory_count"], 1);
    assert_eq!(with_directories["matched_count"], 3);
    assert!(with_directories["entries"]
        .as_array()
        .unwrap()
        .iter()
        .any(|entry| entry["is_directory"] == true));
}

#[test]
fn the_directory_count_names_records_the_filter_kept_not_ones_it_removed() {
    // Counted over the whole archive, the count made the header promise that `--directories` would
    // bring back a `NPC/` record that `--filter DIA_` rejects too -- passing it produced identical
    // output. Advice for an action that changes nothing is worse than no advice, because the reader
    // spends the round trip to find out.
    let temp = TempDir::new().unwrap();
    let archive = temp.path().join("voices.zip");
    make_archive_with_directories(
        &archive,
        &["NPC"],
        &[("NPC/DIA_A.ogg", b"one"), ("NPC/AMB_B.ogg", b"two")],
    );

    let value = list_json(&archive, &["--filter", "DIA_"]);

    assert_eq!(value["entry_count"], 3);
    assert_eq!(value["directory_count"], 0);
    assert_eq!(value["matched_count"], 1);

    let header = list_stdout(&archive, &["--filter", "DIA_"]);
    assert!(
        !header.contains("--directories"),
        "the header must not offer a flag the filter would undo, got {header:?}"
    );
    assert_eq!(
        list_stdout(&archive, &["--filter", "DIA_", "--directories"]),
        header,
        "the advice was empty: taking it changes nothing"
    );
}

#[test]
fn the_header_counts_one_omitted_directory_record_in_the_singular() {
    // "1 directories omitted" shipped in the patch's own fixture, and no test read the header.
    let temp = TempDir::new().unwrap();
    let one = temp.path().join("one.zip");
    make_archive_with_directories(&one, &["NPC"], &[("NPC/One.ogg", b"one")]);
    let two = temp.path().join("two.zip");
    make_archive_with_directories(&two, &["NPC", "AMB"], &[("NPC/One.ogg", b"one")]);

    let single = list_stdout(&one, &[]);
    assert!(
        single.contains("1 directory record omitted"),
        "one record must not be reported in the plural, got {single:?}"
    );

    let plural = list_stdout(&two, &[]);
    assert!(
        plural.contains("2 directory records omitted"),
        "two records must be reported in the plural, got {plural:?}"
    );
}

#[test]
fn a_listing_that_dropped_directory_records_does_not_call_itself_complete() {
    // `truncated` answers "did `--max` stop this", so a default run reported `truncated: false`
    // while holding fewer entries than the archive has -- and the comment beside it invites a
    // consumer to branch on exactly that. One boolean cannot answer two questions, so `complete`
    // answers the other one: is this array the archive?
    let temp = TempDir::new().unwrap();
    let archive = temp.path().join("voices.zip");
    make_archive_with_directories(&archive, &["NPC"], &[("NPC/One.ogg", b"one")]);

    let value = list_json(&archive, &[]);

    assert_eq!(value["entry_count"], 2);
    assert_eq!(value["listed_count"], 1);
    assert_eq!(value["truncated"], false);
    assert_eq!(value["complete"], false);

    let with_directories = list_json(&archive, &["--directories"]);

    assert_eq!(with_directories["listed_count"], 2);
    assert_eq!(with_directories["truncated"], false);
    assert_eq!(with_directories["complete"], true);
}

#[test]
fn list_max_and_filter_compose_so_the_filter_runs_first() {
    // Capping before filtering would make `matched_count` a count of whatever happened to land in
    // the first page. The non-matching entries come first precisely so that the wrong order would
    // report zero matches instead of ten.
    let temp = TempDir::new().unwrap();
    let archive = temp.path().join("voices.zip");
    let mut names = numbered_entry_names("NPC/Ambient/AMB_", 10);
    names.extend(numbered_entry_names("NPC/Quest/DIA_", 10));
    make_archive(&archive, &stored_entries(&names, b"audio"));

    let value = list_json(&archive, &["--filter", "dia_", "--max", "3"]);

    assert_eq!(value["entry_count"], 20);
    assert_eq!(value["matched_count"], 10);
    assert_eq!(value["listed_count"], 3);
    assert_eq!(value["truncated"], true);
    assert!(value["entries"]
        .as_array()
        .unwrap()
        .iter()
        .all(|entry| entry["path"].as_str().unwrap().contains("DIA_")));
}

#[test]
fn the_agent_and_the_shell_user_read_the_same_help_for_every_flag_voice_list_declares() {
    // `crates/gore-mcp/src/spec/groups/files.rs` opens by promising every help string is copied
    // verbatim from the clap doc comment, and `mcp_spec_sync` compares flag names, positional
    // counts and positional order -- never the prose. So `--max` carried a sentence about its own
    // counting behaviour that only an agent could read, and it would have stayed true by luck.
    // `archive` is excluded on purpose: one shared ArgSpec covers six subcommands whose clap
    // wording differs, so it cannot be verbatim for all of them.
    let assert = Command::cargo_bin("gore")
        .unwrap()
        .args(["voice", "list", "--help"])
        .assert()
        .success();
    let help = collapsed(&String::from_utf8(assert.get_output().stdout.clone()).unwrap());

    let list = gore_mcp::spec::group("gore_voice")
        .expect("the table exposes gore_voice")
        .command("list")
        .expect("the table exposes voice list");
    for name in ["filter", "max", "directories"] {
        let arg = list
            .arg(name)
            .unwrap_or_else(|| panic!("the table declares `{name}` for voice list"));
        // clap_derive drops one trailing period from every short help before printing it
        // (`remove_period` in its `doc_comments.rs`), so that character is the one difference a
        // verbatim copy is still allowed to have.
        let expected = collapsed(arg.help);
        let expected = expected.strip_suffix('.').unwrap_or(&expected);
        assert!(
            help.contains(expected),
            "the MCP table's help for `--{name}` is not the help clap prints\n  table: \
             {expected:?}\n  clap:  {help:?}"
        );
    }
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

/// A voice actor handed `replace` the WAV her recording tool wrote and got "invalid Ogg capture
/// pattern at byte 0": neither the format she supplied nor the one required, and no way forward.
/// She called it the single blocker of her session. The refusal must name WAV and hand over the
/// conversion — and still publish nothing.
#[test]
fn a_wav_payload_is_refused_by_name_with_a_conversion_and_publishes_nothing() {
    let temp = TempDir::new().unwrap();
    let input = temp.path().join("input.zip");
    let wav = temp.path().join("recording.wav");
    let output = temp.path().join("output.zip");
    let original = vorbis_ogg(22_050);
    make_archive(&input, &[("NPC/Old.ogg", &original)]);

    let mut header = Vec::from(*b"RIFF");
    header.extend_from_slice(&36u32.to_le_bytes());
    header.extend_from_slice(b"WAVEfmt ");
    std::fs::write(&wav, &header).unwrap();

    Command::cargo_bin("gore")
        .unwrap()
        .args([
            "voice",
            "replace",
            "--archive",
            input.to_str().unwrap(),
            "--path",
            "NPC/Old.ogg",
            "--ogg",
            wav.to_str().unwrap(),
            "--out",
            output.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(contains("a WAV file (RIFF/WAVE), not an Ogg stream"))
        .stderr(contains(
            "ffmpeg -i line.wav -c:a libvorbis -ar 48000 -ac 1 -q:a 5 line.ogg",
        ))
        .stderr(contains("capture pattern").not());
    assert!(!output.exists());
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
        // A payload no signature names still has to say what was wanted and how to get there.
        .stderr(contains("not an Ogg stream"))
        .stderr(contains("-c:a libvorbis"));
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
        .stderr(contains("not an Ogg stream"));
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
