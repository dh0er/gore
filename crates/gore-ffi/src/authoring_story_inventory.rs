//! Read-only construction of one sealed base-game Story collision inventory.

use std::fs::{File, OpenOptions};
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use gore_story_catalog::{
    build_known_catalog, CatalogError, GenerationInputLimits, GenerationPaths,
};
use gore_story_inventory::{
    build_base_game_inventory, StoryInventoryError, MAX_BINDS_CACHE_SOURCE_BYTES,
    MAX_INVENTORY_JSON_BYTES, MAX_SHIPPING_CACHE_SOURCE_BYTES,
};
use serde_json::{json, Map, Value};
use sha2::{Digest as _, Sha256};

use crate::err;

const MAX_PATH_BYTES: usize = 32 * 1024;
const MAX_BUILD_RESPONSE_BYTES: usize = 56 * 1024 * 1024;
const REQUEST_BINDING_DOMAIN: &[u8] = b"gore-story-inventory.authoring-build-v1.request-binding\0";

pub(super) fn build_story_inventory_v1(payload: Value) -> Value {
    match build_story_inventory_v1_inner(&payload) {
        Ok(response) | Err(response) => response,
    }
}

fn build_story_inventory_v1_inner(payload: &Value) -> Result<Value, Value> {
    let object = exact_payload(payload)?;
    let executable = bounded_path(object, "executable")?;
    let shipping_cache = bounded_path(object, "shipping_cache")?;
    let binds_cache = bounded_path(object, "binds_cache")?;
    let request_binding_sha256 = request_binding(executable, shipping_cache, binds_cache);
    let paths = GenerationPaths {
        executable: PathBuf::from(executable),
        shipping_cache: PathBuf::from(shipping_cache),
        binds_cache: PathBuf::from(binds_cache),
    };

    // This creates the closed StoryCatalogFile capability only for the exact compiled generation
    // and retains guards for all three source identities, including the executable.
    let catalog =
        build_known_catalog(&paths, GenerationInputLimits::default()).map_err(map_catalog_error)?;
    catalog
        .revalidate_generation_inputs()
        .map_err(map_catalog_error)?;

    let shipping_bytes = read_source_no_follow(
        &paths.shipping_cache,
        MAX_SHIPPING_CACHE_SOURCE_BYTES as u64,
    )
    .map_err(map_read_error)?;
    let binds_bytes =
        read_source_no_follow(&paths.binds_cache, MAX_BINDS_CACHE_SOURCE_BYTES as u64)
            .map_err(map_read_error)?;
    let inventory = build_base_game_inventory(&catalog, &shipping_bytes, &binds_bytes)
        .map_err(map_inventory_error)?;

    // Reopen all three guarded generation paths immediately around both artifact and outer
    // response serialization. No file is written and no parsed catalog lacking a live guard is
    // accepted here.
    catalog
        .revalidate_generation_inputs()
        .map_err(map_catalog_error)?;
    let inventory_json = inventory.to_canonical_json().map_err(map_inventory_error)?;
    if inventory_json.len() > MAX_INVENTORY_JSON_BYTES {
        return Err(err(
            "AUTHORING_STORY_INVENTORY_BUILD_RESPONSE_LIMIT",
            "built Story inventory exceeds its canonical artifact limit",
        ));
    }
    let inventory_json = String::from_utf8(inventory_json).map_err(|_| {
        err(
            "AUTHORING_STORY_INVENTORY_BUILD_FAILED",
            "built Story inventory was not UTF-8",
        )
    })?;
    let response = json!({
        "ok": true,
        "request_binding_sha256": request_binding_sha256,
        "inventory_json": inventory_json,
        "generation": inventory.generation(),
        "story_catalog_seal": inventory.story_catalog_seal(),
        "source_pair_seal": inventory.source_pair_seal(),
        "payload_seal": inventory.payload_seal(),
        "catalog_layer": inventory.catalog_layer(),
        "coverage": inventory.coverage(),
        "runtime_qualification": inventory.runtime_qualification(),
        "publication_status": inventory.publication_status(),
    });
    let encoded = serde_json::to_vec(&response).map_err(|_| {
        err(
            "AUTHORING_STORY_INVENTORY_BUILD_FAILED",
            "built Story inventory response could not be serialized",
        )
    })?;
    if encoded.len() > MAX_BUILD_RESPONSE_BYTES {
        return Err(err(
            "AUTHORING_STORY_INVENTORY_BUILD_RESPONSE_LIMIT",
            "built Story inventory response exceeds its bounded transport budget",
        ));
    }
    catalog
        .revalidate_generation_inputs()
        .map_err(map_catalog_error)?;
    Ok(response)
}

