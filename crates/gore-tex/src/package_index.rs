//! Bounded, metadata-only discovery of installed `/Game` package candidates.
//!
//! This module deliberately does not parse Zen package headers and never reads an
//! `ExportBundleData` payload. It budgets every physical entry exposed by
//! [`IoStoreTrait::chunks_all`], applies the composite store's first-winner ordering locally, and
//! uses the concrete winner's borrowed Directory Index path from
//! [`retoc::iostore::ChunkInfo::path_ref`]. A result is therefore a package *candidate* index, not
//! evidence about a package's class or contents.
//!
//! This API starts from an already-open store. A future path-based installed browser must reject
//! unsafe filenames, nested/reparse-point mountables, and ambiguous priority names *before*
//! calling `retoc::iostore::open`; the post-open priority check here is defense in depth and does
//! not make that filesystem preflight unnecessary.

use retoc::iostore::IoStoreTrait;
use retoc::{EIoChunkType, FIoChunkId, FIoContainerId, FPackageId};
use serde::Serialize;
use std::collections::{BTreeMap, HashSet};
use thiserror::Error;

const GAME_CONTENT_PREFIX: &str = "../../../G1R/Content/";
const GAME_CONTENT_ROOT: &str = "../../../G1R/Content";
const UASSET_SUFFIX: &str = ".uasset";
const MAX_SAFE_NUMERIC_PATCH_VERSION: u32 = u32::MAX - 2;

pub const MAX_CHILD_CONTAINERS_SCANNED: usize = 256;
pub const MAX_CONTAINER_PRIORITY_NAME_BYTES: usize = 512;
pub const MAX_AGGREGATE_CONTAINER_PRIORITY_NAME_BYTES: usize =
    MAX_CHILD_CONTAINERS_SCANNED * MAX_CONTAINER_PRIORITY_NAME_BYTES;
pub const MAX_PHYSICAL_CHUNKS_SCANNED: usize = 1_000_000;
pub const MAX_WINNING_EXPORT_BUNDLES: usize = 250_000;
pub const MAX_INSTALLED_PACKAGE_CANDIDATES: usize = 250_000;
pub const MAX_DIRECTORY_INDEX_PATH_BYTES: usize = 16 * 1024;
pub const MAX_AGGREGATE_DIRECTORY_INDEX_PATH_BYTES: usize = 32 * 1024 * 1024;
pub const MAX_GAME_PACKAGE_PATH_BYTES: usize = 512;
pub const MAX_GAME_PACKAGE_SEGMENTS: usize = 32;

/// Tightenable limits for one metadata-only package-index pass.
///
/// Callers may lower these limits, but cannot raise them above the hard bounds of this module.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PackageIndexLimits {
    max_child_containers_scanned: usize,
    max_container_priority_name_bytes: usize,
    max_aggregate_container_priority_name_bytes: usize,
    max_physical_chunks_scanned: usize,
    max_winning_export_bundles: usize,
    max_candidates: usize,
    max_directory_path_bytes: usize,
    max_aggregate_directory_path_bytes: usize,
}

impl PackageIndexLimits {
    pub fn tightened(
        max_physical_chunks_scanned: usize,
        max_winning_export_bundles: usize,
        max_candidates: usize,
        max_directory_path_bytes: usize,
        max_aggregate_directory_path_bytes: usize,
    ) -> Result<Self, PackageIndexError> {
        Self {
            max_physical_chunks_scanned,
            max_winning_export_bundles,
            max_candidates,
            max_directory_path_bytes,
            max_aggregate_directory_path_bytes,
            ..Self::default()
        }
        .validate()
    }

    pub fn with_container_priority_limits(
        mut self,
        max_child_containers_scanned: usize,
        max_container_priority_name_bytes: usize,
        max_aggregate_container_priority_name_bytes: usize,
    ) -> Result<Self, PackageIndexError> {
        self.max_child_containers_scanned = max_child_containers_scanned;
        self.max_container_priority_name_bytes = max_container_priority_name_bytes;
        self.max_aggregate_container_priority_name_bytes =
            max_aggregate_container_priority_name_bytes;
        self.validate()
    }

    fn validate(self) -> Result<Self, PackageIndexError> {
        validate_limit(
            "child container scan count",
            self.max_child_containers_scanned,
            MAX_CHILD_CONTAINERS_SCANNED,
        )?;
        validate_limit(
            "container priority name bytes",
            self.max_container_priority_name_bytes,
            MAX_CONTAINER_PRIORITY_NAME_BYTES,
        )?;
        validate_limit(
            "aggregate container priority name bytes",
            self.max_aggregate_container_priority_name_bytes,
            MAX_AGGREGATE_CONTAINER_PRIORITY_NAME_BYTES,
        )?;
        validate_limit(
            "physical chunk scan count",
            self.max_physical_chunks_scanned,
            MAX_PHYSICAL_CHUNKS_SCANNED,
        )?;
        validate_limit(
            "winning ExportBundle count",
            self.max_winning_export_bundles,
            MAX_WINNING_EXPORT_BUNDLES,
        )?;
        validate_limit(
            "candidate count",
            self.max_candidates,
            MAX_INSTALLED_PACKAGE_CANDIDATES,
        )?;
        validate_limit(
            "Directory Index path bytes",
            self.max_directory_path_bytes,
            MAX_DIRECTORY_INDEX_PATH_BYTES,
        )?;
        validate_limit(
            "aggregate Directory Index path bytes",
            self.max_aggregate_directory_path_bytes,
            MAX_AGGREGATE_DIRECTORY_INDEX_PATH_BYTES,
        )?;
        if self.max_aggregate_directory_path_bytes < self.max_directory_path_bytes {
            return Err(PackageIndexError::InvalidLimits(
                "aggregate Directory Index path limit is below the per-path limit",
            ));
        }
        if self.max_aggregate_container_priority_name_bytes < self.max_container_priority_name_bytes
        {
            return Err(PackageIndexError::InvalidLimits(
                "aggregate container priority name limit is below the per-name limit",
            ));
        }
        Ok(self)
    }
}

