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
    if !dir.is_dir() {
        bail!(
            "no FMOD bank directory at '{}'. That path is fixed inside a Gothic 1 Remake install, \
             so either --game (or the configured game path) points at something that is not one, \
             or this install is incomplete — verify the game files and try again.",
            dir.display()
        );
    }

    let rows = bank_rows(&dir, &key_bytes(key))?;
    if rows.is_empty() {
        bail!(
            "'{}' holds no .bank files. A Gothic 1 Remake install keeps ten there, so this is an \
             install to verify rather than a listing to read.",
            dir.display()
        );
    }

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&banks_document(&dir, &rows))?
        );
    } else {
        print!("{}", banks_table(&dir, &rows));
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
        .collect();
    paths.sort();

    Ok(paths
        .into_iter()
        .map(|path| {
            // Reading is the whole cost here: `bank_summary` decrypts 60 bytes per bank, so the
            // ten files cost one pass over ~520 MB of disk and nothing else.
            let summary = match std::fs::read(&path) {
                Ok(bytes) => gore_fmod::bank_summary(&bytes, key),
                Err(e) => Err(format!("{e}")),
            };
            BankRow { path, summary }
        })
        .collect())
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

fn banks_table(dir: &Path, rows: &[BankRow]) -> String {
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
    out
}

/// The same shape `list --json` uses: the path under `bank`, the codec spelled the way `Codec`'s
/// `Debug` spells it, and counts that answer their question without a reader subtracting anything.
fn banks_document(dir: &Path, rows: &[BankRow]) -> serde_json::Value {
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
    let bytes = std::fs::read(&bank).with_context(|| format!("reading '{}'", bank.display()))?;
    // Through the view, so a replaced sample is read out of the sub-bank it was repointed at.
    // Reading sub-bank 0 wrote the audio the replacement replaced into a file named after the
    // replacement — the one failure mode a caller cannot detect, because the file is there and
    // plays.
    let view = gore_fmod::read_bank(&bytes, &key_bytes(key))
        .map_err(|e| anyhow::anyhow!("{e}"))
        .context("decoding bank")?;
    std::fs::create_dir_all(&out).with_context(|| format!("creating '{}'", out.display()))?;

    // A named `--sample` and a `--filter` are two different selections, and honouring one means
    // ignoring the other. Silently keeping the sample meant a caller who passed both got a
    // successful extraction that answered half their request. `--sample all` is not a conflict:
    // it is the default, and it means "no sample selection", which is exactly what a filter narrows.
    if let (Some(name), Some(needle)) = (&sample, &filter) {
        if name != "all" {
            bail!(
                "--sample '{name}' and --filter '{needle}' select differently and cannot both be \
                 honoured. Pass one: --sample for a single known name, --filter for every name \
                 containing a substring."
            );
        }
    }

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
    let (mut ok, mut skipped) = (0usize, 0usize);
    let mut skips: BTreeMap<String, (usize, usize)> = BTreeMap::new();
    // Path plus the length written, so the rollback can tell its own output from a file that
    // something else put there since.
    let mut published: Vec<(PathBuf, u64)> = Vec::new();

    // Best effort by nature: the run has already failed, and a file that cannot be removed is not a
    // reason to fail differently.
    //
    // It removes only what still looks like what this run wrote. `create_new` stops the *creation*
    // from clobbering anything, but rollback happens later, and an editor or a watcher that
    // replaced an early WAV while a long extraction was still running would otherwise have its file
    // deleted by a failure it had nothing to do with. Length is the cheap identity available here;
    // a file replaced by different content of exactly the same length is not distinguished, and
    // that is the residual this note exists to record.
    let roll_back = |published: &[(PathBuf, u64)]| {
        for (path, written) in published {
            let ours = std::fs::metadata(path).map(|m| m.len() == *written).unwrap_or(false);
            if ours {
                let _ = std::fs::remove_file(path);
            }
        }
    };

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
                roll_back(&published);
                bail!(
                    "'{}' already exists; extract writes one file per sample under a name taken \
                     from the bank, so this would replace it. Nothing was written. Delete it, or \
                     pass a different --out.",
                    dest.display()
                );
            }
            Err(error) => {
                roll_back(&published);
                return Err(error).with_context(|| format!("creating '{}'", dest.display()));
            }
        };
        if let Err(error) = file.write_all(&wav) {
            drop(file);
            let _ = std::fs::remove_file(&dest);
            roll_back(&published);
            return Err(error).with_context(|| format!("writing '{}'", dest.display()));
        }
        published.push((dest, wav.len() as u64));
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

        let table = banks_table(temp.path(), &rows);
        assert!(
            table.contains("Master.bank") && table.contains("no sample data"),
            "the sample-free bank must be present and explained, got {table:?}"
        );
        assert!(
            table.contains("2 in ") && table.contains("(1 carry samples, 7 samples in total)"),
            "the header must count the files and the samples separately, got {table:?}"
        );

        let document = banks_document(temp.path(), &rows);
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

        let table = banks_table(temp.path(), &rows);
        assert!(table.contains("partial count"), "{table:?}");

        let document = banks_document(temp.path(), &rows);
        assert_eq!(document["unreadable_count"], 1);
        assert_eq!(document["totals_complete"], false);
        assert_eq!(document["sample_count"], 7, "the readable bank still counts");
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
            banks_table(temp.path(), &rows).contains(&expected),
            "the table must print a pasteable path"
        );
        // `bank` is the key `list --json` uses for the same string, so a caller can take this
        // field and pass it straight back as `--bank` without renaming anything.
        assert_eq!(
            banks_document(temp.path(), &rows)["banks"][0]["bank"],
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

        let table = banks_table(temp.path(), &rows);
        assert!(
            table.contains("Broken.bank") && table.contains("could not be read:"),
            "the unreadable bank must be named together with why, got {table:?}"
        );
        assert!(
            table.contains("SFX.bank"),
            "the readable banks must still be listed"
        );

        let document = banks_document(temp.path(), &rows);
        let broken = &document["banks"][0];
        assert_eq!(broken["name"], "Broken.bank");
        assert!(broken["error"]
            .as_str()
            .is_some_and(|text| !text.is_empty()));
        assert_eq!(broken["carries_samples"], false);
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
        let table = banks_table(temp.path(), &rows);
        assert!(
            table.contains("[injected") && table.contains("gore audio restore"),
            "an injected bank must be marked and the way back named, got {table:?}"
        );

        let bank = &banks_document(temp.path(), &rows)["banks"][0];
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
        let table = banks_table(temp.path(), &rows);
        let document = banks_document(temp.path(), &rows);

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
