//! `gore audio` — read and replace audio in the game's encrypted FMOD sound banks
//! (pure Rust, no FMOD). The banks are loose at `…/G1R/Content/FMOD/Desktop/*.bank`.
//!
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

pub fn list(bank: PathBuf, key: Option<String>) -> Result<()> {
    let bytes = std::fs::read(&bank).with_context(|| format!("reading '{}'", bank.display()))?;
    let f0 = gore_fmod::bank_fsb0(&bytes, &key_bytes(key))
        .map_err(|e| anyhow::anyhow!("{e}"))
        .context("decoding bank")?;
    println!("{} samples, codec {:?}", f0.samples.len(), f0.codec);
    for (i, s) in f0.samples.iter().enumerate() {
        let secs = if s.freq > 0 { s.num_samples as f64 / s.freq as f64 } else { 0.0 };
        println!("#{i:<5} {:6}Hz {}ch {:6.2}s  {}", s.freq, s.channels, secs, s.name);
    }
    Ok(())
}

pub fn extract(bank: PathBuf, out: PathBuf, sample: Option<String>, key: Option<String>) -> Result<()> {
    let bytes = std::fs::read(&bank).with_context(|| format!("reading '{}'", bank.display()))?;
    let (block, fsb) = gore_fmod::decrypt_fsb0(&bytes, &key_bytes(key))
        .map_err(|e| anyhow::anyhow!("{e}"))
        .context("decoding bank")?;
    std::fs::create_dir_all(&out).with_context(|| format!("creating '{}'", out.display()))?;

    // indices to extract
    let indices: Vec<usize> = match &sample {
        Some(name) if name != "all" => vec![fsb
            .samples
            .iter()
            .position(|s| &s.name == name)
            .with_context(|| format!("sample not found: {name}"))?],
        _ => (0..fsb.samples.len()).collect(),
    };

    let (mut ok, mut skipped) = (0usize, 0usize);
    for i in indices {
        match gore_fmod::extract_wav(&block, &fsb, i) {
            Ok(wav) => {
                // Prefix with the sample index so two names that sanitize to the same basename
                // (e.g. differing only by punctuation) don't collide and silently overwrite.
                let path = out.join(format!("{i}_{}.wav", sanitize(&fsb.samples[i].name)));
                std::fs::write(&path, &wav)
                    .with_context(|| format!("writing '{}'", path.display()))?;
                ok += 1;
            }
            Err(e) => {
                skipped += 1;
                eprintln!("skip #{i} {}: {e}", fsb.samples[i].name);
            }
        }
    }
    println!("extracted {ok} wav file(s) to {} ({skipped} skipped)", out.display());
    Ok(())
}

fn sanitize(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.' { c } else { '_' })
        .collect()
}

pub fn replace(map: PathBuf, bank: PathBuf, out: Option<PathBuf>, key: Option<String>) -> Result<()> {
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
    let base = map.parent().map(Path::to_path_buf).unwrap_or_else(|| PathBuf::from("."));
    let mut replacements = Vec::with_capacity(entries.len());
    for (name, wav_rel) in &entries {
        let wav_path = resolve(&base, wav_rel);
        let wav = std::fs::read(&wav_path)
            .with_context(|| format!("reading wav '{}'", wav_path.display()))?;
        let (rate, channels, pcm) = gore_fmod::read_wav_pcm16(&wav)
            .map_err(|e| anyhow::anyhow!("{wav_rel}: {e}"))?;
        replacements.push((
            name.clone(),
            gore_fmod::Pcm16Sample { name: name.clone(), freq: rate, channels, pcm },
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
            let live = std::fs::read(bank).with_context(|| format!("reading '{}'", bank.display()))?;
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
    let base = map.parent().map(Path::to_path_buf).unwrap_or_else(|| PathBuf::from("."));

    let file = std::fs::File::create(&out).with_context(|| format!("creating '{}'", out.display()))?;
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
    zip.start_file("manifest.json", opts).context("zip manifest")?;
    zip.write_all(&serde_json::to_vec_pretty(&manifest)?)?;
    zip.finish().context("finishing zip")?;
    println!("wrote patch {} ({} sample(s))", out.display(), entries.len());
    Ok(())
}

/// Apply a patch zip (from `export-patch`) to a bank.
pub fn apply_patch(patch: PathBuf, bank: PathBuf, out: Option<PathBuf>, key: Option<String>) -> Result<()> {
    let key = key_bytes(key);
    let bytes = read_pristine_bank(&bank)?;

    let file = std::fs::File::open(&patch).with_context(|| format!("opening '{}'", patch.display()))?;
    let mut zip = zip::ZipArchive::new(file).context("reading patch zip")?;

    let manifest: BTreeMap<String, String> = {
        let mut f = zip.by_name("manifest.json").context("patch missing manifest.json")?;
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
        let (rate, channels, pcm) = gore_fmod::read_wav_pcm16(&wav)
            .map_err(|e| anyhow::anyhow!("{name}: {e}"))?;
        replacements.push((
            name.clone(),
            gore_fmod::Pcm16Sample { name: name.clone(), freq: rate, channels, pcm },
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
