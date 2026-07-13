//! Bounded, read-only voice-archive and standalone Ogg-inspection commands for Mod Studio.
//!
//! The source ZIP is never edited. `gore-vo` copies each no-follow source handle into a private,
//! bounded, disk-backed snapshot before parsing. Extraction verifies the list response's exact
//! size/SHA-256 seal before ZIP parsing, takes another seal-checked snapshot before payload reads,
//! and creates the destination with `create_new`.
//! Listing and line matching additionally apply FFI-specific entry, central-directory, input, and
//! serialized-response ceilings; extraction applies FFI-specific entry-path and output-byte
//! ceilings. Oversized responses fail with `VOICE_RESPONSE_LIMIT` instead of being materialized.

use std::fs::{self, File as FsFile, OpenOptions};
use std::io::{self, Read as _};
use std::path::{Path, PathBuf};

use gore_vo::{
    validate_archive_entry_path, validate_output_root_ancestors, ArchiveEntry, ArchiveIndex,
    ArchiveSeal, Error, Limits, OggCodec, OggError,
};
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest as _, Sha256};

use super::err;

const MAX_FILESYSTEM_PATH_BYTES: usize = 32 * 1024;
const MAX_FFI_ARCHIVE_ENTRIES: usize = 50_000;
const MAX_FFI_CENTRAL_DIRECTORY_BYTES: u64 = 64 * 1024 * 1024;
const MAX_FFI_ENTRY_PATH_BYTES: usize = 1024;
const MAX_FFI_ENTRY_OUTPUT_BYTES: u64 = 256 * 1024 * 1024;
const MAX_FFI_TOTAL_UNCOMPRESSED_BYTES: u64 = 16 * 1024 * 1024 * 1024;
const MAX_FFI_LIST_JSON_BYTES: usize = 8 * 1024 * 1024;
const MAX_FFI_MATCH_JSON_BYTES: usize = 1024 * 1024;
const MAX_FFI_LOC_ID_BYTES: usize = 512;
const MAX_FFI_OGG_INSPECT_JSON_BYTES: usize = 1024;
// One decoded path byte can occupy six raw JSON bytes (`\uXXXX`), plus a fixed closed envelope.
const MAX_FFI_OGG_INSPECT_REQUEST_BYTES: usize = MAX_FILESYSTEM_PATH_BYTES * 6 + 256;

#[derive(Debug)]
struct VoiceFailure {
    code: &'static str,
    message: String,
}

