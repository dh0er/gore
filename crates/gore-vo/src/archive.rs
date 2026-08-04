use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Cursor, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use zip::read::{ArchiveOffset, Config as ReadConfig};
use zip::{DateTime, ZipArchive, ZipWriter};

use crate::{validate_ogg, Error, Limits, OggInfo, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchiveEntry {
    pub index: usize,
    pub path: String,
    pub basename: String,
    pub compressed_size: u64,
    pub uncompressed_size: u64,
    pub crc32: u32,
    pub compression: zip::CompressionMethod,
    pub last_modified: Option<DateTime>,
    pub unix_mode: Option<u32>,
    pub is_directory: bool,
    pub is_symlink: bool,
    pub encrypted: bool,
}

/// Content identity captured from the same no-follow handle that was indexed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArchiveSeal {
    pub size: u64,
    pub sha256: [u8; 32],
}

#[derive(Debug, Clone)]
pub struct ArchiveIndex {
    path: PathBuf,
    entries: Vec<ArchiveEntry>,
    limits: Limits,
    archive_bytes: u64,
    archive_sha256: [u8; 32],
    archive_comment: Vec<u8>,
    zip64_comment: Option<Vec<u8>>,
    total_uncompressed_bytes: u64,
}

#[derive(Debug, Clone, Copy)]
pub enum ArchiveEdit<'a> {
    Add {
        path: &'a str,
        ogg: &'a [u8],
    },
    Replace {
        basename: &'a str,
        exact_path: Option<&'a str>,
        ogg: &'a [u8],
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditAction {
    Added,
    Replaced,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WriteReport {
    pub output: PathBuf,
    pub action: EditAction,
    pub entry_index: usize,
    pub archive_path: String,
    pub sha256: [u8; 32],
    pub ogg: OggInfo,
}

/// Per-edit result returned by an in-memory archive rewrite.
///
/// Unlike [`WriteReport`], this does not contain an output path because no filesystem artifact
/// is created. Reports remain in the same order as the supplied edits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RewriteReport {
    pub action: EditAction,
    pub entry_index: usize,
    pub archive_path: String,
    pub sha256: [u8; 32],
    pub ogg: OggInfo,
}

#[derive(Debug)]
struct ExtractionPlan {
    index: usize,
    components: Vec<String>,
    is_directory: bool,
    source_path: String,
}

#[derive(Debug)]
struct EditPlan<'a> {
    replacement_index: Option<usize>,
    output_index: usize,
    archive_path: String,
    ogg: &'a [u8],
    ogg_info: OggInfo,
    sha256: [u8; 32],
    action: EditAction,
    compression: zip::CompressionMethod,
}

#[derive(Debug)]
struct ExpectedEntry {
    path: String,
    compression: zip::CompressionMethod,
    uncompressed_size: Option<u64>,
    compressed_size: Option<u64>,
    crc32: Option<u32>,
    modified: DateTime,
    permissions: Option<u32>,
    sha256: Option<[u8; 32]>,
}

#[derive(Debug)]
struct PreparedRewrite {
    bytes: Vec<u8>,
    reports: Vec<RewriteReport>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RawZipPreflight {
    entry_count: usize,
    archive_offset: u64,
    central_directory_start: u64,
    eocd_offset: u64,
}

#[derive(Debug, Clone, Copy)]
struct RawCentralDirectory {
    archive_offset: u64,
    start: u64,
    end: u64,
    eocd_offset: u64,
    entry_count: usize,
}

#[derive(Debug)]
struct SourceSnapshot {
    file: File,
    size: u64,
    sha256: [u8; 32],
}

const ZIP_EOCD_FIXED_BYTES: usize = 22;
const ZIP_EOCD_SCAN_BYTES: u64 = ZIP_EOCD_FIXED_BYTES as u64 + u16::MAX as u64;
const ZIP64_LOCATOR_BYTES: u64 = 20;
const ZIP64_EOCD_FIXED_BYTES: usize = 56;
const MAX_ZIP64_EOCD_BYTES: u64 = 1024 * 1024;
const CENTRAL_HEADER_FIXED_BYTES: u64 = 46;
const EOCD_SCAN_CHUNK_BYTES: u64 = 64 * 1024;
const MAX_EOCD_SIGNATURE_CANDIDATES: usize = 1024;

/// Copy one no-follow source handle into a private bounded tempfile while sealing exactly the
/// metadata-captured byte range. ZIP metadata and payloads are never parsed from the mutable
/// source handle; the returned `File` owns the tempfile for as long as a `ZipArchive` needs it.
fn snapshot_source(path: &Path, limits: &Limits) -> Result<SourceSnapshot> {
    let mut source = open_source_file(path)?;
    let initial_metadata = source_metadata(&source, path)?;
    let size = initial_metadata.len();
    check_limit("archive bytes", size, limits.max_archive_bytes)?;
    source
        .seek(SeekFrom::Start(0))
        .map_err(|error| source_io(path, error))?;

    let mut snapshot = tempfile::tempfile()?;
    let mut hash = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    let mut remaining = size;
    while remaining != 0 {
        let wanted = usize::try_from(remaining.min(buffer.len() as u64))
            .expect("bounded snapshot read fits usize");
        let read = source
            .read(&mut buffer[..wanted])
            .map_err(|error| source_io(path, error))?;
        if read == 0 {
            return Err(Error::ArchiveChanged);
        }
        snapshot.write_all(&buffer[..read])?;
        hash.update(&buffer[..read]);
        remaining -= read as u64;
    }

    // One bounded byte detects growth after metadata without following an unbounded source.
    let mut probe = [0u8; 1];
    if source
        .read(&mut probe)
        .map_err(|error| source_io(path, error))?
        != 0
    {
        return Err(Error::ArchiveChanged);
    }
    ensure_source_revision(&source, path, &initial_metadata)?;
    snapshot.sync_all()?;
    if snapshot.metadata()?.len() != size {
        return Err(Error::ArchiveChanged);
    }
    snapshot.seek(SeekFrom::Start(0))?;

    Ok(SourceSnapshot {
        file: snapshot,
        size,
        sha256: hash.finalize().into(),
    })
}

fn parse_snapshot(mut snapshot: SourceSnapshot, limits: &Limits) -> Result<ZipArchive<File>> {
    let raw = preflight_raw_zip(&mut snapshot.file, snapshot.size, limits)?;
    snapshot.file.seek(SeekFrom::Start(0))?;
    let config = ReadConfig {
        archive_offset: ArchiveOffset::Known(raw.archive_offset),
    };
    let archive = ZipArchive::with_config(config, snapshot.file)?;
    if archive.offset() != raw.archive_offset
        || archive.central_directory_start() != raw.central_directory_start
        || archive.len() != raw.entry_count
    {
        return Err(invalid_archive(
            "ZIP parser metadata does not match the raw central-directory preflight",
        ));
    }
    Ok(archive)
}

fn preflight_raw_zip<R: Read + Seek>(
    reader: &mut R,
    archive_bytes: u64,
    limits: &Limits,
) -> Result<RawZipPreflight> {
    let directory = locate_raw_central_directory(reader, archive_bytes, limits)?;
    let minimum_directory_bytes = (directory.entry_count as u64)
        .checked_mul(CENTRAL_HEADER_FIXED_BYTES)
        .ok_or_else(|| invalid_archive("central-directory record count overflowed"))?;
    if directory.end - directory.start < minimum_directory_bytes {
        return Err(invalid_archive(
            "central directory is too short for its declared record count",
        ));
    }

    reader.seek(SeekFrom::Start(directory.start))?;
    let mut cursor = directory.start;
    let mut raw_name_digests = Vec::<[u8; 32]>::new();
    raw_name_digests
        .try_reserve_exact(directory.entry_count)
        .map_err(|error| allocation_io("raw ZIP filename index", error))?;
    let mut total_uncompressed = 0u64;

    for _ in 0..directory.entry_count {
        let fixed_end = cursor
            .checked_add(CENTRAL_HEADER_FIXED_BYTES)
            .ok_or_else(|| invalid_archive("central-directory cursor overflowed"))?;
        if fixed_end > directory.end {
            return Err(invalid_archive("truncated central-directory header"));
        }
        let mut fixed = [0u8; CENTRAL_HEADER_FIXED_BYTES as usize];
        read_exact(reader, &mut fixed)?;
        if fixed[..4] != 0x0201_4b50u32.to_le_bytes() {
            return Err(invalid_archive(
                "invalid central-directory header signature",
            ));
        }

        let flags = le_u16(&fixed[8..10]);
        if flags & 0x0040 != 0 && flags & 0x0001 == 0 {
            return Err(invalid_archive(
                "strong-encryption flag is set without the encrypted flag",
            ));
        }
        if flags & 0x2000 != 0 && flags & 0x0001 == 0 {
            return Err(invalid_archive(
                "masked-header flag is set without the encrypted flag",
            ));
        }

        let compression_method = le_u16(&fixed[10..12]);
        let compressed_32 = le_u32(&fixed[20..24]);
        let uncompressed_32 = le_u32(&fixed[24..28]);
        let name_len = usize::from(le_u16(&fixed[28..30]));
        let extra_len = usize::from(le_u16(&fixed[30..32]));
        let comment_len = usize::from(le_u16(&fixed[32..34]));
        let disk_start_16 = le_u16(&fixed[34..36]);
        let local_header_32 = le_u32(&fixed[42..46]);
        if name_len == 0 {
            return Err(invalid_archive("central-directory filename is empty"));
        }
        check_limit(
            "entry path bytes",
            name_len as u64,
            limits.max_path_bytes as u64,
        )?;

        let variable_len = name_len
            .checked_add(extra_len)
            .and_then(|value| value.checked_add(comment_len))
            .ok_or_else(|| invalid_archive("central-directory variable fields overflowed"))?;
        let record_end = fixed_end
            .checked_add(variable_len as u64)
            .ok_or_else(|| invalid_archive("central-directory record length overflowed"))?;
        if record_end > directory.end {
            return Err(invalid_archive("truncated central-directory record"));
        }

        let mut name = fallible_zeroed(name_len, "raw ZIP filename")?;
        read_exact(reader, &mut name)?;
        raw_name_digests.push(Sha256::digest(&name).into());

        let mut extra = fallible_zeroed(extra_len, "raw ZIP extra field")?;
        read_exact(reader, &mut extra)?;
        let resolved = resolve_central_entry_metadata(
            &extra,
            flags,
            compression_method,
            compressed_32,
            uncompressed_32,
            local_header_32,
            disk_start_16,
        )?;
        if resolved.disk_start != 0 {
            return Err(invalid_archive("multi-disk ZIP entries are unsupported"));
        }
        let local_header = resolved
            .local_header_offset
            .checked_add(directory.archive_offset)
            .ok_or_else(|| invalid_archive("local-header offset overflowed"))?;
        if local_header >= directory.start {
            return Err(invalid_archive(
                "local-header offset points into the central directory",
            ));
        }
        if resolved.compressed_size > archive_bytes {
            return Err(invalid_archive(
                "compressed entry size exceeds archive size",
            ));
        }
        check_limit(
            "entry uncompressed bytes",
            resolved.uncompressed_size,
            limits.max_entry_uncompressed_bytes,
        )?;
        check_ratio(resolved.uncompressed_size, resolved.compressed_size, limits)?;
        total_uncompressed = total_uncompressed
            .checked_add(resolved.uncompressed_size)
            .ok_or(Error::LimitExceeded {
                kind: "total uncompressed bytes",
                actual: u64::MAX,
                limit: limits.max_total_uncompressed_bytes,
            })?;
        check_limit(
            "total uncompressed bytes",
            total_uncompressed,
            limits.max_total_uncompressed_bytes,
        )?;

        if comment_len != 0 {
            reader.seek(SeekFrom::Current(comment_len as i64))?;
        }
        cursor = record_end;
    }

    if cursor != directory.end {
        return Err(invalid_archive(
            "central-directory byte size does not match its declared records",
        ));
    }
    raw_name_digests.sort_unstable();
    if raw_name_digests.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(invalid_archive("duplicate raw central-directory filename"));
    }

    reject_parser_fallback_candidates(reader, archive_bytes, &directory, limits)?;

    Ok(RawZipPreflight {
        entry_count: directory.entry_count,
        archive_offset: directory.archive_offset,
        central_directory_start: directory.start,
        eocd_offset: directory.eocd_offset,
    })
}

#[derive(Debug, Clone, Copy)]
struct ResolvedCentralEntry {
    compressed_size: u64,
    uncompressed_size: u64,
    local_header_offset: u64,
    disk_start: u32,
}

fn resolve_central_entry_metadata(
    extra: &[u8],
    flags: u16,
    compression_method: u16,
    compressed_32: u32,
    uncompressed_32: u32,
    local_header_32: u32,
    disk_start_16: u16,
) -> Result<ResolvedCentralEntry> {
    let needs_uncompressed = uncompressed_32 == u32::MAX;
    let needs_compressed = compressed_32 == u32::MAX;
    let needs_local_header = local_header_32 == u32::MAX;
    let needs_disk = disk_start_16 == u16::MAX;
    let mut uncompressed = (!needs_uncompressed).then_some(u64::from(uncompressed_32));
    let mut compressed = (!needs_compressed).then_some(u64::from(compressed_32));
    let mut local_header = (!needs_local_header).then_some(u64::from(local_header_32));
    let mut disk_start = (!needs_disk).then_some(u32::from(disk_start_16));
    let mut zip64_seen = false;
    let mut aes_seen = false;
    let mut offset = 0usize;

    while offset < extra.len() {
        if extra.len() - offset < 4 {
            return Err(invalid_archive("truncated central-directory extra field"));
        }
        let id = le_u16(&extra[offset..offset + 2]);
        let length = usize::from(le_u16(&extra[offset + 2..offset + 4]));
        offset += 4;
        let end = offset
            .checked_add(length)
            .ok_or_else(|| invalid_archive("central-directory extra field overflowed"))?;
        if end > extra.len() {
            return Err(invalid_archive("truncated central-directory extra payload"));
        }
        let data = &extra[offset..end];
        match id {
            0x0001 => {
                if zip64_seen {
                    return Err(invalid_archive("duplicate ZIP64 extra field"));
                }
                zip64_seen = true;
                let mut field = 0usize;
                if needs_uncompressed {
                    uncompressed = Some(take_u64(data, &mut field).ok_or_else(|| {
                        invalid_archive("ZIP64 extra field lacks uncompressed size")
                    })?);
                }
                if needs_compressed {
                    compressed = Some(take_u64(data, &mut field).ok_or_else(|| {
                        invalid_archive("ZIP64 extra field lacks compressed size")
                    })?);
                }
                if needs_local_header {
                    local_header = Some(take_u64(data, &mut field).ok_or_else(|| {
                        invalid_archive("ZIP64 extra field lacks local-header offset")
                    })?);
                }
                if needs_disk {
                    disk_start =
                        Some(take_u32(data, &mut field).ok_or_else(|| {
                            invalid_archive("ZIP64 extra field lacks disk number")
                        })?);
                }
            }
            0x9901 => {
                if aes_seen || data.len() != 7 {
                    return Err(invalid_archive("malformed or duplicate AES extra field"));
                }
                aes_seen = true;
                let vendor_version = le_u16(&data[0..2]);
                let strength = data[4];
                let actual_method = le_u16(&data[5..7]);
                if !matches!(vendor_version, 1 | 2)
                    || &data[2..4] != b"AE"
                    || !matches!(strength, 1..=3)
                    || actual_method == 99
                {
                    return Err(invalid_archive("malformed AES encryption metadata"));
                }
            }
            _ => {}
        }
        offset = end;
    }

    if (needs_uncompressed || needs_compressed || needs_local_header || needs_disk) && !zip64_seen {
        return Err(invalid_archive("ZIP64 sentinel lacks a ZIP64 extra field"));
    }
    if compression_method == 99 {
        if !aes_seen || flags & 0x0001 == 0 {
            return Err(invalid_archive(
                "AES compression method lacks valid encrypted AES metadata",
            ));
        }
    } else if aes_seen {
        return Err(invalid_archive(
            "AES extra field is paired with a non-AES compression method",
        ));
    }

    Ok(ResolvedCentralEntry {
        compressed_size: compressed
            .ok_or_else(|| invalid_archive("missing compressed entry size"))?,
        uncompressed_size: uncompressed
            .ok_or_else(|| invalid_archive("missing uncompressed entry size"))?,
        local_header_offset: local_header
            .ok_or_else(|| invalid_archive("missing local-header offset"))?,
        disk_start: disk_start.ok_or_else(|| invalid_archive("missing disk number"))?,
    })
}

