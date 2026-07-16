//! Exact-current, read-only readiness planning for managed revision-3 Voice content.
//!
//! This route binds the pure Voice build planner to one fully verified managed Store checkpoint.
//! It accepts neither a game installation nor an output path, creates no artifact, and grants no
//! build, deployment, publication, runtime, game-write, or save-write authority.

use std::fs::{self, File, OpenOptions};
use std::io;
use std::path::{Component, Path, PathBuf};

use gore_authoring::{
    plan_revision3_voice_build_v1, AssetVerification, Revision3VoiceBuildPlanEvaluationV1,
    WorkingHead, WorkingProjectStore, WorkingStoreError, WorkingStoreLimits,
    MAX_PROJECT_JSON_BYTES,
};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::err;

pub(super) const COMMAND: &str = "authoring_store_plan_revision3_voice_v1";

const MAX_PATH_BYTES: usize = 32 * 1024;
const MAX_HEAD_JSON_BYTES: usize = 64 * 1024;
const MAX_RESPONSE_BYTES: usize = 4 * 1024 * 1024;
const MAX_ERROR_MESSAGE_BYTES: usize = 4 * 1024;
const MAX_WIRE_BYTES: usize =
    MAX_PROJECT_JSON_BYTES * 2 + MAX_HEAD_JSON_BYTES * 2 + MAX_PATH_BYTES * 2 + 4096;

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExactWireRequest<P> {
    command: String,
    payload: P,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PlanVoiceWirePayload {
    current_project_json: String,
    expected_head_json: String,
    root: String,
}

#[derive(Debug)]
struct Failure {
    code: &'static str,
    message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DirectoryIdentity {
    device: u64,
    inode: u64,
}

#[derive(Debug)]
struct HeldDirectoryIdentity {
    _file: File,
    identity: DirectoryIdentity,
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

pub(super) fn plan_revision3_voice_v1_raw(input: &str) -> Value {
    plan_revision3_voice_v1_inner(input).unwrap_or_else(Failure::response)
}

fn plan_revision3_voice_v1_inner(input: &str) -> Result<Value, Failure> {
    plan_revision3_voice_v1_inner_with_guard(input, |_| {})
}

fn plan_revision3_voice_v1_inner_with_guard<F>(
    input: &str,
    mut after_plan_guard: F,
) -> Result<Value, Failure>
where
    F: FnMut(&Path),
{
    let payload: PlanVoiceWirePayload = parse_exact_wire(input)?;
    validate_path(&payload.root)?;
    if payload.current_project_json.is_empty()
        || payload.current_project_json.len() > MAX_PROJECT_JSON_BYTES
    {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_PLAN_INPUT_LIMIT",
            "current_project_json is empty or exceeds its bounded transport limit",
        ));
    }
    let expected_head = parse_canonical_head(&payload.expected_head_json)?;
    let requested_root = Path::new(&payload.root);
    let canonical_root = canonical_existing_directory_no_reparse(requested_root)?;
    let held_root = hold_directory_identity(&canonical_root).map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_VOICE_PLAN_STORE_UNAVAILABLE",
            "managed Store root identity could not be captured safely",
        )
    })?;

    let store = WorkingProjectStore::open_existing(&canonical_root, WorkingStoreLimits::default())
        .map_err(map_store_error)?;
    let basis = store
        .open_current_revision3(AssetVerification::Full)
        .map_err(map_store_error)?;
    if basis.head != expected_head {
        return Err(head_conflict());
    }
    let canonical_project = basis.project.to_canonical_json().map_err(|_| invariant())?;
    if canonical_project != payload.current_project_json {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_PLAN_PROJECT_CONFLICT",
            "current_project_json differs from the exact published revision-3 project",
        ));
    }
    // Planner and bounded-wire failures are evidence about this exact basis, not permission to
    // skip the closing Store/root audit. Hold either failure until both mutable windows have been
    // closed; an intervening Store/root change must still win over stale project diagnostics.
    let evaluation = validate_signed_wire_values(&basis.project).and_then(|()| {
        plan_revision3_voice_build_v1(&basis.project).map_err(|error| {
            Failure::new(
                "AUTHORING_REVISION3_VOICE_PLAN_PROJECT_INVALID",
                error.to_string(),
            )
        })
    });
    after_plan_guard(&canonical_root);

    // Close both mutable windows before returning readiness evidence: all Store assets are fully
    // reopened and the caller's root spelling must still identify the same safe real directory.
    let after = store
        .open_current_revision3(AssetVerification::Full)
        .map_err(map_store_error)?;
    if after.head != expected_head || after.project != basis.project {
        return Err(head_conflict());
    }
    let revalidated_root =
        canonical_existing_directory_no_reparse(requested_root).map_err(|_| {
            Failure::new(
            "AUTHORING_REVISION3_VOICE_PLAN_STORE_ROOT_CHANGED",
            "the managed Store root became unavailable or changed identity during Voice planning",
        )
        })?;
    let revalidated_identity = hold_directory_identity(&revalidated_root).map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_VOICE_PLAN_STORE_ROOT_CHANGED",
            "the managed Store root identity became unavailable during Voice planning",
        )
    })?;
    if revalidated_root != canonical_root || revalidated_identity.identity != held_root.identity {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_PLAN_STORE_ROOT_CHANGED",
            "the managed Store root changed identity during Voice planning",
        ));
    }
    let evaluation = evaluation?;

    let basis_head_json = canonical_head_json(&basis.head)?;
    let common = |outcome: &'static str, total_slots: u64, ready_slots: u64, blockers: Value| {
        json!({
            "ok": true,
            "outcome": outcome,
            "basis_head_json": basis_head_json,
            "project_id": basis.project.project_id.to_string(),
            "project_revision": basis.project.revision,
            "total_slots": total_slots,
            "ready_slots": ready_slots,
            "blockers": blockers,
            "plan_authority": "read_only_voice_build_plan_v1",
            "build_authority": "not_granted",
            "deployment_status": "not_performed",
        })
    };
    let response = match evaluation {
        Revision3VoiceBuildPlanEvaluationV1::Ready { plan } => {
            let ready_slots = u64::try_from(plan.edits.len()).map_err(|_| {
                Failure::new(
                    "AUTHORING_REVISION3_VOICE_PLAN_RESPONSE_LIMIT",
                    "ready Voice slot count is outside the bounded wire range",
                )
            })?;
            common("ready", ready_slots, ready_slots, json!([]))
        }
        Revision3VoiceBuildPlanEvaluationV1::Blocked { report } => common(
            "blocked",
            report.total_slots,
            report.ready_slots,
            serde_json::to_value(report.blockers).map_err(|_| invariant())?,
        ),
    };
    enforce_response_budget(response)
}

