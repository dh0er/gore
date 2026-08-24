//! Read-only planning of a complete authored source tree against one sealed base cache.
//!
//! Every `.as` file in `source_root` is authoritative: an identity already present in the base is
//! an Edit, a new identity is an Add, and every base identity absent from the tree is a Delete.
//! The planner reads all source bytes into the returned value, so compilation never reopens the
//! caller's tree. It does not launch the game or mutate the installation.

use std::collections::{BTreeMap, BTreeSet};
use std::io::Read;
use std::path::{Component, Path, PathBuf};

use crate::compile::{
    base_full_graph_manifest_v1, FullGraphCompileChangeV1, FullGraphCompileOperationV1,
    FullGraphFinalModuleV1, MAX_FULL_GRAPH_COMPILE_CHANGES_V1, MAX_FULL_GRAPH_FINAL_MODULES_V1,
    MAX_FULL_GRAPH_SOURCE_BYTES_V1,
};

const MAX_FULL_GRAPH_SOURCE_FILE_BYTES_V1: u64 = 16 * 1024 * 1024;
const MAX_FULL_GRAPH_SOURCE_PATH_BYTES_V1: usize = 4 * 1024;

#[derive(Debug)]
pub struct PlannedFullGraphSourceTreeV1 {
    changes: Vec<FullGraphCompileChangeV1>,
    final_manifest: Vec<FullGraphFinalModuleV1>,
}

impl PlannedFullGraphSourceTreeV1 {
    pub fn changes(&self) -> &[FullGraphCompileChangeV1] {
        &self.changes
    }

    pub fn final_manifest(&self) -> &[FullGraphFinalModuleV1] {
        &self.final_manifest
    }

    pub fn into_parts(self) -> (Vec<FullGraphCompileChangeV1>, Vec<FullGraphFinalModuleV1>) {
        (self.changes, self.final_manifest)
    }
}

#[derive(Debug)]
struct AuthoredSourceV1 {
    module_name: String,
    relative_path: String,
    bytes: Vec<u8>,
}

/// Build an exact Add/Edit/Delete plan for `source_root` without writing anywhere.
pub fn plan_complete_source_tree_v1(
    base_cache: &[u8],
    source_root: &Path,
) -> Result<PlannedFullGraphSourceTreeV1, FullGraphSourcePlanErrorV1> {
    let base = base_full_graph_manifest_v1(base_cache)
        .map_err(|error| FullGraphSourcePlanErrorV1::BaseCache(error.to_string()))?
        .into_iter()
        .map(|entry| (entry.module_name, entry.relative_path))
        .collect::<Vec<_>>();
    let sources = collect_authored_sources(source_root)?;
    plan_inventory(base, sources)
}

