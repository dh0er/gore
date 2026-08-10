//! `gore audio` — read and replace audio in the game's encrypted FMOD sound banks
//! (pure Rust, no FMOD). The banks are loose at `…/G1R/Content/FMOD/Desktop/*.bank`.
//!
//! - `banks`:   list the banks the configured install carries, with a path for `--bank`
//! - `list`:    decrypt + list a bank's samples (name, codec, rate, channels, duration)
//! - `replace`: swap samples with new audio (WAV) via PCM injection; in-place with a
//!   `*.gore-bak` backup, or to `--out`
//! - `restore`: restore a bank from its `*.gore-bak`
//!
//! Replacement re-encodes the new audio as PCM16 in an appended FSB5 sub-bank and repoints
//! the bank's waveform references to it — arbitrary size, no whole-bank re-encode.

use anyhow::{bail, Context, Result};
use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

fn key_bytes(key: Option<String>) -> Vec<u8> {
    match key {
        Some(k) if !k.is_empty() => k.into_bytes(),
        _ => gore_fmod::GOTHIC_STUDIO_KEY.to_vec(),
    }
}

/// Read a bank's PRISTINE bytes. The live bank is the source of truth when it isn't injected yet
/// (a single FSB5): that covers the first replace AND the case where a `restore` or a Steam
/// update refreshed the live bank, so we never rebuild from a stale `*.gore-bak` and downgrade the
/// updated audio. Only when the live bank is already injected (>1 FSB5) — i.e. a repeated in-place
/// replace — do we fall back to the backup, which holds the true pristine to avoid compounding.
fn read_pristine_bank(bank: &Path) -> Result<Vec<u8>> {
    let live = std::fs::read(bank).with_context(|| format!("reading '{}'", bank.display()))?;
    if !gore_fmod::is_pristine_bank(&live) {
        // The live bank is injected (or unparseable) — its true pristine is the backup, if any.
        let bak = backup_path(bank);
        if bak.exists() {
            return std::fs::read(&bak).with_context(|| format!("reading '{}'", bak.display()));
        }
    }
    Ok(live)
}

/// Write `bytes` to `path` via a temp file + rename so a failed write never truncates the
/// original game file in place.
fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    let tmp = path.with_extension("gore-tmp");
    std::fs::write(&tmp, bytes).with_context(|| format!("writing '{}'", tmp.display()))?;
    // `std::fs::rename` REPLACES an existing destination on Windows too — Rust implements it via
    // `MoveFileExW(.., MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH)`, not the bare
    // `MoveFile`/`rename()` that fails when the target exists. So overwriting an existing .bank in
    // place works; do NOT switch to remove-then-rename (it would add a non-atomic crash window).
    std::fs::rename(&tmp, path).with_context(|| format!("renaming into '{}'", path.display()))?;
    Ok(())
}

/// One `.bank` in the install's FMOD directory, as `banks` describes it.
struct BankRow {
    /// The whole path, because that is the string every other subcommand's `--bank` wants and the
    /// thing this command exists to hand over. It is built from `resolve_game_paths(..)`, which is
    /// also what a bundle resolves a bare bank name against, so a bank listed here and a bank a
    /// bundle names can never turn out to be two different files.
    path: PathBuf,
    /// What reading the file's header produced. An `Err` is a row rather than an aborted listing:
    /// one unreadable file among ten must not hide the nine that are fine, and a directory
    /// described as nine files would be described wrongly.
    summary: Result<gore_fmod::BankSummary, String>,
}

impl BankRow {
    fn name(&self) -> std::borrow::Cow<'_, str> {
        self.path.file_name().unwrap_or_default().to_string_lossy()
    }
}

/// List the banks the install carries. There is no `--max` here on purpose: the directory holds
/// ten files, so a bound would only be able to hide something.
pub fn banks(game: Option<PathBuf>, json: bool, key: Option<String>) -> Result<()> {
    let root = gore_loc::config::game_root(game)?;
    // `resolve_game_paths`, not a hand-built join: this is the same directory a bundle resolves a
    // bare bank name against, and two spellings of one path is how a listing starts describing a
    // file nothing else will open.
    let dir = gore_mod::resolve_game_paths(&root).fmod_desktop;
    // Not `Path::is_dir()`, which answers `false` when it could not tell as readily as when the
    // folder is not there. The message below sends the reader to re-point `--game` or verify the
    // game files, and both are the wrong move for a directory sitting right there behind an ACL.
    match std::fs::metadata(&dir) {
        Ok(metadata) if metadata.is_dir() => {}
        Err(error) if error.kind() != std::io::ErrorKind::NotFound => {
            bail!(
                "the FMOD bank directory at '{}' could not be read: {error}. That path is fixed \
                 inside a Gothic 1 Remake install, so this is a permissions or I/O problem rather \
                 than a wrong --game: run the same command from a shell that can read the install.",
                dir.display()
            );
        }
        // There, and not a directory. Folded into the sentence below it read as "no FMOD bank
        // directory", which sends the reader to re-point `--game` or verify the game files —
        // neither of which can put a directory where something else is already sitting, and the
        // thing in the way never gets named.
        Ok(_) => bail!(
            "'{}' is not a directory. That path is fixed inside a Gothic 1 Remake install and the \
             game's banks live in it, so nothing can create it while something else occupies that \
             name: remove or rename what is there.",
            dir.display()
        ),
        // Nothing the path RESOLVES to. A dangling link resolves to nothing and reports
        // `NotFound`, while the name is taken — so this fell into the sentence below, which sends
        // the reader to re-point `--game` or verify game files, and neither can create a
        // directory while a link holds its name. Asked without following the link, which is the
        // only way to tell "there is nothing here" from "there is something unusable here".
        _ if std::fs::symlink_metadata(&dir).is_ok() => bail!(
            "'{}' is a link that leads nowhere. The game's banks live at that fixed path and \
             nothing can create it while the link holds the name: remove or rename it.",
            dir.display()
        ),
        _ => bail!(
            "no FMOD bank directory at '{}'. That path is fixed inside a Gothic 1 Remake install, \
             so either --game (or the configured game path) points at something that is not one, \
             or this install is incomplete — verify the game files and try again.",
            dir.display()
        ),
    }

    let rows = bank_rows(&dir, &key_bytes(key))?;
    let occupied = occupied_bank_names(&dir);
    if rows.is_empty() {
        // Naming them here rather than only in the table, because this branch never reaches the
        // table: "verify the game files" is the wrong move for a directory called `Music.bank`,
        // and it is the only sentence somebody in this state would otherwise get.
        let held = match occupied.is_empty() {
            true => String::new(),
            false => format!(
                " {} entr(y/ies) there are named like a bank and are not files ({}); remove or \
                 rename them.",
                occupied.len(),
                occupied.join(", ")
            ),
        };
        bail!(
            "'{}' holds no .bank files. A Gothic 1 Remake install keeps ten there, so this is an \
             install to verify rather than a listing to read.{held}",
            dir.display()
        );
    }

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&banks_document(&dir, &rows, &occupied))?
        );
    } else {
        print!("{}", banks_table(&dir, &rows, &occupied));
    }
    Ok(())
}

/// Read and summarise every `.bank` in `dir`, in file-name order.
///
/// The ordering is imposed rather than inherited: `read_dir` promises nothing about it, and two
/// runs that listed the same ten files in two orders would make a diff of the output unreadable.
fn bank_rows(dir: &Path, key: &[u8]) -> Result<Vec<BankRow>> {
    // Every entry or none. A bank whose *contents* cannot be read is kept as an error row on
    // purpose — one damaged file must not cost the other nine — but an entry the directory itself
    // could not yield is different: dropping it would print a bank count, a sample total and a
    // listing that all describe a subset while claiming to describe the directory.
    let mut paths: Vec<PathBuf> = std::fs::read_dir(dir)
        .with_context(|| format!("reading the FMOD bank directory '{}'", dir.display()))?
        .map(|entry| {
            entry
                .map(|entry| entry.path())
                .with_context(|| format!("reading an entry of '{}'", dir.display()))
        })
        .collect::<Result<Vec<PathBuf>>>()?
        .into_iter()
        // The `.bank` extension is the whole filter, which also excludes this toolkit's own
        // `*.bank.gore-bak` backups and `*.gore-tmp` half-writes: those are not banks the game
        // loads, and offering one as a `--bank` would send a replacement into a file the game
        // never reads.
        .filter(|path| {
            path.extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("bank"))
        })
        // A directory named `Music.bank` is not a bank the game can load, and counting it as one
        // put it in `bank_count`, in the "N bank(s)" header, and past the "holds no .bank files"
        // check — a listing claiming to describe the directory, describing something that is not
        // in it. Reported separately by `occupied_bank_names` rather than dropped, because going
        // silent about it is the same failure with the sign flipped.
        //
        // Only when the metadata says so. An entry whose type cannot be read stays a row and
        // becomes an unreadable one, which is what it is — dropping it would print totals that
        // describe a subset.
        .filter(|path| !std::fs::metadata(path).is_ok_and(|meta| !meta.is_file()))
        .collect();
    paths.sort();

    Ok(paths
        .into_iter()
        .map(|path| {
            // Reading is the whole cost here, and this listing exists so as not to pay it: the
            // summary needs the RIFF wrapper and a few dozen bytes at each FSB5 offset, which is
            // about 20 MB across the ten shipped banks instead of a pass over ~520 MB. Reading
            // each file whole to hand it to `bank_summary` paid exactly the price the summary was
            // written to avoid, and allocated 260 MB for `SFX.bank` on the way.
            let summary = gore_fmod::bank_summary_at(&path, key);
            BankRow { path, summary }
        })
        .collect())
}

/// Entries named like a bank that are not files, in directory order.
///
/// The listing above leaves them out because the game cannot load a directory, and this is what
/// keeps that from being silent: somebody looking for `Music.bank` in a listing that does not
/// mention it goes back to searching the filesystem, which is where they started.
fn occupied_bank_names(dir: &Path) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut names: Vec<String> = entries
        .filter_map(std::result::Result::ok)
        .filter(|entry| {
            let path = entry.path();
            path.extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("bank"))
                && std::fs::metadata(&path).is_ok_and(|meta| !meta.is_file())
        })
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

