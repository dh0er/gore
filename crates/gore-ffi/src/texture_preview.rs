//! Native-owned, bounded texture-preview byte capabilities.
//!
//! Preview files never cross the FFI as ambient paths. A caller receives one
//! opaque token, reads the immutable file sequentially in bounded chunks, and
//! releases it explicitly. At most two extraction/preview capabilities can be
//! live at once. Windows creates the file delete-on-close with sharing that
//! denies later writers and renames; Unix unlinks it immediately after create.

use std::collections::{HashMap, HashSet};
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use base64::Engine as _;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use crate::err;

pub(super) const READ_COMMAND: &str = "texture_preview_read";
pub(super) const RELEASE_COMMAND: &str = "texture_preview_release";

const TOKEN_BYTES: usize = 32;
const TOKEN_HEX_BYTES: usize = TOKEN_BYTES * 2;
const MAX_ACTIVE_PREVIEWS: usize = 2;
const ACTIVE_PREVIEW_IDLE_TTL: Duration = Duration::from_secs(5 * 60);
const READ_CHUNK_BYTES: usize = 512 * 1024;
const MAX_PREVIEW_PNG_BYTES: u64 = 64 * 1024 * 1024;
const PREVIEW_PREFIX: &str = "gore-tex-preview-";

#[derive(Debug)]
pub(super) struct PreviewFailure {
    pub(super) code: &'static str,
    pub(super) message: String,
}

impl PreviewFailure {
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
struct FileIdentity {
    device: u64,
    file: u64,
}

#[derive(Debug, Clone, Copy)]
struct FileSnapshot {
    identity: FileIdentity,
    byte_len: u64,
    is_regular: bool,
    is_reparse: bool,
}

struct ActivePreview {
    file: File,
    identity: FileIdentity,
    byte_len: u64,
    sha256: [u8; 32],
    next_offset: u64,
    last_activity: Instant,
}

#[derive(Default)]
struct PreviewRegistry {
    pending: HashSet<String>,
    active: HashMap<String, ActivePreview>,
}

fn registry() -> &'static Mutex<PreviewRegistry> {
    static REGISTRY: OnceLock<Mutex<PreviewRegistry>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(PreviewRegistry::default()))
}

/// One reserved extraction slot and its exact native-owned output file.
#[derive(Debug)]
pub(super) struct PendingPreview {
    token: String,
    file: Option<File>,
    published: bool,
}

pub(super) struct PublishedPreview {
    pub(super) token: String,
    pub(super) byte_len: u64,
    pub(super) sha256: String,
}

impl PendingPreview {
    pub(super) fn create() -> Result<Self, PreviewFailure> {
        Self::create_at(Instant::now())
    }

    fn create_at(now: Instant) -> Result<Self, PreviewFailure> {
        let token = reserve_token_at(now)?;
        match create_private_preview_file(&token) {
            Ok(file) => Ok(Self {
                token,
                file: Some(file),
                published: false,
            }),
            Err(_) => {
                cancel_pending(&token);
                Err(PreviewFailure::new(
                    "TEXTURE_PREVIEW_IO",
                    "native texture preview file could not be created safely",
                ))
            }
        }
    }

    pub(super) fn file_mut(&mut self) -> &mut File {
        self.file
            .as_mut()
            .expect("pending texture preview file must remain available")
    }

