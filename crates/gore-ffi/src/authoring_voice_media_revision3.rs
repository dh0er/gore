//! Exact-current, read-only media QA for one managed revision-3 Voice take.
//!
//! The route reuses the closed Voice preview request and binding, reads only the selected sealed
//! CAS object, and returns pathless media facts. It never materializes media or grants project,
//! game, save, build, deployment, perceptual-quality, audibility, or runtime authority.

use std::path::Path;

use gore_authoring::model_revision3::OggCodec as VoiceOggCodec;
use gore_authoring::{
    bind_revision3_voice_take_preview_v1, inspect_revision3_voice_take_media_qa_v1,
    AssetVerification, ProjectRevision3, Revision3VoiceTakeMediaAssuranceV1,
    Revision3VoiceTakeMediaQaV1, Revision3VoiceTakePreviewBindingV1,
    Revision3VoiceTakePreviewConflictV1, Revision3VoiceTakePreviewOggErrorV1,
    Revision3VoiceTakePreviewRequestJsonErrorV1, Revision3VoiceTakePreviewRequestV1, WorkingHead,
    WorkingProjectStore, WorkingStoreError, WorkingStoreLimits, MAX_PROJECT_JSON_BYTES,
    MAX_REVISION3_VOICE_TAKE_PREVIEW_REQUEST_JSON_BYTES_V1,
};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::err;

pub(super) const COMMAND: &str = "authoring_store_inspect_revision3_voice_take_media_v1";

const MAX_PATH_BYTES: usize = 32 * 1024;
const MAX_RESPONSE_BYTES: usize = 512 * 1024;
const MAX_ERROR_MESSAGE_BYTES: usize = 4 * 1024;
const MAX_WIRE_BYTES: usize =
    MAX_PATH_BYTES * 6 + MAX_REVISION3_VOICE_TAKE_PREVIEW_REQUEST_JSON_BYTES_V1 * 2 + 8 * 1024;

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ExactWireRequest<P> {
    command: String,
    payload: P,
}