fn exact_payload(payload: &Value) -> Result<&Map<String, Value>, Value> {
    let Some(object) = payload.as_object() else {
        return Err(invalid_request());
    };
    if object.len() != 3
        || !object.contains_key("executable")
        || !object.contains_key("shipping_cache")
        || !object.contains_key("binds_cache")
    {
        return Err(invalid_request());
    }
    Ok(object)
}

fn bounded_path<'a>(object: &'a Map<String, Value>, field: &str) -> Result<&'a str, Value> {
    object
        .get(field)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty() && value.len() <= MAX_PATH_BYTES && !value.contains('\0'))
        .ok_or_else(invalid_request)
}

fn invalid_request() -> Value {
    err(
        "AUTHORING_STORY_INVENTORY_BUILD_REQUEST_INVALID",
        "payload must contain exactly three non-empty bounded generation paths",
    )
}

fn request_binding(executable: &str, shipping_cache: &str, binds_cache: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(REQUEST_BINDING_DOMAIN);
    for value in [executable, shipping_cache, binds_cache] {
        let bytes = value.as_bytes();
        hasher.update((bytes.len() as u64).to_le_bytes());
        hasher.update(bytes);
    }
    hex_digest(hasher.finalize())
}

fn hex_digest(digest: impl IntoIterator<Item = u8>) -> String {
    let mut output = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write as _;
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FileIdentity {
    volume: u64,
    file: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ChangeStamp([i64; 4]);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct HandleSnapshot {
    identity: FileIdentity,
    byte_len: u64,
    link_count: u64,
    change_stamp: ChangeStamp,
    is_directory: bool,
    is_reparse: bool,
}

#[derive(Debug)]
enum SourceReadError {
    Missing,
    Unsafe,
    Limit,
    Changed,
    Io,
}

fn read_source_no_follow(path: &Path, max_bytes: u64) -> Result<Vec<u8>, SourceReadError> {
    let (mut file, initial) = open_regular_no_follow(path)?;
    if initial.byte_len == 0 || initial.byte_len > max_bytes {
        return Err(SourceReadError::Limit);
    }
    let capacity = usize::try_from(initial.byte_len).map_err(|_| SourceReadError::Limit)?;
    let mut bytes = Vec::with_capacity(capacity);
    let read_limit = max_bytes.checked_add(1).ok_or(SourceReadError::Limit)?;
    file.by_ref()
        .take(read_limit)
        .read_to_end(&mut bytes)
        .map_err(|_| SourceReadError::Io)?;
    if bytes.len() as u64 > max_bytes {
        return Err(SourceReadError::Limit);
    }
    if bytes.len() as u64 != initial.byte_len {
        return Err(SourceReadError::Changed);
    }
    let final_snapshot = snapshot_open_handle(&file)?;
    validate_snapshot(final_snapshot)?;
    if final_snapshot != initial {
        return Err(SourceReadError::Changed);
    }
    let (_reopened, reopened) = open_regular_no_follow(path)?;
    if reopened != initial {
        return Err(SourceReadError::Changed);
    }
    Ok(bytes)
}

fn open_regular_no_follow(path: &Path) -> Result<(File, HandleSnapshot), SourceReadError> {
    let file = open_regular_handle_no_follow(path).map_err(classify_open_error)?;
    let snapshot = snapshot_open_handle(&file)?;
    validate_snapshot(snapshot)?;
    Ok((file, snapshot))
}

fn validate_snapshot(snapshot: HandleSnapshot) -> Result<(), SourceReadError> {
    if snapshot.is_directory || snapshot.is_reparse || snapshot.link_count != 1 {
        return Err(SourceReadError::Unsafe);
    }
    Ok(())
}

fn classify_open_error(error: io::Error) -> SourceReadError {
    if error.kind() == io::ErrorKind::NotFound {
        return SourceReadError::Missing;
    }
    #[cfg(unix)]
    if error.raw_os_error() == Some(libc::ELOOP) {
        return SourceReadError::Unsafe;
    }
    SourceReadError::Io
}

#[cfg(windows)]
fn open_regular_handle_no_follow(path: &Path) -> io::Result<File> {
    use std::os::windows::fs::OpenOptionsExt;
    use windows_sys::Win32::Storage::FileSystem::FILE_FLAG_OPEN_REPARSE_POINT;

    let mut options = OpenOptions::new();
    options
        .read(true)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT);
    options.open(path)
}

#[cfg(unix)]
fn open_regular_handle_no_follow(path: &Path) -> io::Result<File> {
    use std::os::unix::fs::OpenOptionsExt;

    let mut options = OpenOptions::new();
    options
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC);
    options.open(path)
}