fn locate_raw_central_directory<R: Read + Seek>(
    reader: &mut R,
    archive_bytes: u64,
    limits: &Limits,
) -> Result<RawCentralDirectory> {
    if archive_bytes < ZIP_EOCD_FIXED_BYTES as u64 {
        return Err(invalid_archive("ZIP is too short to contain an EOCD"));
    }
    let tail_start = archive_bytes.saturating_sub(ZIP_EOCD_SCAN_BYTES);
    let tail_len = usize::try_from(archive_bytes - tail_start)
        .map_err(|_| invalid_archive("EOCD scan length does not fit memory"))?;
    let mut tail = fallible_zeroed(tail_len, "ZIP EOCD scan")?;
    read_at(reader, tail_start, &mut tail)?;

    let mut eocd_relative = None;
    for offset in (0..=tail.len() - 4).rev() {
        if tail[offset..offset + 4] != 0x0605_4b50u32.to_le_bytes() {
            continue;
        }
        let Some(fixed_end) = offset.checked_add(ZIP_EOCD_FIXED_BYTES) else {
            continue;
        };
        if fixed_end > tail.len() {
            continue;
        }
        let comment_len = usize::from(le_u16(&tail[offset + 20..offset + 22]));
        if fixed_end.checked_add(comment_len) == Some(tail.len()) {
            eocd_relative = Some(offset);
            break;
        }
    }
    let eocd_relative = eocd_relative.ok_or_else(|| {
        invalid_archive("EOCD is missing, truncated, or followed by unsupported trailing data")
    })?;
    let eocd_offset = tail_start + eocd_relative as u64;
    let eocd = &tail[eocd_relative..eocd_relative + ZIP_EOCD_FIXED_BYTES];
    let disk = le_u16(&eocd[4..6]);
    let directory_disk = le_u16(&eocd[6..8]);
    let entries_on_disk = le_u16(&eocd[8..10]);
    let entries_total = le_u16(&eocd[10..12]);
    let directory_size_32 = le_u32(&eocd[12..16]);
    let directory_offset_32 = le_u32(&eocd[16..20]);
    let may_be_zip64 = disk == u16::MAX
        || directory_disk == u16::MAX
        || entries_on_disk == u16::MAX
        || entries_total == u16::MAX
        || directory_size_32 == u32::MAX
        || directory_offset_32 == u32::MAX;

    let (entry_count, directory_size, directory_offset, archive_offset, expected_end) =
        if may_be_zip64 {
            locate_zip64_directory(
                reader,
                eocd_offset,
                disk,
                directory_disk,
                entries_on_disk,
                entries_total,
                directory_size_32,
                directory_offset_32,
            )?
        } else {
            if disk != 0 || directory_disk != 0 || entries_on_disk != entries_total {
                return Err(invalid_archive("multi-disk ZIP archives are unsupported"));
            }
            let directory_size = u64::from(directory_size_32);
            let physical_start = eocd_offset
                .checked_sub(directory_size)
                .ok_or_else(|| invalid_archive("central directory begins before the file"))?;
            let relative_start = u64::from(directory_offset_32);
            let archive_offset = physical_start.checked_sub(relative_start).ok_or_else(|| {
                invalid_archive("central-directory relative offset is inconsistent")
            })?;
            (
                u64::from(entries_total),
                directory_size,
                relative_start,
                archive_offset,
                eocd_offset,
            )
        };

    check_limit("entry count", entry_count, limits.max_entries as u64)?;
    check_limit(
        "central directory bytes",
        directory_size,
        limits.max_central_directory_bytes,
    )?;
    let entry_count = usize::try_from(entry_count)
        .map_err(|_| invalid_archive("central-directory entry count does not fit memory"))?;
    let start = archive_offset
        .checked_add(directory_offset)
        .ok_or_else(|| invalid_archive("central-directory offset overflowed"))?;
    let end = start
        .checked_add(directory_size)
        .ok_or_else(|| invalid_archive("central-directory size overflowed"))?;
    if end != expected_end || end > archive_bytes {
        return Err(invalid_archive(
            "central-directory offset/size does not terminate at its EOCD",
        ));
    }
    Ok(RawCentralDirectory {
        archive_offset,
        start,
        end,
        eocd_offset,
        entry_count,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AlternateZip64Candidate {
    /// The EOCD64 locator could not be parsed, so `zip` falls back to the ZIP32 fields.
    FallBackToZip32,
    /// A locator was parsed, but its EOCD64 chain cannot be accepted by `zip`.
    NotViable,
    /// The EOCD64 chain reaches the point where `zip` may allocate/read its central directory.
    Viable,
}

fn reject_parser_fallback_candidates<R: Read + Seek>(
    reader: &mut R,
    archive_bytes: u64,
    selected: &RawCentralDirectory,
    limits: &Limits,
) -> Result<()> {
    let mut chunk_start = 0u64;
    let mut signature_count = 0usize;
    while chunk_start < archive_bytes {
        let owned = (archive_bytes - chunk_start).min(EOCD_SCAN_CHUNK_BYTES);
        let scan_len = (archive_bytes - chunk_start).min(owned.saturating_add(3));
        let mut chunk = fallible_zeroed(
            usize::try_from(scan_len)
                .map_err(|_| invalid_archive("EOCD candidate scan does not fit memory"))?,
            "EOCD candidate scan",
        )?;
        read_at(reader, chunk_start, &mut chunk)?;
        if chunk.len() >= 4 {
            for relative in 0..=chunk.len() - 4 {
                if relative as u64 >= owned {
                    break;
                }
                if chunk[relative..relative + 4] != 0x0605_4b50u32.to_le_bytes() {
                    continue;
                }
                signature_count = signature_count.saturating_add(1);
                if signature_count > MAX_EOCD_SIGNATURE_CANDIDATES {
                    return Err(Error::LimitExceeded {
                        kind: "EOCD signature candidates",
                        actual: signature_count as u64,
                        limit: MAX_EOCD_SIGNATURE_CANDIDATES as u64,
                    });
                }
                let candidate = chunk_start + relative as u64;
                if candidate == selected.eocd_offset {
                    continue;
                }
                if parser_fallback_candidate_is_viable(
                    reader,
                    archive_bytes,
                    candidate,
                    selected.archive_offset,
                    limits,
                )? {
                    return Err(invalid_archive(
                        "multiple parser-viable EOCD records are unsupported",
                    ));
                }
            }
        }
        chunk_start = chunk_start
            .checked_add(owned)
            .ok_or_else(|| invalid_archive("EOCD candidate scan offset overflowed"))?;
    }
    Ok(())
}

fn parser_fallback_candidate_is_viable<R: Read + Seek>(
    reader: &mut R,
    archive_bytes: u64,
    eocd_offset: u64,
    archive_offset: u64,
    limits: &Limits,
) -> Result<bool> {
    if eocd_offset
        .checked_add(ZIP_EOCD_FIXED_BYTES as u64)
        .is_none_or(|end| end > archive_bytes)
    {
        return Ok(false);
    }
    let mut eocd = [0u8; ZIP_EOCD_FIXED_BYTES];
    read_at(reader, eocd_offset, &mut eocd)?;
    if eocd[..4] != 0x0605_4b50u32.to_le_bytes() {
        return Ok(false);
    }
    let comment_len = u64::from(le_u16(&eocd[20..22]));
    if eocd_offset
        .checked_add(ZIP_EOCD_FIXED_BYTES as u64)
        .and_then(|end| end.checked_add(comment_len))
        .is_none_or(|end| end > archive_bytes)
    {
        return Ok(false);
    }

    let entries_total = le_u16(&eocd[10..12]);
    let directory_offset = le_u32(&eocd[16..20]);
    if entries_total == u16::MAX || directory_offset == u32::MAX {
        match alternate_zip64_candidate(reader, eocd_offset, archive_offset, limits)? {
            AlternateZip64Candidate::Viable => return Ok(true),
            AlternateZip64Candidate::NotViable => return Ok(false),
            AlternateZip64Candidate::FallBackToZip32 => {}
        }
    }

    let disk = le_u16(&eocd[4..6]);
    let directory_disk = le_u16(&eocd[6..8]);
    if disk != directory_disk {
        return Ok(false);
    }
    let entries_on_disk = u64::from(le_u16(&eocd[8..10]));
    let directory_size = u64::from(le_u32(&eocd[12..16]));
    if entries_total != 0 {
        let relative = u64::from(directory_offset);
        if relative >= eocd_offset {
            return Ok(false);
        }
        let Some(physical) = relative.checked_add(archive_offset) else {
            return Ok(false);
        };
        if physical >= eocd_offset {
            return Ok(false);
        }
        let mut signature = [0u8; 4];
        read_at(reader, physical, &mut signature)?;
        if signature != 0x0201_4b50u32.to_le_bytes() {
            return Ok(false);
        }
    }
    check_limit("entry count", entries_on_disk, limits.max_entries as u64)?;
    check_limit(
        "central directory bytes",
        directory_size,
        limits.max_central_directory_bytes,
    )?;
    Ok(true)
}

fn alternate_zip64_candidate<R: Read + Seek>(
    reader: &mut R,
    eocd_offset: u64,
    archive_offset: u64,
    limits: &Limits,
) -> Result<AlternateZip64Candidate> {
    let Some(locator_offset) = eocd_offset.checked_sub(ZIP64_LOCATOR_BYTES) else {
        return Ok(AlternateZip64Candidate::FallBackToZip32);
    };
    let mut locator = [0u8; ZIP64_LOCATOR_BYTES as usize];
    read_at(reader, locator_offset, &mut locator)?;
    if locator[..4] != 0x0706_4b50u32.to_le_bytes() {
        return Ok(AlternateZip64Candidate::FallBackToZip32);
    }
    let locator_disk = le_u32(&locator[4..8]);
    let eocd64_relative = le_u64(&locator[8..16]);
    let disk_count = le_u32(&locator[16..20]);
    if eocd64_relative >= locator_offset || disk_count > 1 {
        return Ok(AlternateZip64Candidate::NotViable);
    }
    let Some(eocd64_offset) = eocd64_relative.checked_add(archive_offset) else {
        return Ok(AlternateZip64Candidate::NotViable);
    };
    if eocd64_offset >= locator_offset {
        return Ok(AlternateZip64Candidate::NotViable);
    }
    if locator_offset - eocd64_offset < 4 {
        return Ok(AlternateZip64Candidate::NotViable);
    }
    let mut signature = [0u8; 4];
    read_at(reader, eocd64_offset, &mut signature)?;
    if signature != 0x0606_4b50u32.to_le_bytes() {
        return Ok(AlternateZip64Candidate::NotViable);
    }

    // `zip`'s optimistic finder starts at the Known-offset guess, but after finding EOCD64 magic
    // it continues scanning later signatures when parsing that record fails. From this point on,
    // any inconsistency therefore makes the enclosing fallback EOCD parser-viable; conservatively
    // reject it instead of assuming the guessed record is the only one `zip` will consider.
    if locator_offset - eocd64_offset < ZIP64_EOCD_FIXED_BYTES as u64 {
        return Ok(AlternateZip64Candidate::Viable);
    }
    let mut record = [0u8; ZIP64_EOCD_FIXED_BYTES];
    read_at(reader, eocd64_offset, &mut record)?;
    let record_size = le_u64(&record[4..12]);
    let Some(total_size) = record_size.checked_add(12) else {
        return Ok(AlternateZip64Candidate::Viable);
    };
    if record_size < 44
        || eocd64_offset.checked_add(total_size) != Some(locator_offset)
        || le_u32(&record[20..24]) != locator_disk
    {
        return Ok(AlternateZip64Candidate::Viable);
    }
    check_limit("ZIP64 EOCD bytes", total_size, MAX_ZIP64_EOCD_BYTES)?;
    let disk = le_u32(&record[16..20]);
    let directory_disk = le_u32(&record[20..24]);
    let entries_on_disk = le_u64(&record[24..32]);
    let entries_total = le_u64(&record[32..40]);
    let directory_size = le_u64(&record[40..48]);
    let directory_offset = le_u64(&record[48..56]);
    if entries_on_disk > entries_total || disk != directory_disk {
        return Ok(AlternateZip64Candidate::Viable);
    }
    check_limit("entry count", entries_total, limits.max_entries as u64)?;
    check_limit(
        "central directory bytes",
        directory_size,
        limits.max_central_directory_bytes,
    )?;
    let minimum_position = entries_total
        .saturating_mul(CENTRAL_HEADER_FIXED_BYTES)
        .saturating_add(directory_offset);
    if eocd64_offset < minimum_position {
        return Ok(AlternateZip64Candidate::Viable);
    }
    Ok(AlternateZip64Candidate::Viable)
}

#[allow(clippy::too_many_arguments)]
fn locate_zip64_directory<R: Read + Seek>(
    reader: &mut R,
    eocd_offset: u64,
    disk_16: u16,
    directory_disk_16: u16,
    entries_on_disk_16: u16,
    entries_total_16: u16,
    directory_size_32: u32,
    directory_offset_32: u32,
) -> Result<(u64, u64, u64, u64, u64)> {
    let locator_offset = eocd_offset
        .checked_sub(ZIP64_LOCATOR_BYTES)
        .ok_or_else(|| invalid_archive("ZIP64 locator does not fit before EOCD"))?;
    let mut locator = [0u8; ZIP64_LOCATOR_BYTES as usize];
    read_at(reader, locator_offset, &mut locator)?;
    if locator[..4] != 0x0706_4b50u32.to_le_bytes() {
        return Err(invalid_archive("ZIP64 sentinel lacks an EOCD64 locator"));
    }
    let locator_disk = le_u32(&locator[4..8]);
    let eocd64_relative = le_u64(&locator[8..16]);
    let disk_count = le_u32(&locator[16..20]);
    if locator_disk != 0 || disk_count != 1 {
        return Err(invalid_archive("multi-disk ZIP64 archives are unsupported"));
    }

    let scan_start = locator_offset.saturating_sub(MAX_ZIP64_EOCD_BYTES);
    let scan_len = usize::try_from(locator_offset - scan_start)
        .map_err(|_| invalid_archive("ZIP64 EOCD scan length does not fit memory"))?;
    let mut scan = fallible_zeroed(scan_len, "ZIP64 EOCD scan")?;
    read_at(reader, scan_start, &mut scan)?;
    if scan.len() < 4 {
        return Err(invalid_archive(
            "ZIP64 EOCD does not fit before its locator",
        ));
    }
    let mut found = None;
    for offset in 0..=scan.len() - 4 {
        if scan[offset..offset + 4] != 0x0606_4b50u32.to_le_bytes() || scan.len() - offset < 12 {
            continue;
        }
        let record_size = le_u64(&scan[offset + 4..offset + 12]);
        let Some(total_size) = record_size.checked_add(12) else {
            continue;
        };
        if total_size > MAX_ZIP64_EOCD_BYTES {
            continue;
        }
        let physical = scan_start + offset as u64;
        if physical.checked_add(total_size) == Some(locator_offset) {
            if found.is_some() {
                return Err(invalid_archive(
                    "multiple structurally valid ZIP64 EOCD records precede one locator",
                ));
            }
            found = Some((physical, offset, record_size));
        }
    }
    let (eocd64_offset, scan_offset, record_size) = found.ok_or_else(|| {
        invalid_archive("ZIP64 EOCD is missing, oversized, or inconsistent with its locator")
    })?;
    if record_size < 44 || scan.len() - scan_offset < ZIP64_EOCD_FIXED_BYTES {
        return Err(invalid_archive("truncated ZIP64 EOCD"));
    }
    let record = &scan[scan_offset..scan_offset + ZIP64_EOCD_FIXED_BYTES];
    let disk = le_u32(&record[16..20]);
    let directory_disk = le_u32(&record[20..24]);
    let entries_on_disk = le_u64(&record[24..32]);
    let entries_total = le_u64(&record[32..40]);
    let directory_size = le_u64(&record[40..48]);
    let directory_offset = le_u64(&record[48..56]);
    if disk != 0 || directory_disk != 0 || entries_on_disk != entries_total {
        return Err(invalid_archive("multi-disk ZIP64 archives are unsupported"));
    }
    if (disk_16 != u16::MAX && u32::from(disk_16) != disk)
        || (directory_disk_16 != u16::MAX && u32::from(directory_disk_16) != directory_disk)
        || (entries_on_disk_16 != u16::MAX && u64::from(entries_on_disk_16) != entries_on_disk)
        || (entries_total_16 != u16::MAX && u64::from(entries_total_16) != entries_total)
        || (directory_size_32 != u32::MAX && u64::from(directory_size_32) != directory_size)
        || (directory_offset_32 != u32::MAX && u64::from(directory_offset_32) != directory_offset)
    {
        return Err(invalid_archive("ZIP32 and ZIP64 EOCD metadata disagree"));
    }
    let archive_offset = eocd64_offset
        .checked_sub(eocd64_relative)
        .ok_or_else(|| invalid_archive("ZIP64 archive offset is inconsistent"))?;
    Ok((
        entries_total,
        directory_size,
        directory_offset,
        archive_offset,
        eocd64_offset,
    ))
}

fn invalid_archive(reason: &'static str) -> Error {
    Error::Zip(zip::result::ZipError::InvalidArchive(reason))
}

fn allocation_io(context: &'static str, error: std::collections::TryReserveError) -> Error {
    Error::Io(io::Error::new(
        io::ErrorKind::OutOfMemory,
        format!("allocating {context}: {error}"),
    ))
}

fn fallible_zeroed(length: usize, context: &'static str) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(length)
        .map_err(|error| allocation_io(context, error))?;
    bytes.resize(length, 0);
    Ok(bytes)
}

fn read_at<R: Read + Seek>(reader: &mut R, offset: u64, bytes: &mut [u8]) -> Result<()> {
    reader.seek(SeekFrom::Start(offset))?;
    read_exact(reader, bytes)
}

fn read_exact(reader: &mut impl Read, bytes: &mut [u8]) -> Result<()> {
    reader.read_exact(bytes)?;
    Ok(())
}

fn le_u16(bytes: &[u8]) -> u16 {
    u16::from_le_bytes(bytes.try_into().expect("fixed-width u16 slice"))
}

fn le_u32(bytes: &[u8]) -> u32 {
    u32::from_le_bytes(bytes.try_into().expect("fixed-width u32 slice"))
}

fn le_u64(bytes: &[u8]) -> u64 {
    u64::from_le_bytes(bytes.try_into().expect("fixed-width u64 slice"))
}

fn take_u32(bytes: &[u8], offset: &mut usize) -> Option<u32> {
    let end = offset.checked_add(4)?;
    let value = le_u32(bytes.get(*offset..end)?);
    *offset = end;
    Some(value)
}

fn take_u64(bytes: &[u8], offset: &mut usize) -> Option<u64> {
    let end = offset.checked_add(8)?;
    let value = le_u64(bytes.get(*offset..end)?);
    *offset = end;
    Some(value)
}

#[derive(Debug)]
struct ArchiveOutputLimit {
    actual: u64,
    limit: u64,
}

impl std::fmt::Display for ArchiveOutputLimit {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "archive output would grow to {} bytes; limit is {}",
            self.actual, self.limit
        )
    }
}

impl std::error::Error for ArchiveOutputLimit {}

struct BoundedWriter<W> {
    inner: W,
    position: u64,
    length: u64,
    limit: u64,
    tripped: bool,
}

impl<W: Write + Seek> BoundedWriter<W> {
    fn new(mut inner: W, limit: u64) -> io::Result<Self> {
        let position = inner.stream_position()?;
        if position > limit {
            return Err(archive_output_limit_error(position, limit));
        }
        Ok(Self {
            inner,
            position,
            length: position,
            limit,
            tripped: false,
        })
    }

    fn into_inner(self) -> W {
        self.inner
    }
}

impl<W: Write> Write for BoundedWriter<W> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let requested = u64::try_from(bytes.len())
            .map_err(|_| archive_output_limit_error(u64::MAX, self.limit))?;
        let Some(end) = self.position.checked_add(requested) else {
            self.tripped = true;
            return Err(archive_output_limit_error(u64::MAX, self.limit));
        };
        if self.tripped {
            self.position = end;
            self.length = self.length.max(end);
            return Ok(bytes.len());
        }
        if end > self.limit {
            self.tripped = true;
            return Err(archive_output_limit_error(end, self.limit));
        }
        let written = self.inner.write(bytes)?;
        self.position = self
            .position
            .checked_add(written as u64)
            .ok_or_else(|| archive_output_limit_error(u64::MAX, self.limit))?;
        self.length = self.length.max(self.position);
        Ok(written)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

impl<W: Seek> Seek for BoundedWriter<W> {
    fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
        if self.tripped {
            let next = virtual_seek(self.position, self.length, position)?;
            self.position = next;
            return Ok(next);
        }
        let next = self.inner.seek(position)?;
        if next > self.limit {
            self.tripped = true;
            self.position = next;
            self.length = self.length.max(next);
            return Err(archive_output_limit_error(next, self.limit));
        }
        self.position = next;
        Ok(next)
    }
}