fn parse_exact_wire<P>(input: &str) -> Result<P, Failure>
where
    P: DeserializeOwned + Serialize,
{
    if input.len() > MAX_WIRE_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_PLAN_INPUT_LIMIT",
            "revision-3 Voice plan request exceeds its bounded wire limit",
        ));
    }
    let request: ExactWireRequest<P> =
        serde_json::from_str(input).map_err(|_| invalid_request())?;
    if request.command != COMMAND {
        return Err(invalid_request());
    }
    let canonical = serde_json::to_string(&request).map_err(|_| invariant())?;
    if canonical != input {
        return Err(invalid_request());
    }
    Ok(request.payload)
}

fn validate_path(path: &str) -> Result<(), Failure> {
    if path.is_empty() || path.len() > MAX_PATH_BYTES || path.contains('\0') {
        return Err(invalid_request());
    }
    Ok(())
}

fn canonical_existing_directory_no_reparse(path: &Path) -> Result<PathBuf, Failure> {
    if path
        .components()
        .any(|component| component == Component::ParentDir)
    {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_PLAN_STORE_UNAVAILABLE",
            "managed Store root must not contain '..' traversal",
        ));
    }
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|_| {
                Failure::new(
                    "AUTHORING_REVISION3_VOICE_PLAN_STORE_UNAVAILABLE",
                    "managed Store root could not be resolved",
                )
            })?
            .join(path)
    };
    for ancestor in absolute.ancestors() {
        let metadata = fs::symlink_metadata(ancestor).map_err(|_| {
            Failure::new(
                "AUTHORING_REVISION3_VOICE_PLAN_STORE_UNAVAILABLE",
                "managed Store root has an unavailable path component",
            )
        })?;
        if metadata_is_reparse(&metadata) || !metadata.is_dir() {
            return Err(Failure::new(
                "AUTHORING_REVISION3_VOICE_PLAN_STORE_UNAVAILABLE",
                "managed Store root crosses a symbolic link, reparse point, or non-directory",
            ));
        }
    }
    fs::canonicalize(&absolute).map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_VOICE_PLAN_STORE_UNAVAILABLE",
            "managed Store root could not be canonicalized",
        )
    })
}

