//! Exact-current materialization of one managed revision-3 Voice take for local preview.
//!
//! On Windows, native code atomically creates and retains one unique system-temporary directory as
//! a narrow cleanup capability before selecting the exact current Store asset. It writes only the
//! fixed `preview.ogg` leaf with create-new semantics and returns that ephemeral copy. Platforms
//! without the exact retained-handle cleanup contract fail closed at registration. No private CAS
//! path, caller-chosen output filename, game/save path, build, deployment, publication, or project
//! mutation authority crosses this boundary.

use std::collections::{HashMap, VecDeque};
use std::ffi::OsString;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

use gore_authoring::{
    bind_revision3_voice_take_preview_v1, inspect_revision3_voice_take_media_qa_v1,
    AssetVerification, Revision3VoiceTakePreviewConflictV1,
    Revision3VoiceTakePreviewRequestJsonErrorV1, Revision3VoiceTakePreviewRequestV1, Sha256Digest,
    WorkingHead, WorkingProjectStore, WorkingStoreError, WorkingStoreLimits,
    MAX_PROJECT_JSON_BYTES, MAX_REVISION3_VOICE_TAKE_PREVIEW_REQUEST_JSON_BYTES_V1,
};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use crate::err;

pub(super) const REGISTER_COMMAND: &str =
    "authoring_store_register_revision3_voice_take_preview_v1";
pub(super) const COMMAND: &str = "authoring_store_materialize_revision3_voice_take_preview_v1";
pub(super) const RELEASE_COMMAND: &str = "authoring_store_release_revision3_voice_take_preview_v1";
const PREVIEW_FILE_NAME: &str = "preview.ogg";
const PREVIEW_ROOT_PREFIX: &str = "gore-mod-studio-voice-preview-";
const CLEANUP_TOKEN_BYTES: usize = 32;
const CLEANUP_TOKEN_HEX_BYTES: usize = CLEANUP_TOKEN_BYTES * 2;
const MAX_ACTIVE_PREVIEW_CAPABILITIES: usize = 64;
const MAX_RELEASED_PREVIEW_TOMBSTONES: usize = 256;
const MAX_PATH_BYTES: usize = 32 * 1024;
const MAX_RESPONSE_BYTES: usize = 512 * 1024;
const MAX_ERROR_MESSAGE_BYTES: usize = 4 * 1024;
const MAX_WIRE_BYTES: usize =
    MAX_REVISION3_VOICE_TAKE_PREVIEW_REQUEST_JSON_BYTES_V1 * 2 + MAX_PATH_BYTES * 12 + 8 * 1024;

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ExactWireRequest<P> {
    command: String,
    payload: P,
}

/// Field order is part of the canonical outer transport.
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct MaterializeVoicePreviewWirePayload {
    root: String,
    cleanup_token: String,
    voice_take_preview_request_json: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct RegisterVoicePreviewWirePayload {
    root: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ReleaseVoicePreviewWirePayload {
    cleanup_token: String,
}

#[derive(Debug)]
struct Failure {
    code: &'static str,
    message: String,
}

impl Failure {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: truncate_utf8(message.into(), MAX_ERROR_MESSAGE_BYTES),
        }
    }

    fn response(self) -> Value {
        err(self.code, self.message)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct ObjectIdentity {
    device: u64,
    inode: u64,
}

#[derive(Debug)]
struct HeldDirectoryIdentity {
    file: File,
    directory: cap_std::fs::Dir,
    identity: ObjectIdentity,
}

#[derive(Debug)]
enum RetainedPreviewLeaf {
    ExactHandle(File),
    Identified {
        identity: ObjectIdentity,
        byte_len: u64,
        sha256: Sha256Digest,
        complete: bool,
        sealed: Option<File>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PreviewCapabilityState {
    Registered,
    Materialized,
    Released,
}

#[derive(Debug)]
struct PreviewCapabilityEntry {
    requested_store_root: PathBuf,
    canonical_store_root: PathBuf,
    registered_store: HeldDirectoryIdentity,
    requested_root: PathBuf,
    canonical_root: PathBuf,
    root_name: OsString,
    requested_system_temp: PathBuf,
    canonical_system_temp: PathBuf,
    system_temp: HeldDirectoryIdentity,
    preview_root: HeldDirectoryIdentity,
    retained_leaf: Option<RetainedPreviewLeaf>,
    state: PreviewCapabilityState,
}

#[derive(Default)]
struct PreviewCapabilityRegistry {
    active: HashMap<String, Arc<Mutex<PreviewCapabilityEntry>>>,
    active_roots: HashMap<ObjectIdentity, String>,
    released: VecDeque<String>,
}

fn preview_capability_registry() -> &'static Mutex<PreviewCapabilityRegistry> {
    static REGISTRY: OnceLock<Mutex<PreviewCapabilityRegistry>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(PreviewCapabilityRegistry::default()))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RegularFileSnapshot {
    identity: ObjectIdentity,
    byte_len: u64,
    link_count: u64,
    is_regular: bool,
    is_reparse: bool,
}

pub(super) fn register_revision3_voice_take_preview_v1_raw(input: &str) -> Value {
    register_revision3_voice_take_preview_v1_inner(input).unwrap_or_else(Failure::response)
}

fn register_revision3_voice_take_preview_v1_inner(input: &str) -> Result<Value, Failure> {
    let payload: RegisterVoicePreviewWirePayload = parse_exact_wire(input, REGISTER_COMMAND)?;
    validate_path(&payload.root)?;
    #[cfg(not(windows))]
    {
        let _ = payload;
        return Err(preview_registry_unavailable());
    }
    #[cfg(windows)]
    register_native_windows_preview_capability(&payload.root)
}

#[cfg(windows)]
fn register_native_windows_preview_capability(
    requested_store_root: &str,
) -> Result<Value, Failure> {
    {
        let registry = preview_capability_registry()
            .lock()
            .map_err(|_| preview_registry_unavailable())?;
        if registry.active.len() >= MAX_ACTIVE_PREVIEW_CAPABILITIES {
            return Err(preview_capability_limit());
        }
    }

    let requested_store_root = Path::new(requested_store_root);
    if !requested_store_root.is_absolute() {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_PREVIEW_STORE_UNSAFE",
            "managed Store root must be absolute for preview capability registration",
        ));
    }
    let canonical_store_root =
        canonical_existing_directory_no_reparse(requested_store_root, DirectoryKind::Store)?;
    let registered_store = hold_shared_directory_identity(&canonical_store_root).map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_VOICE_PREVIEW_STORE_UNAVAILABLE",
            "managed Store root identity could not be retained for preview registration",
        )
    })?;
    let entry = create_native_windows_preview_entry(
        requested_store_root.to_path_buf(),
        canonical_store_root,
        registered_store,
    )?;
    let root_identity = entry.preview_root.identity;
    let identities_overlap =
        match pinned_directories_overlap(&entry.registered_store, &entry.preview_root) {
            Ok(overlap) => overlap,
            Err(_) => {
                return Err(discard_unregistered_preview_entry(
                    entry,
                    preview_capability_invalid("directory ancestry could not be proven"),
                ));
            }
        };
    if paths_overlap(&entry.canonical_store_root, &entry.canonical_root) || identities_overlap {
        return Err(discard_unregistered_preview_entry(
            entry,
            preview_capability_invalid(
                "native preview root unexpectedly overlaps the managed Store",
            ),
        ));
    }
    let preview_root_string = match entry.requested_root.to_str() {
        Some(path) => path.to_owned(),
        None => {
            return Err(discard_unregistered_preview_entry(
                entry,
                preview_capability_invalid("preview root is not Unicode"),
            ));
        }
    };
    let preview_path_string = match entry.requested_root.join(PREVIEW_FILE_NAME).to_str() {
        Some(path) => path.to_owned(),
        None => {
            return Err(discard_unregistered_preview_entry(
                entry,
                preview_capability_invalid("preview output path is not Unicode"),
            ));
        }
    };
    if let Err(failure) = validate_path(&preview_root_string) {
        return Err(discard_unregistered_preview_entry(entry, failure));
    }
    if let Err(failure) = validate_path(&preview_path_string) {
        return Err(discard_unregistered_preview_entry(entry, failure));
    }
    match named_child_directory_identity(&entry.system_temp.directory, &entry.root_name) {
        Ok(identity) if identity == root_identity => {}
        _ => {
            return Err(discard_unregistered_preview_entry(
                entry,
                preview_capability_invalid("native preview root identity changed during creation"),
            ));
        }
    }

    let mut registry = match preview_capability_registry().lock() {
        Ok(registry) => registry,
        Err(_) => {
            return Err(discard_unregistered_preview_entry(
                entry,
                preview_registry_unavailable(),
            ));
        }
    };
    if registry.active.len() >= MAX_ACTIVE_PREVIEW_CAPABILITIES {
        drop(registry);
        return Err(discard_unregistered_preview_entry(
            entry,
            preview_capability_limit(),
        ));
    }
    if registry.active_roots.contains_key(&root_identity) {
        drop(registry);
        return Err(discard_unregistered_preview_entry(
            entry,
            Failure::new(
                "AUTHORING_REVISION3_VOICE_PREVIEW_PREVIEW_CAPABILITY_CONFLICT",
                "native preview root is already retained by another cleanup capability",
            ),
        ));
    }
    let mut cleanup_token = None;
    for _ in 0..64 {
        let token = match fresh_cleanup_token() {
            Ok(token) => token,
            Err(failure) => {
                drop(registry);
                return Err(discard_unregistered_preview_entry(entry, failure));
            }
        };
        if !registry.active.contains_key(&token)
            && !registry.released.iter().any(|released| released == &token)
        {
            cleanup_token = Some(token);
            break;
        }
    }
    let Some(cleanup_token) = cleanup_token else {
        drop(registry);
        return Err(discard_unregistered_preview_entry(
            entry,
            preview_registry_unavailable(),
        ));
    };
    let response = match enforce_response_budget(json!({
        "ok": true,
        "outcome": "preview_capability_registered",
        "cleanup_token": cleanup_token,
        "preview_root": preview_root_string,
        "preview_path": preview_path_string,
        "preview_leaf": PREVIEW_FILE_NAME,
        "preview_authority": "native_owned_ephemeral_temp_capability_v1",
        "preview_lifecycle": "native_opaque_cleanup_capability_v1",
        "project_write_status": "not_performed",
        "game_write_status": "not_performed",
        "save_write_status": "not_performed",
        "build_status": "not_performed",
        "deployment_status": "not_performed",
        "runtime_status": "not_qualified",
    })) {
        Ok(response) => response,
        Err(failure) => {
            drop(registry);
            return Err(discard_unregistered_preview_entry(entry, failure));
        }
    };

    registry
        .active_roots
        .insert(root_identity, cleanup_token.clone());
    registry
        .active
        .insert(cleanup_token, Arc::new(Mutex::new(entry)));
    Ok(response)
}

