//! `gore voice` -- safe, copy-on-write tools for the game's voice-over ZIPs.

use std::collections::BTreeMap;
use std::fs;
use std::io::Read;
use std::path::{Component, Path, PathBuf};

use anyhow::{bail, Context, Result};
use clap::{Args, Subcommand};
use gore_vo::{ArchiveEdit, ArchiveIndex, EditAction, Limits, OggCodec, OggInfo, WriteReport};
use serde::Deserialize;

const VOICE_MANIFEST_FORMAT: u32 = 1;
const MAX_MANIFEST_BYTES: u64 = 16 * 1024 * 1024;
const MAX_MANIFEST_OGG_BYTES: u64 = 256 * 1024 * 1024;

#[derive(Debug, Subcommand)]
pub enum VoiceAction {
    /// Index and list every entry in a voice archive
    #[command(visible_alias = "index")]
    List {
        /// Input voice ZIP
        #[arg(long)]
        archive: PathBuf,
        /// Emit one JSON document instead of the human-readable table
        #[arg(long)]
        json: bool,
    },
    /// Extract one entry without overwriting an existing file
    Extract {
        /// Input voice ZIP
        #[arg(long)]
        archive: PathBuf,
        #[command(flatten)]
        selector: VoiceSelector,
        /// Extraction root; the archive path is preserved below it
        #[arg(short = 'o', long)]
        out: PathBuf,
    },
    /// Append a validated Ogg file to a new archive
    Add {
        /// Input voice ZIP (never modified)
        #[arg(long)]
        archive: PathBuf,
        /// Full path for the new entry inside the archive
        #[arg(long, value_name = "ARCHIVE_PATH")]
        path: String,
        /// Ogg/Vorbis or Ogg/Opus file to add
        #[arg(long)]
        ogg: PathBuf,
        /// New output ZIP; must not already exist
        #[arg(short = 'o', long)]
        out: PathBuf,
    },
    /// Replace one entry with a validated Ogg file in a new archive
    Replace {
        /// Input voice ZIP (never modified)
        #[arg(long)]
        archive: PathBuf,
        #[command(flatten)]
        selector: VoiceSelector,
        /// Ogg/Vorbis or Ogg/Opus replacement file
        #[arg(long)]
        ogg: PathBuf,
        /// New output ZIP; must not already exist
        #[arg(short = 'o', long)]
        out: PathBuf,
    },
    /// Apply a versioned JSON edit manifest to a new archive in one pass
    #[command(visible_alias = "apply")]
    ApplyManifest {
        /// Input voice ZIP (never modified)
        #[arg(long)]
        archive: PathBuf,
        /// Versioned JSON manifest; Ogg paths are relative to this file
        #[arg(long)]
        manifest: PathBuf,
        /// New output ZIP; must not already exist
        #[arg(short = 'o', long)]
        out: PathBuf,
    },
}

#[derive(Debug, Args)]
pub struct VoiceSelector {
    /// Case-insensitive basename; accepted only when it identifies one entry
    #[arg(
        long,
        value_name = "NAME",
        required_unless_present = "path",
        conflicts_with = "path"
    )]
    pub basename: Option<String>,
    /// Case-sensitive complete archive path (use this to disambiguate basenames)
    #[arg(
        long,
        value_name = "ARCHIVE_PATH",
        required_unless_present = "basename",
        conflicts_with = "basename"
    )]
    pub path: Option<String>,
}

impl VoiceSelector {
    fn for_archive(&self) -> (&str, Option<&str>) {
        match (&self.basename, &self.path) {
            (Some(basename), None) => (basename, None),
            (None, Some(path)) => ("", Some(path)),
            _ => unreachable!("clap requires exactly one voice selector"),
        }
    }
}