fn metadata_is_reparse(metadata: &fs::Metadata) -> bool {
    if metadata.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt as _;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0000_0400;
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
    Ok(HeldDirectoryIdentity {
        _file: file,
        identity,
    })
}

#[cfg(windows)]
fn open_directory_no_follow(path: &Path) -> io::Result<File> {
    use std::os::windows::fs::OpenOptionsExt as _;
    use windows_sys::Win32::Storage::FileSystem::{
        FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT,
    };

    let mut options = OpenOptions::new();
    options
        .read(true)
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

#[cfg(windows)]
fn directory_identity(file: &File) -> io::Result<DirectoryIdentity> {
    use std::mem::MaybeUninit;
    use std::os::windows::io::AsRawHandle as _;
    use windows_sys::Win32::Foundation::HANDLE;
    use windows_sys::Win32::Storage::FileSystem::{
        GetFileInformationByHandle, BY_HANDLE_FILE_INFORMATION, FILE_ATTRIBUTE_DIRECTORY,
        FILE_ATTRIBUTE_REPARSE_POINT,
    };

    let mut info = MaybeUninit::<BY_HANDLE_FILE_INFORMATION>::uninit();
    // SAFETY: `file` owns a live directory handle and `info` is writable for the exact ABI type.
    if unsafe { GetFileInformationByHandle(file.as_raw_handle() as HANDLE, info.as_mut_ptr()) } == 0
    {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: a successful call initializes the entire structure.
    let info = unsafe { info.assume_init() };
    if info.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY == 0
        || info.dwFileAttributes & FILE_ATTRIBUTE_REPARSE_POINT != 0
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "managed Store root handle is not one real directory",
        ));
    }
    Ok(DirectoryIdentity {
        device: u64::from(info.dwVolumeSerialNumber),
        inode: (u64::from(info.nFileIndexHigh) << 32) | u64::from(info.nFileIndexLow),
    })
}

#[cfg(unix)]
fn directory_identity(file: &File) -> io::Result<DirectoryIdentity> {
    use std::os::unix::fs::MetadataExt as _;

    let metadata = file.metadata()?;
    if !metadata.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "managed Store root handle is not a directory",
        ));
    }
    Ok(DirectoryIdentity {
        device: metadata.dev(),
        inode: metadata.ino(),
    })
}

fn parse_canonical_head(input: &str) -> Result<WorkingHead, Failure> {
    if input.is_empty() || input.len() > MAX_HEAD_JSON_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_PLAN_HEAD_INVALID",
            "expected_head_json is empty or exceeds its bounded transport limit",
        ));
    }
    let head: WorkingHead = serde_json::from_str(input).map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_VOICE_PLAN_HEAD_INVALID",
            "expected_head_json is not one closed working head",
        )
    })?;
    if canonical_head_json(&head)? != input {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_PLAN_HEAD_INVALID",
            "expected_head_json is not duplicate-free canonical JSON",
        ));
    }
    Ok(head)
}

fn canonical_head_json(head: &WorkingHead) -> Result<String, Failure> {
    serde_json::to_string(head).map_err(|_| invariant())
}

fn validate_signed_wire_values(project: &gore_authoring::ProjectRevision3) -> Result<(), Failure> {
    for value in [project.revision, project.target.executable.byte_len] {
        if value > i64::MAX as u64 {
            return Err(Failure::new(
                "AUTHORING_REVISION3_VOICE_PLAN_RESPONSE_LIMIT",
                "revision-3 Voice plan contains an integer outside the signed wire range",
            ));
        }
    }
    Ok(())
}

fn enforce_response_budget(response: Value) -> Result<Value, Failure> {
    let bytes = serde_json::to_vec(&response).map_err(|_| invariant())?;
    if bytes.len() > MAX_RESPONSE_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_PLAN_RESPONSE_LIMIT",
            "revision-3 Voice plan response exceeds its bounded transport budget",
        ));
    }
    Ok(response)
}

fn map_store_error(error: WorkingStoreError) -> Failure {
    use WorkingStoreError::*;
    let code = match error {
        HeadConflict { .. } => "AUTHORING_REVISION3_VOICE_PLAN_HEAD_CONFLICT",
        MissingRoot(_) | MissingObject(_) => "AUTHORING_REVISION3_VOICE_PLAN_STORE_MISSING",
        UnsafePath { .. } => "AUTHORING_REVISION3_VOICE_PLAN_STORE_UNSAFE",
        LimitExceeded { .. } | InvalidLimits { .. } => "AUTHORING_REVISION3_VOICE_PLAN_STORE_LIMIT",
        _ => "AUTHORING_REVISION3_VOICE_PLAN_STORE_INVARIANT",
    };
    Failure::new(code, error.to_string())
}

