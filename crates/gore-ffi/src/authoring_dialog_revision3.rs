//! Prepare-only insertion of one managed revision-3 DialogLine/LocalizationEntry pair.
//!
//! The route accepts only the exact current project bytes, a canonical semantic request, and the
//! existing Store root. It has no game-root, installation, save, compiler, build, deployment,
//! topic-registration, or fixed-head publication authority. The pure transaction creates an
//! immutable candidate which is fully reopened with asset verification; the fixed project head
//! is checked after preparation and after response construction and is never replaced here.

use std::path::Path;

use gore_authoring::{
    apply_revision3_dialog_line_insert_transaction_v1, AssetVerification, EntityId,
    ProjectRevision3, Revision3DialogBuildStatusV1, Revision3DialogLineInsertConflictV1,
    Revision3DialogLineInsertErrorV1, Revision3DialogLineInsertEvaluationV1,
    Revision3DialogLineInsertRequestV1, Revision3DialogLocalizationActionV1,
    Revision3DialogLocalizationIntentV1, Revision3DialogPublicationStatusV1,
    Revision3DialogRuntimeStatusV1, Revision3DialogTopicAuthorityV1, WorkingHead,
    WorkingProjectStore, WorkingStoreError, WorkingStoreLimits, MAX_PROJECT_JSON_BYTES,
    MAX_REVISION3_DIALOG_LINE_REQUEST_JSON_BYTES_V1,
};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::err;

pub(super) const COMMAND: &str = "authoring_store_prepare_revision3_dialog_line_v1";

const MAX_PATH_BYTES: usize = 32 * 1024;
const MAX_HEAD_JSON_BYTES: usize = 64 * 1024;
const MAX_RESPONSE_BYTES: usize = 64 * 1024 * 1024;
const MAX_ERROR_MESSAGE_BYTES: usize = 4 * 1024;
const MAX_BASIS_REVISION: u64 = i64::MAX as u64 - 1;
// Nested canonical JSON strings need at most one extra escape byte per source byte. Store roots
// retain the complete six-byte JSON escape allowance.
const MAX_WIRE_BYTES: usize = MAX_PROJECT_JSON_BYTES * 2
    + MAX_REVISION3_DIALOG_LINE_REQUEST_JSON_BYTES_V1 * 2
    + MAX_PATH_BYTES * 6
    + 4 * 1024;

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ExactWireRequest<P> {
    command: String,
    payload: P,
}