    /// Flush, seal, and publish this exact handle into the bounded registry.
    pub(super) fn publish(mut self) -> Result<PublishedPreview, PreviewFailure> {
        let file = self.file.as_mut().ok_or_else(registry_unavailable)?;
        file.flush().and_then(|_| file.sync_all()).map_err(|_| {
            PreviewFailure::new("TEXTURE_PREVIEW_IO", "preview could not be synchronized")
        })?;

        let before = snapshot_regular_file(file).map_err(|_| {
            PreviewFailure::new(
                "TEXTURE_PREVIEW_IO",
                "preview file identity could not be inspected",
            )
        })?;
        if !before.is_regular
            || before.is_reparse
            || before.byte_len == 0
            || before.byte_len > MAX_PREVIEW_PNG_BYTES
        {
            return Err(PreviewFailure::new(
                "TEXTURE_PREVIEW_LIMIT",
                "encoded texture preview is not one bounded regular file",
            ));
        }

        file.seek(SeekFrom::Start(0)).map_err(|_| {
            PreviewFailure::new("TEXTURE_PREVIEW_IO", "preview could not be rewound")
        })?;
        let mut hasher = Sha256::new();
        let mut chunk = [0_u8; 64 * 1024];
        let mut hashed = 0_u64;
        loop {
            let read = file.read(&mut chunk).map_err(|_| {
                PreviewFailure::new("TEXTURE_PREVIEW_IO", "preview could not be sealed")
            })?;
            if read == 0 {
                break;
            }
            hashed = hashed.checked_add(read as u64).ok_or_else(|| {
                PreviewFailure::new("TEXTURE_PREVIEW_LIMIT", "preview length overflowed")
            })?;
            if hashed > MAX_PREVIEW_PNG_BYTES {
                return Err(PreviewFailure::new(
                    "TEXTURE_PREVIEW_LIMIT",
                    "encoded texture preview exceeds 64 MiB",
                ));
            }
            hasher.update(&chunk[..read]);
        }
        let after = snapshot_regular_file(file).map_err(|_| {
            PreviewFailure::new(
                "TEXTURE_PREVIEW_IO",
                "preview file identity could not be rechecked",
            )
        })?;
        if before.identity != after.identity
            || before.byte_len != after.byte_len
            || hashed != before.byte_len
            || !after.is_regular
            || after.is_reparse
        {
            return Err(PreviewFailure::new(
                "TEXTURE_PREVIEW_CHANGED",
                "native texture preview changed while it was being sealed",
            ));
        }
        let digest: [u8; 32] = hasher.finalize().into();
        let digest_hex = encode_hex(&digest);
        file.seek(SeekFrom::Start(0)).map_err(|_| {
            PreviewFailure::new("TEXTURE_PREVIEW_IO", "preview could not be rewound")
        })?;

        let mut active = ActivePreview {
            file: self.file.take().ok_or_else(registry_unavailable)?,
            identity: before.identity,
            byte_len: before.byte_len,
            sha256: digest,
            next_offset: 0,
            last_activity: Instant::now(),
        };
        let mut locked = registry().lock().map_err(|_| registry_unavailable())?;
        let now = Instant::now();
        prune_expired_active(&mut locked, now);
        if !locked.pending.remove(&self.token)
            || locked.active.len() >= MAX_ACTIVE_PREVIEWS
            || locked.active.contains_key(&self.token)
        {
            return Err(registry_unavailable());
        }
        active.last_activity = now;
        locked.active.insert(self.token.clone(), active);
        self.published = true;
        Ok(PublishedPreview {
            token: self.token.clone(),
            byte_len: before.byte_len,
            sha256: digest_hex,
        })
    }
}

impl Drop for PendingPreview {
    fn drop(&mut self) {
        if !self.published {
            cancel_pending(&self.token);
        }
        // The Windows handle is delete-on-close; Unix unlinked it at create.
        self.file.take();
    }
}

pub(super) fn read(payload: Value) -> Value {
    read_inner(payload).unwrap_or_else(PreviewFailure::response)
}

