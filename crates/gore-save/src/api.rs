//! Native access to the same request contract used by the Save Editor FFI.
use crate::CoreError;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Request {
    pub command: String,
    #[serde(default)]
    pub payload: Value,
}

pub fn execute(request: &Request) -> Result<Value, CoreError> {
    crate::execute_json_inner(
        &serde_json::to_string(request).map_err(|e| CoreError::InvalidRequest(e.to_string()))?,
    )
}

pub fn error_details(error: &CoreError) -> Value {
    let code = match error {
        CoreError::InvalidRequest(_) => "INVALID_REQUEST",
        CoreError::Io(_) => "IO_ERROR",
        CoreError::Parse(_) => "PARSE_ERROR",
        CoreError::UnsupportedEdit(_) => "UNSUPPORTED_EDIT",
        CoreError::Codec(_) => "CODEC_ERROR",
        CoreError::Validation(_) => "VALIDATION_FAILED",
        CoreError::Update(_) => "UPDATE_ERROR",
        CoreError::PlanConflict { .. } => "PLAN_CONFLICT",
    };
    let mut data = json!({"code":code,"message":error.to_string()});
    if let CoreError::PlanConflict { kind, path } = error {
        data["kind"] = json!(kind);
        data["path"] = json!(path);
    }
    data
}

pub const COMMANDS: &[&str] = &[
    "scan_save_dir",
    "scan_save_dir_readonly",
    "warm_save",
    "inspect_save",
    "check_codec",
    "search_typed_properties",
    "query_progression",
    "private.skills.list",
    "private.inventory.list",
    "private.npc.list",
    "private.characters.list",
    "private.npc.attributes",
    "private.npc.position",
    "private.npc.inventory",
    "private.factions.list",
    "private.locks.list",
    "private.traders.list",
    "private.traders.detail",
    "validate_roundtrip",
    "list_backups",
    "restore_backup",
    "restore_deleted_save",
    "dismiss_deleted_save_recovery",
    "delete_backup",
    "rename_backup",
    "validate_codec_roundtrip",
    "write_save",
    "write_difficulty",
    "assign_save_profile",
    "remove_save_from_profile",
    "delete_save",
    "loc_status",
    "loc_find",
    "loc_extract",
    "item_icons_prepare",
    "item_icons_source_identity",
    "item_icons_release",
    "plan_edits",
    "apply_edits",
    "capabilities",
    "recovery_status",
];
pub const EDITS: &[&str] = &[
    "public.m_PlayerSaveName",
    "private.replaceFString",
    "private.fstring",
    "private.player.setPlayerName",
    "private.profile.setProfileName",
    "private.player.setAttribute",
    "private.player.setTransform",
    "private.inventory.setItemCount",
    "private.inventory.addItem",
    "private.inventory.removeItem",
    "private.inventory.reset",
    "private.inventory.repairSlots",
    "private.traders.setStock",
    "private.traders.addItem",
    "private.traders.removeItem",
    "private.story.apply",
    "private.typed.setValue",
    "private.typed.setAdd",
    "private.typed.setRemove",
    "private.typed.arrayRemove",
    "private.typed.arrayDuplicate",
    "private.npc.revive",
    "private.npc.setRelationship",
    "private.glossary.setSegment",
    "private.factions.forgive",
    "private.knowledge.addCharacter",
    "private.knowledge.setEntry",
    "private.locks.setUnlocked",
    "private.skills.set",
];

pub fn mutates(command: &str) -> bool {
    matches!(
        command,
        "scan_save_dir"
            | "restore_backup"
            | "restore_deleted_save"
            | "dismiss_deleted_save_recovery"
            | "delete_backup"
            | "rename_backup"
            | "write_save"
            | "write_difficulty"
            | "assign_save_profile"
            | "remove_save_from_profile"
            | "delete_save"
            | "loc_extract"
            | "item_icons_prepare"
            | "item_icons_release"
            | "apply_edits"
    )
}

pub fn editor_parity() -> Value {
    serde_json::from_str(include_str!("../../../docs/save-editor-parity.json"))
        .expect("Save Editor parity manifest")
}

