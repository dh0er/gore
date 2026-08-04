//! `gore voice` -- safe, copy-on-write tools for the game's voice-over ZIPs.

use std::collections::BTreeMap;
use std::fs;
use std::io::Read;
use std::path::{Component, Path, PathBuf};

use anyhow::{bail, Context, Result};
use clap::{Args, Subcommand};
use gore_vo::{
    validate_archive_entry_path, ArchiveEdit, ArchiveEntry, ArchiveIndex, EditAction, Limits,
    OggCodec, OggInfo, WriteReport,
};
use serde::{Deserialize, Serialize};

const VOICE_MANIFEST_FORMAT: u32 = 1;
const MAX_MANIFEST_BYTES: u64 = 16 * 1024 * 1024;
const MAX_MANIFEST_OGG_BYTES: u64 = 256 * 1024 * 1024;
const MAX_MATCH_ARCHIVE_ENTRIES: usize = 50_000;
const MAX_MATCH_CENTRAL_DIRECTORY_BYTES: u64 = 64 * 1024 * 1024;
const MAX_MATCH_ENTRY_PATH_BYTES: usize = 1024;
const MAX_MATCH_ENTRY_OUTPUT_BYTES: u64 = 256 * 1024 * 1024;
const MAX_MATCH_TOTAL_UNCOMPRESSED_BYTES: u64 = 16 * 1024 * 1024 * 1024;
const MAX_MATCH_JSON_BYTES: usize = 1024 * 1024;
const MAX_LOC_ID_BYTES: usize = 512;