/// Field order is part of the exact canonical outer transport.
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct PrepareDialogLineWirePayload {
    current_project_json: String,
    dialog_line_request_json: String,
    root: String,
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

pub(super) fn prepare_revision3_dialog_line_v1_raw(input: &str) -> Value {
    prepare_revision3_dialog_line_v1_inner(input).unwrap_or_else(Failure::response)
}

fn prepare_revision3_dialog_line_v1_inner(input: &str) -> Result<Value, Failure> {
    prepare_revision3_dialog_line_v1_inner_with_test_seams(input, || {}, || {})
}

#[cfg(test)]
fn prepare_revision3_dialog_line_v1_inner_with_post_prepare_guard<A>(
    input: &str,
    after_checkpoint: A,
) -> Result<Value, Failure>
where
    A: FnOnce(),
{
    prepare_revision3_dialog_line_v1_inner_with_test_seams(input, after_checkpoint, || {})
}

#[cfg(test)]
fn prepare_revision3_dialog_line_v1_inner_with_final_guard<F>(
    input: &str,
    final_guard: F,
) -> Result<Value, Failure>
where
    F: FnOnce(),
{
    prepare_revision3_dialog_line_v1_inner_with_test_seams(input, || {}, final_guard)
}

fn prepare_revision3_dialog_line_v1_inner_with_test_seams<A, F>(
    input: &str,
    after_checkpoint: A,
    final_guard: F,
) -> Result<Value, Failure>
where
    A: FnOnce(),
    F: FnOnce(),
{
    let payload: PrepareDialogLineWirePayload = parse_exact_wire(input)?;
    validate_payload(&payload)?;

    let request = Revision3DialogLineInsertRequestV1::from_json(&payload.dialog_line_request_json)
        .map_err(map_request_error)?;
    require_signed_serializable(&request)?;
    validate_request_shape(&request)?;

    let store = WorkingProjectStore::open_existing(Path::new(&payload.root), ffi_store_limits())
        .map_err(map_store_error)?;
    let basis = store
        .open_current_revision3(AssetVerification::Full)
        .map_err(map_store_error)?;
    validate_basis_revision(basis.project.revision)?;
    require_signed_serializable(&basis.project)?;
    require_signed_serializable(&basis.head)?;

    let canonical_basis = basis.project.to_canonical_json().map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_DIALOG_STORE_INVARIANT",
            "the exact current revision-3 project could not be serialized canonically",
        )
    })?;
    if canonical_basis.as_bytes() != payload.current_project_json.as_bytes() {
        return Err(Failure::new(
            "AUTHORING_REVISION3_DIALOG_PROJECT_CONFLICT",
            "current_project_json differs from the exact published revision-3 project",
        ));
    }
    bind_request_to_basis(&basis.head, &basis.project, &request)?;

    let outcome = match apply_revision3_dialog_line_insert_transaction_v1(
        &basis.head,
        &canonical_basis,
        &payload.dialog_line_request_json,
    )
    .map_err(map_transaction_error)?
    {
        Revision3DialogLineInsertEvaluationV1::Applied(outcome) => *outcome,
        Revision3DialogLineInsertEvaluationV1::Rejected(rejection) => {
            return Err(map_transaction_conflict(rejection.conflict));
        }
    };
    require_signed_serializable(&outcome.project)?;
    verify_outcome_binding(&basis.head, &basis.project, &request, &outcome)?;
    match outcome.build_status {
        Revision3DialogBuildStatusV1::Blocked => {}
    }
    match outcome.runtime_status {
        Revision3DialogRuntimeStatusV1::RuntimeUnqualified => {}
    }
    match outcome.topic_authority {
        Revision3DialogTopicAuthorityV1::NotGranted => {}
    }
    match outcome.publication_status {
        Revision3DialogPublicationStatusV1::NotSupported => {}
    }

    let prepared = store
        .prepare_revision3_checkpoint(Some(&basis.head), &outcome.project)
        .map_err(map_store_error)?;
    let reopened = store
        .open_revision3_head_bytes(&prepared.head_bytes, AssetVerification::Full)
        .map_err(map_store_error)?;
    if reopened.head != prepared.head || reopened.project != outcome.project {
        return Err(Failure::new(
            "AUTHORING_REVISION3_DIALOG_STORE_INVARIANT",
            "the prepared dialog-line checkpoint did not fully reopen exactly",
        ));
    }
    let reopened_json = reopened.project.to_canonical_json().map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_DIALOG_STORE_INVARIANT",
            "the fully reopened dialog-line candidate could not be serialized",
        )
    })?;
    if reopened_json != outcome.canonical_project_json {
        return Err(Failure::new(
            "AUTHORING_REVISION3_DIALOG_STORE_INVARIANT",
            "the fully reopened dialog-line candidate changed canonical bytes",
        ));
    }

    after_checkpoint();
    require_fixed_basis(&store, &basis.head, &basis.project)?;

    let basis_head_json = canonical_head_json(&basis.head)?;
    let candidate_head_json = String::from_utf8(prepared.head_bytes).map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_DIALOG_STORE_INVARIANT",
            "the prepared dialog-line head is not UTF-8 JSON",
        )
    })?;
    if candidate_head_json.is_empty() || candidate_head_json.len() > MAX_HEAD_JSON_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_DIALOG_RESPONSE_LIMIT",
            "the prepared dialog-line head exceeds its bounded transport limit",
        ));
    }
    require_signed_serializable(&prepared.head)?;

    let localization_action = match outcome.localization_action {
        Revision3DialogLocalizationActionV1::Created => "created",
        Revision3DialogLocalizationActionV1::ReusedExact => "reused_exact",
    };
    let response = json!({
        "ok": true,
        "outcome": "prepared_unpublished",
        "basis_head_json": basis_head_json,
        "head_json": candidate_head_json,
        "project_json": outcome.canonical_project_json,
        "project_id": outcome.project.project_id.to_string(),
        "revision": outcome.project.revision,
        "line_id": outcome.line_id.to_string(),
        "localization_id": outcome.localization_id.to_string(),
        "localization_action": localization_action,
        "voice_slot_id": outcome.voice_slot_id.map(|id| id.to_string()),
        "build_status": "blocked",
        "runtime_status": "runtime_unqualified",
        "topic_authority": "not_granted",
        "publication_status": "not_supported",
    });
    enforce_response_budget(&response)?;

    final_guard();
    require_fixed_basis(&store, &basis.head, &basis.project)?;
    Ok(response)
}

fn parse_exact_wire<P>(input: &str) -> Result<P, Failure>
where
    P: DeserializeOwned + Serialize,
{
    if input.len() > MAX_WIRE_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_DIALOG_INPUT_LIMIT",
            format!("revision-3 dialog-line request exceeds the {MAX_WIRE_BYTES}-byte wire limit"),
        ));
    }
    let request: ExactWireRequest<P> =
        serde_json::from_str(input).map_err(|_| invalid_request())?;
    if request.command != COMMAND {
        return Err(invalid_request());
    }
    let canonical = serde_json::to_string(&request).map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_DIALOG_INVARIANT",
            "the dialog-line outer request could not be serialized",
        )
    })?;
    if canonical.as_bytes() != input.as_bytes() {
        return Err(invalid_request());
    }
    Ok(request.payload)
}