fn read_inner(payload: Value) -> Result<Value, PreviewFailure> {
    let object = exact_object(&payload, &["offset", "preview_token"])?;
    let token = required_token(object.get("preview_token"))?;
    let offset = object
        .get("offset")
        .and_then(Value::as_u64)
        .ok_or_else(invalid_request)?;

    let mut locked = registry().lock().map_err(|_| registry_unavailable())?;
    prune_expired_active(&mut locked, Instant::now());
    let preview = locked.active.get_mut(token).ok_or_else(|| {
        PreviewFailure::new(
            "TEXTURE_PREVIEW_UNKNOWN",
            "texture preview token is unknown or already released",
        )
    })?;
    if offset != preview.next_offset || offset >= preview.byte_len {
        return Err(PreviewFailure::new(
            "TEXTURE_PREVIEW_OFFSET",
            "texture preview reads must be sequential and in bounds",
        ));
    }
    let snapshot = snapshot_regular_file(&preview.file).map_err(|_| {
        PreviewFailure::new(
            "TEXTURE_PREVIEW_CHANGED",
            "retained texture preview identity is unavailable",
        )
    })?;
    if snapshot.identity != preview.identity
        || snapshot.byte_len != preview.byte_len
        || !snapshot.is_regular
        || snapshot.is_reparse
    {
        return Err(PreviewFailure::new(
            "TEXTURE_PREVIEW_CHANGED",
            "retained texture preview identity changed",
        ));
    }

    let remaining = preview.byte_len - offset;
    let wanted = usize::try_from(remaining.min(READ_CHUNK_BYTES as u64))
        .map_err(|_| registry_unavailable())?;
    let mut bytes = vec![0_u8; wanted];
    preview
        .file
        .seek(SeekFrom::Start(offset))
        .and_then(|_| preview.file.read_exact(&mut bytes))
        .map_err(|_| {
            PreviewFailure::new(
                "TEXTURE_PREVIEW_CHANGED",
                "retained texture preview could not be read exactly",
            )
        })?;
    let next_offset = offset + bytes.len() as u64;
    preview.next_offset = next_offset;
    let encoded = base64::engine::general_purpose::STANDARD.encode(&bytes);
    preview.last_activity = Instant::now();
    Ok(json!({
        "ok": true,
        "preview_token": token,
        "offset": offset,
        "chunk_byte_len": bytes.len(),
        "chunk_base64": encoded,
        "next_offset": next_offset,
        "total_byte_len": preview.byte_len,
        "eof": next_offset == preview.byte_len,
    }))
}

pub(super) fn release(payload: Value) -> Value {
    release_inner(payload).unwrap_or_else(PreviewFailure::response)
}

fn release_inner(payload: Value) -> Result<Value, PreviewFailure> {
    let object = exact_object(&payload, &["preview_token"])?;
    let token = required_token(object.get("preview_token"))?.to_owned();
    let preview = {
        let mut locked = registry().lock().map_err(|_| registry_unavailable())?;
        prune_expired_active(&mut locked, Instant::now());
        locked.active.remove(&token).ok_or_else(|| {
            PreviewFailure::new(
                "TEXTURE_PREVIEW_UNKNOWN",
                "texture preview token is unknown or already released",
            )
        })?
    };
    let fully_read = preview.next_offset == preview.byte_len;
    let byte_len = preview.byte_len;
    let sha256 = encode_hex(&preview.sha256);
    // Dropping the exact native handle unlinks the delete-on-close Windows
    // object; Unix removed its ambient name immediately after create.
    drop(preview);
    Ok(json!({
        "ok": true,
        "preview_token": token,
        "released": true,
        "fully_read": fully_read,
        "png_byte_len": byte_len,
        "png_sha256": sha256,
    }))
}

fn reserve_token_at(now: Instant) -> Result<String, PreviewFailure> {
    let mut locked = registry().lock().map_err(|_| registry_unavailable())?;
    prune_expired_active(&mut locked, now);
    if locked.pending.len() + locked.active.len() >= MAX_ACTIVE_PREVIEWS {
        return Err(PreviewFailure::new(
            "TEXTURE_PREVIEW_LIMIT",
            "at most two native texture previews may be active",
        ));
    }
    for _ in 0..64 {
        let token = fresh_token()?;
        if !locked.pending.contains(&token) && !locked.active.contains_key(&token) {
            locked.pending.insert(token.clone());
            return Ok(token);
        }
    }
    Err(registry_unavailable())
}