#[cfg(windows)]
fn create_native_windows_preview_entry(
    requested_store_root: PathBuf,
    canonical_store_root: PathBuf,
    registered_store: HeldDirectoryIdentity,
) -> Result<PreviewCapabilityEntry, Failure> {
    use std::mem::size_of;
    use std::os::windows::ffi::OsStrExt as _;
    use std::os::windows::io::{AsRawHandle as _, FromRawHandle as _};
    use windows_sys::Wdk::Foundation::OBJECT_ATTRIBUTES;
    use windows_sys::Wdk::Storage::FileSystem::{
        NtCreateFile, FILE_CREATE, FILE_DIRECTORY_FILE, FILE_SYNCHRONOUS_IO_NONALERT,
    };
    use windows_sys::Win32::Foundation::{
        RtlNtStatusToDosError, HANDLE, OBJ_CASE_INSENSITIVE, OBJ_DONT_REPARSE,
        STATUS_OBJECT_NAME_COLLISION, UNICODE_STRING,
    };
    use windows_sys::Win32::Storage::FileSystem::{
        DELETE, FILE_ADD_FILE, FILE_ATTRIBUTE_NORMAL, FILE_LIST_DIRECTORY, FILE_READ_ATTRIBUTES,
        FILE_SHARE_READ, FILE_SHARE_WRITE, FILE_TRAVERSE, SYNCHRONIZE,
    };
    use windows_sys::Win32::System::IO::IO_STATUS_BLOCK;

    let requested_system_temp = std::env::temp_dir();
    if !requested_system_temp.is_absolute() {
        return Err(preview_capability_invalid(
            "system temporary root must resolve to one absolute directory",
        ));
    }
    let canonical_system_temp =
        canonical_existing_directory_no_reparse(&requested_system_temp, DirectoryKind::SystemTemp)?;
    let held_system_temp = hold_shared_directory_identity(&canonical_system_temp)
        .map_err(|_| preview_capability_invalid("system temporary root is unavailable"))?;
    require_system_temp_outside_store(
        &canonical_system_temp,
        &held_system_temp,
        &canonical_store_root,
        &registered_store,
    )?;
    let parent = held_system_temp
        .file
        .try_clone()
        .map_err(|_| preview_registry_unavailable())?;

    for _ in 0..64 {
        let root_name = OsString::from(format!("{PREVIEW_ROOT_PREFIX}{}", fresh_cleanup_token()?));
        let mut name = root_name.encode_wide().collect::<Vec<_>>();
        let name_bytes = name
            .len()
            .checked_mul(size_of::<u16>())
            .and_then(|length| u16::try_from(length).ok())
            .ok_or_else(preview_registry_unavailable)?;
        let unicode_name = UNICODE_STRING {
            Length: name_bytes,
            MaximumLength: name_bytes,
            Buffer: name.as_mut_ptr(),
        };
        let object_attributes = OBJECT_ATTRIBUTES {
            Length: size_of::<OBJECT_ATTRIBUTES>() as u32,
            RootDirectory: parent.as_raw_handle() as HANDLE,
            ObjectName: &unicode_name,
            Attributes: OBJ_CASE_INSENSITIVE | OBJ_DONT_REPARSE,
            SecurityDescriptor: std::ptr::null(),
            SecurityQualityOfService: std::ptr::null(),
        };
        let mut raw_handle: HANDLE = std::ptr::null_mut();
        let mut io_status = IO_STATUS_BLOCK::default();
        // SAFETY: parent is a live pinned directory handle; `unicode_name` is one bounded child
        // component whose backing UTF-16 buffer outlives the call; both ABI output pointers are
        // valid. FILE_CREATE makes creation atomic and never opens an existing name.
        let status = unsafe {
            NtCreateFile(
                &mut raw_handle,
                FILE_LIST_DIRECTORY
                    | FILE_ADD_FILE
                    | FILE_TRAVERSE
                    | FILE_READ_ATTRIBUTES
                    | SYNCHRONIZE
                    | DELETE,
                &object_attributes,
                &mut io_status,
                std::ptr::null(),
                FILE_ATTRIBUTE_NORMAL,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                FILE_CREATE,
                FILE_DIRECTORY_FILE | FILE_SYNCHRONOUS_IO_NONALERT,
                std::ptr::null(),
                0,
            )
        };
        if status == STATUS_OBJECT_NAME_COLLISION {
            continue;
        }
        if status < 0 {
            return Err(Failure::new(
                "AUTHORING_REVISION3_VOICE_PREVIEW_PREVIEW_CAPABILITY_UNAVAILABLE",
                format!("native preview root could not be created ({})", unsafe {
                    RtlNtStatusToDosError(status)
                }),
            ));
        }
        if raw_handle.is_null() {
            return Err(preview_registry_unavailable());
        }
        // SAFETY: NtCreateFile returned one new owned handle on success.
        let file = unsafe { File::from_raw_handle(raw_handle) };
        let identity = match directory_identity(&file) {
            Ok(identity) => identity,
            Err(_) => {
                let _ = delete_file_by_exact_handle(&file);
                return Err(preview_capability_invalid(
                    "native preview root is not a directory",
                ));
            }
        };
        let cloned = match file.try_clone() {
            Ok(cloned) => cloned,
            Err(_) => {
                let _ = delete_file_by_exact_handle(&file);
                return Err(preview_registry_unavailable());
            }
        };
        let directory = cap_std::fs::Dir::from_std_file(cloned);
        let requested_root = requested_system_temp.join(&root_name);
        let canonical_root = canonical_system_temp.join(&root_name);
        return Ok(PreviewCapabilityEntry {
            requested_store_root,
            canonical_store_root,
            registered_store,
            requested_root,
            canonical_root,
            root_name,
            requested_system_temp,
            canonical_system_temp,
            system_temp: held_system_temp,
            preview_root: HeldDirectoryIdentity {
                file,
                directory,
                identity,
            },
            retained_leaf: None,
            state: PreviewCapabilityState::Registered,
        });
    }
    Err(preview_registry_unavailable())
}

fn require_system_temp_outside_store(
    canonical_system_temp: &Path,
    held_system_temp: &HeldDirectoryIdentity,
    canonical_store_root: &Path,
    registered_store: &HeldDirectoryIdentity,
) -> Result<(), Failure> {
    if canonical_system_temp.starts_with(canonical_store_root)
        || pinned_directory_is_at_or_below(&held_system_temp.directory, registered_store.identity)
            .map_err(|_| preview_capability_invalid("directory ancestry could not be proven"))?
    {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_PREVIEW_PREVIEW_CAPABILITY_INVALID",
            "system temporary root must not be the managed Store or one of its descendants",
        ));
    }
    Ok(())
}

#[cfg(windows)]
fn discard_unregistered_preview_entry(
    mut entry: PreviewCapabilityEntry,
    original: Failure,
) -> Failure {
    if release_preview_capability_entry(&mut entry).is_ok() {
        original
    } else {
        preview_registry_unavailable()
    }
}

fn preview_capability_limit() -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_VOICE_PREVIEW_PREVIEW_CAPABILITY_LIMIT",
        "too many Voice preview cleanup capabilities are active",
    )
}

pub(super) fn release_revision3_voice_take_preview_v1_raw(input: &str) -> Value {
    release_revision3_voice_take_preview_v1_inner(input).unwrap_or_else(Failure::response)
}

fn release_revision3_voice_take_preview_v1_inner(input: &str) -> Result<Value, Failure> {
    let payload: ReleaseVoicePreviewWirePayload = parse_exact_wire(input, RELEASE_COMMAND)?;
    validate_cleanup_token(&payload.cleanup_token)?;
    let entry = {
        let registry = preview_capability_registry()
            .lock()
            .map_err(|_| preview_registry_unavailable())?;
        if let Some(entry) = registry.active.get(&payload.cleanup_token) {
            Arc::clone(entry)
        } else if registry
            .released
            .iter()
            .any(|token| token == &payload.cleanup_token)
        {
            return cleanup_success_response();
        } else {
            return Err(Failure::new(
                "AUTHORING_REVISION3_VOICE_PREVIEW_CLEANUP_TOKEN_UNKNOWN",
                "Voice preview cleanup token is unknown or no longer retained",
            ));
        }
    };

    let root_identity = {
        let mut entry = entry.lock().map_err(|_| preview_registry_unavailable())?;
        if entry.state != PreviewCapabilityState::Released {
            release_preview_capability_entry(&mut entry)?;
            entry.state = PreviewCapabilityState::Released;
        }
        entry.preview_root.identity
    };

    let mut registry = preview_capability_registry()
        .lock()
        .map_err(|_| preview_registry_unavailable())?;
    registry.active.remove(&payload.cleanup_token);
    registry.active_roots.remove(&root_identity);
    if !registry
        .released
        .iter()
        .any(|token| token == &payload.cleanup_token)
    {
        registry.released.push_back(payload.cleanup_token);
        while registry.released.len() > MAX_RELEASED_PREVIEW_TOMBSTONES {
            registry.released.pop_front();
        }
    }
    cleanup_success_response()
}

pub(super) fn materialize_revision3_voice_take_preview_v1_raw(input: &str) -> Value {
    materialize_revision3_voice_take_preview_v1_inner(input).unwrap_or_else(Failure::response)
}

fn materialize_revision3_voice_take_preview_v1_inner(input: &str) -> Result<Value, Failure> {
    materialize_revision3_voice_take_preview_v1_inner_with_guard(input, |_, _| {})
}

fn materialize_revision3_voice_take_preview_v1_inner_with_guard<F>(
    input: &str,
    after_materialize_guard: F,
) -> Result<Value, Failure>
where
    F: FnOnce(&Path, &Path),
{
    let payload: MaterializeVoicePreviewWirePayload = parse_exact_wire(input, COMMAND)?;
    validate_path(&payload.root)?;
    validate_cleanup_token(&payload.cleanup_token)?;
    let request =
        Revision3VoiceTakePreviewRequestV1::from_json(&payload.voice_take_preview_request_json)
            .map_err(map_request_error)?;
    require_signed_request(&request)?;

    let requested_store_root = Path::new(&payload.root);
    let capability = lookup_active_preview_capability(&payload.cleanup_token)?;
    let mut capability = capability
        .lock()
        .map_err(|_| preview_registry_unavailable())?;
    if capability.state != PreviewCapabilityState::Registered || capability.retained_leaf.is_some()
    {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_PREVIEW_PREVIEW_OUTPUT_CONFLICT",
            "cleanup token already owns a materialized Voice preview",
        ));
    }
    let requested_preview_root = capability.requested_root.clone();
    let canonical_preview_root = capability.canonical_root.clone();

    let canonical_store_root =
        canonical_existing_directory_no_reparse(requested_store_root, DirectoryKind::Store)?;
    let held_store_root = hold_directory_identity(&canonical_store_root).map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_VOICE_PREVIEW_STORE_UNAVAILABLE",
            "managed Store root identity could not be captured safely",
        )
    })?;
    if canonical_store_root != capability.canonical_store_root
        || held_store_root.identity != capability.registered_store.identity
    {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_PREVIEW_STORE_ROOT_CHANGED",
            "managed Store root differs from the registered preview capability basis",
        ));
    }
    revalidate_directory_identity(
        &capability.requested_store_root,
        &capability.canonical_store_root,
        &capability.registered_store,
        DirectoryKind::Store,
    )?;
    if paths_overlap(&canonical_store_root, &canonical_preview_root)
        || pinned_directories_overlap(&held_store_root, &capability.preview_root)
            .map_err(|_| preview_capability_invalid("directory ancestry could not be proven"))?
    {
        return Err(preview_capability_invalid(
            "preview_root must not contain or be contained by the managed Store",
        ));
    }
    ensure_initial_preview_root_empty(&capability.preview_root.directory)?;

    let store = WorkingProjectStore::open_existing(&canonical_store_root, ffi_store_limits())
        .map_err(map_store_open_error)?;
    // Preview is deliberately browsing-scoped: immutable project/head/entity objects and every
    // asset shape are reopened structurally, while the selected Voice Ogg is fully hashed below.
    // This avoids hashing an unrelated multi-gigabyte CAS twice merely to audition one take.
    let basis = store
        .open_current_revision3(AssetVerification::Structural)
        .map_err(map_store_open_error)?;
    require_signed_basis(&basis.head, &basis.project, &request)?;
    let binding = bind_revision3_voice_take_preview_v1(&basis.head, &basis.project, &request)
        .map_err(map_binding_conflict)?;

    let source_bytes = store
        .read_verified_ogg_asset(&binding.asset)
        .map_err(|error| map_selected_asset_error(error, &canonical_store_root))?;
    let actual_ogg = inspect_revision3_voice_take_media_qa_v1(&source_bytes)
        .map_err(|_| asset_invalid("selected VoiceTake Ogg bytes are not valid preview input"))?
        .ogg()
        .clone();
    if actual_ogg != binding.ogg {
        return Err(asset_invalid(
            "selected VoiceTake Ogg metadata differs from its exact project declaration",
        ));
    }

    if named_child_directory_identity(&capability.system_temp.directory, &capability.root_name)
        .map_err(|_| {
            Failure::new(
                "AUTHORING_REVISION3_VOICE_PREVIEW_PREVIEW_CAPABILITY_CHANGED",
                "preview_root changed before materialization",
            )
        })?
        != capability.preview_root.identity
        || pinned_directories_overlap(&held_store_root, &capability.preview_root)
            .map_err(|_| preview_capability_invalid("directory ancestry could not be proven"))?
    {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_PREVIEW_PREVIEW_CAPABILITY_CHANGED",
            "preview_root identity or ancestry changed before materialization",
        ));
    }

    let output_identity =
        materialize_preview_file_owned(&mut capability, &source_bytes, binding.asset.sha256)?;

    let basis_head_json = canonical_head_json(&basis.head)?;
    // Return the caller's already-validated absolute capability spelling, not Windows' internal
    // extended-length canonical prefix. The fixed leaf is still materialized through the held
    // canonical directory identity above.
    let response_preview_path = requested_preview_root.join(PREVIEW_FILE_NAME);
    let preview_path_string = response_preview_path
        .to_str()
        .ok_or_else(|| preview_capability_invalid("preview output path is not Unicode"))?;
    let response = enforce_response_budget(json!({
        "ok": true,
        "outcome": "preview_ready",
        "basis_head_json": basis_head_json,
        "project_id": binding.project_id.to_string(),
        "project_revision": binding.project_revision,
        "line_id": binding.line_id.to_string(),
        "line_revision": binding.line_revision,
        "localization_id": binding.localization_id.to_string(),
        "localization_revision": binding.localization_revision,
        "loc_id": binding.loc_id,
        "slot_id": binding.slot_id.to_string(),
        "slot_revision": binding.slot_revision,
        "locale": binding.locale.to_string(),
        "take_id": binding.take_id.to_string(),
        "take_revision": binding.take_revision,
        "asset": binding.asset,
        "status": binding.status,
        "ogg": binding.ogg,
        "preview_path": preview_path_string,
        "preview_leaf": PREVIEW_FILE_NAME,
        "preview_authority": "exact_current_managed_cas_voice_take_v1",
        "cleanup_token": payload.cleanup_token,
        "preview_lifecycle": "native_opaque_cleanup_capability_v1",
        "project_write_status": "not_performed",
        "game_write_status": "not_performed",
        "save_write_status": "not_performed",
        "build_status": "not_performed",
        "deployment_status": "not_performed",
        "runtime_status": "not_qualified",
    }))?;

    after_materialize_guard(&canonical_store_root, &canonical_preview_root);

    // Close every mutable window after output and response construction. Reopen the graph, bind
    // the request again, and fully hash only the selected asset a second time. Unrelated asset
    // corruption therefore never grants build authority and does not turn a one-take preview into
    // an O(total CAS) operation.
    let after = store
        .open_current_revision3(AssetVerification::Structural)
        .map_err(map_store_open_error)?;
    if after.head != basis.head || after.project != basis.project {
        return Err(head_conflict());
    }
    let after_binding = bind_revision3_voice_take_preview_v1(&after.head, &after.project, &request)
        .map_err(map_binding_conflict)?;
    if after_binding != binding {
        return Err(invariant());
    }
    let after_bytes = store
        .read_verified_ogg_asset(&after_binding.asset)
        .map_err(|error| map_selected_asset_error(error, &canonical_store_root))?;
    let after_ogg = inspect_revision3_voice_take_media_qa_v1(&after_bytes)
        .map_err(|_| asset_invalid("selected VoiceTake changed during preview"))?
        .ogg()
        .clone();
    if after_bytes != source_bytes || after_ogg != actual_ogg {
        return Err(asset_invalid(
            "selected VoiceTake changed during preview materialization",
        ));
    }

    revalidate_directory_identity(
        requested_store_root,
        &canonical_store_root,
        &held_store_root,
        DirectoryKind::Store,
    )?;
    revalidate_directory_identity(
        &capability.requested_system_temp,
        &capability.canonical_system_temp,
        &capability.system_temp,
        DirectoryKind::SystemTemp,
    )?;
    revalidate_registered_preview_entry(&capability)?;
    ensure_final_preview_root_shape(&capability.preview_root.directory)?;
    verify_preview_leaf(
        &capability.preview_root.directory,
        output_identity,
        binding.asset.byte_len,
        binding.asset.sha256,
    )?;

    Ok(response)
}

