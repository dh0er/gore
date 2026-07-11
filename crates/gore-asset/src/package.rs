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
//! Filesystems do not offer one atomic rename for two sibling files, so there is
//! a very small interval between the two publishes; if the second publish fails,
//! the first is removed. Existing destinations are never overwritten.

use std::fmt;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PackageComponent {
    Uasset,
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

#[derive(Debug, Error)]
pub enum PackageError {
    #[error("expected a .{expected} path, got {path}")]
    InvalidExtension {
        expected: &'static str,
        path: PathBuf,
    },
    #[error("package path has no non-empty file stem: {0}")]
    MissingFileStem(PathBuf),
    #[error("symbolic-link package paths are refused: {0}")]
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
    #[error(
        "publishing {failed_destination} failed ({publish_error}); removing already-published {published} also failed: {rollback_error}"
    )]
    PublishRollbackFailed {
        published: PathBuf,
        failed_destination: PathBuf,
        publish_error: String,
        rollback_error: io::Error,
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
    /// Load and re-open both components. Re-opening verifies length and SHA-256
    /// so a file changed during the read is not silently accepted.
    pub fn load(
        uasset_path: impl AsRef<Path>,
        limits: PackageLimits,
    ) -> Result<Self, PackageError> {
        let requested = PackagePaths::from_uasset(uasset_path)?;
        let uasset_path = canonical_regular_path(requested.uasset())?;
        let uexp_path = canonical_regular_path(requested.uexp())?;

        // Check both advertised lengths before allocating either component, so
        // a small pair limit cannot be bypassed by a large first file.
        let advertised_uasset = inspect_component_length(
            &uasset_path,
            PackageComponent::Uasset,
            limits.max_uasset_bytes,
        )?;
        let advertised_uexp =
            inspect_component_length(&uexp_path, PackageComponent::Uexp, limits.max_uexp_bytes)?;
        validate_pair_sizes_u64(advertised_uasset, advertised_uexp, limits)?;

        let uasset = read_verified_component(
            &uasset_path,
            PackageComponent::Uasset,
            limits.max_uasset_bytes,
        )?;
        let uexp =
            read_verified_component(&uexp_path, PackageComponent::Uexp, limits.max_uexp_bytes)?;
        validate_pair_sizes(uasset.len(), uexp.len(), limits)?;

        Ok(Self {
            source: Some(PackagePaths {
                uasset: uasset_path,
                uexp: uexp_path,
            }),
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

        persist_new(staged_uasset, output.uasset())?;
        if let Err(publish_error) = persist_new(staged_uexp, output.uexp()) {
            return match fs::remove_file(output.uasset()) {
                Ok(()) => Err(publish_error),
                Err(rollback_error) => Err(PackageError::PublishRollbackFailed {
                    published: output.uasset().to_path_buf(),
                    failed_destination: output.uexp().to_path_buf(),
                    publish_error: publish_error.to_string(),
                    rollback_error,
                }),
            };
        }

        let uasset = verify_published(
            output.uasset(),
            PackageComponent::Uasset,
            &self.uasset,
            self.limits.max_uasset_bytes,
        )?;
        let uexp = verify_published(
            output.uexp(),
            PackageComponent::Uexp,
            &self.uexp,
            self.limits.max_uexp_bytes,
        )?;
        Ok(PackageWriteReceipt { uasset, uexp })
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

fn canonical_regular_path(path: &Path) -> Result<PathBuf, PackageError> {
    let link_metadata = fs::symlink_metadata(path).map_err(|source| PackageError::Io {
        operation: "inspect input",
        path: path.to_path_buf(),
        source,
    })?;
    if link_metadata.file_type().is_symlink() {
        return Err(PackageError::SymlinkPath(path.to_path_buf()));
    }
    if !link_metadata.is_file() {
        return Err(PackageError::NotRegularFile(path.to_path_buf()));
    }
    fs::canonicalize(path).map_err(|source| PackageError::Io {
        operation: "canonicalize input",
        path: path.to_path_buf(),
        source,
    })
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
    limit: u64,
) -> Result<Vec<u8>, PackageError> {
    let mut file = open_regular(path)?;
    let advertised = file_metadata_len(&file, path)?;
    check_component_limit(component, advertised, limit)?;
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
    let read_limit = limit.saturating_add(1);
    (&mut file)
        .take(read_limit)
        .read_to_end(&mut bytes)
        .map_err(|source| PackageError::Io {
            operation: "read input",
            path: path.to_path_buf(),
            source,
        })?;
    let actual = u64::try_from(bytes.len()).map_err(|_| PackageError::SizeOverflow)?;
    check_component_limit(component, actual, limit)?;
    if actual != advertised {
        return Err(PackageError::ConcurrentLengthChange {
            path: path.to_path_buf(),
            expected: advertised,
            actual,
        });
    }
    let expected_hash = sha256(&bytes);
    verify_path(path, component, actual, expected_hash, limit)?;
    Ok(bytes)
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

fn open_regular(path: &Path) -> Result<File, PackageError> {
    let link_metadata = fs::symlink_metadata(path).map_err(|source| PackageError::Io {
        operation: "inspect file",
        path: path.to_path_buf(),
        source,
    })?;
    if link_metadata.file_type().is_symlink() {
        return Err(PackageError::SymlinkPath(path.to_path_buf()));
    }
    if !link_metadata.is_file() {
        return Err(PackageError::NotRegularFile(path.to_path_buf()));
    }
    File::open(path).map_err(|source| PackageError::Io {
        operation: "open file",
        path: path.to_path_buf(),
        source,
    })
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

fn persist_new(temp: NamedTempFile, destination: &Path) -> Result<(), PackageError> {
    match temp.persist_noclobber(destination) {
        // The staged file was already flushed and synced before this atomic
        // rename. Avoid introducing a post-publication failure point here: a
        // returned error must never leave an unreported first component behind.
        Ok(file) => {
            drop(file);
            Ok(())
        }
        Err(error) => Err(PackageError::Io {
            operation: "publish new output",
            path: destination.to_path_buf(),
            source: error.error,
        }),
    }
}

fn verify_published(
    path: &Path,
    component: PackageComponent,
    expected: &[u8],
    limit: u64,
) -> Result<ComponentDigest, PackageError> {
    let length = u64::try_from(expected.len()).map_err(|_| PackageError::SizeOverflow)?;
    let expected_hash = sha256(expected);
    verify_path(path, component, length, expected_hash, limit)?;
    Ok(ComponentDigest {
        path: path.to_path_buf(),
        length,
        sha256: expected_hash,
    })
}

fn verify_path(
    path: &Path,
    component: PackageComponent,
    expected_length: u64,
    expected_hash: [u8; 32],
    limit: u64,
) -> Result<(), PackageError> {
    let mut file = open_regular(path)?;
    let metadata_length = file_metadata_len(&file, path)?;
    if metadata_length != expected_length {
        return Err(PackageError::VerificationLength {
            path: path.to_path_buf(),
            expected: expected_length,
            actual: metadata_length,
        });
    }
    check_component_limit(component, metadata_length, limit)?;

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