/// The counts the header states, so the table and the JSON document cannot claim different totals.
///
/// The third number is the point of the tuple. A bank that could not be summarized — unreadable,
/// corrupt, or decrypted with the wrong key — contributes nothing to the first two, and `SFX.bank`
/// alone carries most of the samples in the install. Dropping it silently and then printing
/// "samples in total" states a number that is not the total of anything.
fn bank_totals(rows: &[BankRow]) -> (usize, usize, usize) {
    let mut with_samples = 0;
    let mut samples = 0;
    let mut unreadable = 0;
    for row in rows {
        match &row.summary {
            Ok(gore_fmod::BankSummary::Samples { sample_count, .. }) => {
                with_samples += 1;
                samples += sample_count;
            }
            Ok(gore_fmod::BankSummary::SampleFree) => {}
            Err(_) => unreadable += 1,
        }
    }
    (with_samples, samples, unreadable)
}

fn banks_table(dir: &Path, rows: &[BankRow], occupied: &[String]) -> String {
    use std::fmt::Write as _;

    let (with_samples, sample_total, unreadable) = bank_totals(rows);
    // Said in the header, where the number is, rather than left to be inferred from error rows
    // further down the table.
    let caveat = match unreadable {
        0 => String::new(),
        n => format!(", {n} could not be read so this is a partial count"),
    };
    let mut out = String::new();
    let _ = writeln!(
        out,
        "FMOD banks: {} in {} ({with_samples} carry samples, {sample_total} samples in \
         total{caveat})",
        rows.len(),
        dir.display()
    );
    // The column header names what the third column is FOR. The reported defect was not that the
    // paths were hard to find, it was that nothing said a path was what `--bank` wanted.
    let _ = writeln!(
        out,
        "SAMPLES  CODEC     BANK (pass this whole path as --bank)"
    );
    for row in rows {
        let (samples, codec, note) = match &row.summary {
            Ok(gore_fmod::BankSummary::Samples {
                sub_banks,
                sample_count,
                codec,
            }) => (
                sample_count.to_string(),
                format!("{codec:?}"),
                // Worth a marker for the same reason `list` marks a replaced sample: an injected
                // bank is byte-for-byte ordinary from the outside, and someone who has forgotten a
                // replacement is deployed will read the next surprise as a bug in the tool.
                match *sub_banks > 1 {
                    true => "  [injected — `gore audio restore` puts the shipped bank back]",
                    false => "",
                },
            ),
            // Named as a fact about the file rather than as a failure. These six are intact; the
            // guide's own account of them is that there is simply nothing in them.
            Ok(gore_fmod::BankSummary::SampleFree) => (
                "—".to_string(),
                "—".to_string(),
                "  [no sample data: nothing here to list, extract or replace]",
            ),
            Err(_) => ("—".to_string(), "—".to_string(), ""),
        };
        let _ = writeln!(
            out,
            "{samples:>7}  {codec:<8}  {}{note}",
            row.path.display()
        );
        if let Err(reason) = &row.summary {
            // On its own line because the reason is a sentence, not a cell, and because a reader
            // scanning the SAMPLES column has to be able to tell "carries nothing" from "could not
            // be read at all".
            let _ = writeln!(out, "{:>7}  {:<8}  could not be read: {reason}", "", "");
        }
    }
    // After the rows, because they are not rows: the game cannot load a directory, so counting one
    // as a bank would put it in the header's total. Said all the same — somebody looking for
    // `Music.bank` in a listing that never mentions it goes back to searching the filesystem.
    if !occupied.is_empty() {
        let _ = writeln!(
            out,
            "{} entr(y/ies) are named like a bank and are not files, so they are not counted \
             above: {}. Remove or rename them.",
            occupied.len(),
            occupied.join(", ")
        );
    }
    out
}

/// The same shape `list --json` uses: the path under `bank`, the codec spelled the way `Codec`'s
/// `Debug` spells it, and counts that answer their question without a reader subtracting anything.
fn banks_document(dir: &Path, rows: &[BankRow], occupied: &[String]) -> serde_json::Value {
    let (with_samples, sample_total, unreadable) = bank_totals(rows);
    let banks = rows
        .iter()
        .map(|row| {
            let mut entry = serde_json::json!({
                // `bank` rather than `path`, because it is literally the value of the `--bank` the
                // next call passes; `list --json` names the same thing the same way.
                "bank": row.path.display().to_string(),
                "name": row.name(),
                "carries_samples": false,
                "sample_count": 0,
                "codec": serde_json::Value::Null,
                "sub_banks": 0,
                "injected": false,
            });
            match &row.summary {
                Ok(gore_fmod::BankSummary::Samples {
                    sub_banks,
                    sample_count,
                    codec,
                }) => {
                    entry["carries_samples"] = serde_json::json!(true);
                    entry["sample_count"] = serde_json::json!(sample_count);
                    entry["codec"] = serde_json::json!(format!("{codec:?}"));
                    entry["sub_banks"] = serde_json::json!(sub_banks);
                    // Derived from `sub_banks`, and carried anyway for the reason `list --json`
                    // carries `replaced`: a caller should not have to know that "more than one
                    // FSB5" is what this toolkit means by modded in order to read the answer.
                    entry["injected"] = serde_json::json!(*sub_banks > 1);
                }
                Ok(gore_fmod::BankSummary::SampleFree) => {
                    entry["note"] = serde_json::json!(
                        "carries no sample data: a placeholder or a metadata-only bank, intact but \
                         with nothing in it to list, extract or replace"
                    );
                }
                Err(reason) => {
                    entry["error"] = serde_json::json!(reason);
                    // The defaults above are assertions — carries no samples, none of them, not
                    // injected — and nothing measured any of them for this file. A consumer could
                    // not tell an unknown `SFX.bank` from one successfully inspected and found
                    // empty; `totals_complete` qualifies the aggregate, not the row.
                    for unknown in ["carries_samples", "sample_count", "codec", "sub_banks", "injected"] {
                        entry[unknown] = serde_json::Value::Null;
                    }
                }
            }
            entry
        })
        .collect::<Vec<_>>();
    serde_json::json!({
        "directory": dir.display().to_string(),
        "bank_count": rows.len(),
        "with_samples_count": with_samples,
        "sample_count": sample_total,
        // A caller gating on `sample_count` cannot see the per-bank `error` rows without walking
        // the list, so the aggregate says for itself whether it covers every bank.
        "unreadable_count": unreadable,
        "totals_complete": unreadable == 0,
        "banks": banks,
        // Not banks, and not silence either. The game cannot load a directory, so one named
        // `Music.bank` is out of `bank_count` — but a caller that cannot see it has the same
        // blind spot the count would have had.
        "occupied_names": occupied,
    })
}

/// List a bank's samples under a bound. Decoding reads and decrypts the whole bank either way --
/// `SFX.bank` is 260 MB on disk -- so the narrowing here is a presentation decision only. A listing
/// that stopped silently would let a caller read the first `max` samples as the whole bank and
/// conclude a sound does not exist, so both output modes label the cut.
pub fn list(
    bank: PathBuf,
    filter: Option<String>,
    max: usize,
    json: bool,
    key: Option<String>,
) -> Result<()> {
    let bytes = std::fs::read(&bank).with_context(|| format!("reading '{}'", bank.display()))?;
    // `read_bank`, not `bank_fsb0`: a replacement appends a sub-bank and repoints the waveform at
    // it rather than overwriting sub-bank 0, so a listing built from sub-bank 0 describes the audio
    // a replacement replaced and calls it the bank's current contents.
    let view = gore_fmod::read_bank(&bytes, &key_bytes(key))
        .map_err(|e| anyhow::anyhow!("{e}"))
        .context("decoding bank")?;
    let sample_count = view.samples.len();
    // Filter first, cap second: `matched_count` is only meaningful if the cap never hides a
    // candidate the filter would have kept.
    let needle = filter.as_deref().map(str::to_lowercase);
    let matched = view
        .samples
        .iter()
        .enumerate()
        .filter(|(_, sample)| {
            needle
                .as_deref()
                .is_none_or(|needle| super::contains_case_insensitive(&sample.name, needle))
        })
        .collect::<Vec<_>>();
    let listed = &matched[..matched.len().min(max)];
    let notice =
        (listed.len() < matched.len()).then(|| list_truncation_notice(matched.len(), listed.len()));

    if json {
        let samples = listed
            .iter()
            .map(|(index, sample)| {
                // The same spelling `gore-ffi`'s `audio_list` gives the mod studio, so the CLI and
                // the GUI describe one sample the same way.
                // `replaced` is the answer to the only question a deploy leaves open, and nothing
                // else in the document can stand in for it: a replacement keeps the sample's name
                // and index, so a caller comparing two listings sees a rate and a duration change
                // and cannot tell that from having read the wrong bank.
                serde_json::json!({
                    "index": index,
                    "name": sample.name,
                    "freq": sample.freq,
                    "channels": sample.channels,
                    "seconds": sample_seconds(sample),
                    "replaced": sample.replaced,
                })
            })
            .collect::<Vec<_>>();
        // Two booleans because there are two questions and one answer cannot serve both.
        // `truncated` says whether `--max` stopped the listing, and it is what `truncation_notice`
        // belongs to. "Is this array the whole bank" is a different question -- a filter narrows
        // without truncating -- and `complete` answers it, so neither has to be inferred by
        // comparing counts.
        let mut document = serde_json::json!({
            "bank": bank.display().to_string(),
            "codec": format!("{:?}", view.codec()),
            "sample_count": sample_count,
            "matched_count": matched.len(),
            "listed_count": samples.len(),
            "truncated": notice.is_some(),
            "complete": samples.len() == sample_count,
            "samples": samples,
        });
        if let Some(notice) = &notice {
            document["truncation_notice"] = serde_json::json!(notice);
        }
        println!("{}", serde_json::to_string_pretty(&document)?);
        return Ok(());
    }

    // Without this clause a filter that matched nothing prints a header and no rows, and a bank
    // with no rows is a documented failure of its own (`Master.bank` and the placeholders), so the
    // reader cannot tell "nothing matched" from "wrong bank".
    let narrowed = match filter {
        Some(_) => format!(", {} matched --filter", matched.len()),
        None => String::new(),
    };
    println!("{sample_count} samples, codec {:?}{narrowed}", view.codec());
    for (i, s) in listed {
        // The marker carries the replacement's own codec because that is the fact `extract` turns
        // on, and because a row that differs from the shipped bank in nothing but two numbers is
        // otherwise indistinguishable from a mistyped `--bank`.
        let replaced = match s.replaced {
            true => format!("  [replaced, {:?}]", s.codec),
            false => String::new(),
        };
        println!(
            "#{i:<5} {:6}Hz {}ch {:6.2}s  {}{replaced}",
            s.freq,
            s.channels,
            sample_seconds(s),
            s.name
        );
    }
    if let Some(notice) = &notice {
        // The same marker the MCP server appends to a clipped result, so a reader who has learned
        // to look for one line has learned to look for both.
        println!("… [truncated: {notice}]");
    }
    Ok(())
}