impl Default for PackageIndexLimits {
    fn default() -> Self {
        Self {
            max_child_containers_scanned: MAX_CHILD_CONTAINERS_SCANNED,
            max_container_priority_name_bytes: MAX_CONTAINER_PRIORITY_NAME_BYTES,
            max_aggregate_container_priority_name_bytes:
                MAX_AGGREGATE_CONTAINER_PRIORITY_NAME_BYTES,
            max_physical_chunks_scanned: MAX_PHYSICAL_CHUNKS_SCANNED,
            max_winning_export_bundles: MAX_WINNING_EXPORT_BUNDLES,
            max_candidates: MAX_INSTALLED_PACKAGE_CANDIDATES,
            max_directory_path_bytes: MAX_DIRECTORY_INDEX_PATH_BYTES,
            max_aggregate_directory_path_bytes: MAX_AGGREGATE_DIRECTORY_INDEX_PATH_BYTES,
        }
    }
}

fn validate_limit(
    label: &'static str,
    value: usize,
    hard_maximum: usize,
) -> Result<(), PackageIndexError> {
    if value == 0 || value > hard_maximum {
        return Err(PackageIndexError::InvalidLimit {
            label,
            value,
            hard_maximum,
        });
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PackageIndexStatus {
    CompleteIndex,
    PartialIndex,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PackageIndexPartialReason {
    NoncanonicalExportBundleChunkId,
    MissingDirectoryIndexPath,
    NoncanonicalGameDirectoryIndexPath,
    PackageIdMismatch,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PackageIndexPartialReasonCount {
    pub reason: PackageIndexPartialReason,
    pub count: u64,
}

/// One path-only candidate. `package_id_hex` is display/provenance metadata, not content
/// authority; exact extraction must derive and revalidate the package again.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InstalledPackageCandidate {
    pub target_path: String,
    pub package_id_hex: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InstalledPackageIndex {
    pub status: PackageIndexStatus,
    pub physical_chunk_count: u64,
    pub winning_export_bundle_count: u64,
    pub directory_indexed_export_bundle_count: u64,
    pub out_of_scope_export_bundle_count: u64,
    pub candidates: Vec<InstalledPackageCandidate>,
    pub partial_reasons: Vec<PackageIndexPartialReasonCount>,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum PackageIndexError {
    #[error("invalid package-index limits: {0}")]
    InvalidLimits(&'static str),
    #[error("invalid package-index {label} limit {value}; expected 1..={hard_maximum}")]
    InvalidLimit {
        label: &'static str,
        value: usize,
        hard_maximum: usize,
    },
    #[error("IoStore does not expose one container file version")]
    ContainerVersionUnavailable,
    #[error(
        "containers {first:?} and {second:?} have the same package priority; winner order is ambiguous"
    )]
    AmbiguousContainerPriority { first: String, second: String },
    #[error("child container count {actual} exceeds the package-index limit {limit}")]
    ChildContainerLimit { actual: u64, limit: usize },
    #[error("container priority name has {actual} bytes; package-index limit is {limit}")]
    ContainerPriorityNameLimit { actual: usize, limit: usize },
    #[error(
        "aggregate container priority names have {actual} bytes; package-index limit is {limit}"
    )]
    AggregateContainerPriorityNameLimit { actual: usize, limit: usize },
    #[error(
        "container {container_name:?} uses numeric patch version {version}; maximum safe version is {maximum}"
    )]
    ContainerPriorityVersionOverflow {
        container_name: String,
        version: u32,
        maximum: u32,
    },
    #[error("physical chunk scan count {actual} exceeds the package-index limit {limit}")]
    ChunkScanLimit { actual: u64, limit: usize },
    #[error("winning ExportBundle count {actual} exceeds the package-index limit {limit}")]
    WinningExportBundleLimit { actual: u64, limit: usize },
    #[error("Directory Index path has {actual} bytes; package-index limit is {limit}")]
    DirectoryPathLimit { actual: usize, limit: usize },
    #[error("aggregate Directory Index paths have {actual} bytes; package-index limit is {limit}")]
    AggregateDirectoryPathLimit { actual: usize, limit: usize },
    #[error("package candidate count {actual} exceeds the package-index limit {limit}")]
    CandidateLimit { actual: u64, limit: usize },
    #[error("package-index counter overflowed")]
    CounterOverflow,
}

fn materialize_directory_path_after_budget(
    directory_path_bytes: usize,
    aggregate_directory_path_bytes: &mut usize,
    limits: PackageIndexLimits,
    materialize: impl FnOnce() -> String,
) -> Result<String, PackageIndexError> {
    if directory_path_bytes > limits.max_directory_path_bytes {
        return Err(PackageIndexError::DirectoryPathLimit {
            actual: directory_path_bytes,
            limit: limits.max_directory_path_bytes,
        });
    }
    let prospective_aggregate_directory_path_bytes = aggregate_directory_path_bytes
        .checked_add(directory_path_bytes)
        .ok_or(PackageIndexError::CounterOverflow)?;
    if prospective_aggregate_directory_path_bytes > limits.max_aggregate_directory_path_bytes {
        return Err(PackageIndexError::AggregateDirectoryPathLimit {
            actual: prospective_aggregate_directory_path_bytes,
            limit: limits.max_aggregate_directory_path_bytes,
        });
    }

    // Keeping materialization behind the budget checks makes the allocation order testable and
    // prevents both hard and caller-tightened limits from being bypassed by an owned path clone.
    let directory_path = materialize();
    debug_assert_eq!(directory_path.len(), directory_path_bytes);
    *aggregate_directory_path_bytes = prospective_aggregate_directory_path_bytes;
    Ok(directory_path)
}

/// Build a bounded candidate index from winning IoStore chunk metadata.
pub fn index_winning_game_packages(
    store: &dyn IoStoreTrait,
) -> Result<InstalledPackageIndex, PackageIndexError> {
    index_winning_game_packages_with_limits(store, PackageIndexLimits::default())
}