fn validate_payload(payload: &PrepareDialogLineWirePayload) -> Result<(), Failure> {
    validate_path(&payload.root)?;
    if payload.current_project_json.is_empty() || payload.dialog_line_request_json.is_empty() {
        return Err(invalid_request());
    }
    if payload.current_project_json.len() > MAX_PROJECT_JSON_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_DIALOG_PROJECT_LIMIT",
            format!("current_project_json exceeds the {MAX_PROJECT_JSON_BYTES}-byte limit"),
        ));
    }
    if payload.dialog_line_request_json.len() > MAX_REVISION3_DIALOG_LINE_REQUEST_JSON_BYTES_V1 {
        return Err(Failure::new(
            "AUTHORING_REVISION3_DIALOG_REQUEST_LIMIT",
            format!(
                "dialog_line_request_json exceeds the {MAX_REVISION3_DIALOG_LINE_REQUEST_JSON_BYTES_V1}-byte limit"
            ),
        ));
    }
    Ok(())
}

fn validate_path(path: &str) -> Result<(), Failure> {
    if path.is_empty() || path.len() > MAX_PATH_BYTES || path.contains('\0') {
        return Err(invalid_request());
    }
    Ok(())
}

fn validate_request_shape(request: &Revision3DialogLineInsertRequestV1) -> Result<(), Failure> {
    if is_zero_entity_id(request.line_id) {
        return Err(request_rejected("DialogLine identity must be non-zero"));
    }
    match &request.localization {
        Revision3DialogLocalizationIntentV1::Create {
            localization_id, ..
        }
        | Revision3DialogLocalizationIntentV1::ReuseExact {
            localization_id, ..
        } => {
            if is_zero_entity_id(*localization_id) || *localization_id == request.line_id {
                return Err(request_rejected(
                    "LocalizationEntry identity must be non-zero and distinct from the DialogLine",
                ));
            }
        }
    }
    if let Some(slot) = &request.voice_slot {
        let localization_id = match &request.localization {
            Revision3DialogLocalizationIntentV1::Create {
                localization_id, ..
            }
            | Revision3DialogLocalizationIntentV1::ReuseExact {
                localization_id, ..
            } => *localization_id,
        };
        if is_zero_entity_id(slot.slot_id)
            || slot.slot_id == request.line_id
            || slot.slot_id == localization_id
        {
            return Err(request_rejected(
                "VoiceSlot identity must be non-zero and distinct from the line and localization",
            ));
        }
    }
    Ok(())
}

fn bind_request_to_basis(
    head: &WorkingHead,
    project: &ProjectRevision3,
    request: &Revision3DialogLineInsertRequestV1,
) -> Result<(), Failure> {
    if request.expected_head != *head {
        return Err(head_conflict());
    }
    if request.expected_project_id != project.project_id
        || request.expected_revision != project.revision
    {
        return Err(Failure::new(
            "AUTHORING_REVISION3_DIALOG_PROJECT_CONFLICT",
            "the dialog-line request project differs from the exact published project",
        ));
    }
    if request.expected_target != project.target {
        return Err(Failure::new(
            "AUTHORING_REVISION3_DIALOG_TARGET_CONFLICT",
            "the dialog-line request target differs from the exact published project target",
        ));
    }
    Ok(())
}

fn verify_outcome_binding(
    basis_head: &WorkingHead,
    basis: &ProjectRevision3,
    request: &Revision3DialogLineInsertRequestV1,
    outcome: &gore_authoring::Revision3DialogLineInsertOutcomeV1,
) -> Result<(), Failure> {
    let expected_revision = basis
        .revision
        .checked_add(1)
        .ok_or_else(|| revision_limit("the project revision cannot be incremented"))?;
    let expected_localization_id = match &request.localization {
        Revision3DialogLocalizationIntentV1::Create {
            localization_id, ..
        }
        | Revision3DialogLocalizationIntentV1::ReuseExact {
            localization_id, ..
        } => *localization_id,
    };
    let expected_slot_id = request.voice_slot.as_ref().map(|slot| slot.slot_id);
    if outcome.basis_head != *basis_head
        || outcome.project.project_id != basis.project_id
        || outcome.project.target != basis.target
        || outcome.project.revision != expected_revision
        || outcome.line_id != request.line_id
        || outcome.localization_id != expected_localization_id
        || outcome.voice_slot_id != expected_slot_id
    {
        return Err(Failure::new(
            "AUTHORING_REVISION3_DIALOG_INVARIANT",
            "the dialog-line transaction outcome escaped its exact request basis",
        ));
    }
    let canonical = outcome.project.to_canonical_json().map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_DIALOG_INVARIANT",
            "the dialog-line transaction outcome could not be serialized canonically",
        )
    })?;
    if canonical != outcome.canonical_project_json {
        return Err(Failure::new(
            "AUTHORING_REVISION3_DIALOG_INVARIANT",
            "the dialog-line transaction outcome carries inconsistent canonical bytes",
        ));
    }
    Ok(())
}