pub fn run(action: VoiceAction) -> Result<()> {
    match action {
        VoiceAction::List { archive, json } => list(&archive, json),
        VoiceAction::Extract {
            archive,
            selector,
            out,
        } => extract(&archive, &selector, &out),
        VoiceAction::Add {
            archive,
            path,
            ogg,
            out,
        } => add(&archive, &path, &ogg, &out),
        VoiceAction::Replace {
            archive,
            selector,
            ogg,
            out,
        } => replace(&archive, &selector, &ogg, &out),
        VoiceAction::ApplyManifest {
            archive,
            manifest,
            out,
        } => apply_manifest(&archive, &manifest, &out),
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct VoiceManifest {
    format: u32,
    edits: Vec<VoiceManifestEdit>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "op", rename_all = "lowercase", deny_unknown_fields)]
enum VoiceManifestEdit {
    Add { path: String, ogg: String },
    Replace { path: String, ogg: String },
}

impl VoiceManifestEdit {
    fn archive_path(&self) -> &str {
        match self {
            Self::Add { path, .. } | Self::Replace { path, .. } => path,
        }
    }

    fn ogg_path(&self) -> &str {
        match self {
            Self::Add { ogg, .. } | Self::Replace { ogg, .. } => ogg,
        }
    }
}

#[derive(Debug)]
struct LoadedManifestEdit {
    operation: LoadedOperation,
    archive_path: String,
    ogg: Vec<u8>,
}

#[derive(Debug, Clone, Copy)]
enum LoadedOperation {
    Add,
    Replace,
}

fn open_archive(path: &Path) -> Result<ArchiveIndex> {
    ArchiveIndex::open(path, Limits::default())
        .with_context(|| format!("indexing voice archive '{}'", path.display()))
}

fn list(path: &Path, json: bool) -> Result<()> {
    let archive = open_archive(path)?;
    if json {
        let entries = archive
            .list()
            .map(|entry| {
                serde_json::json!({
                    "index": entry.index,
                    "path": entry.path,
                    "basename": entry.basename,
                    "compressed_size": entry.compressed_size,
                    "uncompressed_size": entry.uncompressed_size,
                    "crc32": entry.crc32,
                    "compression": format!("{:?}", entry.compression),
                    "is_directory": entry.is_directory,
                    "is_symlink": entry.is_symlink,
                    "encrypted": entry.encrypted,
                })
            })
            .collect::<Vec<_>>();
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "archive": archive.path().display().to_string(),
                "entry_count": entries.len(),
                "entries": entries,
            }))?
        );
        return Ok(());
    }

    println!(
        "Voice archive: {} ({} entries)",
        archive.path().display(),
        archive.entries().len()
    );
    println!(" INDEX        SIZE       PACKED  METHOD      CRC32     PATH");
    for entry in archive.list() {
        let flags = match (entry.is_directory, entry.is_symlink, entry.encrypted) {
            (true, _, _) => " [directory]",
            (_, true, _) => " [symlink]",
            (_, _, true) => " [encrypted]",
            _ => "",
        };
        println!(
            "{:>6} {:>11} {:>11}  {:<10}  {:08x}  {}{}",
            entry.index,
            entry.uncompressed_size,
            entry.compressed_size,
            format!("{:?}", entry.compression),
            entry.crc32,
            entry.path,
            flags
        );
    }
    Ok(())
}

fn extract(archive_path: &Path, selector: &VoiceSelector, output_root: &Path) -> Result<()> {
    let archive = open_archive(archive_path)?;
    let (basename, exact_path) = selector.for_archive();
    let output = archive
        .extract(basename, exact_path, output_root)
        .context("extracting selected voice entry")?;
    println!("Extracted {}", output.display());
    Ok(())
}

fn add(archive_path: &Path, entry_path: &str, ogg_path: &Path, output: &Path) -> Result<()> {
    let archive = open_archive(archive_path)?;
    let ogg = read_ogg(ogg_path)?;
    let report = archive
        .write_edited(
            output,
            ArchiveEdit::Add {
                path: entry_path,
                ogg: &ogg,
            },
        )
        .context("adding voice entry")?;
    print_report(&report);
    Ok(())
}

fn replace(
    archive_path: &Path,
    selector: &VoiceSelector,
    ogg_path: &Path,
    output: &Path,
) -> Result<()> {
    let archive = open_archive(archive_path)?;
    let ogg = read_ogg(ogg_path)?;
    let (basename, exact_path) = selector.for_archive();
    let report = archive
        .write_edited(
            output,
            ArchiveEdit::Replace {
                basename,
                exact_path,
                ogg: &ogg,
            },
        )
        .context("replacing voice entry")?;
    print_report(&report);
    Ok(())
}