/// Field order is part of the exact canonical outer wire.
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct InspectVoiceTakeMediaWirePayload {
    root: String,
    voice_take_preview_request_json: String,
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

pub(super) fn inspect_revision3_voice_take_media_v1_raw(input: &str) -> Value {
    inspect_revision3_voice_take_media_v1_inner(input).unwrap_or_else(Failure::response)
}

fn inspect_revision3_voice_take_media_v1_inner(input: &str) -> Result<Value, Failure> {
    inspect_revision3_voice_take_media_v1_inner_with_seam_and_limit(
        input,
        || {},
        MAX_RESPONSE_BYTES,
    )
}

fn inspect_revision3_voice_take_media_v1_inner_with_seam_and_limit<F>(
    input: &str,
    between_exact_reads: F,
    response_limit: usize,
) -> Result<Value, Failure>
where
    F: FnOnce(),
{
    let payload: InspectVoiceTakeMediaWirePayload = parse_exact_wire(input)?;
    validate_root(&payload.root)?;
    let request =
        Revision3VoiceTakePreviewRequestV1::from_json(&payload.voice_take_preview_request_json)
            .map_err(map_request_error)?;
    require_signed_request(&request)?;

    let store = WorkingProjectStore::open_existing(Path::new(&payload.root), ffi_store_limits())
        .map_err(map_store_open_error)?;
    // This operation is scoped to one selected take. The graph and all referenced object shapes
    // are reopened structurally; the selected CAS object is independently read with a full seal.
    let basis = store
        .open_current_revision3(AssetVerification::Structural)
        .map_err(map_store_open_error)?;
    let binding = bind_revision3_voice_take_preview_v1(&basis.head, &basis.project, &request)
        .map_err(map_binding_conflict)?;
    let source_bytes = store
        .read_verified_ogg_asset(&binding.asset)
        .map_err(map_selected_asset_error)?;
    let media = inspect_revision3_voice_take_media_qa_v1(&source_bytes).map_err(map_media_error)?;
    validate_bound_media(&basis.head, &basis.project, &binding, &media)?;

    let basis_head_json = canonical_head_json(&basis.head)?;
    let response = json!({
        "ok": true,
        "outcome": "media_qa_complete",
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
        "duration_sample_frames": media.duration().sample_frames(),
        "duration_timebase_hz": media.duration().timebase_hz(),
        "assurance": media.assurance(),
        "media_authority": "exact_current_managed_cas_voice_take_media_qa_v1",
        "inspection_scope": "selected_voice_take_media_input_only",
        "quality_status": "not_evaluated",
        "audibility_status": "not_evaluated",
        "project_write_status": "not_performed",
        "game_write_status": "not_performed",
        "save_write_status": "not_performed",
        "build_status": "not_evaluated",
        "deployment_status": "not_performed",
        "runtime_status": "not_qualified",
    });

    between_exact_reads();

    // Close the mutable Store window after every response fact has been derived. Rebind the full
    // graph, reread the selected seal, and rerun codec QA so neither a head race nor a same-path
    // CAS replacement can lend stale facts authority.
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
        .map_err(map_selected_asset_error)?;
    let after_media =
        inspect_revision3_voice_take_media_qa_v1(&after_bytes).map_err(map_media_error)?;
    validate_bound_media(&after.head, &after.project, &after_binding, &after_media)?;
    if after_bytes != source_bytes || after_media != media {
        return Err(asset_invalid(
            "selected VoiceTake media changed during exact media QA",
        ));
    }

    enforce_response_budget(response, response_limit)
}

fn parse_exact_wire<P>(input: &str) -> Result<P, Failure>
where
    P: DeserializeOwned + Serialize,
{
    if input.len() > MAX_WIRE_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_MEDIA_INPUT_LIMIT",
            "revision-3 Voice media QA request exceeds its bounded wire limit",
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

fn validate_root(root: &str) -> Result<(), Failure> {
    if root.is_empty()
        || root.len() > MAX_PATH_BYTES
        || root.contains('\0')
        || !Path::new(root).is_absolute()
    {
        return Err(invalid_request());
    }
    Ok(())
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
        signed_wire_u64(value)?;
    }
    Ok(())
}

fn validate_bound_media(
    head: &WorkingHead,
    project: &ProjectRevision3,
    binding: &Revision3VoiceTakePreviewBindingV1,
    media: &Revision3VoiceTakeMediaQaV1,
) -> Result<(), Failure> {
    for value in [
        head.snapshot.byte_len,
        project.revision,
        binding.project_revision,
        binding.line_revision,
        binding.localization_revision,
        binding.slot_revision,
        binding.take_revision,
        binding.asset.byte_len,
        media.duration().sample_frames(),
        u64::from(media.duration().timebase_hz()),
    ] {
        signed_wire_u64(value)?;
    }
    if media.ogg() != &binding.ogg {
        return Err(asset_invalid(
            "selected VoiceTake Ogg metadata differs from its exact project declaration",
        ));
    }
    if binding.basis_head != *head
        || binding.project_id != project.project_id
        || binding.project_revision != project.revision
        || media.duration().timebase_hz() != binding.ogg.sample_rate
    {
        return Err(invariant());
    }
    let assurance_matches_codec = matches!(
        (binding.ogg.codec, media.assurance()),
        (
            VoiceOggCodec::Vorbis,
            Revision3VoiceTakeMediaAssuranceV1::VorbisFullPcmDecode
        ) | (
            VoiceOggCodec::Opus,
            Revision3VoiceTakeMediaAssuranceV1::OpusPacketAndTimingStructureOnly
        )
    );
    if !assurance_matches_codec {
        return Err(invariant());
    }
    Ok(())
}

fn signed_wire_u64(value: u64) -> Result<(), Failure> {
    if value > i64::MAX as u64 {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_MEDIA_SIGNED_WIRE_LIMIT",
            "Voice media QA contains an integer outside the signed transport range",
        ));
    }
    Ok(())
}

fn canonical_head_json(head: &WorkingHead) -> Result<String, Failure> {
    serde_json::to_string(head).map_err(|_| invariant())
}