fn parse_exact_wire<P>(input: &str, expected_command: &str) -> Result<P, Failure>
where
    P: DeserializeOwned + Serialize,
{
    if input.len() > MAX_WIRE_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_PREVIEW_INPUT_LIMIT",
            "revision-3 Voice preview request exceeds its bounded wire limit",
        ));
    }
    let request: ExactWireRequest<P> =
        serde_json::from_str(input).map_err(|_| invalid_request())?;
    if request.command != expected_command {
        return Err(invalid_request());
    }
    let canonical = serde_json::to_string(&request).map_err(|_| invariant())?;
    if canonical != input {
        return Err(invalid_request());
    }
    Ok(request.payload)
}

fn fresh_cleanup_token() -> Result<String, Failure> {
    let mut bytes = [0u8; CLEANUP_TOKEN_BYTES];
    getrandom::fill(&mut bytes).map_err(|_| preview_registry_unavailable())?;
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut token = String::with_capacity(CLEANUP_TOKEN_HEX_BYTES);
    for byte in bytes {
        token.push(HEX[usize::from(byte >> 4)] as char);
        token.push(HEX[usize::from(byte & 0x0f)] as char);
    }
    Ok(token)
}

fn validate_cleanup_token(token: &str) -> Result<(), Failure> {
    if token.len() != CLEANUP_TOKEN_HEX_BYTES
        || !token
            .as_bytes()
            .iter()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(byte))
    {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_PREVIEW_INPUT_INVALID",
            "cleanup_token must be one exact opaque lowercase token",
        ));
    }
    Ok(())
}

fn lookup_active_preview_capability(
    cleanup_token: &str,
) -> Result<Arc<Mutex<PreviewCapabilityEntry>>, Failure> {
    let registry = preview_capability_registry()
        .lock()
        .map_err(|_| preview_registry_unavailable())?;
    registry
        .active
        .get(cleanup_token)
        .map(Arc::clone)
        .ok_or_else(|| {
            Failure::new(
                "AUTHORING_REVISION3_VOICE_PREVIEW_CLEANUP_TOKEN_UNKNOWN",
                "Voice preview cleanup token is unknown or no longer retained",
            )
        })
}

fn cleanup_success_response() -> Result<Value, Failure> {
    enforce_response_budget(json!({
        "ok": true,
        "outcome": "preview_cleanup_complete",
        "cleanup_status": "performed",
        "project_write_status": "not_performed",
        "game_write_status": "not_performed",
        "save_write_status": "not_performed",
        "build_status": "not_performed",
        "deployment_status": "not_performed",
        "runtime_status": "not_qualified",
    }))
}

fn release_preview_capability_entry(entry: &mut PreviewCapabilityEntry) -> Result<(), Failure> {
    if entry.state == PreviewCapabilityState::Released {
        return Ok(());
    }

    let entries = entry
        .preview_root
        .directory
        .entries()
        .map_err(|_| preview_cleanup_retained("preview capability contents are unavailable"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| preview_cleanup_retained("preview capability contents are unavailable"))?;
    if entries.len() > 1
        || entries
            .first()
            .is_some_and(|candidate| candidate.file_name() != PREVIEW_FILE_NAME)
    {
        return Err(preview_cleanup_retained(
            "preview capability contains an unexpected entry",
        ));
    }

    if entries.is_empty() {
        // The fixed leaf was already removed through some prior local cleanup attempt. Because
        // the pinned root is proven empty, there is no same-name object to chase or delete.
        entry.retained_leaf.take();
    }

    match entry.retained_leaf.take() {
        Some(leaf) => {
            if let Err((error, leaf)) =
                remove_retained_preview_leaf(&entry.preview_root.directory, leaf)
            {
                entry.retained_leaf = Some(leaf);
                return Err(preview_cleanup_retained(error));
            }
        }
        None if !entries.is_empty() => {
            return Err(preview_cleanup_retained(
                "unowned preview leaf appeared before cleanup",
            ));
        }
        None => {}
    }

    if entry
        .preview_root
        .directory
        .entries()
        .map_err(|_| preview_cleanup_retained("preview root could not be rechecked"))?
        .next()
        .transpose()
        .map_err(|_| preview_cleanup_retained("preview root could not be rechecked"))?
        .is_some()
    {
        return Err(preview_cleanup_retained(
            "preview root is not empty after fixed-leaf cleanup",
        ));
    }

    remove_preview_root_identity_safe(entry)
        .map_err(|_| preview_cleanup_retained("preview root could not be removed safely"))?;
    Ok(())
}

#[cfg(windows)]
fn remove_retained_preview_leaf(
    directory: &cap_std::fs::Dir,
    leaf: RetainedPreviewLeaf,
) -> Result<(), (&'static str, RetainedPreviewLeaf)> {
    use cap_std::fs::OpenOptionsExt as _;
    use windows_sys::Win32::Storage::FileSystem::{
        DELETE, FILE_FLAG_OPEN_REPARSE_POINT, FILE_GENERIC_READ, FILE_SHARE_READ,
    };

    match leaf {
        RetainedPreviewLeaf::ExactHandle(file) => {
            let snapshot = match snapshot_regular_file(&file) {
                Ok(snapshot) => snapshot,
                Err(_) => {
                    return Err((
                        "created preview leaf handle identity is unavailable",
                        RetainedPreviewLeaf::ExactHandle(file),
                    ));
                }
            };
            if !snapshot.is_regular || snapshot.is_reparse || snapshot.link_count != 1 {
                return Err((
                    "created preview leaf handle became unsafe",
                    RetainedPreviewLeaf::ExactHandle(file),
                ));
            }
            if delete_file_by_exact_handle(&file).is_err() {
                return Err((
                    "created preview leaf could not be removed by its exact handle",
                    RetainedPreviewLeaf::ExactHandle(file),
                ));
            }
            drop(file);
            Ok(())
        }
        RetainedPreviewLeaf::Identified {
            identity,
            byte_len,
            sha256,
            complete,
            mut sealed,
        } => {
            if let Some(mut seal) = sealed.take() {
                if !cleanup_leaf_handle_matches(
                    &mut seal,
                    identity,
                    complete.then_some((byte_len, sha256)),
                ) {
                    return Err((
                        "retained preview seal changed before cleanup",
                        RetainedPreviewLeaf::Identified {
                            identity,
                            byte_len,
                            sha256,
                            complete,
                            sealed: Some(seal),
                        },
                    ));
                }
                drop(seal);
            }
            let mut options = cap_std::fs::OpenOptions::new();
            options
                .access_mode(FILE_GENERIC_READ | DELETE)
                .share_mode(FILE_SHARE_READ)
                .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT);
            let mut file = match directory.open_with(PREVIEW_FILE_NAME, &options) {
                Ok(file) => file.into_std(),
                Err(_) => {
                    return Err((
                        "preview leaf could not be opened for exact cleanup",
                        retain_identified_preview_leaf(
                            directory, identity, byte_len, sha256, complete,
                        ),
                    ));
                }
            };
            if !cleanup_leaf_handle_matches(
                &mut file,
                identity,
                complete.then_some((byte_len, sha256)),
            ) {
                drop(file);
                return Err((
                    "preview leaf identity or seal changed before cleanup",
                    retain_identified_preview_leaf(directory, identity, byte_len, sha256, complete),
                ));
            }
            if delete_file_by_exact_handle(&file).is_err() {
                drop(file);
                return Err((
                    "preview leaf could not be removed by its verified handle",
                    retain_identified_preview_leaf(directory, identity, byte_len, sha256, complete),
                ));
            }
            drop(file);
            Ok(())
        }
    }
}

#[cfg(windows)]
fn retain_identified_preview_leaf(
    directory: &cap_std::fs::Dir,
    identity: ObjectIdentity,
    byte_len: u64,
    sha256: Sha256Digest,
    complete: bool,
) -> RetainedPreviewLeaf {
    let mut sealed = open_preview_file_sealed(directory).ok();
    if sealed.as_mut().is_some_and(|file| {
        !cleanup_leaf_handle_matches(file, identity, complete.then_some((byte_len, sha256)))
    }) {
        sealed = None;
    }
    RetainedPreviewLeaf::Identified {
        identity,
        byte_len,
        sha256,
        complete,
        sealed,
    }
}

#[cfg(unix)]
fn remove_retained_preview_leaf(
    _directory: &cap_std::fs::Dir,
    leaf: RetainedPreviewLeaf,
) -> Result<(), (&'static str, RetainedPreviewLeaf)> {
    Err((
        "exact retained-handle preview cleanup is unavailable on this platform",
        leaf,
    ))
}

fn cleanup_leaf_handle_matches(
    file: &mut File,
    expected_identity: ObjectIdentity,
    expected_complete: Option<(u64, Sha256Digest)>,
) -> bool {
    let Ok(snapshot) = snapshot_regular_file(file) else {
        return false;
    };
    if !snapshot.is_regular
        || snapshot.is_reparse
        || snapshot.link_count != 1
        || snapshot.identity != expected_identity
    {
        return false;
    }
    let Some((expected_len, expected_sha256)) = expected_complete else {
        return true;
    };
    if snapshot.byte_len != expected_len || file.seek(SeekFrom::Start(0)).is_err() {
        return false;
    }
    digest_reader(file).is_ok_and(|digest| digest == expected_sha256)
}