fn apply_manifest(archive_path: &Path, manifest_path: &Path, output: &Path) -> Result<()> {
    // Load and validate every external input before opening the source archive or asking
    // gore-vo to create its verified temporary output. The byte buffers also sever any
    // dependency on the source paths after this point.
    let limits = Limits::default();
    let edits = load_manifest_edits(manifest_path, &limits)?;
    let archive = open_archive(archive_path)?;
    let archive_edits = edits.iter().map(|edit| match edit.operation {
        LoadedOperation::Add => ArchiveEdit::Add {
            path: &edit.archive_path,
            ogg: &edit.ogg,
        },
        LoadedOperation::Replace => ArchiveEdit::Replace {
            basename: "",
            exact_path: Some(&edit.archive_path),
            ogg: &edit.ogg,
        },
    });
    let reports = archive
        .write_edits(output, archive_edits)
        .with_context(|| format!("applying voice manifest '{}'", manifest_path.display()))?;

    println!(
        "Applied {} voice edit(s) in one pass -> {}",
        reports.len(),
        output.display()
    );
    for report in &reports {
        print_report(report);
    }
    Ok(())
}

fn load_manifest_edits(path: &Path, limits: &Limits) -> Result<Vec<LoadedManifestEdit>> {
    load_manifest_edits_with_budget(path, limits, MAX_MANIFEST_OGG_BYTES)
}

fn load_manifest_edits_with_budget(
    path: &Path,
    limits: &Limits,
    max_total_ogg_bytes: u64,
) -> Result<Vec<LoadedManifestEdit>> {
    let bytes = read_file_bounded(path, "voice manifest", MAX_MANIFEST_BYTES)?;
    let manifest: VoiceManifest = serde_json::from_slice(&bytes)
        .with_context(|| format!("parsing voice manifest '{}'", path.display()))?;
    if manifest.format != VOICE_MANIFEST_FORMAT {
        bail!(
            "unsupported voice manifest format {} (expected {VOICE_MANIFEST_FORMAT})",
            manifest.format
        );
    }
    if manifest.edits.is_empty() {
        bail!("voice manifest contains no edits");
    }
    if manifest.edits.len() > limits.max_entries {
        bail!(
            "voice manifest contains {} edits (limit: {})",
            manifest.edits.len(),
            limits.max_entries
        );
    }

    // Reject conflicts before touching any referenced Ogg file. gore-vo performs the same
    // defense again, but surfacing manifest indices makes distributable manifests easier to fix.
    let mut targets = BTreeMap::<String, (usize, String)>::new();
    for (index, edit) in manifest.edits.iter().enumerate() {
        let archive_path = edit.archive_path();
        if archive_path.is_empty() {
            bail!(
                "voice manifest edit #{} has an empty archive path",
                index + 1
            );
        }
        let key = archive_path.replace('\\', "/").to_lowercase();
        if let Some((first_index, first_path)) =
            targets.insert(key, (index + 1, archive_path.to_owned()))
        {
            bail!(
                "voice manifest edits #{first_index} ({first_path:?}) and #{} ({archive_path:?}) target the same case-insensitive archive path",
                index + 1
            );
        }
    }

    let manifest_dir = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let root = fs::canonicalize(manifest_dir).with_context(|| {
        format!(
            "resolving voice manifest directory '{}'",
            manifest_dir.display()
        )
    })?;
    let mut loaded = Vec::with_capacity(manifest.edits.len());
    let mut total_ogg_bytes = 0u64;
    for (index, edit) in manifest.edits.into_iter().enumerate() {
        let relative_text = edit.ogg_path().to_owned();
        let relative = normalized_relative_manifest_path(&relative_text).with_context(|| {
            format!(
                "voice manifest edit #{} has invalid Ogg path {relative_text:?}",
                index + 1
            )
        })?;
        let remaining = max_total_ogg_bytes
            .checked_sub(total_ogg_bytes)
            .ok_or_else(|| anyhow::anyhow!("voice manifest Ogg byte budget underflowed"))?;
        let ogg = read_manifest_ogg(
            &root,
            &relative,
            &relative_text,
            limits,
            (limits.max_ogg_bytes as u64).min(remaining),
        )
        .with_context(|| format!("loading Ogg for voice manifest edit #{}", index + 1))?;
        total_ogg_bytes = total_ogg_bytes
            .checked_add(ogg.len() as u64)
            .ok_or_else(|| anyhow::anyhow!("voice manifest Ogg byte total overflowed"))?;
        if total_ogg_bytes > max_total_ogg_bytes {
            bail!(
                "voice manifest Ogg payloads total {total_ogg_bytes} bytes (limit: {})",
                max_total_ogg_bytes
            );
        }

        let (operation, archive_path) = match edit {
            VoiceManifestEdit::Add { path, .. } => (LoadedOperation::Add, path),
            VoiceManifestEdit::Replace { path, .. } => (LoadedOperation::Replace, path),
        };
        loaded.push(LoadedManifestEdit {
            operation,
            archive_path,
            ogg,
        });
    }
    Ok(loaded)
}