/// Playing time of one sample. A bank can carry a zero frequency for a placeholder entry, and
/// dividing by it would print `NaN`/`inf` into a JSON document that then fails to parse.
fn sample_seconds(sample: &gore_fmod::BankSample) -> f64 {
    if sample.freq > 0 {
        sample.num_samples as f64 / sample.freq as f64
    } else {
        0.0
    }
}

/// One sentence that must answer "how much am I not seeing" and "what do I type instead". It
/// deliberately does not hand back the `--max` that would list everything: followed on the 7,218
/// samples of `SFX.bank` that is a 458,589-byte table against a 256 KiB result budget
/// (`gore_mcp::DEFAULT_MAX_STDOUT_BYTES`), and the cut lands mid-line inside sample #4122 -- so the
/// 3,095 samples past it are not merely unshown, they are absent, and a caller who filters what
/// arrived is told a sound does not exist. Sending a caller there is the failure this bound
/// prevents.
fn list_truncation_notice(matched: usize, listed: usize) -> String {
    format!(
        "{matched} samples matched and only the first {listed} are shown. Narrow the query with \
         --filter, and raise --max only as far as you need: asking for all {matched} at once \
         produces a document large enough to be cut off in transit, and a cut-off JSON array no \
         longer parses."
    )
}

pub fn extract(
    bank: PathBuf,
    out: PathBuf,
    sample: Option<String>,
    filter: Option<String>,
    key: Option<String>,
) -> Result<()> {
    // A named `--sample` and a `--filter` are two different selections, and honouring one means
    // ignoring the other. Silently keeping the sample meant a caller who passed both got a
    // successful extraction that answered half their request. `--sample all` is not a conflict:
    // it is the default, and it means "no sample selection", which is exactly what a filter narrows.
    //
    // First, before the bank is read and before the output directory exists. This needs nothing
    // but the two arguments, and running it last meant a call that was never going to be honoured
    // still read 260 MB of `SFX.bank` and left an empty directory behind that the caller then had
    // to clean up after an error telling them they had passed the wrong flags.
    if let (Some(name), Some(needle)) = (&sample, &filter) {
        if name != "all" {
            bail!(
                "--sample '{name}' and --filter '{needle}' select differently and cannot both be \
                 honoured. Pass one: --sample for a single known name, --filter for every name \
                 containing a substring."
            );
        }
    }

    let bytes = std::fs::read(&bank).with_context(|| format!("reading '{}'", bank.display()))?;
    // Through the view, so a replaced sample is read out of the sub-bank it was repointed at.
    // Reading sub-bank 0 wrote the audio the replacement replaced into a file named after the
    // replacement — the one failure mode a caller cannot detect, because the file is there and
    // plays.
    let view = gore_fmod::read_bank(&bytes, &key_bytes(key))
        .map_err(|e| anyhow::anyhow!("{e}"))
        .context("decoding bank")?;

    // indices to extract
    let indices: Vec<usize> = match (&sample, &filter) {
        (Some(name), _) if name != "all" => vec![view
            .samples
            .iter()
            .position(|s| &s.name == name)
            .with_context(|| format!("sample not found: {name}"))?],
        // Same semantics as `list --filter`: case-insensitive substring. Auditioning a variant set
        // is the normal way into this command, and doing it one `--sample` at a time was the
        // reason a directory kept filling up.
        (_, Some(needle)) => {
            let needle = needle.to_lowercase();
            let matched: Vec<usize> = view
                .samples
                .iter()
                .enumerate()
                .filter(|(_, s)| super::contains_case_insensitive(&s.name, &needle))
                .map(|(i, _)| i)
                .collect();
            if matched.is_empty() {
                anyhow::bail!(
                    "no sample name contains '{needle}'; `gore audio list --bank <BANK> \
                     --filter {needle}` shows what the bank does carry"
                );
            }
            matched
        }
        _ => (0..view.samples.len()).collect(),
    };

    // One line per distinct reason, not per sample. `extract_wav` decodes Vorbis, so a bank whose
    // codec is anything else rejects every sample for the same cause: on the 7,218 samples of
    // `SFX.bank` that was 7,218 identical stderr lines (~400 KB) describing one fact. The first
    // sample to hit a reason is named, which is what a single-sample run needs, and the count says
    // how far it went.
    // Each WAV is published with an atomic create-if-absent open, and a failure removes whatever
    // this run already wrote. Three requirements meet here and only this shape satisfies all of
    // them.
    //
    // Never overwrite. A separate `exists()` check followed by a write is a race: `std::fs::rename`
    // replaces an existing destination on every platform this ships to — the comment on
    // `write_atomic` above says so — so an editor or a second extraction that created the file in
    // between would lose it. `create_new(true)` asks the filesystem the question and takes the file
    // in one operation, so there is no window to lose.
    //
    // Only refuse for files this run would really write. Whether a sample yields audio is known
    // only after `extract_wav` has tried, because a codec it cannot read is skipped — so planning
    // every selected destination up front refused runs over leftovers belonging to samples that
    // were never going to be written. Opening the destination only once there are bytes for it
    // makes the question exact.
    //
    // Leave nothing behind on failure. A collision on the fifth sample used to leave four WAVs
    // written, and the obvious retry then failed on one of those rather than on the file the caller
    // had to deal with.
    //
    // Prefix with the sample index so two names that sanitize to the same basename (e.g. differing
    // only by punctuation) do not collide and silently overwrite.
    // Created only now, with a selection that is known to be non-empty. A `--filter` matching
    // nothing errors below, and doing this first left an empty directory behind for a call that
    // extracted nothing — the same "refuse before touching anything" the selector check above got.
    std::fs::create_dir_all(&out).with_context(|| format!("creating '{}'", out.display()))?;

    let (mut ok, mut skipped) = (0usize, 0usize);
    let mut skips: BTreeMap<String, (usize, usize)> = BTreeMap::new();
    // Path, the length written, and the modification time the filesystem gave the file the moment
    // this run finished writing it — recorded while it is certainly still ours, because rollback
    // can only ask what is on disk later.
    let mut published: Vec<Published> = Vec::new();

    // Best effort by nature: the run has already failed, and a file that cannot be removed is not a
    // reason to fail differently.
    //
    // It removes only what still looks like what this run wrote. `create_new` stops the *creation*
    // from clobbering anything, but rollback happens later, and an editor or a watcher that
    // replaced an early WAV while a long extraction was still running would otherwise have its file
    // deleted by a failure it had nothing to do with.
    //
    // Length AND modification time, because length alone let exactly the commonest such edit
    // through: rewriting audio without changing its duration or encoding leaves the byte count
    // where it was. What remains undistinguished is a rewrite that also restores the old timestamp,
    // which no editor does by accident. A real file identity (inode, file index) would not close it
    // either — an in-place rewrite keeps that too — so the mtime is the signal that matters here.

    for i in indices {
        let wav = match view.extract_wav(i) {
            Ok(wav) => wav,
            Err(e) => {
                skipped += 1;
                let seen = skips.entry(e).or_insert((0, i));
                seen.0 += 1;
                continue;
            }
        };

        let dest = out.join(format!("{i}_{}.wav", sanitize(&view.samples[i].name)));
        let mut file = match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&dest)
        {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                let stuck = roll_back(&published);
                bail!(
                    "'{}' already exists; extract writes one file per sample under a name taken \
                     from the bank, so this would replace it.{} Delete it, or pass a different \
                     --out.",
                    dest.display(),
                    match stuck.is_empty() {
                        true => " Nothing was written.".to_string(),
                        false => stuck,
                    }
                );
            }
            Err(error) => {
                let stuck = roll_back(&published);
                return Err(error)
                    .with_context(|| format!("creating '{}'{stuck}", dest.display()));
            }
        };
        if let Err(error) = file.write_all(&wav) {
            // The digest that identifies a finished file cannot identify this one: the write
            // failed partway, so what is on disk is a prefix of unknown length. What can still be
            // told is whether the path leads to the file this run opened, which is the same
            // question and the only one that matters before deleting anything.
            // Through the same move-then-decide path as rollback, so the file that gets deleted is
            // the one that was verified and not whatever the name leads to a moment later. The
            // handle stays valid across the rename and still identifies the object it was opened
            // on, which is what makes it usable as the second check.
            let removal = remove_our_file(&dest, &|path| still_the_same_file(&file, path));
            drop(file);
            let stuck = roll_back(&published);
            // One sentence per outcome. Folded to a boolean, three different endings came out as
            // "something else replaced that file" — including a partial file that is still ours
            // and sitting there, which is the one case where saying so matters most.
            return Err(error).with_context(|| match removal {
                Removal::Removed | Removal::Absent => {
                    format!("writing '{}'{stuck}", dest.display())
                }
                // Left in place on purpose, and said so: something replaced the file between
                // `create_new` opening it and this write failing, and deleting a file this run did
                // not write is worse than leaving a partial one somebody can see and remove.
                Removal::NotOurs => format!(
                    "writing '{}' — something else replaced that file while it was being written, \
                     so it was left as it is{stuck}",
                    dest.display()
                ),
                // Ours, still there, and it could not be taken back. The retry collides on this
                // path, so the reason has to travel with the path rather than be read as somebody
                // else's file.
                Removal::Failed(why) => format!(
                    "writing '{}' — the partial file could not be removed ({why}){stuck}",
                    dest.display()
                ),
            });
        }
        // Taken from the buffer, not from the file: this is what was written, with no window
        // between writing it and identifying it.
        let digest = digest_of(&wav);
        published.push(Published { path: dest, written: wav.len() as u64, digest });
        ok += 1;
    }
    for (reason, (count, first)) in &skips {
        eprintln!(
            "skipped {count} sample(s), first #{first} {}: {reason}",
            view.samples[*first].name
        );
    }
    println!(
        "extracted {ok} wav file(s) to {} ({skipped} skipped)",
        out.display()
    );
    Ok(())
}

