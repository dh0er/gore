//! Native prepare-only orchestration for one revision-3 dialog VoiceTake transaction.
//!
//! The route first proves the managed Store and configured game installation are disjoint. It then
//! imports and validates one bounded Ogg into immutable Store CAS under the exact fixed head,
//! applies the filesystem-free line/locale/slot/take transaction, rechecks the source into the same
//! CAS identity, fully reopens the candidate, and returns it without publishing the head.

use std::{
    fs,
    path::{Path, PathBuf},
};

use gore_authoring::{
    apply_revision3_voice_take_transaction_v1, preflight_revision3_voice_take_transaction_v1,
    AssetVerification, OggImportError, OggImportFailureContext, Revision3VoiceBuildStatusV1,
    Revision3VoicePublicationStatusV1, Revision3VoiceRuntimeStatusV1,
    Revision3VoiceTakePreflightEvaluationV1, Revision3VoiceTakeStageConflictV1,
    Revision3VoiceTakeStageErrorV1, Revision3VoiceTakeStageEvaluationV1,
    Revision3VoiceTakeStageRequestV1, Revision3VoiceTargetAuthorityV1, WorkingHead,
    WorkingProjectStore, WorkingStoreError, WorkingStoreLimits, MAX_PROJECT_JSON_BYTES,
    MAX_REVISION3_VOICE_REQUEST_JSON_BYTES_V1,
};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::err;

pub(super) const COMMAND: &str = "authoring_store_prepare_revision3_voice_take_v1";

