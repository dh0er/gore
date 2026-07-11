use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Cursor, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
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

impl ArchiveIndex {
    pub fn open(path: impl AsRef<Path>, limits: Limits) -> Result<Self> {
        let path = path.as_ref();
        let archive_bytes = fs::metadata(path)?.len();
        check_limit("archive bytes", archive_bytes, limits.max_archive_bytes)?;

        let mut file = File::open(path)?;
        let archive_sha256 = hash_reader(&mut file)?;
        if file.stream_position()? != archive_bytes || file.metadata()?.len() != archive_bytes {
            return Err(Error::ArchiveChanged);
        }
        file.seek(SeekFrom::Start(0))?;
        let mut archive = ZipArchive::new(file)?;
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
            let file = archive.by_index(index)?;
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
            let (_, reports, expected) = self.compose_rewrite(plans, temp.as_file_mut())?;
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
    /// order, and reports remain in input-edit order. The complete candidate ZIP is held in RAM;
    /// `max_archive_bytes` is checked when composition finishes because the compressed output size
    /// is not known in advance.
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
            let (_, reports, expected) = self.compose_rewrite(plans, temp.as_file_mut())?;
            (reports, expected)
        };
        temp.as_file().sync_all()?;
        self.verify_output(temp.path(), &expected)?;
        Ok((temp.into_temp_path(), reports))
    }

    fn prepare_rewrite<'a>(&self, edits: Vec<ArchiveEdit<'a>>) -> Result<PreparedRewrite> {
        let plans = self.plan_edits(edits)?;
        self.check_rewritable_entries()?;

        let (cursor, reports, expected) = self.compose_rewrite(plans, Cursor::new(Vec::new()))?;
        let bytes = cursor.into_inner();
        self.verify_output_bytes(&bytes, &expected)?;

        Ok(PreparedRewrite { bytes, reports })
    }

    fn compose_rewrite<'a, W>(
        &self,
        plans: Vec<EditPlan<'a>>,
        output: W,
    ) -> Result<(W, Vec<RewriteReport>, Vec<ExpectedEntry>)>
    where
        W: Write + Seek,
    {
        let mut input = self.open_current()?;
        let mut writer = ZipWriter::new(output);
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
            let file = input.by_index(index)?;
            if let Some(plan_index) = replacements.get(&index) {
                let plan = &plans[*plan_index];
                let options = file.options();
                let modified = effective_modified(file.last_modified());
                let permissions = effective_permissions(file.unix_mode());
                writer.start_file(file.name(), options)?;
                writer.write_all(plan.ogg)?;
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
                writer.raw_copy_file(file)?;
            }
        }

        for plan in plans.iter().filter(|plan| plan.replacement_index.is_none()) {
            let options = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored)
                .unix_permissions(0o644);
            writer.start_file(&plan.archive_path, options)?;
            writer.write_all(plan.ogg)?;
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

        let output = writer.finish()?;
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
        fs::create_dir_all(output_root)?;
        let root = fs::canonicalize(output_root)?;
        if !root.is_dir() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "extraction root is not a directory",
            )
            .into());
        }

        let mut outputs = Vec::with_capacity(plans.len());
        for plan in plans {
            if plan.is_directory {
                let directory = ensure_directory_chain(&root, &plan.components, &plan.source_path)?;
                outputs.push(directory);
                continue;
            }

            let (filename, parents) = plan
                .components
                .split_last()
                .expect("safe file path is non-empty");
            let parent = ensure_directory_chain(&root, parents, &plan.source_path)?;
            let canonical_parent = fs::canonicalize(&parent)?;
            if !canonical_parent.starts_with(&root) {
                return Err(Error::UnsafePath {
                    path: plan.source_path,
                    reason: "parent resolves outside extraction root",
                });
            }
            let output = parent.join(filename);
            let mut source = archive.by_index(plan.index)?;
            let mut destination = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&output)?;
            let copy_result = (|| -> Result<()> {
                let copied = io::copy(&mut source, &mut destination)?;
                if copied != self.entries[plan.index].uncompressed_size {
                    return Err(Error::Verification(format!(
                        "entry {:?} extracted {copied} bytes, expected {}",
                        self.entries[plan.index].path, self.entries[plan.index].uncompressed_size
                    )));
                }
                destination.sync_all()?;
                Ok(())
            })();
            if let Err(error) = copy_result {
                drop(destination);
                let _ = fs::remove_file(&output);
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
        let file = self.open_hash_checked_file()?;
        let mut archive = ZipArchive::new(file)?;
        if archive.len() != self.entries.len()
            || archive.comment() != self.archive_comment
            || archive.zip64_comment() != self.zip64_comment.as_deref()
        {
            return Err(Error::ArchiveChanged);
        }
        for expected in &self.entries {
            let current = archive.by_index(expected.index)?;
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
        self.open_hash_checked_file().map(drop)
    }

    fn open_hash_checked_file(&self) -> Result<File> {
        let mut file = File::open(&self.path)?;
        if file.metadata()?.len() != self.archive_bytes {
            return Err(Error::ArchiveChanged);
        }
        let current_sha256 = hash_reader(&mut file)?;
        if file.stream_position()? != self.archive_bytes
            || file.metadata()?.len() != self.archive_bytes
            || current_sha256 != self.archive_sha256
        {
            return Err(Error::ArchiveChanged);
        }
        file.seek(SeekFrom::Start(0))?;
        Ok(file)
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
        reader: R,
        archive_bytes: u64,
        expected: &[ExpectedEntry],
    ) -> Result<()> {
        check_limit(
            "archive bytes",
            archive_bytes,
            self.limits.max_archive_bytes,
        )?;
        let mut archive = ZipArchive::new(reader)?;
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
            let mut actual = archive.by_index(index)?;
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
            if let Some(expected_hash) = expected.sha256 {
                if hash_reader(&mut actual)? != expected_hash {
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

fn ensure_directory_chain(
    root: &Path,
    components: &[String],
    source_path: &str,
) -> Result<PathBuf> {
    let mut current = root.to_path_buf();
    for component in components {
        current.push(component);
        match fs::symlink_metadata(&current) {
            Ok(metadata) => {
                if metadata.file_type().is_symlink() || !metadata.is_dir() {
                    return Err(Error::UnsafePath {
                        path: source_path.to_owned(),
                        reason: "extraction parent is a symlink or non-directory",
                    });
                }
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                fs::create_dir(&current)?;
            }
            Err(error) => return Err(error.into()),
        }
        if !fs::canonicalize(&current)?.starts_with(root) {
            return Err(Error::UnsafePath {
                path: source_path.to_owned(),
                reason: "directory resolves outside extraction root",
            });
        }
    }
    Ok(current)
}

fn hash_reader(reader: &mut impl Read) -> Result<[u8; 32]> {
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
