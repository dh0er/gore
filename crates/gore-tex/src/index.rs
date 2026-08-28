//! Cached texture index: asset_path -> package_id, for instant search + scan-free extract.

use std::collections::BTreeMap;
use std::fs::{File, OpenOptions};
use std::io::{Cursor, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

use serde::{Deserialize, Serialize};

use retoc::script_objects::FPackageObjectIndex;
use retoc::zen::FZenPackageHeader;
use retoc::{EIoChunkType, FIoChunkId};

use crate::error::Result;

const SOURCE_FINGERPRINT_DOMAIN: &[u8] = b"gore-tex.installed-texture-composite.v3\0";
const SOURCE_FILE_DIGEST_CACHE_FORMAT: &str = "gore-texture-source-file-digest-cache";
const SOURCE_FILE_DIGEST_CACHE_VERSION: u32 = 1;
const MAX_SOURCE_FILES: usize = 513;
const MAX_SOURCE_FINGERPRINT_CACHE_ENTRIES: usize = 8;
const MAX_SOURCE_FILE_DIGEST_CACHE_ENTRIES: usize = 64;
const MAX_SOURCE_FILE_DIGEST_CACHE_BYTES: u64 = 8 * 1024;
const MAX_PERSISTED_SOURCE_FILE_DIGESTS: usize = 1024;
const MAX_MANAGED_TEXTURE_INDEX_ENTRIES: usize = 65_536;
const MAX_MANAGED_TEXTURE_ASSET_PATH_BYTES: usize = 1024;
const MAX_MANAGED_TEXTURE_INDEX_BYTES: u64 = 64 * 1024 * 1024;
const MAX_MANAGED_TEXTURE_INDEX_FILES: usize = 8;
const MAX_PACKAGE_ID_DECIMAL_BYTES: usize = 20;
static INDEX_TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);
#[cfg(test)]
static SOURCE_FILE_HASHED_BYTES: AtomicU64 = AtomicU64::new(0);
static SOURCE_FINGERPRINT_CACHE: OnceLock<Mutex<Vec<(SourceIdentity, String)>>> = OnceLock::new();
static SOURCE_FILE_DIGEST_CACHE: OnceLock<Mutex<Vec<(FileIdentity, String)>>> = OnceLock::new();

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct FileIdentity {
    byte_len: u64,
    modified_stamp: String,
    change_stamp: String,
    reliable_change_token: bool,
    platform_identity: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
struct SourceBindingIdentity {
    role: String,
    file: FileIdentity,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
struct SourceIdentity(Vec<SourceBindingIdentity>);

#[derive(Debug, Eq, PartialEq, Serialize)]
struct QuickSourceBindingIdentity {
    role: String,
    file: FileIdentity,
    unreliable_content_blake3: Option<String>,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
struct QuickSourceIdentity(Vec<QuickSourceBindingIdentity>);

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PersistedSourceFileDigest {
    format: String,
    version: u32,
    identity: FileIdentity,
    digest: String,
}

/// Maps each Texture2D asset path to its IoStore package id (u64). Built once per game
/// build (a full container scan); cached to the shared gore dir.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct TextureIndex {
    /// Identifies the game build the index was built against (the .usmap filename), so a
    /// game update invalidates a stale cache.
    pub build_id: String,
    /// asset_path -> package_id
    pub entries: BTreeMap<String, u64>,
}

impl TextureIndex {
    pub fn to_json(&self) -> Result<Vec<u8>> {
        serde_json::to_vec(self).map_err(|e| {
            crate::error::TexError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                e.to_string(),
            ))
        })
    }
    pub fn from_json(bytes: &[u8]) -> Result<Self> {
        serde_json::from_slice(bytes).map_err(|e| {
            crate::error::TexError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                e.to_string(),
            ))
        })
    }
    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(p) = path.parent() {
            std::fs::create_dir_all(p)?;
        }
        std::fs::write(path, self.to_json()?)?;
        Ok(())
    }

    /// Publish one immutable cache entry without ever exposing a partial JSON file.
    /// Existing content is accepted only when it is the same logical index.
    pub fn save_atomic_immutable(&self, path: &Path) -> Result<()> {
        if self.entries.len() > MAX_MANAGED_TEXTURE_INDEX_ENTRIES {
            return Err(invalid_data("managed texture index entry limit exceeded"));
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        if path.exists() {
            return match Self::load_current(path, &self.build_id) {
                Some(existing) if existing == *self => Ok(()),
                _ => Err(invalid_data("immutable texture index cache collision")),
            };
        }

        let bytes = self.to_json()?;
        if bytes.len() as u64 > MAX_MANAGED_TEXTURE_INDEX_BYTES {
            return Err(invalid_data("managed texture index byte limit exceeded"));
        }
        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("texture-index");
        let sequence = INDEX_TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let temp = path.with_file_name(format!(
            ".{file_name}.tmp-{}-{sequence}",
            std::process::id()
        ));
        let publication = (|| -> std::io::Result<()> {
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temp)?;
            file.write_all(&bytes)?;
            file.sync_all()?;
            drop(file);
            // Same-directory hard-link publication is atomic and, unlike Unix rename,
            // never replaces an existing immutable cache entry.
            std::fs::hard_link(&temp, path)
        })();
        let _ = std::fs::remove_file(&temp);
        match publication {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                match Self::load_current(path, &self.build_id) {
                    Some(existing) if existing == *self => Ok(()),
                    _ => Err(invalid_data("immutable texture index cache collision")),
                }
            }
            Err(error) => Err(error.into()),
        }
    }
    pub fn load(path: &Path) -> Result<Self> {
        Self::from_json(&std::fs::read(path)?)
    }
    /// Load the cached index only if it is still current for this game build.
    /// Returns `None` if the cache is absent, unreadable, or its `build_id` does not
    /// match `expected_build_id` (e.g. a game patch changed the .usmap) — so a stale
    /// cache mapping asset paths to outdated package ids is never trusted.
    pub fn load_current(path: &Path, expected_build_id: &str) -> Option<Self> {
        let file = File::open(path).ok()?;
        let metadata = file.metadata().ok()?;
        if !metadata.file_type().is_file() || metadata.len() > MAX_MANAGED_TEXTURE_INDEX_BYTES {
            return None;
        }
        let capacity = usize::try_from(metadata.len()).ok()?;
        let mut bytes = Vec::with_capacity(capacity);
        file.take(MAX_MANAGED_TEXTURE_INDEX_BYTES + 1)
            .read_to_end(&mut bytes)
            .ok()?;
        if bytes.len() as u64 != metadata.len() {
            return None;
        }
        Self::from_json(&bytes).ok().filter(|index| {
            index.build_id == expected_build_id
                && index.entries.len() <= MAX_MANAGED_TEXTURE_INDEX_ENTRIES
        })
    }
}

/// Cryptographically seals every input admitted by the exact installed composite used by
/// texture indexing/extraction: mapping plus each winning-order UTOC/UCAS pair, including
/// hotfix and global siblings. Per-file content digests are persisted independently, so a
/// hotfix rehashes only changed/new files rather than every multi-GiB UCAS.
pub fn build_id_for(utoc: &Path, usmap: &Path) -> Result<String> {
    build_id_for_in_cache_dir(utoc, usmap, &gore_loc::paths::shared_data_dir())
}

