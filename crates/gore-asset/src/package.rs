//! Lossless carrier for split cooked `.uasset` / `.uexp` package files.
//!
//! This module deliberately does not infer Unreal export boundaries. The
//! fixtures and package-version context needed to prove those boundaries are
//! not part of this crate yet. Callers therefore select an explicit component,
//! byte offset, and byte length. Replacements must have exactly the selected
//! length: shifting unknown package data without updating version-specific
//! package tables would be unsafe.
//!
//! Output is always written to a new pair. Both components are staged and
//! verified before each file is atomically published with no-clobber semantics.
//! The `.uexp` payload is published first and `.uasset` last as the visible
//! commit marker. Filesystems do not offer one atomic rename for two sibling
//! files, so ordinary process termination between those publishes may leave an
//! orphan `.uexp`; `.uasset` is attempted only after `.uexp` succeeds. Parent
//! directory sync is best-effort, so power-loss durability remains filesystem-
//! and platform-dependent. Existing destinations are never overwritten.
//!
//! The final cleanup operation is necessarily a path-based unlink. The carrier
//! retains the published file handle, checks SHA-256 during cleanup, then
//! rechecks file identity immediately before unlinking. The standard library
//! cannot make that identity check and unlink one atomic operation. Output
//! directories therefore remain a trusted boundary and must have no concurrent
//! writer or renamer, adversarial or otherwise.

use std::fmt;
use std::fs::{self, File, Metadata, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tempfile::NamedTempFile;
use thiserror::Error;

const HASH_BUFFER_BYTES: usize = 64 * 1024;

/// Conservative allocation limits used while loading a package pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PackageLimits {
    pub max_uasset_bytes: u64,
    pub max_uexp_bytes: u64,
    pub max_total_bytes: u64,
}

impl Default for PackageLimits {
    fn default() -> Self {
        Self {
            max_uasset_bytes: 512 * 1024 * 1024,
            max_uexp_bytes: 512 * 1024 * 1024,
            max_total_bytes: 1024 * 1024 * 1024,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PackageComponent {
    #[serde(rename = "uasset")]
    Uasset,
    #[serde(rename = "uexp")]
    Uexp,
}

impl PackageComponent {
    fn extension(self) -> &'static str {
        match self {
            Self::Uasset => "uasset",
            Self::Uexp => "uexp",
        }
    }
}

impl fmt::Display for PackageComponent {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.extension())
    }
}

/// Paths for one split cooked package. Constructing this from a `.uasset`
/// path deterministically derives the sibling `.uexp` path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackagePaths {
    uasset: PathBuf,
    uexp: PathBuf,
}

impl PackagePaths {
    pub fn from_uasset(path: impl AsRef<Path>) -> Result<Self, PackageError> {
        let uasset = path.as_ref();
        validate_package_path(uasset, PackageComponent::Uasset)?;
        Ok(Self {
            uasset: uasset.to_path_buf(),
            uexp: uasset.with_extension(PackageComponent::Uexp.extension()),
        })
    }

    pub fn uasset(&self) -> &Path {
        &self.uasset
    }

    pub fn uexp(&self) -> &Path {
        &self.uexp
    }

    pub fn component(&self, component: PackageComponent) -> &Path {
        match component {
            PackageComponent::Uasset => self.uasset(),
            PackageComponent::Uexp => self.uexp(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComponentDigest {
    pub path: PathBuf,
    pub length: u64,
    pub sha256: [u8; 32],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageWriteReceipt {
    pub uasset: ComponentDigest,
    pub uexp: ComponentDigest,
}

/// A cleanup step that was deliberately refused or could not be completed.
///
/// Cleanup is conditional on the destination still identifying the exact file
/// published by this process. A changed path is retained rather than risking
/// deletion of another process's replacement.
#[derive(Debug, Error)]
pub enum CleanupFailure {
    #[error("refusing to remove published {component} path {path}: {reason}")]
    OwnershipChanged {
        component: PackageComponent,
        path: PathBuf,
        reason: &'static str,
    },
    #[error("failed to inspect published {component} path {path} during cleanup: {source}")]
    Inspect {
        component: PackageComponent,
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("failed to verify published {component} path {path} during cleanup: {source}")]
    Verify {
        component: PackageComponent,
        path: PathBuf,
        #[source]
        source: Box<PackageError>,
    },
    #[error("failed to remove published {component} path {path}: {source}")]
    Remove {
        component: PackageComponent,
        path: PathBuf,
        #[source]
        source: io::Error,
    },
}

#[derive(Debug, Error)]
pub enum PackageError {
    #[error("expected a .{expected} path, got {path}")]
    InvalidExtension {
        expected: &'static str,
        path: PathBuf,
    },
    #[error("package path has no non-empty file stem: {0}")]
    MissingFileStem(PathBuf),
    #[error("symbolic-link or reparse-point package paths are refused: {0}")]
    SymlinkPath(PathBuf),
    #[error("package component is not a regular file: {0}")]
    NotRegularFile(PathBuf),
    #[error("output parent is not a directory: {0}")]
    OutputParentNotDirectory(PathBuf),
    #[error("output would replace an input component: {0}")]
    InPlaceOutput(PathBuf),
    #[error("output already exists; existing files are never overwritten: {0}")]
    DestinationExists(PathBuf),
    #[error("{component} component is {actual} bytes; limit is {limit} bytes")]
    ComponentTooLarge {
        component: PackageComponent,
        actual: u64,
        limit: u64,
    },
    #[error("package pair is {actual} bytes; limit is {limit} bytes")]
    PairTooLarge { actual: u64, limit: u64 },
    #[error("package size arithmetic overflowed")]
    SizeOverflow,
    #[error("{component} component length {length} does not fit in memory on this platform")]
    LengthDoesNotFitMemory {
        component: PackageComponent,
        length: u64,
    },
    #[error("could not reserve {length} bytes for {component} component: {message}")]
    AllocationFailed {
        component: PackageComponent,
        length: usize,
        message: String,
    },
    #[error("byte range offset {offset} + length {length} overflowed")]
    RangeOverflow { offset: usize, length: usize },
    #[error(
        "{component} byte range {offset}..{end} is outside the {component_length}-byte component"
    )]
    RangeOutOfBounds {
        component: PackageComponent,
        offset: usize,
        end: usize,
        component_length: usize,
    },
    #[error(
        "replacement has {replacement_length} bytes, but the selected range has {range_length}; resizing unknown package data is refused"
    )]
    ReplacementLengthMismatch {
        range_length: usize,
        replacement_length: usize,
    },
    #[error(
        "{component} byte range at offset {offset} with length {length} drifted at byte {mismatch_offset}: expected 0x{expected:02x}, got 0x{actual:02x}"
    )]
    RangeDrift {
        component: PackageComponent,
        offset: usize,
        length: usize,
        mismatch_offset: usize,
        expected: u8,
        actual: u8,
    },
    #[error("{path} changed length while it was being read: expected {expected}, got {actual}")]
    ConcurrentLengthChange {
        path: PathBuf,
        expected: u64,
        actual: u64,
    },
    #[error("reopened file length differs for {path}: expected {expected}, got {actual}")]
    VerificationLength {
        path: PathBuf,
        expected: u64,
        actual: u64,
    },
    #[error("reopened file hash differs for {0}")]
    VerificationHash(PathBuf),
    #[error("reopened file identity differs for {0}")]
    VerificationIdentity(PathBuf),
    #[error("package path changed identity while it was being opened: {0}")]
    ConcurrentIdentityChange(PathBuf),
    #[error("loaded package pair changed while revalidating {component} component {path}")]
    PairGenerationChanged {
        component: PackageComponent,
        path: PathBuf,
        #[source]
        source: Box<PackageError>,
    },
    #[error("package pair publication failed and cleanup was incomplete: {cleanup_failures:?}")]
    PublishCleanupFailed {
        #[source]
        cause: Box<PackageError>,
        cleanup_failures: Vec<CleanupFailure>,
    },
    #[error("failed to {operation} {path}: {source}")]
    Io {
        operation: &'static str,
        path: PathBuf,
        #[source]
        source: io::Error,
    },
}