fn enforce_response_budget(response: Value, limit: usize) -> Result<Value, Failure> {
    let bytes = serde_json::to_vec(&response).map_err(|_| invariant())?;
    if bytes.len() > limit {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_MEDIA_RESPONSE_LIMIT",
            "revision-3 Voice media QA response exceeds its bounded transport budget",
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
            "AUTHORING_REVISION3_VOICE_MEDIA_INPUT_LIMIT",
            "voice_take_preview_request_json exceeds its bounded transport limit",
        ),
        _ => invalid_request(),
    }
}

fn map_binding_conflict(error: Revision3VoiceTakePreviewConflictV1) -> Failure {
    use Revision3VoiceTakePreviewConflictV1::*;
    let (code, message) = match error {
        CurrentHeadMismatch => (
            "AUTHORING_REVISION3_VOICE_MEDIA_HEAD_CONFLICT",
            "published revision-3 head differs from the Voice media QA request",
        ),
        ProjectIdentityMismatch { .. } | ProjectRevisionConflict { .. } => (
            "AUTHORING_REVISION3_VOICE_MEDIA_PROJECT_CONFLICT",
            "exact project identity or revision differs from the Voice media QA request",
        ),
        InvalidEntityIdentity => (
            "AUTHORING_REVISION3_VOICE_MEDIA_INPUT_INVALID",
            "Voice media QA request contains invalid entity identities",
        ),
        InvalidDialogLine { .. } | DialogLineRevisionConflict { .. } => (
            "AUTHORING_REVISION3_VOICE_MEDIA_LINE_CONFLICT",
            "exact DialogLine differs from the Voice media QA request",
        ),
        LocalizationReferenceMismatch { .. }
        | InvalidLocalization { .. }
        | LocalizationRevisionConflict { .. }
        | LocalizationIdentityMismatch => (
            "AUTHORING_REVISION3_VOICE_MEDIA_LOCALIZATION_CONFLICT",
            "exact LocalizationEntry differs from the Voice media QA request",
        ),
        VoiceSlotReferenceMismatch { .. }
        | InvalidVoiceSlot { .. }
        | VoiceSlotRevisionConflict { .. }
        | VoiceSlotLocaleMismatch { .. } => (
            "AUTHORING_REVISION3_VOICE_MEDIA_SLOT_CONFLICT",
            "exact VoiceSlot differs from the Voice media QA request",
        ),
        VoiceTakeNotCandidate { .. }
        | InvalidVoiceTake { .. }
        | VoiceTakeRevisionConflict { .. }
        | VoiceTakeLocaleMismatch { .. } => (
            "AUTHORING_REVISION3_VOICE_MEDIA_TAKE_CONFLICT",
            "exact VoiceTake differs from the Voice media QA request",
        ),
        VoiceTakeAssetMismatch { .. }
        | MissingVoiceAsset { .. }
        | VoiceAssetMetadataMismatch { .. } => (
            "AUTHORING_REVISION3_VOICE_MEDIA_ASSET_CONFLICT",
            "exact VoiceTake asset differs from the Voice media QA request",
        ),
    };
    Failure::new(code, message)
}

fn map_store_open_error(error: WorkingStoreError) -> Failure {
    use WorkingStoreError::*;
    let (code, message) = match error {
        HeadConflict { .. } => (
            "AUTHORING_REVISION3_VOICE_MEDIA_HEAD_CONFLICT",
            "published revision-3 head changed during Voice media QA",
        ),
        MissingHead(_) | MissingRoot(_) | MissingObject(_) => (
            "AUTHORING_REVISION3_VOICE_MEDIA_STORE_UNAVAILABLE",
            "managed Store is unavailable or incomplete",
        ),
        UnsafePath { .. } => (
            "AUTHORING_REVISION3_VOICE_MEDIA_STORE_UNSAFE",
            "managed Store contains an unsafe path or object",
        ),
        LimitExceeded { .. } | InvalidLimits(_) => (
            "AUTHORING_REVISION3_VOICE_MEDIA_STORE_LIMIT",
            "managed Store exceeds a bounded Voice media QA limit",
        ),
        _ => (
            "AUTHORING_REVISION3_VOICE_MEDIA_STORE_INVARIANT",
            "managed Store could not be reopened exactly for Voice media QA",
        ),
    };
    Failure::new(code, message)
}