/// Cheap change token for the installed IoStore source set. On filesystems with
/// strong change tokens this binds only direct UTOC/UCAS metadata. A source
/// lacking such a token is hashed so restored timestamps cannot hide updates.
pub(crate) fn quick_composite_source_identity(main_utoc: &Path) -> Result<String> {
    let paks = main_utoc
        .parent()
        .ok_or_else(|| invalid_data("installed texture UTOC has no Paks parent"))?;
    let canonical_paks = std::fs::canonicalize(paks)?;
    if !std::fs::symlink_metadata(&canonical_paks)?
        .file_type()
        .is_dir()
    {
        return Err(invalid_data(
            "installed texture Paks authority is not a directory",
        ));
    }
    let canonical_main = std::fs::canonicalize(main_utoc)?;
    if canonical_main.parent() != Some(canonical_paks.as_path()) {
        return Err(invalid_data(
            "installed texture UTOC escaped the Paks authority",
        ));
    }

    let mut found_main = false;
    let mut bindings = Vec::new();
    for entry in std::fs::read_dir(&canonical_paks)? {
        let entry = entry?;
        let path = entry.path();
        let relevant = path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| {
                extension.eq_ignore_ascii_case("utoc") || extension.eq_ignore_ascii_case("ucas")
            });
        if !relevant {
            continue;
        }
        if bindings.len() >= MAX_SOURCE_FILES {
            return Err(invalid_data("too many installed texture source files"));
        }
        let metadata = std::fs::symlink_metadata(&path)?;
        if !metadata.file_type().is_file() || metadata.file_type().is_symlink() {
            return Err(invalid_data("texture source is not a plain regular file"));
        }
        let mut file = open_regular_no_follow(&path)?;
        let metadata = file.metadata()?;
        let identity = file_identity(&file, &metadata)?;
        found_main |= path == canonical_main;
        let role = path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| invalid_data("installed texture source name is not Unicode"))?
            .to_string();
        bindings.push(quick_source_binding_identity(role, &mut file, identity)?);
    }
    if !found_main {
        return Err(invalid_data(
            "requested main UTOC is absent from the installed texture sources",
        ));
    }
    bindings.sort_by(|left, right| left.role.cmp(&right.role));
    let encoded = serde_json::to_vec(&QuickSourceIdentity(bindings)).map_err(|error| {
        crate::error::TexError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            error.to_string(),
        ))
    })?;
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"gore-tex.quick-composite-source-identity.v1\0");
    hasher.update(&encoded);
    Ok(hasher.finalize().to_hex().to_string())
}

fn quick_source_binding_identity(
    role: String,
    file: &mut File,
    identity: FileIdentity,
) -> Result<QuickSourceBindingIdentity> {
    let unreliable_content_blake3 = if identity.reliable_change_token {
        None
    } else {
        file.seek(SeekFrom::Start(0))?;
        let mut content = blake3::Hasher::new();
        let copied = std::io::copy(file, &mut content)?;
        if copied != identity.byte_len {
            return Err(invalid_data(
                "texture source length changed while checking its identity",
            ));
        }
        Some(content.finalize().to_hex().to_string())
    };
    Ok(QuickSourceBindingIdentity {
        role,
        file: identity,
        unreliable_content_blake3,
    })
}

fn build_id_for_in_cache_dir(utoc: &Path, usmap: &Path, cache_directory: &Path) -> Result<String> {
    let composite = crate::container::InstalledTextureComposite::open(utoc)?;
    let mut sources = open_source_files(&composite, usmap)?;
    fingerprint_open_sources(&mut sources, cache_directory)
}

/// One captured installed-container generation whose source files stay open
/// for an entire batch. This lets batch consumers verify the same build before
/// publication without reopening the IoStore composite.
pub(crate) struct OpenTextureGeneration {
    sources: Vec<OpenSourceFile>,
    build_id: String,
    cache_directory: PathBuf,
}

impl OpenTextureGeneration {
    pub(crate) fn capture(
        composite: &crate::container::InstalledTextureComposite,
        cache_directory: &Path,
    ) -> Result<Self> {
        let mut sources = open_composite_source_files(composite)?;
        let build_id = fingerprint_open_sources(&mut sources, cache_directory)?;
        Ok(Self {
            sources,
            build_id,
            cache_directory: cache_directory.to_path_buf(),
        })
    }

    pub(crate) fn captured_build_id(&self) -> &str {
        &self.build_id
    }

    /// Recompute (or identity-cache) the fingerprint through the same captured
    /// file handles after revalidating that every named source still resolves
    /// to the captured file. A changed source either produces a different id or
    /// fails closed; both prevent publication by the batch caller.
    pub(crate) fn current_build_id(&mut self) -> Result<String> {
        for source in &self.sources {
            revalidate_source_file(source)?;
        }
        fingerprint_open_sources(&mut self.sources, &self.cache_directory)
    }
}

fn fingerprint_open_sources(
    sources: &mut [OpenSourceFile],
    cache_directory: &Path,
) -> Result<String> {
    let identity = SourceIdentity(
        sources
            .iter()
            .map(|source| SourceBindingIdentity {
                role: source.role.clone(),
                file: source.identity.clone(),
            })
            .collect(),
    );
    let cacheable = identity
        .0
        .iter()
        .all(|binding| binding.file.reliable_change_token);
    source_fingerprint_from_cache_or_compute(identity, cacheable, || {
        let mut aggregate = blake3::Hasher::new();
        aggregate.update(SOURCE_FINGERPRINT_DOMAIN);
        aggregate.update(&(sources.len() as u64).to_le_bytes());
        for source in sources {
            aggregate.update(&(source.role.len() as u64).to_le_bytes());
            aggregate.update(source.role.as_bytes());
            aggregate.update(&source.identity.byte_len.to_le_bytes());
            let digest = digest_source_file(source, cache_directory)?;
            aggregate.update(&digest);
        }
        Ok(format!(
            "gore-texture-composite-v3:{}",
            aggregate.finalize().to_hex()
        ))
    })
}

fn source_fingerprint_from_cache_or_compute(
    identity: SourceIdentity,
    cacheable: bool,
    compute: impl FnOnce() -> Result<String>,
) -> Result<String> {
    let cache = SOURCE_FINGERPRINT_CACHE.get_or_init(|| Mutex::new(Vec::new()));
    if cacheable {
        let mut cache = cache
            .lock()
            .map_err(|_| invalid_data("texture source fingerprint cache is poisoned"))?;
        if let Some(position) = cache.iter().position(|(cached, _)| cached == &identity) {
            let hit = cache.remove(position);
            let fingerprint = hit.1.clone();
            cache.push(hit);
            return Ok(fingerprint);
        }
    }

    // Computing a source fingerprint can hash multiple multi-GiB files. Keep that work
    // outside the process-wide cache lock so unrelated installs can fingerprint in parallel.
    let fingerprint = compute()?;
    if cacheable {
        let mut cache = cache
            .lock()
            .map_err(|_| invalid_data("texture source fingerprint cache is poisoned"))?;
        // Another caller may have published this identity while we were computing it.
        // Replace that entry so the LRU stays unique and the just-validated result wins.
        if let Some(position) = cache.iter().position(|(cached, _)| cached == &identity) {
            cache.remove(position);
        }
        cache.push((identity, fingerprint.clone()));
        if cache.len() > MAX_SOURCE_FINGERPRINT_CACHE_ENTRIES {
            cache.remove(0);
        }
    }
    Ok(fingerprint)
}

fn digest_source_file(source: &mut OpenSourceFile, cache_directory: &Path) -> Result<[u8; 32]> {
    let cache = SOURCE_FILE_DIGEST_CACHE.get_or_init(|| Mutex::new(Vec::new()));
    let mut cache = cache
        .lock()
        .map_err(|_| invalid_data("texture source file digest cache is poisoned"))?;
    let mut cached = source
        .identity
        .reliable_change_token
        .then(|| {
            cache
                .iter()
                .position(|(identity, _)| identity == &source.identity)
                .map(|position| cache.remove(position))
        })
        .flatten();
    if cached.is_none() && source.identity.reliable_change_token {
        cached = load_persisted_source_file_digest(cache_directory, &source.identity)?
            .map(|digest| (source.identity.clone(), digest));
    }
    if let Some(hit) = cached {
        let digest = decode_blake3(&hit.1)?;
        verify_parsed_utoc_digest(source, &digest)?;
        revalidate_source_file(source)?;
        cache.push(hit);
        if cache.len() > MAX_SOURCE_FILE_DIGEST_CACHE_ENTRIES {
            cache.remove(0);
        }
        return Ok(digest);
    }
    drop(cache);

    source.file.seek(SeekFrom::Start(0))?;
    let mut content = blake3::Hasher::new();
    let copied = std::io::copy(&mut source.file, &mut content)?;
    if copied != source.identity.byte_len {
        return Err(invalid_data("texture source length changed while hashing"));
    }
    #[cfg(test)]
    SOURCE_FILE_HASHED_BYTES.fetch_add(copied, Ordering::Relaxed);
    let digest = *content.finalize().as_bytes();
    verify_parsed_utoc_digest(source, &digest)?;
    revalidate_source_file(source)?;
    let encoded = blake3_hex(&digest);
    if source.identity.reliable_change_token {
        persist_source_file_digest(cache_directory, &source.identity, &encoded)?;
    }

    let mut cache = SOURCE_FILE_DIGEST_CACHE
        .get()
        .expect("source digest cache was initialized")
        .lock()
        .map_err(|_| invalid_data("texture source file digest cache is poisoned"))?;
    if source.identity.reliable_change_token {
        cache.push((source.identity.clone(), encoded));
        if cache.len() > MAX_SOURCE_FILE_DIGEST_CACHE_ENTRIES {
            cache.remove(0);
        }
    }
    Ok(digest)
}