/// In-memory, byte-preserving representation of a cooked package pair.
#[derive(Debug, Clone)]
pub struct PackageCarrier {
    source: Option<PackagePaths>,
    uasset: Vec<u8>,
    uexp: Vec<u8>,
    limits: PackageLimits,
}

impl PackageCarrier {
    /// Load and re-open both components. After both reads, the complete pair is
    /// revalidated against the exact loaded lengths and SHA-256 digests, with
    /// `.uexp` checked first and the `.uasset` commit marker last. These are
    /// sequential point checks, not an atomic pair snapshot: callers must ensure
    /// there is no concurrent source writer. Observed component mutation fails
    /// closed, but opaque vanilla pairs carry no shared semantic generation id.
    pub fn load(
        uasset_path: impl AsRef<Path>,
        limits: PackageLimits,
    ) -> Result<Self, PackageError> {
        Self::load_with_hooks(uasset_path.as_ref(), limits, |_| {}, |_| {})
    }

    #[cfg(test)]
    fn load_with_after_reads<F>(
        uasset_path: &Path,
        limits: PackageLimits,
        after_reads: F,
    ) -> Result<Self, PackageError>
    where
        F: FnOnce(&PackagePaths),
    {
        Self::load_with_hooks(uasset_path, limits, |_| {}, after_reads)
    }

    fn load_with_hooks<F, G>(
        uasset_path: &Path,
        limits: PackageLimits,
        after_uasset_read: F,
        after_reads: G,
    ) -> Result<Self, PackageError>
    where
        F: FnOnce(&PackagePaths),
        G: FnOnce(&PackagePaths),
    {
        let requested = PackagePaths::from_uasset(uasset_path)?;
        let uasset_path = canonical_regular_path(requested.uasset())?;
        let uexp_path = canonical_regular_path(requested.uexp())?;
        let source = PackagePaths {
            uasset: uasset_path,
            uexp: uexp_path,
        };

        // Check both advertised lengths before allocating either component, so
        // a small pair limit cannot be bypassed by a large first file.
        let advertised_uasset = inspect_component_length(
            source.uasset(),
            PackageComponent::Uasset,
            limits.max_uasset_bytes,
        )?;
        let advertised_uexp =
            inspect_component_length(source.uexp(), PackageComponent::Uexp, limits.max_uexp_bytes)?;
        validate_pair_sizes_u64(advertised_uasset, advertised_uexp, limits)?;

        let uasset = read_verified_component(
            source.uasset(),
            PackageComponent::Uasset,
            limits.max_uasset_bytes,
            0,
            limits.max_total_bytes,
        )?;
        after_uasset_read(&source);
        let loaded_uasset = u64::try_from(uasset.len()).map_err(|_| PackageError::SizeOverflow)?;
        let uexp = read_verified_component(
            source.uexp(),
            PackageComponent::Uexp,
            limits.max_uexp_bytes,
            loaded_uasset,
            limits.max_total_bytes,
        )?;
        validate_pair_sizes(uasset.len(), uexp.len(), limits)?;
        after_reads(&source);
        verify_loaded_pair(&source, &uasset, &uexp, limits)?;

        Ok(Self {
            source: Some(source),
            uasset,
            uexp,
            limits,
        })
    }

    /// Construct a carrier from already-owned bytes while enforcing the same
    /// size limits as file loading.
    pub fn from_bytes(
        uasset: Vec<u8>,
        uexp: Vec<u8>,
        limits: PackageLimits,
    ) -> Result<Self, PackageError> {
        validate_pair_sizes(uasset.len(), uexp.len(), limits)?;
        Ok(Self {
            source: None,
            uasset,
            uexp,
            limits,
        })
    }

    pub fn source_paths(&self) -> Option<&PackagePaths> {
        self.source.as_ref()
    }

    pub fn limits(&self) -> PackageLimits {
        self.limits
    }

    pub fn bytes(&self, component: PackageComponent) -> &[u8] {
        match component {
            PackageComponent::Uasset => &self.uasset,
            PackageComponent::Uexp => &self.uexp,
        }
    }

    pub fn len(&self, component: PackageComponent) -> usize {
        self.bytes(component).len()
    }

    pub fn is_empty(&self, component: PackageComponent) -> bool {
        self.bytes(component).is_empty()
    }

    /// Borrow an explicitly selected byte range without interpreting it.
    pub fn slice(
        &self,
        component: PackageComponent,
        offset: usize,
        length: usize,
    ) -> Result<&[u8], PackageError> {
        let bytes = self.bytes(component);
        let range = checked_range(component, bytes.len(), offset, length)?;
        Ok(&bytes[range])
    }

    /// Replace an explicitly selected range, preserving every byte outside it.
    ///
    /// The replacement length must match the range length. This carrier does
    /// not guess which version-specific offsets would need adjustment after a
    /// resizing edit.
    pub fn replace_range(
        &mut self,
        component: PackageComponent,
        offset: usize,
        length: usize,
        replacement: &[u8],
    ) -> Result<(), PackageError> {
        let component_length = self.bytes(component).len();
        let range = checked_range(component, component_length, offset, length)?;
        if replacement.len() != length {
            return Err(PackageError::ReplacementLengthMismatch {
                range_length: length,
                replacement_length: replacement.len(),
            });
        }
        match component {
            PackageComponent::Uasset => self.uasset[range].copy_from_slice(replacement),
            PackageComponent::Uexp => self.uexp[range].copy_from_slice(replacement),
        }
        Ok(())
    }

    /// Replace an explicitly selected range only when its current bytes still
    /// equal `expected`.
    ///
    /// The expected and replacement lengths must match. Bounds, length, and
    /// byte equality are all checked before mutation, so every error leaves
    /// both package components unchanged.
    pub fn replace_range_if_equal(
        &mut self,
        component: PackageComponent,
        offset: usize,
        expected: &[u8],
        replacement: &[u8],
    ) -> Result<(), PackageError> {
        let component_length = self.bytes(component).len();
        let range = checked_range(component, component_length, offset, expected.len())?;
        if replacement.len() != expected.len() {
            return Err(PackageError::ReplacementLengthMismatch {
                range_length: expected.len(),
                replacement_length: replacement.len(),
            });
        }

        let mismatch = self.bytes(component)[range.clone()]
            .iter()
            .zip(expected)
            .enumerate()
            .find(|(_, (actual, expected))| actual != expected);
        if let Some((relative_offset, (&actual, &expected_byte))) = mismatch {
            return Err(PackageError::RangeDrift {
                component,
                offset,
                length: expected.len(),
                mismatch_offset: offset + relative_offset,
                expected: expected_byte,
                actual,
            });
        }

        match component {
            PackageComponent::Uasset => self.uasset[range].copy_from_slice(replacement),
            PackageComponent::Uexp => self.uexp[range].copy_from_slice(replacement),
        }
        Ok(())
    }

    /// Stage, hash-verify, and publish this package to a new sibling
    /// `.uasset` / `.uexp` pair. Existing files and source files are refused.
    pub fn write_new(
        &self,
        output_uasset: impl AsRef<Path>,
    ) -> Result<PackageWriteReceipt, PackageError> {
        validate_pair_sizes(self.uasset.len(), self.uexp.len(), self.limits)?;
        let output = normalized_output_paths(output_uasset.as_ref())?;
        self.validate_output_paths(&output)?;

        let parent = output
            .uasset()
            .parent()
            .expect("normalized output always has a parent");
        let staged_uasset = stage_component(
            parent,
            PackageComponent::Uasset,
            &self.uasset,
            self.limits.max_uasset_bytes,
        )?;
        let staged_uexp = stage_component(
            parent,
            PackageComponent::Uexp,
            &self.uexp,
            self.limits.max_uexp_bytes,
        )?;

        publish_staged_pair_with(
            &output,
            staged_uasset,
            staged_uexp,
            &self.uasset,
            &self.uexp,
            self.limits,
            persist_new,
            |published, limit| published.verify(limit),
            PublishedComponent::remove_if_owned,
        )
    }