fn map_selected_asset_error(error: WorkingStoreError) -> Failure {
    match error {
        WorkingStoreError::LimitExceeded { .. } => Failure::new(
            "AUTHORING_REVISION3_VOICE_MEDIA_STORE_LIMIT",
            "selected VoiceTake media exceeds its bounded Store limit",
        ),
        WorkingStoreError::UnsafePath { .. } => Failure::new(
            "AUTHORING_REVISION3_VOICE_MEDIA_STORE_UNSAFE",
            "selected VoiceTake media path is unsafe",
        ),
        WorkingStoreError::MissingRoot(_) | WorkingStoreError::MissingHead(_) => Failure::new(
            "AUTHORING_REVISION3_VOICE_MEDIA_STORE_UNAVAILABLE",
            "managed Store changed while reading the selected VoiceTake",
        ),
        _ => asset_invalid("selected VoiceTake asset is missing, corrupt, or not exact"),
    }
}

fn map_media_error(error: Revision3VoiceTakePreviewOggErrorV1) -> Failure {
    match error {
        Revision3VoiceTakePreviewOggErrorV1::MetadataLimit => Failure::new(
            "AUTHORING_REVISION3_VOICE_MEDIA_RESPONSE_LIMIT",
            "selected VoiceTake media facts exceed the signed response range",
        ),
        Revision3VoiceTakePreviewOggErrorV1::Invalid(_) => {
            asset_invalid("selected VoiceTake is not bounded valid Vorbis or Opus media")
        }
    }
}

fn invalid_request() -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_VOICE_MEDIA_INPUT_INVALID",
        "request must be exact canonical JSON containing only root and voice_take_preview_request_json",
    )
}

fn asset_invalid(message: &'static str) -> Failure {
    Failure::new("AUTHORING_REVISION3_VOICE_MEDIA_ASSET_INVALID", message)
}

fn head_conflict() -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_VOICE_MEDIA_HEAD_CONFLICT",
        "published revision-3 project changed during Voice media QA",
    )
}

fn invariant() -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_VOICE_MEDIA_INVARIANT",
        "revision-3 Voice media QA could not preserve its exact internal contract",
    )
}

