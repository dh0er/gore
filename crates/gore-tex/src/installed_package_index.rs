//! Safe path orchestration for one installed, metadata-oriented package-index snapshot.
//!
//! [`crate::package_index`] deliberately starts from an already-open IoStore. This module owns
//! the filesystem boundary needed by higher-level installed-game browsers: it refuses parent
//! traversal and reparse/symlink traversal, inventories the complete bounded Paks tree, validates
//! direct mount names and pairs before Retoc sees the directory, retains no-follow handles, and
//! revalidates every relevant identity after indexing.
//!
//! The resulting evidence is path-free and read-only. Candidate paths are not package-content,
//! extraction, edit, build, deployment, or runtime authority. The package-index pass never asks
//! Retoc to read an `ExportBundleData` chunk. Opening a real IoStore may still make Retoc read
//! other metadata-supporting chunks such as `ContainerHeader`; this module intentionally makes no
//! broader zero-payload claim.
//!
//! V1 is Windows-only. On Unix, retained file descriptors cannot prevent a pathname ABA swap
//! while Retoc independently reopens the directory by path. Every public V1 entry point therefore
//! returns [`InstalledPackageIndexErrorV1::UnsupportedPlatform`] before validating inputs or
//! touching the filesystem on any non-Windows target.

use crate::package_index::{
    index_winning_game_packages_with_limits, validate_unambiguous_container_priority_names,
    InstalledPackageIndex, PackageIndexError, PackageIndexLimits,
};
use retoc::iostore;
use retoc::Config;
use serde::Serialize;
use sha2::{Digest as _, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsStr;
use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read as _, Seek as _, SeekFrom, Write};
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use thiserror::Error;

const G1R_DIRECTORY: &str = "G1R";
const PAKS_RELATIVE_PATH: &str = "Content/Paks";
const EXECUTABLE_RELATIVE_PATH: &str = "Binaries/Win64/G1R-Win64-Shipping.exe";
const MAIN_CONTAINER_FILE_NAME: &str = "G1R-Windows.utoc";

const MOUNT_INVENTORY_SEAL_DOMAIN: &[u8] = b"gore-tex.installed-package-index.mount-inventory.v1\0";
const SOURCE_SNAPSHOT_SEAL_DOMAIN: &[u8] = b"gore-tex.installed-package-index.source-snapshot.v1\0";

pub const MAX_INSTALLED_PAKS_TREE_ENTRIES: usize = 8_192;
pub const MAX_INSTALLED_PAKS_TREE_DEPTH: usize = 8;
pub const MAX_INSTALLED_PAKS_RELATIVE_PATH_BYTES: usize = 4 * 1024;
pub const MAX_INSTALLED_PAKS_AGGREGATE_PATH_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_INSTALLED_DIRECT_MOUNT_FILES: usize = 768;
pub const MAX_INSTALLED_MOUNT_FILE_NAME_BYTES: usize = 255;
pub const MAX_INSTALLED_EXECUTABLE_BYTES: u64 = 2 * 1024 * 1024 * 1024;
pub const MAX_INSTALLED_UTOC_BYTES: u64 = 256 * 1024 * 1024;
pub const MAX_INSTALLED_PAK_BYTES: u64 = 2 * 1024 * 1024 * 1024;
pub const MAX_INSTALLED_UCAS_BYTES: u64 = 128 * 1024 * 1024 * 1024;
pub const MAX_INSTALLED_AGGREGATE_HASHED_MOUNT_BYTES: u64 = 4 * 1024 * 1024 * 1024;
pub const MAX_INSTALLED_PACKAGE_INDEX_JSON_BYTES: usize = 64 * 1024 * 1024;

/// Exact project-owned executable identity expected at the selected installation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExpectedInstalledExecutableV1 {
    pub byte_len: u64,
    pub sha256: [u8; 32],
}

/// Public, path-free SHA-256 content seal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InstalledPackageContentSealV1 {
    pub byte_len: u64,
    pub sha256: String,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct RawSeal {
    byte_len: u64,
    sha256: [u8; 32],
}

impl RawSeal {
    fn public(self) -> InstalledPackageContentSealV1 {
        InstalledPackageContentSealV1 {
            byte_len: self.byte_len,
            sha256: hex_digest(&self.sha256),
        }
    }
}

/// One exact installed package candidate index plus retained local read guards.
///
/// Filesystem paths and platform identities remain private. Callers receive only the candidate
/// document and domain-separated seals, and may cheaply request a fresh full revalidation before
/// using those values as a compare token for a later *separate* operation.
pub struct VerifiedInstalledPackageIndexV1 {
    executable: HeldFile,
    inventory: MountInventory,
    index: InstalledPackageIndex,
    index_json: String,
    target_executable: InstalledPackageContentSealV1,
    mount_inventory_entry_count: u64,
    mount_inventory_seal: InstalledPackageContentSealV1,
    index_seal: InstalledPackageContentSealV1,
    source_snapshot_seal: InstalledPackageContentSealV1,
}

impl fmt::Debug for VerifiedInstalledPackageIndexV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("VerifiedInstalledPackageIndexV1")
            .field("index_status", &self.index.status)
            .field("candidate_count", &self.index.candidates.len())
            .field(
                "mount_inventory_entry_count",
                &self.mount_inventory_entry_count,
            )
            .field("target_executable", &self.target_executable)
            .field("mount_inventory_seal", &self.mount_inventory_seal)
            .field("index_seal", &self.index_seal)
            .field("source_snapshot_seal", &self.source_snapshot_seal)
            .finish()
    }
}

impl VerifiedInstalledPackageIndexV1 {
    pub fn index(&self) -> &InstalledPackageIndex {
        &self.index
    }

    pub fn index_json(&self) -> &str {
        &self.index_json
    }

    pub fn target_executable(&self) -> &InstalledPackageContentSealV1 {
        &self.target_executable
    }

    pub const fn mount_inventory_entry_count(&self) -> u64 {
        self.mount_inventory_entry_count
    }

    pub fn mount_inventory_seal(&self) -> &InstalledPackageContentSealV1 {
        &self.mount_inventory_seal
    }

    pub fn index_seal(&self) -> &InstalledPackageContentSealV1 {
        &self.index_seal
    }

    pub fn source_snapshot_seal(&self) -> &InstalledPackageContentSealV1 {
        &self.source_snapshot_seal
    }

    /// Reopen and compare every retained filesystem identity, fully rehash the executable and
    /// hashed mount files, and rescan the complete bounded Paks tree.
    pub fn revalidate(&self) -> Result<(), InstalledPackageIndexErrorV1> {
        self.executable.revalidate_hashed("game executable")?;
        self.inventory.revalidate()
    }
}

