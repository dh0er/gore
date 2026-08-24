use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File, OpenOptions};
use std::io::Read as _;
use std::path::{Component, Path, PathBuf};

use anyhow::{bail, Context as _, Result};
use gore_as::compiler_profile::capture::promote_unqualified_profile_package_v1;
use gore_as::compiler_profile::qualification_runner::CompilerProbeBackendKindV1;
use gore_as::compiler_profile::qualification_suite::{
    full_qualification_corpus_v1, offline_artifact_authority_summary_from_manifest_json_v1,
    promote_generated_offline_qualification_artifacts_v1,
    reload_generated_offline_qualification_artifacts_v1, GeneratedOfflineCompilerProbeArtifactsV1,
};

const MAX_MANIFEST_BYTES: u64 = 32 * 1024 * 1024;
const USAGE: &str = "usage: gore-as-promote-qualified-profile \
<unqualified-profile-root> <embedded-artifacts-root> \
<standalone-artifacts-root> <new-qualified-profile-root>";

fn main() {
    if let Err(error) = run() {
        eprintln!("qualified profile promotion failed: {error:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let mut args = std::env::args_os().skip(1).map(PathBuf::from);
    let source_profile_root = args.next().context(USAGE)?;
    let embedded_root = args.next().context(USAGE)?;
    let standalone_root = args.next().context(USAGE)?;
    let output_root = args.next().context(USAGE)?;
    if args.next().is_some() {
        bail!(USAGE);
    }
    for (label, path) in [
        ("source profile root", &source_profile_root),
        ("embedded artifacts root", &embedded_root),
        ("standalone artifacts root", &standalone_root),
        ("qualified output root", &output_root),
    ] {
        require_absolute_normalized(path, label)?;
    }
    if output_root.exists() {
        bail!(
            "qualified output root already exists: {}",
            output_root.display()
        );
    }

    let corpus = full_qualification_corpus_v1().context("building canonical corpus")?;
    let embedded = load_artifacts(
        &corpus,
        CompilerProbeBackendKindV1::EmbeddedGame,
        &embedded_root,
        "embedded-qualification-artifacts.json",
    )
    .context("reloading embedded qualification authority")?;
    let standalone = load_artifacts(
        &corpus,
        CompilerProbeBackendKindV1::Standalone,
        &standalone_root,
        "standalone-qualification-artifacts.json",
    )
    .context("reloading standalone qualification authority")?;
    let promotion =
        promote_generated_offline_qualification_artifacts_v1(&corpus, &embedded, &standalone)
            .context("requiring exact embedded/standalone differential parity")?;
    let materialized = promote_unqualified_profile_package_v1(
        &source_profile_root,
        &output_root,
        &corpus,
        &promotion,
    )
    .context("materializing qualified compiler profile")?;
    println!(
        "{{\"output_root\":{},\"profile_sha256\":{},\"sidecar_sha256\":{},\"qualified\":true}}",
        serde_json::to_string(&output_root)?,
        serde_json::to_string(&materialized.profile_sha256)?,
        serde_json::to_string(&promotion.standalone_compiler().sha256)?,
    );
    Ok(())
}

fn load_artifacts(
    corpus: &gore_as::compiler_profile::qualification::CompilerProbeCorpusV1,
    expected_backend: CompilerProbeBackendKindV1,
    root: &Path,
    manifest_name: &str,
) -> Result<GeneratedOfflineCompilerProbeArtifactsV1> {
    let _root_pin = require_real_directory(root, "artifact root")?;
    let manifest_path = root.join(manifest_name);
    let manifest_json = read_regular_file_exact(&manifest_path, Some(MAX_MANIFEST_BYTES))
        .with_context(|| format!("reading {}", manifest_path.display()))?;
    let authority = offline_artifact_authority_summary_from_manifest_json_v1(&manifest_json)
        .context("validating artifact manifest authority")?;
    if authority.backend != expected_backend {
        bail!("artifact manifest backend does not match its input lane");
    }
    let mut blob_names = BTreeSet::new();
    let mut cache_blobs = BTreeMap::new();
    for seal in authority.cache_seals {
        let name = seal.cache.blob_id;
        if !blob_names.insert(name.clone()) {
            bail!("artifact manifest repeats cache blob {name:?}");
        }
        require_plain_file_name(&name)?;
        let path = root.join(&name);
        let bytes = read_regular_file_exact(&path, Some(seal.cache.byte_len))
            .with_context(|| format!("reading sealed cache blob {name:?}"))?;
        if bytes.len() as u64 != seal.cache.byte_len {
            bail!("sealed cache blob {name:?} has the wrong byte length");
        }
        cache_blobs.insert(name, bytes);
    }
    validate_artifact_root_shape(root, manifest_name, expected_backend, &blob_names)?;
    reload_generated_offline_qualification_artifacts_v1(
        corpus,
        expected_backend,
        &manifest_json,
        cache_blobs,
    )
    .context("strictly reloading generated qualification artifacts")
}

fn validate_artifact_root_shape(
    root: &Path,
    manifest_name: &str,
    backend: CompilerProbeBackendKindV1,
    blob_names: &BTreeSet<String>,
) -> Result<()> {
    let mut expected_files = blob_names.clone();
    expected_files.insert(manifest_name.to_owned());
    let expected_directories: BTreeSet<&str> = match backend {
        CompilerProbeBackendKindV1::EmbeddedGame => {
            ["scratch", "invoke-observer-scratch"].into_iter().collect()
        }
        CompilerProbeBackendKindV1::Standalone => ["scratch"].into_iter().collect(),
    };
    let mut observed_files = BTreeSet::new();
    let mut observed_directories = BTreeSet::new();
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| anyhow::anyhow!("artifact root contains a non-UTF-8 entry"))?;
        let metadata = fs::symlink_metadata(entry.path())?;
        if metadata_is_reparse(&metadata) {
            bail!("artifact root contains a reparse entry: {name:?}");
        }
        if metadata.is_file() {
            if !observed_files.insert(name.clone()) || !expected_files.contains(&name) {
                bail!("artifact root contains an unexpected regular file: {name:?}");
            }
        } else if metadata.is_dir() {
            if !expected_directories.contains(name.as_str()) {
                bail!("artifact root contains an unexpected directory: {name:?}");
            }
            if fs::read_dir(entry.path())?.next().is_some() {
                bail!("artifact scratch directory is not empty: {name:?}");
            }
            observed_directories.insert(name);
        } else {
            bail!("artifact root contains an unsupported entry: {name:?}");
        }
    }
    if observed_files != expected_files
        || observed_directories.len() != expected_directories.len()
        || !expected_directories
            .iter()
            .all(|name| observed_directories.contains(*name))
    {
        bail!("artifact root does not exactly match its sealed output shape");
    }
    Ok(())
}