#[cfg(windows)]
fn delete_file_by_exact_handle(file: &File) -> io::Result<()> {
    use std::mem::size_of;
    use std::os::windows::io::AsRawHandle as _;
    use windows_sys::Win32::Foundation::HANDLE;
    use windows_sys::Win32::Storage::FileSystem::{
        FileDispositionInfo, SetFileInformationByHandle, FILE_DISPOSITION_INFO,
    };

    let disposition = FILE_DISPOSITION_INFO { DeleteFile: true };
    // SAFETY: file is an exact live handle opened with DELETE access, and disposition has the
    // documented ABI type and length. No ambient name participates in deletion.
    let deleted = unsafe {
        SetFileInformationByHandle(
            file.as_raw_handle() as HANDLE,
            FileDispositionInfo,
            (&disposition as *const FILE_DISPOSITION_INFO).cast(),
            size_of::<FILE_DISPOSITION_INFO>() as u32,
        )
    };
    if deleted == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(windows)]
fn remove_preview_root_identity_safe(entry: &mut PreviewCapabilityEntry) -> io::Result<()> {
    use std::mem::size_of;
    use std::os::windows::io::AsRawHandle as _;
    use windows_sys::Win32::Foundation::HANDLE;
    use windows_sys::Win32::Storage::FileSystem::{
        FileDispositionInfo, SetFileInformationByHandle, FILE_DISPOSITION_INFO,
    };

    let disposition = FILE_DISPOSITION_INFO { DeleteFile: true };
    // SAFETY: the retained preview handle was opened with DELETE access, names the exact verified
    // empty directory, and the ABI buffer has the documented type and length.
    let deleted = unsafe {
        SetFileInformationByHandle(
            entry.preview_root.file.as_raw_handle() as HANDLE,
            FileDispositionInfo,
            (&disposition as *const FILE_DISPOSITION_INFO).cast(),
            size_of::<FILE_DISPOSITION_INFO>() as u32,
        )
    };
    if deleted == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(unix)]
fn remove_preview_root_identity_safe(entry: &mut PreviewCapabilityEntry) -> io::Result<()> {
    let _ = entry;
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "exact retained-handle directory cleanup is unavailable on this platform",
    ))
}

fn preview_registry_unavailable() -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_VOICE_PREVIEW_PREVIEW_CAPABILITY_UNAVAILABLE",
        "Voice preview cleanup capability registry is unavailable",
    )
}

fn preview_cleanup_retained(message: &'static str) -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_VOICE_PREVIEW_CLEANUP_RETAINED",
        message,
    )
}

fn validate_path(path: &str) -> Result<(), Failure> {
    if path.is_empty() || path.len() > MAX_PATH_BYTES || path.contains('\0') {
        return Err(invalid_request());
    }
    Ok(())
}

#[derive(Debug, Clone, Copy)]
enum DirectoryKind {
    Store,
    Preview,
    SystemTemp,
}

fn canonical_existing_directory_no_reparse(
    path: &Path,
    kind: DirectoryKind,
) -> Result<PathBuf, Failure> {
    if path
        .components()
        .any(|component| component == Component::ParentDir)
    {
        return Err(directory_failure(
            kind,
            true,
            "directory path must not contain '..' traversal",
        ));
    }
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|_| directory_failure(kind, false, "directory could not be resolved"))?
            .join(path)
    };
    for ancestor in absolute.ancestors() {
        let metadata = fs::symlink_metadata(ancestor).map_err(|_| {
            directory_failure(kind, false, "directory has an unavailable path component")
        })?;
        if metadata_is_reparse(&metadata) || !metadata.is_dir() {
            return Err(directory_failure(
                kind,
                true,
                "directory crosses a symbolic link, reparse point, or non-directory",
            ));
        }
    }
    fs::canonicalize(&absolute)
        .map_err(|_| directory_failure(kind, false, "directory could not be canonicalized"))
}

fn directory_failure(kind: DirectoryKind, unsafe_path: bool, message: &'static str) -> Failure {
    match kind {
        DirectoryKind::Store if unsafe_path => {
            Failure::new("AUTHORING_REVISION3_VOICE_PREVIEW_STORE_UNSAFE", message)
        }
        DirectoryKind::Store => Failure::new(
            "AUTHORING_REVISION3_VOICE_PREVIEW_STORE_UNAVAILABLE",
            message,
        ),
        DirectoryKind::Preview | DirectoryKind::SystemTemp => preview_capability_invalid(message),
    }
}

fn metadata_is_reparse(metadata: &fs::Metadata) -> bool {
    if metadata.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt as _;
        use windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_REPARSE_POINT;
        metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
    }
    #[cfg(not(windows))]
    {
        false
    }
}

fn hold_directory_identity(path: &Path) -> io::Result<HeldDirectoryIdentity> {
    let file = open_directory_no_follow(path)?;
    let identity = directory_identity(&file)?;
    let directory = cap_std::fs::Dir::from_std_file(file.try_clone()?);
    Ok(HeldDirectoryIdentity {
        file,
        directory,
        identity,
    })
}

fn hold_shared_directory_identity(path: &Path) -> io::Result<HeldDirectoryIdentity> {
    let file = open_shared_directory_no_follow(path)?;
    let identity = directory_identity(&file)?;
    let directory = cap_std::fs::Dir::from_std_file(file.try_clone()?);
    Ok(HeldDirectoryIdentity {
        file,
        directory,
        identity,
    })
}

#[cfg(windows)]
fn named_child_directory_identity(
    parent: &cap_std::fs::Dir,
    name: &std::ffi::OsStr,
) -> io::Result<ObjectIdentity> {
    use cap_std::fs::OpenOptionsExt as _;
    use windows_sys::Win32::Storage::FileSystem::{
        FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT, FILE_GENERIC_READ,
        FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE,
    };

    let mut options = cap_std::fs::OpenOptions::new();
    options
        .access_mode(FILE_GENERIC_READ)
        // The retained root requests DELETE while deliberately denying delete sharing. This
        // read-only identity handle must nevertheless share that existing DELETE access.
        .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT);
    let file = parent.open_with(name, &options)?.into_std();
    directory_identity(&file)
}

#[cfg(unix)]
fn named_child_directory_identity(
    parent: &cap_std::fs::Dir,
    name: &std::ffi::OsStr,
) -> io::Result<ObjectIdentity> {
    use cap_std::fs::OpenOptionsExt as _;

    let mut options = cap_std::fs::OpenOptions::new();
    options
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC);
    let file = parent.open_with(name, &options)?.into_std();
    directory_identity(&file)
}

#[cfg(windows)]
fn open_directory_no_follow(path: &Path) -> io::Result<File> {
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
fn open_shared_directory_no_follow(path: &Path) -> io::Result<File> {
    use std::os::windows::fs::OpenOptionsExt as _;
    use windows_sys::Win32::Storage::FileSystem::{
        FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT, FILE_SHARE_DELETE,
        FILE_SHARE_READ, FILE_SHARE_WRITE,
    };

    let mut options = OpenOptions::new();
    options
        .read(true)
        .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT);
    options.open(path)
}

#[cfg(unix)]
fn open_directory_no_follow(path: &Path) -> io::Result<File> {
    use std::os::unix::fs::OpenOptionsExt as _;

    let mut options = OpenOptions::new();
    options
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC);
    options.open(path)
}

#[cfg(unix)]
fn open_shared_directory_no_follow(path: &Path) -> io::Result<File> {
    open_directory_no_follow(path)
}

#[cfg(windows)]
fn directory_identity(file: &File) -> io::Result<ObjectIdentity> {
    use std::mem::MaybeUninit;
    use std::os::windows::io::AsRawHandle as _;
    use windows_sys::Win32::Foundation::HANDLE;
    use windows_sys::Win32::Storage::FileSystem::{
        GetFileInformationByHandle, BY_HANDLE_FILE_INFORMATION, FILE_ATTRIBUTE_DIRECTORY,
        FILE_ATTRIBUTE_REPARSE_POINT,
    };

    let mut info = MaybeUninit::<BY_HANDLE_FILE_INFORMATION>::uninit();
    // SAFETY: `file` owns a live directory handle and `info` is the exact writable ABI type.
    if unsafe { GetFileInformationByHandle(file.as_raw_handle() as HANDLE, info.as_mut_ptr()) } == 0
    {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: a successful call initializes the complete structure.
    let info = unsafe { info.assume_init() };
    if info.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY == 0
        || info.dwFileAttributes & FILE_ATTRIBUTE_REPARSE_POINT != 0
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "handle is not one real directory",
        ));
    }
    Ok(ObjectIdentity {
        device: u64::from(info.dwVolumeSerialNumber),
        inode: (u64::from(info.nFileIndexHigh) << 32) | u64::from(info.nFileIndexLow),
    })
}

fn cap_directory_identity(directory: &cap_std::fs::Dir) -> io::Result<ObjectIdentity> {
    let file = directory.try_clone()?.into_std_file();
    directory_identity(&file)
}

fn pinned_directory_is_at_or_below(
    directory: &cap_std::fs::Dir,
    possible_ancestor: ObjectIdentity,
) -> io::Result<bool> {
    let mut current = directory.try_clone()?;
    for _ in 0..4_096 {
        let current_identity = cap_directory_identity(&current)?;
        if current_identity == possible_ancestor {
            return Ok(true);
        }
        let parent = current.open_parent_dir(cap_std::ambient_authority())?;
        let parent_identity = cap_directory_identity(&parent)?;
        if parent_identity == current_identity {
            return Ok(false);
        }
        current = parent;
    }
    Err(io::Error::new(
        io::ErrorKind::InvalidData,
        "directory ancestry exceeded its closed traversal bound",
    ))
}

fn pinned_directories_overlap(
    left: &HeldDirectoryIdentity,
    right: &HeldDirectoryIdentity,
) -> io::Result<bool> {
    Ok(
        pinned_directory_is_at_or_below(&left.directory, right.identity)?
            || pinned_directory_is_at_or_below(&right.directory, left.identity)?,
    )
}

#[cfg(unix)]
fn directory_identity(file: &File) -> io::Result<ObjectIdentity> {
    use std::os::unix::fs::MetadataExt as _;

    let metadata = file.metadata()?;
    if !metadata.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "handle is not one real directory",
        ));
    }
    Ok(ObjectIdentity {
        device: metadata.dev(),
        inode: metadata.ino(),
    })
}

fn revalidate_directory_identity(
    requested: &Path,
    canonical: &Path,
    held: &HeldDirectoryIdentity,
    kind: DirectoryKind,
) -> Result<(), Failure> {
    let after = canonical_existing_directory_no_reparse(requested, kind).map_err(|_| {
        directory_changed_failure(kind, "directory became unavailable during Voice preview")
    })?;
    let after_held = hold_directory_identity(&after).map_err(|_| {
        directory_changed_failure(kind, "directory identity became unavailable during preview")
    })?;
    if after != canonical || after_held.identity != held.identity {
        return Err(directory_changed_failure(
            kind,
            "directory changed identity during Voice preview",
        ));
    }
    Ok(())
}

fn revalidate_registered_preview_entry(entry: &PreviewCapabilityEntry) -> Result<(), Failure> {
    let metadata = entry
        .system_temp
        .directory
        .symlink_metadata(&entry.root_name)
        .map_err(|_| {
            directory_changed_failure(
                DirectoryKind::Preview,
                "preview_root became unavailable during Voice preview",
            )
        })?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(directory_changed_failure(
            DirectoryKind::Preview,
            "preview_root name became unsafe during Voice preview",
        ));
    }
    let actual = named_child_directory_identity(&entry.system_temp.directory, &entry.root_name)
        .map_err(|_| {
            directory_changed_failure(
                DirectoryKind::Preview,
                "preview_root identity became unavailable during Voice preview",
            )
        })?;
    if actual != entry.preview_root.identity {
        return Err(directory_changed_failure(
            DirectoryKind::Preview,
            "preview_root changed identity during Voice preview",
        ));
    }
    Ok(())
}