    fn validate_output_paths(&self, output: &PackagePaths) -> Result<(), PackageError> {
        if let Some(source) = &self.source {
            for destination in [output.uasset(), output.uexp()] {
                if [source.uasset(), source.uexp()].contains(&destination) {
                    return Err(PackageError::InPlaceOutput(destination.to_path_buf()));
                }
            }
        }
        for destination in [output.uasset(), output.uexp()] {
            match fs::symlink_metadata(destination) {
                Ok(_) => return Err(PackageError::DestinationExists(destination.to_path_buf())),
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(source) => {
                    return Err(PackageError::Io {
                        operation: "inspect output",
                        path: destination.to_path_buf(),
                        source,
                    });
                }
            }
        }
        Ok(())
    }
}

fn validate_package_path(path: &Path, component: PackageComponent) -> Result<(), PackageError> {
    let hidden_extension_only_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.eq_ignore_ascii_case(&format!(".{}", component.extension())));
    if hidden_extension_only_name {
        return Err(PackageError::MissingFileStem(path.to_path_buf()));
    }
    let valid_extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case(component.extension()));
    if !valid_extension {
        return Err(PackageError::InvalidExtension {
            expected: component.extension(),
            path: path.to_path_buf(),
        });
    }
    if path
        .file_stem()
        .is_none_or(|stem| stem.is_empty() || stem == ".")
    {
        return Err(PackageError::MissingFileStem(path.to_path_buf()));
    }
    Ok(())
}

#[cfg(windows)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FileIdentity {
    volume: u64,
    id: [u8; 16],
}

#[cfg(unix)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FileIdentity {
    device: u64,
    inode: u64,
}

#[cfg(not(any(unix, windows)))]
#[derive(Debug, Clone, PartialEq, Eq)]
struct FileIdentity {
    length: u64,
    created: Option<std::time::SystemTime>,
    modified: Option<std::time::SystemTime>,
}

#[cfg(windows)]
fn identity_from_open_file(file: &File, path: &Path) -> Result<FileIdentity, PackageError> {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Storage::FileSystem::{
        FileIdInfo, GetFileInformationByHandleEx, FILE_ID_INFO,
    };

    let mut id = FILE_ID_INFO::default();
    let ok = unsafe {
        GetFileInformationByHandleEx(
            file.as_raw_handle(),
            FileIdInfo,
            (&mut id as *mut FILE_ID_INFO).cast(),
            std::mem::size_of::<FILE_ID_INFO>() as u32,
        )
    };
    if ok == 0 {
        return Err(PackageError::Io {
            operation: "query opened file identity",
            path: path.to_path_buf(),
            source: io::Error::last_os_error(),
        });
    }
    Ok(FileIdentity {
        volume: id.VolumeSerialNumber,
        id: id.FileId.Identifier,
    })
}

#[cfg(unix)]
fn identity_from_open_file(file: &File, path: &Path) -> Result<FileIdentity, PackageError> {
    use std::os::unix::fs::MetadataExt;

    let metadata = file.metadata().map_err(|source| PackageError::Io {
        operation: "query opened file identity",
        path: path.to_path_buf(),
        source,
    })?;
    Ok(FileIdentity {
        device: metadata.dev(),
        inode: metadata.ino(),
    })
}

#[cfg(not(any(unix, windows)))]
fn identity_from_open_file(file: &File, path: &Path) -> Result<FileIdentity, PackageError> {
    let metadata = file.metadata().map_err(|source| PackageError::Io {
        operation: "query opened file identity",
        path: path.to_path_buf(),
        source,
    })?;
    Ok(FileIdentity {
        length: metadata.len(),
        created: metadata.created().ok(),
        modified: metadata.modified().ok(),
    })
}

#[cfg(windows)]
fn metadata_is_reparse_point(metadata: &Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    use windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_REPARSE_POINT;

    metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

#[cfg(not(windows))]
fn metadata_is_reparse_point(_metadata: &Metadata) -> bool {
    false
}

fn ensure_regular_metadata(path: &Path, metadata: &Metadata) -> Result<(), PackageError> {
    if metadata.file_type().is_symlink() || metadata_is_reparse_point(metadata) {
        return Err(PackageError::SymlinkPath(path.to_path_buf()));
    }
    if !metadata.is_file() {
        return Err(PackageError::NotRegularFile(path.to_path_buf()));
    }
    Ok(())
}

fn open_read_without_following_reparse(path: &Path) -> io::Result<File> {
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        use windows_sys::Win32::Storage::FileSystem::FILE_FLAG_OPEN_REPARSE_POINT;

        options.custom_flags(FILE_FLAG_OPEN_REPARSE_POINT);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;

        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC);
    }
    options.open(path)
}

fn canonical_regular_path(path: &Path) -> Result<PathBuf, PackageError> {
    let original = open_regular(path)?;
    let original_identity = identity_from_open_file(&original, path)?;
    let canonical = fs::canonicalize(path).map_err(|source| PackageError::Io {
        operation: "canonicalize input",
        path: path.to_path_buf(),
        source,
    })?;
    let canonical_file = open_regular(&canonical)?;
    if identity_from_open_file(&canonical_file, &canonical)? != original_identity {
        return Err(PackageError::ConcurrentIdentityChange(path.to_path_buf()));
    }
    Ok(canonical)
}

fn normalized_output_paths(path: &Path) -> Result<PackagePaths, PackageError> {
    let paths = PackagePaths::from_uasset(path)?;
    let parent = paths
        .uasset()
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let canonical_parent = fs::canonicalize(parent).map_err(|source| PackageError::Io {
        operation: "canonicalize output parent",
        path: parent.to_path_buf(),
        source,
    })?;
    if !canonical_parent.is_dir() {
        return Err(PackageError::OutputParentNotDirectory(canonical_parent));
    }
    let file_name = paths
        .uasset()
        .file_name()
        .expect("validated package path has a file name");
    PackagePaths::from_uasset(canonical_parent.join(file_name))
}

fn read_verified_component(
    path: &Path,
    component: PackageComponent,
    component_limit: u64,
    already_loaded: u64,
    total_limit: u64,
) -> Result<Vec<u8>, PackageError> {
    let mut file = open_regular(path)?;
    let advertised = file_metadata_len(&file, path)?;
    check_component_limit(component, advertised, component_limit)?;
    check_total_limit(already_loaded, advertised, total_limit)?;
    let allocation =
        usize::try_from(advertised).map_err(|_| PackageError::LengthDoesNotFitMemory {
            component,
            length: advertised,
        })?;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(allocation)
        .map_err(|error| PackageError::AllocationFailed {
            component,
            length: allocation,
            message: error.to_string(),
        })?;
    let remaining_total = total_limit.saturating_sub(already_loaded);
    let read_limit = component_limit.min(remaining_total).saturating_add(1);
    (&mut file)
        .take(read_limit)
        .read_to_end(&mut bytes)
        .map_err(|source| PackageError::Io {
            operation: "read input",
            path: path.to_path_buf(),
            source,
        })?;
    let actual = u64::try_from(bytes.len()).map_err(|_| PackageError::SizeOverflow)?;
    check_component_limit(component, actual, component_limit)?;
    check_total_limit(already_loaded, actual, total_limit)?;
    if actual != advertised {
        return Err(PackageError::ConcurrentLengthChange {
            path: path.to_path_buf(),
            expected: advertised,
            actual,
        });
    }
    let expected_hash = sha256(&bytes);
    verify_path(path, component, actual, expected_hash, component_limit)?;
    Ok(bytes)
}

fn check_total_limit(already_loaded: u64, component: u64, limit: u64) -> Result<(), PackageError> {
    let actual = already_loaded
        .checked_add(component)
        .ok_or(PackageError::SizeOverflow)?;
    if actual > limit {
        return Err(PackageError::PairTooLarge { actual, limit });
    }
    Ok(())
}

fn verify_loaded_pair(
    paths: &PackagePaths,
    uasset: &[u8],
    uexp: &[u8],
    limits: PackageLimits,
) -> Result<(), PackageError> {
    verify_loaded_pair_with(paths, uasset, uexp, limits, verify_path)
}