fn virtual_seek(current: u64, length: u64, position: SeekFrom) -> io::Result<u64> {
    let next = match position {
        SeekFrom::Start(position) => i128::from(position),
        SeekFrom::End(delta) => i128::from(length) + i128::from(delta),
        SeekFrom::Current(delta) => i128::from(current) + i128::from(delta),
    };
    if !(0..=i128::from(u64::MAX)).contains(&next) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "invalid archive output seek",
        ));
    }
    Ok(next as u64)
}

fn archive_output_limit_error(actual: u64, limit: u64) -> io::Error {
    io::Error::other(ArchiveOutputLimit { actual, limit })
}

fn map_bounded_io(error: io::Error, kind: &'static str, limit: u64) -> Error {
    if let Some(marker) = error
        .get_ref()
        .and_then(|source| source.downcast_ref::<ArchiveOutputLimit>())
    {
        Error::LimitExceeded {
            kind,
            actual: marker.actual,
            limit: marker.limit,
        }
    } else {
        let _ = limit;
        Error::Io(error)
    }
}

fn map_bounded_zip(error: zip::result::ZipError, kind: &'static str, limit: u64) -> Error {
    match error {
        zip::result::ZipError::Io(error) => map_bounded_io(error, kind, limit),
        error => Error::Zip(error),
    }
}

#[derive(Debug, Default)]
struct FallibleBuffer {
    bytes: Vec<u8>,
    position: u64,
}

impl FallibleBuffer {
    fn into_inner(self) -> Vec<u8> {
        self.bytes
    }
}

impl Write for FallibleBuffer {
    fn write(&mut self, input: &[u8]) -> io::Result<usize> {
        let start = usize::try_from(self.position).map_err(|_| {
            io::Error::new(
                io::ErrorKind::OutOfMemory,
                "buffer position does not fit memory",
            )
        })?;
        let end = start.checked_add(input.len()).ok_or_else(|| {
            io::Error::new(io::ErrorKind::OutOfMemory, "buffer length overflowed")
        })?;
        if end > self.bytes.len() {
            self.bytes
                .try_reserve(end - self.bytes.len())
                .map_err(|error| {
                    io::Error::new(
                        io::ErrorKind::OutOfMemory,
                        format!("reserving archive output buffer: {error}"),
                    )
                })?;
            self.bytes.resize(end, 0);
        }
        self.bytes[start..end].copy_from_slice(input);
        self.position = end as u64;
        Ok(input.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Seek for FallibleBuffer {
    fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
        let base = match position {
            SeekFrom::Start(position) => {
                self.position = position;
                return Ok(position);
            }
            SeekFrom::End(delta) => self.bytes.len() as i128 + i128::from(delta),
            SeekFrom::Current(delta) => i128::from(self.position) + i128::from(delta),
        };
        if !(0..=i128::from(u64::MAX)).contains(&base) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "invalid archive buffer seek",
            ));
        }
        self.position = base as u64;
        Ok(self.position)
    }
}

impl ArchiveIndex {
    pub fn open(path: impl AsRef<Path>, limits: Limits) -> Result<Self> {
        let path = path.as_ref();
        let snapshot = snapshot_source(path, &limits)?;
        Self::from_snapshot(path, limits, snapshot)
    }

    /// Open an archive only when its exact disk snapshot matches a previously captured seal.
    ///
    /// The no-follow source handle is copied and hashed into a private bounded tempfile first.
    /// A mismatch returns [`Error::ArchiveChanged`] before ZIP metadata or payload bytes from that
    /// snapshot are parsed.
    pub fn open_with_expected_seal(
        path: impl AsRef<Path>,
        limits: Limits,
        expected: ArchiveSeal,
    ) -> Result<Self> {
        let path = path.as_ref();
        let snapshot = snapshot_source(path, &limits)?;
        if snapshot.size != expected.size || snapshot.sha256 != expected.sha256 {
            return Err(Error::ArchiveChanged);
        }
        Self::from_snapshot(path, limits, snapshot)
    }

    fn from_snapshot(path: &Path, limits: Limits, snapshot: SourceSnapshot) -> Result<Self> {
        let archive_bytes = snapshot.size;
        let archive_sha256 = snapshot.sha256;

        // `zip` 2.x parses and preallocates the complete central directory before exposing
        // `ZipArchive::len()`. Raw-preflight the private immutable snapshot first, reject parser
        // fallback candidates, and bind the parser to the validated physical archive offset.
        let mut archive = parse_snapshot(snapshot, &limits)?;
        check_limit(
            "entry count",
            archive.len() as u64,
            limits.max_entries as u64,
        )?;

        let archive_comment = archive.comment().to_vec();
        let zip64_comment = archive.zip64_comment().map(<[u8]>::to_vec);
        let mut entries = Vec::with_capacity(archive.len());
        let mut total_uncompressed_bytes = 0u64;
        for index in 0..archive.len() {
            // Raw metadata access does not request a password or instantiate a decompressor, so
            // encrypted and unsupported members remain listable and can be rejected before any
            // extraction output is created.
            let file = archive.by_index_raw(index)?;
            let path = file.name().to_owned();
            check_limit(
                "entry path bytes",
                path.len() as u64,
                limits.max_path_bytes as u64,
            )?;
            check_limit(
                "entry uncompressed bytes",
                file.size(),
                limits.max_entry_uncompressed_bytes,
            )?;
            check_ratio(file.size(), file.compressed_size(), &limits)?;
            total_uncompressed_bytes =
                total_uncompressed_bytes
                    .checked_add(file.size())
                    .ok_or(Error::LimitExceeded {
                        kind: "total uncompressed bytes",
                        actual: u64::MAX,
                        limit: limits.max_total_uncompressed_bytes,
                    })?;
            check_limit(
                "total uncompressed bytes",
                total_uncompressed_bytes,
                limits.max_total_uncompressed_bytes,
            )?;
            entries.push(entry_from_file(index, &file));
        }

        Ok(Self {
            path: path.to_path_buf(),
            entries,
            limits,
            archive_bytes,
            archive_sha256,
            archive_comment,
            zip64_comment,
            total_uncompressed_bytes,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn seal(&self) -> ArchiveSeal {
        ArchiveSeal {
            size: self.archive_bytes,
            sha256: self.archive_sha256,
        }
    }

    pub fn archive_bytes(&self) -> u64 {
        self.archive_bytes
    }

    pub fn archive_sha256(&self) -> [u8; 32] {
        self.archive_sha256
    }

    pub fn entries(&self) -> &[ArchiveEntry] {
        &self.entries
    }

    pub fn list(&self) -> impl ExactSizeIterator<Item = &ArchiveEntry> {
        self.entries.iter()
    }

    /// Resolve an entry by case-insensitive basename. When supplied, `exact_path` takes
    /// precedence and is matched case-sensitively against the complete archive path.
    pub fn resolve(&self, basename: &str, exact_path: Option<&str>) -> Result<&ArchiveEntry> {
        let (query, matches): (String, Vec<&ArchiveEntry>) = if let Some(exact_path) = exact_path {
            (
                exact_path.to_owned(),
                self.entries
                    .iter()
                    .filter(|entry| entry.path == exact_path)
                    .collect(),
            )
        } else {
            let folded = fold_case(basename);
            (
                basename.to_owned(),
                self.entries
                    .iter()
                    .filter(|entry| fold_case(&entry.basename) == folded)
                    .collect(),
            )
        };

        match matches.as_slice() {
            [] => Err(Error::NotFound { query }),
            [entry] => Ok(entry),
            _ => Err(Error::Ambiguous {
                query,
                candidates: matches
                    .iter()
                    .map(|entry| format!("{} (index {})", entry.path, entry.index))
                    .collect(),
            }),
        }
    }

    /// Extract one member without overwriting an existing output.
    ///
    /// Existing link/reparse components are rejected and path checks are repeated around creation.
    /// The selected output tree must remain under a trusted single writer during extraction because
    /// portable `std` pathname APIs cannot atomically lock every ancestor against hostile swaps.
    pub fn extract(
        &self,
        basename: &str,
        exact_path: Option<&str>,
        output_root: impl AsRef<Path>,
    ) -> Result<PathBuf> {
        let entry = self.resolve(basename, exact_path)?;
        let mut paths = self.extract_indices(&[entry.index], output_root.as_ref())?;
        Ok(paths.pop().expect("one entry was selected"))
    }

    /// Extract every member with the same output-tree safety boundary as [`Self::extract`].
    pub fn extract_all(&self, output_root: impl AsRef<Path>) -> Result<Vec<PathBuf>> {
        let indices = (0..self.entries.len()).collect::<Vec<_>>();
        self.extract_indices(&indices, output_root.as_ref())
    }

    /// Copy the archive and add or replace one validated Ogg entry.
    ///
    /// `output` must differ from the input and must not already exist. Unchanged entries are
    /// raw-copied in their original order; their compressed bytes, method, CRC, timestamp, and
    /// permission bits are retained. A replacement remains at the original index, while an add
    /// is appended.
    pub fn write_edited(
        &self,
        output: impl AsRef<Path>,
        edit: ArchiveEdit<'_>,
    ) -> Result<WriteReport> {
        let mut reports = self.write_edits(output, std::iter::once(edit))?;
        Ok(reports.pop().expect("one edit was supplied"))
    }

    /// Copy the archive and atomically apply a batch of validated Ogg edits in one pass.
    ///
    /// Every selector, target, Ogg stream, and aggregate limit is resolved before a temporary
    /// output is created. Replacements keep their original positions; additions are appended in
    /// the iterator's order. The returned reports use that same input-edit order.
    pub fn write_edits<'a, I>(&self, output: impl AsRef<Path>, edits: I) -> Result<Vec<WriteReport>>
    where
        I: IntoIterator<Item = ArchiveEdit<'a>>,
    {
        let output = output.as_ref();
        self.check_output_path(output)?;
        let parent = output
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        if !parent.is_dir() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "output parent directory does not exist",
            )
            .into());
        }

        let plans = self.plan_edits(edits.into_iter().collect())?;
        self.check_rewritable_entries()?;
        let mut temp = tempfile::Builder::new()
            .prefix(".gore-vo-")
            .suffix(".zip.tmp")
            .tempfile_in(parent)?;
        let (reports, expected) = {
            let (_, reports, expected) = self.compose_rewrite(
                plans,
                temp.as_file_mut(),
                self.limits.max_archive_bytes,
                "archive bytes",
            )?;
            (reports, expected)
        };
        temp.as_file().sync_all()?;
        self.verify_output(temp.path(), &expected)?;
        temp.persist_noclobber(output)
            .map_err(|error| Error::Io(error.error))?;

        Ok(reports
            .into_iter()
            .map(|report| WriteReport {
                output: output.to_path_buf(),
                action: report.action,
                entry_index: report.entry_index,
                archive_path: report.archive_path,
                sha256: report.sha256,
                ogg: report.ogg,
            })
            .collect())
    }

    /// Apply a batch of validated Ogg edits and return a verified ZIP entirely in memory.
    ///
    /// This uses the same preflight, ordering, raw-copy, limit, and verification path as
    /// [`Self::write_edits`], but creates no filesystem artifact. The source archive is hashed
    /// when indexed and checked both before and after composition so drift cannot be silently
    /// incorporated. Replacements keep their original indices, additions are appended in input
    /// order, and reports remain in input-edit order. The complete candidate ZIP is held in a
    /// fallibly-growing RAM buffer and is rejected before composition when the source already
    /// exceeds `max_in_memory_archive_bytes`; a counting writer enforces the smaller of that limit
    /// and `max_archive_bytes` during every write.
    pub fn rewrite_edits<'a, I>(&self, edits: I) -> Result<(Vec<u8>, Vec<RewriteReport>)>
    where
        I: IntoIterator<Item = ArchiveEdit<'a>>,
    {
        let prepared = self.prepare_rewrite(edits.into_iter().collect())?;
        Ok((prepared.bytes, prepared.reports))
    }