const MAX_PATH_BYTES: usize = 32 * 1024;
const MAX_HEAD_JSON_BYTES: usize = 64 * 1024;
const MAX_RESPONSE_BYTES: usize = 64 * 1024 * 1024;
const MAX_ERROR_MESSAGE_BYTES: usize = 4 * 1024;
const MAX_BASIS_REVISION: u64 = i64::MAX as u64 - 1;
const MAX_WIRE_BYTES: usize = MAX_PROJECT_JSON_BYTES * 2
    + MAX_REVISION3_VOICE_REQUEST_JSON_BYTES_V1 * 2
    + MAX_PATH_BYTES * 8
    + 8 * 1024;

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExactWireRequest<P> {
    command: String,
    payload: P,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PrepareVoiceWirePayload {
    current_project_json: String,
    game_root: String,
    root: String,
    source: String,
    voice_request_json: String,
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

pub(super) fn prepare_revision3_voice_take_v1_raw(input: &str) -> Value {
    prepare_revision3_voice_take_v1_inner(input).unwrap_or_else(Failure::response)
}

fn prepare_revision3_voice_take_v1_inner(input: &str) -> Result<Value, Failure> {
    prepare_revision3_voice_take_v1_inner_with_test_seams(input, |preview| preview, || {})
}

#[cfg(test)]
fn prepare_revision3_voice_take_v1_inner_with_source_guard<F>(
    input: &str,
    after_first_source_prepare: F,
) -> Result<Value, Failure>
where
    F: FnOnce(),
{
    prepare_revision3_voice_take_v1_inner_with_test_seams(
        input,
        |preview| preview,
        after_first_source_prepare,
    )
}

fn prepare_revision3_voice_take_v1_inner_with_test_seams<T, F>(
    input: &str,
    transform_preview: T,
    after_first_source_prepare: F,
) -> Result<Value, Failure>
where
    T: FnOnce(gore_authoring::ImportedOgg) -> gore_authoring::ImportedOgg,
    F: FnOnce(),
{
    let payload: PrepareVoiceWirePayload = parse_exact_wire(input)?;
    validate_payload(&payload)?;

    ensure_store_is_outside_game(Path::new(&payload.root), Path::new(&payload.game_root))?;
    let store = WorkingProjectStore::open_existing(Path::new(&payload.root), ffi_store_limits())
        .map_err(map_store_error)?;
    let basis = store
        .open_current_revision3(AssetVerification::Full)
        .map_err(map_store_error)?;
    validate_basis_revision(basis.project.revision)?;
    require_signed_serializable(&basis.project)?;
    let canonical_basis = basis.project.to_canonical_json().map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_VOICE_STORE_INVARIANT",
            "the exact current revision-3 project could not be serialized",
        )
    })?;
    if canonical_basis.as_bytes() != payload.current_project_json.as_bytes() {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_PROJECT_CONFLICT",
            "current_project_json differs from the exact published revision-3 project",
        ));
    }

    let request = Revision3VoiceTakeStageRequestV1::from_json(&payload.voice_request_json)
        .map_err(map_request_error)?;
    require_signed_serializable(&request)?;
    if request.expected_head != basis.head {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_HEAD_CONFLICT",
            "the Voice request head differs from the exact published revision-3 head",
        ));
    }
    if request.expected_project_id != basis.project.project_id
        || request.expected_revision != basis.project.revision
        || request.expected_target != basis.project.target
    {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_PROJECT_CONFLICT",
            "the Voice request identity, revision, or target differs from the exact published revision-3 project",
        ));
    }

    match preflight_revision3_voice_take_transaction_v1(
        &basis.head,
        &payload.current_project_json,
        &payload.voice_request_json,
    )
    .map_err(map_transaction_error)?
    {
        Revision3VoiceTakePreflightEvaluationV1::Ready => {}
        Revision3VoiceTakePreflightEvaluationV1::Rejected(rejection) => {
            return Err(map_transaction_conflict(rejection.conflict));
        }
    }

    let prepared_ogg = store
        .prepare_ogg_import_classified(Path::new(&payload.source), request.logical_name.clone())
        .map_err(map_ogg_import_error)?;
    let preview = transform_preview(prepared_ogg.preview());

    let mut outcome = match apply_revision3_voice_take_transaction_v1(
        &basis.head,
        &payload.current_project_json,
        &payload.voice_request_json,
        preview,
    )
    .map_err(map_transaction_error)?
    {
        Revision3VoiceTakeStageEvaluationV1::Applied(outcome) => *outcome,
        Revision3VoiceTakeStageEvaluationV1::Rejected(rejection) => {
            return Err(map_transaction_conflict(rejection.conflict));
        }
    };

    // A second complete source preparation is read-only with respect to Store staging/CAS. Only
    // an exact byte/receipt match may advance to immutable installation of the first accepted
    // bytes. The test seam makes same-path source races deterministic without global state.
    after_first_source_prepare();
    let reprepared_ogg = store
        .prepare_ogg_import_classified(Path::new(&payload.source), request.logical_name)
        .map_err(map_ogg_import_error)?;
    if !prepared_ogg.has_same_content(&reprepared_ogg) {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_INPUT_CHANGED",
            "the Ogg source changed during revision-3 Voice preparation",
        ));
    }
    drop(reprepared_ogg);

    let imported = store
        .install_prepared_ogg(prepared_ogg, Some(&basis.head))
        .map_err(map_store_error)?;
    if imported.asset != outcome.imported_ogg.asset || imported.ogg != outcome.imported_ogg.ogg {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_STORE_INVARIANT",
            "the installed Ogg receipt differs from its accepted source preparation",
        ));
    }
    store
        .verify_asset(&imported.asset, AssetVerification::Full)
        .map_err(map_store_error)?;
    outcome.imported_ogg = imported;

    let current_before_prepare = store
        .open_current_revision3(AssetVerification::Full)
        .map_err(map_store_error)?;
    if current_before_prepare.head != basis.head || current_before_prepare.project != basis.project
    {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_HEAD_CONFLICT",
            "the published revision-3 project changed during Voice preparation",
        ));
    }

    let prepared = store
        .prepare_revision3_checkpoint(Some(&basis.head), &outcome.project)
        .map_err(map_store_error)?;
    let reopened = store
        .open_revision3_head_bytes(&prepared.head_bytes, AssetVerification::Full)
        .map_err(map_store_error)?;
    if reopened.head != prepared.head || reopened.project != outcome.project {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_STORE_INVARIANT",
            "the prepared revision-3 Voice checkpoint did not reopen exactly",
        ));
    }
    store
        .verify_asset(&outcome.imported_ogg.asset, AssetVerification::Full)
        .map_err(map_store_error)?;

    let current_after = store
        .open_current_revision3(AssetVerification::Full)
        .map_err(map_store_error)?;
    if current_after.head != basis.head || current_after.project != basis.project {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_HEAD_CONFLICT",
            "the published revision-3 project changed before Voice preparation completed",
        ));
    }

    let basis_head_json = canonical_head_json(&basis.head)?;
    let candidate_head_json = String::from_utf8(prepared.head_bytes).map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_VOICE_STORE_INVARIANT",
            "the prepared revision-3 Voice head is not UTF-8 JSON",
        )
    })?;
    if candidate_head_json.is_empty() || candidate_head_json.len() > MAX_HEAD_JSON_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_RESPONSE_LIMIT",
            "the prepared revision-3 Voice head exceeds its transport limit",
        ));
    }
    require_signed_serializable(&prepared.head)?;

    let codec = match outcome.imported_ogg.ogg.codec {
        gore_authoring::OggCodec::Vorbis => "vorbis",
        gore_authoring::OggCodec::Opus => "opus",
    };
    let take_status = match outcome.status {
        gore_authoring::VoiceTakeStatus::Draft => "draft",
        gore_authoring::VoiceTakeStatus::Recorded => "recorded",
        gore_authoring::VoiceTakeStatus::Reviewed => "reviewed",
        gore_authoring::VoiceTakeStatus::Approved => "approved",
    };
    let build_status = match outcome.build_status {
        Revision3VoiceBuildStatusV1::Blocked => "blocked",
    };
    let runtime_status = match outcome.runtime_status {
        Revision3VoiceRuntimeStatusV1::RuntimeUnqualified => "runtime_unqualified",
    };
    let target_authority = match outcome.target_authority {
        Revision3VoiceTargetAuthorityV1::NotGranted => "not_granted",
    };
    let publication_status = match outcome.publication_status {
        Revision3VoicePublicationStatusV1::NotSupported => "not_supported",
    };
    let response = json!({
        "ok": true,
        "outcome": "prepared_unpublished",
        "basis_head_json": basis_head_json,
        "head_json": candidate_head_json,
        "project_json": outcome.canonical_project_json,
        "revision": outcome.project.revision,
        "line_id": outcome.line_id.to_string(),
        "localization_id": outcome.localization_id.to_string(),
        "slot_id": outcome.slot_id.to_string(),
        "take_id": outcome.take_id.to_string(),
        "locale": outcome.locale.to_string(),
        "take_status": take_status,
        "slot_created": outcome.slot_created,
        "selected": outcome.selected,
        "asset": {
            "sha256": outcome.imported_ogg.asset.sha256.to_string(),
            "byte_len": outcome.imported_ogg.asset.byte_len,
            "logical_name": outcome.imported_ogg.asset.logical_name,
        },
        "ogg": {
            "codec": codec,
            "channels": outcome.imported_ogg.ogg.channels,
            "sample_rate": outcome.imported_ogg.ogg.sample_rate,
            "pages": outcome.imported_ogg.ogg.pages,
            "logical_streams": outcome.imported_ogg.ogg.logical_streams,
        },
        "asset_deduplicated": outcome.imported_ogg.deduplicated,
        "build_status": build_status,
        "runtime_status": runtime_status,
        "target_authority": target_authority,
        "publication_status": publication_status,
    });
    enforce_response_budget(&response)?;

    let final_current = store
        .open_current_revision3(AssetVerification::Full)
        .map_err(map_store_error)?;
    if final_current.head != basis.head || final_current.project != basis.project {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_HEAD_CONFLICT",
            "the published revision-3 project changed before the Voice response was returned",
        ));
    }
    Ok(response)
}