fn plan_inventory(
    base: Vec<(String, String)>,
    mut sources: Vec<AuthoredSourceV1>,
) -> Result<PlannedFullGraphSourceTreeV1, FullGraphSourcePlanErrorV1> {
    if sources.is_empty() {
        return Err(FullGraphSourcePlanErrorV1::EmptySourceTree);
    }
    sources.sort_by(|left, right| {
        canonical_identity(&left.module_name, &left.relative_path).cmp(&canonical_identity(
            &right.module_name,
            &right.relative_path,
        ))
    });
    let mut base_by_name = BTreeMap::new();
    for (module_name, relative_path) in base {
        if base_by_name
            .insert(fold(&module_name), (module_name, relative_path))
            .is_some()
        {
            return Err(FullGraphSourcePlanErrorV1::BaseCache(
                "base cache contains case-colliding module names".into(),
            ));
        }
    }

    let mut seen_names = BTreeSet::new();
    let mut seen_paths = BTreeSet::new();
    let mut final_manifest = Vec::with_capacity(sources.len());
    let mut changes = Vec::with_capacity(sources.len().saturating_add(base_by_name.len()));
    for source in sources {
        if !seen_names.insert(fold(&source.module_name))
            || !seen_paths.insert(fold(&source.relative_path))
        {
            return Err(FullGraphSourcePlanErrorV1::IdentityCollision {
                module_name: source.module_name,
                relative_path: source.relative_path,
            });
        }
        let operation = match base_by_name.remove(&fold(&source.module_name)) {
            Some((base_name, base_path)) => {
                if base_name != source.module_name || base_path != source.relative_path {
                    return Err(FullGraphSourcePlanErrorV1::BaseIdentityMismatch {
                        source_module: source.module_name,
                        source_path: source.relative_path,
                        base_module: base_name,
                        base_path,
                    });
                }
                FullGraphCompileOperationV1::Edit
            }
            None => FullGraphCompileOperationV1::Add,
        };
        final_manifest.push(FullGraphFinalModuleV1 {
            module_name: source.module_name.clone(),
            relative_path: source.relative_path.clone(),
        });
        changes.push(FullGraphCompileChangeV1 {
            operation,
            module_name: source.module_name,
            relative_path: source.relative_path,
            source: Some(source.bytes),
        });
    }

    for (_, (module_name, relative_path)) in base_by_name {
        changes.push(FullGraphCompileChangeV1 {
            operation: FullGraphCompileOperationV1::Delete,
            module_name,
            relative_path,
            source: None,
        });
    }
    changes.sort_by(|left, right| {
        canonical_identity(&left.module_name, &left.relative_path).cmp(&canonical_identity(
            &right.module_name,
            &right.relative_path,
        ))
    });
    if changes.len() > MAX_FULL_GRAPH_COMPILE_CHANGES_V1
        || final_manifest.len() > MAX_FULL_GRAPH_FINAL_MODULES_V1
    {
        return Err(FullGraphSourcePlanErrorV1::TooManyModules {
            changes: changes.len(),
            final_modules: final_manifest.len(),
        });
    }
    Ok(PlannedFullGraphSourceTreeV1 {
        changes,
        final_manifest,
    })
}