fn directory_changed_failure(kind: DirectoryKind, message: &'static str) -> Failure {
    match kind {
        DirectoryKind::Store => Failure::new(
            "AUTHORING_REVISION3_VOICE_PREVIEW_STORE_ROOT_CHANGED",
            message,
        ),
        DirectoryKind::Preview | DirectoryKind::SystemTemp => Failure::new(
            "AUTHORING_REVISION3_VOICE_PREVIEW_PREVIEW_CAPABILITY_CHANGED",
            message,
        ),
    }
}

fn paths_overlap(left: &Path, right: &Path) -> bool {
    left.starts_with(right) || right.starts_with(left)
}

fn ensure_initial_preview_root_empty(root: &cap_std::fs::Dir) -> Result<(), Failure> {
    match root.symlink_metadata(PREVIEW_FILE_NAME) {
        Ok(_) => {
            return Err(Failure::new(
                "AUTHORING_REVISION3_VOICE_PREVIEW_PREVIEW_OUTPUT_CONFLICT",
                "preview.ogg already exists; native preview never overwrites output",
            ));
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(_) => {
            return Err(preview_capability_invalid(
                "preview_root could not be inspected safely",
            ));
        }
    }
    let mut entries = root
        .entries()
        .map_err(|_| preview_capability_invalid("preview_root could not be inspected safely"))?;
    if entries
        .next()
        .transpose()
        .map_err(|_| preview_capability_invalid("preview_root could not be inspected safely"))?
        .is_some()
    {
        return Err(preview_capability_invalid(
            "preview_root must be empty before materialization",
        ));
    }
    Ok(())
}

fn ensure_final_preview_root_shape(root: &cap_std::fs::Dir) -> Result<(), Failure> {
    let entries = root
        .entries()
        .map_err(|_| directory_changed_failure(DirectoryKind::Preview, "preview_root changed"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| directory_changed_failure(DirectoryKind::Preview, "preview_root changed"))?;
    if entries.len() != 1 || entries[0].file_name() != PREVIEW_FILE_NAME {
        return Err(directory_changed_failure(
            DirectoryKind::Preview,
            "preview_root contents changed during Voice preview",
        ));
    }
    Ok(())
}

fn materialize_preview_file_owned(
    capability: &mut PreviewCapabilityEntry,
    bytes: &[u8],
    expected_sha256: Sha256Digest,
) -> Result<ObjectIdentity, Failure> {
    let directory = capability
        .preview_root
        .directory
        .try_clone()
        .map_err(|_| preview_registry_unavailable())?;
    let mut file = create_preview_file_no_follow(&directory).map_err(|error| {
        if error.kind() == io::ErrorKind::AlreadyExists {
            Failure::new(
                "AUTHORING_REVISION3_VOICE_PREVIEW_PREVIEW_OUTPUT_CONFLICT",
                "preview.ogg appeared before create-new materialization",
            )
        } else {
            Failure::new(
                "AUTHORING_REVISION3_VOICE_PREVIEW_PREVIEW_IO",
                "preview.ogg could not be created inside the retained capability",
            )
        }
    })?;
    let retained_handle = match file.try_clone() {
        Ok(retained) => retained,
        Err(_) => {
            capability.retained_leaf = Some(RetainedPreviewLeaf::ExactHandle(file));
            capability.state = PreviewCapabilityState::Materialized;
            return Err(Failure::new(
                "AUTHORING_REVISION3_VOICE_PREVIEW_PREVIEW_IO",
                "created preview.ogg handle could not be retained for cleanup",
            ));
        }
    };
    capability.retained_leaf = Some(RetainedPreviewLeaf::ExactHandle(retained_handle));
    capability.state = PreviewCapabilityState::Materialized;
    let created = snapshot_regular_file(&file).map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_VOICE_PREVIEW_PREVIEW_IO",
            "created preview.ogg handle identity could not be retained",
        )
    })?;
    validate_regular_preview_snapshot(created, 0)?;

    // Promote the exact handle to a stable inode identity before any write. Partial write/sync/hash
    // failures stay safely deletable without pretending they carry the final managed CAS seal.
    capability.retained_leaf = Some(RetainedPreviewLeaf::Identified {
        identity: created.identity,
        byte_len: bytes.len() as u64,
        sha256: expected_sha256,
        complete: false,
        sealed: None,
    });

    file.write_all(bytes)
        .and_then(|_| file.flush())
        .and_then(|_| file.sync_all())
        .map_err(|_| {
            Failure::new(
                "AUTHORING_REVISION3_VOICE_PREVIEW_PREVIEW_IO",
                "preview.ogg could not be written and synchronized",
            )
        })?;
    let written = snapshot_regular_file(&file).map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_VOICE_PREVIEW_PREVIEW_IO",
            "preview.ogg handle could not be verified",
        )
    })?;
    validate_regular_preview_snapshot(written, bytes.len() as u64)?;
    if written.identity != created.identity {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_PREVIEW_PREVIEW_IO",
            "preview.ogg changed identity while materializing",
        ));
    }
    file.seek(SeekFrom::Start(0)).map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_VOICE_PREVIEW_PREVIEW_IO",
            "preview.ogg could not be rewound for verification",
        )
    })?;
    if digest_reader(&mut file)? != expected_sha256 {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_PREVIEW_PREVIEW_IO",
            "preview.ogg differs from the verified managed VoiceTake",
        ));
    }
    // Close the DELETE-desired writer before playback. Release later reopens this exact fixed leaf
    // handle-relatively, proves the retained identity/seal on that handle, and deletes by handle.
    drop(file);
    verify_preview_leaf(
        &directory,
        created.identity,
        bytes.len() as u64,
        expected_sha256,
    )?;
    let mut sealed = open_preview_file_sealed(&directory).map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_VOICE_PREVIEW_PREVIEW_IO",
            "preview.ogg could not be retained as an immutable playback seal",
        )
    })?;
    if !cleanup_leaf_handle_matches(
        &mut sealed,
        created.identity,
        Some((bytes.len() as u64, expected_sha256)),
    ) {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_PREVIEW_PREVIEW_IO",
            "preview.ogg changed before its immutable playback seal was retained",
        ));
    }
    capability.retained_leaf = Some(RetainedPreviewLeaf::Identified {
        identity: created.identity,
        byte_len: bytes.len() as u64,
        sha256: expected_sha256,
        complete: true,
        sealed: Some(sealed),
    });
    Ok(created.identity)
}

#[cfg(windows)]
fn create_preview_file_no_follow(directory: &cap_std::fs::Dir) -> io::Result<File> {
    use cap_std::fs::OpenOptionsExt as _;
    use windows_sys::Win32::Storage::FileSystem::{
        DELETE, FILE_GENERIC_READ, FILE_GENERIC_WRITE, FILE_SHARE_READ,
    };

    let mut options = cap_std::fs::OpenOptions::new();
    options
        // cap-primitives validates the semantic read/write flags before applying access_mode.
        .read(true)
        .write(true)
        .access_mode(FILE_GENERIC_READ | FILE_GENERIC_WRITE | DELETE)
        .create_new(true)
        // CREATE_NEW atomically rejects every existing same-name object, including reparses.
        .share_mode(FILE_SHARE_READ);
    directory
        .open_with(PREVIEW_FILE_NAME, &options)
        .map(cap_std::fs::File::into_std)
}

#[cfg(unix)]
fn create_preview_file_no_follow(directory: &cap_std::fs::Dir) -> io::Result<File> {
    use cap_std::fs::OpenOptionsExt as _;

    let mut options = cap_std::fs::OpenOptions::new();
    options
        .read(true)
        .write(true)
        .create_new(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC);
    directory
        .open_with(PREVIEW_FILE_NAME, &options)
        .map(cap_std::fs::File::into_std)
}

#[cfg(windows)]
fn open_preview_file_no_follow(directory: &cap_std::fs::Dir) -> io::Result<File> {
    use cap_std::fs::OpenOptionsExt as _;
    use windows_sys::Win32::Storage::FileSystem::{
        FILE_FLAG_OPEN_REPARSE_POINT, FILE_SHARE_DELETE, FILE_SHARE_READ,
    };

    let mut options = cap_std::fs::OpenOptions::new();
    options
        .read(true)
        .share_mode(FILE_SHARE_READ | FILE_SHARE_DELETE)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT);
    directory
        .open_with(PREVIEW_FILE_NAME, &options)
        .map(cap_std::fs::File::into_std)
}

#[cfg(windows)]
fn open_preview_file_sealed(directory: &cap_std::fs::Dir) -> io::Result<File> {
    use cap_std::fs::OpenOptionsExt as _;
    use windows_sys::Win32::Storage::FileSystem::{
        FILE_FLAG_OPEN_REPARSE_POINT, FILE_GENERIC_READ, FILE_SHARE_READ,
    };

    let mut options = cap_std::fs::OpenOptions::new();
    options
        .access_mode(FILE_GENERIC_READ)
        // Playback opens read-only via _wfopen("rb"). Sharing READ permits that while denying
        // in-place writers and delete/rename replacement for the token's entire live lifetime.
        .share_mode(FILE_SHARE_READ)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT);
    directory
        .open_with(PREVIEW_FILE_NAME, &options)
        .map(cap_std::fs::File::into_std)
}

#[cfg(unix)]
fn open_preview_file_no_follow(directory: &cap_std::fs::Dir) -> io::Result<File> {
    use cap_std::fs::OpenOptionsExt as _;

    let mut options = cap_std::fs::OpenOptions::new();
    options
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC);
    directory
        .open_with(PREVIEW_FILE_NAME, &options)
        .map(cap_std::fs::File::into_std)
}

#[cfg(unix)]
fn open_preview_file_sealed(directory: &cap_std::fs::Dir) -> io::Result<File> {
    open_preview_file_no_follow(directory)
}

#[cfg(windows)]
fn snapshot_regular_file(file: &File) -> io::Result<RegularFileSnapshot> {
    use std::mem::MaybeUninit;
    use std::os::windows::io::AsRawHandle as _;
    use windows_sys::Win32::Foundation::HANDLE;
    use windows_sys::Win32::Storage::FileSystem::{
        GetFileInformationByHandle, GetFileType, BY_HANDLE_FILE_INFORMATION,
        FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_REPARSE_POINT, FILE_TYPE_DISK,
    };

    let mut info = MaybeUninit::<BY_HANDLE_FILE_INFORMATION>::uninit();
    // SAFETY: `file` owns a live handle and `info` is the exact writable ABI type.
    if unsafe { GetFileInformationByHandle(file.as_raw_handle() as HANDLE, info.as_mut_ptr()) } == 0
    {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: a successful call initializes the complete structure.
    let info = unsafe { info.assume_init() };
    // SAFETY: `file` owns a live handle.
    let disk = unsafe { GetFileType(file.as_raw_handle() as HANDLE) } == FILE_TYPE_DISK;
    Ok(RegularFileSnapshot {
        identity: ObjectIdentity {
            device: u64::from(info.dwVolumeSerialNumber),
            inode: (u64::from(info.nFileIndexHigh) << 32) | u64::from(info.nFileIndexLow),
        },
        byte_len: (u64::from(info.nFileSizeHigh) << 32) | u64::from(info.nFileSizeLow),
        link_count: u64::from(info.nNumberOfLinks),
        is_regular: disk && info.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY == 0,
        is_reparse: info.dwFileAttributes & FILE_ATTRIBUTE_REPARSE_POINT != 0,
    })
}

#[cfg(unix)]
fn snapshot_regular_file(file: &File) -> io::Result<RegularFileSnapshot> {
    use std::os::unix::fs::MetadataExt as _;

    let metadata = file.metadata()?;
    Ok(RegularFileSnapshot {
        identity: ObjectIdentity {
            device: metadata.dev(),
            inode: metadata.ino(),
        },
        byte_len: metadata.len(),
        link_count: metadata.nlink(),
        is_regular: metadata.file_type().is_file(),
        is_reparse: false,
    })
}

fn validate_regular_preview_snapshot(
    snapshot: RegularFileSnapshot,
    expected_len: u64,
) -> Result<(), Failure> {
    if !snapshot.is_regular
        || snapshot.is_reparse
        || snapshot.link_count != 1
        || snapshot.byte_len != expected_len
    {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_PREVIEW_PREVIEW_IO",
            "preview.ogg is not one exact single-link regular file",
        ));
    }
    Ok(())
}