fn head_conflict() -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_VOICE_PLAN_HEAD_CONFLICT",
        "the published revision-3 project changed during Voice planning",
    )
}

fn invalid_request() -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_VOICE_PLAN_INPUT_INVALID",
        "request must be exact canonical JSON containing command and exactly current_project_json, expected_head_json, and root",
    )
}

fn invariant() -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_VOICE_PLAN_INVARIANT",
        "revision-3 Voice planning could not preserve its exact internal contract",
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
    use std::cell::Cell;
    use std::collections::{BTreeMap, BTreeSet};
    use std::path::{Path, PathBuf};

    use gore_authoring::model_revision3::{
        DialogLine, Entity, EntityKind, EntityPayload, LocalizationEntry,
        OggCodec as ModelOggCodec, OggMetadata as ModelOggMetadata, OriginRef, SchemaRevisionV3,
        TypedRef, VoiceMemberProof, VoiceOperation, VoiceSlot, VoiceTake, VoiceTakeStatus,
        VoiceTarget, VoiceTargetResolution,
    };
    use gore_authoring::{
        ArchiveSeal, AssetMeta, AssetStoreIndex, ContentSeal, EntityId, FormatV2,
        GameGenerationAnchor, LocaleCode, ProjectId, ProjectMeta, ProjectRevision3, Sha256Digest,
        WorkingHead, WorkingProjectStore, WorkingStoreLimits,
    };
    use serde_json::{json, Value};

    use super::{
        plan_revision3_voice_v1_inner_with_guard, ExactWireRequest, PlanVoiceWirePayload, COMMAND,
    };

    fn entity_id(value: u8) -> EntityId {
        EntityId::from_bytes([value; 16])
    }

    fn project_id() -> ProjectId {
        ProjectId::from_bytes([0x31; 16])
    }

    fn digest(value: u8) -> Sha256Digest {
        Sha256Digest::from_bytes([value; 32])
    }

    fn locale() -> LocaleCode {
        "de".parse().unwrap()
    }

    fn new_origin(label: &str) -> OriginRef {
        OriginRef::New {
            authored_runtime_id: label.to_owned(),
        }
    }

    fn resolved_target() -> VoiceTargetResolution {
        VoiceTargetResolution::Resolved {
            target: VoiceTarget {
                archive: "german_new.zip".into(),
                member: "NPC/Asghan/GRD_263_ASGHAN_E2E.ogg".into(),
                operation: VoiceOperation::Replace,
                archive_seal: ArchiveSeal {
                    byte_len: 4096,
                    sha256: digest(0x55),
                },
                member_proof: VoiceMemberProof::Present {
                    uncompressed_size: 1024,
                    crc32: 0x1234_5678,
                },
            },
        }
    }

    fn voice_project(
        imported: &gore_authoring::ImportedOgg,
        target_resolution: VoiceTargetResolution,
        select_take: bool,
    ) -> ProjectRevision3 {
        let project_id = project_id();
        let localization_id = entity_id(1);
        let line_id = entity_id(2);
        let slot_id = entity_id(3);
        let take_id = entity_id(4);
        let locale = locale();
        let authored_ref = |id, kind| TypedRef::new(project_id, id, kind);
        ProjectRevision3 {
            format: FormatV2,
            schema_revision: SchemaRevisionV3,
            project_id,
            revision: 5,
            meta: ProjectMeta {
                name: "ManagedVoicePlan".into(),
                version: "1.0.0".into(),
                author: "gore-ffi tests".into(),
            },
            target: GameGenerationAnchor {
                executable: ContentSeal {
                    byte_len: 1234,
                    sha256: digest(0x77),
                },
            },
            authoring_locales: BTreeSet::from([locale.clone()]),
            entities: BTreeMap::from([
                (
                    localization_id,
                    Entity {
                        id: localization_id,
                        display_name: "Asghan line".into(),
                        origin: new_origin("loc:asghan:plan"),
                        revision: 1,
                        payload: EntityPayload::LocalizationEntry(LocalizationEntry {
                            loc_id: "GRD_263_ASGHAN_E2E".into(),
                            texts: BTreeMap::from([(locale.clone(), "Geh weiter.".into())]),
                        }),
                    },
                ),
                (
                    line_id,
                    Entity {
                        id: line_id,
                        display_name: "Asghan greeting".into(),
                        origin: new_origin("dialog:asghan:plan"),
                        revision: 1,
                        payload: EntityPayload::DialogLine(DialogLine {
                            localization: authored_ref(
                                localization_id,
                                EntityKind::LocalizationEntry,
                            ),
                            speaker_hint: Some("Asghan".into()),
                            voice_slots: BTreeMap::from([(
                                locale.clone(),
                                authored_ref(slot_id, EntityKind::VoiceSlot),
                            )]),
                        }),
                    },
                ),
                (
                    slot_id,
                    Entity {
                        id: slot_id,
                        display_name: "Asghan DE".into(),
                        origin: new_origin("voice-slot:asghan:plan:de"),
                        revision: 1,
                        payload: EntityPayload::VoiceSlot(VoiceSlot {
                            locale: locale.clone(),
                            target_resolution,
                            candidates: vec![authored_ref(take_id, EntityKind::VoiceTake)],
                            selected: select_take
                                .then(|| authored_ref(take_id, EntityKind::VoiceTake)),
                        }),
                    },
                ),
                (
                    take_id,
                    Entity {
                        id: take_id,
                        display_name: "Approved Asghan take".into(),
                        origin: OriginRef::Imported {
                            importer: "gore-ffi-plan-test".into(),
                            source_seal: ContentSeal {
                                byte_len: imported.asset.byte_len,
                                sha256: imported.asset.sha256,
                            },
                            external_identity: None,
                        },
                        revision: 1,
                        payload: EntityPayload::VoiceTake(VoiceTake {
                            locale,
                            asset: imported.asset.clone(),
                            ogg: ModelOggMetadata {
                                codec: match imported.ogg.codec {
                                    gore_authoring::OggCodec::Vorbis => ModelOggCodec::Vorbis,
                                    gore_authoring::OggCodec::Opus => ModelOggCodec::Opus,
                                },
                                channels: imported.ogg.channels,
                                sample_rate: imported.ogg.sample_rate,
                                pages: imported.ogg.pages,
                                logical_streams: imported.ogg.logical_streams,
                            },
                            status: VoiceTakeStatus::Approved,
                        }),
                    },
                ),
            ]),
            asset_store: AssetStoreIndex {
                assets: BTreeMap::from([(
                    imported.asset.sha256,
                    AssetMeta {
                        byte_len: imported.asset.byte_len,
                        media_type: "audio/ogg".into(),
                    },
                )]),
            },
        }
    }

    fn publish_fixture(
        parent: &Path,
        target_resolution: VoiceTargetResolution,
        select_take: bool,
    ) -> (PathBuf, ProjectRevision3, WorkingHead) {
        publish_fixture_with_project_mutation(parent, target_resolution, select_take, |_| {})
    }

    fn publish_fixture_with_project_mutation<F>(
        parent: &Path,
        target_resolution: VoiceTargetResolution,
        select_take: bool,
        mutate_project: F,
    ) -> (PathBuf, ProjectRevision3, WorkingHead)
    where
        F: FnOnce(&mut ProjectRevision3),
    {
        let store_root = parent.join("store");
        let source = parent.join("take.ogg");
        std::fs::write(
            &source,
            include_bytes!("../../gore-vo/testdata/tiny-vorbis.ogg"),
        )
        .unwrap();
        let store = WorkingProjectStore::at(&store_root, WorkingStoreLimits::default()).unwrap();
        let imported = store.import_ogg(&source, "asghan-plan.ogg", None).unwrap();
        let mut project = voice_project(&imported, target_resolution, select_take);
        mutate_project(&mut project);
        let prepared = store.prepare_revision3_checkpoint(None, &project).unwrap();
        std::fs::write(store_root.join("gore-project.json"), &prepared.head_bytes).unwrap();
        (store_root, project, prepared.head)
    }

    fn request(store_root: &Path, project: &ProjectRevision3, head: &WorkingHead) -> String {
        serde_json::to_string(&ExactWireRequest {
            command: COMMAND.to_owned(),
            payload: PlanVoiceWirePayload {
                current_project_json: project.to_canonical_json().unwrap(),
                expected_head_json: serde_json::to_string(head).unwrap(),
                root: store_root.to_str().unwrap().to_owned(),
            },
        })
        .unwrap()
    }

    fn execute(input: &str) -> Value {
        serde_json::from_str(&crate::execute_json(input)).unwrap()
    }

    fn read_tree(root: &Path) -> BTreeMap<String, Vec<u8>> {
        fn visit(root: &Path, current: &Path, files: &mut BTreeMap<String, Vec<u8>>) {
            for entry in std::fs::read_dir(current).unwrap() {
                let entry = entry.unwrap();
                let path = entry.path();
                if entry.file_type().unwrap().is_dir() {
                    visit(root, &path, files);
                } else {
                    let relative = path
                        .strip_prefix(root)
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .replace('\\', "/");
                    files.insert(relative, std::fs::read(path).unwrap());
                }
            }
        }
        let mut files = BTreeMap::new();
        visit(root, root, &mut files);
        files
    }

    fn copy_tree(source: &Path, destination: &Path) {
        std::fs::create_dir(destination).unwrap();
        for entry in std::fs::read_dir(source).unwrap() {
            let entry = entry.unwrap();
            let source_path = entry.path();
            let destination_path = destination.join(entry.file_name());
            if entry.file_type().unwrap().is_dir() {
                copy_tree(&source_path, &destination_path);
            } else {
                std::fs::copy(source_path, destination_path).unwrap();
            }
        }
    }

    #[cfg(unix)]
    fn make_test_dir_link(target: &Path, link: &Path) -> bool {
        std::os::unix::fs::symlink(target, link).unwrap();
        true
    }

    #[cfg(windows)]
    fn make_test_dir_link(target: &Path, link: &Path) -> bool {
        std::process::Command::new("cmd")
            .args(["/c", "mklink", "/J"])
            .arg(link)
            .arg(target)
            .status()
            .is_ok_and(|status| status.success())
    }

    #[test]
    fn ready_plan_is_exact_read_only_evidence_without_artifact_authority() {
        let temp = tempfile::tempdir().unwrap();
        let (store_root, project, head) = publish_fixture(temp.path(), resolved_target(), true);
        let before = read_tree(temp.path());

        let response = execute(&request(&store_root, &project, &head));

        assert_eq!(response["ok"], true, "{response}");
        assert_eq!(response["outcome"], "ready");
        assert_eq!(
            response["basis_head_json"],
            serde_json::to_string(&head).unwrap()
        );
        assert_eq!(response["project_id"], project.project_id.to_string());
        assert_eq!(response["project_revision"], 5);
        assert_eq!(response["total_slots"], 1);
        assert_eq!(response["ready_slots"], 1);
        assert_eq!(response["blockers"], json!([]));
        assert_eq!(response["plan_authority"], "read_only_voice_build_plan_v1");
        assert_eq!(response["build_authority"], "not_granted");
        assert_eq!(response["deployment_status"], "not_performed");
        let keys: BTreeSet<_> = response
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        assert_eq!(
            keys,
            BTreeSet::from([
                "basis_head_json",
                "blockers",
                "build_authority",
                "deployment_status",
                "ok",
                "outcome",
                "plan_authority",
                "project_id",
                "project_revision",
                "ready_slots",
                "total_slots",
            ])
        );
        assert_eq!(read_tree(temp.path()), before);
    }

    #[test]
    fn blocked_plan_returns_every_exact_line_locale_blocker_without_writing() {
        let temp = tempfile::tempdir().unwrap();
        let (store_root, project, head) =
            publish_fixture(temp.path(), VoiceTargetResolution::Unresolved, false);
        let before = read_tree(temp.path());

        let response = execute(&request(&store_root, &project, &head));

        assert_eq!(response["ok"], true, "{response}");
        assert_eq!(response["outcome"], "blocked");
        assert_eq!(response["total_slots"], 1);
        assert_eq!(response["ready_slots"], 0);
        assert_eq!(response["blockers"].as_array().unwrap().len(), 2);
        let reasons: Vec<_> = response["blockers"]
            .as_array()
            .unwrap()
            .iter()
            .map(|blocker| blocker["reason"].as_str().unwrap())
            .collect();
        assert_eq!(reasons, ["unresolved_target", "missing_selected_take"]);
        for blocker in response["blockers"].as_array().unwrap() {
            assert_eq!(blocker["slot_id"], entity_id(3).to_string());
            assert_eq!(blocker["line_id"], entity_id(2).to_string());
            assert_eq!(blocker["line_label"], "Asghan greeting");
            assert_eq!(blocker["loc_id"], "GRD_263_ASGHAN_E2E");
            assert_eq!(blocker["locale"], "de");
        }
        assert_eq!(response["build_authority"], "not_granted");
        assert_eq!(read_tree(temp.path()), before);
    }

    #[test]
    fn project_head_and_request_shape_are_exactly_bound() {
        let temp = tempfile::tempdir().unwrap();
        let (store_root, project, head) = publish_fixture(temp.path(), resolved_target(), true);

        let mut foreign_project = project.clone();
        foreign_project.revision += 1;
        let project_conflict = execute(&request(&store_root, &foreign_project, &head));
        assert_eq!(
            project_conflict["error"]["code"],
            "AUTHORING_REVISION3_VOICE_PLAN_PROJECT_CONFLICT"
        );

        let mut foreign_head = head.clone();
        foreign_head.snapshot.byte_len += 1;
        let wrong_head = serde_json::to_string(&ExactWireRequest {
            command: COMMAND.to_owned(),
            payload: PlanVoiceWirePayload {
                current_project_json: project.to_canonical_json().unwrap(),
                expected_head_json: serde_json::to_string(&foreign_head).unwrap(),
                root: store_root.display().to_string(),
            },
        })
        .unwrap();
        assert_eq!(
            execute(&wrong_head)["error"]["code"],
            "AUTHORING_REVISION3_VOICE_PLAN_HEAD_CONFLICT"
        );

        let canonical = request(&store_root, &project, &head);
        assert_eq!(
            execute(&format!(" {canonical}"))["error"]["code"],
            "AUTHORING_REVISION3_VOICE_PLAN_INPUT_INVALID"
        );
        for forbidden in ["output", "game_root"] {
            let mut extra: Value = serde_json::from_str(&canonical).unwrap();
            extra["payload"][forbidden] =
                json!(temp.path().join("forbidden").display().to_string());
            assert_eq!(
                execute(&serde_json::to_string(&extra).unwrap())["error"]["code"],
                "AUTHORING_REVISION3_VOICE_PLAN_INPUT_INVALID",
                "forbidden authority field {forbidden} was accepted"
            );
        }
    }

    #[test]
    fn traversal_root_is_rejected_before_store_access() {
        let temp = tempfile::tempdir().unwrap();
        let (store_root, project, head) = publish_fixture(temp.path(), resolved_target(), true);
        let traversing = store_root.join("..").join("store");

        let response = execute(&request(&traversing, &project, &head));

        assert_eq!(
            response["error"]["code"],
            "AUTHORING_REVISION3_VOICE_PLAN_STORE_UNAVAILABLE"
        );
    }

    #[test]
    fn linked_store_root_is_rejected_without_following_it() {
        let temp = tempfile::tempdir().unwrap();
        let (store_root, project, head) = publish_fixture(temp.path(), resolved_target(), true);
        let alias_parent = tempfile::tempdir().unwrap();
        let alias = alias_parent.path().join("store-alias");
        if !make_test_dir_link(&store_root, &alias) {
            return;
        }
        let before = read_tree(&store_root);

        let response = execute(&request(&alias, &project, &head));

        assert_eq!(
            response["error"]["code"],
            "AUTHORING_REVISION3_VOICE_PLAN_STORE_UNAVAILABLE"
        );
        assert_eq!(read_tree(&store_root), before);
        #[cfg(unix)]
        std::fs::remove_file(alias).unwrap();
        #[cfg(windows)]
        std::fs::remove_dir(alias).unwrap();
    }

    #[test]
    fn later_head_publication_is_rejected_by_the_closing_full_open() {
        let temp = tempfile::tempdir().unwrap();
        let (store_root, project, head) = publish_fixture(temp.path(), resolved_target(), true);
        let input = request(&store_root, &project, &head);
        let mut later_project = project.clone();
        later_project.revision += 1;
        later_project.meta.version = "later".into();

        let failure = plan_revision3_voice_v1_inner_with_guard(&input, |root| {
            let store =
                WorkingProjectStore::open_existing(root, WorkingStoreLimits::default()).unwrap();
            let prepared = store
                .prepare_revision3_checkpoint(Some(&head), &later_project)
                .unwrap();
            std::fs::write(root.join("gore-project.json"), prepared.head_bytes).unwrap();
        })
        .unwrap_err();

        assert_eq!(failure.code, "AUTHORING_REVISION3_VOICE_PLAN_HEAD_CONFLICT");
    }

    #[test]
    fn invalid_project_still_closes_the_store_window_before_returning_its_diagnostic() {
        let temp = tempfile::tempdir().unwrap();
        let (store_root, project, head) = publish_fixture_with_project_mutation(
            temp.path(),
            resolved_target(),
            true,
            |project| project.meta.name = "../unsafe-bundle-name".into(),
        );
        let input = request(&store_root, &project, &head);
        let mut later_project = project.clone();
        later_project.revision += 1;
        later_project.meta.version = "later".into();

        let failure = plan_revision3_voice_v1_inner_with_guard(&input, |root| {
            let store =
                WorkingProjectStore::open_existing(root, WorkingStoreLimits::default()).unwrap();
            let prepared = store
                .prepare_revision3_checkpoint(Some(&head), &later_project)
                .unwrap();
            std::fs::write(root.join("gore-project.json"), prepared.head_bytes).unwrap();
        })
        .unwrap_err();

        assert_eq!(failure.code, "AUTHORING_REVISION3_VOICE_PLAN_HEAD_CONFLICT");
    }

    #[test]
    fn deferred_project_and_wire_errors_return_after_an_unchanged_closing_audit() {
        let invalid_temp = tempfile::tempdir().unwrap();
        let (invalid_root, invalid_project, invalid_head) = publish_fixture_with_project_mutation(
            invalid_temp.path(),
            resolved_target(),
            true,
            |project| project.meta.name = "../unsafe-bundle-name".into(),
        );
        let invalid_failure = plan_revision3_voice_v1_inner_with_guard(
            &request(&invalid_root, &invalid_project, &invalid_head),
            |_| {},
        )
        .unwrap_err();
        assert_eq!(
            invalid_failure.code,
            "AUTHORING_REVISION3_VOICE_PLAN_PROJECT_INVALID"
        );

        let limited_temp = tempfile::tempdir().unwrap();
        let (limited_root, limited_project, limited_head) = publish_fixture_with_project_mutation(
            limited_temp.path(),
            resolved_target(),
            true,
            |project| project.revision = i64::MAX as u64 + 1,
        );
        let limited_failure = plan_revision3_voice_v1_inner_with_guard(
            &request(&limited_root, &limited_project, &limited_head),
            |_| {},
        )
        .unwrap_err();
        assert_eq!(
            limited_failure.code,
            "AUTHORING_REVISION3_VOICE_PLAN_RESPONSE_LIMIT"
        );
    }

    #[test]
    fn early_response_limit_still_audits_the_store_root_identity() {
        let temp = tempfile::tempdir().unwrap();
        let (store_root, project, head) = publish_fixture_with_project_mutation(
            temp.path(),
            resolved_target(),
            true,
            |project| project.revision = i64::MAX as u64 + 1,
        );
        let input = request(&store_root, &project, &head);
        let displaced = temp.path().join("displaced-limited-store");
        let swapped = Cell::new(false);

        let outcome = plan_revision3_voice_v1_inner_with_guard(&input, |root| {
            // Some hosts deny renaming an open directory. Such a host cannot exercise this race,
            // so keep the same narrow skip used by the ordinary root-identity regression below.
            if std::fs::rename(root, &displaced).is_err() {
                return;
            }
            copy_tree(&displaced, root);
            swapped.set(true);
        });
        if !swapped.get() {
            return;
        }

        let failure = outcome.unwrap_err();
        assert_eq!(
            failure.code,
            "AUTHORING_REVISION3_VOICE_PLAN_STORE_ROOT_CHANGED"
        );
    }

    #[test]
    fn byte_identical_same_path_store_replacement_is_rejected_by_directory_identity() {
        let temp = tempfile::tempdir().unwrap();
        let (store_root, project, head) = publish_fixture(temp.path(), resolved_target(), true);
        let input = request(&store_root, &project, &head);
        let displaced = temp.path().join("displaced-store");
        let swapped = Cell::new(false);

        let outcome = plan_revision3_voice_v1_inner_with_guard(&input, |root| {
            // Some hosts deny renaming an open directory even when the handle was opened with
            // delete sharing. That host cannot exercise this race, so leave the test skipped.
            if std::fs::rename(root, &displaced).is_err() {
                return;
            }
            copy_tree(&displaced, root);
            swapped.set(true);
        });
        if !swapped.get() {
            return;
        }

        let failure = outcome.unwrap_err();
        assert_eq!(
            failure.code,
            "AUTHORING_REVISION3_VOICE_PLAN_STORE_ROOT_CHANGED"
        );
    }
}