#[derive(Debug, Error)]
pub enum InstalledPackageIndexErrorV1 {
    #[error("the expected installed executable anchor is invalid")]
    InvalidExpectedExecutable,
    #[error("parent-directory traversal is refused for installed package indexing")]
    ParentTraversal,
    #[error("an installed package-index path contains a NUL code unit")]
    PathContainsNul,
    #[error("an installed package-index path is not a plain non-reparse {role}")]
    UnsafePath { role: &'static str },
    #[error("an installed package-index filesystem operation failed: {operation}")]
    Filesystem {
        operation: &'static str,
        #[source]
        source: io::Error,
    },
    #[error("robust installed package-index file identity is unsupported on this platform")]
    UnsupportedPlatform,
    #[error("the Paks tree contains a non-UTF-8 entry")]
    NonUtf8TreeEntry,
    #[error("the Paks tree entry count {actual} exceeds the limit {limit}")]
    TreeEntryLimit { actual: u64, limit: usize },
    #[error("the Paks tree depth {actual} exceeds the limit {limit}")]
    TreeDepthLimit { actual: usize, limit: usize },
    #[error("a Paks relative path has {actual} bytes; limit is {limit}")]
    TreePathLimit { actual: usize, limit: usize },
    #[error("aggregate Paks relative paths have {actual} bytes; limit is {limit}")]
    AggregateTreePathLimit { actual: usize, limit: usize },
    #[error("the Paks tree contains a symbolic-link, reparse, or non-file/non-directory entry")]
    UnsafeTreeEntry,
    #[error("a nested Paks directory contains a mountable .utoc, .ucas, or .pak file")]
    NestedMountable,
    #[error("direct mount file name {file_name:?} is not canonical")]
    NoncanonicalMountName { file_name: String },
    #[error("direct mount file names collide under ASCII case folding")]
    MountNameCollision,
    #[error("direct mount file count {actual} exceeds the limit {limit}")]
    DirectMountLimit { actual: u64, limit: usize },
    #[error("the required main IoStore container is absent")]
    MainContainerMissing,
    #[error("direct IoStore component {file_name:?} has no exact sibling {expected:?}")]
    MountCompanionMissing { file_name: String, expected: String },
    #[error("installed {role} length {actual} exceeds the limit {limit}")]
    FileLengthLimit {
        role: &'static str,
        actual: u64,
        limit: u64,
    },
    #[error("aggregate hashed mount bytes {actual} exceed the limit {limit}")]
    AggregateHashedMountLimit { actual: u64, limit: u64 },
    #[error("the installed executable does not match the exact project generation")]
    ExecutableMismatch,
    #[error("installed {role} changed or was replaced during package indexing")]
    SourceChanged { role: &'static str },
    #[error("the Paks tree changed during package indexing")]
    TreeChanged,
    #[error("container-priority preflight failed: {0}")]
    ContainerPriority(#[source] PackageIndexError),
    #[error("the preflighted and opened IoStore container sets disagree")]
    OpenedContainerSetChanged,
    #[error("the installed IoStore could not be opened safely")]
    IoStoreOpen,
    #[error("installed package indexing failed: {0}")]
    PackageIndex(#[source] PackageIndexError),
    #[error("the installed package index could not be serialized")]
    IndexSerialization,
    #[error("the installed package index JSON exceeds the {limit}-byte limit")]
    IndexJsonLimit { limit: usize },
    #[error("an installed package-index counter overflowed")]
    CounterOverflow,
}

/// Safely inspect one installed game root using the package-index module's default limits.
///
/// V1 supports Windows only. Other targets fail before any filesystem access.
pub fn inspect_installed_package_index_v1(
    game_root: &Path,
    expected_executable: ExpectedInstalledExecutableV1,
) -> Result<VerifiedInstalledPackageIndexV1, InstalledPackageIndexErrorV1> {
    inspect_installed_package_index_with_limits_v1(
        game_root,
        expected_executable,
        PackageIndexLimits::default(),
    )
}

/// As [`inspect_installed_package_index_v1`], with caller-tightened package-index limits.
///
/// V1 supports Windows only. Other targets fail before validating the executable anchor, limits,
/// or any path, and before any filesystem access.
pub fn inspect_installed_package_index_with_limits_v1(
    game_root: &Path,
    expected_executable: ExpectedInstalledExecutableV1,
    index_limits: PackageIndexLimits,
) -> Result<VerifiedInstalledPackageIndexV1, InstalledPackageIndexErrorV1> {
    // `cfg!` is a compile-time constant. Keeping the implementation call in the typed control-flow
    // graph lets every target keep checking the shared code, while non-Windows semantics always
    // return here before input validation or source access. Retoc's later path reopen is the reason
    // a Unix descriptor-only guard is insufficient for V1 exactness.
    if !cfg!(windows) {
        let _ = (game_root, expected_executable, index_limits);
        return Err(InstalledPackageIndexErrorV1::UnsupportedPlatform);
    }
    inspect_installed_package_index_with_hooks_v1(
        game_root,
        expected_executable,
        index_limits,
        |_| {},
        |_| {},
    )
}

fn inspect_installed_package_index_with_hooks_v1<BeforeOpen, AfterIndex>(
    game_root: &Path,
    expected_executable: ExpectedInstalledExecutableV1,
    index_limits: PackageIndexLimits,
    before_retoc_open: BeforeOpen,
    after_index: AfterIndex,
) -> Result<VerifiedInstalledPackageIndexV1, InstalledPackageIndexErrorV1>
where
    BeforeOpen: FnOnce(&Path),
    AfterIndex: FnOnce(&Path),
{
    if expected_executable.byte_len == 0
        || expected_executable.byte_len > MAX_INSTALLED_EXECUTABLE_BYTES
    {
        return Err(InstalledPackageIndexErrorV1::InvalidExpectedExecutable);
    }

    let layout = resolve_game_layout(game_root)?;
    let executable = HeldFile::open_hashed(
        &layout.executable,
        MAX_INSTALLED_EXECUTABLE_BYTES,
        "game executable",
    )?;
    if executable.snapshot.length != expected_executable.byte_len
        || executable
            .sha256
            .is_none_or(|digest| digest != expected_executable.sha256)
    {
        return Err(InstalledPackageIndexErrorV1::ExecutableMismatch);
    }

    let inventory = MountInventory::capture(&layout.paks, index_limits)?;
    before_retoc_open(&layout.paks);

    let store = iostore::open(&layout.paks, Arc::new(Config::default()))
        .map_err(|_| InstalledPackageIndexErrorV1::IoStoreOpen)?;
    let mut opened_container_names = store
        .child_containers()
        .map(|container| container.container_name().to_owned())
        .collect::<Vec<_>>();
    opened_container_names.sort();
    if opened_container_names != inventory.expected_container_names {
        return Err(InstalledPackageIndexErrorV1::OpenedContainerSetChanged);
    }

    let index = index_winning_game_packages_with_limits(store.as_ref(), index_limits)
        .map_err(InstalledPackageIndexErrorV1::PackageIndex)?;
    after_index(&layout.paks);

    executable.revalidate_hashed("game executable")?;
    inventory.revalidate()?;
    let index_json = serialize_index_bounded(&index)?;
    executable.revalidate_hashed("game executable")?;
    inventory.revalidate()?;

    let executable_raw = RawSeal {
        byte_len: executable.snapshot.length,
        sha256: executable
            .sha256
            .expect("a hashed executable retains its digest"),
    };
    let index_raw = RawSeal {
        byte_len: u64::try_from(index_json.len())
            .map_err(|_| InstalledPackageIndexErrorV1::CounterOverflow)?,
        sha256: Sha256::digest(index_json.as_bytes()).into(),
    };
    let source_snapshot_raw =
        source_snapshot_seal(executable_raw, inventory.inventory_seal, index_raw)?;
    let mount_inventory_entry_count = u64::try_from(inventory.tree.entries.len())
        .map_err(|_| InstalledPackageIndexErrorV1::CounterOverflow)?;

    Ok(VerifiedInstalledPackageIndexV1 {
        executable,
        mount_inventory_seal: inventory.inventory_seal.public(),
        inventory,
        index,
        index_json,
        target_executable: executable_raw.public(),
        mount_inventory_entry_count,
        index_seal: index_raw.public(),
        source_snapshot_seal: source_snapshot_raw.public(),
    })
}

struct GameLayout {
    paks: PathBuf,
    executable: PathBuf,
}

fn resolve_game_layout(game_root: &Path) -> Result<GameLayout, InstalledPackageIndexErrorV1> {
    let root = canonical_existing_plain(game_root, NodeKind::Directory, "game root")?;
    let g1r = if root.file_name() == Some(OsStr::new(G1R_DIRECTORY)) {
        root
    } else {
        canonical_existing_plain(
            &root.join(G1R_DIRECTORY),
            NodeKind::Directory,
            "G1R directory",
        )?
    };
    let paks = canonical_existing_plain(
        &g1r.join(PAKS_RELATIVE_PATH),
        NodeKind::Directory,
        "Paks directory",
    )?;
    let executable = g1r.join(EXECUTABLE_RELATIVE_PATH);
    Ok(GameLayout { paks, executable })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum TreeEntryKind {
    Directory,
    File,
}

impl TreeEntryKind {
    const fn seal_tag(self) -> u8 {
        match self {
            Self::Directory => 1,
            Self::File => 2,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct TreeEntryDescriptor {
    relative_path: String,
    kind: TreeEntryKind,
    snapshot: FileSnapshot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum DirectMountKind {
    Pak,
    Ucas,
    Utoc,
}

impl DirectMountKind {
    const fn extension(self) -> &'static str {
        match self {
            Self::Pak => "pak",
            Self::Ucas => "ucas",
            Self::Utoc => "utoc",
        }
    }

    const fn seal_tag(self) -> u8 {
        match self {
            Self::Pak => 1,
            Self::Ucas => 2,
            Self::Utoc => 3,
        }
    }
}

#[derive(Debug, Clone)]
struct MountCandidate {
    file_name: String,
    stem: String,
    kind: DirectMountKind,
    path: PathBuf,
}

impl MountCandidate {
    fn descriptor(&self) -> MountDescriptor {
        MountDescriptor {
            file_name: self.file_name.clone(),
            stem: self.stem.clone(),
            kind: self.kind,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct MountDescriptor {
    file_name: String,
    stem: String,
    kind: DirectMountKind,
}

struct TreeScan {
    entries: Vec<TreeEntryDescriptor>,
    directory_paths: Vec<PathBuf>,
    mounts: Vec<MountCandidate>,
}

impl TreeScan {
    fn mount_descriptors(&self) -> Vec<MountDescriptor> {
        self.mounts.iter().map(MountCandidate::descriptor).collect()
    }

    fn same_shape(&self, other: &Self) -> bool {
        self.entries == other.entries && self.mount_descriptors() == other.mount_descriptors()
    }
}

fn scan_paks_tree(paks: &Path) -> Result<TreeScan, InstalledPackageIndexErrorV1> {
    let mut pending = vec![(paks.to_path_buf(), 0usize)];
    let mut entry_count = 0u64;
    let mut aggregate_path_bytes = 0usize;
    let mut entries = Vec::new();
    let mut directory_paths = Vec::new();
    let mut mounts = Vec::new();

    while let Some((directory, depth)) = pending.pop() {
        let iterator = fs::read_dir(&directory).map_err(|source| {
            InstalledPackageIndexErrorV1::Filesystem {
                operation: "read Paks directory",
                source,
            }
        })?;
        for entry in iterator {
            let entry = entry.map_err(|source| InstalledPackageIndexErrorV1::Filesystem {
                operation: "read Paks entry",
                source,
            })?;
            entry_count = entry_count
                .checked_add(1)
                .ok_or(InstalledPackageIndexErrorV1::CounterOverflow)?;
            if entry_count > MAX_INSTALLED_PAKS_TREE_ENTRIES as u64 {
                return Err(InstalledPackageIndexErrorV1::TreeEntryLimit {
                    actual: entry_count,
                    limit: MAX_INSTALLED_PAKS_TREE_ENTRIES,
                });
            }

            let path = entry.path();
            let metadata = fs::symlink_metadata(&path).map_err(|source| {
                InstalledPackageIndexErrorV1::Filesystem {
                    operation: "inspect Paks entry",
                    source,
                }
            })?;
            if metadata_is_reparse(&metadata) || (!metadata.is_dir() && !metadata.is_file()) {
                return Err(InstalledPackageIndexErrorV1::UnsafeTreeEntry);
            }
            let relative = path
                .strip_prefix(paks)
                .map_err(|_| InstalledPackageIndexErrorV1::UnsafeTreeEntry)?;
            let relative_path = slash_utf8_relative(relative)?;
            if relative_path.len() > MAX_INSTALLED_PAKS_RELATIVE_PATH_BYTES {
                return Err(InstalledPackageIndexErrorV1::TreePathLimit {
                    actual: relative_path.len(),
                    limit: MAX_INSTALLED_PAKS_RELATIVE_PATH_BYTES,
                });
            }
            aggregate_path_bytes = aggregate_path_bytes
                .checked_add(relative_path.len())
                .ok_or(InstalledPackageIndexErrorV1::CounterOverflow)?;
            if aggregate_path_bytes > MAX_INSTALLED_PAKS_AGGREGATE_PATH_BYTES {
                return Err(InstalledPackageIndexErrorV1::AggregateTreePathLimit {
                    actual: aggregate_path_bytes,
                    limit: MAX_INSTALLED_PAKS_AGGREGATE_PATH_BYTES,
                });
            }

            if metadata.is_dir() {
                let next_depth = depth
                    .checked_add(1)
                    .ok_or(InstalledPackageIndexErrorV1::CounterOverflow)?;
                if next_depth > MAX_INSTALLED_PAKS_TREE_DEPTH {
                    return Err(InstalledPackageIndexErrorV1::TreeDepthLimit {
                        actual: next_depth,
                        limit: MAX_INSTALLED_PAKS_TREE_DEPTH,
                    });
                }
                entries.push(TreeEntryDescriptor {
                    relative_path,
                    kind: TreeEntryKind::Directory,
                    snapshot: snapshot_tree_entry(&path, NodeKind::Directory)?,
                });
                directory_paths.push(path.clone());
                pending.push((path, next_depth));
                continue;
            }

            entries.push(TreeEntryDescriptor {
                relative_path,
                kind: TreeEntryKind::File,
                snapshot: snapshot_tree_entry(&path, NodeKind::File)?,
            });
            let mount_kind = mount_kind_for_path(&path);
            if depth != 0 {
                if mount_kind.is_some() {
                    return Err(InstalledPackageIndexErrorV1::NestedMountable);
                }
                continue;
            }
            let Some(kind) = mount_kind else {
                continue;
            };
            let file_name = canonical_mount_file_name(&path, kind)?;
            let stem = file_name
                .strip_suffix(&format!(".{}", kind.extension()))
                .expect("the canonical extension was checked")
                .to_owned();
            mounts.push(MountCandidate {
                file_name,
                stem,
                kind,
                path,
            });
        }
    }

    entries.sort();
    directory_paths.sort();
    mounts.sort_by(|left, right| left.file_name.cmp(&right.file_name));
    Ok(TreeScan {
        entries,
        directory_paths,
        mounts,
    })
}

fn snapshot_tree_entry(
    path: &Path,
    kind: NodeKind,
) -> Result<FileSnapshot, InstalledPackageIndexErrorV1> {
    let held = HeldFile::open(path, kind, None, false, "Paks tree entry")?;
    Ok(held.snapshot)
}

fn slash_utf8_relative(path: &Path) -> Result<String, InstalledPackageIndexErrorV1> {
    let mut output = String::new();
    for component in path.components() {
        let Component::Normal(value) = component else {
            return Err(InstalledPackageIndexErrorV1::UnsafeTreeEntry);
        };
        let value = value
            .to_str()
            .ok_or(InstalledPackageIndexErrorV1::NonUtf8TreeEntry)?;
        if !output.is_empty() {
            output.push('/');
        }
        output.push_str(value);
    }
    if output.is_empty() {
        return Err(InstalledPackageIndexErrorV1::UnsafeTreeEntry);
    }
    Ok(output)
}

fn mount_kind_for_path(path: &Path) -> Option<DirectMountKind> {
    let extension = path.extension()?.to_str()?;
    if extension.eq_ignore_ascii_case("utoc") {
        Some(DirectMountKind::Utoc)
    } else if extension.eq_ignore_ascii_case("ucas") {
        Some(DirectMountKind::Ucas)
    } else if extension.eq_ignore_ascii_case("pak") {
        Some(DirectMountKind::Pak)
    } else {
        None
    }
}

fn canonical_mount_file_name(
    path: &Path,
    kind: DirectMountKind,
) -> Result<String, InstalledPackageIndexErrorV1> {
    let file_name = path
        .file_name()
        .and_then(OsStr::to_str)
        .ok_or(InstalledPackageIndexErrorV1::NonUtf8TreeEntry)?;
    let canonical = !file_name.is_empty()
        && file_name.len() <= MAX_INSTALLED_MOUNT_FILE_NAME_BYTES
        && file_name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
        && !file_name.ends_with(['.', ' '])
        && !windows_reserved_name(file_name)
        && path.extension().and_then(OsStr::to_str) == Some(kind.extension());
    let suffix = format!(".{}", kind.extension());
    if !canonical
        || file_name
            .strip_suffix(&suffix)
            .is_none_or(|stem| stem.is_empty() || windows_reserved_name(stem))
    {
        return Err(InstalledPackageIndexErrorV1::NoncanonicalMountName {
            file_name: truncate_utf8(file_name, MAX_INSTALLED_MOUNT_FILE_NAME_BYTES),
        });
    }
    Ok(file_name.to_owned())
}

fn windows_reserved_name(name: &str) -> bool {
    let base = name.split('.').next().unwrap_or(name).to_ascii_uppercase();
    matches!(base.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || (base.len() == 4
            && (base.starts_with("COM") || base.starts_with("LPT"))
            && matches!(base.as_bytes()[3], b'1'..=b'9'))
}

struct MountLayout {
    expected_container_names: Vec<String>,
}

fn validate_mount_layout(
    mounts: &[MountCandidate],
    index_limits: PackageIndexLimits,
) -> Result<MountLayout, InstalledPackageIndexErrorV1> {
    if mounts.len() > MAX_INSTALLED_DIRECT_MOUNT_FILES {
        return Err(InstalledPackageIndexErrorV1::DirectMountLimit {
            actual: u64::try_from(mounts.len())
                .map_err(|_| InstalledPackageIndexErrorV1::CounterOverflow)?,
            limit: MAX_INSTALLED_DIRECT_MOUNT_FILES,
        });
    }
    let mut folded_names = BTreeSet::new();
    for mount in mounts {
        if !folded_names.insert(mount.file_name.to_ascii_lowercase()) {
            return Err(InstalledPackageIndexErrorV1::MountNameCollision);
        }
    }
    if !mounts
        .iter()
        .any(|mount| mount.file_name == MAIN_CONTAINER_FILE_NAME)
    {
        return Err(InstalledPackageIndexErrorV1::MainContainerMissing);
    }

    let by_name = mounts
        .iter()
        .map(|mount| (mount.file_name.as_str(), mount))
        .collect::<BTreeMap<_, _>>();
    for mount in mounts {
        match mount.kind {
            DirectMountKind::Utoc => {
                let expected = format!("{}.ucas", mount.stem);
                if !by_name.contains_key(expected.as_str()) {
                    return Err(InstalledPackageIndexErrorV1::MountCompanionMissing {
                        file_name: mount.file_name.clone(),
                        expected,
                    });
                }
            }
            DirectMountKind::Ucas => {
                let expected = format!("{}.utoc", mount.stem);
                if !by_name.contains_key(expected.as_str()) {
                    return Err(InstalledPackageIndexErrorV1::MountCompanionMissing {
                        file_name: mount.file_name.clone(),
                        expected,
                    });
                }
            }
            DirectMountKind::Pak => {}
        }
    }

    let mut expected_container_names = mounts
        .iter()
        .filter(|mount| mount.kind == DirectMountKind::Utoc)
        .map(|mount| mount.stem.clone())
        .collect::<Vec<_>>();
    expected_container_names.sort();
    validate_unambiguous_container_priority_names(
        expected_container_names.iter().map(String::as_str),
        index_limits,
    )
    .map_err(InstalledPackageIndexErrorV1::ContainerPriority)?;
    Ok(MountLayout {
        expected_container_names,
    })
}

struct MountAnchor {
    file_name: String,
    kind: DirectMountKind,
    file: HeldFile,
}

struct MountInventory {
    paks: PathBuf,
    paks_guard: HeldFile,
    directory_guards: Vec<HeldFile>,
    mounts: Vec<MountAnchor>,
    tree: TreeScan,
    expected_container_names: Vec<String>,
    index_limits: PackageIndexLimits,
    inventory_seal: RawSeal,
}

impl MountInventory {
    fn capture(
        paks: &Path,
        index_limits: PackageIndexLimits,
    ) -> Result<Self, InstalledPackageIndexErrorV1> {
        let paks_guard = HeldFile::open_directory(paks, "Paks directory")?;
        let first = scan_paks_tree(paks)?;
        let layout = validate_mount_layout(&first.mounts, index_limits)?;
        let mut directory_guards = Vec::with_capacity(first.directory_paths.len());
        for path in &first.directory_paths {
            directory_guards.push(HeldFile::open_directory(path, "Paks child directory")?);
        }

        let mut aggregate_hashed_bytes = 0u64;
        let mut mounts = Vec::with_capacity(first.mounts.len());
        for candidate in &first.mounts {
            let (limit, hashed, role) = match candidate.kind {
                DirectMountKind::Utoc => (MAX_INSTALLED_UTOC_BYTES, true, "UTOC"),
                DirectMountKind::Pak => (MAX_INSTALLED_PAK_BYTES, true, "PAK"),
                DirectMountKind::Ucas => (MAX_INSTALLED_UCAS_BYTES, false, "UCAS"),
            };
            let file = if hashed {
                let file = HeldFile::open_hashed(&candidate.path, limit, role)?;
                aggregate_hashed_bytes = aggregate_hashed_bytes
                    .checked_add(file.snapshot.length)
                    .ok_or(InstalledPackageIndexErrorV1::CounterOverflow)?;
                if aggregate_hashed_bytes > MAX_INSTALLED_AGGREGATE_HASHED_MOUNT_BYTES {
                    return Err(InstalledPackageIndexErrorV1::AggregateHashedMountLimit {
                        actual: aggregate_hashed_bytes,
                        limit: MAX_INSTALLED_AGGREGATE_HASHED_MOUNT_BYTES,
                    });
                }
                file
            } else {
                HeldFile::open_identity(&candidate.path, limit, role)?
            };
            mounts.push(MountAnchor {
                file_name: candidate.file_name.clone(),
                kind: candidate.kind,
                file,
            });
        }

        let second = scan_paks_tree(paks)?;
        let second_layout = validate_mount_layout(&second.mounts, index_limits)?;
        if !first.same_shape(&second)
            || layout.expected_container_names != second_layout.expected_container_names
        {
            return Err(InstalledPackageIndexErrorV1::TreeChanged);
        }
        paks_guard.revalidate_identity("Paks directory")?;
        for guard in &directory_guards {
            guard.revalidate_identity("Paks child directory")?;
        }
        for mount in &mounts {
            mount.revalidate()?;
        }
        let inventory_seal = mount_inventory_seal(&first, &mounts)?;
        Ok(Self {
            paks: paks.to_path_buf(),
            paks_guard,
            directory_guards,
            mounts,
            tree: first,
            expected_container_names: layout.expected_container_names,
            index_limits,
            inventory_seal,
        })
    }

    fn revalidate(&self) -> Result<(), InstalledPackageIndexErrorV1> {
        self.paks_guard.revalidate_identity("Paks directory")?;
        for guard in &self.directory_guards {
            guard.revalidate_identity("Paks child directory")?;
        }
        for mount in &self.mounts {
            mount.revalidate()?;
        }
        let current = scan_paks_tree(&self.paks)?;
        let layout = validate_mount_layout(&current.mounts, self.index_limits)?;
        if !self.tree.same_shape(&current)
            || self.expected_container_names != layout.expected_container_names
        {
            return Err(InstalledPackageIndexErrorV1::TreeChanged);
        }
        Ok(())
    }
}

impl MountAnchor {
    fn revalidate(&self) -> Result<(), InstalledPackageIndexErrorV1> {
        match self.kind {
            DirectMountKind::Utoc => self.file.revalidate_hashed("UTOC"),
            DirectMountKind::Pak => self.file.revalidate_hashed("PAK"),
            DirectMountKind::Ucas => self.file.revalidate_identity("UCAS"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NodeKind {
    Directory,
    File,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct FileSnapshot {
    length: u64,
    modified_stamp: String,
    platform_identity: String,
}

struct HeldFile {
    path: PathBuf,
    file: File,
    snapshot: FileSnapshot,
    kind: NodeKind,
    max_bytes: Option<u64>,
    sha256: Option<[u8; 32]>,
}

impl HeldFile {
    fn open_directory(
        path: &Path,
        role: &'static str,
    ) -> Result<Self, InstalledPackageIndexErrorV1> {
        Self::open(path, NodeKind::Directory, None, false, role)
    }

    fn open_identity(
        path: &Path,
        max_bytes: u64,
        role: &'static str,
    ) -> Result<Self, InstalledPackageIndexErrorV1> {
        Self::open(path, NodeKind::File, Some(max_bytes), false, role)
    }

    fn open_hashed(
        path: &Path,
        max_bytes: u64,
        role: &'static str,
    ) -> Result<Self, InstalledPackageIndexErrorV1> {
        Self::open(path, NodeKind::File, Some(max_bytes), true, role)
    }

    fn open(
        path: &Path,
        kind: NodeKind,
        max_bytes: Option<u64>,
        hash: bool,
        role: &'static str,
    ) -> Result<Self, InstalledPackageIndexErrorV1> {
        let path = canonical_existing_plain(path, kind, role)?;
        let file = open_node_no_follow(&path, kind)?;
        let metadata =
            file.metadata()
                .map_err(|source| InstalledPackageIndexErrorV1::Filesystem {
                    operation: "read retained source metadata",
                    source,
                })?;
        validate_opened_metadata(&metadata, kind, role)?;
        let snapshot = file_identity_snapshot(&file, &metadata)?;
        if let Some(limit) = max_bytes {
            if snapshot.length > limit {
                return Err(InstalledPackageIndexErrorV1::FileLengthLimit {
                    role,
                    actual: snapshot.length,
                    limit,
                });
            }
        }
        let sha256 = if hash {
            Some(hash_held_file(
                &file,
                max_bytes.expect("hashed files have a limit"),
                role,
            )?)
        } else {
            None
        };
        let held = Self {
            path,
            file,
            snapshot,
            kind,
            max_bytes,
            sha256,
        };
        held.revalidate_identity(role)?;
        Ok(held)
    }

    fn revalidate_identity(&self, role: &'static str) -> Result<(), InstalledPackageIndexErrorV1> {
        let handle_metadata =
            self.file
                .metadata()
                .map_err(|source| InstalledPackageIndexErrorV1::Filesystem {
                    operation: "reinspect retained source metadata",
                    source,
                })?;
        validate_opened_metadata(&handle_metadata, self.kind, role)?;
        let handle_snapshot = file_identity_snapshot(&self.file, &handle_metadata)?;
        if handle_snapshot != self.snapshot {
            return Err(InstalledPackageIndexErrorV1::SourceChanged { role });
        }
        let current_path = canonical_existing_plain(&self.path, self.kind, role)?;
        if current_path != self.path {
            return Err(InstalledPackageIndexErrorV1::SourceChanged { role });
        }
        let reopened = open_node_no_follow(&current_path, self.kind)?;
        let reopened_metadata =
            reopened
                .metadata()
                .map_err(|source| InstalledPackageIndexErrorV1::Filesystem {
                    operation: "read reopened source metadata",
                    source,
                })?;
        validate_opened_metadata(&reopened_metadata, self.kind, role)?;
        if file_identity_snapshot(&reopened, &reopened_metadata)? != self.snapshot {
            return Err(InstalledPackageIndexErrorV1::SourceChanged { role });
        }
        Ok(())
    }

    fn revalidate_hashed(&self, role: &'static str) -> Result<(), InstalledPackageIndexErrorV1> {
        self.revalidate_identity(role)?;
        let digest = hash_held_file(
            &self.file,
            self.max_bytes.expect("hashed files retain their limit"),
            role,
        )?;
        if self.sha256 != Some(digest) {
            return Err(InstalledPackageIndexErrorV1::SourceChanged { role });
        }
        self.revalidate_identity(role)
    }
}

fn canonical_existing_plain(
    path: &Path,
    expected: NodeKind,
    role: &'static str,
) -> Result<PathBuf, InstalledPackageIndexErrorV1> {
    if path.as_os_str().is_empty() {
        return Err(InstalledPackageIndexErrorV1::UnsafePath { role });
    }
    if path
        .components()
        .any(|component| component == Component::ParentDir)
    {
        return Err(InstalledPackageIndexErrorV1::ParentTraversal);
    }
    if path_has_nul(path) {
        return Err(InstalledPackageIndexErrorV1::PathContainsNul);
    }
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|source| InstalledPackageIndexErrorV1::Filesystem {
                operation: "resolve current directory",
                source,
            })?
            .join(path)
    };
    validate_plain_chain(&absolute, expected, role)?;
    let canonical =
        fs::canonicalize(&absolute).map_err(|source| InstalledPackageIndexErrorV1::Filesystem {
            operation: "canonicalize installed source",
            source,
        })?;
    validate_plain_chain(&canonical, expected, role)?;
    Ok(canonical)
}

fn validate_plain_chain(
    path: &Path,
    expected: NodeKind,
    role: &'static str,
) -> Result<(), InstalledPackageIndexErrorV1> {
    for ancestor in path.ancestors() {
        let metadata = fs::symlink_metadata(ancestor).map_err(|source| {
            InstalledPackageIndexErrorV1::Filesystem {
                operation: "inspect installed source path",
                source,
            }
        })?;
        if metadata_is_reparse(&metadata) {
            return Err(InstalledPackageIndexErrorV1::UnsafePath { role });
        }
    }
    let metadata =
        fs::symlink_metadata(path).map_err(|source| InstalledPackageIndexErrorV1::Filesystem {
            operation: "inspect installed source node",
            source,
        })?;
    if metadata_is_reparse(&metadata)
        || (expected == NodeKind::Directory && !metadata.is_dir())
        || (expected == NodeKind::File && !metadata.is_file())
    {
        return Err(InstalledPackageIndexErrorV1::UnsafePath { role });
    }
    Ok(())
}

fn validate_opened_metadata(
    metadata: &fs::Metadata,
    expected: NodeKind,
    role: &'static str,
) -> Result<(), InstalledPackageIndexErrorV1> {
    if metadata_is_reparse(metadata)
        || (expected == NodeKind::Directory && !metadata.is_dir())
        || (expected == NodeKind::File && !metadata.is_file())
    {
        return Err(InstalledPackageIndexErrorV1::UnsafePath { role });
    }
    Ok(())
}

#[cfg(windows)]
fn path_has_nul(path: &Path) -> bool {
    use std::os::windows::ffi::OsStrExt as _;
    path.as_os_str().encode_wide().any(|unit| unit == 0)
}

#[cfg(unix)]
fn path_has_nul(path: &Path) -> bool {
    use std::os::unix::ffi::OsStrExt as _;
    path.as_os_str().as_bytes().contains(&0)
}

#[cfg(not(any(windows, unix)))]
fn path_has_nul(_path: &Path) -> bool {
    true
}

#[cfg(windows)]
fn metadata_is_reparse(metadata: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt as _;
    use windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_REPARSE_POINT;
    metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

#[cfg(not(windows))]
fn metadata_is_reparse(metadata: &fs::Metadata) -> bool {
    metadata.file_type().is_symlink()
}

#[cfg(windows)]
fn open_node_no_follow(path: &Path, kind: NodeKind) -> Result<File, InstalledPackageIndexErrorV1> {
    use std::os::windows::fs::OpenOptionsExt as _;
    use windows_sys::Win32::Storage::FileSystem::{
        FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT, FILE_SHARE_READ, FILE_SHARE_WRITE,
    };
    let mut options = OpenOptions::new();
    options.read(true).custom_flags(
        FILE_FLAG_OPEN_REPARSE_POINT
            | if kind == NodeKind::Directory {
                FILE_FLAG_BACKUP_SEMANTICS
            } else {
                0
            },
    );
    if kind == NodeKind::Directory {
        // Child creation remains possible, but replacement/rename of an anchored directory is
        // denied while the snapshot is live. Tree additions are still caught by the set/shape
        // checks; the directory that gives those paths meaning cannot be swapped underneath us.
        options.share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE);
    } else {
        // Retoc may open the same source for reads. Omitting write/delete sharing keeps every
        // direct mount immutable while this snapshot is live on Windows.
        options.share_mode(FILE_SHARE_READ);
    }
    options
        .open(path)
        .map_err(|source| InstalledPackageIndexErrorV1::Filesystem {
            operation: "open installed source without following reparse data",
            source,
        })
}

#[cfg(unix)]
fn open_node_no_follow(path: &Path, kind: NodeKind) -> Result<File, InstalledPackageIndexErrorV1> {
    use std::os::unix::fs::OpenOptionsExt as _;
    let flags = libc::O_NOFOLLOW
        | libc::O_CLOEXEC
        | if kind == NodeKind::Directory {
            libc::O_DIRECTORY
        } else {
            libc::O_NONBLOCK
        };
    OpenOptions::new()
        .read(true)
        .custom_flags(flags)
        .open(path)
        .map_err(|source| InstalledPackageIndexErrorV1::Filesystem {
            operation: "open installed source without following symlinks",
            source,
        })
}

#[cfg(not(any(windows, unix)))]
fn open_node_no_follow(
    _path: &Path,
    _kind: NodeKind,
) -> Result<File, InstalledPackageIndexErrorV1> {
    Err(InstalledPackageIndexErrorV1::UnsupportedPlatform)
}

#[cfg(windows)]
fn file_identity_snapshot(
    file: &File,
    metadata: &fs::Metadata,
) -> Result<FileSnapshot, InstalledPackageIndexErrorV1> {
    use std::os::windows::fs::MetadataExt as _;
    use std::os::windows::io::AsRawHandle as _;
    use windows_sys::Win32::Storage::FileSystem::{
        GetFileInformationByHandle, BY_HANDLE_FILE_INFORMATION,
    };
    // SAFETY: `information` has the exact Win32 layout and `file` owns a valid handle for the
    // duration of the call.
    let mut information: BY_HANDLE_FILE_INFORMATION = unsafe { std::mem::zeroed() };
    let success = unsafe { GetFileInformationByHandle(file.as_raw_handle(), &mut information) };
    if success == 0 {
        return Err(InstalledPackageIndexErrorV1::Filesystem {
            operation: "read stable Windows source identity",
            source: io::Error::last_os_error(),
        });
    }
    let index =
        (u64::from(information.nFileIndexHigh) << 32) | u64::from(information.nFileIndexLow);
    Ok(FileSnapshot {
        length: metadata.file_size(),
        modified_stamp: metadata.last_write_time().to_string(),
        platform_identity: format!(
            "windows-volume-{:08x}-file-{index:016x}",
            information.dwVolumeSerialNumber
        ),
    })
}

#[cfg(unix)]
fn file_identity_snapshot(
    _file: &File,
    metadata: &fs::Metadata,
) -> Result<FileSnapshot, InstalledPackageIndexErrorV1> {
    use std::os::unix::fs::MetadataExt as _;
    Ok(FileSnapshot {
        length: metadata.len(),
        modified_stamp: format!("{}.{:09}", metadata.mtime(), metadata.mtime_nsec()),
        platform_identity: format!("unix-dev-{:x}-ino-{:x}", metadata.dev(), metadata.ino()),
    })
}

#[cfg(not(any(windows, unix)))]
fn file_identity_snapshot(
    _file: &File,
    _metadata: &fs::Metadata,
) -> Result<FileSnapshot, InstalledPackageIndexErrorV1> {
    Err(InstalledPackageIndexErrorV1::UnsupportedPlatform)
}

fn hash_held_file(
    file: &File,
    max_bytes: u64,
    role: &'static str,
) -> Result<[u8; 32], InstalledPackageIndexErrorV1> {
    let metadata = file
        .metadata()
        .map_err(|source| InstalledPackageIndexErrorV1::Filesystem {
            operation: "inspect source before hashing",
            source,
        })?;
    if metadata.len() > max_bytes {
        return Err(InstalledPackageIndexErrorV1::FileLengthLimit {
            role,
            actual: metadata.len(),
            limit: max_bytes,
        });
    }
    let mut reader = file;
    reader
        .seek(SeekFrom::Start(0))
        .map_err(|source| InstalledPackageIndexErrorV1::Filesystem {
            operation: "seek retained source before hashing",
            source,
        })?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    let mut length = 0u64;
    loop {
        let read = reader.read(&mut buffer).map_err(|source| {
            InstalledPackageIndexErrorV1::Filesystem {
                operation: "hash retained source",
                source,
            }
        })?;
        if read == 0 {
            break;
        }
        length = length
            .checked_add(
                u64::try_from(read).map_err(|_| InstalledPackageIndexErrorV1::CounterOverflow)?,
            )
            .ok_or(InstalledPackageIndexErrorV1::CounterOverflow)?;
        if length > max_bytes {
            return Err(InstalledPackageIndexErrorV1::FileLengthLimit {
                role,
                actual: length,
                limit: max_bytes,
            });
        }
        hasher.update(&buffer[..read]);
    }
    if length != metadata.len() {
        return Err(InstalledPackageIndexErrorV1::SourceChanged { role });
    }
    Ok(hasher.finalize().into())
}

fn mount_inventory_seal(
    tree: &TreeScan,
    mounts: &[MountAnchor],
) -> Result<RawSeal, InstalledPackageIndexErrorV1> {
    let mut builder = SealBuilder::new(MOUNT_INVENTORY_SEAL_DOMAIN)?;
    builder.u64(
        u64::try_from(tree.entries.len())
            .map_err(|_| InstalledPackageIndexErrorV1::CounterOverflow)?,
    )?;
    for entry in &tree.entries {
        builder.byte(entry.kind.seal_tag())?;
        builder.bytes(entry.relative_path.as_bytes())?;
        builder.u64(entry.snapshot.length)?;
        builder.bytes(entry.snapshot.modified_stamp.as_bytes())?;
        builder.bytes(entry.snapshot.platform_identity.as_bytes())?;
    }
    builder.u64(
        u64::try_from(mounts.len()).map_err(|_| InstalledPackageIndexErrorV1::CounterOverflow)?,
    )?;
    for mount in mounts {
        builder.byte(mount.kind.seal_tag())?;
        builder.bytes(mount.file_name.as_bytes())?;
        builder.u64(mount.file.snapshot.length)?;
        match mount.file.sha256 {
            Some(digest) => {
                builder.byte(1)?;
                builder.bytes(&digest)?;
            }
            None => {
                builder.byte(2)?;
                builder.bytes(mount.file.snapshot.modified_stamp.as_bytes())?;
                builder.bytes(mount.file.snapshot.platform_identity.as_bytes())?;
            }
        }
    }
    Ok(builder.finish())
}

fn source_snapshot_seal(
    executable: RawSeal,
    inventory: RawSeal,
    index: RawSeal,
) -> Result<RawSeal, InstalledPackageIndexErrorV1> {
    let mut builder = SealBuilder::new(SOURCE_SNAPSHOT_SEAL_DOMAIN)?;
    for seal in [executable, inventory, index] {
        builder.u64(seal.byte_len)?;
        builder.bytes(&seal.sha256)?;
    }
    Ok(builder.finish())
}

struct SealBuilder {
    hasher: Sha256,
    byte_len: u64,
}

impl SealBuilder {
    fn new(domain: &[u8]) -> Result<Self, InstalledPackageIndexErrorV1> {
        let mut hasher = Sha256::new();
        hasher.update(domain);
        Ok(Self {
            hasher,
            byte_len: u64::try_from(domain.len())
                .map_err(|_| InstalledPackageIndexErrorV1::CounterOverflow)?,
        })
    }

    fn byte(&mut self, value: u8) -> Result<(), InstalledPackageIndexErrorV1> {
        self.hasher.update([value]);
        self.byte_len = self
            .byte_len
            .checked_add(1)
            .ok_or(InstalledPackageIndexErrorV1::CounterOverflow)?;
        Ok(())
    }

    fn u64(&mut self, value: u64) -> Result<(), InstalledPackageIndexErrorV1> {
        self.hasher.update(value.to_le_bytes());
        self.byte_len = self
            .byte_len
            .checked_add(8)
            .ok_or(InstalledPackageIndexErrorV1::CounterOverflow)?;
        Ok(())
    }

    fn bytes(&mut self, value: &[u8]) -> Result<(), InstalledPackageIndexErrorV1> {
        self.u64(
            u64::try_from(value.len())
                .map_err(|_| InstalledPackageIndexErrorV1::CounterOverflow)?,
        )?;
        self.hasher.update(value);
        self.byte_len = self
            .byte_len
            .checked_add(
                u64::try_from(value.len())
                    .map_err(|_| InstalledPackageIndexErrorV1::CounterOverflow)?,
            )
            .ok_or(InstalledPackageIndexErrorV1::CounterOverflow)?;
        Ok(())
    }

    fn finish(self) -> RawSeal {
        RawSeal {
            byte_len: self.byte_len,
            sha256: self.hasher.finalize().into(),
        }
    }
}

struct BoundedJsonWriter {
    bytes: Vec<u8>,
    exceeded: bool,
}

impl BoundedJsonWriter {
    fn new() -> Self {
        Self {
            bytes: Vec::with_capacity(8 * 1024),
            exceeded: false,
        }
    }
}

impl Write for BoundedJsonWriter {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        let remaining = MAX_INSTALLED_PACKAGE_INDEX_JSON_BYTES.saturating_sub(self.bytes.len());
        if buffer.len() > remaining {
            self.exceeded = true;
            return Err(io::Error::other(
                "installed package index JSON limit exceeded",
            ));
        }
        self.bytes.extend_from_slice(buffer);
        Ok(buffer.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn serialize_index_bounded(
    index: &InstalledPackageIndex,
) -> Result<String, InstalledPackageIndexErrorV1> {
    let mut writer = BoundedJsonWriter::new();
    if serde_json::to_writer(&mut writer, index).is_err() {
        return if writer.exceeded {
            Err(InstalledPackageIndexErrorV1::IndexJsonLimit {
                limit: MAX_INSTALLED_PACKAGE_INDEX_JSON_BYTES,
            })
        } else {
            Err(InstalledPackageIndexErrorV1::IndexSerialization)
        };
    }
    String::from_utf8(writer.bytes).map_err(|_| InstalledPackageIndexErrorV1::IndexSerialization)
}

fn hex_digest(bytes: &[u8; 32]) -> String {
    let mut output = String::with_capacity(64);
    for byte in bytes {
        use std::fmt::Write as _;
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}

fn truncate_utf8(value: &str, max_bytes: usize) -> String {
    if value.len() <= max_bytes {
        return value.to_owned();
    }
    let mut end = max_bytes;
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    value[..end].to_owned()
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use crate::package_index::PackageIndexStatus;
    use retoc::iostore_writer::IoStoreWriter;
    use retoc::version::EngineVersion;
    use retoc::{EIoChunkType, FIoChunkId, FIoContainerId, FPackageId, UEPath, UEPathBuf};
    use std::collections::BTreeMap;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);
    const EXE_BYTES: &[u8] = b"gore-tex exact installed executable fixture v1";
    const PRIMARY_TARGET: &str = "/Game/Characters/DA_Asghan";

    struct Fixture {
        root: PathBuf,
        install_root: PathBuf,
        game_root: PathBuf,
        paks: PathBuf,
        expected_executable: ExpectedInstalledExecutableV1,
    }

    impl Fixture {
        fn new(label: &str) -> Self {
            let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
            let root = std::env::temp_dir().join(format!(
                "gore-tex-installed-index-{label}-{}-{sequence}",
                std::process::id()
            ));
            let install_root = root.join("install");
            let game_root = install_root.join(G1R_DIRECTORY);
            let paks = game_root.join(PAKS_RELATIVE_PATH);
            let executable = game_root.join(EXECUTABLE_RELATIVE_PATH);
            fs::create_dir_all(&paks).unwrap();
            fs::create_dir_all(executable.parent().unwrap()).unwrap();
            fs::write(&executable, EXE_BYTES).unwrap();
            write_container(
                &paks.join(MAIN_CONTAINER_FILE_NAME),
                PRIMARY_TARGET,
                EIoChunkType::ExportBundleData,
            );
            fs::write(paks.join("G1R-Windows.pak"), b"bounded pak stub").unwrap();
            fs::create_dir_all(paks.join("Notes")).unwrap();
            fs::write(paks.join("Notes/readme.txt"), b"non-mount inventory entry").unwrap();
            Self {
                root,
                install_root,
                game_root,
                paks,
                expected_executable: ExpectedInstalledExecutableV1 {
                    byte_len: EXE_BYTES.len() as u64,
                    sha256: Sha256::digest(EXE_BYTES).into(),
                },
            }
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    fn package_id(target: &str) -> FPackageId {
        FPackageId(FIoContainerId::from_name(target).0)
    }

    fn write_container(path: &Path, target: &str, chunk_type: EIoChunkType) {
        let version = EngineVersion::UE5_4;
        let mut writer = IoStoreWriter::new(
            path,
            version.toc_version(),
            None,
            UEPathBuf::from("../../../"),
        )
        .unwrap();
        let is_export_bundle = chunk_type == EIoChunkType::ExportBundleData;
        let id = FIoChunkId::from_package_id(package_id(target), 0, chunk_type);
        let directory_path = is_export_bundle
            .then(|| UEPath::new("../../../G1R/Content/Characters/DA_Asghan.uasset"));
        writer
            .write_chunk(id, directory_path, b"payload must remain unread")
            .unwrap();
        writer.finalize().unwrap();
    }

    fn tree_bytes(root: &Path) -> BTreeMap<String, Option<Vec<u8>>> {
        let mut output = BTreeMap::new();
        let mut pending = vec![root.to_path_buf()];
        while let Some(directory) = pending.pop() {
            for entry in fs::read_dir(directory).unwrap() {
                let path = entry.unwrap().path();
                let relative = path
                    .strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/");
                if path.is_dir() {
                    output.insert(relative, None);
                    pending.push(path);
                } else {
                    output.insert(relative, Some(fs::read(path).unwrap()));
                }
            }
        }
        output
    }

    fn overwrite_same_length(path: &Path, byte: u8) {
        let length = fs::metadata(path).unwrap().len();
        let mut file = OpenOptions::new().write(true).open(path).unwrap();
        file.seek(SeekFrom::Start(0)).unwrap();
        let buffer = [byte; 64 * 1024];
        let mut remaining = length;
        while remaining > 0 {
            let count = remaining.min(buffer.len() as u64) as usize;
            file.write_all(&buffer[..count]).unwrap();
            remaining -= count as u64;
        }
        file.flush().unwrap();
        assert_eq!(fs::metadata(path).unwrap().len(), length);
    }

    #[test]
    fn exact_install_snapshot_is_read_only_path_free_and_revalidatable() {
        let fixture = Fixture::new("success");
        let before = tree_bytes(&fixture.root);

        let verified =
            inspect_installed_package_index_v1(&fixture.install_root, fixture.expected_executable)
                .unwrap();

        assert_eq!(verified.index().status, PackageIndexStatus::CompleteIndex);
        assert_eq!(verified.index().candidates.len(), 1);
        assert_eq!(verified.index().candidates[0].target_path, PRIMARY_TARGET);
        assert_eq!(verified.mount_inventory_entry_count(), 5);
        assert_eq!(
            verified.target_executable().byte_len,
            EXE_BYTES.len() as u64
        );
        for seal in [
            verified.target_executable(),
            verified.mount_inventory_seal(),
            verified.index_seal(),
            verified.source_snapshot_seal(),
        ] {
            assert_eq!(seal.sha256.len(), 64);
            assert!(seal.sha256.bytes().all(|byte| byte.is_ascii_hexdigit()));
        }
        serde_json::from_str::<serde_json::Value>(verified.index_json()).unwrap();
        verified.revalidate().unwrap();
        assert_eq!(
            tree_bytes(&fixture.root),
            before,
            "inspection must not write"
        );

        let debug = format!("{verified:?}");
        assert!(!debug.contains(&fixture.root.to_string_lossy().to_string()));
        assert!(!debug.contains("platform_identity"));
    }

    #[test]
    fn direct_g1r_root_and_unchanged_install_produce_deterministic_seals() {
        let fixture = Fixture::new("deterministic");
        let first =
            inspect_installed_package_index_v1(&fixture.game_root, fixture.expected_executable)
                .unwrap();
        let second =
            inspect_installed_package_index_v1(&fixture.install_root, fixture.expected_executable)
                .unwrap();
        assert_eq!(first.index_json(), second.index_json());
        assert_eq!(first.mount_inventory_seal(), second.mount_inventory_seal());
        assert_eq!(first.index_seal(), second.index_seal());
        assert_eq!(first.source_snapshot_seal(), second.source_snapshot_seal());
    }

    #[test]
    fn corrupt_export_bundle_storage_is_not_read_by_indexing() {
        let fixture = Fixture::new("unread-export");
        overwrite_same_length(&fixture.paks.join("G1R-Windows.ucas"), 0xa5);

        let verified =
            inspect_installed_package_index_v1(&fixture.install_root, fixture.expected_executable)
                .unwrap();

        assert_eq!(verified.index().candidates[0].target_path, PRIMARY_TARGET);
        verified.revalidate().unwrap();
    }

    #[test]
    fn executable_anchor_mismatch_is_rejected() {
        let fixture = Fixture::new("exe-mismatch");
        let mut expected = fixture.expected_executable;
        expected.sha256[0] ^= 0xff;
        let error =
            inspect_installed_package_index_v1(&fixture.install_root, expected).unwrap_err();
        assert!(matches!(
            error,
            InstalledPackageIndexErrorV1::ExecutableMismatch
        ));
    }

    #[test]
    fn parent_traversal_is_rejected_before_filesystem_resolution() {
        let fixture = Fixture::new("parent-traversal");
        let path = fixture.install_root.join("missing").join("..");
        let error =
            inspect_installed_package_index_v1(&path, fixture.expected_executable).unwrap_err();
        assert!(matches!(
            error,
            InstalledPackageIndexErrorV1::ParentTraversal
        ));
    }

    #[test]
    fn missing_iostore_companion_is_rejected_before_open() {
        let fixture = Fixture::new("missing-pair");
        fs::remove_file(fixture.paks.join("G1R-Windows.ucas")).unwrap();
        let error =
            inspect_installed_package_index_v1(&fixture.install_root, fixture.expected_executable)
                .unwrap_err();
        assert!(matches!(
            error,
            InstalledPackageIndexErrorV1::MountCompanionMissing { .. }
        ));
    }

    #[test]
    fn nested_mountable_and_noncanonical_direct_mount_are_rejected() {
        let nested = Fixture::new("nested-mount");
        fs::write(nested.paks.join("Notes/bad.utoc"), b"not mountable here").unwrap();
        let nested_error =
            inspect_installed_package_index_v1(&nested.install_root, nested.expected_executable)
                .unwrap_err();
        assert!(matches!(
            nested_error,
            InstalledPackageIndexErrorV1::NestedMountable
        ));

        let noncanonical = Fixture::new("noncanonical-mount");
        fs::write(noncanonical.paks.join("Bad.UTOC"), b"uppercase extension").unwrap();
        let name_error = inspect_installed_package_index_v1(
            &noncanonical.install_root,
            noncanonical.expected_executable,
        )
        .unwrap_err();
        assert!(matches!(
            name_error,
            InstalledPackageIndexErrorV1::NoncanonicalMountName { .. }
        ));
    }

    #[test]
    fn ambiguous_priority_is_rejected_before_retoc_open() {
        use std::cell::Cell;

        let fixture = Fixture::new("ambiguous-priority");
        write_container(
            &fixture.paks.join("foo_0_P.utoc"),
            "/Game/A/DA_A",
            EIoChunkType::BulkData,
        );
        write_container(
            &fixture.paks.join("foo_00_P.utoc"),
            "/Game/B/DA_B",
            EIoChunkType::BulkData,
        );
        let before_open_called = Cell::new(false);
        let error = inspect_installed_package_index_with_hooks_v1(
            &fixture.install_root,
            fixture.expected_executable,
            PackageIndexLimits::default(),
            |_| before_open_called.set(true),
            |_| {},
        )
        .unwrap_err();

        assert!(!before_open_called.get());
        assert!(matches!(
            error,
            InstalledPackageIndexErrorV1::ContainerPriority(
                PackageIndexError::AmbiguousContainerPriority { .. }
            )
        ));
    }

    #[test]
    fn tightened_container_limit_is_enforced_before_retoc_open() {
        use std::cell::Cell;

        let fixture = Fixture::new("container-budget");
        write_container(
            &fixture.paks.join("optional.utoc"),
            "/Game/Optional/DA_Optional",
            EIoChunkType::BulkData,
        );
        let limits = PackageIndexLimits::default()
            .with_container_priority_limits(1, 255, 255)
            .unwrap();
        let before_open_called = Cell::new(false);
        let error = inspect_installed_package_index_with_hooks_v1(
            &fixture.install_root,
            fixture.expected_executable,
            limits,
            |_| before_open_called.set(true),
            |_| {},
        )
        .unwrap_err();

        assert!(!before_open_called.get());
        assert!(matches!(
            error,
            InstalledPackageIndexErrorV1::ContainerPriority(
                PackageIndexError::ChildContainerLimit { .. }
            )
        ));
    }

    #[test]
    fn opened_container_set_is_compared_to_preflight() {
        let fixture = Fixture::new("open-race");
        let error = inspect_installed_package_index_with_hooks_v1(
            &fixture.install_root,
            fixture.expected_executable,
            PackageIndexLimits::default(),
            |paks| {
                write_container(
                    &paks.join("late.utoc"),
                    "/Game/Late/DA_Late",
                    EIoChunkType::BulkData,
                );
            },
            |_| {},
        )
        .unwrap_err();

        assert!(matches!(
            error,
            InstalledPackageIndexErrorV1::OpenedContainerSetChanged
        ));
    }

    #[test]
    fn full_postflight_detects_nonmount_file_content_change() {
        let fixture = Fixture::new("postflight-race");
        let error = inspect_installed_package_index_with_hooks_v1(
            &fixture.install_root,
            fixture.expected_executable,
            PackageIndexLimits::default(),
            |_| {},
            |paks| {
                fs::write(
                    paks.join("Notes/readme.txt"),
                    b"changed non-mount content with a different length",
                )
                .unwrap();
            },
        )
        .unwrap_err();

        assert!(matches!(error, InstalledPackageIndexErrorV1::TreeChanged));
    }

    #[test]
    fn public_revalidate_detects_tree_change_after_snapshot() {
        let fixture = Fixture::new("public-revalidate");
        let verified =
            inspect_installed_package_index_v1(&fixture.install_root, fixture.expected_executable)
                .unwrap();
        fs::write(fixture.paks.join("late.txt"), b"late tree member").unwrap();
        let error = verified.revalidate().unwrap_err();
        assert!(matches!(
            error,
            InstalledPackageIndexErrorV1::TreeChanged
                | InstalledPackageIndexErrorV1::SourceChanged {
                    role: "Paks directory"
                }
        ));
    }

    #[test]
    fn mount_content_change_changes_domain_separated_source_seal() {
        let fixture = Fixture::new("seal-change");
        let first =
            inspect_installed_package_index_v1(&fixture.install_root, fixture.expected_executable)
                .unwrap();
        let first_seal = first.source_snapshot_seal().clone();
        drop(first);

        fs::write(fixture.paks.join("G1R-Windows.pak"), b"different pak bytes").unwrap();
        let second =
            inspect_installed_package_index_v1(&fixture.install_root, fixture.expected_executable)
                .unwrap();
        assert_ne!(&first_seal, second.source_snapshot_seal());
    }

    #[test]
    fn symlink_or_reparse_game_root_is_rejected_when_supported() {
        let fixture = Fixture::new("symlink-root");
        let link = fixture.root.join("linked-install");
        if !create_directory_link(&fixture.install_root, &link) {
            return;
        }
        let error =
            inspect_installed_package_index_v1(&link, fixture.expected_executable).unwrap_err();
        assert!(matches!(
            error,
            InstalledPackageIndexErrorV1::UnsafePath { .. }
        ));
    }

    #[cfg(windows)]
    fn create_directory_link(target: &Path, link: &Path) -> bool {
        std::os::windows::fs::symlink_dir(target, link).is_ok()
    }

    #[cfg(unix)]
    fn create_directory_link(target: &Path, link: &Path) -> bool {
        std::os::unix::fs::symlink(target, link).is_ok()
    }
}

#[cfg(all(test, unix))]
mod unix_tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static MISSING_SEQUENCE: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn v1_rejects_unix_before_anchor_validation_or_tree_access_and_writes_nothing() {
        let sequence = MISSING_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let missing = std::env::temp_dir().join(format!(
            "gore-tex-v1-unix-must-stay-missing-{}-{sequence}",
            std::process::id()
        ));
        assert!(!missing.exists());

        let invalid_anchor = ExpectedInstalledExecutableV1 {
            byte_len: 0,
            sha256: [0; 32],
        };
        assert!(matches!(
            inspect_installed_package_index_v1(&missing, invalid_anchor),
            Err(InstalledPackageIndexErrorV1::UnsupportedPlatform)
        ));
        assert!(!missing.exists());

        let syntactically_valid_anchor = ExpectedInstalledExecutableV1 {
            byte_len: 1,
            sha256: [1; 32],
        };
        assert!(matches!(
            inspect_installed_package_index_with_limits_v1(
                &missing,
                syntactically_valid_anchor,
                PackageIndexLimits::default(),
            ),
            Err(InstalledPackageIndexErrorV1::UnsupportedPlatform)
        ));
        assert!(!missing.exists());
    }
}