pub fn capabilities() -> Value {
    json!({"format": "gore.save.capabilities.v1", "commands": COMMANDS.iter()
        .map(|command| json!({"command": command, "mutates": mutates(command)}))
        .collect::<Vec<_>>(), "edits": EDITS,"editorParity":editor_parity()})
}

pub fn file_sha1(path: &std::path::Path) -> Result<String, CoreError> {
    Ok(crate::sha1_hex(&std::fs::read(path)?))
}

pub fn validate_output_path(
    path: &std::path::Path,
    output: &std::path::Path,
) -> Result<(), CoreError> {
    let source = std::fs::File::open(path)?;
    let output = match std::fs::File::open(output) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.into()),
    };
    #[cfg(unix)]
    let same = {
        use std::os::unix::fs::MetadataExt;
        let left = source.metadata()?;
        let right = output.metadata()?;
        (left.dev(), left.ino()) == (right.dev(), right.ino())
    };
    #[cfg(windows)]
    let same = {
        use std::os::windows::io::AsRawHandle;
        use windows_sys::Win32::Storage::FileSystem::{
            BY_HANDLE_FILE_INFORMATION, GetFileInformationByHandle,
        };
        let identity = |file: &std::fs::File| -> Result<_, CoreError> {
            let mut info = unsafe { std::mem::zeroed::<BY_HANDLE_FILE_INFORMATION>() };
            // SAFETY: the file owns a valid handle and info is a writable Win32 buffer.
            if unsafe { GetFileInformationByHandle(file.as_raw_handle(), &mut info) } == 0 {
                return Err(std::io::Error::last_os_error().into());
            }
            Ok((
                info.dwVolumeSerialNumber,
                info.nFileIndexHigh,
                info.nFileIndexLow,
            ))
        };
        identity(&source)? == identity(&output)?
    };
    #[cfg(not(any(unix, windows)))]
    let same = return Err(CoreError::InvalidRequest(
        "output file identity checks are unsupported on this platform".into(),
    ));
    if same {
        return Err(CoreError::InvalidRequest(
            "outputPath must not refer to the source save".into(),
        ));
    }
    Ok(())
}

pub fn check_edit_persistent_snapshots(
    path: &std::path::Path,
    edits: &[Value],
) -> Result<(), CoreError> {
    for edit in edits {
        if edit["path"] == "private.inventory.reset" {
            if let Some(profile) = edit["value"]["persistentPath"].as_str() {
                check_persistent_snapshot_at(std::path::Path::new(profile), &edit["value"])?;
            } else {
                check_persistent_snapshot(path, &edit["value"])?;
            }
        }
    }
    Ok(())
}

pub fn refresh_edit_persistent_snapshots(
    path: &std::path::Path,
    edits: &mut [Value],
    result: &Value,
) {
    let Some(written) = result["persistentWrittenSha1"].as_str() else {
        return;
    };
    let Some(profile) = result["persistentPath"].as_str() else {
        return;
    };
    let physical = |path: &std::path::Path| path.canonicalize().unwrap_or_else(|_| path.to_owned());
    let profile = physical(std::path::Path::new(profile));
    let source = physical(path);
    let fallback = source
        .parent()
        .unwrap_or(std::path::Path::new("."))
        .join("PersistentDataList.sav");
    for edit in edits {
        if edit["path"] != "private.inventory.reset"
            || edit["value"]["expectedPersistentSha1"] != result["persistentOriginalSha1"]
        {
            continue;
        }
        let dependency = edit["value"]["persistentPath"]
            .as_str()
            .map(std::path::Path::new)
            .unwrap_or(&fallback);
        if physical(dependency) == profile {
            edit["value"]["expectedPersistentSha1"] = json!(written);
        }
    }
}

pub(crate) fn check_persistent_snapshot(
    path: &std::path::Path,
    payload: &Value,
) -> Result<(), CoreError> {
    let physical = path.canonicalize().unwrap_or_else(|_| path.to_owned());
    let profile = physical
        .parent()
        .unwrap_or(std::path::Path::new("."))
        .join("PersistentDataList.sav");
    check_persistent_snapshot_at(&profile, payload)
}