fn cancel_pending(token: &str) {
    if let Ok(mut locked) = registry().lock() {
        prune_expired_active(&mut locked, Instant::now());
        locked.pending.remove(token);
    }
}

fn prune_expired_active(registry: &mut PreviewRegistry, now: Instant) {
    registry.active.retain(|_, preview| {
        now.saturating_duration_since(preview.last_activity) < ACTIVE_PREVIEW_IDLE_TTL
    });
}

fn fresh_token() -> Result<String, PreviewFailure> {
    let mut bytes = [0_u8; TOKEN_BYTES];
    getrandom::fill(&mut bytes).map_err(|_| registry_unavailable())?;
    Ok(encode_hex(&bytes))
}

fn encode_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for &byte in bytes {
        encoded.push(HEX[usize::from(byte >> 4)] as char);
        encoded.push(HEX[usize::from(byte & 0x0f)] as char);
    }
    encoded
}

fn required_token(value: Option<&Value>) -> Result<&str, PreviewFailure> {
    let token = value.and_then(Value::as_str).ok_or_else(invalid_request)?;
    if token.len() != TOKEN_HEX_BYTES
        || !token
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(invalid_request());
    }
    Ok(token)
}

fn exact_object<'a>(
    payload: &'a Value,
    expected: &[&str],
) -> Result<&'a serde_json::Map<String, Value>, PreviewFailure> {
    let object = payload.as_object().ok_or_else(invalid_request)?;
    if object.len() != expected.len() || !expected.iter().all(|field| object.contains_key(*field)) {
        return Err(invalid_request());
    }
    Ok(object)
}

fn invalid_request() -> PreviewFailure {
    PreviewFailure::new(
        "TEXTURE_PREVIEW_INPUT_INVALID",
        "texture preview request is invalid",
    )
}

fn registry_unavailable() -> PreviewFailure {
    PreviewFailure::new(
        "TEXTURE_PREVIEW_UNAVAILABLE",
        "native texture preview registry is unavailable",
    )
}

#[cfg(windows)]
fn create_private_preview_file(token: &str) -> io::Result<File> {
    use std::os::windows::fs::OpenOptionsExt as _;
    use windows_sys::Win32::Storage::FileSystem::{
        DELETE, FILE_ATTRIBUTE_TEMPORARY, FILE_FLAG_DELETE_ON_CLOSE, FILE_GENERIC_READ,
        FILE_GENERIC_WRITE, FILE_SHARE_READ,
    };

    let path = preview_path(token);
    let mut options = OpenOptions::new();
    options
        .read(true)
        .write(true)
        .create_new(true)
        .access_mode(FILE_GENERIC_READ | FILE_GENERIC_WRITE | DELETE)
        .share_mode(FILE_SHARE_READ)
        .custom_flags(FILE_ATTRIBUTE_TEMPORARY | FILE_FLAG_DELETE_ON_CLOSE);
    options.open(path)
}

#[cfg(unix)]
fn create_private_preview_file(token: &str) -> io::Result<File> {
    use std::os::unix::fs::OpenOptionsExt as _;

    let path = preview_path(token);
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(&path)?;
    if let Err(error) = std::fs::remove_file(&path) {
        drop(file);
        return Err(error);
    }
    Ok(file)
}

fn preview_path(token: &str) -> PathBuf {
    std::env::temp_dir().join(format!("{PREVIEW_PREFIX}{token}.png"))
}