fn verify_loaded_pair_with<F>(
    paths: &PackagePaths,
    uasset: &[u8],
    uexp: &[u8],
    limits: PackageLimits,
    mut verify: F,
) -> Result<(), PackageError>
where
    F: FnMut(&Path, PackageComponent, u64, [u8; 32], u64) -> Result<(), PackageError>,
{
    // Payload first, commit marker last. These sequential point checks detect a
    // change that overlaps an observation, but they require the caller to keep
    // concurrent source writers out and do not create an atomic pair snapshot.
    for (component, bytes, limit) in [
        (PackageComponent::Uexp, uexp, limits.max_uexp_bytes),
        (PackageComponent::Uasset, uasset, limits.max_uasset_bytes),
    ] {
        let path = paths.component(component);
        let expected_length = u64::try_from(bytes.len()).map_err(|_| PackageError::SizeOverflow)?;
        let expected_hash = sha256(bytes);
        verify(path, component, expected_length, expected_hash, limit).map_err(|source| {
            PackageError::PairGenerationChanged {
                component,
                path: path.to_path_buf(),
                source: Box::new(source),
            }
        })?;
    }
    Ok(())
}

fn validate_pair_sizes(
    uasset_length: usize,
    uexp_length: usize,
    limits: PackageLimits,
) -> Result<(), PackageError> {
    let uasset_length = u64::try_from(uasset_length).map_err(|_| PackageError::SizeOverflow)?;
    let uexp_length = u64::try_from(uexp_length).map_err(|_| PackageError::SizeOverflow)?;
    validate_pair_sizes_u64(uasset_length, uexp_length, limits)
}

fn validate_pair_sizes_u64(
    uasset_length: u64,
    uexp_length: u64,
    limits: PackageLimits,
) -> Result<(), PackageError> {
    check_component_limit(
        PackageComponent::Uasset,
        uasset_length,
        limits.max_uasset_bytes,
    )?;
    check_component_limit(PackageComponent::Uexp, uexp_length, limits.max_uexp_bytes)?;
    let total = uasset_length
        .checked_add(uexp_length)
        .ok_or(PackageError::SizeOverflow)?;
    if total > limits.max_total_bytes {
        return Err(PackageError::PairTooLarge {
            actual: total,
            limit: limits.max_total_bytes,
        });
    }
    Ok(())
}

fn inspect_component_length(
    path: &Path,
    component: PackageComponent,
    limit: u64,
) -> Result<u64, PackageError> {
    let file = open_regular(path)?;
    let length = file_metadata_len(&file, path)?;
    check_component_limit(component, length, limit)?;
    Ok(length)
}

fn check_component_limit(
    component: PackageComponent,
    actual: u64,
    limit: u64,
) -> Result<(), PackageError> {
    if actual > limit {
        return Err(PackageError::ComponentTooLarge {
            component,
            actual,
            limit,
        });
    }
    Ok(())
}

fn checked_range(
    component: PackageComponent,
    component_length: usize,
    offset: usize,
    length: usize,
) -> Result<std::ops::Range<usize>, PackageError> {
    let end = offset
        .checked_add(length)
        .ok_or(PackageError::RangeOverflow { offset, length })?;
    if end > component_length {
        return Err(PackageError::RangeOutOfBounds {
            component,
            offset,
            end,
            component_length,
        });
    }
    Ok(offset..end)
}

fn open_checked_regular_handle(path: &Path) -> Result<File, PackageError> {
    let link_metadata = fs::symlink_metadata(path).map_err(|source| PackageError::Io {
        operation: "inspect file",
        path: path.to_path_buf(),
        source,
    })?;
    ensure_regular_metadata(path, &link_metadata)?;
    let file = open_read_without_following_reparse(path).map_err(|source| PackageError::Io {
        operation: "open file without following links",
        path: path.to_path_buf(),
        source,
    })?;
    let opened = file.metadata().map_err(|source| PackageError::Io {
        operation: "read opened file metadata",
        path: path.to_path_buf(),
        source,
    })?;
    ensure_regular_metadata(path, &opened)?;
    Ok(file)
}

fn open_regular(path: &Path) -> Result<File, PackageError> {
    let file = open_checked_regular_handle(path)?;
    let opened_identity = identity_from_open_file(&file, path)?;
    let recheck = open_checked_regular_handle(path)?;
    if identity_from_open_file(&recheck, path)? != opened_identity {
        return Err(PackageError::ConcurrentIdentityChange(path.to_path_buf()));
    }
    Ok(file)
}

fn current_path_identity(path: &Path) -> Result<FileIdentity, PackageError> {
    let file = open_regular(path)?;
    identity_from_open_file(&file, path)
}

fn file_metadata_len(file: &File, path: &Path) -> Result<u64, PackageError> {
    let metadata = file.metadata().map_err(|source| PackageError::Io {
        operation: "read file metadata",
        path: path.to_path_buf(),
        source,
    })?;
    if !metadata.is_file() {
        return Err(PackageError::NotRegularFile(path.to_path_buf()));
    }
    Ok(metadata.len())
}

fn stage_component(
    parent: &Path,
    component: PackageComponent,
    bytes: &[u8],
    limit: u64,
) -> Result<NamedTempFile, PackageError> {
    let mut temp = tempfile::Builder::new()
        .prefix(".gore-asset-")
        .suffix(&format!(".{}.tmp", component.extension()))
        .tempfile_in(parent)
        .map_err(|source| PackageError::Io {
            operation: "create staged output",
            path: parent.to_path_buf(),
            source,
        })?;
    temp.as_file_mut()
        .write_all(bytes)
        .map_err(|source| PackageError::Io {
            operation: "write staged output",
            path: temp.path().to_path_buf(),
            source,
        })?;
    temp.as_file_mut()
        .flush()
        .map_err(|source| PackageError::Io {
            operation: "flush staged output",
            path: temp.path().to_path_buf(),
            source,
        })?;
    temp.as_file()
        .sync_all()
        .map_err(|source| PackageError::Io {
            operation: "sync staged output",
            path: temp.path().to_path_buf(),
            source,
        })?;

    let expected_length = u64::try_from(bytes.len()).map_err(|_| PackageError::SizeOverflow)?;
    verify_path(
        temp.path(),
        component,
        expected_length,
        sha256(bytes),
        limit,
    )?;
    Ok(temp)
}

#[derive(Debug)]
struct PublishedHandle {
    file: File,
    identity: FileIdentity,
}

fn persist_new(temp: NamedTempFile, destination: &Path) -> Result<PublishedHandle, PackageError> {
    let identity = identity_from_open_file(temp.as_file(), temp.path())?;
    match temp.persist_noclobber(destination) {
        // The staged file was already flushed and synced. Retain its open
        // handle so post-publication verification and cleanup can compare file
        // identity rather than trusting only a reusable path.
        Ok(file) => {
            sync_parent_directory_best_effort(destination);
            Ok(PublishedHandle { file, identity })
        }
        Err(error) => Err(PackageError::Io {
            operation: "publish new output",
            path: destination.to_path_buf(),
            source: error.error,
        }),
    }
}

fn sync_parent_directory_best_effort(path: &Path) {
    // Unix permits opening and syncing a directory to durably record a rename
    // or unlink. Windows does not expose a portable std equivalent. This is
    // intentionally best-effort: once a name is visible, a directory-sync
    // failure must not be reported as though publication never happened.
    #[cfg(unix)]
    if let Some(parent) = path.parent() {
        if let Ok(directory) = File::open(parent) {
            let _ = directory.sync_all();
        }
    }
    #[cfg(not(unix))]
    let _ = path;
}

#[derive(Debug)]
struct PublishedComponent {
    component: PackageComponent,
    path: PathBuf,
    length: u64,
    sha256: [u8; 32],
    handle: PublishedHandle,
}

impl PublishedComponent {
    fn new(
        component: PackageComponent,
        path: &Path,
        expected: &[u8],
        handle: PublishedHandle,
    ) -> Self {
        Self {
            component,
            path: path.to_path_buf(),
            length: expected.len() as u64,
            sha256: sha256(expected),
            handle,
        }
    }

