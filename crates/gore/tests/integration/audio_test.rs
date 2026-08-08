//! `gore audio list` against real `.bank` files, through the built binary.
//!
//! Only this tier can prove the thing that broke: how much a caller actually receives, and whether
//! `replace` and `list` describe the same bank. The banks the game ships are 260 MB and are never
//! vendored here, so every bank below is built by `gore_fmod`'s `test-fixtures` builder — a genuine
//! encrypted RIFF/`FEV ` wrapper the same reader walks, whose codec is PCM16 rather than the
//! shipped Vorbis, so a `codec` field can only be right by being read.

use std::path::Path;

use assert_cmd::Command;
use gore_fmod::test_fixture::{numbered_pcm16_samples, pristine_bank_pcm16};
use gore_fmod::{wav_pcm16, Pcm16Sample, GOTHIC_STUDIO_KEY};
use predicates::str::contains;
use tempfile::TempDir;

/// Write a pristine bank of `count` samples named `{prefix}00`, `{prefix}01`, … .
fn numbered_bank(path: &Path, prefix: &str, count: usize) {
    let samples = numbered_pcm16_samples(prefix, count, 44_100);
    named_bank(path, &samples);
}

fn named_bank(path: &Path, samples: &[Pcm16Sample]) {
    let bank = pristine_bank_pcm16(samples, GOTHIC_STUDIO_KEY).unwrap();
    std::fs::write(path, bank).unwrap();
}

fn samples_named(names: &[&str]) -> Vec<Pcm16Sample> {
    names
        .iter()
        .map(|name| Pcm16Sample {
            name: (*name).to_owned(),
            freq: 44_100,
            channels: 1,
            pcm: vec![0i16; 1],
        })
        .collect()
}

fn list_json(bank: &Path, extra: &[&str]) -> serde_json::Value {
    let mut args = vec!["audio", "list", "--bank", bank.to_str().unwrap(), "--json"];
    args.extend_from_slice(extra);
    let output = Command::cargo_bin("gore")
        .unwrap()
        .args(args)
        .output()
        .unwrap();
    assert!(output.status.success(), "{:?}", output);
    serde_json::from_slice(&output.stdout).unwrap()
}