fn verify_parsed_utoc_digest(source: &OpenSourceFile, digest: &[u8; 32]) -> Result<()> {
    if source
        .parsed_blake3
        .is_some_and(|expected| expected != *digest)
    {
        return Err(invalid_data(
            "texture UTOC differs from the exact bytes parsed by the composite",
        ));
    }
    Ok(())
}

fn file_identity_key(identity: &FileIdentity) -> Result<String> {
    let bytes = serde_json::to_vec(identity).map_err(|error| {
        crate::error::TexError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            error.to_string(),
        ))
    })?;
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"gore-tex.source-file-digest-cache-key.v1\0");
    hasher.update(&bytes);
    Ok(hasher.finalize().to_hex().to_string())
}

fn load_persisted_source_file_digest(
    cache_directory: &Path,
    identity: &FileIdentity,
) -> Result<Option<String>> {
    let path = cache_directory.join(format!(
        "texture-source-file-digest-v1-{}.json",
        file_identity_key(identity)?
    ));
    let file = match File::open(&path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    let metadata = file.metadata()?;
    if !metadata.file_type().is_file() || metadata.len() > MAX_SOURCE_FILE_DIGEST_CACHE_BYTES {
        let _ = std::fs::remove_file(path);
        return Ok(None);
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    file.take(MAX_SOURCE_FILE_DIGEST_CACHE_BYTES + 1)
        .read_to_end(&mut bytes)?;
    let parsed: PersistedSourceFileDigest = match serde_json::from_slice(&bytes) {
        Ok(parsed) => parsed,
        Err(_) => {
            let _ = std::fs::remove_file(path);
            return Ok(None);
        }
    };
    if parsed.format != SOURCE_FILE_DIGEST_CACHE_FORMAT
        || parsed.version != SOURCE_FILE_DIGEST_CACHE_VERSION
        || parsed.identity != *identity
        || !valid_blake3(&parsed.digest)
    {
        let _ = std::fs::remove_file(path);
        return Ok(None);
    }
    Ok(Some(parsed.digest))
}

fn persist_source_file_digest(
    cache_directory: &Path,
    identity: &FileIdentity,
    digest: &str,
) -> Result<()> {
    let path = cache_directory.join(format!(
        "texture-source-file-digest-v1-{}.json",
        file_identity_key(identity)?
    ));
    let record = PersistedSourceFileDigest {
        format: SOURCE_FILE_DIGEST_CACHE_FORMAT.to_string(),
        version: SOURCE_FILE_DIGEST_CACHE_VERSION,
        identity: identity.clone(),
        digest: digest.to_string(),
    };
    let bytes = serde_json::to_vec(&record).map_err(|error| {
        crate::error::TexError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            error.to_string(),
        ))
    })?;
    if bytes.len() as u64 > MAX_SOURCE_FILE_DIGEST_CACHE_BYTES {
        return Err(invalid_data(
            "texture source digest cache entry is too large",
        ));
    }
    publish_immutable_bytes(&path, &bytes)?;
    let parent = path
        .parent()
        .ok_or_else(|| invalid_data("texture source cache has no parent"))?
        .to_path_buf();
    prune_cache_files(
        &parent,
        "texture-source-file-digest-v1-",
        MAX_PERSISTED_SOURCE_FILE_DIGESTS,
        MAX_SOURCE_FILE_DIGEST_CACHE_BYTES * MAX_PERSISTED_SOURCE_FILE_DIGESTS as u64,
        &[path],
    )
}

fn valid_blake3(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn decode_blake3(value: &str) -> Result<[u8; 32]> {
    if !valid_blake3(value) {
        return Err(invalid_data("cached texture source digest is invalid"));
    }
    let mut output = [0_u8; 32];
    for (index, pair) in value.as_bytes().chunks_exact(2).enumerate() {
        let nibble = |byte: u8| match byte {
            b'0'..=b'9' => Some(byte - b'0'),
            b'a'..=b'f' => Some(byte - b'a' + 10),
            _ => None,
        };
        output[index] = (nibble(pair[0]).expect("validated hex") << 4)
            | nibble(pair[1]).expect("validated hex");
    }
    Ok(output)
}

fn blake3_hex(digest: &[u8; 32]) -> String {
    let mut output = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write as _;
        write!(&mut output, "{byte:02x}").expect("writing into a String cannot fail");
    }
    output
}

struct OpenSourceFile {
    role: String,
    path: PathBuf,
    file: File,
    identity: FileIdentity,
    parsed_blake3: Option<[u8; 32]>,
}

fn open_source_files(
    composite: &crate::container::InstalledTextureComposite,
    usmap: &Path,
) -> Result<Vec<OpenSourceFile>> {
    let mut sources = Vec::with_capacity(composite.sources().len().saturating_add(1));
    sources.push(open_source_file(
        usmap.to_path_buf(),
        "mapping".to_string(),
        None,
    )?);
    sources.extend(open_composite_source_files(composite)?);
    if sources.len() > MAX_SOURCE_FILES {
        return Err(invalid_data("too many installed texture source files"));
    }
    Ok(sources)
}

fn open_composite_source_files(
    composite: &crate::container::InstalledTextureComposite,
) -> Result<Vec<OpenSourceFile>> {
    if composite.sources().len() > MAX_SOURCE_FILES {
        return Err(invalid_data("too many installed texture source files"));
    }
    composite
        .sources()
        .iter()
        .map(|source| {
            open_source_file(
                source.path.clone(),
                source.role.clone(),
                source.parsed_blake3,
            )
        })
        .collect()
}

fn open_source_file(
    path: PathBuf,
    role: String,
    parsed_blake3: Option<[u8; 32]>,
) -> Result<OpenSourceFile> {
    let metadata = std::fs::symlink_metadata(&path)?;
    if !metadata.file_type().is_file() || metadata.file_type().is_symlink() {
        return Err(invalid_data("texture source is not a plain regular file"));
    }
    let file = open_regular_no_follow(&path)?;
    let metadata = file.metadata()?;
    if !metadata.file_type().is_file() {
        return Err(invalid_data("texture source is not a regular file"));
    }
    let identity = file_identity(&file, &metadata)?;
    Ok(OpenSourceFile {
        role,
        path,
        file,
        identity,
        parsed_blake3,
    })
}

fn revalidate_source_file(source: &OpenSourceFile) -> Result<()> {
    let handle_metadata = source.file.metadata()?;
    let handle_identity = file_identity(&source.file, &handle_metadata)?;
    let reopened = open_regular_no_follow(&source.path)?;
    let reopened_metadata = reopened.metadata()?;
    let reopened_identity = file_identity(&reopened, &reopened_metadata)?;
    if handle_identity != source.identity || reopened_identity != source.identity {
        return Err(invalid_data(
            "texture source changed while computing its fingerprint",
        ));
    }
    Ok(())
}

#[cfg(windows)]
fn open_regular_no_follow(path: &Path) -> std::io::Result<File> {
    use std::os::windows::fs::OpenOptionsExt as _;
    use windows_sys::Win32::Storage::FileSystem::FILE_FLAG_OPEN_REPARSE_POINT;

    let mut options = OpenOptions::new();
    options
        .read(true)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT);
    options.open(path)
}

#[cfg(unix)]
fn open_regular_no_follow(path: &Path) -> std::io::Result<File> {
    use std::os::unix::fs::OpenOptionsExt as _;

    let mut options = OpenOptions::new();
    options
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC);
    options.open(path)
}

