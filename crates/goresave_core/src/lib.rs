pub mod codec_backend;
mod kraken;
pub mod properties;

use base64::{Engine as _, engine::general_purpose};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha1::{Digest, Sha1};
use std::collections::HashMap;
use std::ffi::{CStr, CString, c_char};
use std::fs;
use std::path::{Path, PathBuf};
use std::ptr;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;

const PACKAGE_FILE_TAG: u32 = 0x9E2A83C1;
const COMPRESSED_HEADER_V2: u32 = 0x22222222;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("invalid request: {0}")]
    InvalidRequest(String),
    #[error("io error: {0}")]
    Io(String),
    #[error("parse error: {0}")]
    Parse(String),
    #[error("unsupported edit: {0}")]
    UnsupportedEdit(String),
    #[error("codec error: {0}")]
    Codec(String),
    #[error("validation failed: {0}")]
    Validation(String),
    #[error("update error: {0}")]
    Update(String),
}

impl From<std::io::Error> for CoreError {
    fn from(value: std::io::Error) -> Self {
        CoreError::Io(value.to_string())
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompressedChunk {
    pub index: usize,
    pub compressed_size: u64,
    pub uncompressed_size: u64,
    pub compressed_offset: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompressedStream {
    pub stream_offset: usize,
    pub uncompressed_size_prefix: u64,
    pub method: String,
    pub package_tag: u32,
    pub header_version: u32,
    pub max_chunk_size: u64,
    pub algorithm_id: Option<u8>,
    pub summary_compressed_size: u64,
    pub summary_uncompressed_size: u64,
    pub chunk_count: usize,
    pub compressed_payload_offset: usize,
    pub compressed_payload_size: u64,
    pub stream_end_offset: usize,
    pub trailing_size: usize,
    pub chunks: Vec<CompressedChunk>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GsavInfo {
    pub format: &'static str,
    pub path: Option<String>,
    pub slot: Option<String>,
    pub size: usize,
    pub sha1: String,
    pub version_byte: u8,
    pub body_size_field: u32,
    pub body_size_delta: isize,
    pub public_payload_offset: usize,
    pub public_payload_size: usize,
    pub public_payload_sha1: String,
    pub public: PublicSummary,
    pub compressed_stream: CompressedStreamSummary,
    pub trailer_offset: usize,
    pub trailer_size: usize,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicSummary {
    pub slot_name: Option<String>,
    pub player_save_name: Option<String>,
    pub profile_id: Option<i32>,
    pub object_paths: Vec<String>,
    pub strings: Vec<String>,
    pub editable: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompressedStreamSummary {
    pub method: String,
    pub algorithm_id: Option<u8>,
    pub chunk_count: usize,
    pub compressed_size: u64,
    pub uncompressed_size: u64,
    pub trailing_size: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveListItem {
    pub path: String,
    pub slot: String,
    pub format: String,
    pub file_size: u64,
    pub sha1: String,
    pub status: String,
    pub player_save_name: Option<String>,
    pub slot_name: Option<String>,
    pub compression_method: Option<String>,
    pub chunk_count: Option<usize>,
    pub persistent_player_save_name: Option<String>,
    pub chapter_id: Option<i32>,
    pub map_name: Option<String>,
    pub time_played_seconds: Option<f64>,
    pub time_loaded_seconds: Option<f64>,
    pub quick_save: Option<bool>,
    pub auto_save: Option<bool>,
    pub persistent_profile_id: Option<i32>,
    pub screenshot: Option<ScreenshotSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub difficulty: Option<DifficultySettings>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenshotSummary {
    pub mime_type: String,
    pub byte_length: usize,
    pub bytes_base64: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileSummary {
    pub profile_id: i32,
    pub profile_name: Option<String>,
    pub quick_save_slots: Vec<String>,
    pub auto_save_slots: Vec<String>,
    pub saved_slots: Vec<String>,
    pub difficulty_preset: Option<String>,
    pub custom_combat_settings: Option<String>,
    pub custom_resources_settings: Option<String>,
    pub custom_progression_settings: Option<String>,
    pub survival: Option<bool>,
    pub permanent_death: Option<bool>,
    pub permanent_death_game_over: Option<bool>,
    pub fake_sloppy_combos: Option<bool>,
    pub max_quick: Option<i32>,
    pub max_auto: Option<i32>,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DifficultySettings {
    pub preset: Option<String>,
    pub combat: Option<String>,
    pub resources: Option<String>,
    pub progression: Option<String>,
    pub flow_helper: Option<bool>,
    pub permadeath: Option<bool>,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveDirSummary {
    pub saves: Vec<SaveListItem>,
    pub profiles: Vec<ProfileSummary>,
    pub active_profile_id: Option<i32>,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PersistentSlotMetadata {
    pub player_save_name: Option<String>,
    pub slot_name: Option<String>,
    pub chapter_id: Option<i32>,
    pub map_name: Option<String>,
    pub time_played_seconds: Option<f64>,
    pub time_loaded_seconds: Option<f64>,
    pub quick_save: Option<bool>,
    pub auto_save: Option<bool>,
    pub profile_id: Option<i32>,
}

#[derive(Debug, Clone, Default)]
struct PersistentDataListSummary {
    slots: HashMap<String, PersistentSlotMetadata>,
    profiles: Vec<ProfileSummary>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupListItem {
    pub path: String,
    pub file_name: String,
    pub file_size: u64,
    pub sha1: String,
    pub created_epoch: Option<u64>,
    pub status: String,
    pub player_save_name: Option<String>,
    pub slot_name: Option<String>,
    pub scope: String,
}

#[derive(Debug, Clone)]
struct FStringRef {
    value: String,
    len_offset: usize,
    total_len: usize,
    utf16: bool,
}

pub(crate) struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
    base_offset: usize,
}

impl<'a> Reader<'a> {
    pub(crate) fn new(data: &'a [u8], base_offset: usize) -> Self {
        Self {
            data,
            pos: 0,
            base_offset,
        }
    }

    pub(crate) fn abs_pos(&self) -> usize {
        self.base_offset + self.pos
    }

    pub(crate) fn remaining(&self) -> usize {
        self.data.len().saturating_sub(self.pos)
    }

    pub(crate) fn read(&mut self, n: usize) -> Result<&'a [u8], CoreError> {
        if self.pos + n > self.data.len() {
            return Err(CoreError::Parse(format!(
                "read out of bounds at 0x{:x}: need {}, remaining {}",
                self.abs_pos(),
                n,
                self.remaining()
            )));
        }
        let out = &self.data[self.pos..self.pos + n];
        self.pos += n;
        Ok(out)
    }

    pub(crate) fn u8(&mut self) -> Result<u8, CoreError> {
        Ok(self.read(1)?[0])
    }

    pub(crate) fn u32(&mut self) -> Result<u32, CoreError> {
        let mut b = [0u8; 4];
        b.copy_from_slice(self.read(4)?);
        Ok(u32::from_le_bytes(b))
    }

    pub(crate) fn i32(&mut self) -> Result<i32, CoreError> {
        let mut b = [0u8; 4];
        b.copy_from_slice(self.read(4)?);
        Ok(i32::from_le_bytes(b))
    }

    pub(crate) fn u64(&mut self) -> Result<u64, CoreError> {
        let mut b = [0u8; 8];
        b.copy_from_slice(self.read(8)?);
        Ok(u64::from_le_bytes(b))
    }

    pub(crate) fn i64(&mut self) -> Result<i64, CoreError> {
        let mut b = [0u8; 8];
        b.copy_from_slice(self.read(8)?);
        Ok(i64::from_le_bytes(b))
    }

    pub(crate) fn f32(&mut self) -> Result<f32, CoreError> {
        let mut b = [0u8; 4];
        b.copy_from_slice(self.read(4)?);
        Ok(f32::from_le_bytes(b))
    }

    pub(crate) fn f64(&mut self) -> Result<f64, CoreError> {
        let mut b = [0u8; 8];
        b.copy_from_slice(self.read(8)?);
        Ok(f64::from_le_bytes(b))
    }

    pub(crate) fn fstring(&mut self) -> Result<String, CoreError> {
        let n = self.i32()?;
        if n == 0 {
            return Ok(String::new());
        }
        if n > 0 {
            let raw = self.read(n as usize)?;
            let body = raw.strip_suffix(&[0]).unwrap_or(raw);
            return Ok(String::from_utf8_lossy(body).to_string());
        }
        // `-n` overflows (panic in debug, wrap in release) when n == i32::MIN
        // (0x80000000). Reject it via checked negation instead of aborting
        // across the FFI boundary on a corrupt UTF-16 length.
        let chars = n
            .checked_neg()
            .ok_or_else(|| CoreError::Parse("invalid UTF-16 FString length".to_string()))?
            as usize;
        let raw = self.read(chars * 2)?;
        let units = raw
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .take_while(|u| *u != 0)
            .collect::<Vec<_>>();
        String::from_utf16(&units).map_err(|e| CoreError::Parse(e.to_string()))
    }
}

pub fn execute_json(input: &str) -> String {
    match execute_json_inner(input) {
        Ok(data) => json!({ "ok": true, "data": data }).to_string(),
        Err(err) => {
            let code = match &err {
                CoreError::InvalidRequest(_) => "INVALID_REQUEST",
                CoreError::Io(_) => "IO_ERROR",
                CoreError::Parse(_) => "PARSE_ERROR",
                CoreError::UnsupportedEdit(_) => "UNSUPPORTED_EDIT",
                CoreError::Codec(_) => "CODEC_ERROR",
                CoreError::Validation(_) => "VALIDATION_FAILED",
                CoreError::Update(_) => "UPDATE_ERROR",
            };
            json!({
                "ok": false,
                "error": {
                    "code": code,
                    "message": err.to_string()
                }
            })
            .to_string()
        }
    }
}

fn execute_json_inner(input: &str) -> Result<Value, CoreError> {
    let value: Value =
        serde_json::from_str(input).map_err(|e| CoreError::InvalidRequest(e.to_string()))?;
    let command = value
        .get("command")
        .and_then(Value::as_str)
        .ok_or_else(|| CoreError::InvalidRequest("missing command".to_string()))?;
    let payload = value.get("payload").cloned().unwrap_or_else(|| json!({}));

    match command {
        "scan_save_dir" => {
            let path = payload
                .get("path")
                .and_then(Value::as_str)
                .map(PathBuf::from)
                .unwrap_or_else(default_save_root);
            let codec_backend = payload
                .get("binaryHost")
                .map(ensured_binary_host_from_config)
                .transpose()?;
            let codec_backend = codec_backend
                .as_ref()
                .map(|backend| backend as &dyn codec_backend::CodecBackend);
            let summary = scan_save_dir_summary_with_codec_backend(&path, codec_backend)?;
            Ok(json!({
                "saveRoot": path,
                "saves": summary.saves,
                "profiles": summary.profiles,
                "activeProfileId": summary.active_profile_id,
            }))
        }
        "inspect_save" => {
            let path = required_path(&payload)?;
            let include_private = payload
                .get("includePrivate")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let private_chunk_limit = payload
                .get("privateChunkLimit")
                .and_then(Value::as_u64)
                .map(|value| value as usize);
            let codec_backend = payload
                .get("binaryHost")
                .map(ensured_binary_host_from_config)
                .transpose()?;
            let codec_backend = codec_backend
                .as_ref()
                .map(|backend| backend as &dyn codec_backend::CodecBackend);
            Ok(inspect_save_with_codec_backend(
                &path,
                include_private,
                codec_backend,
                private_chunk_limit,
            )?)
        }
        "check_codec" => check_codec(&payload),
        "search_typed_properties" => {
            let path = required_path(&payload)?;
            let codec_backend = payload
                .get("binaryHost")
                .map(ensured_binary_host_from_config)
                .transpose()?;
            let codec_backend = codec_backend
                .as_ref()
                .map(|backend| backend as &dyn codec_backend::CodecBackend);
            search_typed_properties(&path, &payload, codec_backend)
        }
        "query_progression" => {
            let path = required_path(&payload)?;
            let codec_backend = payload
                .get("binaryHost")
                .map(ensured_binary_host_from_config)
                .transpose()?;
            let codec_backend = codec_backend
                .as_ref()
                .map(|backend| backend as &dyn codec_backend::CodecBackend);
            query_progression(&path, &payload, codec_backend)
        }
        "validate_roundtrip" => {
            let path = required_path(&payload)?;
            Ok(validate_roundtrip(&path)?)
        }
        "list_backups" => {
            let path = required_path(&payload)?;
            Ok(json!({
                "path": path,
                "backups": list_save_backups(&path)?,
                "companionBackups": list_persistent_data_list_backups_for_save(&path)?,
            }))
        }
        "restore_backup" => {
            let path = required_path(&payload)?;
            let backup_path = payload
                .get("backupPath")
                .and_then(Value::as_str)
                .map(PathBuf::from)
                .ok_or_else(|| {
                    CoreError::InvalidRequest("missing payload.backupPath".to_string())
                })?;
            Ok(restore_backup(&path, &backup_path)?)
        }
        "validate_codec_roundtrip" => {
            let path = required_path(&payload)?;
            let binary_host = payload.get("binaryHost").ok_or_else(|| {
                CoreError::InvalidRequest(
                    "validate_codec_roundtrip requires payload.binaryHost".to_string(),
                )
            })?;
            let backend = ensured_binary_host_from_config(binary_host)?;
            Ok(validate_codec_roundtrip_with_backend(&path, &backend)?)
        }
        "write_save" => {
            let path = required_path(&payload)?;
            let edits = payload
                .get("edits")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            let backup = payload
                .get("backup")
                .and_then(Value::as_bool)
                .unwrap_or(true);
            let output_path = payload
                .get("outputPath")
                .and_then(Value::as_str)
                .map(PathBuf::from);
            let sync_persistent_data_list = payload
                .get("syncPersistentDataList")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let codec_backend = payload
                .get("binaryHost")
                .map(ensured_binary_host_from_config)
                .transpose()?;
            let codec_backend = codec_backend
                .as_ref()
                .map(|backend| backend as &dyn codec_backend::CodecBackend);
            Ok(write_save_internal(
                &path,
                &edits,
                backup,
                output_path.as_deref(),
                codec_backend,
                sync_persistent_data_list,
            )?)
        }
        "write_difficulty" => {
            let req: DifficultyRequest =
                serde_json::from_value(payload.get("difficulty").cloned().unwrap_or(Value::Null))
                    .map_err(|e| CoreError::InvalidRequest(e.to_string()))?;
            let backup = payload
                .get("backup")
                .and_then(Value::as_bool)
                .unwrap_or(true);
            // Difficulty is profile-only now; PersistentDataList.sav is a plain
            // GVAS file with no compressed stream, so no codec host is needed.
            // Accept BOTH the in-tree `targets: { profile }` wrapper and the
            // top-level `profile: { path, profileId }` shape documented in the
            // spec, so direct/API callers following the docs work too.
            let targets = match payload.get("targets") {
                Some(t) if !t.is_null() => t.clone(),
                _ => match payload.get("profile") {
                    Some(profile) if !profile.is_null() => json!({ "profile": profile }),
                    _ => Value::Null,
                },
            };
            Ok(write_difficulty_internal(&req, &targets, backup)?)
        }
        other => Err(CoreError::InvalidRequest(format!(
            "unknown command {other:?}"
        ))),
    }
}

fn required_path(payload: &Value) -> Result<PathBuf, CoreError> {
    payload
        .get("path")
        .and_then(Value::as_str)
        .map(PathBuf::from)
        .ok_or_else(|| CoreError::InvalidRequest("missing payload.path".to_string()))
}

pub fn scan_save_dir(path: &Path) -> Result<Vec<SaveListItem>, CoreError> {
    Ok(scan_save_dir_summary(path)?.saves)
}

pub fn scan_save_dir_summary(path: &Path) -> Result<SaveDirSummary, CoreError> {
    scan_save_dir_summary_with_codec_backend(path, None)
}

fn scan_save_dir_summary_with_codec_backend(
    path: &Path,
    codec_backend: Option<&dyn codec_backend::CodecBackend>,
) -> Result<SaveDirSummary, CoreError> {
    if !path.exists() {
        return Err(CoreError::Io(format!(
            "save directory does not exist: {}",
            path.display()
        )));
    }
    let persistent = persistent_data_list_summary_for_dir(path).unwrap_or_default();
    let persistent_slots = &persistent.slots;
    let screenshots =
        screenshot_summaries_for_dir(path, &persistent.profiles, codec_backend).unwrap_or_default();
    let mut saves = Vec::new();
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("sav") {
            continue;
        }
        let slot = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();
        if is_sidecar_save_stem(&slot) || !looks_slot_name(&slot) {
            continue;
        }
        let data = fs::read(&path)?;
        let persistent = persistent_slots.get(&slot);
        let screenshot = screenshots.get(&slot).cloned();
        match inspect_bytes(&data, Some(&path), false) {
            Ok(info) => {
                let public = info.get("public").cloned().unwrap_or_else(|| json!({}));
                let stream = info
                    .get("compressedStream")
                    .cloned()
                    .unwrap_or_else(|| json!({}));
                saves.push(SaveListItem {
                    path: path.display().to_string(),
                    slot,
                    format: info
                        .get("format")
                        .and_then(Value::as_str)
                        .unwrap_or("UNKNOWN")
                        .to_string(),
                    file_size: data.len() as u64,
                    sha1: sha1_hex(&data),
                    status: "ok".to_string(),
                    player_save_name: public
                        .get("playerSaveName")
                        .and_then(Value::as_str)
                        .map(ToOwned::to_owned),
                    slot_name: public
                        .get("slotName")
                        .and_then(Value::as_str)
                        .map(ToOwned::to_owned),
                    compression_method: stream
                        .get("method")
                        .and_then(Value::as_str)
                        .map(ToOwned::to_owned),
                    chunk_count: stream
                        .get("chunkCount")
                        .and_then(Value::as_u64)
                        .map(|v| v as usize),
                    persistent_player_save_name: persistent
                        .and_then(|metadata| metadata.player_save_name.clone()),
                    chapter_id: persistent.and_then(|metadata| metadata.chapter_id),
                    map_name: persistent.and_then(|metadata| metadata.map_name.clone()),
                    time_played_seconds: persistent
                        .and_then(|metadata| metadata.time_played_seconds),
                    time_loaded_seconds: persistent
                        .and_then(|metadata| metadata.time_loaded_seconds),
                    quick_save: persistent.and_then(|metadata| metadata.quick_save),
                    auto_save: persistent.and_then(|metadata| metadata.auto_save),
                    persistent_profile_id: persistent.and_then(|metadata| metadata.profile_id),
                    screenshot: screenshot.clone(),
                    difficulty: difficulty_for_gsav_bytes(&data),
                });
            }
            Err(err) => saves.push(SaveListItem {
                path: path.display().to_string(),
                slot,
                format: "UNKNOWN".to_string(),
                file_size: data.len() as u64,
                sha1: sha1_hex(&data),
                status: err.to_string(),
                player_save_name: None,
                slot_name: None,
                compression_method: None,
                chunk_count: None,
                persistent_player_save_name: persistent
                    .and_then(|metadata| metadata.player_save_name.clone()),
                chapter_id: persistent.and_then(|metadata| metadata.chapter_id),
                map_name: persistent.and_then(|metadata| metadata.map_name.clone()),
                time_played_seconds: persistent.and_then(|metadata| metadata.time_played_seconds),
                time_loaded_seconds: persistent.and_then(|metadata| metadata.time_loaded_seconds),
                quick_save: persistent.and_then(|metadata| metadata.quick_save),
                auto_save: persistent.and_then(|metadata| metadata.auto_save),
                persistent_profile_id: persistent.and_then(|metadata| metadata.profile_id),
                screenshot: screenshot.clone(),
                difficulty: None,
            }),
        }
    }
    saves.sort_by(|a, b| a.slot.cmp(&b.slot));
    let active_profile_id = saves
        .iter()
        .find_map(|save| save.persistent_profile_id)
        .or_else(|| {
            persistent
                .profiles
                .first()
                .map(|profile| profile.profile_id)
        });
    Ok(SaveDirSummary {
        saves,
        profiles: persistent.profiles,
        active_profile_id,
    })
}

fn screenshot_summaries_for_dir(
    dir: &Path,
    profiles: &[ProfileSummary],
    codec_backend: Option<&dyn codec_backend::CodecBackend>,
) -> Result<HashMap<String, ScreenshotSummary>, CoreError> {
    let mut profile_ids = profiles
        .iter()
        .map(|profile| profile.profile_id)
        .collect::<Vec<_>>();
    if profile_ids.is_empty() {
        profile_ids.push(0);
    }
    profile_ids.sort_unstable();
    profile_ids.dedup();

    let mut screenshots = HashMap::new();
    for profile_id in profile_ids {
        let path = dir.join(format!("Profile_{profile_id}_Screenshots.sav"));
        if !path.exists() {
            continue;
        }
        // Screenshots are optional. A missing/failed codec or an unreadable
        // sidecar must not drop other profiles' thumbnails or abort the scan;
        // skip just this profile and leave its thumbnails unavailable.
        let Ok(data) = fs::read(&path) else {
            continue;
        };
        match parse_screenshot_save(&data, codec_backend) {
            Ok(parsed) => {
                for (slot, screenshot) in parsed {
                    screenshots.insert(slot, screenshot);
                }
            }
            Err(_) => continue,
        }
    }
    Ok(screenshots)
}

fn parse_screenshot_save(
    data: &[u8],
    codec_backend: Option<&dyn codec_backend::CodecBackend>,
) -> Result<HashMap<String, ScreenshotSummary>, CoreError> {
    if !data.starts_with(b"GSAV") {
        return Ok(parse_screenshot_payload(data));
    }
    let private_offset = gsav_private_payload_offset(data)?;
    match parse_compressed_stream(data, private_offset) {
        Ok(stream) => {
            let payload = decode_private_payload_best_effort(data, &stream, codec_backend)?;
            Ok(parse_screenshot_payload(&payload))
        }
        Err(_) => Ok(parse_screenshot_payload(
            data.get(private_offset..).unwrap_or_default(),
        )),
    }
}

fn decode_private_payload_best_effort(
    data: &[u8],
    stream: &CompressedStream,
    codec_backend: Option<&dyn codec_backend::CodecBackend>,
) -> Result<Vec<u8>, CoreError> {
    if let Some(backend) = codec_backend {
        if let Ok(payload) = decompress_private_payload(data, stream, backend) {
            return Ok(payload);
        }
    }
    stored_private_payload(data, stream)
        .ok_or_else(|| CoreError::Codec("screenshot payload requires a codec backend".to_string()))
}

fn stored_private_payload(data: &[u8], stream: &CompressedStream) -> Option<Vec<u8>> {
    // Only valid when every chunk is stored uncompressed. Accumulate the real
    // stored length from the chunk table and bound it by the file size before
    // reserving, so a corrupt header declaring a huge summary_uncompressed_size
    // can't force a multi-gigabyte allocation here.
    let mut total: usize = 0;
    for chunk in &stream.chunks {
        if chunk.compressed_size != chunk.uncompressed_size {
            return None;
        }
        total = total.checked_add(chunk.compressed_size as usize)?;
    }
    if total > data.len() {
        return None;
    }
    let mut out = Vec::with_capacity(total);
    for chunk in &stream.chunks {
        let start = chunk.compressed_offset;
        let end = start.checked_add(chunk.compressed_size as usize)?;
        out.extend_from_slice(data.get(start..end)?);
    }
    Some(out)
}

fn gsav_private_payload_offset(data: &[u8]) -> Result<usize, CoreError> {
    if !data.starts_with(b"GSAV") {
        return Err(CoreError::Parse("not a GSAV file".to_string()));
    }
    if data.len() < 13 {
        return Err(CoreError::Parse(
            "GSAV file is shorter than header".to_string(),
        ));
    }
    let public_payload_size = u32::from_le_bytes(data[9..13].try_into().unwrap()) as usize;
    let private_offset = 13usize
        .checked_add(public_payload_size)
        .ok_or_else(|| CoreError::Parse("public payload size overflow".to_string()))?;
    if private_offset > data.len() {
        return Err(CoreError::Parse(
            "public payload extends past EOF".to_string(),
        ));
    }
    Ok(private_offset)
}

fn persistent_slot_metadata_for_save(path: &Path) -> Option<PersistentSlotMetadata> {
    let slot = path.file_stem()?.to_str()?;
    let parent = path.parent()?;
    persistent_slot_metadata_for_dir(parent)
        .ok()
        .and_then(|metadata| metadata.get(slot).cloned())
}

fn persistent_slot_metadata_for_dir(
    dir: &Path,
) -> Result<HashMap<String, PersistentSlotMetadata>, CoreError> {
    Ok(persistent_data_list_summary_for_dir(dir)?.slots)
}

fn persistent_data_list_summary_for_dir(
    dir: &Path,
) -> Result<PersistentDataListSummary, CoreError> {
    let path = dir.join("PersistentDataList.sav");
    if !path.exists() {
        return Ok(PersistentDataListSummary::default());
    }
    let data = fs::read(path)?;
    Ok(parse_persistent_data_list_summary(&data))
}

fn parse_persistent_slot_metadata(data: &[u8]) -> HashMap<String, PersistentSlotMetadata> {
    parse_persistent_data_list_summary(data).slots
}

fn parse_persistent_data_list_summary(data: &[u8]) -> PersistentDataListSummary {
    if !data.starts_with(b"GVAS") {
        return PersistentDataListSummary::default();
    }
    let refs = scan_fstrings(data, 0);
    let Some((public_data_idx, end_idx)) = persistent_public_data_ref_bounds(&refs) else {
        return PersistentDataListSummary::default();
    };
    let mut slots = HashMap::new();
    let mut idx = public_data_idx + 1;
    while idx < end_idx {
        if !looks_slot_name(&refs[idx].value)
            || refs.get(idx + 1).map(|reference| reference.value.as_str()) != Some("m_SlotName")
        {
            idx += 1;
            continue;
        }
        let slot = refs[idx].value.clone();
        let block_end = refs
            .iter()
            .enumerate()
            .take(end_idx)
            .skip(idx + 1)
            .find(|(candidate_idx, reference)| {
                looks_slot_name(&reference.value)
                    && refs.get(candidate_idx + 1).map(|next| next.value.as_str())
                        == Some("m_SlotName")
            })
            .map(|(candidate_idx, _)| candidate_idx)
            .unwrap_or(end_idx);
        slots.insert(
            slot,
            persistent_metadata_from_ref_range(data, &refs, idx, block_end),
        );
        idx = block_end;
    }
    let profiles = parse_profile_summaries(data, &refs, &slots);
    PersistentDataListSummary { slots, profiles }
}

fn persistent_public_data_ref_bounds(refs: &[FStringRef]) -> Option<(usize, usize)> {
    let public_data_idx = refs
        .iter()
        .position(|reference| reference.value == "m_SavedGamesPublicData")?;
    let end_idx = refs
        .iter()
        .enumerate()
        .skip(public_data_idx + 1)
        .find(|(_, reference)| {
            matches!(reference.value.as_str(), "m_Profiles" | "m_ScreenshotSave")
        })
        .map(|(idx, _)| idx)
        .unwrap_or(refs.len());
    Some((public_data_idx, end_idx))
}

fn persistent_slot_ref_range(refs: &[FStringRef], slot: &str) -> Option<(usize, usize)> {
    let (public_data_idx, end_idx) = persistent_public_data_ref_bounds(refs)?;
    let mut idx = public_data_idx + 1;
    while idx < end_idx {
        if !looks_slot_name(&refs[idx].value)
            || refs.get(idx + 1).map(|reference| reference.value.as_str()) != Some("m_SlotName")
        {
            idx += 1;
            continue;
        }
        let block_end = refs
            .iter()
            .enumerate()
            .take(end_idx)
            .skip(idx + 1)
            .find(|(candidate_idx, reference)| {
                looks_slot_name(&reference.value)
                    && refs.get(candidate_idx + 1).map(|next| next.value.as_str())
                        == Some("m_SlotName")
            })
            .map(|(candidate_idx, _)| candidate_idx)
            .unwrap_or(end_idx);
        if refs[idx].value == slot {
            return Some((idx, block_end));
        }
        idx = block_end;
    }
    None
}

fn persistent_metadata_from_ref_range(
    payload: &[u8],
    refs: &[FStringRef],
    start_idx: usize,
    end_idx: usize,
) -> PersistentSlotMetadata {
    PersistentSlotMetadata {
        player_save_name: value_after_property_in_range(
            refs,
            start_idx,
            end_idx,
            "m_PlayerSaveName",
        ),
        slot_name: value_after_property_in_range(refs, start_idx, end_idx, "m_SlotName"),
        chapter_id: read_i32_property_in_range(payload, refs, start_idx, end_idx, "m_ChapterID"),
        map_name: value_after_property_in_range(refs, start_idx, end_idx, "m_MapName"),
        time_played_seconds: read_f64_property_in_range(
            payload,
            refs,
            start_idx,
            end_idx,
            "m_TimePlayed",
        ),
        time_loaded_seconds: read_f64_property_in_range(
            payload,
            refs,
            start_idx,
            end_idx,
            "m_TimeLoaded",
        ),
        quick_save: read_bool_property_in_range(payload, refs, start_idx, end_idx, "m_QuickSave"),
        auto_save: read_bool_property_in_range(payload, refs, start_idx, end_idx, "m_AutoSave"),
        profile_id: read_i32_property_in_range(payload, refs, start_idx, end_idx, "m_ProfileId"),
    }
}

fn parse_profile_summaries(
    payload: &[u8],
    refs: &[FStringRef],
    slots: &HashMap<String, PersistentSlotMetadata>,
) -> Vec<ProfileSummary> {
    let Some(profiles_idx) = refs
        .iter()
        .position(|reference| reference.value == "m_Profiles")
    else {
        return profile_summaries_from_slots(slots);
    };
    let profiles_end = refs
        .iter()
        .enumerate()
        .skip(profiles_idx + 1)
        .find(|(_, reference)| matches!(reference.value.as_str(), "SavedDataVersion"))
        .map(|(idx, _)| idx)
        .unwrap_or(refs.len());
    let starts = refs
        .iter()
        .enumerate()
        .take(profiles_end)
        .skip(profiles_idx + 1)
        .filter_map(|(idx, reference)| (reference.value == "m_ProfileName").then_some(idx))
        .collect::<Vec<_>>();
    if starts.is_empty() {
        return profile_summaries_from_slots(slots);
    }

    let mut profiles = starts
        .iter()
        .enumerate()
        .map(|(ordinal, start_idx)| {
            let end_idx = starts.get(ordinal + 1).copied().unwrap_or(profiles_end);
            let profile_id =
                read_i32_property_in_range(payload, refs, *start_idx, end_idx, "m_ProfileId")
                    .unwrap_or(ordinal as i32);
            let mut saved_slots =
                slot_values_after_property_in_range(refs, *start_idx, end_idx, "m_SavedSlotsNames");
            if saved_slots.is_empty() {
                saved_slots = slots_for_profile(slots, profile_id, |_| true);
            }
            ProfileSummary {
                profile_id,
                profile_name: value_after_property_in_range(
                    refs,
                    *start_idx,
                    end_idx,
                    "m_ProfileName",
                ),
                quick_save_slots: slot_values_after_property_in_range(
                    refs,
                    *start_idx,
                    end_idx,
                    "m_QuickSaveName",
                ),
                auto_save_slots: slot_values_after_property_in_range(
                    refs,
                    *start_idx,
                    end_idx,
                    "m_AutoSaveName",
                ),
                saved_slots,
                difficulty_preset: value_after_property_in_range(
                    refs,
                    *start_idx,
                    end_idx,
                    "m_difficultyPreset",
                ),
                custom_combat_settings: value_after_property_in_range(
                    refs,
                    *start_idx,
                    end_idx,
                    "m_customCombatSettings",
                ),
                custom_resources_settings: value_after_property_in_range(
                    refs,
                    *start_idx,
                    end_idx,
                    "m_customResourcesSettings",
                ),
                custom_progression_settings: value_after_property_in_range(
                    refs,
                    *start_idx,
                    end_idx,
                    "m_customProgressionSettings",
                ),
                survival: read_bool_property_in_range(
                    payload,
                    refs,
                    *start_idx,
                    end_idx,
                    "m_Survival",
                )
                .or_else(|| {
                    read_bool_property_in_range(
                        payload,
                        refs,
                        *start_idx,
                        end_idx,
                        "m_SurvivalMode",
                    )
                }),
                permanent_death: read_bool_property_in_range(
                    payload,
                    refs,
                    *start_idx,
                    end_idx,
                    "m_PermanentDeath",
                )
                .or_else(|| {
                    read_bool_property_in_range(payload, refs, *start_idx, end_idx, "m_PermaDeath")
                }),
                permanent_death_game_over: read_bool_property_in_range(
                    payload,
                    refs,
                    *start_idx,
                    end_idx,
                    "m_PermanentDeathGameOver",
                ),
                fake_sloppy_combos: read_bool_property_in_range(
                    payload,
                    refs,
                    *start_idx,
                    end_idx,
                    "m_FakeSloppyCombos",
                ),
                max_quick: read_i32_property_in_range(
                    payload,
                    refs,
                    *start_idx,
                    end_idx,
                    "m_MaxQuick",
                ),
                max_auto: read_i32_property_in_range(
                    payload,
                    refs,
                    *start_idx,
                    end_idx,
                    "m_MaxAuto",
                ),
            }
        })
        .collect::<Vec<_>>();
    profiles.sort_by_key(|profile| profile.profile_id);
    profiles
}

fn profile_summaries_from_slots(
    slots: &HashMap<String, PersistentSlotMetadata>,
) -> Vec<ProfileSummary> {
    let mut profile_ids = slots
        .values()
        .filter_map(|metadata| metadata.profile_id)
        .collect::<Vec<_>>();
    profile_ids.sort_unstable();
    profile_ids.dedup();
    profile_ids
        .into_iter()
        .map(|profile_id| ProfileSummary {
            profile_id,
            profile_name: Some(profile_id.to_string()),
            quick_save_slots: slots_for_profile(slots, profile_id, |metadata| {
                metadata.quick_save == Some(true)
            }),
            auto_save_slots: slots_for_profile(slots, profile_id, |metadata| {
                metadata.auto_save == Some(true)
            }),
            saved_slots: slots_for_profile(slots, profile_id, |_| true),
            difficulty_preset: None,
            custom_combat_settings: None,
            custom_resources_settings: None,
            custom_progression_settings: None,
            survival: None,
            permanent_death: None,
            permanent_death_game_over: None,
            fake_sloppy_combos: None,
            max_quick: None,
            max_auto: None,
        })
        .collect()
}

fn slots_for_profile(
    slots: &HashMap<String, PersistentSlotMetadata>,
    profile_id: i32,
    filter: impl Fn(&PersistentSlotMetadata) -> bool,
) -> Vec<String> {
    let mut values = slots
        .iter()
        .filter_map(|(slot, metadata)| {
            (metadata.profile_id == Some(profile_id) && filter(metadata)).then(|| slot.clone())
        })
        .collect::<Vec<_>>();
    values.sort();
    values
}

fn slot_values_after_property_in_range(
    refs: &[FStringRef],
    start_idx: usize,
    end_idx: usize,
    name: &str,
) -> Vec<String> {
    let Some(name_idx) = find_ref_in_range(refs, start_idx, end_idx, name) else {
        return Vec::new();
    };
    let mut values = Vec::new();
    for reference in refs.iter().take(end_idx).skip(name_idx + 1) {
        if reference.value == "SavedDataVersion"
            || reference.value == "None"
            || is_profile_summary_property(reference.value.as_str())
        {
            break;
        }
        if looks_slot_name(&reference.value) && !values.contains(&reference.value) {
            values.push(reference.value.clone());
        }
    }
    values
}

fn is_profile_summary_property(value: &str) -> bool {
    matches!(
        value,
        "m_ProfileName"
            | "m_ProfileId"
            | "m_QuickSaveName"
            | "m_AutoSaveName"
            | "m_SavedSlotsNames"
            | "m_difficultyPreset"
            | "m_customCombatSettings"
            | "m_customResourcesSettings"
            | "m_customProgressionSettings"
            | "m_Survival"
            | "m_SurvivalMode"
            | "m_PermanentDeath"
            | "m_PermaDeath"
            | "m_PermanentDeathGameOver"
            | "m_FakeSloppyCombos"
            | "m_MaxQuick"
            | "m_MaxAuto"
    )
}

fn parse_screenshot_payload(payload: &[u8]) -> HashMap<String, ScreenshotSummary> {
    let refs = scan_fstrings(payload, 0);
    let mut screenshots = HashMap::new();
    for (idx, reference) in refs.iter().enumerate() {
        if !looks_slot_name(&reference.value) {
            continue;
        }
        let start = reference.len_offset + reference.total_len;
        let end = refs
            .iter()
            .skip(idx + 1)
            .find(|candidate| looks_slot_name(&candidate.value))
            .map(|candidate| candidate.len_offset)
            .unwrap_or(payload.len());
        let Some(jpeg) = jpeg_after_slot(payload, start, end) else {
            continue;
        };
        screenshots.insert(
            reference.value.clone(),
            ScreenshotSummary {
                mime_type: "image/jpeg".to_string(),
                byte_length: jpeg.len(),
                bytes_base64: general_purpose::STANDARD.encode(jpeg),
            },
        );
    }
    screenshots
}

fn jpeg_after_slot(payload: &[u8], start: usize, end: usize) -> Option<&[u8]> {
    if start >= end || end > payload.len() {
        return None;
    }
    let soi = find_bytes(payload, start, end, &[0xff, 0xd8])?;
    let eoi = find_bytes(payload, soi + 2, end, &[0xff, 0xd9])? + 2;
    payload.get(soi..eoi)
}

fn find_bytes(haystack: &[u8], start: usize, end: usize, needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || end > haystack.len() || start >= end || needle.len() > end - start {
        return None;
    }
    haystack[start..end]
        .windows(needle.len())
        .position(|window| window == needle)
        .map(|offset| start + offset)
}

fn is_sidecar_save_stem(value: &str) -> bool {
    value == "PersistentDataList"
        || (value.starts_with("Profile_") && value.ends_with("_Screenshots"))
}

fn looks_slot_name(value: &str) -> bool {
    let Some(number) = value.strip_prefix("G1R-") else {
        return false;
    };
    number.len() == 3 && number.chars().all(|ch| ch.is_ascii_digit())
}

pub fn list_save_backups(path: &Path) -> Result<Vec<BackupListItem>, CoreError> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    if !parent.exists() {
        return Ok(Vec::new());
    }
    let prefix = backup_file_prefix(path)?;
    let mut backups = Vec::new();

    // Collect candidate (backup_path, file_name) pairs from both locations:
    // 1. Legacy: files in the save's parent directory matching the prefix.
    // 2. New: files in the goresave_backups subfolder matching the prefix.
    let mut candidates: Vec<(PathBuf, String)> = Vec::new();
    for entry in fs::read_dir(parent)? {
        let entry = entry?;
        let p = entry.path();
        if !p.is_file() {
            continue;
        }
        let Some(file_name) = p.file_name().and_then(|v| v.to_str()).map(str::to_owned) else {
            continue;
        };
        if file_name.starts_with(&prefix) {
            candidates.push((p, file_name));
        }
    }
    let subfolder = parent.join("goresave_backups");
    if subfolder.is_dir() {
        for entry in fs::read_dir(&subfolder)? {
            let entry = entry?;
            let p = entry.path();
            if !p.is_file() {
                continue;
            }
            let Some(file_name) = p.file_name().and_then(|v| v.to_str()).map(str::to_owned) else {
                continue;
            };
            if file_name.starts_with(&prefix) {
                candidates.push((p, file_name));
            }
        }
    }

    for (backup_path, file_name) in candidates {
        let data = fs::read(&backup_path)?;
        let metadata = fs::metadata(&backup_path)?;
        let created_epoch = parse_backup_epoch(&file_name, &prefix);
        let (status, player_save_name, slot_name) =
            match inspect_bytes(&data, Some(&backup_path), false) {
                Ok(info) => {
                    let public = info.get("public").cloned().unwrap_or_else(|| json!({}));
                    (
                        "ok".to_string(),
                        public
                            .get("playerSaveName")
                            .and_then(Value::as_str)
                            .map(ToOwned::to_owned),
                        public
                            .get("slotName")
                            .and_then(Value::as_str)
                            .map(ToOwned::to_owned),
                    )
                }
                Err(err) => (err.to_string(), None, None),
            };
        backups.push(BackupListItem {
            path: backup_path.display().to_string(),
            file_name,
            file_size: metadata.len(),
            sha1: sha1_hex(&data),
            created_epoch,
            status,
            player_save_name,
            slot_name,
            scope: "save".to_string(),
        });
    }
    backups.sort_by(|a, b| {
        b.created_epoch
            .cmp(&a.created_epoch)
            .then_with(|| b.file_name.cmp(&a.file_name))
    });
    Ok(backups)
}

fn list_persistent_data_list_backups_for_save(
    path: &Path,
) -> Result<Vec<BackupListItem>, CoreError> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    if !parent.exists() {
        return Ok(Vec::new());
    }
    let Some(slot) = path.file_stem().and_then(|value| value.to_str()) else {
        return Ok(Vec::new());
    };
    let persistent_path = parent.join("PersistentDataList.sav");
    let prefix = backup_file_prefix(&persistent_path)?;
    let mut backups = Vec::new();

    // Collect candidate (backup_path, file_name) pairs from both locations:
    // 1. Legacy: files in the save's parent directory matching the prefix.
    // 2. New: files in the goresave_backups subfolder matching the prefix.
    let mut candidates: Vec<(PathBuf, String)> = Vec::new();
    for entry in fs::read_dir(parent)? {
        let entry = entry?;
        let p = entry.path();
        if !p.is_file() {
            continue;
        }
        let Some(file_name) = p.file_name().and_then(|v| v.to_str()).map(str::to_owned) else {
            continue;
        };
        if file_name.starts_with(&prefix) {
            candidates.push((p, file_name));
        }
    }
    let subfolder = parent.join("goresave_backups");
    if subfolder.is_dir() {
        for entry in fs::read_dir(&subfolder)? {
            let entry = entry?;
            let p = entry.path();
            if !p.is_file() {
                continue;
            }
            let Some(file_name) = p.file_name().and_then(|v| v.to_str()).map(str::to_owned) else {
                continue;
            };
            if file_name.starts_with(&prefix) {
                candidates.push((p, file_name));
            }
        }
    }

    for (backup_path, file_name) in candidates {
        let data = fs::read(&backup_path)?;
        let metadata = fs::metadata(&backup_path)?;
        let created_epoch = parse_backup_epoch(&file_name, &prefix);
        let (status, player_save_name, slot_name) =
            match inspect_bytes(&data, Some(&backup_path), false) {
                Ok(_) => {
                    let persistent_slots = parse_persistent_slot_metadata(&data);
                    let slot_meta = persistent_slots.get(slot);
                    let player_save_name = slot_meta.and_then(|m| m.player_save_name.clone());
                    let slot_name = slot_meta
                        .and_then(|m| m.slot_name.clone())
                        .unwrap_or_else(|| slot.to_string());
                    // inspect_bytes' GVAS branch only checks the magic and scans
                    // strings, so require a STRICT profile parse before reporting
                    // a restorable "ok": a truncated/manual backup that still
                    // contains the slot strings must not enable the Restore
                    // action (which would overwrite the live profile with corrupt
                    // bytes). Metadata is still surfaced for display.
                    let status = if parse_profile_file(&data).is_err() {
                        "invalid PersistentDataList structure".to_string()
                    } else if slot_meta.is_none() {
                        "selected slot metadata missing".to_string()
                    } else {
                        "ok".to_string()
                    };
                    (status, player_save_name, Some(slot_name))
                }
                Err(err) => (err.to_string(), None, Some(slot.to_string())),
            };
        backups.push(BackupListItem {
            path: backup_path.display().to_string(),
            file_name,
            file_size: metadata.len(),
            sha1: sha1_hex(&data),
            created_epoch,
            status,
            player_save_name,
            slot_name,
            scope: "persistent_data_list".to_string(),
        });
    }
    backups.sort_by(|a, b| {
        b.created_epoch
            .cmp(&a.created_epoch)
            .then_with(|| b.file_name.cmp(&a.file_name))
    });
    Ok(backups)
}

fn restore_backup(path: &Path, backup_path: &Path) -> Result<Value, CoreError> {
    ensure_backup_belongs_to_save(path, backup_path)?;
    let backup_data = fs::read(backup_path)?;
    inspect_bytes(&backup_data, Some(backup_path), false)?;

    // inspect_bytes' GVAS branch only checks the magic and scans strings. Before
    // overwriting the live PersistentDataList.sav with a profile backup, require
    // it to STRICTLY parse as a profile (m_Profiles, consumes to EOF) so a
    // truncated/manual backup can never replace a valid profile file. Gate on
    // the TARGET being the profile file so an unrelated (non-profile) GVAS save
    // backup — which has no m_Profiles — stays restorable.
    let target_is_profile =
        path.file_name().and_then(|n| n.to_str()) == Some("PersistentDataList.sav");
    if target_is_profile {
        parse_profile_file(&backup_data).map_err(|err| {
            CoreError::Validation(format!(
                "backup is not a valid PersistentDataList profile file: {err}"
            ))
        })?;
    }

    let original = fs::read(path)?;
    inspect_bytes(&original, Some(path), false)?;

    // inspect_bytes accepts both GSAV and GVAS containers, so require the backup
    // to share the slot's container format. Otherwise a misnamed GVAS sidecar
    // backup could pass validation and replace the GSAV slot with an unusable
    // file.
    if save_container_magic(&backup_data) != save_container_magic(&original) {
        return Err(CoreError::Validation(
            "backup container format does not match the selected save".to_string(),
        ));
    }

    // Discover and validate the paired companion rollback *before* mutating the
    // slot file, so a companion failure aborts the whole restore instead of
    // leaving the slot restored while PersistentDataList.sav stays out of sync.
    let companion_plan = prepare_paired_persistent_data_list_restore(path, backup_path)?;

    // Take safety backups of both files up front. For a profile-file restore,
    // the safety backup must avoid existing slot-backup suffixes too — otherwise
    // it could land on a slot's suffix and be wrongly paired as that slot's
    // companion on a later slot restore (same hazard as the write path).
    let current_backup_path = if target_is_profile {
        create_unique_backup_avoiding(path, &existing_foreign_backup_suffixes(path))?
    } else {
        create_backup_copy(path)?
    };
    let companion_safety_backup = match &companion_plan {
        Some(plan) => Some(create_backup_copy(&plan.persistent_path)?),
        None => None,
    };

    // Stage both writes to temp files and validate before committing either.
    let slot_tmp = path.with_extension("sav.tmp-goresave-restore");
    fs::write(&slot_tmp, &backup_data)?;
    inspect_save(&slot_tmp, false)?;
    let companion_tmp = match &companion_plan {
        Some(plan) => {
            let tmp = plan
                .persistent_path
                .with_extension("sav.tmp-goresave-restore");
            fs::write(&tmp, &plan.companion_data)?;
            Some(tmp)
        }
        None => None,
    };

    // Commit: both files are validated and staged. Replace the slot first, then
    // the companion; if the companion replace fails, roll the slot back so they
    // never end up restored to different edits.
    let slot_pending = begin_replace(path, &slot_tmp)?;
    if let (Some(plan), Some(tmp)) = (&companion_plan, &companion_tmp) {
        match begin_replace(&plan.persistent_path, tmp) {
            Ok(companion_pending) => {
                companion_pending.commit();
                slot_pending.commit();
            }
            Err(err) => {
                slot_pending.rollback();
                return Err(err);
            }
        }
    } else {
        slot_pending.commit();
    }

    Ok(json!({
        "path": path,
        "restoredFrom": backup_path,
        "backupPath": current_backup_path,
        "previousSha1": sha1_hex(&original),
        "restoredSha1": sha1_hex(&backup_data),
        "bytesChanged": original != backup_data,
        "persistentPath": companion_plan.as_ref().map(|p| p.persistent_path.display().to_string()),
        "persistentRestoredFrom": companion_plan
            .as_ref()
            .map(|p| p.companion_backup_path.display().to_string()),
        "persistentBackupPath": companion_safety_backup
            .as_ref()
            .map(|p| p.display().to_string()),
        "persistentBytesChanged": companion_plan.is_some(),
        // Whether a PersistentDataList.sav sits next to the save. When this is
        // true but persistentRestoredFrom is null, no paired companion backup
        // matched the slot backup, so the list was left unchanged and the caller
        // can warn that slot/list metadata may now diverge.
        "persistentCompanionPresent": path
            .parent()
            .map(|parent| parent.join("PersistentDataList.sav").exists())
            .unwrap_or(false),
    }))
}

struct CompanionRestorePlan {
    persistent_path: PathBuf,
    companion_backup_path: PathBuf,
    companion_data: Vec<u8>,
}

/// Locate and validate the paired `PersistentDataList.sav` backup that
/// `syncPersistentDataList` created in the same write as `slot_backup_path`.
/// Backups are paired by the full suffix after each file's `.bak.` prefix (e.g.
/// `123` or `123.1`), so two edits within the same second still match the right
/// companion. This performs no mutations: it only reads and validates so the
/// caller can abort the restore before touching any file. Returns `None` when
/// there is no companion to roll back (no PersistentDataList.sav, no matching
/// backup, or already identical).
fn prepare_paired_persistent_data_list_restore(
    save_path: &Path,
    slot_backup_path: &Path,
) -> Result<Option<CompanionRestorePlan>, CoreError> {
    // When the restore target IS PersistentDataList.sav (restoring a profile
    // difficulty backup directly), there is no separate companion: pairing here
    // would treat the file as its own companion and try to replace it twice.
    if save_path.file_name().and_then(|n| n.to_str()) == Some("PersistentDataList.sav") {
        return Ok(None);
    }
    let Some(parent) = save_path.parent() else {
        return Ok(None);
    };
    let persistent_path = parent.join("PersistentDataList.sav");
    if !persistent_path.exists() {
        return Ok(None);
    }
    let slot_prefix = backup_file_prefix(save_path)?;
    let slot_backup_name = slot_backup_path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| CoreError::InvalidRequest("backup path has no file name".to_string()))?;
    // Full suffix after the slot prefix, e.g. "123" or "123.1". Matching the
    // whole suffix (not just the epoch) keeps same-second paired backups from
    // rolling the slot and companion back to different edits.
    let Some(slot_suffix) = slot_backup_name.strip_prefix(&slot_prefix) else {
        return Ok(None);
    };

    let companion_prefix = backup_file_prefix(&persistent_path)?;
    let mut companion_backup_path = None;
    // Paired backups share one suffix AND one directory (they are written in
    // the same round). Search the selected slot backup's own directory first,
    // so a companion in the other location with a colliding suffix cannot
    // pair the slot and companion to different write rounds.
    let search_dirs: Vec<PathBuf> = {
        let mut dirs = Vec::new();
        if let Some(backup_dir) = slot_backup_path.parent() {
            dirs.push(backup_dir.to_path_buf());
        }
        for fallback in [parent.to_path_buf(), parent.join("goresave_backups")] {
            if !dirs.contains(&fallback) {
                dirs.push(fallback);
            }
        }
        dirs
    };
    'outer: for search_dir in &search_dirs {
        if !search_dir.is_dir() {
            continue;
        }
        for entry in fs::read_dir(search_dir)? {
            let entry = entry?;
            let candidate = entry.path();
            let Some(name) = candidate.file_name().and_then(|value| value.to_str()) else {
                continue;
            };
            if name.strip_prefix(&companion_prefix) == Some(slot_suffix) {
                companion_backup_path = Some(candidate);
                break 'outer;
            }
        }
    }
    let Some(companion_backup_path) = companion_backup_path else {
        return Ok(None);
    };

    let companion_data = fs::read(&companion_backup_path)?;
    if !companion_data.starts_with(b"GVAS") {
        return Err(CoreError::Parse(
            "PersistentDataList backup is not a GVAS file".to_string(),
        ));
    }
    let current = fs::read(&persistent_path)?;
    if current == companion_data {
        return Ok(None);
    }

    Ok(Some(CompanionRestorePlan {
        persistent_path,
        companion_backup_path,
        companion_data,
    }))
}

/// Leading container magic ("GSAV" / "GVAS") used to ensure a backup matches
/// the slot's format before a restore replaces it.
fn save_container_magic(data: &[u8]) -> &[u8] {
    &data[..data.len().min(4)]
}

fn ensure_backup_belongs_to_save(path: &Path, backup_path: &Path) -> Result<(), CoreError> {
    let prefix = backup_file_prefix(path)?;
    let file_name = backup_path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| CoreError::InvalidRequest("backup path has no file name".to_string()))?;
    if !file_name.starts_with(&prefix) {
        return Err(CoreError::InvalidRequest(
            "backupPath does not match the selected save file".to_string(),
        ));
    }
    let save_parent = path.parent().unwrap_or_else(|| Path::new("."));
    let backup_parent = backup_path.parent().unwrap_or_else(|| Path::new("."));
    let canonical_save_parent = fs::canonicalize(save_parent)?;
    let canonical_backup_parent = fs::canonicalize(backup_parent)?;
    // Accept backups both in the save's parent directory (legacy) and in
    // its goresave_backups subfolder (new layout).
    let subfolder = canonical_save_parent.join("goresave_backups");
    if canonical_backup_parent != canonical_save_parent && canonical_backup_parent != subfolder {
        return Err(CoreError::InvalidRequest(
            "backupPath must be next to the selected save file or in its goresave_backups subfolder".to_string(),
        ));
    }
    Ok(())
}

/// An applied-but-not-finalized file replacement. Hold one per file so a
/// multi-file commit can be rolled back if a later file fails, keeping the slot
/// file and PersistentDataList.sav consistent.
struct PendingReplace {
    target: PathBuf,
    /// The moved-aside previous file, kept until [`PendingReplace::commit`]. If
    /// `None`, the target was newly created and rollback just removes it.
    aside: Option<PathBuf>,
}

impl PendingReplace {
    /// Finalize the replacement by discarding the moved-aside original.
    fn commit(self) {
        if let Some(aside) = self.aside {
            let _ = fs::remove_file(aside);
        }
    }

    /// Undo the replacement, restoring the original file (or removing a
    /// newly-created target) so the path returns to its pre-commit contents.
    fn rollback(self) {
        match self.aside {
            Some(aside) => {
                let _ = fs::remove_file(&self.target);
                let _ = fs::rename(&aside, &self.target);
            }
            None => {
                let _ = fs::remove_file(&self.target);
            }
        }
    }
}

/// Map an I/O error that occurred while renaming or replacing a live save file
/// into a human-readable [`CoreError`] when the error indicates that another
/// process holds the file open (Windows sharing/lock violation).
///
/// `context` is appended to the message for disambiguation (e.g. the file path
/// or operation description). On non-Windows platforms or for unrelated errors
/// the original error is wrapped unchanged.
fn map_locked_file_error(err: std::io::Error, context: &str) -> CoreError {
    let is_locked = match err.raw_os_error() {
        // ERROR_SHARING_VIOLATION = 32, ERROR_LOCK_VIOLATION = 33
        Some(32) | Some(33) => true,
        _ => err.kind() == std::io::ErrorKind::PermissionDenied,
    };
    if is_locked {
        CoreError::Io(format!(
            "the save file is locked by another process \
             (is the game running?) — close the game or its load screen, \
             then retry: {err} ({context})"
        ))
    } else {
        CoreError::Io(err.to_string())
    }
}

/// Replace `target` with the staged file at `staged` without ever leaving
/// `target` missing on failure. Windows `rename` cannot overwrite, so the
/// current file is moved aside first; if renaming the staged file in fails, the
/// aside copy is moved back so the slot is never lost. The returned
/// [`PendingReplace`] must be either committed or rolled back.
fn begin_replace(target: &Path, staged: &Path) -> Result<PendingReplace, CoreError> {
    if !target.exists() {
        fs::rename(staged, target)
            .map_err(|e| map_locked_file_error(e, &target.display().to_string()))?;
        return Ok(PendingReplace {
            target: target.to_path_buf(),
            aside: None,
        });
    }
    let aside = target.with_extension("sav.replaced-goresave");
    // Clear any leftover aside from a previously interrupted write.
    let _ = fs::remove_file(&aside);
    fs::rename(target, &aside)
        .map_err(|e| map_locked_file_error(e, &target.display().to_string()))?;
    match fs::rename(staged, target) {
        Ok(()) => Ok(PendingReplace {
            target: target.to_path_buf(),
            aside: Some(aside),
        }),
        Err(err) => {
            // Roll back so the target path is never left absent.
            let _ = fs::rename(&aside, target);
            Err(map_locked_file_error(err, &target.display().to_string()))
        }
    }
}

fn create_backup_copy(path: &Path) -> Result<PathBuf, CoreError> {
    let backup_path = unique_backup_path(path);
    if let Some(parent) = backup_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::copy(path, &backup_path)?;
    Ok(backup_path)
}

/// Back up `path` with a suffix that is free on disk for this file AND not in
/// `avoid`. Used when backing up several independent files in one round: each
/// file gets a distinct suffix so the paired-restore heuristic
/// ([`prepare_paired_persistent_data_list_restore`]) never auto-couples them,
/// even when their names differ (a per-file existence check alone would not
/// stop two differently-named files from sharing the same timestamp suffix).
fn create_unique_backup_avoiding(path: &Path, avoid: &[String]) -> Result<PathBuf, CoreError> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let mut suffix = format!("{timestamp}");
    let mut attempt = 0u32;
    while avoid.iter().any(|s| s == &suffix) || backup_path_with_suffix(path, &suffix).exists() {
        attempt += 1;
        suffix = format!("{timestamp}.{attempt}");
        if attempt >= 1_000_000 {
            suffix = format!("{timestamp}.overflow");
            break;
        }
    }
    create_backup_with_suffix(path, &suffix)
}

fn unique_backup_path(path: &Path) -> PathBuf {
    let suffix = shared_backup_suffix(std::slice::from_ref(&path));
    backup_path_with_suffix(path, &suffix)
}

/// Suffixes of backups in `path`'s backup locations that belong to a DIFFERENT
/// save file (e.g. slot backups when `path` is PersistentDataList.sav). A
/// standalone profile backup must avoid these so the suffix-only pairing in
/// [`prepare_paired_persistent_data_list_restore`] cannot later mistake it for
/// an unrelated slot's companion and roll the profile back on a slot restore.
fn existing_foreign_backup_suffixes(path: &Path) -> Vec<String> {
    let mut suffixes = Vec::new();
    let Some(parent) = path.parent() else {
        return suffixes;
    };
    let own_prefix = backup_file_prefix(path).ok();
    for dir in [parent.to_path_buf(), parent.join("goresave_backups")] {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
                continue;
            };
            // Skip our own file's backups; only foreign suffixes can collide.
            if let Some(own) = &own_prefix {
                if name.starts_with(own) {
                    continue;
                }
            }
            if let Some(idx) = name.rfind(".bak.") {
                if name[..idx].ends_with(".sav") {
                    suffixes.push(name[idx + ".bak.".len()..].to_string());
                }
            }
        }
    }
    suffixes
}

fn backup_path_with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let file_name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    parent
        .join("goresave_backups")
        .join(format!("{file_name}.bak.{suffix}"))
}

fn create_backup_with_suffix(path: &Path, suffix: &str) -> Result<PathBuf, CoreError> {
    let backup_path = backup_path_with_suffix(path, suffix);
    if let Some(parent) = backup_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::copy(path, &backup_path)?;
    Ok(backup_path)
}

/// Pick a single `.bak` suffix that is free for every target, so paired backups
/// (slot + companion PersistentDataList) share one suffix and restore can match
/// them even when their creation straddles a one-second boundary.
fn shared_backup_suffix(targets: &[&Path]) -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    for attempt in 0..1000 {
        let suffix = if attempt == 0 {
            format!("{timestamp}")
        } else {
            format!("{timestamp}.{attempt}")
        };
        if targets
            .iter()
            .all(|target| !backup_path_with_suffix(target, &suffix).exists())
        {
            return suffix;
        }
    }
    format!("{timestamp}.overflow")
}

/// Default save directory used when a caller omits `payload.path`. Derived from
/// the running user's environment rather than a hardcoded developer profile.
fn default_save_root() -> PathBuf {
    default_save_root_from(
        std::env::var_os("LOCALAPPDATA"),
        std::env::var_os("USERPROFILE"),
    )
}

fn default_save_root_from(
    local_app_data: Option<std::ffi::OsString>,
    user_profile: Option<std::ffi::OsString>,
) -> PathBuf {
    let suffix = ["G1R", "Saved", "SaveGames"];
    if let Some(local_app_data) = local_app_data {
        if !local_app_data.is_empty() {
            return suffix
                .iter()
                .fold(PathBuf::from(local_app_data), |p, c| p.join(c));
        }
    }
    if let Some(user_profile) = user_profile {
        if !user_profile.is_empty() {
            let base = PathBuf::from(user_profile).join("AppData").join("Local");
            return suffix.iter().fold(base, |p, c| p.join(c));
        }
    }
    suffix.iter().fold(PathBuf::new(), |p, c| p.join(c))
}

fn backup_file_prefix(path: &Path) -> Result<String, CoreError> {
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| CoreError::InvalidRequest("save path has no file name".to_string()))?;
    Ok(format!("{file_name}.bak."))
}

fn parse_backup_epoch(file_name: &str, prefix: &str) -> Option<u64> {
    file_name
        .strip_prefix(prefix)?
        .split('.')
        .next()
        .and_then(|value| value.parse::<u64>().ok())
}

fn inspect_save(path: &Path, include_private: bool) -> Result<Value, CoreError> {
    inspect_save_with_codec_backend(path, include_private, None, None)
}

fn inspect_save_with_codec_backend(
    path: &Path,
    include_private: bool,
    codec_backend: Option<&dyn codec_backend::CodecBackend>,
    private_chunk_limit: Option<usize>,
) -> Result<Value, CoreError> {
    let data = fs::read(path)?;
    inspect_bytes_with_codec_backend(
        &data,
        Some(path),
        include_private,
        codec_backend,
        private_chunk_limit,
    )
}

fn inspect_bytes(
    data: &[u8],
    path: Option<&Path>,
    include_private: bool,
) -> Result<Value, CoreError> {
    inspect_bytes_with_codec_backend(data, path, include_private, None, None)
}

fn inspect_bytes_with_codec_backend(
    data: &[u8],
    path: Option<&Path>,
    include_private: bool,
    codec_backend: Option<&dyn codec_backend::CodecBackend>,
    private_chunk_limit: Option<usize>,
) -> Result<Value, CoreError> {
    if data.starts_with(b"GSAV") {
        let info = parse_gsav(data, path)?;
        let mut value = serde_json::to_value(info).map_err(|e| CoreError::Parse(e.to_string()))?;
        if include_private {
            let stream_offset = value
                .get("publicPayloadOffset")
                .and_then(Value::as_u64)
                .unwrap_or(13) as usize
                + value
                    .get("publicPayloadSize")
                    .and_then(Value::as_u64)
                    .unwrap_or(0) as usize;
            let stream = parse_compressed_stream(data, stream_offset)?;
            value["private"] =
                inspect_private_payload(data, path, &stream, codec_backend, private_chunk_limit)?;
        }
        if let Some(metadata) = path.and_then(persistent_slot_metadata_for_save) {
            value["persistent"] =
                serde_json::to_value(metadata).map_err(|e| CoreError::Parse(e.to_string()))?;
        }
        value["difficulty"] = serde_json::to_value(difficulty_for_gsav_bytes(data))
            .map_err(|e| CoreError::Parse(e.to_string()))?;
        Ok(value)
    } else if data.starts_with(b"GVAS") {
        Ok(parse_gvas(data, path))
    } else {
        Err(CoreError::Parse("unknown save container magic".to_string()))
    }
}

fn parse_gsav(data: &[u8], path: Option<&Path>) -> Result<GsavInfo, CoreError> {
    if data.len() < 13 {
        return Err(CoreError::Parse(
            "GSAV file is shorter than header".to_string(),
        ));
    }
    let version = data[4];
    let body_size = u32::from_le_bytes(data[5..9].try_into().unwrap());
    let public_payload_size = u32::from_le_bytes(data[9..13].try_into().unwrap()) as usize;
    let public_start = 13usize;
    let public_end = public_start
        .checked_add(public_payload_size)
        .ok_or_else(|| CoreError::Parse("public payload size overflow".to_string()))?;
    if public_end > data.len() {
        return Err(CoreError::Parse(
            "public payload extends past EOF".to_string(),
        ));
    }
    let stream = parse_compressed_stream(data, public_end)?;
    let public_payload = &data[public_start..public_end];
    let public = summarize_public_payload(public_payload);
    let path_string = path.map(|p| p.display().to_string());
    let slot = path
        .and_then(|p| p.file_stem())
        .and_then(|s| s.to_str())
        .map(ToOwned::to_owned);
    Ok(GsavInfo {
        format: "GSAV",
        path: path_string,
        slot,
        size: data.len(),
        sha1: sha1_hex(data),
        version_byte: version,
        body_size_field: body_size,
        body_size_delta: data.len() as isize - body_size as isize,
        public_payload_offset: public_start,
        public_payload_size,
        public_payload_sha1: sha1_hex(public_payload),
        public,
        compressed_stream: CompressedStreamSummary {
            method: stream.method.clone(),
            algorithm_id: stream.algorithm_id,
            chunk_count: stream.chunk_count,
            compressed_size: stream.summary_compressed_size,
            uncompressed_size: stream.summary_uncompressed_size,
            trailing_size: stream.trailing_size,
        },
        trailer_offset: stream.stream_end_offset,
        trailer_size: data.len() - stream.stream_end_offset,
    })
}

fn parse_compressed_stream(data: &[u8], offset: usize) -> Result<CompressedStream, CoreError> {
    let mut r = Reader::new(
        data.get(offset..)
            .ok_or_else(|| CoreError::Parse("compressed stream offset past EOF".to_string()))?,
        offset,
    );
    let uncompressed_size_prefix = r.u64()?;
    let method = r.fstring()?;
    let tag = r.u32()?;
    if tag != PACKAGE_FILE_TAG {
        return Err(CoreError::Parse(format!(
            "expected PACKAGE_FILE_TAG at 0x{:x}, got 0x{tag:08x}",
            r.abs_pos() - 4
        )));
    }
    let header_version = r.u32()?;
    let (max_chunk_size, algorithm_id, summary_compressed_size, summary_uncompressed_size) =
        if header_version == COMPRESSED_HEADER_V2 {
            (r.u64()?, Some(r.u8()?), r.u64()?, r.u64()?)
        } else if header_version == 0 {
            (r.u32()? as u64, None, r.u32()? as u64, r.u32()? as u64)
        } else {
            return Err(CoreError::Parse(format!(
                "unsupported compressed header version 0x{header_version:08x}"
            )));
        };
    if summary_uncompressed_size != uncompressed_size_prefix {
        return Err(CoreError::Parse(format!(
            "compressed stream size mismatch: prefix={uncompressed_size_prefix}, summary={summary_uncompressed_size}"
        )));
    }
    if max_chunk_size == 0 {
        return Err(CoreError::Parse("max chunk size is zero".to_string()));
    }
    let chunk_count = if summary_uncompressed_size == 0 {
        0
    } else {
        summary_uncompressed_size.div_ceil(max_chunk_size) as usize
    };
    // Each chunk-table entry is two u64s (16 bytes). A corrupt header with a
    // huge summary size and small max chunk size yields an enormous chunk_count;
    // reject any count that cannot fit in the remaining bytes before allocating,
    // so one malformed save can't OOM/panic across the FFI boundary.
    let remaining = data.len().saturating_sub(r.abs_pos());
    let max_possible_chunks = remaining / 16;
    if chunk_count > max_possible_chunks {
        return Err(CoreError::Parse(format!(
            "compressed chunk table declares {chunk_count} chunks but only {max_possible_chunks} fit in the remaining {remaining} bytes"
        )));
    }
    // Fallible reservation: even with the bound above, never panic/abort across
    // the FFI boundary on a large count — surface a parse error instead.
    let mut chunks: Vec<CompressedChunk> = Vec::new();
    chunks
        .try_reserve(chunk_count)
        .map_err(|_| CoreError::Parse(format!("cannot reserve space for {chunk_count} chunks")))?;
    for index in 0..chunk_count {
        let compressed_size = r.u64()?;
        let uncompressed_size = r.u64()?;
        if uncompressed_size > max_chunk_size {
            return Err(CoreError::Parse(format!(
                "chunk {index} uncompressed size {uncompressed_size} exceeds max chunk size {max_chunk_size}"
            )));
        }
        chunks.push(CompressedChunk {
            index,
            compressed_size,
            uncompressed_size,
            compressed_offset: 0,
        });
    }
    let payload_offset = r.abs_pos();
    let mut cursor = payload_offset;
    for chunk in chunks.iter_mut() {
        chunk.compressed_offset = cursor;
        cursor = cursor
            .checked_add(chunk.compressed_size as usize)
            .ok_or_else(|| CoreError::Parse("compressed chunk cursor overflow".to_string()))?;
    }
    let compressed_sum = chunks.iter().map(|c| c.compressed_size).sum::<u64>();
    let uncompressed_sum = chunks.iter().map(|c| c.uncompressed_size).sum::<u64>();
    if compressed_sum != summary_compressed_size {
        return Err(CoreError::Parse(format!(
            "compressed stream size mismatch: table sum={compressed_sum}, summary={summary_compressed_size}"
        )));
    }
    if uncompressed_sum != summary_uncompressed_size {
        return Err(CoreError::Parse(format!(
            "uncompressed stream size mismatch: table sum={uncompressed_sum}, summary={summary_uncompressed_size}"
        )));
    }
    if cursor > data.len() {
        return Err(CoreError::Parse(format!(
            "compressed payload runs past EOF: 0x{cursor:x} > 0x{:x}",
            data.len()
        )));
    }
    Ok(CompressedStream {
        stream_offset: offset,
        uncompressed_size_prefix,
        method,
        package_tag: tag,
        header_version,
        max_chunk_size,
        algorithm_id,
        summary_compressed_size,
        summary_uncompressed_size,
        chunk_count,
        compressed_payload_offset: payload_offset,
        compressed_payload_size: summary_compressed_size,
        stream_end_offset: cursor,
        trailing_size: data.len() - cursor,
        chunks,
    })
}

fn parse_gvas(data: &[u8], path: Option<&Path>) -> Value {
    let script_paths = extract_script_paths(data);
    json!({
        "format": "GVAS",
        "path": path.map(|p| p.display().to_string()),
        "slot": path.and_then(|p| p.file_stem()).and_then(|s| s.to_str()),
        "size": data.len(),
        "sha1": sha1_hex(data),
        "scriptPaths": script_paths,
        "status": "read_only"
    })
}

fn summarize_public_payload(payload: &[u8]) -> PublicSummary {
    let refs = scan_fstrings(payload, 0);
    let strings = refs
        .iter()
        .map(|r| r.value.clone())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>();
    let slot_name = value_after_property(&refs, "m_SlotName");
    let player_save_name = value_after_property(&refs, "m_PlayerSaveName");
    let profile_id = read_i32_after_property(payload, &refs, "m_ProfileId");
    let object_paths = strings
        .iter()
        .filter(|s| s.starts_with("/Script/"))
        .cloned()
        .collect::<Vec<_>>();
    PublicSummary {
        slot_name,
        player_save_name,
        profile_id,
        object_paths,
        strings: strings.into_iter().take(80).collect(),
        editable: vec!["public.m_PlayerSaveName".to_string()],
    }
}

fn value_after_property(refs: &[FStringRef], name: &str) -> Option<String> {
    refs.iter().position(|r| r.value == name).and_then(|idx| {
        refs.iter()
            .skip(idx + 1)
            .find(|r| {
                !matches!(
                    r.value.as_str(),
                    "StrProperty"
                        | "NameProperty"
                        | "ObjectProperty"
                        | "BoolProperty"
                        | "IntProperty"
                        | "MapProperty"
                        | "ArrayProperty"
                        | "StructProperty"
                )
            })
            .map(|r| r.value.clone())
    })
}

fn value_after_property_in_range(
    refs: &[FStringRef],
    start_idx: usize,
    end_idx: usize,
    name: &str,
) -> Option<String> {
    let name_idx = find_ref_in_range(refs, start_idx, end_idx, name)?;
    refs.iter()
        .take(end_idx)
        .skip(name_idx + 1)
        .find(|r| !is_property_type_name(r.value.as_str()))
        .map(|r| r.value.clone())
}

fn short_class_name(path: &str) -> String {
    path.rsplit('.').next().unwrap_or(path).to_string()
}

pub fn parse_difficulty_settings(payload: &[u8]) -> DifficultySettings {
    let refs = scan_fstrings(payload, 0);
    let end = refs.len();
    let class = |name: &str| {
        value_after_property_in_range(&refs, 0, end, name).map(|v| short_class_name(&v))
    };
    DifficultySettings {
        preset: class("m_difficultyPreset"),
        combat: class("m_customCombatSettings"),
        resources: class("m_customResourcesSettings"),
        progression: class("m_customProgressionSettings"),
        flow_helper: read_bool_property_in_range(payload, &refs, 0, end, "m_FakeSloppyCombos"),
        permadeath: PERMADEATH_NAMES
            .iter()
            .find_map(|n| read_bool_property_in_range(payload, &refs, 0, end, n)),
    }
}

fn difficulty_for_gsav_bytes(data: &[u8]) -> Option<DifficultySettings> {
    if !data.starts_with(b"GSAV") {
        return None;
    }
    let parts = split_gsav(data).ok()?;
    Some(parse_difficulty_settings(parts.public_payload))
}

const ANGELSCRIPT: &str = "/Script/Angelscript.";

fn level_suffix(label: &str) -> Result<&'static str, CoreError> {
    match label {
        "Novice" => Ok("Easy"),
        "Gothic" => Ok("Standard"),
        "Hard" => Ok("Hard"),
        other => Err(CoreError::InvalidRequest(format!(
            "unknown difficulty level {other}"
        ))),
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DifficultyRequest {
    /// The UI preset to write (Novice|Gothic|Hard|Custom), or `None` to leave the
    /// stored preset and sub-settings untouched and write only the bool toggles.
    /// This lets a profile whose stored preset class is unrecognised (e.g. a
    /// `/Game/...` path or a new game-update preset) still take a flow-helper /
    /// permadeath edit without forcing a preset rewrite.
    #[serde(default)]
    preset: Option<String>,
    #[serde(default)]
    combat: Option<String>,
    #[serde(default)]
    resources: Option<String>,
    #[serde(default)]
    progression: Option<String>,
    #[serde(default)]
    flow_helper: Option<bool>,
    #[serde(default)]
    permadeath: Option<bool>,
}

impl DifficultyRequest {
    /// Resolved short class names (no package prefix). Empty when no `preset` was
    /// requested — a bool-only edit leaves the preset/sub-setting asset paths
    /// exactly as stored.
    ///
    /// For Custom, each sub-setting is rewritten ONLY when the request supplies
    /// it; an omitted Custom sub-level leaves the stored class untouched (so an
    /// unrecognised stored sub-setting is preserved on a bool-only or
    /// partial-Custom save). For Novice/Gothic/Hard, all three sub-settings are
    /// mirrored to the preset level (the in-game behaviour).
    fn class_edits(&self) -> Result<Vec<(&'static str, String)>, CoreError> {
        let Some(preset_label) = self.preset.as_deref() else {
            return Ok(Vec::new());
        };
        if preset_label == "Custom" {
            let mut edits = vec![("m_difficultyPreset", "DifficultyPreset_Custom".to_string())];
            if let Some(combat) = &self.combat {
                edits.push((
                    "m_customCombatSettings",
                    format!("CombatDifficultySettings_{}", level_suffix(combat)?),
                ));
            }
            if let Some(resources) = &self.resources {
                edits.push((
                    "m_customResourcesSettings",
                    format!("ResourcesDifficultySettings_{}", level_suffix(resources)?),
                ));
            }
            if let Some(progression) = &self.progression {
                edits.push((
                    "m_customProgressionSettings",
                    format!(
                        "ProgressionDifficultySettings_{}",
                        level_suffix(progression)?
                    ),
                ));
            }
            Ok(edits)
        } else {
            let suffix = level_suffix(preset_label)?;
            Ok(vec![
                ("m_difficultyPreset", format!("DifficultyPreset_{suffix}")),
                (
                    "m_customCombatSettings",
                    format!("CombatDifficultySettings_{suffix}"),
                ),
                (
                    "m_customResourcesSettings",
                    format!("ResourcesDifficultySettings_{suffix}"),
                ),
                (
                    "m_customProgressionSettings",
                    format!("ProgressionDifficultySettings_{suffix}"),
                ),
            ])
        }
    }

    /// Permadeath is locked off for Novice. With no preset requested the stored
    /// permadeath toggle is honoured as-is.
    fn resolved_permadeath(&self) -> Option<bool> {
        if self.preset.as_deref() == Some("Novice") {
            Some(false)
        } else {
            self.permadeath
        }
    }
}

/// Properties of a single profile element inside `m_Profiles`, regardless of
/// whether the array element is a plain struct or an InstancedStruct wrapper.
fn profile_element_properties(
    value: &properties::PropertyValue,
) -> Option<&[properties::Property]> {
    match value {
        properties::PropertyValue::Struct(properties::StructValue::Properties(p)) => Some(p),
        properties::PropertyValue::Struct(properties::StructValue::Instanced(Some(i))) => {
            Some(&i.properties)
        }
        _ => None,
    }
}

/// Locate the `m_Profiles` array element whose `m_ProfileId` equals
/// `profile_id`, returning the addressable path prefix to that element
/// (`["m_Profiles", "[i]"]`) together with the element's property list. An
/// element without an `m_ProfileId` falls back to its array index as its id,
/// matching how the listing path exposes such profiles.
///
/// `m_Profiles` is an ArrayProperty of profile structs in the real
/// PersistentDataList; only that shape is supported. Returns `None` when no
/// matching profile exists so the caller can raise a clear error.
fn profile_element<'a>(
    root: &'a properties::RootObject,
    profile_id: i32,
) -> Option<(Vec<String>, &'a [properties::Property])> {
    let (profiles_path, profiles) = properties::find_property_by_name(root, "m_Profiles")?;
    let properties::PropertyValue::Array { elements } = &profiles.value else {
        return None;
    };
    for (i, element) in elements.iter().enumerate() {
        let Some(props) = profile_element_properties(element) else {
            continue;
        };
        // Mirror the listing path (`read_i32_property_in_range(...).unwrap_or(ordinal)`):
        // a profile that omits `m_ProfileId` is exposed under its array index, so we
        // must accept that same index here or the write fails as "profile not found".
        let id = props
            .iter()
            .find_map(|p| match (&p.name[..], &p.value) {
                ("m_ProfileId", properties::PropertyValue::Int(v)) => Some(*v),
                _ => None,
            })
            .unwrap_or(i as i32);
        if id == profile_id {
            let mut prefix = profiles_path.clone();
            prefix.push(format!("[{i}]"));
            return Some((prefix, props));
        }
    }
    None
}

/// Resolve the full addressable path to a difficulty property `name` WITHIN the
/// profile whose id is `profile_id`. Scoped to that profile element so a field
/// in another profile is never matched. Returns `Ok(None)` when the field is
/// absent in that profile (caller skips it); errs only when the profile itself
/// is missing.
fn profile_difficulty_path(
    root: &properties::RootObject,
    profile_id: i32,
    name: &str,
) -> Result<Option<Vec<String>>, CoreError> {
    let (prefix, element_props) = profile_element(root, profile_id)
        .ok_or_else(|| CoreError::Validation(format!("profile {profile_id} not found")))?;
    let Some((rel, _)) = properties::find_path_in_properties(element_props, name) else {
        return Ok(None);
    };
    let mut full = prefix;
    full.extend(rel);
    Ok(Some(full))
}

/// Parse the GVAS save object inside a PersistentDataList file, returning a tree
/// whose offsets are ABSOLUTE within `data` so it patches the file directly.
///
/// The file starts with a variable-length GVAS header (save-game/package
/// versions + a custom-version array of unpredictable length + the save-game
/// class name) before the object body. Rather than parse every header version
/// exactly, probe for the offset at which the body parses AND consumes the rest
/// of the file — version-agnostic and deterministic, bounded to the header
/// region (class names appear within a few KB) so it stays linear in practice.
///
/// Two body framings are accepted, mirroring `parse_save_tree`:
///   * a nested `class` + flag + props + footer object — `parse_private_root_at`;
///   * a bare property list that follows the header's class name directly, with
///     no nested object framing — `parse_property_list_root_at`. This is the
///     shape of a standard GVAS save-game file (the common PersistentDataList
///     layout), where the object-only probe would otherwise hard-fail.
fn parse_profile_file(data: &[u8]) -> Result<properties::RootObject, CoreError> {
    // A candidate offset is the real profile root only if it consumes the whole
    // file AND carries the top-level `m_Profiles` array. Without the second
    // check the trailing `None` terminator alone parses as an empty property
    // list that also consumes to EOF, so a length-changing edit that left an
    // enclosing size stale (real root parse fails) could still be accepted here
    // — and this helper IS write_profile_difficulty's strict post-edit gate.
    let is_profile_root = |root: &properties::RootObject| {
        root.consumed == data.len() && root.properties.iter().any(|p| p.name == "m_Profiles")
    };
    let limit = data.len().min(8192);
    for off in 0..limit {
        // Object framing (class + flag + props + footer): read_object reads the
        // class FString AT `off`, so the candidate start is self-validated.
        if let Ok(root) = properties::parse_private_root_at(data, off) {
            if is_profile_root(&root) {
                return Ok(root);
            }
        }
        // Bare-list framing: the property list follows the header's save-game
        // class name directly. Require a valid class-name FString to end exactly
        // at `off` so the skipped prefix is a real header — not arbitrary or
        // truncated bytes the scan would otherwise accept by starting the parse
        // straight at `m_Profiles` and treating a corrupt prefix as the header.
        if class_name_fstring_ends_at(data, off) {
            if let Ok(root) = properties::parse_property_list_root_at(data, off) {
                if is_profile_root(&root) {
                    return Ok(root);
                }
            }
        }
    }
    Err(CoreError::Parse(
        "could not locate the GVAS save object in the file".into(),
    ))
}

/// True when a valid ASCII FString — the save-game class name that terminates a
/// GVAS header — ends exactly at byte `off`. Used to confirm the bytes skipped
/// before a bare property-list parse are a real header tail rather than
/// arbitrary data the offset scan happened to skip over.
fn class_name_fstring_ends_at(data: &[u8], off: usize) -> bool {
    // FString layout: i32 length (INCLUDING the trailing NUL) + bytes. Search a
    // bounded window for a length prefix `s` whose string terminates at `off`.
    for content_len in 2..=512usize {
        let total = 4 + content_len;
        if total > off {
            break;
        }
        let s = off - total;
        // Keep the prefix after the 4-byte GVAS magic.
        if s < 4 {
            continue;
        }
        let declared = i32::from_le_bytes(match data[s..s + 4].try_into() {
            Ok(bytes) => bytes,
            Err(_) => continue,
        });
        if declared <= 0 || declared as usize != content_len {
            continue;
        }
        let content = &data[s + 4..off];
        if *content.last().unwrap() != 0 {
            continue;
        }
        // A class name is a printable path (e.g. "/Script/G1R.PersistentDataList").
        if content[..content_len - 1]
            .iter()
            .all(|&b| b.is_ascii_graphic() || b == b' ')
        {
            return true;
        }
    }
    false
}

fn write_profile_difficulty(
    data: &mut Vec<u8>,
    profile_id: i32,
    req: &DifficultyRequest,
) -> Result<(), CoreError> {
    if !data.starts_with(b"GVAS") {
        return Err(CoreError::Parse(
            "PersistentDataList.sav is not a GVAS file".into(),
        ));
    }
    // Verify the profile exists up front so a missing-profile request errors
    // (rather than silently skipping every field), mirroring the old behavior.
    {
        let root = parse_profile_file(data)?;
        profile_element(&root, profile_id)
            .ok_or_else(|| CoreError::Validation(format!("profile {profile_id} not found")))?;
    }

    // Asset-path sub-settings: typed length-changing patch scoped to the named
    // profile, re-parsing between every edit (offsets shift after each splice).
    for (name, class) in req.class_edits()? {
        patch_profile_difficulty_string(data, profile_id, name, &format!("{ANGELSCRIPT}{class}"))?;
    }
    // Permadeath under whichever spelling exists in THIS profile.
    if let Some(perma) = req.resolved_permadeath() {
        for name in PERMADEATH_NAMES {
            let present = {
                let root = parse_profile_file(data)?;
                profile_difficulty_path(&root, profile_id, name)?.is_some()
            };
            if present {
                patch_profile_difficulty_bool(data, profile_id, name, perma)?;
                break;
            }
        }
    }
    // Flow helper (no alternate spelling).
    if let Some(flow) = req.flow_helper {
        patch_profile_difficulty_bool(data, profile_id, "m_FakeSloppyCombos", flow)?;
    }
    // Validation gate: the edited PersistentDataList must strictly re-parse from
    // its GVAS object and consume every byte. A length-changing patch that left
    // an enclosing size field stale misaligns here, so we abort rather than ship
    // a profile the game would reject.
    let root = parse_profile_file(data).map_err(|err| {
        CoreError::Validation(format!(
            "profile difficulty edit produced a payload that does not strictly re-parse: {err}"
        ))
    })?;
    if root.consumed != data.len() {
        return Err(CoreError::Validation(format!(
            "profile difficulty edit left {} trailing unparsed bytes",
            data.len() - root.consumed
        )));
    }
    Ok(())
}

/// Patch one string-valued difficulty field within a specific profile, with
/// full enclosing-size propagation. Skips silently if the field is absent.
fn patch_profile_difficulty_string(
    data: &mut Vec<u8>,
    profile_id: i32,
    name: &str,
    new_value: &str,
) -> Result<(), CoreError> {
    let root = parse_profile_file(data)?;
    let Some(full_path) = profile_difficulty_path(&root, profile_id, name)? else {
        return Ok(());
    };
    let segs = properties::parse_path(&full_path)?;
    let chain = properties::resolve_chain(&root.properties, &segs)?;
    let target = chain.target.clone();
    let enclosing = chain.enclosing_size_fields.clone();
    drop(root);
    properties::patch_string(data, &target, &enclosing, new_value)
}

/// Patch one BoolProperty difficulty field within a specific profile. Skips
/// silently if the field is absent.
fn patch_profile_difficulty_bool(
    data: &mut Vec<u8>,
    profile_id: i32,
    name: &str,
    value: bool,
) -> Result<(), CoreError> {
    let root = parse_profile_file(data)?;
    let Some(full_path) = profile_difficulty_path(&root, profile_id, name)? else {
        return Ok(());
    };
    let segs = properties::parse_path(&full_path)?;
    let chain = properties::resolve_chain(&root.properties, &segs)?;
    let target = chain.target.clone();
    drop(root);
    properties::patch_scalar(
        data.as_mut_slice(),
        &target,
        properties::ScalarValue::Bool(value),
    )
}

fn read_i32_after_property(payload: &[u8], refs: &[FStringRef], name: &str) -> Option<i32> {
    let name_idx = refs.iter().position(|r| r.value == name)?;
    read_i32_property_at(payload, refs, name_idx)
}

fn read_i32_property_in_range(
    payload: &[u8],
    refs: &[FStringRef],
    start_idx: usize,
    end_idx: usize,
    name: &str,
) -> Option<i32> {
    let name_idx = find_ref_in_range(refs, start_idx, end_idx, name)?;
    read_i32_property_at(payload, refs, name_idx)
}

fn read_f64_property_in_range(
    payload: &[u8],
    refs: &[FStringRef],
    start_idx: usize,
    end_idx: usize,
    name: &str,
) -> Option<f64> {
    let name_idx = find_ref_in_range(refs, start_idx, end_idx, name)?;
    read_f64_property_at(payload, refs, name_idx)
}

fn read_bool_property_in_range(
    payload: &[u8],
    refs: &[FStringRef],
    start_idx: usize,
    end_idx: usize,
    name: &str,
) -> Option<bool> {
    let name_idx = find_ref_in_range(refs, start_idx, end_idx, name)?;
    read_bool_property_at(payload, refs, name_idx)
}

/// Permadeath may be stored under either spelling depending on save version.
/// The read path falls back from `m_PermanentDeath` to `m_PermaDeath`
/// (see `parse_profile_summaries`); the typed write path mirrors that by
/// patching whichever spelling the parsed tree actually contains.
const PERMADEATH_NAMES: [&str; 2] = ["m_PermanentDeath", "m_PermaDeath"];

fn find_ref_in_range(
    refs: &[FStringRef],
    start_idx: usize,
    end_idx: usize,
    value: &str,
) -> Option<usize> {
    refs.iter()
        .enumerate()
        .take(end_idx)
        .skip(start_idx)
        .find(|(_, reference)| reference.value == value)
        .map(|(idx, _)| idx)
}

fn scan_fstrings(data: &[u8], base_offset: usize) -> Vec<FStringRef> {
    let mut out = Vec::new();
    for offset in 0..data.len().saturating_sub(4) {
        let n = i32::from_le_bytes(data[offset..offset + 4].try_into().unwrap());
        if n == 0 {
            continue;
        }
        if n > 0 && n <= 1024 {
            let len = n as usize;
            if offset + 4 + len > data.len() || len == 0 {
                continue;
            }
            let raw = &data[offset + 4..offset + 4 + len];
            if raw.last() != Some(&0) {
                continue;
            }
            let body = &raw[..raw.len() - 1];
            if body.is_empty() || !looks_texty(body) {
                continue;
            }
            out.push(FStringRef {
                value: String::from_utf8_lossy(body).to_string(),
                len_offset: base_offset + offset,
                total_len: 4 + len,
                utf16: false,
            });
        } else if n < 0 && n >= -512 {
            let chars = (-n) as usize;
            let byte_len = chars * 2;
            if offset + 4 + byte_len > data.len() || chars == 0 {
                continue;
            }
            let raw = &data[offset + 4..offset + 4 + byte_len];
            if raw.len() < 2 || raw[raw.len() - 2..] != [0, 0] {
                continue;
            }
            let units = raw[..raw.len() - 2]
                .chunks_exact(2)
                .map(|c| u16::from_le_bytes([c[0], c[1]]))
                .collect::<Vec<_>>();
            if let Ok(value) = String::from_utf16(&units) {
                if !value.is_empty() {
                    out.push(FStringRef {
                        value,
                        len_offset: base_offset + offset,
                        total_len: 4 + byte_len,
                        utf16: true,
                    });
                }
            }
        }
    }
    out.sort_by_key(|r| r.len_offset);
    out
}

fn looks_texty(bytes: &[u8]) -> bool {
    std::str::from_utf8(bytes)
        .map(|s| {
            s.chars()
                .all(|c| !c.is_control() && c != '\u{fffd}' && c != '\0')
        })
        .unwrap_or(false)
}

fn extract_script_paths(data: &[u8]) -> Vec<String> {
    scan_fstrings(data, 0)
        .into_iter()
        .map(|r| r.value)
        .filter(|s| s.starts_with("/Script/"))
        .take(40)
        .collect()
}

fn inspect_private_payload(
    data: &[u8],
    path: Option<&Path>,
    stream: &CompressedStream,
    codec_backend: Option<&dyn codec_backend::CodecBackend>,
    private_chunk_limit: Option<usize>,
) -> Result<Value, CoreError> {
    let Some(backend) = codec_backend else {
        return kraken::inspect_private_payload(data, stream);
    };
    match decompress_private_payload_with_limit(data, stream, backend, private_chunk_limit) {
        Ok((payload, decoded_chunk_count)) => {
            let preview = decoded_chunk_count < stream.chunk_count;
            // A full (non-preview) decode here is identical to what the typed
            // property browser would re-decode on its first search. Seed the
            // shared cache so the common inspect-then-browse path pays the
            // ~20s decode only once per save.
            if !preview {
                if let Some(p) = path {
                    store_decoded_payload_cache(p, sha1_hex(data), payload.clone());
                }
            }
            let refs = scan_fstrings(&payload, 0);
            let strings = refs
                .iter()
                .map(|reference| reference.value.clone())
                .filter(|value| !value.is_empty())
                .take(200)
                .collect::<Vec<_>>();
            let player = summarize_private_player_payload(&payload, &refs);
            let typed_result = if preview {
                None
            } else {
                Some(properties::parse_private_root(&payload))
            };
            let main_container = typed_result
                .as_ref()
                .and_then(|r| r.as_ref().ok())
                .and_then(main_container_summary);
            let inventory =
                summarize_private_inventory_payload(&payload, &refs, main_container.as_ref());
            let typed_parse = summarize_typed_parse_result(&payload, typed_result.as_ref());
            let typed_ok = typed_parse["status"] == "ok";
            let progression = summarize_private_progression_overview(
                typed_result
                    .as_ref()
                    .and_then(|r| r.as_ref().ok())
                    .filter(|_| typed_ok),
            );
            let mut writable = vec!["private.replaceFString"];
            if typed_parse["status"] == "ok" {
                writable.extend([
                    "private.typed.setValue",
                    "private.typed.setAdd",
                    "private.typed.setRemove",
                    "private.typed.arrayRemove",
                    "private.typed.arrayDuplicate",
                    // Inserts a new entry into the CharacterKnowledgeByUniqueName
                    // map; needs only a decodable typed parse (no inventory
                    // main_container gating).
                    "private.knowledge.addCharacter",
                ]);
                // addItem/removeItem are gated per save (clean template /
                // removable item); mirror the inventory summary's gating so the
                // top-level writable list never advertises an op write_save
                // would reject.
                if let Some(mc) = &main_container {
                    if mc.has_clean_template {
                        writable.push("private.inventory.addItem");
                    }
                    if !mc.removable_paths.is_empty() {
                        writable.push("private.inventory.removeItem");
                    }
                }
            }
            Ok(json!({
                "status": if preview { "decoded_preview" } else { "decoded" },
                "message": if preview {
                    "Private payload preview decoded through the G1R codec host."
                } else {
                    "Private payload decoded through the G1R codec host."
                },
                "method": stream.method,
                "algorithmId": stream.algorithm_id,
                "chunkCount": stream.chunk_count,
                "decodedChunkCount": decoded_chunk_count,
                "totalChunkCount": stream.chunk_count,
                "preview": preview,
                "compressedSize": stream.summary_compressed_size,
                "uncompressedSize": stream.summary_uncompressed_size,
                "decompressedSize": payload.len(),
                "stringCount": strings.len(),
                "strings": strings,
                "player": player,
                "inventory": inventory,
                "progression": progression,
                "typedParse": typed_parse,
                "writable": writable,
            }))
        }
        Err(err) => Ok(json!({
            "status": "codec_host_failed",
            "message": err.to_string(),
            "method": stream.method,
            "algorithmId": stream.algorithm_id,
            "chunkCount": stream.chunk_count,
            "compressedSize": stream.summary_compressed_size,
            "uncompressedSize": stream.summary_uncompressed_size,
            "writable": [],
        })),
    }
}

fn summarize_private_player_payload(payload: &[u8], refs: &[FStringRef]) -> Value {
    let script_paths = unique_strings(
        refs.iter()
            .map(|r| r.value.as_str())
            .filter(|value| value.starts_with("/Script/")),
        80,
    );
    let properties = unique_strings(
        refs.iter()
            .map(|r| r.value.as_str())
            .filter(|value| value.starts_with("m_")),
        120,
    );
    let player_name = private_player_name_value_refs(refs)
        .into_iter()
        .next()
        .map(|reference| reference.value.value)
        .filter(|value| value != "None");
    let profile_name = private_profile_name_value_refs(refs)
        .into_iter()
        .next()
        .map(|reference| reference.value.value)
        .filter(|value| value != "None");
    let attributes = private_player_attribute_refs(payload, refs)
        .into_iter()
        .map(|attribute| {
            json!({
                "id": attribute.id,
                "baseValue": attribute.base_value.map(f64::from),
                "currentValue": attribute.current_value.map(f64::from),
            })
        })
        .collect::<Vec<_>>();
    let transform = private_player_transform_refs(payload, refs)
        .into_iter()
        .next()
        .map(|transform| {
            json!({
                "location": {
                    "x": transform.location.x,
                    "y": transform.location.y,
                    "z": transform.location.z,
                },
                "rotation": {
                    "pitch": transform.rotation.pitch,
                    "yaw": transform.rotation.yaw,
                    "roll": transform.rotation.roll,
                }
            })
        });
    json!({
        "saveVersionNumber": read_i32_after_property(payload, refs, "m_SaveVersionNumber"),
        "currentWorld": value_after_property(refs, "m_CurrentWorld")
            .or_else(|| value_after_property(refs, "m_World")),
        "playerName": player_name,
        "profileName": profile_name,
        "attributes": attributes,
        "transform": transform,
        "scriptPaths": script_paths,
        "properties": properties,
        "writable": private_player_writable_edits(payload, refs),
    })
}

fn private_player_writable_edits(payload: &[u8], refs: &[FStringRef]) -> Vec<&'static str> {
    let mut writable = Vec::new();
    if private_player_name_value_refs(refs).len() == 1 {
        writable.push("private.player.setPlayerName");
    }
    if private_profile_name_value_refs(refs).len() == 1 {
        writable.push("private.profile.setProfileName");
    }
    if hero_attribute_region(refs).is_some() {
        writable.push("private.player.setAttribute");
    }
    if private_player_transform_refs(payload, refs).len() == 1 {
        writable.push("private.player.setTransform");
    }
    writable
}

fn summarize_private_inventory_payload(
    payload: &[u8],
    refs: &[FStringRef],
    main_container: Option<&MainContainerSummary>,
) -> Value {
    let script_paths = unique_strings(
        refs.iter().map(|r| r.value.as_str()).filter(|value| {
            value.starts_with("/Script/") && contains_any_ci(value, &["inventory", "item"])
        }),
        80,
    );
    let properties = unique_strings(
        refs.iter().map(|r| r.value.as_str()).filter(|value| {
            value.starts_with("m_")
                && contains_any_ci(value, &["inventory", "item", "stack", "quantity", "amount"])
        }),
        120,
    );
    let candidates = unique_strings(
        refs.iter()
            .map(|r| r.value.as_str())
            .filter(|value| looks_inventory_candidate(value)),
        200,
    );
    let (mut items, item_stack_count, item_scope) =
        summarize_private_inventory_items(payload, refs, 200);
    // Mark which rows can be deleted. removeItem addresses by path, so only a
    // path that occurs exactly once across the whole inventory is safe — a row
    // sharing its path with another container's stack must not offer delete, or
    // it could drop the wrong stack.
    for item in &mut items {
        let path = item["path"].as_str().unwrap_or("");
        item["removable"] = json!(
            !path.is_empty() && main_container.is_some_and(|mc| mc.removable_paths.contains(path))
        );
    }
    // setItemCount patches an existing scanned stack in place, so it depends on
    // the FString scan finding at least one stack in the player region.
    // addItem/removeItem are structural edits on the *typed* MainContainer and
    // are independent of the scan: a save can have a resolvable MainContainer
    // (with a clean template and/or removable items) while the FString scan
    // reports zero stacks, and the core can still perform the structural edit.
    let mut writable = Vec::new();
    if item_scope == "player_inventory_region" && item_stack_count > 0 {
        writable.push("private.inventory.setItemCount");
    }
    if let Some(mc) = main_container {
        // addItem clones a clean template slot; only offer it when one exists
        // somewhere in the inventory.
        if mc.has_clean_template {
            writable.push("private.inventory.addItem");
        }
        if !mc.removable_paths.is_empty() {
            writable.push("private.inventory.removeItem");
        }
    }
    // The complete set of MainContainer item paths (from the typed tree, not
    // the capped summary scan) so the add dialog can exclude already-owned
    // items even when the displayed list is truncated.
    let mut main_paths: Vec<&String> = main_container
        .map(|mc| mc.all_paths.iter().collect())
        .unwrap_or_default();
    main_paths.sort();
    json!({
        "candidateCount": candidates.len(),
        "candidates": candidates,
        "itemStackCount": item_stack_count,
        "itemScope": item_scope,
        "items": items,
        "mainContainerPaths": main_paths,
        "scriptPaths": script_paths,
        "properties": properties,
        "writable": writable,
    })
}

/// Attempt a strict typed parse of the full decompressed payload and report a
/// compact status. This is the foundation for typed (layout-verified) private
/// edits; for now it surfaces whether the proven UE property grammar fully
/// accounts for this save's bytes. Skipped for previews (truncated payloads
/// cannot parse to completion).
///
/// `result` is `None` when the parse was skipped (preview mode); `Some(&Ok(...))`
/// or `Some(&Err(...))` when parsing was attempted.
fn summarize_typed_parse_result(
    payload: &[u8],
    result: Option<&Result<properties::RootObject, CoreError>>,
) -> Value {
    let Some(result) = result else {
        return json!({
            "status": "skipped_preview",
            "message": "Typed parse needs the full decoded payload.",
        });
    };
    match result {
        Ok(root) => {
            let counts = properties::count_properties(&root.properties);
            json!({
                "status": "ok",
                "rootClass": root.class,
                "topLevelProperties": root.properties.len(),
                "propertyCount": counts.total,
                "maxDepth": counts.max_depth,
                "consumed": root.consumed,
                "payloadSize": payload.len(),
            })
        }
        Err(err) => json!({
            "status": "failed",
            "message": err.to_string(),
        }),
    }
}

/// In-memory cache of the most recently decoded private payload. Decoding all
/// chunks costs ~20s, so the typed property browser must not re-decode on every
/// search/edit. Holds a single entry (the active save), bounded to one payload
/// (~77 MB) and keyed by the save file's SHA-1 so an external change misses.
static DECODED_PAYLOAD_CACHE: Mutex<Option<DecodedPayloadEntry>> = Mutex::new(None);

struct DecodedPayloadEntry {
    path: PathBuf,
    save_sha1: String,
    payload: Vec<u8>,
}

/// Return the decoded private payload for `path`, using the cache when the save
/// file is unchanged, otherwise decoding through the codec backend and storing
/// the result.
fn decoded_private_payload_cached(
    path: &Path,
    data: &[u8],
    stream: &CompressedStream,
    backend: &dyn codec_backend::CodecBackend,
) -> Result<Vec<u8>, CoreError> {
    let save_sha1 = sha1_hex(data);
    {
        let guard = DECODED_PAYLOAD_CACHE
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        if let Some(entry) = guard.as_ref() {
            if entry.path == path && entry.save_sha1 == save_sha1 {
                return Ok(entry.payload.clone());
            }
        }
    }
    let payload = decompress_private_payload(data, stream, backend)?;
    store_decoded_payload_cache(path, save_sha1, payload.clone());
    Ok(payload)
}

/// Store a freshly decoded full private payload as the single cache entry.
fn store_decoded_payload_cache(path: &Path, save_sha1: String, payload: Vec<u8>) {
    let mut guard = DECODED_PAYLOAD_CACHE
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    *guard = Some(DecodedPayloadEntry {
        path: path.to_path_buf(),
        save_sha1,
        payload,
    });
}

/// Drop the cached decoded payload for `path` (called after a write changes it).
fn invalidate_decoded_payload_cache(path: &Path) {
    let mut guard = DECODED_PAYLOAD_CACHE
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    if guard.as_ref().is_some_and(|entry| entry.path == path) {
        *guard = None;
    }
}

/// Search every typed property in the decoded private payload. Powers the
/// "all data" property browser: a query filters by display path, results carry
/// a setValue-addressable path and an editable flag for fixed-size scalars.
fn search_typed_properties(
    path: &Path,
    payload: &Value,
    backend: Option<&dyn codec_backend::CodecBackend>,
) -> Result<Value, CoreError> {
    let backend = backend.ok_or_else(|| {
        CoreError::Codec(
            "typed property search requires a configured and verified G1R codec host".to_string(),
        )
    })?;
    let query = payload.get("query").and_then(Value::as_str).unwrap_or("");
    let limit = payload
        .get("limit")
        .and_then(Value::as_u64)
        .map(|v| v as usize)
        .unwrap_or(50)
        .clamp(1, 1000);
    let offset = payload
        .get("offset")
        .and_then(Value::as_u64)
        .map(|v| v as usize)
        .unwrap_or(0);

    let data = fs::read(path)?;
    if !data.starts_with(b"GSAV") {
        return Err(CoreError::UnsupportedEdit(
            "typed property search is only available for GSAV files".to_string(),
        ));
    }
    let parts = split_gsav(&data)?;
    let stream = parse_compressed_stream(&data, 13 + parts.public_payload.len())?;
    let decoded = decoded_private_payload_cached(path, &data, &stream, backend)?;
    let root = properties::parse_private_root(&decoded)?;
    let (hits, total) = properties::search_properties(&root, query, offset, limit);

    let results = hits
        .into_iter()
        .map(|hit| {
            json!({
                "path": hit.path,
                "display": hit.display,
                "type": hit.type_name,
                "value": hit.value_display,
                "editable": hit.editable,
            })
        })
        .collect::<Vec<_>>();

    Ok(json!({
        "query": query,
        "offset": offset,
        "limit": limit,
        "total": total,
        "count": results.len(),
        "results": results,
    }))
}

/// Structured progression queries over the decoded private payload. Sections:
/// "quests" (QuestDataByClass entries with setValue-addressable state paths),
/// "knowledge" (per-NPC dialog knowledge sets), "events" (per-character
/// memorized event arrays). Uses the shared decode cache like the typed
/// property search.
fn query_progression(
    path: &Path,
    payload: &Value,
    backend: Option<&dyn codec_backend::CodecBackend>,
) -> Result<Value, CoreError> {
    let backend = backend.ok_or_else(|| {
        CoreError::Codec(
            "progression queries require a configured and verified G1R codec host".to_string(),
        )
    })?;
    let section = payload
        .get("section")
        .and_then(Value::as_str)
        .unwrap_or("quests");
    let query = payload
        .get("query")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();
    let limit = payload
        .get("limit")
        .and_then(Value::as_u64)
        .map(|v| v as usize)
        .unwrap_or(100)
        .clamp(1, 1000);
    let offset = payload
        .get("offset")
        .and_then(Value::as_u64)
        .map(|v| v as usize)
        .unwrap_or(0);
    let character = payload.get("character").and_then(Value::as_str);
    let state_filter = payload
        .get("state")
        .and_then(Value::as_str)
        .map(|s| s.trim().to_ascii_lowercase())
        .filter(|s| !s.is_empty());
    let group_filter = payload
        .get("group")
        .and_then(Value::as_str)
        .map(|s| s.trim().to_ascii_lowercase())
        .filter(|s| !s.is_empty());

    let data = fs::read(path)?;
    if !data.starts_with(b"GSAV") {
        return Err(CoreError::UnsupportedEdit(
            "progression queries are only available for GSAV files".to_string(),
        ));
    }
    let parts = split_gsav(&data)?;
    let stream = parse_compressed_stream(&data, 13 + parts.public_payload.len())?;
    let decoded = decoded_private_payload_cached(path, &data, &stream, backend)?;
    let root = properties::parse_private_root(&decoded)?;
    match section {
        "quests" => progression_quests(
            &root,
            &query,
            state_filter.as_deref(),
            group_filter.as_deref(),
            offset,
            limit,
        ),
        "knowledge" => progression_knowledge(&root, &query, character, offset, limit),
        "events" => progression_events(&root, &query, character, offset, limit),
        other => Err(CoreError::InvalidRequest(format!(
            "unknown progression section {other:?}"
        ))),
    }
}

/// Property lookup inside a struct-valued map entry (tagged property list or
/// InstancedStruct wrapper).
fn struct_member<'a>(
    value: &'a properties::PropertyValue,
    name: &str,
) -> Option<&'a properties::PropertyValue> {
    let props = match value {
        properties::PropertyValue::Struct(properties::StructValue::Properties(p)) => p,
        properties::PropertyValue::Struct(properties::StructValue::Instanced(Some(i))) => {
            &i.properties
        }
        _ => return None,
    };
    props.iter().find(|p| p.name == name).map(|p| &p.value)
}

fn map_key_string(key: &properties::PropertyValue) -> Option<&str> {
    match key {
        properties::PropertyValue::Str(s)
        | properties::PropertyValue::Name(s)
        | properties::PropertyValue::Enum(s)
        | properties::PropertyValue::Object(s) => Some(s),
        _ => None,
    }
}

/// "EQuestState::Running" → "Running" for the overview/state-count labels.
fn short_enum_label(value: &str) -> &str {
    value.rsplit("::").next().unwrap_or(value)
}

/// Parse the id, group, and name from a quest class path. The tail after
/// the last '.' is the id; strip "Quest_" prefix, then split on the first
/// '_' to get group and name. This is factored out so the filter and the
/// page-entry renderer use identical logic and can never diverge.
fn quest_id_group_name(class_path: &str) -> (String, String, String) {
    let id = class_path
        .rsplit('.')
        .next()
        .unwrap_or(class_path)
        .to_string();
    let trimmed = id.strip_prefix("Quest_").unwrap_or(&id);
    let (group, name) = match trimmed.split_once('_') {
        Some((g, n)) => (g.to_string(), n.to_string()),
        None => (trimmed.to_string(), String::new()),
    };
    (id, group, name)
}

fn progression_quests(
    root: &properties::RootObject,
    query: &str,
    state_filter: Option<&str>,
    group_filter: Option<&str>,
    offset: usize,
    limit: usize,
) -> Result<Value, CoreError> {
    let (base_path, map_prop) = properties::find_property_by_name(root, "QuestDataByClass")
        .ok_or_else(|| {
            CoreError::Parse("QuestDataByClass not found in the decoded payload".to_string())
        })?;
    let properties::PropertyValue::Map { entries, .. } = &map_prop.value else {
        return Err(CoreError::Parse(
            "QuestDataByClass is not a map".to_string(),
        ));
    };
    // Collect all entries as tuples first, then do three filtered passes for
    // faceted counts (each facet counts over every OTHER active filter but not
    // its own) plus one pass for the quests list.
    struct Entry {
        class_path: String,
        id: String,
        group: String,
        name: String,
        state: Option<String>,
        label: String,
    }
    let mut all: Vec<Entry> = Vec::new();
    for (key, value) in entries {
        let Some(class_path) = map_key_string(key) else {
            continue;
        };
        let state = struct_member(value, "CurrentState").and_then(|v| match v {
            properties::PropertyValue::Enum(s) => Some(s.clone()),
            _ => None,
        });
        let label = state
            .as_deref()
            .map(short_enum_label)
            .unwrap_or("unknown")
            .to_string();
        let (id, group, name) = quest_id_group_name(class_path);
        all.push(Entry {
            class_path: class_path.to_string(),
            id,
            group,
            name,
            state,
            label,
        });
    }

    // Helper closures for each filter predicate.
    let matches_query = |e: &Entry| -> bool {
        query.is_empty() || e.class_path.to_ascii_lowercase().contains(query)
    };
    let matches_state = |e: &Entry| -> bool {
        match state_filter {
            None => true,
            Some(sf) => {
                // Accept short label ("Running") or full enum form ("EQuestState::Running"),
                // both case-insensitive.
                let short = short_enum_label(e.state.as_deref().unwrap_or("")).to_ascii_lowercase();
                let full = e.state.as_deref().unwrap_or("").to_ascii_lowercase();
                short == sf || full == sf
            }
        }
    };
    let matches_group = |e: &Entry| -> bool {
        match group_filter {
            None => true,
            Some(gf) => e.group.to_ascii_lowercase() == gf,
        }
    };

    // stateCounts: query + group filter, state filter NOT applied.
    let mut state_counts = std::collections::BTreeMap::<String, usize>::new();
    for e in &all {
        if matches_query(e) && matches_group(e) {
            *state_counts.entry(e.label.clone()).or_default() += 1;
        }
    }

    // groupCounts: query + state filter, group filter NOT applied.
    let mut group_counts = std::collections::BTreeMap::<String, usize>::new();
    for e in &all {
        if matches_query(e) && matches_state(e) {
            *group_counts.entry(e.group.clone()).or_default() += 1;
        }
    }

    // Quests list: all three filters, sorted by class_path.
    let mut matches: Vec<&Entry> = all
        .iter()
        .filter(|e| matches_query(e) && matches_state(e) && matches_group(e))
        .collect();
    matches.sort_by(|a, b| a.class_path.cmp(&b.class_path));
    let total = matches.len();
    let quests = matches
        .into_iter()
        .skip(offset)
        .take(limit)
        .map(|e| {
            let mut state_path = base_path.clone();
            state_path.push(format!("{{{}}}", e.class_path));
            state_path.push("CurrentState".to_string());
            let writable = e.state.is_some();
            json!({
                "questClass": e.class_path,
                "id": e.id,
                "group": e.group,
                "name": e.name,
                "currentState": e.state,
                "statePath": state_path,
                "writable": writable,
            })
        })
        .collect::<Vec<_>>();
    Ok(json!({
        "section": "quests",
        "total": total,
        "offset": offset,
        "limit": limit,
        "count": quests.len(),
        "stateCounts": state_counts,
        "groupCounts": group_counts,
        "quests": quests,
    }))
}

fn progression_knowledge(
    root: &properties::RootObject,
    query: &str,
    character: Option<&str>,
    offset: usize,
    limit: usize,
) -> Result<Value, CoreError> {
    let (base_path, map_prop) =
        properties::find_property_by_name(root, "CharacterKnowledgeByUniqueName").ok_or_else(
            || {
                CoreError::Parse(
                    "CharacterKnowledgeByUniqueName not found in the decoded payload".to_string(),
                )
            },
        )?;
    let properties::PropertyValue::Map { entries, .. } = &map_prop.value else {
        return Err(CoreError::Parse(
            "CharacterKnowledgeByUniqueName is not a map".to_string(),
        ));
    };
    let knowledge_entries = |value: &properties::PropertyValue| -> Vec<String> {
        match struct_member(value, "Knowledge") {
            Some(properties::PropertyValue::Set { elements, .. }) => elements
                .iter()
                .filter_map(|e| match e {
                    properties::PropertyValue::Name(s) | properties::PropertyValue::Str(s) => {
                        Some(s.clone())
                    }
                    _ => None,
                })
                .collect(),
            _ => Vec::new(),
        }
    };
    match character {
        None => {
            let mut characters: Vec<(String, usize)> = entries
                .iter()
                .filter_map(|(key, value)| {
                    let name = map_key_string(key)?;
                    if !query.is_empty() && !name.to_ascii_lowercase().contains(query) {
                        return None;
                    }
                    Some((name.to_string(), knowledge_entries(value).len()))
                })
                .collect();
            characters.sort_by(|a, b| a.0.cmp(&b.0));
            let total = characters.len();
            let page = characters
                .into_iter()
                .skip(offset)
                .take(limit)
                .map(|(name, entry_count)| json!({ "name": name, "entryCount": entry_count }))
                .collect::<Vec<_>>();
            Ok(json!({
                "section": "knowledge",
                "total": total,
                "offset": offset,
                "limit": limit,
                "count": page.len(),
                "characters": page,
            }))
        }
        Some(character) => {
            let value = entries
                .iter()
                .find(|(key, _)| map_key_string(key) == Some(character))
                .map(|(_, value)| value)
                .ok_or_else(|| {
                    CoreError::Parse(format!("character {character:?} has no knowledge entry"))
                })?;
            let mut all = knowledge_entries(value);
            if !query.is_empty() {
                all.retain(|e| e.to_ascii_lowercase().contains(query));
            }
            let total = all.len();
            let page = all.into_iter().skip(offset).take(limit).collect::<Vec<_>>();
            let mut set_path = base_path.clone();
            set_path.push(format!("{{{character}}}"));
            set_path.push("Knowledge".to_string());
            Ok(json!({
                "section": "knowledge",
                "character": character,
                "total": total,
                "offset": offset,
                "limit": limit,
                "count": page.len(),
                "entries": page,
                "setPath": set_path,
            }))
        }
    }
}

/// Serialize one `CharacterKnowledgeByUniqueName` map entry: an inline Name key
/// (the NPC's unique name) followed by an inline `KnowledgeSet` struct value that
/// holds an empty `Knowledge` set. The struct value is a property list with one
/// (empty) Name set named "Knowledge", terminated by the "None" sentinel that
/// closes a non-native struct's property list.
///
/// The returned bytes are a schema-valid entry for the proven map layout and are
/// meant to feed `ContainerEdit::MapInsert`. Wired into the
/// `private.knowledge.addCharacter` IPC op.
fn encode_knowledge_map_entry(unique_name: &str) -> Vec<u8> {
    let mut out = properties::encode_fstring_value(unique_name); // inline Name key
    out.extend_from_slice(&encode_empty_name_set_property("Knowledge"));
    out.extend_from_slice(&properties::encode_fstring_value("None"));
    out
}

/// A tagged `SetProperty<NameProperty>` carrying zero elements. The byte layout
/// matches the proven `name_set_property("Knowledge", &[])` fixture exactly:
/// name fstring, "SetProperty" fstring, `1u32`, "NameProperty" fstring,
/// `0u32` array_index, body-size `u32`, `0u8` tag_flags, then the body
/// (`num_to_remove u32` + `count u32`).
fn encode_empty_name_set_property(name: &str) -> Vec<u8> {
    let mut body = 0u32.to_le_bytes().to_vec(); // num_to_remove
    body.extend_from_slice(&0u32.to_le_bytes()); // count
    let mut out = properties::encode_fstring_value(name);
    out.extend_from_slice(&properties::encode_fstring_value("SetProperty"));
    out.extend_from_slice(&1u32.to_le_bytes()); // array_index marker
    out.extend_from_slice(&properties::encode_fstring_value("NameProperty"));
    out.extend_from_slice(&0u32.to_le_bytes()); // array_index
    out.extend_from_slice(&(body.len() as u32).to_le_bytes()); // body size
    out.push(0); // tag_flags
    out.extend_from_slice(&body);
    out
}

fn progression_events(
    root: &properties::RootObject,
    query: &str,
    character: Option<&str>,
    offset: usize,
    limit: usize,
) -> Result<Value, CoreError> {
    let (base_path, map_prop) = properties::find_property_by_name(root, "LongTermMemoryByGlobalId")
        .ok_or_else(|| {
            CoreError::Parse(
                "LongTermMemoryByGlobalId not found in the decoded payload".to_string(),
            )
        })?;
    let properties::PropertyValue::Map { entries, .. } = &map_prop.value else {
        return Err(CoreError::Parse(
            "LongTermMemoryByGlobalId is not a map".to_string(),
        ));
    };
    let memorized = |value: &properties::PropertyValue| -> Option<usize> {
        match struct_member(value, "MemorizedEvents") {
            Some(properties::PropertyValue::Array { elements }) => Some(elements.len()),
            _ => None,
        }
    };
    match character {
        None => {
            let mut characters: Vec<(String, usize)> = entries
                .iter()
                .filter_map(|(key, value)| {
                    let id = map_key_string(key)?;
                    if !query.is_empty() && !id.to_ascii_lowercase().contains(query) {
                        return None;
                    }
                    Some((id.to_string(), memorized(value).unwrap_or(0)))
                })
                .collect();
            characters.sort_by(|a, b| a.0.cmp(&b.0));
            let total = characters.len();
            let page = characters
                .into_iter()
                .skip(offset)
                .take(limit)
                .map(|(id, event_count)| json!({ "id": id, "eventCount": event_count }))
                .collect::<Vec<_>>();
            Ok(json!({
                "section": "events",
                "total": total,
                "offset": offset,
                "limit": limit,
                "count": page.len(),
                "characters": page,
            }))
        }
        Some(character) => {
            let value = entries
                .iter()
                .find(|(key, _)| map_key_string(key) == Some(character))
                .map(|(_, value)| value)
                .ok_or_else(|| {
                    CoreError::Parse(format!("character {character:?} has no memory entry"))
                })?;
            let Some(properties::PropertyValue::Array { elements }) =
                struct_member(value, "MemorizedEvents")
            else {
                return Err(CoreError::Parse(
                    "MemorizedEvents missing or not an array".to_string(),
                ));
            };
            let event_json = |index: usize, element: &properties::PropertyValue| -> Value {
                let tags = match struct_member(element, "EventTags") {
                    Some(properties::PropertyValue::Struct(
                        properties::StructValue::GameplayTagContainer(tags),
                    )) => tags.clone(),
                    _ => Vec::new(),
                };
                let in_game_seconds = |name: &str| -> Option<f64> {
                    match struct_member(element, name)
                        .and_then(|t| struct_member(t, "TotalSeconds"))
                    {
                        Some(properties::PropertyValue::Double(v)) => Some(*v),
                        _ => None,
                    }
                };
                let name_member = |name: &str| -> Option<String> {
                    match struct_member(element, name) {
                        Some(properties::PropertyValue::Name(s)) => Some(s.clone()),
                        _ => None,
                    }
                };
                let soft_member = |name: &str| -> Option<String> {
                    match struct_member(element, name) {
                        Some(properties::PropertyValue::SoftObject(p))
                            if !p.package_name.is_empty() && p.package_name != "None" =>
                        {
                            Some(p.package_name.clone())
                        }
                        _ => None,
                    }
                };
                let magnitude = match struct_member(element, "Magnitude") {
                    Some(properties::PropertyValue::Float(v)) => Some(f64::from(*v)),
                    _ => None,
                };
                json!({
                    "index": index,
                    "tags": tags,
                    "magnitude": magnitude,
                    "timeSeconds": in_game_seconds("Time"),
                    "durationSeconds": in_game_seconds("Duration"),
                    "instigator": name_member("InstigatorGlobalId"),
                    "affected": name_member("AffectedCharacterGlobalId"),
                    "optionalClass1": soft_member("OptionalClass1"),
                    "optionalClass2": soft_member("OptionalClass2"),
                })
            };
            let matches_query = |event: &Value| -> bool {
                if query.is_empty() {
                    return true;
                }
                let hay = [
                    event["tags"].to_string(),
                    event["instigator"].to_string(),
                    event["affected"].to_string(),
                    event["optionalClass1"].to_string(),
                    event["optionalClass2"].to_string(),
                ]
                .join(" ")
                .to_ascii_lowercase();
                hay.contains(query)
            };
            let all: Vec<Value> = elements
                .iter()
                .enumerate()
                .map(|(index, element)| event_json(index, element))
                .filter(|e| matches_query(e))
                .collect();
            let total = all.len();
            let page = all.into_iter().skip(offset).take(limit).collect::<Vec<_>>();
            let mut array_path = base_path.clone();
            array_path.push(format!("{{{character}}}"));
            array_path.push("MemorizedEvents".to_string());
            Ok(json!({
                "section": "events",
                "character": character,
                "total": total,
                "offset": offset,
                "limit": limit,
                "count": page.len(),
                "events": page,
                "arrayPath": array_path,
            }))
        }
    }
}

/// Structured progression overview for the inspect response: quest counts by
/// state plus knowledge/memory totals. `root` is Some only when the strict
/// typed parse succeeded on a full (non-preview) decode.
fn summarize_private_progression_overview(root: Option<&properties::RootObject>) -> Value {
    let Some(root) = root else {
        return json!({ "status": "unavailable", "writable": [] });
    };
    let mut quest_total = 0usize;
    let mut quest_states = std::collections::BTreeMap::<String, usize>::new();
    if let Some((_, prop)) = properties::find_property_by_name(root, "QuestDataByClass") {
        if let properties::PropertyValue::Map { entries, .. } = &prop.value {
            quest_total = entries.len();
            for (_, value) in entries {
                let label = match struct_member(value, "CurrentState") {
                    Some(properties::PropertyValue::Enum(s)) => short_enum_label(s).to_string(),
                    _ => "unknown".to_string(),
                };
                *quest_states.entry(label).or_default() += 1;
            }
        }
    }
    let mut knowledge_characters = 0usize;
    let mut knowledge_entries = 0usize;
    if let Some((_, prop)) =
        properties::find_property_by_name(root, "CharacterKnowledgeByUniqueName")
    {
        if let properties::PropertyValue::Map { entries, .. } = &prop.value {
            knowledge_characters = entries.len();
            for (_, value) in entries {
                if let Some(properties::PropertyValue::Set { elements, .. }) =
                    struct_member(value, "Knowledge")
                {
                    knowledge_entries += elements.len();
                }
            }
        }
    }
    let mut memory_characters = 0usize;
    let mut memory_events = 0usize;
    if let Some((_, prop)) = properties::find_property_by_name(root, "LongTermMemoryByGlobalId") {
        if let properties::PropertyValue::Map { entries, .. } = &prop.value {
            memory_characters = entries.len();
            for (_, value) in entries {
                if let Some(properties::PropertyValue::Array { elements }) =
                    struct_member(value, "MemorizedEvents")
                {
                    memory_events += elements.len();
                }
            }
        }
    }
    json!({
        "status": "ok",
        "questTotal": quest_total,
        "questStates": quest_states,
        "knowledgeCharacters": knowledge_characters,
        "knowledgeEntries": knowledge_entries,
        "memoryCharacters": memory_characters,
        "memoryEvents": memory_events,
        // Inventory edit ops (addItem/removeItem) are intentionally excluded:
        // they are not progression edits, and their availability is gated per
        // save (clean template / removable item). The inventory summary computes
        // the correctly gated writable list for those.
        "writable": [
            "private.typed.setValue",
            "private.typed.setAdd",
            "private.typed.setRemove",
            "private.typed.arrayRemove",
            "private.typed.arrayDuplicate",
            // Knowledge add-character is a progression edit on the
            // CharacterKnowledgeByUniqueName map summarized above; it requires
            // only the typed parse (guaranteed here since `root` is Some).
            "private.knowledge.addCharacter",
        ],
    })
}

fn summarize_private_inventory_items(
    payload: &[u8],
    refs: &[FStringRef],
    limit: usize,
) -> (Vec<Value>, usize, &'static str) {
    let (start_idx, end_idx, scope) = inventory_item_region(refs);
    let mut total = 0usize;
    let mut items = Vec::new();
    for (idx, reference) in refs.iter().enumerate().take(end_idx).skip(start_idx) {
        if reference.value != "m_ItemDefinition" {
            continue;
        }
        let Some(type_ref) = refs.get(idx + 1) else {
            continue;
        };
        if type_ref.value != "ObjectProperty" {
            continue;
        }
        let Some(path_ref) = refs
            .iter()
            .skip(idx + 2)
            .take(4)
            .find(|candidate| !is_property_type_name(candidate.value.as_str()))
        else {
            continue;
        };
        if !looks_item_definition_path(&path_ref.value) {
            continue;
        }
        total += 1;
        if items.len() >= limit {
            continue;
        }
        let count = refs
            .iter()
            .enumerate()
            .skip(idx + 3)
            .take(8)
            .find(|(_, candidate)| candidate.value == "m_ItemCount")
            .and_then(|(count_idx, _)| read_i32_property_at(payload, refs, count_idx));
        items.push(json!({
            "id": item_id_from_path(&path_ref.value),
            "path": path_ref.value,
            "count": count,
        }));
    }
    (items, total, scope)
}

fn inventory_item_region(refs: &[FStringRef]) -> (usize, usize, &'static str) {
    // Scope to the controlled player's (Party ID 0) inventory: the first
    // m_Inventory after that player's id marker, so the displayed/count-edited
    // rows match the inventory add/remove operate on (see resolve_inventory_path).
    // Fall back to the first m_Inventory anywhere when no such marker is present
    // (synthetic payloads without a m_SavedPlayers/Party ID 0 structure).
    let player_inventory = refs
        .iter()
        .position(|r| r.value == PLAYER_PARTY_ID)
        .and_then(|party0_idx| {
            refs.iter()
                .skip(party0_idx + 1)
                .position(|r| r.value == "m_Inventory")
                .map(|rel| party0_idx + 1 + rel)
        });
    let Some(start_idx) =
        player_inventory.or_else(|| refs.iter().position(|r| r.value == "m_Inventory"))
    else {
        return (0, refs.len(), "global_observed");
    };
    let end_idx = refs
        .iter()
        .enumerate()
        .skip(start_idx + 1)
        .find(|(_, reference)| {
            matches!(
                reference.value.as_str(),
                "m_MapOfAttachedItems" | "m_DoorsOpen" | "m_SavedDoorsMessagesName"
            )
        })
        .map(|(idx, _)| idx)
        .unwrap_or(refs.len());
    (start_idx + 1, end_idx, "player_inventory_region")
}

fn read_i32_property_at(payload: &[u8], refs: &[FStringRef], name_idx: usize) -> Option<i32> {
    let value_offset = i32_value_offset_at(payload, refs, name_idx)?;
    Some(i32::from_le_bytes(
        payload
            .get(value_offset..value_offset + 4)?
            .try_into()
            .ok()?,
    ))
}

fn read_f64_property_at(payload: &[u8], refs: &[FStringRef], name_idx: usize) -> Option<f64> {
    let type_ref = refs.get(name_idx + 1)?;
    if type_ref.value != "DoubleProperty" {
        return None;
    }
    let mut cursor = type_ref.len_offset + type_ref.total_len;
    if cursor + 17 > payload.len() {
        return None;
    }
    cursor += 4; // flags
    let size = u32::from_le_bytes(payload.get(cursor..cursor + 4)?.try_into().ok()?) as usize;
    cursor += 4;
    cursor += 1; // tag
    if size != 8 || cursor + 8 > payload.len() {
        return None;
    }
    Some(f64::from_le_bytes(
        payload.get(cursor..cursor + 8)?.try_into().ok()?,
    ))
}

fn read_bool_property_at(payload: &[u8], refs: &[FStringRef], name_idx: usize) -> Option<bool> {
    let type_ref = refs.get(name_idx + 1)?;
    if type_ref.value != "BoolProperty" {
        return None;
    }
    let cursor = type_ref.len_offset + type_ref.total_len;
    if cursor + 9 > payload.len() {
        return None;
    }
    // A BoolProperty's value lives in the 0x10 (TAG_FLAG_BOOL_TRUE) bit of the
    // tag byte, matching the typed parser/writer. Test that bit rather than
    // "nonzero", or a false bool carrying another tag flag (e.g. 0x08) would be
    // misread as true and written back as true on the next save.
    Some(*payload.get(cursor + 8)? & properties::TAG_FLAG_BOOL_TRUE != 0)
}

fn i32_value_offset_at(payload: &[u8], refs: &[FStringRef], name_idx: usize) -> Option<usize> {
    let type_ref = refs.get(name_idx + 1)?;
    if type_ref.value != "IntProperty" {
        return None;
    }
    i32_value_offset_after_type_ref(payload, type_ref)
}

fn i32_value_offset_after_type_ref(payload: &[u8], type_ref: &FStringRef) -> Option<usize> {
    let mut cursor = type_ref.len_offset + type_ref.total_len;
    if cursor + 9 > payload.len() {
        return None;
    }
    cursor += 4; // flags
    let size = u32::from_le_bytes(payload.get(cursor..cursor + 4)?.try_into().ok()?) as usize;
    cursor += 4;
    cursor += 1; // tag
    if size != 4 || cursor + 4 > payload.len() {
        return None;
    }
    Some(cursor)
}

fn f32_value_offset_at(payload: &[u8], refs: &[FStringRef], name_idx: usize) -> Option<usize> {
    let type_ref = refs.get(name_idx + 1)?;
    if type_ref.value != "FloatProperty" {
        return None;
    }
    f32_value_offset_after_type_ref(payload, type_ref)
}

fn f32_value_offset_after_type_ref(payload: &[u8], type_ref: &FStringRef) -> Option<usize> {
    let mut cursor = type_ref.len_offset + type_ref.total_len;
    if cursor + 9 > payload.len() {
        return None;
    }
    cursor += 4; // flags
    let size = u32::from_le_bytes(payload.get(cursor..cursor + 4)?.try_into().ok()?) as usize;
    cursor += 4;
    cursor += 1; // tag
    if size != 4 || cursor + 4 > payload.len() {
        return None;
    }
    Some(cursor)
}

fn item_id_from_path(path: &str) -> String {
    path.rsplit(['/', '.'])
        .find(|part| !part.is_empty())
        .unwrap_or(path)
        .to_string()
}

fn looks_item_definition_path(value: &str) -> bool {
    value.starts_with("/Script/Angelscript.") || looks_inventory_candidate(value)
}

/// The bundled item catalog, embedded at build time. addItem only accepts a
/// path in this allow-list, so a typo or non-item class (even a well-formed
/// `/Script/Angelscript.It*` name) cannot persist an unresolvable
/// m_ItemDefinition. Regenerated by tools/build_item_catalog.py.
const ITEM_CATALOG_JSON: &str = include_str!("../../../apps/goresave/assets/item_catalog.json");

/// Set of valid item-definition asset paths from the bundled catalog.
fn item_catalog_paths() -> &'static std::collections::HashSet<String> {
    static PATHS: std::sync::OnceLock<std::collections::HashSet<String>> =
        std::sync::OnceLock::new();
    PATHS.get_or_init(|| {
        let entries: Vec<Value> = serde_json::from_str(ITEM_CATALOG_JSON).unwrap_or_default();
        entries
            .iter()
            .filter_map(|e| e.get("path").and_then(Value::as_str).map(String::from))
            .collect()
    })
}

/// Whether `path` is a writable item-definition class: present in the bundled
/// item catalog. addItem writes this into m_ItemDefinition, so anything not in
/// the catalog (typo, non-item class, or unknown asset) is rejected up front.
fn is_item_definition_class(path: &str) -> bool {
    item_catalog_paths().contains(path)
}

fn looks_inventory_candidate(value: &str) -> bool {
    if value.starts_with("m_") || value.starts_with("/Script/") {
        return false;
    }
    let lower = value.to_ascii_lowercase();
    if lower.starts_with("itmi_")
        || lower.starts_with("itfo_")
        || lower.starts_with("itmw_")
        || lower.starts_with("itar_")
        || lower.starts_with("itru_")
        || lower.starts_with("itpo_")
        || lower.starts_with("itke_")
        || lower.starts_with("itwr_")
    {
        return true;
    }
    lower.contains("/items/") || lower.contains("bp_item_") || lower.contains("inventoryitem")
}

fn contains_any_ci(value: &str, needles: &[&str]) -> bool {
    let lower = value.to_ascii_lowercase();
    needles.iter().any(|needle| lower.contains(needle))
}

fn is_property_type_name(value: &str) -> bool {
    matches!(
        value,
        "StrProperty"
            | "NameProperty"
            | "ObjectProperty"
            | "BoolProperty"
            | "IntProperty"
            | "MapProperty"
            | "ArrayProperty"
            | "StructProperty"
            | "ByteProperty"
            | "EnumProperty"
            | "DoubleProperty"
            | "ClassProperty"
    )
}

fn unique_strings<'a>(values: impl Iterator<Item = &'a str>, limit: usize) -> Vec<String> {
    let mut out = Vec::new();
    for value in values {
        if out.iter().any(|existing| existing == value) {
            continue;
        }
        out.push(value.to_string());
        if out.len() >= limit {
            break;
        }
    }
    out
}

fn check_codec(payload: &Value) -> Result<Value, CoreError> {
    let pure_backend = codec_backend::PureRustKrakenBackend;
    let pure_probe = codec_backend::CodecBackend::probe(&pure_backend)?;
    let binary_probe = payload.get("binaryHost").map(probe_binary_host_from_config);
    codec_status_from_probes(pure_probe, binary_probe)
}

/// Build the user-facing codec fields shown in the UI. `error_text` is the raw
/// host/probe error string when the binary backend probe failed (used only to
/// categorize, never shown verbatim to the user).
fn codec_user_message_for(
    selected_backend: &str,
    available: bool,
    can_compress: bool,
    can_decompress: bool,
    error_text: Option<&str>,
) -> Value {
    // Pure-Rust backend selected (no game codec needed for read-only flows):
    // leave user fields empty; callers fall back to existing behavior.
    if selected_backend != "g1r_binary_host" {
        return json!({});
    }
    if let Some(err) = error_text {
        let lower = err.to_ascii_lowercase();
        // Game executable missing (host: "G1R executable not found: <path>").
        if lower.contains("executable not found") {
            return json!({
                "userSeverity": "error",
                "userTitle": "Gothic 1 Remake not found",
                "userMessage": "The game executable wasn't found at the saved path.",
                "userHint": "Set the game path in settings.",
            });
        }
        // The build resolves but its codec can't be verified (new/unknown build
        // or a failed calibration) -- the genuine "wait for an editor update" case.
        if lower.contains("could not be resolved")
            || lower.contains("calibration did not produce")
            || lower.contains("unsupported")
        {
            return json!({
                "userSeverity": "error",
                "userTitle": "This game version can't be opened yet",
                "userMessage": "Looks like a new game update the editor doesn't recognize yet.",
                "userHint": "Check for an editor update - a new version usually follows shortly.",
            });
        }
        // Anything else (missing/misconfigured codec helper, IO or launch
        // failures) is a local setup problem the user can fix in Settings, not a
        // new game build to wait out.
        return json!({
            "userSeverity": "error",
            "userTitle": "Codec helper isn't set up",
            "userMessage": "The editor couldn't start its codec helper for this game.",
            "userHint": "Check the codec helper and game paths in settings.",
        });
    }
    if can_compress {
        return json!({
            "userSeverity": "ok",
            "userTitle": "Game codec ready",
            "userMessage": "The editor can read and write this game version.",
            "userHint": "",
        });
    }
    if available || can_decompress {
        // Usable for reading, but compression is not verified so writing stays
        // gated -- do not claim full "ready".
        return json!({
            "userSeverity": "warn",
            "userTitle": "Game codec partly ready",
            "userMessage": "The editor can read this game's saves, but saving isn't verified yet.",
            "userHint": "",
        });
    }
    json!({
        "userSeverity": "error",
        "userTitle": "This game version can't be opened yet",
        "userMessage": "Looks like a new game update the editor doesn't recognize yet.",
        "userHint": "Check for an editor update - a new version usually follows shortly.",
    })
}

fn codec_status_from_probes(
    pure_probe: codec_backend::CodecBackendProbe,
    binary_probe: Option<Result<codec_backend::CodecBackendProbe, CoreError>>,
) -> Result<Value, CoreError> {
    let mut backends =
        vec![serde_json::to_value(&pure_probe).map_err(|e| CoreError::Codec(e.to_string()))?];
    let mut selected_probe = pure_probe.clone();
    let mut binary_error: Option<String> = None;
    let mut binary_host_attempted = false;
    let mut binary_available = false;
    let mut binary_can_compress = false;
    let mut binary_can_decompress = false;

    if let Some(binary_probe) = binary_probe {
        match binary_probe {
            Ok(probe) => {
                backends.push(
                    serde_json::to_value(&probe).map_err(|e| CoreError::Codec(e.to_string()))?,
                );
                binary_host_attempted = true;
                binary_available = probe.available;
                binary_can_compress = probe.can_compress;
                binary_can_decompress = probe.can_decompress;
                if probe.available {
                    selected_probe = probe;
                }
            }
            Err(err) => {
                binary_host_attempted = true;
                binary_error = Some(err.to_string());
                backends.push(json!({
                    "backend": "g1r_binary_host",
                    "available": false,
                    "canDecompress": false,
                    "canCompress": false,
                    "status": "probe_failed",
                    "profile": null,
                    "resolutionMode": null,
                    "details": {},
                    "error": err.to_string()
                }));
            }
        }
    }

    let mut value = selected_probe.details.clone();
    value["selectedBackend"] = json!(selected_probe.backend);
    value["adapter"] = json!(selected_probe.backend);
    value["available"] = json!(selected_probe.available);
    value["canDecompress"] = json!(selected_probe.can_decompress);
    value["canCompress"] = json!(selected_probe.can_compress);
    value["profile"] = json!(selected_probe.profile);
    value["resolutionMode"] = json!(selected_probe.resolution_mode);
    value["backends"] = json!(backends);

    if value["selectedBackend"] == "g1r_binary_host" {
        value["status"] = json!(binary_host_status(&selected_probe));
        value["message"] = json!(binary_host_message(&selected_probe));
    }

    let user = if binary_host_attempted {
        codec_user_message_for(
            "g1r_binary_host",
            binary_available,
            binary_can_compress,
            binary_can_decompress,
            binary_error.as_deref(),
        )
    } else {
        // No binary host configured: pure-Rust selected, no user-facing codec
        // message (read-only flows).
        codec_user_message_for(
            &selected_probe.backend,
            selected_probe.available,
            selected_probe.can_compress,
            selected_probe.can_decompress,
            None,
        )
    };
    if let Some(obj) = user.as_object() {
        for (k, v) in obj {
            value[k.clone()] = v.clone();
        }
    }

    Ok(value)
}

fn binary_host_status(probe: &codec_backend::CodecBackendProbe) -> &'static str {
    if probe.can_compress {
        "codec_host_ready"
    } else if probe.can_decompress {
        "codec_host_decompress_ready"
    } else if probe.available {
        "codec_host_supported_needs_runtime_selftest"
    } else {
        "codec_host_unavailable"
    }
}

fn binary_host_message(probe: &codec_backend::CodecBackendProbe) -> &'static str {
    if probe.can_compress {
        "G1R codec host is configured and verified for compress/decompress."
    } else if probe.can_decompress {
        "G1R codec host is configured and verified for decompression; compression is not enabled yet."
    } else if probe.available {
        "G1R codec host resolved the game codec profile. Run a runtime selftest before enabling private writes."
    } else {
        "G1R codec host is configured but not available."
    }
}

fn probe_binary_host_from_config(
    config: &Value,
) -> Result<codec_backend::CodecBackendProbe, CoreError> {
    let backend = binary_host_backend_from_config(config)?;
    let probe = codec_backend::CodecBackend::probe(&backend)?;
    Ok(auto_calibrate_if_pattern_profile(
        &backend,
        probe,
        &host_config_suffix(config),
    ))
}

/// The host configuration, besides the executable, that affects codec
/// resolution: the helper used to run the host and the derived-profile cache it
/// reads/writes. Used to scope both the ensured-backend cache and the
/// calibration attempt budget, so changing either (e.g. fixing a bad helper
/// path) is treated as a fresh configuration rather than reusing stale state.
fn host_config_suffix(config: &Value) -> String {
    let field = |name: &str| config.get(name).and_then(Value::as_str).unwrap_or("");
    format!(
        "{}\u{1f}{}",
        field("helperPath"),
        field("derivedProfileCachePath"),
    )
}

/// Executables whose calibration has already been ensured this session, keyed by
/// exe path. Prevents re-probing (which hashes the whole game executable) on
/// every codec operation once the build has been calibrated.
fn binary_host_ensured_exes() -> &'static std::sync::Mutex<std::collections::HashSet<String>> {
    static ENSURED: std::sync::OnceLock<std::sync::Mutex<std::collections::HashSet<String>>> =
        std::sync::OnceLock::new();
    ENSURED.get_or_init(|| std::sync::Mutex::new(std::collections::HashSet::new()))
}

/// Build the binary host backend for a codec OPERATION (decode/encode), ensuring
/// a pattern-resolved build has been calibrated first. `check_codec` is not the
/// only entry point -- `inspect_save` and friends decode too, and on startup the
/// app inspects in parallel with `check_codec`. Without this, the first inspect
/// of an unknown build would fail (the host rejects decode for `pattern_profile`)
/// until a separate `check_codec` happened to write the derived cache. Ensuring
/// calibration here writes that cache before this backend is used. Runs at most
/// once per executable per session; a no-op for known profiles or when the probe
/// fails (e.g. no helper).
fn ensured_binary_host_from_config(
    config: &Value,
) -> Result<codec_backend::G1rBinaryHostBackend, CoreError> {
    let backend = binary_host_backend_from_config(config)?;
    // Key by the full host configuration that affects resolution, not just the
    // exe path: the same executable used with a different derived-profile cache
    // (or helper) needs its own probe/calibration. (The session cache still
    // assumes the executable bytes at a given path are stable for the session;
    // an in-place game update would require an explicit codec re-check.)
    let exe_key = config
        .get("exePath")
        .and_then(Value::as_str)
        .map(|exe| format!("{exe}\u{1f}{}", host_config_suffix(config)));
    if let Some(key) = &exe_key {
        if binary_host_ensured_exes()
            .lock()
            .map(|set| set.contains(key))
            .unwrap_or(false)
        {
            return Ok(backend);
        }
    }
    // Probe and (for an untrusted build) calibrate. Only mark the executable
    // "ensured" -- to skip this re-probe on later operations -- once it is fully
    // usable: available AND write-capable. A probe failure, an unsupported build,
    // or a decode-only build (available but not yet compress-verified) is left
    // unmarked so a later operation retries calibration (e.g. after the user
    // fixes Settings, within the attempt budget, or to verify compression),
    // instead of being stuck for the session.
    let fully_usable = match codec_backend::CodecBackend::probe(&backend) {
        // Side effect: a pattern-resolved build writes its derived cache here, so
        // the decode/encode that follows on this backend resolves it.
        Ok(probe) => {
            let p = auto_calibrate_if_pattern_profile(&backend, probe, &host_config_suffix(config));
            p.available && p.can_compress
        }
        Err(_) => false,
    };
    if fully_usable {
        if let Some(key) = exe_key {
            if let Ok(mut set) = binary_host_ensured_exes().lock() {
                set.insert(key);
            }
        }
    }
    Ok(backend)
}

/// Maximum failed calibration attempts per executable per session. A failed
/// calibration runs the expensive runtime selftest; without a cap, every codec
/// check on an unpromotable build would re-run it. A small budget still lets a
/// transient failure -- or a setup the user just fixed -- recover on a retry,
/// then stops. (Classifying durable vs transient failures by error text is
/// unreliable; a bounded attempt count is robust and simple.)
const MAX_CALIBRATION_ATTEMPTS: u32 = 2;

/// Process-global count of failed calibration attempts per executable SHA-256.
/// The core runs as a long-lived FFI library, so this persists for the session.
/// Keyed by SHA-256 so a different (e.g. newly patched) build is independent.
fn calibration_attempts() -> &'static std::sync::Mutex<std::collections::HashMap<String, u32>> {
    static ATTEMPTS: std::sync::OnceLock<std::sync::Mutex<std::collections::HashMap<String, u32>>> =
        std::sync::OnceLock::new();
    ATTEMPTS.get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()))
}

/// Auto-calibration: an untrusted (pattern-resolved) or decode-only build is
/// promoted to a verified write-capable derived profile by running one runtime
/// selftest with the host's embedded sample. Best-effort: a failure leaves the
/// build unpromoted (the UI shows a plain message). Bounded so an unpromotable
/// build does not re-run the selftest forever.
fn auto_calibrate_if_pattern_profile(
    backend: &codec_backend::G1rBinaryHostBackend,
    probe: codec_backend::CodecBackendProbe,
    config_suffix: &str,
) -> codec_backend::CodecBackendProbe {
    auto_calibrate_bounded(backend, probe, config_suffix, calibration_attempts())
}

fn auto_calibrate_bounded(
    backend: &codec_backend::G1rBinaryHostBackend,
    probe: codec_backend::CodecBackendProbe,
    config_suffix: &str,
    attempts: &std::sync::Mutex<std::collections::HashMap<String, u32>>,
) -> codec_backend::CodecBackendProbe {
    // Calibrate an untrusted (pattern-resolved) build OR one usable only for
    // decompression (a decode-only derived-cache entry that can be promoted to
    // write-capable). Write-capable supported profiles and the pure-Rust
    // fallback need no calibration.
    let needs_calibration = probe.resolution_mode.as_deref() == Some("pattern_profile")
        || (probe.available && !probe.can_compress);
    if !needs_calibration {
        return probe;
    }
    // Scope the attempt budget by executable AND host configuration, so fixing a
    // bad helper/cache path mid-session is a fresh build to retry rather than a
    // capped one (matching the ensured-backend cache key).
    let exe_sha = probe
        .details
        .get("exeSha256")
        .and_then(Value::as_str)
        .map(|sha| format!("{sha}\u{1f}{config_suffix}"));
    // Give up re-running the expensive selftest once an executable has failed the
    // capped number of times this session.
    if let Some(sha) = &exe_sha {
        if attempts
            .lock()
            .map(|m| m.get(sha).copied().unwrap_or(0) >= MAX_CALIBRATION_ATTEMPTS)
            .unwrap_or(false)
        {
            return probe;
        }
    }
    match backend.calibrate() {
        Ok(calibrated) => calibrated,
        Err(_) => {
            // A failed calibration may still have written a decode-only derived
            // cache. Re-probe so the response reflects that (read-only usable)
            // instead of the stale pre-calibration probe.
            let after =
                codec_backend::CodecBackend::probe(backend).unwrap_or_else(|_| probe.clone());
            // Count every failed calibration against the budget -- including a
            // decode-only outcome (available but not write-capable). It still gets
            // up to MAX_CALIBRATION_ATTEMPTS retries (a transient compress failure
            // may clear), but a build whose compressor consistently fails will not
            // re-run the expensive selftest on every operation forever.
            if let Some(sha) = exe_sha {
                if let Ok(mut map) = attempts.lock() {
                    *map.entry(sha).or_insert(0) += 1;
                }
            }
            after
        }
    }
}

fn binary_host_backend_from_config(
    config: &Value,
) -> Result<codec_backend::G1rBinaryHostBackend, CoreError> {
    let helper_path = config
        .get("helperPath")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            CoreError::InvalidRequest("binaryHost.helperPath is required".to_string())
        })?;
    let exe_path = config
        .get("exePath")
        .and_then(Value::as_str)
        .ok_or_else(|| CoreError::InvalidRequest("binaryHost.exePath is required".to_string()))?;
    let mut backend = codec_backend::G1rBinaryHostBackend::new(helper_path, exe_path);
    if let Some(cache_path) = config
        .get("derivedProfileCachePath")
        .and_then(Value::as_str)
    {
        backend = backend.with_derived_profile_cache_path(cache_path);
    }
    Ok(backend)
}

fn validate_roundtrip(path: &Path) -> Result<Value, CoreError> {
    let data = fs::read(path)?;
    if !data.starts_with(b"GSAV") {
        return Ok(json!({
            "path": path,
            "format": if data.starts_with(b"GVAS") { "GVAS" } else { "UNKNOWN" },
            "identical": true,
            "message": "Roundtrip rebuild is only needed for GSAV containers; this file is unchanged."
        }));
    }
    let rebuilt = rebuild_gsav_preserving_stream(&data)?;
    Ok(json!({
        "path": path,
        "format": "GSAV",
        "identical": rebuilt == data,
        "originalSize": data.len(),
        "rebuiltSize": rebuilt.len(),
        "originalSha1": sha1_hex(&data),
        "rebuiltSha1": sha1_hex(&rebuilt),
    }))
}

fn validate_codec_roundtrip_with_backend(
    path: &Path,
    backend: &dyn codec_backend::CodecBackend,
) -> Result<Value, CoreError> {
    let data = fs::read(path)?;
    if !data.starts_with(b"GSAV") {
        return Err(CoreError::UnsupportedEdit(
            "codec roundtrip validation is only available for GSAV files".to_string(),
        ));
    }
    let parts = split_gsav(&data)?;
    let stream = parse_compressed_stream(&data, 13 + parts.public_payload.len())?;
    let chunk = stream
        .chunks
        .first()
        .ok_or_else(|| CoreError::Validation("save has no private chunks".to_string()))?;
    let compressed_start = chunk.compressed_offset;
    let compressed_end = compressed_start
        .checked_add(chunk.compressed_size as usize)
        .ok_or_else(|| CoreError::Parse("compressed chunk range overflow".to_string()))?;
    let compressed = data.get(compressed_start..compressed_end).ok_or_else(|| {
        CoreError::Parse(format!(
            "compressed chunk {} points outside the save",
            chunk.index
        ))
    })?;
    let decoded = backend.decompress(compressed, chunk.uncompressed_size as usize)?;
    let recompressed = backend.compress(&decoded, 6)?;
    let roundtrip = backend.decompress(&recompressed, decoded.len())?;
    if roundtrip != decoded {
        return Err(CoreError::Codec(
            "codec roundtrip output did not match decoded chunk".to_string(),
        ));
    }
    Ok(json!({
        "status": "codec_roundtrip_passed",
        "path": path,
        "chunkIndex": chunk.index,
        "originalCompressedSize": chunk.compressed_size,
        "decompressedSize": decoded.len(),
        "recompressedSize": recompressed.len(),
        "method": stream.method,
        "algorithmId": stream.algorithm_id,
    }))
}

fn rebuild_gsav_preserving_stream(data: &[u8]) -> Result<Vec<u8>, CoreError> {
    let parts = split_gsav(data)?;
    Ok(build_gsav(
        parts.version,
        parts.public_payload,
        parts.compressed_stream,
        parts.trailer,
    ))
}

struct GsavParts<'a> {
    version: u8,
    public_payload: &'a [u8],
    compressed_stream: &'a [u8],
    trailer: &'a [u8],
}

fn split_gsav(data: &[u8]) -> Result<GsavParts<'_>, CoreError> {
    if !data.starts_with(b"GSAV") {
        return Err(CoreError::Parse("not a GSAV file".to_string()));
    }
    if data.len() < 13 {
        return Err(CoreError::Parse(
            "GSAV file is shorter than header".to_string(),
        ));
    }
    let version = data[4];
    let public_payload_size = u32::from_le_bytes(data[9..13].try_into().unwrap()) as usize;
    let public_start = 13usize;
    let public_end = public_start + public_payload_size;
    if public_end > data.len() {
        return Err(CoreError::Parse(
            "public payload extends past EOF".to_string(),
        ));
    }
    let stream = parse_compressed_stream(data, public_end)?;
    Ok(GsavParts {
        version,
        public_payload: &data[public_start..public_end],
        compressed_stream: &data[public_end..stream.stream_end_offset],
        trailer: &data[stream.stream_end_offset..],
    })
}

fn build_gsav(
    version: u8,
    public_payload: &[u8],
    compressed_stream: &[u8],
    trailer: &[u8],
) -> Vec<u8> {
    let body_size = 13 + public_payload.len() + compressed_stream.len();
    let mut out = Vec::with_capacity(body_size + trailer.len());
    out.extend_from_slice(b"GSAV");
    out.push(version);
    out.extend_from_slice(&(body_size as u32).to_le_bytes());
    out.extend_from_slice(&(public_payload.len() as u32).to_le_bytes());
    out.extend_from_slice(public_payload);
    out.extend_from_slice(compressed_stream);
    out.extend_from_slice(trailer);
    out
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Edit {
    path: String,
    value: Value,
}

struct PersistentDataListSyncPlan {
    path: PathBuf,
    slot: String,
    player_save_name: String,
    original: Vec<u8>,
    edited: Vec<u8>,
}

#[cfg(test)]
fn write_save(
    path: &Path,
    raw_edits: &[Value],
    backup: bool,
    output_path: Option<&Path>,
) -> Result<Value, CoreError> {
    write_save_with_codec_backend(path, raw_edits, backup, output_path, None)
}

#[cfg(test)]
fn write_save_with_codec_backend(
    path: &Path,
    raw_edits: &[Value],
    backup: bool,
    output_path: Option<&Path>,
    codec_backend: Option<&dyn codec_backend::CodecBackend>,
) -> Result<Value, CoreError> {
    write_save_internal(path, raw_edits, backup, output_path, codec_backend, false)
}

fn write_save_internal(
    path: &Path,
    raw_edits: &[Value],
    backup: bool,
    output_path: Option<&Path>,
    codec_backend: Option<&dyn codec_backend::CodecBackend>,
    sync_persistent_data_list: bool,
) -> Result<Value, CoreError> {
    if sync_persistent_data_list && output_path.is_some() {
        return Err(CoreError::InvalidRequest(
            "syncPersistentDataList cannot be used with outputPath".to_string(),
        ));
    }
    let original = fs::read(path)?;
    let mut edited = original.clone();
    let edits = raw_edits
        .iter()
        .map(|v| serde_json::from_value::<Edit>(v.clone()))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| CoreError::InvalidRequest(e.to_string()))?;

    for edit in edits
        .iter()
        .filter(|edit| !edit.path.starts_with("private."))
    {
        apply_public_edit(&mut edited, edit)?;
    }
    let private_edits = edits
        .iter()
        .filter(|edit| edit.path.starts_with("private."))
        .collect::<Vec<_>>();
    if !private_edits.is_empty() {
        edited = apply_private_edits(&edited, &private_edits, codec_backend)?;
    }

    inspect_bytes(&edited, None, false)?;
    if edited.starts_with(b"GSAV") {
        let rebuilt = rebuild_gsav_preserving_stream(&edited)?;
        if rebuilt != edited {
            return Err(CoreError::Validation(
                "edited GSAV does not rebuild byte-identically".to_string(),
            ));
        }
    }

    let persistent_sync = if sync_persistent_data_list {
        prepare_persistent_data_list_sync(path, &edits)?
    } else {
        None
    };
    let target = output_path.unwrap_or(path);
    let companion_backup_target = match &persistent_sync {
        Some(plan) if backup && output_path.is_none() && plan.original != plan.edited => {
            Some(plan.path.as_path())
        }
        _ => None,
    };
    let (backup_path, persistent_backup_path) = if backup && output_path.is_none() {
        match companion_backup_target {
            // Back up both files under one shared suffix so restore can pair
            // them by suffix even if creation straddles a one-second boundary.
            Some(companion) => {
                let suffix = shared_backup_suffix(&[path, companion]);
                (
                    Some(create_backup_with_suffix(path, &suffix)?),
                    Some(create_backup_with_suffix(companion, &suffix)?),
                )
            }
            None => (Some(create_backup_copy(path)?), None),
        }
    } else {
        (None, None)
    };

    let tmp_path = target.with_extension("sav.tmp-goresave");
    fs::write(&tmp_path, &edited)?;
    inspect_save(&tmp_path, false)?;
    let persistent_tmp_path = if let Some(plan) = &persistent_sync {
        if plan.original != plan.edited {
            let tmp_path = plan.path.with_extension("sav.tmp-goresave");
            fs::write(&tmp_path, &plan.edited)?;
            validate_persistent_data_list_sync(plan, &tmp_path)?;
            Some(tmp_path)
        } else {
            None
        }
    } else {
        None
    };
    // Replace the slot first, then the synced PersistentDataList; if the
    // companion replace fails, roll the slot write back so the two files never
    // diverge.
    let slot_pending = begin_replace(target, &tmp_path)?;
    if let Some(tmp_path) = &persistent_tmp_path {
        let plan = persistent_sync
            .as_ref()
            .expect("persistent_tmp_path implies a sync plan");
        match begin_replace(&plan.path, tmp_path) {
            Ok(companion_pending) => {
                companion_pending.commit();
                slot_pending.commit();
            }
            Err(err) => {
                slot_pending.rollback();
                return Err(err);
            }
        }
    } else {
        slot_pending.commit();
    }

    // The save bytes changed on disk; drop any cached decoded payload so the
    // typed property browser re-decodes the edited save on its next search.
    invalidate_decoded_payload_cache(target);

    Ok(json!({
        "path": target,
        "backupPath": backup_path,
        "editsApplied": edits.len(),
        "originalSha1": sha1_hex(&original),
        "writtenSha1": sha1_hex(&edited),
        "bytesChanged": original != edited,
        "persistentPath": persistent_sync.as_ref().map(|plan| plan.path.display().to_string()),
        "persistentBackupPath": persistent_backup_path,
        "persistentOriginalSha1": persistent_sync.as_ref().map(|plan| sha1_hex(&plan.original)),
        "persistentWrittenSha1": persistent_sync.as_ref().map(|plan| sha1_hex(&plan.edited)),
        "persistentBytesChanged": persistent_sync
            .as_ref()
            .map(|plan| plan.original != plan.edited)
            .unwrap_or(false),
    }))
}

fn prepare_persistent_data_list_sync(
    save_path: &Path,
    edits: &[Edit],
) -> Result<Option<PersistentDataListSyncPlan>, CoreError> {
    let Some(player_save_name) = player_save_name_edit_value(edits) else {
        return Ok(None);
    };
    let Some(slot) = save_path.file_stem().and_then(|value| value.to_str()) else {
        return Ok(None);
    };
    if !looks_slot_name(slot) {
        return Ok(None);
    }
    let Some(parent) = save_path.parent() else {
        return Ok(None);
    };
    let path = parent.join("PersistentDataList.sav");
    if !path.exists() {
        return Ok(None);
    }
    let original = fs::read(&path)?;
    let mut edited = original.clone();
    if !replace_persistent_slot_player_save_name(&mut edited, slot, player_save_name)? {
        // The companion list exists and a sync was requested, but it has no
        // entry for this slot. Abort instead of silently writing the slot file
        // and leaving PersistentDataList.sav showing the old player save name.
        return Err(CoreError::Validation(format!(
            "PersistentDataList.sav has no entry for slot {slot}; cannot sync the \
             player save name without leaving the slot file and companion list out of sync"
        )));
    }
    validate_persistent_data_list_bytes(slot, player_save_name, &edited)?;
    Ok(Some(PersistentDataListSyncPlan {
        path,
        slot: slot.to_string(),
        player_save_name: player_save_name.to_string(),
        original,
        edited,
    }))
}

fn player_save_name_edit_value(edits: &[Edit]) -> Option<&str> {
    edits
        .iter()
        .rev()
        .find(|edit| edit.path == "public.m_PlayerSaveName")
        .and_then(|edit| edit.value.as_str())
}

fn replace_persistent_slot_player_save_name(
    data: &mut Vec<u8>,
    slot: &str,
    new_value: &str,
) -> Result<bool, CoreError> {
    if !data.starts_with(b"GVAS") {
        return Err(CoreError::Parse(
            "PersistentDataList.sav is not a GVAS file".to_string(),
        ));
    }
    let refs = scan_fstrings(data, 0);
    let Some((start_idx, end_idx)) = persistent_slot_ref_range(&refs, slot) else {
        return Ok(false);
    };
    replace_str_property_fstring_in_range(
        data,
        &refs,
        start_idx,
        end_idx,
        "m_PlayerSaveName",
        new_value,
    )?;
    Ok(true)
}

fn validate_persistent_data_list_sync(
    plan: &PersistentDataListSyncPlan,
    tmp_path: &Path,
) -> Result<(), CoreError> {
    let written = fs::read(tmp_path)?;
    validate_persistent_data_list_bytes(&plan.slot, &plan.player_save_name, &written)
}

fn validate_persistent_data_list_bytes(
    slot: &str,
    player_save_name: &str,
    data: &[u8],
) -> Result<(), CoreError> {
    let metadata = parse_persistent_slot_metadata(data);
    let actual = metadata
        .get(slot)
        .and_then(|metadata| metadata.player_save_name.as_deref());
    if actual != Some(player_save_name) {
        return Err(CoreError::Validation(format!(
            "PersistentDataList.sav did not retain synced m_PlayerSaveName for {slot}"
        )));
    }
    Ok(())
}

fn apply_public_edit(data: &mut Vec<u8>, edit: &Edit) -> Result<(), CoreError> {
    match edit.path.as_str() {
        "public.m_PlayerSaveName" => {
            let value = edit.value.as_str().ok_or_else(|| {
                CoreError::InvalidRequest("m_PlayerSaveName must be a string".to_string())
            })?;
            replace_public_fstring(data, "m_PlayerSaveName", value)
        }
        other => Err(CoreError::UnsupportedEdit(format!(
            "{other} is not writable in this build"
        ))),
    }
}

fn apply_private_edits(
    data: &[u8],
    edits: &[&Edit],
    codec_backend: Option<&dyn codec_backend::CodecBackend>,
) -> Result<Vec<u8>, CoreError> {
    if !data.starts_with(b"GSAV") {
        return Err(CoreError::UnsupportedEdit(
            "private edits are only available for GSAV files".to_string(),
        ));
    }
    let backend = codec_backend.ok_or_else(|| {
        CoreError::Codec(
            "private edits require a configured and verified G1R codec host".to_string(),
        )
    })?;
    let parts = split_gsav(data)?;
    let stream = parse_compressed_stream(data, 13 + parts.public_payload.len())?;
    let edit_specs =
        edits
            .iter()
            .map(|edit| match edit.path.as_str() {
                "private.replaceFString" | "private.fstring" => {
                    parse_private_fstring_edit(edit).map(PrivateEdit::FString)
                }
                "private.player.setPlayerName" => {
                    parse_private_player_name_edit(edit).map(PrivateEdit::PlayerName)
                }
                "private.profile.setProfileName" => {
                    parse_private_profile_name_edit(edit).map(PrivateEdit::ProfileName)
                }
                "private.player.setAttribute" => {
                    parse_private_player_attribute_edit(edit).map(PrivateEdit::PlayerAttribute)
                }
                "private.player.setTransform" => {
                    parse_private_player_transform_edit(edit).map(PrivateEdit::PlayerTransform)
                }
                "private.inventory.setItemCount" => parse_private_inventory_item_count_edit(edit)
                    .map(PrivateEdit::InventoryItemCount),
                "private.inventory.addItem" => {
                    parse_private_inventory_add_item_edit(edit).map(PrivateEdit::InventoryAddItem)
                }
                "private.inventory.removeItem" => parse_private_inventory_remove_item_edit(edit)
                    .map(PrivateEdit::InventoryRemoveItem),
                "private.typed.setValue" => {
                    parse_private_typed_set_value_edit(edit).map(PrivateEdit::TypedSetValue)
                }
                // Index-addressed edits (arrayRemove/arrayDuplicate) target indices
                // that shift after each structural change within the same batch;
                // callers must submit at most one structural array edit per write.
                // Each edit re-parses the payload, so value-addressed ops
                // (setAdd/setRemove) batch safely.
                "private.typed.setAdd"
                | "private.typed.setRemove"
                | "private.typed.arrayRemove"
                | "private.typed.arrayDuplicate" => {
                    parse_private_typed_container_edit(edit, edit.path.as_str())
                        .map(PrivateEdit::TypedContainer)
                }
                "private.knowledge.addCharacter" => {
                    let name = edit
                        .value
                        .get("value")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| {
                            CoreError::InvalidRequest(
                                "private.knowledge.addCharacter requires a string `value`"
                                    .to_string(),
                            )
                        })?
                        .to_string();
                    Ok(PrivateEdit::KnowledgeAddCharacter(name))
                }
                other => Err(CoreError::UnsupportedEdit(format!(
                    "{other} is not writable in this build"
                ))),
            })
            .collect::<Result<Vec<_>, _>>()?;
    // Structural edits (arrayRemove, arrayDuplicate, addItem, removeItem) change
    // the length of array or set data; a second such edit in the same batch
    // would silently target shifted offsets/indices.  Reject the batch
    // instead of guessing the caller's intent.
    let structural_array_edits = edit_specs
        .iter()
        .filter(|edit| {
            matches!(
                edit,
                PrivateEdit::TypedContainer(PrivateTypedContainerEdit {
                    edit: properties::ContainerEdit::ArrayRemove(_)
                        | properties::ContainerEdit::ArrayDuplicate(_),
                    ..
                }) | PrivateEdit::InventoryAddItem(_)
                    | PrivateEdit::InventoryRemoveItem(_)
            )
        })
        .count();
    if structural_array_edits > 1 {
        return Err(CoreError::UnsupportedEdit(format!(
            "a write may contain at most one structural array edit \
             (arrayRemove/arrayDuplicate/addItem/removeItem); got {structural_array_edits} — \
             indices shift after each structural change, submit them as \
             separate writes"
        )));
    }
    // A splicing structural edit inserts or removes bytes mid-payload and shifts
    // every byte after the splice point:
    //   - inventory addItem/removeItem splice the MainContainer slot array, and
    //   - knowledge.addCharacter inserts a new entry into the
    //     CharacterKnowledgeByUniqueName MapProperty.
    // Any peer edit in the same batch is unsafe: a later edit resolves its target
    // against the pre-splice layout — an in-place setItemCount patches stale byte
    // offsets, and a typed setValue re-resolves a now-shifted array index — so it
    // can corrupt the save or hit the wrong slot. Require such an edit to stand alone.
    let splicing_structural_edits = edit_specs
        .iter()
        .filter(|edit| {
            matches!(
                edit,
                PrivateEdit::InventoryAddItem(_)
                    | PrivateEdit::InventoryRemoveItem(_)
                    | PrivateEdit::KnowledgeAddCharacter(_)
            )
        })
        .count();
    if splicing_structural_edits >= 1 && edit_specs.len() > 1 {
        return Err(CoreError::UnsupportedEdit(
            "a write containing private.inventory.addItem, private.inventory.removeItem, \
             or private.knowledge.addCharacter must contain no other edits — the \
             structural splice (slot-array or map insert) shifts the byte offsets and \
             array indices later edits resolve against; submit them as separate writes"
                .to_string(),
        ));
    }
    let mut private_payload = decompress_private_payload(data, &stream, backend)?;
    for edit in &edit_specs {
        apply_private_edit_to_payload(&mut private_payload, edit)?;
    }
    let compressed_stream = rebuild_compressed_stream(&stream, &private_payload, backend)?;
    Ok(build_gsav(
        parts.version,
        parts.public_payload,
        &compressed_stream,
        parts.trailer,
    ))
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PrivateFStringEdit {
    old_value: String,
    new_value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PrivatePlayerNameEdit {
    name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PrivateProfileNameEdit {
    name: String,
}

#[derive(Debug, Clone, PartialEq)]
struct PrivatePlayerAttributeEdit {
    id: String,
    base_value: Option<f32>,
    current_value: Option<f32>,
}

#[derive(Debug, Clone, PartialEq)]
struct PrivateVector3Edit {
    x: f64,
    y: f64,
    z: f64,
}

#[derive(Debug, Clone, PartialEq)]
struct PrivateRotatorEdit {
    pitch: f64,
    yaw: f64,
    roll: f64,
}

#[derive(Debug, Clone, PartialEq)]
struct PrivatePlayerTransformEdit {
    location: Option<PrivateVector3Edit>,
    rotation: Option<PrivateRotatorEdit>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PrivateInventoryItemCountEdit {
    id: Option<String>,
    path: Option<String>,
    count: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PrivateInventoryAddItemEdit {
    path: String,
    count: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PrivateInventoryRemoveItemEdit {
    path: String,
}

#[derive(Debug, Clone, PartialEq)]
enum PrivateEdit {
    FString(PrivateFStringEdit),
    PlayerName(PrivatePlayerNameEdit),
    ProfileName(PrivateProfileNameEdit),
    PlayerAttribute(PrivatePlayerAttributeEdit),
    PlayerTransform(PrivatePlayerTransformEdit),
    InventoryItemCount(PrivateInventoryItemCountEdit),
    InventoryAddItem(PrivateInventoryAddItemEdit),
    InventoryRemoveItem(PrivateInventoryRemoveItemEdit),
    TypedSetValue(PrivateTypedSetValueEdit),
    TypedContainer(PrivateTypedContainerEdit),
    KnowledgeAddCharacter(String),
}

#[derive(Debug, Clone, PartialEq)]
struct PrivateTypedSetValueEdit {
    path: Vec<properties::PathSeg>,
    value: Value,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PrivateTypedContainerEdit {
    path: Vec<properties::PathSeg>,
    edit: properties::ContainerEdit,
}

fn parse_typed_edit_path(
    op: &str,
    value: &serde_json::Map<String, Value>,
) -> Result<Vec<properties::PathSeg>, CoreError> {
    let segments = value
        .get("path")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            CoreError::InvalidRequest(format!("{op} requires value.path as an array of segments"))
        })?
        .iter()
        .map(|segment| {
            segment.as_str().map(str::to_string).ok_or_else(|| {
                CoreError::InvalidRequest(format!("{op} path segments must be strings"))
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    if segments.is_empty() {
        return Err(CoreError::InvalidRequest(format!(
            "{op} requires a non-empty value.path"
        )));
    }
    properties::parse_path(&segments)
}

fn parse_private_typed_set_value_edit(edit: &Edit) -> Result<PrivateTypedSetValueEdit, CoreError> {
    let value = edit.value.as_object().ok_or_else(|| {
        CoreError::InvalidRequest("private.typed.setValue value must be an object".to_string())
    })?;
    let path = parse_typed_edit_path("private.typed.setValue", value)?;
    let new_value = value.get("value").cloned().ok_or_else(|| {
        CoreError::InvalidRequest("private.typed.setValue requires value.value".to_string())
    })?;
    Ok(PrivateTypedSetValueEdit {
        path,
        value: new_value,
    })
}

fn parse_private_typed_container_edit(
    edit: &Edit,
    op: &str,
) -> Result<PrivateTypedContainerEdit, CoreError> {
    let value = edit
        .value
        .as_object()
        .ok_or_else(|| CoreError::InvalidRequest(format!("{op} value must be an object")))?;
    let path = parse_typed_edit_path(op, value)?;
    let container_edit = match op {
        "private.typed.setAdd" | "private.typed.setRemove" => {
            let element = value
                .get("value")
                .and_then(Value::as_str)
                .filter(|s| !s.trim().is_empty())
                .ok_or_else(|| {
                    CoreError::InvalidRequest(format!(
                        "{op} requires a non-empty string value.value"
                    ))
                })?
                .to_string();
            if op == "private.typed.setAdd" {
                properties::ContainerEdit::SetAdd(element)
            } else {
                properties::ContainerEdit::SetRemove(element)
            }
        }
        "private.typed.arrayRemove" | "private.typed.arrayDuplicate" => {
            let index = value
                .get("index")
                .and_then(Value::as_u64)
                .and_then(|v| usize::try_from(v).ok())
                .ok_or_else(|| {
                    CoreError::InvalidRequest(format!(
                        "{op} requires a non-negative integer value.index"
                    ))
                })?;
            if op == "private.typed.arrayRemove" {
                properties::ContainerEdit::ArrayRemove(index)
            } else {
                properties::ContainerEdit::ArrayDuplicate(index)
            }
        }
        other => {
            return Err(CoreError::UnsupportedEdit(format!(
                "{other} is not a typed container edit"
            )));
        }
    };
    Ok(PrivateTypedContainerEdit {
        path,
        edit: container_edit,
    })
}

/// Replacement value for `private.typed.setValue`: fixed-size scalars patch
/// in place, string-valued properties (Str/Name/Object/Enum and the
/// enum-as-byte form of ByteProperty) may change the payload length.
#[derive(Debug, Clone, PartialEq)]
enum TypedSetValue {
    Scalar(properties::ScalarValue),
    Text(String),
}

fn coerce_typed_value(
    property: &properties::Property,
    value: &Value,
) -> Result<TypedSetValue, CoreError> {
    use properties::ScalarValue;
    let type_name = property.type_name.as_str();
    let err = |expected: &str| {
        CoreError::InvalidRequest(format!(
            "private.typed.setValue: property is {type_name}, value must be {expected}"
        ))
    };
    let scalar = |s: ScalarValue| TypedSetValue::Scalar(s);
    match type_name {
        "IntProperty" => value
            .as_i64()
            .and_then(|v| i32::try_from(v).ok())
            .map(|v| scalar(ScalarValue::Int(v)))
            .ok_or_else(|| err("an i32 integer")),
        "UInt32Property" => value
            .as_u64()
            .and_then(|v| u32::try_from(v).ok())
            .map(|v| scalar(ScalarValue::UInt32(v)))
            .ok_or_else(|| err("a u32 integer")),
        "Int64Property" => value
            .as_i64()
            .map(|v| scalar(ScalarValue::Int64(v)))
            .ok_or_else(|| err("an i64 integer")),
        "FloatProperty" => value
            .as_f64()
            .filter(|v| v.is_finite() && (*v as f32).is_finite())
            .map(|v| scalar(ScalarValue::Float(v as f32)))
            .ok_or_else(|| err("a finite number")),
        "DoubleProperty" => value
            .as_f64()
            .filter(|v| v.is_finite())
            .map(|v| scalar(ScalarValue::Double(v)))
            .ok_or_else(|| err("a finite number")),
        "BoolProperty" => value
            .as_bool()
            .map(|v| scalar(ScalarValue::Bool(v)))
            .ok_or_else(|| err("a boolean")),
        // ByteProperty has two serialized forms: a plain byte (scalar patch in
        // place) and an enum-as-FString (string patch). Dispatch on the parsed
        // value, not the tag type.
        "ByteProperty" => match property.value {
            properties::PropertyValue::Byte(_) => value
                .as_u64()
                .and_then(|v| u8::try_from(v).ok())
                .map(|v| scalar(ScalarValue::Byte(v)))
                .ok_or_else(|| err("a u8 integer")),
            // Enum-as-byte stores the label as an FString. A UI that cannot
            // tell the two Byte forms apart sends a number when the input
            // parses as one, so an all-digit enum label (e.g. an unchanged
            // "1") would arrive as a JSON number. Accept that and stringify
            // the integer to the label instead of rejecting the write.
            properties::PropertyValue::Enum(_) => {
                if let Some(s) = value.as_str() {
                    Ok(TypedSetValue::Text(s.to_string()))
                } else if let Some(n) = value.as_i64() {
                    Ok(TypedSetValue::Text(n.to_string()))
                } else {
                    Err(err("a string or integer enum label"))
                }
            }
            _ => Err(CoreError::UnsupportedEdit(
                "private.typed.setValue does not support this ByteProperty form".to_string(),
            )),
        },
        "StrProperty" | "NameProperty" | "ObjectProperty" | "ClassProperty" | "EnumProperty" => {
            value
                .as_str()
                .map(|v| TypedSetValue::Text(v.to_string()))
                .ok_or_else(|| err("a string"))
        }
        other => Err(CoreError::UnsupportedEdit(format!(
            "private.typed.setValue does not support {other} targets \
             (fixed-size scalars and string-valued properties only)"
        ))),
    }
}

fn apply_private_typed_set_value_edit_to_payload(
    payload: &mut Vec<u8>,
    edit: &PrivateTypedSetValueEdit,
) -> Result<(), CoreError> {
    let root = properties::parse_private_root(payload)?;
    let resolved = properties::resolve_chain(&root.properties, &edit.path)?;
    let value = coerce_typed_value(resolved.target, &edit.value)?;
    let target = resolved.target.clone();
    match value {
        TypedSetValue::Scalar(scalar) => properties::patch_scalar(payload, &target, scalar),
        TypedSetValue::Text(text) => {
            // Length-changing patch: work on a scratch copy and prove with a
            // strict re-parse that every enclosing size field was fixed up, so
            // a bug cannot corrupt the caller's payload (or the save).
            let mut patched = payload.clone();
            properties::patch_string(
                &mut patched,
                &target,
                &resolved.enclosing_size_fields,
                &text,
            )?;
            properties::parse_private_root(&patched).map_err(|err| {
                CoreError::Parse(format!(
                    "string patch produced an inconsistent payload: {err}"
                ))
            })?;
            *payload = patched;
            Ok(())
        }
    }
}

fn parse_private_fstring_edit(edit: &Edit) -> Result<PrivateFStringEdit, CoreError> {
    let value = edit.value.as_object().ok_or_else(|| {
        CoreError::InvalidRequest("private.replaceFString value must be an object".to_string())
    })?;
    let old_value = value
        .get("oldValue")
        .or_else(|| value.get("old"))
        .and_then(Value::as_str)
        .ok_or_else(|| {
            CoreError::InvalidRequest("private.replaceFString requires value.oldValue".to_string())
        })?;
    let new_value = value
        .get("newValue")
        .or_else(|| value.get("new"))
        .and_then(Value::as_str)
        .ok_or_else(|| {
            CoreError::InvalidRequest("private.replaceFString requires value.newValue".to_string())
        })?;
    Ok(PrivateFStringEdit {
        old_value: old_value.to_string(),
        new_value: new_value.to_string(),
    })
}

fn parse_private_player_name_edit(edit: &Edit) -> Result<PrivatePlayerNameEdit, CoreError> {
    Ok(PrivatePlayerNameEdit {
        name: parse_private_name_value(edit, "private.player.setPlayerName")?,
    })
}

fn parse_private_profile_name_edit(edit: &Edit) -> Result<PrivateProfileNameEdit, CoreError> {
    Ok(PrivateProfileNameEdit {
        name: parse_private_name_value(edit, "private.profile.setProfileName")?,
    })
}

fn parse_private_player_attribute_edit(
    edit: &Edit,
) -> Result<PrivatePlayerAttributeEdit, CoreError> {
    let value = edit.value.as_object().ok_or_else(|| {
        CoreError::InvalidRequest("private.player.setAttribute value must be an object".to_string())
    })?;
    let id = value
        .get("id")
        .or_else(|| value.get("attribute"))
        .and_then(Value::as_str)
        .filter(|id| !id.trim().is_empty())
        .ok_or_else(|| {
            CoreError::InvalidRequest("private.player.setAttribute requires value.id".to_string())
        })?;
    if private_player_attribute_set_for_id(id).is_none() {
        return Err(CoreError::InvalidRequest(format!(
            "private.player.setAttribute does not support attribute {id:?}"
        )));
    }
    let shared_value = value
        .get("value")
        .map(|raw| parse_private_f32_edit_value(raw, "value"))
        .transpose()?;
    let base_value = value
        .get("baseValue")
        .map(|raw| parse_private_f32_edit_value(raw, "baseValue"))
        .transpose()?
        .or(shared_value);
    let current_value = value
        .get("currentValue")
        .map(|raw| parse_private_f32_edit_value(raw, "currentValue"))
        .transpose()?
        .or(shared_value);
    if base_value.is_none() && current_value.is_none() {
        return Err(CoreError::InvalidRequest(
            "private.player.setAttribute requires value.value, value.baseValue, or value.currentValue"
                .to_string(),
        ));
    }
    Ok(PrivatePlayerAttributeEdit {
        id: id.to_string(),
        base_value,
        current_value,
    })
}

fn parse_private_f32_edit_value(value: &Value, field: &str) -> Result<f32, CoreError> {
    let number = value.as_f64().ok_or_else(|| {
        CoreError::InvalidRequest(format!(
            "private.player.setAttribute value.{field} must be a number"
        ))
    })?;
    if !number.is_finite() || !(-100_000.0..=100_000_000.0).contains(&number) {
        return Err(CoreError::InvalidRequest(format!(
            "private.player.setAttribute value.{field} is outside the supported range"
        )));
    }
    Ok(number as f32)
}

fn parse_private_player_transform_edit(
    edit: &Edit,
) -> Result<PrivatePlayerTransformEdit, CoreError> {
    let value = edit.value.as_object().ok_or_else(|| {
        CoreError::InvalidRequest("private.player.setTransform value must be an object".to_string())
    })?;
    let location = value
        .get("location")
        .map(parse_private_vector3_edit)
        .transpose()?;
    let rotation = value
        .get("rotation")
        .map(parse_private_rotator_edit)
        .transpose()?;
    if location.is_none() && rotation.is_none() {
        return Err(CoreError::InvalidRequest(
            "private.player.setTransform requires value.location or value.rotation".to_string(),
        ));
    }
    Ok(PrivatePlayerTransformEdit { location, rotation })
}

fn parse_private_vector3_edit(value: &Value) -> Result<PrivateVector3Edit, CoreError> {
    let object = value.as_object().ok_or_else(|| {
        CoreError::InvalidRequest(
            "private.player.setTransform value.location must be an object".to_string(),
        )
    })?;
    Ok(PrivateVector3Edit {
        x: parse_private_f64_member(object, "x")?,
        y: parse_private_f64_member(object, "y")?,
        z: parse_private_f64_member(object, "z")?,
    })
}

fn parse_private_rotator_edit(value: &Value) -> Result<PrivateRotatorEdit, CoreError> {
    let object = value.as_object().ok_or_else(|| {
        CoreError::InvalidRequest(
            "private.player.setTransform value.rotation must be an object".to_string(),
        )
    })?;
    Ok(PrivateRotatorEdit {
        pitch: parse_private_f64_member_alias(object, "pitch", "x")?,
        yaw: parse_private_f64_member_alias(object, "yaw", "y")?,
        roll: parse_private_f64_member_alias(object, "roll", "z")?,
    })
}

fn parse_private_f64_member(
    object: &serde_json::Map<String, Value>,
    key: &str,
) -> Result<f64, CoreError> {
    let number = object.get(key).and_then(Value::as_f64).ok_or_else(|| {
        CoreError::InvalidRequest(format!(
            "private.player.setTransform requires numeric value.{key}"
        ))
    })?;
    validate_private_f64(number, key)
}

fn parse_private_f64_member_alias(
    object: &serde_json::Map<String, Value>,
    key: &str,
    alias: &str,
) -> Result<f64, CoreError> {
    let number = object
        .get(key)
        .or_else(|| object.get(alias))
        .and_then(Value::as_f64)
        .ok_or_else(|| {
            CoreError::InvalidRequest(format!(
                "private.player.setTransform requires numeric value.{key}"
            ))
        })?;
    validate_private_f64(number, key)
}

fn validate_private_f64(number: f64, field: &str) -> Result<f64, CoreError> {
    if !number.is_finite() || !(-10_000_000.0..=10_000_000.0).contains(&number) {
        return Err(CoreError::InvalidRequest(format!(
            "private.player.setTransform value.{field} is outside the supported range"
        )));
    }
    Ok(number)
}

fn parse_private_name_value(edit: &Edit, path: &str) -> Result<String, CoreError> {
    let name = if let Some(name) = edit.value.as_str() {
        name
    } else {
        edit.value
            .as_object()
            .and_then(|value| value.get("name"))
            .and_then(Value::as_str)
            .ok_or_else(|| {
                CoreError::InvalidRequest(format!("{path} requires a string value or value.name"))
            })?
    };
    if name.trim().is_empty() {
        return Err(CoreError::InvalidRequest(format!(
            "{path} value must not be empty"
        )));
    }
    Ok(name.to_string())
}

fn parse_private_inventory_item_count_edit(
    edit: &Edit,
) -> Result<PrivateInventoryItemCountEdit, CoreError> {
    let value = edit.value.as_object().ok_or_else(|| {
        CoreError::InvalidRequest(
            "private.inventory.setItemCount value must be an object".to_string(),
        )
    })?;
    let id = value
        .get("id")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(ToOwned::to_owned);
    let path = value
        .get("path")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(ToOwned::to_owned);
    if id.is_none() && path.is_none() {
        return Err(CoreError::InvalidRequest(
            "private.inventory.setItemCount requires value.id or value.path".to_string(),
        ));
    }
    let count = value.get("count").and_then(Value::as_i64).ok_or_else(|| {
        CoreError::InvalidRequest(
            "private.inventory.setItemCount requires integer value.count".to_string(),
        )
    })?;
    if !(0..=i32::MAX as i64).contains(&count) {
        return Err(CoreError::InvalidRequest(
            "private.inventory.setItemCount value.count must fit a non-negative i32".to_string(),
        ));
    }
    Ok(PrivateInventoryItemCountEdit {
        id,
        path,
        count: count as i32,
    })
}

fn parse_private_inventory_add_item_edit(
    edit: &Edit,
) -> Result<PrivateInventoryAddItemEdit, CoreError> {
    let value = edit.value.as_object().ok_or_else(|| {
        CoreError::InvalidRequest("private.inventory.addItem value must be an object".to_string())
    })?;
    let path = value
        .get("path")
        .and_then(Value::as_str)
        .filter(|v| !v.trim().is_empty())
        .ok_or_else(|| {
            CoreError::InvalidRequest(
                "private.inventory.addItem requires a non-empty string value.path".to_string(),
            )
        })?;
    // addItem WRITES this string into m_ItemDefinition (an ObjectProperty), so
    // it must be a real item-definition class — the exact set the bundled
    // catalog includes. A bare id (unresolvable ref), an arbitrary /Script
    // object (e.g. /Script/Engine.Foo), a non-item Angelscript class (e.g.
    // /Script/Angelscript.GothicFinalDataGame), or a non-inventory It* class
    // (e.g. ItemAnimConfig) would all persist an invalid inventory entry.
    if !is_item_definition_class(path) {
        return Err(CoreError::InvalidRequest(format!(
            "private.inventory.addItem value.path must be an item-definition \
             class (e.g. /Script/Angelscript.ItMi_Orenugget), got {path:?}"
        )));
    }
    let count = value.get("count").and_then(Value::as_i64).ok_or_else(|| {
        CoreError::InvalidRequest(
            "private.inventory.addItem requires integer value.count".to_string(),
        )
    })?;
    if count < 1 || count > i32::MAX as i64 {
        return Err(CoreError::InvalidRequest(
            "private.inventory.addItem value.count must be a positive i32 (>= 1)".to_string(),
        ));
    }
    Ok(PrivateInventoryAddItemEdit {
        path: path.to_owned(),
        count: count as i32,
    })
}

fn parse_private_inventory_remove_item_edit(
    edit: &Edit,
) -> Result<PrivateInventoryRemoveItemEdit, CoreError> {
    let value = edit.value.as_object().ok_or_else(|| {
        CoreError::InvalidRequest(
            "private.inventory.removeItem value must be an object".to_string(),
        )
    })?;
    let path = value
        .get("path")
        .and_then(Value::as_str)
        .filter(|v| !v.trim().is_empty())
        .ok_or_else(|| {
            CoreError::InvalidRequest(
                "private.inventory.removeItem requires a non-empty string value.path".to_string(),
            )
        })?;
    if !looks_item_definition_path(path) {
        return Err(CoreError::InvalidRequest(format!(
            "private.inventory.removeItem value.path does not look like an item definition path: {path:?}"
        )));
    }
    Ok(PrivateInventoryRemoveItemEdit {
        path: path.to_owned(),
    })
}

fn decompress_private_payload(
    data: &[u8],
    stream: &CompressedStream,
    backend: &dyn codec_backend::CodecBackend,
) -> Result<Vec<u8>, CoreError> {
    let (payload, _) = decompress_private_payload_with_limit(data, stream, backend, None)?;
    Ok(payload)
}

fn decompress_private_payload_with_limit(
    data: &[u8],
    stream: &CompressedStream,
    backend: &dyn codec_backend::CodecBackend,
    chunk_limit: Option<usize>,
) -> Result<(Vec<u8>, usize), CoreError> {
    let chunks_to_decode = chunk_limit
        .unwrap_or(stream.chunks.len())
        .min(stream.chunks.len());
    let expected_size = if chunks_to_decode == stream.chunks.len() {
        stream.summary_uncompressed_size as usize
    } else {
        stream
            .chunks
            .iter()
            .take(chunks_to_decode)
            .map(|chunk| chunk.uncompressed_size as usize)
            .sum::<usize>()
    };
    // expected_size derives from the untrusted summary/chunk table; reserve
    // fallibly so a crafted huge size returns a codec error instead of aborting
    // the process on OOM through the FFI boundary.
    let mut out: Vec<u8> = Vec::new();
    out.try_reserve(expected_size).map_err(|_| {
        CoreError::Codec(format!(
            "cannot reserve {expected_size} bytes for decoded private payload"
        ))
    })?;
    let chunks = stream
        .chunks
        .iter()
        .take(chunks_to_decode)
        .collect::<Vec<_>>();
    let mut decode_chunks = Vec::with_capacity(chunks.len());
    for chunk in &chunks {
        let compressed_start = chunk.compressed_offset;
        let compressed_end = compressed_start
            .checked_add(chunk.compressed_size as usize)
            .ok_or_else(|| CoreError::Parse("compressed chunk range overflow".to_string()))?;
        let compressed = data.get(compressed_start..compressed_end).ok_or_else(|| {
            CoreError::Parse(format!(
                "compressed chunk {} points outside the save",
                chunk.index
            ))
        })?;
        decode_chunks.push(codec_backend::CodecDecodeChunk {
            input: compressed,
            expected_size: chunk.uncompressed_size as usize,
        });
    }
    let decoded_chunks = backend.decompress_many(&decode_chunks)?;
    if decoded_chunks.len() != chunks.len() {
        return Err(CoreError::Codec(format!(
            "codec backend decompressed {} chunks, expected {}",
            decoded_chunks.len(),
            chunks.len()
        )));
    }
    for (chunk, decompressed) in chunks.iter().zip(decoded_chunks.iter()) {
        if decompressed.len() != chunk.uncompressed_size as usize {
            return Err(CoreError::Codec(format!(
                "codec backend decompressed chunk {} to {} bytes, expected {}",
                chunk.index,
                decompressed.len(),
                chunk.uncompressed_size
            )));
        }
        out.extend_from_slice(decompressed);
    }
    if out.len() != expected_size {
        return Err(CoreError::Codec(format!(
            "decoded private payload has {} bytes, expected {}",
            out.len(),
            expected_size
        )));
    }
    Ok((out, chunks_to_decode))
}

fn apply_private_edit_to_payload(
    payload: &mut Vec<u8>,
    edit: &PrivateEdit,
) -> Result<(), CoreError> {
    match edit {
        PrivateEdit::FString(edit) => apply_private_fstring_edit_to_payload(payload, edit),
        PrivateEdit::PlayerName(edit) => apply_private_player_name_edit_to_payload(payload, edit),
        PrivateEdit::ProfileName(edit) => apply_private_profile_name_edit_to_payload(payload, edit),
        PrivateEdit::PlayerAttribute(edit) => {
            apply_private_player_attribute_edit_to_payload(payload, edit)
        }
        PrivateEdit::PlayerTransform(edit) => {
            apply_private_player_transform_edit_to_payload(payload, edit)
        }
        PrivateEdit::InventoryItemCount(edit) => {
            apply_private_inventory_item_count_edit_to_payload(payload, edit)
        }
        PrivateEdit::InventoryAddItem(edit) => {
            apply_private_inventory_add_item_to_payload(payload, edit)
        }
        PrivateEdit::InventoryRemoveItem(edit) => {
            apply_private_inventory_remove_item_to_payload(payload, edit)
        }
        PrivateEdit::TypedSetValue(edit) => {
            apply_private_typed_set_value_edit_to_payload(payload, edit)
        }
        PrivateEdit::TypedContainer(edit) => {
            apply_private_typed_container_edit_to_payload(payload, edit)
        }
        PrivateEdit::KnowledgeAddCharacter(name) => {
            apply_private_knowledge_add_character_to_payload(payload, name)
        }
    }
}

fn apply_private_typed_container_edit_to_payload(
    payload: &mut Vec<u8>,
    edit: &PrivateTypedContainerEdit,
) -> Result<(), CoreError> {
    let root = properties::parse_private_root(payload)?;
    let resolved = properties::resolve_chain(&root.properties, &edit.path)?;
    let target = resolved.target.clone();
    // Length-changing patch: work on a scratch copy and prove with a strict
    // re-parse that every size and count field was fixed up, so a bug cannot
    // corrupt the caller's payload (or the save).
    let mut patched = payload.clone();
    properties::patch_container(
        &mut patched,
        &target,
        &resolved.enclosing_size_fields,
        &edit.edit,
    )?;
    properties::parse_private_root(&patched).map_err(|err| {
        CoreError::Parse(format!(
            "container patch produced an inconsistent payload: {err}"
        ))
    })?;
    *payload = patched;
    Ok(())
}

/// Insert a brand-new NPC (empty `Knowledge` set) into the savegame's
/// `CharacterKnowledgeByUniqueName` map. Resolves the nested map plus its
/// enclosing size fields, rejects a duplicate name (case-insensitive Name
/// semantics), then splices in a schema-valid empty-knowledge entry. All
/// resolution and validation happen before `patch_container`, and the patch is
/// applied on a scratch copy proven consistent by a strict re-parse, so a
/// failed edit leaves the caller's payload untouched.
fn apply_private_knowledge_add_character_to_payload(
    payload: &mut Vec<u8>,
    unique_name: &str,
) -> Result<(), CoreError> {
    let name = unique_name.trim();
    if name.is_empty() {
        return Err(CoreError::InvalidRequest(
            "character name is empty".to_string(),
        ));
    }
    // Resolve the map + enclosing size fields, and reject duplicates, in a scope
    // that drops the borrow before the &mut payload patch.
    let (target, enclosing) = {
        let root = properties::parse_private_root(payload)?;
        let (path, map_prop) =
            properties::find_property_by_name(&root, "CharacterKnowledgeByUniqueName")
                .ok_or_else(|| {
                    CoreError::Parse("CharacterKnowledgeByUniqueName not found".to_string())
                })?;
        if let properties::PropertyValue::Map { entries, .. } = &map_prop.value {
            if entries.iter().any(|(k, _)| {
                map_key_string(k)
                    .map(|s| s.eq_ignore_ascii_case(name))
                    .unwrap_or(false)
            }) {
                return Err(CoreError::InvalidRequest(format!(
                    "character {name:?} already has a knowledge entry"
                )));
            }
        }
        let segs = properties::parse_path(&path)?;
        let chain = properties::resolve_chain(&root.properties, &segs)?;
        (chain.target.clone(), chain.enclosing_size_fields.clone())
    };
    let entry = encode_knowledge_map_entry(name);
    // Length-changing patch on a scratch copy, proven consistent by a strict
    // re-parse before it touches the caller's payload.
    let mut patched = payload.clone();
    properties::patch_container(
        &mut patched,
        &target,
        &enclosing,
        &properties::ContainerEdit::MapInsert { entry_bytes: entry },
    )?;
    let root2 = properties::parse_private_root(&patched).map_err(|err| {
        CoreError::Parse(format!(
            "knowledge add-character produced an inconsistent payload: {err}"
        ))
    })?;
    // Strict re-parse validation: the key must now resolve.
    properties::find_property_by_name(&root2, "CharacterKnowledgeByUniqueName")
        .and_then(|(_, p)| match &p.value {
            properties::PropertyValue::Map { entries, .. } => entries
                .iter()
                .find(|(k, _)| map_key_string(k) == Some(name)),
            _ => None,
        })
        .ok_or_else(|| CoreError::Parse("post-insert validation failed".to_string()))?;
    *payload = patched;
    Ok(())
}

/// Enum label identifying the player's main item container inside
/// `m_Inventory.m_Keys`. The container index varies between saves, so it must
/// always be looked up by value, never hardcoded.
const MAIN_CONTAINER_ENUM_LABEL: &str = "EInventoryTypes::MainContainer";

/// `m_PlayerID` of the controlled player among `m_SavedPlayers`. A save may hold
/// several saved players (party members); inventory edits target this one — the
/// same anchor the transform/attribute editors use.
const PLAYER_PARTY_ID: &str = "Party ID 0";

/// Addressable path to the controlled player's `m_Inventory`: the `m_Inventory`
/// of the `m_SavedPlayers` entry whose `m_PlayerID` is [`PLAYER_PARTY_ID`].
///
/// Falls back to `None` when no such entry is found (e.g. minimal test
/// fixtures); callers then use the first `m_Inventory` in the tree. This avoids
/// editing another saved player's inventory when the controlled player is not
/// the first `m_SavedPlayers` element.
fn player_inventory_path(root: &properties::RootObject) -> Option<Vec<String>> {
    let (saved_players_path, saved_players) =
        properties::find_property_by_name(root, "m_SavedPlayers")?;
    let properties::PropertyValue::Array { elements } = &saved_players.value else {
        return None;
    };
    for (index, element) in elements.iter().enumerate() {
        let Some(props) = struct_element_properties(element) else {
            continue;
        };
        let is_player = props.iter().any(|p| {
            p.name == "m_PlayerID"
                && matches!(&p.value, properties::PropertyValue::Str(s) if s == PLAYER_PARTY_ID)
        });
        if !is_player || !props.iter().any(|p| p.name == "m_Inventory") {
            continue;
        }
        let mut path = saved_players_path.clone();
        path.push(format!("[{index}]"));
        path.push("m_Inventory".to_string());
        return Some(path);
    }
    None
}

/// The controlled player's `m_Inventory` path, falling back to the first
/// `m_Inventory` anywhere in the tree (synthetic fixtures without a
/// `m_SavedPlayers`/`Party ID 0` structure).
fn resolve_inventory_path(root: &properties::RootObject) -> Option<Vec<String>> {
    player_inventory_path(root)
        .or_else(|| properties::find_property_by_name(root, "m_Inventory").map(|(path, _)| path))
}

/// Inner property list of a slot/container element parsed from a plain
/// Array<StructProperty> (each element is a tagged property list).
fn struct_element_properties(
    element: &properties::PropertyValue,
) -> Option<&[properties::Property]> {
    match element {
        properties::PropertyValue::Struct(properties::StructValue::Properties(props)) => {
            Some(props)
        }
        _ => None,
    }
}

fn struct_element_property<'a>(
    element: &'a properties::PropertyValue,
    name: &str,
) -> Option<&'a properties::Property> {
    struct_element_properties(element)?
        .iter()
        .find(|p| p.name == name)
}

/// The `m_SlotData.m_ItemDefinition` asset path of an ItemVirtualData slot.
fn slot_item_definition(slot: &properties::PropertyValue) -> Option<&str> {
    let slot_data = struct_element_property(slot, "m_SlotData")?;
    let props = match &slot_data.value {
        properties::PropertyValue::Struct(properties::StructValue::Properties(props)) => props,
        _ => return None,
    };
    props
        .iter()
        .find(|p| p.name == "m_ItemDefinition")
        .and_then(|p| match &p.value {
            properties::PropertyValue::Object(path) => Some(path.as_str()),
            _ => None,
        })
}

fn slot_id(slot: &properties::PropertyValue) -> Option<i32> {
    match struct_element_property(slot, "m_Id")?.value {
        properties::PropertyValue::Int(id) => Some(id),
        _ => None,
    }
}

/// The `m_SlotData.m_ItemCount` of an ItemVirtualData slot.
fn slot_item_count(slot: &properties::PropertyValue) -> Option<i32> {
    let slot_data = struct_element_property(slot, "m_SlotData")?;
    let props = match &slot_data.value {
        properties::PropertyValue::Struct(properties::StructValue::Properties(props)) => props,
        _ => return None,
    };
    props
        .iter()
        .find(|p| p.name == "m_ItemCount")
        .and_then(|p| match &p.value {
            properties::PropertyValue::Int(v) => Some(*v),
            _ => None,
        })
}

/// MainContainer membership derived from the typed inventory tree.
struct MainContainerSummary {
    /// Every item path held in the MainContainer (used to exclude already-owned
    /// items from the add picker — addItem rejects any MainContainer duplicate).
    all_paths: std::collections::HashSet<String>,
    /// MainContainer paths safe to remove by path: those that occur exactly
    /// once across the WHOLE inventory. removeItem is path-addressed and the
    /// summary rows carry no stable per-slot id, so only a globally-unique path
    /// maps unambiguously to the row the user clicked. A path duplicated within
    /// the MainContainer or shared with another container is not removable.
    removable_paths: std::collections::HashSet<String>,
    /// Whether any container holds a clean (state-free m_Payload) slot that
    /// addItem can use as a template. Without one, addItem cannot succeed, so it
    /// must not be advertised.
    has_clean_template: bool,
}

/// Summarize the player's MainContainer. addItem and removeItem only operate on
/// this container. Returns `None` when the typed tree has no resolvable
/// MainContainer (structural ops are then not offered); `Some` when it resolves
/// — `all_paths` is empty for an empty MainContainer, which addItem can still
/// seed from another container.
fn main_container_summary(root: &properties::RootObject) -> Option<MainContainerSummary> {
    let inventory_path = resolve_inventory_path(root)?;
    let resolve_child = |suffix: &[&str]| -> Option<properties::PropertyValue> {
        let mut segs = inventory_path.clone();
        segs.extend(suffix.iter().map(|s| s.to_string()));
        let parsed = properties::parse_path(&segs).ok()?;
        properties::resolve(&root.properties, &parsed)
            .ok()
            .map(|prop| prop.value.clone())
    };
    let properties::PropertyValue::Array { elements: keys } = resolve_child(&["m_Keys"])? else {
        return None;
    };
    let main_index = keys.iter().position(|element| {
        matches!(element, properties::PropertyValue::Enum(label)
            if label == MAIN_CONTAINER_ENUM_LABEL)
    })?;
    // Resolve the MainContainer's m_Slots (must exist for a valid result).
    let main_segment = format!("[{main_index}]");
    let properties::PropertyValue::Array {
        elements: main_slots,
    } = resolve_child(&["m_Values", "Items", &main_segment, "m_Slots"])?
    else {
        return None;
    };
    // Scan every container for a clean template slot (addItem clones it), and
    // count how many times each item path occurs across the WHOLE inventory.
    let mut has_clean_template = false;
    let mut global_counts: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    if let Some(properties::PropertyValue::Array {
        elements: containers,
    }) = resolve_child(&["m_Values", "Items"])
    {
        for index in 0..containers.len() {
            let segment = format!("[{index}]");
            if let Some(properties::PropertyValue::Array { elements: slots }) =
                resolve_child(&["m_Values", "Items", &segment, "m_Slots"])
            {
                for slot in &slots {
                    if !struct_element_property(slot, "m_Payload")
                        .is_some_and(property_carries_state)
                    {
                        has_clean_template = true;
                    }
                    if let Some(path) = slot_item_definition(slot) {
                        if !path.is_empty() {
                            *global_counts.entry(path.to_string()).or_default() += 1;
                        }
                    }
                }
            }
        }
    }
    // A path is removable only when it occurs exactly once across the entire
    // inventory. removeItem is addressed by path (the FString summary rows carry
    // no stable per-slot id), so a path shared by another container OR repeated
    // within the MainContainer is ambiguous — deleting one row could drop a
    // different stack than the one shown. Unique paths map 1:1 to their row.
    let mut all_paths = std::collections::HashSet::new();
    let mut removable_paths = std::collections::HashSet::new();
    for slot in &main_slots {
        if let Some(path) = slot_item_definition(slot) {
            if path.is_empty() {
                continue;
            }
            all_paths.insert(path.to_string());
            if global_counts.get(path) == Some(&1) {
                removable_paths.insert(path.to_string());
            }
        }
    }
    Some(MainContainerSummary {
        all_paths,
        removable_paths,
        has_clean_template,
    })
}

/// Scalar `m_Payload` fields that are part of the known default ItemPayload
/// shape and may legitimately appear at their default value in a clean template.
/// A direct scalar leaf NOT in this set is treated as item-specific state even
/// when it currently holds the type default (0/""/false/"None"), because its
/// real default is unknown and cloning it could seed an unrelated new item with
/// stale state (e.g. a directly-stored durability that happens to be 0).
const CLEAN_PAYLOAD_SCALAR_FIELDS: &[&str] = &["m_StageLevel", "m_OptionalObject", "TagName"];

/// True when a scalar payload leaf holds a non-default value. Default is the
/// type's zero value: 0 / 0.0 / false, and "" or the FName null "None" for
/// string/name/object/class/enum leaves.
fn scalar_leaf_is_nondefault(value: &properties::PropertyValue) -> bool {
    use properties::PropertyValue as PV;
    match value {
        PV::Int(0) | PV::UInt32(0) | PV::Int64(0) | PV::Byte(0) => false,
        PV::Float(f) => *f != 0.0,
        PV::Double(f) => *f != 0.0,
        PV::Bool(b) => *b,
        PV::Str(s) | PV::Name(s) | PV::Object(s) | PV::Enum(s) => !(s.is_empty() || s == "None"),
        PV::Opaque(bytes) => !bytes.is_empty(),
        // Any remaining leaf (non-zero number, SoftObject) is item state.
        _ => true,
    }
}

/// True when a property within a template's `m_Payload` carries item-specific
/// state that must not be cloned onto a new item. A "clean" payload has every
/// container empty and every scalar leaf either an empty/default container field
/// or a known default ItemPayload scalar at its default value.
///
/// A real default-initialised ItemPayload is NOT a bare empty map: it carries
/// `m_StageLevel=0`, `m_OptionalObject=""`, empty `m_GenericData`/`m_InnerItems`
/// maps, `m_Ownership.UseOwnershipOfArea.TagName="None"`, and an EMPTY native
/// GameplayTagContainer (`m_Ownership.OwnedByGuild`). The old rule flagged any
/// scalar leaf as state, so every real slot looked dirty, `has_clean_template`
/// was always false, and the inventory Add button vanished.
///
/// Containers are state iff non-empty (name-independent). Scalars are gated by
/// name: only the known default ItemPayload fields may be default-clean; an
/// unrecognised direct scalar is always state, so a directly-stored value such
/// as a durability is never cloned even when it currently equals the default.
fn property_carries_state(prop: &properties::Property) -> bool {
    use properties::{PropertyValue as PV, StructValue as SV};
    match &prop.value {
        // Containers carry state only when non-empty.
        PV::Array { elements } | PV::Set { elements, .. } => !elements.is_empty(),
        PV::Map { entries, .. } => !entries.is_empty(),
        PV::ObjectInstances(instances) => !instances.is_empty(),
        PV::Struct(s) => match s {
            SV::Properties(props) => props.iter().any(property_carries_state),
            SV::Instanced(Some(instanced)) => {
                instanced.properties.iter().any(property_carries_state)
            }
            SV::Instanced(None) => false,
            // Native-serialized struct variants carry state only when non-default.
            SV::GameplayTagContainer(tags) => !tags.is_empty(),
            SV::Vector3 { x, y, z } => *x != 0.0 || *y != 0.0 || *z != 0.0,
            SV::Vector3f { x, y, z } => *x != 0.0 || *y != 0.0 || *z != 0.0,
            SV::Vector4 { x, y, z, w } => *x != 0.0 || *y != 0.0 || *z != 0.0 || *w != 0.0,
            SV::Vector2 { x, y } => *x != 0.0 || *y != 0.0,
            SV::Guid(bytes) => bytes.iter().any(|&b| b != 0),
            SV::DateTime(v) => *v != 0,
        },
        // Scalar leaf: state unless it is a known default ItemPayload field
        // currently at its default value.
        _ => {
            !CLEAN_PAYLOAD_SCALAR_FIELDS.contains(&prop.name.as_str())
                || scalar_leaf_is_nondefault(&prop.value)
        }
    }
}

/// Append a new item slot to the player's MainContainer inventory by cloning
/// the last existing slot (template) and retargeting its definition path,
/// count, and id. Every length-changing step works through the existing
/// size-chain-aware patch helpers and is proven by a strict re-parse; the
/// caller's payload is only replaced once the final payload re-parses AND the
/// new item surfaces in the inventory summary scan with the requested count.
fn apply_private_inventory_add_item_to_payload(
    payload: &mut Vec<u8>,
    edit: &PrivateInventoryAddItemEdit,
) -> Result<(), CoreError> {
    // 1. Typed parse + locate the MainContainer by enum value in m_Keys.
    let root = properties::parse_private_root(payload).map_err(|err| {
        CoreError::Parse(format!(
            "private.inventory.addItem requires a typed-parsable private payload: {err}"
        ))
    })?;
    let inventory_path = resolve_inventory_path(&root).ok_or_else(|| {
        CoreError::Parse(
            "private payload has no m_Inventory property; cannot add an item".to_string(),
        )
    })?;
    let child_segments = |suffix: &[String]| -> Result<Vec<properties::PathSeg>, CoreError> {
        let mut segments = inventory_path.clone();
        segments.extend_from_slice(suffix);
        properties::parse_path(&segments)
    };
    let keys_segs = child_segments(&["m_Keys".to_string()])?;
    let keys = properties::resolve(&root.properties, &keys_segs)?;
    let properties::PropertyValue::Array {
        elements: key_elements,
    } = &keys.value
    else {
        return Err(CoreError::Parse(
            "m_Inventory.m_Keys is not a plain enum array".to_string(),
        ));
    };
    let main_index = key_elements
        .iter()
        .position(|element| {
            matches!(element, properties::PropertyValue::Enum(label)
                if label == MAIN_CONTAINER_ENUM_LABEL)
        })
        .ok_or_else(|| {
            CoreError::Parse(format!(
                "m_Inventory.m_Keys has no {MAIN_CONTAINER_ENUM_LABEL} entry"
            ))
        })?;
    let slots_suffix = vec![
        "m_Values".to_string(),
        "Items".to_string(),
        format!("[{main_index}]"),
        "m_Slots".to_string(),
    ];
    let slots_segs = child_segments(&slots_suffix)?;
    let chain = properties::resolve_chain(&root.properties, &slots_segs)?;
    let properties::PropertyValue::Array { elements: slots } = &chain.target.value else {
        return Err(CoreError::Parse(
            "MainContainer m_Slots is not a plain slot array".to_string(),
        ));
    };

    // 2. Reject duplicates within the MainContainer and pick the template.
    if slots
        .iter()
        .any(|slot| slot_item_definition(slot) == Some(edit.path.as_str()))
    {
        return Err(CoreError::InvalidRequest(format!(
            "the player inventory already contains {}; \
             use private.inventory.setItemCount to change its count",
            edit.path
        )));
    }
    // Choose the template. Prefer a clean (state-free m_Payload) MainContainer
    // slot and duplicate it in place — its m_InventoryType is already
    // MainContainer. If no MainContainer slot is clean (including an empty
    // MainContainer), borrow a clean slot's bytes from another container and fix
    // m_InventoryType to MainContainer below.
    let is_clean = |slot: &properties::PropertyValue| {
        !struct_element_property(slot, "m_Payload").is_some_and(property_carries_state)
    };
    let max_id = slots.iter().filter_map(slot_id).max().unwrap_or(-1);
    let new_id = max_id.checked_add(1).ok_or_else(|| {
        CoreError::Parse("inventory slot ids exhausted (m_Id overflow)".to_string())
    })?;
    let (container_edit, new_index, needs_type_patch) =
        if let Some(source) = slots.iter().rposition(|slot| is_clean(slot)) {
            // ArrayDuplicate inserts the copy right after the source slot.
            (
                properties::ContainerEdit::ArrayDuplicate(source),
                source + 1,
                false,
            )
        } else {
            let template_bytes =
                donor_slot_template_bytes(payload, &root, &inventory_path, main_index)?
                    .ok_or_else(|| {
                        CoreError::UnsupportedEdit(
                            "no inventory container has a clean (state-free) slot to use as a \
                         template; cannot synthesize a new item slot"
                                .to_string(),
                        )
                    })?;
            // ArrayInsertBytes appends to the end of the MainContainer.
            (
                properties::ContainerEdit::ArrayInsertBytes(template_bytes),
                slots.len(),
                true,
            )
        };

    // 3. Apply the structural edit on a scratch copy (size chains fixed up by
    //    patch_container; failed patches leave the original untouched).
    let mut patched = payload.clone();
    properties::patch_container(
        &mut patched,
        chain.target,
        &chain.enclosing_size_fields,
        &container_edit,
    )?;

    // 4. Retarget the duplicate: definition path first (length-changing, so
    //    re-resolve from a fresh parse), then the fixed-size count and id.
    let slot_segment = format!("[{new_index}]");
    let definition_segs = child_segments(&{
        let mut suffix = slots_suffix.clone();
        suffix.extend([
            slot_segment.clone(),
            "m_SlotData".to_string(),
            "m_ItemDefinition".to_string(),
        ]);
        suffix
    })?;
    {
        let duplicated = properties::parse_private_root(&patched).map_err(|err| {
            CoreError::Parse(format!(
                "inventory slot duplication produced an inconsistent payload: {err}"
            ))
        })?;
        let definition_chain = properties::resolve_chain(&duplicated.properties, &definition_segs)
            .map_err(|err| {
                CoreError::Parse(format!(
                    "duplicated inventory slot is missing m_SlotData.m_ItemDefinition: {err}"
                ))
            })?;
        properties::patch_string(
            &mut patched,
            definition_chain.target,
            &definition_chain.enclosing_size_fields,
            &edit.path,
        )?;
    }
    if needs_type_patch {
        // A borrowed template carries the donor container's m_InventoryType;
        // fix it to MainContainer (length-changing, so re-resolve and patch on
        // its own before the fixed-size scalar writes below).
        let reparsed = properties::parse_private_root(&patched).map_err(|err| {
            CoreError::Parse(format!(
                "inventory item definition patch produced an inconsistent payload: {err}"
            ))
        })?;
        let type_segs = child_segments(&{
            let mut suffix = slots_suffix.clone();
            suffix.extend([slot_segment.clone(), "m_InventoryType".to_string()]);
            suffix
        })?;
        let type_chain =
            properties::resolve_chain(&reparsed.properties, &type_segs).map_err(|err| {
                CoreError::Parse(format!(
                    "synthesized inventory slot is missing m_InventoryType: {err}"
                ))
            })?;
        properties::patch_string(
            &mut patched,
            type_chain.target,
            &type_chain.enclosing_size_fields,
            MAIN_CONTAINER_ENUM_LABEL,
        )?;
    }
    {
        let retargeted = properties::parse_private_root(&patched).map_err(|err| {
            CoreError::Parse(format!(
                "inventory item definition patch produced an inconsistent payload: {err}"
            ))
        })?;
        let count_segs = child_segments(&{
            let mut suffix = slots_suffix.clone();
            suffix.extend([
                slot_segment.clone(),
                "m_SlotData".to_string(),
                "m_ItemCount".to_string(),
            ]);
            suffix
        })?;
        let id_segs = child_segments(&{
            let mut suffix = slots_suffix.clone();
            suffix.extend([slot_segment.clone(), "m_Id".to_string()]);
            suffix
        })?;
        let count_target = properties::resolve(&retargeted.properties, &count_segs)?;
        let id_target = properties::resolve(&retargeted.properties, &id_segs)?;
        // Fixed-size scalar patches: no offsets shift between the two writes.
        properties::patch_scalar(
            &mut patched,
            count_target,
            properties::ScalarValue::Int(edit.count),
        )?;
        properties::patch_scalar(
            &mut patched,
            id_target,
            properties::ScalarValue::Int(new_id),
        )?;
    }

    // 5. Final proof: strict re-parse AND the new slot must exist in the
    //    MainContainer itself with the requested path and count. A global
    //    region scan would be satisfied by the same item already living in a
    //    different container (e.g. Quickslots), so a no-op add could be wrongly
    //    committed — verify against the edited MainContainer m_Slots only.
    let reparsed = properties::parse_private_root(&patched).map_err(|err| {
        CoreError::Parse(format!(
            "inventory addItem produced an inconsistent payload: {err}"
        ))
    })?;
    let patched_slots_segs = child_segments(&slots_suffix)?;
    let patched_slots = properties::resolve(&reparsed.properties, &patched_slots_segs)?;
    let properties::PropertyValue::Array {
        elements: patched_slot_elems,
    } = &patched_slots.value
    else {
        return Err(CoreError::Parse(
            "MainContainer m_Slots is not a plain slot array after add".to_string(),
        ));
    };
    let appeared = patched_slot_elems.iter().any(|slot| {
        slot_item_definition(slot) == Some(edit.path.as_str())
            && slot_item_count(slot) == Some(edit.count)
    });
    if !appeared {
        return Err(CoreError::Validation(format!(
            "added item {} (count {}) did not appear in the MainContainer after the edit; \
             aborting the write",
            edit.path, edit.count
        )));
    }
    *payload = patched;
    Ok(())
}

/// Raw bytes of a template item slot borrowed from a non-Main inventory
/// container, used to seed an empty MainContainer. Returns the last slot of the
/// first other container that has one with an empty m_Payload, or None when no
/// container has a usable slot. The caller fixes the borrowed slot's
/// m_InventoryType to MainContainer.
fn donor_slot_template_bytes(
    payload: &[u8],
    root: &properties::RootObject,
    inventory_path: &[String],
    main_index: usize,
) -> Result<Option<Vec<u8>>, CoreError> {
    let child = |suffix: &[String]| -> Result<Vec<properties::PathSeg>, CoreError> {
        let mut segments = inventory_path.to_vec();
        segments.extend_from_slice(suffix);
        properties::parse_path(&segments)
    };
    let items_segs = child(&["m_Values".to_string(), "Items".to_string()])?;
    let items = properties::resolve(&root.properties, &items_segs)?;
    let properties::PropertyValue::Array {
        elements: containers,
    } = &items.value
    else {
        return Ok(None);
    };
    for index in 0..containers.len() {
        if index == main_index {
            continue;
        }
        let slots_segs = child(&[
            "m_Values".to_string(),
            "Items".to_string(),
            format!("[{index}]"),
            "m_Slots".to_string(),
        ])?;
        let Ok(slots_prop) = properties::resolve(&root.properties, &slots_segs) else {
            continue;
        };
        let properties::PropertyValue::Array {
            elements: donor_slots,
        } = &slots_prop.value
        else {
            continue;
        };
        // Find any slot with an empty m_Payload (don't clone item-specific
        // state) — not just the last one, which may be stateful.
        let Some(donor_index) = donor_slots.iter().position(|slot| {
            !struct_element_property(slot, "m_Payload").is_some_and(property_carries_state)
        }) else {
            continue;
        };
        let layout = properties::container_layout(payload, slots_prop)?;
        if let Some(range) = layout.element_ranges.get(donor_index) {
            return Ok(Some(payload[range.clone()].to_vec()));
        }
    }
    Ok(None)
}

fn apply_private_inventory_remove_item_to_payload(
    payload: &mut Vec<u8>,
    edit: &PrivateInventoryRemoveItemEdit,
) -> Result<(), CoreError> {
    // 1. Typed parse + locate the MainContainer m_Slots (mirrors addItem).
    let root = properties::parse_private_root(payload).map_err(|err| {
        CoreError::Parse(format!(
            "private.inventory.removeItem requires a typed-parsable private payload: {err}"
        ))
    })?;
    let inventory_path = resolve_inventory_path(&root).ok_or_else(|| {
        CoreError::Parse(
            "private payload has no m_Inventory property; cannot remove an item".to_string(),
        )
    })?;
    let child_segments = |suffix: &[String]| -> Result<Vec<properties::PathSeg>, CoreError> {
        let mut segments = inventory_path.clone();
        segments.extend_from_slice(suffix);
        properties::parse_path(&segments)
    };
    let keys_segs = child_segments(&["m_Keys".to_string()])?;
    let keys = properties::resolve(&root.properties, &keys_segs)?;
    let properties::PropertyValue::Array {
        elements: key_elements,
    } = &keys.value
    else {
        return Err(CoreError::Parse(
            "m_Inventory.m_Keys is not a plain enum array".to_string(),
        ));
    };
    let main_index = key_elements
        .iter()
        .position(|element| {
            matches!(element, properties::PropertyValue::Enum(label)
                if label == MAIN_CONTAINER_ENUM_LABEL)
        })
        .ok_or_else(|| {
            CoreError::Parse(format!(
                "m_Inventory.m_Keys has no {MAIN_CONTAINER_ENUM_LABEL} entry"
            ))
        })?;
    let slots_suffix = vec![
        "m_Values".to_string(),
        "Items".to_string(),
        format!("[{main_index}]"),
        "m_Slots".to_string(),
    ];
    let slots_segs = child_segments(&slots_suffix)?;
    let chain = properties::resolve_chain(&root.properties, &slots_segs)?;
    let properties::PropertyValue::Array { elements: slots } = &chain.target.value else {
        return Err(CoreError::Parse(
            "MainContainer m_Slots is not a plain slot array".to_string(),
        ));
    };

    // 2. Find the slot whose m_SlotData.m_ItemDefinition matches the path.
    //    Real saves do contain a few same-path slots (e.g. two of a
    //    non-stacking item), which the summary surfaces as indistinguishable
    //    rows (same id/path). We remove the first match: refusing would leave
    //    those items permanently undeletable, and the rows are interchangeable
    //    from the UI's perspective.
    let index = slots
        .iter()
        .position(|slot| slot_item_definition(slot) == Some(edit.path.as_str()))
        .ok_or_else(|| {
            CoreError::InvalidRequest(format!(
                "the player inventory does not contain {}",
                edit.path
            ))
        })?;

    // 3. Remove the slot on a scratch copy (size chains fixed up by
    //    patch_container; a failed patch leaves the original untouched).
    let mut patched = payload.clone();
    properties::patch_container(
        &mut patched,
        chain.target,
        &chain.enclosing_size_fields,
        &properties::ContainerEdit::ArrayRemove(index),
    )?;

    // 4. Final proof: strict re-parse, and the targeted slot must be gone from
    //    the MainContainer specifically. The same item path may legitimately
    //    still exist in another container (e.g. Quickslots), so a global
    //    player-inventory-region scan would wrongly abort the write — verify
    //    against the edited MainContainer m_Slots only.
    let reparsed = properties::parse_private_root(&patched).map_err(|err| {
        CoreError::Parse(format!(
            "inventory removeItem produced an inconsistent payload: {err}"
        ))
    })?;
    let patched_slots_segs = child_segments(&slots_suffix)?;
    let patched_slots = properties::resolve(&reparsed.properties, &patched_slots_segs)?;
    let properties::PropertyValue::Array {
        elements: patched_slot_elems,
    } = &patched_slots.value
    else {
        return Err(CoreError::Parse(
            "MainContainer m_Slots is not a plain slot array after removal".to_string(),
        ));
    };
    // Exactly one matching slot must be gone. (Some saves hold duplicate-path
    // slots, so the path may still be present afterwards — only require the
    // count to drop by one.)
    let match_count = |elems: &[properties::PropertyValue]| {
        elems
            .iter()
            .filter(|slot| slot_item_definition(slot) == Some(edit.path.as_str()))
            .count()
    };
    let before = match_count(slots);
    let after = match_count(patched_slot_elems);
    if after != before - 1 {
        return Err(CoreError::Validation(format!(
            "removeItem for {} changed the MainContainer match count from {before} to {after}; \
             expected exactly one fewer — aborting the write",
            edit.path
        )));
    }
    *payload = patched;
    Ok(())
}

fn apply_private_fstring_edit_to_payload(
    payload: &mut Vec<u8>,
    edit: &PrivateFStringEdit,
) -> Result<(), CoreError> {
    let refs = scan_fstrings(payload, 0);
    let matches: Vec<_> = refs
        .iter()
        .filter(|reference| reference.value == edit.old_value)
        .collect();
    let target = match matches.as_slice() {
        [] => {
            return Err(CoreError::Parse(format!(
                "private FString {:?} was not found",
                edit.old_value
            )));
        }
        [target] => *target,
        _ => {
            return Err(CoreError::UnsupportedEdit(format!(
                "private FString {:?} is ambiguous: {} matches found",
                edit.old_value,
                matches.len()
            )));
        }
    };
    if target.utf16 {
        return Err(CoreError::UnsupportedEdit(
            "UTF-16 private FString replacement is not implemented yet".to_string(),
        ));
    }
    let replacement = encode_fstring(&edit.new_value);
    let start = target.len_offset;
    let end = target.len_offset + target.total_len;
    payload.splice(start..end, replacement);
    Ok(())
}

fn apply_private_player_name_edit_to_payload(
    payload: &mut Vec<u8>,
    edit: &PrivatePlayerNameEdit,
) -> Result<(), CoreError> {
    let refs = scan_fstrings(payload, 0);
    let matches = private_player_name_value_refs(&refs);
    let target = match matches.as_slice() {
        [] => {
            return Err(CoreError::Parse(
                "private player name property was not found".to_string(),
            ));
        }
        [target] => target,
        _ => {
            return Err(CoreError::UnsupportedEdit(
                "private player name edit is ambiguous".to_string(),
            ));
        }
    };
    if target.value.utf16 {
        return Err(CoreError::UnsupportedEdit(
            "UTF-16 private player name replacement is not implemented yet".to_string(),
        ));
    }
    write_str_property_value(payload, target.size_offset, &target.value, &edit.name)
}

fn apply_private_profile_name_edit_to_payload(
    payload: &mut Vec<u8>,
    edit: &PrivateProfileNameEdit,
) -> Result<(), CoreError> {
    let refs = scan_fstrings(payload, 0);
    let matches = private_profile_name_value_refs(&refs);
    let target = match matches.as_slice() {
        [] => {
            return Err(CoreError::Parse(
                "private profile name property was not found".to_string(),
            ));
        }
        [target] => target,
        _ => {
            return Err(CoreError::UnsupportedEdit(
                "private profile name edit is ambiguous".to_string(),
            ));
        }
    };
    if target.value.utf16 {
        return Err(CoreError::UnsupportedEdit(
            "UTF-16 private profile name replacement is not implemented yet".to_string(),
        ));
    }
    write_str_property_value(payload, target.size_offset, &target.value, &edit.name)
}

fn apply_private_player_attribute_edit_to_payload(
    payload: &mut [u8],
    edit: &PrivatePlayerAttributeEdit,
) -> Result<(), CoreError> {
    let refs = scan_fstrings(payload, 0);
    let matches = private_player_attribute_refs(payload, &refs)
        .into_iter()
        .filter(|attribute| attribute.id == edit.id)
        .collect::<Vec<_>>();
    let target = match matches.as_slice() {
        [] => {
            return Err(CoreError::Parse(format!(
                "private player attribute {:?} was not found",
                edit.id
            )));
        }
        [target] => target,
        _ => {
            return Err(CoreError::UnsupportedEdit(format!(
                "private player attribute {:?} edit is ambiguous",
                edit.id
            )));
        }
    };
    if let Some(value) = edit.base_value {
        let offset = target.base_value_offset.ok_or_else(|| {
            CoreError::Parse(format!(
                "private player attribute {:?} has no writable BaseValue",
                edit.id
            ))
        })?;
        write_f32_at(payload, offset, value)?;
    }
    if let Some(value) = edit.current_value {
        let offset = target.current_value_offset.ok_or_else(|| {
            CoreError::Parse(format!(
                "private player attribute {:?} has no writable CurrentValue",
                edit.id
            ))
        })?;
        write_f32_at(payload, offset, value)?;
    }
    Ok(())
}

fn apply_private_player_transform_edit_to_payload(
    payload: &mut [u8],
    edit: &PrivatePlayerTransformEdit,
) -> Result<(), CoreError> {
    let refs = scan_fstrings(payload, 0);
    let matches = private_player_transform_refs(payload, &refs);
    let target = match matches.as_slice() {
        [] => {
            return Err(CoreError::Parse(
                "private player transform was not found".to_string(),
            ));
        }
        [target] => target,
        _ => {
            return Err(CoreError::UnsupportedEdit(
                "private player transform edit is ambiguous".to_string(),
            ));
        }
    };
    if let Some(location) = &edit.location {
        write_f64_at(payload, target.location.offset, location.x)?;
        write_f64_at(payload, target.location.offset + 8, location.y)?;
        write_f64_at(payload, target.location.offset + 16, location.z)?;
    }
    if let Some(rotation) = &edit.rotation {
        write_f64_at(payload, target.rotation.offset, rotation.pitch)?;
        write_f64_at(payload, target.rotation.offset + 8, rotation.yaw)?;
        write_f64_at(payload, target.rotation.offset + 16, rotation.roll)?;
    }
    Ok(())
}

/// A located StrProperty whose value can be edited. Carries both the value
/// FString and the offset of the StrProperty's 4-byte payload-size field, which
/// must be rewritten alongside the value whenever the value length changes.
#[derive(Clone)]
struct StrPropertyValueRef {
    value: FStringRef,
    size_offset: usize,
}

fn private_player_name_value_refs(refs: &[FStringRef]) -> Vec<StrPropertyValueRef> {
    private_str_property_value_refs(refs, &["m_PlayerName", "m_CharacterName", "m_UserName"])
}

fn private_profile_name_value_refs(refs: &[FStringRef]) -> Vec<StrPropertyValueRef> {
    private_str_property_value_refs(refs, &["m_ProfileName"])
}

fn private_str_property_value_refs(
    refs: &[FStringRef],
    property_names: &[&str],
) -> Vec<StrPropertyValueRef> {
    refs.iter()
        .enumerate()
        .filter_map(|(idx, reference)| {
            if !property_names.contains(&reference.value.as_str()) {
                return None;
            }
            let type_ref = refs.get(idx + 1)?;
            if type_ref.value != "StrProperty" {
                return None;
            }
            let value = refs.get(idx + 2)?.clone();
            // Layout: StrProperty name, "StrProperty", u32 (unused), u32 size,
            // u8 guid, value FString. The size word sits 4 bytes after the type
            // FString, matching the public StrProperty editor.
            let size_offset = type_ref.len_offset + type_ref.total_len + 4;
            Some(StrPropertyValueRef { value, size_offset })
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq)]
struct PrivatePlayerAttributeRef {
    id: String,
    base_value: Option<f32>,
    base_value_offset: Option<usize>,
    current_value: Option<f32>,
    current_value_offset: Option<usize>,
}

fn private_player_attribute_refs(
    payload: &[u8],
    refs: &[FStringRef],
) -> Vec<PrivatePlayerAttributeRef> {
    let Some((start_idx, end_idx)) = hero_attribute_region(refs) else {
        return Vec::new();
    };
    let mut current_set: Option<&str> = None;
    let mut attributes = Vec::new();
    for idx in start_idx..end_idx {
        let value = refs[idx].value.as_str();
        if value.starts_with("/Script/G1R.AttributeSet_") {
            current_set = Some(value);
            continue;
        }
        let Some(expected_set) = private_player_attribute_set_for_id(value) else {
            continue;
        };
        if current_set != Some(expected_set) {
            continue;
        }
        let base_value_offset =
            private_attribute_float_field_offset(payload, refs, idx, "BaseValue", end_idx);
        let current_value_offset =
            private_attribute_float_field_offset(payload, refs, idx, "CurrentValue", end_idx);
        if base_value_offset.is_none() && current_value_offset.is_none() {
            continue;
        }
        attributes.push(PrivatePlayerAttributeRef {
            id: value.to_string(),
            base_value: base_value_offset.and_then(|offset| read_f32_at(payload, offset)),
            base_value_offset,
            current_value: current_value_offset.and_then(|offset| read_f32_at(payload, offset)),
            current_value_offset,
        });
    }
    attributes
}

fn hero_attribute_region(refs: &[FStringRef]) -> Option<(usize, usize)> {
    let start_idx = refs.iter().enumerate().find_map(|(idx, reference)| {
        if reference.value != "Hero" {
            return None;
        }
        refs.iter()
            .skip(idx + 1)
            .take(7)
            .any(|candidate| candidate.value == "AttributeSetsByClass")
            .then_some(idx)
    })?;
    let end_idx = refs
        .iter()
        .enumerate()
        .skip(start_idx + 1)
        .find(|(_, reference)| {
            matches!(
                reference.value.as_str(),
                "MemorizedEvents" | "ActiveEffects" | "Knowledge" | "Inventory"
            )
        })
        .map(|(idx, _)| idx)?;
    Some((start_idx, end_idx))
}

fn private_player_attribute_set_for_id(id: &str) -> Option<&'static str> {
    match id {
        "Health" | "MaxHealth" => Some("/Script/G1R.AttributeSet_Health"),
        "Level" | "Experience" => Some("/Script/G1R.AttributeSet_LevelProgression"),
        "Strength" => Some("/Script/G1R.AttributeSet_Strength"),
        "Dexterity" => Some("/Script/G1R.AttributeSet_Dexterity"),
        "Mana" | "MaxMana" | "MagicianLevel" => Some("/Script/G1R.AttributeSet_Mana"),
        _ => None,
    }
}

fn private_attribute_float_field_offset(
    payload: &[u8],
    refs: &[FStringRef],
    attribute_idx: usize,
    field_name: &str,
    end_idx: usize,
) -> Option<usize> {
    refs.iter()
        .enumerate()
        .take(end_idx)
        .skip(attribute_idx + 1)
        .take_while(|(_, reference)| reference.value != "None")
        .find(|(_, reference)| reference.value == field_name)
        .and_then(|(idx, _)| f32_value_offset_at(payload, refs, idx))
}

fn read_f32_at(payload: &[u8], offset: usize) -> Option<f32> {
    Some(f32::from_le_bytes(
        payload.get(offset..offset + 4)?.try_into().ok()?,
    ))
}

fn read_f64_at(payload: &[u8], offset: usize) -> Option<f64> {
    Some(f64::from_le_bytes(
        payload.get(offset..offset + 8)?.try_into().ok()?,
    ))
}

fn write_f32_at(payload: &mut [u8], offset: usize, value: f32) -> Result<(), CoreError> {
    let end = offset + 4;
    if end > payload.len() {
        return Err(CoreError::Parse(
            "private float property offset points outside payload".to_string(),
        ));
    }
    payload[offset..end].copy_from_slice(&value.to_le_bytes());
    Ok(())
}

fn write_f64_at(payload: &mut [u8], offset: usize, value: f64) -> Result<(), CoreError> {
    let end = offset + 8;
    if end > payload.len() {
        return Err(CoreError::Parse(
            "private double property offset points outside payload".to_string(),
        ));
    }
    payload[offset..end].copy_from_slice(&value.to_le_bytes());
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct PrivateVector3Ref {
    x: f64,
    y: f64,
    z: f64,
    offset: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct PrivateRotatorRef {
    pitch: f64,
    yaw: f64,
    roll: f64,
    offset: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct PrivatePlayerTransformRef {
    location: PrivateVector3Ref,
    rotation: PrivateRotatorRef,
}

fn private_player_transform_refs(
    payload: &[u8],
    refs: &[FStringRef],
) -> Vec<PrivatePlayerTransformRef> {
    let mut transforms = Vec::new();
    for (saved_idx, saved_ref) in refs.iter().enumerate() {
        if saved_ref.value != "m_SavedPlayers" {
            continue;
        }
        let search_end = (saved_idx + 80).min(refs.len());
        for player_id_idx in saved_idx + 1..search_end.saturating_sub(2) {
            if refs[player_id_idx].value != "m_PlayerID"
                || refs[player_id_idx + 1].value != "StrProperty"
                || refs[player_id_idx + 2].value != "Party ID 0"
            {
                continue;
            }
            let location_idx = refs
                .iter()
                .enumerate()
                .take(search_end)
                .skip(player_id_idx + 3)
                .find(|(_, reference)| reference.value == "m_Location")
                .map(|(idx, _)| idx);
            let rotation_idx = refs
                .iter()
                .enumerate()
                .take(search_end)
                .skip(player_id_idx + 3)
                .find(|(_, reference)| reference.value == "m_Rotation")
                .map(|(idx, _)| idx);
            let Some(location_idx) = location_idx else {
                continue;
            };
            let Some(rotation_idx) = rotation_idx else {
                continue;
            };
            let Some(location) = private_vector3_ref_at(payload, refs, location_idx) else {
                continue;
            };
            let Some(rotation) = private_rotator_ref_at(payload, refs, rotation_idx) else {
                continue;
            };
            transforms.push(PrivatePlayerTransformRef { location, rotation });
        }
    }
    transforms
}

fn private_vector3_ref_at(
    payload: &[u8],
    refs: &[FStringRef],
    name_idx: usize,
) -> Option<PrivateVector3Ref> {
    let offset = struct_triplet_value_offset_at(payload, refs, name_idx, "Vector")?;
    Some(PrivateVector3Ref {
        x: read_f64_at(payload, offset)?,
        y: read_f64_at(payload, offset + 8)?,
        z: read_f64_at(payload, offset + 16)?,
        offset,
    })
}

fn private_rotator_ref_at(
    payload: &[u8],
    refs: &[FStringRef],
    name_idx: usize,
) -> Option<PrivateRotatorRef> {
    let offset = struct_triplet_value_offset_at(payload, refs, name_idx, "Rotator")?;
    Some(PrivateRotatorRef {
        pitch: read_f64_at(payload, offset)?,
        yaw: read_f64_at(payload, offset + 8)?,
        roll: read_f64_at(payload, offset + 16)?,
        offset,
    })
}

fn struct_triplet_value_offset_at(
    payload: &[u8],
    refs: &[FStringRef],
    name_idx: usize,
    expected_struct_type: &str,
) -> Option<usize> {
    let type_ref = refs.get(name_idx + 1)?;
    if type_ref.value != "StructProperty" {
        return None;
    }
    let mut cursor = type_ref.len_offset + type_ref.total_len;
    let descriptor_count = u32::from_le_bytes(payload.get(cursor..cursor + 4)?.try_into().ok()?);
    cursor += 4;
    let (struct_type, struct_len) = read_payload_fstring_at(payload, cursor)?;
    cursor += struct_len;
    let script_count = u32::from_le_bytes(payload.get(cursor..cursor + 4)?.try_into().ok()?);
    cursor += 4;
    let (script_path, script_len) = read_payload_fstring_at(payload, cursor)?;
    cursor += script_len;
    let flags = u32::from_le_bytes(payload.get(cursor..cursor + 4)?.try_into().ok()?);
    let size = u32::from_le_bytes(payload.get(cursor + 4..cursor + 8)?.try_into().ok()?);
    let tag = *payload.get(cursor + 8)?;
    cursor += 9;
    if descriptor_count != 1
        || struct_type != expected_struct_type
        || script_count != 1
        || script_path != "/Script/CoreUObject"
        || flags != 0
        || size != 24
        || !matches!(tag, 0 | 8)
    {
        return None;
    }
    if cursor + 24 > payload.len() {
        return None;
    }
    Some(cursor)
}

fn read_payload_fstring_at(payload: &[u8], offset: usize) -> Option<(String, usize)> {
    let len = i32::from_le_bytes(payload.get(offset..offset + 4)?.try_into().ok()?);
    if len <= 0 {
        return None;
    }
    let len = len as usize;
    let raw = payload.get(offset + 4..offset + 4 + len)?;
    let body = raw.strip_suffix(&[0])?;
    Some((String::from_utf8_lossy(body).to_string(), 4 + len))
}

/// Rewrite a StrProperty's value FString, keeping the property's 4-byte
/// payload-size field in sync with the new value length. Used by both the
/// public and private name editors so a length-changing rename never leaves a
/// stale size word that misparses on the next load.
fn write_str_property_value(
    payload: &mut Vec<u8>,
    size_offset: usize,
    value_ref: &FStringRef,
    new_value: &str,
) -> Result<(), CoreError> {
    if size_offset + 4 > payload.len() {
        return Err(CoreError::Parse(
            "StrProperty size field points outside payload".to_string(),
        ));
    }
    let replacement = encode_fstring(new_value);
    payload[size_offset..size_offset + 4]
        .copy_from_slice(&(replacement.len() as u32).to_le_bytes());
    let start = value_ref.len_offset;
    let end = value_ref.len_offset + value_ref.total_len;
    payload.splice(start..end, replacement);
    Ok(())
}

fn apply_private_inventory_item_count_edit_to_payload(
    payload: &mut [u8],
    edit: &PrivateInventoryItemCountEdit,
) -> Result<(), CoreError> {
    let refs = scan_fstrings(payload, 0);
    let (start_idx, end_idx, scope) = inventory_item_region(&refs);
    if scope != "player_inventory_region" {
        return Err(CoreError::UnsupportedEdit(
            "private inventory item count edits require a detected player inventory region"
                .to_string(),
        ));
    }
    let mut matches = Vec::new();
    for (idx, reference) in refs.iter().enumerate().take(end_idx).skip(start_idx) {
        if reference.value != "m_ItemDefinition" {
            continue;
        }
        let Some(type_ref) = refs.get(idx + 1) else {
            continue;
        };
        if type_ref.value != "ObjectProperty" {
            continue;
        }
        let Some(path_ref) = refs
            .iter()
            .skip(idx + 2)
            .take(4)
            .find(|candidate| !is_property_type_name(candidate.value.as_str()))
        else {
            continue;
        };
        if !looks_item_definition_path(&path_ref.value)
            || !inventory_edit_matches_item(edit, &path_ref.value)
        {
            continue;
        }
        let value_offset = find_item_count_value_offset(payload, &refs, idx).ok_or_else(|| {
            CoreError::Validation(format!(
                "inventory item {} has no writable m_ItemCount IntProperty",
                path_ref.value
            ))
        })?;
        matches.push((path_ref.value.clone(), value_offset));
    }
    match matches.as_slice() {
        [] => Err(CoreError::Validation(
            "inventory item count edit did not match an item in the player inventory region"
                .to_string(),
        )),
        [(_, value_offset)] => {
            let end = value_offset + 4;
            if end > payload.len() {
                return Err(CoreError::Parse(
                    "inventory item count offset points outside payload".to_string(),
                ));
            }
            payload[*value_offset..end].copy_from_slice(&edit.count.to_le_bytes());
            Ok(())
        }
        matches => Err(CoreError::Validation(format!(
            "inventory item count edit matched {} items; use a more specific path",
            matches.len()
        ))),
    }
}

fn inventory_edit_matches_item(edit: &PrivateInventoryItemCountEdit, path: &str) -> bool {
    // Require every selector the edit provides to match. Matching on path alone
    // collapsed multiple stacks that share a definition path; honouring the id
    // too lets the edit target the stack the UI selected.
    let mut matched_any = false;
    if let Some(expected_path) = &edit.path {
        if expected_path != path {
            return false;
        }
        matched_any = true;
    }
    if let Some(expected_id) = edit.id.as_deref() {
        if expected_id != item_id_from_path(path) {
            return false;
        }
        matched_any = true;
    }
    matched_any
}

fn find_item_count_value_offset(
    payload: &[u8],
    refs: &[FStringRef],
    item_definition_idx: usize,
) -> Option<usize> {
    refs.iter()
        .enumerate()
        .skip(item_definition_idx + 3)
        .take(8)
        .find(|(_, candidate)| candidate.value == "m_ItemCount")
        .and_then(|(count_idx, _)| i32_value_offset_at(payload, refs, count_idx))
}

fn rebuild_compressed_stream(
    template: &CompressedStream,
    private_payload: &[u8],
    backend: &dyn codec_backend::CodecBackend,
) -> Result<Vec<u8>, CoreError> {
    if template.header_version != COMPRESSED_HEADER_V2 {
        return Err(CoreError::UnsupportedEdit(format!(
            "private stream header version 0x{:08x} is not writable yet",
            template.header_version
        )));
    }
    let max_chunk_size = usize::try_from(template.max_chunk_size).map_err(|_| {
        CoreError::Parse("private stream max chunk size does not fit usize".to_string())
    })?;
    if max_chunk_size == 0 {
        return Err(CoreError::Parse(
            "private stream max chunk size is zero".to_string(),
        ));
    }

    let payload_chunks = private_payload.chunks(max_chunk_size).collect::<Vec<_>>();
    let uncompressed_sizes = payload_chunks
        .iter()
        .map(|chunk| chunk.len() as u64)
        .collect::<Vec<_>>();
    let encode_chunks = payload_chunks
        .iter()
        .map(|chunk| codec_backend::CodecEncodeChunk {
            input: chunk,
            level: 6,
        })
        .collect::<Vec<_>>();
    let compressed_chunks = backend.compress_many(&encode_chunks)?;
    if compressed_chunks.len() != payload_chunks.len() {
        return Err(CoreError::Codec(format!(
            "codec backend compressed {} chunks, expected {}",
            compressed_chunks.len(),
            payload_chunks.len()
        )));
    }
    let decode_chunks = compressed_chunks
        .iter()
        .zip(payload_chunks.iter())
        .map(|(compressed, original)| codec_backend::CodecDecodeChunk {
            input: compressed,
            expected_size: original.len(),
        })
        .collect::<Vec<_>>();
    let roundtrip_chunks = backend.decompress_many(&decode_chunks)?;
    if roundtrip_chunks.len() != payload_chunks.len() {
        return Err(CoreError::Codec(format!(
            "codec backend post-compress validation decoded {} chunks, expected {}",
            roundtrip_chunks.len(),
            payload_chunks.len()
        )));
    }
    for (roundtrip, original) in roundtrip_chunks.iter().zip(payload_chunks.iter()) {
        if roundtrip != original {
            return Err(CoreError::Codec(
                "codec backend failed post-compress validation".to_string(),
            ));
        }
    }

    build_compressed_stream_v2(template, &compressed_chunks, &uncompressed_sizes)
}

fn build_compressed_stream_v2(
    template: &CompressedStream,
    compressed_chunks: &[Vec<u8>],
    uncompressed_sizes: &[u64],
) -> Result<Vec<u8>, CoreError> {
    if compressed_chunks.len() != uncompressed_sizes.len() {
        return Err(CoreError::Validation(
            "compressed chunk table length mismatch".to_string(),
        ));
    }
    let summary_compressed_size = compressed_chunks
        .iter()
        .map(|chunk| chunk.len() as u64)
        .sum::<u64>();
    let summary_uncompressed_size = uncompressed_sizes.iter().sum::<u64>();
    let mut out = Vec::new();
    out.extend_from_slice(&summary_uncompressed_size.to_le_bytes());
    out.extend_from_slice(&encode_fstring(&template.method));
    out.extend_from_slice(&PACKAGE_FILE_TAG.to_le_bytes());
    out.extend_from_slice(&COMPRESSED_HEADER_V2.to_le_bytes());
    out.extend_from_slice(&template.max_chunk_size.to_le_bytes());
    out.push(template.algorithm_id.unwrap_or(2));
    out.extend_from_slice(&summary_compressed_size.to_le_bytes());
    out.extend_from_slice(&summary_uncompressed_size.to_le_bytes());
    for (compressed, uncompressed_size) in compressed_chunks.iter().zip(uncompressed_sizes) {
        out.extend_from_slice(&(compressed.len() as u64).to_le_bytes());
        out.extend_from_slice(&uncompressed_size.to_le_bytes());
    }
    for compressed in compressed_chunks {
        out.extend_from_slice(compressed);
    }
    Ok(out)
}

fn encode_fstring(value: &str) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&((value.len() + 1) as i32).to_le_bytes());
    out.extend_from_slice(value.as_bytes());
    out.push(0);
    out
}

fn replace_public_fstring(
    data: &mut Vec<u8>,
    property_name: &str,
    new_value: &str,
) -> Result<(), CoreError> {
    if !data.starts_with(b"GSAV") {
        return Err(CoreError::UnsupportedEdit(
            "public edits are only available for GSAV files".to_string(),
        ));
    }
    let parts = split_gsav(data)?;
    let version = parts.version;
    let mut public_payload = parts.public_payload.to_vec();
    let compressed_stream = parts.compressed_stream.to_vec();
    let trailer = parts.trailer.to_vec();
    replace_str_property_fstring(&mut public_payload, property_name, new_value)?;
    *data = build_gsav(version, &public_payload, &compressed_stream, &trailer);
    Ok(())
}

/// One save or profile file the difficulty write will touch, captured up front
/// so backups, staging, and the atomic replace operate on already-validated
/// bytes — never on a partially-edited buffer.
struct DifficultyWritePlan {
    path: PathBuf,
    original: Vec<u8>,
    edited: Vec<u8>,
}

/// Orchestrate a difficulty write across any combination of save slots and a
/// PersistentDataList profile, mirroring `write_save_internal`'s
/// edit -> validate -> backup -> stage -> atomic-replace pipeline so a failure
/// at any step leaves every target untouched.
fn write_difficulty_internal(
    req: &DifficultyRequest,
    targets: &Value,
    backup: bool,
) -> Result<Value, CoreError> {
    let mut plans: Vec<DifficultyWritePlan> = Vec::new();

    // Difficulty is written ONLY to the profile's `ProfileData`. The profile
    // copy is the authoritative, profile-wide value the game reads on load;
    // editing a save's own copy has no in-game effect, so there is no per-save
    // write path.
    if let Some(profile) = targets.get("profile").filter(|v| !v.is_null()) {
        let path = PathBuf::from(profile.get("path").and_then(Value::as_str).ok_or_else(|| {
            CoreError::InvalidRequest("targets.profile.path is required".to_string())
        })?);
        let profile_id = profile
            .get("profileId")
            .and_then(Value::as_i64)
            .ok_or_else(|| {
                CoreError::InvalidRequest("targets.profile.profileId is required".to_string())
            })? as i32;
        let original = fs::read(&path)?;
        let mut edited = original.clone();
        write_profile_difficulty(&mut edited, profile_id, req)?;
        plans.push(DifficultyWritePlan {
            path,
            original,
            edited,
        });
    }

    if plans.is_empty() {
        return Err(CoreError::InvalidRequest(
            "write_difficulty requires at least one target".to_string(),
        ));
    }

    // Only touch files whose bytes actually changed; an unchanged target needs
    // no backup, no staging, and no replace.
    let changed: Vec<&DifficultyWritePlan> =
        plans.iter().filter(|p| p.original != p.edited).collect();

    // Back up every changed target with its OWN unique suffix BEFORE writing
    // anything. Difficulty edits to separate files (slot saves, profile) are
    // logically independent and must each restore on their own — they must NOT
    // be coupled by a shared suffix, or prepare_paired_persistent_data_list_restore
    // would auto-roll-back the profile when a single slot is restored.
    if backup {
        // Track the suffixes already chosen in this batch so two files with
        // different names (e.g. G1R-001.sav and PersistentDataList.sav) never
        // land on the same suffix — create_backup_copy only checks its own
        // file's backup path for collisions, not its siblings'.
        let mut used_suffixes: Vec<String> = Vec::new();
        for p in &changed {
            // Also avoid suffixes already used by OTHER files' backups (e.g. slot
            // backups), so a profile-only PDL backup never shares a suffix with a
            // slot backup — which the suffix-only paired-restore heuristic would
            // otherwise wrongly treat as that slot's companion.
            let mut avoid = used_suffixes.clone();
            avoid.extend(existing_foreign_backup_suffixes(&p.path));
            let backup_path = create_unique_backup_avoiding(&p.path, &avoid)?;
            if let Some(name) = backup_path.file_name().and_then(|n| n.to_str()) {
                if let Ok(prefix) = backup_file_prefix(&p.path) {
                    if let Some(suffix) = name.strip_prefix(&prefix) {
                        used_suffixes.push(suffix.to_string());
                    }
                }
            }
        }
    }

    // Stage every edited buffer to a tmp file and validate it on disk before
    // any target is replaced.
    let mut tmps: Vec<(PathBuf, PathBuf)> = Vec::new();
    for p in &changed {
        let tmp = p.path.with_extension("sav.tmp-goresave");
        fs::write(&tmp, &p.edited)?;
        inspect_save(&tmp, false)?;
        tmps.push((p.path.clone(), tmp));
    }

    // Atomic replace: begin each (move-aside + rename-in). If any begin_replace
    // fails, roll back the ones already begun so no target is left swapped.
    // Mirrors write_save_internal's PendingReplace ownership: commit/rollback
    // each consume the value, so we collect by value and drain.
    let mut committed: Vec<PendingReplace> = Vec::new();
    for (target, tmp) in &tmps {
        match begin_replace(target, tmp) {
            Ok(pending) => committed.push(pending),
            Err(err) => {
                for pending in committed {
                    pending.rollback();
                }
                return Err(err);
            }
        }
    }
    for pending in committed {
        pending.commit();
    }

    // The bytes on disk changed; drop any cached decoded payloads.
    for (target, _) in &tmps {
        invalidate_decoded_payload_cache(target);
    }

    Ok(json!({
        "targetsWritten": changed.len(),
        "paths": changed
            .iter()
            .map(|p| p.path.display().to_string())
            .collect::<Vec<_>>(),
    }))
}

fn replace_str_property_fstring(
    payload: &mut Vec<u8>,
    property_name: &str,
    new_value: &str,
) -> Result<(), CoreError> {
    let refs = scan_fstrings(payload, 0);
    replace_str_property_fstring_in_range(payload, &refs, 0, refs.len(), property_name, new_value)
}

fn replace_str_property_fstring_in_range(
    payload: &mut Vec<u8>,
    refs: &[FStringRef],
    start_idx: usize,
    end_idx: usize,
    property_name: &str,
    new_value: &str,
) -> Result<(), CoreError> {
    let name_idx = refs
        .iter()
        .enumerate()
        .take(end_idx)
        .skip(start_idx)
        .find(|(_, r)| r.value == property_name)
        .map(|(idx, _)| idx)
        .ok_or_else(|| CoreError::Parse(format!("property {property_name} was not found")))?;
    if name_idx + 2 >= end_idx {
        return Err(CoreError::Parse(format!(
            "value for {property_name} was not found"
        )));
    }
    let type_ref = refs
        .get(name_idx + 1)
        .ok_or_else(|| CoreError::Parse(format!("type for {property_name} was not found")))?;
    if type_ref.value != "StrProperty" {
        return Err(CoreError::Parse(format!(
            "property {property_name} is not a StrProperty"
        )));
    }
    let value_ref = refs
        .get(name_idx + 2)
        .ok_or_else(|| CoreError::Parse(format!("value for {property_name} was not found")))?;
    if value_ref.utf16 {
        return Err(CoreError::UnsupportedEdit(
            "UTF-16 FString replacement is not implemented yet".to_string(),
        ));
    }
    let size_offset = type_ref.len_offset + type_ref.total_len + 4;
    write_str_property_value(payload, size_offset, value_ref, new_value)
}

fn sha1_hex(data: &[u8]) -> String {
    let mut hasher = Sha1::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

#[unsafe(no_mangle)]
pub extern "C" fn goresave_execute(request_json: *const c_char) -> *mut c_char {
    if request_json.is_null() {
        return cstring_ptr(execute_json(r#"{"command":null}"#));
    }
    let input = unsafe { CStr::from_ptr(request_json) }
        .to_string_lossy()
        .to_string();
    cstring_ptr(execute_json(&input))
}

#[unsafe(no_mangle)]
pub extern "C" fn goresave_free(ptr: *mut c_char) {
    if ptr.is_null() {
        return;
    }
    unsafe {
        drop(CString::from_raw(ptr));
    }
}

fn cstring_ptr(value: String) -> *mut c_char {
    match CString::new(value) {
        Ok(s) => s.into_raw(),
        Err(_) => ptr::null_mut(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    use tempfile::tempdir;

    #[test]
    fn summarize_typed_parse_reports_status() {
        // skipped for previews (result = None)
        let skipped = summarize_typed_parse_result(&[], None);
        assert_eq!(skipped["status"], "skipped_preview");

        // ok for a valid minimal root object
        let mut payload = fstring("/Script/Test.Save");
        payload.push(0); // object flag
        payload.extend_from_slice(&fstring("m_X"));
        payload.extend_from_slice(&fstring("IntProperty"));
        payload.extend_from_slice(&0u32.to_le_bytes()); // array_index
        payload.extend_from_slice(&4u32.to_le_bytes()); // size
        payload.push(0); // tag_flags
        payload.extend_from_slice(&7i32.to_le_bytes());
        payload.extend_from_slice(&fstring("None"));
        payload.extend_from_slice(&0u32.to_le_bytes()); // footer
        let parse_ok = properties::parse_private_root(&payload);
        let ok = summarize_typed_parse_result(&payload, Some(&parse_ok));
        assert_eq!(ok["status"], "ok");
        assert_eq!(ok["propertyCount"], 1);
        assert_eq!(ok["consumed"], payload.len());

        // failed for garbage
        let parse_bad = properties::parse_private_root(&[1, 2, 3, 4, 5, 6]);
        let bad = summarize_typed_parse_result(&[1, 2, 3, 4, 5, 6], Some(&parse_bad));
        assert_eq!(bad["status"], "failed");
    }

    fn fstring(value: &str) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&((value.len() + 1) as i32).to_le_bytes());
        out.extend_from_slice(value.as_bytes());
        out.push(0);
        out
    }

    // --- Typed GVAS test-fixture builders -------------------------------------
    //
    // The difficulty write now operates on the typed property tree, so fixtures
    // must be REAL parseable GVAS, not flat fstring soup. These helpers mirror
    // the real save shape: the difficulty fields are ObjectProperty asset paths
    // (length-changing on edit) and BoolProperty toggles, nested inside a
    // `CustomPayload` MapProperty -> InstancedStruct so a missed enclosing-size
    // fixup desyncs the strict re-parse.

    /// `[u32 size][u8 tag_flags]` value header preceding a sized property value.
    fn diff_header(size: u32, flags: u8) -> Vec<u8> {
        let mut out = 0u32.to_le_bytes().to_vec(); // array_index
        out.extend_from_slice(&size.to_le_bytes());
        out.push(flags);
        out
    }

    /// `name`/`type` tag pair.
    fn diff_tag(name: &str, type_name: &str) -> Vec<u8> {
        let mut out = fstring(name);
        out.extend_from_slice(&fstring(type_name));
        out
    }

    /// An ObjectProperty whose value is an asset path (the difficulty sub-settings).
    fn obj_prop(name: &str, path: &str) -> Vec<u8> {
        let value = fstring(path);
        let mut out = diff_tag(name, "ObjectProperty");
        out.extend_from_slice(&diff_header(value.len() as u32, 0));
        out.extend_from_slice(&value);
        out
    }

    /// A BoolProperty (value carried in the 0x10 tag bit).
    fn bool_prop(name: &str, on: bool) -> Vec<u8> {
        let mut out = diff_tag(name, "BoolProperty");
        out.extend_from_slice(&diff_header(
            0,
            if on {
                properties::TAG_FLAG_BOOL_TRUE
            } else {
                0
            },
        ));
        out
    }

    /// The difficulty property list (no `None` terminator).
    fn difficulty_props(preset_level: &str, perma_name: &str, perma_on: bool) -> Vec<u8> {
        let mut out = obj_prop(
            "m_difficultyPreset",
            &format!("/Script/Angelscript.DifficultyPreset_{preset_level}"),
        );
        out.extend_from_slice(&obj_prop(
            "m_customCombatSettings",
            &format!("/Script/Angelscript.CombatDifficultySettings_{preset_level}"),
        ));
        out.extend_from_slice(&obj_prop(
            "m_customResourcesSettings",
            &format!("/Script/Angelscript.ResourcesDifficultySettings_{preset_level}"),
        ));
        out.extend_from_slice(&obj_prop(
            "m_customProgressionSettings",
            &format!("/Script/Angelscript.ProgressionDifficultySettings_{preset_level}"),
        ));
        out.extend_from_slice(&bool_prop(perma_name, perma_on));
        out.extend_from_slice(&bool_prop("m_FakeSloppyCombos", false));
        out
    }

    /// One profile's property list (no `None`): id + nested difficulty in a
    /// `ProfileData` InstancedStruct (mirrors the real ProfileData nesting).
    fn profile_props(id: i32, preset_level: &str, perma_on: bool) -> Vec<u8> {
        let mut out = diff_tag("m_ProfileName", "StrProperty");
        let name = fstring(&format!("Profile{id}"));
        out.extend_from_slice(&diff_header(name.len() as u32, 0));
        out.extend_from_slice(&name);
        out.extend_from_slice(&diff_tag("m_ProfileId", "IntProperty"));
        out.extend_from_slice(&diff_header(4, 0));
        out.extend_from_slice(&id.to_le_bytes());
        // ProfileData InstancedStruct wrapping the difficulty fields.
        let mut struct_body = difficulty_props(preset_level, "m_PermaDeath", perma_on);
        struct_body.extend_from_slice(&fstring("None"));
        let mut instanced = fstring("/Script/G1R.ProfileData");
        instanced.extend_from_slice(&(struct_body.len() as u32).to_le_bytes());
        instanced.extend_from_slice(&struct_body);
        out.extend_from_slice(&diff_tag("ProfileData", "StructProperty"));
        out.extend_from_slice(&1u32.to_le_bytes());
        out.extend_from_slice(&fstring("InstancedStruct"));
        out.extend_from_slice(&1u32.to_le_bytes());
        out.extend_from_slice(&fstring("/Script/StructUtils"));
        out.extend_from_slice(&diff_header(
            instanced.len() as u32,
            properties::TAG_FLAG_NATIVE_SERIALIZE,
        ));
        out.extend_from_slice(&instanced);
        out
    }

    /// Build a 2-profile PersistentDataList.sav (full GVAS object): profile 0 =
    /// Custom, profile 1 = Easy, in an `m_Profiles` ArrayProperty<StructProperty>.
    ///
    /// Layout per the parser: tag (name/type), the ARRAY inner descriptor
    /// (inner_count u32 + "StructProperty" + struct descriptor), then
    /// array_index u32 + size u32 + tag_flags u8, then the body
    /// `[element_count u32][struct0 props + None][struct1 props + None]`.
    fn difficulty_persistent_profiles() -> Vec<u8> {
        let props = persistent_profiles_property_block();

        // The object body (class + flag + props + None + footer).
        let mut object = fstring("/Script/G1R.PersistentDataList");
        object.push(0); // object flag
        object.extend_from_slice(&props);
        object.extend_from_slice(&fstring("None"));
        object.extend_from_slice(&0u32.to_le_bytes()); // footer

        // Prepend a GVAS header (real files carry a variable-length header before
        // the object). The bytes are opaque filler the probe skips; they must not
        // themselves parse as a clean full-file object. `GVAS` magic satisfies the
        // starts_with check; the version/filler block stands in for the real
        // save-game/package versions + custom-version array + class name.
        let mut data = b"GVAS".to_vec();
        data.extend_from_slice(&[0u8; 24]); // opaque header filler
        data.extend_from_slice(&object);
        data
    }

    /// The `m_Profiles` ArrayProperty<StructProperty> property block shared by
    /// both PersistentDataList framings (object-wrapped and bare).
    fn persistent_profiles_property_block() -> Vec<u8> {
        let mut body = 2u32.to_le_bytes().to_vec(); // element count
        let mut p0 = profile_props(0, "Custom", false);
        p0.extend_from_slice(&fstring("None"));
        let mut p1 = profile_props(1, "Easy", false);
        p1.extend_from_slice(&fstring("None"));
        body.extend_from_slice(&p0);
        body.extend_from_slice(&p1);

        let mut props = diff_tag("m_Profiles", "ArrayProperty");
        props.extend_from_slice(&1u32.to_le_bytes()); // inner_count
        props.extend_from_slice(&fstring("StructProperty")); // inner type
        props.extend_from_slice(&1u32.to_le_bytes()); // struct desc count
        props.extend_from_slice(&fstring("ProfileEntry")); // struct type
        props.extend_from_slice(&1u32.to_le_bytes()); // package count
        props.extend_from_slice(&fstring("/Script/G1R")); // package
        props.extend_from_slice(&0u32.to_le_bytes()); // array_index
        props.extend_from_slice(&(body.len() as u32).to_le_bytes()); // size
        props.push(0); // tag_flags
        props.extend_from_slice(&body);
        props
    }

    /// Build a 2-profile PersistentDataList.sav in the STANDARD GVAS save-game
    /// shape: a variable-length header ending with the save-game class name
    /// FString, followed DIRECTLY by the property list terminated by `None` — no
    /// nested `class`/flag object framing and no footer. This is the layout the
    /// object-only probe used to hard-fail on.
    fn difficulty_persistent_profiles_bare() -> Vec<u8> {
        let mut data = b"GVAS".to_vec();
        data.extend_from_slice(&[0u8; 24]); // opaque version/custom-version filler
        // The header ends with the save-game class name FString, directly before
        // the property list — what class_name_fstring_ends_at validates.
        data.extend_from_slice(&fstring("/Script/G1R.PersistentDataList"));
        data.extend_from_slice(&persistent_profiles_property_block());
        data.extend_from_slice(&fstring("None")); // property-list terminator, then EOF
        data
    }

    #[test]
    fn parse_profile_file_handles_bare_property_list_framing() {
        // A standard GVAS save-game file has no nested class+flag+footer object:
        // the property list follows the header directly. parse_profile_file must
        // locate and fully consume it, and write_profile_difficulty must edit a
        // profile in place and still strictly re-parse.
        let mut data = difficulty_persistent_profiles_bare();

        let root = parse_profile_file(&data).unwrap();
        assert_eq!(root.consumed, data.len());
        assert!(profile_element(&root, 1).is_some());

        let req = DifficultyRequest {
            preset: Some("Hard".into()),
            combat: None,
            resources: None,
            progression: None,
            flow_helper: None,
            permadeath: None,
        };
        write_profile_difficulty(&mut data, 1, &req).unwrap();

        let root = parse_profile_file(&data).unwrap();
        assert_eq!(root.consumed, data.len());
        let p1 = profile_difficulty_path(&root, 1, "m_difficultyPreset")
            .unwrap()
            .unwrap();
        let p1v =
            properties::resolve(&root.properties, &properties::parse_path(&p1).unwrap()).unwrap();
        assert_eq!(
            p1v.value,
            properties::PropertyValue::Object("/Script/Angelscript.DifficultyPreset_Hard".into()),
        );
        // Profile 0 must remain untouched.
        let p0 = profile_difficulty_path(&root, 0, "m_difficultyPreset")
            .unwrap()
            .unwrap();
        let p0v =
            properties::resolve(&root.properties, &properties::parse_path(&p0).unwrap()).unwrap();
        assert_eq!(
            p0v.value,
            properties::PropertyValue::Object("/Script/Angelscript.DifficultyPreset_Custom".into()),
        );
    }

    #[test]
    fn parse_profile_file_rejects_terminator_only_parse() {
        // A header followed only by the top-level `None` terminator parses as an
        // empty property list that consumes to EOF. Without the m_Profiles guard
        // that empty parse would be accepted as the profile root, letting a
        // corrupt length-changing edit slip past the post-edit validation gate.
        let mut data = b"GVAS".to_vec();
        data.extend_from_slice(&[0u8; 24]); // header filler
        data.extend_from_slice(&fstring("None")); // bare terminator, no m_Profiles

        // Sanity: the bare list DOES parse-and-consume from the terminator
        // offset (so the guard, not a parse failure, is what rejects it).
        assert!(
            (0..data.len()).any(|off| {
                properties::parse_property_list_root_at(&data, off)
                    .map(|r| r.consumed == data.len())
                    .unwrap_or(false)
            }),
            "precondition: some offset parses an empty list to EOF",
        );
        assert!(
            parse_profile_file(&data).is_err(),
            "a terminator-only file has no m_Profiles and must be rejected",
        );
    }

    #[test]
    fn parse_profile_file_rejects_corrupt_header_prefix() {
        // The property list (with m_Profiles) parses to EOF, but the prefix is
        // not a valid GVAS header — no class-name FString ends at the list start.
        // The scan must NOT accept it by treating the garbage prefix as a header.
        let mut data = b"GVAS".to_vec();
        data.extend_from_slice(&[0xFFu8; 20]); // garbage where a header would be
        data.extend_from_slice(&persistent_profiles_property_block());
        data.extend_from_slice(&fstring("None"));

        // Sanity: the property list itself parses-and-consumes from its start,
        // so the class-name-prefix check (not a parse failure) is what rejects it.
        let list_off = 4 + 20;
        assert!(
            properties::parse_property_list_root_at(&data, list_off)
                .map(|r| r.consumed == data.len()
                    && r.properties.iter().any(|p| p.name == "m_Profiles"))
                .unwrap_or(false),
            "precondition: the property list parses to EOF from its start",
        );
        assert!(
            parse_profile_file(&data).is_err(),
            "a corrupt (non-header) prefix must be rejected",
        );
    }

    #[test]
    fn write_difficulty_internal_requires_at_least_one_target() {
        let req = DifficultyRequest {
            preset: Some("Hard".into()),
            combat: None,
            resources: None,
            progression: None,
            flow_helper: None,
            permadeath: None,
        };
        let err = write_difficulty_internal(&req, &json!({}), false).unwrap_err();
        assert!(matches!(err, CoreError::InvalidRequest(_)));
    }

    #[test]
    fn write_difficulty_internal_profile_only_needs_no_codec() {
        let dir = tempdir().unwrap();
        let profile_path = dir.path().join("PersistentDataList.sav");
        fs::write(&profile_path, difficulty_persistent_profiles()).unwrap();

        let req = DifficultyRequest {
            preset: Some("Hard".into()),
            combat: None,
            resources: None,
            progression: None,
            flow_helper: None,
            permadeath: None,
        };
        let targets = json!({
            "profile": { "path": profile_path.display().to_string(), "profileId": 1 },
        });

        // The profile-only path needs no codec backend.
        let response = write_difficulty_internal(&req, &targets, true).unwrap();
        assert_eq!(response["targetsWritten"], 1);

        let written = fs::read(&profile_path).unwrap();
        let presets: Vec<_> = scan_fstrings(&written, 0)
            .into_iter()
            .filter(|r| r.value.contains("DifficultyPreset_"))
            .map(|r| r.value)
            .collect();
        assert!(presets.iter().any(|p| p.ends_with("DifficultyPreset_Hard")));
        assert!(!presets.iter().any(|p| p.ends_with("DifficultyPreset_Easy")));
        assert!(dir.path().join("goresave_backups").exists());
    }

    #[test]
    fn write_difficulty_profile_backup_avoids_existing_slot_backup_suffix() {
        // A standalone profile backup must not reuse a slot backup's suffix, or
        // the suffix-only paired-restore heuristic would later treat it as that
        // slot's companion and roll the profile back on a slot restore.
        let dir = tempdir().unwrap();
        let pdl = dir.path().join("PersistentDataList.sav");
        fs::write(&pdl, difficulty_persistent_profiles()).unwrap();

        // Pre-existing slot backup whose suffix is the current second.
        let subfolder = dir.path().join("goresave_backups");
        fs::create_dir_all(&subfolder).unwrap();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        fs::write(
            subfolder.join(format!("G1R-001.sav.bak.{now}")),
            b"slot backup",
        )
        .unwrap();

        let req = DifficultyRequest {
            preset: Some("Hard".into()),
            combat: None,
            resources: None,
            progression: None,
            flow_helper: None,
            permadeath: None,
        };
        let targets = json!({
            "profile": { "path": pdl.display().to_string(), "profileId": 1 },
        });
        write_difficulty_internal(&req, &targets, true).unwrap();

        // The profile backup must NOT have landed on the slot backup's suffix.
        assert!(
            !subfolder
                .join(format!("PersistentDataList.sav.bak.{now}"))
                .exists(),
            "profile backup must avoid the existing slot backup suffix",
        );
        // Exactly one profile backup was created (under a different suffix).
        let pdl_backups = fs::read_dir(&subfolder)
            .unwrap()
            .flatten()
            .filter(|e| {
                e.file_name()
                    .to_string_lossy()
                    .starts_with("PersistentDataList.sav.bak.")
            })
            .count();
        assert_eq!(pdl_backups, 1);
    }

    #[test]
    fn write_difficulty_accepts_top_level_profile_shape() {
        // The spec documents `profile: { path, profileId }` at the top level of
        // the payload (no `targets` wrapper). execute_json_inner must accept it
        // so direct/API callers following the docs can write profile difficulty.
        let dir = tempdir().unwrap();
        let profile_path = dir.path().join("PersistentDataList.sav");
        fs::write(&profile_path, difficulty_persistent_profiles()).unwrap();

        let input = json!({
            "command": "write_difficulty",
            "payload": {
                "difficulty": { "preset": "Hard" },
                "profile": {
                    "path": profile_path.display().to_string(),
                    "profileId": 1,
                },
                "backup": true,
            }
        })
        .to_string();

        let response = execute_json_inner(&input).unwrap();
        assert_eq!(response["targetsWritten"], 1);
        let written = fs::read(&profile_path).unwrap();
        assert!(
            scan_fstrings(&written, 0)
                .into_iter()
                .any(|r| r.value.ends_with("DifficultyPreset_Hard")),
        );
    }

    #[test]
    fn write_profile_difficulty_without_preset_edits_only_bools() {
        // A bool-only request (no `preset`) must leave the stored preset and
        // sub-setting asset paths untouched and patch only the toggles, so a
        // profile with an unrecognised preset class can still take a
        // flow-helper / permadeath edit without rewriting the preset.
        let mut data = difficulty_persistent_profiles();
        // Profile 1 stores Easy; assert it survives a permadeath-only edit.
        let req = DifficultyRequest {
            preset: None,
            combat: None,
            resources: None,
            progression: None,
            flow_helper: Some(true),
            permadeath: None,
        };
        write_profile_difficulty(&mut data, 1, &req).unwrap();

        let root = parse_profile_file(&data).unwrap();
        assert_eq!(root.consumed, data.len());
        // Profile 1's preset is unchanged (still Easy), proving no preset write.
        let p1 = profile_difficulty_path(&root, 1, "m_difficultyPreset")
            .unwrap()
            .unwrap();
        let p1v =
            properties::resolve(&root.properties, &properties::parse_path(&p1).unwrap()).unwrap();
        assert_eq!(
            p1v.value,
            properties::PropertyValue::Object("/Script/Angelscript.DifficultyPreset_Easy".into()),
        );
        // The flow-helper bool was written.
        let flow = profile_difficulty_path(&root, 1, "m_FakeSloppyCombos")
            .unwrap()
            .unwrap();
        let flowv =
            properties::resolve(&root.properties, &properties::parse_path(&flow).unwrap()).unwrap();
        assert_eq!(flowv.value, properties::PropertyValue::Bool(true));
    }

    #[test]
    fn write_profile_difficulty_custom_preserves_omitted_sub_levels() {
        // A Custom request that supplies only `resources` must rewrite ONLY the
        // resources sub-setting and leave combat/progression as stored — so an
        // unrecognised stored Custom sub-setting survives a partial edit.
        let mut data = difficulty_persistent_profiles(); // profile 0 = Custom
        let combat_before = {
            let root = parse_profile_file(&data).unwrap();
            let path = profile_difficulty_path(&root, 0, "m_customCombatSettings")
                .unwrap()
                .unwrap();
            properties::resolve(&root.properties, &properties::parse_path(&path).unwrap())
                .unwrap()
                .value
                .clone()
        };

        let req = DifficultyRequest {
            preset: Some("Custom".into()),
            combat: None,
            resources: Some("Novice".into()),
            progression: None,
            flow_helper: None,
            permadeath: None,
        };
        write_profile_difficulty(&mut data, 0, &req).unwrap();

        let root = parse_profile_file(&data).unwrap();
        assert_eq!(root.consumed, data.len());
        // resources was rewritten to Easy.
        let r = profile_difficulty_path(&root, 0, "m_customResourcesSettings")
            .unwrap()
            .unwrap();
        assert_eq!(
            properties::resolve(&root.properties, &properties::parse_path(&r).unwrap())
                .unwrap()
                .value,
            properties::PropertyValue::Object(
                "/Script/Angelscript.ResourcesDifficultySettings_Easy".into()
            ),
        );
        // combat is untouched (omitted from the request).
        let c = profile_difficulty_path(&root, 0, "m_customCombatSettings")
            .unwrap()
            .unwrap();
        assert_eq!(
            properties::resolve(&root.properties, &properties::parse_path(&c).unwrap())
                .unwrap()
                .value,
            combat_before,
        );
    }

    #[test]
    fn write_profile_difficulty_targets_only_the_named_profile() {
        // 2 profiles: 0 = Custom, 1 = Easy. Editing profile 1 to Hard must touch
        // ONLY profile 1; profile 0 stays Custom. The edited file must strictly
        // re-parse from its GVAS object (enclosing sizes propagated).
        let mut data = difficulty_persistent_profiles();
        let req = DifficultyRequest {
            preset: Some("Hard".into()),
            combat: None,
            resources: None,
            progression: None,
            flow_helper: None,
            permadeath: None,
        };
        write_profile_difficulty(&mut data, 1, &req).unwrap();

        // Strict re-parse from the GVAS object (the validation gate already ran,
        // but assert here to be explicit).
        let root = parse_profile_file(&data).unwrap();
        assert_eq!(root.consumed, data.len());

        // Profile 0 untouched (Custom), profile 1 now Hard, no leftover Easy.
        let p0 = profile_difficulty_path(&root, 0, "m_difficultyPreset")
            .unwrap()
            .unwrap();
        let p0v =
            properties::resolve(&root.properties, &properties::parse_path(&p0).unwrap()).unwrap();
        assert_eq!(
            p0v.value,
            properties::PropertyValue::Object("/Script/Angelscript.DifficultyPreset_Custom".into()),
        );
        let p1 = profile_difficulty_path(&root, 1, "m_difficultyPreset")
            .unwrap()
            .unwrap();
        let p1v =
            properties::resolve(&root.properties, &properties::parse_path(&p1).unwrap()).unwrap();
        assert_eq!(
            p1v.value,
            properties::PropertyValue::Object("/Script/Angelscript.DifficultyPreset_Hard".into()),
        );
    }

    #[test]
    fn write_profile_difficulty_writes_permadeath_under_alternate_spelling() {
        // The fixture stores permadeath as `m_PermaDeath` in each profile. Novice
        // forces permadeath off even though Some(true) was requested, and the
        // write must target the alternate spelling that is actually present.
        let mut data = difficulty_persistent_profiles();
        // Seed profile 1's permadeath ON so we can observe it being forced off.
        {
            let root = parse_profile_file(&data).unwrap();
            let path = profile_difficulty_path(&root, 1, "m_PermaDeath")
                .unwrap()
                .unwrap();
            let chain = properties::resolve_chain(
                &root.properties,
                &properties::parse_path(&path).unwrap(),
            )
            .unwrap();
            let target = chain.target.clone();
            drop(root);
            properties::patch_scalar(
                data.as_mut_slice(),
                &target,
                properties::ScalarValue::Bool(true),
            )
            .unwrap();
        }

        let req = DifficultyRequest {
            preset: Some("Novice".into()),
            combat: None,
            resources: None,
            progression: None,
            flow_helper: None,
            permadeath: Some(true),
        };
        write_profile_difficulty(&mut data, 1, &req).unwrap();

        let root = parse_profile_file(&data).unwrap();
        let path = profile_difficulty_path(&root, 1, "m_PermaDeath")
            .unwrap()
            .unwrap();
        let v =
            properties::resolve(&root.properties, &properties::parse_path(&path).unwrap()).unwrap();
        assert_eq!(
            v.value,
            properties::PropertyValue::Bool(false),
            "Novice-forced permadeath-off must be written under the m_PermaDeath spelling",
        );
    }

    fn minimal_stream() -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&0u64.to_le_bytes());
        out.extend_from_slice(&fstring("Oodle"));
        out.extend_from_slice(&PACKAGE_FILE_TAG.to_le_bytes());
        out.extend_from_slice(&COMPRESSED_HEADER_V2.to_le_bytes());
        out.extend_from_slice(&131_072u64.to_le_bytes());
        out.push(2);
        out.extend_from_slice(&0u64.to_le_bytes());
        out.extend_from_slice(&0u64.to_le_bytes());
        out
    }

    fn public_payload(name: &str) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&fstring("m_PlayerSaveName"));
        out.extend_from_slice(&fstring("StrProperty"));
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&((name.len() + 1 + 4) as u32).to_le_bytes());
        out.push(0);
        out.extend_from_slice(&fstring(name));
        out.extend_from_slice(&fstring("None"));
        out
    }

    fn str_property(name: &str, value: &str) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&fstring(name));
        out.extend_from_slice(&fstring("StrProperty"));
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&((value.len() + 1 + 4) as u32).to_le_bytes());
        out.push(0);
        out.extend_from_slice(&fstring(value));
        out
    }

    fn string_array_property(name: &str, values: &[&str]) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&fstring(name));
        out.extend_from_slice(&fstring("ArrayProperty"));
        out.extend_from_slice(&fstring("StrProperty"));
        for value in values {
            out.extend_from_slice(&fstring(value));
        }
        out
    }

    fn int_property(name: &str, value: i32) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&fstring(name));
        out.extend_from_slice(&fstring("IntProperty"));
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&4u32.to_le_bytes());
        out.push(0);
        out.extend_from_slice(&value.to_le_bytes());
        out
    }

    fn bool_property(name: &str, value: bool) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&fstring(name));
        out.extend_from_slice(&fstring("BoolProperty"));
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
        // The bool lives in the 0x10 (TAG_FLAG_BOOL_TRUE) tag bit, as real saves
        // and the typed parser/writer encode it.
        out.push(if value {
            properties::TAG_FLAG_BOOL_TRUE
        } else {
            0
        });
        out
    }

    fn private_str_property(name: &str, value: &str) -> Vec<u8> {
        let payload = fstring(value);
        let mut out = Vec::new();
        out.extend_from_slice(&fstring(name));
        out.extend_from_slice(&fstring("StrProperty"));
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        out.push(0);
        out.extend_from_slice(&payload);
        out
    }

    /// ByteProperty in its enum-as-FString form.
    fn private_byte_enum_property(name: &str, value: &str) -> Vec<u8> {
        let payload = fstring(value);
        let mut out = Vec::new();
        out.extend_from_slice(&fstring(name));
        out.extend_from_slice(&fstring("ByteProperty"));
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        out.push(0);
        out.extend_from_slice(&payload);
        out
    }

    /// ByteProperty in its plain one-byte form.
    fn private_byte_plain_property(name: &str, value: u8) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&fstring(name));
        out.extend_from_slice(&fstring("ByteProperty"));
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&1u32.to_le_bytes());
        out.push(0);
        out.push(value);
        out
    }

    fn double_property(name: &str, value: f64) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&fstring(name));
        out.extend_from_slice(&fstring("DoubleProperty"));
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&8u32.to_le_bytes());
        out.push(0);
        out.extend_from_slice(&value.to_le_bytes());
        out
    }

    fn float_property(name: &str, value: f32) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&fstring(name));
        out.extend_from_slice(&fstring("FloatProperty"));
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&4u32.to_le_bytes());
        out.push(0);
        out.extend_from_slice(&value.to_le_bytes());
        out
    }

    fn struct_double_triplet_property(
        name: &str,
        struct_type: &str,
        x: f64,
        y: f64,
        z: f64,
    ) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&fstring(name));
        out.extend_from_slice(&fstring("StructProperty"));
        out.extend_from_slice(&1u32.to_le_bytes());
        out.extend_from_slice(&fstring(struct_type));
        out.extend_from_slice(&1u32.to_le_bytes());
        out.extend_from_slice(&fstring("/Script/CoreUObject"));
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&24u32.to_le_bytes());
        out.push(8);
        out.extend_from_slice(&x.to_le_bytes());
        out.extend_from_slice(&y.to_le_bytes());
        out.extend_from_slice(&z.to_le_bytes());
        out
    }

    fn gameplay_attribute(name: &str, base_value: f32, current_value: f32) -> Vec<u8> {
        [
            fstring("GameplayAttributeData"),
            fstring("/Script/GameplayAbilities"),
            fstring(name),
            float_property("BaseValue", base_value),
            float_property("CurrentValue", current_value),
            fstring("None"),
        ]
        .concat()
    }

    fn player_transform_payload() -> Vec<u8> {
        [
            fstring("m_SavedPlayers"),
            str_property("m_PlayerID", "Party ID 1"),
            struct_double_triplet_property("m_Location", "Vector", 1.0, 2.0, 3.0),
            struct_double_triplet_property("m_Rotation", "Rotator", 4.0, 5.0, 6.0),
            str_property("m_PlayerID", "Party ID 0"),
            struct_double_triplet_property("m_Location", "Vector", 10.0, 20.0, 30.0),
            struct_double_triplet_property("m_Rotation", "Rotator", 40.0, 50.0, 60.0),
            fstring("m_ResumeAtTransform"),
        ]
        .concat()
    }

    fn hero_attribute_payload() -> Vec<u8> {
        [
            fstring("Lurker"),
            fstring("AttributeSetsByClass"),
            fstring("/Script/G1R.AttributeSet_Health"),
            gameplay_attribute("Health", 12.0, 12.0),
            fstring("Hero"),
            fstring("AttributeSetsByClass"),
            fstring("MapProperty"),
            fstring("ObjectProperty"),
            fstring("StructProperty"),
            fstring("CharacterStateSaveGameData_AttributeSet"),
            fstring("/Script/G1R"),
            fstring("/Script/G1R.AttributeSet_Health"),
            fstring("Attributes"),
            fstring("MapProperty"),
            fstring("NameProperty"),
            fstring("StructProperty"),
            gameplay_attribute("Health", 40.0, 25.0),
            gameplay_attribute("MaxHealth", 40.0, 40.0),
            fstring("/Script/G1R.AttributeSet_Strength"),
            fstring("Attributes"),
            gameplay_attribute("Strength", 10.0, 10.0),
            fstring("/Script/G1R.AttributeSet_Dexterity"),
            fstring("Attributes"),
            gameplay_attribute("Dexterity", 10.0, 10.0),
            fstring("MemorizedEvents"),
        ]
        .concat()
    }

    fn read_test_f32_property_at(
        payload: &[u8],
        refs: &[FStringRef],
        name_idx: usize,
    ) -> Option<f32> {
        let type_ref = refs.get(name_idx + 1)?;
        if type_ref.value != "FloatProperty" {
            return None;
        }
        let cursor = type_ref.len_offset + type_ref.total_len;
        if cursor + 13 > payload.len() {
            return None;
        }
        let size = u32::from_le_bytes(payload.get(cursor + 4..cursor + 8)?.try_into().ok()?);
        if size != 4 {
            return None;
        }
        Some(f32::from_le_bytes(
            payload.get(cursor + 9..cursor + 13)?.try_into().ok()?,
        ))
    }

    fn minimal_gsav(name: &str) -> Vec<u8> {
        build_gsav(2, &public_payload(name), &minimal_stream(), &[0, 0, 0, 0])
    }

    fn persistent_slot_public_data(
        slot: &str,
        player_save_name: &str,
        chapter_id: i32,
        map_name: &str,
        time_played_seconds: f64,
        quick_save: bool,
        auto_save: bool,
    ) -> Vec<u8> {
        [
            fstring(slot),
            str_property("m_SlotName", slot),
            str_property("m_PlayerSaveName", player_save_name),
            bool_property("m_IsPlayerSaveNameCustom", true),
            fstring("m_CompressedBitmap"),
            fstring("ArrayProperty"),
            fstring("ByteProperty"),
            int_property("m_ChapterID", chapter_id),
            fstring("m_difficultyPreset"),
            fstring("ObjectProperty"),
            str_property("m_MapName", map_name),
            fstring("m_Date"),
            fstring("StructProperty"),
            fstring("DateTime"),
            fstring("/Script/CoreUObject"),
            double_property("m_TimePlayed", time_played_seconds),
            double_property("m_TimeLoaded", 0.0),
            bool_property("m_QuickSave", quick_save),
            bool_property("m_AutoSave", auto_save),
            int_property("m_ProfileId", 0),
            fstring("None"),
        ]
        .concat()
    }

    fn persistent_data_list(slots: &[(&str, &str, i32, &str, f64, bool, bool)]) -> Vec<u8> {
        let mut out = b"GVAS".to_vec();
        out.extend_from_slice(&fstring("/Script/Angelscript.GothicFinalList"));
        out.extend_from_slice(&fstring("m_SavedGamesPublicData"));
        out.extend_from_slice(&fstring("MapProperty"));
        out.extend_from_slice(&fstring("StrProperty"));
        out.extend_from_slice(&fstring("StructProperty"));
        out.extend_from_slice(&fstring("SaveGamePublicData"));
        out.extend_from_slice(&fstring("/Script/G1R"));
        for (slot, name, chapter, map, time_played, quick, auto) in slots {
            out.extend_from_slice(&persistent_slot_public_data(
                slot,
                name,
                *chapter,
                map,
                *time_played,
                *quick,
                *auto,
            ));
        }
        let saved_slots = slots
            .iter()
            .map(|(slot, _, _, _, _, _, _)| *slot)
            .collect::<Vec<_>>();
        out.extend_from_slice(&fstring("m_Profiles"));
        out.extend_from_slice(&fstring("ArrayProperty"));
        out.extend_from_slice(&fstring("StructProperty"));
        out.extend_from_slice(&fstring("ProfileData"));
        out.extend_from_slice(&fstring("/Script/G1R"));
        out.extend_from_slice(&str_property("m_ProfileName", "0"));
        out.extend_from_slice(&int_property("m_ProfileId", 0));
        out.extend_from_slice(&string_array_property(
            "m_QuickSaveName",
            &["G1R-001", "G1R-002", "G1R-003"],
        ));
        out.extend_from_slice(&string_array_property(
            "m_AutoSaveName",
            &["G1R-001", "G1R-002"],
        ));
        out.extend_from_slice(&string_array_property("m_SavedSlotsNames", &saved_slots));
        out.extend_from_slice(&fstring("m_difficultyPreset"));
        out.extend_from_slice(&fstring("ObjectProperty"));
        out.extend_from_slice(&fstring("/Game/G1R/Gameplay/Difficulty/Normal"));
        out.extend_from_slice(&fstring("m_customCombatSettings"));
        out.extend_from_slice(&fstring("ObjectProperty"));
        out.extend_from_slice(&fstring("/Game/G1R/Gameplay/Difficulty/CombatDefault"));
        out.extend_from_slice(&fstring("m_customResourcesSettings"));
        out.extend_from_slice(&fstring("ObjectProperty"));
        out.extend_from_slice(&fstring("/Game/G1R/Gameplay/Difficulty/ResourcesDefault"));
        out.extend_from_slice(&fstring("m_customProgressionSettings"));
        out.extend_from_slice(&fstring("ObjectProperty"));
        out.extend_from_slice(&fstring("/Game/G1R/Gameplay/Difficulty/ProgressionDefault"));
        out.extend_from_slice(&bool_property("m_Survival", false));
        out.extend_from_slice(&bool_property("m_PermanentDeath", false));
        out.extend_from_slice(&bool_property("m_PermanentDeathGameOver", false));
        out.extend_from_slice(&bool_property("m_FakeSloppyCombos", false));
        out.extend_from_slice(&int_property("m_MaxQuick", 3));
        out.extend_from_slice(&int_property("m_MaxAuto", 3));
        out.extend_from_slice(&fstring("None"));
        out.extend_from_slice(&fstring("SavedDataVersion"));
        out
    }

    fn screenshot_private_payload(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&fstring("m_Screenshots"));
        out.extend_from_slice(&fstring("MapProperty"));
        out.extend_from_slice(&fstring("StrProperty"));
        out.extend_from_slice(&fstring("ArrayProperty"));
        out.extend_from_slice(&fstring("ByteProperty"));
        for (slot, jpeg) in entries {
            out.extend_from_slice(&fstring(slot));
            out.extend_from_slice(jpeg);
        }
        out.extend_from_slice(&fstring("None"));
        out
    }

    fn screenshot_gsav_for_tests(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let payload = screenshot_private_payload(entries);
        build_gsav(
            2,
            &public_payload("Screenshots"),
            &compressed_stream_with_one_chunk(&payload, payload.len()),
            &[0, 0, 0, 0],
        )
    }

    fn raw_screenshot_gsav_for_tests(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let public_payload = public_payload("Screenshots");
        let private_payload = screenshot_private_payload(entries);
        let body_size = 13 + public_payload.len() + private_payload.len();
        let mut out = Vec::new();
        out.extend_from_slice(b"GSAV");
        out.push(2);
        out.extend_from_slice(&(body_size as u32).to_le_bytes());
        out.extend_from_slice(&(public_payload.len() as u32).to_le_bytes());
        out.extend_from_slice(&public_payload);
        out.extend_from_slice(&private_payload);
        out.extend_from_slice(&[0, 0, 0, 0]);
        out
    }

    fn compressed_stream_with_one_chunk(compressed: &[u8], uncompressed_size: usize) -> Vec<u8> {
        compressed_stream_with_chunks(&[(compressed.to_vec(), uncompressed_size as u64)])
    }

    fn compressed_stream_with_chunks(chunks: &[(Vec<u8>, u64)]) -> Vec<u8> {
        let summary_compressed_size = chunks
            .iter()
            .map(|(compressed, _)| compressed.len() as u64)
            .sum::<u64>();
        let summary_uncompressed_size = chunks
            .iter()
            .map(|(_, uncompressed_size)| *uncompressed_size)
            .sum::<u64>();
        let max_chunk_size = chunks
            .iter()
            .map(|(_, uncompressed_size)| *uncompressed_size)
            .max()
            .unwrap_or(131_072)
            .max(1);
        let mut out = Vec::new();
        out.extend_from_slice(&summary_uncompressed_size.to_le_bytes());
        out.extend_from_slice(&fstring("Oodle"));
        out.extend_from_slice(&PACKAGE_FILE_TAG.to_le_bytes());
        out.extend_from_slice(&COMPRESSED_HEADER_V2.to_le_bytes());
        out.extend_from_slice(&max_chunk_size.to_le_bytes());
        out.push(2);
        out.extend_from_slice(&summary_compressed_size.to_le_bytes());
        out.extend_from_slice(&summary_uncompressed_size.to_le_bytes());
        for (compressed, uncompressed_size) in chunks {
            out.extend_from_slice(&(compressed.len() as u64).to_le_bytes());
            out.extend_from_slice(&uncompressed_size.to_le_bytes());
        }
        for (compressed, _) in chunks {
            out.extend_from_slice(compressed);
        }
        out
    }

    struct PrefixCodecBackend {
        seed_compressed: Vec<u8>,
        seed_uncompressed: Vec<u8>,
    }

    impl codec_backend::CodecBackend for PrefixCodecBackend {
        fn probe(&self) -> Result<codec_backend::CodecBackendProbe, CoreError> {
            Ok(codec_backend::CodecBackendProbe {
                backend: "prefix_test_codec".to_string(),
                available: true,
                can_decompress: true,
                can_compress: true,
                status: "ready".to_string(),
                profile: None,
                resolution_mode: None,
                details: json!({}),
            })
        }

        fn decompress(&self, input: &[u8], _expected_size: usize) -> Result<Vec<u8>, CoreError> {
            if input == self.seed_compressed {
                return Ok(self.seed_uncompressed.clone());
            }
            input
                .strip_prefix(b"CMP:")
                .map(|payload| payload.to_vec())
                .ok_or_else(|| CoreError::Codec("unexpected test compressed payload".to_string()))
        }

        fn compress(&self, input: &[u8], _level: u8) -> Result<Vec<u8>, CoreError> {
            let mut out = b"CMP:".to_vec();
            out.extend_from_slice(input);
            Ok(out)
        }
    }

    struct BatchOnlyCodecBackend {
        chunks: Vec<(Vec<u8>, Vec<u8>)>,
        batch_calls: Mutex<Vec<Vec<Vec<u8>>>>,
    }

    impl BatchOnlyCodecBackend {
        fn new(chunks: Vec<(Vec<u8>, Vec<u8>)>) -> Self {
            Self {
                chunks,
                batch_calls: Mutex::new(Vec::new()),
            }
        }

        fn batch_calls(&self) -> Vec<Vec<Vec<u8>>> {
            self.batch_calls.lock().unwrap().clone()
        }
    }

    impl codec_backend::CodecBackend for BatchOnlyCodecBackend {
        fn probe(&self) -> Result<codec_backend::CodecBackendProbe, CoreError> {
            Ok(codec_backend::CodecBackendProbe {
                backend: "batch_only_test_codec".to_string(),
                available: true,
                can_decompress: true,
                can_compress: false,
                status: "ready".to_string(),
                profile: None,
                resolution_mode: None,
                details: json!({}),
            })
        }

        fn decompress(&self, _input: &[u8], _expected_size: usize) -> Result<Vec<u8>, CoreError> {
            Err(CoreError::Codec(
                "single-chunk decompress should not be used for private payload decode".to_string(),
            ))
        }

        fn decompress_many(
            &self,
            chunks: &[codec_backend::CodecDecodeChunk<'_>],
        ) -> Result<Vec<Vec<u8>>, CoreError> {
            self.batch_calls.lock().unwrap().push(
                chunks
                    .iter()
                    .map(|chunk| chunk.input.to_vec())
                    .collect::<Vec<_>>(),
            );
            chunks
                .iter()
                .map(|chunk| {
                    if let Some(payload) = chunk.input.strip_prefix(b"CMP:") {
                        return Ok(payload.to_vec());
                    }
                    self.chunks
                        .iter()
                        .find(|(compressed, _)| compressed == chunk.input)
                        .map(|(_, decompressed)| decompressed.clone())
                        .ok_or_else(|| {
                            CoreError::Codec("unexpected batch compressed payload".to_string())
                        })
                })
                .collect()
        }

        fn compress(&self, _input: &[u8], _level: u8) -> Result<Vec<u8>, CoreError> {
            Err(CoreError::Codec(
                "single-chunk compress should not be used for private payload rebuild".to_string(),
            ))
        }

        fn compress_many(
            &self,
            chunks: &[codec_backend::CodecEncodeChunk<'_>],
        ) -> Result<Vec<Vec<u8>>, CoreError> {
            Ok(chunks
                .iter()
                .map(|chunk| {
                    let mut out = b"CMP:".to_vec();
                    out.extend_from_slice(chunk.input);
                    out
                })
                .collect())
        }
    }

    #[test]
    fn parses_minimal_gsav() {
        let data = minimal_gsav("Slot A");
        let info = parse_gsav(&data, None).unwrap();
        assert_eq!(info.format, "GSAV");
        assert_eq!(info.public.player_save_name.as_deref(), Some("Slot A"));
        assert_eq!(info.compressed_stream.method, "Oodle");
        assert_eq!(info.compressed_stream.chunk_count, 0);
    }

    #[test]
    fn roundtrip_preserves_minimal_gsav() {
        let data = minimal_gsav("Slot A");
        let rebuilt = rebuild_gsav_preserving_stream(&data).unwrap();
        assert_eq!(rebuilt, data);
    }

    #[test]
    fn rejects_corrupt_gsav() {
        let err = parse_gsav(b"GSAV\x02\x00", None).unwrap_err();
        assert!(err.to_string().contains("shorter"));
    }

    #[test]
    fn scan_save_dir_enriches_slots_from_persistent_data_list() {
        let dir = tempdir().unwrap();
        let save_path = dir.path().join("G1R-001.sav");
        fs::write(&save_path, minimal_gsav("Public name")).unwrap();
        fs::write(
            dir.path().join("PersistentDataList.sav"),
            persistent_data_list(&[(
                "G1R-001",
                "Persistent name",
                2,
                "OldCamp",
                3661.5,
                true,
                false,
            )]),
        )
        .unwrap();

        let saves = scan_save_dir(dir.path()).unwrap();
        let slot = saves.iter().find(|save| save.slot == "G1R-001").unwrap();
        let value = serde_json::to_value(slot).unwrap();

        assert_eq!(value["playerSaveName"], "Public name");
        assert_eq!(value["persistentPlayerSaveName"], "Persistent name");
        assert_eq!(value["chapterId"], 2);
        assert_eq!(value["mapName"], "OldCamp");
        assert_eq!(value["timePlayedSeconds"], 3661.5);
        assert_eq!(value["quickSave"], true);
        assert_eq!(value["autoSave"], false);
    }

    #[test]
    fn scan_save_dir_reports_profile_summaries_from_persistent_data_list() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("G1R-001.sav"), minimal_gsav("Auto")).unwrap();
        fs::write(dir.path().join("G1R-002.sav"), minimal_gsav("Quick")).unwrap();
        fs::write(
            dir.path().join("PersistentDataList.sav"),
            persistent_data_list(&[
                ("G1R-001", "Auto", 1, "MainMap", 60.0, false, true),
                ("G1R-002", "Quick", 1, "MainMap", 120.0, true, false),
            ]),
        )
        .unwrap();

        let value = execute_json_inner(
            &json!({
                "command": "scan_save_dir",
                "payload": { "path": dir.path() }
            })
            .to_string(),
        )
        .unwrap();

        assert_eq!(value["activeProfileId"], 0);
        assert_eq!(value["profiles"][0]["profileId"], 0);
        assert_eq!(value["profiles"][0]["profileName"], "0");
        assert_eq!(
            value["profiles"][0]["quickSaveSlots"],
            json!(["G1R-001", "G1R-002", "G1R-003"])
        );
        assert_eq!(
            value["profiles"][0]["autoSaveSlots"],
            json!(["G1R-001", "G1R-002"])
        );
        assert_eq!(
            value["profiles"][0]["savedSlots"],
            json!(["G1R-001", "G1R-002"])
        );
    }

    #[test]
    fn scan_save_dir_filters_non_slot_sav_files() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("G1R-001.sav"), minimal_gsav("Auto")).unwrap();
        fs::write(
            dir.path().join("EnhancedInputUserSettings.sav"),
            minimal_gsav("Settings"),
        )
        .unwrap();

        let saves = scan_save_dir(dir.path()).unwrap();

        assert_eq!(saves.len(), 1);
        assert_eq!(saves[0].slot, "G1R-001");
    }

    #[test]
    fn parse_screenshot_payload_extracts_jpeg_by_slot() {
        let payload =
            screenshot_private_payload(&[("G1R-001", &[0xff, 0xd8, 0x01, 0x02, 0xff, 0xd9])]);

        let screenshots = parse_screenshot_payload(&payload);

        assert_eq!(screenshots["G1R-001"].mime_type, "image/jpeg");
        assert_eq!(screenshots["G1R-001"].byte_length, 6);
        assert_eq!(screenshots["G1R-001"].bytes_base64, "/9gBAv/Z");
    }

    #[test]
    fn scan_save_dir_attaches_screenshot_to_matching_slot() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("G1R-001.sav"), minimal_gsav("Auto")).unwrap();
        fs::write(
            dir.path().join("PersistentDataList.sav"),
            persistent_data_list(&[("G1R-001", "Auto", 1, "MainMap", 60.0, false, true)]),
        )
        .unwrap();
        fs::write(
            dir.path().join("Profile_0_Screenshots.sav"),
            screenshot_gsav_for_tests(&[("G1R-001", &[0xff, 0xd8, 0x01, 0x02, 0xff, 0xd9])]),
        )
        .unwrap();

        let value = execute_json_inner(
            &json!({
                "command": "scan_save_dir",
                "payload": { "path": dir.path() }
            })
            .to_string(),
        )
        .unwrap();
        let saves = value["saves"].as_array().unwrap();
        let save = saves.iter().find(|save| save["slot"] == "G1R-001").unwrap();

        assert_eq!(save["screenshot"]["mimeType"], "image/jpeg");
        assert_eq!(save["screenshot"]["byteLength"], 6);
        assert_eq!(save["screenshot"]["bytesBase64"], "/9gBAv/Z");
        assert!(
            !saves
                .iter()
                .any(|save| save["slot"] == "Profile_0_Screenshots")
        );
    }

    #[test]
    fn scan_save_dir_attaches_raw_gsav_screenshot_sidecar() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("G1R-001.sav"), minimal_gsav("Auto")).unwrap();
        fs::write(
            dir.path().join("PersistentDataList.sav"),
            persistent_data_list(&[("G1R-001", "Auto", 1, "MainMap", 60.0, false, true)]),
        )
        .unwrap();
        fs::write(
            dir.path().join("Profile_0_Screenshots.sav"),
            raw_screenshot_gsav_for_tests(&[("G1R-001", &[0xff, 0xd8, 0xaa, 0xbb, 0xff, 0xd9])]),
        )
        .unwrap();

        let value = execute_json_inner(
            &json!({
                "command": "scan_save_dir",
                "payload": { "path": dir.path() }
            })
            .to_string(),
        )
        .unwrap();
        let saves = value["saves"].as_array().unwrap();
        let save = saves.iter().find(|save| save["slot"] == "G1R-001").unwrap();

        assert_eq!(save["screenshot"]["mimeType"], "image/jpeg");
        assert_eq!(save["screenshot"]["byteLength"], 6);
        assert_eq!(save["screenshot"]["bytesBase64"], "/9iqu//Z");
    }

    #[test]
    fn inspect_save_includes_matching_persistent_slot_metadata() {
        let dir = tempdir().unwrap();
        let save_path = dir.path().join("G1R-001.sav");
        fs::write(&save_path, minimal_gsav("Public name")).unwrap();
        fs::write(
            dir.path().join("PersistentDataList.sav"),
            persistent_data_list(&[(
                "G1R-001",
                "Persistent name",
                1,
                "MainMap",
                6963.25,
                false,
                true,
            )]),
        )
        .unwrap();

        let info = inspect_save(&save_path, false).unwrap();

        assert_eq!(info["persistent"]["playerSaveName"], "Persistent name");
        assert_eq!(info["persistent"]["chapterId"], 1);
        assert_eq!(info["persistent"]["mapName"], "MainMap");
        assert_eq!(info["persistent"]["timePlayedSeconds"], 6963.25);
        assert_eq!(info["persistent"]["quickSave"], false);
        assert_eq!(info["persistent"]["autoSave"], true);
    }

    #[test]
    fn write_save_applies_same_length_public_name_with_backup() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("G1R-001.sav");
        fs::write(&path, minimal_gsav("Slot A")).unwrap();
        let response = write_save(
            &path,
            &[json!({"path": "public.m_PlayerSaveName", "value": "Slot B"})],
            true,
            None,
        )
        .unwrap();
        assert_eq!(response["editsApplied"], 1);
        assert!(response["backupPath"].as_str().unwrap().contains(".bak."));
        let info = inspect_save(&path, false).unwrap();
        assert_eq!(info["public"]["playerSaveName"], "Slot B");
    }

    #[test]
    fn write_save_applies_length_changing_public_name_and_preserves_stream() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("G1R-001.sav");
        let original = minimal_gsav("Slot A");
        let original_parts = split_gsav(&original).unwrap();
        fs::write(&path, &original).unwrap();

        let response = write_save(
            &path,
            &[json!({"path": "public.m_PlayerSaveName", "value": "Much Longer"})],
            true,
            None,
        )
        .unwrap();

        assert_eq!(response["editsApplied"], 1);
        assert!(response["bytesChanged"].as_bool().unwrap());
        let written = fs::read(&path).unwrap();
        let written_parts = split_gsav(&written).unwrap();
        assert_eq!(
            written_parts.compressed_stream,
            original_parts.compressed_stream
        );
        let info = inspect_save(&path, false).unwrap();
        assert_eq!(info["public"]["playerSaveName"], "Much Longer");
        assert_eq!(
            info["publicPayloadSize"].as_u64().unwrap(),
            public_payload("Much Longer").len() as u64
        );
    }

    #[test]
    fn write_save_syncs_player_save_name_to_persistent_data_list() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("G1R-001.sav");
        let persistent_path = dir.path().join("PersistentDataList.sav");
        fs::write(&path, minimal_gsav("Public name")).unwrap();
        fs::write(
            &persistent_path,
            persistent_data_list(&[
                (
                    "G1R-001",
                    "Persistent old",
                    1,
                    "MainMap",
                    3600.0,
                    false,
                    true,
                ),
                ("G1R-002", "Other slot", 2, "OldCamp", 7200.0, true, false),
            ]),
        )
        .unwrap();

        let response = execute_json(
            &json!({
                "command": "write_save",
                "payload": {
                    "path": path,
                    "backup": true,
                    "syncPersistentDataList": true,
                    "edits": [
                        {"path": "public.m_PlayerSaveName", "value": "Synced Slot Name"}
                    ]
                }
            })
            .to_string(),
        );
        let value: Value = serde_json::from_str(&response).unwrap();

        assert_eq!(value["ok"], true);
        assert!(
            value["data"]["persistentBackupPath"]
                .as_str()
                .unwrap()
                .contains("PersistentDataList.sav.bak.")
        );
        assert_eq!(value["data"]["persistentBytesChanged"], true);

        // Slot and companion backups must share one suffix so restore can pair
        // them (see prepare_paired_persistent_data_list_restore).
        let slot_backup = value["data"]["backupPath"].as_str().unwrap();
        let companion_backup = value["data"]["persistentBackupPath"].as_str().unwrap();
        let slot_suffix = slot_backup.rsplit("G1R-001.sav.bak.").next().unwrap();
        let companion_suffix = companion_backup
            .rsplit("PersistentDataList.sav.bak.")
            .next()
            .unwrap();
        assert_eq!(slot_suffix, companion_suffix);

        let metadata = persistent_slot_metadata_for_dir(dir.path()).unwrap();
        assert_eq!(
            metadata["G1R-001"].player_save_name.as_deref(),
            Some("Synced Slot Name")
        );
        assert_eq!(
            metadata["G1R-002"].player_save_name.as_deref(),
            Some("Other slot")
        );
    }

    #[test]
    fn write_save_sync_errors_when_persistent_list_missing_slot_entry() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("G1R-001.sav");
        let persistent_path = dir.path().join("PersistentDataList.sav");
        fs::write(&path, minimal_gsav("Public name")).unwrap();
        // Companion list exists but has no entry for G1R-001.
        fs::write(
            &persistent_path,
            persistent_data_list(&[("G1R-002", "Other slot", 2, "OldCamp", 7200.0, true, false)]),
        )
        .unwrap();

        let response = execute_json(
            &json!({
                "command": "write_save",
                "payload": {
                    "path": path,
                    "backup": true,
                    "syncPersistentDataList": true,
                    "edits": [
                        {"path": "public.m_PlayerSaveName", "value": "Synced Slot Name"}
                    ]
                }
            })
            .to_string(),
        );
        let value: Value = serde_json::from_str(&response).unwrap();

        // Sync was requested but the slot is absent: surface an error instead of
        // silently leaving the slot file and companion list out of sync.
        assert_eq!(value["ok"], false);
        // Slot file must be untouched.
        assert_eq!(
            inspect_save(&path, false).unwrap()["public"]["playerSaveName"],
            "Public name"
        );
    }

    #[test]
    fn list_backups_returns_matching_save_backups_newest_first() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("G1R-001.sav");
        let subfolder = dir.path().join("goresave_backups");
        fs::create_dir_all(&subfolder).unwrap();
        let older = subfolder.join("G1R-001.sav.bak.100");
        let newer = subfolder.join("G1R-001.sav.bak.200");
        // Unrelated file in subfolder must not appear.
        let unrelated = subfolder.join("G1R-002.sav.bak.300");
        fs::write(&path, minimal_gsav("Live")).unwrap();
        fs::write(&older, minimal_gsav("Older")).unwrap();
        fs::write(&newer, minimal_gsav("Newer")).unwrap();
        fs::write(&unrelated, minimal_gsav("Other")).unwrap();

        let response = execute_json(
            &json!({
                "command": "list_backups",
                "payload": {"path": path}
            })
            .to_string(),
        );
        let value: Value = serde_json::from_str(&response).unwrap();

        assert_eq!(value["ok"], true);
        let backups = value["data"]["backups"].as_array().unwrap();
        assert_eq!(backups.len(), 2);
        assert_eq!(
            backups[0]["path"].as_str().unwrap(),
            newer.to_string_lossy()
        );
        assert_eq!(backups[0]["createdEpoch"], 200);
        assert_eq!(backups[0]["playerSaveName"], "Newer");
        assert_eq!(backups[0]["status"], "ok");
        assert_eq!(
            backups[1]["path"].as_str().unwrap(),
            older.to_string_lossy()
        );
        assert_eq!(backups[1]["createdEpoch"], 100);
    }

    #[test]
    fn list_backups_includes_legacy_backups_next_to_save_alongside_subfolder_ones() {
        // Legacy backups (placed directly in the save's parent dir) must still
        // appear in list_save_backups alongside new subfolder backups.
        let dir = tempdir().unwrap();
        let path = dir.path().join("G1R-001.sav");
        let subfolder = dir.path().join("goresave_backups");
        fs::create_dir_all(&subfolder).unwrap();
        // Legacy file next to the save.
        let legacy = dir.path().join("G1R-001.sav.bak.50");
        // New file in the subfolder.
        let new_backup = subfolder.join("G1R-001.sav.bak.150");
        fs::write(&path, minimal_gsav("Live")).unwrap();
        fs::write(&legacy, minimal_gsav("Legacy")).unwrap();
        fs::write(&new_backup, minimal_gsav("New")).unwrap();

        let backups = list_save_backups(&path).unwrap();
        assert_eq!(
            backups.len(),
            2,
            "both legacy and subfolder backups expected"
        );
        // Newest-first: epoch 150 before epoch 50.
        assert!(backups[0].path.contains("150"), "subfolder backup first");
        assert!(backups[1].path.contains("50"), "legacy backup second");
    }

    #[test]
    fn create_backup_copy_writes_two_consecutive_backups_to_subfolder() {
        // Two consecutive backup writes must produce two distinct files, both
        // located in the goresave_backups subfolder.
        let dir = tempdir().unwrap();
        let path = dir.path().join("G1R-001.sav");
        fs::write(&path, minimal_gsav("Slot A")).unwrap();

        // First backup — also triggers subfolder creation.
        let b1 = create_backup_copy(&path).unwrap();
        // Mutate the file so the second backup is distinct.
        fs::write(&path, minimal_gsav("Slot B")).unwrap();
        let b2 = create_backup_copy(&path).unwrap();

        assert_ne!(b1, b2, "two backups must have distinct paths");
        assert!(b1.exists(), "first backup file must exist");
        assert!(b2.exists(), "second backup file must exist");

        let subfolder = dir.path().join("goresave_backups");
        assert!(
            b1.starts_with(&subfolder),
            "first backup must be in goresave_backups subfolder"
        );
        assert!(
            b2.starts_with(&subfolder),
            "second backup must be in goresave_backups subfolder"
        );
    }

    #[test]
    fn list_backups_returns_persistent_data_list_companion_backups_for_selected_slot() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("G1R-001.sav");
        let subfolder = dir.path().join("goresave_backups");
        fs::create_dir_all(&subfolder).unwrap();
        let companion = subfolder.join("PersistentDataList.sav.bak.250");
        // Unrelated file in subfolder must not appear.
        let unrelated = subfolder.join("G1R-002.sav.bak.300");
        fs::write(&path, minimal_gsav("Live")).unwrap();
        fs::write(
            &companion,
            persistent_data_list(&[
                (
                    "G1R-001",
                    "Companion before edit",
                    1,
                    "MainMap",
                    42.0,
                    false,
                    false,
                ),
                ("G1R-002", "Other slot", 2, "OldCamp", 84.0, false, false),
            ]),
        )
        .unwrap();
        fs::write(&unrelated, minimal_gsav("Other")).unwrap();

        let response = execute_json(
            &json!({
                "command": "list_backups",
                "payload": {"path": path}
            })
            .to_string(),
        );
        let value: Value = serde_json::from_str(&response).unwrap();

        assert_eq!(value["ok"], true);
        let companion_backups = value["data"]["companionBackups"].as_array().unwrap();
        assert_eq!(companion_backups.len(), 1);
        assert_eq!(
            companion_backups[0]["path"].as_str().unwrap(),
            companion.to_string_lossy()
        );
        assert_eq!(
            companion_backups[0]["fileName"],
            "PersistentDataList.sav.bak.250"
        );
        assert_eq!(companion_backups[0]["createdEpoch"], 250);
        // The slot metadata is surfaced for display from the string scan. The
        // status reflects STRICT profile validation: this fixture is a flat
        // string-scan soup (not a byte-accurate typed GVAS), so the strict parse
        // flags it — a real, byte-accurate profile backup reports "ok" and is
        // restorable (covered by restore_backup_restores_..._directly).
        assert_eq!(
            companion_backups[0]["status"],
            "invalid PersistentDataList structure"
        );
        assert_eq!(companion_backups[0]["slotName"], "G1R-001");
        assert_eq!(
            companion_backups[0]["playerSaveName"],
            "Companion before edit"
        );
    }

    #[test]
    fn restore_backup_validates_backup_and_preserves_current_save() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("G1R-001.sav");
        let subfolder = dir.path().join("goresave_backups");
        fs::create_dir_all(&subfolder).unwrap();
        let backup = subfolder.join("G1R-001.sav.bak.200");
        fs::write(&path, minimal_gsav("Live")).unwrap();
        fs::write(&backup, minimal_gsav("Backup")).unwrap();

        let response = execute_json(
            &json!({
                "command": "restore_backup",
                "payload": {"path": path, "backupPath": backup}
            })
            .to_string(),
        );
        let value: Value = serde_json::from_str(&response).unwrap();

        assert_eq!(value["ok"], true);
        assert_eq!(
            value["data"]["restoredFrom"].as_str().unwrap(),
            backup.to_string_lossy()
        );
        let current_backup = PathBuf::from(value["data"]["backupPath"].as_str().unwrap());
        assert!(current_backup.exists());
        // The safety backup of "Live" must also be in the subfolder.
        assert!(
            current_backup.starts_with(&subfolder),
            "safety backup must be written to goresave_backups subfolder"
        );
        assert_eq!(
            inspect_save(&path, false).unwrap()["public"]["playerSaveName"],
            "Backup"
        );
        assert_eq!(
            inspect_save(&current_backup, false).unwrap()["public"]["playerSaveName"],
            "Live"
        );
    }

    #[test]
    fn restore_backup_restores_persistent_data_list_profile_backup_directly() {
        // A profile difficulty write backs up only PersistentDataList.sav.
        // Restoring that companion backup must target the PDL itself and must
        // NOT self-pair (treating the file as its own companion would replace
        // it twice). The file rolls back to the backup content.
        let dir = tempdir().unwrap();
        let pdl = dir.path().join("PersistentDataList.sav");
        let subfolder = dir.path().join("goresave_backups");
        fs::create_dir_all(&subfolder).unwrap();
        let backup = subfolder.join("PersistentDataList.sav.bak.200");

        let backup_data = difficulty_persistent_profiles(); // profile 1 = Easy
        let mut live = backup_data.clone();
        write_profile_difficulty(
            &mut live,
            1,
            &DifficultyRequest {
                preset: Some("Hard".into()),
                combat: None,
                resources: None,
                progression: None,
                flow_helper: None,
                permadeath: None,
            },
        )
        .unwrap();
        fs::write(&pdl, &live).unwrap();
        fs::write(&backup, &backup_data).unwrap();
        assert_ne!(live, backup_data, "fixture precondition: live differs");

        let response = execute_json(
            &json!({
                "command": "restore_backup",
                "payload": {"path": pdl, "backupPath": backup}
            })
            .to_string(),
        );
        let value: Value = serde_json::from_str(&response).unwrap();
        assert_eq!(value["ok"], true, "restore failed: {value:?}");

        // The PDL rolled back to the backup: profile 1 is Easy again, no Hard.
        let restored = fs::read(&pdl).unwrap();
        assert_eq!(restored, backup_data);
        let presets: Vec<_> = scan_fstrings(&restored, 0)
            .into_iter()
            .filter(|r| r.value.contains("DifficultyPreset_"))
            .map(|r| r.value)
            .collect();
        assert!(presets.iter().any(|p| p.ends_with("DifficultyPreset_Easy")));
        assert!(!presets.iter().any(|p| p.ends_with("DifficultyPreset_Hard")));
    }

    #[test]
    fn restore_backup_rejects_invalid_persistent_data_list_backup() {
        // A GVAS backup that passes the weak magic/string inspection but is not
        // a valid profile (no m_Profiles) must NOT overwrite the live profile.
        let dir = tempdir().unwrap();
        let pdl = dir.path().join("PersistentDataList.sav");
        let subfolder = dir.path().join("goresave_backups");
        fs::create_dir_all(&subfolder).unwrap();
        let backup = subfolder.join("PersistentDataList.sav.bak.200");

        let live = difficulty_persistent_profiles();
        fs::write(&pdl, &live).unwrap();
        let mut bad = b"GVAS".to_vec();
        bad.extend_from_slice(&[0u8; 24]);
        bad.extend_from_slice(&fstring("None")); // GVAS magic but no m_Profiles
        fs::write(&backup, &bad).unwrap();

        assert!(restore_backup(&pdl, &backup).is_err());
        // The live profile is untouched.
        assert_eq!(fs::read(&pdl).unwrap(), live);
    }

    #[test]
    fn restore_backup_also_restores_paired_persistent_data_list_backup() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("G1R-001.sav");
        let subfolder = dir.path().join("goresave_backups");
        fs::create_dir_all(&subfolder).unwrap();
        let slot_backup = subfolder.join("G1R-001.sav.bak.200");
        let persistent = dir.path().join("PersistentDataList.sav");
        let persistent_backup = subfolder.join("PersistentDataList.sav.bak.200");

        fs::write(&path, minimal_gsav("Live")).unwrap();
        fs::write(&slot_backup, minimal_gsav("Backup")).unwrap();
        // Current PersistentDataList carries the newer name; its paired backup
        // (same epoch as the slot backup) carries the older name.
        fs::write(&persistent, b"GVAS-new-name").unwrap();
        fs::write(&persistent_backup, b"GVAS-old-name").unwrap();

        let response = execute_json(
            &json!({
                "command": "restore_backup",
                "payload": {"path": path, "backupPath": slot_backup}
            })
            .to_string(),
        );
        let value: Value = serde_json::from_str(&response).unwrap();

        assert_eq!(value["ok"], true);
        assert_eq!(value["data"]["persistentBytesChanged"], true);
        assert_eq!(
            value["data"]["persistentRestoredFrom"].as_str().unwrap(),
            persistent_backup.to_string_lossy()
        );
        // PersistentDataList rolled back to the paired backup contents.
        assert_eq!(fs::read(&persistent).unwrap(), b"GVAS-old-name");
        // A safety backup of the pre-restore PersistentDataList was created.
        let safety = PathBuf::from(value["data"]["persistentBackupPath"].as_str().unwrap());
        assert_eq!(fs::read(&safety).unwrap(), b"GVAS-new-name");
    }

    #[test]
    fn fstring_rejects_i32_min_utf16_length() {
        // i32::MIN (0x80000000) as a negative UTF-16 length must error, not
        // panic on `-n` overflow.
        let data = i32::MIN.to_le_bytes();
        let mut r = Reader::new(&data, 0);
        assert!(r.fstring().is_err());
    }

    #[test]
    fn parse_compressed_stream_rejects_chunk_count_beyond_file() {
        // Corrupt header: huge summary size + max chunk size 1 => ~4B chunks,
        // but no chunk table follows. Must return a parse error instead of
        // attempting a multi-gigabyte allocation.
        let huge = u32::MAX as u64;
        let mut data = Vec::new();
        data.extend_from_slice(&huge.to_le_bytes()); // uncompressed_size_prefix
        data.extend_from_slice(&fstring("Oodle")); // method
        data.extend_from_slice(&PACKAGE_FILE_TAG.to_le_bytes()); // tag
        data.extend_from_slice(&0u32.to_le_bytes()); // header_version 0
        data.extend_from_slice(&1u32.to_le_bytes()); // max_chunk_size
        data.extend_from_slice(&0u32.to_le_bytes()); // summary_compressed_size
        data.extend_from_slice(&(u32::MAX).to_le_bytes()); // summary_uncompressed_size

        let err = parse_compressed_stream(&data, 0).unwrap_err();
        assert!(matches!(err, CoreError::Parse(_)));
    }

    #[test]
    fn restore_backup_pairs_companion_by_full_suffix_within_same_second() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("G1R-001.sav");
        let subfolder = dir.path().join("goresave_backups");
        fs::create_dir_all(&subfolder).unwrap();
        // Two paired backups created in the same second: ".200" and ".200.1".
        let slot_backup_first = subfolder.join("G1R-001.sav.bak.200");
        let slot_backup_second = subfolder.join("G1R-001.sav.bak.200.1");
        let persistent = dir.path().join("PersistentDataList.sav");
        let persistent_first = subfolder.join("PersistentDataList.sav.bak.200");
        let persistent_second = subfolder.join("PersistentDataList.sav.bak.200.1");

        fs::write(&path, minimal_gsav("Live")).unwrap();
        fs::write(&slot_backup_first, minimal_gsav("First")).unwrap();
        fs::write(&slot_backup_second, minimal_gsav("Second")).unwrap();
        fs::write(&persistent, b"GVAS-current").unwrap();
        fs::write(&persistent_first, b"GVAS-first").unwrap();
        fs::write(&persistent_second, b"GVAS-second").unwrap();

        // Restoring the ".200.1" slot backup must roll back the ".200.1"
        // companion, not the first ".200" entry encountered.
        let response = execute_json(
            &json!({
                "command": "restore_backup",
                "payload": {"path": path, "backupPath": slot_backup_second}
            })
            .to_string(),
        );
        let value: Value = serde_json::from_str(&response).unwrap();

        assert_eq!(value["ok"], true);
        assert_eq!(
            value["data"]["persistentRestoredFrom"].as_str().unwrap(),
            persistent_second.to_string_lossy()
        );
        assert_eq!(fs::read(&persistent).unwrap(), b"GVAS-second");
    }

    #[test]
    fn restore_backup_flags_present_companion_left_unrestored() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("G1R-001.sav");
        let subfolder = dir.path().join("goresave_backups");
        fs::create_dir_all(&subfolder).unwrap();
        let slot_backup = subfolder.join("G1R-001.sav.bak.200");
        let persistent = dir.path().join("PersistentDataList.sav");
        // PersistentDataList exists but there is no .bak.200 companion for it.
        fs::write(&path, minimal_gsav("Live")).unwrap();
        fs::write(&slot_backup, minimal_gsav("Backup")).unwrap();
        fs::write(&persistent, b"GVAS-current").unwrap();

        let response = execute_json(
            &json!({
                "command": "restore_backup",
                "payload": {"path": path, "backupPath": slot_backup}
            })
            .to_string(),
        );
        let value: Value = serde_json::from_str(&response).unwrap();

        assert_eq!(value["ok"], true);
        assert_eq!(value["data"]["persistentCompanionPresent"], true);
        assert!(value["data"]["persistentRestoredFrom"].is_null());
        // Companion left untouched.
        assert_eq!(fs::read(&persistent).unwrap(), b"GVAS-current");
    }

    #[test]
    fn restore_backup_rejects_mismatched_container_format() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("G1R-001.sav");
        let subfolder = dir.path().join("goresave_backups");
        fs::create_dir_all(&subfolder).unwrap();
        let backup = subfolder.join("G1R-001.sav.bak.200");
        fs::write(&path, minimal_gsav("Live")).unwrap();
        // A GVAS sidecar misnamed as a slot backup must not replace the GSAV slot.
        fs::write(
            &backup,
            persistent_data_list(&[("G1R-001", "X", 1, "M", 1.0, false, false)]),
        )
        .unwrap();

        let result = restore_backup(&path, &backup);
        assert!(result.is_err());
        assert_eq!(
            inspect_save(&path, false).unwrap()["public"]["playerSaveName"],
            "Live"
        );
    }

    #[test]
    fn restore_backup_aborts_without_touching_slot_when_companion_invalid() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("G1R-001.sav");
        let subfolder = dir.path().join("goresave_backups");
        fs::create_dir_all(&subfolder).unwrap();
        let slot_backup = subfolder.join("G1R-001.sav.bak.200");
        let persistent = dir.path().join("PersistentDataList.sav");
        let persistent_backup = subfolder.join("PersistentDataList.sav.bak.200");

        fs::write(&path, minimal_gsav("Live")).unwrap();
        fs::write(&slot_backup, minimal_gsav("Backup")).unwrap();
        fs::write(&persistent, b"GVAS-new-name").unwrap();
        // Paired companion backup (same epoch) is not a valid GVAS file.
        fs::write(&persistent_backup, b"not-gvas").unwrap();

        let result = restore_backup(&path, &slot_backup);
        assert!(result.is_err());
        // Slot file must be untouched because the companion failed validation
        // before any slot mutation.
        assert_eq!(
            inspect_save(&path, false).unwrap()["public"]["playerSaveName"],
            "Live"
        );
        assert_eq!(fs::read(&persistent).unwrap(), b"GVAS-new-name");
    }

    #[test]
    fn map_locked_file_error_produces_game_running_message_for_sharing_violation() {
        // ERROR_SHARING_VIOLATION = 32 (Windows). The helper must produce a
        // message that contains "is the game running" so the user knows the
        // cause without a raw OS error code.
        let io_err = std::io::Error::from_raw_os_error(32);
        let core_err = map_locked_file_error(io_err, "G1R-001.sav");
        let msg = core_err.to_string();
        assert!(
            msg.contains("is the game running"),
            "message should mention game running, got: {msg}"
        );

        // ERROR_LOCK_VIOLATION = 33 must also trigger the friendly message.
        let io_err_33 = std::io::Error::from_raw_os_error(33);
        let core_err_33 = map_locked_file_error(io_err_33, "G1R-001.sav");
        assert!(core_err_33.to_string().contains("is the game running"));

        // An unrelated OS error (e.g. ENOENT = 2) must NOT produce the game-
        // running message; it must propagate the original description.
        let io_unrelated = std::io::Error::from_raw_os_error(2);
        let core_unrelated = map_locked_file_error(io_unrelated, "G1R-001.sav");
        assert!(!core_unrelated.to_string().contains("is the game running"));
    }

    #[test]
    fn default_save_root_derives_from_environment() {
        let suffix = PathBuf::from("G1R").join("Saved").join("SaveGames");

        // Prefers LOCALAPPDATA.
        let from_local =
            default_save_root_from(Some(r"D:\LocalAppData".into()), Some(r"D:\Profile".into()));
        assert_eq!(from_local, PathBuf::from(r"D:\LocalAppData").join(&suffix));

        // Falls back to USERPROFILE\AppData\Local when LOCALAPPDATA is unset/empty.
        let from_profile = default_save_root_from(Some("".into()), Some(r"D:\Profile".into()));
        assert_eq!(
            from_profile,
            PathBuf::from(r"D:\Profile")
                .join("AppData")
                .join("Local")
                .join(&suffix)
        );

        // Neutral relative path when neither variable is available, never a
        // hardcoded developer profile.
        let neutral = default_save_root_from(None, None);
        assert_eq!(neutral, suffix);
    }

    #[test]
    fn write_save_rebuilds_private_stream_with_codec_backend() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("G1R-001.sav");
        let output_path = dir.path().join("G1R-001-edited.sav");
        let private_payload = fstring("Hero");
        let seed_compressed = b"seed-compressed".to_vec();
        let stream = compressed_stream_with_one_chunk(&seed_compressed, private_payload.len());
        fs::write(
            &path,
            build_gsav(2, &public_payload("Slot A"), &stream, &[0, 0, 0, 0]),
        )
        .unwrap();
        let backend = PrefixCodecBackend {
            seed_compressed,
            seed_uncompressed: private_payload,
        };

        let response = write_save_with_codec_backend(
            &path,
            &[json!({
                "path": "private.replaceFString",
                "value": { "oldValue": "Hero", "newValue": "Mage" }
            })],
            false,
            Some(&output_path),
            Some(&backend),
        )
        .unwrap();

        assert_eq!(response["editsApplied"], 1);
        let written = fs::read(&output_path).unwrap();
        let parts = split_gsav(&written).unwrap();
        let stream = parse_compressed_stream(&written, 13 + parts.public_payload.len()).unwrap();
        let decoded = decompress_private_payload(&written, &stream, &backend).unwrap();
        let private_strings = scan_fstrings(&decoded, 0)
            .into_iter()
            .map(|r| r.value)
            .collect::<Vec<_>>();
        assert_eq!(private_strings, vec!["Mage"]);
        assert_eq!(stream.summary_uncompressed_size, decoded.len() as u64);
        assert_eq!(stream.summary_compressed_size, (decoded.len() + 4) as u64);
    }

    #[test]
    fn private_name_edit_updates_str_property_size_word_on_length_change() {
        // m_PlayerName "Hero" -> "Nameless" changes the value length, so the
        // StrProperty's 4-byte size word must be rewritten to match. (inspect
        // scans FStrings by length prefix and would miss a stale size word, so
        // assert the raw size bytes directly.)
        let mut payload = [
            fstring("Hero"),
            str_property("m_PlayerName", "Hero"),
            fstring("None"),
        ]
        .concat();

        apply_private_player_name_edit_to_payload(
            &mut payload,
            &PrivatePlayerNameEdit {
                name: "Nameless".to_string(),
            },
        )
        .unwrap();

        let refs = scan_fstrings(&payload, 0);
        let name_idx = refs.iter().position(|r| r.value == "m_PlayerName").unwrap();
        let type_ref = &refs[name_idx + 1];
        assert_eq!(type_ref.value, "StrProperty");
        let value_ref = &refs[name_idx + 2];
        assert_eq!(value_ref.value, "Nameless");

        // Size word lives 4 bytes after the type FString and must equal the new
        // encoded value length (4-byte len prefix + bytes + NUL).
        let size_offset = type_ref.len_offset + type_ref.total_len + 4;
        let size = u32::from_le_bytes(payload[size_offset..size_offset + 4].try_into().unwrap());
        assert_eq!(size as usize, encode_fstring("Nameless").len());
        // Trailing "None" terminator survived the splice.
        assert!(refs.iter().any(|r| r.value == "None"));
    }

    #[test]
    fn write_save_updates_private_player_name_property_only() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("G1R-001.sav");
        let output_path = dir.path().join("G1R-001-edited.sav");
        let private_payload = [
            fstring("Hero"),
            str_property("m_PlayerName", "Hero"),
            str_property("m_CurrentWorld", "WORLD"),
            fstring("None"),
        ]
        .concat();
        let seed_compressed = b"seed-compressed".to_vec();
        let stream = compressed_stream_with_one_chunk(&seed_compressed, private_payload.len());
        fs::write(
            &path,
            build_gsav(2, &public_payload("Slot A"), &stream, &[0, 0, 0, 0]),
        )
        .unwrap();
        let backend = PrefixCodecBackend {
            seed_compressed,
            seed_uncompressed: private_payload,
        };

        let response = write_save_with_codec_backend(
            &path,
            &[json!({
                "path": "private.player.setPlayerName",
                "value": "Nameless"
            })],
            false,
            Some(&output_path),
            Some(&backend),
        )
        .unwrap();

        assert_eq!(response["editsApplied"], 1);
        let value =
            inspect_save_with_codec_backend(&output_path, true, Some(&backend), None).unwrap();
        assert_eq!(value["private"]["player"]["playerName"], "Nameless");
        assert_eq!(
            value["private"]["player"]["writable"],
            json!(["private.player.setPlayerName"])
        );
        let strings = value["private"]["strings"].as_array().unwrap();
        assert!(strings.iter().any(|value| value == "Hero"));
        assert!(strings.iter().any(|value| value == "Nameless"));
    }

    #[test]
    fn write_save_updates_private_profile_name_property_only() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("G1R-001.sav");
        let output_path = dir.path().join("G1R-001-edited.sav");
        let private_payload = [
            fstring("0"),
            fstring("/Script/G1R.ProfileData"),
            str_property("m_ProfileName", "0"),
            str_property("m_QuickSaveName", "G1R-005"),
            fstring("None"),
        ]
        .concat();
        let seed_compressed = b"seed-compressed".to_vec();
        let stream = compressed_stream_with_one_chunk(&seed_compressed, private_payload.len());
        fs::write(
            &path,
            build_gsav(2, &public_payload("Slot A"), &stream, &[0, 0, 0, 0]),
        )
        .unwrap();
        let backend = PrefixCodecBackend {
            seed_compressed,
            seed_uncompressed: private_payload,
        };

        let response = write_save_with_codec_backend(
            &path,
            &[json!({
                "path": "private.profile.setProfileName",
                "value": "goresave"
            })],
            false,
            Some(&output_path),
            Some(&backend),
        )
        .unwrap();

        assert_eq!(response["editsApplied"], 1);
        let value =
            inspect_save_with_codec_backend(&output_path, true, Some(&backend), None).unwrap();
        assert_eq!(value["private"]["player"]["profileName"], "goresave");
        assert_eq!(
            value["private"]["player"]["writable"],
            json!(["private.profile.setProfileName"])
        );
        let strings = value["private"]["strings"].as_array().unwrap();
        assert!(strings.iter().any(|value| value == "0"));
        assert!(strings.iter().any(|value| value == "goresave"));
        assert!(strings.iter().any(|value| value == "G1R-005"));
    }

    #[test]
    fn write_save_updates_private_hero_attribute_only() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("G1R-001.sav");
        let output_path = dir.path().join("G1R-001-edited.sav");
        let private_payload = hero_attribute_payload();
        let seed_compressed = b"seed-compressed".to_vec();
        let stream = compressed_stream_with_one_chunk(&seed_compressed, private_payload.len());
        fs::write(
            &path,
            build_gsav(2, &public_payload("Slot A"), &stream, &[0, 0, 0, 0]),
        )
        .unwrap();
        let backend = PrefixCodecBackend {
            seed_compressed,
            seed_uncompressed: private_payload,
        };

        let response = write_save_with_codec_backend(
            &path,
            &[json!({
                "path": "private.player.setAttribute",
                "value": { "id": "Health", "baseValue": 77.0, "currentValue": 66.0 }
            })],
            false,
            Some(&output_path),
            Some(&backend),
        )
        .unwrap();

        assert_eq!(response["editsApplied"], 1);
        let value =
            inspect_save_with_codec_backend(&output_path, true, Some(&backend), None).unwrap();
        assert_eq!(
            value["private"]["player"]["attributes"],
            json!([
                { "id": "Health", "baseValue": 77.0, "currentValue": 66.0 },
                { "id": "MaxHealth", "baseValue": 40.0, "currentValue": 40.0 },
                { "id": "Strength", "baseValue": 10.0, "currentValue": 10.0 },
                { "id": "Dexterity", "baseValue": 10.0, "currentValue": 10.0 }
            ])
        );
        let written = fs::read(&output_path).unwrap();
        let parts = split_gsav(&written).unwrap();
        let stream = parse_compressed_stream(&written, 13 + parts.public_payload.len()).unwrap();
        let decoded = decompress_private_payload(&written, &stream, &backend).unwrap();
        let refs = scan_fstrings(&decoded, 0);
        let lurker_idx = refs.iter().position(|r| r.value == "Lurker").unwrap();
        let lurker_health_idx = refs
            .iter()
            .enumerate()
            .skip(lurker_idx)
            .find(|(_, reference)| reference.value == "Health")
            .map(|(idx, _)| idx)
            .unwrap();
        let lurker_base_idx = refs
            .iter()
            .enumerate()
            .skip(lurker_health_idx)
            .find(|(_, reference)| reference.value == "BaseValue")
            .map(|(idx, _)| idx)
            .unwrap();
        assert_eq!(
            read_test_f32_property_at(&decoded, &refs, lurker_base_idx).unwrap(),
            12.0
        );
    }

    #[test]
    fn write_save_updates_private_player_transform_only() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("G1R-001.sav");
        let output_path = dir.path().join("G1R-001-edited.sav");
        let private_payload = player_transform_payload();
        let seed_compressed = b"seed-compressed".to_vec();
        let stream = compressed_stream_with_one_chunk(&seed_compressed, private_payload.len());
        fs::write(
            &path,
            build_gsav(2, &public_payload("Slot A"), &stream, &[0, 0, 0, 0]),
        )
        .unwrap();
        let backend = PrefixCodecBackend {
            seed_compressed,
            seed_uncompressed: private_payload,
        };

        let response = write_save_with_codec_backend(
            &path,
            &[json!({
                "path": "private.player.setTransform",
                "value": {
                    "location": { "x": 100.0, "y": 200.0, "z": 300.0 },
                    "rotation": { "pitch": 1.0, "yaw": 2.0, "roll": 3.0 }
                }
            })],
            false,
            Some(&output_path),
            Some(&backend),
        )
        .unwrap();

        assert_eq!(response["editsApplied"], 1);
        let value =
            inspect_save_with_codec_backend(&output_path, true, Some(&backend), None).unwrap();
        assert_eq!(
            value["private"]["player"]["transform"],
            json!({
                "location": { "x": 100.0, "y": 200.0, "z": 300.0 },
                "rotation": { "pitch": 1.0, "yaw": 2.0, "roll": 3.0 }
            })
        );
        let written = fs::read(&output_path).unwrap();
        let parts = split_gsav(&written).unwrap();
        let stream = parse_compressed_stream(&written, 13 + parts.public_payload.len()).unwrap();
        let decoded = decompress_private_payload(&written, &stream, &backend).unwrap();
        let first_location = decoded
            .windows(fstring("m_Location").len())
            .position(|window| window == fstring("m_Location").as_slice())
            .unwrap();
        let npc_location = first_location + fstring("m_Location").len();
        assert!(decoded[npc_location..].windows(24).any(|window| {
            window
                == [
                    1.0f64.to_le_bytes(),
                    2.0f64.to_le_bytes(),
                    3.0f64.to_le_bytes(),
                ]
                .concat()
                .as_slice()
        }));
    }

    #[test]
    fn write_save_updates_private_inventory_item_count_by_id() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("G1R-001.sav");
        let output_path = dir.path().join("G1R-001-edited.sav");
        let private_payload = [
            fstring("m_ItemDefinition"),
            fstring("ObjectProperty"),
            fstring("/Script/Angelscript.ItAt_Lurker_01"),
            int_property("m_ItemCount", 2),
            fstring("m_Inventory"),
            fstring("StructProperty"),
            fstring("ReplicatedInventoryMap"),
            fstring("m_ItemDefinition"),
            fstring("ObjectProperty"),
            fstring("/Script/Angelscript.ItMi_Orenugget"),
            int_property("m_ItemCount", 23),
            fstring("m_ItemDefinition"),
            fstring("ObjectProperty"),
            fstring("/Script/Angelscript.ItFo_Cheese"),
            int_property("m_ItemCount", 1),
            fstring("m_MapOfAttachedItems"),
            fstring("MapProperty"),
        ]
        .concat();
        let seed_compressed = b"seed-compressed".to_vec();
        let stream = compressed_stream_with_one_chunk(&seed_compressed, private_payload.len());
        fs::write(
            &path,
            build_gsav(2, &public_payload("Slot A"), &stream, &[0, 0, 0, 0]),
        )
        .unwrap();
        let backend = PrefixCodecBackend {
            seed_compressed,
            seed_uncompressed: private_payload,
        };

        let response = write_save_with_codec_backend(
            &path,
            &[json!({
                "path": "private.inventory.setItemCount",
                "value": { "id": "ItMi_Orenugget", "count": 99 }
            })],
            false,
            Some(&output_path),
            Some(&backend),
        )
        .unwrap();

        assert_eq!(response["editsApplied"], 1);
        let value =
            inspect_save_with_codec_backend(&output_path, true, Some(&backend), None).unwrap();
        assert_eq!(value["private"]["inventory"]["itemStackCount"], 2);
        assert_eq!(
            value["private"]["inventory"]["items"],
            json!([
                {
                    "id": "ItMi_Orenugget",
                    "path": "/Script/Angelscript.ItMi_Orenugget",
                    "count": 99,
                    "removable": false,
                },
                {
                    "id": "ItFo_Cheese",
                    "path": "/Script/Angelscript.ItFo_Cheese",
                    "count": 1,
                    "removable": false,
                }
            ])
        );
    }

    #[test]
    fn search_typed_properties_finds_editable_scalars() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("G1R-search.sav");
        let private_payload = {
            let mut p = fstring("/Script/Angelscript.GothicFinalDataGame");
            p.push(0);
            p.extend_from_slice(&int_property("m_SaveVersionNumber", 17));
            p.extend_from_slice(&int_property("m_MaxQuick", 3));
            p.extend_from_slice(&fstring("None"));
            p.extend_from_slice(&0u32.to_le_bytes());
            p
        };
        let seed_compressed = b"seed".to_vec();
        let stream = compressed_stream_with_one_chunk(&seed_compressed, private_payload.len());
        fs::write(
            &path,
            build_gsav(2, &public_payload("Slot"), &stream, &[0, 0, 0, 0]),
        )
        .unwrap();
        let backend = PrefixCodecBackend {
            seed_compressed,
            seed_uncompressed: private_payload,
        };

        let value = search_typed_properties(&path, &json!({ "query": "MaxQuick" }), Some(&backend))
            .unwrap();

        assert_eq!(value["count"], 1);
        assert_eq!(value["total"], 1);
        let hit = &value["results"][0];
        assert_eq!(hit["display"], "m_MaxQuick");
        assert_eq!(hit["type"], "IntProperty");
        assert_eq!(hit["value"], "3");
        assert_eq!(hit["editable"], true);
        assert_eq!(hit["path"], json!(["m_MaxQuick"]));

        // empty query lists every leaf scalar
        let all = search_typed_properties(&path, &json!({ "query": "" }), Some(&backend)).unwrap();
        assert_eq!(all["total"], 2);
        assert_eq!(all["count"], 2);

        // pagination: page size 1 returns one entry and the full total
        let page0 = search_typed_properties(
            &path,
            &json!({ "query": "", "offset": 0, "limit": 1 }),
            Some(&backend),
        )
        .unwrap();
        assert_eq!(page0["count"], 1);
        assert_eq!(page0["total"], 2);
        let page1 = search_typed_properties(
            &path,
            &json!({ "query": "", "offset": 1, "limit": 1 }),
            Some(&backend),
        )
        .unwrap();
        assert_eq!(page1["count"], 1);
        assert_ne!(
            page0["results"][0]["display"],
            page1["results"][0]["display"]
        );
    }

    #[test]
    fn write_save_applies_typed_set_value_edit() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("G1R-001.sav");
        let output_path = dir.path().join("G1R-001-typed.sav");
        // Valid typed root: class + flag + {m_SaveVersionNumber:17, m_MaxQuick:3} + None + footer
        let private_payload = {
            let mut p = fstring("/Script/Angelscript.GothicFinalDataGame");
            p.push(0);
            p.extend_from_slice(&int_property("m_SaveVersionNumber", 17));
            p.extend_from_slice(&int_property("m_MaxQuick", 3));
            p.extend_from_slice(&fstring("None"));
            p.extend_from_slice(&0u32.to_le_bytes());
            p
        };
        let seed_compressed = b"seed-compressed".to_vec();
        let stream = compressed_stream_with_one_chunk(&seed_compressed, private_payload.len());
        fs::write(
            &path,
            build_gsav(2, &public_payload("Slot A"), &stream, &[0, 0, 0, 0]),
        )
        .unwrap();
        let backend = PrefixCodecBackend {
            seed_compressed,
            seed_uncompressed: private_payload,
        };

        // typed parse must gate the writable entry
        let inspected = inspect_save_with_codec_backend(&path, true, Some(&backend), None).unwrap();
        assert_eq!(inspected["private"]["typedParse"]["status"], "ok");
        assert!(
            inspected["private"]["writable"]
                .as_array()
                .unwrap()
                .contains(&json!("private.typed.setValue"))
        );
        // knowledge.addCharacter is gated only on the typed parse (no map/
        // main_container gating), so a typed-ok payload advertises it even when
        // it carries no CharacterKnowledgeByUniqueName map.
        assert!(
            inspected["private"]["writable"]
                .as_array()
                .unwrap()
                .contains(&json!("private.knowledge.addCharacter"))
        );

        let response = write_save_with_codec_backend(
            &path,
            &[json!({
                "path": "private.typed.setValue",
                "value": { "path": ["m_MaxQuick"], "value": 9 }
            })],
            false,
            Some(&output_path),
            Some(&backend),
        )
        .unwrap();
        assert_eq!(response["editsApplied"], 1);

        let value =
            inspect_save_with_codec_backend(&output_path, true, Some(&backend), None).unwrap();
        assert_eq!(value["private"]["typedParse"]["status"], "ok");
        // patched payload re-parses and carries the new value
        let strings = value["private"]["strings"].as_array().unwrap();
        assert!(strings.iter().any(|s| s == "m_MaxQuick"));
    }

    #[test]
    fn typed_set_value_dispatches_byte_property_forms() {
        let mut payload = fstring("/Script/Test.Save");
        payload.push(0);
        payload.extend_from_slice(&private_byte_plain_property("m_Level", 3));
        payload.extend_from_slice(&private_byte_enum_property("m_Rank", "ERank::Novice"));
        payload.extend_from_slice(&fstring("None"));
        payload.extend_from_slice(&0u32.to_le_bytes());

        // Plain byte: number accepted, string rejected without mutation.
        let copy = payload.clone();
        let bad = PrivateTypedSetValueEdit {
            path: properties::parse_path(&["m_Level".to_string()]).unwrap(),
            value: json!("ERank::Master"),
        };
        assert!(apply_private_typed_set_value_edit_to_payload(&mut payload, &bad).is_err());
        assert_eq!(payload, copy);
        let edit = PrivateTypedSetValueEdit {
            path: properties::parse_path(&["m_Level".to_string()]).unwrap(),
            value: json!(42),
        };
        apply_private_typed_set_value_edit_to_payload(&mut payload, &edit).unwrap();

        // Enum-as-byte: a JSON number is accepted and stringified to the
        // label, so an all-digit enum value (e.g. an unchanged "1") still
        // saves instead of failing. A plain string label works too.
        let edit = PrivateTypedSetValueEdit {
            path: properties::parse_path(&["m_Rank".to_string()]).unwrap(),
            value: json!(1),
        };
        apply_private_typed_set_value_edit_to_payload(&mut payload, &edit).unwrap();
        assert_eq!(
            properties::parse_private_root(&payload).unwrap().properties[1].value,
            properties::PropertyValue::Enum("1".to_string())
        );
        let edit = PrivateTypedSetValueEdit {
            path: properties::parse_path(&["m_Rank".to_string()]).unwrap(),
            value: json!("ERank::Master"),
        };
        apply_private_typed_set_value_edit_to_payload(&mut payload, &edit).unwrap();

        let root = properties::parse_private_root(&payload).unwrap();
        assert_eq!(
            root.properties[0].value,
            properties::PropertyValue::Byte(42)
        );
        assert_eq!(
            root.properties[1].value,
            properties::PropertyValue::Enum("ERank::Master".to_string())
        );
    }

    #[test]
    fn write_save_applies_typed_string_edit_with_length_change() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("G1R-001.sav");
        let output_path = dir.path().join("G1R-001-string.sav");
        let private_payload = {
            let mut p = fstring("/Script/Angelscript.GothicFinalDataGame");
            p.push(0);
            p.extend_from_slice(&private_str_property("m_PlayerName", "Hero"));
            p.extend_from_slice(&int_property("m_MaxQuick", 3));
            p.extend_from_slice(&fstring("None"));
            p.extend_from_slice(&0u32.to_le_bytes());
            p
        };
        let seed_compressed = b"seed-compressed".to_vec();
        let stream = compressed_stream_with_one_chunk(&seed_compressed, private_payload.len());
        fs::write(
            &path,
            build_gsav(2, &public_payload("Slot A"), &stream, &[0, 0, 0, 0]),
        )
        .unwrap();
        let backend = PrefixCodecBackend {
            seed_compressed,
            seed_uncompressed: private_payload,
        };

        let response = write_save_with_codec_backend(
            &path,
            &[json!({
                "path": "private.typed.setValue",
                "value": { "path": ["m_PlayerName"], "value": "Nameless" }
            })],
            false,
            Some(&output_path),
            Some(&backend),
        )
        .unwrap();
        assert_eq!(response["editsApplied"], 1);

        // The grown payload re-parses strictly and surfaces the new value.
        let value =
            inspect_save_with_codec_backend(&output_path, true, Some(&backend), None).unwrap();
        assert_eq!(value["private"]["typedParse"]["status"], "ok");
        let strings = value["private"]["strings"].as_array().unwrap();
        assert!(strings.iter().any(|s| s == "Nameless"));

        let searched = search_typed_properties(
            &output_path,
            &json!({ "query": "PlayerName" }),
            Some(&backend),
        )
        .unwrap();
        let hit = &searched["results"][0];
        assert_eq!(hit["value"], "Nameless");
        assert_eq!(hit["editable"], true);
    }

    #[test]
    fn typed_set_value_rejects_wrong_type_and_unknown_path() {
        let mut payload = fstring("/Script/Test.Save");
        payload.push(0);
        payload.extend_from_slice(&int_property("m_X", 1));
        payload.extend_from_slice(&fstring("None"));
        payload.extend_from_slice(&0u32.to_le_bytes());

        let mut copy = payload.clone();
        let bad_type = PrivateTypedSetValueEdit {
            path: properties::parse_path(&["m_X".to_string()]).unwrap(),
            value: json!(true),
        };
        assert!(apply_private_typed_set_value_edit_to_payload(&mut copy, &bad_type).is_err());

        let unknown = PrivateTypedSetValueEdit {
            path: properties::parse_path(&["m_Y".to_string()]).unwrap(),
            value: json!(2),
        };
        assert!(apply_private_typed_set_value_edit_to_payload(&mut copy, &unknown).is_err());
        assert_eq!(copy, payload, "failed edits must not mutate the payload");

        let string_on_int = PrivateTypedSetValueEdit {
            path: properties::parse_path(&["m_X".to_string()]).unwrap(),
            value: json!("oops"),
        };
        assert!(apply_private_typed_set_value_edit_to_payload(&mut copy, &string_on_int).is_err());
        assert_eq!(copy, payload, "failed edits must not mutate the payload");

        let ok = PrivateTypedSetValueEdit {
            path: properties::parse_path(&["m_X".to_string()]).unwrap(),
            value: json!(42),
        };
        apply_private_typed_set_value_edit_to_payload(&mut copy, &ok).unwrap();
        let root = properties::parse_private_root(&copy).unwrap();
        assert_eq!(root.properties[0].value, properties::PropertyValue::Int(42));
    }

    #[test]
    fn typed_set_value_patches_string_with_length_change() {
        let mut payload = fstring("/Script/Test.Save");
        payload.push(0);
        payload.extend_from_slice(&private_str_property("m_PlayerName", "Hero"));
        payload.extend_from_slice(&int_property("m_Gold", 250));
        payload.extend_from_slice(&fstring("None"));
        payload.extend_from_slice(&0u32.to_le_bytes());
        let original_len = payload.len();

        // Non-string value on a string target must fail without mutation.
        let copy = payload.clone();
        let bad = PrivateTypedSetValueEdit {
            path: properties::parse_path(&["m_PlayerName".to_string()]).unwrap(),
            value: json!(7),
        };
        assert!(apply_private_typed_set_value_edit_to_payload(&mut payload, &bad).is_err());
        assert_eq!(payload, copy);

        let edit = PrivateTypedSetValueEdit {
            path: properties::parse_path(&["m_PlayerName".to_string()]).unwrap(),
            value: json!("Nameless"),
        };
        apply_private_typed_set_value_edit_to_payload(&mut payload, &edit).unwrap();

        assert_eq!(payload.len(), original_len + 4);
        let root = properties::parse_private_root(&payload).unwrap();
        assert_eq!(
            root.properties[0].value,
            properties::PropertyValue::Str("Nameless".to_string())
        );
        assert_eq!(
            root.properties[1].value,
            properties::PropertyValue::Int(250)
        );
    }

    #[test]
    fn write_save_rebuilds_private_stream_with_batch_compress_codec_call() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("G1R-001.sav");
        let output_path = dir.path().join("G1R-001-edited.sav");
        let first_payload = [
            fstring("m_Inventory"),
            fstring("m_ItemDefinition"),
            fstring("ObjectProperty"),
            fstring("/Script/Angelscript.ItMi_Orenugget"),
            int_property("m_ItemCount", 23),
        ]
        .concat();
        let second_payload = [fstring("m_MapOfAttachedItems"), fstring("MapProperty")].concat();
        let first_compressed = b"first-compressed".to_vec();
        let second_compressed = b"second-compressed".to_vec();
        let stream = compressed_stream_with_chunks(&[
            (first_compressed.clone(), first_payload.len() as u64),
            (second_compressed.clone(), second_payload.len() as u64),
        ]);
        fs::write(
            &path,
            build_gsav(2, &public_payload("Slot A"), &stream, &[0, 0, 0, 0]),
        )
        .unwrap();
        let backend = BatchOnlyCodecBackend::new(vec![
            (first_compressed, first_payload),
            (second_compressed, second_payload),
        ]);

        write_save_with_codec_backend(
            &path,
            &[json!({
                "path": "private.inventory.setItemCount",
                "value": { "id": "ItMi_Orenugget", "count": 99 }
            })],
            false,
            Some(&output_path),
            Some(&backend),
        )
        .unwrap();

        let value =
            inspect_save_with_codec_backend(&output_path, true, Some(&backend), None).unwrap();
        assert_eq!(value["private"]["inventory"]["items"][0]["count"], 99);
    }

    #[test]
    fn inspect_save_decodes_private_stream_with_codec_backend() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("G1R-001.sav");
        let private_payload = [fstring("Hero"), fstring("ChapterOne")].concat();
        let seed_compressed = b"seed-compressed".to_vec();
        let stream = compressed_stream_with_one_chunk(&seed_compressed, private_payload.len());
        fs::write(
            &path,
            build_gsav(2, &public_payload("Slot A"), &stream, &[0, 0, 0, 0]),
        )
        .unwrap();
        let backend = PrefixCodecBackend {
            seed_compressed,
            seed_uncompressed: private_payload,
        };

        let value = inspect_save_with_codec_backend(&path, true, Some(&backend), None).unwrap();

        assert_eq!(value["private"]["status"], "decoded");
        assert_eq!(value["private"]["decompressedSize"], 24);
        assert_eq!(value["private"]["stringCount"], 2);
        assert_eq!(value["private"]["strings"][0], "Hero");
        assert_eq!(value["private"]["strings"][1], "ChapterOne");
        assert_eq!(value["private"]["writable"][0], "private.replaceFString");
    }

    #[test]
    fn inspect_save_decodes_private_chunks_with_one_batch_codec_call() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("G1R-001.sav");
        let first_payload = fstring("Hero");
        let second_payload = fstring("ChapterOne");
        let first_compressed = b"first-compressed".to_vec();
        let second_compressed = b"second-compressed".to_vec();
        let stream = compressed_stream_with_chunks(&[
            (first_compressed.clone(), first_payload.len() as u64),
            (second_compressed.clone(), second_payload.len() as u64),
        ]);
        fs::write(
            &path,
            build_gsav(2, &public_payload("Slot A"), &stream, &[0, 0, 0, 0]),
        )
        .unwrap();
        let backend = BatchOnlyCodecBackend::new(vec![
            (first_compressed.clone(), first_payload),
            (second_compressed.clone(), second_payload),
        ]);

        let value = inspect_save_with_codec_backend(&path, true, Some(&backend), None).unwrap();

        assert_eq!(value["private"]["status"], "decoded");
        assert_eq!(value["private"]["decodedChunkCount"], 2);
        assert_eq!(value["private"]["strings"], json!(["Hero", "ChapterOne"]));
        assert_eq!(
            backend.batch_calls(),
            vec![vec![first_compressed, second_compressed]]
        );
    }

    #[test]
    fn inspect_save_reports_typed_private_player_summary() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("G1R-001.sav");
        let private_payload = [
            fstring("/Script/Angelscript.GothicFinalDataGame"),
            int_property("m_SaveVersionNumber", 17),
            str_property("m_CurrentWorld", "WORLD"),
            fstring("None"),
        ]
        .concat();
        let seed_compressed = b"seed-compressed".to_vec();
        let stream = compressed_stream_with_one_chunk(&seed_compressed, private_payload.len());
        fs::write(
            &path,
            build_gsav(2, &public_payload("Slot A"), &stream, &[0, 0, 0, 0]),
        )
        .unwrap();
        let backend = PrefixCodecBackend {
            seed_compressed,
            seed_uncompressed: private_payload,
        };

        let value = inspect_save_with_codec_backend(&path, true, Some(&backend), None).unwrap();

        assert_eq!(value["private"]["player"]["saveVersionNumber"], 17);
        assert_eq!(value["private"]["player"]["currentWorld"], "WORLD");
        assert_eq!(
            value["private"]["player"]["scriptPaths"][0],
            "/Script/Angelscript.GothicFinalDataGame"
        );
        assert_eq!(
            value["private"]["player"]["properties"][0],
            "m_SaveVersionNumber"
        );
        assert_eq!(
            value["private"]["player"]["properties"][1],
            "m_CurrentWorld"
        );
    }

    #[test]
    fn inspect_save_reports_private_player_name_from_user_name_property() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("G1R-001.sav");
        let private_payload = [
            fstring("/Script/Angelscript.GothicFinalDataGame"),
            str_property("m_UserName", "Hero"),
            fstring("None"),
        ]
        .concat();
        let seed_compressed = b"seed-compressed".to_vec();
        let stream = compressed_stream_with_one_chunk(&seed_compressed, private_payload.len());
        fs::write(
            &path,
            build_gsav(2, &public_payload("Slot A"), &stream, &[0, 0, 0, 0]),
        )
        .unwrap();
        let backend = PrefixCodecBackend {
            seed_compressed,
            seed_uncompressed: private_payload,
        };

        let value = inspect_save_with_codec_backend(&path, true, Some(&backend), None).unwrap();

        assert_eq!(value["private"]["player"]["playerName"], "Hero");
        assert_eq!(
            value["private"]["player"]["writable"],
            json!(["private.player.setPlayerName"])
        );
    }

    #[test]
    fn inspect_save_does_not_report_private_player_name_from_none_sentinel() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("G1R-001.sav");
        let private_payload = [
            fstring("/Script/Angelscript.GothicFinalDataGame"),
            fstring("m_UserName"),
            fstring("None"),
        ]
        .concat();
        let seed_compressed = b"seed-compressed".to_vec();
        let stream = compressed_stream_with_one_chunk(&seed_compressed, private_payload.len());
        fs::write(
            &path,
            build_gsav(2, &public_payload("Slot A"), &stream, &[0, 0, 0, 0]),
        )
        .unwrap();
        let backend = PrefixCodecBackend {
            seed_compressed,
            seed_uncompressed: private_payload,
        };

        let value = inspect_save_with_codec_backend(&path, true, Some(&backend), None).unwrap();

        assert_eq!(value["private"]["player"]["playerName"], Value::Null);
        assert_eq!(value["private"]["player"]["writable"], json!([]));
    }

    #[test]
    fn inspect_save_reports_private_profile_name_property() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("G1R-001.sav");
        let private_payload = [
            fstring("/Script/G1R.ProfileData"),
            str_property("m_ProfileName", "0"),
            fstring("None"),
        ]
        .concat();
        let seed_compressed = b"seed-compressed".to_vec();
        let stream = compressed_stream_with_one_chunk(&seed_compressed, private_payload.len());
        fs::write(
            &path,
            build_gsav(2, &public_payload("Slot A"), &stream, &[0, 0, 0, 0]),
        )
        .unwrap();
        let backend = PrefixCodecBackend {
            seed_compressed,
            seed_uncompressed: private_payload,
        };

        let value = inspect_save_with_codec_backend(&path, true, Some(&backend), None).unwrap();

        assert_eq!(value["private"]["player"]["profileName"], "0");
        assert_eq!(
            value["private"]["player"]["writable"],
            json!(["private.profile.setProfileName"])
        );
    }

    #[test]
    fn inspect_save_does_not_report_private_profile_name_without_str_property() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("G1R-001.sav");
        let private_payload = [
            fstring("/Script/G1R.ProfileData"),
            fstring("m_ProfileName"),
            fstring("None"),
        ]
        .concat();
        let seed_compressed = b"seed-compressed".to_vec();
        let stream = compressed_stream_with_one_chunk(&seed_compressed, private_payload.len());
        fs::write(
            &path,
            build_gsav(2, &public_payload("Slot A"), &stream, &[0, 0, 0, 0]),
        )
        .unwrap();
        let backend = PrefixCodecBackend {
            seed_compressed,
            seed_uncompressed: private_payload,
        };

        let value = inspect_save_with_codec_backend(&path, true, Some(&backend), None).unwrap();

        assert_eq!(value["private"]["player"]["profileName"], Value::Null);
        assert_eq!(value["private"]["player"]["writable"], json!([]));
    }

    #[test]
    fn inspect_save_reports_private_hero_attributes() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("G1R-001.sav");
        let private_payload = hero_attribute_payload();
        let seed_compressed = b"seed-compressed".to_vec();
        let stream = compressed_stream_with_one_chunk(&seed_compressed, private_payload.len());
        fs::write(
            &path,
            build_gsav(2, &public_payload("Slot A"), &stream, &[0, 0, 0, 0]),
        )
        .unwrap();
        let backend = PrefixCodecBackend {
            seed_compressed,
            seed_uncompressed: private_payload,
        };

        let value = inspect_save_with_codec_backend(&path, true, Some(&backend), None).unwrap();

        assert_eq!(
            value["private"]["player"]["attributes"],
            json!([
                { "id": "Health", "baseValue": 40.0, "currentValue": 25.0 },
                { "id": "MaxHealth", "baseValue": 40.0, "currentValue": 40.0 },
                { "id": "Strength", "baseValue": 10.0, "currentValue": 10.0 },
                { "id": "Dexterity", "baseValue": 10.0, "currentValue": 10.0 }
            ])
        );
        assert_eq!(
            value["private"]["player"]["writable"],
            json!(["private.player.setAttribute"])
        );
    }

    #[test]
    fn inspect_save_reports_private_player_transform() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("G1R-001.sav");
        let private_payload = player_transform_payload();
        let seed_compressed = b"seed-compressed".to_vec();
        let stream = compressed_stream_with_one_chunk(&seed_compressed, private_payload.len());
        fs::write(
            &path,
            build_gsav(2, &public_payload("Slot A"), &stream, &[0, 0, 0, 0]),
        )
        .unwrap();
        let backend = PrefixCodecBackend {
            seed_compressed,
            seed_uncompressed: private_payload,
        };

        let value = inspect_save_with_codec_backend(&path, true, Some(&backend), None).unwrap();

        assert_eq!(
            value["private"]["player"]["transform"],
            json!({
                "location": { "x": 10.0, "y": 20.0, "z": 30.0 },
                "rotation": { "pitch": 40.0, "yaw": 50.0, "roll": 60.0 }
            })
        );
        assert_eq!(
            value["private"]["player"]["writable"],
            json!(["private.player.setTransform"])
        );
    }

    #[test]
    fn inspect_save_reports_private_inventory_summary() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("G1R-001.sav");
        let private_payload = [
            fstring("/Script/G1R.InventorySaveGameData"),
            fstring("m_InventoryItems"),
            fstring("m_StackCount"),
            fstring("ITMI_GOLD"),
            fstring("/Game/G1R/Items/BP_Item_Ore.BP_Item_Ore_C"),
            fstring("m_ItemDefinition"),
            fstring("ObjectProperty"),
            fstring("/Script/Angelscript.ItMi_Orenugget"),
            int_property("m_ItemCount", 42),
            fstring("m_SaveVersionNumber"),
        ]
        .concat();
        let seed_compressed = b"seed-compressed".to_vec();
        let stream = compressed_stream_with_one_chunk(&seed_compressed, private_payload.len());
        fs::write(
            &path,
            build_gsav(2, &public_payload("Slot A"), &stream, &[0, 0, 0, 0]),
        )
        .unwrap();
        let backend = PrefixCodecBackend {
            seed_compressed,
            seed_uncompressed: private_payload,
        };

        let value = inspect_save_with_codec_backend(&path, true, Some(&backend), None).unwrap();

        assert_eq!(
            value["private"]["inventory"]["scriptPaths"][0],
            "/Script/G1R.InventorySaveGameData"
        );
        assert_eq!(
            value["private"]["inventory"]["properties"],
            json!([
                "m_InventoryItems",
                "m_StackCount",
                "m_ItemDefinition",
                "m_ItemCount"
            ])
        );
        assert_eq!(
            value["private"]["inventory"]["candidates"],
            json!(["ITMI_GOLD", "/Game/G1R/Items/BP_Item_Ore.BP_Item_Ore_C"])
        );
        assert_eq!(value["private"]["inventory"]["candidateCount"], 2);
        assert_eq!(value["private"]["inventory"]["itemStackCount"], 1);
        assert_eq!(
            value["private"]["inventory"]["items"][0],
            json!({
                "id": "ItMi_Orenugget",
                "path": "/Script/Angelscript.ItMi_Orenugget",
                "count": 42,
                "removable": false,
            })
        );
    }

    #[test]
    fn inspect_save_reports_progression_overview_unavailable_on_bad_payload() {
        // When the private payload cannot be typed-parsed the overview reports
        // "unavailable" rather than returning the deleted heuristic shape.
        let dir = tempdir().unwrap();
        let path = dir.path().join("G1R-001.sav");
        // Raw fstrings are not a valid typed payload — the typed parse will fail.
        let private_payload = [
            fstring("/Script/G1R.QuestSaveGameData"),
            fstring("m_GeneratedEvents"),
            fstring("Quest.Main.Chapter01"),
        ]
        .concat();
        let seed_compressed = b"seed-compressed".to_vec();
        let stream = compressed_stream_with_one_chunk(&seed_compressed, private_payload.len());
        fs::write(
            &path,
            build_gsav(2, &public_payload("Slot A"), &stream, &[0, 0, 0, 0]),
        )
        .unwrap();
        let backend = PrefixCodecBackend {
            seed_compressed,
            seed_uncompressed: private_payload,
        };

        let value = inspect_save_with_codec_backend(&path, true, Some(&backend), None).unwrap();

        // Typed parse fails on garbage → overview is unavailable.
        assert_eq!(value["private"]["progression"]["status"], "unavailable");
        // Old heuristic fields must not appear.
        assert!(value["private"]["progression"].get("candidates").is_none());
        assert!(value["private"]["progression"].get("sections").is_none());
        assert!(
            value["private"]["progression"]
                .get("gameplayTags")
                .is_none()
        );
    }

    #[test]
    fn inspect_save_prefers_item_stacks_inside_private_inventory_region() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("G1R-001.sav");
        let private_payload = [
            fstring("m_ItemDefinition"),
            fstring("ObjectProperty"),
            fstring("/Script/Angelscript.ItAt_Lurker_01"),
            int_property("m_ItemCount", 2),
            fstring("m_Inventory"),
            fstring("StructProperty"),
            fstring("ReplicatedInventoryMap"),
            fstring("m_ItemDefinition"),
            fstring("ObjectProperty"),
            fstring("/Script/Angelscript.ItMi_Orenugget"),
            int_property("m_ItemCount", 23),
            fstring("m_MapOfAttachedItems"),
            fstring("MapProperty"),
            fstring("m_ItemDefinition"),
            fstring("ObjectProperty"),
            fstring("/Script/Angelscript.ItFo_Cheese"),
            int_property("m_ItemCount", 1),
        ]
        .concat();
        let seed_compressed = b"seed-compressed".to_vec();
        let stream = compressed_stream_with_one_chunk(&seed_compressed, private_payload.len());
        fs::write(
            &path,
            build_gsav(2, &public_payload("Slot A"), &stream, &[0, 0, 0, 0]),
        )
        .unwrap();
        let backend = PrefixCodecBackend {
            seed_compressed,
            seed_uncompressed: private_payload,
        };

        let value = inspect_save_with_codec_backend(&path, true, Some(&backend), None).unwrap();

        assert_eq!(
            value["private"]["inventory"]["itemScope"],
            "player_inventory_region"
        );
        assert_eq!(value["private"]["inventory"]["itemStackCount"], 1);
        // This synthetic payload is detected by the FString region scan but is
        // not a complete typed property tree, so the MainContainer cannot be
        // resolved: only the in-place setItemCount edit is offered, and no row
        // is marked removable (addItem/removeItem need a typed MainContainer).
        assert_eq!(
            value["private"]["inventory"]["writable"],
            json!(["private.inventory.setItemCount"])
        );
        assert_eq!(
            value["private"]["inventory"]["items"],
            json!([{
                "id": "ItMi_Orenugget",
                "path": "/Script/Angelscript.ItMi_Orenugget",
                "count": 23,
                "removable": false,
            }])
        );
    }

    #[test]
    fn inspect_save_can_decode_limited_private_preview() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("G1R-001.sav");
        let first_payload = fstring("Hero");
        let second_payload = fstring("HiddenLater");
        let first_compressed = b"seed-compressed".to_vec();
        let mut second_compressed = b"CMP:".to_vec();
        second_compressed.extend_from_slice(&second_payload);
        let stream = compressed_stream_with_chunks(&[
            (first_compressed.clone(), first_payload.len() as u64),
            (second_compressed, second_payload.len() as u64),
        ]);
        fs::write(
            &path,
            build_gsav(2, &public_payload("Slot A"), &stream, &[0, 0, 0, 0]),
        )
        .unwrap();
        let backend = PrefixCodecBackend {
            seed_compressed: first_compressed,
            seed_uncompressed: first_payload,
        };

        let value = inspect_save_with_codec_backend(&path, true, Some(&backend), Some(1)).unwrap();

        assert_eq!(value["private"]["status"], "decoded_preview");
        assert_eq!(value["private"]["decodedChunkCount"], 1);
        assert_eq!(value["private"]["totalChunkCount"], 2);
        assert_eq!(value["private"]["strings"], json!(["Hero"]));
    }

    #[test]
    fn validate_codec_roundtrip_uses_first_save_chunk() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("G1R-001.sav");
        let private_payload = fstring("Hero");
        let seed_compressed = b"seed-compressed".to_vec();
        let stream = compressed_stream_with_one_chunk(&seed_compressed, private_payload.len());
        fs::write(
            &path,
            build_gsav(2, &public_payload("Slot A"), &stream, &[0, 0, 0, 0]),
        )
        .unwrap();
        let backend = PrefixCodecBackend {
            seed_compressed,
            seed_uncompressed: private_payload,
        };

        let value = validate_codec_roundtrip_with_backend(&path, &backend).unwrap();

        assert_eq!(value["status"], "codec_roundtrip_passed");
        assert_eq!(value["chunkIndex"], 0);
        assert_eq!(value["decompressedSize"], 9);
        assert_eq!(value["recompressedSize"], 13);
    }

    #[test]
    fn ffi_execute_returns_json() {
        let response = execute_json(r#"{"command":"check_codec","payload":{}}"#);
        let value: Value = serde_json::from_str(&response).unwrap();
        assert_eq!(value["ok"], true);
        assert_eq!(value["data"]["available"], false);
        assert_eq!(value["data"]["status"], "native_encoder_in_progress");
        assert!(value["data"].to_string().contains("pure_rust_kraken"));
        assert!(!value["data"].to_string().contains("oo2core"));
    }

    #[test]
    fn check_codec_exposes_pure_backend_by_default() {
        let response = execute_json(r#"{"command":"check_codec","payload":{}}"#);
        let value: Value = serde_json::from_str(&response).unwrap();

        assert_eq!(value["ok"], true);
        assert_eq!(value["data"]["selectedBackend"], "pure_rust_kraken");
        assert_eq!(value["data"]["backends"][0]["backend"], "pure_rust_kraken");
        assert_eq!(
            value["data"]["backends"][0]["status"],
            "native_encoder_in_progress"
        );
        assert_eq!(
            value["data"]["message"],
            "Native private payload encoder is not available yet."
        );
    }

    #[test]
    fn codec_status_prefers_configured_binary_host_when_available() {
        let pure_probe = codec_backend::CodecBackendProbe {
            backend: "pure_rust_kraken".to_string(),
            available: false,
            can_decompress: false,
            can_compress: false,
            status: "native_encoder_in_progress".to_string(),
            profile: None,
            resolution_mode: None,
            details: json!({
                "adapter": "pure_rust_kraken",
                "available": false,
                "canDecompress": false,
                "canCompress": false,
                "status": "native_encoder_in_progress",
                "message": "native encoder is unavailable"
            }),
        };
        let binary_probe = codec_backend::CodecBackendProbe {
            backend: "g1r_binary_host".to_string(),
            available: true,
            can_decompress: true,
            can_compress: true,
            status: "supported".to_string(),
            profile: Some("g1r-23A85CE7".to_string()),
            resolution_mode: Some("known_profile".to_string()),
            details: json!({
                "supported": true,
                "canDecompress": true,
                "canCompress": true,
                "profile": "g1r-23A85CE7",
                "resolutionMode": "known_profile"
            }),
        };

        let value = codec_status_from_probes(pure_probe, Some(Ok(binary_probe))).unwrap();

        assert_eq!(value["selectedBackend"], "g1r_binary_host");
        assert_eq!(value["adapter"], "g1r_binary_host");
        assert_eq!(value["available"], true);
        assert_eq!(value["canDecompress"], true);
        assert_eq!(value["canCompress"], true);
        assert_eq!(value["profile"], "g1r-23A85CE7");
        assert_eq!(value["resolutionMode"], "known_profile");
        assert_eq!(value["status"], "codec_host_ready");
        assert!(
            value["message"]
                .as_str()
                .unwrap()
                .contains("G1R codec host")
        );
    }

    #[test]
    fn check_codec_reports_optional_binary_host_probe_errors_without_selecting_it() {
        let response = execute_json(
            r#"{
                "command": "check_codec",
                "payload": {
                    "binaryHost": {
                        "helperPath": "Z:\\missing\\goresave_g1r_codec_host.exe",
                        "exePath": "D:\\G1R-Win64-Shipping.exe"
                    }
                }
            }"#,
        );
        let value: Value = serde_json::from_str(&response).unwrap();

        assert_eq!(value["ok"], true);
        assert_eq!(value["data"]["selectedBackend"], "pure_rust_kraken");
        assert_eq!(value["data"]["backends"][1]["backend"], "g1r_binary_host");
        assert_eq!(value["data"]["backends"][1]["available"], false);
        assert_eq!(value["data"]["backends"][1]["status"], "probe_failed");
        assert!(
            value["data"]["backends"][1]["error"]
                .as_str()
                .unwrap()
                .contains("io error")
        );
    }

    #[test]
    fn check_codec_auto_calibrates_pattern_profile_then_reports_supported() {
        let backend = codec_backend::G1rBinaryHostBackend::with_command_dispatch_for_tests(
            "D:\\G1R-Win64-Shipping.exe",
            |command| match command {
                "probe" => Ok(json!({
                    "supported": false,
                    "canDecompress": false,
                    "canCompress": false,
                    "profile": "g1r-23A85CE7",
                    "resolutionMode": "pattern_profile"
                })),
                "calibrate" => Ok(json!({
                    "supported": true,
                    "canDecompress": true,
                    "canCompress": true,
                    "profile": "g1r-derived-77f3d48c",
                    "resolutionMode": "derived_profile_cache",
                    "calibrationRan": true
                })),
                other => Err(CoreError::Codec(format!("unexpected command {other}"))),
            },
        );

        let probe = codec_backend::CodecBackend::probe(&backend).unwrap();
        assert_eq!(probe.resolution_mode.as_deref(), Some("pattern_profile"));
        assert!(!probe.available);

        let promoted = auto_calibrate_if_pattern_profile(&backend, probe, "");

        assert!(promoted.available);
        assert!(promoted.can_compress);
        assert_eq!(
            promoted.resolution_mode.as_deref(),
            Some("derived_profile_cache")
        );
    }

    #[test]
    fn auto_calibrate_keeps_unsupported_probe_when_calibrate_errors() {
        let backend = codec_backend::G1rBinaryHostBackend::with_command_dispatch_for_tests(
            "D:\\G1R-Win64-Shipping.exe",
            |command| match command {
                "probe" => Ok(json!({
                    "supported": false,
                    "canDecompress": false,
                    "canCompress": false,
                    "profile": "g1r-23A85CE7",
                    "resolutionMode": "pattern_profile"
                })),
                "calibrate" => Err(CoreError::Codec("calibration selftest failed".to_string())),
                other => Err(CoreError::Codec(format!("unexpected command {other}"))),
            },
        );

        let probe = codec_backend::CodecBackend::probe(&backend).unwrap();
        assert_eq!(probe.resolution_mode.as_deref(), Some("pattern_profile"));
        assert!(!probe.available);

        let result = auto_calibrate_if_pattern_profile(&backend, probe, "");

        // Best-effort: a calibration failure leaves the original unsupported
        // probe, not an error.
        assert!(!result.available);
        assert_eq!(result.resolution_mode.as_deref(), Some("pattern_profile"));
    }

    #[test]
    fn auto_calibrate_skips_rerun_after_failure_in_session() {
        let calls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let calls_in = calls.clone();
        let backend = codec_backend::G1rBinaryHostBackend::with_command_dispatch_for_tests(
            "D:\\G1R-Win64-Shipping.exe",
            move |command| match command {
                "calibrate" => {
                    calls_in.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    Err(CoreError::Codec("calibration selftest failed".to_string()))
                }
                other => Err(CoreError::Codec(format!("unexpected command {other}"))),
            },
        );
        let cache = std::sync::Mutex::new(std::collections::HashMap::new());
        let probe = codec_backend::CodecBackendProbe {
            backend: "g1r_binary_host".to_string(),
            available: false,
            can_decompress: false,
            can_compress: false,
            status: "unsupported".to_string(),
            profile: Some("g1r-23A85CE7".to_string()),
            resolution_mode: Some("pattern_profile".to_string()),
            details: json!({ "exeSha256": "deadbeefdeadbeef" }),
        };

        // Four checks of the same unpromotable build: calibration retries up to
        // the attempt budget (MAX_CALIBRATION_ATTEMPTS), then stops re-running the
        // expensive selftest. The build stays unsupported throughout.
        let mut last = probe.clone();
        for _ in 0..4 {
            last = auto_calibrate_bounded(&backend, probe.clone(), "", &cache);
        }
        assert!(!last.available);
        assert_eq!(
            calls.load(std::sync::atomic::Ordering::SeqCst),
            MAX_CALIBRATION_ATTEMPTS as usize
        );
    }

    #[test]
    fn auto_calibrate_recalibrates_decompress_only_supported_probe() {
        // A supported-but-decompress-only derived-cache entry must be
        // recalibrated so it can be promoted to write-capable.
        let backend = codec_backend::G1rBinaryHostBackend::with_command_dispatch_for_tests(
            "D:\\G1R-Win64-Shipping.exe",
            |command| match command {
                "calibrate" => Ok(json!({
                    "supported": true,
                    "canDecompress": true,
                    "canCompress": true,
                    "profile": "g1r-derived-77f3d48c",
                    "resolutionMode": "derived_profile_cache",
                    "calibrationRan": true
                })),
                other => Err(CoreError::Codec(format!("unexpected command {other}"))),
            },
        );
        let cache = std::sync::Mutex::new(std::collections::HashMap::new());
        let probe = codec_backend::CodecBackendProbe {
            backend: "g1r_binary_host".to_string(),
            available: true,
            can_decompress: true,
            can_compress: false,
            status: "codec_host_decompress_ready".to_string(),
            profile: Some("g1r-derived-77f3d48c".to_string()),
            resolution_mode: Some("derived_profile_cache".to_string()),
            details: json!({ "exeSha256": "cafecafecafecafe" }),
        };

        let promoted = auto_calibrate_bounded(&backend, probe, "", &cache);

        assert!(promoted.can_compress);
    }

    #[test]
    fn auto_calibrate_reprobes_and_retries_after_decode_only_calibration() {
        // calibrate fails (compress selftest failed) but a decode-only derived
        // cache now exists, which the re-probe reports.
        let calls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let calls_in = calls.clone();
        let backend = codec_backend::G1rBinaryHostBackend::with_command_dispatch_for_tests(
            "D:\\G1R-Win64-Shipping.exe",
            move |command| match command {
                "calibrate" => {
                    calls_in.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    Err(CoreError::Codec("compress selftest failed".to_string()))
                }
                "probe" => Ok(json!({
                    "supported": true,
                    "canDecompress": true,
                    "canCompress": false,
                    "profile": "g1r-derived-77f3d48c",
                    "resolutionMode": "derived_profile_cache",
                    "exeSha256": "feedfacefeedface"
                })),
                other => Err(CoreError::Codec(format!("unexpected command {other}"))),
            },
        );
        let cache = std::sync::Mutex::new(std::collections::HashMap::new());
        let probe = codec_backend::CodecBackendProbe {
            backend: "g1r_binary_host".to_string(),
            available: false,
            can_decompress: false,
            can_compress: false,
            status: "unsupported".to_string(),
            profile: Some("g1r-23A85CE7".to_string()),
            resolution_mode: Some("pattern_profile".to_string()),
            details: json!({ "exeSha256": "feedfacefeedface" }),
        };

        let first = auto_calibrate_bounded(&backend, probe.clone(), "", &cache);
        // Not stale: reflects the decode-only cache, usable for reading.
        assert!(first.available);
        assert!(!first.can_compress);

        // A decode-only build retries the compress selftest (a transient failure
        // may clear) but is still bounded: after MAX_CALIBRATION_ATTEMPTS it stops
        // re-running the expensive selftest instead of doing so on every check.
        for _ in 0..4 {
            let _ = auto_calibrate_bounded(&backend, probe.clone(), "", &cache);
        }
        assert_eq!(
            calls.load(std::sync::atomic::Ordering::SeqCst),
            MAX_CALIBRATION_ATTEMPTS as usize
        );
    }

    #[test]
    fn codec_status_unsupported_binary_build_shows_plain_message() {
        let pure_probe = codec_backend::CodecBackendProbe {
            backend: "pure_rust_kraken".to_string(),
            available: false,
            can_decompress: false,
            can_compress: false,
            status: "native_encoder_in_progress".to_string(),
            profile: None,
            resolution_mode: None,
            details: json!({}),
        };
        let binary_probe = codec_backend::CodecBackendProbe {
            backend: "g1r_binary_host".to_string(),
            available: false,
            can_decompress: false,
            can_compress: false,
            status: "unsupported".to_string(),
            profile: Some("g1r-23A85CE7".to_string()),
            resolution_mode: Some("pattern_profile".to_string()),
            details: json!({}),
        };

        let value = codec_status_from_probes(pure_probe, Some(Ok(binary_probe))).unwrap();

        assert_eq!(value["userSeverity"], "error");
        assert_eq!(value["userTitle"], "This game version can't be opened yet");
    }

    #[test]
    fn codec_status_ready_binary_build_shows_ready_message() {
        let pure_probe = codec_backend::CodecBackendProbe {
            backend: "pure_rust_kraken".to_string(),
            available: false,
            can_decompress: false,
            can_compress: false,
            status: "native_encoder_in_progress".to_string(),
            profile: None,
            resolution_mode: None,
            details: json!({}),
        };
        let binary_probe = codec_backend::CodecBackendProbe {
            backend: "g1r_binary_host".to_string(),
            available: true,
            can_decompress: true,
            can_compress: true,
            status: "supported".to_string(),
            profile: Some("g1r-23A85CE7".to_string()),
            resolution_mode: Some("known_profile".to_string()),
            details: json!({}),
        };

        let value = codec_status_from_probes(pure_probe, Some(Ok(binary_probe))).unwrap();

        assert_eq!(value["userSeverity"], "ok");
        assert_eq!(value["userTitle"], "Game codec ready");
    }

    #[test]
    fn codec_user_message_ready_for_compress_capable_build() {
        let m = codec_user_message_for("g1r_binary_host", true, true, true, None);
        assert_eq!(m["userSeverity"], "ok");
        assert_eq!(m["userTitle"], "Game codec ready");
    }

    #[test]
    fn codec_user_message_unsupported_for_unavailable_build() {
        let m = codec_user_message_for("g1r_binary_host", false, false, false, None);
        assert_eq!(m["userSeverity"], "error");
        assert_eq!(m["userTitle"], "This game version can't be opened yet");
        assert!(m["userHint"].as_str().unwrap().contains("editor update"));
    }

    #[test]
    fn codec_user_message_exe_not_found() {
        let m = codec_user_message_for(
            "g1r_binary_host",
            false,
            false,
            false,
            Some("G1R executable not found: D:/x/G1R-Win64-Shipping.exe"),
        );
        assert_eq!(m["userTitle"], "Gothic 1 Remake not found");
        assert!(m["userHint"].as_str().unwrap().contains("game path"));
    }

    #[test]
    fn codec_user_message_helper_misconfig_is_setup_error() {
        let m = codec_user_message_for(
            "g1r_binary_host",
            false,
            false,
            false,
            Some("binaryHost.helperPath is required"),
        );
        assert_eq!(m["userSeverity"], "error");
        assert_eq!(m["userTitle"], "Codec helper isn't set up");
        assert!(m["userHint"].as_str().unwrap().contains("settings"));
        // Must NOT send the user to wait for a game/editor update.
        assert_ne!(m["userTitle"], "This game version can't be opened yet");
    }

    #[test]
    fn codec_user_message_unresolved_build_waits_for_update() {
        let m = codec_user_message_for(
            "g1r_binary_host",
            false,
            false,
            false,
            Some("G1R executable could not be resolved to verified codec functions"),
        );
        assert_eq!(m["userTitle"], "This game version can't be opened yet");
        assert!(m["userHint"].as_str().unwrap().contains("editor update"));
    }

    #[test]
    fn codec_user_message_decode_only_is_not_ready() {
        // available + decompress but no verified compress: reading works, writing
        // is gated, so it must not claim full "ready".
        let m = codec_user_message_for("g1r_binary_host", true, false, true, None);
        assert_eq!(m["userSeverity"], "warn");
        assert_eq!(m["userTitle"], "Game codec partly ready");
        assert_ne!(m["userTitle"], "Game codec ready");
    }

    fn private_name_set_property(name: &str, values: &[&str]) -> Vec<u8> {
        let mut body = 0u32.to_le_bytes().to_vec();
        body.extend_from_slice(&(values.len() as u32).to_le_bytes());
        for v in values {
            body.extend_from_slice(&fstring(v));
        }
        let mut out = fstring(name);
        out.extend_from_slice(&fstring("SetProperty"));
        out.extend_from_slice(&1u32.to_le_bytes());
        out.extend_from_slice(&fstring("NameProperty"));
        out.extend_from_slice(&0u32.to_le_bytes()); // array_index
        out.extend_from_slice(&(body.len() as u32).to_le_bytes());
        out.push(0); // tag_flags
        out.extend_from_slice(&body);
        out
    }

    /// Wrap a single `encode_knowledge_map_entry` blob in a one-entry
    /// `CharacterKnowledgeByUniqueName` map, frame it as a private root, parse,
    /// and return the entry's key string plus its `Knowledge` set elements.
    ///
    /// The map tag header mirrors `properties::knowledge_map_property` (the
    /// corrected Task 3 layout); `parse_private_root` is the oracle — if the
    /// produced entry bytes are wrong, this won't parse.
    fn parse_single_knowledge_entry(entry_bytes: &[u8]) -> (String, Vec<String>) {
        let mut body = 0u32.to_le_bytes().to_vec(); // num_to_remove
        body.extend_from_slice(&1u32.to_le_bytes()); // count
        body.extend_from_slice(entry_bytes);

        // MapProperty<NameProperty, StructProperty(KnowledgeSet)> tag.
        let mut prop = fstring("CharacterKnowledgeByUniqueName");
        prop.extend_from_slice(&fstring("MapProperty"));
        prop.extend_from_slice(&2u32.to_le_bytes()); // descriptor count
        prop.extend_from_slice(&fstring("NameProperty")); // key type
        prop.extend_from_slice(&0u32.to_le_bytes()); // key_flags
        prop.extend_from_slice(&fstring("StructProperty")); // value type
        prop.extend_from_slice(&1u32.to_le_bytes()); // struct descriptor count
        prop.extend_from_slice(&fstring("KnowledgeSet")); // value struct type
        prop.extend_from_slice(&1u32.to_le_bytes()); // package count
        prop.extend_from_slice(&fstring("/Script/G1R")); // package
        prop.extend_from_slice(&0u32.to_le_bytes()); // array_index
        prop.extend_from_slice(&(body.len() as u32).to_le_bytes()); // size
        prop.push(0); // tag_flags
        prop.extend_from_slice(&body);

        // Private-root framing: class fstring + object flag + props + "None" + footer.
        let mut payload = fstring("/Script/Test.Save");
        payload.push(0);
        payload.extend_from_slice(&prop);
        payload.extend_from_slice(&fstring("None"));
        payload.extend_from_slice(&0u32.to_le_bytes());

        let root = properties::parse_private_root(&payload).unwrap();
        let (_, map_prop) =
            properties::find_property_by_name(&root, "CharacterKnowledgeByUniqueName").unwrap();
        let properties::PropertyValue::Map { entries, .. } = &map_prop.value else {
            panic!("CharacterKnowledgeByUniqueName is not a map");
        };
        assert_eq!(entries.len(), 1, "expected exactly one entry");
        let (key, value) = &entries[0];
        let key = match key {
            properties::PropertyValue::Name(s) | properties::PropertyValue::Str(s) => s.clone(),
            other => panic!("unexpected key {other:?}"),
        };
        let knowledge = match struct_member(value, "Knowledge") {
            Some(properties::PropertyValue::Set { elements, .. }) => elements
                .iter()
                .filter_map(|e| match e {
                    properties::PropertyValue::Name(s) | properties::PropertyValue::Str(s) => {
                        Some(s.clone())
                    }
                    _ => None,
                })
                .collect(),
            other => panic!("Knowledge member is not a set: {other:?}"),
        };
        (key, knowledge)
    }

    #[test]
    fn empty_knowledge_value_roundtrips_as_struct_with_empty_set() {
        let key = "OC_TEST_Npc";
        let entry = encode_knowledge_map_entry(key);
        let (parsed_key, knowledge) = parse_single_knowledge_entry(&entry);
        assert_eq!(parsed_key, key);
        assert!(knowledge.is_empty());
    }

    /// Build a full private-root payload whose only property is a
    /// `CharacterKnowledgeByUniqueName` map carrying `chars` (each with an empty
    /// `Knowledge` set). Uses the same corrected map-tag header proven in
    /// Tasks 3/5 (`parse_single_knowledge_entry`), generalised to N entries.
    fn build_knowledge_map_payload(chars: &[&str]) -> Vec<u8> {
        let mut entries = Vec::new();
        for name in chars {
            entries.extend_from_slice(&encode_knowledge_map_entry(name));
        }
        let mut body = 0u32.to_le_bytes().to_vec(); // num_to_remove
        body.extend_from_slice(&(chars.len() as u32).to_le_bytes()); // count
        body.extend_from_slice(&entries);

        // MapProperty<NameProperty, StructProperty(KnowledgeSet)> tag.
        let mut prop = fstring("CharacterKnowledgeByUniqueName");
        prop.extend_from_slice(&fstring("MapProperty"));
        prop.extend_from_slice(&2u32.to_le_bytes()); // descriptor count
        prop.extend_from_slice(&fstring("NameProperty")); // key type
        prop.extend_from_slice(&0u32.to_le_bytes()); // key_flags
        prop.extend_from_slice(&fstring("StructProperty")); // value type
        prop.extend_from_slice(&1u32.to_le_bytes()); // struct descriptor count
        prop.extend_from_slice(&fstring("KnowledgeSet")); // value struct type
        prop.extend_from_slice(&1u32.to_le_bytes()); // package count
        prop.extend_from_slice(&fstring("/Script/G1R")); // package
        prop.extend_from_slice(&0u32.to_le_bytes()); // array_index
        prop.extend_from_slice(&(body.len() as u32).to_le_bytes()); // size
        prop.push(0); // tag_flags
        prop.extend_from_slice(&body);

        // Private-root framing: class fstring + object flag + props + "None" + footer.
        let mut payload = fstring("/Script/Test.Save");
        payload.push(0);
        payload.extend_from_slice(&prop);
        payload.extend_from_slice(&fstring("None"));
        payload.extend_from_slice(&0u32.to_le_bytes());
        payload
    }

    #[test]
    fn add_character_inserts_empty_entry_and_rejects_duplicate() {
        let mut payload = build_knowledge_map_payload(&["OC_STT_Diego"]);
        let new_npc = "OC_TEST_BrandNew";

        // not present yet
        let root0 = properties::parse_private_root(&payload).unwrap();
        let before = progression_knowledge(&root0, "", None, 0, 10_000).unwrap();
        assert!(!before["characters"]
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c["name"] == new_npc));

        // apply
        apply_private_knowledge_add_character_to_payload(&mut payload, new_npc).unwrap();

        // present now with 0 entries; payload fully consistent
        let root1 = properties::parse_private_root(&payload).unwrap();
        assert_eq!(root1.consumed, payload.len());
        let after = progression_knowledge(&root1, "", Some(new_npc), 0, 10).unwrap();
        assert_eq!(after["total"], 0);

        // duplicate rejected (case-insensitive Name semantics)
        assert!(apply_private_knowledge_add_character_to_payload(&mut payload, new_npc).is_err());
        assert!(
            apply_private_knowledge_add_character_to_payload(&mut payload, "oc_test_brandnew")
                .is_err()
        );
    }

    #[test]
    #[ignore = "needs GORESAVE_PAYLOAD_BIN=<a decompressed host.bin>"]
    fn add_character_roundtrips_on_real_payload() {
        let path = std::env::var("GORESAVE_PAYLOAD_BIN").expect("set GORESAVE_PAYLOAD_BIN");
        let mut payload = std::fs::read(path).unwrap();
        let new_npc = "OC_TEST_BrandNew";
        apply_private_knowledge_add_character_to_payload(&mut payload, new_npc).unwrap();
        let root = properties::parse_private_root(&payload).unwrap();
        assert_eq!(root.consumed, payload.len());
        let after = progression_knowledge(&root, "", Some(new_npc), 0, 10).unwrap();
        assert_eq!(after["total"], 0);
    }

    #[test]
    fn typed_container_edits_apply_and_validate() {
        let mut payload = fstring("/Script/Test.Save");
        payload.push(0);
        payload.extend_from_slice(&private_name_set_property("Knowledge", &["A", "B"]));
        payload.extend_from_slice(&fstring("None"));
        payload.extend_from_slice(&0u32.to_le_bytes());

        let edit = PrivateTypedContainerEdit {
            path: properties::parse_path(&["Knowledge".to_string()]).unwrap(),
            edit: properties::ContainerEdit::SetAdd("C".to_string()),
        };
        apply_private_typed_container_edit_to_payload(&mut payload, &edit).unwrap();
        let edit = PrivateTypedContainerEdit {
            path: properties::parse_path(&["Knowledge".to_string()]).unwrap(),
            edit: properties::ContainerEdit::SetRemove("A".to_string()),
        };
        apply_private_typed_container_edit_to_payload(&mut payload, &edit).unwrap();

        let root = properties::parse_private_root(&payload).unwrap();
        assert_eq!(
            root.properties[0].value,
            properties::PropertyValue::Set {
                num_to_remove: 0,
                elements: vec![
                    properties::PropertyValue::Name("B".to_string()),
                    properties::PropertyValue::Name("C".to_string()),
                ],
            }
        );

        // Unknown path fails without mutation.
        let copy = payload.clone();
        let bad = PrivateTypedContainerEdit {
            path: properties::parse_path(&["Nope".to_string()]).unwrap(),
            edit: properties::ContainerEdit::SetAdd("X".to_string()),
        };
        assert!(apply_private_typed_container_edit_to_payload(&mut payload, &bad).is_err());
        assert_eq!(payload, copy);
    }

    #[test]
    fn write_save_applies_typed_container_edits() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("G1R-001.sav");
        let output_path = dir.path().join("G1R-001-container.sav");
        let private_payload = {
            let mut p = fstring("/Script/Angelscript.GothicFinalDataGame");
            p.push(0);
            p.extend_from_slice(&private_name_set_property("Knowledge", &["Voiceline_A"]));
            p.extend_from_slice(&fstring("None"));
            p.extend_from_slice(&0u32.to_le_bytes());
            p
        };
        let seed_compressed = b"seed-compressed".to_vec();
        let stream = compressed_stream_with_one_chunk(&seed_compressed, private_payload.len());
        fs::write(
            &path,
            build_gsav(2, &public_payload("Slot A"), &stream, &[0, 0, 0, 0]),
        )
        .unwrap();
        let backend = PrefixCodecBackend {
            seed_compressed,
            seed_uncompressed: private_payload,
        };

        // The container ops must be advertised once the typed parse is ok.
        let inspected = inspect_save_with_codec_backend(&path, true, Some(&backend), None).unwrap();
        let writable = inspected["private"]["writable"].as_array().unwrap();
        for op in [
            "private.typed.setAdd",
            "private.typed.setRemove",
            "private.typed.arrayRemove",
            "private.typed.arrayDuplicate",
        ] {
            assert!(writable.contains(&json!(op)), "missing writable {op}");
        }

        let response = write_save_with_codec_backend(
            &path,
            &[json!({
                "path": "private.typed.setAdd",
                "value": { "path": ["Knowledge"], "value": "ChoiceB" }
            })],
            false,
            Some(&output_path),
            Some(&backend),
        )
        .unwrap();
        assert_eq!(response["editsApplied"], 1);

        let value =
            inspect_save_with_codec_backend(&output_path, true, Some(&backend), None).unwrap();
        assert_eq!(value["private"]["typedParse"]["status"], "ok");
        let strings = value["private"]["strings"].as_array().unwrap();
        assert!(strings.iter().any(|s| s == "ChoiceB"));
    }

    fn private_str_array_property(name: &str, values: &[&str]) -> Vec<u8> {
        let mut body = (values.len() as u32).to_le_bytes().to_vec();
        for v in values {
            body.extend_from_slice(&fstring(v));
        }
        let mut out = fstring(name);
        out.extend_from_slice(&fstring("ArrayProperty"));
        out.extend_from_slice(&1u32.to_le_bytes());
        out.extend_from_slice(&fstring("StrProperty"));
        out.extend_from_slice(&0u32.to_le_bytes()); // array_index
        out.extend_from_slice(&(body.len() as u32).to_le_bytes());
        out.push(0); // tag_flags
        out.extend_from_slice(&body);
        out
    }

    #[test]
    fn write_save_rejects_multiple_structural_array_edits() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("G1R-001.sav");
        let output_path = dir.path().join("G1R-001-multi.sav");
        let private_payload = {
            let mut p = fstring("/Script/Angelscript.GothicFinalDataGame");
            p.push(0);
            p.extend_from_slice(&private_str_array_property("Events", &["A", "B", "C"]));
            p.extend_from_slice(&fstring("None"));
            p.extend_from_slice(&0u32.to_le_bytes());
            p
        };
        let seed_compressed = b"seed-compressed".to_vec();
        let stream = compressed_stream_with_one_chunk(&seed_compressed, private_payload.len());
        fs::write(
            &path,
            build_gsav(2, &public_payload("Slot A"), &stream, &[0, 0, 0, 0]),
        )
        .unwrap();
        let backend = PrefixCodecBackend {
            seed_compressed,
            seed_uncompressed: private_payload,
        };

        // Two index-addressed structural edits in one batch are rejected: the
        // second index would silently target the post-splice array.
        let err = write_save_with_codec_backend(
            &path,
            &[
                json!({
                    "path": "private.typed.arrayRemove",
                    "value": { "path": ["Events"], "index": 0 }
                }),
                json!({
                    "path": "private.typed.arrayRemove",
                    "value": { "path": ["Events"], "index": 1 }
                }),
            ],
            false,
            Some(&output_path),
            Some(&backend),
        )
        .unwrap_err();
        assert!(
            err.to_string()
                .contains("at most one structural array edit"),
            "unexpected error: {err}"
        );
        assert!(
            !output_path.exists(),
            "rejected write must not produce a file"
        );

        // A single structural edit (even alongside a value-addressed edit)
        // still applies.
        let response = write_save_with_codec_backend(
            &path,
            &[json!({
                "path": "private.typed.arrayRemove",
                "value": { "path": ["Events"], "index": 1 }
            })],
            false,
            Some(&output_path),
            Some(&backend),
        )
        .unwrap();
        assert_eq!(response["editsApplied"], 1);
    }

    #[test]
    fn write_save_rejects_inventory_structural_edit_with_peer() {
        // An inventory add/remove must stand alone: a peer edit in the same
        // write resolves against the pre-splice layout and would be corrupted.
        let dir = tempdir().unwrap();
        let path = dir.path().join("G1R-001.sav");
        let private_payload = inventory_payload_for_add_item_tests();
        let seed_compressed = b"seed-peer".to_vec();
        let stream = compressed_stream_with_one_chunk(&seed_compressed, private_payload.len());
        fs::write(
            &path,
            build_gsav(2, &public_payload("Slot A"), &stream, &[0, 0, 0, 0]),
        )
        .unwrap();
        let backend = PrefixCodecBackend {
            seed_compressed,
            seed_uncompressed: private_payload,
        };

        let err = write_save_with_codec_backend(
            &path,
            &[
                json!({
                    "path": "private.inventory.addItem",
                    "value": { "path": "/Script/Angelscript.ItMi_Orenugget", "count": 1 }
                }),
                json!({
                    "path": "private.typed.setValue",
                    "value": { "path": ["m_MaxQuick"], "value": 9 }
                }),
            ],
            false,
            None,
            Some(&backend),
        )
        .unwrap_err();
        assert!(
            err.to_string().contains("must contain no other edits"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn write_save_rejects_knowledge_add_character_with_peer() {
        // knowledge.addCharacter splices the CharacterKnowledgeByUniqueName map,
        // shifting every byte after the insert. Like an inventory structural
        // edit, it must stand alone: a peer edit in the same write resolves
        // against the pre-splice layout and would corrupt the save.
        let dir = tempdir().unwrap();
        let path = dir.path().join("G1R-knowledge-peer.sav");
        let private_payload = build_knowledge_map_payload(&["OC_STT_Diego"]);
        let seed_compressed = b"seed-knowledge-peer".to_vec();
        let stream = compressed_stream_with_one_chunk(&seed_compressed, private_payload.len());
        fs::write(
            &path,
            build_gsav(2, &public_payload("Slot A"), &stream, &[0, 0, 0, 0]),
        )
        .unwrap();
        let backend = PrefixCodecBackend {
            seed_compressed,
            seed_uncompressed: private_payload,
        };

        let err = write_save_with_codec_backend(
            &path,
            &[
                json!({
                    "path": "private.knowledge.addCharacter",
                    "value": { "value": "OC_TEST_BrandNew" }
                }),
                json!({
                    "path": "private.typed.setValue",
                    "value": { "path": ["m_MaxQuick"], "value": 9 }
                }),
            ],
            false,
            None,
            Some(&backend),
        )
        .unwrap_err();
        assert!(
            err.to_string().contains("must contain no other edits"),
            "unexpected error: {err}"
        );
        // The error must call out the map-insert case, not only inventory.
        assert!(
            err.to_string().contains("private.knowledge.addCharacter"),
            "error should mention the knowledge op: {err}"
        );
    }

    #[test]
    fn write_save_accepts_knowledge_add_character_alone() {
        // A solo knowledge.addCharacter is accepted and applied: the
        // stand-alone rule only fires when peer edits are present.
        let dir = tempdir().unwrap();
        let path = dir.path().join("G1R-knowledge-solo.sav");
        let output_path = dir.path().join("G1R-knowledge-solo.out.sav");
        let private_payload = build_knowledge_map_payload(&["OC_STT_Diego"]);
        let seed_compressed = b"seed-knowledge-solo".to_vec();
        let stream = compressed_stream_with_one_chunk(&seed_compressed, private_payload.len());
        fs::write(
            &path,
            build_gsav(2, &public_payload("Slot A"), &stream, &[0, 0, 0, 0]),
        )
        .unwrap();
        let backend = PrefixCodecBackend {
            seed_compressed,
            seed_uncompressed: private_payload,
        };

        let response = write_save_with_codec_backend(
            &path,
            &[json!({
                "path": "private.knowledge.addCharacter",
                "value": { "value": "OC_TEST_BrandNew" }
            })],
            false,
            Some(&output_path),
            Some(&backend),
        )
        .unwrap();
        assert_eq!(response["editsApplied"], 1);
    }

    // ── Task 5 helpers ──────────────────────────────────────────────────────

    fn private_enum_property(name: &str, enum_type: &str, label: &str) -> Vec<u8> {
        let body = fstring(label);
        let mut out = fstring(name);
        out.extend_from_slice(&fstring("EnumProperty"));
        out.extend_from_slice(&1u32.to_le_bytes());
        out.extend_from_slice(&fstring(enum_type));
        out.extend_from_slice(&1u32.to_le_bytes());
        out.extend_from_slice(&fstring("/Script/G1R"));
        out.extend_from_slice(&1u32.to_le_bytes());
        out.extend_from_slice(&fstring("ByteProperty"));
        out.extend_from_slice(&0u32.to_le_bytes()); // array_index
        out.extend_from_slice(&(body.len() as u32).to_le_bytes());
        out.push(0); // tag_flags
        out.extend_from_slice(&body);
        out
    }

    fn quest_map_payload() -> Vec<u8> {
        let quest_value = |state: &str| {
            let mut v = private_enum_property("CurrentState", "EQuestState", state);
            v.extend_from_slice(&fstring("None"));
            v
        };
        let mut map_body = 0u32.to_le_bytes().to_vec(); // num_to_remove
        map_body.extend_from_slice(&2u32.to_le_bytes()); // count
        map_body.extend_from_slice(&fstring("/Script/Angelscript.Quest_OldCamp_SLEEPER"));
        map_body.extend_from_slice(&quest_value("EQuestState::Running"));
        map_body.extend_from_slice(&fstring(
            "/Script/Angelscript.Quest_BanditsCamp_BANDITSTRUST",
        ));
        map_body.extend_from_slice(&quest_value("EQuestState::Available"));

        let mut props = fstring("QuestDataByClass");
        props.extend_from_slice(&fstring("MapProperty"));
        props.extend_from_slice(&2u32.to_le_bytes());
        props.extend_from_slice(&fstring("ObjectProperty"));
        props.extend_from_slice(&0u32.to_le_bytes()); // key_flags
        props.extend_from_slice(&fstring("StructProperty"));
        props.extend_from_slice(&1u32.to_le_bytes());
        props.extend_from_slice(&fstring("SingleQuestSaveGameData"));
        props.extend_from_slice(&1u32.to_le_bytes());
        props.extend_from_slice(&fstring("/Script/G1R"));
        props.extend_from_slice(&0u32.to_le_bytes()); // array_index
        props.extend_from_slice(&(map_body.len() as u32).to_le_bytes());
        props.push(0); // tag_flags (struct map values are tagged property lists)
        props.extend_from_slice(&map_body);

        let mut p = fstring("/Script/Angelscript.GothicFinalDataGame");
        p.push(0);
        p.extend_from_slice(&props);
        p.extend_from_slice(&fstring("None"));
        p.extend_from_slice(&0u32.to_le_bytes());
        p
    }

    #[test]
    fn query_progression_lists_quests_with_state_paths() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("G1R-quests.sav");
        let private_payload = quest_map_payload();
        let seed_compressed = b"seed".to_vec();
        let stream = compressed_stream_with_one_chunk(&seed_compressed, private_payload.len());
        fs::write(
            &path,
            build_gsav(2, &public_payload("Slot"), &stream, &[0, 0, 0, 0]),
        )
        .unwrap();
        let backend = PrefixCodecBackend {
            seed_compressed,
            seed_uncompressed: private_payload,
        };

        let value =
            query_progression(&path, &json!({ "section": "quests" }), Some(&backend)).unwrap();
        assert_eq!(value["section"], "quests");
        assert_eq!(value["total"], 2);
        assert_eq!(value["stateCounts"]["Running"], 1);
        assert_eq!(value["stateCounts"]["Available"], 1);
        // Sorted by class path: BanditsCamp before OldCamp.
        let first = &value["quests"][0];
        assert_eq!(
            first["questClass"],
            "/Script/Angelscript.Quest_BanditsCamp_BANDITSTRUST"
        );
        assert_eq!(first["id"], "Quest_BanditsCamp_BANDITSTRUST");
        assert_eq!(first["group"], "BanditsCamp");
        assert_eq!(first["name"], "BANDITSTRUST");
        assert_eq!(first["currentState"], "EQuestState::Available");
        assert_eq!(
            first["statePath"],
            json!([
                "QuestDataByClass",
                "{/Script/Angelscript.Quest_BanditsCamp_BANDITSTRUST}",
                "CurrentState"
            ])
        );

        // Query filter + paging.
        let filtered = query_progression(
            &path,
            &json!({ "section": "quests", "query": "sleeper" }),
            Some(&backend),
        )
        .unwrap();
        assert_eq!(filtered["total"], 1);
        assert_eq!(filtered["quests"][0]["name"], "SLEEPER");

        // The statePath round-trips through the existing setValue write.
        let output_path = dir.path().join("G1R-quests-out.sav");
        let response = write_save_with_codec_backend(
            &path,
            &[json!({
                "path": "private.typed.setValue",
                "value": {
                    "path": [
                        "QuestDataByClass",
                        "{/Script/Angelscript.Quest_BanditsCamp_BANDITSTRUST}",
                        "CurrentState"
                    ],
                    "value": "EQuestState::Succeeded"
                }
            })],
            false,
            Some(&output_path),
            Some(&backend),
        )
        .unwrap();
        assert_eq!(response["editsApplied"], 1);
        let after = query_progression(
            &output_path,
            &json!({ "section": "quests", "query": "banditstrust" }),
            Some(&backend),
        )
        .unwrap();
        assert_eq!(after["quests"][0]["currentState"], "EQuestState::Succeeded");
    }

    #[test]
    fn query_progression_quests_state_and_group_filters() {
        // Fixture: OldCamp_SLEEPER Running, BanditsCamp_BANDITSTRUST Available.
        let dir = tempdir().unwrap();
        let path = dir.path().join("G1R-filter.sav");
        let private_payload = quest_map_payload();
        let seed_compressed = b"seed".to_vec();
        let stream = compressed_stream_with_one_chunk(&seed_compressed, private_payload.len());
        fs::write(
            &path,
            build_gsav(2, &public_payload("Slot"), &stream, &[0, 0, 0, 0]),
        )
        .unwrap();
        let backend = PrefixCodecBackend {
            seed_compressed,
            seed_uncompressed: private_payload,
        };

        // No filters: both facets cover all entries.
        let base =
            query_progression(&path, &json!({ "section": "quests" }), Some(&backend)).unwrap();
        assert_eq!(base["groupCounts"]["BanditsCamp"], 1);
        assert_eq!(base["groupCounts"]["OldCamp"], 1);
        assert_eq!(base["stateCounts"]["Running"], 1);
        assert_eq!(base["stateCounts"]["Available"], 1);

        // state:"Running" → 1 hit (SLEEPER).
        // groupCounts respects query+state (not group), so only Running entries counted.
        // stateCounts respects query+group (no group filter), so all entries counted.
        let running = query_progression(
            &path,
            &json!({ "section": "quests", "state": "Running" }),
            Some(&backend),
        )
        .unwrap();
        assert_eq!(running["total"], 1);
        assert_eq!(running["quests"][0]["name"], "SLEEPER");
        // groupCounts: only Running entries → OldCamp=1, BanditsCamp absent.
        assert_eq!(running["groupCounts"]["OldCamp"], 1);
        assert!(running["groupCounts"]["BanditsCamp"].is_null());
        // stateCounts: no group filter applied → all entries.
        assert_eq!(running["stateCounts"]["Running"], 1);
        assert_eq!(running["stateCounts"]["Available"], 1);

        // state:"EQuestState::Running" (full form) → same result.
        let running_full = query_progression(
            &path,
            &json!({ "section": "quests", "state": "EQuestState::Running" }),
            Some(&backend),
        )
        .unwrap();
        assert_eq!(running_full["total"], 1);
        assert_eq!(running_full["quests"][0]["name"], "SLEEPER");

        // state:"running" (case-insensitive) → same result.
        let running_lower = query_progression(
            &path,
            &json!({ "section": "quests", "state": "running" }),
            Some(&backend),
        )
        .unwrap();
        assert_eq!(running_lower["total"], 1);
        assert_eq!(running_lower["quests"][0]["name"], "SLEEPER");

        // group:"banditscamp" → 1 hit (BANDITSTRUST).
        // stateCounts respects query+group → only BanditsCamp entries → Available=1 only.
        // groupCounts respects query+state (no state filter) → all entries.
        let by_group = query_progression(
            &path,
            &json!({ "section": "quests", "group": "banditscamp" }),
            Some(&backend),
        )
        .unwrap();
        assert_eq!(by_group["total"], 1);
        assert_eq!(by_group["quests"][0]["name"], "BANDITSTRUST");
        assert_eq!(by_group["stateCounts"]["Available"], 1);
        assert!(by_group["stateCounts"]["Running"].is_null());
        assert_eq!(by_group["groupCounts"]["BanditsCamp"], 1);
        assert_eq!(by_group["groupCounts"]["OldCamp"], 1);

        // Combined state=Running + group=BanditsCamp → 0 results (Running quest is in OldCamp).
        // stateCounts: query+group(BanditsCamp) → only BanditsCamp entry → Available=1.
        // groupCounts: query+state(Running) → only Running entry → OldCamp=1.
        let no_match = query_progression(
            &path,
            &json!({ "section": "quests", "state": "Running", "group": "BanditsCamp" }),
            Some(&backend),
        )
        .unwrap();
        assert_eq!(no_match["total"], 0);
        // stateCounts sees only BanditsCamp entries (Available).
        assert_eq!(no_match["stateCounts"]["Available"], 1);
        assert!(no_match["stateCounts"]["Running"].is_null());
        // groupCounts sees only Running entries (OldCamp).
        assert_eq!(no_match["groupCounts"]["OldCamp"], 1);
        assert!(no_match["groupCounts"]["BanditsCamp"].is_null());
    }

    // ── Task 6 helpers ──────────────────────────────────────────────────────

    fn private_double_property(name: &str, value: f64) -> Vec<u8> {
        let mut out = fstring(name);
        out.extend_from_slice(&fstring("DoubleProperty"));
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&8u32.to_le_bytes());
        out.push(0);
        out.extend_from_slice(&value.to_le_bytes());
        out
    }

    fn private_struct_property(name: &str, struct_type: &str, body: &[u8], flags: u8) -> Vec<u8> {
        let mut out = fstring(name);
        out.extend_from_slice(&fstring("StructProperty"));
        out.extend_from_slice(&1u32.to_le_bytes());
        out.extend_from_slice(&fstring(struct_type));
        out.extend_from_slice(&1u32.to_le_bytes());
        out.extend_from_slice(&fstring("/Script/G1R"));
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&(body.len() as u32).to_le_bytes());
        out.push(flags);
        out.extend_from_slice(body);
        out
    }

    fn name_keyed_struct_map(map_name: &str, entries: &[(&str, Vec<u8>)]) -> Vec<u8> {
        let mut map_body = 0u32.to_le_bytes().to_vec();
        map_body.extend_from_slice(&(entries.len() as u32).to_le_bytes());
        for (key, value_props) in entries {
            map_body.extend_from_slice(&fstring(key));
            map_body.extend_from_slice(value_props);
            map_body.extend_from_slice(&fstring("None"));
        }
        let mut out = fstring(map_name);
        out.extend_from_slice(&fstring("MapProperty"));
        out.extend_from_slice(&2u32.to_le_bytes());
        out.extend_from_slice(&fstring("NameProperty"));
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&fstring("StructProperty"));
        out.extend_from_slice(&1u32.to_le_bytes());
        out.extend_from_slice(&fstring("KnowledgeSet"));
        out.extend_from_slice(&1u32.to_le_bytes());
        out.extend_from_slice(&fstring("/Script/G1R"));
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&(map_body.len() as u32).to_le_bytes());
        out.push(0);
        out.extend_from_slice(&map_body);
        out
    }

    #[test]
    fn query_progression_knowledge_lists_characters_and_entries() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("G1R-knowledge.sav");
        let private_payload = {
            let diego = private_name_set_property(
                "Knowledge",
                &[
                    "Voiceline_info_diego_gamestart_11_00",
                    "ChoiceDiegoGamestart",
                ],
            );
            let xardas = private_name_set_property("Knowledge", &["Voiceline_xardas_intro"]);
            let map = name_keyed_struct_map(
                "CharacterKnowledgeByUniqueName",
                &[("OC_STT_Diego", diego), ("NoneCamp_Xardas", xardas)],
            );
            let mut p = fstring("/Script/Angelscript.GothicFinalDataGame");
            p.push(0);
            p.extend_from_slice(&map);
            p.extend_from_slice(&fstring("None"));
            p.extend_from_slice(&0u32.to_le_bytes());
            p
        };
        let seed_compressed = b"seed".to_vec();
        let stream = compressed_stream_with_one_chunk(&seed_compressed, private_payload.len());
        fs::write(
            &path,
            build_gsav(2, &public_payload("Slot"), &stream, &[0, 0, 0, 0]),
        )
        .unwrap();
        let backend = PrefixCodecBackend {
            seed_compressed,
            seed_uncompressed: private_payload,
        };

        // Character list (no `character` param).
        let value =
            query_progression(&path, &json!({ "section": "knowledge" }), Some(&backend)).unwrap();
        assert_eq!(value["total"], 2);
        // Sorted by name: NoneCamp_Xardas before OC_STT_Diego.
        assert_eq!(value["characters"][0]["name"], "NoneCamp_Xardas");
        assert_eq!(value["characters"][0]["entryCount"], 1);
        assert_eq!(value["characters"][1]["name"], "OC_STT_Diego");
        assert_eq!(value["characters"][1]["entryCount"], 2);

        // Entries for one character.
        let value = query_progression(
            &path,
            &json!({ "section": "knowledge", "character": "OC_STT_Diego" }),
            Some(&backend),
        )
        .unwrap();
        assert_eq!(value["character"], "OC_STT_Diego");
        assert_eq!(value["total"], 2);
        assert_eq!(
            value["setPath"],
            json!([
                "CharacterKnowledgeByUniqueName",
                "{OC_STT_Diego}",
                "Knowledge"
            ])
        );
        let entries = value["entries"].as_array().unwrap();
        assert!(entries.contains(&json!("ChoiceDiegoGamestart")));

        // Query filter on entries.
        let value = query_progression(
            &path,
            &json!({ "section": "knowledge", "character": "OC_STT_Diego", "query": "choice" }),
            Some(&backend),
        )
        .unwrap();
        assert_eq!(value["total"], 1);

        // Unknown character errors.
        assert!(
            query_progression(
                &path,
                &json!({ "section": "knowledge", "character": "Nobody" }),
                Some(&backend),
            )
            .is_err()
        );
    }

    #[test]
    fn query_progression_events_lists_characters_and_events() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("G1R-events.sav");
        let private_payload = {
            // One memory event struct: EventTags (native GameplayTagContainer)
            // + Time (InGameTime property list) + AffectedCharacterGlobalId.
            let event = {
                let mut tags_body = 1u32.to_le_bytes().to_vec();
                tags_body.extend_from_slice(&fstring("Memory.Quest.Started"));
                let mut e = private_struct_property(
                    "EventTags",
                    "GameplayTagContainer",
                    &tags_body,
                    properties::TAG_FLAG_NATIVE_SERIALIZE,
                );
                let time_body = {
                    let mut t = private_double_property("TotalSeconds", 1234.5);
                    t.extend_from_slice(&fstring("None"));
                    t
                };
                e.extend_from_slice(&private_struct_property(
                    "Time",
                    "InGameTime",
                    &time_body,
                    0,
                ));
                let mut affected = fstring("AffectedCharacterGlobalId");
                affected.extend_from_slice(&fstring("NameProperty"));
                affected.extend_from_slice(&0u32.to_le_bytes());
                let hero = fstring("Hero");
                affected.extend_from_slice(&(hero.len() as u32).to_le_bytes());
                affected.push(0);
                affected.extend_from_slice(&hero);
                e.extend_from_slice(&affected);
                e
            };
            // MemorizedEvents: ArrayProperty of MemoryEvent structs (inline
            // tagged property lists, "None"-terminated).
            let memorized = {
                let mut element = event.clone();
                element.extend_from_slice(&fstring("None"));
                let mut body = 1u32.to_le_bytes().to_vec();
                body.extend_from_slice(&element);
                let mut out = fstring("MemorizedEvents");
                out.extend_from_slice(&fstring("ArrayProperty"));
                out.extend_from_slice(&1u32.to_le_bytes());
                out.extend_from_slice(&fstring("StructProperty"));
                out.extend_from_slice(&1u32.to_le_bytes());
                out.extend_from_slice(&fstring("MemoryEvent"));
                out.extend_from_slice(&1u32.to_le_bytes());
                out.extend_from_slice(&fstring("/Script/G1R"));
                out.extend_from_slice(&0u32.to_le_bytes());
                out.extend_from_slice(&(body.len() as u32).to_le_bytes());
                out.push(0);
                out.extend_from_slice(&body);
                out
            };
            let map = name_keyed_struct_map("LongTermMemoryByGlobalId", &[("Hero", memorized)]);
            let mut p = fstring("/Script/Angelscript.GothicFinalDataGame");
            p.push(0);
            p.extend_from_slice(&map);
            p.extend_from_slice(&fstring("None"));
            p.extend_from_slice(&0u32.to_le_bytes());
            p
        };
        let seed_compressed = b"seed".to_vec();
        let stream = compressed_stream_with_one_chunk(&seed_compressed, private_payload.len());
        fs::write(
            &path,
            build_gsav(2, &public_payload("Slot"), &stream, &[0, 0, 0, 0]),
        )
        .unwrap();
        let backend = PrefixCodecBackend {
            seed_compressed,
            seed_uncompressed: private_payload,
        };

        let value =
            query_progression(&path, &json!({ "section": "events" }), Some(&backend)).unwrap();
        assert_eq!(value["total"], 1);
        assert_eq!(value["characters"][0]["id"], "Hero");
        assert_eq!(value["characters"][0]["eventCount"], 1);

        let value = query_progression(
            &path,
            &json!({ "section": "events", "character": "Hero" }),
            Some(&backend),
        )
        .unwrap();
        assert_eq!(value["character"], "Hero");
        assert_eq!(value["total"], 1);
        assert_eq!(
            value["arrayPath"],
            json!(["LongTermMemoryByGlobalId", "{Hero}", "MemorizedEvents"])
        );
        let event = &value["events"][0];
        assert_eq!(event["index"], 0);
        assert_eq!(event["tags"], json!(["Memory.Quest.Started"]));
        assert_eq!(event["timeSeconds"], 1234.5);
        assert_eq!(event["affected"], "Hero");

        // Tag query filter.
        let filtered = query_progression(
            &path,
            &json!({ "section": "events", "character": "Hero", "query": "guild" }),
            Some(&backend),
        )
        .unwrap();
        assert_eq!(filtered["total"], 0);
    }

    #[test]
    fn inspect_reports_structured_progression_overview() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("G1R-overview.sav");
        let private_payload = quest_map_payload();
        let seed_compressed = b"seed".to_vec();
        let stream = compressed_stream_with_one_chunk(&seed_compressed, private_payload.len());
        fs::write(
            &path,
            build_gsav(2, &public_payload("Slot"), &stream, &[0, 0, 0, 0]),
        )
        .unwrap();
        let backend = PrefixCodecBackend {
            seed_compressed,
            seed_uncompressed: private_payload,
        };

        let value = inspect_save_with_codec_backend(&path, true, Some(&backend), None).unwrap();
        let progression = &value["private"]["progression"];
        assert_eq!(progression["status"], "ok");
        assert_eq!(progression["questTotal"], 2);
        assert_eq!(progression["questStates"]["Running"], 1);
        assert_eq!(progression["questStates"]["Available"], 1);
        // No knowledge/memory maps in this fixture.
        assert_eq!(progression["knowledgeCharacters"], 0);
        assert_eq!(progression["memoryCharacters"], 0);
        assert!(
            progression["writable"]
                .as_array()
                .unwrap()
                .contains(&json!("private.typed.setValue"))
        );
        // The knowledge map is editable via add-character; the progression
        // summary advertises it whenever the typed parse succeeds.
        assert!(
            progression["writable"]
                .as_array()
                .unwrap()
                .contains(&json!("private.knowledge.addCharacter"))
        );
        // The old heuristic fields are gone.
        assert!(progression.get("candidates").is_none());
        assert!(progression.get("sections").is_none());
    }

    // ── private.inventory.addItem parse tests ───────────────────────────────

    /// Build a minimal inventory private payload with one item at a valid
    /// player-inventory-region path, so that the save can be accepted by the
    /// codec backend.  Used by the structural-limit tests (5 and 6) which need
    /// a real save file on disk.
    fn inventory_payload_for_add_item_tests() -> Vec<u8> {
        [
            fstring("m_ItemDefinition"),
            fstring("ObjectProperty"),
            fstring("/Script/Angelscript.ItMi_Orenugget"),
            int_property("m_ItemCount", 1),
            fstring("m_MapOfAttachedItems"),
            fstring("MapProperty"),
        ]
        .concat()
    }

    // ── typed inventory fixture (Task 7: addItem payload mechanics) ─────────
    //
    // Synthetic payload shaped exactly like the verified real structure:
    // m_GenericData{PlayersSavedData}.m_SavedPlayers[0].m_Inventory with
    // m_Keys (Array<Enum EInventoryTypes>) parallel to m_Values.Items
    // (Array<Struct ContainerVirtualData>), each container holding a plain
    // ArrayProperty<StructProperty ItemVirtualData> m_Slots. The strict
    // byte-accounting parser is the referee: parse_private_root must accept
    // it and resolve_chain/container_layout must resolve like on real saves.

    const INV_MAIN_LABEL: &str = "EInventoryTypes::MainContainer";
    const INV_OTHER_LABEL: &str = "EInventoryTypes::Quickslots";

    /// Tagged property: name | type | descriptor | array_index | size | flags | body.
    fn inv_tagged(
        name: &str,
        type_name: &str,
        descriptor: &[u8],
        flags: u8,
        body: &[u8],
    ) -> Vec<u8> {
        let mut out = fstring(name);
        out.extend_from_slice(&fstring(type_name));
        out.extend_from_slice(descriptor);
        out.extend_from_slice(&0u32.to_le_bytes()); // array_index
        out.extend_from_slice(&(body.len() as u32).to_le_bytes());
        out.push(flags);
        out.extend_from_slice(body);
        out
    }

    fn inv_struct_descriptor(struct_type: &str) -> Vec<u8> {
        let mut out = 1u32.to_le_bytes().to_vec();
        out.extend_from_slice(&fstring(struct_type));
        out.extend_from_slice(&1u32.to_le_bytes());
        out.extend_from_slice(&fstring("/Script/G1R"));
        out
    }

    fn inv_enum_descriptor() -> Vec<u8> {
        let mut out = 1u32.to_le_bytes().to_vec();
        out.extend_from_slice(&fstring("EInventoryTypes"));
        out.extend_from_slice(&1u32.to_le_bytes());
        out.extend_from_slice(&fstring("/Script/G1R"));
        out.extend_from_slice(&1u32.to_le_bytes());
        out.extend_from_slice(&fstring("ByteProperty"));
        out
    }

    /// StructProperty whose value is a "None"-terminated property list.
    fn inv_struct_property(name: &str, struct_type: &str, props: &[u8]) -> Vec<u8> {
        let mut body = props.to_vec();
        body.extend_from_slice(&fstring("None"));
        inv_tagged(
            name,
            "StructProperty",
            &inv_struct_descriptor(struct_type),
            0,
            &body,
        )
    }

    fn inv_enum_property(name: &str, label: &str) -> Vec<u8> {
        inv_tagged(
            name,
            "EnumProperty",
            &inv_enum_descriptor(),
            0,
            &fstring(label),
        )
    }

    fn inv_object_property(name: &str, path: &str) -> Vec<u8> {
        inv_tagged(name, "ObjectProperty", &[], 0, &fstring(path))
    }

    /// ArrayProperty<StructProperty struct_type>; each element is a
    /// "None"-terminated property list.
    fn inv_struct_array_property(name: &str, struct_type: &str, elements: &[Vec<u8>]) -> Vec<u8> {
        let mut descriptor = 1u32.to_le_bytes().to_vec();
        descriptor.extend_from_slice(&fstring("StructProperty"));
        descriptor.extend_from_slice(&inv_struct_descriptor(struct_type));
        let mut body = (elements.len() as u32).to_le_bytes().to_vec();
        for element in elements {
            body.extend_from_slice(element);
            body.extend_from_slice(&fstring("None"));
        }
        inv_tagged(name, "ArrayProperty", &descriptor, 0, &body)
    }

    /// ArrayProperty<EnumProperty EInventoryTypes>.
    fn inv_enum_array_property(name: &str, labels: &[&str]) -> Vec<u8> {
        let mut descriptor = 1u32.to_le_bytes().to_vec();
        descriptor.extend_from_slice(&fstring("EnumProperty"));
        descriptor.extend_from_slice(&inv_enum_descriptor());
        let mut body = (labels.len() as u32).to_le_bytes().to_vec();
        for label in labels {
            body.extend_from_slice(&fstring(label));
        }
        inv_tagged(name, "ArrayProperty", &descriptor, 0, &body)
    }

    /// Empty generic-data map mirroring an ordinary item's ItemPayload content.
    fn inv_empty_payload_map() -> Vec<u8> {
        let mut descriptor = 2u32.to_le_bytes().to_vec();
        descriptor.extend_from_slice(&fstring("NameProperty"));
        descriptor.extend_from_slice(&0u32.to_le_bytes()); // key_flags
        descriptor.extend_from_slice(&fstring("StrProperty"));
        let mut body = 0u32.to_le_bytes().to_vec(); // num_to_remove
        body.extend_from_slice(&0u32.to_le_bytes()); // count
        inv_tagged("m_GenericData", "MapProperty", &descriptor, 0, &body)
    }

    /// Same map with one entry: item-specific state the edit must refuse to clone.
    fn inv_nonempty_payload_map() -> Vec<u8> {
        let mut descriptor = 2u32.to_le_bytes().to_vec();
        descriptor.extend_from_slice(&fstring("NameProperty"));
        descriptor.extend_from_slice(&0u32.to_le_bytes()); // key_flags
        descriptor.extend_from_slice(&fstring("StrProperty"));
        let mut body = 0u32.to_le_bytes().to_vec(); // num_to_remove
        body.extend_from_slice(&1u32.to_le_bytes()); // count
        body.extend_from_slice(&fstring("Durability"));
        body.extend_from_slice(&fstring("42"));
        inv_tagged("m_GenericData", "MapProperty", &descriptor, 0, &body)
    }

    /// One ItemVirtualData slot property list (terminating "None" is appended
    /// by the array builder).
    fn inv_item_slot(
        id: i32,
        inventory_type: &str,
        item_path: &str,
        count: i32,
        payload_props: &[u8],
    ) -> Vec<u8> {
        let mut slot_data = inv_object_property("m_ItemDefinition", item_path);
        slot_data.extend_from_slice(&int_property("m_ItemCount", count));
        let mut out = int_property("m_Id", id);
        out.extend_from_slice(&inv_enum_property("m_InventoryType", inventory_type));
        out.extend_from_slice(&inv_struct_property("m_SlotData", "ItemSlot", &slot_data));
        out.extend_from_slice(&inv_struct_property(
            "m_Payload",
            "ItemPayload",
            payload_props,
        ));
        out
    }

    /// ContainerVirtualData property list (element of m_Values.Items).
    fn inv_container(inventory_type: &str, slots: &[Vec<u8>]) -> Vec<u8> {
        let mut out = inv_struct_array_property("m_Slots", "ItemVirtualData", slots);
        out.extend_from_slice(&inv_enum_property("m_InventoryType", inventory_type));
        out.extend_from_slice(&int_property("m_Capacity", -1));
        out
    }

    /// Full private payload. MainContainer is deliberately NOT at index 0 in
    /// m_Keys/Items so the implementation must match by enum value.
    fn typed_inventory_private_payload(other_slots: &[Vec<u8>], main_slots: &[Vec<u8>]) -> Vec<u8> {
        let keys = inv_enum_array_property("m_Keys", &[INV_OTHER_LABEL, INV_MAIN_LABEL]);
        let items = inv_struct_array_property(
            "Items",
            "ContainerVirtualData",
            &[
                inv_container(INV_OTHER_LABEL, other_slots),
                inv_container(INV_MAIN_LABEL, main_slots),
            ],
        );
        let values = inv_struct_property("m_Values", "ContainerVirtualDataArray", &items);
        let mut inventory_props = keys;
        inventory_props.extend_from_slice(&values);
        let inventory =
            inv_struct_property("m_Inventory", "ReplicatedInventoryMap", &inventory_props);

        let saved_players =
            inv_struct_array_property("m_SavedPlayers", "PlayerSavedData", &[inventory]);
        let mut instanced_body = saved_players;
        instanced_body.extend_from_slice(&fstring("None"));
        let mut instanced = fstring("/Script/Angelscript.PlayersSavedData");
        instanced.extend_from_slice(&(instanced_body.len() as u32).to_le_bytes());
        instanced.extend_from_slice(&instanced_body);

        let mut map_body = 0u32.to_le_bytes().to_vec(); // num_to_remove
        map_body.extend_from_slice(&1u32.to_le_bytes()); // count
        map_body.extend_from_slice(&fstring("PlayersSavedData"));
        map_body.extend_from_slice(&instanced);
        let mut map_descriptor = 2u32.to_le_bytes().to_vec();
        map_descriptor.extend_from_slice(&fstring("NameProperty"));
        map_descriptor.extend_from_slice(&0u32.to_le_bytes()); // key_flags
        map_descriptor.extend_from_slice(&fstring("StructProperty"));
        map_descriptor.extend_from_slice(&inv_struct_descriptor("InstancedStruct"));
        let generic = inv_tagged(
            "m_GenericData",
            "MapProperty",
            &map_descriptor,
            0,
            &map_body,
        );

        let mut p = fstring("/Script/Angelscript.GothicFinalDataGame");
        p.push(0);
        p.extend_from_slice(&generic);
        p.extend_from_slice(&fstring("None"));
        p.extend_from_slice(&0u32.to_le_bytes()); // footer
        p
    }

    /// A single-MainContainer m_Inventory struct holding one item, as a child
    /// property of a saved-player entry.
    fn inventory_struct_with_item(item_path: &str) -> Vec<u8> {
        let main_slots = vec![inv_item_slot(
            0,
            INV_MAIN_LABEL,
            item_path,
            1,
            &inv_empty_payload_map(),
        )];
        let keys = inv_enum_array_property("m_Keys", &[INV_MAIN_LABEL]);
        let items = inv_struct_array_property(
            "Items",
            "ContainerVirtualData",
            &[inv_container(INV_MAIN_LABEL, &main_slots)],
        );
        let values = inv_struct_property("m_Values", "ContainerVirtualDataArray", &items);
        let mut inventory_props = keys;
        inventory_props.extend_from_slice(&values);
        inv_struct_property("m_Inventory", "ReplicatedInventoryMap", &inventory_props)
    }

    /// Private payload with two saved players. The controlled player (Party
    /// ID 0) is the SECOND m_SavedPlayers element, so a first-match lookup would
    /// wrongly target the first (Party ID 1) player's inventory.
    fn two_saved_players_private_payload(party1_item: &str, party0_item: &str) -> Vec<u8> {
        let mut player1 = str_property("m_PlayerID", "Party ID 1");
        player1.extend_from_slice(&inventory_struct_with_item(party1_item));
        let mut player0 = str_property("m_PlayerID", "Party ID 0");
        player0.extend_from_slice(&inventory_struct_with_item(party0_item));

        let saved_players =
            inv_struct_array_property("m_SavedPlayers", "PlayerSavedData", &[player1, player0]);
        let mut instanced_body = saved_players;
        instanced_body.extend_from_slice(&fstring("None"));
        let mut instanced = fstring("/Script/Angelscript.PlayersSavedData");
        instanced.extend_from_slice(&(instanced_body.len() as u32).to_le_bytes());
        instanced.extend_from_slice(&instanced_body);

        let mut map_body = 0u32.to_le_bytes().to_vec(); // num_to_remove
        map_body.extend_from_slice(&1u32.to_le_bytes()); // count
        map_body.extend_from_slice(&fstring("PlayersSavedData"));
        map_body.extend_from_slice(&instanced);
        let mut map_descriptor = 2u32.to_le_bytes().to_vec();
        map_descriptor.extend_from_slice(&fstring("NameProperty"));
        map_descriptor.extend_from_slice(&0u32.to_le_bytes()); // key_flags
        map_descriptor.extend_from_slice(&fstring("StructProperty"));
        map_descriptor.extend_from_slice(&inv_struct_descriptor("InstancedStruct"));
        let generic = inv_tagged(
            "m_GenericData",
            "MapProperty",
            &map_descriptor,
            0,
            &map_body,
        );

        let mut p = fstring("/Script/Angelscript.GothicFinalDataGame");
        p.push(0);
        p.extend_from_slice(&generic);
        p.extend_from_slice(&fstring("None"));
        p.extend_from_slice(&0u32.to_le_bytes()); // footer
        p
    }

    #[test]
    fn main_container_summary_targets_controlled_player_not_first() {
        let party1_item = "/Script/Angelscript.ItMi_Sulfur";
        let party0_item = "/Script/Angelscript.ItMi_Orenugget";
        let payload = two_saved_players_private_payload(party1_item, party0_item);
        let root = properties::parse_private_root(&payload).unwrap();
        let summary = main_container_summary(&root).expect("player inventory resolves");
        assert!(
            summary.all_paths.contains(party0_item),
            "summary must target the Party ID 0 player's inventory"
        );
        assert!(
            !summary.all_paths.contains(party1_item),
            "summary must NOT target the first (Party ID 1) player's inventory"
        );
    }

    #[test]
    fn inspect_displays_controlled_player_inventory_rows() {
        // The displayed/count-edited rows must come from the same player the
        // add/remove ops target (Party ID 0), even when it is not the first
        // m_SavedPlayers entry.
        let party1_item = "/Script/Angelscript.ItMi_Sulfur";
        let party0_item = "/Script/Angelscript.ItMi_Orenugget";
        let dir = tempdir().unwrap();
        let path = dir.path().join("G1R-2p.sav");
        let private_payload = two_saved_players_private_payload(party1_item, party0_item);
        let seed_compressed = b"seed-2p".to_vec();
        let stream = compressed_stream_with_one_chunk(&seed_compressed, private_payload.len());
        fs::write(
            &path,
            build_gsav(2, &public_payload("Slot A"), &stream, &[0, 0, 0, 0]),
        )
        .unwrap();
        let backend = PrefixCodecBackend {
            seed_compressed,
            seed_uncompressed: private_payload,
        };

        let value = inspect_save_with_codec_backend(&path, true, Some(&backend), None).unwrap();
        let items = value["private"]["inventory"]["items"].as_array().unwrap();
        let paths: Vec<&str> = items.iter().filter_map(|it| it["path"].as_str()).collect();
        assert!(
            paths.contains(&party0_item),
            "displayed rows must include the controlled player's item, got {paths:?}"
        );
        assert!(
            !paths.contains(&party1_item),
            "displayed rows must not include the other player's item, got {paths:?}"
        );
    }

    /// MainContainer with two ordinary items: ItMi_Orenugget (id 0, count 3)
    /// and ItFo_Apple (id 1, count 1; the template as last element).
    fn default_main_slots() -> Vec<Vec<u8>> {
        vec![
            inv_item_slot(
                0,
                INV_MAIN_LABEL,
                "/Script/Angelscript.ItMi_Orenugget",
                3,
                &inv_empty_payload_map(),
            ),
            inv_item_slot(
                1,
                INV_MAIN_LABEL,
                "/Script/Angelscript.ItFo_Apple",
                1,
                &inv_empty_payload_map(),
            ),
        ]
    }

    fn inv_resolve<'a>(
        root: &'a properties::RootObject,
        segments: &[&str],
    ) -> &'a properties::Property {
        let path: Vec<String> = segments.iter().map(|s| s.to_string()).collect();
        let segs = properties::parse_path(&path).unwrap();
        properties::resolve(&root.properties, &segs).unwrap()
    }

    /// Path prefix to a container's m_Slots in the fixture ([0] = other, [1] = main).
    fn inv_slots_prefix(container_index: usize) -> Vec<String> {
        [
            "m_GenericData",
            "{PlayersSavedData}",
            "m_SavedPlayers",
            "[0]",
            "m_Inventory",
            "m_Values",
            "Items",
            &format!("[{container_index}]"),
            "m_Slots",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect()
    }

    fn inv_slot_count(payload: &[u8], container_index: usize) -> usize {
        let root = properties::parse_private_root(payload).unwrap();
        let path = inv_slots_prefix(container_index);
        let segs = properties::parse_path(&path).unwrap();
        let prop = properties::resolve(&root.properties, &segs).unwrap();
        match &prop.value {
            properties::PropertyValue::Array { elements } => elements.len(),
            other => panic!("m_Slots is not an array: {other:?}"),
        }
    }

    #[test]
    fn typed_inventory_fixture_resolves_like_real_saves() {
        // The fixture must satisfy the same invariants the prior investigation
        // verified on real saves: strict parse, resolve_chain to m_Slots with
        // 6 enclosing size fields, container_layout Ok (Array of StructProperty).
        let payload = typed_inventory_private_payload(&[], &default_main_slots());
        let root = properties::parse_private_root(&payload).unwrap();
        let path = inv_slots_prefix(1);
        let segs = properties::parse_path(&path).unwrap();
        let chain = properties::resolve_chain(&root.properties, &segs).unwrap();
        assert_eq!(chain.enclosing_size_fields.len(), 6);
        let layout = properties::container_layout(&payload, chain.target).unwrap();
        assert_eq!(layout.kind, properties::ContainerKind::Array);
        assert_eq!(layout.inner_type, "StructProperty");
        assert_eq!(layout.count, 2);
    }

    #[test]
    fn payload_carries_state_flags_direct_scalar_fields() {
        // Clean: payload is only an empty m_GenericData map.
        let clean_slot = inv_item_slot(
            0,
            INV_MAIN_LABEL,
            "/Script/Angelscript.ItMi_Orenugget",
            1,
            &inv_empty_payload_map(),
        );
        // Stateful: an item-specific scalar stored directly in the payload (no
        // non-empty container), which the old container-only check missed.
        let mut scalar_payload = inv_empty_payload_map();
        scalar_payload.extend_from_slice(&int_property("Durability", 50));
        let scalar_slot = inv_item_slot(
            1,
            INV_MAIN_LABEL,
            "/Script/Angelscript.ItMi_Orenugget",
            1,
            &scalar_payload,
        );

        let payload = typed_inventory_private_payload(&[], &[clean_slot, scalar_slot]);
        let root = properties::parse_private_root(&payload).unwrap();
        let slots_segs = properties::parse_path(&inv_slots_prefix(1)).unwrap();
        let slots = properties::resolve(&root.properties, &slots_segs).unwrap();
        let properties::PropertyValue::Array { elements } = &slots.value else {
            panic!("m_Slots is not an array");
        };
        let payload_prop = |slot| struct_element_property(slot, "m_Payload").unwrap();
        assert!(
            !property_carries_state(payload_prop(&elements[0])),
            "an empty-map payload must be treated as a clean template"
        );
        assert!(
            property_carries_state(payload_prop(&elements[1])),
            "a payload with a direct scalar field must be treated as state"
        );
    }

    #[test]
    fn payload_carries_state_flags_zero_valued_unknown_scalar() {
        // An unrecognised direct scalar at the type default (Durability=0) must
        // still read as state: its real default is unknown, so cloning the slot
        // would seed an unrelated new item with a stale field. Only the known
        // default ItemPayload scalars may be default-clean.
        let mut zero_scalar = inv_empty_payload_map();
        zero_scalar.extend_from_slice(&int_property("Durability", 0));
        let slot = inv_item_slot(
            0,
            INV_MAIN_LABEL,
            "/Script/Angelscript.ItMi_Orenugget",
            1,
            &zero_scalar,
        );
        let payload = typed_inventory_private_payload(&[], &[slot]);
        let root = properties::parse_private_root(&payload).unwrap();
        let slots = properties::resolve(
            &root.properties,
            &properties::parse_path(&inv_slots_prefix(1)).unwrap(),
        )
        .unwrap();
        let properties::PropertyValue::Array { elements } = &slots.value else {
            panic!("m_Slots is not an array");
        };
        assert!(
            property_carries_state(struct_element_property(&elements[0], "m_Payload").unwrap()),
            "a zero-valued unknown scalar must be treated as state"
        );
    }

    #[test]
    fn add_item_appends_slot_to_main_container() {
        // Longer path than the template's (length change upward).
        let mut payload = typed_inventory_private_payload(&[], &default_main_slots());
        let edit = PrivateInventoryAddItemEdit {
            path: "/Script/Angelscript.ItMi_Sulfur".to_string(),
            count: 7,
        };
        apply_private_inventory_add_item_to_payload(&mut payload, &edit).unwrap();

        let root = properties::parse_private_root(&payload).unwrap();
        let prefix = inv_slots_prefix(1);
        let seg = |suffix: &[&'static str]| -> Vec<&str> {
            let mut v: Vec<&str> = prefix.iter().map(String::as_str).collect();
            v.extend_from_slice(suffix);
            v
        };
        let id = inv_resolve(&root, &seg(&["[2]", "m_Id"]));
        assert_eq!(id.value, properties::PropertyValue::Int(2));
        let definition = inv_resolve(&root, &seg(&["[2]", "m_SlotData", "m_ItemDefinition"]));
        assert_eq!(
            definition.value,
            properties::PropertyValue::Object("/Script/Angelscript.ItMi_Sulfur".to_string())
        );
        let count = inv_resolve(&root, &seg(&["[2]", "m_SlotData", "m_ItemCount"]));
        assert_eq!(count.value, properties::PropertyValue::Int(7));
        // Existing slots untouched.
        let first = inv_resolve(&root, &seg(&["[0]", "m_SlotData", "m_ItemCount"]));
        assert_eq!(first.value, properties::PropertyValue::Int(3));

        // The inventory summary scan sees the new item with the requested count.
        let refs = scan_fstrings(&payload, 0);
        let (items, total, scope) = summarize_private_inventory_items(&payload, &refs, 200);
        assert_eq!(scope, "player_inventory_region");
        assert_eq!(total, 3);
        assert!(
            items.iter().any(|item| item["path"]
                == "/Script/Angelscript.ItMi_Sulfur"
                && item["count"] == 7),
            "summary missing new item: {items:?}"
        );
    }

    #[test]
    fn add_item_handles_shorter_item_path() {
        // Shorter path than the template's (length change downward).
        let mut payload = typed_inventory_private_payload(&[], &default_main_slots());
        let edit = PrivateInventoryAddItemEdit {
            path: "/Script/Angelscript.ItFo_Egg".to_string(),
            count: 2,
        };
        apply_private_inventory_add_item_to_payload(&mut payload, &edit).unwrap();

        let root = properties::parse_private_root(&payload).unwrap();
        let prefix = inv_slots_prefix(1);
        let mut def_path: Vec<&str> = prefix.iter().map(String::as_str).collect();
        def_path.extend_from_slice(&["[2]", "m_SlotData", "m_ItemDefinition"]);
        let definition = inv_resolve(&root, &def_path);
        assert_eq!(
            definition.value,
            properties::PropertyValue::Object("/Script/Angelscript.ItFo_Egg".to_string())
        );
        let mut id_path: Vec<&str> = prefix.iter().map(String::as_str).collect();
        id_path.extend_from_slice(&["[2]", "m_Id"]);
        assert_eq!(
            inv_resolve(&root, &id_path).value,
            properties::PropertyValue::Int(2)
        );
        let refs = scan_fstrings(&payload, 0);
        let (items, _, _) = summarize_private_inventory_items(&payload, &refs, 200);
        assert!(
            items
                .iter()
                .any(|item| item["path"] == "/Script/Angelscript.ItFo_Egg" && item["count"] == 2)
        );
    }

    #[test]
    fn add_item_rejects_existing_main_container_path() {
        let mut payload = typed_inventory_private_payload(&[], &default_main_slots());
        let before = payload.clone();
        let edit = PrivateInventoryAddItemEdit {
            path: "/Script/Angelscript.ItMi_Orenugget".to_string(),
            count: 5,
        };
        let err = apply_private_inventory_add_item_to_payload(&mut payload, &edit).unwrap_err();
        assert!(
            err.to_string().contains("already"),
            "unexpected error: {err}"
        );
        assert_eq!(payload, before, "failed edit must not mutate the payload");
    }

    #[test]
    fn add_item_rejects_when_entire_inventory_empty() {
        // No container has a slot, so there is no template to borrow.
        let mut payload = typed_inventory_private_payload(&[], &[]);
        let before = payload.clone();
        let edit = PrivateInventoryAddItemEdit {
            path: "/Script/Angelscript.ItMi_Sulfur".to_string(),
            count: 1,
        };
        let err = apply_private_inventory_add_item_to_payload(&mut payload, &edit).unwrap_err();
        assert!(err.to_string().contains("clean"), "unexpected error: {err}");
        assert_eq!(payload, before, "failed edit must not mutate the payload");
    }

    #[test]
    fn add_item_seeds_empty_main_container_from_another_container() {
        // MainContainer is empty but Quickslots has a slot: addItem borrows it
        // as a template, appends to MainContainer, and fixes m_InventoryType.
        let other_slots = vec![inv_item_slot(
            5,
            INV_OTHER_LABEL,
            "/Script/Angelscript.ItMi_Sulfur",
            1,
            &inv_empty_payload_map(),
        )];
        let mut payload = typed_inventory_private_payload(&other_slots, &[]);
        assert_eq!(inv_slot_count(&payload, 1), 0, "MainContainer starts empty");
        let edit = PrivateInventoryAddItemEdit {
            path: "/Script/Angelscript.ItMi_Orenugget".to_string(),
            count: 9,
        };
        apply_private_inventory_add_item_to_payload(&mut payload, &edit).unwrap();

        // MainContainer now has the new item; the donor container is untouched.
        assert_eq!(inv_slot_count(&payload, 1), 1);
        assert_eq!(inv_slot_count(&payload, 0), 1);
        let root = properties::parse_private_root(&payload).unwrap();
        let prefix = inv_slots_prefix(1);
        let resolve_main = |leaf: &[&str]| {
            let mut p: Vec<&str> = prefix.iter().map(String::as_str).collect();
            p.extend_from_slice(leaf);
            inv_resolve(&root, &p).value.clone()
        };
        assert_eq!(
            resolve_main(&["[0]", "m_SlotData", "m_ItemDefinition"]),
            properties::PropertyValue::Object("/Script/Angelscript.ItMi_Orenugget".to_string())
        );
        assert_eq!(
            resolve_main(&["[0]", "m_SlotData", "m_ItemCount"]),
            properties::PropertyValue::Int(9)
        );
        // m_InventoryType was retargeted from Quickslots to MainContainer.
        assert_eq!(
            resolve_main(&["[0]", "m_InventoryType"]),
            properties::PropertyValue::Enum(INV_MAIN_LABEL.to_string())
        );
    }

    #[test]
    fn add_item_seeds_empty_main_from_non_last_donor_slot() {
        // The donor container's LAST slot is stateful (non-empty m_Payload);
        // an earlier slot is clean. addItem must use the clean earlier slot
        // rather than skipping the whole container.
        let other_slots = vec![
            inv_item_slot(
                5,
                INV_OTHER_LABEL,
                "/Script/Angelscript.ItMi_Sulfur",
                1,
                &inv_empty_payload_map(),
            ),
            inv_item_slot(
                6,
                INV_OTHER_LABEL,
                "/Script/Angelscript.ItMw_Sword",
                1,
                &inv_nonempty_payload_map(),
            ),
        ];
        let mut payload = typed_inventory_private_payload(&other_slots, &[]);
        let edit = PrivateInventoryAddItemEdit {
            path: "/Script/Angelscript.ItMi_Orenugget".to_string(),
            count: 2,
        };
        apply_private_inventory_add_item_to_payload(&mut payload, &edit).unwrap();
        assert_eq!(inv_slot_count(&payload, 1), 1);
        let root = properties::parse_private_root(&payload).unwrap();
        let prefix = inv_slots_prefix(1);
        let mut def: Vec<&str> = prefix.iter().map(String::as_str).collect();
        def.extend_from_slice(&["[0]", "m_SlotData", "m_ItemDefinition"]);
        assert_eq!(
            inv_resolve(&root, &def).value,
            properties::PropertyValue::Object("/Script/Angelscript.ItMi_Orenugget".to_string())
        );
    }

    #[test]
    fn add_item_rejects_untyped_payload() {
        // The legacy scan-style payload is not parseable by the typed parser.
        let mut payload = inventory_payload_for_add_item_tests();
        let before = payload.clone();
        let edit = PrivateInventoryAddItemEdit {
            path: "/Script/Angelscript.ItMi_Sulfur".to_string(),
            count: 1,
        };
        let err = apply_private_inventory_add_item_to_payload(&mut payload, &edit).unwrap_err();
        assert!(
            matches!(err, CoreError::Parse(_)),
            "expected Parse error for untyped payload, got: {err}"
        );
        assert_eq!(payload, before);
    }

    #[test]
    fn add_item_uses_clean_slot_when_last_is_stateful() {
        // The last MainContainer slot carries item-specific state, but an
        // earlier slot is clean: addItem must clone the clean slot rather than
        // refusing.
        let main_slots = vec![
            inv_item_slot(
                0,
                INV_MAIN_LABEL,
                "/Script/Angelscript.ItMi_Orenugget",
                3,
                &inv_empty_payload_map(),
            ),
            inv_item_slot(
                1,
                INV_MAIN_LABEL,
                "/Script/Angelscript.ItMw_Sword",
                1,
                &inv_nonempty_payload_map(),
            ),
        ];
        let mut payload = typed_inventory_private_payload(&[], &main_slots);
        let edit = PrivateInventoryAddItemEdit {
            path: "/Script/Angelscript.ItMi_Sulfur".to_string(),
            count: 4,
        };
        apply_private_inventory_add_item_to_payload(&mut payload, &edit).unwrap();
        assert_eq!(inv_slot_count(&payload, 1), 3);
        // The new item exists in the MainContainer with the requested count.
        let refs = scan_fstrings(&payload, 0);
        let (items, _t, _s) = summarize_private_inventory_items(&payload, &refs, usize::MAX);
        assert!(
            items
                .iter()
                .any(|it| it["path"] == "/Script/Angelscript.ItMi_Sulfur" && it["count"] == 4)
        );
    }

    #[test]
    fn add_item_rejects_when_no_clean_template_anywhere() {
        // Every slot in every container carries item-specific state, so there
        // is no clean template to clone.
        let stateful =
            |id, path| inv_item_slot(id, INV_MAIN_LABEL, path, 1, &inv_nonempty_payload_map());
        let other = vec![inv_item_slot(
            9,
            INV_OTHER_LABEL,
            "/Script/Angelscript.ItMw_Axe",
            1,
            &inv_nonempty_payload_map(),
        )];
        let main = vec![stateful(0, "/Script/Angelscript.ItMw_Sword")];
        let mut payload = typed_inventory_private_payload(&other, &main);
        let before = payload.clone();
        let edit = PrivateInventoryAddItemEdit {
            path: "/Script/Angelscript.ItMi_Sulfur".to_string(),
            count: 1,
        };
        let err = apply_private_inventory_add_item_to_payload(&mut payload, &edit).unwrap_err();
        assert!(err.to_string().contains("clean"), "unexpected error: {err}");
        assert_eq!(payload, before);
    }

    fn inv_name_property(name: &str, value: &str) -> Vec<u8> {
        inv_tagged(name, "NameProperty", &[], 0, &fstring(value))
    }

    /// Native-serialized empty GameplayTagContainer (tag count 0), mirroring
    /// m_Ownership.OwnedByGuild in a real default ItemPayload.
    fn inv_empty_gameplay_tag_container(name: &str) -> Vec<u8> {
        let mut descriptor = 1u32.to_le_bytes().to_vec();
        descriptor.extend_from_slice(&fstring("GameplayTagContainer"));
        descriptor.extend_from_slice(&1u32.to_le_bytes());
        descriptor.extend_from_slice(&fstring("/Script/GameplayTags"));
        inv_tagged(
            name,
            "StructProperty",
            &descriptor,
            properties::TAG_FLAG_NATIVE_SERIALIZE,
            &0u32.to_le_bytes(),
        )
    }

    /// A default-initialised ItemPayload as it actually appears in real G1R
    /// saves: all scalar leaves at default (m_StageLevel=0, m_OptionalObject="",
    /// TagName="None") AND a nested EMPTY native GameplayTagContainer
    /// (m_Ownership.OwnedByGuild). This is the genuine "clean" template shape; the
    /// empty tag container is the leaf the first fix overlooked.
    fn inv_default_scalar_payload() -> Vec<u8> {
        let mut ownership_inner = inv_struct_property(
            "UseOwnershipOfArea",
            "GameplayTag",
            &inv_name_property("TagName", "None"),
        );
        ownership_inner.extend_from_slice(&inv_empty_gameplay_tag_container("OwnedByGuild"));
        let ownership = inv_struct_property("m_Ownership", "OwnershipSettings", &ownership_inner);
        let mut out = int_property("m_StageLevel", 0);
        out.extend_from_slice(&inv_object_property("m_OptionalObject", ""));
        out.extend_from_slice(&ownership);
        out
    }

    /// Same shape but with a non-default scalar leaf (m_StageLevel set): genuine
    /// item-specific state the add must refuse to clone.
    fn inv_nondefault_scalar_payload() -> Vec<u8> {
        let mut out = int_property("m_StageLevel", 7);
        out.extend_from_slice(&inv_object_property("m_OptionalObject", ""));
        out
    }

    #[test]
    fn add_item_clones_default_scalar_payload_slot() {
        // Real G1R item slots carry a default-initialised ItemPayload whose scalar
        // leaves are all zero/empty/None. These are semantically clean and must be
        // usable as an addItem template. The old rule flagged ANY scalar leaf as
        // state, so every real slot looked stateful, has_clean_template was always
        // false on real saves, and the inventory Add button vanished.
        let main_slots = vec![
            inv_item_slot(
                0,
                INV_MAIN_LABEL,
                "/Script/Angelscript.ItMi_Orenugget",
                3,
                &inv_default_scalar_payload(),
            ),
            inv_item_slot(
                1,
                INV_MAIN_LABEL,
                "/Script/Angelscript.ItFo_Apple",
                1,
                &inv_default_scalar_payload(),
            ),
        ];
        let mut payload = typed_inventory_private_payload(&[], &main_slots);
        let edit = PrivateInventoryAddItemEdit {
            path: "/Script/Angelscript.ItMi_Sulfur".to_string(),
            count: 4,
        };
        apply_private_inventory_add_item_to_payload(&mut payload, &edit).unwrap();
        assert_eq!(inv_slot_count(&payload, 1), 3);
        let refs = scan_fstrings(&payload, 0);
        let (items, _t, _s) = summarize_private_inventory_items(&payload, &refs, usize::MAX);
        assert!(
            items
                .iter()
                .any(|it| it["path"] == "/Script/Angelscript.ItMi_Sulfur" && it["count"] == 4)
        );
    }

    #[test]
    fn add_item_rejects_nondefault_scalar_payload_as_state() {
        // A non-default scalar leaf (m_StageLevel != 0) is genuine item state. If
        // that is the only payload shape available, there is no clean template —
        // relaxing the default-value rule must NOT swallow real state.
        let main = vec![inv_item_slot(
            0,
            INV_MAIN_LABEL,
            "/Script/Angelscript.ItMw_Sword",
            1,
            &inv_nondefault_scalar_payload(),
        )];
        let mut payload = typed_inventory_private_payload(&[], &main);
        let before = payload.clone();
        let edit = PrivateInventoryAddItemEdit {
            path: "/Script/Angelscript.ItMi_Sulfur".to_string(),
            count: 1,
        };
        let err = apply_private_inventory_add_item_to_payload(&mut payload, &edit).unwrap_err();
        assert!(err.to_string().contains("clean"), "unexpected error: {err}");
        assert_eq!(payload, before);
    }

    #[test]
    fn add_item_ignores_duplicate_in_other_container() {
        // The same item path in the OTHER container must not trigger the
        // "already" check, and the add must land in MainContainer.
        let other_slots = vec![inv_item_slot(
            0,
            INV_OTHER_LABEL,
            "/Script/Angelscript.ItMi_Sulfur",
            1,
            &inv_empty_payload_map(),
        )];
        let mut payload = typed_inventory_private_payload(&other_slots, &default_main_slots());
        let edit = PrivateInventoryAddItemEdit {
            path: "/Script/Angelscript.ItMi_Sulfur".to_string(),
            count: 4,
        };
        apply_private_inventory_add_item_to_payload(&mut payload, &edit).unwrap();

        // Other container untouched, MainContainer grew by one.
        assert_eq!(inv_slot_count(&payload, 0), 1);
        assert_eq!(inv_slot_count(&payload, 1), 3);
        let root = properties::parse_private_root(&payload).unwrap();
        let prefix = inv_slots_prefix(1);
        let mut def_path: Vec<&str> = prefix.iter().map(String::as_str).collect();
        def_path.extend_from_slice(&["[2]", "m_SlotData", "m_ItemDefinition"]);
        assert_eq!(
            inv_resolve(&root, &def_path).value,
            properties::PropertyValue::Object("/Script/Angelscript.ItMi_Sulfur".to_string())
        );
    }

    #[test]
    fn inspect_marks_only_main_container_items_removable() {
        // An item living only in another container (e.g. Quickslots) shows in
        // the region scan but is not in the MainContainer, so it must be marked
        // removable: false (its delete button is hidden), while MainContainer
        // rows are removable: true.
        let dir = tempdir().unwrap();
        let path = dir.path().join("G1R-001.sav");
        // Sulfur lives only in the other container. Apple lives in BOTH the
        // other container and the MainContainer; its path collides, so the
        // other container's scan row can't be addressed per-row by path and
        // Apple must not be marked removable.
        let other_slots = vec![
            inv_item_slot(
                0,
                INV_OTHER_LABEL,
                "/Script/Angelscript.ItMi_Sulfur",
                1,
                &inv_empty_payload_map(),
            ),
            inv_item_slot(
                1,
                INV_OTHER_LABEL,
                "/Script/Angelscript.ItFo_Apple",
                1,
                &inv_empty_payload_map(),
            ),
        ];
        let private_payload = typed_inventory_private_payload(&other_slots, &default_main_slots());
        let seed_compressed = b"seed-removable".to_vec();
        let stream = compressed_stream_with_one_chunk(&seed_compressed, private_payload.len());
        fs::write(
            &path,
            build_gsav(2, &public_payload("Slot A"), &stream, &[0, 0, 0, 0]),
        )
        .unwrap();
        let backend = PrefixCodecBackend {
            seed_compressed,
            seed_uncompressed: private_payload,
        };

        let value = inspect_save_with_codec_backend(&path, true, Some(&backend), None).unwrap();
        let inv = &value["private"]["inventory"];
        assert_eq!(
            inv["writable"],
            json!([
                "private.inventory.setItemCount",
                "private.inventory.addItem",
                "private.inventory.removeItem"
            ])
        );
        let items = inv["items"].as_array().expect("items array");
        let removable_for = |path: &str| -> Option<bool> {
            items
                .iter()
                .find(|it| it["path"] == path)
                .map(|it| it["removable"].as_bool().unwrap_or(false))
        };
        // Orenugget is unique to the MainContainer → removable.
        assert_eq!(
            removable_for("/Script/Angelscript.ItMi_Orenugget"),
            Some(true)
        );
        // Apple collides across containers → not removable (the other
        // container's row can't be addressed per-row by path).
        assert_eq!(removable_for("/Script/Angelscript.ItFo_Apple"), Some(false));
        // Sulfur is only in the other container → not removable.
        assert_eq!(
            removable_for("/Script/Angelscript.ItMi_Sulfur"),
            Some(false),
            "an item only in the other container must not be removable"
        );
    }

    #[test]
    fn inspect_does_not_mark_intra_main_container_duplicate_removable() {
        // Two MainContainer slots of the same item (a real non-stacking case).
        // removeItem is path-addressed and the summary rows carry no per-slot
        // id, so the two duplicate rows are indistinguishable — deleting one
        // could drop the other. The duplicate path must NOT be removable.
        let dir = tempdir().unwrap();
        let path = dir.path().join("G1R-dup.sav");
        let other_slots = vec![inv_item_slot(
            0,
            INV_OTHER_LABEL,
            "/Script/Angelscript.ItMi_Sulfur",
            1,
            &inv_empty_payload_map(),
        )];
        let main_slots = vec![
            inv_item_slot(
                0,
                INV_MAIN_LABEL,
                "/Script/Angelscript.ItMi_Orenugget",
                3,
                &inv_empty_payload_map(),
            ),
            inv_item_slot(
                1,
                INV_MAIN_LABEL,
                "/Script/Angelscript.ItMi_Orenugget",
                5,
                &inv_empty_payload_map(),
            ),
        ];
        let private_payload = typed_inventory_private_payload(&other_slots, &main_slots);
        let seed_compressed = b"seed-dup".to_vec();
        let stream = compressed_stream_with_one_chunk(&seed_compressed, private_payload.len());
        fs::write(
            &path,
            build_gsav(2, &public_payload("Slot A"), &stream, &[0, 0, 0, 0]),
        )
        .unwrap();
        let backend = PrefixCodecBackend {
            seed_compressed,
            seed_uncompressed: private_payload,
        };

        let value = inspect_save_with_codec_backend(&path, true, Some(&backend), None).unwrap();
        let inv = &value["private"]["inventory"];
        // No globally-unique MainContainer path exists (Orenugget is duplicated,
        // Sulfur is only in the other container), so removeItem is not offered.
        assert!(
            !inv["writable"]
                .as_array()
                .unwrap()
                .contains(&json!("private.inventory.removeItem")),
            "a duplicated MainContainer item must not be removable"
        );
        let orenugget_rows: Vec<bool> = inv["items"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|it| it["path"] == "/Script/Angelscript.ItMi_Orenugget")
            .map(|it| it["removable"].as_bool().unwrap_or(false))
            .collect();
        assert_eq!(
            orenugget_rows,
            vec![false, false],
            "duplicate MainContainer rows must not be removable"
        );
    }

    #[test]
    fn write_save_applies_inventory_add_item() {
        // End to end: write_save parses the edit, applies it to the typed
        // payload, recompresses, and the output save's inventory summary
        // contains the new item.
        let dir = tempdir().unwrap();
        let path = dir.path().join("G1R-001.sav");
        let output_path = dir.path().join("G1R-001-out.sav");
        let private_payload = typed_inventory_private_payload(&[], &default_main_slots());
        let seed_compressed = b"seed-add".to_vec();
        let stream = compressed_stream_with_one_chunk(&seed_compressed, private_payload.len());
        fs::write(
            &path,
            build_gsav(2, &public_payload("Slot A"), &stream, &[0, 0, 0, 0]),
        )
        .unwrap();
        let backend = PrefixCodecBackend {
            seed_compressed,
            seed_uncompressed: private_payload,
        };

        let response = write_save_with_codec_backend(
            &path,
            &[json!({
                "path": "private.inventory.addItem",
                "value": {
                    "path": "/Script/Angelscript.ItFo_Cheese",
                    "count": 5
                }
            })],
            false,
            Some(&output_path),
            Some(&backend),
        )
        .unwrap();
        assert_eq!(response["editsApplied"], 1);

        let value =
            inspect_save_with_codec_backend(&output_path, true, Some(&backend), None).unwrap();
        let items = value["private"]["inventory"]["items"].as_array().unwrap();
        assert!(
            items.iter().any(|item| item["path"]
                == "/Script/Angelscript.ItFo_Cheese"
                && item["count"] == 5),
            "output save inventory missing new item: {items:?}"
        );
    }

    #[test]
    fn parse_private_inventory_add_item_rejects_count_zero() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("G1R-001.sav");
        let private_payload = inventory_payload_for_add_item_tests();
        let seed_compressed = b"seed-add0".to_vec();
        let stream = compressed_stream_with_one_chunk(&seed_compressed, private_payload.len());
        fs::write(
            &path,
            build_gsav(2, &public_payload("Slot A"), &stream, &[0, 0, 0, 0]),
        )
        .unwrap();
        let backend = PrefixCodecBackend {
            seed_compressed,
            seed_uncompressed: private_payload,
        };

        let err = write_save_with_codec_backend(
            &path,
            &[json!({
                "path": "private.inventory.addItem",
                "value": {
                    "path": "/Script/Angelscript.ItMi_Orenugget",
                    "count": 0
                }
            })],
            false,
            None,
            Some(&backend),
        )
        .unwrap_err();
        assert!(
            matches!(err, CoreError::InvalidRequest(_)),
            "expected InvalidRequest for count 0, got: {err}"
        );
    }

    #[test]
    fn parse_private_inventory_add_item_rejects_missing_count() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("G1R-001.sav");
        let private_payload = inventory_payload_for_add_item_tests();
        let seed_compressed = b"seed-nocount".to_vec();
        let stream = compressed_stream_with_one_chunk(&seed_compressed, private_payload.len());
        fs::write(
            &path,
            build_gsav(2, &public_payload("Slot A"), &stream, &[0, 0, 0, 0]),
        )
        .unwrap();
        let backend = PrefixCodecBackend {
            seed_compressed,
            seed_uncompressed: private_payload,
        };

        let err = write_save_with_codec_backend(
            &path,
            &[json!({
                "path": "private.inventory.addItem",
                "value": {
                    "path": "/Script/Angelscript.ItMi_Orenugget"
                }
            })],
            false,
            None,
            Some(&backend),
        )
        .unwrap_err();
        assert!(
            matches!(err, CoreError::InvalidRequest(_)),
            "expected InvalidRequest for missing count, got: {err}"
        );
    }

    #[test]
    fn parse_private_inventory_add_item_rejects_invalid_path() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("G1R-001.sav");
        let private_payload = inventory_payload_for_add_item_tests();
        let seed_compressed = b"seed-badpath".to_vec();
        let stream = compressed_stream_with_one_chunk(&seed_compressed, private_payload.len());
        fs::write(
            &path,
            build_gsav(2, &public_payload("Slot A"), &stream, &[0, 0, 0, 0]),
        )
        .unwrap();
        let backend = PrefixCodecBackend {
            seed_compressed,
            seed_uncompressed: private_payload,
        };

        let err = write_save_with_codec_backend(
            &path,
            &[json!({
                "path": "private.inventory.addItem",
                "value": {
                    "path": "not_a_path",
                    "count": 5
                }
            })],
            false,
            None,
            Some(&backend),
        )
        .unwrap_err();
        assert!(
            matches!(err, CoreError::InvalidRequest(_)),
            "expected InvalidRequest for invalid item path, got: {err}"
        );
    }

    #[test]
    fn parse_private_inventory_add_item_rejects_non_item_script_path() {
        // A /Script object outside the Angelscript item namespace (e.g.
        // /Script/Engine.Foo) parses as a "full asset path" but is not an item
        // definition; addItem must reject it rather than persist a non-item ref.
        let dir = tempdir().unwrap();
        let path = dir.path().join("G1R-001.sav");
        let private_payload = inventory_payload_for_add_item_tests();
        let seed_compressed = b"seed-nonitem".to_vec();
        let stream = compressed_stream_with_one_chunk(&seed_compressed, private_payload.len());
        fs::write(
            &path,
            build_gsav(2, &public_payload("Slot A"), &stream, &[0, 0, 0, 0]),
        )
        .unwrap();
        let backend = PrefixCodecBackend {
            seed_compressed,
            seed_uncompressed: private_payload,
        };

        let err = write_save_with_codec_backend(
            &path,
            &[json!({
                "path": "private.inventory.addItem",
                "value": {
                    "path": "/Script/Engine.Foo",
                    "count": 5
                }
            })],
            false,
            None,
            Some(&backend),
        )
        .unwrap_err();
        assert!(
            matches!(err, CoreError::InvalidRequest(_)),
            "expected InvalidRequest for non-item /Script path, got: {err}"
        );

        // A non-item Angelscript class (right namespace, not an `It*` item
        // definition) must also be rejected.
        let err = write_save_with_codec_backend(
            &path,
            &[json!({
                "path": "private.inventory.addItem",
                "value": {
                    "path": "/Script/Angelscript.GothicFinalDataGame",
                    "count": 1
                }
            })],
            false,
            None,
            Some(&backend),
        )
        .unwrap_err();
        assert!(
            matches!(err, CoreError::InvalidRequest(_)),
            "expected InvalidRequest for non-item Angelscript class, got: {err}"
        );

        // A non-inventory It* class that the catalog builder excludes (e.g.
        // ItemCollisionFX) matches the broad It prefix but must be rejected.
        let err = write_save_with_codec_backend(
            &path,
            &[json!({
                "path": "private.inventory.addItem",
                "value": {
                    "path": "/Script/Angelscript.ItemCollisionFX",
                    "count": 1
                }
            })],
            false,
            None,
            Some(&backend),
        )
        .unwrap_err();
        assert!(
            matches!(err, CoreError::InvalidRequest(_)),
            "expected InvalidRequest for non-inventory It* class, got: {err}"
        );

        // A well-formed but non-existent It* class (typo / not in the catalog)
        // must be rejected — the allow-list is the catalog, not the It* shape.
        let err = write_save_with_codec_backend(
            &path,
            &[json!({
                "path": "private.inventory.addItem",
                "value": {
                    "path": "/Script/Angelscript.ItMi_NotARealClass",
                    "count": 1
                }
            })],
            false,
            None,
            Some(&backend),
        )
        .unwrap_err();
        assert!(
            matches!(err, CoreError::InvalidRequest(_)),
            "expected InvalidRequest for an uncatalogued It* class, got: {err}"
        );
    }

    #[test]
    fn parse_private_inventory_add_item_rejects_bare_id_path() {
        // A bare inventory id (no /Script/ prefix) is accepted by the matcher
        // heuristic but must be rejected for addItem, which writes the string
        // into m_ItemDefinition and would otherwise yield an unresolvable ref.
        let dir = tempdir().unwrap();
        let path = dir.path().join("G1R-001.sav");
        let private_payload = inventory_payload_for_add_item_tests();
        let seed_compressed = b"seed-bareid".to_vec();
        let stream = compressed_stream_with_one_chunk(&seed_compressed, private_payload.len());
        fs::write(
            &path,
            build_gsav(2, &public_payload("Slot A"), &stream, &[0, 0, 0, 0]),
        )
        .unwrap();
        let backend = PrefixCodecBackend {
            seed_compressed,
            seed_uncompressed: private_payload,
        };

        let err = write_save_with_codec_backend(
            &path,
            &[json!({
                "path": "private.inventory.addItem",
                "value": {
                    "path": "ItMi_Orenugget",
                    "count": 5
                }
            })],
            false,
            None,
            Some(&backend),
        )
        .unwrap_err();
        assert!(
            matches!(err, CoreError::InvalidRequest(_)),
            "expected InvalidRequest for bare-id item path, got: {err}"
        );
    }

    #[test]
    fn parse_private_inventory_add_item_rejects_two_in_batch() {
        // Two addItem edits in one batch must be rejected as structural edits
        // (array-length changes) for the same reason as two arrayDuplicate ops.
        let dir = tempdir().unwrap();
        let path = dir.path().join("G1R-001.sav");
        let private_payload = inventory_payload_for_add_item_tests();
        let seed_compressed = b"seed-2add".to_vec();
        let stream = compressed_stream_with_one_chunk(&seed_compressed, private_payload.len());
        fs::write(
            &path,
            build_gsav(2, &public_payload("Slot A"), &stream, &[0, 0, 0, 0]),
        )
        .unwrap();
        let backend = PrefixCodecBackend {
            seed_compressed,
            seed_uncompressed: private_payload,
        };

        let err = write_save_with_codec_backend(
            &path,
            &[
                json!({
                    "path": "private.inventory.addItem",
                    "value": {
                        "path": "/Script/Angelscript.ItMi_Orenugget",
                        "count": 1
                    }
                }),
                json!({
                    "path": "private.inventory.addItem",
                    "value": {
                        "path": "/Script/Angelscript.ItAt_Lurker_01",
                        "count": 2
                    }
                }),
            ],
            false,
            None,
            Some(&backend),
        )
        .unwrap_err();
        assert!(
            matches!(err, CoreError::UnsupportedEdit(_)),
            "expected UnsupportedEdit for two addItem in batch, got: {err}"
        );
        assert!(
            err.to_string()
                .contains("at most one structural array edit"),
            "unexpected error message: {err}"
        );
    }

    #[test]
    fn parse_private_inventory_add_item_rejects_mixed_with_array_duplicate() {
        // One addItem + one arrayDuplicate counts as two structural edits and
        // must be rejected.
        let dir = tempdir().unwrap();
        let path = dir.path().join("G1R-001.sav");
        // Need a payload that also has a typed str-array for arrayDuplicate to
        // parse cleanly; reuse private_str_array_property helper.
        let private_payload = {
            let mut p = fstring("/Script/Angelscript.GothicFinalDataGame");
            p.push(0);
            p.extend_from_slice(&private_str_array_property("Events", &["A"]));
            p.extend_from_slice(&fstring("None"));
            p.extend_from_slice(&0u32.to_le_bytes());
            p
        };
        let seed_compressed = b"seed-mixed".to_vec();
        let stream = compressed_stream_with_one_chunk(&seed_compressed, private_payload.len());
        fs::write(
            &path,
            build_gsav(2, &public_payload("Slot A"), &stream, &[0, 0, 0, 0]),
        )
        .unwrap();
        let backend = PrefixCodecBackend {
            seed_compressed,
            seed_uncompressed: private_payload,
        };

        let err = write_save_with_codec_backend(
            &path,
            &[
                json!({
                    "path": "private.inventory.addItem",
                    "value": {
                        "path": "/Script/Angelscript.ItMi_Orenugget",
                        "count": 1
                    }
                }),
                json!({
                    "path": "private.typed.arrayDuplicate",
                    "value": { "path": ["Events"], "index": 0 }
                }),
            ],
            false,
            None,
            Some(&backend),
        )
        .unwrap_err();
        assert!(
            matches!(err, CoreError::UnsupportedEdit(_)),
            "expected UnsupportedEdit for addItem + arrayDuplicate, got: {err}"
        );
        assert!(
            err.to_string()
                .contains("at most one structural array edit"),
            "unexpected error message: {err}"
        );
    }

    #[test]
    fn parse_private_inventory_add_item_rejects_negative_count() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("G1R-001.sav");
        let private_payload = inventory_payload_for_add_item_tests();
        let seed_compressed = b"seed-negcount".to_vec();
        let stream = compressed_stream_with_one_chunk(&seed_compressed, private_payload.len());
        fs::write(
            &path,
            build_gsav(2, &public_payload("Slot A"), &stream, &[0, 0, 0, 0]),
        )
        .unwrap();
        let backend = PrefixCodecBackend {
            seed_compressed,
            seed_uncompressed: private_payload,
        };

        let err = write_save_with_codec_backend(
            &path,
            &[json!({
                "path": "private.inventory.addItem",
                "value": {
                    "path": "/Script/Angelscript.ItMi_Orenugget",
                    "count": -1
                }
            })],
            false,
            None,
            Some(&backend),
        )
        .unwrap_err();
        assert!(
            matches!(err, CoreError::InvalidRequest(_)),
            "expected InvalidRequest for count -1, got: {err}"
        );
    }

    #[test]
    fn parse_private_inventory_add_item_rejects_non_object_value() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("G1R-001.sav");
        let private_payload = inventory_payload_for_add_item_tests();
        let seed_compressed = b"seed-strval".to_vec();
        let stream = compressed_stream_with_one_chunk(&seed_compressed, private_payload.len());
        fs::write(
            &path,
            build_gsav(2, &public_payload("Slot A"), &stream, &[0, 0, 0, 0]),
        )
        .unwrap();
        let backend = PrefixCodecBackend {
            seed_compressed,
            seed_uncompressed: private_payload,
        };

        let err = write_save_with_codec_backend(
            &path,
            &[json!({
                "path": "private.inventory.addItem",
                "value": "not_an_object"
            })],
            false,
            None,
            Some(&backend),
        )
        .unwrap_err();
        assert!(
            matches!(err, CoreError::InvalidRequest(_)),
            "expected InvalidRequest for non-object value, got: {err}"
        );
    }

    // ── private.inventory.removeItem tests ──────────────────────────────────

    #[test]
    fn parse_private_inventory_remove_item_accepts_valid_path() {
        let edit = Edit {
            path: "private.inventory.removeItem".to_string(),
            value: json!({ "path": "/Script/Angelscript.ItMi_Orenugget" }),
        };
        let parsed = parse_private_inventory_remove_item_edit(&edit).unwrap();
        assert_eq!(
            parsed,
            PrivateInventoryRemoveItemEdit {
                path: "/Script/Angelscript.ItMi_Orenugget".to_string(),
            }
        );
    }

    #[test]
    fn parse_private_inventory_remove_item_rejects_non_object_value() {
        let edit = Edit {
            path: "private.inventory.removeItem".to_string(),
            value: json!("not_an_object"),
        };
        let err = parse_private_inventory_remove_item_edit(&edit).unwrap_err();
        assert!(
            matches!(err, CoreError::InvalidRequest(_)),
            "expected InvalidRequest for non-object value, got: {err}"
        );
    }

    #[test]
    fn parse_private_inventory_remove_item_rejects_invalid_path() {
        let edit = Edit {
            path: "private.inventory.removeItem".to_string(),
            value: json!({ "path": "not_a_path" }),
        };
        let err = parse_private_inventory_remove_item_edit(&edit).unwrap_err();
        assert!(
            matches!(err, CoreError::InvalidRequest(_)),
            "expected InvalidRequest for invalid item path, got: {err}"
        );
    }

    #[test]
    fn remove_item_deletes_matching_slot_from_main_container() {
        // Two slots; removing Orenugget must leave Apple intact and shrink the
        // MainContainer slot count by one.
        let mut payload = typed_inventory_private_payload(&[], &default_main_slots());
        assert_eq!(inv_slot_count(&payload, 1), 2);
        let edit = PrivateInventoryRemoveItemEdit {
            path: "/Script/Angelscript.ItMi_Orenugget".to_string(),
        };
        apply_private_inventory_remove_item_to_payload(&mut payload, &edit).unwrap();

        // Strict re-parse and slot count decreased by one.
        assert_eq!(inv_slot_count(&payload, 1), 1);
        let root = properties::parse_private_root(&payload).unwrap();
        // The surviving slot is the OTHER item (Apple).
        let prefix = inv_slots_prefix(1);
        let mut def_path: Vec<&str> = prefix.iter().map(String::as_str).collect();
        def_path.extend_from_slice(&["[0]", "m_SlotData", "m_ItemDefinition"]);
        let definition = inv_resolve(&root, &def_path);
        assert_eq!(
            definition.value,
            properties::PropertyValue::Object("/Script/Angelscript.ItFo_Apple".to_string())
        );

        // The inventory summary scan no longer lists the removed item, but does
        // still list the survivor.
        let refs = scan_fstrings(&payload, 0);
        let (items, _total, scope) = summarize_private_inventory_items(&payload, &refs, 200);
        assert_eq!(scope, "player_inventory_region");
        assert!(
            !items
                .iter()
                .any(|item| item["path"] == "/Script/Angelscript.ItMi_Orenugget"),
            "removed item still present in summary: {items:?}"
        );
        assert!(
            items
                .iter()
                .any(|item| item["path"] == "/Script/Angelscript.ItFo_Apple"),
            "survivor missing from summary: {items:?}"
        );
    }

    #[test]
    fn remove_item_succeeds_when_path_also_in_other_container() {
        // The same item path in another container (e.g. Quickslots) must not
        // block removing it from the MainContainer: the post-check verifies the
        // MainContainer slot is gone, not global absence across all containers.
        let other_slots = vec![inv_item_slot(
            0,
            INV_OTHER_LABEL,
            "/Script/Angelscript.ItMi_Orenugget",
            1,
            &inv_empty_payload_map(),
        )];
        let mut payload = typed_inventory_private_payload(&other_slots, &default_main_slots());
        assert_eq!(inv_slot_count(&payload, 0), 1);
        assert_eq!(inv_slot_count(&payload, 1), 2);
        let edit = PrivateInventoryRemoveItemEdit {
            path: "/Script/Angelscript.ItMi_Orenugget".to_string(),
        };
        apply_private_inventory_remove_item_to_payload(&mut payload, &edit).unwrap();

        // MainContainer shrank; the other container's copy is untouched.
        assert_eq!(inv_slot_count(&payload, 1), 1);
        assert_eq!(inv_slot_count(&payload, 0), 1);
        let root = properties::parse_private_root(&payload).unwrap();
        let main_prefix = inv_slots_prefix(1);
        let mut main_def: Vec<&str> = main_prefix.iter().map(String::as_str).collect();
        main_def.extend_from_slice(&["[0]", "m_SlotData", "m_ItemDefinition"]);
        assert_eq!(
            inv_resolve(&root, &main_def).value,
            properties::PropertyValue::Object("/Script/Angelscript.ItFo_Apple".to_string()),
            "MainContainer should retain only Apple after removing Orenugget"
        );
        let other_prefix = inv_slots_prefix(0);
        let mut other_def: Vec<&str> = other_prefix.iter().map(String::as_str).collect();
        other_def.extend_from_slice(&["[0]", "m_SlotData", "m_ItemDefinition"]);
        assert_eq!(
            inv_resolve(&root, &other_def).value,
            properties::PropertyValue::Object("/Script/Angelscript.ItMi_Orenugget".to_string()),
            "the other container's Orenugget must be left intact"
        );
    }

    #[test]
    fn remove_item_rejects_path_not_present() {
        let mut payload = typed_inventory_private_payload(&[], &default_main_slots());
        let before = payload.clone();
        let edit = PrivateInventoryRemoveItemEdit {
            path: "/Script/Angelscript.ItMi_Sulfur".to_string(),
        };
        let err = apply_private_inventory_remove_item_to_payload(&mut payload, &edit).unwrap_err();
        assert!(
            err.to_string().contains("does not contain"),
            "unexpected error: {err}"
        );
        assert_eq!(payload, before, "failed edit must not mutate the payload");
    }

    #[test]
    fn remove_item_removes_first_of_duplicate_paths() {
        // Real saves contain a few same-path slots; removing must delete the
        // first match (leaving one) rather than refusing — refusing would make
        // such items permanently undeletable.
        let dup_main = vec![
            inv_item_slot(
                0,
                INV_MAIN_LABEL,
                "/Script/Angelscript.ItMi_Orenugget",
                3,
                &inv_empty_payload_map(),
            ),
            inv_item_slot(
                1,
                INV_MAIN_LABEL,
                "/Script/Angelscript.ItMi_Orenugget",
                5,
                &inv_empty_payload_map(),
            ),
        ];
        let mut payload = typed_inventory_private_payload(&[], &dup_main);
        let edit = PrivateInventoryRemoveItemEdit {
            path: "/Script/Angelscript.ItMi_Orenugget".to_string(),
        };
        apply_private_inventory_remove_item_to_payload(&mut payload, &edit).unwrap();
        // One Orenugget slot remains.
        assert_eq!(inv_slot_count(&payload, 1), 1);
        let root = properties::parse_private_root(&payload).unwrap();
        let prefix = inv_slots_prefix(1);
        let mut def: Vec<&str> = prefix.iter().map(String::as_str).collect();
        def.extend_from_slice(&["[0]", "m_SlotData", "m_ItemDefinition"]);
        assert_eq!(
            inv_resolve(&root, &def).value,
            properties::PropertyValue::Object("/Script/Angelscript.ItMi_Orenugget".to_string())
        );
    }

    #[test]
    fn remove_item_rejects_untyped_payload() {
        let mut payload = inventory_payload_for_add_item_tests();
        let before = payload.clone();
        let edit = PrivateInventoryRemoveItemEdit {
            path: "/Script/Angelscript.ItMi_Orenugget".to_string(),
        };
        let err = apply_private_inventory_remove_item_to_payload(&mut payload, &edit).unwrap_err();
        assert!(
            matches!(err, CoreError::Parse(_)),
            "expected Parse error for untyped payload, got: {err}"
        );
        assert_eq!(payload, before);
    }

    #[test]
    fn write_save_applies_inventory_remove_item() {
        // End to end: write_save parses the edit, applies it to the typed
        // payload, recompresses, and the output save's inventory summary no
        // longer contains the removed item.
        let dir = tempdir().unwrap();
        let path = dir.path().join("G1R-001.sav");
        let output_path = dir.path().join("G1R-001-out.sav");
        let private_payload = typed_inventory_private_payload(&[], &default_main_slots());
        let seed_compressed = b"seed-remove".to_vec();
        let stream = compressed_stream_with_one_chunk(&seed_compressed, private_payload.len());
        fs::write(
            &path,
            build_gsav(2, &public_payload("Slot A"), &stream, &[0, 0, 0, 0]),
        )
        .unwrap();
        let backend = PrefixCodecBackend {
            seed_compressed,
            seed_uncompressed: private_payload,
        };

        let response = write_save_with_codec_backend(
            &path,
            &[json!({
                "path": "private.inventory.removeItem",
                "value": { "path": "/Script/Angelscript.ItMi_Orenugget" }
            })],
            false,
            Some(&output_path),
            Some(&backend),
        )
        .unwrap();
        assert_eq!(response["editsApplied"], 1);

        let value =
            inspect_save_with_codec_backend(&output_path, true, Some(&backend), None).unwrap();
        let items = value["private"]["inventory"]["items"].as_array().unwrap();
        assert!(
            !items
                .iter()
                .any(|item| item["path"] == "/Script/Angelscript.ItMi_Orenugget"),
            "output save inventory still has removed item: {items:?}"
        );
        assert!(
            items
                .iter()
                .any(|item| item["path"] == "/Script/Angelscript.ItFo_Apple"),
            "output save inventory missing survivor: {items:?}"
        );
    }

    #[test]
    fn parse_private_inventory_remove_item_rejects_two_in_batch() {
        // Two removeItem edits in one batch are two structural edits.
        let dir = tempdir().unwrap();
        let path = dir.path().join("G1R-001.sav");
        let private_payload = typed_inventory_private_payload(&[], &default_main_slots());
        let seed_compressed = b"seed-2remove".to_vec();
        let stream = compressed_stream_with_one_chunk(&seed_compressed, private_payload.len());
        fs::write(
            &path,
            build_gsav(2, &public_payload("Slot A"), &stream, &[0, 0, 0, 0]),
        )
        .unwrap();
        let backend = PrefixCodecBackend {
            seed_compressed,
            seed_uncompressed: private_payload,
        };

        let err = write_save_with_codec_backend(
            &path,
            &[
                json!({
                    "path": "private.inventory.removeItem",
                    "value": { "path": "/Script/Angelscript.ItMi_Orenugget" }
                }),
                json!({
                    "path": "private.inventory.removeItem",
                    "value": { "path": "/Script/Angelscript.ItFo_Apple" }
                }),
            ],
            false,
            None,
            Some(&backend),
        )
        .unwrap_err();
        assert!(
            matches!(err, CoreError::UnsupportedEdit(_)),
            "expected UnsupportedEdit for two removeItem in batch, got: {err}"
        );
        assert!(
            err.to_string()
                .contains("at most one structural array edit"),
            "unexpected error message: {err}"
        );
    }

    #[test]
    fn parse_private_inventory_remove_item_rejects_mixed_with_add_item() {
        // addItem + removeItem in one batch counts as two structural edits.
        let dir = tempdir().unwrap();
        let path = dir.path().join("G1R-001.sav");
        let private_payload = typed_inventory_private_payload(&[], &default_main_slots());
        let seed_compressed = b"seed-mixed-rm".to_vec();
        let stream = compressed_stream_with_one_chunk(&seed_compressed, private_payload.len());
        fs::write(
            &path,
            build_gsav(2, &public_payload("Slot A"), &stream, &[0, 0, 0, 0]),
        )
        .unwrap();
        let backend = PrefixCodecBackend {
            seed_compressed,
            seed_uncompressed: private_payload,
        };

        let err = write_save_with_codec_backend(
            &path,
            &[
                json!({
                    "path": "private.inventory.addItem",
                    "value": { "path": "/Script/Angelscript.ItFo_Cheese", "count": 1 }
                }),
                json!({
                    "path": "private.inventory.removeItem",
                    "value": { "path": "/Script/Angelscript.ItMi_Orenugget" }
                }),
            ],
            false,
            None,
            Some(&backend),
        )
        .unwrap_err();
        assert!(
            matches!(err, CoreError::UnsupportedEdit(_)),
            "expected UnsupportedEdit for addItem + removeItem, got: {err}"
        );
        assert!(
            err.to_string()
                .contains("at most one structural array edit"),
            "unexpected error message: {err}"
        );
    }

    #[test]
    fn parse_difficulty_settings_reads_preset_and_bools() {
        let mut payload = Vec::new();
        payload.extend_from_slice(&fstring("m_difficultyPreset"));
        payload.extend_from_slice(&fstring("ClassProperty"));
        payload.extend_from_slice(&fstring("/Script/Angelscript.DifficultyPreset_Custom"));
        payload.extend_from_slice(&fstring("m_customCombatSettings"));
        payload.extend_from_slice(&fstring("ClassProperty"));
        payload.extend_from_slice(&fstring(
            "/Script/Angelscript.CombatDifficultySettings_Hard",
        ));
        payload.extend_from_slice(&fstring("m_FakeSloppyCombos"));
        payload.extend_from_slice(&fstring("BoolProperty"));
        payload.extend_from_slice(&[0u8; 8]); // array_index + size
        payload.push(properties::TAG_FLAG_BOOL_TRUE); // bool tag bit = true
        payload.extend_from_slice(&fstring("m_PermanentDeath"));
        payload.extend_from_slice(&fstring("BoolProperty"));
        payload.extend_from_slice(&[0u8; 8]);
        payload.push(0); // false

        let d = parse_difficulty_settings(&payload);
        assert_eq!(d.preset.as_deref(), Some("DifficultyPreset_Custom"));
        assert_eq!(d.combat.as_deref(), Some("CombatDifficultySettings_Hard"));
        assert_eq!(d.flow_helper, Some(true));
        assert_eq!(d.permadeath, Some(false));
    }

    #[test]
    fn parse_difficulty_settings_reads_permadeath_alternate_spelling() {
        let mut payload = Vec::new();
        payload.extend_from_slice(&fstring("m_difficultyPreset"));
        payload.extend_from_slice(&fstring("ClassProperty"));
        payload.extend_from_slice(&fstring("/Script/Angelscript.DifficultyPreset_Custom"));
        // Permadeath stored under the alternate spelling `m_PermaDeath`.
        payload.extend_from_slice(&fstring("m_PermaDeath"));
        payload.extend_from_slice(&fstring("BoolProperty"));
        payload.extend_from_slice(&[0u8; 8]); // array_index + size
        payload.push(properties::TAG_FLAG_BOOL_TRUE); // bool tag bit = true

        let d = parse_difficulty_settings(&payload);
        assert_eq!(d.permadeath, Some(true));
    }

    #[test]
    fn parse_difficulty_settings_bool_only_true_on_0x10_bit() {
        // A FALSE BoolProperty whose tag byte carries another flag (0x08) must
        // read false — only the 0x10 (TAG_FLAG_BOOL_TRUE) bit means true. A
        // "nonzero" test would misread it and a later save would flip it on.
        let mut payload = Vec::new();
        payload.extend_from_slice(&fstring("m_FakeSloppyCombos"));
        payload.extend_from_slice(&fstring("BoolProperty"));
        payload.extend_from_slice(&[0u8; 8]);
        payload.push(0x08); // a non-0x10 flag; the bool value is FALSE
        let d = parse_difficulty_settings(&payload);
        assert_eq!(d.flow_helper, Some(false));
    }

    #[test]
    fn difficulty_for_gsav_bytes_none_for_non_gsav() {
        assert!(difficulty_for_gsav_bytes(b"NOPE").is_none());
    }
}