fn list_stdout(bank: &Path, extra: &[&str]) -> String {
    let mut args = vec!["audio", "list", "--bank", bank.to_str().unwrap()];
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

#[test]
fn audio_list_says_how_many_samples_matched_when_it_stops_at_max() {
    // `SFX.bank` printed 458,589 bytes over 7,219 lines and nothing bounded the loop. Through the
    // MCP server that was cut at 256 KiB mid-line inside sample #4122, so the 3,095 samples behind
    // it were not merely unshown — they never arrived, and a caller who filtered what did arrive
    // was told a sound does not exist. A listing that stops has to carry its own counts.
    let temp = TempDir::new().unwrap();
    let bank = temp.path().join("SFX.bank");
    numbered_bank(&bank, "SFX_UI_Click_", 12);

    let value = list_json(&bank, &["--max", "5"]);

    assert_eq!(value["sample_count"], 12);
    assert_eq!(value["matched_count"], 12);
    assert_eq!(value["listed_count"], 5);
    assert_eq!(value["truncated"], true);
    assert_eq!(value["samples"].as_array().unwrap().len(), 5);
    let notice = value["truncation_notice"].as_str().unwrap();
    assert!(
        notice.contains("12 samples matched") && notice.contains("first 5"),
        "the notice must name both numbers, got {notice:?}"
    );
}

#[test]
fn the_default_max_keeps_a_real_bank_listing_inside_one_mcp_result() {
    // The reported defect, restated as a test: a caller who passes no flags at all must still get a
    // bounded answer. Raise `default_value_t` past what a result can hold and this fails; leave the
    // bound out of the default path and the bug is back for exactly the caller who hit it first.
    let temp = TempDir::new().unwrap();
    let bank = temp.path().join("SFX.bank");
    numbered_bank(&bank, "SFX_CREA_Orcdog_Grunt_", 120);

    let value = list_json(&bank, &[]);

    assert_eq!(value["sample_count"], 120);
    assert_eq!(value["listed_count"], 100);
    assert_eq!(value["truncated"], true);
    assert_eq!(value["complete"], false);
}

#[test]
fn audio_list_max_zero_lists_nothing_and_reports_only_the_counts() {
    // Many tools read 0 as "unlimited", so an agent asking for everything this way would get the
    // opposite. Which one it is now stands in the flag's own help, and this is the behaviour that
    // help describes: the counts still answer "how many match?" for the price of one call.
    let temp = TempDir::new().unwrap();
    let bank = temp.path().join("SFX.bank");
    numbered_bank(&bank, "SFX_UI_Click_", 3);

    let value = list_json(&bank, &["--max", "0"]);

    assert_eq!(value["sample_count"], 3);
    assert_eq!(value["matched_count"], 3);
    assert_eq!(value["listed_count"], 0);
    assert_eq!(value["truncated"], true);
    assert_eq!(value["complete"], false);
    assert!(value["samples"].as_array().unwrap().is_empty());
}

#[test]
fn audio_list_prints_a_truncation_marker_the_mcp_guide_teaches_people_to_recognise() {
    // docs/guide/mcp.md tells every reader that output ending in `… [truncated]` means "narrow the
    // query with the command's own filter". For `audio list` that row was a dead end: no marker and
    // no filter, on the one command in the family whose output actually overflows. Both halves of
    // the promise are asserted here.
    let temp = TempDir::new().unwrap();
    let bank = temp.path().join("SFX.bank");
    numbered_bank(&bank, "SFX_UI_Click_", 12);

    Command::cargo_bin("gore")
        .unwrap()
        .args([
            "audio",
            "list",
            "--bank",
            bank.to_str().unwrap(),
            "--max",
            "5",
        ])
        .assert()
        .success()
        .stdout(contains("… [truncated:"))
        .stdout(contains("--filter"));
}

#[test]
fn the_truncation_notice_never_hands_back_a_max_that_would_be_cut_off_in_transit() {
    // The remedy a notice names has to be one that works. `--max 7218` on `SFX.bank` reproduces the
    // very cut the bound exists to prevent — a 458 KB table against a 256 KiB result budget — and
    // an agent follows advice literally. So the notice names the flag that narrows, and never a
    // number that lists them all.
    let temp = TempDir::new().unwrap();
    let bank = temp.path().join("SFX.bank");
    numbered_bank(&bank, "SFX_UI_Click_", 12);

    let value = list_json(&bank, &["--max", "5"]);
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
fn audio_list_leaves_a_complete_result_unlabelled() {
    // A truncation notice on a listing that hid nothing is its own kind of lie: it would send a
    // caller looking for samples that are already in front of them.
    let temp = TempDir::new().unwrap();
    let bank = temp.path().join("Music.bank");
    numbered_bank(&bank, "MUS_Theme_", 3);

    let value = list_json(&bank, &["--max", "100"]);

    assert_eq!(value["sample_count"], 3);
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
fn audio_list_filter_matches_regardless_of_case_and_the_counts_follow_it() {
    // FMOD sample names carry their own casing (`SFX_CREA_Orcdog_Grunt_L1_05`), and nobody retypes
    // one exactly. A case-sensitive filter would answer "no such sample" when the truth is "wrong
    // case" — a false negative dressed as a fact, and the same fold `voice list --filter` uses.
    let temp = TempDir::new().unwrap();
    let bank = temp.path().join("SFX.bank");
    named_bank(
        &bank,
        &samples_named(&[
            "SFX_AMB_Cave_Drip",
            "SFX_CREA_Orcdog_Grunt_L1_05",
            "SFX_UI_Click",
        ]),
    );

    let value = list_json(&bank, &["--filter", "orcdog"]);

    assert_eq!(value["sample_count"], 3);
    assert_eq!(value["matched_count"], 1);
    assert_eq!(value["listed_count"], 1);
    assert_eq!(value["samples"][0]["name"], "SFX_CREA_Orcdog_Grunt_L1_05");
    // `index` stays the sample's own FSB5 index. Renumbering it to the position in a filtered page
    // would make `extract`'s `{index}_{name}.wav` output name a different sample than the one the
    // listing showed.
    assert_eq!(value["samples"][0]["index"], 1);
}

#[test]
fn a_filter_that_matches_nothing_does_not_read_like_a_bank_with_no_samples() {
    // Sharper here than for voice: a bank that carries no samples is a documented failure of its
    // own (`Master.bank` and the four placeholders), so a header with no rows under it is genuinely
    // ambiguous between "nothing matched" and "wrong bank". Only the count tells them apart.
    let temp = TempDir::new().unwrap();
    let bank = temp.path().join("SFX.bank");
    numbered_bank(&bank, "SFX_UI_Click_", 3);

    let empty = list_stdout(&bank, &["--filter", "zzz"]);
    assert!(
        empty.contains("3 samples, codec Pcm16, 0 matched --filter"),
        "a filter that kept nothing must say so, got {empty:?}"
    );

    let matched = list_stdout(&bank, &["--filter", "click_0"]);
    assert!(
        matched.contains("3 samples, codec Pcm16, 3 matched --filter"),
        "the header must count what the filter kept, got {matched:?}"
    );

    let unfiltered = list_stdout(&bank, &[]);
    assert!(
        !unfiltered.contains("matched --filter"),
        "a listing with no filter must not report one, got {unfiltered:?}"
    );
}

#[test]
fn audio_list_max_and_filter_compose_so_the_filter_runs_first() {
    // Capping before filtering would make `matched_count` a count of whatever happened to land in
    // the first page. The non-matching samples come first precisely so that the wrong order would
    // report zero matches instead of ten.
    let temp = TempDir::new().unwrap();
    let bank = temp.path().join("SFX.bank");
    let mut samples = numbered_pcm16_samples("SFX_AMB_Cave_", 10, 44_100);
    samples.extend(numbered_pcm16_samples("SFX_CREA_Orcdog_", 10, 44_100));
    named_bank(&bank, &samples);

    let value = list_json(&bank, &["--filter", "crea_", "--max", "3"]);

    assert_eq!(value["sample_count"], 20);
    assert_eq!(value["matched_count"], 10);
    assert_eq!(value["listed_count"], 3);
    assert_eq!(value["truncated"], true);
    assert!(value["samples"]
        .as_array()
        .unwrap()
        .iter()
        .all(|sample| sample["name"].as_str().unwrap().contains("CREA")));
}

#[test]
fn the_json_document_names_the_codec_the_human_header_prints() {
    // The bank-level fact a voice archive has no analogue for, and the one every other `audio`
    // subcommand's behaviour hangs off: `extract` only decodes Vorbis. A JSON mode that dropped it
    // would be strictly less informative than the table it replaces, so the modes are compared to
    // each other rather than to a literal.
    let temp = TempDir::new().unwrap();
    let bank = temp.path().join("SFX.bank");
    numbered_bank(&bank, "SFX_UI_Click_", 3);

    let value = list_json(&bank, &[]);
    let human = list_stdout(&bank, &[]);

    let codec = value["codec"].as_str().unwrap();
    assert_eq!(codec, "Pcm16", "the fixture bank is PCM16, not the shipped Vorbis");
    assert!(
        human.contains(&format!("codec {codec}")),
        "both modes must name the same codec, got {human:?}"
    );
    assert_eq!(value["bank"], bank.display().to_string());
}

#[test]
fn the_agent_and_the_shell_user_read_the_same_help_for_every_flag_audio_list_declares() {
    // `crates/gore-mcp/src/spec/groups/files.rs` opens by promising every help string is copied
    // verbatim from the clap doc comment, and `mcp_spec_sync` compares flag names, positional counts
    // and positional order — never the prose. So `--max` carries a sentence about its own counting
    // behaviour that only an agent could read, and it would have stayed true by luck. `bank` and
    // `key` are excluded on purpose: one shared `AUDIO_KEY` ArgSpec covers four subcommands whose
    // clap wording differs, so it cannot be verbatim for all of them.
    let assert = Command::cargo_bin("gore")
        .unwrap()
        .args(["audio", "list", "--help"])
        .assert()
        .success();
    let help = collapsed(&String::from_utf8(assert.get_output().stdout.clone()).unwrap());

    let list = gore_mcp::spec::group("gore_audio")
        .expect("the table exposes gore_audio")
        .command("list")
        .expect("the table exposes audio list");
    for name in ["filter", "max"] {
        let arg = list
            .arg(name)
            .unwrap_or_else(|| panic!("the table declares `{name}` for audio list"));
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
fn a_replaced_sample_is_listed_as_the_replacement_and_not_as_the_audio_it_replaced() {
    // The whole reason a deploy could not be checked. `replace` appends a second FSB5 and repoints
    // the waveform at it — the game follows that repoint — but sub-bank 0 keeps the original audio
    // byte for byte by design, and every reader here went there. So a user who replaced a 0.30 s
    // click was shown the original's 32.70 s by the very command that had just written the file,
    // and "the toolkit cannot read back its own output" was indistinguishable from "the write did
    // nothing". Only this tier proves it, because `replace` and `list` are two separate runs of the
    // binary over one file on disk.
    let temp = TempDir::new().unwrap();
    let bank = temp.path().join("SFX.bank");
    let replaced = temp.path().join("SFX_modded.bank");
    named_bank(
        &bank,
        &[
            Pcm16Sample {
                name: "SFX_UI_Action_Button_Click_01".into(),
                freq: 22_050,
                channels: 1,
                pcm: vec![0i16; 2_205], // 0.10 s
            },
            Pcm16Sample {
                name: "SFX_UI_Action_Button_Hover_01".into(),
                freq: 22_050,
                channels: 1,
                pcm: vec![0i16; 4_410],
            },
        ],
    );
    // 0.75 s at a different rate, so neither the duration nor the rate can match by accident.
    let wav = temp.path().join("click.wav");
    std::fs::write(&wav, wav_pcm16(44_100, 1, &vec![0i16; 33_075])).unwrap();
    let map = temp.path().join("map.json");
    std::fs::write(
        &map,
        serde_json::json!({ "SFX_UI_Action_Button_Click_01": "click.wav" }).to_string(),
    )
    .unwrap();

    Command::cargo_bin("gore")
        .unwrap()
        .args([
            "audio",
            "replace",
            "--map",
            map.to_str().unwrap(),
            "--bank",
            bank.to_str().unwrap(),
            "--out",
            replaced.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(contains("1 sample(s) replaced"));

    let value = list_json(&replaced, &[]);
    let click = &value["samples"][0];
    assert_eq!(click["name"], "SFX_UI_Action_Button_Click_01");
    assert_eq!(
        click["replaced"], true,
        "a listing has to say which samples a replacement reached, got {value}"
    );
    assert_eq!(click["freq"], 44_100, "the replacement's rate, not the 22050 Hz it replaced");
    assert_eq!(
        click["seconds"].as_f64().unwrap(),
        0.75,
        "the replacement's length, not the 0.10 s it replaced"
    );

    let hover = &value["samples"][1];
    assert_eq!(hover["replaced"], false, "the untouched sample must not read as replaced");
    assert_eq!(hover["freq"], 22_050);

    // And the human table, because that is what a person actually looks at after a deploy.
    let human = list_stdout(&replaced, &["--filter", "click"]);
    assert!(
        human.contains("44100Hz") && human.contains("[replaced"),
        "the table must mark the replacement, got {human:?}"
    );
}

#[test]
fn every_sample_of_a_pcm16_bank_extracts_rather_than_being_skipped() {
    // PCM16 is what `audio replace` appends, so this is the codec of every replacement — and
    // `extract` used to reject it, skipping precisely the sample a user had just written and
    // reporting success with no file. This bank is built the same way a replaced one ends up.
    //
    // It replaces a test that asserted the opposite: that a whole PCM16 bank was skipped for one
    // shared reason, reported once rather than once per sample. That de-duplication is still in
    // `cmd/audio.rs` and still worth having — 7,218 identical stderr lines was the bug that put it
    // there — but it has no end-to-end fixture any more, because the only bank these tests can
    // build is now one that extracts. Reinstating it needs a builder for a codec `extract_wav`
    // genuinely cannot read.
    let temp = TempDir::new().unwrap();
    let bank = temp.path().join("SFX.bank");
    let extracted = temp.path().join("wavs");
    numbered_bank(&bank, "SFX_UI_Click_", 12);

    let output = Command::cargo_bin("gore")
        .unwrap()
        .args([
            "audio",
            "extract",
            "--bank",
            bank.to_str().unwrap(),
            "--out",
            extracted.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(output.status.success(), "{:?}", output);

    let stderr = String::from_utf8(output.stderr.clone()).unwrap();
    assert!(stderr.trim().is_empty(), "nothing was skipped, so nothing is reported: {stderr:?}");

    let stdout = String::from_utf8(output.stdout.clone()).unwrap();
    assert!(stdout.contains("extracted 12 wav file(s)"), "{stdout:?}");
    assert!(stdout.contains("(0 skipped)"), "{stdout:?}");

    let written = std::fs::read_dir(&extracted).unwrap().count();
    assert_eq!(written, 12, "a count in the summary is not a file on disk");
}

#[test]
fn a_collision_anywhere_in_the_selection_writes_nothing_at_all() {
    // The collision refusal used to live inside the write loop, so a clash on a later sample left
    // the earlier ones on disk — and the obvious retry then failed on one of those instead of on
    // the file the caller actually had to deal with. Names come from the bank, not the arguments,
    // so the caller cannot compute them in advance; settling every destination before the first
    // write is the only way this command can fail without leaving a mess.
    let temp = TempDir::new().unwrap();
    let bank = temp.path().join("SFX.bank");
    let extracted = temp.path().join("wavs");
    numbered_bank(&bank, "SFX_UI_Click_", 12);

    // Occupy a destination that is NOT the first one written, which is the case the old code got
    // wrong. The index prefix is the sample's own, so this is the name sample 7 would claim.
    std::fs::create_dir_all(&extracted).unwrap();
    let occupied = extracted.join("7_SFX_UI_Click_07.wav");
    std::fs::write(&occupied, b"mine, edited, not to be replaced").unwrap();

    let output = Command::cargo_bin("gore")
        .unwrap()
        .args([
            "audio",
            "extract",
            "--bank",
            bank.to_str().unwrap(),
            "--out",
            extracted.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(!output.status.success(), "a collision must fail the command: {output:?}");

    let stderr = String::from_utf8(output.stderr.clone()).unwrap();
    assert!(
        stderr.contains("7_SFX_UI_Click_07.wav"),
        "the refusal names the file in the way: {stderr:?}"
    );
    assert!(
        stderr.contains("Nothing was written"),
        "the refusal says the run left no output: {stderr:?}"
    );

    assert_eq!(
        std::fs::read(&occupied).unwrap(),
        b"mine, edited, not to be replaced",
        "the file that caused the refusal is untouched"
    );
    let present: Vec<String> = std::fs::read_dir(&extracted)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(
        present,
        vec!["7_SFX_UI_Click_07.wav".to_string()],
        "no partial output beside it: {present:?}"
    );
}