fn collect_authored_sources(
    root: &Path,
) -> Result<Vec<AuthoredSourceV1>, FullGraphSourcePlanErrorV1> {
    if !root.is_absolute() {
        return Err(FullGraphSourcePlanErrorV1::UnsafeRoot);
    }
    let mut directory_pins = Vec::new();
    directory_pins.push(open_directory_pin(root)?);
    let mut pending = vec![PathBuf::new()];
    let mut paths = Vec::new();
    while let Some(relative_directory) = pending.pop() {
        let directory = root.join(&relative_directory);
        let mut entries = std::fs::read_dir(&directory)
            .map_err(|error| FullGraphSourcePlanErrorV1::Io(error.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| FullGraphSourcePlanErrorV1::Io(error.to_string()))?;
        entries.sort_by_key(std::fs::DirEntry::file_name);
        for entry in entries.into_iter().rev() {
            let relative = relative_directory.join(entry.file_name());
            let metadata = std::fs::symlink_metadata(entry.path())
                .map_err(|error| FullGraphSourcePlanErrorV1::Io(error.to_string()))?;
            if metadata.file_type().is_symlink() || metadata_is_reparse(&metadata) {
                return Err(FullGraphSourcePlanErrorV1::UnsafeEntry(relative));
            }
            if metadata.is_dir() {
                directory_pins.push(open_directory_pin(&entry.path())?);
                pending.push(relative);
            } else if metadata.is_file() {
                if is_angelscript_path(&relative) {
                    paths.push(relative);
                    if paths.len() > MAX_FULL_GRAPH_FINAL_MODULES_V1 {
                        return Err(FullGraphSourcePlanErrorV1::TooManyModules {
                            changes: paths.len(),
                            final_modules: paths.len(),
                        });
                    }
                }
            } else {
                return Err(FullGraphSourcePlanErrorV1::UnsafeEntry(relative));
            }
        }
    }
    let mut normalized_paths = paths
        .into_iter()
        .map(|path| {
            let normalized = path_to_slashes(&path)?;
            Ok((path, normalized))
        })
        .collect::<Result<Vec<_>, FullGraphSourcePlanErrorV1>>()?;
    normalized_paths.sort_by(|left, right| left.1.cmp(&right.1));

    let mut total = 0u64;
    let mut sources = Vec::with_capacity(normalized_paths.len());
    for (relative, relative_path) in normalized_paths {
        validate_relative_path(&relative_path)?;
        let mut file = open_source_pin(&root.join(&relative))?;
        let metadata = file
            .metadata()
            .map_err(|error| FullGraphSourcePlanErrorV1::Io(error.to_string()))?;
        if metadata.len() > MAX_FULL_GRAPH_SOURCE_FILE_BYTES_V1 {
            return Err(FullGraphSourcePlanErrorV1::SourceTooLarge {
                path: relative,
                actual: metadata.len(),
                max: MAX_FULL_GRAPH_SOURCE_FILE_BYTES_V1,
            });
        }
        let mut bytes = Vec::with_capacity(metadata.len() as usize);
        (&mut file)
            .take(MAX_FULL_GRAPH_SOURCE_FILE_BYTES_V1 + 1)
            .read_to_end(&mut bytes)
            .map_err(|error| FullGraphSourcePlanErrorV1::Io(error.to_string()))?;
        if bytes.len() as u64 != metadata.len() {
            return Err(FullGraphSourcePlanErrorV1::SourceChanged(relative));
        }
        total = total.checked_add(bytes.len() as u64).ok_or(
            FullGraphSourcePlanErrorV1::AggregateTooLarge {
                actual: u64::MAX,
                max: MAX_FULL_GRAPH_SOURCE_BYTES_V1 as u64,
            },
        )?;
        if total > MAX_FULL_GRAPH_SOURCE_BYTES_V1 as u64 {
            return Err(FullGraphSourcePlanErrorV1::AggregateTooLarge {
                actual: total,
                max: MAX_FULL_GRAPH_SOURCE_BYTES_V1 as u64,
            });
        }
        std::str::from_utf8(&bytes)
            .map_err(|_| FullGraphSourcePlanErrorV1::InvalidUtf8(relative.clone()))?;
        if bytes.contains(&0) {
            return Err(FullGraphSourcePlanErrorV1::NulSource(relative));
        }
        sources.push(AuthoredSourceV1 {
            module_name: module_name_from_relative_path_v1(&relative_path)?,
            relative_path,
            bytes,
        });
    }
    drop(directory_pins);
    Ok(sources)
}

/// Applies the exact donor module-name rule after enforcing this planner's path policy.
pub fn module_name_from_relative_path_v1(
    relative_path: &str,
) -> Result<String, FullGraphSourcePlanErrorV1> {
    validate_relative_path(relative_path)?;
    let (module_name, normalized_path) =
        crate::compile::module_name_from_relative_path_v1(relative_path)
            .map_err(|error| FullGraphSourcePlanErrorV1::InvalidModuleName(error.to_string()))?;
    if normalized_path != relative_path {
        return Err(FullGraphSourcePlanErrorV1::UnsafeRelativePath(
            relative_path.into(),
        ));
    }
    Ok(module_name)
}

fn validate_relative_path(value: &str) -> Result<(), FullGraphSourcePlanErrorV1> {
    if value.is_empty()
        || value.len() > MAX_FULL_GRAPH_SOURCE_PATH_BYTES_V1
        || value.contains('\\')
        || value.contains(':')
        || value.contains('\0')
        || value.chars().any(char::is_control)
    {
        return Err(FullGraphSourcePlanErrorV1::UnsafeRelativePath(value.into()));
    }
    let path = Path::new(value);
    if path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
        || value.split('/').any(|component| {
            component.is_empty()
                || component.ends_with([' ', '.'])
                || component
                    .chars()
                    .any(|character| matches!(character, '<' | '>' | '"' | '|' | '?' | '*'))
                || windows_reserved_component(component)
        })
    {
        return Err(FullGraphSourcePlanErrorV1::UnsafeRelativePath(value.into()));
    }
    Ok(())
}

fn windows_reserved_component(component: &str) -> bool {
    let stem = component.split('.').next().unwrap_or(component);
    let upper = stem.to_ascii_uppercase();
    matches!(upper.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || upper
            .strip_prefix("COM")
            .or_else(|| upper.strip_prefix("LPT"))
            .is_some_and(|suffix| suffix.len() == 1 && matches!(suffix.as_bytes()[0], b'1'..=b'9'))
}

fn is_angelscript_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("as"))
}