fn parse_exact_wire<P>(input: &str) -> Result<P, Failure>
where
    P: DeserializeOwned + Serialize,
{
    if input.len() > MAX_WIRE_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_INPUT_LIMIT",
            format!("revision-3 Voice request exceeds the {MAX_WIRE_BYTES}-byte wire limit"),
        ));
    }
    let request: ExactWireRequest<P> =
        serde_json::from_str(input).map_err(|_| invalid_request())?;
    if request.command != COMMAND {
        return Err(invalid_request());
    }
    let canonical = serde_json::to_string(&request).map_err(|_| invalid_request())?;
    if canonical.as_bytes() != input.as_bytes() {
        return Err(invalid_request());
    }
    Ok(request.payload)
}

fn validate_payload(payload: &PrepareVoiceWirePayload) -> Result<(), Failure> {
    for path in [&payload.game_root, &payload.root, &payload.source] {
        if path.is_empty() || path.len() > MAX_PATH_BYTES || path.contains('\0') {
            return Err(invalid_request());
        }
    }
    if payload.current_project_json.is_empty() {
        return Err(invalid_request());
    }
    if payload.current_project_json.len() > MAX_PROJECT_JSON_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_PROJECT_LIMIT",
            format!("current_project_json exceeds the {MAX_PROJECT_JSON_BYTES}-byte limit"),
        ));
    }
    if payload.voice_request_json.is_empty() {
        return Err(invalid_request());
    }
    if payload.voice_request_json.len() > MAX_REVISION3_VOICE_REQUEST_JSON_BYTES_V1 {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_REQUEST_LIMIT",
            format!(
                "voice_request_json exceeds the {MAX_REVISION3_VOICE_REQUEST_JSON_BYTES_V1}-byte limit"
            ),
        ));
    }
    Ok(())
}

fn ensure_store_is_outside_game(store_root: &Path, game_root: &Path) -> Result<(), Failure> {
    let store_root = fs::canonicalize(store_root).map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_VOICE_STORE_PATH_UNSAFE",
            "the revision-3 working-store root could not be resolved safely",
        )
    })?;
    let install_root = fs::canonicalize(semantic_install_root(game_root)).map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_VOICE_GAME_ROOT_UNAVAILABLE",
            "the configured game installation root could not be resolved safely",
        )
    })?;
    if store_root.starts_with(&install_root) || install_root.starts_with(&store_root) {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_STORE_GAME_ALIAS",
            "the working-store root and configured game installation must be disjoint",
        ));
    }
    Ok(())
}

fn semantic_install_root(game_root: &Path) -> PathBuf {
    if game_root.file_name().is_some_and(is_g1r_component) {
        game_root
            .parent()
            .filter(|value| !value.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf()
    } else {
        game_root.to_path_buf()
    }
}

fn is_g1r_component(value: &std::ffi::OsStr) -> bool {
    value.as_encoded_bytes().eq_ignore_ascii_case(b"G1R")
}

fn validate_basis_revision(revision: u64) -> Result<(), Failure> {
    if revision > MAX_BASIS_REVISION {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_REVISION_LIMIT",
            format!("the published basis revision exceeds {MAX_BASIS_REVISION}"),
        ));
    }
    Ok(())
}

fn require_signed_serializable(value: &impl Serialize) -> Result<(), Failure> {
    let value = serde_json::to_value(value).map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_VOICE_INVARIANT",
            "a revision-3 Voice wire value could not be inspected",
        )
    })?;
    require_signed_json_value(&value)
}

fn require_signed_json_value(value: &Value) -> Result<(), Failure> {
    match value {
        Value::Number(number) if number.as_u64().is_some_and(|value| value > i64::MAX as u64) => {
            Err(Failure::new(
                "AUTHORING_REVISION3_VOICE_SIGNED_WIRE_LIMIT",
                "a revision-3 Voice wire integer exceeds the signed 64-bit transport range",
            ))
        }
        Value::Array(values) => values.iter().try_for_each(require_signed_json_value),
        Value::Object(values) => values.values().try_for_each(require_signed_json_value),
        _ => Ok(()),
    }
}

fn canonical_head_json(head: &WorkingHead) -> Result<String, Failure> {
    require_signed_serializable(head)?;
    let value = serde_json::to_string(head).map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_VOICE_INVARIANT",
            "the revision-3 Voice basis head could not be serialized",
        )
    })?;
    if value.is_empty() || value.len() > MAX_HEAD_JSON_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_RESPONSE_LIMIT",
            "the revision-3 Voice basis head exceeds its transport limit",
        ));
    }
    Ok(value)
}

fn enforce_response_budget(response: &Value) -> Result<(), Failure> {
    let encoded = serde_json::to_vec(response).map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_VOICE_INVARIANT",
            "the revision-3 Voice response could not be serialized",
        )
    })?;
    if encoded.len() > MAX_RESPONSE_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_RESPONSE_LIMIT",
            "the revision-3 Voice response exceeds its bounded transport budget",
        ));
    }
    Ok(())
}

fn ffi_store_limits() -> WorkingStoreLimits {
    WorkingStoreLimits {
        max_referenced_entity_bytes: MAX_PROJECT_JSON_BYTES as u64,
        ..WorkingStoreLimits::default()
    }
}