impl VoiceFailure {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    fn response(self) -> Value {
        err(self.code, self.message)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct OggFileIdentity {
    volume: u64,
    file: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct OggChangeStamp([i64; 4]);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct OggHandleSnapshot {
    identity: OggFileIdentity,
    byte_len: u64,
    link_count: u64,
    change_stamp: OggChangeStamp,
    is_regular: bool,
    is_reparse: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OggSourceReadError {
    Unavailable,
    Unsafe,
    Limit,
    Changed,
}

/// Stable, command-neutral classification for the guarded source reader shared by read-only
/// native commands. Details stay private so callers cannot accidentally expose native paths or
/// platform I/O messages across the FFI boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SecureSourceReadError {
    Unavailable,
    Unsafe,
    Limit,
    Changed,
}

impl From<OggSourceReadError> for SecureSourceReadError {
    fn from(error: OggSourceReadError) -> Self {
        match error {
            OggSourceReadError::Unavailable => Self::Unavailable,
            OggSourceReadError::Unsafe => Self::Unsafe,
            OggSourceReadError::Limit => Self::Limit,
            OggSourceReadError::Changed => Self::Changed,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct OggInspectWireRequest {
    command: String,
    payload: OggInspectWirePayload,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct OggInspectWirePayload {
    ogg_path: String,
}

/// `payload: {ogg_path}` -> bounded structural facts and the exact seal of the bytes validated.
///
/// This command never returns the native source path. It opens the final component without
/// following links, requires a single-link regular file, captures handle identity/size/change
/// metadata around the bounded read, and reopens the path to detect replacement.
pub(super) fn ogg_inspect_v1_raw(input: &str) -> Value {
    if input.len() > MAX_FFI_OGG_INSPECT_REQUEST_BYTES {
        return ogg_bad_request().response();
    }
    let request: OggInspectWireRequest = match serde_json::from_str(input) {
        Ok(request) => request,
        Err(_) => return ogg_bad_request().response(),
    };
    if request.command != "voice_ogg_inspect_v1" {
        return ogg_bad_request().response();
    }
    match ogg_inspect_v1_inner(&request.payload.ogg_path) {
        Ok(response) => response,
        Err(error) => error.response(),
    }
}

fn ogg_inspect_v1_inner(ogg_path: &str) -> Result<Value, VoiceFailure> {
    if ogg_path.is_empty() || ogg_path.len() > MAX_FILESYSTEM_PATH_BYTES || ogg_path.contains('\0')
    {
        return Err(ogg_bad_request());
    }
    let project_max_ogg_bytes = gore_authoring::WorkingStoreLimits::default().max_ogg_bytes;
    let limits = Limits {
        max_ogg_bytes: project_max_ogg_bytes,
        ..Limits::default()
    };
    let max_ogg_bytes = u64::try_from(project_max_ogg_bytes).map_err(|_| {
        VoiceFailure::new(
            "VOICE_OGG_LIMIT",
            "the supported Ogg byte limit cannot be represented on this platform",
        )
    })?;
    let bytes = read_ogg_source_no_follow(Path::new(ogg_path), max_ogg_bytes)
        .map_err(map_ogg_source_read_error)?;
    let info = gore_vo::validate_ogg(&bytes, &limits).map_err(map_ogg_validation_error)?;
    let codec = match info.codec {
        OggCodec::Vorbis { .. } => "vorbis",
        OggCodec::Opus { .. } => "opus",
        OggCodec::Unknown => {
            return Err(VoiceFailure::new(
                "VOICE_OGG_INVALID",
                "the source is not a supported valid Vorbis or Opus Ogg stream",
            ));
        }
    };
    let byte_len = u64::try_from(bytes.len()).map_err(|_| {
        VoiceFailure::new(
            "VOICE_OGG_LIMIT",
            "the validated Ogg byte length cannot be represented",
        )
    })?;
    let sha256 = format_sha256(Sha256::digest(&bytes).into());
    let response = json!({
        "ok": true,
        "codec": codec,
        "pages": info.pages,
        "streams": info.logical_streams,
        "content_seal": {
            "byte_len": byte_len,
            "sha256": sha256,
        },
    });
    let serialized_len = serde_json::to_vec(&response)
        .map_err(|_| {
            VoiceFailure::new(
                "VOICE_OGG_LIMIT",
                "the bounded Ogg inspection response could not be serialized",
            )
        })?
        .len();
    if serialized_len > MAX_FFI_OGG_INSPECT_JSON_BYTES {
        return Err(VoiceFailure::new(
            "VOICE_OGG_LIMIT",
            "the Ogg inspection response exceeds its resource limit",
        ));
    }
    Ok(response)
}

fn ogg_bad_request() -> VoiceFailure {
    VoiceFailure::new(
        "VOICE_OGG_REQUEST_INVALID",
        "payload must contain exactly one non-empty 'ogg_path' string within the path limit",
    )
}

fn read_ogg_source_no_follow(path: &Path, max_bytes: u64) -> Result<Vec<u8>, OggSourceReadError> {
    read_ogg_source_no_follow_with_hook(path, max_bytes, || {})
}

/// Read one bounded, single-link regular file through the hardened source-handle chain.
///
/// The implementation retains every ancestor directory handle, never follows the final
/// component, checks identity/size/link/change metadata around the read, and reopens relative to
/// the retained parent (Unix) or while all ancestor handles exclude replacement (Windows).
pub(super) fn read_bounded_single_link_source(
    path: &Path,
    max_bytes: u64,
) -> Result<Vec<u8>, SecureSourceReadError> {
    read_ogg_source_no_follow(path, max_bytes).map_err(Into::into)
}

fn read_ogg_source_no_follow_with_hook(
    path: &Path,
    max_bytes: u64,
    after_read: impl FnOnce(),
) -> Result<Vec<u8>, OggSourceReadError> {
    let path = normalized_absolute_ogg_source_path(path)?;
    let mut source = open_guarded_ogg_source(&path)?;
    let initial = source.initial;
    if initial.byte_len > max_bytes {
        return Err(OggSourceReadError::Limit);
    }
    let capacity = usize::try_from(initial.byte_len).map_err(|_| OggSourceReadError::Limit)?;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(capacity)
        .map_err(|_| OggSourceReadError::Limit)?;
    let read_limit = max_bytes.checked_add(1).ok_or(OggSourceReadError::Limit)?;
    source
        .file
        .by_ref()
        .take(read_limit)
        .read_to_end(&mut bytes)
        .map_err(|_| OggSourceReadError::Unavailable)?;
    if bytes.len() as u64 != initial.byte_len {
        return Err(OggSourceReadError::Changed);
    }
    if bytes.len() as u64 > max_bytes {
        return Err(OggSourceReadError::Limit);
    }

    after_read();

    let final_snapshot = snapshot_open_ogg_handle(&source.file).and_then(|snapshot| {
        validate_ogg_snapshot(snapshot)?;
        Ok(snapshot)
    });
    let reopened = source.reopen().map(|(_, snapshot)| snapshot);
    ensure_ogg_source_unchanged(initial, final_snapshot, reopened)?;
    Ok(bytes)
}

fn ensure_ogg_source_unchanged(
    initial: OggHandleSnapshot,
    final_snapshot: Result<OggHandleSnapshot, OggSourceReadError>,
    reopened: Result<OggHandleSnapshot, OggSourceReadError>,
) -> Result<(), OggSourceReadError> {
    let final_snapshot = final_snapshot.map_err(|_| OggSourceReadError::Changed)?;
    let reopened = reopened.map_err(|_| OggSourceReadError::Changed)?;
    if final_snapshot != initial || reopened != initial {
        return Err(OggSourceReadError::Changed);
    }
    Ok(())
}

fn normalized_absolute_ogg_source_path(path: &Path) -> Result<PathBuf, OggSourceReadError> {
    use std::path::Component;

    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|_| OggSourceReadError::Unavailable)?
            .join(path)
    };
    let mut normalized = PathBuf::new();
    for component in absolute.components() {
        match component {
            Component::Prefix(_) | Component::RootDir | Component::Normal(_) => {
                normalized.push(component.as_os_str());
            }
            Component::CurDir => {}
            Component::ParentDir => return Err(OggSourceReadError::Unsafe),
        }
    }
    if !normalized.is_absolute() {
        return Err(OggSourceReadError::Unsafe);
    }
    Ok(normalized)
}

#[cfg(windows)]
fn ogg_metadata_is_reparse_point(metadata: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt as _;
    use windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_REPARSE_POINT;

    metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

struct GuardedOggSource {
    file: FsFile,
    initial: OggHandleSnapshot,
    #[cfg(unix)]
    parent: FsFile,
    #[cfg(unix)]
    final_name: std::ffi::OsString,
    #[cfg(unix)]
    _ancestor_handles: Vec<FsFile>,
    #[cfg(windows)]
    path: PathBuf,
    #[cfg(windows)]
    _ancestor_handles: Vec<FsFile>,
}

impl GuardedOggSource {
    #[cfg(unix)]
    fn reopen(&self) -> Result<(FsFile, OggHandleSnapshot), OggSourceReadError> {
        open_unix_ogg_file_at(&self.parent, &self.final_name)
    }

    #[cfg(windows)]
    fn reopen(&self) -> Result<(FsFile, OggHandleSnapshot), OggSourceReadError> {
        open_windows_ogg_file(&self.path)
    }
}

#[cfg(unix)]
fn open_guarded_ogg_source(path: &Path) -> Result<GuardedOggSource, OggSourceReadError> {
    use std::path::Component;

    let mut names = path
        .components()
        .filter_map(|component| match component {
            Component::Normal(name) => Some(name.to_os_string()),
            _ => None,
        })
        .collect::<Vec<_>>();
    let final_name = names.pop().ok_or(OggSourceReadError::Unsafe)?;
    let mut parent = open_unix_root_directory()?;
    let mut ancestor_handles = Vec::new();
    ancestor_handles
        .try_reserve_exact(names.len())
        .map_err(|_| OggSourceReadError::Limit)?;
    for name in names {
        let next = open_unix_directory_at(&parent, &name)?;
        ancestor_handles.push(parent);
        parent = next;
    }
    let (file, initial) = open_unix_ogg_file_at(&parent, &final_name)?;
    Ok(GuardedOggSource {
        file,
        initial,
        parent,
        final_name,
        _ancestor_handles: ancestor_handles,
    })
}

#[cfg(unix)]
fn open_unix_root_directory() -> Result<FsFile, OggSourceReadError> {
    use std::os::unix::fs::OpenOptionsExt as _;

    let mut options = OpenOptions::new();
    options
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC);
    let root = options
        .open(Path::new("/"))
        .map_err(classify_ogg_open_error)?;
    validate_unix_directory_handle(&root)?;
    Ok(root)
}

#[cfg(unix)]
fn open_unix_directory_at(
    parent: &FsFile,
    name: &std::ffi::OsStr,
) -> Result<FsFile, OggSourceReadError> {
    let file = unix_openat(
        parent,
        name,
        libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC,
    )?;
    validate_unix_directory_handle(&file)?;
    Ok(file)
}

#[cfg(unix)]
fn validate_unix_directory_handle(file: &FsFile) -> Result<(), OggSourceReadError> {
    let metadata = file
        .metadata()
        .map_err(|_| OggSourceReadError::Unavailable)?;
    if !metadata.is_dir() {
        return Err(OggSourceReadError::Unsafe);
    }
    Ok(())
}

#[cfg(unix)]
fn open_unix_ogg_file_at(
    parent: &FsFile,
    name: &std::ffi::OsStr,
) -> Result<(FsFile, OggHandleSnapshot), OggSourceReadError> {
    let file = unix_openat(
        parent,
        name,
        libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC,
    )?;
    let snapshot = snapshot_open_ogg_handle(&file)?;
    validate_ogg_snapshot(snapshot)?;
    Ok((file, snapshot))
}

#[cfg(unix)]
fn unix_openat(
    parent: &FsFile,
    name: &std::ffi::OsStr,
    flags: i32,
) -> Result<FsFile, OggSourceReadError> {
    use std::os::fd::{AsRawFd as _, FromRawFd as _};
    use std::os::unix::ffi::OsStrExt as _;

    let name = std::ffi::CString::new(name.as_bytes()).map_err(|_| OggSourceReadError::Unsafe)?;
    // SAFETY: `parent` owns a valid directory fd, `name` is NUL-terminated, and no mode argument
    // is required because these read-only flags never include O_CREAT.
    let fd = unsafe { libc::openat(parent.as_raw_fd(), name.as_ptr(), flags) };
    if fd < 0 {
        return Err(classify_ogg_open_error(io::Error::last_os_error()));
    }
    // SAFETY: `openat` returned a new owned descriptor on success.
    Ok(unsafe { FsFile::from_raw_fd(fd) })
}

#[cfg(windows)]
fn open_guarded_ogg_source(path: &Path) -> Result<GuardedOggSource, OggSourceReadError> {
    let mut prefixes = path.ancestors().collect::<Vec<_>>();
    prefixes.reverse();
    let mut ancestor_handles = Vec::new();
    ancestor_handles
        .try_reserve_exact(prefixes.len().saturating_sub(1))
        .map_err(|_| OggSourceReadError::Limit)?;
    for prefix in prefixes {
        if prefix.as_os_str().is_empty() || prefix == path {
            continue;
        }
        ancestor_handles.push(open_windows_directory_no_follow(prefix)?);
    }
    let (file, initial) = open_windows_ogg_file(path)?;
    Ok(GuardedOggSource {
        file,
        initial,
        path: path.to_path_buf(),
        _ancestor_handles: ancestor_handles,
    })
}

#[cfg(windows)]
fn open_windows_directory_no_follow(path: &Path) -> Result<FsFile, OggSourceReadError> {
    use std::os::windows::fs::OpenOptionsExt as _;
    use std::os::windows::io::AsRawHandle as _;
    use windows_sys::Win32::Storage::FileSystem::{
        GetFileType, FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT, FILE_SHARE_READ,
        FILE_TYPE_DISK,
    };

    let mut options = OpenOptions::new();
    options
        .read(true)
        // Excluding WRITE/DELETE sharing keeps this exact directory object from being converted,
        // renamed, or replaced while its descendants are opened and verified.
        .share_mode(FILE_SHARE_READ)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT);
    let file = options.open(path).map_err(classify_ogg_open_error)?;
    let metadata = file
        .metadata()
        .map_err(|_| OggSourceReadError::Unavailable)?;
    // SAFETY: `file` owns a valid handle.
    let is_disk = unsafe { GetFileType(file.as_raw_handle()) } == FILE_TYPE_DISK;
    if !is_disk || !metadata.is_dir() || ogg_metadata_is_reparse_point(&metadata) {
        return Err(OggSourceReadError::Unsafe);
    }
    Ok(file)
}

#[cfg(windows)]
fn open_windows_ogg_file(path: &Path) -> Result<(FsFile, OggHandleSnapshot), OggSourceReadError> {
    let file = open_regular_ogg_handle_no_follow(path).map_err(classify_ogg_open_error)?;
    let snapshot = snapshot_open_ogg_handle(&file)?;
    validate_ogg_snapshot(snapshot)?;
    Ok((file, snapshot))
}

fn validate_ogg_snapshot(snapshot: OggHandleSnapshot) -> Result<(), OggSourceReadError> {
    if !snapshot.is_regular || snapshot.is_reparse || snapshot.link_count != 1 {
        return Err(OggSourceReadError::Unsafe);
    }
    Ok(())
}

fn classify_ogg_open_error(error: io::Error) -> OggSourceReadError {
    #[cfg(unix)]
    if let Some(code) = error.raw_os_error() {
        if matches!(code, libc::ELOOP | libc::ENOTDIR | libc::ENXIO) {
            return OggSourceReadError::Unsafe;
        }
    }
    let _ = error;
    OggSourceReadError::Unavailable
}

#[cfg(windows)]
fn open_regular_ogg_handle_no_follow(path: &Path) -> io::Result<FsFile> {
    use std::os::windows::fs::OpenOptionsExt as _;
    use windows_sys::Win32::Storage::FileSystem::{
        FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT, FILE_SHARE_READ,
    };

    let mut options = OpenOptions::new();
    options
        .read(true)
        .share_mode(FILE_SHARE_READ)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT);
    options.open(path)
}

#[cfg(windows)]
fn snapshot_open_ogg_handle(file: &FsFile) -> Result<OggHandleSnapshot, OggSourceReadError> {
    use std::os::windows::io::AsRawHandle as _;
    use windows_sys::Win32::Storage::FileSystem::{
        FileBasicInfo, GetFileInformationByHandle, GetFileInformationByHandleEx, GetFileType,
        BY_HANDLE_FILE_INFORMATION, FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_REPARSE_POINT,
        FILE_BASIC_INFO, FILE_TYPE_DISK,
    };

    let mut info = unsafe { std::mem::zeroed::<BY_HANDLE_FILE_INFORMATION>() };
    // SAFETY: `file` owns a valid handle and `info` is writable for the call.
    if unsafe { GetFileInformationByHandle(file.as_raw_handle(), &mut info) } == 0 {
        return Err(OggSourceReadError::Unavailable);
    }
    let mut basic = FILE_BASIC_INFO::default();
    // SAFETY: `file` owns a valid handle and `basic` is a correctly sized writable buffer.
    if unsafe {
        GetFileInformationByHandleEx(
            file.as_raw_handle(),
            FileBasicInfo,
            std::ptr::addr_of_mut!(basic).cast(),
            std::mem::size_of::<FILE_BASIC_INFO>() as u32,
        )
    } == 0
    {
        return Err(OggSourceReadError::Unavailable);
    }
    let metadata = file
        .metadata()
        .map_err(|_| OggSourceReadError::Unavailable)?;
    // SAFETY: `file` owns a valid handle. Non-disk handles (pipes/devices) are not regular files.
    let is_disk_file = unsafe { GetFileType(file.as_raw_handle()) } == FILE_TYPE_DISK;
    Ok(OggHandleSnapshot {
        identity: OggFileIdentity {
            volume: u64::from(info.dwVolumeSerialNumber),
            file: (u64::from(info.nFileIndexHigh) << 32) | u64::from(info.nFileIndexLow),
        },
        byte_len: (u64::from(info.nFileSizeHigh) << 32) | u64::from(info.nFileSizeLow),
        link_count: u64::from(info.nNumberOfLinks),
        change_stamp: OggChangeStamp([
            basic.ChangeTime,
            basic.LastWriteTime,
            basic.CreationTime,
            i64::from(basic.FileAttributes),
        ]),
        is_regular: is_disk_file
            && metadata.is_file()
            && info.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY == 0,
        is_reparse: info.dwFileAttributes & FILE_ATTRIBUTE_REPARSE_POINT != 0,
    })
}

#[cfg(unix)]
fn snapshot_open_ogg_handle(file: &FsFile) -> Result<OggHandleSnapshot, OggSourceReadError> {
    use std::os::unix::fs::MetadataExt as _;

    let metadata = file
        .metadata()
        .map_err(|_| OggSourceReadError::Unavailable)?;
    Ok(OggHandleSnapshot {
        identity: OggFileIdentity {
            volume: metadata.dev(),
            file: metadata.ino(),
        },
        byte_len: metadata.len(),
        link_count: metadata.nlink(),
        change_stamp: OggChangeStamp([
            metadata.mtime(),
            metadata.mtime_nsec(),
            metadata.ctime(),
            metadata.ctime_nsec(),
        ]),
        is_regular: metadata.is_file(),
        is_reparse: false,
    })
}

fn map_ogg_source_read_error(error: OggSourceReadError) -> VoiceFailure {
    match error {
        OggSourceReadError::Unavailable => VoiceFailure::new(
            "VOICE_OGG_UNAVAILABLE",
            "the Ogg source could not be opened or read",
        ),
        OggSourceReadError::Unsafe => VoiceFailure::new(
            "VOICE_OGG_UNSAFE",
            "the Ogg source is not a safe single-link regular file",
        ),
        OggSourceReadError::Limit => VoiceFailure::new(
            "VOICE_OGG_LIMIT",
            "the Ogg source exceeds the supported resource limit",
        ),
        OggSourceReadError::Changed => VoiceFailure::new(
            "VOICE_OGG_CHANGED",
            "the Ogg source changed while it was read",
        ),
    }
}

fn map_ogg_validation_error(error: OggError) -> VoiceFailure {
    match error {
        OggError::LimitExceeded { .. } => VoiceFailure::new(
            "VOICE_OGG_LIMIT",
            "the Ogg source exceeds the supported resource limit",
        ),
        _ => VoiceFailure::new(
            "VOICE_OGG_INVALID",
            "the source is not a supported valid Vorbis or Opus Ogg stream",
        ),
    }
}

/// `payload: {archive}` -> the archive's exact entry paths and bounded ZIP metadata.
pub(super) fn archive_list(payload: Value) -> Value {
    match archive_list_inner(&payload) {
        Ok(response) => response,
        Err(error) => error.response(),
    }
}

fn archive_list_inner(payload: &Value) -> Result<Value, VoiceFailure> {
    let archive = required_bounded_string(
        payload,
        "archive",
        MAX_FILESYSTEM_PATH_BYTES,
        "filesystem path",
    )?;
    let index = open_archive(Path::new(archive))?;
    let archive_size = index.archive_bytes();
    let archive_sha256 = format_sha256(index.archive_sha256());

    let (entries, total_compressed_size, total_uncompressed_size) =
        bounded_entry_json(&index, MAX_FFI_LIST_JSON_BYTES)?;

    let response = json!({
        "ok": true,
        "archive": index.path().display().to_string(),
        "archive_size": archive_size,
        "archive_sha256": archive_sha256,
        "entry_count": entries.len(),
        "total_compressed_size": total_compressed_size,
        "total_uncompressed_size": total_uncompressed_size,
        "entries": entries,
    });
    enforce_response_budget(&response, MAX_FFI_LIST_JSON_BYTES)?;
    Ok(response)
}

/// `payload: {archive, loc_id}` -> every eligible entry whose basename matches
/// `${loc_id}.ogg` under ASCII case-insensitive comparison, plus the seal captured from the same
/// bounded read-only snapshot.
///
/// Matching never considers the directory/full path, substrings, or fuzzy candidates. Multiple
/// exact basenames are returned as `ambiguous`; this command never selects one or invents a path.
pub(super) fn archive_match_line(payload: Value) -> Value {
    match archive_match_line_inner(&payload) {
        Ok(response) => response,
        Err(error) => error.response(),
    }
}

fn archive_match_line_inner(payload: &Value) -> Result<Value, VoiceFailure> {
    let archive = required_bounded_string(
        payload,
        "archive",
        MAX_FILESYSTEM_PATH_BYTES,
        "filesystem path",
    )?;
    let loc_id = required_loc_id(payload)?;
    let expected_basename = format!("{loc_id}.ogg");
    let index = open_archive(Path::new(archive))?;
    let matches =
        bounded_matching_entry_json(&index, &expected_basename, MAX_FFI_MATCH_JSON_BYTES)?;
    let resolution = match matches.len() {
        0 => "unresolved",
        1 => "unique",
        _ => "ambiguous",
    };

    let response = json!({
        "ok": true,
        "archive": index.path().display().to_string(),
        "archive_size": index.archive_bytes(),
        "archive_sha256": format_sha256(index.archive_sha256()),
        "loc_id": loc_id,
        "expected_basename": expected_basename,
        "resolution": resolution,
        "match_count": matches.len(),
        "matches": matches,
    });
    enforce_response_budget(&response, MAX_FFI_MATCH_JSON_BYTES)?;
    Ok(response)
}

/// `payload: {archive, expected_archive_size, expected_archive_sha256, entry_path, output_root}` ->
/// extract one exact, case-sensitive member from the snapshot previously returned by list.
///
/// The output is `<output_root>/<entry_path>`. It must not already exist; `gore-vo` enforces this
/// again atomically while creating the file.
pub(super) fn archive_extract(payload: Value) -> Value {
    match archive_extract_inner(&payload) {
        Ok(response) => response,
        Err(error) => error.response(),
    }
}

fn archive_extract_inner(payload: &Value) -> Result<Value, VoiceFailure> {
    let limits = ffi_limits();
    let archive = required_bounded_string(
        payload,
        "archive",
        MAX_FILESYSTEM_PATH_BYTES,
        "filesystem path",
    )?;
    let expected_archive_size = required_u64(payload, "expected_archive_size")?;
    let expected_archive_sha256 = required_sha256(payload, "expected_archive_sha256")?;
    let entry_path = required_bounded_string(
        payload,
        "entry_path",
        limits.max_path_bytes,
        "archive entry path",
    )?;
    let output_root = required_bounded_string(
        payload,
        "output_root",
        MAX_FILESYSTEM_PATH_BYTES,
        "filesystem path",
    )?;

    let index = ArchiveIndex::open_with_expected_seal(
        Path::new(archive),
        ffi_limits(),
        ArchiveSeal {
            size: expected_archive_size,
            sha256: expected_archive_sha256,
        },
    )
    .map_err(map_voice_error)?;
    let entry = index
        .resolve("", Some(entry_path))
        .map_err(map_voice_error)?;
    if entry.is_directory {
        return Err(VoiceFailure::new(
            "VOICE_ENTRY_NOT_FILE",
            format!("archive entry is a directory: {:?}", entry.path),
        ));
    }
    validate_archive_entry_path(&entry.path, &limits).map_err(map_voice_error)?;

    let output_root = PathBuf::from(output_root);
    validate_output_root_ancestors(&output_root).map_err(map_voice_error)?;
    let expected_output = entry
        .path
        .split('/')
        .fold(output_root.clone(), |path, component| path.join(component));
    match fs::symlink_metadata(&expected_output) {
        Ok(_) => {
            return Err(VoiceFailure::new(
                "VOICE_OUTPUT_EXISTS",
                format!(
                    "output already exists (refusing to overwrite it): {}",
                    expected_output.display()
                ),
            ));
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(map_output_io(error)),
    }
    if let Some(parent) = expected_output.parent() {
        validate_output_root_ancestors(parent).map_err(map_voice_error)?;
    }

    // This performs the authoritative archive hash/metadata recheck and opens the destination
    // with create_new. Passing exact_path makes selection case-sensitive.
    let output = index
        .extract("", Some(entry_path), &output_root)
        .map_err(map_voice_error)?;

    Ok(json!({
        "ok": true,
        "archive": index.path().display().to_string(),
        "archive_size": index.archive_bytes(),
        "archive_sha256": format_sha256(index.archive_sha256()),
        "entry_path": entry.path,
        "output": output.display().to_string(),
        "uncompressed_size": entry.uncompressed_size,
        "crc32": entry.crc32,
    }))
}

fn entry_json(entry: &ArchiveEntry) -> Value {
    #[allow(deprecated)]
    let compression_code = entry.compression.to_u16();
    let compression = match compression_code {
        0 => "stored",
        8 => "deflated",
        9 => "deflate64",
        12 => "bzip2",
        14 => "lzma",
        93 => "zstd",
        95 => "xz",
        99 => "aes",
        _ => "unsupported",
    };
    let last_modified = entry.last_modified.map(|timestamp| {
        json!({
            "year": timestamp.year(),
            "month": timestamp.month(),
            "day": timestamp.day(),
            "hour": timestamp.hour(),
            "minute": timestamp.minute(),
            "second": timestamp.second(),
        })
    });

    json!({
        "index": entry.index,
        "path": entry.path,
        "basename": entry.basename,
        "compressed_size": entry.compressed_size,
        "uncompressed_size": entry.uncompressed_size,
        "crc32": entry.crc32,
        "compression": compression,
        "compression_code": compression_code,
        "last_modified": last_modified,
        "unix_mode": entry.unix_mode,
        "is_directory": entry.is_directory,
        "is_symlink": entry.is_symlink,
        "encrypted": entry.encrypted,
    })
}

fn bounded_entry_json(
    index: &ArchiveIndex,
    max_json_bytes: usize,
) -> Result<(Vec<Value>, u64, u64), VoiceFailure> {
    let mut entries = Vec::new();
    entries
        .try_reserve_exact(index.entries().len())
        .map_err(|error| {
            VoiceFailure::new(
                "VOICE_RESPONSE_LIMIT",
                format!("reserving bounded voice entry response: {error}"),
            )
        })?;
    let mut serialized_entries = 2usize; // JSON array brackets.
    let mut total_compressed_size = 0u64;
    let mut total_uncompressed_size = 0u64;
    for entry in index.list() {
        total_compressed_size = total_compressed_size
            .checked_add(entry.compressed_size)
            .ok_or_else(|| {
                VoiceFailure::new(
                    "VOICE_ARCHIVE_LIMIT",
                    "compressed entry-size total overflowed u64",
                )
            })?;
        total_uncompressed_size = total_uncompressed_size
            .checked_add(entry.uncompressed_size)
            .ok_or_else(|| {
                VoiceFailure::new(
                    "VOICE_ARCHIVE_LIMIT",
                    "uncompressed entry-size total overflowed u64",
                )
            })?;
        let value = entry_json(entry);
        let value_bytes = serde_json::to_vec(&value).map_err(|error| {
            VoiceFailure::new(
                "VOICE_SERIALIZE",
                format!("serializing voice archive entry failed: {error}"),
            )
        })?;
        serialized_entries = serialized_entries
            .checked_add(value_bytes.len())
            .and_then(|size| size.checked_add(usize::from(!entries.is_empty())))
            .ok_or_else(|| response_limit_failure(usize::MAX, max_json_bytes))?;
        if serialized_entries > max_json_bytes {
            return Err(response_limit_failure(serialized_entries, max_json_bytes));
        }
        entries.push(value);
    }
    Ok((entries, total_compressed_size, total_uncompressed_size))
}

fn bounded_matching_entry_json(
    index: &ArchiveIndex,
    expected_basename: &str,
    max_json_bytes: usize,
) -> Result<Vec<Value>, VoiceFailure> {
    let mut matches = Vec::new();
    let mut serialized_matches = 2usize; // JSON array brackets.
    for entry in index.list() {
        // Deliberately compare only the basename. Directory components cannot make an entry a
        // candidate, and exact ASCII case-insensitive equality rejects every substring/fuzzy
        // spelling.
        if !ascii_case_equal(&entry.basename, expected_basename) {
            continue;
        }
        validate_matching_voice_entry(entry)?;
        let value = entry_json(entry);
        let value_bytes = serde_json::to_vec(&value).map_err(|error| {
            VoiceFailure::new(
                "VOICE_SERIALIZE",
                format!("serializing matching voice archive entry failed: {error}"),
            )
        })?;
        serialized_matches = serialized_matches
            .checked_add(value_bytes.len())
            .and_then(|size| size.checked_add(usize::from(!matches.is_empty())))
            .ok_or_else(|| response_limit_failure(usize::MAX, max_json_bytes))?;
        if serialized_matches > max_json_bytes {
            return Err(response_limit_failure(serialized_matches, max_json_bytes));
        }
        matches.push(value);
    }
    Ok(matches)
}

fn ascii_case_equal(left: &str, right: &str) -> bool {
    left.is_ascii() && right.is_ascii() && left.eq_ignore_ascii_case(right)
}

fn validate_matching_voice_entry(entry: &ArchiveEntry) -> Result<(), VoiceFailure> {
    validate_archive_entry_path(&entry.path, &ffi_limits()).map_err(map_voice_error)?;
    if entry.is_directory {
        return Err(VoiceFailure::new(
            "VOICE_ENTRY_NOT_FILE",
            format!("exact voice match is a directory: {:?}", entry.path),
        ));
    }
    if entry.is_symlink {
        return Err(VoiceFailure::new(
            "VOICE_ENTRY_SYMLINK",
            format!("exact voice match is a symbolic link: {:?}", entry.path),
        ));
    }
    if entry.encrypted {
        return Err(VoiceFailure::new(
            "VOICE_ENTRY_ENCRYPTED",
            format!("exact voice match is encrypted: {:?}", entry.path),
        ));
    }
    if let Some(mode) = entry.unix_mode {
        let file_type = mode & 0o170000;
        if file_type != 0 && file_type != 0o100000 {
            return Err(VoiceFailure::new(
                "VOICE_ENTRY_NOT_FILE",
                format!("exact voice match is not a regular file: {:?}", entry.path),
            ));
        }
    }
    if !entry.basename.to_ascii_lowercase().ends_with(".ogg") {
        return Err(VoiceFailure::new(
            "VOICE_ENTRY_NOT_OGG",
            format!("exact voice match is not an Ogg member: {:?}", entry.path),
        ));
    }

    #[allow(deprecated)]
    let compression_code = entry.compression.to_u16();
    if !matches!(compression_code, 0 | 8) {
        return Err(VoiceFailure::new(
            "VOICE_COMPRESSION_UNSUPPORTED",
            format!(
                "exact voice match uses unsupported compression method {compression_code}: {:?}",
                entry.path
            ),
        ));
    }
    Ok(())
}

fn enforce_response_budget(response: &Value, max_json_bytes: usize) -> Result<(), VoiceFailure> {
    let actual = serde_json::to_vec(response)
        .map_err(|error| {
            VoiceFailure::new(
                "VOICE_SERIALIZE",
                format!("serializing voice response failed: {error}"),
            )
        })?
        .len();
    if actual > max_json_bytes {
        return Err(response_limit_failure(actual, max_json_bytes));
    }
    Ok(())
}

fn response_limit_failure(actual: usize, limit: usize) -> VoiceFailure {
    VoiceFailure::new(
        "VOICE_RESPONSE_LIMIT",
        format!("voice archive response exceeds JSON limit: {actual} > {limit} bytes"),
    )
}

fn required_loc_id(payload: &Value) -> Result<&str, VoiceFailure> {
    let loc_id =
        required_bounded_string(payload, "loc_id", MAX_FFI_LOC_ID_BYTES, "localization ID")?;
    if !loc_id.is_ascii()
        || loc_id.trim() != loc_id
        || loc_id == "."
        || loc_id == ".."
        || loc_id.contains('/')
        || loc_id.contains('\\')
        || loc_id.chars().any(char::is_control)
    {
        return Err(VoiceFailure::new(
            "VOICE_BAD_REQUEST",
            "field 'loc_id' must be one trimmed ASCII, non-control basename stem without path separators",
        ));
    }
    let basename_bytes = loc_id.len().checked_add(".ogg".len()).ok_or_else(|| {
        VoiceFailure::new(
            "VOICE_BAD_REQUEST",
            "field 'loc_id' overflows the archive entry path limit",
        )
    })?;
    if basename_bytes > MAX_FFI_ENTRY_PATH_BYTES {
        return Err(VoiceFailure::new(
            "VOICE_BAD_REQUEST",
            format!(
                "field 'loc_id' plus '.ogg' exceeds the archive entry path limit: {basename_bytes} > {MAX_FFI_ENTRY_PATH_BYTES} bytes"
            ),
        ));
    }
    Ok(loc_id)
}

fn required_bounded_string<'a>(
    payload: &'a Value,
    field: &str,
    max_bytes: usize,
    kind: &str,
) -> Result<&'a str, VoiceFailure> {
    let value = payload.get(field).and_then(Value::as_str).ok_or_else(|| {
        VoiceFailure::new(
            "VOICE_BAD_REQUEST",
            format!("missing/invalid string field '{field}'"),
        )
    })?;
    if value.is_empty() {
        return Err(VoiceFailure::new(
            "VOICE_BAD_REQUEST",
            format!("field '{field}' must not be empty"),
        ));
    }
    if value.len() > max_bytes {
        return Err(VoiceFailure::new(
            "VOICE_BAD_REQUEST",
            format!(
                "field '{field}' exceeds the {kind} limit: {} > {max_bytes} bytes",
                value.len()
            ),
        ));
    }
    if value.contains('\0') {
        return Err(VoiceFailure::new(
            "VOICE_BAD_REQUEST",
            format!("field '{field}' contains NUL"),
        ));
    }
    Ok(value)
}

fn required_u64(payload: &Value, field: &str) -> Result<u64, VoiceFailure> {
    payload.get(field).and_then(Value::as_u64).ok_or_else(|| {
        VoiceFailure::new(
            "VOICE_BAD_REQUEST",
            format!("missing/invalid unsigned integer field '{field}'"),
        )
    })
}

fn required_sha256(payload: &Value, field: &str) -> Result<[u8; 32], VoiceFailure> {
    let value = payload.get(field).and_then(Value::as_str).ok_or_else(|| {
        VoiceFailure::new(
            "VOICE_BAD_REQUEST",
            format!("missing/invalid SHA-256 string field '{field}'"),
        )
    })?;
    if value.len() != 64 || !value.is_ascii() {
        return Err(VoiceFailure::new(
            "VOICE_BAD_REQUEST",
            format!("field '{field}' must contain exactly 64 hexadecimal characters"),
        ));
    }

    let mut digest = [0u8; 32];
    for (index, pair) in value.as_bytes().chunks_exact(2).enumerate() {
        let high = hex_nibble(pair[0]).ok_or_else(|| {
            VoiceFailure::new(
                "VOICE_BAD_REQUEST",
                format!("field '{field}' contains a non-hexadecimal character"),
            )
        })?;
        let low = hex_nibble(pair[1]).ok_or_else(|| {
            VoiceFailure::new(
                "VOICE_BAD_REQUEST",
                format!("field '{field}' contains a non-hexadecimal character"),
            )
        })?;
        digest[index] = (high << 4) | low;
    }
    Ok(digest)
}

fn hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn format_sha256(digest: [u8; 32]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut value = String::with_capacity(64);
    for byte in digest {
        value.push(char::from(HEX[usize::from(byte >> 4)]));
        value.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    value
}

fn open_archive(path: &Path) -> Result<ArchiveIndex, VoiceFailure> {
    ArchiveIndex::open(path, ffi_limits()).map_err(map_voice_error)
}

fn ffi_limits() -> Limits {
    Limits {
        max_entries: MAX_FFI_ARCHIVE_ENTRIES,
        max_central_directory_bytes: MAX_FFI_CENTRAL_DIRECTORY_BYTES,
        max_path_bytes: MAX_FFI_ENTRY_PATH_BYTES,
        max_entry_uncompressed_bytes: MAX_FFI_ENTRY_OUTPUT_BYTES,
        max_total_uncompressed_bytes: MAX_FFI_TOTAL_UNCOMPRESSED_BYTES,
        ..Limits::default()
    }
}

fn map_output_io(error: io::Error) -> VoiceFailure {
    if error.kind() == io::ErrorKind::AlreadyExists {
        VoiceFailure::new(
            "VOICE_OUTPUT_EXISTS",
            "output already exists (refusing to overwrite it)",
        )
    } else {
        VoiceFailure::new(
            "VOICE_OUTPUT_IO",
            format!("voice extraction I/O failed: {error}"),
        )
    }
}

fn map_voice_error(error: Error) -> VoiceFailure {
    let code = match &error {
        Error::Io(io_error) if io_error.kind() == io::ErrorKind::AlreadyExists => {
            "VOICE_OUTPUT_EXISTS"
        }
        Error::Io(_) => "VOICE_IO",
        Error::SourceIo { .. } => "VOICE_SOURCE_IO",
        Error::OutputIo { .. } => "VOICE_OUTPUT_IO",
        Error::Zip(_) => "VOICE_ARCHIVE_INVALID",
        Error::ArchiveData { .. } => "VOICE_ARCHIVE_INVALID",
        Error::LimitExceeded { .. } => "VOICE_ARCHIVE_LIMIT",
        Error::NotFound { .. } => "VOICE_ENTRY_NOT_FOUND",
        Error::Ambiguous { .. } => "VOICE_ENTRY_AMBIGUOUS",
        Error::UnsafePath { .. } => "VOICE_ENTRY_UNSAFE",
        Error::UnsafeSource { .. } => "VOICE_SOURCE_NOT_REGULAR",
        Error::UnsafeOutput { .. } => "VOICE_OUTPUT_UNSAFE",
        Error::EncryptedEntry(_) => "VOICE_ENTRY_ENCRYPTED",
        Error::SymlinkEntry(_) => "VOICE_ENTRY_SYMLINK",
        Error::InputOutputSame(_) => "VOICE_INPUT_OUTPUT_SAME",
        Error::OutputExists(_) => "VOICE_OUTPUT_EXISTS",
        Error::EntryAlreadyExists(_) => "VOICE_ENTRY_EXISTS",
        Error::EmptyEditBatch | Error::ConflictingEdits { .. } | Error::NotOggPath(_) => {
            "VOICE_BAD_REQUEST"
        }
        Error::UnsupportedCompression { .. } => "VOICE_COMPRESSION_UNSUPPORTED",
        Error::ArchiveChanged => "VOICE_ARCHIVE_CHANGED",
        Error::Verification(_) => "VOICE_VERIFICATION_FAILED",
        Error::InvalidOgg(_) => "VOICE_OGG_INVALID",
    };
    VoiceFailure::new(code, error.to_string())
}

#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::io::Write;

    use tempfile::TempDir;
    use zip::write::SimpleFileOptions;
    use zip::{CompressionMethod, ZipWriter};

    use super::*;
    use crate::execute_json;

    fn call(command: &str, payload: Value) -> Value {
        let request = json!({"command": command, "payload": payload});
        serde_json::from_str(&execute_json(&request.to_string())).unwrap()
    }

    fn make_voice_archive(temp: &TempDir) -> (PathBuf, Vec<u8>) {
        let archive = temp.path().join("voices.zip");
        let ogg = write_voice_archive(&archive, 44_100);
        (archive, ogg)
    }

    fn find_eocd(bytes: &[u8]) -> usize {
        bytes
            .windows(4)
            .rposition(|window| window == 0x0605_4b50u32.to_le_bytes())
            .expect("fixture EOCD")
    }

    fn find_first_central_header(bytes: &[u8]) -> usize {
        let eocd = find_eocd(bytes);
        let size = u32::from_le_bytes(bytes[eocd + 12..eocd + 16].try_into().unwrap()) as usize;
        let offset = eocd - size;
        assert_eq!(
            bytes[offset..offset + 4],
            0x0201_4b50u32.to_le_bytes(),
            "fixture central-directory header"
        );
        offset
    }

    fn first_entry_layout(bytes: &[u8]) -> (usize, usize, usize, usize) {
        let eocd = find_eocd(bytes);
        let central = find_first_central_header(bytes);
        let directory_relative =
            u32::from_le_bytes(bytes[eocd + 16..eocd + 20].try_into().unwrap()) as usize;
        let archive_offset = central - directory_relative;
        let local_relative =
            u32::from_le_bytes(bytes[central + 42..central + 46].try_into().unwrap()) as usize;
        let local = archive_offset + local_relative;
        assert_eq!(bytes[local..local + 4], 0x0403_4b50u32.to_le_bytes());
        let name_len =
            u16::from_le_bytes(bytes[local + 26..local + 28].try_into().unwrap()) as usize;
        let extra_len =
            u16::from_le_bytes(bytes[local + 28..local + 30].try_into().unwrap()) as usize;
        let payload = local + 30 + name_len + extra_len;
        let compressed =
            u32::from_le_bytes(bytes[central + 20..central + 24].try_into().unwrap()) as usize;
        (central, local, payload, compressed)
    }

    fn write_voice_archive(archive: &Path, sample_rate: u32) -> Vec<u8> {
        let ogg = synthetic_vorbis_ogg(sample_rate);
        gore_vo::validate_ogg(&ogg, &Limits::default()).unwrap();

        let file = File::create(archive).unwrap();
        let mut writer = ZipWriter::new(file);
        writer
            .start_file(
                "Voices/Hero/LINE.ogg",
                SimpleFileOptions::default().compression_method(CompressionMethod::Deflated),
            )
            .unwrap();
        writer.write_all(&ogg).unwrap();
        writer
            .start_file(
                "notes.txt",
                SimpleFileOptions::default().compression_method(CompressionMethod::Stored),
            )
            .unwrap();
        writer.write_all(b"metadata").unwrap();
        writer.finish().unwrap();
        ogg
    }

    fn write_named_archive(archive: &Path, entries: &[&str]) {
        let file = File::create(archive).unwrap();
        let mut writer = ZipWriter::new(file);
        for (index, path) in entries.iter().enumerate() {
            writer
                .start_file(
                    *path,
                    SimpleFileOptions::default().compression_method(CompressionMethod::Stored),
                )
                .unwrap();
            writer
                .write_all(format!("fixture-{index}").as_bytes())
                .unwrap();
        }
        writer.finish().unwrap();
    }

    fn match_line_payload(archive: &Path, loc_id: &str) -> Value {
        json!({
            "archive": archive.display().to_string(),
            "loc_id": loc_id,
        })
    }

    fn assert_match_line_rejected_read_only(archive: &Path, expected_code: &str) {
        let before = fs::read(archive).unwrap();
        let response = call(
            "voice_archive_match_line",
            match_line_payload(archive, "LINE"),
        );
        assert_eq!(response["ok"], false, "response: {response}");
        assert_eq!(response["error"]["code"], expected_code);
        assert_eq!(fs::read(archive).unwrap(), before);
    }

    fn listed_seal(archive: &Path) -> (u64, String) {
        let response = call(
            "voice_archive_list",
            json!({"archive": archive.display().to_string()}),
        );
        assert_eq!(response["ok"], true, "response: {response}");
        (
            response["archive_size"].as_u64().unwrap(),
            response["archive_sha256"].as_str().unwrap().to_owned(),
        )
    }

    fn extract_payload(
        archive: &Path,
        entry_path: &str,
        output_root: &Path,
        seal: &(u64, String),
    ) -> Value {
        json!({
            "archive": archive.display().to_string(),
            "expected_archive_size": seal.0,
            "expected_archive_sha256": seal.1,
            "entry_path": entry_path,
            "output_root": output_root.display().to_string(),
        })
    }

    #[test]
    fn ogg_inspect_accepts_valid_vorbis_and_opus_with_exact_bounded_schema() {
        let temp = tempfile::tempdir().unwrap();
        let fixtures = [
            ("vorbis", synthetic_vorbis_ogg(44_100)),
            (
                "opus",
                include_bytes!("../../gore-vo/testdata/tiny-opus.ogg").to_vec(),
            ),
        ];

        for (index, (codec, bytes)) in fixtures.into_iter().enumerate() {
            let path = temp.path().join(format!("voice-{index}.ogg"));
            fs::write(&path, &bytes).unwrap();
            let info = gore_vo::validate_ogg(&bytes, &Limits::default()).unwrap();
            let sha256 = format_sha256(Sha256::digest(&bytes).into());

            let response = call(
                "voice_ogg_inspect_v1",
                json!({"ogg_path": path.display().to_string()}),
            );

            assert_eq!(
                response,
                json!({
                    "ok": true,
                    "codec": codec,
                    "pages": info.pages,
                    "streams": info.logical_streams,
                    "content_seal": {
                        "byte_len": bytes.len(),
                        "sha256": sha256,
                    },
                })
            );
            assert!(serde_json::to_vec(&response).unwrap().len() <= MAX_FFI_OGG_INSPECT_JSON_BYTES);
            assert!(!response
                .to_string()
                .contains(path.to_string_lossy().as_ref()));
        }
    }

    #[test]
    fn ogg_inspect_rejects_non_exact_or_unbounded_payloads() {
        let oversized = "x".repeat(MAX_FILESYSTEM_PATH_BYTES + 1);
        for payload in [
            Value::Null,
            json!([]),
            json!({}),
            json!({"ogg_path": 7}),
            json!({"ogg_path": ""}),
            json!({"ogg_path": "a\0b"}),
            json!({"ogg_path": oversized}),
            json!({"ogg_path": "voice.ogg", "extra": true}),
            json!({"other": "voice.ogg"}),
        ] {
            assert_eq!(
                call("voice_ogg_inspect_v1", payload),
                json!({
                    "ok": false,
                    "error": {
                        "code": "VOICE_OGG_REQUEST_INVALID",
                        "message": "payload must contain exactly one non-empty 'ogg_path' string within the path limit",
                    },
                })
            );
        }

        let raw_bad_request = json!({
            "ok": false,
            "error": {
                "code": "VOICE_OGG_REQUEST_INVALID",
                "message": "payload must contain exactly one non-empty 'ogg_path' string within the path limit",
            },
        });
        for raw in [
            r#"{"command":"voice_ogg_inspect_v1","payload":{"ogg_path":"first.ogg","ogg_path":"second.ogg"}}"#,
            r#"{"command":"voice_ogg_inspect_v1","payload":{"ogg_path":"first.ogg"},"payload":{"ogg_path":"second.ogg"}}"#,
            r#"{"command":"voice_ogg_inspect_v1","command":"voice_ogg_inspect_v1","payload":{"ogg_path":"voice.ogg"}}"#,
            r#"{"command":"voice_ogg_inspect_v1","payload":{"ogg_path":"voice.ogg"},"extra":true}"#,
        ] {
            let response: Value = serde_json::from_str(&execute_json(raw)).unwrap();
            assert_eq!(response, raw_bad_request);
        }

        let escaped_at_decoded_limit = "\\u0061".repeat(MAX_FILESYSTEM_PATH_BYTES);
        let raw_at_decoded_limit = format!(
            r#"{{"command":"voice_ogg_inspect_v1","payload":{{"ogg_path":"{escaped_at_decoded_limit}"}}}}"#
        );
        assert!(raw_at_decoded_limit.len() <= MAX_FFI_OGG_INSPECT_REQUEST_BYTES);
        let response: Value = serde_json::from_str(&execute_json(&raw_at_decoded_limit)).unwrap();
        assert_eq!(response["error"]["code"], "VOICE_OGG_UNAVAILABLE");

        let escaped_over_decoded_limit = "\\u0061".repeat(MAX_FILESYSTEM_PATH_BYTES + 1);
        let raw_over_decoded_limit = format!(
            r#"{{"command":"voice_ogg_inspect_v1","payload":{{"ogg_path":"{escaped_over_decoded_limit}"}}}}"#
        );
        assert!(raw_over_decoded_limit.len() <= MAX_FFI_OGG_INSPECT_REQUEST_BYTES);
        let response: Value = serde_json::from_str(&execute_json(&raw_over_decoded_limit)).unwrap();
        assert_eq!(response, raw_bad_request);

        let raw_over_command_limit = format!(
            r#"{{"command":"voice_ogg_inspect_v1","payload":{{"ogg_path":"{}"}}}}"#,
            "x".repeat(MAX_FFI_OGG_INSPECT_REQUEST_BYTES)
        );
        assert!(raw_over_command_limit.len() > MAX_FFI_OGG_INSPECT_REQUEST_BYTES);
        let response: Value = serde_json::from_str(&execute_json(&raw_over_command_limit)).unwrap();
        assert_eq!(response, raw_bad_request);
    }

    #[test]
    fn ogg_inspect_maps_unavailable_and_invalid_without_leaking_native_paths() {
        let temp = tempfile::tempdir().unwrap();
        let missing = temp.path().join("private-missing-voice.ogg");
        let unavailable = call(
            "voice_ogg_inspect_v1",
            json!({"ogg_path": missing.display().to_string()}),
        );
        assert_eq!(unavailable["error"]["code"], "VOICE_OGG_UNAVAILABLE");
        assert!(!unavailable
            .to_string()
            .contains(missing.to_string_lossy().as_ref()));

        let invalid_path = temp.path().join("private-invalid-voice.ogg");
        fs::write(&invalid_path, b"not an Ogg stream").unwrap();
        let invalid = call(
            "voice_ogg_inspect_v1",
            json!({"ogg_path": invalid_path.display().to_string()}),
        );
        assert_eq!(
            invalid,
            json!({
                "ok": false,
                "error": {
                    "code": "VOICE_OGG_INVALID",
                    "message": "the source is not a supported valid Vorbis or Opus Ogg stream",
                },
            })
        );
        assert!(!invalid
            .to_string()
            .contains(invalid_path.to_string_lossy().as_ref()));
    }

    #[test]
    fn ogg_guarded_reader_enforces_limit_and_detects_post_read_drift() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("changing.ogg");
        fs::write(&path, b"1234").unwrap();
        assert_eq!(
            read_ogg_source_no_follow(&path, 3),
            Err(OggSourceReadError::Limit)
        );

        fs::write(&path, b"initial bytes").unwrap();
        #[cfg(unix)]
        assert_eq!(
            read_ogg_source_no_follow_with_hook(&path, 1024, || {
                fs::write(&path, b"changed bytes with a different length").unwrap();
            }),
            Err(OggSourceReadError::Changed)
        );
        // Windows denies concurrent write/delete access while the production handle is alive;
        // every platform also exercises the common post-read drift classifier directly.
        let source = open_guarded_ogg_source(&path).unwrap();
        let initial = source.initial;
        let mut grown = initial;
        grown.byte_len += 1;
        let mut truncated = initial;
        truncated.byte_len -= 1;
        let mut replaced = initial;
        replaced.identity.file ^= 1;
        for result in [
            ensure_ogg_source_unchanged(initial, Ok(grown), Ok(initial)),
            ensure_ogg_source_unchanged(initial, Ok(truncated), Ok(initial)),
            ensure_ogg_source_unchanged(initial, Ok(replaced), Ok(replaced)),
            ensure_ogg_source_unchanged(initial, Ok(initial), Err(OggSourceReadError::Unavailable)),
            ensure_ogg_source_unchanged(initial, Err(OggSourceReadError::Unsafe), Ok(initial)),
        ] {
            assert_eq!(result, Err(OggSourceReadError::Changed));
        }
        assert_eq!(
            map_ogg_source_read_error(OggSourceReadError::Changed).response()["error"]["code"],
            "VOICE_OGG_CHANGED"
        );
    }