#[cfg(windows)]
fn snapshot_open_handle(file: &File) -> Result<HandleSnapshot, SourceReadError> {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Storage::FileSystem::{
        FileBasicInfo, GetFileInformationByHandle, GetFileInformationByHandleEx,
        BY_HANDLE_FILE_INFORMATION, FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_REPARSE_POINT,
        FILE_BASIC_INFO,
    };

    let mut info = unsafe { std::mem::zeroed::<BY_HANDLE_FILE_INFORMATION>() };
    // SAFETY: `file` owns a valid handle and `info` is writable for the call.
    if unsafe { GetFileInformationByHandle(file.as_raw_handle(), &mut info) } == 0 {
        return Err(SourceReadError::Io);
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
        return Err(SourceReadError::Io);
    }
    Ok(HandleSnapshot {
        identity: FileIdentity {
            volume: u64::from(info.dwVolumeSerialNumber),
            file: (u64::from(info.nFileIndexHigh) << 32) | u64::from(info.nFileIndexLow),
        },
        byte_len: (u64::from(info.nFileSizeHigh) << 32) | u64::from(info.nFileSizeLow),
        link_count: u64::from(info.nNumberOfLinks),
        change_stamp: ChangeStamp([
            basic.ChangeTime,
            basic.LastWriteTime,
            basic.CreationTime,
            i64::from(basic.FileAttributes),
        ]),
        is_directory: info.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY != 0,
        is_reparse: info.dwFileAttributes & FILE_ATTRIBUTE_REPARSE_POINT != 0,
    })
}

#[cfg(unix)]
fn snapshot_open_handle(file: &File) -> Result<HandleSnapshot, SourceReadError> {
    use std::os::unix::fs::MetadataExt;

    let metadata = file.metadata().map_err(|_| SourceReadError::Io)?;
    Ok(HandleSnapshot {
        identity: FileIdentity {
            volume: metadata.dev(),
            file: metadata.ino(),
        },
        byte_len: metadata.len(),
        link_count: metadata.nlink(),
        change_stamp: ChangeStamp([
            metadata.mtime(),
            metadata.mtime_nsec(),
            metadata.ctime(),
            metadata.ctime_nsec(),
        ]),
        is_directory: metadata.is_dir(),
        is_reparse: false,
    })
}