    /// Stream a verified rewrite into a private temporary file and return ownership of that
    /// disk-backed candidate. Dropping the returned path removes it; no requested output path or
    /// source archive is modified. All selectors and payloads are preflighted before the temporary
    /// is created, and the finished file is synced and reopened for full verification.
    pub fn rewrite_edits_to_temp<'a, I>(
        &self,
        edits: I,
    ) -> Result<(tempfile::TempPath, Vec<RewriteReport>)>
    where
        I: IntoIterator<Item = ArchiveEdit<'a>>,
    {
        let plans = self.plan_edits(edits.into_iter().collect())?;
        self.check_rewritable_entries()?;

        let mut temp = tempfile::Builder::new()
            .prefix("gore-vo-candidate-")
            .suffix(".zip")
            .tempfile()?;
        let (reports, expected) = {
            let (_, reports, expected) = self.compose_rewrite(
                plans,
                temp.as_file_mut(),
                self.limits.max_archive_bytes,
                "archive bytes",
            )?;
            (reports, expected)
        };
        temp.as_file().sync_all()?;
        self.verify_output(temp.path(), &expected)?;
        Ok((temp.into_temp_path(), reports))
    }

    fn prepare_rewrite<'a>(&self, edits: Vec<ArchiveEdit<'a>>) -> Result<PreparedRewrite> {
        let plans = self.plan_edits(edits)?;
        self.check_rewritable_entries()?;

        let memory_limit = self
            .limits
            .max_archive_bytes
            .min(self.limits.max_in_memory_archive_bytes);
        check_limit("in-memory archive bytes", self.archive_bytes, memory_limit)?;
        let (buffer, reports, expected) = self.compose_rewrite(
            plans,
            FallibleBuffer::default(),
            memory_limit,
            "in-memory archive bytes",
        )?;
        let bytes = buffer.into_inner();
        self.verify_output_bytes(&bytes, &expected)?;

        Ok(PreparedRewrite { bytes, reports })
    }

    fn compose_rewrite<'a, W>(
        &self,
        plans: Vec<EditPlan<'a>>,
        output: W,
        max_output_bytes: u64,
        output_limit_kind: &'static str,
    ) -> Result<(W, Vec<RewriteReport>, Vec<ExpectedEntry>)>
    where
        W: Write + Seek,
    {
        let mut input = self.open_current()?;
        let bounded = BoundedWriter::new(output, max_output_bytes)
            .map_err(|error| map_bounded_io(error, output_limit_kind, max_output_bytes))?;
        let mut writer = ZipWriter::new(bounded);
        writer.set_raw_comment(self.archive_comment.clone().into_boxed_slice());
        writer.set_raw_zip64_comment(self.zip64_comment.clone().map(Vec::into_boxed_slice));

        let replacements = plans
            .iter()
            .enumerate()
            .filter_map(|(plan_index, plan)| {
                plan.replacement_index.map(|index| (index, plan_index))
            })
            .collect::<BTreeMap<_, _>>();
        let add_count = plans
            .iter()
            .filter(|plan| plan.replacement_index.is_none())
            .count();
        let mut expected = Vec::with_capacity(self.entries.len() + add_count);
        for index in 0..input.len() {
            let file = input.by_index_raw(index)?;
            if let Some(plan_index) = replacements.get(&index) {
                let plan = &plans[*plan_index];
                let options = file.options();
                let modified = effective_modified(file.last_modified());
                let permissions = effective_permissions(file.unix_mode());
                writer
                    .start_file(file.name(), options)
                    .map_err(|error| map_bounded_zip(error, output_limit_kind, max_output_bytes))?;
                writer
                    .write_all(plan.ogg)
                    .map_err(|error| map_bounded_io(error, output_limit_kind, max_output_bytes))?;
                expected.push(ExpectedEntry {
                    path: file.name().to_owned(),
                    compression: plan.compression,
                    uncompressed_size: Some(plan.ogg.len() as u64),
                    compressed_size: None,
                    crc32: None,
                    modified,
                    permissions,
                    sha256: Some(plan.sha256),
                });
            } else {
                expected.push(expected_untouched(&file));
                writer
                    .raw_copy_file(file)
                    .map_err(|error| map_bounded_zip(error, output_limit_kind, max_output_bytes))?;
            }
        }

        for plan in plans.iter().filter(|plan| plan.replacement_index.is_none()) {
            let options = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored)
                .unix_permissions(0o644);
            writer
                .start_file(&plan.archive_path, options)
                .map_err(|error| map_bounded_zip(error, output_limit_kind, max_output_bytes))?;
            writer
                .write_all(plan.ogg)
                .map_err(|error| map_bounded_io(error, output_limit_kind, max_output_bytes))?;
            expected.push(ExpectedEntry {
                path: plan.archive_path.clone(),
                compression: zip::CompressionMethod::Stored,
                uncompressed_size: Some(plan.ogg.len() as u64),
                compressed_size: None,
                crc32: None,
                modified: DateTime::default_for_write(),
                permissions: Some(0o644),
                sha256: Some(plan.sha256),
            });
        }

        let output = writer
            .finish()
            .map_err(|error| map_bounded_zip(error, output_limit_kind, max_output_bytes))?
            .into_inner();
        drop(input);
        self.check_current_hash()?;

        let reports = plans
            .into_iter()
            .map(|plan| RewriteReport {
                action: plan.action,
                entry_index: plan.output_index,
                archive_path: plan.archive_path,
                sha256: plan.sha256,
                ogg: plan.ogg_info,
            })
            .collect();

        Ok((output, reports, expected))
    }

    fn plan_edits<'a>(&self, edits: Vec<ArchiveEdit<'a>>) -> Result<Vec<EditPlan<'a>>> {
        if edits.is_empty() {
            return Err(Error::EmptyEditBatch);
        }

        let mut plans = Vec::with_capacity(edits.len());
        let mut targets = BTreeMap::<String, String>::new();
        for edit in edits {
            let (replacement_index, archive_path, ogg, action, compression) = match edit {
                ArchiveEdit::Add { path, ogg } => {
                    validate_archive_entry_path(path, &self.limits)?;
                    ensure_ogg_path(path)?;
                    (
                        None,
                        path.to_owned(),
                        ogg,
                        EditAction::Added,
                        zip::CompressionMethod::Stored,
                    )
                }
                ArchiveEdit::Replace {
                    basename,
                    exact_path,
                    ogg,
                } => {
                    let entry = self.resolve(basename, exact_path)?;
                    if entry.is_directory {
                        return Err(Error::NotOggPath(entry.path.clone()));
                    }
                    validate_archive_entry_path(&entry.path, &self.limits)?;
                    ensure_ogg_path(&entry.path)?;
                    if !matches!(
                        entry.compression,
                        zip::CompressionMethod::Stored | zip::CompressionMethod::Deflated
                    ) {
                        return Err(Error::UnsupportedCompression {
                            path: entry.path.clone(),
                            method: entry.compression,
                        });
                    }
                    (
                        Some(entry.index),
                        entry.path.clone(),
                        ogg,
                        EditAction::Replaced,
                        entry.compression,
                    )
                }
            };

            check_limit(
                "entry uncompressed bytes",
                ogg.len() as u64,
                self.limits.max_entry_uncompressed_bytes,
            )?;
            let ogg_info = validate_ogg(ogg, &self.limits)?;
            let folded_target = target_key(&archive_path);
            if let Some(first) = targets.insert(folded_target, archive_path.clone()) {
                return Err(Error::ConflictingEdits {
                    first,
                    second: archive_path,
                });
            }
            plans.push(EditPlan {
                replacement_index,
                output_index: usize::MAX,
                archive_path,
                ogg,
                ogg_info,
                sha256: Sha256::digest(ogg).into(),
                action,
                compression,
            });
        }

        for plan in plans.iter().filter(|plan| plan.replacement_index.is_none()) {
            if let Some(existing) = self
                .entries
                .iter()
                .find(|entry| target_key(&entry.path) == target_key(&plan.archive_path))
            {
                return Err(Error::EntryAlreadyExists(existing.path.clone()));
            }
        }

        let add_count = plans
            .iter()
            .filter(|plan| plan.replacement_index.is_none())
            .count();
        let output_count =
            self.entries
                .len()
                .checked_add(add_count)
                .ok_or(Error::LimitExceeded {
                    kind: "entry count",
                    actual: u64::MAX,
                    limit: self.limits.max_entries as u64,
                })?;
        check_limit(
            "entry count",
            output_count as u64,
            self.limits.max_entries as u64,
        )?;

        let removed_bytes = plans
            .iter()
            .filter_map(|plan| plan.replacement_index)
            .try_fold(0u64, |total, index| {
                total
                    .checked_add(self.entries[index].uncompressed_size)
                    .ok_or(Error::LimitExceeded {
                        kind: "total uncompressed bytes",
                        actual: u64::MAX,
                        limit: self.limits.max_total_uncompressed_bytes,
                    })
            })?;
        let inserted_bytes = plans.iter().try_fold(0u64, |total, plan| {
            total
                .checked_add(plan.ogg.len() as u64)
                .ok_or(Error::LimitExceeded {
                    kind: "total uncompressed bytes",
                    actual: u64::MAX,
                    limit: self.limits.max_total_uncompressed_bytes,
                })
        })?;
        let total = self
            .total_uncompressed_bytes
            .checked_sub(removed_bytes)
            .and_then(|value| value.checked_add(inserted_bytes))
            .ok_or(Error::LimitExceeded {
                kind: "total uncompressed bytes",
                actual: u64::MAX,
                limit: self.limits.max_total_uncompressed_bytes,
            })?;
        check_limit(
            "total uncompressed bytes",
            total,
            self.limits.max_total_uncompressed_bytes,
        )?;

        let mut next_add_index = self.entries.len();
        for plan in &mut plans {
            if let Some(index) = plan.replacement_index {
                plan.output_index = index;
            } else {
                plan.output_index = next_add_index;
                next_add_index += 1;
            }
        }
        Ok(plans)
    }

    fn check_output_path(&self, output: &Path) -> Result<()> {
        if output.exists() {
            if fs::canonicalize(output)? == fs::canonicalize(&self.path)? {
                return Err(Error::InputOutputSame(output.to_path_buf()));
            }
            return Err(Error::OutputExists(output.to_path_buf()));
        }
        Ok(())
    }

    fn extract_indices(&self, indices: &[usize], output_root: &Path) -> Result<Vec<PathBuf>> {
        let plans = self.plan_extraction(indices)?;
        let mut archive = self.open_current()?;
        let root = prepare_output_root(output_root)?;

        let mut outputs = Vec::with_capacity(plans.len());
        for plan in plans {
            if plan.is_directory {
                let directory = ensure_directory_chain(&root, &plan.components)?;
                outputs.push(directory);
                continue;
            }

            let (filename, parents) = plan
                .components
                .split_last()
                .expect("safe file path is non-empty");
            let parent = ensure_directory_chain(&root, parents)?;
            validate_output_root_ancestors(&parent)?;
            let canonical_parent =
                fs::canonicalize(&parent).map_err(|source| output_io(&parent, source))?;
            if !canonical_parent.starts_with(&root) {
                return Err(Error::UnsafeOutput {
                    path: parent,
                    reason: "parent resolves outside extraction root",
                });
            }
            let output = parent.join(filename);
            let mut source = archive.by_index(plan.index)?;
            let mut destination = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&output)
                .map_err(|source| {
                    if source.kind() == io::ErrorKind::AlreadyExists {
                        Error::OutputExists(output.clone())
                    } else {
                        output_io(&output, source)
                    }
                })?;

            // Revalidate immediately after the pathname-based open and before writing bytes. This
            // fails closed for persistent swaps; see `validate_output_root_ancestors` for the
            // trusted-single-writer boundary of portable path APIs.
            if let Err(error) = validate_output_parent(&root, &parent) {
                drop(destination);
                return Err(error);
            }
            let copy_result = (|| -> Result<()> {
                copy_source_exact(
                    &mut source,
                    &mut destination,
                    &self.path,
                    &output,
                    self.entries[plan.index].uncompressed_size,
                )?;
                destination
                    .sync_all()
                    .map_err(|source| output_io(&output, source))?;
                validate_output_parent(&root, &parent)?;
                Ok(())
            })();
            if let Err(error) = copy_result {
                drop(destination);
                if validate_output_parent(&root, &parent).is_ok() {
                    let _ = fs::remove_file(&output);
                }
                return Err(error);
            }
            outputs.push(output);
        }
        Ok(outputs)
    }

    fn plan_extraction(&self, indices: &[usize]) -> Result<Vec<ExtractionPlan>> {
        let mut plans = Vec::with_capacity(indices.len());
        let mut paths = BTreeMap::<String, bool>::new();
        for index in indices {
            let entry = self.entries.get(*index).ok_or_else(|| {
                Error::Verification(format!("entry index {index} is out of range"))
            })?;
            if entry.encrypted {
                return Err(Error::EncryptedEntry(entry.path.clone()));
            }
            if entry.is_symlink {
                return Err(Error::SymlinkEntry(entry.path.clone()));
            }
            if !entry.is_directory
                && !matches!(
                    entry.compression,
                    zip::CompressionMethod::Stored | zip::CompressionMethod::Deflated
                )
            {
                return Err(Error::UnsupportedCompression {
                    path: entry.path.clone(),
                    method: entry.compression,
                });
            }
            let components = safe_components(&entry.path, entry.is_directory, &self.limits)?;
            let folded = fold_case(&components.join("/"));
            if paths.insert(folded, entry.is_directory).is_some() {
                return Err(Error::UnsafePath {
                    path: entry.path.clone(),
                    reason: "duplicate or case-colliding extraction path",
                });
            }
            plans.push(ExtractionPlan {
                index: *index,
                components,
                is_directory: entry.is_directory,
                source_path: entry.path.clone(),
            });
        }

        for plan in &plans {
            for end in 1..plan.components.len() {
                let parent = fold_case(&plan.components[..end].join("/"));
                if paths.get(&parent) == Some(&false) {
                    return Err(Error::UnsafePath {
                        path: plan.source_path.clone(),
                        reason: "a file entry is the parent of another entry",
                    });
                }
            }
        }
        Ok(plans)
    }

    fn check_rewritable_entries(&self) -> Result<()> {
        // A rewrite raw-copies untouched members, so it must enforce the same complete portable
        // path/collision/file-parent gate as extraction. Otherwise editing one safe Ogg could
        // publish an archive that still contains traversal, Win32 alias, or colliding members.
        let indices = (0..self.entries.len()).collect::<Vec<_>>();
        self.plan_extraction(&indices)?;
        Ok(())
    }

    fn open_current(&self) -> Result<ZipArchive<File>> {
        // Capture and seal the mutable path first. All following metadata and payload reads are
        // from this private tempfile, never from the source descriptor that was hashed.
        let snapshot = self.snapshot_current()?;
        let mut archive = parse_snapshot(snapshot, &self.limits)?;
        if archive.len() != self.entries.len()
            || archive.comment() != self.archive_comment
            || archive.zip64_comment() != self.zip64_comment.as_deref()
        {
            return Err(Error::ArchiveChanged);
        }
        for expected in &self.entries {
            let current = archive.by_index_raw(expected.index)?;
            let matches = current.name() == expected.path
                && current.compressed_size() == expected.compressed_size
                && current.size() == expected.uncompressed_size
                && current.crc32() == expected.crc32
                && current.compression() == expected.compression
                && current.last_modified() == expected.last_modified
                && current.unix_mode() == expected.unix_mode
                && current.is_dir() == expected.is_directory
                && current.is_symlink() == expected.is_symlink
                && current.encrypted() == expected.encrypted;
            if !matches {
                return Err(Error::ArchiveChanged);
            }
        }
        Ok(archive)
    }

    fn check_current_hash(&self) -> Result<()> {
        self.snapshot_current().map(drop)
    }

    fn snapshot_current(&self) -> Result<SourceSnapshot> {
        let snapshot = snapshot_source(&self.path, &self.limits)?;
        if snapshot.size != self.archive_bytes || snapshot.sha256 != self.archive_sha256 {
            return Err(Error::ArchiveChanged);
        }
        Ok(snapshot)
    }

    fn verify_output(&self, path: &Path, expected: &[ExpectedEntry]) -> Result<()> {
        let archive_bytes = fs::metadata(path)?.len();
        let file = File::open(path)?;
        self.verify_output_reader(file, archive_bytes, expected)
    }

    fn verify_output_bytes(&self, bytes: &[u8], expected: &[ExpectedEntry]) -> Result<()> {
        self.verify_output_reader(Cursor::new(bytes), bytes.len() as u64, expected)
    }

    fn verify_output_reader<R: Read + Seek>(
        &self,
        mut reader: R,
        archive_bytes: u64,
        expected: &[ExpectedEntry],
    ) -> Result<()> {
        check_limit(
            "archive bytes",
            archive_bytes,
            self.limits.max_archive_bytes,
        )?;
        let raw = preflight_raw_zip(&mut reader, archive_bytes, &self.limits)?;
        reader.seek(SeekFrom::Start(0))?;
        let config = ReadConfig {
            archive_offset: ArchiveOffset::Known(raw.archive_offset),
        };
        let mut archive = ZipArchive::with_config(config, reader)?;
        if archive.offset() != raw.archive_offset
            || archive.central_directory_start() != raw.central_directory_start
            || archive.len() != raw.entry_count
        {
            return Err(Error::Verification(
                "generated ZIP parser metadata differs from raw preflight".to_owned(),
            ));
        }
        if archive.len() != expected.len() {
            return Err(Error::Verification(format!(
                "entry count is {}, expected {}",
                archive.len(),
                expected.len()
            )));
        }
        if archive.comment() != self.archive_comment
            || archive.zip64_comment() != self.zip64_comment.as_deref()
        {
            return Err(Error::Verification(
                "archive comment metadata changed".to_owned(),
            ));
        }

        let mut total_uncompressed = 0u64;
        for (index, expected) in expected.iter().enumerate() {
            let actual = archive.by_index_raw(index)?;
            check_limit(
                "entry uncompressed bytes",
                actual.size(),
                self.limits.max_entry_uncompressed_bytes,
            )?;
            check_ratio(actual.size(), actual.compressed_size(), &self.limits)?;
            total_uncompressed =
                total_uncompressed
                    .checked_add(actual.size())
                    .ok_or(Error::LimitExceeded {
                        kind: "total uncompressed bytes",
                        actual: u64::MAX,
                        limit: self.limits.max_total_uncompressed_bytes,
                    })?;
            if actual.name() != expected.path
                || actual.compression() != expected.compression
                || effective_modified(actual.last_modified()) != expected.modified
                || actual.unix_mode().map(|mode| mode & 0o777) != expected.permissions
            {
                return Err(Error::Verification(format!(
                    "entry metadata differs at index {index}"
                )));
            }
            if expected
                .uncompressed_size
                .is_some_and(|size| actual.size() != size)
                || expected
                    .compressed_size
                    .is_some_and(|size| actual.compressed_size() != size)
                || expected.crc32.is_some_and(|crc| actual.crc32() != crc)
            {
                return Err(Error::Verification(format!(
                    "entry size or CRC differs at index {index}"
                )));
            }
            drop(actual);
            if let Some(expected_hash) = expected.sha256 {
                let mut actual = archive.by_index(index).map_err(|error| {
                    Error::Verification(format!(
                        "opening edited entry payload at index {index} failed: {error}"
                    ))
                })?;
                let actual_hash = hash_reader(&mut actual).map_err(|error| {
                    Error::Verification(format!(
                        "reading edited entry payload at index {index} failed: {error}"
                    ))
                })?;
                if actual_hash != expected_hash {
                    return Err(Error::Verification(format!(
                        "edited entry hash differs at index {index}"
                    )));
                }
            }
        }
        check_limit(
            "total uncompressed bytes",
            total_uncompressed,
            self.limits.max_total_uncompressed_bytes,
        )?;
        Ok(())
    }
}

fn entry_from_file(index: usize, file: &zip::read::ZipFile<'_>) -> ArchiveEntry {
    ArchiveEntry {
        index,
        path: file.name().to_owned(),
        basename: basename(file.name()).to_owned(),
        compressed_size: file.compressed_size(),
        uncompressed_size: file.size(),
        crc32: file.crc32(),
        compression: file.compression(),
        last_modified: file.last_modified(),
        unix_mode: file.unix_mode(),
        is_directory: file.is_dir(),
        is_symlink: file.is_symlink(),
        encrypted: file.encrypted(),
    }
}

fn expected_untouched(file: &zip::read::ZipFile<'_>) -> ExpectedEntry {
    ExpectedEntry {
        path: file.name().to_owned(),
        compression: file.compression(),
        uncompressed_size: Some(file.size()),
        compressed_size: Some(file.compressed_size()),
        crc32: Some(file.crc32()),
        modified: effective_modified(file.last_modified()),
        permissions: effective_permissions(file.unix_mode()),
        sha256: None,
    }
}

fn effective_modified(value: Option<DateTime>) -> DateTime {
    value
        .filter(DateTime::is_valid)
        .unwrap_or_else(DateTime::default_for_write)
}

fn effective_permissions(value: Option<u32>) -> Option<u32> {
    Some(value.unwrap_or(0o644) & 0o777)
}

fn check_limit(kind: &'static str, actual: u64, limit: u64) -> Result<()> {
    if actual > limit {
        return Err(Error::LimitExceeded {
            kind,
            actual,
            limit,
        });
    }
    Ok(())
}