    #[test]
    fn ogg_inspect_rejects_multi_link_and_symlink_or_reparse_sources() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source.ogg");
        fs::write(&source, synthetic_vorbis_ogg(44_100)).unwrap();
        let hard_link = temp.path().join("hard-link.ogg");
        fs::hard_link(&source, &hard_link).unwrap();
        let multi_link = call(
            "voice_ogg_inspect_v1",
            json!({"ogg_path": source.display().to_string()}),
        );
        assert_eq!(multi_link["error"]["code"], "VOICE_OGG_UNSAFE");
        assert!(!multi_link
            .to_string()
            .contains(source.to_string_lossy().as_ref()));

        fs::remove_file(&hard_link).unwrap();
        let link = temp.path().join("symbolic-link.ogg");
        #[cfg(unix)]
        std::os::unix::fs::symlink(&source, &link).unwrap();
        #[cfg(windows)]
        if std::os::windows::fs::symlink_file(&source, &link).is_err() {
            return;
        }
        let linked = call(
            "voice_ogg_inspect_v1",
            json!({"ogg_path": link.display().to_string()}),
        );
        assert_eq!(linked["error"]["code"], "VOICE_OGG_UNSAFE");
        assert!(!linked.to_string().contains(link.to_string_lossy().as_ref()));
    }