#[cfg(not(any(windows, unix)))]
fn open_regular_no_follow(path: &Path) -> std::io::Result<File> {
    OpenOptions::new().read(true).open(path)
}

#[cfg(windows)]
fn file_identity(file: &File, metadata: &std::fs::Metadata) -> Result<FileIdentity> {
    use std::os::windows::fs::MetadataExt as _;
    use std::os::windows::io::AsRawHandle as _;
    use windows_sys::Win32::Storage::FileSystem::{
        FileBasicInfo, GetFileInformationByHandle, GetFileInformationByHandleEx,
        BY_HANDLE_FILE_INFORMATION, FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_REPARSE_POINT,
        FILE_BASIC_INFO,
    };
    // SAFETY: `file` owns a valid handle and `information` has the Win32 ABI layout.
    let mut information: BY_HANDLE_FILE_INFORMATION = unsafe { std::mem::zeroed() };
    let success = unsafe { GetFileInformationByHandle(file.as_raw_handle(), &mut information) };
    if success == 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    let mut basic = FILE_BASIC_INFO::default();
    // SAFETY: `file` owns a valid handle and `basic` is a correctly-sized output buffer.
    let success = unsafe {
        GetFileInformationByHandleEx(
            file.as_raw_handle(),
            FileBasicInfo,
            std::ptr::addr_of_mut!(basic).cast(),
            std::mem::size_of::<FILE_BASIC_INFO>() as u32,
        )
    };
    if success == 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    if basic.FileAttributes & (FILE_ATTRIBUTE_DIRECTORY | FILE_ATTRIBUTE_REPARSE_POINT) != 0 {
        return Err(invalid_data("texture source is not a plain regular file"));
    }
    let index =
        (u64::from(information.nFileIndexHigh) << 32) | u64::from(information.nFileIndexLow);
    Ok(FileIdentity {
        byte_len: metadata.file_size(),
        modified_stamp: basic.LastWriteTime.to_string(),
        // Unlike LastWriteTime, NTFS ChangeTime is changed by an in-place write
        // even when a patcher deliberately restores the visible modification time.
        change_stamp: basic.ChangeTime.to_string(),
        // Some non-NTFS filesystems report no ChangeTime. They remain supported,
        // but their contents are deliberately rehashed on every generation check.
        reliable_change_token: basic.ChangeTime != 0,
        platform_identity: format!(
            "windows-volume-{:08x}-file-{index:016x}",
            information.dwVolumeSerialNumber
        ),
    })
}

#[cfg(unix)]
fn file_identity(_file: &File, metadata: &std::fs::Metadata) -> Result<FileIdentity> {
    use std::os::unix::fs::MetadataExt as _;
    Ok(FileIdentity {
        byte_len: metadata.len(),
        modified_stamp: format!("{}.{:09}", metadata.mtime(), metadata.mtime_nsec()),
        // ctime cannot be restored with ordinary file timestamp APIs and closes
        // the same-size/restored-mtime cache hole on Unix filesystems.
        change_stamp: format!("{}.{:09}", metadata.ctime(), metadata.ctime_nsec()),
        reliable_change_token: true,
        platform_identity: format!("unix-dev-{:x}-ino-{:x}", metadata.dev(), metadata.ino()),
    })
}

#[cfg(not(any(windows, unix)))]
fn file_identity(_file: &File, _metadata: &std::fs::Metadata) -> Result<FileIdentity> {
    Err(invalid_data(
        "texture source fingerprints are unsupported on this platform",
    ))
}

/// Prune disposable index caches while preserving the generation used by this call.
///
/// The caller owns any live in-memory index. Disk caches are immutable performance
/// artifacts, so generations used by earlier calls do not remain pinned for the
/// lifetime of this process.
pub fn pin_and_prune_managed_texture_cache(path: &Path) -> Result<()> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata)
            if !metadata.file_type().is_file()
                || metadata.len() > MAX_MANAGED_TEXTURE_INDEX_BYTES =>
        {
            return Err(invalid_data(
                "managed texture index cache is not a bounded regular file",
            ));
        }
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            // Another process may already have pruned this disposable cache.
            // The caller's in-memory index remains the live authority.
        }
        Err(error) => return Err(error.into()),
    }
    prune_cache_files(
        path.parent()
            .ok_or_else(|| invalid_data("managed texture cache has no parent"))?,
        "texture-index-v2-",
        MAX_MANAGED_TEXTURE_INDEX_FILES,
        MAX_MANAGED_TEXTURE_INDEX_BYTES,
        &[path.to_path_buf()],
    )
}

fn publish_immutable_bytes(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    if path.exists() {
        let metadata = std::fs::symlink_metadata(path)?;
        if !metadata.file_type().is_file() || metadata.len() != bytes.len() as u64 {
            return Err(invalid_data("immutable cache path collision"));
        }
        return if std::fs::read(path)? == bytes {
            Ok(())
        } else {
            Err(invalid_data("immutable cache content collision"))
        };
    }
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("cache");
    let sequence = INDEX_TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let temp = path.with_file_name(format!(
        ".{file_name}.tmp-{}-{sequence}",
        std::process::id()
    ));
    let publication = (|| -> std::io::Result<()> {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        std::fs::hard_link(&temp, path)
    })();
    let _ = std::fs::remove_file(&temp);
    match publication {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            let metadata = std::fs::symlink_metadata(path)?;
            if metadata.file_type().is_file()
                && metadata.len() == bytes.len() as u64
                && std::fs::read(path)? == bytes
            {
                Ok(())
            } else {
                Err(invalid_data("immutable cache publication collision"))
            }
        }
        Err(error) => Err(error.into()),
    }
}

fn prune_cache_files(
    directory: &Path,
    prefix: &str,
    maximum_files: usize,
    maximum_bytes: u64,
    preserved: &[PathBuf],
) -> Result<()> {
    let mut entries = Vec::new();
    for entry in std::fs::read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if !name.starts_with(prefix) || !name.ends_with(".json") {
            continue;
        }
        let metadata = match std::fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error.into()),
        };
        if !metadata.file_type().is_file() {
            continue;
        }
        entries.push((
            path,
            metadata.len(),
            metadata.modified().unwrap_or(std::time::UNIX_EPOCH),
        ));
    }
    let mut total_bytes = entries
        .iter()
        .try_fold(0_u64, |total, (_, bytes, _)| total.checked_add(*bytes))
        .ok_or_else(|| invalid_data("texture cache byte total overflow"))?;
    let mut total_files = entries.len();
    entries.sort_by_key(|(_, _, modified)| *modified);
    for (path, byte_len, _) in entries {
        if total_files <= maximum_files && total_bytes <= maximum_bytes {
            break;
        }
        if preserved.iter().any(|preserve| preserve == &path) {
            continue;
        }
        match std::fs::remove_file(&path) {
            Ok(()) => {
                total_files -= 1;
                total_bytes -= byte_len;
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                total_files -= 1;
                total_bytes -= byte_len;
            }
            Err(error) => return Err(error.into()),
        }
    }
    if total_files > maximum_files || total_bytes > maximum_bytes {
        return Err(invalid_data(
            "preserved texture cache exceeds bounded storage",
        ));
    }
    Ok(())
}

fn invalid_data(message: &'static str) -> crate::error::TexError {
    std::io::Error::new(std::io::ErrorKind::InvalidData, message).into()
}

#[derive(Clone, Copy)]
struct ManagedTextureIndexBuildLimits {
    entries: usize,
    path_bytes: usize,
    aggregate_path_bytes: usize,
    response_bytes: usize,
}

impl Default for ManagedTextureIndexBuildLimits {
    fn default() -> Self {
        Self {
            entries: MAX_MANAGED_TEXTURE_INDEX_ENTRIES,
            path_bytes: MAX_MANAGED_TEXTURE_ASSET_PATH_BYTES,
            aggregate_path_bytes: crate::package_index::MAX_AGGREGATE_DIRECTORY_INDEX_PATH_BYTES,
            response_bytes: MAX_MANAGED_TEXTURE_INDEX_BYTES as usize,
        }
    }
}