fn path_to_slashes(path: &Path) -> Result<String, FullGraphSourcePlanErrorV1> {
    path.components()
        .map(|component| match component {
            Component::Normal(value) => value
                .to_str()
                .map(str::to_owned)
                .ok_or_else(|| FullGraphSourcePlanErrorV1::UnsafeEntry(path.to_path_buf())),
            _ => Err(FullGraphSourcePlanErrorV1::UnsafeEntry(path.to_path_buf())),
        })
        .collect::<Result<Vec<_>, _>>()
        .map(|components| components.join("/"))
}

fn canonical_identity(module_name: &str, relative_path: &str) -> (String, String, String, String) {
    (
        fold(module_name),
        fold(relative_path),
        module_name.to_owned(),
        relative_path.to_owned(),
    )
}

fn fold(value: &str) -> String {
    value.chars().flat_map(char::to_lowercase).collect()
}

#[cfg(windows)]
fn open_directory_pin(path: &Path) -> Result<std::fs::File, FullGraphSourcePlanErrorV1> {
    use std::os::windows::fs::OpenOptionsExt as _;
    use windows_sys::Win32::Storage::FileSystem::{
        FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT, FILE_GENERIC_READ,
        FILE_SHARE_READ,
    };

    let mut options = std::fs::OpenOptions::new();
    options
        .access_mode(FILE_GENERIC_READ)
        .share_mode(FILE_SHARE_READ)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT);
    let file = options
        .open(path)
        .map_err(|error| FullGraphSourcePlanErrorV1::Io(error.to_string()))?;
    let metadata = file
        .metadata()
        .map_err(|error| FullGraphSourcePlanErrorV1::Io(error.to_string()))?;
    if !metadata.is_dir() || metadata_is_reparse(&metadata) {
        return Err(FullGraphSourcePlanErrorV1::UnsafeRoot);
    }
    Ok(file)
}

#[cfg(not(windows))]
fn open_directory_pin(path: &Path) -> Result<std::fs::File, FullGraphSourcePlanErrorV1> {
    let metadata = std::fs::symlink_metadata(path)
        .map_err(|error| FullGraphSourcePlanErrorV1::Io(error.to_string()))?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(FullGraphSourcePlanErrorV1::UnsafeRoot);
    }
    std::fs::File::open(path).map_err(|error| FullGraphSourcePlanErrorV1::Io(error.to_string()))
}

#[cfg(windows)]
fn open_source_pin(path: &Path) -> Result<std::fs::File, FullGraphSourcePlanErrorV1> {
    use std::os::windows::fs::OpenOptionsExt as _;
    use windows_sys::Win32::Storage::FileSystem::{
        FILE_FLAG_OPEN_REPARSE_POINT, FILE_GENERIC_READ, FILE_SHARE_READ,
    };
    let mut options = std::fs::OpenOptions::new();
    options
        .access_mode(FILE_GENERIC_READ)
        .share_mode(FILE_SHARE_READ)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT);
    let file = options
        .open(path)
        .map_err(|error| FullGraphSourcePlanErrorV1::Io(error.to_string()))?;
    let metadata = file
        .metadata()
        .map_err(|error| FullGraphSourcePlanErrorV1::Io(error.to_string()))?;
    if !metadata.is_file() || metadata_is_reparse(&metadata) {
        return Err(FullGraphSourcePlanErrorV1::UnsafeEntry(path.to_path_buf()));
    }
    Ok(file)
}

#[cfg(unix)]
fn open_source_pin(path: &Path) -> Result<std::fs::File, FullGraphSourcePlanErrorV1> {
    use std::os::unix::fs::OpenOptionsExt as _;
    let mut options = std::fs::OpenOptions::new();
    options
        .read(true)
        .custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW);
    let file = options
        .open(path)
        .map_err(|error| FullGraphSourcePlanErrorV1::Io(error.to_string()))?;
    if !file
        .metadata()
        .map_err(|error| FullGraphSourcePlanErrorV1::Io(error.to_string()))?
        .is_file()
    {
        return Err(FullGraphSourcePlanErrorV1::UnsafeEntry(path.to_path_buf()));
    }
    Ok(file)
}

#[cfg(not(any(windows, unix)))]
fn open_source_pin(path: &Path) -> Result<std::fs::File, FullGraphSourcePlanErrorV1> {
    let metadata = std::fs::symlink_metadata(path)
        .map_err(|error| FullGraphSourcePlanErrorV1::Io(error.to_string()))?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err(FullGraphSourcePlanErrorV1::UnsafeEntry(path.to_path_buf()));
    }
    std::fs::File::open(path).map_err(|error| FullGraphSourcePlanErrorV1::Io(error.to_string()))
}