    #[test]
    fn ogg_inspect_rejects_symlink_or_junction_ancestors() {
        let temp = tempfile::tempdir().unwrap();
        let outside = temp.path().join("outside");
        let linked_parent = temp.path().join("linked-parent");
        fs::create_dir(&outside).unwrap();
        let source = outside.join("voice.ogg");
        fs::write(&source, synthetic_vorbis_ogg(44_100)).unwrap();

        #[cfg(unix)]
        std::os::unix::fs::symlink(&outside, &linked_parent).unwrap();
        #[cfg(windows)]
        {
            let status = std::process::Command::new("cmd")
                .args(["/c", "mklink", "/J"])
                .arg(&linked_parent)
                .arg(&outside)
                .status()
                .unwrap();
            if !status.success() {
                return;
            }
        }

        let requested = linked_parent.join("voice.ogg");
        let response = call(
            "voice_ogg_inspect_v1",
            json!({"ogg_path": requested.display().to_string()}),
        );
        assert_eq!(response["error"]["code"], "VOICE_OGG_UNSAFE");
        assert!(!response
            .to_string()
            .contains(requested.to_string_lossy().as_ref()));

        #[cfg(unix)]
        fs::remove_file(&linked_parent).unwrap();
        #[cfg(windows)]
        fs::remove_dir(&linked_parent).unwrap();
    }