fn verify_preview_leaf(
    directory: &cap_std::fs::Dir,
    expected_identity: ObjectIdentity,
    expected_len: u64,
    expected_sha256: Sha256Digest,
) -> Result<(), Failure> {
    let mut file = open_preview_file_no_follow(directory).map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_VOICE_PREVIEW_PREVIEW_CAPABILITY_CHANGED",
            "preview.ogg became unavailable or unsafe during materialization",
        )
    })?;
    let snapshot = snapshot_regular_file(&file).map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_VOICE_PREVIEW_PREVIEW_CAPABILITY_CHANGED",
            "preview.ogg identity became unavailable during materialization",
        )
    })?;
    validate_regular_preview_snapshot(snapshot, expected_len)?;
    if snapshot.identity != expected_identity || digest_reader(&mut file)? != expected_sha256 {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_PREVIEW_PREVIEW_CAPABILITY_CHANGED",
            "preview.ogg changed identity or content during materialization",
        ));
    }
    Ok(())
}

fn digest_reader(reader: &mut File) -> Result<Sha256Digest, Failure> {
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = reader.read(&mut buffer).map_err(|_| {
            Failure::new(
                "AUTHORING_REVISION3_VOICE_PREVIEW_PREVIEW_IO",
                "preview.ogg could not be verified completely",
            )
        })?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(Sha256Digest::from_bytes(hasher.finalize().into()))
}

fn require_signed_request(request: &Revision3VoiceTakePreviewRequestV1) -> Result<(), Failure> {
    for value in [
        request.expected_head.snapshot.byte_len,
        request.expected_revision,
        request.expected_line_revision,
        request.expected_localization_revision,
        request.expected_slot_revision,
        request.expected_take_revision,
        request.expected_asset.byte_len,
    ] {
        if value > i64::MAX as u64 {
            return Err(Failure::new(
                "AUTHORING_REVISION3_VOICE_PREVIEW_SIGNED_WIRE_LIMIT",
                "Voice preview request contains an integer outside the signed wire range",
            ));
        }
    }
    Ok(())
}

fn require_signed_basis(
    head: &WorkingHead,
    project: &gore_authoring::ProjectRevision3,
    request: &Revision3VoiceTakePreviewRequestV1,
) -> Result<(), Failure> {
    require_signed_request(request)?;
    for value in [head.snapshot.byte_len, project.revision] {
        if value > i64::MAX as u64 {
            return Err(Failure::new(
                "AUTHORING_REVISION3_VOICE_PREVIEW_SIGNED_WIRE_LIMIT",
                "exact Voice preview basis contains an integer outside the signed wire range",
            ));
        }
    }
    Ok(())
}

fn canonical_head_json(head: &WorkingHead) -> Result<String, Failure> {
    serde_json::to_string(head).map_err(|_| invariant())
}

fn enforce_response_budget(response: Value) -> Result<Value, Failure> {
    let bytes = serde_json::to_vec(&response).map_err(|_| invariant())?;
    if bytes.len() > MAX_RESPONSE_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_PREVIEW_RESPONSE_LIMIT",
            "revision-3 Voice preview response exceeds its bounded transport budget",
        ));
    }
    Ok(response)
}

fn ffi_store_limits() -> WorkingStoreLimits {
    WorkingStoreLimits {
        max_referenced_entity_bytes: MAX_PROJECT_JSON_BYTES as u64,
        ..WorkingStoreLimits::default()
    }
}

fn map_request_error(error: Revision3VoiceTakePreviewRequestJsonErrorV1) -> Failure {
    match error {
        Revision3VoiceTakePreviewRequestJsonErrorV1::InputTooLarge { .. } => Failure::new(
            "AUTHORING_REVISION3_VOICE_PREVIEW_INPUT_LIMIT",
            "voice_take_preview_request_json exceeds its bounded transport limit",
        ),
        _ => invalid_request(),
    }
}

fn map_binding_conflict(error: Revision3VoiceTakePreviewConflictV1) -> Failure {
    use Revision3VoiceTakePreviewConflictV1::*;
    let (code, message) = match error {
        CurrentHeadMismatch => (
            "AUTHORING_REVISION3_VOICE_PREVIEW_HEAD_CONFLICT",
            "published revision-3 head differs from the Voice preview request",
        ),
        ProjectIdentityMismatch { .. } | ProjectRevisionConflict { .. } => (
            "AUTHORING_REVISION3_VOICE_PREVIEW_PROJECT_CONFLICT",
            "exact project identity or revision differs from the Voice preview request",
        ),
        InvalidEntityIdentity => (
            "AUTHORING_REVISION3_VOICE_PREVIEW_INPUT_INVALID",
            "Voice preview request contains invalid entity identities",
        ),
        InvalidDialogLine { .. } | DialogLineRevisionConflict { .. } => (
            "AUTHORING_REVISION3_VOICE_PREVIEW_LINE_CONFLICT",
            "exact DialogLine differs from the Voice preview request",
        ),
        LocalizationReferenceMismatch { .. }
        | InvalidLocalization { .. }
        | LocalizationRevisionConflict { .. }
        | LocalizationIdentityMismatch => (
            "AUTHORING_REVISION3_VOICE_PREVIEW_LOCALIZATION_CONFLICT",
            "exact LocalizationEntry differs from the Voice preview request",
        ),
        VoiceSlotReferenceMismatch { .. }
        | InvalidVoiceSlot { .. }
        | VoiceSlotRevisionConflict { .. }
        | VoiceSlotLocaleMismatch { .. } => (
            "AUTHORING_REVISION3_VOICE_PREVIEW_SLOT_CONFLICT",
            "exact VoiceSlot differs from the Voice preview request",
        ),
        VoiceTakeNotCandidate { .. }
        | InvalidVoiceTake { .. }
        | VoiceTakeRevisionConflict { .. }
        | VoiceTakeLocaleMismatch { .. } => (
            "AUTHORING_REVISION3_VOICE_PREVIEW_TAKE_CONFLICT",
            "exact VoiceTake differs from the Voice preview request",
        ),
        VoiceTakeAssetMismatch { .. }
        | MissingVoiceAsset { .. }
        | VoiceAssetMetadataMismatch { .. } => (
            "AUTHORING_REVISION3_VOICE_PREVIEW_ASSET_CONFLICT",
            "exact VoiceTake asset differs from the Voice preview request",
        ),
    };
    Failure::new(code, message)
}

fn map_store_open_error(error: WorkingStoreError) -> Failure {
    use WorkingStoreError::*;
    let (code, message) = match error {
        HeadConflict { .. } => (
            "AUTHORING_REVISION3_VOICE_PREVIEW_HEAD_CONFLICT",
            "published revision-3 head changed during Voice preview",
        ),
        MissingHead(_) | MissingRoot(_) | MissingObject(_) => (
            "AUTHORING_REVISION3_VOICE_PREVIEW_STORE_UNAVAILABLE",
            "managed Store is unavailable or incomplete",
        ),
        UnsafePath { .. } => (
            "AUTHORING_REVISION3_VOICE_PREVIEW_STORE_UNSAFE",
            "managed Store contains an unsafe path or object",
        ),
        LimitExceeded { .. } | InvalidLimits(_) => (
            "AUTHORING_REVISION3_VOICE_PREVIEW_STORE_LIMIT",
            "managed Store exceeds a bounded Voice preview limit",
        ),
        _ => (
            "AUTHORING_REVISION3_VOICE_PREVIEW_STORE_INVARIANT",
            "managed Store could not be reopened exactly for Voice preview",
        ),
    };
    Failure::new(code, message)
}

fn map_selected_asset_error(error: WorkingStoreError, store_root: &Path) -> Failure {
    match error {
        WorkingStoreError::MissingRoot(_) => Failure::new(
            "AUTHORING_REVISION3_VOICE_PREVIEW_STORE_ROOT_CHANGED",
            "managed Store root changed while reading the selected VoiceTake",
        ),
        WorkingStoreError::UnsafePath { path, .. }
            if path == store_root || store_root.starts_with(&path) =>
        {
            Failure::new(
                "AUTHORING_REVISION3_VOICE_PREVIEW_STORE_ROOT_CHANGED",
                "managed Store root changed while reading the selected VoiceTake",
            )
        }
        _ => asset_invalid("selected VoiceTake asset is missing, unsafe, corrupt, or over limit"),
    }
}

fn invalid_request() -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_VOICE_PREVIEW_INPUT_INVALID",
        "request must be exact canonical JSON matching the selected Voice preview command payload",
    )
}

fn preview_capability_invalid(message: &'static str) -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_VOICE_PREVIEW_PREVIEW_CAPABILITY_INVALID",
        message,
    )
}

fn asset_invalid(message: &'static str) -> Failure {
    Failure::new("AUTHORING_REVISION3_VOICE_PREVIEW_ASSET_INVALID", message)
}

fn head_conflict() -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_VOICE_PREVIEW_HEAD_CONFLICT",
        "published revision-3 project changed during Voice preview materialization",
    )
}

fn invariant() -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_VOICE_PREVIEW_INVARIANT",
        "revision-3 Voice preview could not preserve its exact internal contract",
    )
}