#[derive(Debug, Subcommand)]
pub enum VoiceAction {
    /// Index a voice archive and list a bounded page of its entries
    #[command(visible_alias = "index")]
    List {
        /// Input voice ZIP
        #[arg(long)]
        archive: PathBuf,
        /// Keep only entry paths containing this substring (case-insensitive)
        #[arg(long)]
        filter: Option<String>,
        /// Max entries to print. The result states how many matched when it stops here; 0 lists
        /// nothing and reports only the counts
        #[arg(long, default_value_t = 100)]
        max: usize,
        /// Also list the archive's directory entries, which carry no audio
        #[arg(long)]
        directories: bool,
        /// Emit one JSON document instead of the human-readable table
        #[arg(long)]
        json: bool,
    },
    /// Resolve an exact `${loc_id}.ogg` basename without extracting it
    MatchLine {
        /// Input voice ZIP
        #[arg(long)]
        archive: PathBuf,
        /// Trimmed ASCII localization ID (without the `.ogg` suffix)
        #[arg(long, value_name = "ASCII_ID")]
        loc_id: String,
        /// Emit one JSON document instead of human-readable output
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
        VoiceAction::List {
            archive,
            filter,
            max,
            directories,
            json,
        } => list(&archive, filter.as_deref(), max, directories, json),
        VoiceAction::MatchLine {
            archive,
            loc_id,
            json,
        } => match_line(&archive, &loc_id, json),
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
enum VoiceLineResolution {
    Unresolved,
    Unique,
    Ambiguous,
}

impl VoiceLineResolution {
    fn as_str(self) -> &'static str {
        match self {
            Self::Unresolved => "unresolved",
            Self::Unique => "unique",
            Self::Ambiguous => "ambiguous",
        }
    }
}

#[derive(Debug, PartialEq, Eq, Serialize)]
struct VoiceLineMember {
    path: String,
    compressed_size: u64,
    uncompressed_size: u64,
    crc32: u32,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
struct VoiceLineMatch {
    archive: String,
    archive_size: u64,
    archive_sha256: String,
    loc_id: String,
    expected_basename: String,
    resolution: VoiceLineResolution,
    match_count: usize,
    matches: Vec<VoiceLineMember>,
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

/// List archive entries under a bound. `ArchiveIndex::list()` stays complete -- every writer path
/// depends on that -- so the narrowing happens here, where it is a presentation decision. A
/// listing that stopped silently would let a caller read the first `max` entries as the whole
/// archive and conclude a recording does not exist, so both output modes label the cut.
fn list(
    path: &Path,
    filter: Option<&str>,
    max: usize,
    directories: bool,
    json: bool,
) -> Result<()> {
    let archive = open_archive(path)?;
    let entry_count = archive.entries().len();
    // Filter first, cap second: `matched_count` is only meaningful if the cap never hides a
    // candidate the filter would have kept.
    let needle = filter.map(str::to_lowercase);
    let selected = archive
        .list()
        .filter(|entry| {
            needle
                .as_deref()
                .is_none_or(|needle| contains_case_insensitive(&entry.path, needle))
        })
        .collect::<Vec<_>>();
    // Counted after the filter, because the only job this number has is to name records the caller
    // can still get back. Counting the whole archive made `--filter DIA_` advertise `--directories`
    // for a `NPC/` record the filter rejects too, so following the advice changed nothing.
    let directory_count = selected.iter().filter(|entry| entry.is_directory).count();
    let matched = if directories {
        selected
    } else {
        selected
            .into_iter()
            .filter(|entry| !entry.is_directory)
            .collect()
    };
    let listed = &matched[..matched.len().min(max)];
    let notice = (listed.len() < matched.len())
        .then(|| list_truncation_notice(matched.len(), listed.len()));

    if json {
        let entries = listed
            .iter()
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
        // Two booleans because there are two questions and one answer cannot serve both.
        // `truncated` says whether `--max` stopped the listing, and it is what `truncation_notice`
        // belongs to. "Is this array the whole archive" is a different question -- a filter or a
        // dropped directory record narrows without truncating -- and `complete` answers it, so
        // neither has to be inferred by comparing counts.
        let mut document = serde_json::json!({
            "archive": archive.path().display().to_string(),
            "entry_count": entry_count,
            "directory_count": directory_count,
            "matched_count": matched.len(),
            "listed_count": entries.len(),
            "truncated": notice.is_some(),
            "complete": entries.len() == entry_count,
            "entries": entries,
        });
        if let Some(notice) = &notice {
            document["truncation_notice"] = serde_json::json!(notice);
        }
        println!("{}", serde_json::to_string_pretty(&document)?);
        return Ok(());
    }

    // Without this clause a filter that matched nothing prints a header, a column header and no
    // rows -- which is exactly what an empty archive prints.
    let narrowed = match filter {
        Some(_) => format!(", {} matched --filter", matched.len()),
        None => String::new(),
    };
    let omitted = if directories || directory_count == 0 {
        String::new()
    } else {
        let record = if directory_count == 1 {
            "directory record"
        } else {
            "directory records"
        };
        format!(", {directory_count} {record} omitted — pass --directories to include them")
    };
    println!(
        "Voice archive: {} ({entry_count} entries{narrowed}{omitted})",
        archive.path().display()
    );
    println!(" INDEX        SIZE       PACKED  METHOD      CRC32     PATH");
    for entry in listed {
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
    if let Some(notice) = &notice {
        // The same marker the MCP server appends to a clipped result, so a reader who has learned
        // to look for one line has learned to look for both.
        println!("… [truncated: {notice}]");
    }
    Ok(())
}

/// One sentence that must answer "how much am I not seeing" and "what do I type instead". It
/// deliberately does not hand back the `--max` that would list everything: followed on the 33,323
/// entries of `german_new.zip` that is an ~11 MB document against a 256 KiB result budget
/// (`gore_mcp::DEFAULT_MAX_STDOUT_BYTES`), so the cut lands inside `entries` -- and serde_json
/// sorts keys here, so the surviving prefix has lost the counts and `truncated` along with the end
/// of the array, and no longer parses. Sending a caller there is the failure this bound prevents.
fn list_truncation_notice(matched: usize, listed: usize) -> String {
    format!(
        "{matched} entries matched and only the first {listed} are shown. Narrow the query with \
         --filter, and raise --max only as far as you need: asking for all {matched} at once \
         produces a document large enough to be cut off in transit, and a cut-off JSON array no \
         longer parses."
    )
}

/// Case-insensitive substring test for `--filter`. Voice archives genuinely mix case --
/// `LINE_ONE.OGG` sits beside `line.ogg` -- so a case-sensitive filter would answer "no such
/// entry" when the truth is "wrong case", which is a false negative dressed as a fact.
///
/// The fold is `str::to_lowercase`, which is what `gore_vo`'s `fold_case` applies to `--basename`.
/// Folding only ASCII here would move that same false negative one code point up: German archives
/// are the documented target, and `--filter MÜLLER` must not report nothing about an archive where
/// `extract --basename DIA_MÜLLER_01.OGG` resolves. An empty needle keeps every entry.
fn contains_case_insensitive(haystack: &str, lowercase_needle: &str) -> bool {
    haystack.to_lowercase().contains(lowercase_needle)
}

fn match_line(path: &Path, loc_id: &str, json: bool) -> Result<()> {
    let result = resolve_voice_line(path, loc_id)?;
    if json {
        println!("{}", serde_json::to_string_pretty(&result)?);
        return Ok(());
    }

    println!("Voice line: {}", result.loc_id);
    println!(
        "Archive: {} ({} bytes, SHA-256 {})",
        result.archive, result.archive_size, result.archive_sha256
    );
    println!("Expected basename: {}", result.expected_basename);
    println!(
        "Resolution: {} ({} match(es))",
        result.resolution.as_str(),
        result.match_count
    );
    for member in &result.matches {
        println!(
            "  {} ({} bytes, {} packed, CRC32 {:08x})",
            member.path, member.uncompressed_size, member.compressed_size, member.crc32
        );
    }
    Ok(())
}

/// Resolve one localization ID against safe voice members without reading or extracting a member
/// payload. This deliberately mirrors the bounded FFI matcher: exact ASCII case-insensitive
/// basename equality, and no implicit selection when more than one path matches.
fn resolve_voice_line(path: &Path, loc_id: &str) -> Result<VoiceLineMatch> {
    let expected_basename = expected_voice_basename(loc_id)?;
    let limits = voice_match_limits();
    let archive = ArchiveIndex::open(path, limits)
        .with_context(|| format!("indexing voice archive '{}'", path.display()))?;
    let mut matches = Vec::new();
    let mut serialized_matches = 2usize; // JSON array brackets.
    for entry in archive.list() {
        if !ascii_case_equal(&entry.basename, &expected_basename) {
            continue;
        }
        // Do not hide an ineligible exact collision: filtering it out could falsely turn an
        // ambiguous archive into a unique result. The FFI matcher fails closed the same way.
        validate_matching_voice_entry(entry, &limits)?;
        let member = VoiceLineMember {
            path: entry.path.clone(),
            compressed_size: entry.compressed_size,
            uncompressed_size: entry.uncompressed_size,
            crc32: entry.crc32,
        };
        let member_bytes = serde_json::to_vec(&member)
            .context("serializing a matching voice archive member")?
            .len();
        serialized_matches = serialized_matches
            .checked_add(member_bytes)
            .and_then(|size| size.checked_add(usize::from(!matches.is_empty())))
            .ok_or_else(|| anyhow::anyhow!("voice match result size overflowed"))?;
        if serialized_matches > MAX_MATCH_JSON_BYTES {
            bail!(
                "voice match result exceeds JSON limit: {serialized_matches} > {MAX_MATCH_JSON_BYTES} bytes"
            );
        }
        matches.push(member);
    }

    let resolution = match matches.len() {
        0 => VoiceLineResolution::Unresolved,
        1 => VoiceLineResolution::Unique,
        _ => VoiceLineResolution::Ambiguous,
    };
    let result = VoiceLineMatch {
        archive: archive.path().display().to_string(),
        archive_size: archive.archive_bytes(),
        archive_sha256: lowercase_sha256(archive.archive_sha256()),
        loc_id: loc_id.to_owned(),
        expected_basename,
        resolution,
        match_count: matches.len(),
        matches,
    };
    let serialized_result = serde_json::to_vec(&result)
        .context("serializing the voice line match result")?
        .len();
    if serialized_result > MAX_MATCH_JSON_BYTES {
        bail!(
            "voice match result exceeds JSON limit: {serialized_result} > {MAX_MATCH_JSON_BYTES} bytes"
        );
    }
    Ok(result)
}

fn expected_voice_basename(loc_id: &str) -> Result<String> {
    if loc_id.is_empty() {
        bail!("localization ID must not be empty");
    }
    if loc_id.len() > MAX_LOC_ID_BYTES {
        bail!(
            "localization ID exceeds the limit: {} > {MAX_LOC_ID_BYTES} bytes",
            loc_id.len()
        );
    }
    if !loc_id.is_ascii()
        || loc_id.trim() != loc_id
        || loc_id == "."
        || loc_id == ".."
        || loc_id.contains('/')
        || loc_id.contains('\\')
        || loc_id.chars().any(char::is_control)
    {
        bail!(
            "localization ID must be one trimmed ASCII, non-control basename stem without path separators"
        );
    }
    let basename_bytes = loc_id
        .len()
        .checked_add(".ogg".len())
        .ok_or_else(|| anyhow::anyhow!("localization ID overflows the archive entry path limit"))?;
    if basename_bytes > MAX_MATCH_ENTRY_PATH_BYTES {
        bail!(
            "localization ID plus '.ogg' exceeds the archive entry path limit: {basename_bytes} > {MAX_MATCH_ENTRY_PATH_BYTES} bytes"
        );
    }
    Ok(format!("{loc_id}.ogg"))
}

fn ascii_case_equal(left: &str, right: &str) -> bool {
    left.is_ascii() && right.is_ascii() && left.eq_ignore_ascii_case(right)
}

fn validate_matching_voice_entry(entry: &ArchiveEntry, limits: &Limits) -> Result<()> {
    validate_archive_entry_path(&entry.path, limits)
        .with_context(|| format!("exact voice match has an unsafe path: {:?}", entry.path))?;
    if entry.is_directory {
        bail!("exact voice match is a directory: {:?}", entry.path);
    }
    if entry.is_symlink {
        bail!("exact voice match is a symbolic link: {:?}", entry.path);
    }
    if entry.encrypted {
        bail!("exact voice match is encrypted: {:?}", entry.path);
    }
    if let Some(mode) = entry.unix_mode {
        let file_type = mode & 0o170000;
        if file_type != 0 && file_type != 0o100000 {
            bail!("exact voice match is not a regular file: {:?}", entry.path);
        }
    }
    if !entry.basename.to_ascii_lowercase().ends_with(".ogg") {
        bail!("exact voice match is not an Ogg member: {:?}", entry.path);
    }

    #[allow(deprecated)]
    let compression_code = entry.compression.to_u16();
    if !matches!(compression_code, 0 | 8) {
        bail!(
            "exact voice match uses unsupported compression method {compression_code}: {:?}",
            entry.path
        );
    }
    Ok(())
}

fn voice_match_limits() -> Limits {
    Limits {
        max_entries: MAX_MATCH_ARCHIVE_ENTRIES,
        max_central_directory_bytes: MAX_MATCH_CENTRAL_DIRECTORY_BYTES,
        max_path_bytes: MAX_MATCH_ENTRY_PATH_BYTES,
        max_entry_uncompressed_bytes: MAX_MATCH_ENTRY_OUTPUT_BYTES,
        max_total_uncompressed_bytes: MAX_MATCH_TOTAL_UNCOMPRESSED_BYTES,
        ..Limits::default()
    }
}

fn lowercase_sha256(digest: [u8; 32]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut value = String::with_capacity(64);
    for byte in digest {
        value.push(char::from(HEX[usize::from(byte >> 4)]));
        value.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    value
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
    use std::fs::File;
    use std::io::Write;

    use super::*;
    use zip::write::SimpleFileOptions;
    use zip::{CompressionMethod, ZipWriter};

    fn write_match_archive(path: &Path, entries: &[(&str, CompressionMethod, &[u8])]) {
        let file = File::create(path).unwrap();
        let mut writer = ZipWriter::new(file);
        for (name, method, payload) in entries {
            writer
                .start_file(
                    *name,
                    SimpleFileOptions::default().compression_method(*method),
                )
                .unwrap();
            writer.write_all(payload).unwrap();
        }
        writer.finish().unwrap();
    }

    fn test_ogg(sample_rate: u32) -> Vec<u8> {
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
            let mut crc = 0u32;
            for &byte in &data[offset..offset + page_len] {
                crc ^= u32::from(byte) << 24;
                for _ in 0..8 {
                    crc = if crc & 0x8000_0000 != 0 {
                        (crc << 1) ^ 0x04c1_1db7
                    } else {
                        crc << 1
                    };
                }
            }
            data[offset + 22..offset + 26].copy_from_slice(&crc.to_le_bytes());
            offset += page_len;
        }
        data
    }

    #[test]
    fn match_line_resolves_one_exact_ascii_casefolded_basename_read_only() {
        let dir = tempfile::tempdir().unwrap();
        let archive = dir.path().join("unique.zip");
        write_match_archive(
            &archive,
            &[
                (
                    "Voices/Hero/LINE_ONE.OGG",
                    CompressionMethod::Deflated,
                    b"unique voice",
                ),
                (
                    "Voices/Hero/OTHER.ogg",
                    CompressionMethod::Stored,
                    b"other voice",
                ),
            ],
        );
        let before = fs::read(&archive).unwrap();

        let result = resolve_voice_line(&archive, "line_one").unwrap();

        assert_eq!(result.archive_size, before.len() as u64);
        assert_eq!(result.archive_sha256.len(), 64);
        assert!(result
            .archive_sha256
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)));
        assert_eq!(result.expected_basename, "line_one.ogg");
        assert_eq!(result.resolution, VoiceLineResolution::Unique);
        assert_eq!(result.match_count, 1);
        assert_eq!(result.matches.len(), result.match_count);
        assert_eq!(result.matches[0].path, "Voices/Hero/LINE_ONE.OGG");
        assert_eq!(result.matches[0].uncompressed_size, 12);
        assert_ne!(result.matches[0].crc32, 0);
        assert_eq!(fs::read(&archive).unwrap(), before);
    }