struct ManagedTextureIndexEntries {
    entries: BTreeMap<String, u64>,
    aggregate_path_bytes: usize,
    estimated_response_bytes: usize,
    limits: ManagedTextureIndexBuildLimits,
}

impl ManagedTextureIndexEntries {
    fn new(build_id: &str, limits: ManagedTextureIndexBuildLimits) -> Result<Self> {
        let encoded_build_id = serde_json::to_vec(build_id)
            .map_err(|_| invalid_data("managed texture index build id is not serializable"))?;
        let estimated_response_bytes = b"{\"ok\":true,\"build_id\":"
            .len()
            .checked_add(encoded_build_id.len())
            .and_then(|bytes| bytes.checked_add(b",\"count\":".len()))
            .and_then(|bytes| {
                bytes.checked_add(MAX_MANAGED_TEXTURE_INDEX_ENTRIES.to_string().len())
            })
            .and_then(|bytes| bytes.checked_add(b",\"entries\":{}}".len()))
            .ok_or_else(|| invalid_data("managed texture index response byte total overflow"))?;
        if estimated_response_bytes > limits.response_bytes {
            return Err(invalid_data(
                "managed texture index response byte limit exceeded",
            ));
        }
        Ok(Self {
            entries: BTreeMap::new(),
            aggregate_path_bytes: 0,
            estimated_response_bytes,
            limits,
        })
    }

    fn insert(&mut self, path: String, package_id: u64) -> Result<()> {
        if path.len() > self.limits.path_bytes {
            return Err(invalid_data(
                "managed texture index asset path byte limit exceeded",
            ));
        }
        if !is_canonical_managed_texture_path(&path, self.limits.path_bytes) {
            return Err(invalid_data(
                "managed texture index contains a noncanonical asset path",
            ));
        }
        if self.entries.contains_key(&path) {
            self.entries.insert(path, package_id);
            return Ok(());
        }
        if self.entries.len() >= self.limits.entries {
            return Err(invalid_data("managed texture index entry limit exceeded"));
        }
        let aggregate_path_bytes = self
            .aggregate_path_bytes
            .checked_add(path.len())
            .ok_or_else(|| invalid_data("managed texture index path byte total overflow"))?;
        if aggregate_path_bytes > self.limits.aggregate_path_bytes {
            return Err(invalid_data(
                "managed texture index aggregate path byte limit exceeded",
            ));
        }
        // The FFI response encodes each package id as a quoted canonical decimal
        // u64. Budget the maximum width (plus a comma even for the first entry),
        // so the eventual response is never larger than this running estimate.
        let response_entry_bytes = path
            .len()
            .checked_add(1 + 2 + 1 + 2 + MAX_PACKAGE_ID_DECIMAL_BYTES)
            .ok_or_else(|| invalid_data("managed texture index response byte total overflow"))?;
        let estimated_response_bytes = self
            .estimated_response_bytes
            .checked_add(response_entry_bytes)
            .ok_or_else(|| invalid_data("managed texture index response byte total overflow"))?;
        if estimated_response_bytes > self.limits.response_bytes {
            return Err(invalid_data(
                "managed texture index response byte limit exceeded",
            ));
        }
        self.entries.insert(path, package_id);
        self.aggregate_path_bytes = aggregate_path_bytes;
        self.estimated_response_bytes = estimated_response_bytes;
        Ok(())
    }
}

fn is_canonical_managed_texture_path(path: &str, maximum_path_bytes: usize) -> bool {
    if path.len() > maximum_path_bytes || !path.starts_with('/') {
        return false;
    }
    let mut segments = path[1..].split('/');
    let Some(mount) = segments.next() else {
        return false;
    };
    if mount.is_empty()
        || !mount
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
    {
        return false;
    }
    let mut segment_count = 1usize;
    for segment in segments {
        segment_count += 1;
        if segment.is_empty()
            || !segment
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'+' | b'-'))
        {
            return false;
        }
    }
    (2..=crate::package_index::MAX_GAME_PACKAGE_SEGMENTS).contains(&segment_count)
}

fn collect_texture_entries<I, T, F>(
    build_id: &str,
    packages: I,
    inspect: F,
) -> Result<BTreeMap<String, u64>>
where
    I: IntoIterator<Item = T>,
    F: Fn(T) -> Option<(String, u64)>,
{
    collect_texture_entries_with_limits(
        build_id,
        packages,
        inspect,
        ManagedTextureIndexBuildLimits::default(),
    )
}

fn collect_texture_entries_with_limits<I, T, F>(
    build_id: &str,
    packages: I,
    inspect: F,
    limits: ManagedTextureIndexBuildLimits,
) -> Result<BTreeMap<String, u64>>
where
    I: IntoIterator<Item = T>,
    F: Fn(T) -> Option<(String, u64)>,
{
    let mut entries = ManagedTextureIndexEntries::new(build_id, limits)?;
    for package in packages {
        // Retoc can panic while decoding one malformed package. Isolate that
        // package without mutating the process-global panic hook; callers may
        // install their own hook and other threads may panic concurrently.
        let inspected = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| inspect(package)));
        if let Ok(Some((path, package_id))) = inspected {
            entries.insert(path, package_id)?;
        }
    }
    Ok(entries.entries)
}

/// Build the index by scanning the container once (same walk as `list_textures`,
/// additionally capturing each Texture2D package's id). `build_id` identifies the
/// game build (pass the .usmap filename).
pub fn build_index(utoc: &Path, build_id: &str) -> Result<TextureIndex> {
    let composite = crate::container::InstalledTextureComposite::open(utoc)?;
    let store = composite.store();
    let texture2d = FPackageObjectIndex::create_script_import("/Script/Engine.Texture2D");
    let cv = store
        .container_file_version()
        .ok_or_else(|| anyhow::anyhow!("container has no TOC version"))?;
    let hv = store
        .container_header_version()
        .ok_or_else(|| anyhow::anyhow!("container has no header version"))?;

    // `packages_all()` is Retoc's deduplicated, priority-ordered composite view.
    // Reading each package through the same store therefore resolves exactly the
    // same hotfix/global winner later used by preview extraction.
    let entries = collect_texture_entries(build_id, store.packages_all(), |pkg| {
        let pkg_id = pkg.id();
        let cid = FIoChunkId::from_package_id(pkg_id, 0, EIoChunkType::ExportBundleData);
        let data = store.read(cid).ok()?;
        let header = FZenPackageHeader::deserialize(
            &mut Cursor::new(&data),
            store.package_store_entry(pkg_id),
            cv,
            hv,
            None,
        )
        .ok()?;
        if !header.export_map.iter().any(|e| e.class_index == texture2d) {
            return None;
        }
        Some((header.package_name(), pkg_id.0))
    })?;

    Ok(TextureIndex {
        build_id: build_id.to_string(),
        entries,
    })
}

/// Extract a texture to RGBA by package id (fast: no scan). Returns (TexInfo, rgba u32 px).
pub fn extract_by_package_id(
    utoc: &Path,
    usmap: &Path,
    package_id: u64,
    leaf: &str,
) -> Result<(crate::decode::TexInfo, Vec<u32>)> {
    const MAX_PREVIEW_USMAP_BYTES: u64 = 64 * 1024 * 1024;

    let composite = crate::container::InstalledTextureComposite::open(utoc)?;
    let mut converted = crate::container::unpack_texture_preview_by_id_from_open_store(
        composite.store(),
        package_id,
        leaf,
    )?;
    let ubulk = converted
        .sidecars
        .iter_mut()
        .find(|sidecar| sidecar.kind == crate::container::VerifiedLegacySidecarKind::Bulk)
        .map(|sidecar| std::mem::take(&mut sidecar.bytes))
        .unwrap_or_default();
    let usmap_bytes = read_bounded_regular_file(usmap, MAX_PREVIEW_USMAP_BYTES)?;
    let mut info = crate::decode::parse(&converted.uasset, &converted.uexp, &ubulk, &usmap_bytes)?;

    // Decode first, then promptly release every converted source buffer. The
    // returned TexInfo is metadata-only; retaining mip0/VT scratch bytes beside
    // the final RGBA would otherwise double the preview's resident footprint.
    let pixels = crate::decode::to_rgba8(&info)?;
    info.mip0.clear();
    info.decoded_rgba = None;
    drop(converted);
    drop(ubulk);
    drop(usmap_bytes);
    Ok((info, pixels))
}