fn truncate_utf8(mut value: String, max: usize) -> String {
    if value.len() <= max {
        return value;
    }
    let mut end = max;
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    value.truncate(end);
    value
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use gore_authoring::model_revision3::{
        DialogLine, Entity, EntityKind, EntityPayload, LocalizationEntry,
        OggCodec as Revision3OggCodec, OggMetadata as Revision3OggMetadata, OriginRef,
        SchemaRevisionV3, TypedRef, VoiceSlot, VoiceTake, VoiceTakeStatus, VoiceTargetResolution,
    };
    use gore_authoring::{
        AssetMeta, AssetStoreIndex, ContentSeal, EntityId, FormatV2, GameGenerationAnchor,
        LocaleCode, ProjectId, ProjectMeta, ProjectRevision3,
    };
    use tempfile::TempDir;

    use super::*;

    const LOC_ID: &str = "GRD_263_ASGHAN_OPEN_INFO_06_02";

    struct PublishedStore {
        temp: TempDir,
        project: ProjectRevision3,
        head: WorkingHead,
        previous_head_bytes: Vec<u8>,
        asset_path: PathBuf,
        asset_bytes: Vec<u8>,
        unrelated_asset_path: PathBuf,
        unrelated_asset_bytes: Vec<u8>,
    }

    fn id(tag: u8) -> EntityId {
        EntityId::from_bytes([tag; 16])
    }

    fn project_id() -> ProjectId {
        ProjectId::from_bytes([0x31; 16])
    }

    fn locale() -> LocaleCode {
        "de".parse().unwrap()
    }

    fn target() -> GameGenerationAnchor {
        GameGenerationAnchor {
            executable: ContentSeal {
                byte_len: 171_698_176,
                sha256: Sha256Digest::from_bytes([0x41; 32]),
            },
        }
    }

    fn origin(tag: u8) -> OriginRef {
        OriginRef::Imported {
            importer: "voice-preview-ffi-tests".to_owned(),
            source_seal: ContentSeal {
                byte_len: 10,
                sha256: Sha256Digest::from_bytes([tag; 32]),
            },
            external_identity: None,
        }
    }

    fn entity(tag: u8, revision: u64, payload: EntityPayload) -> Entity {
        Entity {
            id: id(tag),
            display_name: format!("entity-{tag}"),
            origin: origin(tag),
            revision,
            payload,
        }
    }

    fn empty_project(revision: u64) -> ProjectRevision3 {
        ProjectRevision3 {
            format: FormatV2,
            schema_revision: SchemaRevisionV3,
            project_id: project_id(),
            revision,
            meta: ProjectMeta {
                name: "Voice preview FFI fixture".to_owned(),
                version: "1.0.0".to_owned(),
                author: "tests".to_owned(),
            },
            target: target(),
            authoring_locales: BTreeSet::from([locale()]),
            entities: BTreeMap::new(),
            asset_store: AssetStoreIndex::default(),
        }
    }

    fn voice_project(
        imported: &gore_authoring::ImportedOgg,
        unrelated: &gore_authoring::ImportedOgg,
    ) -> ProjectRevision3 {
        let localization_id = id(1);
        let line_id = id(2);
        let slot_id = id(3);
        let take_id = id(4);
        let mut asset = imported.asset.clone();
        asset.logical_name = "asghan_take.ogg".to_owned();
        let mut project = empty_project(1);
        project.asset_store.assets.insert(
            asset.sha256,
            AssetMeta {
                byte_len: asset.byte_len,
                media_type: "audio/ogg".to_owned(),
            },
        );
        project.asset_store.assets.insert(
            unrelated.asset.sha256,
            AssetMeta {
                byte_len: unrelated.asset.byte_len,
                media_type: "audio/ogg".to_owned(),
            },
        );
        project.entities = BTreeMap::from([
            (
                localization_id,
                entity(
                    1,
                    5,
                    EntityPayload::LocalizationEntry(LocalizationEntry {
                        loc_id: LOC_ID.to_owned(),
                        texts: BTreeMap::new(),
                    }),
                ),
            ),
            (
                line_id,
                entity(
                    2,
                    6,
                    EntityPayload::DialogLine(DialogLine {
                        localization: TypedRef::new(
                            project_id(),
                            localization_id,
                            EntityKind::LocalizationEntry,
                        ),
                        speaker_hint: Some("Asghan".to_owned()),
                        voice_slots: BTreeMap::from([(
                            locale(),
                            TypedRef::new(project_id(), slot_id, EntityKind::VoiceSlot),
                        )]),
                    }),
                ),
            ),
            (
                slot_id,
                entity(
                    3,
                    7,
                    EntityPayload::VoiceSlot(VoiceSlot {
                        locale: locale(),
                        target_resolution: VoiceTargetResolution::Unresolved,
                        candidates: vec![TypedRef::new(
                            project_id(),
                            take_id,
                            EntityKind::VoiceTake,
                        )],
                        selected: None,
                    }),
                ),
            ),
            (
                take_id,
                entity(
                    4,
                    8,
                    EntityPayload::VoiceTake(VoiceTake {
                        locale: locale(),
                        asset,
                        ogg: Revision3OggMetadata {
                            codec: match imported.ogg.codec {
                                gore_authoring::OggCodec::Vorbis => Revision3OggCodec::Vorbis,
                                gore_authoring::OggCodec::Opus => Revision3OggCodec::Opus,
                            },
                            channels: imported.ogg.channels,
                            sample_rate: imported.ogg.sample_rate,
                            pages: imported.ogg.pages,
                            logical_streams: imported.ogg.logical_streams,
                        },
                        status: VoiceTakeStatus::Recorded,
                    }),
                ),
            ),
        ]);
        project
    }

    fn asset_path(root: &Path, digest: Sha256Digest) -> PathBuf {
        let hex = digest.to_string();
        root.join("assets")
            .join("sha256")
            .join(&hex[..2])
            .join(&hex[2..])
    }

    fn published_store() -> PublishedStore {
        let temp = TempDir::new().unwrap();
        let store = WorkingProjectStore::at(temp.path(), ffi_store_limits()).unwrap();
        let empty = empty_project(0);
        let previous = store.prepare_revision3_checkpoint(None, &empty).unwrap();
        fs::write(temp.path().join("gore-project.json"), &previous.head_bytes).unwrap();
        let prepared = store
            .prepare_ogg_bytes_classified(
                include_bytes!("../../gore-vo/testdata/tiny-vorbis.ogg").to_vec(),
                "asghan_take.ogg",
            )
            .unwrap();
        let imported = store
            .install_prepared_ogg(prepared, Some(&previous.head))
            .unwrap();
        let selected_asset_path = asset_path(temp.path(), imported.asset.sha256);
        let asset_bytes = fs::read(&selected_asset_path).unwrap();
        let unrelated_prepared = store
            .prepare_ogg_bytes_classified(
                include_bytes!("../../gore-vo/testdata/tiny-opus.ogg").to_vec(),
                "unrelated.ogg",
            )
            .unwrap();
        let unrelated = store
            .install_prepared_ogg(unrelated_prepared, Some(&previous.head))
            .unwrap();
        let unrelated_asset_path = asset_path(temp.path(), unrelated.asset.sha256);
        let unrelated_asset_bytes = fs::read(&unrelated_asset_path).unwrap();
        let project = voice_project(&imported, &unrelated);
        let published = store
            .prepare_revision3_checkpoint(Some(&previous.head), &project)
            .unwrap();
        fs::write(temp.path().join("gore-project.json"), &published.head_bytes).unwrap();
        PublishedStore {
            temp,
            project,
            head: published.head,
            previous_head_bytes: previous.head_bytes,
            asset_path: selected_asset_path,
            asset_bytes,
            unrelated_asset_path,
            unrelated_asset_bytes,
        }
    }

    fn request(store: &PublishedStore) -> Revision3VoiceTakePreviewRequestV1 {
        let EntityPayload::VoiceTake(take) = &store.project.entities[&id(4)].payload else {
            unreachable!()
        };
        Revision3VoiceTakePreviewRequestV1 {
            expected_head: store.head.clone(),
            expected_project_id: store.project.project_id,
            expected_revision: store.project.revision,
            line_id: id(2),
            expected_line_revision: 6,
            localization_id: id(1),
            expected_localization_revision: 5,
            expected_loc_id: LOC_ID.to_owned(),
            slot_id: id(3),
            expected_slot_revision: 7,
            locale: locale(),
            take_id: id(4),
            expected_take_revision: 8,
            expected_asset: take.asset.clone(),
        }
    }

    fn test_serial_guard() -> std::sync::MutexGuard<'static, ()> {
        static SERIAL: OnceLock<Mutex<()>> = OnceLock::new();
        SERIAL
            .get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn register_wire(root: &Path) -> String {
        serde_json::to_string(&ExactWireRequest {
            command: REGISTER_COMMAND.to_owned(),
            payload: RegisterVoicePreviewWirePayload {
                root: root.to_string_lossy().into_owned(),
            },
        })
        .unwrap()
    }

    fn release_raw(cleanup_token: &str) -> Value {
        release_revision3_voice_take_preview_v1_raw(
            &serde_json::to_string(&ExactWireRequest {
                command: RELEASE_COMMAND.to_owned(),
                payload: ReleaseVoicePreviewWirePayload {
                    cleanup_token: cleanup_token.to_owned(),
                },
            })
            .unwrap(),
        )
    }

    struct RegisteredPreview {
        cleanup_token: String,
        root: PathBuf,
        released: bool,
    }

    impl RegisteredPreview {
        fn new(store_root: &Path) -> Self {
            let response = register_revision3_voice_take_preview_v1_raw(&register_wire(store_root));
            assert_eq!(
                response["ok"], true,
                "test preview registration failed: {response}"
            );
            assert_eq!(response["outcome"], "preview_capability_registered");
            assert_eq!(
                response["preview_authority"],
                "native_owned_ephemeral_temp_capability_v1"
            );
            let cleanup_token = response["cleanup_token"].as_str().unwrap().to_owned();
            assert_eq!(cleanup_token.len(), CLEANUP_TOKEN_HEX_BYTES);
            let root = PathBuf::from(response["preview_root"].as_str().unwrap());
            let preview_path = PathBuf::from(response["preview_path"].as_str().unwrap());
            assert_eq!(preview_path, root.join(PREVIEW_FILE_NAME));
            assert_eq!(response["preview_leaf"], PREVIEW_FILE_NAME);
            assert!(root.is_absolute());
            assert!(root
                .file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with(PREVIEW_ROOT_PREFIX));
            assert_eq!(
                fs::canonicalize(root.parent().unwrap()).unwrap(),
                fs::canonicalize(std::env::temp_dir()).unwrap()
            );
            assert!(fs::read_dir(&root).unwrap().next().is_none());
            Self {
                cleanup_token,
                root,
                released: false,
            }
        }

        fn wire(&self, store_root: &Path, request: &Revision3VoiceTakePreviewRequestV1) -> String {
            serde_json::to_string(&ExactWireRequest {
                command: COMMAND.to_owned(),
                payload: MaterializeVoicePreviewWirePayload {
                    root: store_root.to_string_lossy().into_owned(),
                    cleanup_token: self.cleanup_token.clone(),
                    voice_take_preview_request_json: request.to_canonical_json().unwrap(),
                },
            })
            .unwrap()
        }

        fn release(&mut self) -> Value {
            let response = release_raw(&self.cleanup_token);
            if response["ok"] == true {
                self.released = true;
            }
            response
        }
    }

    impl Drop for RegisteredPreview {
        fn drop(&mut self) {
            if !self.released {
                let response = release_raw(&self.cleanup_token);
                self.released = response["ok"] == true;
            }
        }
    }

    fn snapshot_files(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
        fn visit(root: &Path, current: &Path, output: &mut BTreeMap<PathBuf, Vec<u8>>) {
            for entry in fs::read_dir(current).unwrap() {
                let entry = entry.unwrap();
                let path = entry.path();
                if entry.file_type().unwrap().is_dir() {
                    visit(root, &path, output);
                } else {
                    output.insert(
                        path.strip_prefix(root).unwrap().to_path_buf(),
                        fs::read(path).unwrap(),
                    );
                }
            }
        }
        let mut output = BTreeMap::new();
        visit(root, root, &mut output);
        output
    }

    #[cfg(windows)]
    #[test]
    fn exact_current_take_materializes_fixed_verified_copy_without_store_writes() {
        let _serial = test_serial_guard();
        let store = published_store();
        let before = snapshot_files(store.temp.path());
        let mut preview = RegisteredPreview::new(store.temp.path());
        let response: Value = serde_json::from_str(&crate::execute_json(
            &preview.wire(store.temp.path(), &request(&store)),
        ))
        .unwrap();

        assert_eq!(response["ok"], true, "materialize failed: {response}");
        assert_eq!(response["outcome"], "preview_ready");
        assert_eq!(response["preview_leaf"], PREVIEW_FILE_NAME);
        assert_eq!(response["status"], "recorded");
        assert_eq!(
            response["asset"]["sha256"],
            request(&store).expected_asset.sha256.to_string()
        );
        assert_eq!(response["project_write_status"], "not_performed");
        let expected_keys = BTreeSet::from([
            "ok",
            "outcome",
            "basis_head_json",
            "project_id",
            "project_revision",
            "line_id",
            "line_revision",
            "localization_id",
            "localization_revision",
            "loc_id",
            "slot_id",
            "slot_revision",
            "locale",
            "take_id",
            "take_revision",
            "asset",
            "status",
            "ogg",
            "preview_path",
            "preview_leaf",
            "preview_authority",
            "cleanup_token",
            "preview_lifecycle",
            "project_write_status",
            "game_write_status",
            "save_write_status",
            "build_status",
            "deployment_status",
            "runtime_status",
        ]);
        let actual_keys = response
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        assert_eq!(actual_keys, expected_keys);
        assert_eq!(
            fs::read(preview.root.join(PREVIEW_FILE_NAME)).unwrap(),
            store.asset_bytes
        );
        assert_eq!(snapshot_files(store.temp.path()), before);
        assert!(!preview.root.starts_with(store.temp.path()));
        assert_eq!(preview.release()["ok"], true);
        assert!(!preview.root.exists());
    }

    #[cfg(windows)]
    #[test]
    fn scoped_preview_verifies_selected_asset_but_not_unrelated_blob() {
        let _serial = test_serial_guard();
        let store = published_store();
        let mut corrupt = store.unrelated_asset_bytes.clone();
        corrupt[0] ^= 0x01;
        fs::write(&store.unrelated_asset_path, corrupt).unwrap();
        let mut preview = RegisteredPreview::new(store.temp.path());

        let response = materialize_revision3_voice_take_preview_v1_raw(
            &preview.wire(store.temp.path(), &request(&store)),
        );
        assert_eq!(response["ok"], true);
        assert_eq!(
            fs::read(preview.root.join(PREVIEW_FILE_NAME)).unwrap(),
            store.asset_bytes
        );
        let exact_store =
            WorkingProjectStore::open_existing(store.temp.path(), ffi_store_limits()).unwrap();
        assert!(exact_store
            .open_current_revision3(AssetVerification::Full)
            .is_err());
        assert_eq!(preview.release()["ok"], true);
    }

    #[cfg(windows)]
    #[test]
    fn wires_are_closed_bounded_and_every_graph_layer_is_stale_bound() {
        let _serial = test_serial_guard();
        let store = published_store();

        let canonical_register = register_wire(store.temp.path());
        assert_eq!(
            register_revision3_voice_take_preview_v1_raw(&format!(" {canonical_register}"))
                ["error"]["code"],
            "AUTHORING_REVISION3_VOICE_PREVIEW_INPUT_INVALID"
        );
        let unknown_register =
            canonical_register.replacen("\"root\":", "\"unknown\":true,\"root\":", 1);
        assert_eq!(
            register_revision3_voice_take_preview_v1_raw(&unknown_register)["error"]["code"],
            "AUTHORING_REVISION3_VOICE_PREVIEW_INPUT_INVALID"
        );
        assert_eq!(
            register_revision3_voice_take_preview_v1_raw(&register_wire(Path::new("relative")))
                ["error"]["code"],
            "AUTHORING_REVISION3_VOICE_PREVIEW_STORE_UNSAFE"
        );

        let mut preview = RegisteredPreview::new(store.temp.path());
        let canonical = preview.wire(store.temp.path(), &request(&store));
        assert_eq!(
            materialize_revision3_voice_take_preview_v1_raw(&format!(" {canonical}"))["error"]
                ["code"],
            "AUTHORING_REVISION3_VOICE_PREVIEW_INPUT_INVALID"
        );
        let unknown = canonical.replacen("\"root\":", "\"unknown\":true,\"root\":", 1);
        assert_eq!(
            materialize_revision3_voice_take_preview_v1_raw(&unknown)["error"]["code"],
            "AUTHORING_REVISION3_VOICE_PREVIEW_INPUT_INVALID"
        );
        assert_eq!(
            materialize_revision3_voice_take_preview_v1_raw(&" ".repeat(MAX_WIRE_BYTES + 1))
                ["error"]["code"],
            "AUTHORING_REVISION3_VOICE_PREVIEW_INPUT_LIMIT"
        );
        assert_eq!(preview.release()["ok"], true);

        let cases = [
            ("head", "AUTHORING_REVISION3_VOICE_PREVIEW_HEAD_CONFLICT"),
            (
                "project",
                "AUTHORING_REVISION3_VOICE_PREVIEW_PROJECT_CONFLICT",
            ),
            ("line", "AUTHORING_REVISION3_VOICE_PREVIEW_LINE_CONFLICT"),
            (
                "localization",
                "AUTHORING_REVISION3_VOICE_PREVIEW_LOCALIZATION_CONFLICT",
            ),
            ("slot", "AUTHORING_REVISION3_VOICE_PREVIEW_SLOT_CONFLICT"),
            ("take", "AUTHORING_REVISION3_VOICE_PREVIEW_TAKE_CONFLICT"),
            ("asset", "AUTHORING_REVISION3_VOICE_PREVIEW_ASSET_CONFLICT"),
        ];
        for (kind, code) in cases {
            let mut stale = request(&store);
            match kind {
                "head" => stale.expected_head.snapshot.byte_len += 1,
                "project" => stale.expected_revision -= 1,
                "line" => stale.expected_line_revision -= 1,
                "localization" => stale.expected_localization_revision -= 1,
                "slot" => stale.expected_slot_revision -= 1,
                "take" => stale.expected_take_revision -= 1,
                "asset" => stale.expected_asset.logical_name = "different.ogg".to_owned(),
                _ => unreachable!(),
            }
            let mut preview = RegisteredPreview::new(store.temp.path());
            let response = materialize_revision3_voice_take_preview_v1_raw(
                &preview.wire(store.temp.path(), &stale),
            );
            assert_eq!(response["error"]["code"], code, "case {kind}");
            assert!(fs::read_dir(&preview.root).unwrap().next().is_none());
            assert_eq!(preview.release()["ok"], true);
        }
    }

    #[cfg(windows)]
    #[test]
    fn native_capability_is_no_clobber_root_locked_and_store_bound() {
        let _serial = test_serial_guard();
        let store = published_store();
        let other_store = published_store();
        let mut preview = RegisteredPreview::new(store.temp.path());

        let moved = preview.root.with_extension("moved");
        assert!(fs::rename(&preview.root, &moved).is_err());
        assert!(fs::remove_dir(&preview.root).is_err());
        let other_temp_child = TempDir::new().unwrap();
        assert!(other_temp_child.path().exists());
        drop(other_temp_child);

        let wrong_store = materialize_revision3_voice_take_preview_v1_raw(
            &preview.wire(other_store.temp.path(), &request(&store)),
        );
        assert_eq!(
            wrong_store["error"]["code"],
            "AUTHORING_REVISION3_VOICE_PREVIEW_STORE_ROOT_CHANGED"
        );

        fs::write(preview.root.join(PREVIEW_FILE_NAME), b"do not overwrite").unwrap();
        let conflict = materialize_revision3_voice_take_preview_v1_raw(
            &preview.wire(store.temp.path(), &request(&store)),
        );
        assert_eq!(
            conflict["error"]["code"],
            "AUTHORING_REVISION3_VOICE_PREVIEW_PREVIEW_OUTPUT_CONFLICT"
        );
        let retained = preview.release();
        assert_eq!(
            retained["error"]["code"],
            "AUTHORING_REVISION3_VOICE_PREVIEW_CLEANUP_RETAINED"
        );
        assert_eq!(
            fs::read(preview.root.join(PREVIEW_FILE_NAME)).unwrap(),
            b"do not overwrite"
        );
        fs::remove_file(preview.root.join(PREVIEW_FILE_NAME)).unwrap();
        assert_eq!(preview.release()["ok"], true);
    }

    #[cfg(windows)]
    #[test]
    fn playback_seal_blocks_mutation_and_cleanup_is_retryable() {
        use std::os::windows::fs::OpenOptionsExt as _;
        use windows_sys::Win32::Storage::FileSystem::{FILE_SHARE_READ, FILE_SHARE_WRITE};

        let _serial = test_serial_guard();
        let store = published_store();
        let mut preview = RegisteredPreview::new(store.temp.path());
        let response = materialize_revision3_voice_take_preview_v1_raw(
            &preview.wire(store.temp.path(), &request(&store)),
        );
        assert_eq!(response["ok"], true);
        let path = preview.root.join(PREVIEW_FILE_NAME);

        let mut playback_options = OpenOptions::new();
        playback_options
            .read(true)
            .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE);
        let playback = playback_options.open(&path).unwrap();
        assert!(OpenOptions::new().write(true).open(&path).is_err());
        assert!(fs::remove_file(&path).is_err());
        assert!(fs::rename(&path, preview.root.join("replacement.ogg")).is_err());

        let retained = preview.release();
        assert_eq!(
            retained["error"]["code"],
            "AUTHORING_REVISION3_VOICE_PREVIEW_CLEANUP_RETAINED"
        );
        assert!(path.exists());
        drop(playback);
        assert_eq!(preview.release()["ok"], true);
        assert!(!preview.root.exists());
    }

    #[cfg(windows)]
    #[test]
    fn post_materialize_races_retain_exact_cleanup_without_touching_foreign_entries() {
        let _serial = test_serial_guard();

        let store = published_store();
        let mut preview = RegisteredPreview::new(store.temp.path());
        let response = materialize_revision3_voice_take_preview_v1_inner_with_guard(
            &preview.wire(store.temp.path(), &request(&store)),
            |store_root, _| {
                fs::write(
                    store_root.join("gore-project.json"),
                    &store.previous_head_bytes,
                )
                .unwrap();
            },
        )
        .unwrap_err()
        .response();
        assert_eq!(
            response["error"]["code"],
            "AUTHORING_REVISION3_VOICE_PREVIEW_HEAD_CONFLICT"
        );
        assert!(preview.root.join(PREVIEW_FILE_NAME).exists());
        assert_eq!(preview.release()["ok"], true);

        let store = published_store();
        let mut preview = RegisteredPreview::new(store.temp.path());
        let response = materialize_revision3_voice_take_preview_v1_inner_with_guard(
            &preview.wire(store.temp.path(), &request(&store)),
            |_, preview_root| fs::write(preview_root.join("foreign.txt"), b"keep").unwrap(),
        )
        .unwrap_err()
        .response();
        assert_eq!(
            response["error"]["code"],
            "AUTHORING_REVISION3_VOICE_PREVIEW_PREVIEW_CAPABILITY_CHANGED"
        );
        let retained = preview.release();
        assert_eq!(
            retained["error"]["code"],
            "AUTHORING_REVISION3_VOICE_PREVIEW_CLEANUP_RETAINED"
        );
        assert_eq!(fs::read(preview.root.join("foreign.txt")).unwrap(), b"keep");
        assert!(preview.root.join(PREVIEW_FILE_NAME).exists());
        fs::remove_file(preview.root.join("foreign.txt")).unwrap();
        assert_eq!(preview.release()["ok"], true);
    }

    #[cfg(windows)]
    #[test]
    fn registry_capacity_unknown_tokens_and_concurrent_terminal_operations_are_bounded() {
        let _serial = test_serial_guard();
        let store = published_store();

        let unknown = "11".repeat(CLEANUP_TOKEN_BYTES);
        assert_eq!(
            release_raw(&unknown)["error"]["code"],
            "AUTHORING_REVISION3_VOICE_PREVIEW_CLEANUP_TOKEN_UNKNOWN"
        );
        assert_eq!(
            release_raw(&"A".repeat(CLEANUP_TOKEN_HEX_BYTES))["error"]["code"],
            "AUTHORING_REVISION3_VOICE_PREVIEW_INPUT_INVALID"
        );

        let mut capabilities = (0..MAX_ACTIVE_PREVIEW_CAPABILITIES)
            .map(|_| RegisteredPreview::new(store.temp.path()))
            .collect::<Vec<_>>();
        let over_limit =
            register_revision3_voice_take_preview_v1_raw(&register_wire(store.temp.path()));
        assert_eq!(
            over_limit["error"]["code"],
            "AUTHORING_REVISION3_VOICE_PREVIEW_PREVIEW_CAPABILITY_LIMIT"
        );
        for capability in &mut capabilities {
            assert_eq!(capability.release()["ok"], true);
            assert!(!capability.root.exists());
        }

        let mut concurrent = RegisteredPreview::new(store.temp.path());
        let token_a = concurrent.cleanup_token.clone();
        let token_b = token_a.clone();
        concurrent.released = true;
        let first = std::thread::spawn(move || release_raw(&token_a));
        let second = std::thread::spawn(move || release_raw(&token_b));
        assert_eq!(first.join().unwrap()["ok"], true);
        assert_eq!(second.join().unwrap()["ok"], true);
        assert!(!concurrent.root.exists());
        assert_eq!(release_raw(&concurrent.cleanup_token)["ok"], true);
    }

    #[cfg(windows)]
    #[test]
    fn system_temp_store_overlap_guard_is_asymmetric_and_non_mutating() {
        let _serial = test_serial_guard();
        let outer = TempDir::new().unwrap();
        let store_path = outer.path().join("store");
        let nested_temp_path = store_path.join("temp");
        fs::create_dir(&store_path).unwrap();
        fs::create_dir(&nested_temp_path).unwrap();
        let held_outer = hold_shared_directory_identity(outer.path()).unwrap();
        let held_store = hold_shared_directory_identity(&store_path).unwrap();
        let held_nested_temp = hold_shared_directory_identity(&nested_temp_path).unwrap();

        assert!(require_system_temp_outside_store(
            outer.path(),
            &held_outer,
            &store_path,
            &held_store,
        )
        .is_ok());
        assert_eq!(
            require_system_temp_outside_store(
                &nested_temp_path,
                &held_nested_temp,
                &store_path,
                &held_store,
            )
            .unwrap_err()
            .code,
            "AUTHORING_REVISION3_VOICE_PREVIEW_PREVIEW_CAPABILITY_INVALID"
        );
        assert!(fs::read_dir(&nested_temp_path).unwrap().next().is_none());
    }

    #[cfg(windows)]
    #[test]
    fn errors_never_expose_store_asset_preview_paths_or_tokens() {
        let _serial = test_serial_guard();
        let store = published_store();
        let mut preview = RegisteredPreview::new(store.temp.path());
        fs::remove_file(&store.asset_path).unwrap();
        let response = materialize_revision3_voice_take_preview_v1_raw(
            &preview.wire(store.temp.path(), &request(&store)),
        );
        let message = response["error"]["message"].as_str().unwrap();
        assert!(!message.contains(store.temp.path().to_string_lossy().as_ref()));
        assert!(!message.contains(preview.root.to_string_lossy().as_ref()));
        assert!(!message.contains(&preview.cleanup_token));
        assert!(!message.contains(&request(&store).expected_asset.sha256.to_string()));
        assert_eq!(preview.release()["ok"], true);
    }

    #[cfg(not(windows))]
    #[test]
    fn unsupported_platform_registration_fails_closed_without_creating_output() {
        let store = published_store();
        let response =
            register_revision3_voice_take_preview_v1_raw(&register_wire(store.temp.path()));
        assert_eq!(
            response["error"]["code"],
            "AUTHORING_REVISION3_VOICE_PREVIEW_PREVIEW_CAPABILITY_UNAVAILABLE"
        );
    }
}