fn check_ratio(uncompressed: u64, compressed: u64, limits: &Limits) -> Result<()> {
    if uncompressed == 0 {
        return Ok(());
    }
    let allowed = compressed.saturating_mul(limits.max_compression_ratio);
    if compressed == 0 || uncompressed > allowed {
        return Err(Error::LimitExceeded {
            kind: "compression ratio",
            actual: uncompressed,
            limit: allowed,
        });
    }
    Ok(())
}

fn basename(path: &str) -> &str {
    path.rsplit(['/', '\\']).next().unwrap_or(path)
}

fn fold_case(value: &str) -> String {
    value.to_lowercase()
}

fn target_key(value: &str) -> String {
    fold_case(&value.replace('\\', "/"))
}

fn ensure_ogg_path(path: &str) -> Result<()> {
    if basename(path).to_ascii_lowercase().ends_with(".ogg") {
        Ok(())
    } else {
        Err(Error::NotOggPath(path.to_owned()))
    }
}

/// Validate a canonical, portable file-entry path for a voice archive.
///
/// Voice member names use forward slashes on every platform. In addition to traversal and
/// absolute-path checks, this rejects spellings Win32 aliases or cannot create (alternate data
/// streams, reserved device names, trailing dots/spaces, and invalid filename characters).
/// `limits.max_path_bytes` is applied to the UTF-8 ZIP member name before any allocation.
pub fn validate_archive_entry_path(path: &str, limits: &Limits) -> Result<()> {
    safe_components(path, false, limits).map(drop)
}

/// Reject every currently existing symbolic-link or Windows reparse-point component of an
/// extraction root. Missing suffix components are allowed and are created by extraction.
///
/// This closes static link/junction escapes. The implementation revalidates around every
/// path-based create/open, but portable `std` APIs cannot make a multi-component creation atomic;
/// callers must keep the selected output tree under a trusted single writer while extraction runs.
pub fn validate_output_root_ancestors(path: impl AsRef<Path>) -> Result<()> {
    let path = path.as_ref();
    if path.as_os_str().is_empty() {
        return Err(Error::UnsafeOutput {
            path: path.to_path_buf(),
            reason: "output root is empty",
        });
    }
    // A missing component followed by `..` can make `create_dir_all` touch a different lexical
    // branch before post-creation validation. Picker/FFI roots are absolute, so reject this
    // ambiguous spelling instead of normalizing through an untrusted filesystem.
    if path
        .components()
        .any(|component| component == std::path::Component::ParentDir)
    {
        return Err(Error::UnsafeOutput {
            path: path.to_path_buf(),
            reason: "output root contains a parent component",
        });
    }
    let absolute = absolute_output_path(path)?;
    for ancestor in absolute.ancestors().collect::<Vec<_>>().into_iter().rev() {
        if ancestor.as_os_str().is_empty() {
            continue;
        }
        match fs::symlink_metadata(ancestor) {
            Ok(metadata) => {
                if metadata.file_type().is_symlink() || metadata_is_reparse_point(&metadata) {
                    return Err(Error::UnsafeOutput {
                        path: ancestor.to_path_buf(),
                        reason: "output path contains a symbolic link or reparse point",
                    });
                }
                if !metadata.is_dir() {
                    return Err(Error::UnsafeOutput {
                        path: ancestor.to_path_buf(),
                        reason: "an existing output path component is not a directory",
                    });
                }
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(source) => return Err(output_io(ancestor, source)),
        }
    }
    Ok(())
}

fn absolute_output_path(path: &Path) -> Result<PathBuf> {
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        let current = std::env::current_dir().map_err(|source| output_io(path, source))?;
        Ok(current.join(path))
    }
}

fn safe_components(path: &str, is_directory: bool, limits: &Limits) -> Result<Vec<String>> {
    check_limit(
        "entry path bytes",
        path.len() as u64,
        limits.max_path_bytes as u64,
    )?;
    if path.is_empty() || path.starts_with(['/', '\\']) {
        return Err(Error::UnsafePath {
            path: path.to_owned(),
            reason: "path is empty or absolute",
        });
    }
    if path.contains('\0') || path.chars().any(char::is_control) {
        return Err(Error::UnsafePath {
            path: path.to_owned(),
            reason: "path contains NUL or control characters",
        });
    }
    if path.contains('\\') {
        return Err(Error::UnsafePath {
            path: path.to_owned(),
            reason: "path must use forward-slash separators",
        });
    }
    let mut raw = path.split('/').collect::<Vec<_>>();
    if is_directory && raw.last() == Some(&"") {
        raw.pop();
    }
    if raw.is_empty() || raw.iter().any(|component| component.is_empty()) {
        return Err(Error::UnsafePath {
            path: path.to_owned(),
            reason: "path contains an empty component",
        });
    }

    let mut components = Vec::with_capacity(raw.len());
    for component in raw {
        if component == "." || component == ".." {
            return Err(Error::UnsafePath {
                path: path.to_owned(),
                reason: "path contains a dot or parent component",
            });
        }
        if component.contains(':') {
            return Err(Error::UnsafePath {
                path: path.to_owned(),
                reason: "path contains a drive or alternate-data-stream separator",
            });
        }
        if component
            .chars()
            .any(|character| matches!(character, '<' | '>' | '"' | '|' | '?' | '*'))
        {
            return Err(Error::UnsafePath {
                path: path.to_owned(),
                reason: "path contains a character invalid in Windows filenames",
            });
        }
        if component.ends_with([' ', '.']) {
            return Err(Error::UnsafePath {
                path: path.to_owned(),
                reason: "path component ends with a space or dot",
            });
        }
        if is_windows_reserved(component) {
            return Err(Error::UnsafePath {
                path: path.to_owned(),
                reason: "path contains a reserved Windows device name",
            });
        }
        components.push(component.to_owned());
    }
    Ok(components)
}

fn is_windows_reserved(component: &str) -> bool {
    // Win32 device parsing ignores spaces/dots immediately before an extension as well as a final
    // extension (`CON .txt`, `LPT1...ogg`). Trim the device stem before comparing aliases.
    let stem = component
        .split('.')
        .next()
        .unwrap_or(component)
        .trim_end_matches([' ', '.']);
    let folded = stem.to_ascii_uppercase();
    if matches!(
        folded.as_str(),
        "CON" | "PRN" | "AUX" | "NUL" | "CLOCK$" | "CONIN$" | "CONOUT$"
    ) {
        return true;
    }
    folded
        .strip_prefix("COM")
        .or_else(|| folded.strip_prefix("LPT"))
        .is_some_and(|suffix| {
            matches!(
                suffix,
                "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³"
            )
        })
}

fn prepare_output_root(path: &Path) -> Result<PathBuf> {
    let absolute = absolute_output_path(path)?;
    validate_output_root_ancestors(&absolute)?;
    fs::create_dir_all(&absolute).map_err(|source| output_io(&absolute, source))?;
    // Fail closed if a static link/reparse component appeared during creation. These path-based
    // rechecks assume no hostile concurrent writer toggles components between every observation.
    validate_output_root_ancestors(&absolute)?;
    let root = fs::canonicalize(&absolute).map_err(|source| output_io(&absolute, source))?;
    validate_output_root_ancestors(&absolute)?;
    let confirmed = fs::canonicalize(&absolute).map_err(|source| output_io(&absolute, source))?;
    if confirmed != root {
        return Err(Error::UnsafeOutput {
            path: absolute,
            reason: "output root changed while it was validated",
        });
    }
    validate_output_root_ancestors(&root)?;
    Ok(root)
}

fn ensure_directory_chain(root: &Path, components: &[String]) -> Result<PathBuf> {
    let mut current = root.to_path_buf();
    for component in components {
        current.push(component);
        match fs::symlink_metadata(&current) {
            Ok(metadata) => {
                if metadata.file_type().is_symlink()
                    || metadata_is_reparse_point(&metadata)
                    || !metadata.is_dir()
                {
                    return Err(Error::UnsafeOutput {
                        path: current,
                        reason: "extraction parent is a link, reparse point, or non-directory",
                    });
                }
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                fs::create_dir(&current).map_err(|source| output_io(&current, source))?;
            }
            Err(source) => return Err(output_io(&current, source)),
        }
        validate_output_root_ancestors(&current)?;
        let canonical = fs::canonicalize(&current).map_err(|source| output_io(&current, source))?;
        if !canonical.starts_with(root) {
            return Err(Error::UnsafeOutput {
                path: current,
                reason: "directory resolves outside extraction root",
            });
        }
    }
    validate_output_root_ancestors(&current)?;
    Ok(current)
}

fn validate_output_parent(root: &Path, parent: &Path) -> Result<()> {
    validate_output_root_ancestors(parent)?;
    let canonical = fs::canonicalize(parent).map_err(|source| output_io(parent, source))?;
    if !canonical.starts_with(root) {
        return Err(Error::UnsafeOutput {
            path: parent.to_path_buf(),
            reason: "output parent changed or resolves outside extraction root",
        });
    }
    Ok(())
}

fn copy_source_exact(
    source: &mut impl Read,
    destination: &mut File,
    archive_path: &Path,
    output_path: &Path,
    expected: u64,
) -> Result<()> {
    let mut remaining = expected;
    let mut buffer = [0u8; 64 * 1024];
    while remaining != 0 {
        let wanted = usize::try_from(remaining.min(buffer.len() as u64))
            .expect("bounded extraction read fits usize");
        let read = source
            .read(&mut buffer[..wanted])
            .map_err(|source| archive_data(archive_path, source))?;
        if read == 0 {
            return Err(Error::Verification(format!(
                "entry extraction ended with {remaining} expected bytes remaining"
            )));
        }
        destination
            .write_all(&buffer[..read])
            .map_err(|source| output_io(output_path, source))?;
        remaining -= read as u64;
    }

    let mut probe = [0u8; 1];
    if source
        .read(&mut probe)
        .map_err(|source| archive_data(archive_path, source))?
        != 0
    {
        return Err(Error::Verification(
            "entry extraction exceeded its indexed uncompressed size".into(),
        ));
    }
    Ok(())
}

fn source_io(path: &Path, source: io::Error) -> Error {
    Error::SourceIo {
        path: path.to_path_buf(),
        source,
    }
}

fn archive_data(path: &Path, source: io::Error) -> Error {
    Error::ArchiveData {
        path: path.display().to_string(),
        source,
    }
}

fn output_io(path: &Path, source: io::Error) -> Error {
    Error::OutputIo {
        path: path.to_path_buf(),
        source,
    }
}

fn open_source_file(path: &Path) -> Result<File> {
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;

        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC);
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt as _;
        use windows_sys::Win32::Storage::FileSystem::{
            FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT, FILE_SHARE_READ,
        };

        // OPEN_REPARSE_POINT makes the handle describe the link/junction itself. BACKUP_SEMANTICS
        // also lets us open a directory so it can be rejected from handle metadata rather than
        // being misreported as an arbitrary access failure. Omitting WRITE/DELETE sharing keeps a
        // validated archive immutable while this Windows handle is alive.
        options
            .share_mode(FILE_SHARE_READ)
            .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT | FILE_FLAG_BACKUP_SEMANTICS);
    }

    let file = options.open(path).map_err(|source| {
        #[cfg(unix)]
        if source.raw_os_error() == Some(libc::ELOOP) {
            return Error::UnsafeSource {
                path: path.to_path_buf(),
                reason: "source is a symbolic link",
            };
        }
        source_io(path, source)
    })?;
    source_metadata(&file, path)?;
    Ok(file)
}

fn source_metadata(file: &File, path: &Path) -> Result<fs::Metadata> {
    let metadata = file.metadata().map_err(|source| source_io(path, source))?;
    if metadata.file_type().is_symlink() || metadata_is_reparse_point(&metadata) {
        return Err(Error::UnsafeSource {
            path: path.to_path_buf(),
            reason: "source is a symbolic link or reparse point",
        });
    }
    if !metadata.is_file() {
        return Err(Error::UnsafeSource {
            path: path.to_path_buf(),
            reason: "source is not a regular file",
        });
    }
    Ok(metadata)
}

fn ensure_source_revision(file: &File, path: &Path, initial: &fs::Metadata) -> Result<()> {
    let current = source_metadata(file, path)?;
    if current.len() != initial.len() || current.modified().ok() != initial.modified().ok() {
        return Err(Error::ArchiveChanged);
    }
    Ok(())
}