/// As [`index_winning_game_packages`], with caller-tightened hard limits.
pub fn index_winning_game_packages_with_limits(
    store: &dyn IoStoreTrait,
    limits: PackageIndexLimits,
) -> Result<InstalledPackageIndex, PackageIndexError> {
    // Validate even values obtained through future construction paths before iterating.
    let limits = limits.validate()?;
    validate_unambiguous_container_priorities(store, limits)?;
    let store_version = store
        .container_file_version()
        .ok_or(PackageIndexError::ContainerVersionUnavailable)?;

    let mut physical_chunk_count = 0u64;
    let mut winning_export_bundle_count = 0u64;
    let mut directory_indexed_export_bundle_count = 0u64;
    let mut out_of_scope_export_bundle_count = 0u64;
    let mut aggregate_directory_path_bytes = 0usize;
    let mut candidates = Vec::new();
    let mut winning_chunk_ids =
        HashSet::<FIoChunkId>::with_capacity(limits.max_physical_chunks_scanned.min(4096));
    let mut noncanonical_chunk_id_count = 0u64;
    let mut missing_path_count = 0u64;
    let mut noncanonical_path_count = 0u64;
    let mut package_id_mismatch_count = 0u64;

    // Retoc's composite `chunks_all()` order is child priority followed by each child's TOC order.
    // Count every physical entry before deduplication or type filtering, then retain the first
    // occurrence of each id locally. This both preserves Retoc winner semantics and prevents a
    // large population of losing duplicates/non-package chunks from bypassing the scan budget.
    // Nothing in this loop calls ChunkInfo::read or IoStoreTrait::read.
    for chunk in store.chunks_all() {
        physical_chunk_count = physical_chunk_count
            .checked_add(1)
            .ok_or(PackageIndexError::CounterOverflow)?;
        if physical_chunk_count > limits.max_physical_chunks_scanned as u64 {
            return Err(PackageIndexError::ChunkScanLimit {
                actual: physical_chunk_count,
                limit: limits.max_physical_chunks_scanned,
            });
        }
        let chunk_id = chunk.id();
        if !winning_chunk_ids.insert(chunk_id) {
            continue;
        }
        if chunk_id.get_chunk_type() != EIoChunkType::ExportBundleData {
            continue;
        }
        winning_export_bundle_count = winning_export_bundle_count
            .checked_add(1)
            .ok_or(PackageIndexError::CounterOverflow)?;
        if winning_export_bundle_count > limits.max_winning_export_bundles as u64 {
            return Err(PackageIndexError::WinningExportBundleLimit {
                actual: winning_export_bundle_count,
                limit: limits.max_winning_export_bundles,
            });
        }

        let package_id = chunk_id.get_package_id();
        let canonical_chunk_id =
            FIoChunkId::from_package_id(package_id, 0, EIoChunkType::ExportBundleData)
                .with_version(store_version);
        if chunk_id != canonical_chunk_id {
            noncanonical_chunk_id_count = noncanonical_chunk_id_count
                .checked_add(1)
                .ok_or(PackageIndexError::CounterOverflow)?;
            continue;
        }

        let Some(directory_path_ref) = chunk.path_ref() else {
            missing_path_count = missing_path_count
                .checked_add(1)
                .ok_or(PackageIndexError::CounterOverflow)?;
            continue;
        };
        directory_indexed_export_bundle_count = directory_indexed_export_bundle_count
            .checked_add(1)
            .ok_or(PackageIndexError::CounterOverflow)?;
        let directory_path_bytes = directory_path_ref
            .joined_byte_len()
            .ok_or(PackageIndexError::CounterOverflow)?;
        let directory_path = materialize_directory_path_after_budget(
            directory_path_bytes,
            &mut aggregate_directory_path_bytes,
            limits,
            || directory_path_ref.materialize(),
        )?;

        let target_path = match normalize_directory_index_path(&directory_path) {
            PathDisposition::Candidate(target_path) => target_path,
            PathDisposition::OutOfScope => {
                out_of_scope_export_bundle_count = out_of_scope_export_bundle_count
                    .checked_add(1)
                    .ok_or(PackageIndexError::CounterOverflow)?;
                continue;
            }
            PathDisposition::NoncanonicalGamePath => {
                noncanonical_path_count = noncanonical_path_count
                    .checked_add(1)
                    .ok_or(PackageIndexError::CounterOverflow)?;
                continue;
            }
        };

        let expected_package_id = package_id_from_target_path(&target_path);
        if package_id != expected_package_id {
            package_id_mismatch_count = package_id_mismatch_count
                .checked_add(1)
                .ok_or(PackageIndexError::CounterOverflow)?;
            continue;
        }
        let prospective_count = u64::try_from(candidates.len())
            .map_err(|_| PackageIndexError::CounterOverflow)?
            .checked_add(1)
            .ok_or(PackageIndexError::CounterOverflow)?;
        if prospective_count > limits.max_candidates as u64 {
            return Err(PackageIndexError::CandidateLimit {
                actual: prospective_count,
                limit: limits.max_candidates,
            });
        }
        candidates.push(InstalledPackageCandidate {
            target_path,
            package_id_hex: format!("{:016x}", expected_package_id.0),
        });
    }

    candidates.sort();
    candidates.dedup();

    let partial_reasons = [
        (
            PackageIndexPartialReason::NoncanonicalExportBundleChunkId,
            noncanonical_chunk_id_count,
        ),
        (
            PackageIndexPartialReason::MissingDirectoryIndexPath,
            missing_path_count,
        ),
        (
            PackageIndexPartialReason::NoncanonicalGameDirectoryIndexPath,
            noncanonical_path_count,
        ),
        (
            PackageIndexPartialReason::PackageIdMismatch,
            package_id_mismatch_count,
        ),
    ]
    .into_iter()
    .filter(|(_, count)| *count != 0)
    .map(|(reason, count)| PackageIndexPartialReasonCount { reason, count })
    .collect::<Vec<_>>();
    let status = if partial_reasons.is_empty() {
        PackageIndexStatus::CompleteIndex
    } else {
        PackageIndexStatus::PartialIndex
    };

    Ok(InstalledPackageIndex {
        status,
        physical_chunk_count,
        winning_export_bundle_count,
        directory_indexed_export_bundle_count,
        out_of_scope_export_bundle_count,
        candidates,
        partial_reasons,
    })
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct ContainerPriorityKey {
    is_global: bool,
    chunk_version: u64,
    base_name: String,
}

fn container_priority_key(full_name: &str) -> Result<ContainerPriorityKey, PackageIndexError> {
    let mut base_name = full_name;
    let mut chunk_version = 0u64;
    if let Some(without_patch_suffix) = base_name.strip_suffix("_P") {
        base_name = without_patch_suffix;
        chunk_version = 1;
        if let Some((candidate_base, version)) = base_name.rsplit_once('_') {
            if let Ok(version) = version.parse::<u32>() {
                if version > MAX_SAFE_NUMERIC_PATCH_VERSION {
                    return Err(PackageIndexError::ContainerPriorityVersionOverflow {
                        container_name: full_name.to_owned(),
                        version,
                        maximum: MAX_SAFE_NUMERIC_PATCH_VERSION,
                    });
                }
                base_name = candidate_base;
                // Retoc stores this as `u32 + 2`. The explicit bound above prevents both its
                // debug-build overflow panic and release-build wraparound priority collisions.
                chunk_version = u64::from(version) + 2;
            }
        }
    }
    Ok(ContainerPriorityKey {
        is_global: full_name == "global",
        chunk_version,
        base_name: base_name.to_owned(),
    })
}

fn validate_unambiguous_container_priorities(
    store: &dyn IoStoreTrait,
    limits: PackageIndexLimits,
) -> Result<(), PackageIndexError> {
    let mut child_count = 0u64;
    let mut aggregate_name_bytes = 0usize;
    let mut bounded_names = Vec::with_capacity(limits.max_child_containers_scanned.min(32));
    for child in store.child_containers() {
        child_count = child_count
            .checked_add(1)
            .ok_or(PackageIndexError::CounterOverflow)?;
        if child_count > limits.max_child_containers_scanned as u64 {
            return Err(PackageIndexError::ChildContainerLimit {
                actual: child_count,
                limit: limits.max_child_containers_scanned,
            });
        }
        let name = child.container_name();
        if name.len() > limits.max_container_priority_name_bytes {
            return Err(PackageIndexError::ContainerPriorityNameLimit {
                actual: name.len(),
                limit: limits.max_container_priority_name_bytes,
            });
        }
        aggregate_name_bytes = aggregate_name_bytes
            .checked_add(name.len())
            .ok_or(PackageIndexError::CounterOverflow)?;
        if aggregate_name_bytes > limits.max_aggregate_container_priority_name_bytes {
            return Err(PackageIndexError::AggregateContainerPriorityNameLimit {
                actual: aggregate_name_bytes,
                limit: limits.max_aggregate_container_priority_name_bytes,
            });
        }
        // Both per-name and aggregate byte budgets have passed before this allocation.
        bounded_names.push(name.to_owned());
    }

    // Retoc may preserve filesystem enumeration order when priority keys compare equal. Sort the
    // bounded owned names first so overflow and collision errors remain stable across that order.
    bounded_names.sort();
    let mut names_by_priority = BTreeMap::<ContainerPriorityKey, Vec<String>>::new();
    for name in bounded_names {
        names_by_priority
            .entry(container_priority_key(&name)?)
            .or_default()
            .push(name);
    }
    for names in names_by_priority.values_mut() {
        if names.len() < 2 {
            continue;
        }
        names.sort();
        return Err(PackageIndexError::AmbiguousContainerPriority {
            first: names[0].clone(),
            second: names[1].clone(),
        });
    }
    Ok(())
}

enum PathDisposition {
    Candidate(String),
    OutOfScope,
    NoncanonicalGamePath,
}

fn normalize_directory_index_path(path: &str) -> PathDisposition {
    let Some(relative) = path.strip_prefix(GAME_CONTENT_PREFIX) else {
        return if resembles_game_content_path(path) {
            PathDisposition::NoncanonicalGamePath
        } else {
            PathDisposition::OutOfScope
        };
    };
    let Some(package_relative) = relative.strip_suffix(UASSET_SUFFIX) else {
        return PathDisposition::NoncanonicalGamePath;
    };
    let target_path = format!("/Game/{package_relative}");
    if !is_canonical_game_package_path(&target_path) {
        return PathDisposition::NoncanonicalGamePath;
    }
    PathDisposition::Candidate(target_path)
}

fn resembles_game_content_path(path: &str) -> bool {
    if path.len() < GAME_CONTENT_ROOT.len() {
        return false;
    }
    let (root, remainder) = path.split_at(GAME_CONTENT_ROOT.len());
    root.eq_ignore_ascii_case(GAME_CONTENT_ROOT)
        && matches!(
            remainder.as_bytes().first(),
            None | Some(b'/') | Some(b'\\')
        )
}

fn is_canonical_game_package_path(path: &str) -> bool {
    if path.len() > MAX_GAME_PACKAGE_PATH_BYTES
        || !path.starts_with("/Game/")
        || path.contains('\\')
        || path.ends_with('/')
    {
        return false;
    }
    let mut count = 0usize;
    for segment in path["/Game/".len()..].split('/') {
        count += 1;
        if segment.is_empty()
            || windows_reserved_name(segment)
            || !segment
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
        {
            return false;
        }
    }
    (1..=MAX_GAME_PACKAGE_SEGMENTS).contains(&count)
}

fn windows_reserved_name(name: &str) -> bool {
    let upper = name.to_ascii_uppercase();
    matches!(upper.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || (upper.len() == 4
            && (upper.starts_with("COM") || upper.starts_with("LPT"))
            && matches!(upper.as_bytes()[3], b'1'..=b'9'))
}

fn package_id_from_target_path(target_path: &str) -> FPackageId {
    FPackageId(FIoContainerId::from_name(target_path).0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use retoc::iostore;
    use retoc::iostore_writer::IoStoreWriter;
    use retoc::version::EngineVersion;
    use retoc::{Config, FIoChunkId, UEPath, UEPathBuf};
    use std::cell::Cell;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::Arc;

    static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

    struct FixtureChunk<'a> {
        target_for_id: &'a str,
        chunk_type: EIoChunkType,
        directory_path: Option<&'a str>,
        payload: &'a [u8],
    }

    fn temp_dir(label: &str) -> PathBuf {
        let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "gore-tex-package-index-{label}-{}-{sequence}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        path
    }

    fn write_fixture_container<'a>(
        path: &Path,
        chunks: impl IntoIterator<Item = FixtureChunk<'a>>,
    ) {
        let version = EngineVersion::UE5_4;
        let mut writer = IoStoreWriter::new(
            path,
            version.toc_version(),
            None,
            UEPathBuf::from("../../../"),
        )
        .unwrap();
        for chunk in chunks {
            let id = FIoChunkId::from_package_id(
                package_id_from_target_path(chunk.target_for_id),
                0,
                chunk.chunk_type,
            );
            writer
                .write_chunk(id, chunk.directory_path.map(UEPath::new), chunk.payload)
                .unwrap();
        }
        writer.finalize().unwrap();
    }

    fn write_export_id_fixture<'a>(
        path: &Path,
        chunks: impl IntoIterator<Item = (&'a str, u16, u8, &'a str)>,
    ) {
        let version = EngineVersion::UE5_4;
        let toc_version = version.toc_version();
        let mut writer =
            IoStoreWriter::new(path, toc_version, None, UEPathBuf::from("../../../")).unwrap();
        for (target, chunk_index, reserved_suffix, directory_path) in chunks {
            let canonical = FIoChunkId::from_package_id(
                package_id_from_target_path(target),
                chunk_index,
                EIoChunkType::ExportBundleData,
            )
            .with_version(toc_version);
            let mut raw = canonical.get_raw();
            raw.id[10] = reserved_suffix;
            writer
                .write_chunk_raw(raw, Some(UEPath::new(directory_path)), b"unread export")
                .unwrap();
        }
        writer.finalize().unwrap();
    }

    fn open(path: &Path) -> Box<dyn IoStoreTrait> {
        iostore::open(path, Arc::new(Config::default())).unwrap()
    }

    fn limits_with_container_budgets(
        max_children: usize,
        max_name_bytes: usize,
        max_aggregate_name_bytes: usize,
    ) -> PackageIndexLimits {
        PackageIndexLimits::default()
            .with_container_priority_limits(max_children, max_name_bytes, max_aggregate_name_bytes)
            .unwrap()
    }

    #[test]
    fn exact_game_paths_normalize_and_sort_lexicographically() {
        let root = temp_dir("sort");
        let utoc = root.join("fixture.utoc");
        write_fixture_container(
            &utoc,
            [
                FixtureChunk {
                    target_for_id: "/Game/Zed/DA_Zed",
                    chunk_type: EIoChunkType::ExportBundleData,
                    directory_path: Some("../../../G1R/Content/Zed/DA_Zed.uasset"),
                    payload: b"zed payload must remain unread",
                },
                FixtureChunk {
                    target_for_id: "/Game/Alpha/DA_Alpha",
                    chunk_type: EIoChunkType::ExportBundleData,
                    directory_path: Some("../../../G1R/Content/Alpha/DA_Alpha.uasset"),
                    payload: b"alpha payload must remain unread",
                },
                FixtureChunk {
                    target_for_id: "/Game/Ignored/DA_Bulk",
                    chunk_type: EIoChunkType::BulkData,
                    directory_path: Some("../../../G1R/Content/Ignored/DA_Bulk.ubulk"),
                    payload: b"ignored bulk",
                },
            ],
        );

        let index = index_winning_game_packages(open(&utoc).as_ref()).unwrap();
        assert_eq!(index.status, PackageIndexStatus::CompleteIndex);
        assert_eq!(index.winning_export_bundle_count, 2);
        assert_eq!(
            index
                .candidates
                .iter()
                .map(|candidate| candidate.target_path.as_str())
                .collect::<Vec<_>>(),
            ["/Game/Alpha/DA_Alpha", "/Game/Zed/DA_Zed"]
        );
        assert!(index.partial_reasons.is_empty());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn composite_uses_only_winning_chunk_and_its_directory_index_path() {
        let root = temp_dir("winner");
        let target_lower = "/Game/characters/DA_Hero";
        let target_winner = "/Game/Characters/DA_Hero";
        write_fixture_container(
            &root.join("G1R-Windows.utoc"),
            [FixtureChunk {
                target_for_id: target_lower,
                chunk_type: EIoChunkType::ExportBundleData,
                directory_path: Some("../../../G1R/Content/characters/DA_Hero.uasset"),
                payload: b"base",
            }],
        );
        write_fixture_container(
            &root.join("G1R-Windows_0_P.utoc"),
            [FixtureChunk {
                target_for_id: target_winner,
                chunk_type: EIoChunkType::ExportBundleData,
                directory_path: Some("../../../G1R/Content/Characters/DA_Hero.uasset"),
                payload: b"patch winner",
            }],
        );

        let index = index_winning_game_packages(open(&root).as_ref()).unwrap();
        assert_eq!(index.winning_export_bundle_count, 1);
        assert_eq!(index.candidates.len(), 1);
        assert_eq!(index.candidates[0].target_path, target_winner);
        assert_eq!(index.status, PackageIndexStatus::CompleteIndex);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn missing_noncanonical_and_mismatched_paths_report_stable_partial_reasons() {
        let root = temp_dir("partial");
        let utoc = root.join("fixture.utoc");
        write_fixture_container(
            &utoc,
            [
                FixtureChunk {
                    target_for_id: "/Game/Missing/DA_Missing",
                    chunk_type: EIoChunkType::ExportBundleData,
                    directory_path: None,
                    payload: b"missing path",
                },
                FixtureChunk {
                    target_for_id: "/Game/Bad_Name/DA_Bad",
                    chunk_type: EIoChunkType::ExportBundleData,
                    directory_path: Some("../../../G1R/Content/Bad-Name/DA_Bad.uasset"),
                    payload: b"bad path",
                },
                FixtureChunk {
                    target_for_id: "/Game/Other/DA_Other",
                    chunk_type: EIoChunkType::ExportBundleData,
                    directory_path: Some("../../../G1R/Content/Expected/DA_Expected.uasset"),
                    payload: b"mismatched id",
                },
                FixtureChunk {
                    target_for_id: "/Engine/EngineMaterials/DefaultMaterial",
                    chunk_type: EIoChunkType::ExportBundleData,
                    directory_path: Some(
                        "../../../Engine/Content/EngineMaterials/DefaultMaterial.uasset",
                    ),
                    payload: b"known out of scope",
                },
            ],
        );

        let index = index_winning_game_packages(open(&utoc).as_ref()).unwrap();
        assert_eq!(index.status, PackageIndexStatus::PartialIndex);
        assert_eq!(index.winning_export_bundle_count, 4);
        assert_eq!(index.directory_indexed_export_bundle_count, 3);
        assert_eq!(index.out_of_scope_export_bundle_count, 1);
        assert!(index.candidates.is_empty());
        assert_eq!(
            index.partial_reasons,
            [
                PackageIndexPartialReasonCount {
                    reason: PackageIndexPartialReason::MissingDirectoryIndexPath,
                    count: 1,
                },
                PackageIndexPartialReasonCount {
                    reason: PackageIndexPartialReason::NoncanonicalGameDirectoryIndexPath,
                    count: 1,
                },
                PackageIndexPartialReasonCount {
                    reason: PackageIndexPartialReason::PackageIdMismatch,
                    count: 1,
                },
            ]
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn metadata_index_succeeds_when_export_payload_is_unreadable() {
        let root = temp_dir("no-read");
        let utoc = root.join("fixture.utoc");
        let target = "/Game/Data/DA_Unread";
        write_fixture_container(
            &utoc,
            [FixtureChunk {
                target_for_id: target,
                chunk_type: EIoChunkType::ExportBundleData,
                directory_path: Some("../../../G1R/Content/Data/DA_Unread.uasset"),
                payload: b"tripwire payload",
            }],
        );
        let store = open(&utoc);
        let hidden_ucas = root.join("payload-hidden.ucas");
        std::fs::rename(utoc.with_extension("ucas"), &hidden_ucas).unwrap();

        let index = index_winning_game_packages(store.as_ref()).unwrap();
        assert_eq!(index.candidates[0].target_path, target);
        let id = FIoChunkId::from_package_id(
            package_id_from_target_path(target),
            0,
            EIoChunkType::ExportBundleData,
        );
        assert!(
            store.read(id).is_err(),
            "the renamed UCAS must make any hidden payload read fail"
        );
        drop(store);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn tightened_path_limits_use_exact_joined_length_without_payload_reads() {
        let root = temp_dir("path-budgets");
        let utoc = root.join("fixture.utoc");
        let path_a = "../../../G1R/Content/A/DA_A.uasset";
        let path_b = "../../../G1R/Content/B/DA_B.uasset";
        assert_eq!(path_a.len(), path_b.len());
        write_fixture_container(
            &utoc,
            [
                FixtureChunk {
                    target_for_id: "/Game/A/DA_A",
                    chunk_type: EIoChunkType::ExportBundleData,
                    directory_path: Some(path_a),
                    payload: b"unread a",
                },
                FixtureChunk {
                    target_for_id: "/Game/B/DA_B",
                    chunk_type: EIoChunkType::ExportBundleData,
                    directory_path: Some(path_b),
                    payload: b"unread b",
                },
            ],
        );
        let store = open(&utoc);
        let hidden_ucas = root.join("payload-hidden.ucas");
        std::fs::rename(utoc.with_extension("ucas"), &hidden_ucas).unwrap();

        let per_path_limits = PackageIndexLimits::tightened(
            MAX_PHYSICAL_CHUNKS_SCANNED,
            MAX_WINNING_EXPORT_BUNDLES,
            MAX_INSTALLED_PACKAGE_CANDIDATES,
            path_a.len() - 1,
            MAX_AGGREGATE_DIRECTORY_INDEX_PATH_BYTES,
        )
        .unwrap();
        assert_eq!(
            index_winning_game_packages_with_limits(store.as_ref(), per_path_limits),
            Err(PackageIndexError::DirectoryPathLimit {
                actual: path_a.len(),
                limit: path_a.len() - 1,
            })
        );

        let aggregate_limit = path_a.len() + path_b.len() - 1;
        let aggregate_limits = PackageIndexLimits::tightened(
            MAX_PHYSICAL_CHUNKS_SCANNED,
            MAX_WINNING_EXPORT_BUNDLES,
            MAX_INSTALLED_PACKAGE_CANDIDATES,
            path_a.len(),
            aggregate_limit,
        )
        .unwrap();
        assert_eq!(
            index_winning_game_packages_with_limits(store.as_ref(), aggregate_limits),
            Err(PackageIndexError::AggregateDirectoryPathLimit {
                actual: path_a.len() + path_b.len(),
                limit: aggregate_limit,
            })
        );

        drop(store);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn hard_path_cap_rejects_oversized_joined_path_without_payload_reads() {
        let root = temp_dir("hard-path-budget");
        let utoc = root.join("fixture.utoc");
        let long_segment = "A".repeat(1024);
        let nested_path = (0..17)
            .map(|_| long_segment.as_str())
            .collect::<Vec<_>>()
            .join("/");
        let oversized_path = format!("../../../G1R/Content/{nested_path}/DA_Long.uasset");
        assert!(oversized_path.len() > MAX_DIRECTORY_INDEX_PATH_BYTES);
        write_fixture_container(
            &utoc,
            [FixtureChunk {
                target_for_id: "/Game/Long/DA_Long",
                chunk_type: EIoChunkType::ExportBundleData,
                directory_path: Some(&oversized_path),
                payload: b"unread oversized-path payload",
            }],
        );
        let store = open(&utoc);
        let hidden_ucas = root.join("payload-hidden.ucas");
        std::fs::rename(utoc.with_extension("ucas"), &hidden_ucas).unwrap();

        assert_eq!(
            index_winning_game_packages(store.as_ref()),
            Err(PackageIndexError::DirectoryPathLimit {
                actual: oversized_path.len(),
                limit: MAX_DIRECTORY_INDEX_PATH_BYTES,
            })
        );

        drop(store);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn rejected_path_budget_does_not_invoke_materializer() {
        let calls = Cell::new(0usize);
        let limits = PackageIndexLimits::tightened(1, 1, 1, 8, 8).unwrap();
        let mut aggregate = 0usize;
        let result = materialize_directory_path_after_budget(9, &mut aggregate, limits, || {
            calls.set(calls.get() + 1);
            "allocated".to_owned()
        });
        assert_eq!(
            result,
            Err(PackageIndexError::DirectoryPathLimit {
                actual: 9,
                limit: 8,
            })
        );
        assert_eq!(calls.get(), 0);
        assert_eq!(aggregate, 0);

        let limits = PackageIndexLimits::tightened(1, 1, 1, 8, 10).unwrap();
        aggregate = 3;
        let result = materialize_directory_path_after_budget(8, &mut aggregate, limits, || {
            calls.set(calls.get() + 1);
            "allocated".to_owned()
        });
        assert_eq!(
            result,
            Err(PackageIndexError::AggregateDirectoryPathLimit {
                actual: 11,
                limit: 10,
            })
        );
        assert_eq!(calls.get(), 0);
        assert_eq!(aggregate, 3);
    }

    #[test]
    fn all_physical_chunks_are_budgeted_before_winner_and_type_filters() {
        let root = temp_dir("physical-budget");
        let bulk_one = "/Game/Bulk/DA_One";
        let bulk_two = "/Game/Bulk/DA_Two";
        write_fixture_container(
            &root.join("scan_0_P.utoc"),
            [
                FixtureChunk {
                    target_for_id: bulk_one,
                    chunk_type: EIoChunkType::BulkData,
                    directory_path: None,
                    payload: b"winning bulk one",
                },
                FixtureChunk {
                    target_for_id: bulk_two,
                    chunk_type: EIoChunkType::BulkData,
                    directory_path: None,
                    payload: b"winning bulk two",
                },
            ],
        );
        write_fixture_container(
            &root.join("scan.utoc"),
            [
                FixtureChunk {
                    target_for_id: bulk_one,
                    chunk_type: EIoChunkType::BulkData,
                    directory_path: None,
                    payload: b"losing duplicate one",
                },
                FixtureChunk {
                    target_for_id: bulk_two,
                    chunk_type: EIoChunkType::BulkData,
                    directory_path: None,
                    payload: b"losing duplicate two",
                },
                FixtureChunk {
                    target_for_id: "/Game/Data/DA_AfterBulk",
                    chunk_type: EIoChunkType::ExportBundleData,
                    directory_path: Some("../../../G1R/Content/Data/DA_AfterBulk.uasset"),
                    payload: b"export after physical noise",
                },
            ],
        );
        let store = open(&root);
        let stopped = PackageIndexLimits::tightened(
            4,
            MAX_WINNING_EXPORT_BUNDLES,
            MAX_INSTALLED_PACKAGE_CANDIDATES,
            MAX_DIRECTORY_INDEX_PATH_BYTES,
            MAX_AGGREGATE_DIRECTORY_INDEX_PATH_BYTES,
        )
        .unwrap();
        assert_eq!(
            index_winning_game_packages_with_limits(store.as_ref(), stopped),
            Err(PackageIndexError::ChunkScanLimit {
                actual: 5,
                limit: 4,
            })
        );

        let exact = PackageIndexLimits::tightened(
            5,
            MAX_WINNING_EXPORT_BUNDLES,
            MAX_INSTALLED_PACKAGE_CANDIDATES,
            MAX_DIRECTORY_INDEX_PATH_BYTES,
            MAX_AGGREGATE_DIRECTORY_INDEX_PATH_BYTES,
        )
        .unwrap();
        let index = index_winning_game_packages_with_limits(store.as_ref(), exact).unwrap();
        assert_eq!(index.physical_chunk_count, 5);
        assert_eq!(index.winning_export_bundle_count, 1);
        assert_eq!(index.candidates[0].target_path, "/Game/Data/DA_AfterBulk");
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn nonzero_export_index_and_reserved_suffix_are_partial_not_candidates() {
        let root = temp_dir("chunk-id-shape");
        let utoc = root.join("fixture.utoc");
        write_export_id_fixture(
            &utoc,
            [
                (
                    "/Game/Bad/DA_Index",
                    1,
                    0,
                    "../../../G1R/Content/Bad/DA_Index.uasset",
                ),
                (
                    "/Game/Bad/DA_Reserved",
                    0,
                    1,
                    "../../../G1R/Content/Bad/DA_Reserved.uasset",
                ),
                (
                    "/Game/Good/DA_Canonical",
                    0,
                    0,
                    "../../../G1R/Content/Good/DA_Canonical.uasset",
                ),
            ],
        );

        let index = index_winning_game_packages(open(&utoc).as_ref()).unwrap();
        assert_eq!(index.status, PackageIndexStatus::PartialIndex);
        assert_eq!(index.physical_chunk_count, 3);
        assert_eq!(index.winning_export_bundle_count, 3);
        assert_eq!(index.candidates.len(), 1);
        assert_eq!(index.candidates[0].target_path, "/Game/Good/DA_Canonical");
        assert_eq!(
            index.partial_reasons,
            [PackageIndexPartialReasonCount {
                reason: PackageIndexPartialReason::NoncanonicalExportBundleChunkId,
                count: 2,
            }]
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn ambiguous_numeric_patch_priority_is_rejected_independent_of_creation_order() {
        fn run(order: [(&str, &str); 2], label: &str) -> PackageIndexError {
            let root = temp_dir(label);
            for (name, target) in order {
                write_fixture_container(
                    &root.join(format!("{name}.utoc")),
                    [FixtureChunk {
                        target_for_id: target,
                        chunk_type: EIoChunkType::BulkData,
                        directory_path: None,
                        payload: b"priority fixture",
                    }],
                );
            }
            let error = index_winning_game_packages(open(&root).as_ref()).unwrap_err();
            let _ = std::fs::remove_dir_all(root);
            error
        }

        let expected = PackageIndexError::AmbiguousContainerPriority {
            first: "foo_00_P".to_owned(),
            second: "foo_0_P".to_owned(),
        };
        assert_eq!(
            run(
                [("foo_0_P", "/Game/A/DA_A"), ("foo_00_P", "/Game/B/DA_B"),],
                "priority-forward",
            ),
            expected
        );
        assert_eq!(
            run(
                [("foo_00_P", "/Game/B/DA_B"), ("foo_0_P", "/Game/A/DA_A"),],
                "priority-reverse",
            ),
            expected
        );
    }

    #[test]
    fn numeric_patch_priority_overflow_boundary_is_rejected_without_panic() {
        let allowed_name = format!("foo_{MAX_SAFE_NUMERIC_PATCH_VERSION}_P");
        let allowed = std::panic::catch_unwind(|| container_priority_key(&allowed_name));
        assert!(allowed.is_ok(), "priority normalization must not panic");
        let allowed = allowed.unwrap().unwrap();
        assert_eq!(allowed.chunk_version, u64::from(u32::MAX));
        assert_ne!(allowed, container_priority_key("foo").unwrap());

        for version in [u32::MAX - 1, u32::MAX] {
            let name = format!("foo_{version}_P");
            let result = std::panic::catch_unwind(|| container_priority_key(&name));
            assert!(result.is_ok(), "priority normalization must not panic");
            assert_eq!(
                result.unwrap(),
                Err(PackageIndexError::ContainerPriorityVersionOverflow {
                    container_name: name,
                    version,
                    maximum: MAX_SAFE_NUMERIC_PATCH_VERSION,
                })
            );
        }
    }

    #[test]
    fn child_container_names_are_budgeted_before_priority_map_allocation() {
        let root = temp_dir("child-budgets");
        for (name, target) in [("alpha", "/Game/A/DA_A"), ("bravo", "/Game/B/DA_B")] {
            write_fixture_container(
                &root.join(format!("{name}.utoc")),
                [FixtureChunk {
                    target_for_id: target,
                    chunk_type: EIoChunkType::BulkData,
                    directory_path: None,
                    payload: b"bounded child",
                }],
            );
        }
        let store = open(&root);

        assert_eq!(
            index_winning_game_packages_with_limits(
                store.as_ref(),
                limits_with_container_budgets(
                    1,
                    MAX_CONTAINER_PRIORITY_NAME_BYTES,
                    MAX_AGGREGATE_CONTAINER_PRIORITY_NAME_BYTES,
                ),
            ),
            Err(PackageIndexError::ChildContainerLimit {
                actual: 2,
                limit: 1,
            })
        );
        assert_eq!(
            index_winning_game_packages_with_limits(
                store.as_ref(),
                limits_with_container_budgets(2, 4, 8),
            ),
            Err(PackageIndexError::ContainerPriorityNameLimit {
                actual: 5,
                limit: 4,
            })
        );
        assert_eq!(
            index_winning_game_packages_with_limits(
                store.as_ref(),
                limits_with_container_budgets(2, 5, 9),
            ),
            Err(PackageIndexError::AggregateContainerPriorityNameLimit {
                actual: 10,
                limit: 9,
            })
        );
        assert!(matches!(
            PackageIndexLimits::default().with_container_priority_limits(
                MAX_CHILD_CONTAINERS_SCANNED + 1,
                1,
                1,
            ),
            Err(PackageIndexError::InvalidLimit { .. })
        ));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn tightened_limits_fail_closed_before_unbounded_results() {
        let root = temp_dir("limits");
        let utoc = root.join("fixture.utoc");
        write_fixture_container(
            &utoc,
            [
                FixtureChunk {
                    target_for_id: "/Game/A/DA_A",
                    chunk_type: EIoChunkType::ExportBundleData,
                    directory_path: Some("../../../G1R/Content/A/DA_A.uasset"),
                    payload: b"a",
                },
                FixtureChunk {
                    target_for_id: "/Game/B/DA_B",
                    chunk_type: EIoChunkType::ExportBundleData,
                    directory_path: Some("../../../G1R/Content/B/DA_B.uasset"),
                    payload: b"b",
                },
            ],
        );
        let store = open(&utoc);
        let limits = PackageIndexLimits::tightened(
            MAX_PHYSICAL_CHUNKS_SCANNED,
            1,
            MAX_INSTALLED_PACKAGE_CANDIDATES,
            MAX_DIRECTORY_INDEX_PATH_BYTES,
            MAX_AGGREGATE_DIRECTORY_INDEX_PATH_BYTES,
        )
        .unwrap();
        assert_eq!(
            index_winning_game_packages_with_limits(store.as_ref(), limits),
            Err(PackageIndexError::WinningExportBundleLimit {
                actual: 2,
                limit: 1,
            })
        );
        assert!(matches!(
            PackageIndexLimits::tightened(
                MAX_PHYSICAL_CHUNKS_SCANNED,
                MAX_WINNING_EXPORT_BUNDLES + 1,
                1,
                1,
                1,
            ),
            Err(PackageIndexError::InvalidLimit { .. })
        ));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn normalizer_requires_exact_mount_extension_and_safe_game_segments() {
        for invalid in [
            "../../../g1r/Content/Foo/DA_Foo.uasset",
            "../../../G1R/Content/Foo/DA_Foo.UASSET",
            "../../../G1R/Content/Foo//DA_Foo.uasset",
            "../../../G1R/Content/CON/DA_Foo.uasset",
            "../../../G1R/Content/Foo/DA-Foo.uasset",
        ] {
            assert!(matches!(
                normalize_directory_index_path(invalid),
                PathDisposition::NoncanonicalGamePath
            ));
        }
        assert!(matches!(
            normalize_directory_index_path(
                "../../../Engine/Content/EngineMaterials/DefaultMaterial.uasset"
            ),
            PathDisposition::OutOfScope
        ));
        assert!(matches!(
            normalize_directory_index_path("../../../G1R/Content/Foo/DA_Foo.uasset"),
            PathDisposition::Candidate(ref value) if value == "/Game/Foo/DA_Foo"
        ));
    }
}