    #[test]
    fn guarded_ancestor_handles_bind_the_source_through_reopen() {
        let temp = tempfile::tempdir().unwrap();
        let parent = temp.path().join("guarded-parent");
        let moved = temp.path().join("moved-parent");
        fs::create_dir(&parent).unwrap();
        let source = parent.join("voice.ogg");
        fs::write(&source, synthetic_vorbis_ogg(44_100)).unwrap();
        let guarded = open_guarded_ogg_source(&source).unwrap();

        #[cfg(windows)]
        {
            // Every root-to-parent handle excludes FILE_SHARE_DELETE, so Windows cannot swap an
            // opened ancestor until verification/reopen releases the complete guard.
            assert!(fs::rename(&parent, &moved).is_err());
            let (reopened, snapshot) = guarded.reopen().unwrap();
            assert_eq!(snapshot, guarded.initial);
            drop(reopened);
            drop(guarded);
            fs::rename(&parent, &moved).unwrap();
        }

        #[cfg(unix)]
        {
            // Unix permits the lexical path to move, but reopen remains relative to the retained
            // parent dirfd and therefore resolves the original file, never the replacement path.
            fs::rename(&parent, &moved).unwrap();
            let replacement = temp.path().join("replacement-parent");
            fs::create_dir(&replacement).unwrap();
            fs::write(replacement.join("voice.ogg"), b"replacement").unwrap();
            std::os::unix::fs::symlink(&replacement, &parent).unwrap();
            let (_reopened, snapshot) = guarded.reopen().unwrap();
            assert_eq!(snapshot, guarded.initial);
            fs::remove_file(&parent).unwrap();
        }
    }