/// Parse a portable normalized relative path. Manifests deliberately use `/` on every OS,
/// which prevents the same JSON from resolving differently on Windows and Unix.
fn normalized_relative_manifest_path(value: &str) -> Result<PathBuf> {
    if value.is_empty() {
        bail!("path is empty");
    }
    if value.contains('\\') {
        bail!("path must use '/' separators");
    }
    let path = Path::new(value);
    if path.is_absolute()
        || path
            .components()
            .any(|part| matches!(part, Component::Prefix(_) | Component::RootDir))
    {
        bail!("path must be relative to the manifest");
    }

    let mut normalized = PathBuf::new();
    for (index, component) in value.split('/').enumerate() {
        if component.is_empty() || component == "." || component == ".." {
            bail!("path is not normalized (empty, '.' and '..' components are forbidden)");
        }
        if component.chars().any(char::is_control) {
            bail!("path contains a control character");
        }
        if index == 0
            && component.len() == 2
            && component.as_bytes()[0].is_ascii_alphabetic()
            && component.as_bytes()[1] == b':'
        {
            bail!("path must not contain a Windows drive prefix");
        }
        normalized.push(component);
    }
    Ok(normalized)
}

fn read_manifest_ogg(
    root: &Path,
    relative: &Path,
    manifest_value: &str,
    limits: &Limits,
    max_bytes: u64,
) -> Result<Vec<u8>> {
    let mut candidate = root.to_path_buf();
    for component in relative.components() {
        candidate.push(component.as_os_str());
        let metadata = fs::symlink_metadata(&candidate).with_context(|| {
            format!("reading metadata for manifest-relative path {manifest_value:?}")
        })?;
        if metadata_is_link(&metadata) {
            bail!(
                "manifest-relative Ogg path {manifest_value:?} contains a symbolic link or reparse point"
            );
        }
    }

    let canonical = fs::canonicalize(&candidate)
        .with_context(|| format!("resolving manifest-relative Ogg path {manifest_value:?}"))?;
    if !canonical.starts_with(root) {
        bail!(
            "manifest-relative Ogg path {manifest_value:?} resolves outside the manifest directory"
        );
    }
    let ogg = read_file_bounded(
        &canonical,
        &format!("manifest-relative Ogg file {manifest_value:?}"),
        max_bytes,
    )?;
    gore_vo::validate_ogg(&ogg, limits)
        .with_context(|| format!("validating manifest-relative Ogg file {manifest_value:?}"))?;
    Ok(ogg)
}

fn metadata_is_link(metadata: &fs::Metadata) -> bool {
    if metadata.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt as _;

        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
        if metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            return true;
        }
    }
    false
}

fn read_ogg(path: &Path) -> Result<Vec<u8>> {
    read_ogg_with_limit(path, Limits::default().max_ogg_bytes)
}

fn read_ogg_with_limit(path: &Path, max_bytes: usize) -> Result<Vec<u8>> {
    read_file_bounded(path, "Ogg file", max_bytes as u64)
}

fn read_file_bounded(path: &Path, label: &str, max_bytes: u64) -> Result<Vec<u8>> {
    let file =
        fs::File::open(path).with_context(|| format!("opening {label} '{}'", path.display()))?;
    let metadata = file
        .metadata()
        .with_context(|| format!("reading {label} metadata '{}'", path.display()))?;
    if !metadata.is_file() {
        bail!("{label} path is not a regular file: '{}'", path.display());
    }
    if metadata.len() > max_bytes {
        bail!(
            "{label} '{}' is too large: {} bytes (limit: {max_bytes})",
            path.display(),
            metadata.len()
        );
    }

    // Metadata avoids a known-oversize allocation. `take(max + 1)` is the second line of defense
    // if the file grows after metadata or a producer keeps writing through another handle.
    let platform_limit = usize::try_from(max_bytes).unwrap_or(usize::MAX);
    let capacity = usize::try_from(metadata.len()).unwrap_or(platform_limit);
    let mut bytes = Vec::with_capacity(capacity.min(platform_limit));
    let read_limit = max_bytes.saturating_add(1);
    file.take(read_limit)
        .read_to_end(&mut bytes)
        .with_context(|| format!("reading {label} '{}'", path.display()))?;
    if bytes.len() as u64 > max_bytes {
        bail!(
            "{label} '{}' grew beyond the {max_bytes}-byte limit while it was read",
            path.display()
        );
    }
    Ok(bytes)
}