fn invalid_request() -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_VOICE_REQUEST_INVALID",
        "request must be exact canonical JSON containing command and exactly current_project_json, game_root, root, source, and voice_request_json",
    )
}

fn map_request_error(error: impl std::fmt::Display) -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_VOICE_REQUEST_INVALID",
        format!("the exact revision-3 Voice request is invalid: {error}"),
    )
}

fn map_transaction_error(error: Revision3VoiceTakeStageErrorV1) -> Failure {
    match error {
        Revision3VoiceTakeStageErrorV1::InvalidProject(_) => Failure::new(
            "AUTHORING_REVISION3_VOICE_PROJECT_INVALID",
            "the exact current revision-3 project is invalid",
        ),
        Revision3VoiceTakeStageErrorV1::InvalidRequest(error) => map_request_error(error),
        Revision3VoiceTakeStageErrorV1::ReopenCandidate(_)
        | Revision3VoiceTakeStageErrorV1::CanonicalReopenMismatch => Failure::new(
            "AUTHORING_REVISION3_VOICE_INVARIANT",
            "the revision-3 Voice candidate failed exact canonical reopen",
        ),
    }
}

fn map_transaction_conflict(error: Revision3VoiceTakeStageConflictV1) -> Failure {
    let code = match &error {
        Revision3VoiceTakeStageConflictV1::CurrentHeadMismatch => {
            "AUTHORING_REVISION3_VOICE_HEAD_CONFLICT"
        }
        Revision3VoiceTakeStageConflictV1::ProjectIdentityMismatch { .. }
        | Revision3VoiceTakeStageConflictV1::ProjectRevisionConflict { .. }
        | Revision3VoiceTakeStageConflictV1::ProjectTargetMismatch
        | Revision3VoiceTakeStageConflictV1::VoiceSlotIdentityMismatch { .. } => {
            "AUTHORING_REVISION3_VOICE_PROJECT_CONFLICT"
        }
        Revision3VoiceTakeStageConflictV1::ProjectRevisionOverflow
        | Revision3VoiceTakeStageConflictV1::LocalizationRevisionOverflow { .. }
        | Revision3VoiceTakeStageConflictV1::DialogLineRevisionOverflow { .. }
        | Revision3VoiceTakeStageConflictV1::VoiceSlotRevisionOverflow { .. } => {
            "AUTHORING_REVISION3_VOICE_REVISION_LIMIT"
        }
        Revision3VoiceTakeStageConflictV1::VoiceTakeIdCollision { .. }
        | Revision3VoiceTakeStageConflictV1::VoiceSlotIdCollision { .. }
        | Revision3VoiceTakeStageConflictV1::SharedEntityId => {
            "AUTHORING_REVISION3_VOICE_COLLISION"
        }
        Revision3VoiceTakeStageConflictV1::ZeroEntityId { .. }
        | Revision3VoiceTakeStageConflictV1::InvalidLocalizedText
        | Revision3VoiceTakeStageConflictV1::InvalidTakeDisplayName
        | Revision3VoiceTakeStageConflictV1::InvalidLogicalName => {
            "AUTHORING_REVISION3_VOICE_INTENT_INVALID"
        }
        Revision3VoiceTakeStageConflictV1::UnapprovedTakeSelection => {
            "AUTHORING_REVISION3_VOICE_STATUS_INVALID"
        }
        Revision3VoiceTakeStageConflictV1::EntityCapacityExceeded
        | Revision3VoiceTakeStageConflictV1::AssetCapacityExceeded => {
            "AUTHORING_REVISION3_VOICE_LIMIT"
        }
        Revision3VoiceTakeStageConflictV1::InvalidImportedOgg => {
            "AUTHORING_REVISION3_VOICE_OGG_INVALID"
        }
        Revision3VoiceTakeStageConflictV1::InvalidDialogLine { .. }
        | Revision3VoiceTakeStageConflictV1::InvalidLocalizationReference { .. }
        | Revision3VoiceTakeStageConflictV1::InvalidVoiceSlot { .. }
        | Revision3VoiceTakeStageConflictV1::AssetMetadataConflict
        | Revision3VoiceTakeStageConflictV1::CandidateNotPersistable { .. } => {
            "AUTHORING_REVISION3_VOICE_PROJECT_INVALID"
        }
    };
    Failure::new(code, error.to_string())
}

fn map_ogg_import_error(error: OggImportError) -> Failure {
    match error.context() {
        OggImportFailureContext::Store => map_store_error(error.into_store_error()),
        OggImportFailureContext::SourceMissing => Failure::new(
            "AUTHORING_REVISION3_VOICE_INPUT_MISSING",
            "the selected Ogg source no longer exists",
        ),
        OggImportFailureContext::SourceUnavailable => Failure::new(
            "AUTHORING_REVISION3_VOICE_INPUT_UNAVAILABLE",
            "the selected Ogg source could not be opened or read",
        ),
        OggImportFailureContext::SourceUnsafe => Failure::new(
            "AUTHORING_REVISION3_VOICE_INPUT_UNSAFE",
            "the selected Ogg source is not one safe regular non-link file",
        ),
        OggImportFailureContext::SourceLimit => Failure::new(
            "AUTHORING_REVISION3_VOICE_INPUT_LIMIT",
            "the selected Ogg source exceeds the bounded import limit",
        ),
        OggImportFailureContext::SourceInvalid => Failure::new(
            "AUTHORING_REVISION3_VOICE_OGG_INVALID",
            "the selected source is not one supported valid Ogg stream",
        ),
        OggImportFailureContext::SourceChanged => Failure::new(
            "AUTHORING_REVISION3_VOICE_INPUT_CHANGED",
            "the selected Ogg source changed while it was being read",
        ),
    }
}