fn truncate_utf8(mut value: String, max_bytes: usize) -> String {
    if value.len() <= max_bytes {
        return value;
    }
    let suffix = "...";
    let mut end = max_bytes - suffix.len();
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    value.truncate(end);
    value.push_str(suffix);
    value
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};
    use std::fs;
    use std::path::{Path, PathBuf};

    use gore_authoring::model_revision3::{
        DialogLine, Entity, EntityKind, EntityPayload, LocalizationEntry,
        OggCodec as Revision3OggCodec, OggMetadata as Revision3OggMetadata, OriginRef,
        SchemaRevisionV3, TypedRef, VoiceSlot, VoiceTake, VoiceTakeStatus, VoiceTargetResolution,
    };
    use gore_authoring::{
        AssetMeta, AssetStoreIndex, ContentSeal, EntityId, FormatV2, GameGenerationAnchor,
        ImportedOgg, LocaleCode, ProjectId, ProjectMeta, Sha256Digest,
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
    }

    fn id(tag: u8) -> EntityId {
        EntityId::from_bytes([tag; 16])
    }

    fn project_id() -> ProjectId {
        ProjectId::from_bytes([0x61; 16])
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
            importer: "voice-media-ffi-tests".to_owned(),
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
                name: "Voice media QA FFI fixture".to_owned(),
                version: "1.0.0".to_owned(),
                author: "tests".to_owned(),
            },
            target: target(),
            authoring_locales: BTreeSet::from([locale()]),
            entities: BTreeMap::new(),
            asset_store: AssetStoreIndex::default(),
        }
    }

    fn voice_project(imported: &ImportedOgg) -> ProjectRevision3 {
        let localization_id = id(1);
        let line_id = id(2);
        let slot_id = id(3);
        let take_id = id(4);
        let asset = imported.asset.clone();
        let mut project = empty_project(1);
        project.asset_store.assets.insert(
            asset.sha256,
            AssetMeta {
                byte_len: asset.byte_len,
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

    fn published_store(bytes: &[u8], logical_name: &str) -> PublishedStore {
        let temp = TempDir::new().unwrap();
        let store = WorkingProjectStore::at(temp.path(), ffi_store_limits()).unwrap();
        let previous = store
            .prepare_revision3_checkpoint(None, &empty_project(0))
            .unwrap();
        fs::write(temp.path().join("gore-project.json"), &previous.head_bytes).unwrap();
        let prepared = store
            .prepare_ogg_bytes_classified(bytes.to_vec(), logical_name)
            .unwrap();
        let imported = store
            .install_prepared_ogg(prepared, Some(&previous.head))
            .unwrap();
        let asset_path = asset_path(temp.path(), imported.asset.sha256);
        let asset_bytes = fs::read(&asset_path).unwrap();
        let project = voice_project(&imported);
        let published = store
            .prepare_revision3_checkpoint(Some(&previous.head), &project)
            .unwrap();
        fs::write(temp.path().join("gore-project.json"), &published.head_bytes).unwrap();
        PublishedStore {
            temp,
            project,
            head: published.head,
            previous_head_bytes: previous.head_bytes,
            asset_path,
            asset_bytes,
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

    fn wire(root: &Path, request: &Revision3VoiceTakePreviewRequestV1) -> String {
        serde_json::to_string(&ExactWireRequest {
            command: COMMAND.to_owned(),
            payload: InspectVoiceTakeMediaWirePayload {
                root: root.to_string_lossy().into_owned(),
                voice_take_preview_request_json: request.to_canonical_json().unwrap(),
            },
        })
        .unwrap()
    }

    fn snapshot_regular_files(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
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

    #[test]
    fn exact_outer_and_existing_inner_wires_reject_ambiguity_and_excess() {
        let canonical = serde_json::to_string(&ExactWireRequest {
            command: COMMAND.to_owned(),
            payload: InspectVoiceTakeMediaWirePayload {
                root: "C:/missing-store".to_owned(),
                voice_take_preview_request_json: "{}".to_owned(),
            },
        })
        .unwrap();
        let parsed: InspectVoiceTakeMediaWirePayload = parse_exact_wire(&canonical).unwrap();
        assert_eq!(parsed.root, "C:/missing-store");

        let duplicate = canonical.replacen(
            &format!("\"command\":\"{COMMAND}\""),
            &format!("\"command\":\"{COMMAND}\",\"command\":\"{COMMAND}\""),
            1,
        );
        let unknown = canonical.replacen(
            "\"voice_take_preview_request_json\":\"{}\"",
            "\"voice_take_preview_request_json\":\"{}\",\"authority\":true",
            1,
        );
        let wrong = canonical.replacen(&format!("\"{COMMAND}\""), "\"wrong\"", 1);
        for invalid in [
            duplicate,
            unknown,
            wrong,
            format!(" {canonical}"),
            r#"{"command":"authoring_store_inspect_revision3_voice_take_media_v1","payload":{"root":"C:/missing-store"}}"#.to_owned(),
        ] {
            assert_eq!(
                inspect_revision3_voice_take_media_v1_raw(&invalid)["error"]["code"],
                "AUTHORING_REVISION3_VOICE_MEDIA_INPUT_INVALID"
            );
        }
        assert_eq!(
            inspect_revision3_voice_take_media_v1_raw(&"x".repeat(MAX_WIRE_BYTES + 1))["error"]
                ["code"],
            "AUTHORING_REVISION3_VOICE_MEDIA_INPUT_LIMIT"
        );
    }

    #[test]
    fn vorbis_media_qa_is_exact_pathless_read_only_and_publicly_dispatched() {
        let store = published_store(
            include_bytes!("../../gore-vo/testdata/tiny-vorbis.ogg"),
            "asghan_take.ogg",
        );
        let before = snapshot_regular_files(store.temp.path());
        let input = wire(store.temp.path(), &request(&store));
        let response: Value = serde_json::from_str(&crate::execute_json(&input)).unwrap();

        assert_eq!(response["ok"], true, "{response}");
        assert_eq!(response["outcome"], "media_qa_complete");
        assert_eq!(
            response["basis_head_json"],
            serde_json::to_string(&store.head).unwrap()
        );
        assert_eq!(response["project_id"], project_id().to_string());
        assert_eq!(response["project_revision"], 1);
        assert_eq!(response["line_id"], id(2).to_string());
        assert_eq!(response["localization_id"], id(1).to_string());
        assert_eq!(response["slot_id"], id(3).to_string());
        assert_eq!(response["take_id"], id(4).to_string());
        assert_eq!(response["locale"], "de");
        assert_eq!(response["duration_sample_frames"], 3_840);
        assert_eq!(response["duration_timebase_hz"], 48_000);
        assert_eq!(response["assurance"], "vorbis_full_pcm_decode");
        assert_eq!(
            response["media_authority"],
            "exact_current_managed_cas_voice_take_media_qa_v1"
        );
        assert_eq!(
            response["inspection_scope"],
            "selected_voice_take_media_input_only"
        );
        assert_eq!(response["quality_status"], "not_evaluated");
        assert_eq!(response["audibility_status"], "not_evaluated");
        assert_eq!(response["project_write_status"], "not_performed");
        assert_eq!(response["game_write_status"], "not_performed");
        assert_eq!(response["save_write_status"], "not_performed");
        assert_eq!(response["build_status"], "not_evaluated");
        assert_eq!(response["deployment_status"], "not_performed");
        assert_eq!(response["runtime_status"], "not_qualified");
        assert_eq!(snapshot_regular_files(store.temp.path()), before);

        let encoded = response.to_string();
        assert!(!encoded.contains(store.temp.path().to_string_lossy().as_ref()));
        for forbidden in [
            "root",
            "path",
            "preview_path",
            "cleanup_token",
            "project_json",
            "media_bytes",
            "build_authority",
            "deployment_authority",
            "runtime_authority",
        ] {
            assert!(response.get(forbidden).is_none(), "leaked {forbidden}");
        }
    }

    #[test]
    fn opus_reports_normative_timebase_and_honest_structural_assurance() {
        let store = published_store(
            include_bytes!("../../gore-vo/testdata/tiny-opus.ogg"),
            "asghan_take.opus.ogg",
        );
        let response =
            inspect_revision3_voice_take_media_v1_raw(&wire(store.temp.path(), &request(&store)));
        assert_eq!(response["ok"], true, "{response}");
        assert_eq!(response["ogg"]["codec"], "opus");
        assert_eq!(response["ogg"]["sample_rate"], 48_000);
        assert_eq!(response["duration_sample_frames"], 3_840);
        assert_eq!(response["duration_timebase_hz"], 48_000);
        assert_eq!(
            response["assurance"],
            "opus_packet_and_timing_structure_only"
        );
        assert_eq!(response["audibility_status"], "not_evaluated");
        assert_eq!(response["runtime_status"], "not_qualified");
    }

    #[test]
    fn exact_graph_and_asset_identity_conflicts_are_typed() {
        let store = published_store(
            include_bytes!("../../gore-vo/testdata/tiny-vorbis.ogg"),
            "asghan_take.ogg",
        );

        let mut duplicate_identity = request(&store);
        duplicate_identity.take_id = duplicate_identity.line_id;
        assert_eq!(
            inspect_revision3_voice_take_media_v1_raw(&wire(
                store.temp.path(),
                &duplicate_identity,
            ))["error"]["code"],
            "AUTHORING_REVISION3_VOICE_MEDIA_INPUT_INVALID"
        );

        let mut stale_line = request(&store);
        stale_line.expected_line_revision += 1;
        assert_eq!(
            inspect_revision3_voice_take_media_v1_raw(&wire(store.temp.path(), &stale_line))
                ["error"]["code"],
            "AUTHORING_REVISION3_VOICE_MEDIA_LINE_CONFLICT"
        );

        let mut redirected_asset = request(&store);
        redirected_asset.expected_asset.logical_name = "redirected.ogg".to_owned();
        assert_eq!(
            inspect_revision3_voice_take_media_v1_raw(&wire(store.temp.path(), &redirected_asset))
                ["error"]["code"],
            "AUTHORING_REVISION3_VOICE_MEDIA_ASSET_CONFLICT"
        );

        let mut stale_head = request(&store);
        stale_head.expected_head.snapshot.sha256 = Sha256Digest::from_bytes([0xee; 32]);
        assert_eq!(
            inspect_revision3_voice_take_media_v1_raw(&wire(store.temp.path(), &stale_head))
                ["error"]["code"],
            "AUTHORING_REVISION3_VOICE_MEDIA_HEAD_CONFLICT"
        );
    }

    #[test]
    fn head_and_selected_cas_races_fail_closed_after_first_media_read() {
        let store = published_store(
            include_bytes!("../../gore-vo/testdata/tiny-vorbis.ogg"),
            "asghan_take.ogg",
        );
        let response = inspect_revision3_voice_take_media_v1_inner_with_seam_and_limit(
            &wire(store.temp.path(), &request(&store)),
            || {
                fs::write(
                    store.temp.path().join("gore-project.json"),
                    &store.previous_head_bytes,
                )
                .unwrap();
            },
            MAX_RESPONSE_BYTES,
        )
        .unwrap_err()
        .response();
        assert_eq!(
            response["error"]["code"],
            "AUTHORING_REVISION3_VOICE_MEDIA_HEAD_CONFLICT"
        );
        assert!(!response
            .to_string()
            .contains(store.temp.path().to_string_lossy().as_ref()));

        let store = published_store(
            include_bytes!("../../gore-vo/testdata/tiny-vorbis.ogg"),
            "asghan_take.ogg",
        );
        let response = inspect_revision3_voice_take_media_v1_inner_with_seam_and_limit(
            &wire(store.temp.path(), &request(&store)),
            || {
                let mut changed = store.asset_bytes.clone();
                changed[0] ^= 1;
                fs::write(&store.asset_path, changed).unwrap();
            },
            MAX_RESPONSE_BYTES,
        )
        .unwrap_err()
        .response();
        assert_eq!(
            response["error"]["code"],
            "AUTHORING_REVISION3_VOICE_MEDIA_ASSET_INVALID"
        );
        assert!(!response
            .to_string()
            .contains(store.temp.path().to_string_lossy().as_ref()));
    }

    #[test]
    fn roots_signed_numbers_responses_and_errors_are_bounded() {
        let store = published_store(
            include_bytes!("../../gore-vo/testdata/tiny-vorbis.ogg"),
            "asghan_take.ogg",
        );
        let relative = serde_json::to_string(&ExactWireRequest {
            command: COMMAND.to_owned(),
            payload: InspectVoiceTakeMediaWirePayload {
                root: "relative-store".to_owned(),
                voice_take_preview_request_json: request(&store).to_canonical_json().unwrap(),
            },
        })
        .unwrap();
        assert_eq!(
            inspect_revision3_voice_take_media_v1_raw(&relative)["error"]["code"],
            "AUTHORING_REVISION3_VOICE_MEDIA_INPUT_INVALID"
        );
        assert_eq!(
            signed_wire_u64(i64::MAX as u64 + 1).unwrap_err().code,
            "AUTHORING_REVISION3_VOICE_MEDIA_SIGNED_WIRE_LIMIT"
        );
        assert_eq!(
            inspect_revision3_voice_take_media_v1_inner_with_seam_and_limit(
                &wire(store.temp.path(), &request(&store)),
                || {},
                64,
            )
            .unwrap_err()
            .code,
            "AUTHORING_REVISION3_VOICE_MEDIA_RESPONSE_LIMIT"
        );
        let truncated = Failure::new("TEST", "é".repeat(MAX_ERROR_MESSAGE_BYTES));
        assert!(truncated.message.len() <= MAX_ERROR_MESSAGE_BYTES);
        assert!(truncated.message.ends_with("..."));
    }
}