fn validate_basis_revision(revision: u64) -> Result<(), Failure> {
    if revision > MAX_BASIS_REVISION {
        return Err(revision_limit(
            "the published project revision cannot be incremented on the signed wire",
        ));
    }
    Ok(())
}

fn require_fixed_basis(
    store: &WorkingProjectStore,
    expected_head: &WorkingHead,
    expected_project: &ProjectRevision3,
) -> Result<(), Failure> {
    let current = store
        .open_current_revision3(AssetVerification::Full)
        .map_err(map_store_error)?;
    if current.head != *expected_head || current.project != *expected_project {
        return Err(head_conflict());
    }
    Ok(())
}

fn require_signed_serializable(value: &impl Serialize) -> Result<(), Failure> {
    let value = serde_json::to_value(value).map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_DIALOG_INVARIANT",
            "a dialog-line value could not be represented on the JSON wire",
        )
    })?;
    require_signed_json_value(&value)
}

fn require_signed_json_value(value: &Value) -> Result<(), Failure> {
    match value {
        Value::Number(number) if number.as_u64().is_some_and(|value| value > i64::MAX as u64) => {
            Err(Failure::new(
                "AUTHORING_REVISION3_DIALOG_SIGNED_WIRE_LIMIT",
                "a dialog-line wire integer exceeds the signed 64-bit transport range",
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
            "AUTHORING_REVISION3_DIALOG_INVARIANT",
            "the dialog-line basis head could not be serialized",
        )
    })?;
    if value.is_empty() || value.len() > MAX_HEAD_JSON_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_DIALOG_RESPONSE_LIMIT",
            "the dialog-line basis head exceeds its bounded transport limit",
        ));
    }
    Ok(value)
}

fn enforce_response_budget(response: &Value) -> Result<(), Failure> {
    let encoded = serde_json::to_vec(response).map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_DIALOG_INVARIANT",
            "the dialog-line response could not be serialized",
        )
    })?;
    if encoded.len() > MAX_RESPONSE_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_DIALOG_RESPONSE_LIMIT",
            "the dialog-line response exceeds its bounded transport budget",
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

fn is_zero_entity_id(id: EntityId) -> bool {
    id.as_bytes().iter().all(|byte| *byte == 0)
}

fn invalid_request() -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_DIALOG_REQUEST_INVALID",
        "request must contain one exact duplicate-free command and exactly current_project_json, dialog_line_request_json, and root",
    )
}

fn request_rejected(message: &'static str) -> Failure {
    Failure::new("AUTHORING_REVISION3_DIALOG_REQUEST_REJECTED", message)
}

fn head_conflict() -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_DIALOG_HEAD_CONFLICT",
        "the published revision-3 head changed or differs from the dialog-line request",
    )
}

fn revision_limit(message: &'static str) -> Failure {
    Failure::new("AUTHORING_REVISION3_DIALOG_REVISION_LIMIT", message)
}

fn map_request_error(error: impl std::fmt::Display) -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_DIALOG_REQUEST_INVALID",
        format!("the exact dialog-line request is invalid: {error}"),
    )
}

fn map_transaction_error(error: Revision3DialogLineInsertErrorV1) -> Failure {
    match error {
        Revision3DialogLineInsertErrorV1::InvalidProject(_) => Failure::new(
            "AUTHORING_REVISION3_DIALOG_PROJECT_INVALID",
            "the exact current project is not a valid dialog-line insertion basis",
        ),
        Revision3DialogLineInsertErrorV1::InvalidRequest(error) => map_request_error(error),
        Revision3DialogLineInsertErrorV1::ReopenCandidate(_)
        | Revision3DialogLineInsertErrorV1::CanonicalReopenMismatch => Failure::new(
            "AUTHORING_REVISION3_DIALOG_INVARIANT",
            "the dialog-line transaction candidate failed exact canonical reopen",
        ),
    }
}