fn read_bounded_regular_file(path: &Path, maximum_bytes: u64) -> Result<Vec<u8>> {
    let mut source = open_source_file(path.to_path_buf(), "preview-mapping".into(), None)?;
    if source.identity.byte_len > maximum_bytes {
        return Err(invalid_data(
            "texture preview mapping is not a bounded regular file",
        ));
    }
    let expected = usize::try_from(source.identity.byte_len)
        .map_err(|_| invalid_data("texture preview mapping length is unsupported"))?;
    let mut bytes = Vec::with_capacity(expected);
    (&mut source.file)
        .take(maximum_bytes.saturating_add(1))
        .read_to_end(&mut bytes)?;
    if bytes.len() != expected {
        return Err(invalid_data(
            "texture preview mapping changed while reading",
        ));
    }
    revalidate_source_file(&source)?;
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    static SOURCE_FINGERPRINT_TEST_LOCK: Mutex<()> = Mutex::new(());
    #[test]
    fn index_json_roundtrips() {
        let mut idx = TextureIndex {
            build_id: "G1R-5.4.3".into(),
            entries: BTreeMap::new(),
        };
        idx.entries
            .insert("/Game/UI/T_X".into(), 0x1122334455667788);
        let back = TextureIndex::from_json(&idx.to_json().unwrap()).unwrap();
        assert_eq!(idx, back);
    }

    #[test]
    fn managed_index_insert_enforces_count_path_memory_and_response_limits() {
        let defaults = ManagedTextureIndexBuildLimits::default();
        assert_eq!(defaults.entries, MAX_MANAGED_TEXTURE_INDEX_ENTRIES);
        assert_eq!(defaults.path_bytes, MAX_MANAGED_TEXTURE_ASSET_PATH_BYTES);
        assert_eq!(
            defaults.aggregate_path_bytes,
            crate::package_index::MAX_AGGREGATE_DIRECTORY_INDEX_PATH_BYTES
        );
        assert_eq!(
            defaults.response_bytes,
            MAX_MANAGED_TEXTURE_INDEX_BYTES as usize
        );

        let mut count_limited = ManagedTextureIndexEntries::new(
            "build",
            ManagedTextureIndexBuildLimits {
                entries: 2,
                ..defaults
            },
        )
        .unwrap();
        count_limited.insert("/Game/A".into(), 1).unwrap();
        count_limited.insert("/Engine/B".into(), 2).unwrap();
        assert_eq!(
            count_limited
                .insert("/Datasmith/C".into(), 3)
                .unwrap_err()
                .to_string(),
            "io error: managed texture index entry limit exceeded"
        );
        assert_eq!(count_limited.entries.len(), 2);

        let at_path_limit = format!(
            "/Plugin/{}",
            "A".repeat(MAX_MANAGED_TEXTURE_ASSET_PATH_BYTES - "/Plugin/".len())
        );
        assert_eq!(at_path_limit.len(), MAX_MANAGED_TEXTURE_ASSET_PATH_BYTES);
        let mut paths = ManagedTextureIndexEntries::new("build", defaults).unwrap();
        paths.insert(at_path_limit.clone(), 1).unwrap();
        assert_eq!(
            paths
                .insert(format!("{at_path_limit}A"), 2)
                .unwrap_err()
                .to_string(),
            "io error: managed texture index asset path byte limit exceeded"
        );

        let first = "/Game/First";
        let second = "/Game/Second";
        let mut memory_limited = ManagedTextureIndexEntries::new(
            "build",
            ManagedTextureIndexBuildLimits {
                aggregate_path_bytes: first.len() + second.len() - 1,
                ..defaults
            },
        )
        .unwrap();
        memory_limited.insert(first.into(), 1).unwrap();
        assert_eq!(
            memory_limited
                .insert(second.into(), 2)
                .unwrap_err()
                .to_string(),
            "io error: managed texture index aggregate path byte limit exceeded"
        );
        assert_eq!(memory_limited.entries.len(), 1);

        let base_response_bytes = ManagedTextureIndexEntries::new("build", defaults)
            .unwrap()
            .estimated_response_bytes;
        let first_response_entry_bytes = first.len() + 1 + 2 + 1 + 2 + MAX_PACKAGE_ID_DECIMAL_BYTES;
        let mut response_limited = ManagedTextureIndexEntries::new(
            "build",
            ManagedTextureIndexBuildLimits {
                response_bytes: base_response_bytes + first_response_entry_bytes,
                ..defaults
            },
        )
        .unwrap();
        response_limited.insert(first.into(), u64::MAX).unwrap();
        assert_eq!(
            response_limited
                .insert(second.into(), 2)
                .unwrap_err()
                .to_string(),
            "io error: managed texture index response byte limit exceeded"
        );
        assert_eq!(response_limited.entries.len(), 1);

        // Replacing an existing winner changes neither count nor byte budgets.
        response_limited.insert(first.into(), 7).unwrap();
        assert_eq!(response_limited.entries.get(first), Some(&7));
    }

    #[test]
    fn managed_index_path_grammar_matches_the_generic_texture_catalog_contract() {
        let limits = ManagedTextureIndexBuildLimits::default();
        let entries = collect_texture_entries_with_limits(
            "build",
            [
                ("/Engine/T_Default", 1),
                ("/DatasmithContent/Textures/T-1+Preview", 2),
                ("/My_Plugin/Folder/T_2", 3),
            ],
            |(path, package_id)| Some((path.to_string(), package_id)),
            limits,
        )
        .unwrap();
        assert_eq!(entries.len(), 3);

        for noncanonical in [
            "/Game",
            "/Plugin-Name/Texture",
            "/Game/Bad.Name",
            "/Game/Bad\\Name",
            "/Game//Texture",
            "/Game/Texture/",
            "Game/Texture",
            "/Game/Täxture",
        ] {
            assert_eq!(
                collect_texture_entries("build", [noncanonical], |path| {
                    Some((path.to_string(), 1))
                })
                .unwrap_err()
                .to_string(),
                "io error: managed texture index contains a noncanonical asset path",
                "unexpected result for {noncanonical:?}"
            );
        }

        let too_many_segments = format!(
            "/{}",
            std::iter::repeat_n(
                "Segment",
                crate::package_index::MAX_GAME_PACKAGE_SEGMENTS + 1
            )
            .collect::<Vec<_>>()
            .join("/")
        );
        assert_eq!(
            collect_texture_entries("build", [too_many_segments], |path| Some((path, 1)))
                .unwrap_err()
                .to_string(),
            "io error: managed texture index contains a noncanonical asset path"
        );
    }

    #[test]
    fn parallel_package_scans_preserve_the_process_panic_hook() {
        const CHILD_ENV: &str = "GORE_TEX_INDEX_PANIC_HOOK_CHILD";
        if std::env::var_os(CHILD_ENV).is_some() {
            use std::sync::atomic::AtomicUsize;
            use std::sync::{Arc, Barrier};

            static HOOK_CALLS: AtomicUsize = AtomicUsize::new(0);
            HOOK_CALLS.store(0, Ordering::SeqCst);
            std::panic::set_hook(Box::new(|_| {
                HOOK_CALLS.fetch_add(1, Ordering::SeqCst);
            }));

            const WORKERS: usize = 8;
            let barrier = Arc::new(Barrier::new(WORKERS));
            let workers = (0..WORKERS)
                .map(|worker| {
                    let barrier = Arc::clone(&barrier);
                    std::thread::spawn(move || {
                        let entries = collect_texture_entries(
                            "build",
                            std::iter::once(worker),
                            move |_| -> Option<(String, u64)> {
                                barrier.wait();
                                panic!("synthetic malformed package");
                            },
                        )
                        .unwrap();
                        assert!(entries.is_empty());
                    })
                })
                .collect::<Vec<_>>();
            for worker in workers {
                worker.join().unwrap();
            }
            assert_eq!(HOOK_CALLS.load(Ordering::SeqCst), WORKERS);
            let _ = std::panic::catch_unwind(|| panic!("sentinel panic"));
            assert_eq!(HOOK_CALLS.load(Ordering::SeqCst), WORKERS + 1);
            return;
        }

        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .arg("index::tests::parallel_package_scans_preserve_the_process_panic_hook")
            .arg("--exact")
            .arg("--nocapture")
            .env(CHILD_ENV, "1")
            .status()
            .unwrap();
        assert!(status.success(), "isolated panic-hook child test failed");
    }

    #[test]
    fn load_current_rejects_stale_build_id() {
        let dir = std::env::temp_dir().join("gore-tex-idx-stale");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("texture_index.json");
        let idx = TextureIndex {
            build_id: "G1R-5.4.3-old.usmap".into(),
            entries: BTreeMap::new(),
        };
        idx.save(&path).unwrap();
        // Matching build id -> Some; mismatched (game patched) -> None; absent -> None.
        assert!(TextureIndex::load_current(&path, "G1R-5.4.3-old.usmap").is_some());
        assert!(TextureIndex::load_current(&path, "G1R-5.4.4-new.usmap").is_none());
        assert!(TextureIndex::load_current(&dir.join("missing.json"), "x").is_none());
    }

    #[test]
    fn quick_composite_identity_tracks_source_files_but_ignores_unrelated_files() {
        let temp = tempfile::tempdir().unwrap();
        let main = temp.path().join("G1R-Windows.utoc");
        let main_data = temp.path().join("G1R-Windows.ucas");
        std::fs::write(&main, b"toc-a").unwrap();
        std::fs::write(&main_data, b"data-a").unwrap();
        let initial = quick_composite_source_identity(&main).unwrap();

        std::fs::write(temp.path().join("readme.txt"), b"ignored").unwrap();
        assert_eq!(quick_composite_source_identity(&main).unwrap(), initial);

        std::fs::write(temp.path().join("G1R-Windows_P.utoc"), b"toc-p").unwrap();
        std::fs::write(temp.path().join("G1R-Windows_P.ucas"), b"data-p").unwrap();
        let with_hotfix = quick_composite_source_identity(&main).unwrap();
        assert_ne!(with_hotfix, initial);

        std::fs::write(&main_data, b"longer-data-a").unwrap();
        assert_ne!(quick_composite_source_identity(&main).unwrap(), with_hotfix,);
    }

    #[test]
    fn quick_identity_hashes_sources_without_reliable_change_tokens() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("source.ucas");
        let identity = FileIdentity {
            byte_len: 6,
            modified_stamp: "unchanged".to_string(),
            change_stamp: "0".to_string(),
            reliable_change_token: false,
            platform_identity: "same-file".to_string(),
        };

        std::fs::write(&path, b"data-a").unwrap();
        let first = quick_source_binding_identity(
            "source.ucas".to_string(),
            &mut File::open(&path).unwrap(),
            identity.clone(),
        )
        .unwrap();
        std::fs::write(&path, b"data-b").unwrap();
        let second = quick_source_binding_identity(
            "source.ucas".to_string(),
            &mut File::open(&path).unwrap(),
            identity,
        )
        .unwrap();

        assert_ne!(
            first.unreliable_content_blake3,
            second.unreliable_content_blake3
        );
    }

    #[test]
    fn source_fingerprint_cache_unlocks_for_compute_and_bounds_racing_inserts() {
        let _guard = SOURCE_FINGERPRINT_TEST_LOCK.lock().unwrap();
        let cache = SOURCE_FINGERPRINT_CACHE.get_or_init(|| Mutex::new(Vec::new()));
        cache.lock().unwrap().clear();
        let identity_for = |marker: usize| {
            SourceIdentity(vec![SourceBindingIdentity {
                role: format!("source-{marker}"),
                file: FileIdentity {
                    byte_len: marker as u64,
                    modified_stamp: format!("modified-{marker}"),
                    change_stamp: format!("changed-{marker}"),
                    reliable_change_token: true,
                    platform_identity: format!("file-{marker}"),
                },
            }])
        };

        let identity = identity_for(0);
        let racing_identity = identity.clone();
        let fingerprint = source_fingerprint_from_cache_or_compute(identity.clone(), true, || {
            {
                let unlocked = cache.try_lock();
                assert!(
                    unlocked.is_ok(),
                    "source fingerprint cache must not be locked while computing"
                );
            }
            // Model another caller publishing the same identity during this computation.
            cache
                .lock()
                .unwrap()
                .push((racing_identity, "stale-racing-result".into()));
            Ok("validated-result".into())
        })
        .unwrap();
        assert_eq!(fingerprint, "validated-result");
        assert_eq!(
            cache
                .lock()
                .unwrap()
                .iter()
                .filter(|(cached, _)| cached == &identity)
                .collect::<Vec<_>>(),
            vec![&(identity.clone(), "validated-result".into())]
        );

        for marker in 1..=(MAX_SOURCE_FINGERPRINT_CACHE_ENTRIES + 3) {
            source_fingerprint_from_cache_or_compute(identity_for(marker), true, || {
                Ok(format!("fingerprint-{marker}"))
            })
            .unwrap();
        }
        let cache = cache.lock().unwrap();
        assert_eq!(cache.len(), MAX_SOURCE_FINGERPRINT_CACHE_ENTRIES);
        assert_eq!(
            cache
                .iter()
                .map(|(identity, _)| identity.0[0].role.as_str())
                .collect::<Vec<_>>(),
            (4..=(MAX_SOURCE_FINGERPRINT_CACHE_ENTRIES + 3))
                .map(|marker| format!("source-{marker}"))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn source_fingerprint_caches_per_file_and_detects_restored_mtime_rewrites() {
        let _guard = SOURCE_FINGERPRINT_TEST_LOCK.lock().unwrap();
        let sequence = INDEX_TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "gore-tex-source-file-digests-{}-{sequence}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let cache_root = root.join("cache");
        let mapping = root.join("Mappings.usmap");
        let toc = root.join("Game-Windows.utoc");
        let data = root.join("Game-Windows.ucas");
        std::fs::write(&mapping, b"mapping-0001").unwrap();
        std::fs::write(&toc, b"toc-data-001").unwrap();
        std::fs::write(&data, b"container-01").unwrap();

        let sources = || {
            vec![
                open_source_file(mapping.clone(), "mapping".into(), None).unwrap(),
                open_source_file(toc.clone(), "winner-000-utoc".into(), None).unwrap(),
                open_source_file(data.clone(), "winner-000-ucas".into(), None).unwrap(),
            ]
        };
        SOURCE_FINGERPRINT_CACHE
            .get_or_init(|| Mutex::new(Vec::new()))
            .lock()
            .unwrap()
            .clear();
        SOURCE_FILE_DIGEST_CACHE
            .get_or_init(|| Mutex::new(Vec::new()))
            .lock()
            .unwrap()
            .clear();
        SOURCE_FILE_HASHED_BYTES.store(0, Ordering::Relaxed);

        let first = fingerprint_open_sources(&mut sources(), &cache_root).unwrap();
        let initial_hashed = SOURCE_FILE_HASHED_BYTES.load(Ordering::Relaxed);
        assert_eq!(
            initial_hashed,
            mapping.metadata().unwrap().len()
                + toc.metadata().unwrap().len()
                + data.metadata().unwrap().len()
        );
        assert_eq!(
            fingerprint_open_sources(&mut sources(), &cache_root).unwrap(),
            first
        );
        assert_eq!(
            SOURCE_FILE_HASHED_BYTES.load(Ordering::Relaxed),
            initial_hashed,
            "an unchanged source set must not be rehashed"
        );

        // Simulate a new process so the next call must use persisted per-file
        // digests. Rewrite one file at the same length and restore LastWriteTime;
        // ChangeTime/ctime must still invalidate only that file's digest.
        SOURCE_FINGERPRINT_CACHE
            .get()
            .unwrap()
            .lock()
            .unwrap()
            .clear();
        SOURCE_FILE_DIGEST_CACHE
            .get()
            .unwrap()
            .lock()
            .unwrap()
            .clear();
        let original_modified = data.metadata().unwrap().modified().unwrap();
        let original_identity = open_source_file(data.clone(), "old".into(), None)
            .unwrap()
            .identity;
        std::fs::write(&data, b"container-02").unwrap();
        File::options()
            .write(true)
            .open(&data)
            .unwrap()
            .set_times(std::fs::FileTimes::new().set_modified(original_modified))
            .unwrap();
        let rewritten_identity = open_source_file(data.clone(), "new".into(), None)
            .unwrap()
            .identity;
        assert_eq!(
            rewritten_identity.modified_stamp,
            original_identity.modified_stamp
        );
        assert_ne!(
            rewritten_identity.change_stamp,
            original_identity.change_stamp
        );

        let second = fingerprint_open_sources(&mut sources(), &cache_root).unwrap();
        assert_ne!(second, first);
        assert_eq!(
            SOURCE_FILE_HASHED_BYTES.load(Ordering::Relaxed) - initial_hashed,
            data.metadata().unwrap().len(),
            "only the rewritten file may be rehashed"
        );

        let wrong_utoc_seal = [0x55; 32];
        SOURCE_FINGERPRINT_CACHE
            .get()
            .unwrap()
            .lock()
            .unwrap()
            .clear();
        let mut sealed = vec![
            open_source_file(mapping, "mapping".into(), None).unwrap(),
            open_source_file(toc, "winner-000-utoc".into(), Some(wrong_utoc_seal)).unwrap(),
            open_source_file(data, "winner-000-ucas".into(), None).unwrap(),
        ];
        assert!(fingerprint_open_sources(&mut sealed, &cache_root).is_err());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn build_id_includes_new_hotfix_siblings_from_the_real_composite() {
        use retoc::iostore_writer::IoStoreWriter;
        use retoc::version::EngineVersion;

        let _guard = SOURCE_FINGERPRINT_TEST_LOCK.lock().unwrap();
        let sequence = INDEX_TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "gore-tex-composite-build-id-{}-{sequence}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let cache = root.join("cache");
        let usmap = root.join("Mappings.usmap");
        std::fs::write(&usmap, b"mapping").unwrap();
        let version = EngineVersion::UE5_4;
        let write_container = |name: &str| {
            let utoc = root.join(format!("{name}.utoc"));
            IoStoreWriter::new(
                &utoc,
                version.toc_version(),
                Some(version.container_header_version()),
                retoc::UEPathBuf::from("../../../"),
            )
            .unwrap()
            .finalize()
            .unwrap();
            utoc
        };
        let main = write_container("pakchunk0-Windows");
        write_container("global");
        let before = build_id_for_in_cache_dir(&main, &usmap, &cache).unwrap();
        write_container("pakchunk0-Windows_P");
        let after = build_id_for_in_cache_dir(&main, &usmap, &cache).unwrap();
        assert_ne!(before, after);
        assert!(after.starts_with("gore-texture-composite-v3:"));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn immutable_cache_publish_never_replaces_existing_generation() {
        let sequence = INDEX_TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "gore-tex-index-publish-{}-{sequence}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("sealed.json");
        let first = TextureIndex {
            build_id: "sealed-source".into(),
            entries: BTreeMap::from([("/Game/T_A".into(), 1)]),
        };
        first.save_atomic_immutable(&path).unwrap();
        first.save_atomic_immutable(&path).unwrap();
        let conflicting = TextureIndex {
            build_id: "sealed-source".into(),
            entries: BTreeMap::from([("/Game/T_B".into(), 2)]),
        };
        assert!(conflicting.save_atomic_immutable(&path).is_err());
        assert_eq!(TextureIndex::load(&path).unwrap(), first);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn cache_pruning_preserves_on_call_generation_and_enforces_both_limits() {
        let sequence = INDEX_TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "gore-tex-index-prune-{}-{sequence}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let preserved = root.join("texture-index-v2-live.json");
        std::fs::write(&preserved, b"live").unwrap();
        for index in 0..9 {
            std::fs::write(
                root.join(format!("texture-index-v2-old-{index}.json")),
                b"old!",
            )
            .unwrap();
        }

        prune_cache_files(&root, "texture-index-v2-", 3, 12, &[preserved.clone()]).unwrap();

        let remaining = std::fs::read_dir(&root)
            .unwrap()
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with("texture-index-v2-")
            })
            .collect::<Vec<_>>();
        assert_eq!(remaining.len(), 3);
        assert!(preserved.exists());
        assert_eq!(
            remaining
                .iter()
                .map(|entry| entry.metadata().unwrap().len())
                .sum::<u64>(),
            12
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn managed_cache_pruning_allows_more_than_eight_sequential_generations() {
        let sequence = INDEX_TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "gore-tex-index-sequential-generations-{}-{sequence}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();

        let mut latest = None;
        for generation in 0..(MAX_MANAGED_TEXTURE_INDEX_FILES + 5) {
            let path = root.join(format!("texture-index-v2-generation-{generation:02}.json"));
            std::fs::write(&path, format!("generation-{generation}")).unwrap();
            pin_and_prune_managed_texture_cache(&path).unwrap();
            assert!(path.exists(), "the on-call generation must be preserved");
            latest = Some(path);
        }

        let remaining = std::fs::read_dir(&root)
            .unwrap()
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                let name = entry.file_name();
                let name = name.to_string_lossy();
                name.starts_with("texture-index-v2-") && name.ends_with(".json")
            })
            .collect::<Vec<_>>();
        assert!(remaining.len() <= MAX_MANAGED_TEXTURE_INDEX_FILES);
        assert!(
            remaining
                .iter()
                .map(|entry| entry.metadata().unwrap().len())
                .sum::<u64>()
                <= MAX_MANAGED_TEXTURE_INDEX_BYTES
        );
        assert!(latest.unwrap().exists());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn managed_cache_pruning_tolerates_a_disposable_current_cache_being_removed() {
        let sequence = INDEX_TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "gore-tex-index-disposable-current-{}-{sequence}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let removed = root.join("texture-index-v2-removed.json");

        pin_and_prune_managed_texture_cache(&removed).unwrap();

        let _ = std::fs::remove_dir_all(root);
    }

    fn game_dir() -> Option<std::path::PathBuf> {
        let p = std::path::PathBuf::from(r"D:\SteamLibrary\steamapps\common\Gothic 1 Remake");
        p.exists().then_some(p)
    }

    #[test]
    #[ignore = "slow: full container scan"]
    fn builds_index_from_real_container() {
        let Some(g) = game_dir() else {
            eprintln!("skip: game not installed");
            return;
        };
        let utoc = crate::paths::main_container(&g).unwrap();
        let idx = build_index(&utoc, "test-build").unwrap();
        assert!(
            idx.entries.len() > 10000,
            "expected ~13k textures, got {}",
            idx.entries.len()
        );
        let pid = idx.entries.get("/Game/UI/Textures/Common/T_HardwareCursor");
        assert!(pid.is_some() && *pid.unwrap() != 0);
    }

    #[test]
    #[ignore = "slow: unpack from real container"]
    fn id_extract_matches_path_extract() {
        let Some(g) = game_dir() else {
            eprintln!("skip");
            return;
        };
        let utoc = crate::paths::main_container(&g).unwrap();
        let usmap = crate::paths::usmap(&g).unwrap();
        let asset = "/Game/UI/Textures/Common/T_HardwareCursor";
        let tmp = std::env::temp_dir().join("gore-tex-ref");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        let ua = crate::container::unpack_asset(&utoc, &usmap, asset, &tmp).unwrap();
        let ref_info = crate::decode::parse(
            &std::fs::read(&ua).unwrap(),
            &std::fs::read(ua.with_extension("uexp")).unwrap(),
            &std::fs::read(ua.with_extension("ubulk")).unwrap_or_default(),
            &std::fs::read(&usmap).unwrap(),
        )
        .unwrap();
        let ref_px = crate::decode::to_rgba8(&ref_info).unwrap();
        let idx = build_index(&utoc, "t").unwrap();
        let pid = *idx.entries.get(asset).unwrap();
        let (info, px) = extract_by_package_id(&utoc, &usmap, pid, "T_HardwareCursor").unwrap();
        assert_eq!(info.width, ref_info.width);
        assert_eq!(info.format, ref_info.format);
        assert_eq!(px, ref_px);
    }
}