    #[test]
    fn match_line_reports_every_exact_path_as_ambiguous_without_selecting() {
        let dir = tempfile::tempdir().unwrap();
        let archive = dir.path().join("ambiguous.zip");
        write_match_archive(
            &archive,
            &[
                ("Voices/A/LINE.ogg", CompressionMethod::Stored, b"one"),
                ("Voices/B/line.OGG", CompressionMethod::Deflated, b"two"),
                ("Voices/C/LiNe.Ogg", CompressionMethod::Stored, b"three"),
                (
                    "Voices/D/NOT_LINE.ogg",
                    CompressionMethod::Stored,
                    b"not a match",
                ),
            ],
        );

        let result = resolve_voice_line(&archive, "lInE").unwrap();

        assert_eq!(result.resolution, VoiceLineResolution::Ambiguous);
        assert_eq!(result.match_count, 3);
        assert_eq!(
            result
                .matches
                .iter()
                .map(|member| member.path.as_str())
                .collect::<Vec<_>>(),
            [
                "Voices/A/LINE.ogg",
                "Voices/B/line.OGG",
                "Voices/C/LiNe.Ogg",
            ]
        );
        let json = serde_json::to_value(&result).unwrap();
        assert_eq!(json["resolution"], "ambiguous");
        assert_eq!(json["match_count"], 3);
        assert_eq!(json["matches"][0]["path"], "Voices/A/LINE.ogg");
        assert!(json.get("selected").is_none());
        assert!(json.get("entry_path").is_none());
    }