fn map_store_error(error: WorkingStoreError) -> Failure {
    let code = match error {
        WorkingStoreError::InvalidLimits(_) => "AUTHORING_REVISION3_VOICE_STORE_LIMITS_INVALID",
        WorkingStoreError::MissingRoot(_) => "AUTHORING_REVISION3_VOICE_STORE_ROOT_MISSING",
        WorkingStoreError::UnsafePath { .. } => "AUTHORING_REVISION3_VOICE_STORE_PATH_UNSAFE",
        WorkingStoreError::LimitExceeded { .. } => "AUTHORING_REVISION3_VOICE_STORE_LIMIT",
        WorkingStoreError::HeadConflict { .. } => "AUTHORING_REVISION3_VOICE_HEAD_CONFLICT",
        WorkingStoreError::MissingHead(_) => "AUTHORING_REVISION3_VOICE_HEAD_MISSING",
        WorkingStoreError::MissingObject(_) => "AUTHORING_REVISION3_VOICE_STORE_OBJECT_MISSING",
        WorkingStoreError::SealMismatch { .. } => "AUTHORING_REVISION3_VOICE_STORE_SEAL_MISMATCH",
        WorkingStoreError::Collision { .. } => "AUTHORING_REVISION3_VOICE_STORE_COLLISION",
        WorkingStoreError::InvalidJson { .. } | WorkingStoreError::NonCanonicalJson { .. } => {
            "AUTHORING_REVISION3_VOICE_STORE_JSON_INVALID"
        }
        WorkingStoreError::Invariant(_)
        | WorkingStoreError::InvalidOgg(_)
        | WorkingStoreError::OggMetadataMismatch { .. } => {
            "AUTHORING_REVISION3_VOICE_STORE_INVARIANT"
        }
        WorkingStoreError::StagingCleanup { .. } | WorkingStoreError::Io(_) => {
            "AUTHORING_REVISION3_VOICE_STORE_IO"
        }
    };
    Failure::new(code, "the revision-3 Voice working-store operation failed")
}