/// One file this run created, and the evidence that it is still that file.
struct Published {
    path: PathBuf,
    written: u64,
    /// SHA-256 of the bytes written here, taken from the buffer that was about to be written.
    ///
    /// Length and modification time came first and were not enough. Two of the three signals a
    /// stat can offer are coarse by design: an editor rewriting a WAV without changing its
    /// duration or encoding keeps the byte count, and a filesystem with a two-second timestamp
    /// tick — FAT32, and the removable media people keep game files on — can leave the mtime
    /// unchanged as well. Both together still described a file this run did not write, and
    /// rollback would have deleted it.
    ///
    /// Hashing costs one pass over bytes already in memory, and the comparison only happens on the
    /// failure path, where a re-read is not what anybody is waiting for.
    digest: [u8; 32],
}

impl Published {
    /// Whether what is at `path` now is still what this run wrote there.
    ///
    /// Unreadable counts as not ours: rollback exists to undo this run's own writes, and a file it
    /// cannot even look at is not one it should delete.
    fn is_still_ours(&self) -> bool {
        self.is_ours_at(&self.path)
    }

    /// The same question about a path this file may have been moved to.
    fn is_ours_at(&self, path: &Path) -> bool {
        // What this run created was a regular file, through `create_new`. Anything else on that
        // path now is a replacement, whatever it points at — and both `metadata` and `read` follow
        // a link, so a symlink aimed at a file with these very bytes answered every question below
        // as if it were ours. Rollback would then have deleted somebody's link.
        if !is_regular_file(path) {
            return false;
        }
        let Ok(metadata) = std::fs::metadata(path) else {
            return false;
        };
        // Cheap reject first: a different length cannot be the same content, and this spares the
        // read for every file something else has plainly replaced.
        if metadata.len() != self.written {
            return false;
        }
        let Ok(bytes) = std::fs::read(path) else {
            return false;
        };
        digest_of(&bytes) == self.digest
    }
}

/// A name in the same directory that nothing else can be holding or about to hold.
///
/// Same directory because the move below has to be a rename and not a copy, and a rename across
/// volumes is not one. The counter separates two files rolled back in one run.
///
/// Nothing is created here. Two earlier shapes of this were both wrong: a process id plus a
/// process-local counter REPEATS — this code deliberately leaves quarantined files behind when it
/// cannot restore them, and an operating system reuses process ids, so a later run produced the
/// same name and the rename replaced that file. Creating the name first to prove it was free only
/// moved the problem: `rename` replaces its destination, so the placeholder itself was something
/// that could be swapped between reserving it and moving onto it.
///
/// A name nothing can predict has neither failure. `RandomState` is seeded by the operating
/// system, so the suffix does not repeat across runs and is not derived from anything an outside
/// process can reproduce. What is left is a 2^-64 coincidence rather than a window between two
/// calls — and std offers no rename that refuses an occupied destination to close even that.
fn quarantine_path(path: &Path) -> PathBuf {
    use std::hash::{BuildHasher, Hasher};
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let serial = NEXT.fetch_add(1, Ordering::Relaxed);
    let entropy = std::collections::hash_map::RandomState::new()
        .build_hasher()
        .finish();
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(format!(".gore-rm-{entropy:016x}-{serial}"));
    path.with_file_name(name)
}

/// What became of a file this run tried to take back.
enum Removal {
    Removed,
    /// Nothing on the path, so nothing to undo.
    Absent,
    /// Something else's file is there now. Left exactly as found.
    NotOurs,
    Failed(String),
}

/// Delete a file this run wrote, deciding about the file rather than about the path.
///
/// Verifying a path and then unlinking it are two resolutions of one name, and whatever arrives
/// between them is what gets deleted — someone else's file, destroyed by a rollback whose whole
/// purpose is to touch only this run's own writes. Renaming first moves the object off the
/// contested name in one operation, onto a name nothing else knows, so the verification that
/// authorises the delete is made about a file no other writer can still reach.
///
/// The residual is a replacement landing between the first check and the rename: the rename then
/// moves a file that is not ours and the second check says so. Putting it back is a `hard_link`,
/// which refuses an occupied destination in one operation — never a look followed by a rename,
/// because a file created between those two is destroyed by the code that exists not to destroy
/// one. Where that cannot be done at all, on a filesystem with no links, the file stays under the
/// quarantine name and the message says where. That costs somebody a rename; guessing costs them
/// a file.
fn remove_our_file(path: &Path, ours: &dyn Fn(&Path) -> bool) -> Removal {
    if !ours(path) {
        return match std::fs::symlink_metadata(path) {
            Ok(_) => Removal::NotOurs,
            // Only "not there" is absent. Every other reason a path cannot be inspected — an ACL
            // that changed under the run, an I/O error — was folded into it, so a file this run
            // wrote and could not even look at was reported as a path with nothing on it, and the
            // caller went on to say "Nothing was written" about a directory still holding it. The
            // next attempt then collides on a path the message had just called clear.
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Removal::Absent,
            Err(error) => Removal::Failed(format!("could not be inspected: {error}")),
        };
    }
    let quarantine = quarantine_path(path);
    if let Err(error) = std::fs::rename(path, &quarantine) {
        return match error.kind() {
            std::io::ErrorKind::NotFound => Removal::Absent,
            _ => Removal::Failed(error.to_string()),
        };
    }
    if !ours(&quarantine) {
        // Something replaced our file in the window above and this moved that instead. Put it back
        // where it was — but `rename` silently overwrites its destination, and by now a third file
        // may have been created on that path. Restoring over it would destroy exactly what this
        // whole dance exists to protect, so the restore is a link that refuses to overwrite:
        // `hard_link` fails when the destination exists, in one operation, with no window between
        // asking whether the path is free and taking it.
        return match std::fs::hard_link(&quarantine, path) {
            Ok(()) => match std::fs::remove_file(&quarantine) {
                Ok(()) => Removal::NotOurs,
                // The file IS back where it belongs; only the extra name is left over. Saying so
                // is better than calling the restore a failure.
                Err(error) => Removal::Failed(format!(
                    "something else replaced it, it was put back, and the copy at {} could not be \
                     cleaned up: {error}",
                    quarantine.display()
                )),
            },
            // The path is taken again, or links are not available at all — FAT32 has none. Both
            // end here, and neither is worth a fallback: looking at the path and then renaming
            // onto it is two operations with a window between them, and a file created in that
            // window is destroyed by the very code whose whole purpose is not to destroy one.
            // Leaving it under a name the message gives costs somebody a rename. Guessing wrong
            // costs them a file.
            Err(error) => Removal::Failed(match error.kind() {
                std::io::ErrorKind::AlreadyExists => format!(
                    "something else replaced it, and another file has taken that path since, so \
                     it was left at {} rather than written over the newer one",
                    quarantine.display()
                ),
                _ => format!(
                    "something else replaced it and it could not be put back without risking \
                     whatever is on that path now ({error}), so it was left at {}",
                    quarantine.display()
                ),
            }),
        };
    }
    match std::fs::remove_file(&quarantine) {
        Ok(()) => Removal::Removed,
        Err(error) => Removal::Failed(format!(
            "moved to {} and could not be deleted: {error}",
            quarantine.display()
        )),
    }
}

/// A file's identity as this platform's stable API can express it.
///
/// Unix has the real thing — inode and device. Windows exposes `file_index`/`volume_serial_number`
/// only behind an unstable feature, so the creation and last-write times stand in: both are
/// 100-nanosecond NTFS values, and a file that replaced ours between `create_new` and a failed
/// write would have to have been created at the very same tick to pass for it. That is a far
/// narrower residual than the one it replaces, and it is stated rather than assumed.
///
/// `None` where nothing can be had, and `None` never compares equal to `None` in the use below:
/// where identity cannot be established, nothing is deleted.
#[cfg(windows)]
fn identity_of(metadata: &std::fs::Metadata) -> Option<(u64, u64)> {
    use std::os::windows::fs::MetadataExt as _;
    Some((metadata.creation_time(), metadata.last_write_time()))
}

#[cfg(unix)]
fn identity_of(metadata: &std::fs::Metadata) -> Option<(u64, u64)> {
    use std::os::unix::fs::MetadataExt as _;
    Some((metadata.ino(), metadata.dev()))
}

#[cfg(not(any(windows, unix)))]
fn identity_of(_: &std::fs::Metadata) -> Option<(u64, u64)> {
    None
}

/// Whether the path holds a regular file — not a link to one, not a directory, not a device.
///
/// `symlink_metadata` looks AT the path instead of through it, which is the whole point: every
/// other question here is about the bytes at the other end of a link, and a link is not the file
/// this run wrote.
fn is_regular_file(path: &Path) -> bool {
    std::fs::symlink_metadata(path).is_ok_and(|metadata| metadata.file_type().is_file())
}

/// Whether the path still leads to the file this handle holds. `false` when either side cannot be
/// identified, so an unknown answer never authorises a delete.
fn still_the_same_file(file: &std::fs::File, path: &Path) -> bool {
    // Same reason as in `is_ours_at`: a link that resolves to the object this handle holds is
    // still not the object this run created on that path.
    if !is_regular_file(path) {
        return false;
    }
    let held = file.metadata().ok().and_then(|m| identity_of(&m));
    let there = std::fs::metadata(path).ok().and_then(|m| identity_of(&m));
    match (held, there) {
        (Some(held), Some(there)) => held == there,
        _ => false,
    }
}