    #[test]
    fn archive_list_returns_exact_paths_and_metadata() {
        let temp = tempfile::tempdir().unwrap();
        let (archive, ogg) = make_voice_archive(&temp);

        let response = call(
            "voice_archive_list",
            json!({"archive": archive.display().to_string()}),
        );

        assert_eq!(response["ok"], true, "response: {response}");
        assert!(response["archive_size"].as_u64().unwrap() > 0);
        assert_eq!(response["archive_sha256"].as_str().unwrap().len(), 64);
        assert_eq!(response["entry_count"], 2);
        assert_eq!(response["entries"][0]["path"], "Voices/Hero/LINE.ogg");
        assert_eq!(response["entries"][0]["basename"], "LINE.ogg");
        assert_eq!(response["entries"][0]["uncompressed_size"], ogg.len());
        assert_eq!(response["entries"][0]["compression"], "deflated");
        assert_eq!(response["entries"][0]["is_directory"], false);
        assert!(response["entries"][0]["last_modified"].is_object());
        assert_eq!(response["entries"][1]["path"], "notes.txt");
        assert_eq!(response["entries"][1]["compression"], "stored");
    }

    #[test]
    fn archive_match_line_unique_casefold_returns_metadata_and_is_read_only() {
        let temp = tempfile::tempdir().unwrap();
        let archive = temp.path().join("unique.zip");
        write_named_archive(
            &archive,
            &["Voices/Hero/LINE_ONE.OGG", "Voices/Hero/OTHER.ogg"],
        );
        let before = fs::read(&archive).unwrap();
        let listed_seal = listed_seal(&archive);

        let response = call(
            "voice_archive_match_line",
            match_line_payload(&archive, "line_one"),
        );

        assert_eq!(response["ok"], true, "response: {response}");
        assert_eq!(response["archive_size"], listed_seal.0);
        assert_eq!(response["archive_sha256"], listed_seal.1);
        assert_eq!(response["loc_id"], "line_one");
        assert_eq!(response["expected_basename"], "line_one.ogg");
        assert_eq!(response["resolution"], "unique");
        assert_eq!(response["match_count"], 1);
        assert_eq!(response["matches"][0]["path"], "Voices/Hero/LINE_ONE.OGG");
        assert_eq!(response["matches"][0]["basename"], "LINE_ONE.OGG");
        assert_eq!(response["matches"][0]["compression"], "stored");
        assert_eq!(response["matches"][0]["is_directory"], false);
        assert_eq!(fs::read(&archive).unwrap(), before);
    }

    #[test]
    fn archive_match_line_zero_rejects_substrings_and_directory_text() {
        let temp = tempfile::tempdir().unwrap();
        let archive = temp.path().join("zero.zip");
        write_named_archive(
            &archive,
            &[
                "Voices/Hero/PREFIX_LINE_SUFFIX.ogg",
                "Voices/LINE/OTHER.ogg",
                "Voices/Hero/LINE.ogg.backup",
            ],
        );
        let before = fs::read(&archive).unwrap();

        let response = call(
            "voice_archive_match_line",
            match_line_payload(&archive, "LINE"),
        );

        assert_eq!(response["ok"], true, "response: {response}");
        assert_eq!(response["resolution"], "unresolved");
        assert_eq!(response["match_count"], 0);
        assert!(response["matches"].as_array().unwrap().is_empty());
        assert_eq!(fs::read(&archive).unwrap(), before);
    }

    #[test]
    fn archive_match_line_multiple_returns_every_exact_match_without_selecting() {
        let temp = tempfile::tempdir().unwrap();
        let archive = temp.path().join("ambiguous.zip");
        write_named_archive(
            &archive,
            &[
                "Voices/A/LINE.ogg",
                "Voices/B/line.OGG",
                "Voices/C/LiNe.Ogg",
                "Voices/D/NOT_LINE.ogg",
            ],
        );
        let before = fs::read(&archive).unwrap();

        let response = call(
            "voice_archive_match_line",
            match_line_payload(&archive, "lInE"),
        );

        assert_eq!(response["ok"], true, "response: {response}");
        assert_eq!(response["resolution"], "ambiguous");
        assert_eq!(response["match_count"], 3);
        let paths: Vec<_> = response["matches"]
            .as_array()
            .unwrap()
            .iter()
            .map(|entry| entry["path"].as_str().unwrap())
            .collect();
        assert_eq!(
            paths,
            [
                "Voices/A/LINE.ogg",
                "Voices/B/line.OGG",
                "Voices/C/LiNe.Ogg",
            ]
        );
        assert!(response.get("selected").is_none());
        assert!(response.get("entry_path").is_none());
        assert_eq!(fs::read(&archive).unwrap(), before);
    }

