pub mod codec_backend;
mod kraken;

use base64::{Engine as _, engine::general_purpose};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha1::{Digest, Sha1};
use std::collections::HashMap;
use std::ffi::{CStr, CString, c_char};
use std::fs;
use std::path::{Path, PathBuf};
use std::ptr;
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

struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
    base_offset: usize,
}

impl<'a> Reader<'a> {
    fn new(data: &'a [u8], base_offset: usize) -> Self {
        Self {
            data,
            pos: 0,
            base_offset,
        }
    }

    fn abs_pos(&self) -> usize {
        self.base_offset + self.pos
    }

    fn remaining(&self) -> usize {
        self.data.len().saturating_sub(self.pos)
    }

    fn read(&mut self, n: usize) -> Result<&'a [u8], CoreError> {
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

    fn u8(&mut self) -> Result<u8, CoreError> {
        Ok(self.read(1)?[0])
    }

    fn u32(&mut self) -> Result<u32, CoreError> {
        let mut b = [0u8; 4];
        b.copy_from_slice(self.read(4)?);
        Ok(u32::from_le_bytes(b))
    }

    fn i32(&mut self) -> Result<i32, CoreError> {
        let mut b = [0u8; 4];
        b.copy_from_slice(self.read(4)?);
        Ok(i32::from_le_bytes(b))
    }

    fn u64(&mut self) -> Result<u64, CoreError> {
        let mut b = [0u8; 8];
        b.copy_from_slice(self.read(8)?);
        Ok(u64::from_le_bytes(b))
    }