/// Undo this run's own writes, and return what it could NOT undo.
///
/// Discarding that and then telling the caller "Nothing was written" claimed a clean rollback the
/// code had not established — and a file left behind is exactly what makes the obvious retry
/// collide again, on a path the message had just called clear.
fn roll_back(published: &[Published]) -> String {
    let mut stuck: Vec<String> = Vec::new();
    for entry in published {
        match remove_our_file(&entry.path, &|path| entry.is_ours_at(path)) {
            Removal::Removed => {}
            // Gone already — deleted by hand, or by another run. Warning about a collision on a
            // path that is free is the opposite of the help this sentence exists to give.
            Removal::Absent => {}
            // Something replaced it while this run was working, and deleting that would destroy
            // it. It is still ON the path this run wrote to, though, so the next attempt collides
            // there — reporting the rollback as complete sent the reader back into the same wall.
            Removal::NotOurs => stuck.push(format!(
                "{} (changed by something else; left alone)",
                entry.path.display()
            )),
            Removal::Failed(error) => {
                stuck.push(format!("{} ({error})", entry.path.display()))
            }
        }
    }
    describe_stuck(&stuck)
}

/// The sentence a caller appends to its own error. Separated from the removal so the wording can
/// be exercised: making `remove_file` fail on demand needs an ACL this suite cannot set, and
/// Windows deletes a file happily through an open handle — `File::open` shares delete access —
/// so there is no portable way to provoke the failure itself.
fn describe_stuck(stuck: &[String]) -> String {
    match stuck.is_empty() {
        true => String::new(),
        false => format!(
            " {} file(s) this run wrote are still there: {}.",
            stuck.len(),
            stuck.join(", ")
        ),
    }
}

fn digest_of(bytes: &[u8]) -> [u8; 32] {
    use sha2::{Digest as _, Sha256};
    Sha256::digest(bytes).into()
}

fn sanitize(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

pub fn replace(
    map: PathBuf,
    bank: PathBuf,
    out: Option<PathBuf>,
    key: Option<String>,
) -> Result<()> {
    let key = key_bytes(key);
    let bytes = read_pristine_bank(&bank)?;

    let map_json = std::fs::read_to_string(&map)
        .with_context(|| format!("reading map '{}'", map.display()))?;
    let entries: BTreeMap<String, String> = serde_json::from_str(&map_json)
        .context("parsing map (expected {\"SampleName\": \"path/to/new.wav\"})")?;
    if entries.is_empty() {
        bail!("map is empty");
    }

    // resolve WAV paths relative to the map file's directory
    let base = map
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let mut replacements = Vec::with_capacity(entries.len());
    for (name, wav_rel) in &entries {
        let wav_path = resolve(&base, wav_rel);
        let wav = std::fs::read(&wav_path)
            .with_context(|| format!("reading wav '{}'", wav_path.display()))?;
        let (rate, channels, pcm) =
            gore_fmod::read_wav_pcm16(&wav).map_err(|e| anyhow::anyhow!("{wav_rel}: {e}"))?;
        replacements.push((
            name.clone(),
            gore_fmod::Pcm16Sample {
                name: name.clone(),
                freq: rate,
                channels,
                pcm,
            },
        ));
    }
    let count = entries.len();

    let new_bank = gore_fmod::replace_samples(&bytes, &key, replacements)
        .map_err(|e| anyhow::anyhow!("{e}"))
        .context("injecting replacements")?;

    write_result(&bank, &new_bank, out, count)
}

/// Write a rebuilt bank to `out`, or overwrite `bank` in place after backing it up.
fn write_result(bank: &Path, new_bank: &[u8], out: Option<PathBuf>, count: usize) -> Result<()> {
    match out {
        Some(o) => {
            write_atomic(&o, new_bank)?;
            println!("wrote {} ({count} sample(s) replaced)", o.display());
        }
        None => {
            let bak = backup_path(bank);
            // Refresh the backup from the CURRENT live bank when that bank is itself pristine (a
            // single FSB5): this covers the first replace AND a Steam update/restore that refreshed
            // the live bank while a stale pre-update *.gore-bak lingered — without this, that stale
            // backup would survive and a later `restore` would write it over the updated bank. When
            // the live bank is already injected, an existing backup is the true pristine: keep it.
            let live =
                std::fs::read(bank).with_context(|| format!("reading '{}'", bank.display()))?;
            if gore_fmod::is_pristine_bank(&live) {
                std::fs::write(&bak, &live)
                    .with_context(|| format!("backing up to '{}'", bak.display()))?;
                println!("backed up -> {}", bak.display());
            } else if !bak.exists() {
                std::fs::copy(bank, &bak)
                    .with_context(|| format!("backing up to '{}'", bak.display()))?;
                println!("backed up -> {}", bak.display());
            }
            write_atomic(bank, new_bank)?;
            println!("replaced {count} sample(s) in place -> {}", bank.display());
        }
    }
    Ok(())
}

/// Build a shareable audio patch zip: a manifest + the replacement WAVs (no game audio).
pub fn export_patch(map: PathBuf, out: PathBuf) -> Result<()> {
    let map_json = std::fs::read_to_string(&map)
        .with_context(|| format!("reading map '{}'", map.display()))?;
    let entries: BTreeMap<String, String> = serde_json::from_str(&map_json)
        .context("parsing map (expected {\"SampleName\": \"path/to/new.wav\"})")?;
    if entries.is_empty() {
        bail!("map is empty");
    }
    let base = map
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));

    let file =
        std::fs::File::create(&out).with_context(|| format!("creating '{}'", out.display()))?;
    let mut zip = zip::ZipWriter::new(file);
    let opts: zip::write::FileOptions<()> =
        zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    let mut manifest = BTreeMap::new();
    for (i, (name, wav_rel)) in entries.iter().enumerate() {
        let wav = std::fs::read(resolve(&base, wav_rel))
            .with_context(|| format!("reading wav for '{name}'"))?;
        // Prefix with the index so distinct sample names that sanitize to the same string
        // (e.g. "a:b" and "a/b") can't collide on one zip member.
        let entry = format!("audio/{i}_{}.wav", sanitize(name));
        zip.start_file(&entry, opts).context("zip start_file")?;
        zip.write_all(&wav).context("zip write")?;
        manifest.insert(name.clone(), entry);
    }
    zip.start_file("manifest.json", opts)
        .context("zip manifest")?;
    zip.write_all(&serde_json::to_vec_pretty(&manifest)?)?;
    zip.finish().context("finishing zip")?;
    println!(
        "wrote patch {} ({} sample(s))",
        out.display(),
        entries.len()
    );
    Ok(())
}

/// Apply a patch zip (from `export-patch`) to a bank.
pub fn apply_patch(
    patch: PathBuf,
    bank: PathBuf,
    out: Option<PathBuf>,
    key: Option<String>,
) -> Result<()> {
    let key = key_bytes(key);
    let bytes = read_pristine_bank(&bank)?;

    let file =
        std::fs::File::open(&patch).with_context(|| format!("opening '{}'", patch.display()))?;
    let mut zip = zip::ZipArchive::new(file).context("reading patch zip")?;

    let manifest: BTreeMap<String, String> = {
        let mut f = zip
            .by_name("manifest.json")
            .context("patch missing manifest.json")?;
        let mut s = String::new();
        f.read_to_string(&mut s)?;
        serde_json::from_str(&s).context("parsing manifest.json")?
    };
    if manifest.is_empty() {
        bail!("patch manifest is empty");
    }

    let mut replacements = Vec::with_capacity(manifest.len());
    for (name, entry) in &manifest {
        let wav = {
            let mut f = zip
                .by_name(entry)
                .with_context(|| format!("patch missing entry '{entry}' for '{name}'"))?;
            let mut buf = Vec::new();
            f.read_to_end(&mut buf)?;
            buf
        };
        let (rate, channels, pcm) =
            gore_fmod::read_wav_pcm16(&wav).map_err(|e| anyhow::anyhow!("{name}: {e}"))?;
        replacements.push((
            name.clone(),
            gore_fmod::Pcm16Sample {
                name: name.clone(),
                freq: rate,
                channels,
                pcm,
            },
        ));
    }
    let count = replacements.len();
    let new_bank = gore_fmod::replace_samples(&bytes, &key, replacements)
        .map_err(|e| anyhow::anyhow!("{e}"))
        .context("applying patch")?;
    write_result(&bank, &new_bank, out, count)
}

pub fn restore(bank: PathBuf) -> Result<()> {
    let bak = backup_path(&bank);
    if !bak.exists() {
        bail!("no backup found at '{}'", bak.display());
    }
    // If the live bank is a clean, un-injected pristine (a single FSB5), it is already current —
    // e.g. Steam verified/updated it since we backed it up. Restoring the stale backup would
    // downgrade the newer file, so just drop the stale backup instead. A corrupt/injected live bank
    // is NOT pristine, so it still gets restored from the backup.
    let live = std::fs::read(&bank).with_context(|| format!("reading '{}'", bank.display()))?;
    if gore_fmod::is_pristine_bank(&live) {
        let _ = std::fs::remove_file(&bak);
        println!(
            "{} is not injected (already pristine); removed stale backup {}",
            bank.display(),
            bak.display()
        );
        return Ok(());
    }
    let bytes = std::fs::read(&bak).with_context(|| format!("reading '{}'", bak.display()))?;
    write_atomic(&bank, &bytes)?;
    // Drop the backup now that the bank is back to pristine: keeping it would let it go stale
    // against a later Steam update, and a fresh one is re-created from the pristine bank on the
    // next replace.
    let _ = std::fs::remove_file(&bak);
    println!("restored {} from {}", bank.display(), bak.display());
    Ok(())
}

fn backup_path(bank: &Path) -> PathBuf {
    let mut s = bank.as_os_str().to_os_string();
    s.push(".gore-bak");
    PathBuf::from(s)
}

fn resolve(base: &Path, rel: &str) -> PathBuf {
    let p = Path::new(rel);
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        base.join(p)
    }
}

/// `banks` against a directory of fixture banks.
///
/// These run below the process boundary because the rest of `audio`'s coverage lives in
/// `tests/integration/audio_test.rs` and needs a bank on disk either way; what cannot be faked here
/// is a game installation, so `banks` is split into a resolver and two renderers and the renderers
/// are what these drive. The banks themselves are real: `gore_fmod`'s fixture builder emits the
/// same encrypted RIFF/`FEV ` wrapper the reader walks, and its codec is PCM16 rather than the
/// shipped Vorbis, so a codec cell can only be right by having been read.
#[cfg(test)]
mod rollback_tests {
    use super::Published;
    use std::io::Write as _;
    use tempfile::TempDir;