    #[test]
    fn archive_match_line_fails_closed_on_ineligible_exact_collisions() {
        let temp = tempfile::tempdir().unwrap();

        let traversal = temp.path().join("traversal.zip");
        write_named_archive(&traversal, &["Voices/LINE.ogg", "../LINE.ogg"]);
        assert_match_line_rejected_read_only(&traversal, "VOICE_ENTRY_UNSAFE");

        let symlink = temp.path().join("symlink.zip");
        let file = File::create(&symlink).unwrap();
        let mut writer = ZipWriter::new(file);
        writer
            .add_symlink(
                "Voices/LINE.ogg",
                "target.ogg",
                SimpleFileOptions::default(),
            )
            .unwrap();
        writer.finish().unwrap();
        assert_match_line_rejected_read_only(&symlink, "VOICE_ENTRY_SYMLINK");

        let encrypted = temp.path().join("encrypted.zip");
        write_named_archive(&encrypted, &["Voices/LINE.ogg"]);
        let mut bytes = fs::read(&encrypted).unwrap();
        let (central, local, _, _) = first_entry_layout(&bytes);
        let central_flags =
            u16::from_le_bytes(bytes[central + 8..central + 10].try_into().unwrap()) | 1;
        let local_flags = u16::from_le_bytes(bytes[local + 6..local + 8].try_into().unwrap()) | 1;
        bytes[central + 8..central + 10].copy_from_slice(&central_flags.to_le_bytes());
        bytes[local + 6..local + 8].copy_from_slice(&local_flags.to_le_bytes());
        fs::write(&encrypted, bytes).unwrap();
        assert_match_line_rejected_read_only(&encrypted, "VOICE_ENTRY_ENCRYPTED");

        let unsupported = temp.path().join("unsupported.zip");
        write_named_archive(&unsupported, &["Voices/LINE.ogg"]);
        let mut bytes = fs::read(&unsupported).unwrap();
        let (central, local, _, _) = first_entry_layout(&bytes);
        bytes[central + 10..central + 12].copy_from_slice(&12u16.to_le_bytes());
        bytes[local + 8..local + 10].copy_from_slice(&12u16.to_le_bytes());
        fs::write(&unsupported, bytes).unwrap();
        assert_match_line_rejected_read_only(&unsupported, "VOICE_COMPRESSION_UNSUPPORTED");
    }

    #[test]
    fn archive_match_line_rejects_unsafe_and_oversized_inputs_without_source_changes() {
        let temp = tempfile::tempdir().unwrap();
        let archive = temp.path().join("inputs.zip");
        write_named_archive(&archive, &["Voices/Hero/LINE.ogg"]);
        let before = fs::read(&archive).unwrap();

        for loc_id in [
            "../LINE", "..\\LINE", " LINE", "LINE\n", "LINE\0", ".", "..", "LÍNE",
        ] {
            let response = call(
                "voice_archive_match_line",
                match_line_payload(&archive, loc_id),
            );
            assert_eq!(response["error"]["code"], "VOICE_BAD_REQUEST");
        }

        let oversized_loc_id = call(
            "voice_archive_match_line",
            match_line_payload(&archive, &"x".repeat(MAX_FFI_LOC_ID_BYTES + 1)),
        );
        assert_eq!(oversized_loc_id["error"]["code"], "VOICE_BAD_REQUEST");

        let oversized_archive_path = call(
            "voice_archive_match_line",
            json!({
                "archive": "x".repeat(MAX_FILESYSTEM_PATH_BYTES + 1),
                "loc_id": "LINE",
            }),
        );
        assert_eq!(oversized_archive_path["error"]["code"], "VOICE_BAD_REQUEST");
        assert_eq!(fs::read(&archive).unwrap(), before);
    }

    #[test]
    fn archive_match_line_fails_closed_on_oversized_match_response() {
        let temp = tempfile::tempdir().unwrap();
        let archive = temp.path().join("match-response-budget.zip");
        let file = File::create(&archive).unwrap();
        let mut writer = ZipWriter::new(file);
        let suffix = "x".repeat(850);
        for index in 0..1_500 {
            writer
                .start_file(
                    format!("Voices/{index:05}_{suffix}/LINE.ogg"),
                    SimpleFileOptions::default().compression_method(CompressionMethod::Stored),
                )
                .unwrap();
        }
        writer.finish().unwrap();
        let before = fs::read(&archive).unwrap();

        let response = call(
            "voice_archive_match_line",
            match_line_payload(&archive, "LINE"),
        );

        assert_eq!(response["ok"], false, "response: {response}");
        assert_eq!(response["error"]["code"], "VOICE_RESPONSE_LIMIT");
        assert_eq!(fs::read(&archive).unwrap(), before);
    }

    #[test]
    fn archive_list_fails_closed_before_materializing_an_oversized_json_response() {
        let temp = tempfile::tempdir().unwrap();
        let archive = temp.path().join("response-budget.zip");
        let file = File::create(&archive).unwrap();
        let mut writer = ZipWriter::new(file);
        let suffix = "x".repeat(850);
        for index in 0..5_000 {
            let path = format!("Voices/{index:05}_{suffix}.ogg");
            writer
                .start_file(
                    path,
                    SimpleFileOptions::default().compression_method(CompressionMethod::Stored),
                )
                .unwrap();
        }
        writer.finish().unwrap();

        let response = call(
            "voice_archive_list",
            json!({"archive": archive.display().to_string()}),
        );
        assert_eq!(response["ok"], false, "response: {response}");
        assert_eq!(response["error"]["code"], "VOICE_RESPONSE_LIMIT");
    }

    #[test]
    fn archive_list_reports_a_stable_code_for_raw_entry_count_limit() {
        let temp = tempfile::tempdir().unwrap();
        let (archive, _) = make_voice_archive(&temp);
        let mut bytes = fs::read(&archive).unwrap();
        let eocd = find_eocd(&bytes);
        let declared_entries = u16::try_from(MAX_FFI_ARCHIVE_ENTRIES + 1).unwrap();
        bytes[eocd + 8..eocd + 10].copy_from_slice(&declared_entries.to_le_bytes());
        bytes[eocd + 10..eocd + 12].copy_from_slice(&declared_entries.to_le_bytes());
        fs::write(&archive, bytes).unwrap();

        let response = call(
            "voice_archive_list",
            json!({"archive": archive.display().to_string()}),
        );
        assert_eq!(response["ok"], false, "response: {response}");
        assert_eq!(response["error"]["code"], "VOICE_ARCHIVE_LIMIT");
    }

    #[test]
    fn archive_list_enforces_the_ffi_entry_output_budget_before_zip_parsing() {
        let temp = tempfile::tempdir().unwrap();
        let (archive, _) = make_voice_archive(&temp);
        let mut bytes = fs::read(&archive).unwrap();
        let central = find_first_central_header(&bytes);
        let declared_output = u32::try_from(MAX_FFI_ENTRY_OUTPUT_BYTES + 1).unwrap();
        bytes[central + 24..central + 28].copy_from_slice(&declared_output.to_le_bytes());
        fs::write(&archive, bytes).unwrap();

        let response = call(
            "voice_archive_list",
            json!({"archive": archive.display().to_string()}),
        );
        assert_eq!(response["ok"], false, "response: {response}");
        assert_eq!(response["error"]["code"], "VOICE_ARCHIVE_LIMIT");
    }

    #[test]
    fn archive_extract_is_exact_no_clobber_and_read_only() {
        let temp = tempfile::tempdir().unwrap();
        let (archive, ogg) = make_voice_archive(&temp);
        let archive_before = fs::read(&archive).unwrap();
        let output_root = temp.path().join("extract");
        let seal = listed_seal(&archive);

        let wrong_case = call(
            "voice_archive_extract",
            extract_payload(&archive, "voices/hero/line.ogg", &output_root, &seal),
        );
        assert_eq!(wrong_case["ok"], false);
        assert_eq!(wrong_case["error"]["code"], "VOICE_ENTRY_NOT_FOUND");
        assert!(!output_root.exists());

        let extracted = call(
            "voice_archive_extract",
            extract_payload(&archive, "Voices/Hero/LINE.ogg", &output_root, &seal),
        );
        assert_eq!(extracted["ok"], true, "response: {extracted}");
        let output = output_root.join("Voices").join("Hero").join("LINE.ogg");
        assert_eq!(fs::read(&output).unwrap(), ogg);
        assert_eq!(fs::read(&archive).unwrap(), archive_before);

        fs::write(&output, b"keep me").unwrap();
        let clobber = call(
            "voice_archive_extract",
            extract_payload(&archive, "Voices/Hero/LINE.ogg", &output_root, &seal),
        );
        assert_eq!(clobber["ok"], false);
        assert_eq!(clobber["error"]["code"], "VOICE_OUTPUT_EXISTS");
        assert_eq!(fs::read(&output).unwrap(), b"keep me");
        assert_eq!(fs::read(&archive).unwrap(), archive_before);
    }

    #[test]
    fn archive_extract_reports_stable_encrypted_unsupported_and_corrupt_payload_codes() {
        let temp = tempfile::tempdir().unwrap();

        let encrypted = temp.path().join("encrypted.zip");
        write_voice_archive(&encrypted, 44_100);
        let mut bytes = fs::read(&encrypted).unwrap();
        let (central, local, _, _) = first_entry_layout(&bytes);
        let central_flags =
            u16::from_le_bytes(bytes[central + 8..central + 10].try_into().unwrap()) | 1;
        let local_flags = u16::from_le_bytes(bytes[local + 6..local + 8].try_into().unwrap()) | 1;
        bytes[central + 8..central + 10].copy_from_slice(&central_flags.to_le_bytes());
        bytes[local + 6..local + 8].copy_from_slice(&local_flags.to_le_bytes());
        fs::write(&encrypted, bytes).unwrap();
        let seal = listed_seal(&encrypted);
        let encrypted_output = temp.path().join("encrypted-output");
        let response = call(
            "voice_archive_extract",
            extract_payload(&encrypted, "Voices/Hero/LINE.ogg", &encrypted_output, &seal),
        );
        assert_eq!(response["error"]["code"], "VOICE_ENTRY_ENCRYPTED");
        assert!(!encrypted_output.exists());

        let unsupported = temp.path().join("unsupported.zip");
        write_voice_archive(&unsupported, 44_100);
        let mut bytes = fs::read(&unsupported).unwrap();
        let (central, local, _, _) = first_entry_layout(&bytes);
        bytes[central + 10..central + 12].copy_from_slice(&12u16.to_le_bytes());
        bytes[local + 8..local + 10].copy_from_slice(&12u16.to_le_bytes());
        fs::write(&unsupported, bytes).unwrap();
        let seal = listed_seal(&unsupported);
        let unsupported_output = temp.path().join("unsupported-output");
        let response = call(
            "voice_archive_extract",
            extract_payload(
                &unsupported,
                "Voices/Hero/LINE.ogg",
                &unsupported_output,
                &seal,
            ),
        );
        assert_eq!(response["error"]["code"], "VOICE_COMPRESSION_UNSUPPORTED");
        assert!(!unsupported_output.exists());

        let corrupt = temp.path().join("corrupt.zip");
        write_voice_archive(&corrupt, 44_100);
        let mut bytes = fs::read(&corrupt).unwrap();
        let (_, _, payload, compressed) = first_entry_layout(&bytes);
        bytes[payload + compressed / 2] ^= 0x80;
        fs::write(&corrupt, bytes).unwrap();
        let seal = listed_seal(&corrupt);
        let corrupt_output = temp.path().join("corrupt-output");
        let response = call(
            "voice_archive_extract",
            extract_payload(&corrupt, "Voices/Hero/LINE.ogg", &corrupt_output, &seal),
        );
        assert!(matches!(
            response["error"]["code"].as_str(),
            Some("VOICE_ARCHIVE_INVALID" | "VOICE_VERIFICATION_FAILED")
        ));
        assert_ne!(response["error"]["code"], "VOICE_SOURCE_IO");
        assert!(!corrupt_output.join("Voices/Hero/LINE.ogg").exists());
    }