    fn fstring(&mut self) -> Result<String, CoreError> {
        let n = self.i32()?;
        if n == 0 {
            return Ok(String::new());
        }
        if n > 0 {
            let raw = self.read(n as usize)?;
            let body = raw.strip_suffix(&[0]).unwrap_or(raw);
            return Ok(String::from_utf8_lossy(body).to_string());
        }
        let chars = (-n) as usize;
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
                .map(binary_host_backend_from_config)
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
                .map(binary_host_backend_from_config)
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
            let backend = binary_host_backend_from_config(binary_host)?;
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
                .map(binary_host_backend_from_config)
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
        return Ok(SaveDirSummary::default());
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
    let mut out = Vec::with_capacity(stream.summary_uncompressed_size as usize);
    for chunk in &stream.chunks {
        if chunk.compressed_size != chunk.uncompressed_size {
            return None;
        }
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
    for entry in fs::read_dir(parent)? {
        let entry = entry?;
        let backup_path = entry.path();
        let Some(file_name) = backup_path.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        if !file_name.starts_with(&prefix) {
            continue;
        }
        let data = fs::read(&backup_path)?;
        let metadata = entry.metadata()?;
        let created_epoch = parse_backup_epoch(file_name, &prefix);
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
            file_name: file_name.to_string(),
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
    for entry in fs::read_dir(parent)? {
        let entry = entry?;
        let backup_path = entry.path();
        let Some(file_name) = backup_path.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        if !file_name.starts_with(&prefix) {
            continue;
        }
        let data = fs::read(&backup_path)?;
        let metadata = entry.metadata()?;
        let created_epoch = parse_backup_epoch(file_name, &prefix);
        let (status, player_save_name, slot_name) =
            match inspect_bytes(&data, Some(&backup_path), false) {
                Ok(_) => {
                    let persistent_slots = parse_persistent_slot_metadata(&data);
                    match persistent_slots.get(slot) {
                        Some(metadata) => (
                            "ok".to_string(),
                            metadata.player_save_name.clone(),
                            metadata
                                .slot_name
                                .clone()
                                .or_else(|| Some(slot.to_string())),
                        ),
                        None => (
                            "selected slot metadata missing".to_string(),
                            None,
                            Some(slot.to_string()),
                        ),
                    }
                }
                Err(err) => (err.to_string(), None, Some(slot.to_string())),
            };
        backups.push(BackupListItem {
            path: backup_path.display().to_string(),
            file_name: file_name.to_string(),
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

    let original = fs::read(path)?;
    inspect_bytes(&original, Some(path), false)?;

    // Discover and validate the paired companion rollback *before* mutating the
    // slot file, so a companion failure aborts the whole restore instead of
    // leaving the slot restored while PersistentDataList.sav stays out of sync.
    let companion_plan = prepare_paired_persistent_data_list_restore(path, backup_path)?;

    // Take safety backups of both files up front.
    let current_backup_path = create_backup_copy(path)?;
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
    for entry in fs::read_dir(parent)? {
        let entry = entry?;
        let candidate = entry.path();
        let Some(name) = candidate.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        if name.strip_prefix(&companion_prefix) == Some(slot_suffix) {
            companion_backup_path = Some(candidate);
            break;
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
    if fs::canonicalize(save_parent)? != fs::canonicalize(backup_parent)? {
        return Err(CoreError::InvalidRequest(
            "backupPath must be next to the selected save file".to_string(),
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

/// Replace `target` with the staged file at `staged` without ever leaving
/// `target` missing on failure. Windows `rename` cannot overwrite, so the
/// current file is moved aside first; if renaming the staged file in fails, the
/// aside copy is moved back so the slot is never lost. The returned
/// [`PendingReplace`] must be either committed or rolled back.
fn begin_replace(target: &Path, staged: &Path) -> Result<PendingReplace, CoreError> {
    if !target.exists() {
        fs::rename(staged, target)?;
        return Ok(PendingReplace {
            target: target.to_path_buf(),
            aside: None,
        });
    }
    let aside = target.with_extension("sav.replaced-goresave");
    // Clear any leftover aside from a previously interrupted write.
    let _ = fs::remove_file(&aside);
    fs::rename(target, &aside)?;
    match fs::rename(staged, target) {
        Ok(()) => Ok(PendingReplace {
            target: target.to_path_buf(),
            aside: Some(aside),
        }),
        Err(err) => {
            // Roll back so the target path is never left absent.
            let _ = fs::rename(&aside, target);
            Err(err.into())
        }
    }
}

fn create_backup_copy(path: &Path) -> Result<PathBuf, CoreError> {
    let backup_path = unique_backup_path(path);
    fs::copy(path, &backup_path)?;
    Ok(backup_path)
}

fn unique_backup_path(path: &Path) -> PathBuf {
    let suffix = shared_backup_suffix(std::slice::from_ref(&path));
    backup_path_with_suffix(path, &suffix)
}

fn backup_path_with_suffix(path: &Path, suffix: &str) -> PathBuf {
    path.with_extension(format!("sav.bak.{suffix}"))
}

fn create_backup_with_suffix(path: &Path, suffix: &str) -> Result<PathBuf, CoreError> {
    let backup_path = backup_path_with_suffix(path, suffix);
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
                inspect_private_payload(data, &stream, codec_backend, private_chunk_limit)?;
        }
        if let Some(metadata) = path.and_then(persistent_slot_metadata_for_save) {
            value["persistent"] =
                serde_json::to_value(metadata).map_err(|e| CoreError::Parse(e.to_string()))?;
        }
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
    let mut chunks = Vec::with_capacity(chunk_count);
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
    stream: &CompressedStream,
    codec_backend: Option<&dyn codec_backend::CodecBackend>,
    private_chunk_limit: Option<usize>,
) -> Result<Value, CoreError> {
    let Some(backend) = codec_backend else {
        return kraken::inspect_private_payload(data, stream);
    };
    match decompress_private_payload_with_limit(data, stream, backend, private_chunk_limit) {
        Ok((payload, decoded_chunk_count)) => {
            let refs = scan_fstrings(&payload, 0);
            let strings = refs
                .iter()
                .map(|reference| reference.value.clone())
                .filter(|value| !value.is_empty())
                .take(200)
                .collect::<Vec<_>>();
            let player = summarize_private_player_payload(&payload, &refs);
            let inventory = summarize_private_inventory_payload(&payload, &refs);
            let progression = summarize_private_progression_payload(&refs);
            let preview = decoded_chunk_count < stream.chunk_count;
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
                "writable": ["private.replaceFString"],
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

fn summarize_private_inventory_payload(payload: &[u8], refs: &[FStringRef]) -> Value {
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
    let (items, item_stack_count, item_scope) =
        summarize_private_inventory_items(payload, refs, 200);
    let writable = if item_scope == "player_inventory_region" && item_stack_count > 0 {
        vec!["private.inventory.setItemCount"]
    } else {
        Vec::new()
    };
    json!({
        "candidateCount": candidates.len(),
        "candidates": candidates,
        "itemStackCount": item_stack_count,
        "itemScope": item_scope,
        "items": items,
        "scriptPaths": script_paths,
        "properties": properties,
        "writable": writable,
    })
}

fn summarize_private_progression_payload(refs: &[FStringRef]) -> Value {
    let script_paths = unique_strings(
        refs.iter()
            .map(|r| r.value.as_str())
            .filter(|value| value.starts_with("/Script/") && looks_progression_text(value)),
        80,
    );
    let properties = unique_strings(
        refs.iter()
            .map(|r| r.value.as_str())
            .filter(|value| value.starts_with("m_") && looks_progression_text(value)),
        120,
    );
    let candidates = unique_strings(
        refs.iter()
            .map(|r| r.value.as_str())
            .filter(|value| looks_progression_candidate(value)),
        240,
    );
    let gameplay_tags = unique_strings(
        candidates
            .iter()
            .map(String::as_str)
            .filter(|value| looks_gameplay_tag_candidate(value)),
        240,
    );
    let sections = unique_strings(
        properties
            .iter()
            .filter_map(|property| progression_section_label(property)),
        80,
    );
    json!({
        "candidateCount": candidates.len(),
        "candidates": candidates,
        "gameplayTags": gameplay_tags,
        "sections": sections,
        "scriptPaths": script_paths,
        "properties": properties,
        "writable": [],
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
    let Some(start_idx) = refs.iter().position(|r| r.value == "m_Inventory") else {
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
    Some(*payload.get(cursor + 8)? != 0)
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

fn looks_progression_candidate(value: &str) -> bool {
    if value.starts_with("m_") || value.starts_with("/Script/") || is_property_type_name(value) {
        return false;
    }
    if matches!(
        value,
        "GameplayTag" | "GameplayTagContainer" | "TagName" | "None"
    ) {
        return false;
    }
    looks_progression_text(value) && value.contains('.')
}

fn looks_gameplay_tag_candidate(value: &str) -> bool {
    value.contains('.')
        && !value.starts_with('/')
        && !value.starts_with("m_")
        && looks_progression_text(value)
}

fn looks_progression_text(value: &str) -> bool {
    contains_any_ci(
        value,
        &[
            "quest",
            "dialog",
            "knowledge",
            "event",
            "chapter",
            "guild",
            "faction",
            "mission",
            "journal",
            "progress",
            "gameplaytag",
        ],
    )
}

fn progression_section_label(property: &str) -> Option<&'static str> {
    match property {
        "m_GeneratedEvents" | "GeneratedEvents" => Some("Generated events"),
        "m_MemorizedEvents" | "MemorizedEvents" => Some("Memorized events"),
        "m_ActiveQuestTags" | "ActiveQuestTags" => Some("Active quest tags"),
        "m_ActiveQuests" | "ActiveQuests" => Some("Active quests"),
        "m_CompletedQuests" | "CompletedQuests" => Some("Completed quests"),
        "m_QuestLog" | "QuestLog" => Some("Quest log"),
        "m_Knowledge" | "Knowledge" => Some("Knowledge"),
        _ => None,
    }
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

fn codec_status_from_probes(
    pure_probe: codec_backend::CodecBackendProbe,
    binary_probe: Option<Result<codec_backend::CodecBackendProbe, CoreError>>,
) -> Result<Value, CoreError> {
    let mut backends =
        vec![serde_json::to_value(&pure_probe).map_err(|e| CoreError::Codec(e.to_string()))?];
    let mut selected_probe = pure_probe.clone();

    if let Some(binary_probe) = binary_probe {
        match binary_probe {
            Ok(probe) => {
                backends.push(
                    serde_json::to_value(&probe).map_err(|e| CoreError::Codec(e.to_string()))?,
                );
                if probe.available {
                    selected_probe = probe;
                }
            }
            Err(err) => {
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
    codec_backend::CodecBackend::probe(&backend)
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
    let edit_specs = edits
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
            "private.inventory.setItemCount" => {
                parse_private_inventory_item_count_edit(edit).map(PrivateEdit::InventoryItemCount)
            }
            other => Err(CoreError::UnsupportedEdit(format!(
                "{other} is not writable in this build"
            ))),
        })
        .collect::<Result<Vec<_>, _>>()?;
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

#[derive(Debug, Clone, PartialEq)]
enum PrivateEdit {
    FString(PrivateFStringEdit),
    PlayerName(PrivatePlayerNameEdit),
    ProfileName(PrivateProfileNameEdit),
    PlayerAttribute(PrivatePlayerAttributeEdit),
    PlayerTransform(PrivatePlayerTransformEdit),
    InventoryItemCount(PrivateInventoryItemCountEdit),
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
    let mut out = Vec::with_capacity(expected_size);
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
    }
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
    if let Some(expected_path) = &edit.path {
        return expected_path == path;
    }
    edit.id
        .as_deref()
        .is_some_and(|expected_id| expected_id == item_id_from_path(path))
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

    fn fstring(value: &str) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&((value.len() + 1) as i32).to_le_bytes());
        out.extend_from_slice(value.as_bytes());
        out.push(0);
        out
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
        out.push(u8::from(value));
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
        let older = dir.path().join("G1R-001.sav.bak.100");
        let newer = dir.path().join("G1R-001.sav.bak.200");
        let unrelated = dir.path().join("G1R-002.sav.bak.300");
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
    fn list_backups_returns_persistent_data_list_companion_backups_for_selected_slot() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("G1R-001.sav");
        let companion = dir.path().join("PersistentDataList.sav.bak.250");
        let unrelated = dir.path().join("G1R-002.sav.bak.300");
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
        assert_eq!(companion_backups[0]["status"], "ok");
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
        let backup = dir.path().join("G1R-001.sav.bak.200");
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
    fn restore_backup_also_restores_paired_persistent_data_list_backup() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("G1R-001.sav");
        let slot_backup = dir.path().join("G1R-001.sav.bak.200");
        let persistent = dir.path().join("PersistentDataList.sav");
        let persistent_backup = dir.path().join("PersistentDataList.sav.bak.200");

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
    fn restore_backup_pairs_companion_by_full_suffix_within_same_second() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("G1R-001.sav");
        // Two paired backups created in the same second: ".200" and ".200.1".
        let slot_backup_first = dir.path().join("G1R-001.sav.bak.200");
        let slot_backup_second = dir.path().join("G1R-001.sav.bak.200.1");
        let persistent = dir.path().join("PersistentDataList.sav");
        let persistent_first = dir.path().join("PersistentDataList.sav.bak.200");
        let persistent_second = dir.path().join("PersistentDataList.sav.bak.200.1");

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
    fn restore_backup_aborts_without_touching_slot_when_companion_invalid() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("G1R-001.sav");
        let slot_backup = dir.path().join("G1R-001.sav.bak.200");
        let persistent = dir.path().join("PersistentDataList.sav");
        let persistent_backup = dir.path().join("PersistentDataList.sav.bak.200");

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
    fn default_save_root_derives_from_environment() {
        let suffix = PathBuf::from("G1R").join("Saved").join("SaveGames");

        // Prefers LOCALAPPDATA.
        let from_local = default_save_root_from(
            Some(r"D:\LocalAppData".into()),
            Some(r"D:\Profile".into()),
        );
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
        let name_idx = refs
            .iter()
            .position(|r| r.value == "m_PlayerName")
            .unwrap();
        let type_ref = &refs[name_idx + 1];
        assert_eq!(type_ref.value, "StrProperty");
        let value_ref = &refs[name_idx + 2];
        assert_eq!(value_ref.value, "Nameless");

        // Size word lives 4 bytes after the type FString and must equal the new
        // encoded value length (4-byte len prefix + bytes + NUL).
        let size_offset = type_ref.len_offset + type_ref.total_len + 4;
        let size = u32::from_le_bytes(
            payload[size_offset..size_offset + 4].try_into().unwrap(),
        );
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
                },
                {
                    "id": "ItFo_Cheese",
                    "path": "/Script/Angelscript.ItFo_Cheese",
                    "count": 1,
                }
            ])
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
            })
        );
    }

    #[test]
    fn inspect_save_reports_private_progression_summary() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("G1R-001.sav");
        let private_payload = [
            fstring("/Script/G1R.QuestSaveGameData"),
            fstring("m_GeneratedEvents"),
            fstring("m_MemorizedEvents"),
            fstring("m_ActiveQuestTags"),
            fstring("GameplayTag"),
            fstring("TagName"),
            fstring("Quest.Main.Chapter01"),
            fstring("Dialog.Diego.IntroDone"),
            fstring("Knowledge.OldCamp.PathKnown"),
            fstring("m_ItemCount"),
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

        assert_eq!(value["private"]["progression"]["candidateCount"], 3);
        assert_eq!(
            value["private"]["progression"]["candidates"],
            json!([
                "Quest.Main.Chapter01",
                "Dialog.Diego.IntroDone",
                "Knowledge.OldCamp.PathKnown"
            ])
        );
        assert_eq!(
            value["private"]["progression"]["properties"],
            json!([
                "m_GeneratedEvents",
                "m_MemorizedEvents",
                "m_ActiveQuestTags"
            ])
        );
        assert_eq!(
            value["private"]["progression"]["scriptPaths"],
            json!(["/Script/G1R.QuestSaveGameData"])
        );
        assert_eq!(
            value["private"]["progression"]["sections"],
            json!(["Generated events", "Memorized events", "Active quest tags"])
        );
        assert_eq!(
            value["private"]["progression"]["gameplayTags"],
            json!([
                "Quest.Main.Chapter01",
                "Dialog.Diego.IntroDone",
                "Knowledge.OldCamp.PathKnown"
            ])
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
}