    fn verify(&self, limit: u64) -> Result<ComponentDigest, PackageError> {
        let mut file = open_regular(&self.path)?;
        if identity_from_open_file(&file, &self.path)? != self.handle.identity {
            return Err(PackageError::VerificationIdentity(self.path.clone()));
        }
        verify_open_file(
            &mut file,
            &self.path,
            self.component,
            self.length,
            self.sha256,
            limit,
        )?;
        if current_path_identity(&self.path)? != self.handle.identity {
            return Err(PackageError::VerificationIdentity(self.path.clone()));
        }
        Ok(ComponentDigest {
            path: self.path.clone(),
            length: self.length,
            sha256: self.sha256,
        })
    }

    fn remove_if_owned(&self) -> Result<(), CleanupFailure> {
        self.remove_if_owned_with_before_final_check(|_| {})
    }

    fn remove_if_owned_with_before_final_check<F>(
        &self,
        before_final_check: F,
    ) -> Result<(), CleanupFailure>
    where
        F: FnOnce(&Path),
    {
        let published_identity = identity_from_open_file(&self.handle.file, &self.path)
            .map_err(|source| self.cleanup_verification_failure(source))?;
        if published_identity != self.handle.identity {
            return Err(self.ownership_changed("published file-handle identity changed"));
        }

        let mut current = match open_regular(&self.path) {
            Ok(file) => file,
            Err(PackageError::Io { source, .. }) if source.kind() == io::ErrorKind::NotFound => {
                return Ok(());
            }
            Err(source) => return Err(self.cleanup_verification_failure(source)),
        };
        let current_identity = identity_from_open_file(&current, &self.path)
            .map_err(|source| self.cleanup_verification_failure(source))?;
        if current_identity != self.handle.identity {
            return Err(self.ownership_changed("path no longer identifies the published file"));
        }
        if let Err(source) = verify_open_file(
            &mut current,
            &self.path,
            self.component,
            self.length,
            self.sha256,
            self.length,
        ) {
            return Err(self.cleanup_verification_failure(source));
        }

        // Tests can deterministically replace the name here. Production uses a
        // no-op; the following lstat is the last identity check available via
        // std before the necessarily path-based unlink.
        before_final_check(&self.path);
        match current_path_identity(&self.path) {
            Err(PackageError::Io { source, .. }) if source.kind() == io::ErrorKind::NotFound => {
                return Ok(());
            }
            Err(source) => return Err(self.cleanup_verification_failure(source)),
            Ok(identity) if identity != self.handle.identity => {
                return Err(self.ownership_changed("path identity changed before removal"));
            }
            Ok(_) => {}
        }
        drop(current);
        match fs::remove_file(&self.path) {
            Ok(()) => {
                sync_parent_directory_best_effort(&self.path);
                Ok(())
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(source) => Err(CleanupFailure::Remove {
                component: self.component,
                path: self.path.clone(),
                source,
            }),
        }
    }

    fn ownership_changed(&self, reason: &'static str) -> CleanupFailure {
        CleanupFailure::OwnershipChanged {
            component: self.component,
            path: self.path.clone(),
            reason,
        }
    }

    fn cleanup_verification_failure(&self, source: PackageError) -> CleanupFailure {
        match source {
            PackageError::VerificationLength { .. }
            | PackageError::VerificationHash(_)
            | PackageError::VerificationIdentity(_)
            | PackageError::ConcurrentIdentityChange(_)
            | PackageError::SymlinkPath(_)
            | PackageError::NotRegularFile(_) => {
                self.ownership_changed("identity, length, or SHA-256 no longer matches")
            }
            source => CleanupFailure::Verify {
                component: self.component,
                path: self.path.clone(),
                source: Box::new(source),
            },
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn publish_staged_pair_with<P, V, C>(
    output: &PackagePaths,
    staged_uasset: NamedTempFile,
    staged_uexp: NamedTempFile,
    expected_uasset: &[u8],
    expected_uexp: &[u8],
    limits: PackageLimits,
    mut persist: P,
    mut verify: V,
    mut cleanup: C,
) -> Result<PackageWriteReceipt, PackageError>
where
    P: FnMut(NamedTempFile, &Path) -> Result<PublishedHandle, PackageError>,
    V: FnMut(&PublishedComponent, u64) -> Result<ComponentDigest, PackageError>,
    C: FnMut(&PublishedComponent) -> Result<(), CleanupFailure>,
{
    // Publish the payload first. The `.uasset` is the commit marker and is
    // deliberately the final visible rename.
    let uexp_handle = persist(staged_uexp, output.uexp())?;
    let uexp = PublishedComponent::new(
        PackageComponent::Uexp,
        output.uexp(),
        expected_uexp,
        uexp_handle,
    );
    let uasset_handle = match persist(staged_uasset, output.uasset()) {
        Ok(handle) => handle,
        Err(cause) => return publication_failure(cause, [&uexp], &mut cleanup),
    };
    let uasset = PublishedComponent::new(
        PackageComponent::Uasset,
        output.uasset(),
        expected_uasset,
        uasset_handle,
    );

    // Verify the payload first and the commit marker last, matching load order.
    let uexp_receipt = match verify(&uexp, limits.max_uexp_bytes) {
        Ok(receipt) => receipt,
        Err(cause) => return publication_failure(cause, [&uasset, &uexp], &mut cleanup),
    };
    let uasset_receipt = match verify(&uasset, limits.max_uasset_bytes) {
        Ok(receipt) => receipt,
        Err(cause) => return publication_failure(cause, [&uasset, &uexp], &mut cleanup),
    };

    Ok(PackageWriteReceipt {
        uasset: uasset_receipt,
        uexp: uexp_receipt,
    })
}

fn publication_failure<C, const N: usize>(
    cause: PackageError,
    cleanup_order: [&PublishedComponent; N],
    cleanup: &mut C,
) -> Result<PackageWriteReceipt, PackageError>
where
    C: FnMut(&PublishedComponent) -> Result<(), CleanupFailure>,
{
    let mut cleanup_failures = Vec::new();
    for published in cleanup_order {
        if let Err(failure) = cleanup(published) {
            cleanup_failures.push(failure);
            // Cleanup order encodes dependency: `.uasset` is the commit marker
            // and `.uexp` its payload. If marker cleanup fails or is refused,
            // retain the payload rather than manufacturing a visible marker
            // whose required sibling was deleted.
            break;
        }
    }
    if cleanup_failures.is_empty() {
        Err(cause)
    } else {
        Err(PackageError::PublishCleanupFailed {
            cause: Box::new(cause),
            cleanup_failures,
        })
    }
}

fn verify_path(
    path: &Path,
    component: PackageComponent,
    expected_length: u64,
    expected_hash: [u8; 32],
    limit: u64,
) -> Result<(), PackageError> {
    let mut file = open_regular(path)?;
    let opened_identity = identity_from_open_file(&file, path)?;
    verify_open_file(
        &mut file,
        path,
        component,
        expected_length,
        expected_hash,
        limit,
    )?;
    if current_path_identity(path)? != opened_identity {
        return Err(PackageError::ConcurrentIdentityChange(path.to_path_buf()));
    }
    Ok(())
}

fn verify_open_file(
    file: &mut File,
    path: &Path,
    component: PackageComponent,
    expected_length: u64,
    expected_hash: [u8; 32],
    limit: u64,
) -> Result<(), PackageError> {
    let metadata_length = file_metadata_len(file, path)?;
    if metadata_length != expected_length {
        return Err(PackageError::VerificationLength {
            path: path.to_path_buf(),
            expected: expected_length,
            actual: metadata_length,
        });
    }
    check_component_limit(component, metadata_length, limit)?;
    file.seek(SeekFrom::Start(0))
        .map_err(|source| PackageError::Io {
            operation: "rewind reopened file",
            path: path.to_path_buf(),
            source,
        })?;

    let mut hasher = Sha256::new();
    let mut buffer = [0u8; HASH_BUFFER_BYTES];
    let mut actual_length = 0u64;
    loop {
        let read = file.read(&mut buffer).map_err(|source| PackageError::Io {
            operation: "verify reopened file",
            path: path.to_path_buf(),
            source,
        })?;
        if read == 0 {
            break;
        }
        actual_length = actual_length
            .checked_add(u64::try_from(read).map_err(|_| PackageError::SizeOverflow)?)
            .ok_or(PackageError::SizeOverflow)?;
        if actual_length > limit {
            return Err(PackageError::VerificationLength {
                path: path.to_path_buf(),
                expected: expected_length,
                actual: actual_length,
            });
        }
        hasher.update(&buffer[..read]);
    }
    if actual_length != expected_length {
        return Err(PackageError::VerificationLength {
            path: path.to_path_buf(),
            expected: expected_length,
            actual: actual_length,
        });
    }
    let actual_hash: [u8; 32] = hasher.finalize().into();
    if actual_hash != expected_hash {
        return Err(PackageError::VerificationHash(path.to_path_buf()));
    }
    Ok(())
}

fn sha256(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn small_limits() -> PackageLimits {
        PackageLimits {
            max_uasset_bytes: 64,
            max_uexp_bytes: 64,
            max_total_bytes: 96,
        }
    }

    #[test]
    fn derives_the_coupled_path_and_rejects_wrong_extensions() {
        let paths = PackagePaths::from_uasset("Some.Asset.uasset").unwrap();
        assert_eq!(paths.uasset(), Path::new("Some.Asset.uasset"));
        assert_eq!(paths.uexp(), Path::new("Some.Asset.uexp"));
        assert!(matches!(
            PackagePaths::from_uasset("Some.Asset.bin"),
            Err(PackageError::InvalidExtension {
                expected: "uasset",
                ..
            })
        ));
        assert!(matches!(
            PackagePaths::from_uasset(".uasset"),
            Err(PackageError::MissingFileStem(_))
        ));
    }

    #[test]
    fn explicit_ranges_check_overflow_and_bounds() {
        let carrier =
            PackageCarrier::from_bytes(vec![0, 1, 2, 3], vec![4, 5, 6], small_limits()).unwrap();
        assert_eq!(
            carrier.slice(PackageComponent::Uasset, 1, 2).unwrap(),
            &[1, 2]
        );
        assert!(matches!(
            carrier.slice(PackageComponent::Uasset, usize::MAX, 2),
            Err(PackageError::RangeOverflow { .. })
        ));
        assert!(matches!(
            carrier.slice(PackageComponent::Uexp, 2, 2),
            Err(PackageError::RangeOutOfBounds {
                component: PackageComponent::Uexp,
                component_length: 3,
                ..
            })
        ));
    }

    #[test]
    fn replacement_is_same_length_and_preserves_all_unknown_bytes() {
        let original_uasset = (0..32).collect::<Vec<_>>();
        let original_uexp = (100..132).collect::<Vec<_>>();
        let mut carrier = PackageCarrier::from_bytes(
            original_uasset.clone(),
            original_uexp.clone(),
            small_limits(),
        )
        .unwrap();

        carrier
            .replace_range(PackageComponent::Uexp, 8, 4, &[9, 8, 7, 6])
            .unwrap();
        assert_eq!(
            &carrier.bytes(PackageComponent::Uexp)[..8],
            &original_uexp[..8]
        );
        assert_eq!(
            &carrier.bytes(PackageComponent::Uexp)[12..],
            &original_uexp[12..]
        );
        assert_eq!(carrier.bytes(PackageComponent::Uasset), original_uasset);
        assert!(matches!(
            carrier.replace_range(PackageComponent::Uasset, 1, 2, &[1]),
            Err(PackageError::ReplacementLengthMismatch {
                range_length: 2,
                replacement_length: 1
            })
        ));
    }

    #[test]
    fn conditional_replacement_is_strict_and_preserves_every_other_byte() {
        let original_uasset = (0..32).collect::<Vec<_>>();
        let original_uexp = (100..132).collect::<Vec<_>>();
        let mut carrier = PackageCarrier::from_bytes(
            original_uasset.clone(),
            original_uexp.clone(),
            small_limits(),
        )
        .unwrap();
        let expected = &original_uexp[8..12];
        let replacement = [9, 8, 7, 6];

        carrier
            .replace_range_if_equal(PackageComponent::Uexp, 8, expected, &replacement)
            .unwrap();
        assert_eq!(
            &carrier.bytes(PackageComponent::Uexp)[..8],
            &original_uexp[..8]
        );
        assert_eq!(&carrier.bytes(PackageComponent::Uexp)[8..12], &replacement);
        assert_eq!(
            &carrier.bytes(PackageComponent::Uexp)[12..],
            &original_uexp[12..]
        );
        assert_eq!(carrier.bytes(PackageComponent::Uasset), original_uasset);

        let once_applied_uasset = carrier.bytes(PackageComponent::Uasset).to_vec();
        let once_applied_uexp = carrier.bytes(PackageComponent::Uexp).to_vec();
        assert!(matches!(
            carrier.replace_range_if_equal(PackageComponent::Uexp, 8, expected, &replacement),
            Err(PackageError::RangeDrift {
                component: PackageComponent::Uexp,
                offset: 8,
                length: 4,
                mismatch_offset: 8,
                expected: 108,
                actual: 9,
            })
        ));
        assert_eq!(carrier.bytes(PackageComponent::Uasset), once_applied_uasset);
        assert_eq!(carrier.bytes(PackageComponent::Uexp), once_applied_uexp);
    }

    #[test]
    fn conditional_replacement_reports_first_drift_byte_without_mutating() {
        let original_uasset = vec![10, 11, 12, 13];
        let original_uexp = vec![20, 21, 22, 23, 24];

        for relative_mismatch in 0..3 {
            let mut carrier = PackageCarrier::from_bytes(
                original_uasset.clone(),
                original_uexp.clone(),
                small_limits(),
            )
            .unwrap();
            let mut expected = original_uexp[1..4].to_vec();
            expected[relative_mismatch] ^= 0xff;
            let expected_byte = expected[relative_mismatch];
            let actual_byte = original_uexp[1 + relative_mismatch];

            assert!(matches!(
                carrier.replace_range_if_equal(
                    PackageComponent::Uexp,
                    1,
                    &expected,
                    &[30, 31, 32]
                ),
                Err(PackageError::RangeDrift {
                    component: PackageComponent::Uexp,
                    offset: 1,
                    length: 3,
                    mismatch_offset,
                    expected,
                    actual,
                }) if mismatch_offset == 1 + relative_mismatch
                    && expected == expected_byte
                    && actual == actual_byte
            ));
            assert_eq!(carrier.bytes(PackageComponent::Uasset), original_uasset);
            assert_eq!(carrier.bytes(PackageComponent::Uexp), original_uexp);
        }
    }

    #[test]
    fn every_conditional_replacement_precondition_error_is_non_mutating() {
        let original_uasset = vec![0, 1, 2, 3];
        let original_uexp = vec![4, 5, 6];
        let mut carrier = PackageCarrier::from_bytes(
            original_uasset.clone(),
            original_uexp.clone(),
            small_limits(),
        )
        .unwrap();

        assert!(matches!(
            carrier.replace_range_if_equal(PackageComponent::Uasset, 1, &[1, 2], &[9]),
            Err(PackageError::ReplacementLengthMismatch {
                range_length: 2,
                replacement_length: 1,
            })
        ));
        assert_eq!(carrier.bytes(PackageComponent::Uasset), original_uasset);
        assert_eq!(carrier.bytes(PackageComponent::Uexp), original_uexp);

        assert!(matches!(
            carrier.replace_range_if_equal(PackageComponent::Uexp, 2, &[6, 7], &[8, 9]),
            Err(PackageError::RangeOutOfBounds {
                component: PackageComponent::Uexp,
                offset: 2,
                end: 4,
                component_length: 3,
            })
        ));
        assert_eq!(carrier.bytes(PackageComponent::Uasset), original_uasset);
        assert_eq!(carrier.bytes(PackageComponent::Uexp), original_uexp);

        assert!(matches!(
            carrier.replace_range_if_equal(PackageComponent::Uasset, usize::MAX, &[0, 1], &[8, 9]),
            Err(PackageError::RangeOverflow {
                offset: usize::MAX,
                length: 2,
            })
        ));
        assert_eq!(carrier.bytes(PackageComponent::Uasset), original_uasset);
        assert_eq!(carrier.bytes(PackageComponent::Uexp), original_uexp);
    }

    #[test]
    fn limits_cover_each_component_and_the_pair() {
        let limits = PackageLimits {
            max_uasset_bytes: 4,
            max_uexp_bytes: 4,
            max_total_bytes: 6,
        };
        assert!(matches!(
            PackageCarrier::from_bytes(vec![0; 5], vec![], limits),
            Err(PackageError::ComponentTooLarge {
                component: PackageComponent::Uasset,
                actual: 5,
                limit: 4
            })
        ));
        assert!(matches!(
            PackageCarrier::from_bytes(vec![0; 4], vec![0; 3], limits),
            Err(PackageError::PairTooLarge {
                actual: 7,
                limit: 6
            })
        ));
    }

    #[test]
    fn loads_and_reopens_a_real_pair_without_changing_bytes() {
        let temp = tempfile::tempdir().unwrap();
        let uasset_path = temp.path().join("Fixture.uasset");
        let uexp_path = temp.path().join("Fixture.uexp");
        let uasset = b"opaque header bytes";
        let uexp = b"opaque export bytes\0\xff";
        fs::write(&uasset_path, uasset).unwrap();
        fs::write(&uexp_path, uexp).unwrap();

        let carrier = PackageCarrier::load(&uasset_path, small_limits()).unwrap();
        assert_eq!(carrier.bytes(PackageComponent::Uasset), uasset);
        assert_eq!(carrier.bytes(PackageComponent::Uexp), uexp);
        let source = carrier.source_paths().unwrap();
        assert!(source.uasset().is_absolute());
        assert!(source.uexp().is_absolute());
    }

    #[test]
    fn load_checks_the_pair_limit_before_reading_components() {
        let temp = tempfile::tempdir().unwrap();
        let uasset_path = temp.path().join("TooLargeTogether.uasset");
        fs::write(&uasset_path, [1; 4]).unwrap();
        fs::write(uasset_path.with_extension("uexp"), [2; 3]).unwrap();
        let limits = PackageLimits {
            max_uasset_bytes: 8,
            max_uexp_bytes: 8,
            max_total_bytes: 6,
        };

        assert!(matches!(
            PackageCarrier::load(&uasset_path, limits),
            Err(PackageError::PairTooLarge {
                actual: 7,
                limit: 6
            })
        ));
    }

    #[test]
    fn load_caps_replaced_second_component_to_remaining_pair_budget() {
        let temp = tempfile::tempdir().unwrap();
        let uasset = temp.path().join("GrowingPair.uasset");
        fs::write(&uasset, [1; 4]).unwrap();
        fs::write(uasset.with_extension("uexp"), [2; 2]).unwrap();
        let limits = PackageLimits {
            max_uasset_bytes: 8,
            max_uexp_bytes: 8,
            max_total_bytes: 6,
        };

        let result = PackageCarrier::load_with_hooks(
            &uasset,
            limits,
            |paths| fs::write(paths.uexp(), [3; 5]).unwrap(),
            |_| {},
        );
        assert!(matches!(
            result,
            Err(PackageError::PairTooLarge {
                actual: 9,
                limit: 6
            })
        ));
    }

    #[test]
    fn load_rechecks_payload_then_commit_marker_after_both_reads() {
        let paths = PackagePaths::from_uasset("Fixture.uasset").unwrap();
        let mut order = Vec::new();
        verify_loaded_pair_with(
            &paths,
            b"head",
            b"body",
            small_limits(),
            |_path, component, _length, _hash, _limit| {
                order.push(component);
                Ok(())
            },
        )
        .unwrap();
        assert_eq!(order, [PackageComponent::Uexp, PackageComponent::Uasset]);
    }

    #[test]
    fn load_rejects_either_component_changing_after_both_reads() {
        for changed in [PackageComponent::Uexp, PackageComponent::Uasset] {
            let temp = tempfile::tempdir().unwrap();
            let uasset = temp.path().join("Fixture.uasset");
            fs::write(&uasset, b"head-a").unwrap();
            fs::write(uasset.with_extension("uexp"), b"body-a").unwrap();

            let result = PackageCarrier::load_with_after_reads(&uasset, small_limits(), |paths| {
                fs::write(
                    paths.component(changed),
                    match changed {
                        PackageComponent::Uasset => &b"head-b"[..],
                        PackageComponent::Uexp => &b"body-b"[..],
                    },
                )
                .unwrap();
            });
            assert!(matches!(
                result,
                Err(PackageError::PairGenerationChanged { component, .. })
                    if component == changed
            ));
        }
    }

    #[test]
    fn writes_only_a_new_verified_pair() {
        let temp = tempfile::tempdir().unwrap();
        let output = temp.path().join("Edited.uasset");
        let expected_uasset = b"unknown-uasset".to_vec();
        let expected_uexp = b"unknown-uexp".to_vec();
        let carrier = PackageCarrier::from_bytes(
            expected_uasset.clone(),
            expected_uexp.clone(),
            small_limits(),
        )
        .unwrap();

        let receipt = carrier.write_new(&output).unwrap();
        assert_eq!(fs::read(&output).unwrap(), expected_uasset);
        assert_eq!(
            fs::read(output.with_extension("uexp")).unwrap(),
            expected_uexp
        );
        assert_eq!(receipt.uasset.length, 14);
        assert_eq!(receipt.uexp.length, 12);
        assert_eq!(receipt.uasset.sha256, sha256(b"unknown-uasset"));
        assert_eq!(receipt.uexp.sha256, sha256(b"unknown-uexp"));
        assert!(matches!(
            carrier.write_new(&output),
            Err(PackageError::DestinationExists(_))
        ));
    }

    #[test]
    fn second_publish_failure_removes_payload_without_exposing_commit_marker() {
        use std::cell::RefCell;
        use std::rc::Rc;

        let temp = tempfile::tempdir().unwrap();
        let output = PackagePaths::from_uasset(temp.path().join("Edited.uasset")).unwrap();
        let staged_uasset = stage_component(
            temp.path(),
            PackageComponent::Uasset,
            b"head",
            small_limits().max_uasset_bytes,
        )
        .unwrap();
        let staged_uexp = stage_component(
            temp.path(),
            PackageComponent::Uexp,
            b"body",
            small_limits().max_uexp_bytes,
        )
        .unwrap();
        let events = Rc::new(RefCell::new(Vec::new()));
        let publish_events = Rc::clone(&events);
        let cleanup_events = Rc::clone(&events);
        let mut publishes = 0usize;

        let result = publish_staged_pair_with(
            &output,
            staged_uasset,
            staged_uexp,
            b"head",
            b"body",
            small_limits(),
            move |staged, destination| {
                let component = if destination.extension() == Some(std::ffi::OsStr::new("uexp")) {
                    PackageComponent::Uexp
                } else {
                    PackageComponent::Uasset
                };
                publish_events
                    .borrow_mut()
                    .push(format!("publish:{component}"));
                publishes += 1;
                if publishes == 2 {
                    return Err(PackageError::Io {
                        operation: "injected second publish",
                        path: destination.to_path_buf(),
                        source: io::Error::other("injected"),
                    });
                }
                persist_new(staged, destination)
            },
            |_published, _limit| unreachable!("verification follows both publishes"),
            move |published| {
                cleanup_events
                    .borrow_mut()
                    .push(format!("cleanup:{}", published.component));
                published.remove_if_owned()
            },
        );

        assert!(matches!(result, Err(PackageError::Io { .. })));
        assert_eq!(
            *events.borrow(),
            ["publish:uexp", "publish:uasset", "cleanup:uexp"]
        );
        assert!(!output.uasset().exists());
        assert!(!output.uexp().exists());
    }

    #[test]
    fn postverify_failure_cleans_commit_marker_before_payload() {
        use std::cell::RefCell;
        use std::rc::Rc;

        let temp = tempfile::tempdir().unwrap();
        let output = PackagePaths::from_uasset(temp.path().join("Edited.uasset")).unwrap();
        let staged_uasset =
            stage_component(temp.path(), PackageComponent::Uasset, b"head", 64).unwrap();
        let staged_uexp =
            stage_component(temp.path(), PackageComponent::Uexp, b"body", 64).unwrap();
        let events = Rc::new(RefCell::new(Vec::new()));
        let verify_events = Rc::clone(&events);
        let cleanup_events = Rc::clone(&events);

        let result = publish_staged_pair_with(
            &output,
            staged_uasset,
            staged_uexp,
            b"head",
            b"body",
            small_limits(),
            persist_new,
            move |published, limit| {
                verify_events
                    .borrow_mut()
                    .push(format!("verify:{}", published.component));
                if published.component == PackageComponent::Uasset {
                    return Err(PackageError::VerificationHash(published.path.clone()));
                }
                published.verify(limit)
            },
            move |published| {
                cleanup_events
                    .borrow_mut()
                    .push(format!("cleanup:{}", published.component));
                published.remove_if_owned()
            },
        );

        assert!(matches!(result, Err(PackageError::VerificationHash(_))));
        assert_eq!(
            *events.borrow(),
            [
                "verify:uexp",
                "verify:uasset",
                "cleanup:uasset",
                "cleanup:uexp"
            ]
        );
        assert!(!output.uasset().exists());
        assert!(!output.uexp().exists());
    }

    #[test]
    fn marker_cleanup_failure_retains_dependent_payload() {
        use std::cell::RefCell;

        let temp = tempfile::tempdir().unwrap();
        let output = PackagePaths::from_uasset(temp.path().join("Edited.uasset")).unwrap();
        let staged_uasset =
            stage_component(temp.path(), PackageComponent::Uasset, b"head", 64).unwrap();
        let staged_uexp =
            stage_component(temp.path(), PackageComponent::Uexp, b"body", 64).unwrap();
        let cleanup_order = RefCell::new(Vec::new());

        let result = publish_staged_pair_with(
            &output,
            staged_uasset,
            staged_uexp,
            b"head",
            b"body",
            small_limits(),
            persist_new,
            |published, limit| {
                if published.component == PackageComponent::Uasset {
                    return Err(PackageError::VerificationHash(published.path.clone()));
                }
                published.verify(limit)
            },
            |published| {
                cleanup_order.borrow_mut().push(published.component);
                Err(CleanupFailure::OwnershipChanged {
                    component: published.component,
                    path: published.path.clone(),
                    reason: "injected marker cleanup refusal",
                })
            },
        );

        assert!(matches!(
            result,
            Err(PackageError::PublishCleanupFailed {
                cleanup_failures,
                ..
            }) if cleanup_failures.len() == 1
        ));
        assert_eq!(*cleanup_order.borrow(), [PackageComponent::Uasset]);
        assert_eq!(fs::read(output.uasset()).unwrap(), b"head");
        assert_eq!(fs::read(output.uexp()).unwrap(), b"body");
    }

    #[test]
    fn cleanup_never_deletes_a_same_length_changed_destination() {
        let temp = tempfile::tempdir().unwrap();
        let output = PackagePaths::from_uasset(temp.path().join("Edited.uasset")).unwrap();
        let staged_uasset =
            stage_component(temp.path(), PackageComponent::Uasset, b"head", 64).unwrap();
        let staged_uexp =
            stage_component(temp.path(), PackageComponent::Uexp, b"body", 64).unwrap();

        let result = publish_staged_pair_with(
            &output,
            staged_uasset,
            staged_uexp,
            b"head",
            b"body",
            small_limits(),
            persist_new,
            |published, limit| {
                if published.component == PackageComponent::Uasset {
                    fs::write(&published.path, b"evil").unwrap();
                    return Err(PackageError::VerificationHash(published.path.clone()));
                }
                published.verify(limit)
            },
            PublishedComponent::remove_if_owned,
        );

        assert!(matches!(
            result,
            Err(PackageError::PublishCleanupFailed {
                cleanup_failures,
                ..
            }) if cleanup_failures.len() == 1
        ));
        assert_eq!(fs::read(output.uasset()).unwrap(), b"evil");
        assert_eq!(fs::read(output.uexp()).unwrap(), b"body");
    }

    #[test]
    fn cleanup_detects_same_bytes_replacement_before_final_identity_check() {
        let temp = tempfile::tempdir().unwrap();
        let output = PackagePaths::from_uasset(temp.path().join("Edited.uasset")).unwrap();
        let displaced = temp.path().join("published-original.uasset");
        let staged_uasset =
            stage_component(temp.path(), PackageComponent::Uasset, b"head", 64).unwrap();
        let staged_uexp =
            stage_component(temp.path(), PackageComponent::Uexp, b"body", 64).unwrap();

        let result = publish_staged_pair_with(
            &output,
            staged_uasset,
            staged_uexp,
            b"head",
            b"body",
            small_limits(),
            persist_new,
            |published, limit| {
                if published.component == PackageComponent::Uasset {
                    return Err(PackageError::VerificationHash(published.path.clone()));
                }
                published.verify(limit)
            },
            |published| {
                if published.component == PackageComponent::Uasset {
                    published.remove_if_owned_with_before_final_check(|path| {
                        fs::rename(path, &displaced).unwrap();
                        fs::write(path, b"head").unwrap();
                    })
                } else {
                    published.remove_if_owned()
                }
            },
        );

        assert!(matches!(
            result,
            Err(PackageError::PublishCleanupFailed {
                cleanup_failures,
                ..
            }) if matches!(
                cleanup_failures.as_slice(),
                [CleanupFailure::OwnershipChanged { .. }]
            )
        ));
        assert_eq!(fs::read(output.uasset()).unwrap(), b"head");
        assert_eq!(fs::read(displaced).unwrap(), b"head");
        assert_eq!(fs::read(output.uexp()).unwrap(), b"body");
    }

    #[test]
    fn cleanup_preserves_non_ownership_verification_errors() {
        let temp = tempfile::tempdir().unwrap();
        let output = temp.path().join("Edited.uexp");
        let staged = stage_component(temp.path(), PackageComponent::Uexp, b"body", 64).unwrap();
        let handle = persist_new(staged, &output).unwrap();
        let published = PublishedComponent::new(PackageComponent::Uexp, &output, b"body", handle);

        let failure = published.cleanup_verification_failure(PackageError::Io {
            operation: "injected cleanup verification",
            path: output.clone(),
            source: io::Error::new(io::ErrorKind::PermissionDenied, "injected"),
        });
        assert!(matches!(
            failure,
            CleanupFailure::Verify { source, .. }
                if matches!(*source, PackageError::Io { .. })
        ));
        published.remove_if_owned().unwrap();
    }

    #[test]
    fn preflight_prevents_a_partial_pair_when_either_destination_exists() {
        let temp = tempfile::tempdir().unwrap();
        let output = temp.path().join("Edited.uasset");
        let output_uexp = output.with_extension("uexp");
        fs::write(&output_uexp, b"do not replace").unwrap();
        let carrier = PackageCarrier::from_bytes(vec![1, 2], vec![3, 4], small_limits()).unwrap();

        assert!(matches!(
            carrier.write_new(&output),
            Err(PackageError::DestinationExists(path))
                if path == fs::canonicalize(&output_uexp).unwrap()
        ));
        assert!(!output.exists());
        assert_eq!(fs::read(output_uexp).unwrap(), b"do not replace");
    }

    #[test]
    fn loaded_carrier_explicitly_rejects_in_place_output() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("Fixture.uasset");
        fs::write(&source, b"header").unwrap();
        fs::write(source.with_extension("uexp"), b"export").unwrap();
        let carrier = PackageCarrier::load(&source, small_limits()).unwrap();

        assert!(matches!(
            carrier.write_new(&source),
            Err(PackageError::InPlaceOutput(path)) if path == fs::canonicalize(&source).unwrap()
        ));
    }

    #[test]
    fn directories_are_not_accepted_as_components() {
        let temp = tempfile::tempdir().unwrap();
        let fake = temp.path().join("Folder.uasset");
        fs::create_dir(&fake).unwrap();
        assert!(matches!(
            PackageCarrier::load(&fake, small_limits()),
            Err(PackageError::NotRegularFile(path)) if path == fake
        ));
    }
}