    #[test]
    fn voice_inputs_are_bounded_and_source_must_be_regular() {
        let temp = tempfile::tempdir().unwrap();
        let (archive, _) = make_voice_archive(&temp);
        let seal = listed_seal(&archive);
        let response = call(
            "voice_archive_extract",
            json!({
                "archive": archive.display().to_string(),
                "expected_archive_size": seal.0,
                "expected_archive_sha256": seal.1,
                "entry_path": "x".repeat(MAX_FFI_ENTRY_PATH_BYTES + 1),
                "output_root": temp.path().join("out").display().to_string(),
            }),
        );
        assert_eq!(response["error"]["code"], "VOICE_BAD_REQUEST");

        let oversized_archive_path = call(
            "voice_archive_list",
            json!({"archive": "x".repeat(MAX_FILESYSTEM_PATH_BYTES + 1)}),
        );
        assert_eq!(oversized_archive_path["error"]["code"], "VOICE_BAD_REQUEST");

        let oversized_output_root = call(
            "voice_archive_extract",
            json!({
                "archive": archive.display().to_string(),
                "expected_archive_size": seal.0,
                "expected_archive_sha256": seal.1,
                "entry_path": "Voices/Hero/LINE.ogg",
                "output_root": "x".repeat(MAX_FILESYSTEM_PATH_BYTES + 1),
            }),
        );
        assert_eq!(oversized_output_root["error"]["code"], "VOICE_BAD_REQUEST");

        let missing_seal = call(
            "voice_archive_extract",
            json!({
                "archive": archive.display().to_string(),
                "entry_path": "Voices/Hero/LINE.ogg",
                "output_root": temp.path().join("out").display().to_string(),
            }),
        );
        assert_eq!(missing_seal["error"]["code"], "VOICE_BAD_REQUEST");

        let malformed_seal = call(
            "voice_archive_extract",
            json!({
                "archive": archive.display().to_string(),
                "expected_archive_size": fs::metadata(&archive).unwrap().len(),
                "expected_archive_sha256": "not-a-sha256",
                "entry_path": "Voices/Hero/LINE.ogg",
                "output_root": temp.path().join("out").display().to_string(),
            }),
        );
        assert_eq!(malformed_seal["error"]["code"], "VOICE_BAD_REQUEST");

        let directory = call(
            "voice_archive_list",
            json!({"archive": temp.path().display().to_string()}),
        );
        assert_eq!(directory["error"]["code"], "VOICE_SOURCE_NOT_REGULAR");
    }

    #[test]
    fn unsafe_archive_path_is_rejected_before_output_creation() {
        let temp = tempfile::tempdir().unwrap();
        let archive = temp.path().join("unsafe.zip");
        let file = File::create(&archive).unwrap();
        let mut writer = ZipWriter::new(file);
        writer
            .start_file(
                "../escape.ogg",
                SimpleFileOptions::default().compression_method(CompressionMethod::Stored),
            )
            .unwrap();
        writer.write_all(&synthetic_vorbis_ogg(48_000)).unwrap();
        writer.finish().unwrap();
        let output_root = temp.path().join("extract");
        let seal = listed_seal(&archive);

        let response = call(
            "voice_archive_extract",
            extract_payload(&archive, "../escape.ogg", &output_root, &seal),
        );

        assert_eq!(response["ok"], false);
        assert_eq!(response["error"]["code"], "VOICE_ENTRY_UNSAFE");
        assert!(!output_root.exists());
        assert!(!temp.path().join("escape.ogg").exists());
    }

    #[test]
    fn extract_rejects_archive_replaced_after_list() {
        let temp = tempfile::tempdir().unwrap();
        let (archive, _) = make_voice_archive(&temp);
        let listed = listed_seal(&archive);
        let replacement = temp.path().join("replacement.zip");
        write_voice_archive(&replacement, 48_000);
        fs::remove_file(&archive).unwrap();
        fs::rename(&replacement, &archive).unwrap();
        let current = listed_seal(&archive);
        assert_ne!(listed.1, current.1);
        let output_root = temp.path().join("extract");

        let response = call(
            "voice_archive_extract",
            extract_payload(&archive, "Voices/Hero/LINE.ogg", &output_root, &listed),
        );

        assert_eq!(response["error"]["code"], "VOICE_ARCHIVE_CHANGED");
        assert!(!output_root.exists());
    }

    #[test]
    fn extract_checks_the_expected_seal_before_parsing_changed_zip_metadata() {
        let temp = tempfile::tempdir().unwrap();
        let (archive, _) = make_voice_archive(&temp);
        let listed = listed_seal(&archive);
        let mut bytes = fs::read(&archive).unwrap();
        let eocd = find_eocd(&bytes);
        bytes[eocd..eocd + 4].fill(0);
        fs::write(&archive, bytes).unwrap();
        let output_root = temp.path().join("extract-invalid-change");

        let response = call(
            "voice_archive_extract",
            extract_payload(&archive, "Voices/Hero/LINE.ogg", &output_root, &listed),
        );

        assert_eq!(response["error"]["code"], "VOICE_ARCHIVE_CHANGED");
        assert!(!output_root.exists());
    }

    #[test]
    fn output_root_link_ancestor_is_rejected_before_creation() {
        let temp = tempfile::tempdir().unwrap();
        let (archive, _) = make_voice_archive(&temp);
        let seal = listed_seal(&archive);
        let outside = temp.path().join("outside");
        let link = temp.path().join("linked-root");
        fs::create_dir(&outside).unwrap();

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

        let output_root = link.join("extract");
        let response = call(
            "voice_archive_extract",
            extract_payload(&archive, "Voices/Hero/LINE.ogg", &output_root, &seal),
        );
        assert_eq!(response["error"]["code"], "VOICE_OUTPUT_UNSAFE");
        assert!(!outside.join("extract").exists());

        #[cfg(unix)]
        fs::remove_file(&link).unwrap();
        #[cfg(windows)]
        fs::remove_dir(&link).unwrap();
    }

    #[test]
    fn source_symlink_is_rejected_when_platform_allows_creating_one() {
        let temp = tempfile::tempdir().unwrap();
        let (archive, _) = make_voice_archive(&temp);
        let link = temp.path().join("voices-link.zip");

        #[cfg(unix)]
        std::os::unix::fs::symlink(&archive, &link).unwrap();
        #[cfg(windows)]
        if std::os::windows::fs::symlink_file(&archive, &link).is_err() {
            return;
        }

        let response = call(
            "voice_archive_list",
            json!({"archive": link.display().to_string()}),
        );
        assert_eq!(response["error"]["code"], "VOICE_SOURCE_NOT_REGULAR");
    }

    #[test]
    fn source_output_archive_data_and_verification_errors_keep_distinct_codes() {
        let source = map_voice_error(Error::SourceIo {
            path: PathBuf::from("voices.zip"),
            source: io::Error::new(io::ErrorKind::PermissionDenied, "source denied"),
        })
        .response();
        let output = map_voice_error(Error::OutputIo {
            path: PathBuf::from("extract"),
            source: io::Error::new(io::ErrorKind::PermissionDenied, "output denied"),
        })
        .response();
        let archive_data = map_voice_error(Error::ArchiveData {
            path: "corrupt.ogg".to_owned(),
            source: io::Error::new(io::ErrorKind::InvalidData, "invalid checksum"),
        })
        .response();
        let verification =
            map_voice_error(Error::Verification("edited entry hash mismatch".to_owned()))
                .response();

        assert_eq!(source["error"]["code"], "VOICE_SOURCE_IO");
        assert_eq!(output["error"]["code"], "VOICE_OUTPUT_IO");
        assert_eq!(archive_data["error"]["code"], "VOICE_ARCHIVE_INVALID");
        assert_eq!(verification["error"]["code"], "VOICE_VERIFICATION_FAILED");
    }

    /// Optional real-install qualification for the exact Mod Studio FFI path. The source archive
    /// stays read-only; one indexed Ogg is extracted into a private temporary directory and then
    /// removed with it. Set `GORE_VOICE_REAL_ARCHIVE` to opt in.
    #[test]
    #[ignore = "requires GORE_VOICE_REAL_ARCHIVE pointing at an installed voice ZIP"]
    fn configured_real_archive_lists_and_extracts_one_exact_entry() {
        let archive = std::env::var("GORE_VOICE_REAL_ARCHIVE")
            .expect("set GORE_VOICE_REAL_ARCHIVE to an installed voice ZIP");
        let archive = PathBuf::from(archive);
        let before = fs::metadata(&archive).expect("real archive metadata");
        let listed = call(
            "voice_archive_list",
            json!({"archive": archive.display().to_string()}),
        );
        assert_eq!(listed["ok"], true, "response: {listed}");
        let entry = listed["entries"]
            .as_array()
            .expect("real archive entries")
            .iter()
            .find(|entry| {
                entry["is_directory"] == false
                    && entry["path"]
                        .as_str()
                        .is_some_and(|path| path.to_ascii_lowercase().ends_with(".ogg"))
            })
            .expect("real archive contains an Ogg entry");
        let path = entry["path"].as_str().expect("real entry path");
        let seal = (
            listed["archive_size"].as_u64().expect("real archive size"),
            listed["archive_sha256"]
                .as_str()
                .expect("real archive hash")
                .to_owned(),
        );
        let output = tempfile::tempdir().expect("real extraction tempdir");

        let extracted = call(
            "voice_archive_extract",
            extract_payload(&archive, path, output.path(), &seal),
        );

        assert_eq!(extracted["ok"], true, "response: {extracted}");
        let extracted_path = PathBuf::from(
            extracted["output"]
                .as_str()
                .expect("real extracted output path"),
        );
        let canonical_output = fs::canonicalize(output.path()).expect("canonical real output root");
        assert!(extracted_path.starts_with(canonical_output));
        assert_eq!(
            fs::metadata(&extracted_path)
                .expect("real extracted file metadata")
                .len(),
            entry["uncompressed_size"]
                .as_u64()
                .expect("real entry size")
        );
        let after = fs::metadata(&archive).expect("real archive metadata after extraction");
        assert_eq!(after.len(), before.len());
        assert_eq!(after.modified().ok(), before.modified().ok());
    }

    fn synthetic_vorbis_ogg(sample_rate: u32) -> Vec<u8> {
        let mut data = include_bytes!("../../gore-vo/testdata/tiny-vorbis.ogg").to_vec();
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
            let crc = ogg_crc(&data[offset..offset + page_len]);
            data[offset + 22..offset + 26].copy_from_slice(&crc.to_le_bytes());
            offset += page_len;
        }
        data
    }

    fn ogg_crc(bytes: &[u8]) -> u32 {
        let mut crc = 0u32;
        for byte in bytes {
            crc ^= u32::from(*byte) << 24;
            for _ in 0..8 {
                crc = if crc & 0x8000_0000 != 0 {
                    (crc << 1) ^ 0x04c1_1db7
                } else {
                    crc << 1
                };
            }
        }
        crc
    }
}