fn map_catalog_error(error: CatalogError) -> Value {
    let (code, message) = match error {
        CatalogError::InvalidLimits(_) | CatalogError::LimitExceeded { .. } => (
            "AUTHORING_STORY_INVENTORY_BUILD_LIMIT",
            "a generation input exceeds the supported resource limits",
        ),
        CatalogError::UnsafeInput(_) | CatalogError::OutputAliasesInput { .. } => (
            "AUTHORING_STORY_INVENTORY_BUILD_UNSAFE_INPUT",
            "a generation input is not a safe single-link regular file",
        ),
        CatalogError::IdentityChanged(_) | CatalogError::SourceChanged { .. } => (
            "AUTHORING_STORY_INVENTORY_BUILD_INPUT_CHANGED",
            "a generation input changed while the inventory was being built",
        ),
        CatalogError::UnsupportedGeneration { .. } => (
            "AUTHORING_STORY_INVENTORY_BUILD_UNSUPPORTED_GENERATION",
            "the three inputs do not match the pinned supported game generation",
        ),
        CatalogError::Io { source, .. } if source.kind() == io::ErrorKind::NotFound => (
            "AUTHORING_STORY_INVENTORY_BUILD_INPUT_MISSING",
            "a required generation input does not exist",
        ),
        CatalogError::Io { .. } => (
            "AUTHORING_STORY_INVENTORY_BUILD_INPUT_IO",
            "a generation input could not be read safely",
        ),
        CatalogError::MissingInputGuard => (
            "AUTHORING_STORY_INVENTORY_BUILD_FAILED",
            "built Story catalog lost its generation-input guard",
        ),
        _ => (
            "AUTHORING_STORY_INVENTORY_BUILD_FAILED",
            "the base-game Story inventory could not be built",
        ),
    };
    err(code, message)
}

fn map_read_error(error: SourceReadError) -> Value {
    let (code, message) = match error {
        SourceReadError::Missing => (
            "AUTHORING_STORY_INVENTORY_BUILD_INPUT_MISSING",
            "a required generation input does not exist",
        ),
        SourceReadError::Unsafe => (
            "AUTHORING_STORY_INVENTORY_BUILD_UNSAFE_INPUT",
            "a generation input is not a safe single-link regular file",
        ),
        SourceReadError::Limit => (
            "AUTHORING_STORY_INVENTORY_BUILD_LIMIT",
            "a generation input exceeds the supported resource limits",
        ),
        SourceReadError::Changed => (
            "AUTHORING_STORY_INVENTORY_BUILD_INPUT_CHANGED",
            "a generation input changed while the inventory was being built",
        ),
        SourceReadError::Io => (
            "AUTHORING_STORY_INVENTORY_BUILD_INPUT_IO",
            "a generation input could not be read safely",
        ),
    };
    err(code, message)
}