fn print_report(report: &WriteReport) {
    let action = match report.action {
        EditAction::Added => "Added",
        EditAction::Replaced => "Replaced",
    };
    let sha256 = report
        .sha256
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    println!(
        "{action} {} at index {} -> {}",
        report.archive_path,
        report.entry_index,
        report.output.display()
    );
    println!("  Ogg: {}", describe_ogg(&report.ogg));
    println!("  SHA-256: {sha256}");
}

fn describe_ogg(info: &OggInfo) -> String {
    let codec = match &info.codec {
        OggCodec::Vorbis {
            channels,
            sample_rate,
        } => format!("Vorbis, {channels} channel(s), {sample_rate} Hz"),
        OggCodec::Opus {
            channels,
            input_sample_rate,
        } => format!("Opus, {channels} channel(s), input {input_sample_rate} Hz"),
        OggCodec::Unknown => "unknown codec".to_owned(),
    };
    format!(
        "{codec}, {} page(s), {} logical stream(s)",
        info.pages, info.logical_streams
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_ogg(sample_rate: u32) -> Vec<u8> {
        let mut packet = Vec::new();
        packet.extend_from_slice(b"\x01vorbis");
        packet.extend_from_slice(&0u32.to_le_bytes());
        packet.push(1);
        packet.extend_from_slice(&sample_rate.to_le_bytes());
        packet.extend_from_slice(&0i32.to_le_bytes());
        packet.extend_from_slice(&0i32.to_le_bytes());
        packet.extend_from_slice(&0i32.to_le_bytes());
        packet.push(0x86);
        packet.push(1);

        let mut page = Vec::new();
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
        let mut crc = 0u32;
        for &byte in &page {
            crc ^= u32::from(byte) << 24;
            for _ in 0..8 {
                crc = if crc & 0x8000_0000 != 0 {
                    (crc << 1) ^ 0x04c1_1db7
                } else {
                    crc << 1
                };
            }
        }
        page[22..26].copy_from_slice(&crc.to_le_bytes());
        page
    }

    #[test]
    fn direct_ogg_read_rejects_from_metadata_before_allocating_past_limit() {
        let dir = tempfile::tempdir().unwrap();
        let oversized = dir.path().join("oversized.ogg");
        fs::File::create(&oversized).unwrap().set_len(5).unwrap();

        let error = read_ogg_with_limit(&oversized, 4).unwrap_err();
        assert!(
            error.to_string().contains("too large") && error.to_string().contains("limit: 4"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn direct_ogg_read_accepts_exact_limit_without_extra_allocation() {
        let dir = tempfile::tempdir().unwrap();
        let exact = dir.path().join("exact.ogg");
        fs::write(&exact, b"OggS").unwrap();
        assert_eq!(read_ogg_with_limit(&exact, 4).unwrap(), b"OggS");
    }

    #[test]
    fn manifest_ogg_budget_is_reserved_before_reading_the_next_payload() {
        let dir = tempfile::tempdir().unwrap();
        let first = test_ogg(22_050);
        let second = test_ogg(44_100);
        fs::write(dir.path().join("first.ogg"), &first).unwrap();
        fs::write(dir.path().join("second.ogg"), &second).unwrap();
        let manifest = dir.path().join("manifest.json");
        fs::write(
            &manifest,
            serde_json::to_vec(&serde_json::json!({
                "format": 1,
                "edits": [
                    {"op": "add", "path": "Added/First.ogg", "ogg": "first.ogg"},
                    {"op": "add", "path": "Added/Second.ogg", "ogg": "second.ogg"}
                ]
            }))
            .unwrap(),
        )
        .unwrap();

        let error =
            load_manifest_edits_with_budget(&manifest, &Limits::default(), first.len() as u64 + 1)
                .unwrap_err();
        let chain = format!("{error:#}");
        assert!(
            chain.contains("voice manifest edit #2")
                && chain.contains("too large")
                && chain.contains("limit: 1"),
            "unexpected error: {chain}"
        );
    }
}