fn check_persistent_snapshot_at(
    profile: &std::path::Path,
    payload: &Value,
) -> Result<(), CoreError> {
    if let Some(expected) = payload.get("expectedPersistentSha1") {
        if !expected.is_null() && !expected.is_string() {
            return Err(CoreError::InvalidRequest(
                "expectedPersistentSha1 must be a hash or null".into(),
            ));
        }
        let actual = if profile.exists() {
            Some(file_sha1(profile)?)
        } else {
            None
        };
        if &json!(actual) != expected {
            return Err(CoreError::Validation(
                "profile changed since draft creation".into(),
            ));
        }
    }
    Ok(())
}

/// Exercise the real image extraction and PNG-seal validation in isolation.
pub fn validate_item_icons(payload: &Value) -> Result<Value, CoreError> {
    let temp = tempfile::tempdir()?;
    let result = crate::item_icons_prepare_with(
        payload,
        crate::ITEM_CATALOG_JSON,
        |explicit| {
            gore_loc::config::game_root(explicit)
                .map_err(|e| CoreError::InvalidRequest(e.to_string()))
        },
        |game, items| {
            gore_tex::item_icons::prepare_item_icon_cache_at(game, items, temp.path(), false)
        },
        || {
            if gore_loc::config::autodetect_disabled() {
                None
            } else {
                gore_loc::discover::find_game_root()
            }
        },
    )?;
    let manifest = gore_tex::item_icons::verified_item_icon_manifest(std::path::Path::new(
        result["manifestPath"]
            .as_str()
            .ok_or_else(|| CoreError::Parse("no simulated icon manifest".into()))?,
    ))
    .map_err(|e| CoreError::Io(e.to_string()))?;
    Ok(
        json!({"dryRun":true,"validated":true,"sourceGamePath":result["sourceGamePath"],"itemCount":manifest.item_count,"buildId":manifest.build_id}),
    )
}

/// Preserve unknown fields and use the core's guarded, same-directory publication.
pub fn update_json_file<F>(path: &std::path::Path, update: F) -> Result<Value, CoreError>
where
    F: FnOnce(Value) -> Result<Value, CoreError>,
{
    let snapshot = crate::snapshot_file(path)?;
    let before = match &snapshot {
        crate::FileSnapshot::Present(bytes) => {
            serde_json::from_slice(bytes).map_err(|e| CoreError::Parse(e.to_string()))?
        }
        crate::FileSnapshot::Missing => json!({}),
    };
    let after = update(before)?;
    publish_json_file(path, &snapshot, after)
}

/// Deliberately discard preferences, including malformed JSON, with a guarded replace.
pub fn reset_json_file(path: &std::path::Path) -> Result<Value, CoreError> {
    let snapshot = crate::snapshot_file(path)?;
    publish_json_file(path, &snapshot, json!({}))
}

fn publish_json_file(
    path: &std::path::Path,
    snapshot: &crate::FileSnapshot,
    after: Value,
) -> Result<Value, CoreError> {
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)?;
    }
    let bytes = serde_json::to_vec_pretty(&after).map_err(|e| CoreError::Parse(e.to_string()))?;
    let staged = crate::ScratchFile::create(path, "tmp-json", &bytes)?;
    crate::begin_replace_if_unchanged(path, staged.path(), snapshot)?.commit();
    Ok(after)
}

pub fn recovery_status(root: &std::path::Path) -> Result<Value, CoreError> {
    let mut manifests = Vec::new();
    if let Ok(entries) = std::fs::read_dir(root.join("goresave_backups")) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with(crate::DELETED_SAVE_RECOVERY_MANIFEST_PREFIX)
                && name.ends_with(crate::DELETED_SAVE_RECOVERY_MANIFEST_SUFFIX)
            {
                if let Ok(mut manifest) = crate::read_deleted_save_recovery_manifest(&entry.path())
                {
                    let Ok(Some(expected)) = crate::validate_discovered_deleted_save_recovery(
                        root,
                        &entry.path(),
                        &manifest,
                        false,
                    ) else {
                        continue;
                    };
                    manifest.persistent_post_delete_sha1 = expected;
                    manifests.push(
                        serde_json::to_value(manifest)
                            .map_err(|e| CoreError::Parse(e.to_string()))?,
                    );
                }
            }
        }
    }
    manifests.sort_by_key(|v| v["createdEpoch"].as_u64().unwrap_or(0));
    Ok(json!({"recoveries":manifests}))
}