    /// Exactly what the extraction loop records: the bytes, and their digest taken from the
    /// buffer that was about to be written.
    fn publish(dir: &std::path::Path, name: &str, bytes: &[u8]) -> Published {
        let path = dir.join(name);
        let mut file = std::fs::File::create(&path).unwrap();
        file.write_all(bytes).unwrap();
        Published {
            path,
            written: bytes.len() as u64,
            digest: super::digest_of(bytes),
        }
    }

    #[test]
    fn a_same_length_rewrite_is_not_claimed_as_this_runs_output() {
        let temp = TempDir::new().unwrap();
        let entry = publish(temp.path(), "0_line.wav", b"original bytes");
        assert!(entry.is_still_ours());

        // What an editor does to a WAV: different audio, same duration and encoding, so the same
        // byte count. Nothing here touches the timestamp, and it does not need to — that was the
        // signal this test used to rely on, and a filesystem with a two-second tick can leave it
        // unchanged through exactly this edit.
        std::fs::write(&entry.path, b"replaced bytes").unwrap();
        assert_eq!(
            std::fs::metadata(&entry.path).unwrap().len(),
            entry.written,
            "the fixture is only interesting while the length still matches"
        );
        assert!(!entry.is_still_ours());
    }

    #[test]
    fn a_rollback_that_could_not_remove_everything_says_so() {
        // The wording is what the caller pastes into its own error, and the claim that used to be
        // there — "Nothing was written" — was the thing worth fixing. A file left behind is also
        // what makes the obvious retry collide again, on a path the message had called clear.
        assert!(super::describe_stuck(&[]).is_empty());

        let said = super::describe_stuck(&["C:/out/0_line.wav (Access is denied)".to_string()]);
        assert!(said.contains("are still there"), "{said}");
        assert!(said.contains("0_line.wav"), "{said}");
        assert!(said.contains("Access is denied"), "{said}");

        // A file something else replaced is kept AND named: it still occupies the path this run
        // wrote to, so the next attempt collides there, and silence sent the reader into the same
        // wall twice.
        let temp = TempDir::new().unwrap();
        let mut theirs = publish(temp.path(), "1_line.wav", b"original bytes");
        std::fs::write(&theirs.path, b"replaced bytes").unwrap();
        theirs.digest = super::digest_of(b"something else entirely");
        let kept = super::roll_back(std::slice::from_ref(&theirs));
        assert!(kept.contains("left alone"), "{kept}");
        assert!(kept.contains("1_line.wav"), "{kept}");
        assert!(theirs.path.exists(), "somebody else's file must survive the rollback");

        // A path that is simply gone is not retained: warning about a collision on a free path
        // is the opposite of the help this sentence exists to give.
        let temp = TempDir::new().unwrap();
        let mut vanished = publish(temp.path(), "2_line.wav", b"bytes");
        vanished.digest = super::digest_of(b"something else entirely");
        std::fs::remove_file(&vanished.path).unwrap();
        assert!(
            super::roll_back(std::slice::from_ref(&vanished)).is_empty(),
            "a path nobody occupies is not something to warn about"
        );

        // And a clean rollback still says nothing, so the caller's own sentence stands alone.
        let temp = TempDir::new().unwrap();
        let entry = publish(temp.path(), "0_line.wav", b"bytes");
        assert!(super::roll_back(std::slice::from_ref(&entry)).is_empty());
        assert!(!entry.path.exists(), "a clean rollback removes what it wrote");
    }

    #[test]
    fn a_file_that_arrives_between_the_check_and_the_delete_is_not_the_one_deleted() {
        // Verifying a path and then unlinking it are two resolutions of one name, and whatever
        // lands between them is what gets destroyed — by a rollback whose entire purpose is to
        // touch only this run's own writes. The move-then-decide path makes the second look happen
        // on a name nothing else knows, and this drives the window itself: the verifier answers
        // yes, then no, which is exactly what a replacement arriving in between looks like.
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("0_line.wav");
        std::fs::write(&path, b"somebody else's audio").unwrap();

        let answers = std::cell::Cell::new(0u32);
        let outcome = super::remove_our_file(&path, &|_| {
            answers.set(answers.get() + 1);
            answers.get() == 1
        });

        assert_eq!(answers.get(), 2, "the second look is the point of the exercise");
        assert!(matches!(outcome, super::Removal::NotOurs), "a foreign file must survive");
        assert_eq!(
            std::fs::read(&path).unwrap(),
            b"somebody else's audio",
            "and survive under the name it was left at"
        );
        let left: Vec<_> = std::fs::read_dir(temp.path())
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect();
        assert_eq!(left.len(), 1, "no quarantine name may be left behind: {left:?}");
    }

    #[test]
    fn removing_our_own_file_takes_the_file_and_leaves_nothing_beside_it() {
        // The ordinary path, and the control for the test above: the same code deletes when both
        // looks agree, and leaves no quarantine name behind when it does.
        let temp = TempDir::new().unwrap();
        let entry = publish(temp.path(), "0_line.wav", b"ours");

        let outcome = super::remove_our_file(&entry.path, &|path| entry.is_ours_at(path));
        assert!(matches!(outcome, super::Removal::Removed));
        assert_eq!(std::fs::read_dir(temp.path()).unwrap().count(), 0);

        // And a path with nothing on it is not a failure to report.
        let outcome = super::remove_our_file(&entry.path, &|path| entry.is_ours_at(path));
        assert!(matches!(outcome, super::Removal::Absent));
    }