fn map_transaction_conflict(error: Revision3DialogLineInsertConflictV1) -> Failure {
    let code = match &error {
        Revision3DialogLineInsertConflictV1::CurrentHeadMismatch => {
            "AUTHORING_REVISION3_DIALOG_HEAD_CONFLICT"
        }
        Revision3DialogLineInsertConflictV1::ProjectIdentityMismatch { .. }
        | Revision3DialogLineInsertConflictV1::ProjectRevisionConflict { .. } => {
            "AUTHORING_REVISION3_DIALOG_PROJECT_CONFLICT"
        }
        Revision3DialogLineInsertConflictV1::ProjectTargetMismatch => {
            "AUTHORING_REVISION3_DIALOG_TARGET_CONFLICT"
        }
        Revision3DialogLineInsertConflictV1::ProjectRevisionOverflow => {
            "AUTHORING_REVISION3_DIALOG_REVISION_LIMIT"
        }
        Revision3DialogLineInsertConflictV1::ZeroEntityId { .. }
        | Revision3DialogLineInsertConflictV1::SharedEntityId
        | Revision3DialogLineInsertConflictV1::EntityIdCollision { .. } => {
            "AUTHORING_REVISION3_DIALOG_ENTITY_CONFLICT"
        }
        Revision3DialogLineInsertConflictV1::AuthoredIdentityCollision { .. } => {
            "AUTHORING_REVISION3_DIALOG_IDENTITY_CONFLICT"
        }
        Revision3DialogLineInsertConflictV1::DuplicateLocalizationIdentity { .. }
        | Revision3DialogLineInsertConflictV1::LocalizationMissingOrWrongKind { .. }
        | Revision3DialogLineInsertConflictV1::LocalizationRevisionConflict { .. }
        | Revision3DialogLineInsertConflictV1::LocalizationIdentityConflict { .. }
        | Revision3DialogLineInsertConflictV1::LocalizationAlreadyReferenced { .. } => {
            "AUTHORING_REVISION3_DIALOG_LOCALIZATION_CONFLICT"
        }
        Revision3DialogLineInsertConflictV1::VoiceSlotLocaleHasNoText { .. } => {
            "AUTHORING_REVISION3_DIALOG_LOCALE_CONFLICT"
        }
        Revision3DialogLineInsertConflictV1::EntityCapacityExceeded
        | Revision3DialogLineInsertConflictV1::CandidateTooLarge { .. } => {
            "AUTHORING_REVISION3_DIALOG_PROJECT_LIMIT"
        }
        Revision3DialogLineInsertConflictV1::CandidateNotPersistable { .. } => {
            "AUTHORING_REVISION3_DIALOG_PROJECT_INVALID"
        }
        Revision3DialogLineInsertConflictV1::InvalidLineDisplayName
        | Revision3DialogLineInsertConflictV1::InvalidLineAuthoredIdentity
        | Revision3DialogLineInsertConflictV1::InvalidSpeakerHint
        | Revision3DialogLineInsertConflictV1::InvalidLocalizationDisplayName
        | Revision3DialogLineInsertConflictV1::InvalidLocalizationId
        | Revision3DialogLineInsertConflictV1::InvalidLocalizationTexts
        | Revision3DialogLineInsertConflictV1::InvalidVoiceSlotDisplayName => {
            "AUTHORING_REVISION3_DIALOG_REQUEST_REJECTED"
        }
    };
    Failure::new(code, error.to_string())
}