#[cfg(windows)]
fn metadata_is_reparse_point(metadata: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt as _;
    use windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_REPARSE_POINT;

    metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

#[cfg(not(windows))]
fn metadata_is_reparse_point(_metadata: &fs::Metadata) -> bool {
    false
}

fn hash_reader(reader: &mut impl Read) -> io::Result<[u8; 32]> {
    let mut hash = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hash.update(&buffer[..read]);
    }
    Ok(hash.finalize().into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ogg::tests::vorbis_ogg;
    use tempfile::TempDir;
    use zip::write::SimpleFileOptions;

    struct FixtureEntry<'a> {
        name: &'a str,
        bytes: &'a [u8],
        method: zip::CompressionMethod,
    }

    fn make_archive(path: &Path, entries: &[FixtureEntry<'_>]) {
        let file = File::create(path).unwrap();
        let mut writer = ZipWriter::new(file);
        writer.set_raw_comment(b"gore-vo fixture".to_vec().into_boxed_slice());
        for entry in entries {
            let options = SimpleFileOptions::default()
                .compression_method(entry.method)
                .last_modified_time(DateTime::from_date_and_time(2025, 6, 7, 8, 9, 10).unwrap())
                .unix_permissions(0o640);
            writer.start_file(entry.name, options).unwrap();
            writer.write_all(entry.bytes).unwrap();
        }
        writer.finish().unwrap().sync_all().unwrap();
    }

    fn file_hash(path: &Path) -> [u8; 32] {
        let mut file = File::open(path).unwrap();
        hash_reader(&mut file).unwrap()
    }

    fn find_eocd(bytes: &[u8]) -> usize {
        bytes
            .windows(4)
            .rposition(|window| window == 0x0605_4b50u32.to_le_bytes())
            .expect("fixture EOCD")
    }

    fn first_entry_layout(bytes: &[u8]) -> (usize, usize, usize, usize) {
        let eocd = find_eocd(bytes);
        let directory_size = usize::try_from(le_u32(&bytes[eocd + 12..eocd + 16])).unwrap();
        let directory_relative = usize::try_from(le_u32(&bytes[eocd + 16..eocd + 20])).unwrap();
        let central = eocd - directory_size;
        assert_eq!(bytes[central..central + 4], 0x0201_4b50u32.to_le_bytes());
        let archive_offset = central - directory_relative;
        let local_relative = usize::try_from(le_u32(&bytes[central + 42..central + 46])).unwrap();
        let local = archive_offset + local_relative;
        assert_eq!(bytes[local..local + 4], 0x0403_4b50u32.to_le_bytes());
        let name_len = usize::from(le_u16(&bytes[local + 26..local + 28]));
        let extra_len = usize::from(le_u16(&bytes[local + 28..local + 30]));
        let payload = local + 30 + name_len + extra_len;
        let compressed = usize::try_from(le_u32(&bytes[central + 20..central + 24])).unwrap();
        (central, local, payload, compressed)
    }

    fn promote_fixture_to_zip64(path: &Path) {
        let bytes = fs::read(path).unwrap();
        let eocd_offset = find_eocd(&bytes);
        let eocd = &bytes[eocd_offset..eocd_offset + ZIP_EOCD_FIXED_BYTES];
        let entries = le_u16(&eocd[10..12]);
        let directory_size = le_u32(&eocd[12..16]);
        let directory_offset = le_u32(&eocd[16..20]);
        let comment_len = usize::from(le_u16(&eocd[20..22]));
        assert_eq!(
            eocd_offset + ZIP_EOCD_FIXED_BYTES + comment_len,
            bytes.len()
        );

        let mut output = bytes[..eocd_offset].to_vec();
        let eocd64_offset = output.len() as u64;
        output.extend_from_slice(&0x0606_4b50u32.to_le_bytes());
        output.extend_from_slice(&44u64.to_le_bytes());
        output.extend_from_slice(&45u16.to_le_bytes());
        output.extend_from_slice(&45u16.to_le_bytes());
        output.extend_from_slice(&0u32.to_le_bytes());
        output.extend_from_slice(&0u32.to_le_bytes());
        output.extend_from_slice(&u64::from(entries).to_le_bytes());
        output.extend_from_slice(&u64::from(entries).to_le_bytes());
        output.extend_from_slice(&u64::from(directory_size).to_le_bytes());
        output.extend_from_slice(&u64::from(directory_offset).to_le_bytes());
        output.extend_from_slice(&0x0706_4b50u32.to_le_bytes());
        output.extend_from_slice(&0u32.to_le_bytes());
        output.extend_from_slice(&eocd64_offset.to_le_bytes());
        output.extend_from_slice(&1u32.to_le_bytes());

        let mut saturated = eocd.to_vec();
        saturated[8..12].fill(0xff);
        saturated[12..20].fill(0xff);
        output.extend_from_slice(&saturated);
        output.extend_from_slice(&bytes[eocd_offset + ZIP_EOCD_FIXED_BYTES..]);
        fs::write(path, output).unwrap();
    }

    #[test]
    fn raw_preflight_rejects_declared_entry_count_before_zip_parser() {
        let temp = TempDir::new().unwrap();
        let input = temp.path().join("declared-count.zip");
        make_archive(
            &input,
            &[FixtureEntry {
                name: "line.ogg",
                bytes: b"voice",
                method: zip::CompressionMethod::Stored,
            }],
        );
        let mut bytes = fs::read(&input).unwrap();
        let eocd = find_eocd(&bytes);
        bytes[eocd + 8..eocd + 10].copy_from_slice(&2u16.to_le_bytes());
        bytes[eocd + 10..eocd + 12].copy_from_slice(&2u16.to_le_bytes());
        fs::write(&input, bytes).unwrap();
        let limits = Limits {
            max_entries: 1,
            ..Limits::default()
        };
        assert!(matches!(
            ArchiveIndex::open(&input, limits),
            Err(Error::LimitExceeded {
                kind: "entry count",
                actual: 2,
                limit: 1,
            })
        ));
    }

    #[test]
    fn raw_preflight_rejects_central_directory_over_budget() {
        let temp = TempDir::new().unwrap();
        let input = temp.path().join("central-budget.zip");
        make_archive(
            &input,
            &[FixtureEntry {
                name: "line.ogg",
                bytes: b"voice",
                method: zip::CompressionMethod::Stored,
            }],
        );
        let bytes = fs::read(&input).unwrap();
        let eocd = find_eocd(&bytes);
        let actual = u64::from(le_u32(&bytes[eocd + 12..eocd + 16]));
        let limits = Limits {
            max_central_directory_bytes: actual - 1,
            ..Limits::default()
        };
        assert!(matches!(
            ArchiveIndex::open(&input, limits),
            Err(Error::LimitExceeded {
                kind: "central directory bytes",
                actual: rejected,
                limit,
            }) if rejected == actual && limit == actual - 1
        ));
    }

    #[test]
    fn raw_preflight_rejects_duplicate_central_names_before_deduplication() {
        let temp = TempDir::new().unwrap();
        let input = temp.path().join("duplicate.zip");
        make_archive(
            &input,
            &[
                FixtureEntry {
                    name: "a.ogg",
                    bytes: b"first",
                    method: zip::CompressionMethod::Stored,
                },
                FixtureEntry {
                    name: "b.ogg",
                    bytes: b"second",
                    method: zip::CompressionMethod::Stored,
                },
            ],
        );
        let mut bytes = fs::read(&input).unwrap();
        let headers = bytes
            .windows(4)
            .enumerate()
            .filter_map(|(offset, window)| {
                (window == 0x0201_4b50u32.to_le_bytes()).then_some(offset)
            })
            .collect::<Vec<_>>();
        assert_eq!(headers.len(), 2);
        bytes[headers[1] + 46..headers[1] + 51].copy_from_slice(b"a.ogg");
        fs::write(&input, bytes).unwrap();

        assert!(matches!(
            ArchiveIndex::open(&input, Limits::default()),
            Err(Error::Zip(zip::result::ZipError::InvalidArchive(
                "duplicate raw central-directory filename"
            )))
        ));
    }

    #[test]
    fn raw_preflight_accepts_consistent_zip64_eocd() {
        let temp = TempDir::new().unwrap();
        let input = temp.path().join("zip64.zip");
        make_archive(
            &input,
            &[FixtureEntry {
                name: "line.ogg",
                bytes: b"voice",
                method: zip::CompressionMethod::Stored,
            }],
        );
        promote_fixture_to_zip64(&input);
        let archive = ArchiveIndex::open(&input, Limits::default()).unwrap();
        assert_eq!(archive.entries().len(), 1);
        assert_eq!(archive.entries()[0].path, "line.ogg");
    }

    #[test]
    fn raw_preflight_rejects_nested_zip64_eocd_candidates() {
        let temp = TempDir::new().unwrap();
        let input = temp.path().join("nested-zip64.zip");
        make_archive(
            &input,
            &[FixtureEntry {
                name: "line.ogg",
                bytes: b"voice",
                method: zip::CompressionMethod::Stored,
            }],
        );
        promote_fixture_to_zip64(&input);
        let bytes = fs::read(&input).unwrap();
        let eocd = find_eocd(&bytes);
        let locator = eocd - ZIP64_LOCATOR_BYTES as usize;
        let eocd64 = locator - ZIP64_EOCD_FIXED_BYTES;
        let mut outer = bytes[eocd64..locator].to_vec();
        outer[4..12].copy_from_slice(&100u64.to_le_bytes());
        let nested = bytes[eocd64..locator].to_vec();
        let mut output = bytes[..eocd64].to_vec();
        output.extend_from_slice(&outer);
        output.extend_from_slice(&nested);
        output.extend_from_slice(&bytes[locator..]);
        fs::write(&input, output).unwrap();

        assert!(matches!(
            ArchiveIndex::open(&input, Limits::default()),
            Err(Error::Zip(zip::result::ZipError::InvalidArchive(
                "multiple structurally valid ZIP64 EOCD records precede one locator"
            )))
        ));
    }

    #[test]
    fn raw_preflight_limits_a_zip32_comment_decoy_before_parser_allocation() {
        let temp = TempDir::new().unwrap();
        let input = temp.path().join("zip32-decoy.zip");
        make_archive(
            &input,
            &[FixtureEntry {
                name: "line.ogg",
                bytes: b"voice",
                method: zip::CompressionMethod::Stored,
            }],
        );
        let bytes = fs::read(&input).unwrap();
        let eocd = find_eocd(&bytes);
        let mut true_eocd = bytes[eocd..eocd + ZIP_EOCD_FIXED_BYTES].to_vec();
        let mut decoy = true_eocd.clone();
        decoy[8..10].copy_from_slice(&2u16.to_le_bytes());
        decoy[10..12].copy_from_slice(&2u16.to_le_bytes());
        decoy[20..22].copy_from_slice(&0u16.to_le_bytes());
        let comment_len = u16::try_from(decoy.len() + 1).unwrap();
        true_eocd[20..22].copy_from_slice(&comment_len.to_le_bytes());
        let mut output = bytes[..eocd].to_vec();
        output.extend_from_slice(&true_eocd);
        output.extend_from_slice(&decoy);
        output.push(0);
        fs::write(&input, output).unwrap();
        let limits = Limits {
            max_entries: 1,
            ..Limits::default()
        };

        assert!(matches!(
            ArchiveIndex::open(&input, limits),
            Err(Error::LimitExceeded {
                kind: "entry count",
                actual: 2,
                limit: 1,
            })
        ));
    }

    #[test]
    fn known_archive_offset_accepts_a_prefixed_zip_without_parser_redetection() {
        let temp = TempDir::new().unwrap();
        let input = temp.path().join("prefixed.zip");
        make_archive(
            &input,
            &[FixtureEntry {
                name: "line.ogg",
                bytes: b"voice",
                method: zip::CompressionMethod::Stored,
            }],
        );
        let bytes = fs::read(&input).unwrap();
        let prefix = b"private executable-style prefix\n";
        let mut prefixed = prefix.to_vec();
        prefixed.extend_from_slice(&bytes);
        fs::write(&input, prefixed).unwrap();

        let archive = ArchiveIndex::open(&input, Limits::default()).unwrap();
        assert_eq!(archive.entries()[0].path, "line.ogg");
    }

    #[test]
    fn generated_prefixless_candidate_preflights_a_newly_viable_comment_decoy() {
        let temp = TempDir::new().unwrap();
        let input = temp.path().join("prefixed-decoy.zip");
        let ogg = vorbis_ogg(44_100);
        make_archive(
            &input,
            &[FixtureEntry {
                name: "line.ogg",
                bytes: &ogg,
                method: zip::CompressionMethod::Stored,
            }],
        );
        let bytes = fs::read(&input).unwrap();
        let eocd = find_eocd(&bytes);
        let directory_size = usize::try_from(le_u32(&bytes[eocd + 12..eocd + 16])).unwrap();
        let central = eocd - directory_size;
        let gap_len = 64usize;
        let prefix = b"executable-style-prefix\n";

        let mut true_eocd = bytes[eocd..eocd + ZIP_EOCD_FIXED_BYTES].to_vec();
        true_eocd[16..20].copy_from_slice(&u32::try_from(central + gap_len).unwrap().to_le_bytes());
        let mut decoy = true_eocd.clone();
        decoy[8..10].copy_from_slice(&2u16.to_le_bytes());
        decoy[10..12].copy_from_slice(&2u16.to_le_bytes());
        decoy[16..20].copy_from_slice(&u32::try_from(central).unwrap().to_le_bytes());
        decoy[20..22].copy_from_slice(&0u16.to_le_bytes());
        true_eocd[20..22].copy_from_slice(&u16::try_from(decoy.len() + 1).unwrap().to_le_bytes());

        let mut crafted = prefix.to_vec();
        crafted.extend_from_slice(&bytes[..central]);
        crafted.resize(crafted.len() + gap_len, 0);
        crafted.extend_from_slice(&bytes[central..eocd]);
        crafted.extend_from_slice(&true_eocd);
        crafted.extend_from_slice(&decoy);
        crafted.push(0);
        fs::write(&input, crafted).unwrap();

        let limits = Limits {
            max_entries: 1,
            ..Limits::default()
        };
        let archive = ArchiveIndex::open(&input, limits).unwrap();
        assert!(matches!(
            archive.rewrite_edits([ArchiveEdit::Replace {
                basename: "line.ogg",
                exact_path: None,
                ogg: &ogg,
            }]),
            Err(Error::LimitExceeded {
                kind: "entry count",
                actual: 2,
                limit: 1,
            })
        ));
    }

    #[test]
    fn alternate_zip64_exact_magic_parse_failure_rejects_a_later_oversized_record() {
        let temp = TempDir::new().unwrap();
        let input = temp.path().join("alternate-zip64-scan.zip");
        make_archive(
            &input,
            &[FixtureEntry {
                name: "line.ogg",
                bytes: b"voice payload long enough for central offsets",
                method: zip::CompressionMethod::Stored,
            }],
        );
        let bytes = fs::read(&input).unwrap();
        let eocd = find_eocd(&bytes);
        let directory_size = le_u32(&bytes[eocd + 12..eocd + 16]);
        let directory_offset = le_u32(&bytes[eocd + 16..eocd + 20]);
        assert!(directory_offset > ZIP64_EOCD_FIXED_BYTES as u32);

        let invalid_offset = eocd + ZIP_EOCD_FIXED_BYTES;
        let valid_offset = invalid_offset + ZIP64_EOCD_FIXED_BYTES;
        let locator_offset = valid_offset + ZIP64_EOCD_FIXED_BYTES;
        let mut invalid = Vec::new();
        invalid.extend_from_slice(&0x0606_4b50u32.to_le_bytes());
        invalid.extend_from_slice(&44u64.to_le_bytes());
        invalid.resize(ZIP64_EOCD_FIXED_BYTES, 0);

        let mut valid = Vec::new();
        valid.extend_from_slice(&0x0606_4b50u32.to_le_bytes());
        valid.extend_from_slice(&44u64.to_le_bytes());
        valid.extend_from_slice(&45u16.to_le_bytes());
        valid.extend_from_slice(&45u16.to_le_bytes());
        valid.extend_from_slice(&0u32.to_le_bytes());
        valid.extend_from_slice(&0u32.to_le_bytes());
        valid.extend_from_slice(&2u64.to_le_bytes());
        valid.extend_from_slice(&2u64.to_le_bytes());
        valid.extend_from_slice(&u64::from(directory_size).to_le_bytes());
        valid.extend_from_slice(
            &(u64::from(directory_offset) - ZIP64_EOCD_FIXED_BYTES as u64).to_le_bytes(),
        );
        assert_eq!(valid.len(), ZIP64_EOCD_FIXED_BYTES);

        let mut locator = Vec::new();
        locator.extend_from_slice(&0x0706_4b50u32.to_le_bytes());
        locator.extend_from_slice(&0u32.to_le_bytes());
        locator.extend_from_slice(&u64::try_from(invalid_offset).unwrap().to_le_bytes());
        locator.extend_from_slice(&1u32.to_le_bytes());
        assert_eq!(locator.len(), ZIP64_LOCATOR_BYTES as usize);

        let mut decoy = bytes[eocd..eocd + ZIP_EOCD_FIXED_BYTES].to_vec();
        decoy[8..12].fill(0xff);
        decoy[16..20].fill(0xff);
        decoy[20..22].copy_from_slice(&0u16.to_le_bytes());
        let comment_len = invalid.len() + valid.len() + locator.len() + decoy.len() + 1;
        let mut true_eocd = bytes[eocd..eocd + ZIP_EOCD_FIXED_BYTES].to_vec();
        true_eocd[20..22].copy_from_slice(&u16::try_from(comment_len).unwrap().to_le_bytes());

        let mut crafted = bytes[..eocd].to_vec();
        crafted.extend_from_slice(&true_eocd);
        crafted.extend_from_slice(&invalid);
        crafted.extend_from_slice(&valid);
        crafted.extend_from_slice(&locator);
        crafted.extend_from_slice(&decoy);
        crafted.push(0);
        assert_eq!(
            locator_offset,
            eocd + ZIP_EOCD_FIXED_BYTES + invalid.len() + valid.len()
        );
        fs::write(&input, crafted).unwrap();

        assert!(matches!(
            ArchiveIndex::open(
                &input,
                Limits {
                    max_entries: 1,
                    ..Limits::default()
                }
            ),
            Err(Error::Zip(zip::result::ZipError::InvalidArchive(
                "multiple parser-viable EOCD records are unsupported"
            )))
        ));
    }

    #[test]
    fn portable_archive_entry_paths_cover_windows_aliases_and_spellings() {
        let limits = Limits::default();
        for safe in [
            "NPC/Hero/line.ogg",
            "NPC/Änne/line.ogg",
            "devices/COM0.ogg",
            "devices/COM10.ogg",
            "devices/LPT0.ogg",
            "devices/LPT10.ogg",
            "devices/CLOCKWORK$.ogg",
            "devices/CONSOLE.ogg",
        ] {
            validate_archive_entry_path(safe, &limits)
                .unwrap_or_else(|error| panic!("safe path {safe:?} was rejected: {error}"));
        }

        for unsafe_path in [
            "",
            "/absolute.ogg",
            "\\absolute.ogg",
            "C:/drive.ogg",
            "NPC\\backslash.ogg",
            "NPC//empty.ogg",
            "./line.ogg",
            "NPC/../line.ogg",
            "NPC/control\n.ogg",
            "NPC/nul\0.ogg",
            "NPC/name:stream.ogg",
            "NPC/trailing. ",
            "NPC/trailing.",
            "NPC/<bad>.ogg",
            "NPC/quote\".ogg",
            "NPC/pipe|.ogg",
            "NPC/question?.ogg",
            "NPC/star*.ogg",
            "CON.ogg",
            "CON .ogg",
            "prn.txt",
            "AUX",
            "nul.anything",
            "CLOCK$.ogg",
            "clock$/line.ogg",
            "CONIN$.ogg",
            "conout$/line.ogg",
            "COM1.ogg",
            "COM1 .ogg",
            "com9/line.ogg",
            "LPT1.ogg",
            "lpt9/line.ogg",
            "COM¹.ogg",
            "COM¹ .ogg",
            "com²/line.ogg",
            "COM³.txt",
            "LPT¹.ogg",
            "lpt²/line.ogg",
            "LPT³.txt",
        ] {
            assert!(
                matches!(
                    validate_archive_entry_path(unsafe_path, &limits),
                    Err(Error::UnsafePath { .. })
                ),
                "unsafe path was accepted: {unsafe_path:?}"
            );
        }
    }

    #[test]
    fn add_path_obeys_the_archive_path_byte_limit() {
        let temp = TempDir::new().unwrap();
        let input = temp.path().join("input.zip");
        let output = temp.path().join("output.zip");
        let ogg = vorbis_ogg(44_100);
        make_archive(
            &input,
            &[FixtureEntry {
                name: "x.ogg",
                bytes: &ogg,
                method: zip::CompressionMethod::Stored,
            }],
        );
        let exact = "a/b.ogg";
        let limits = Limits {
            max_path_bytes: exact.len(),
            ..Limits::default()
        };
        validate_archive_entry_path(exact, &limits).unwrap();
        assert!(matches!(
            validate_archive_entry_path("aa/b.ogg", &limits),
            Err(Error::LimitExceeded {
                kind: "entry path bytes",
                actual,
                limit,
            }) if actual == exact.len() as u64 + 1 && limit == exact.len() as u64
        ));

        let archive = ArchiveIndex::open(&input, limits).unwrap();
        assert!(matches!(
            archive.write_edited(
                &output,
                ArchiveEdit::Add {
                    path: "aa/b.ogg",
                    ogg: &ogg,
                },
            ),
            Err(Error::LimitExceeded {
                kind: "entry path bytes",
                ..
            })
        ));
        assert!(!output.exists());
    }

    #[test]
    fn resolves_case_insensitive_basename_with_exact_override() {
        let temp = TempDir::new().unwrap();
        let input = temp.path().join("voices.zip");
        let ogg = vorbis_ogg(44_100);
        make_archive(
            &input,
            &[
                FixtureEntry {
                    name: "NPC/A/Line.ogg",
                    bytes: &ogg,
                    method: zip::CompressionMethod::Stored,
                },
                FixtureEntry {
                    name: "NPC/B/LINE.OGG",
                    bytes: &ogg,
                    method: zip::CompressionMethod::Stored,
                },
            ],
        );
        let archive = ArchiveIndex::open(&input, Limits::default()).unwrap();

        let error = archive.resolve("line.ogg", None).unwrap_err();
        assert!(matches!(
            error,
            Error::Ambiguous { candidates, .. } if candidates.len() == 2
        ));
        assert_eq!(
            archive
                .resolve("ignored.ogg", Some("NPC/B/LINE.OGG"))
                .unwrap()
                .index,
            1
        );
        assert!(matches!(
            archive.resolve("missing.ogg", None),
            Err(Error::NotFound { .. })
        ));
    }

    #[test]
    fn extract_rejects_traversal_before_writing() {
        let temp = TempDir::new().unwrap();
        let input = temp.path().join("traversal.zip");
        make_archive(
            &input,
            &[FixtureEntry {
                name: "../escape.ogg",
                bytes: b"not reached",
                method: zip::CompressionMethod::Stored,
            }],
        );
        let archive = ArchiveIndex::open(&input, Limits::default()).unwrap();
        let output_root = temp.path().join("extract");

        assert!(matches!(
            archive.extract_all(&output_root),
            Err(Error::UnsafePath { .. })
        ));
        assert!(!output_root.exists());
        assert!(!temp.path().join("escape.ogg").exists());
    }

    #[test]
    fn extracts_selected_entry_under_root() {
        let temp = TempDir::new().unwrap();
        let input = temp.path().join("safe.zip");
        make_archive(
            &input,
            &[FixtureEntry {
                name: "NPC/HELLO.ogg",
                bytes: b"voice bytes",
                method: zip::CompressionMethod::Stored,
            }],
        );
        let archive = ArchiveIndex::open(&input, Limits::default()).unwrap();
        let output = archive
            .extract("hello.ogg", None, temp.path().join("extract"))
            .unwrap();
        assert_eq!(fs::read(output).unwrap(), b"voice bytes");
    }

    #[test]
    fn private_snapshot_payload_is_stable_and_a_later_source_hash_mismatch_rejects() {
        let temp = TempDir::new().unwrap();
        let input = temp.path().join("snapshot.zip");
        let original = b"immutable snapshot payload";
        make_archive(
            &input,
            &[FixtureEntry {
                name: "NPC/HELLO.ogg",
                bytes: original,
                method: zip::CompressionMethod::Stored,
            }],
        );
        let index = ArchiveIndex::open(&input, Limits::default()).unwrap();
        let mut snapshot_archive = index.open_current().unwrap();

        let mut changed = fs::read(&input).unwrap();
        let (_, _, payload, _) = first_entry_layout(&changed);
        changed[payload] ^= 0x5a;
        fs::write(&input, changed).unwrap();

        let mut snapshot_entry = snapshot_archive.by_index(0).unwrap();
        let mut payload = Vec::new();
        snapshot_entry.read_to_end(&mut payload).unwrap();
        assert_eq!(payload, original);
        drop(snapshot_entry);
        drop(snapshot_archive);
        assert!(matches!(index.open_current(), Err(Error::ArchiveChanged)));
    }

    #[test]
    fn raw_metadata_lists_encrypted_and_unsupported_entries_before_safe_rejection() {
        let temp = TempDir::new().unwrap();
        let encrypted = temp.path().join("encrypted.zip");
        make_archive(
            &encrypted,
            &[FixtureEntry {
                name: "encrypted.ogg",
                bytes: b"ciphertext-shaped bytes",
                method: zip::CompressionMethod::Stored,
            }],
        );
        let mut bytes = fs::read(&encrypted).unwrap();
        let (central, local, _, _) = first_entry_layout(&bytes);
        let central_flags = le_u16(&bytes[central + 8..central + 10]) | 1;
        let local_flags = le_u16(&bytes[local + 6..local + 8]) | 1;
        bytes[central + 8..central + 10].copy_from_slice(&central_flags.to_le_bytes());
        bytes[local + 6..local + 8].copy_from_slice(&local_flags.to_le_bytes());
        fs::write(&encrypted, bytes).unwrap();
        let encrypted_index = ArchiveIndex::open(&encrypted, Limits::default()).unwrap();
        assert!(encrypted_index.entries()[0].encrypted);
        let encrypted_output = temp.path().join("encrypted-output");
        assert!(matches!(
            encrypted_index.extract("", Some("encrypted.ogg"), &encrypted_output),
            Err(Error::EncryptedEntry(path)) if path == "encrypted.ogg"
        ));
        assert!(!encrypted_output.exists());

        let unsupported = temp.path().join("unsupported.zip");
        make_archive(
            &unsupported,
            &[FixtureEntry {
                name: "unsupported.ogg",
                bytes: b"raw unsupported payload",
                method: zip::CompressionMethod::Stored,
            }],
        );
        let mut bytes = fs::read(&unsupported).unwrap();
        let (central, local, _, _) = first_entry_layout(&bytes);
        bytes[central + 10..central + 12].copy_from_slice(&12u16.to_le_bytes());
        bytes[local + 8..local + 10].copy_from_slice(&12u16.to_le_bytes());
        fs::write(&unsupported, bytes).unwrap();
        let unsupported_index = ArchiveIndex::open(&unsupported, Limits::default()).unwrap();
        assert_eq!(
            unsupported_index.entries()[0].compression,
            zip::CompressionMethod::BZIP2
        );
        let unsupported_output = temp.path().join("unsupported-output");
        assert!(matches!(
            unsupported_index.extract("", Some("unsupported.ogg"), &unsupported_output),
            Err(Error::UnsupportedCompression { path, method })
                if path == "unsupported.ogg" && method == zip::CompressionMethod::BZIP2
        ));
        assert!(!unsupported_output.exists());
    }

    #[test]
    fn corrupted_stored_and_deflated_payloads_are_archive_data_not_source_io() {
        let temp = TempDir::new().unwrap();
        for (label, method) in [
            ("stored", zip::CompressionMethod::Stored),
            ("deflated", zip::CompressionMethod::Deflated),
        ] {
            let input = temp.path().join(format!("corrupt-{label}.zip"));
            make_archive(
                &input,
                &[FixtureEntry {
                    name: "corrupt.ogg",
                    bytes: b"payload that must fail its CRC after corruption",
                    method,
                }],
            );
            let mut bytes = fs::read(&input).unwrap();
            let (_, _, payload, compressed) = first_entry_layout(&bytes);
            assert_ne!(compressed, 0);
            bytes[payload + compressed / 2] ^= 0x80;
            fs::write(&input, bytes).unwrap();
            let index = ArchiveIndex::open(&input, Limits::default()).unwrap();
            let output_root = temp.path().join(format!("corrupt-{label}-output"));
            let error = index
                .extract("", Some("corrupt.ogg"), &output_root)
                .unwrap_err();
            assert!(
                matches!(error, Error::ArchiveData { .. } | Error::Verification(_)),
                "unexpected {label} payload error: {error}"
            );
            assert!(!output_root.join("corrupt.ogg").exists());
        }
    }

    #[test]
    fn extract_rejects_link_or_reparse_entry_parent() {
        let temp = TempDir::new().unwrap();
        let input = temp.path().join("safe.zip");
        make_archive(
            &input,
            &[FixtureEntry {
                name: "NPC/HELLO.ogg",
                bytes: b"voice bytes",
                method: zip::CompressionMethod::Stored,
            }],
        );
        let archive = ArchiveIndex::open(&input, Limits::default()).unwrap();
        let output_root = temp.path().join("extract");
        let outside = temp.path().join("outside");
        fs::create_dir(&output_root).unwrap();
        fs::create_dir(&outside).unwrap();
        let link = output_root.join("NPC");

        #[cfg(unix)]
        std::os::unix::fs::symlink(&outside, &link).unwrap();
        #[cfg(windows)]
        {
            let status = std::process::Command::new("cmd")
                .args(["/c", "mklink", "/J"])
                .arg(&link)
                .arg(&outside)
                .status()
                .unwrap();
            if !status.success() {
                return;
            }
        }

        assert!(matches!(
            archive.extract("", Some("NPC/HELLO.ogg"), &output_root),
            Err(Error::UnsafeOutput { .. })
        ));
        assert!(!outside.join("HELLO.ogg").exists());

        #[cfg(unix)]
        fs::remove_file(&link).unwrap();
        #[cfg(windows)]
        fs::remove_dir(&link).unwrap();
    }

    #[test]
    fn replace_is_copy_on_write_and_preserves_untouched_order_and_metadata() {
        let temp = TempDir::new().unwrap();
        let input = temp.path().join("input.zip");
        let output = temp.path().join("output.zip");
        let original = vorbis_ogg(22_050);
        let replacement = vorbis_ogg(48_000);
        make_archive(
            &input,
            &[
                FixtureEntry {
                    name: "NPC/line.ogg",
                    bytes: &original,
                    method: zip::CompressionMethod::Stored,
                },
                FixtureEntry {
                    name: "manifest.txt",
                    bytes: b"metadata-12345",
                    method: zip::CompressionMethod::Deflated,
                },
            ],
        );
        let before = file_hash(&input);
        let archive = ArchiveIndex::open(&input, Limits::default()).unwrap();
        let report = archive
            .write_edited(
                &output,
                ArchiveEdit::Replace {
                    basename: "LINE.OGG",
                    exact_path: None,
                    ogg: &replacement,
                },
            )
            .unwrap();

        assert_eq!(report.action, EditAction::Replaced);
        assert_eq!(report.entry_index, 0);
        assert_eq!(file_hash(&input), before);
        let mut rewritten = ZipArchive::new(File::open(&output).unwrap()).unwrap();
        assert_eq!(rewritten.comment(), b"gore-vo fixture");
        let mut first = rewritten.by_index(0).unwrap();
        assert_eq!(first.name(), "NPC/line.ogg");
        assert_eq!(first.compression(), zip::CompressionMethod::Stored);
        let mut bytes = Vec::new();
        first.read_to_end(&mut bytes).unwrap();
        assert_eq!(bytes, replacement);
        drop(first);
        let mut untouched = rewritten.by_index(1).unwrap();
        assert_eq!(untouched.name(), "manifest.txt");
        assert_eq!(untouched.compression(), zip::CompressionMethod::Deflated);
        assert_eq!(untouched.unix_mode().unwrap() & 0o777, 0o640);
        let mut bytes = Vec::new();
        untouched.read_to_end(&mut bytes).unwrap();
        assert_eq!(bytes, b"metadata-12345");
    }

    #[test]
    fn add_appends_a_stored_entry_and_refuses_existing_output() {
        let temp = TempDir::new().unwrap();
        let input = temp.path().join("input.zip");
        let output = temp.path().join("output.zip");
        let original = vorbis_ogg(22_050);
        let added = vorbis_ogg(44_100);
        make_archive(
            &input,
            &[FixtureEntry {
                name: "first.ogg",
                bytes: &original,
                method: zip::CompressionMethod::Stored,
            }],
        );
        let archive = ArchiveIndex::open(&input, Limits::default()).unwrap();
        archive
            .write_edited(
                &output,
                ArchiveEdit::Add {
                    path: "NPC/new.ogg",
                    ogg: &added,
                },
            )
            .unwrap();

        let rewritten = ArchiveIndex::open(&output, Limits::default()).unwrap();
        assert_eq!(rewritten.entries()[0].path, "first.ogg");
        assert_eq!(rewritten.entries()[1].path, "NPC/new.ogg");
        assert_eq!(
            rewritten.entries()[1].compression,
            zip::CompressionMethod::Stored
        );
        assert!(matches!(
            archive.write_edited(
                &output,
                ArchiveEdit::Add {
                    path: "another.ogg",
                    ogg: &added,
                }
            ),
            Err(Error::OutputExists(_))
        ));
        assert!(matches!(
            archive.write_edited(
                &input,
                ArchiveEdit::Add {
                    path: "another.ogg",
                    ogg: &added,
                }
            ),
            Err(Error::InputOutputSame(_))
        ));
    }

    #[test]
    fn rewrite_accepts_entries_without_unix_mode_metadata() {
        let temp = TempDir::new().unwrap();
        let input = temp.path().join("dos-metadata.zip");
        let output = temp.path().join("output.zip");
        let original = vorbis_ogg(22_050);
        let replacement = vorbis_ogg(44_100);
        let file = File::create(&input).unwrap();
        let mut writer = ZipWriter::new(file);
        writer
            .start_file(
                "line.ogg",
                SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored),
            )
            .unwrap();
        writer.write_all(&original).unwrap();
        writer.finish().unwrap();
        let mut raw = fs::read(&input).unwrap();
        let central = raw
            .windows(4)
            .position(|window| window == b"PK\x01\x02")
            .unwrap();
        raw[central + 5] = 0;
        raw[central + 38..central + 42].fill(0);
        fs::write(&input, raw).unwrap();

        let archive = ArchiveIndex::open(&input, Limits::default()).unwrap();
        assert_eq!(archive.entries()[0].unix_mode, None);
        archive
            .write_edited(
                &output,
                ArchiveEdit::Replace {
                    basename: "line.ogg",
                    exact_path: None,
                    ogg: &replacement,
                },
            )
            .unwrap();
        let rewritten = ArchiveIndex::open(&output, Limits::default()).unwrap();
        assert_eq!(rewritten.entries()[0].unix_mode.unwrap() & 0o777, 0o644);
    }

    #[test]
    fn mixed_batch_replaces_in_place_and_appends_adds_in_edit_order() {
        let temp = TempDir::new().unwrap();
        let input = temp.path().join("input.zip");
        let output = temp.path().join("output.zip");
        let old_one = vorbis_ogg(16_000);
        let old_two = vorbis_ogg(22_050);
        let new_one = vorbis_ogg(32_000);
        let added_first = vorbis_ogg(44_100);
        let added_second = vorbis_ogg(48_000);
        make_archive(
            &input,
            &[
                FixtureEntry {
                    name: "NPC/one.ogg",
                    bytes: &old_one,
                    method: zip::CompressionMethod::Stored,
                },
                FixtureEntry {
                    name: "manifest.txt",
                    bytes: b"untouched metadata",
                    method: zip::CompressionMethod::Deflated,
                },
                FixtureEntry {
                    name: "NPC/two.ogg",
                    bytes: &old_two,
                    method: zip::CompressionMethod::Deflated,
                },
            ],
        );
        let source = ArchiveIndex::open(&input, Limits::default()).unwrap();
        let untouched = source.entries()[1].clone();
        let reports = source
            .write_edits(
                &output,
                [
                    ArchiveEdit::Add {
                        path: "Added/first.ogg",
                        ogg: &added_first,
                    },
                    ArchiveEdit::Replace {
                        basename: "ONE.OGG",
                        exact_path: None,
                        ogg: &new_one,
                    },
                    ArchiveEdit::Add {
                        path: "Added/second.ogg",
                        ogg: &added_second,
                    },
                ],
            )
            .unwrap();

        assert_eq!(reports.len(), 3);
        assert_eq!(reports[0].action, EditAction::Added);
        assert_eq!(reports[0].entry_index, 3);
        assert_eq!(reports[1].action, EditAction::Replaced);
        assert_eq!(reports[1].entry_index, 0);
        assert_eq!(reports[2].action, EditAction::Added);
        assert_eq!(reports[2].entry_index, 4);
        assert_eq!(
            reports[0].sha256,
            Into::<[u8; 32]>::into(Sha256::digest(&added_first))
        );
        assert_eq!(
            reports[1].sha256,
            Into::<[u8; 32]>::into(Sha256::digest(&new_one))
        );
        assert_eq!(
            reports[2].sha256,
            Into::<[u8; 32]>::into(Sha256::digest(&added_second))
        );

        let rewritten = ArchiveIndex::open(&output, Limits::default()).unwrap();
        assert_eq!(
            rewritten
                .entries()
                .iter()
                .map(|entry| entry.path.as_str())
                .collect::<Vec<_>>(),
            [
                "NPC/one.ogg",
                "manifest.txt",
                "NPC/two.ogg",
                "Added/first.ogg",
                "Added/second.ogg"
            ]
        );
        assert_eq!(
            rewritten.entries()[0].compression,
            zip::CompressionMethod::Stored
        );
        assert_eq!(
            rewritten.entries()[3].compression,
            zip::CompressionMethod::Stored
        );
        assert_eq!(
            rewritten.entries()[4].compression,
            zip::CompressionMethod::Stored
        );
        assert_eq!(rewritten.entries()[1].crc32, untouched.crc32);
        assert_eq!(
            rewritten.entries()[1].compressed_size,
            untouched.compressed_size
        );
        assert_eq!(
            rewritten.entries()[1].last_modified,
            untouched.last_modified
        );
        assert_eq!(rewritten.entries()[1].unix_mode, untouched.unix_mode);

        let mut zip = ZipArchive::new(File::open(&output).unwrap()).unwrap();
        assert_eq!(read_zip_entry(&mut zip, 0), new_one);
        assert_eq!(read_zip_entry(&mut zip, 3), added_first);
        assert_eq!(read_zip_entry(&mut zip, 4), added_second);
    }

    #[test]
    fn in_memory_mixed_batch_matches_published_rewrite_without_creating_an_artifact() {
        let temp = TempDir::new().unwrap();
        let input = temp.path().join("input.zip");
        let output = temp.path().join("output.zip");
        let old = vorbis_ogg(16_000);
        let replacement = vorbis_ogg(32_000);
        let added_first = vorbis_ogg(44_100);
        let added_second = vorbis_ogg(48_000);
        make_archive(
            &input,
            &[
                FixtureEntry {
                    name: "NPC/line.ogg",
                    bytes: &old,
                    method: zip::CompressionMethod::Deflated,
                },
                FixtureEntry {
                    name: "manifest.txt",
                    bytes: b"raw-copied",
                    method: zip::CompressionMethod::Stored,
                },
            ],
        );
        let source = ArchiveIndex::open(&input, Limits::default()).unwrap();

        let (bytes, reports) = source
            .rewrite_edits([
                ArchiveEdit::Add {
                    path: "Added/first.ogg",
                    ogg: &added_first,
                },
                ArchiveEdit::Replace {
                    basename: "LINE.OGG",
                    exact_path: None,
                    ogg: &replacement,
                },
                ArchiveEdit::Add {
                    path: "Added/second.ogg",
                    ogg: &added_second,
                },
            ])
            .unwrap();

        assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 1);
        assert_eq!(reports.len(), 3);
        assert_eq!(reports[0].action, EditAction::Added);
        assert_eq!(reports[0].entry_index, 2);
        assert_eq!(reports[1].action, EditAction::Replaced);
        assert_eq!(reports[1].entry_index, 0);
        assert_eq!(reports[2].action, EditAction::Added);
        assert_eq!(reports[2].entry_index, 3);

        let mut memory_zip = ZipArchive::new(Cursor::new(&bytes)).unwrap();
        assert_eq!(memory_zip.len(), 4);
        assert_eq!(read_zip_entry(&mut memory_zip, 0), replacement);
        assert_eq!(read_zip_entry(&mut memory_zip, 1), b"raw-copied");
        assert_eq!(read_zip_entry(&mut memory_zip, 2), added_first);
        assert_eq!(read_zip_entry(&mut memory_zip, 3), added_second);

        source
            .write_edits(
                &output,
                [
                    ArchiveEdit::Add {
                        path: "Added/first.ogg",
                        ogg: &added_first,
                    },
                    ArchiveEdit::Replace {
                        basename: "LINE.OGG",
                        exact_path: None,
                        ogg: &replacement,
                    },
                    ArchiveEdit::Add {
                        path: "Added/second.ogg",
                        ogg: &added_second,
                    },
                ],
            )
            .unwrap();
        assert_eq!(fs::read(output).unwrap(), bytes);
    }

    #[test]
    fn disk_backed_rewrite_candidate_is_verified_private_and_drop_cleaned() {
        let temp = TempDir::new().unwrap();
        let input = temp.path().join("input.zip");
        let original = vorbis_ogg(16_000);
        let replacement = vorbis_ogg(48_000);
        make_archive(
            &input,
            &[FixtureEntry {
                name: "NPC/line.ogg",
                bytes: &original,
                method: zip::CompressionMethod::Deflated,
            }],
        );
        let before = fs::read(&input).unwrap();
        let source = ArchiveIndex::open(&input, Limits::default()).unwrap();

        let (candidate, reports) = source
            .rewrite_edits_to_temp([ArchiveEdit::Replace {
                basename: "line.ogg",
                exact_path: Some("NPC/line.ogg"),
                ogg: &replacement,
            }])
            .unwrap();
        let candidate_path = candidate.to_path_buf();
        assert!(candidate_path.is_file());
        assert_eq!(reports.len(), 1);
        assert_eq!(fs::read(&input).unwrap(), before);
        let mut zip = ZipArchive::new(File::open(&candidate_path).unwrap()).unwrap();
        assert_eq!(read_zip_entry(&mut zip, 0), replacement);
        drop(zip);
        drop(candidate);
        assert!(!candidate_path.exists());
    }

    #[test]
    fn in_memory_rewrite_preflights_all_edits_and_creates_no_artifact_on_failure() {
        let temp = TempDir::new().unwrap();
        let input = temp.path().join("input.zip");
        let original = vorbis_ogg(22_050);
        let valid = vorbis_ogg(44_100);
        make_archive(
            &input,
            &[FixtureEntry {
                name: "line.ogg",
                bytes: &original,
                method: zip::CompressionMethod::Stored,
            }],
        );
        let source = ArchiveIndex::open(&input, Limits::default()).unwrap();

        assert!(matches!(
            source.rewrite_edits([
                ArchiveEdit::Add {
                    path: "valid.ogg",
                    ogg: &valid,
                },
                ArchiveEdit::Add {
                    path: "invalid.ogg",
                    ogg: b"not an Ogg stream",
                },
            ]),
            Err(Error::InvalidOgg(_))
        ));
        assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 1);
    }

    #[test]
    fn in_memory_rewrite_detects_same_length_source_payload_drift() {
        let temp = TempDir::new().unwrap();
        let input = temp.path().join("input.zip");
        let original = vorbis_ogg(22_050);
        let added = vorbis_ogg(44_100);
        make_archive(
            &input,
            &[FixtureEntry {
                name: "line.ogg",
                bytes: &original,
                method: zip::CompressionMethod::Stored,
            }],
        );
        let source = ArchiveIndex::open(&input, Limits::default()).unwrap();
        let mut changed = fs::read(&input).unwrap();
        let payload = changed
            .windows(4)
            .position(|window| window == b"OggS")
            .unwrap();
        changed[payload + 30] ^= 0x55;
        fs::write(&input, &changed).unwrap();
        assert_eq!(fs::metadata(&input).unwrap().len(), changed.len() as u64);

        assert!(matches!(
            source.rewrite_edits([ArchiveEdit::Add {
                path: "added.ogg",
                ogg: &added,
            }]),
            Err(Error::ArchiveChanged)
        ));
        assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 1);
    }

    #[test]
    fn batch_supports_two_replacements_and_preserves_each_method_and_metadata() {
        let temp = TempDir::new().unwrap();
        let input = temp.path().join("input.zip");
        let output = temp.path().join("output.zip");
        let old_a = vorbis_ogg(16_000);
        let old_b = vorbis_ogg(22_050);
        let new_a = vorbis_ogg(44_100);
        let new_b = vorbis_ogg(48_000);
        make_archive(
            &input,
            &[
                FixtureEntry {
                    name: "A/a.ogg",
                    bytes: &old_a,
                    method: zip::CompressionMethod::Stored,
                },
                FixtureEntry {
                    name: "B/b.ogg",
                    bytes: &old_b,
                    method: zip::CompressionMethod::Deflated,
                },
            ],
        );
        let source = ArchiveIndex::open(&input, Limits::default()).unwrap();
        let before = source.entries().to_vec();
        source
            .write_edits(
                &output,
                [
                    ArchiveEdit::Replace {
                        basename: "a.ogg",
                        exact_path: None,
                        ogg: &new_a,
                    },
                    ArchiveEdit::Replace {
                        basename: "b.ogg",
                        exact_path: None,
                        ogg: &new_b,
                    },
                ],
            )
            .unwrap();

        let after = ArchiveIndex::open(&output, Limits::default()).unwrap();
        assert_eq!(after.entries().len(), 2);
        for (index, previous) in before.iter().enumerate() {
            assert_eq!(after.entries()[index].path, previous.path);
            assert_eq!(after.entries()[index].compression, previous.compression);
            assert_eq!(after.entries()[index].last_modified, previous.last_modified);
            assert_eq!(after.entries()[index].unix_mode, previous.unix_mode);
        }
        let mut zip = ZipArchive::new(File::open(&output).unwrap()).unwrap();
        assert_eq!(read_zip_entry(&mut zip, 0), new_a);
        assert_eq!(read_zip_entry(&mut zip, 1), new_b);
    }

    #[test]
    fn batch_rejects_duplicate_casefold_and_add_replace_targets() {
        let temp = TempDir::new().unwrap();
        let input = temp.path().join("input.zip");
        let original = vorbis_ogg(22_050);
        let replacement = vorbis_ogg(44_100);
        make_archive(
            &input,
            &[FixtureEntry {
                name: "NPC/Line.ogg",
                bytes: &original,
                method: zip::CompressionMethod::Stored,
            }],
        );
        let source = ArchiveIndex::open(&input, Limits::default()).unwrap();

        let duplicate_replace = temp.path().join("duplicate-replace.zip");
        assert!(matches!(
            source.write_edits(
                &duplicate_replace,
                [
                    ArchiveEdit::Replace {
                        basename: "line.ogg",
                        exact_path: None,
                        ogg: &replacement,
                    },
                    ArchiveEdit::Replace {
                        basename: "line.ogg",
                        exact_path: None,
                        ogg: &replacement,
                    },
                ]
            ),
            Err(Error::ConflictingEdits { .. })
        ));
        assert!(!duplicate_replace.exists());

        let duplicate_add = temp.path().join("duplicate-add.zip");
        assert!(matches!(
            source.write_edits(
                &duplicate_add,
                [
                    ArchiveEdit::Add {
                        path: "New/Voice.ogg",
                        ogg: &replacement,
                    },
                    ArchiveEdit::Add {
                        path: "new/VOICE.OGG",
                        ogg: &replacement,
                    },
                ]
            ),
            Err(Error::ConflictingEdits { .. })
        ));
        assert!(!duplicate_add.exists());

        let add_replace = temp.path().join("add-replace.zip");
        assert!(matches!(
            source.write_edits(
                &add_replace,
                [
                    ArchiveEdit::Add {
                        path: "npc/LINE.OGG",
                        ogg: &replacement,
                    },
                    ArchiveEdit::Replace {
                        basename: "line.ogg",
                        exact_path: None,
                        ogg: &replacement,
                    },
                ]
            ),
            Err(Error::ConflictingEdits { .. })
        ));
        assert!(!add_replace.exists());
    }

    #[test]
    fn ambiguous_batch_selector_creates_no_output() {
        let temp = TempDir::new().unwrap();
        let input = temp.path().join("input.zip");
        let output = temp.path().join("output.zip");
        let ogg = vorbis_ogg(22_050);
        make_archive(
            &input,
            &[
                FixtureEntry {
                    name: "A/line.ogg",
                    bytes: &ogg,
                    method: zip::CompressionMethod::Stored,
                },
                FixtureEntry {
                    name: "B/LINE.OGG",
                    bytes: &ogg,
                    method: zip::CompressionMethod::Stored,
                },
            ],
        );
        let source = ArchiveIndex::open(&input, Limits::default()).unwrap();
        assert!(matches!(
            source.write_edits(
                &output,
                [ArchiveEdit::Replace {
                    basename: "line.ogg",
                    exact_path: None,
                    ogg: &ogg,
                }]
            ),
            Err(Error::Ambiguous { .. })
        ));
        assert!(!output.exists());
    }

    #[test]
    fn invalid_later_ogg_aborts_batch_before_any_output() {
        let temp = TempDir::new().unwrap();
        let input = temp.path().join("input.zip");
        let output = temp.path().join("output.zip");
        let original = vorbis_ogg(22_050);
        let valid = vorbis_ogg(44_100);
        make_archive(
            &input,
            &[FixtureEntry {
                name: "line.ogg",
                bytes: &original,
                method: zip::CompressionMethod::Stored,
            }],
        );
        let source = ArchiveIndex::open(&input, Limits::default()).unwrap();
        assert!(matches!(
            source.write_edits(
                &output,
                [
                    ArchiveEdit::Add {
                        path: "valid.ogg",
                        ogg: &valid,
                    },
                    ArchiveEdit::Add {
                        path: "invalid.ogg",
                        ogg: b"not an Ogg stream",
                    },
                ]
            ),
            Err(Error::InvalidOgg(_))
        ));
        assert!(!output.exists());
        assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 1);
    }

    #[test]
    fn rewrite_rejects_unsafe_untouched_member_before_creating_output() {
        let temp = TempDir::new().unwrap();
        let input = temp.path().join("input.zip");
        let output = temp.path().join("output.zip");
        let original = vorbis_ogg(22_050);
        let replacement = vorbis_ogg(44_100);
        make_archive(
            &input,
            &[
                FixtureEntry {
                    name: "NPC/line.ogg",
                    bytes: &original,
                    method: zip::CompressionMethod::Stored,
                },
                FixtureEntry {
                    name: "../untouched.txt",
                    bytes: b"unsafe",
                    method: zip::CompressionMethod::Stored,
                },
            ],
        );
        let source = ArchiveIndex::open(&input, Limits::default()).unwrap();

        assert!(matches!(
            source.write_edited(
                &output,
                ArchiveEdit::Replace {
                    basename: "line.ogg",
                    exact_path: Some("NPC/line.ogg"),
                    ogg: &replacement,
                }
            ),
            Err(Error::UnsafePath { .. })
        ));
        assert!(!output.exists());
        assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 1);
    }

    #[test]
    fn rewrite_rejects_untouched_case_collision_and_file_parent_aliases() {
        let replacement = vorbis_ogg(44_100);
        for (label, unsafe_entries) in [
            (
                "case",
                [
                    ("Path/File.txt", b"a".as_slice()),
                    ("path/file.TXT", b"b".as_slice()),
                ],
            ),
            (
                "parent",
                [
                    ("parent", b"a".as_slice()),
                    ("parent/child.txt", b"b".as_slice()),
                ],
            ),
        ] {
            let temp = TempDir::new().unwrap();
            let input = temp.path().join(format!("{label}.zip"));
            let output = temp.path().join(format!("{label}-output.zip"));
            let original = vorbis_ogg(22_050);
            make_archive(
                &input,
                &[
                    FixtureEntry {
                        name: "line.ogg",
                        bytes: &original,
                        method: zip::CompressionMethod::Stored,
                    },
                    FixtureEntry {
                        name: unsafe_entries[0].0,
                        bytes: unsafe_entries[0].1,
                        method: zip::CompressionMethod::Stored,
                    },
                    FixtureEntry {
                        name: unsafe_entries[1].0,
                        bytes: unsafe_entries[1].1,
                        method: zip::CompressionMethod::Stored,
                    },
                ],
            );
            let source = ArchiveIndex::open(&input, Limits::default()).unwrap();
            assert!(matches!(
                source.write_edited(
                    &output,
                    ArchiveEdit::Replace {
                        basename: "line.ogg",
                        exact_path: None,
                        ogg: &replacement,
                    }
                ),
                Err(Error::UnsafePath { .. })
            ));
            assert!(!output.exists());
            assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 1);
        }
    }

    #[test]
    fn in_memory_rewrite_rejects_source_above_ram_ceiling_before_composition() {
        let temp = TempDir::new().unwrap();
        let input = temp.path().join("memory-ceiling.zip");
        let ogg = vorbis_ogg(44_100);
        make_archive(
            &input,
            &[FixtureEntry {
                name: "line.ogg",
                bytes: &ogg,
                method: zip::CompressionMethod::Stored,
            }],
        );
        let source_bytes = fs::metadata(&input).unwrap().len();
        let limits = Limits {
            max_in_memory_archive_bytes: source_bytes - 1,
            ..Limits::default()
        };
        let archive = ArchiveIndex::open(&input, limits).unwrap();
        assert!(matches!(
            archive.rewrite_edits([ArchiveEdit::Add {
                path: "added.ogg",
                ogg: &ogg,
            }]),
            Err(Error::LimitExceeded {
                kind: "in-memory archive bytes",
                actual,
                limit,
            }) if actual == source_bytes && limit == source_bytes - 1
        ));
    }

    #[test]
    fn rewrite_writers_stop_during_composition_at_their_output_ceiling() {
        let temp = TempDir::new().unwrap();
        let input = temp.path().join("rewrite-ceiling.zip");
        let ogg = vorbis_ogg(48_000);
        make_archive(
            &input,
            &[FixtureEntry {
                name: "line.ogg",
                bytes: &ogg,
                method: zip::CompressionMethod::Stored,
            }],
        );
        let source_bytes = fs::metadata(&input).unwrap().len();
        let output_limit = source_bytes + 8;

        let disk_archive = ArchiveIndex::open(
            &input,
            Limits {
                max_archive_bytes: output_limit,
                ..Limits::default()
            },
        )
        .unwrap();
        assert!(matches!(
            disk_archive.rewrite_edits_to_temp([ArchiveEdit::Add {
                path: "disk-added.ogg",
                ogg: &ogg,
            }]),
            Err(Error::LimitExceeded {
                kind: "archive bytes",
                actual,
                limit,
            }) if actual > limit && limit == output_limit
        ));

        let memory_archive = ArchiveIndex::open(
            &input,
            Limits {
                max_in_memory_archive_bytes: output_limit,
                ..Limits::default()
            },
        )
        .unwrap();
        assert!(matches!(
            memory_archive.rewrite_edits([ArchiveEdit::Add {
                path: "memory-added.ogg",
                ogg: &ogg,
            }]),
            Err(Error::LimitExceeded {
                kind: "in-memory archive bytes",
                actual,
                limit,
            }) if actual > limit && limit == output_limit
        ));
    }

    #[test]
    fn indexing_enforces_uncompressed_size_limit() {
        let temp = TempDir::new().unwrap();
        let input = temp.path().join("limited.zip");
        make_archive(
            &input,
            &[FixtureEntry {
                name: "large.ogg",
                bytes: b"0123456789",
                method: zip::CompressionMethod::Stored,
            }],
        );
        let limits = Limits {
            max_entry_uncompressed_bytes: 9,
            ..Limits::default()
        };
        assert!(matches!(
            ArchiveIndex::open(&input, limits),
            Err(Error::LimitExceeded {
                kind: "entry uncompressed bytes",
                ..
            })
        ));
    }

    #[test]
    fn indexing_enforces_archive_size_before_hashing() {
        let temp = TempDir::new().unwrap();
        let input = temp.path().join("limited.zip");
        make_archive(
            &input,
            &[FixtureEntry {
                name: "line.ogg",
                bytes: b"voice",
                method: zip::CompressionMethod::Stored,
            }],
        );
        let actual = fs::metadata(&input).unwrap().len();
        let limits = Limits {
            max_archive_bytes: actual - 1,
            ..Limits::default()
        };
        assert!(matches!(
            ArchiveIndex::open(&input, limits),
            Err(Error::LimitExceeded {
                kind: "archive bytes",
                actual: rejected,
                limit,
            }) if rejected == actual && limit == actual - 1
        ));
    }

    #[test]
    fn indexing_rejects_excessive_compression_ratio() {
        let temp = TempDir::new().unwrap();
        let input = temp.path().join("bomb.zip");
        let highly_compressible = vec![0u8; 16 * 1024];
        make_archive(
            &input,
            &[FixtureEntry {
                name: "bomb.ogg",
                bytes: &highly_compressible,
                method: zip::CompressionMethod::Deflated,
            }],
        );
        let limits = Limits {
            max_compression_ratio: 2,
            ..Limits::default()
        };
        assert!(matches!(
            ArchiveIndex::open(&input, limits),
            Err(Error::LimitExceeded {
                kind: "compression ratio",
                ..
            })
        ));
    }

    fn read_zip_entry<R: Read + Seek>(archive: &mut ZipArchive<R>, index: usize) -> Vec<u8> {
        let mut file = archive.by_index(index).unwrap();
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes).unwrap();
        bytes
    }
}