    /// A symlink at `link` pointing at `target`, or the reason there is none.
    ///
    /// Creating one needs privileges a plain Windows session does not have, so this test cannot
    /// simply assume it. It also must not quietly pass where it CAN run — the CI runner creates
    /// them fine — so the caller fails there and skips only on a developer machine.
    fn try_symlink(target: &std::path::Path, link: &std::path::Path) -> std::io::Result<()> {
        #[cfg(windows)]
        {
            std::os::windows::fs::symlink_file(target, link)
        }
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(target, link)
        }
    }

    #[test]
    fn a_link_put_in_our_place_is_not_ours_however_identical_the_bytes_are() {
        // `metadata` and `read` both follow a link, so a symlink aimed at a file carrying exactly
        // these bytes answered every ownership question as if it were this run's own output —
        // and rollback deleted somebody's link. What this run created was a regular file; nothing
        // else on that path is it, whatever it resolves to.
        let temp = TempDir::new().unwrap();
        let entry = publish(temp.path(), "0_line.wav", b"ours");
        let elsewhere = temp.path().join("theirs.wav");
        std::fs::write(&elsewhere, b"ours").unwrap();
        std::fs::remove_file(&entry.path).unwrap();

        if let Err(error) = try_symlink(&elsewhere, &entry.path) {
            assert!(
                std::env::var_os("CI").is_none(),
                "this test has to run where links can be made: {error}"
            );
            eprintln!("skipped: creating a symlink needs privileges here ({error})");
            return;
        }

        assert_eq!(
            std::fs::read(&entry.path).unwrap(),
            b"ours",
            "the fixture is only interesting while the link resolves to identical bytes"
        );
        assert!(
            !entry.is_still_ours(),
            "identical bytes through a link are still not this run's file"
        );
        let kept = super::roll_back(std::slice::from_ref(&entry));
        assert!(kept.contains("left alone"), "{kept}");
        assert!(entry.path.exists(), "the link must survive the rollback");
        assert!(elsewhere.exists(), "and so must what it points at");
    }

    #[test]
    fn a_path_taken_again_is_not_written_over_by_the_file_that_was_moved_off_it() {
        // The far end of the same race. Our file is replaced, the move takes the replacement, and
        // then a third file lands on the path — at which point putting the replacement back means
        // destroying something newer, which is the exact harm this code exists to avoid.
        // `rename` would have done it silently.
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("0_line.wav");
        std::fs::write(&path, b"the replacement").unwrap();

        let answers = std::cell::Cell::new(0u32);
        let taken = path.clone();
        let outcome = super::remove_our_file(&path, &|_| {
            answers.set(answers.get() + 1);
            if answers.get() == 2 {
                // The path is free at this moment — the move emptied it — and somebody fills it.
                std::fs::write(&taken, b"a newer file entirely").unwrap();
                return false;
            }
            true
        });

        match outcome {
            super::Removal::Failed(why) => {
                assert!(why.contains("taken that path"), "{why}");
                assert!(why.contains("gore-rm-"), "the file has to be findable: {why}");
            }
            _ => panic!("a path taken again cannot be reported as a clean outcome"),
        }
        assert_eq!(
            std::fs::read(&path).unwrap(),
            b"a newer file entirely",
            "the newer file must be exactly as it was left"
        );
        assert_eq!(
            std::fs::read_dir(temp.path()).unwrap().count(),
            2,
            "and the moved one is still there, under the name the message gives"
        );
    }

    #[test]
    fn a_quarantine_name_is_one_nothing_else_can_be_holding() {
        // Two earlier shapes of this were both wrong. A process id plus a counter REPEATS — this
        // code deliberately leaves quarantined files behind, and process ids are reused, so a
        // later run produced the same name and the move replaced that file. Creating the name
        // first to prove it was free only moved the problem: `rename` replaces its destination,
        // so the placeholder was itself something that could be swapped in between.
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("0_line.wav");

        let first = super::quarantine_path(&path);
        let second = super::quarantine_path(&path);
        assert_ne!(first, second, "two names in one run are two names");
        assert!(!first.exists() && !second.exists(), "nothing is created to be swapped");
        assert_eq!(std::fs::read_dir(temp.path()).unwrap().count(), 0);

        // Under the file they belong to, in its own directory, so the move can be a rename rather
        // than a copy across volumes.
        for name in [&first, &second] {
            assert_eq!(name.parent(), path.parent());
            let name = name.file_name().unwrap().to_string_lossy().into_owned();
            assert!(name.starts_with("0_line.wav.gore-rm-"), "{name}");
            // Not derived from anything an outside process can reproduce, which is what the
            // process-id version was.
            assert!(
                !name.contains(&std::process::id().to_string()),
                "the name must not be predictable from this process: {name}"
            );
        }
    }

    #[test]
    fn a_move_that_takes_nothing_back_leaves_nothing_behind() {
        // True by construction now that no placeholder is created, and worth holding: the two
        // earlier shapes of the quarantine name both wrote to the directory before the move, and
        // a rollback that fails must not litter it with one file per output it could not take.
        let temp = TempDir::new().unwrap();
        let entry = publish(temp.path(), "0_line.wav", b"ours");
        let path = entry.path.clone();

        let outcome = super::remove_our_file(&entry.path, &|_| {
            let _ = std::fs::remove_file(&path);
            true
        });
        assert!(matches!(outcome, super::Removal::Absent));
        assert_eq!(
            std::fs::read_dir(temp.path()).unwrap().count(),
            0,
            "a rollback that took nothing back must leave nothing beside it"
        );
    }

    #[test]
    fn a_path_that_cannot_be_inspected_is_not_a_path_with_nothing_on_it() {
        // Every other check in this toolkit separates "not there" from "could not tell", and this
        // arm folded them together: an output directory whose permissions changed under the run
        // made `symlink_metadata` fail, that came back as `Absent`, and the caller went on to say
        // "Nothing was written" about a directory still holding the file. The next attempt then
        // collides on a path the message had just called clear.
        //
        // Provoked without an ACL, which this suite cannot set. The cause here is synthetic — a
        // path the operating system will not even accept — and the cause is not the point: what
        // this arm has to get right is every inspection error that is not "not there". A path
        // under a FILE was the first attempt and turned out to be `NotFound` on Windows, which is
        // why the fixture asserts its own premise below.
        let temp = TempDir::new().unwrap();
        let unreachable = temp.path().join("0_line.wav\u{0}");
        let error = std::fs::symlink_metadata(&unreachable).unwrap_err();
        assert_ne!(
            error.kind(),
            std::io::ErrorKind::NotFound,
            "the fixture is only interesting while the error is not 'not there': {error}"
        );

        let outcome = super::remove_our_file(&unreachable, &|_| false);
        match outcome {
            super::Removal::Failed(why) => assert!(why.contains("could not be inspected"), "{why}"),
            _ => panic!("a path nobody could look at must not be reported as free"),
        }

        // And the caller keeps it, so the sentence names the path instead of claiming the run
        // wrote nothing.
        let entry = super::Published {
            path: unreachable.clone(),
            written: 1,
            digest: super::digest_of(b"x"),
        };
        let kept = super::roll_back(std::slice::from_ref(&entry));
        assert!(kept.contains("are still there"), "{kept}");
        assert!(kept.contains("0_line.wav"), "{kept}");
    }

    #[test]
    fn a_file_that_disappears_before_the_move_is_absent_and_not_a_failure() {
        // The three endings that are not `Removed` are three different sentences, and folding them
        // together is what the last report was about. This one is reachable on demand: the file is
        // gone by the time the move runs, which is a path with nothing left on it — not a removal
        // that failed, and not somebody else's file.
        let temp = TempDir::new().unwrap();
        let entry = publish(temp.path(), "0_line.wav", b"ours");

        let path = entry.path.clone();
        let outcome = super::remove_our_file(&entry.path, &|_| {
            let _ = std::fs::remove_file(&path);
            true
        });
        assert!(matches!(outcome, super::Removal::Absent), "nothing is there to have failed on");
        assert_eq!(std::fs::read_dir(temp.path()).unwrap().count(), 0);
    }

    #[test]
    fn a_file_rewritten_with_the_very_same_bytes_is_still_ours() {
        // The other direction, and the reason content is the right identity rather than a stricter
        // stat: a file whose bytes are what this run wrote IS what this run wrote, whenever it was
        // written and by whom. Refusing to roll it back would leave output behind for a difference
        // nobody can observe.
        let temp = TempDir::new().unwrap();
        let entry = publish(temp.path(), "0_line.wav", b"original bytes");
        std::fs::write(&entry.path, b"original bytes").unwrap();
        assert!(entry.is_still_ours());

        // And a different length is rejected before the file is read at all.
        std::fs::write(&entry.path, b"a different length entirely").unwrap();
        assert!(!entry.is_still_ours());
    }

    #[test]
    fn conflicting_selectors_are_refused_before_anything_is_read_or_created() {
        // The bank path below does not exist, which is the assertion: reaching the read at all
        // would fail with a different error. A call that was never going to be honoured used to
        // decode 260 MB of `SFX.bank` first and leave an empty output directory behind.
        let temp = TempDir::new().unwrap();
        let out = temp.path().join("out");

        let error = super::extract(
            temp.path().join("no-such-bank.bank"),
            out.clone(),
            Some("SFX_UI_Click_0".into()),
            Some("click".into()),
            None,
        )
        .expect_err("two selectors cannot both be honoured");

        assert!(error.to_string().contains("cannot both be honoured"), "{error}");
        assert!(!out.exists(), "no output directory may be left behind");
    }

    #[test]
    fn a_file_that_is_no_longer_there_is_not_ours_either() {
        let temp = TempDir::new().unwrap();
        let entry = publish(temp.path(), "0_line.wav", b"bytes");
        std::fs::remove_file(&entry.path).unwrap();
        assert!(!entry.is_still_ours());
    }
}

#[cfg(test)]
mod banks_tests {
    use super::{bank_rows, banks_document, banks_table, BankRow};
    use gore_fmod::test_fixture::{numbered_pcm16_samples, pristine_bank_pcm16, sample_free_bank};
    use gore_fmod::GOTHIC_STUDIO_KEY;
    use std::path::Path;
    use tempfile::TempDir;

    fn write_sample_bank(dir: &Path, name: &str, samples: usize) {
        let samples = numbered_pcm16_samples("SFX_UI_Click_", samples, 44_100);
        let bank = pristine_bank_pcm16(&samples, GOTHIC_STUDIO_KEY).unwrap();
        std::fs::write(dir.join(name), bank).unwrap();
    }

    fn rows(dir: &Path) -> Vec<BankRow> {
        bank_rows(dir, GOTHIC_STUDIO_KEY).unwrap()
    }

    #[test]
    fn a_directory_named_like_a_bank_is_not_counted_as_one_and_not_hidden_either() {
        // The game cannot load a directory, so counting one put it in `bank_count`, in the "N
        // bank(s)" header and past the "holds no .bank files" check — a listing describing
        // something that is not in the directory it claims to describe. Dropping it silently
        // would be the same failure with the sign flipped: somebody looking for `Music.bank` in
        // a listing that never mentions it goes back to searching the filesystem.
        let temp = TempDir::new().unwrap();
        write_sample_bank(temp.path(), "SFX.bank", 3);
        std::fs::create_dir_all(temp.path().join("Music.bank")).unwrap();

        let rows = rows(temp.path());
        assert_eq!(rows.len(), 1, "only the file is a bank: {:?}", rows.len());
        let occupied = super::occupied_bank_names(temp.path());
        assert_eq!(occupied, vec!["Music.bank".to_string()]);

        let document = banks_document(temp.path(), &rows, &occupied);
        assert_eq!(document["bank_count"], 1, "the directory is not a bank");
        assert_eq!(document["occupied_names"][0], "Music.bank", "and it is not invisible");

        let table = banks_table(temp.path(), &rows, &occupied);
        assert!(table.contains("FMOD banks: 1 in"), "{table}");
        assert!(table.contains("Music.bank"), "{table}");
        assert!(table.contains("Remove or rename"), "{table}");

        // And where it is the ONLY thing there, the command says so instead of reporting an
        // install to verify — which is the wrong move for a directory somebody created.
        let temp = TempDir::new().unwrap();
        std::fs::create_dir_all(
            temp.path()
                .join("G1R")
                .join("Content")
                .join("FMOD")
                .join("Desktop")
                .join("Music.bank"),
        )
        .unwrap();
        let error = super::banks(Some(temp.path().to_path_buf()), false, None)
            .expect_err("a directory named like a bank is not a bank");
        let said = format!("{error}");
        assert!(said.contains("Music.bank"), "{said}");
        assert!(said.contains("remove or rename"), "{said}");
    }

    #[test]
    fn something_occupying_the_bank_directory_is_named_rather_than_called_absent() {
        // "No FMOD bank directory" sends the reader to re-point `--game` or verify the game files,
        // and neither of those can put a directory where something else is already sitting. The
        // path is fixed inside the install, so the only thing that helps is being told what is in
        // the way.
        let temp = TempDir::new().unwrap();
        let desktop = temp.path().join("G1R").join("Content").join("FMOD").join("Desktop");
        std::fs::create_dir_all(desktop.parent().unwrap()).unwrap();
        std::fs::write(&desktop, b"not a directory").unwrap();

        let error = super::banks(Some(temp.path().to_path_buf()), false, None)
            .expect_err("a file where the bank directory belongs is not a listing");
        let said = format!("{error}");
        assert!(said.contains("is not a directory"), "{said}");
        assert!(said.contains("remove or rename"), "{said}");
        assert!(!said.contains("verify the game files"), "the install is not the problem: {said}");

        // A dangling link resolves to nothing, so it reported the directory as absent and sent
        // the reader to verify game files — which cannot create it while the link holds the name.
        std::fs::remove_file(&desktop).unwrap();
        #[cfg(windows)]
        let linked = std::os::windows::fs::symlink_dir(temp.path().join("gone"), &desktop);
        #[cfg(unix)]
        let linked = std::os::unix::fs::symlink(temp.path().join("gone"), &desktop);
        if linked.is_ok() {
            let error = super::banks(Some(temp.path().to_path_buf()), false, None)
                .expect_err("a link that leads nowhere is not an empty install");
            let said = format!("{error}");
            assert!(said.contains("leads nowhere"), "{said}");
            assert!(said.contains("remove or rename"), "{said}");
            assert!(!said.contains("verify the game files"), "{said}");
            std::fs::remove_dir(&desktop).unwrap();
        }

        // The control: with the path genuinely absent the sentence and the remedy stand as they
        // were, so the branch is answering the obstruction and not every missing directory.
        let error = super::banks(Some(temp.path().to_path_buf()), false, None)
            .expect_err("and neither is a directory that is not there");
        let said = format!("{error}");
        assert!(said.contains("no FMOD bank directory"), "{said}");
        assert!(said.contains("verify the game files"), "{said}");
    }