fn map_store_error(error: WorkingStoreError) -> Failure {
    let code = match error {
        WorkingStoreError::InvalidLimits(_) => "AUTHORING_REVISION3_DIALOG_STORE_LIMITS_INVALID",
        WorkingStoreError::MissingRoot(_) => "AUTHORING_REVISION3_DIALOG_STORE_ROOT_MISSING",
        WorkingStoreError::UnsafePath { .. } => "AUTHORING_REVISION3_DIALOG_STORE_PATH_UNSAFE",
        WorkingStoreError::LimitExceeded { .. } => "AUTHORING_REVISION3_DIALOG_STORE_LIMIT",
        WorkingStoreError::HeadConflict { .. } => "AUTHORING_REVISION3_DIALOG_HEAD_CONFLICT",
        WorkingStoreError::MissingHead(_) => "AUTHORING_REVISION3_DIALOG_HEAD_MISSING",
        WorkingStoreError::MissingObject(_) => "AUTHORING_REVISION3_DIALOG_STORE_OBJECT_MISSING",
        WorkingStoreError::SealMismatch { .. } => "AUTHORING_REVISION3_DIALOG_STORE_SEAL_MISMATCH",
        WorkingStoreError::Collision { .. } => "AUTHORING_REVISION3_DIALOG_STORE_COLLISION",
        WorkingStoreError::InvalidJson { .. } | WorkingStoreError::NonCanonicalJson { .. } => {
            "AUTHORING_REVISION3_DIALOG_STORE_JSON_INVALID"
        }
        WorkingStoreError::Invariant(_)
        | WorkingStoreError::InvalidOgg(_)
        | WorkingStoreError::OggMetadataMismatch { .. } => {
            "AUTHORING_REVISION3_DIALOG_STORE_INVARIANT"
        }
        WorkingStoreError::StagingCleanup { .. } | WorkingStoreError::Io(_) => {
            "AUTHORING_REVISION3_DIALOG_STORE_IO"
        }
    };
    Failure::new(code, "the revision-3 dialog-line Store operation failed")
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
    use std::path::PathBuf;

    use gore_authoring::model_revision3::{
        DialogLine as Revision3DialogLine, EntityKind as Revision3EntityKind,
        LocalizationEntry as Revision3LocalizationEntry, TypedRef as Revision3TypedRef,
    };
    use gore_authoring::{
        AssetStoreIndex, ContentSeal, FormatV2, GameGenerationAnchor, LocaleCode, ProjectId,
        ProjectMeta, Revision3Entity, Revision3EntityPayload, Revision3OriginRef, SchemaRevisionV3,
        Sha256Digest,
    };
    use tempfile::TempDir;

    use super::*;

    struct PublishedStore {
        temp: TempDir,
        project: ProjectRevision3,
        project_json: String,
        head: WorkingHead,
        fixed_head_bytes: Vec<u8>,
    }

    fn entity_id(value: u8) -> EntityId {
        EntityId::from_bytes([value; 16])
    }

    fn target() -> GameGenerationAnchor {
        GameGenerationAnchor {
            executable: ContentSeal {
                byte_len: 170_000_000,
                sha256: Sha256Digest::from_bytes([0x10; 32]),
            },
        }
    }

    fn empty_project(revision: u64) -> ProjectRevision3 {
        ProjectRevision3 {
            format: FormatV2,
            schema_revision: SchemaRevisionV3,
            project_id: ProjectId::from_bytes([0x20; 16]),
            revision,
            meta: ProjectMeta {
                name: "Dialog FFI fixture".to_owned(),
                version: "1.0.0".to_owned(),
                author: "tests".to_owned(),
            },
            target: target(),
            authoring_locales: BTreeSet::from(["de".parse().unwrap()]),
            entities: BTreeMap::new(),
            asset_store: AssetStoreIndex::default(),
        }
    }

    fn published_store() -> PublishedStore {
        published_store_for(empty_project(0))
    }

    fn published_store_for(project: ProjectRevision3) -> PublishedStore {
        let temp = TempDir::new().unwrap();
        let store = WorkingProjectStore::at(temp.path(), ffi_store_limits()).unwrap();
        let project_json = project.to_canonical_json().unwrap();
        let prepared = store.prepare_revision3_checkpoint(None, &project).unwrap();
        fs::write(temp.path().join("gore-project.json"), &prepared.head_bytes).unwrap();
        PublishedStore {
            temp,
            project,
            project_json,
            head: prepared.head,
            fixed_head_bytes: prepared.head_bytes,
        }
    }

    fn create_request(store: &PublishedStore) -> Revision3DialogLineInsertRequestV1 {
        let locale: LocaleCode = "de".parse().unwrap();
        Revision3DialogLineInsertRequestV1 {
            expected_head: store.head.clone(),
            expected_project_id: store.project.project_id,
            expected_revision: store.project.revision,
            expected_target: store.project.target.clone(),
            line_id: entity_id(0x31),
            line_display_name: "Viper greeting".to_owned(),
            line_authored_identity: "GORE_DIALOG_VIPER_GREETING".to_owned(),
            speaker_hint: Some("Viper".to_owned()),
            localization: Revision3DialogLocalizationIntentV1::Create {
                localization_id: entity_id(0x32),
                display_name: "Viper greeting text".to_owned(),
                loc_id: "info_gore_viper_greeting_01".to_owned(),
                texts: BTreeMap::from([(locale.clone(), "Willkommen.".to_owned())]),
            },
            voice_slot: Some(gore_authoring::Revision3DialogEmptyVoiceSlotIntentV1 {
                slot_id: entity_id(0x33),
                locale,
                display_name: "Viper greeting German Voice".to_owned(),
            }),
        }
    }

    fn raw_request(payload: Value) -> String {
        serde_json::to_string(&json!({"command": COMMAND, "payload": payload})).unwrap()
    }

    fn call(store: &PublishedStore, request: &Revision3DialogLineInsertRequestV1) -> Value {
        prepare_revision3_dialog_line_v1_raw(&raw_request(json!({
            "current_project_json": store.project_json,
            "dialog_line_request_json": request.to_canonical_json().unwrap(),
            "root": store.temp.path(),
        })))
    }

    fn fixed_head(store: &PublishedStore) -> Vec<u8> {
        fs::read(store.temp.path().join("gore-project.json")).unwrap()
    }

    #[test]
    fn create_pair_prepares_fully_reopenable_unpublished_candidate() {
        let store = published_store();
        let request = create_request(&store);
        let response = call(&store, &request);

        assert_eq!(response["ok"], true, "{response}");
        assert_eq!(response["outcome"], "prepared_unpublished");
        assert_eq!(response["project_id"], store.project.project_id.to_string());
        assert_eq!(response["revision"], 1);
        assert_eq!(response["line_id"], request.line_id.to_string());
        assert_eq!(response["localization_id"], entity_id(0x32).to_string());
        assert_eq!(response["localization_action"], "created");
        assert_eq!(response["voice_slot_id"], entity_id(0x33).to_string());
        assert_eq!(response["build_status"], "blocked");
        assert_eq!(response["runtime_status"], "runtime_unqualified");
        assert_eq!(response["topic_authority"], "not_granted");
        assert_eq!(response["publication_status"], "not_supported");
        assert_eq!(fixed_head(&store), store.fixed_head_bytes);

        let opened_store =
            WorkingProjectStore::open_existing(store.temp.path(), ffi_store_limits()).unwrap();
        let candidate = opened_store
            .open_revision3_head_bytes(
                response["head_json"].as_str().unwrap().as_bytes(),
                AssetVerification::Full,
            )
            .unwrap();
        assert_eq!(
            candidate.project.to_canonical_json().unwrap(),
            response["project_json"]
        );
        assert!(candidate.project.entities.contains_key(&request.line_id));
        assert!(candidate.project.entities.contains_key(&entity_id(0x32)));
        assert!(candidate.project.entities.contains_key(&entity_id(0x33)));
    }

    #[test]
    fn outer_contract_rejects_game_root_and_does_not_publish() {
        let store = published_store();
        let request = create_request(&store);
        let response = prepare_revision3_dialog_line_v1_raw(&raw_request(json!({
            "current_project_json": store.project_json,
            "dialog_line_request_json": request.to_canonical_json().unwrap(),
            "game_root": "C:/Games/Gothic",
            "root": store.temp.path(),
        })));

        assert_eq!(
            response["error"]["code"],
            "AUTHORING_REVISION3_DIALOG_REQUEST_INVALID"
        );
        assert_eq!(fixed_head(&store), store.fixed_head_bytes);
    }

    #[test]
    fn candidate_size_pressure_maps_to_the_correctable_project_limit() {
        let failure =
            map_transaction_conflict(Revision3DialogLineInsertConflictV1::CandidateTooLarge {
                actual: MAX_PROJECT_JSON_BYTES + 1,
                limit: MAX_PROJECT_JSON_BYTES,
            });

        assert_eq!(failure.code, "AUTHORING_REVISION3_DIALOG_PROJECT_LIMIT");
    }

    #[test]
    fn exact_existing_localization_can_be_reused_without_claiming_a_speaker() {
        let localization_id = entity_id(0x44);
        let locale: LocaleCode = "de".parse().unwrap();
        let mut project = empty_project(7);
        project.entities.insert(
            localization_id,
            Revision3Entity {
                id: localization_id,
                display_name: "Existing managed text".to_owned(),
                origin: Revision3OriginRef::New {
                    authored_runtime_id: "info_gore_existing_01".to_owned(),
                },
                revision: 3,
                payload: Revision3EntityPayload::LocalizationEntry(Revision3LocalizationEntry {
                    loc_id: "info_gore_existing_01".to_owned(),
                    texts: BTreeMap::from([(locale.clone(), "Vorhanden.".to_owned())]),
                }),
            },
        );
        let store = published_store_for(project);
        let request = Revision3DialogLineInsertRequestV1 {
            expected_head: store.head.clone(),
            expected_project_id: store.project.project_id,
            expected_revision: store.project.revision,
            expected_target: store.project.target.clone(),
            line_id: entity_id(0x45),
            line_display_name: "Existing-text dialog line".to_owned(),
            line_authored_identity: "GORE_DIALOG_EXISTING_TEXT".to_owned(),
            speaker_hint: None,
            localization: Revision3DialogLocalizationIntentV1::ReuseExact {
                localization_id,
                expected_localization_revision: 3,
                expected_loc_id: "info_gore_existing_01".to_owned(),
            },
            voice_slot: Some(gore_authoring::Revision3DialogEmptyVoiceSlotIntentV1 {
                slot_id: entity_id(0x46),
                locale,
                display_name: "Existing German Voice".to_owned(),
            }),
        };

        let response = call(&store, &request);

        assert_eq!(response["ok"], true, "{response}");
        assert_eq!(response["localization_action"], "reused_exact");
        assert_eq!(response["localization_id"], localization_id.to_string());
        assert_eq!(response["revision"], 8);
        assert_eq!(fixed_head(&store), store.fixed_head_bytes);
    }

    #[test]
    fn exact_reuse_rejects_a_localization_already_referenced_by_another_line() {
        let localization_id = entity_id(0x54);
        let owner_line_id = entity_id(0x55);
        let locale: LocaleCode = "de".parse().unwrap();
        let mut project = empty_project(7);
        project.entities.insert(
            localization_id,
            Revision3Entity {
                id: localization_id,
                display_name: "Already owned text".to_owned(),
                origin: Revision3OriginRef::New {
                    authored_runtime_id: "info_gore_owned_01".to_owned(),
                },
                revision: 3,
                payload: Revision3EntityPayload::LocalizationEntry(Revision3LocalizationEntry {
                    loc_id: "info_gore_owned_01".to_owned(),
                    texts: BTreeMap::from([(locale, "Bereits verwendet.".to_owned())]),
                }),
            },
        );
        project.entities.insert(
            owner_line_id,
            Revision3Entity {
                id: owner_line_id,
                display_name: "Existing owner line".to_owned(),
                origin: Revision3OriginRef::New {
                    authored_runtime_id: "GORE_DIALOG_EXISTING_OWNER".to_owned(),
                },
                revision: 0,
                payload: Revision3EntityPayload::DialogLine(Revision3DialogLine {
                    localization: Revision3TypedRef::new(
                        project.project_id,
                        localization_id,
                        Revision3EntityKind::LocalizationEntry,
                    ),
                    speaker_hint: None,
                    voice_slots: BTreeMap::new(),
                }),
            },
        );
        let store = published_store_for(project);
        let request = Revision3DialogLineInsertRequestV1 {
            expected_head: store.head.clone(),
            expected_project_id: store.project.project_id,
            expected_revision: store.project.revision,
            expected_target: store.project.target.clone(),
            line_id: entity_id(0x56),
            line_display_name: "Alias attempt".to_owned(),
            line_authored_identity: "GORE_DIALOG_ALIAS_ATTEMPT".to_owned(),
            speaker_hint: None,
            localization: Revision3DialogLocalizationIntentV1::ReuseExact {
                localization_id,
                expected_localization_revision: 3,
                expected_loc_id: "info_gore_owned_01".to_owned(),
            },
            voice_slot: None,
        };

        let response = call(&store, &request);

        assert_eq!(
            response["error"]["code"],
            "AUTHORING_REVISION3_DIALOG_LOCALIZATION_CONFLICT"
        );
        assert!(response["error"]["message"]
            .as_str()
            .unwrap()
            .contains(&owner_line_id.to_string()));
        assert_eq!(fixed_head(&store), store.fixed_head_bytes);
    }

    #[test]
    fn caller_project_bytes_are_exact_current_cas_evidence() {
        let store = published_store();
        let request = create_request(&store);
        let response = prepare_revision3_dialog_line_v1_raw(&raw_request(json!({
            "current_project_json": empty_project(0).to_canonical_json().unwrap().replace("Dialog FFI fixture", "Stale fixture"),
            "dialog_line_request_json": request.to_canonical_json().unwrap(),
            "root": store.temp.path(),
        })));

        assert_eq!(
            response["error"]["code"],
            "AUTHORING_REVISION3_DIALOG_PROJECT_CONFLICT"
        );
        assert_eq!(fixed_head(&store), store.fixed_head_bytes);
    }

    #[test]
    fn concurrent_fixed_head_publish_after_prepare_fails_closed() {
        let store = published_store();
        let request = create_request(&store);
        let raw = raw_request(json!({
            "current_project_json": store.project_json,
            "dialog_line_request_json": request.to_canonical_json().unwrap(),
            "root": store.temp.path(),
        }));
        let root = PathBuf::from(store.temp.path());
        let basis_head = store.head.clone();
        let mut external_project = store.project.clone();
        external_project.meta.name = "External publisher".to_owned();

        let result = prepare_revision3_dialog_line_v1_inner_with_post_prepare_guard(&raw, || {
            let external_store =
                WorkingProjectStore::open_existing(&root, ffi_store_limits()).unwrap();
            let external = external_store
                .prepare_revision3_checkpoint(Some(&basis_head), &external_project)
                .unwrap();
            fs::write(root.join("gore-project.json"), external.head_bytes).unwrap();
        })
        .unwrap_err();

        assert_eq!(result.code, "AUTHORING_REVISION3_DIALOG_HEAD_CONFLICT");
    }

    #[test]
    fn concurrent_fixed_head_publish_after_response_construction_fails_closed() {
        let store = published_store();
        let request = create_request(&store);
        let raw = raw_request(json!({
            "current_project_json": store.project_json,
            "dialog_line_request_json": request.to_canonical_json().unwrap(),
            "root": store.temp.path(),
        }));
        let root = PathBuf::from(store.temp.path());
        let basis_head = store.head.clone();
        let mut external_project = store.project.clone();
        external_project.meta.name = "Late external publisher".to_owned();

        let result = prepare_revision3_dialog_line_v1_inner_with_final_guard(&raw, || {
            let external_store =
                WorkingProjectStore::open_existing(&root, ffi_store_limits()).unwrap();
            let external = external_store
                .prepare_revision3_checkpoint(Some(&basis_head), &external_project)
                .unwrap();
            fs::write(root.join("gore-project.json"), external.head_bytes).unwrap();
        })
        .unwrap_err();

        assert_eq!(result.code, "AUTHORING_REVISION3_DIALOG_HEAD_CONFLICT");
    }
}