#[cfg(windows)]
fn snapshot_regular_file(file: &File) -> io::Result<FileSnapshot> {
    use std::mem::MaybeUninit;
    use std::os::windows::io::AsRawHandle as _;
    use windows_sys::Win32::Foundation::HANDLE;
    use windows_sys::Win32::Storage::FileSystem::{
        GetFileInformationByHandle, GetFileType, BY_HANDLE_FILE_INFORMATION,
        FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_REPARSE_POINT, FILE_TYPE_DISK,
    };

    let mut info = MaybeUninit::<BY_HANDLE_FILE_INFORMATION>::uninit();
    // SAFETY: `file` owns a live handle and `info` is the documented writable ABI type.
    if unsafe { GetFileInformationByHandle(file.as_raw_handle() as HANDLE, info.as_mut_ptr()) } == 0
    {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: successful GetFileInformationByHandle initialized the complete structure.
    let info = unsafe { info.assume_init() };
    // SAFETY: `file` owns a live handle.
    let disk = unsafe { GetFileType(file.as_raw_handle() as HANDLE) } == FILE_TYPE_DISK;
    Ok(FileSnapshot {
        identity: FileIdentity {
            device: u64::from(info.dwVolumeSerialNumber),
            file: (u64::from(info.nFileIndexHigh) << 32) | u64::from(info.nFileIndexLow),
        },
        byte_len: (u64::from(info.nFileSizeHigh) << 32) | u64::from(info.nFileSizeLow),
        is_regular: disk && info.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY == 0,
        is_reparse: info.dwFileAttributes & FILE_ATTRIBUTE_REPARSE_POINT != 0,
    })
}

#[cfg(unix)]
fn snapshot_regular_file(file: &File) -> io::Result<FileSnapshot> {
    use std::os::unix::fs::MetadataExt as _;

    let metadata = file.metadata()?;
    Ok(FileSnapshot {
        identity: FileIdentity {
            device: metadata.dev(),
            file: metadata.ino(),
        },
        byte_len: metadata.len(),
        is_regular: metadata.file_type().is_file(),
        is_reparse: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_lock() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(())).lock().unwrap()
    }

    #[test]
    fn preview_capability_streams_exact_bytes_then_releases() {
        let _guard = test_lock();
        let bytes = b"bounded native texture preview";
        let mut pending = PendingPreview::create().unwrap();
        pending.file_mut().write_all(bytes).unwrap();
        let published = pending.publish().unwrap();

        let first = read(json!({
            "preview_token": published.token,
            "offset": 0,
        }));
        assert_eq!(first["ok"], true);
        assert_eq!(first["eof"], true);
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(first["chunk_base64"].as_str().unwrap())
            .unwrap();
        assert_eq!(decoded, bytes);

        let released = release(json!({"preview_token": published.token}));
        assert_eq!(released["ok"], true);
        assert_eq!(released["released"], true);
        assert_eq!(released["fully_read"], true);
        assert_eq!(released["png_byte_len"], bytes.len() as u64);
    }

    #[test]
    fn preview_capability_streams_multiple_chunks_in_order_then_fully_releases() {
        let _guard = test_lock();
        let bytes = (0..(READ_CHUNK_BYTES * 2 + 73))
            .map(|index| ((index * 31 + 7) % 251) as u8)
            .collect::<Vec<_>>();
        let mut pending = PendingPreview::create().unwrap();
        pending.file_mut().write_all(&bytes).unwrap();
        let published = pending.publish().unwrap();
        let token = published.token.clone();

        assert_eq!(published.byte_len, bytes.len() as u64);
        let expected_offsets = [
            0_u64,
            READ_CHUNK_BYTES as u64,
            (READ_CHUNK_BYTES * 2) as u64,
        ];
        let expected_lengths = [READ_CHUNK_BYTES, READ_CHUNK_BYTES, 73];
        let mut streamed = Vec::with_capacity(bytes.len());

        for (index, (&offset, &chunk_len)) in expected_offsets
            .iter()
            .zip(expected_lengths.iter())
            .enumerate()
        {
            let response = read(json!({
                "preview_token": token,
                "offset": offset,
            }));
            let next_offset = offset + chunk_len as u64;

            assert_eq!(response["ok"], true);
            assert_eq!(response["preview_token"], token);
            assert_eq!(response["offset"], offset);
            assert_eq!(response["chunk_byte_len"], chunk_len as u64);
            assert_eq!(response["next_offset"], next_offset);
            assert_eq!(response["total_byte_len"], bytes.len() as u64);
            assert_eq!(response["eof"], index == expected_offsets.len() - 1);

            let decoded = base64::engine::general_purpose::STANDARD
                .decode(response["chunk_base64"].as_str().unwrap())
                .unwrap();
            assert_eq!(decoded, bytes[offset as usize..next_offset as usize]);
            streamed.extend_from_slice(&decoded);
        }

        assert_eq!(streamed, bytes);
        let released = release(json!({"preview_token": token}));
        assert_eq!(released["ok"], true);
        assert_eq!(released["preview_token"], token);
        assert_eq!(released["released"], true);
        assert_eq!(released["fully_read"], true);
        assert_eq!(released["png_byte_len"], bytes.len() as u64);
        assert_eq!(released["png_sha256"], published.sha256);

        let read_after_release = read(json!({
            "preview_token": token,
            "offset": 0,
        }));
        assert_eq!(
            read_after_release["error"]["code"],
            "TEXTURE_PREVIEW_UNKNOWN"
        );
    }

    #[test]
    fn preview_capability_idle_ttl_reclaims_only_expired_active_entries() {
        let _guard = test_lock();
        let mut expired_pending = PendingPreview::create().unwrap();
        expired_pending.file_mut().write_all(b"expired").unwrap();
        let expired = expired_pending.publish().unwrap();
        let expired_path = preview_path(&expired.token);

        let mut fresh_pending = PendingPreview::create().unwrap();
        fresh_pending.file_mut().write_all(b"fresh").unwrap();
        let fresh = fresh_pending.publish().unwrap();

        let baseline = Instant::now();
        let expiry_boundary = baseline
            .checked_add(ACTIVE_PREVIEW_IDLE_TTL)
            .expect("five-minute test deadline must fit in Instant");
        {
            let mut locked = registry().lock().unwrap();
            locked.active.get_mut(&expired.token).unwrap().last_activity = baseline;
            locked.active.get_mut(&fresh.token).unwrap().last_activity = expiry_boundary;
        }

        // Slot reservation performs the same prune used in production, but
        // with a deterministic clock boundary instead of sleeping five minutes.
        let replacement = PendingPreview::create_at(expiry_boundary).unwrap();
        assert!(!expired_path.exists());

        let expired_read = read(json!({
            "preview_token": expired.token,
            "offset": 0,
        }));
        assert_eq!(expired_read["error"]["code"], "TEXTURE_PREVIEW_UNKNOWN");

        let fresh_read = read(json!({
            "preview_token": fresh.token,
            "offset": 0,
        }));
        assert_eq!(fresh_read["ok"], true);
        assert_eq!(fresh_read["eof"], true);
        let fresh_release = release(json!({"preview_token": fresh.token}));
        assert_eq!(fresh_release["ok"], true);
        assert_eq!(fresh_release["fully_read"], true);

        let replacement_token = replacement.token.clone();
        let far_future = expiry_boundary
            .checked_add(ACTIVE_PREVIEW_IDLE_TTL)
            .expect("second five-minute test deadline must fit in Instant");
        {
            let mut locked = registry().lock().unwrap();
            prune_expired_active(&mut locked, far_future);
            assert!(locked.pending.contains(&replacement_token));
        }
        drop(replacement);
    }

    #[test]
    fn preview_capability_limit_includes_pending_outputs() {
        let _guard = test_lock();
        let first = PendingPreview::create().unwrap();
        let second = PendingPreview::create().unwrap();
        let error = PendingPreview::create().unwrap_err();
        assert_eq!(error.code, "TEXTURE_PREVIEW_LIMIT");
        drop(first);
        drop(second);
    }
}