    #[test]
    fn a_bank_that_carries_no_samples_is_a_row_saying_so_and_not_a_bank_left_out() {
        // The reported defect is that nothing describes the directory, and six of the ten files in
        // the real one carry no sample data. A listing that printed only the four that do would
        // still be a listing that does not describe the directory — a reader who could not find
        // `Master.bank` in it would go back to searching the filesystem, which is where they
        // started.
        let temp = TempDir::new().unwrap();
        write_sample_bank(temp.path(), "SFX.bank", 7);
        std::fs::write(temp.path().join("Master.bank"), sample_free_bank()).unwrap();

        let rows = rows(temp.path());
        assert_eq!(rows.len(), 2, "both files are banks, so both are rows");

        let table = banks_table(temp.path(), &rows, &[]);
        assert!(
            table.contains("Master.bank") && table.contains("no sample data"),
            "the sample-free bank must be present and explained, got {table:?}"
        );
        assert!(
            table.contains("2 in ") && table.contains("(1 carry samples, 7 samples in total)"),
            "the header must count the files and the samples separately, got {table:?}"
        );

        let document = banks_document(temp.path(), &rows, &[]);
        assert_eq!(document["bank_count"], 2);
        assert_eq!(document["with_samples_count"], 1);
        assert_eq!(document["sample_count"], 7);
        assert_eq!(document["banks"][0]["name"], "Master.bank");
        assert_eq!(document["banks"][0]["carries_samples"], false);
        assert_eq!(document["banks"][0]["sample_count"], 0);
        assert_eq!(document["banks"][0]["codec"], serde_json::Value::Null);
    }

    #[test]
    fn a_total_that_could_not_count_every_bank_says_so_where_the_number_is() {
        // `SFX.bank` carries almost every sample in the install, so one summary failing can move
        // this total by thousands. Printing what is left as "samples in total" states a number
        // that is not the total of anything, and a caller reading the JSON aggregate cannot see
        // the per-bank error without walking the list.
        let temp = TempDir::new().unwrap();
        write_sample_bank(temp.path(), "SFX.bank", 7);
        std::fs::write(temp.path().join("Broken.bank"), b"not a bank at all").unwrap();

        let rows = rows(temp.path());
        assert_eq!(rows.len(), 2);

        let table = banks_table(temp.path(), &rows, &[]);
        assert!(table.contains("partial count"), "{table:?}");

        let document = banks_document(temp.path(), &rows, &[]);
        assert_eq!(document["unreadable_count"], 1);
        assert_eq!(document["totals_complete"], false);
        assert_eq!(document["sample_count"], 7, "the readable bank still counts");
    }

    #[test]
    fn a_filter_that_matches_nothing_leaves_no_output_directory_behind() {
        // The selector conflict is refused before anything is read; this one cannot be, because
        // whether a filter matches is only known once the bank is decoded. What it can do is
        // create nothing until the selection is known to be non-empty — otherwise a call that
        // extracted nothing still left a directory for the caller to clean up.
        let temp = TempDir::new().unwrap();
        write_sample_bank(temp.path(), "SFX.bank", 3);
        let out = temp.path().join("out");

        let error = super::extract(
            temp.path().join("SFX.bank"),
            out.clone(),
            None,
            Some("no-sample-is-called-this".into()),
            None,
        )
        .expect_err("a filter matching nothing is an error");

        assert!(!out.exists(), "no output directory may be left behind: {error}");
    }

    #[test]
    fn every_row_carries_the_whole_path_because_that_is_what_bank_wants() {
        // The one thing this command exists to produce. A row naming only `SFX.bank` would leave
        // the reader building the path themselves out of a directory printed once at the top —
        // which is the manual step the command is here to remove.
        let temp = TempDir::new().unwrap();
        write_sample_bank(temp.path(), "SFX.bank", 3);

        let rows = rows(temp.path());
        let expected = temp.path().join("SFX.bank").display().to_string();

        assert!(
            banks_table(temp.path(), &rows, &[]).contains(&expected),
            "the table must print a pasteable path"
        );
        // `bank` is the key `list --json` uses for the same string, so a caller can take this
        // field and pass it straight back as `--bank` without renaming anything.
        assert_eq!(
            banks_document(temp.path(), &rows, &[])["banks"][0]["bank"],
            expected
        );
    }

    #[test]
    fn a_bank_that_cannot_be_read_is_a_row_with_its_reason_rather_than_an_aborted_listing() {
        // One damaged or foreign `.bank` in the directory must not cost the reader the other nine.
        // Failing the whole command would reproduce the original defect exactly: no listing, and a
        // filesystem search to find out which file was the problem.
        let temp = TempDir::new().unwrap();
        write_sample_bank(temp.path(), "SFX.bank", 3);
        std::fs::write(temp.path().join("Broken.bank"), b"not a bank at all").unwrap();

        let rows = rows(temp.path());
        assert_eq!(rows.len(), 2);

        let table = banks_table(temp.path(), &rows, &[]);
        assert!(
            table.contains("Broken.bank") && table.contains("could not be read:"),
            "the unreadable bank must be named together with why, got {table:?}"
        );
        assert!(
            table.contains("SFX.bank"),
            "the readable banks must still be listed"
        );

        let document = banks_document(temp.path(), &rows, &[]);
        let broken = &document["banks"][0];
        assert_eq!(broken["name"], "Broken.bank");
        assert!(broken["error"]
            .as_str()
            .is_some_and(|text| !text.is_empty()));
        // `false` here was an assertion nothing had measured: a consumer could not tell this from
        // a bank successfully inspected and found empty. Every field the summary would have filled
        // is null instead.
        for unknown in ["carries_samples", "sample_count", "codec", "sub_banks", "injected"] {
            assert_eq!(
                broken[unknown],
                serde_json::Value::Null,
                "{unknown} was never established for this file"
            );
        }
        assert_eq!(
            document["with_samples_count"], 1,
            "an unreadable bank counts as neither"
        );
    }

    #[test]
    fn an_injected_bank_is_marked_so_a_forgotten_replacement_does_not_read_as_a_broken_game() {
        // An injected bank looks entirely ordinary from the outside: same name, same sample names,
        // same counts. Someone returning to an install weeks later has no way to tell a deployed
        // replacement from a bug in the game, and `restore` is the answer to only one of those.
        let temp = TempDir::new().unwrap();
        write_sample_bank(temp.path(), "SFX.bank", 3);
        let pristine = std::fs::read(temp.path().join("SFX.bank")).unwrap();
        let injected = gore_fmod::replace_samples(
            &pristine,
            GOTHIC_STUDIO_KEY,
            vec![(
                "SFX_UI_Click_00".into(),
                gore_fmod::Pcm16Sample {
                    name: "tone".into(),
                    freq: 44_100,
                    channels: 1,
                    pcm: vec![0i16; 8],
                },
            )],
        )
        .unwrap();
        std::fs::write(temp.path().join("SFX.bank"), injected).unwrap();

        let rows = rows(temp.path());
        let table = banks_table(temp.path(), &rows, &[]);
        assert!(
            table.contains("[injected") && table.contains("gore audio restore"),
            "an injected bank must be marked and the way back named, got {table:?}"
        );

        let bank = &banks_document(temp.path(), &rows, &[])["banks"][0];
        assert_eq!(bank["injected"], true);
        assert_eq!(bank["sub_banks"], 2);
        // The count is still the shipped one. A replacement repoints a waveform, it never adds one,
        // so a listing that reported three samples here and four after a second deploy would be
        // describing the injection rather than the bank.
        assert_eq!(bank["sample_count"], 3);
    }

    #[test]
    fn only_bank_files_are_offered_and_the_toolkits_own_backups_are_not() {
        // `replace` leaves a `SFX.bank.gore-bak` next to the bank it edited. Offering that as a
        // `--bank` would let someone inject into a file the game never loads and then wonder why
        // nothing changed — and `restore` would afterwards overwrite the real bank with it.
        let temp = TempDir::new().unwrap();
        write_sample_bank(temp.path(), "SFX.bank", 3);
        std::fs::copy(
            temp.path().join("SFX.bank"),
            temp.path().join("SFX.bank.gore-bak"),
        )
        .unwrap();
        std::fs::write(temp.path().join("notes.txt"), b"unrelated").unwrap();

        let rows = rows(temp.path());
        assert_eq!(rows.len(), 1, "only the bank itself is a bank");
        assert_eq!(rows[0].name(), "SFX.bank");
    }

    #[test]
    fn the_rows_are_ordered_by_name_so_two_runs_of_one_directory_read_the_same() {
        // `read_dir` promises no order at all. A listing whose ten rows moved between runs would
        // make a before/after comparison — the ordinary way to check a deploy — unreadable.
        let temp = TempDir::new().unwrap();
        for name in ["VO.bank", "CINEMATICS.bank", "SFX.bank", "Music.bank"] {
            write_sample_bank(temp.path(), name, 1);
        }

        let names: Vec<String> = rows(temp.path())
            .iter()
            .map(|row| row.name().into_owned())
            .collect();
        assert_eq!(
            names,
            ["CINEMATICS.bank", "Music.bank", "SFX.bank", "VO.bank"]
        );
    }

    #[test]
    fn the_table_and_the_document_never_state_different_totals() {
        // They are two renderings of one answer, and a reader who switches to `--json` after
        // reading the table is entitled to find the same numbers there.
        let temp = TempDir::new().unwrap();
        write_sample_bank(temp.path(), "SFX.bank", 5);
        write_sample_bank(temp.path(), "Music.bank", 2);
        std::fs::write(temp.path().join("Master.bank"), sample_free_bank()).unwrap();

        let rows = rows(temp.path());
        let table = banks_table(temp.path(), &rows, &[]);
        let document = banks_document(temp.path(), &rows, &[]);

        assert!(
            table.contains("FMOD banks: 3 in ")
                && table.contains("(2 carry samples, 7 samples in total)"),
            "got {table:?}"
        );
        assert_eq!(document["bank_count"], 3);
        assert_eq!(document["with_samples_count"], 2);
        assert_eq!(document["sample_count"], 7);
        assert_eq!(document["directory"], temp.path().display().to_string());
    }
}