fn read_regular_file_exact(path: &Path, maximum: Option<u64>) -> Result<Vec<u8>> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.file_type().is_file() {
        bail!("path is not a regular file: {}", path.display());
    }
    if maximum.is_some_and(|limit| metadata.len() > limit) {
        bail!("file exceeds its maximum sealed length: {}", path.display());
    }
    let mut file = open_regular_no_follow(path)?;
    let opened = file.metadata()?;
    if !opened.is_file() || opened.len() != metadata.len() {
        bail!("file changed while opening: {}", path.display());
    }
    let capacity = usize::try_from(opened.len()).context("file is too large to address")?;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(capacity)
        .context("allocating file buffer")?;
    file.read_to_end(&mut bytes)?;
    if bytes.len() as u64 != opened.len() || file.metadata()?.len() != opened.len() {
        bail!("file changed while reading: {}", path.display());
    }
    Ok(bytes)
}

fn require_real_directory(path: &Path, label: &str) -> Result<File> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.file_type().is_dir() {
        bail!("{label} is not a real directory: {}", path.display());
    }
    open_directory_no_follow(path).with_context(|| format!("pinning {label} {}", path.display()))
}

#[cfg(windows)]
fn open_regular_no_follow(path: &Path) -> Result<File> {
    use std::os::windows::fs::{MetadataExt as _, OpenOptionsExt as _};
    use windows_sys::Win32::Storage::FileSystem::{
        FILE_ATTRIBUTE_REPARSE_POINT, FILE_FLAG_OPEN_REPARSE_POINT, FILE_SHARE_READ,
    };

    let file = OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
        .open(path)?;
    let metadata = file.metadata()?;
    if !metadata.is_file() || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
        bail!("path is not a non-reparse regular file: {}", path.display());
    }
    Ok(file)
}

#[cfg(not(windows))]
fn open_regular_no_follow(path: &Path) -> Result<File> {
    let file = OpenOptions::new().read(true).open(path)?;
    if !file.metadata()?.is_file() {
        bail!("path is not a regular file: {}", path.display());
    }
    Ok(file)
}

#[cfg(windows)]
fn open_directory_no_follow(path: &Path) -> Result<File> {
    use std::os::windows::fs::{MetadataExt as _, OpenOptionsExt as _};
    use windows_sys::Win32::Storage::FileSystem::{
        FILE_ATTRIBUTE_REPARSE_POINT, FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT,
        FILE_GENERIC_READ, FILE_SHARE_READ,
    };

    let mut options = OpenOptions::new();
    options
        .access_mode(FILE_GENERIC_READ)
        .share_mode(FILE_SHARE_READ)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT);
    let file = options.open(path)?;
    let metadata = file.metadata()?;
    if !metadata.is_dir() || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
        bail!("path is not a non-reparse directory: {}", path.display());
    }
    Ok(file)
}

#[cfg(not(windows))]
fn open_directory_no_follow(path: &Path) -> Result<File> {
    let file = File::open(path)?;
    if !file.metadata()?.is_dir() {
        bail!("path is not a directory: {}", path.display());
    }
    Ok(file)
}

fn require_plain_file_name(value: &str) -> Result<()> {
    let mut components = Path::new(value).components();
    if !matches!(components.next(), Some(Component::Normal(_))) || components.next().is_some() {
        bail!("artifact cache blob id is not a plain file name: {value:?}");
    }
    Ok(())
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

fn require_absolute_normalized(path: &Path, label: &str) -> Result<()> {
    if !path.is_absolute()
        || path
            .components()
            .any(|component| matches!(component, Component::CurDir | Component::ParentDir))
    {
        bail!(
            "{label} path must be absolute and normalized: {}",
            path.display()
        );
    }
    Ok(())
}