#[cfg(windows)]
fn metadata_is_reparse(metadata: &std::fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt as _;
    use windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_REPARSE_POINT;
    metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

#[cfg(not(windows))]
fn metadata_is_reparse(_: &std::fs::Metadata) -> bool {
    false
}

#[derive(Debug, thiserror::Error)]
pub enum FullGraphSourcePlanErrorV1 {
    #[error("base cache cannot supply a canonical module manifest: {0}")]
    BaseCache(String),
    #[error("full-graph source root must be an absolute real directory")]
    UnsafeRoot,
    #[error("full-graph source tree contains an unsafe entry at {0}")]
    UnsafeEntry(PathBuf),
    #[error("full-graph source path is unsafe: {0:?}")]
    UnsafeRelativePath(String),
    #[error("full-graph source tree contains no .as modules")]
    EmptySourceTree,
    #[error("full-graph source tree has {changes} changes and {final_modules} final modules")]
    TooManyModules {
        changes: usize,
        final_modules: usize,
    },
    #[error("full-graph source {path:?} is {actual} bytes; maximum is {max}")]
    SourceTooLarge {
        path: PathBuf,
        actual: u64,
        max: u64,
    },
    #[error("full-graph source aggregate is {actual} bytes; maximum is {max}")]
    AggregateTooLarge { actual: u64, max: u64 },
    #[error("full-graph source changed while it was read: {0}")]
    SourceChanged(PathBuf),
    #[error("full-graph source is not canonical UTF-8: {0}")]
    InvalidUtf8(PathBuf),
    #[error("full-graph source contains NUL: {0}")]
    NulSource(PathBuf),
    #[error("relative path derives an invalid module name: {0:?}")]
    InvalidModuleName(String),
    #[error("full-graph source identities collide at {module_name:?}/{relative_path:?}")]
    IdentityCollision {
        module_name: String,
        relative_path: String,
    },
    #[error(
        "source identity {source_module:?}/{source_path:?} does not exactly match base identity {base_module:?}/{base_path:?}"
    )]
    BaseIdentityMismatch {
        source_module: String,
        source_path: String,
        base_module: String,
        base_path: String,
    },
    #[error("reading full-graph source tree: {0}")]
    Io(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source(path: &str, bytes: &[u8]) -> AuthoredSourceV1 {
        AuthoredSourceV1 {
            module_name: module_name_from_relative_path_v1(path).unwrap(),
            relative_path: path.into(),
            bytes: bytes.into(),
        }
    }

    #[test]
    fn donor_names_and_complete_add_edit_delete_plan_are_exact() {
        assert_eq!(
            module_name_from_relative_path_v1("AI/Foo.as.generated.as").unwrap(),
            "AI.Foo.generated"
        );
        let planned = plan_inventory(
            vec![
                ("AI.Foo".into(), "AI/Foo.as".into()),
                ("Old".into(), "Old.as".into()),
            ],
            vec![source("AI/Foo.as", b"edit"), source("New.as", b"add")],
        )
        .unwrap();
        assert_eq!(planned.final_manifest.len(), 2);
        assert_eq!(planned.changes.len(), 3);
        assert!(planned.changes.iter().any(|change| {
            change.module_name == "AI.Foo" && change.operation == FullGraphCompileOperationV1::Edit
        }));
        assert!(planned.changes.iter().any(|change| {
            change.module_name == "New" && change.operation == FullGraphCompileOperationV1::Add
        }));
        assert!(planned.changes.iter().any(|change| {
            change.module_name == "Old"
                && change.operation == FullGraphCompileOperationV1::Delete
                && change.source.is_none()
        }));
    }

    #[test]
    fn case_or_path_drift_is_never_reclassified_as_an_edit() {
        let error = plan_inventory(
            vec![("AI.Foo".into(), "AI/Foo.as".into())],
            vec![source("ai/foo.as", b"drift")],
        )
        .unwrap_err();
        assert!(matches!(
            error,
            FullGraphSourcePlanErrorV1::BaseIdentityMismatch { .. }
        ));
    }
}