    #[test]
    fn match_line_is_unresolved_for_substrings_directory_text_and_suffixes() {
        let dir = tempfile::tempdir().unwrap();
        let archive = dir.path().join("unresolved.zip");
        write_match_archive(
            &archive,
            &[
                (
                    "Voices/Hero/PREFIX_LINE_SUFFIX.ogg",
                    CompressionMethod::Stored,
                    b"prefix",
                ),
                (
                    "Voices/LINE/OTHER.ogg",
                    CompressionMethod::Stored,
                    b"directory",
                ),
                (
                    "Voices/Hero/LINE.ogg.backup",
                    CompressionMethod::Stored,
                    b"suffix",
                ),
            ],
        );

        let result = resolve_voice_line(&archive, "LINE").unwrap();

        assert_eq!(result.resolution, VoiceLineResolution::Unresolved);
        assert_eq!(result.match_count, 0);
        assert!(result.matches.is_empty());
    }

    #[test]
    fn match_line_fails_closed_on_an_ineligible_exact_symlink() {
        let dir = tempfile::tempdir().unwrap();
        let archive = dir.path().join("symlink.zip");
        let file = File::create(&archive).unwrap();
        let mut writer = ZipWriter::new(file);
        writer
            .add_symlink(
                "Voices/LINE.ogg",
                "target.ogg",
                SimpleFileOptions::default(),
            )
            .unwrap();
        writer.finish().unwrap();
        let before = fs::read(&archive).unwrap();

        let error = resolve_voice_line(&archive, "line").unwrap_err();

        assert!(error.to_string().contains("symbolic link"), "{error:#}");
        assert_eq!(fs::read(&archive).unwrap(), before);
    }

    #[test]
    fn match_line_rejects_invalid_or_oversized_localization_ids() {
        for loc_id in [
            "", "../LINE", "..\\LINE", " LINE", "LINE ", "LINE\n", "LINE\0", ".", "..", "LÍNE",
        ] {
            assert!(
                expected_voice_basename(loc_id).is_err(),
                "unexpectedly accepted {loc_id:?}"
            );
        }
        assert!(expected_voice_basename(&"x".repeat(MAX_LOC_ID_BYTES + 1)).is_err());
        assert_eq!(
            expected_voice_basename(&"x".repeat(MAX_LOC_ID_BYTES)).unwrap(),
            format!("{}.ogg", "x".repeat(MAX_LOC_ID_BYTES))
        );
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