fn truncate_utf8(mut value: String, max_bytes: usize) -> String {
    if value.len() <= max_bytes {
        return value;
    }
    const SUFFIX: &str = "...";
    let mut end = max_bytes - SUFFIX.len();
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    value.truncate(end);
    value.push_str(SUFFIX);
    value
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};
    use std::fs;
    use std::path::{Path, PathBuf};

    use gore_authoring::model_revision3::{
        DialogLine, Entity, EntityKind, EntityPayload, LocalizationEntry, OriginRef,
        SchemaRevisionV3, TypedRef, VoiceTakeStatus,
    };
    use gore_authoring::{
        AssetStoreIndex, ContentSeal, EntityId, FormatV2, GameGenerationAnchor, LocaleCode,
        ProjectId, ProjectMeta, ProjectRevision3, Revision3VoiceTakeStageRequestV1, Sha256Digest,
        MAX_REVISION3_REFERENCED_ASSET_BYTES,
    };
    use tempfile::TempDir;

    use super::*;

    fn raw_request(payload: Value) -> String {
        json!({"command": COMMAND, "payload": payload}).to_string()
    }

    fn id(tag: u8) -> EntityId {
        EntityId::from_bytes([tag; 16])
    }

    fn project_id() -> ProjectId {
        ProjectId::from_bytes([0x10; 16])
    }

    fn locale() -> LocaleCode {
        "de".parse().unwrap()
    }

    fn target() -> GameGenerationAnchor {
        GameGenerationAnchor {
            executable: ContentSeal {
                byte_len: 171_698_176,
                sha256: Sha256Digest::from_bytes([0x21; 32]),
            },
        }
    }

    fn imported_origin(tag: u8) -> OriginRef {
        OriginRef::Imported {
            importer: "tests".to_owned(),
            source_seal: ContentSeal {
                byte_len: 10,
                sha256: Sha256Digest::from_bytes([tag; 32]),
            },
            external_identity: None,
        }
    }

    fn basis_project(revision: u64) -> ProjectRevision3 {
        let localization_id = id(2);
        let line_id = id(3);
        ProjectRevision3 {
            format: FormatV2,
            schema_revision: SchemaRevisionV3,
            project_id: project_id(),
            revision,
            meta: ProjectMeta {
                name: "Voice FFI".to_owned(),
                version: "1.0.0".to_owned(),
                author: "tests".to_owned(),
            },
            target: target(),
            authoring_locales: BTreeSet::new(),
            entities: BTreeMap::from([
                (
                    localization_id,
                    Entity {
                        id: localization_id,
                        display_name: "Asghan line text".to_owned(),
                        origin: imported_origin(2),
                        revision: 4,
                        payload: EntityPayload::LocalizationEntry(LocalizationEntry {
                            loc_id: "GRD_263_ASGHAN_OPEN_INFO_06_02".to_owned(),
                            texts: BTreeMap::new(),
                        }),
                    },
                ),
                (
                    line_id,
                    Entity {
                        id: line_id,
                        display_name: "Asghan greeting".to_owned(),
                        origin: imported_origin(3),
                        revision: 2,
                        payload: EntityPayload::DialogLine(DialogLine {
                            localization: TypedRef::new(
                                project_id(),
                                localization_id,
                                EntityKind::LocalizationEntry,
                            ),
                            speaker_hint: Some("Asghan".to_owned()),
                            voice_slots: BTreeMap::new(),
                        }),
                    },
                ),
            ]),
            asset_store: AssetStoreIndex::default(),
        }
    }

    fn published_store(revision: u64) -> (TempDir, ProjectRevision3, String, WorkingHead, Vec<u8>) {
        let temp = TempDir::new().unwrap();
        let store = WorkingProjectStore::at(temp.path(), ffi_store_limits()).unwrap();
        let project = basis_project(revision);
        let project_json = project.to_canonical_json().unwrap();
        let prepared = store.prepare_revision3_checkpoint(None, &project).unwrap();
        fs::write(temp.path().join("gore-project.json"), &prepared.head_bytes).unwrap();
        (
            temp,
            project,
            project_json,
            prepared.head,
            prepared.head_bytes,
        )
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

    fn voice_request(
        project: &ProjectRevision3,
        head: &WorkingHead,
        status: VoiceTakeStatus,
        select_take: bool,
    ) -> String {
        Revision3VoiceTakeStageRequestV1 {
            expected_head: head.clone(),
            expected_project_id: project.project_id,
            expected_revision: project.revision,
            expected_target: project.target.clone(),
            line_id: id(3),
            slot_id: id(4),
            take_id: id(5),
            locale: locale(),
            text: None,
            take_display_name: "Asghan DE Take 1".to_owned(),
            logical_name: "GRD_263_ASGHAN_OPEN_INFO_06_02.ogg".to_owned(),
            status,
            select_take,
        }
        .to_canonical_json()
        .unwrap()
    }

    fn write_tiny_ogg(root: &TempDir) -> std::path::PathBuf {
        let source = root.path().join("asghan.ogg");
        fs::write(
            &source,
            include_bytes!("../../gore-vo/testdata/tiny-vorbis.ogg"),
        )
        .unwrap();
        source
    }

    #[test]
    fn exact_raw_wire_and_public_route_reject_duplicate_or_forged_fields() {
        let valid = raw_request(json!({
            "current_project_json": "{}",
            "game_root": "C:/missing-game",
            "root": "C:/missing-store",
            "source": "C:/missing.ogg",
            "voice_request_json": "{}",
        }));
        let parsed: PrepareVoiceWirePayload = parse_exact_wire(&valid).unwrap();
        assert_eq!(parsed.current_project_json, "{}");

        let duplicate = format!(
            "{{\"command\":\"{COMMAND}\",\"payload\":{{\"current_project_json\":\"{{}}\",\"game_root\":\"g\",\"root\":\"r\",\"source\":\"a.ogg\",\"source\":\"forged.ogg\",\"voice_request_json\":\"{{}}\"}}}}"
        );
        let response: Value = serde_json::from_str(&crate::execute_json(&duplicate)).unwrap();
        assert_eq!(
            response["error"]["code"],
            "AUTHORING_REVISION3_VOICE_REQUEST_INVALID"
        );

        let forged = raw_request(json!({
            "current_project_json": "{}",
            "game_root": "g",
            "root": "r",
            "source": "a.ogg",
            "voice_request_json": "{}",
            "target_authority": "granted",
        }));
        assert_eq!(
            prepare_revision3_voice_take_v1_raw(&forged)["error"]["code"],
            "AUTHORING_REVISION3_VOICE_REQUEST_INVALID"
        );
    }

    #[test]
    fn working_store_and_game_roots_are_bidirectionally_disjoint() {
        let temp = TempDir::new().unwrap();
        let store_root = temp.path().join("store");
        let game_root = temp.path().join("game");
        fs::create_dir(&store_root).unwrap();
        fs::create_dir(&game_root).unwrap();
        ensure_store_is_outside_game(&store_root, &game_root).unwrap();

        let nested_game = store_root.join("game");
        fs::create_dir(&nested_game).unwrap();
        assert_eq!(
            ensure_store_is_outside_game(&store_root, &nested_game)
                .unwrap_err()
                .code,
            "AUTHORING_REVISION3_VOICE_STORE_GAME_ALIAS"
        );

        let nested_store = game_root.join("project");
        fs::create_dir(&nested_store).unwrap();
        assert_eq!(
            ensure_store_is_outside_game(&nested_store, &game_root)
                .unwrap_err()
                .code,
            "AUTHORING_REVISION3_VOICE_STORE_GAME_ALIAS"
        );

        // A configured `.../G1R` path denotes the whole installation parent.
        let install_root = temp.path().join("install");
        let direct_g1r = install_root.join("g1r");
        let sibling_store = install_root.join("projects");
        fs::create_dir_all(&direct_g1r).unwrap();
        fs::create_dir(&sibling_store).unwrap();
        assert_eq!(semantic_install_root(&direct_g1r), install_root);
        assert_eq!(
            ensure_store_is_outside_game(&sibling_store, &direct_g1r)
                .unwrap_err()
                .code,
            "AUTHORING_REVISION3_VOICE_STORE_GAME_ALIAS"
        );
    }

    #[test]
    fn valid_ogg_prepares_fully_reopenable_candidate_without_publishing_head() {
        let (temp, project, project_json, basis_head, fixed_head) = published_store(7);
        let source_root = TempDir::new().unwrap();
        let source = write_tiny_ogg(&source_root);
        let response = prepare_revision3_voice_take_v1_raw(&raw_request(json!({
            "current_project_json": project_json,
            "game_root": source_root.path(),
            "root": temp.path(),
            "source": source,
            "voice_request_json": voice_request(
                &project,
                &basis_head,
                VoiceTakeStatus::Recorded,
                false,
            ),
        })));

        assert_eq!(response["ok"], true, "{response}");
        assert_eq!(response["outcome"], "prepared_unpublished");
        assert_eq!(response["revision"], 8);
        assert_eq!(response["line_id"], id(3).to_string());
        assert_eq!(response["localization_id"], id(2).to_string());
        assert_eq!(response["slot_id"], id(4).to_string());
        assert_eq!(response["take_id"], id(5).to_string());
        assert_eq!(response["locale"], "de");
        assert_eq!(response["take_status"], "recorded");
        assert_eq!(response["slot_created"], true);
        assert_eq!(response["selected"], false);
        assert_eq!(response["build_status"], "blocked");
        assert_eq!(response["runtime_status"], "runtime_unqualified");
        assert_eq!(response["target_authority"], "not_granted");
        assert_eq!(response["publication_status"], "not_supported");
        assert_eq!(
            fs::read(temp.path().join("gore-project.json")).unwrap(),
            fixed_head
        );

        let store = WorkingProjectStore::open_existing(temp.path(), ffi_store_limits()).unwrap();
        let reopened = store
            .open_revision3_head_bytes(
                response["head_json"].as_str().unwrap().as_bytes(),
                AssetVerification::Full,
            )
            .unwrap();
        assert_eq!(reopened.project.revision, 8);
        assert_eq!(
            reopened.project.to_canonical_json().unwrap(),
            response["project_json"].as_str().unwrap()
        );
        assert!(matches!(
            reopened.project.entities[&id(4)].payload,
            EntityPayload::VoiceSlot(_)
        ));
        assert!(matches!(
            reopened.project.entities[&id(5)].payload,
            EntityPayload::VoiceTake(_)
        ));
        let encoded = response.to_string();
        assert!(!encoded.contains(source_root.path().to_string_lossy().as_ref()));
        assert!(!encoded.contains("target_authority\":\"granted"));
    }

    #[test]
    fn invalid_ogg_and_unapproved_selection_never_publish_the_fixed_head() {
        let (temp, project, project_json, basis_head, fixed_head) = published_store(7);
        let source_root = TempDir::new().unwrap();
        let before = snapshot_regular_files(temp.path());
        let invalid_source = source_root.path().join("invalid.ogg");
        fs::write(&invalid_source, b"not an ogg stream").unwrap();
        let invalid_ogg = prepare_revision3_voice_take_v1_raw(&raw_request(json!({
            "current_project_json": project_json,
            "game_root": source_root.path(),
            "root": temp.path(),
            "source": invalid_source,
            "voice_request_json": voice_request(
                &project,
                &basis_head,
                VoiceTakeStatus::Recorded,
                false,
            ),
        })));
        assert_eq!(
            invalid_ogg["error"]["code"],
            "AUTHORING_REVISION3_VOICE_OGG_INVALID"
        );
        assert_eq!(
            fs::read(temp.path().join("gore-project.json")).unwrap(),
            fixed_head
        );
        assert_eq!(snapshot_regular_files(temp.path()), before);

        let source = write_tiny_ogg(&source_root);
        let unapproved = prepare_revision3_voice_take_v1_raw(&raw_request(json!({
            "current_project_json": project.to_canonical_json().unwrap(),
            "game_root": source_root.path(),
            "root": temp.path(),
            "source": source,
            "voice_request_json": voice_request(
                &project,
                &basis_head,
                VoiceTakeStatus::Reviewed,
                true,
            ),
        })));
        assert_eq!(
            unapproved["error"]["code"],
            "AUTHORING_REVISION3_VOICE_STATUS_INVALID"
        );
        assert_eq!(
            fs::read(temp.path().join("gore-project.json")).unwrap(),
            fixed_head
        );
        assert_eq!(snapshot_regular_files(temp.path()), before);
    }

    #[test]
    fn semantic_rejections_preflight_before_any_ogg_cas_write() {
        let (temp, project, project_json, basis_head, fixed_head) = published_store(7);
        let source_root = TempDir::new().unwrap();
        let source = write_tiny_ogg(&source_root);
        let original = Revision3VoiceTakeStageRequestV1::from_json(&voice_request(
            &project,
            &basis_head,
            VoiceTakeStatus::Recorded,
            false,
        ))
        .unwrap();
        let before = snapshot_regular_files(temp.path());

        let mut missing_line = original.clone();
        missing_line.line_id = id(0x77);
        let mut take_collision = original.clone();
        take_collision.take_id = id(2);
        let mut invalid_text = original;
        invalid_text.text = Some("  ".to_owned());
        for (request, expected_code) in [
            (missing_line, "AUTHORING_REVISION3_VOICE_PROJECT_INVALID"),
            (take_collision, "AUTHORING_REVISION3_VOICE_COLLISION"),
            (invalid_text, "AUTHORING_REVISION3_VOICE_INTENT_INVALID"),
        ] {
            let response = prepare_revision3_voice_take_v1_raw(&raw_request(json!({
                "current_project_json": project_json,
                "game_root": source_root.path(),
                "root": temp.path(),
                "source": source,
                "voice_request_json": request.to_canonical_json().unwrap(),
            })));
            assert_eq!(response["error"]["code"], expected_code);
            assert_eq!(snapshot_regular_files(temp.path()), before);
            assert_eq!(
                fs::read(temp.path().join("gore-project.json")).unwrap(),
                fixed_head
            );
        }
    }

    #[test]
    fn aggregate_asset_capacity_rejection_precedes_second_read_staging_and_cas() {
        let (temp, project, project_json, basis_head, fixed_head) = published_store(7);
        let source_root = TempDir::new().unwrap();
        let source = write_tiny_ogg(&source_root);
        let before = snapshot_regular_files(temp.path());
        let wire = raw_request(json!({
            "current_project_json": project_json,
            "game_root": source_root.path(),
            "root": temp.path(),
            "source": source,
            "voice_request_json": voice_request(
                &project,
                &basis_head,
                VoiceTakeStatus::Recorded,
                false,
            ),
        }));

        // Exercise the real FFI ordering without materializing a 64-GiB test source. Only the
        // private preview supplied to the pure transaction is enlarged; Store/source code and the
        // public route always use the identity transform.
        let error = prepare_revision3_voice_take_v1_inner_with_test_seams(
            &wire,
            |mut preview| {
                preview.asset.byte_len = MAX_REVISION3_REFERENCED_ASSET_BYTES + 1;
                preview
            },
            || panic!("capacity rejection must precede the second source preparation"),
        )
        .unwrap_err();

        assert_eq!(error.code, "AUTHORING_REVISION3_VOICE_LIMIT");
        assert_eq!(snapshot_regular_files(temp.path()), before);
        assert_eq!(
            fs::read(temp.path().join("gore-project.json")).unwrap(),
            fixed_head
        );
        let staging = temp.path().join(".gore").join("staging");
        assert!(fs::read_dir(staging)
            .map(|mut entries| entries.next().is_none())
            .unwrap_or(true));
    }

    #[test]
    fn user_source_failures_have_retryable_context_without_store_mutation() {
        let (temp, project, project_json, basis_head, fixed_head) = published_store(7);
        let source_root = TempDir::new().unwrap();
        let missing = source_root.path().join("missing.ogg");
        let too_large = source_root.path().join("too-large.ogg");
        fs::File::create(&too_large)
            .unwrap()
            .set_len(64 * 1024 * 1024 + 1)
            .unwrap();
        let invalid = source_root.path().join("invalid.ogg");
        fs::write(&invalid, b"not an ogg stream").unwrap();
        let before = snapshot_regular_files(temp.path());
        let request = voice_request(&project, &basis_head, VoiceTakeStatus::Recorded, false);

        for (source, expected_code) in [
            (missing.as_path(), "AUTHORING_REVISION3_VOICE_INPUT_MISSING"),
            (source_root.path(), "AUTHORING_REVISION3_VOICE_INPUT_UNSAFE"),
            (too_large.as_path(), "AUTHORING_REVISION3_VOICE_INPUT_LIMIT"),
            (invalid.as_path(), "AUTHORING_REVISION3_VOICE_OGG_INVALID"),
        ] {
            let response = prepare_revision3_voice_take_v1_raw(&raw_request(json!({
                "current_project_json": project_json,
                "game_root": source_root.path(),
                "root": temp.path(),
                "source": source,
                "voice_request_json": request,
            })));
            assert_eq!(response["error"]["code"], expected_code, "{response}");
            assert_eq!(snapshot_regular_files(temp.path()), before);
            assert_eq!(
                fs::read(temp.path().join("gore-project.json")).unwrap(),
                fixed_head
            );
        }
    }

    #[test]
    fn changed_second_valid_source_is_rejected_before_any_ogg_cas_write() {
        let (temp, project, project_json, basis_head, fixed_head) = published_store(7);
        let source_root = TempDir::new().unwrap();
        let source = write_tiny_ogg(&source_root);
        let before = snapshot_regular_files(temp.path());
        let wire = raw_request(json!({
            "current_project_json": project_json,
            "game_root": source_root.path(),
            "root": temp.path(),
            "source": source,
            "voice_request_json": voice_request(
                &project,
                &basis_head,
                VoiceTakeStatus::Recorded,
                false,
            ),
        }));

        let error = prepare_revision3_voice_take_v1_inner_with_source_guard(&wire, || {
            fs::write(
                &source,
                include_bytes!("../../gore-vo/testdata/tiny-opus.ogg"),
            )
            .unwrap();
        })
        .unwrap_err();

        assert_eq!(error.code, "AUTHORING_REVISION3_VOICE_INPUT_CHANGED");
        assert_eq!(snapshot_regular_files(temp.path()), before);
        assert_eq!(
            fs::read(temp.path().join("gore-project.json")).unwrap(),
            fixed_head
        );
        let staging = temp.path().join(".gore").join("staging");
        assert!(fs::read_dir(staging)
            .map(|mut entries| entries.next().is_none())
            .unwrap_or(true));
    }

    #[test]
    fn foreign_identity_revision_or_target_fail_before_any_ogg_cas_write() {
        let (temp, project, project_json, basis_head, fixed_head) = published_store(7);
        let source_root = TempDir::new().unwrap();
        let source = write_tiny_ogg(&source_root);
        let original = Revision3VoiceTakeStageRequestV1::from_json(&voice_request(
            &project,
            &basis_head,
            VoiceTakeStatus::Recorded,
            false,
        ))
        .unwrap();
        let before = snapshot_regular_files(temp.path());

        let mut foreign_id = original.clone();
        foreign_id.expected_project_id = ProjectId::from_bytes([0x99; 16]);
        let mut foreign_revision = original.clone();
        foreign_revision.expected_revision -= 1;
        let mut foreign_target = original;
        foreign_target.expected_target.executable.sha256 = Sha256Digest::from_bytes([0x99; 32]);
        for request in [foreign_id, foreign_revision, foreign_target] {
            let response = prepare_revision3_voice_take_v1_raw(&raw_request(json!({
                "current_project_json": project_json,
                "game_root": source_root.path(),
                "root": temp.path(),
                "source": source,
                "voice_request_json": request.to_canonical_json().unwrap(),
            })));
            assert_eq!(
                response["error"]["code"],
                "AUTHORING_REVISION3_VOICE_PROJECT_CONFLICT"
            );
            assert_eq!(snapshot_regular_files(temp.path()), before);
            assert_eq!(
                fs::read(temp.path().join("gore-project.json")).unwrap(),
                fixed_head
            );
        }
    }
}