fn map_inventory_error(error: StoryInventoryError) -> Value {
    let code = if matches!(error, StoryInventoryError::LimitExceeded { .. }) {
        "AUTHORING_STORY_INVENTORY_BUILD_LIMIT"
    } else {
        "AUTHORING_STORY_INVENTORY_BUILD_FAILED"
    };
    err(
        code,
        "the sealed base-game Story inventory could not be built",
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn request_schema_binding_and_path_limits_are_closed() {
        let valid = json!({
            "executable": "A/game.exe",
            "shipping_cache": "B/Shipping.cache",
            "binds_cache": "C/Binds.cache",
        });
        let object = exact_payload(&valid).unwrap();
        let binding = request_binding(
            bounded_path(object, "executable").unwrap(),
            bounded_path(object, "shipping_cache").unwrap(),
            bounded_path(object, "binds_cache").unwrap(),
        );
        assert_eq!(binding.len(), 64);
        assert_ne!(
            binding,
            request_binding("A/game.exe", "C/Binds.cache", "B/Shipping.cache")
        );
        for invalid in [
            Value::Null,
            json!({}),
            json!({"executable":"a","shipping_cache":"b","binds_cache":"c","extra":true}),
            json!({"executable":"","shipping_cache":"b","binds_cache":"c"}),
            json!({"executable":"a\0b","shipping_cache":"b","binds_cache":"c"}),
            json!({"executable":"x".repeat(MAX_PATH_BYTES + 1),"shipping_cache":"b","binds_cache":"c"}),
        ] {
            assert_eq!(
                build_story_inventory_v1(invalid)["error"]["code"],
                "AUTHORING_STORY_INVENTORY_BUILD_REQUEST_INVALID"
            );
        }
    }

    #[test]
    fn guarded_reader_is_bounded_and_rejects_hard_links() {
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("source.cache");
        fs::write(&source, b"bounded source").unwrap();
        assert_eq!(
            read_source_no_follow(&source, 1024).unwrap(),
            b"bounded source"
        );
        assert!(matches!(
            read_source_no_follow(&source, 3),
            Err(SourceReadError::Limit)
        ));

        let alias = root.path().join("alias.cache");
        fs::hard_link(&source, &alias).unwrap();
        assert!(matches!(
            read_source_no_follow(&source, 1024),
            Err(SourceReadError::Unsafe)
        ));
    }

    #[test]
    fn transport_maps_untrusted_fixture_paths_without_leaking_them() {
        let root = tempfile::tempdir().unwrap();
        let executable = root.path().join("fixture-game.exe");
        let shipping = root.path().join("fixture-shipping.cache");
        let binds = root.path().join("fixture-binds.cache");
        fs::write(&executable, b"fixture executable").unwrap();
        fs::write(&shipping, b"fixture shipping").unwrap();
        fs::write(&binds, b"fixture binds").unwrap();
        let payload = json!({
            "executable": executable.to_string_lossy(),
            "shipping_cache": shipping.to_string_lossy(),
            "binds_cache": binds.to_string_lossy(),
        });
        let response = build_story_inventory_v1(payload);
        assert_eq!(
            response["error"]["code"],
            "AUTHORING_STORY_INVENTORY_BUILD_UNSUPPORTED_GENERATION"
        );
        assert!(!response
            .to_string()
            .contains(root.path().to_string_lossy().as_ref()));

        let request = serde_json::to_string(&json!({
            "command": "authoring_story_inventory_v1_build",
            "payload": {
                "executable": executable.to_string_lossy(),
                "shipping_cache": shipping.to_string_lossy(),
                "binds_cache": binds.to_string_lossy(),
            },
        }))
        .unwrap();
        let dispatched: Value = serde_json::from_str(&crate::execute_json(&request)).unwrap();
        assert_eq!(dispatched, response);
    }

    #[test]
    #[ignore = "requires GORE_STORY_INVENTORY_EXE, _SHIPPING, and _BINDS for the pinned install"]
    fn configured_pinned_install_builds_a_closed_read_only_artifact() {
        let executable = std::env::var("GORE_STORY_INVENTORY_EXE").unwrap();
        let shipping = std::env::var("GORE_STORY_INVENTORY_SHIPPING").unwrap();
        let binds = std::env::var("GORE_STORY_INVENTORY_BINDS").unwrap();
        let response = build_story_inventory_v1_inner(&json!({
            "executable": executable,
            "shipping_cache": shipping,
            "binds_cache": binds,
        }))
        .unwrap();
        assert_eq!(response["ok"], true);
        assert_eq!(response["coverage"], "base_game_only");
        assert_eq!(response["runtime_qualification"], "runtime_unqualified");
        assert_eq!(response["publication_status"], "not_supported");
        let artifact: Value = serde_json::from_str(response["inventory_json"].as_str().unwrap())
            .expect("native command returned canonical inventory JSON");
        assert_eq!(artifact["format"], "story_script_collision_inventory");
        assert_eq!(artifact["schema_revision"], 1);
        assert!(artifact["inventory"]["modules"]
            .as_array()
            .is_some_and(|entries| !entries.is_empty()));
        assert!(artifact["inventory"]["symbols"]
            .as_array()
            .is_some_and(|entries| !entries.is_empty()));
    }
}
