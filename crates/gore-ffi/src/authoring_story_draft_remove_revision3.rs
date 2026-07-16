//! Prepare-only semantic removal of one managed revision-3 Story Draft closure.
//!
//! The command accepts only the exact current project, Store root, and a closed request binding
//! one NPC/Quest Draft to its uniquely owned generated ScriptModule. It prepares and fully reopens
//! an immutable candidate, but never publishes the fixed head or touches game/save/build/runtime.

use std::path::Path;

use gore_authoring::{
    apply_revision3_story_draft_removal_transaction_v1, AssetVerification, EntityId,
    ProjectRevision3, Revision3StoryDraftRemovalArtifactAuthorityV1,
    Revision3StoryDraftRemovalBuildStatusV1, Revision3StoryDraftRemovalConflictV1,
    Revision3StoryDraftRemovalErrorV1, Revision3StoryDraftRemovalEvaluationV1,
    Revision3StoryDraftRemovalKindV1, Revision3StoryDraftRemovalOutcomeV1,
    Revision3StoryDraftRemovalPublicationStatusV1, Revision3StoryDraftRemovalRequestV1,
    Revision3StoryDraftRemovalRuntimeStatusV1, WorkingHead, WorkingProjectStore, WorkingStoreError,
    WorkingStoreLimits, MAX_PROJECT_JSON_BYTES,
    MAX_REVISION3_STORY_DRAFT_REMOVAL_REQUEST_JSON_BYTES_V1,
};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::err;

pub(super) const COMMAND: &str = "authoring_store_prepare_remove_revision3_story_draft_v1";

const MAX_PATH_BYTES: usize = 32 * 1024;
const MAX_HEAD_JSON_BYTES: usize = 64 * 1024;
const MAX_RESPONSE_BYTES: usize = 64 * 1024 * 1024;
const MAX_ERROR_MESSAGE_BYTES: usize = 4 * 1024;
const MAX_BASIS_REVISION: u64 = i64::MAX as u64 - 1;
const MAX_WIRE_BYTES: usize = MAX_PROJECT_JSON_BYTES * 2
    + MAX_REVISION3_STORY_DRAFT_REMOVAL_REQUEST_JSON_BYTES_V1 * 2
    + MAX_PATH_BYTES * 6
    + 4 * 1024;

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ExactWireRequest<P> {
    command: String,
    payload: P,
}

/// Field order is the exact canonical outer transport order.
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct PrepareStoryDraftRemovalWirePayload {
    current_project_json: String,
    root: String,
    story_draft_removal_request_json: String,
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

pub(super) fn prepare_revision3_story_draft_removal_v1_raw(input: &str) -> Value {
    prepare_revision3_story_draft_removal_v1_inner(input).unwrap_or_else(Failure::response)
}

fn prepare_revision3_story_draft_removal_v1_inner(input: &str) -> Result<Value, Failure> {
    prepare_revision3_story_draft_removal_v1_inner_with_test_seams(input, || {}, || {})
}

fn prepare_revision3_story_draft_removal_v1_inner_with_test_seams<A, F>(
    input: &str,
    after_checkpoint: A,
    final_guard: F,
) -> Result<Value, Failure>
where
    A: FnOnce(),
    F: FnOnce(),
{
    prepare_revision3_story_draft_removal_v1_inner_impl(input, after_checkpoint, final_guard)
}

fn prepare_revision3_story_draft_removal_v1_inner_impl<A, F>(
    input: &str,
    after_checkpoint: A,
    final_guard: F,
) -> Result<Value, Failure>
where
    A: FnOnce(),
    F: FnOnce(),
{
    let payload: PrepareStoryDraftRemovalWirePayload = parse_exact_wire(input)?;
    validate_payload(&payload)?;
    let request =
        Revision3StoryDraftRemovalRequestV1::from_json(&payload.story_draft_removal_request_json)
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

    let canonical_basis = basis.project.to_canonical_json().map_err(|_| invariant())?;
    if canonical_basis.as_bytes() != payload.current_project_json.as_bytes() {
        return Err(Failure::new(
            "AUTHORING_REVISION3_STORY_DRAFT_REMOVE_PROJECT_CONFLICT",
            "current_project_json differs from the exact published revision-3 project",
        ));
    }
    bind_request_to_basis(&basis.head, &basis.project, &request)?;

    let outcome = match apply_revision3_story_draft_removal_transaction_v1(
        &basis.head,
        &canonical_basis,
        &payload.story_draft_removal_request_json,
    )
    .map_err(map_transaction_error)?
    {
        Revision3StoryDraftRemovalEvaluationV1::Applied(outcome) => *outcome,
        Revision3StoryDraftRemovalEvaluationV1::Rejected(rejection) => {
            return Err(map_transaction_conflict(rejection.conflict));
        }
    };
    verify_outcome_binding(&basis.head, &basis.project, &request, &outcome)?;
    require_signed_serializable(&outcome.project)?;
    match outcome.build_status {
        Revision3StoryDraftRemovalBuildStatusV1::Blocked => {}
    }
    match outcome.runtime_status {
        Revision3StoryDraftRemovalRuntimeStatusV1::RuntimeUnqualified => {}
    }
    match outcome.artifact_authority {
        Revision3StoryDraftRemovalArtifactAuthorityV1::NotGranted => {}
    }
    match outcome.publication_status {
        Revision3StoryDraftRemovalPublicationStatusV1::NotSupported => {}
    }

    require_fixed_basis(&store, &basis.head, &basis.project)?;
    let prepared = store
        .prepare_revision3_checkpoint(Some(&basis.head), &outcome.project)
        .map_err(map_store_error)?;
    let reopened = store
        .open_revision3_head_bytes(&prepared.head_bytes, AssetVerification::Full)
        .map_err(map_store_error)?;
    if reopened.head != prepared.head || reopened.project != outcome.project {
        return Err(invariant());
    }
    let reopened_json = reopened
        .project
        .to_canonical_json()
        .map_err(|_| invariant())?;
    if reopened_json != outcome.canonical_project_json {
        return Err(invariant());
    }

    after_checkpoint();
    require_fixed_basis(&store, &basis.head, &basis.project)?;

    let basis_head_json = canonical_head_json(&basis.head)?;
    let head_json = checkpoint_head_json(&prepared.head_bytes)?;
    let draft_kind = match outcome.draft_kind {
        Revision3StoryDraftRemovalKindV1::NpcDraft => "npc_draft",
        Revision3StoryDraftRemovalKindV1::QuestDraft => "quest_draft",
    };
    let response = json!({
        "ok": true,
        "outcome": "prepared_remove_unpublished",
        "basis_head_json": basis_head_json,
        "head_json": head_json,
        "project_json": outcome.canonical_project_json,
        "project_id": outcome.project.project_id.to_string(),
        "revision": outcome.project.revision,
        "removed": {
            "draft": {
                "id": outcome.draft_id.to_string(),
                "kind": draft_kind,
                "revision": outcome.draft_revision,
            },
            "script_module": {
                "id": outcome.script_module_id.to_string(),
                "kind": "script_module",
                "revision": outcome.script_module_revision,
            },
        },
        "build_status": "blocked",
        "runtime_status": "runtime_unqualified",
        "artifact_authority": "not_granted",
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
            "AUTHORING_REVISION3_STORY_DRAFT_REMOVE_INPUT_LIMIT",
            format!("story Draft removal request exceeds the {MAX_WIRE_BYTES}-byte wire limit"),
        ));
    }
    let request: ExactWireRequest<P> =
        serde_json::from_str(input).map_err(|_| invalid_request())?;
    if request.command != COMMAND {
        return Err(invalid_request());
    }
    let canonical = serde_json::to_string(&request).map_err(|_| invariant())?;
    if canonical.as_bytes() != input.as_bytes() {
        return Err(invalid_request());
    }
    Ok(request.payload)
}

fn validate_payload(payload: &PrepareStoryDraftRemovalWirePayload) -> Result<(), Failure> {
    validate_path(&payload.root)?;
    if payload.current_project_json.is_empty()
        || payload.story_draft_removal_request_json.is_empty()
    {
        return Err(invalid_request());
    }
    if payload.current_project_json.len() > MAX_PROJECT_JSON_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_STORY_DRAFT_REMOVE_PROJECT_LIMIT",
            format!("current_project_json exceeds the {MAX_PROJECT_JSON_BYTES}-byte limit"),
        ));
    }
    if payload.story_draft_removal_request_json.len()
        > MAX_REVISION3_STORY_DRAFT_REMOVAL_REQUEST_JSON_BYTES_V1
    {
        return Err(Failure::new(
            "AUTHORING_REVISION3_STORY_DRAFT_REMOVE_REQUEST_LIMIT",
            format!(
                "story_draft_removal_request_json exceeds the {MAX_REVISION3_STORY_DRAFT_REMOVAL_REQUEST_JSON_BYTES_V1}-byte limit"
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

fn validate_request_shape(request: &Revision3StoryDraftRemovalRequestV1) -> Result<(), Failure> {
    if is_zero_entity_id(request.draft_id)
        || is_zero_entity_id(request.script_module_id)
        || request.draft_id == request.script_module_id
    {
        return Err(Failure::new(
            "AUTHORING_REVISION3_STORY_DRAFT_REMOVE_REQUEST_REJECTED",
            "Draft and ScriptModule identities must be distinct and non-zero",
        ));
    }
    Ok(())
}

fn bind_request_to_basis(
    head: &WorkingHead,
    project: &ProjectRevision3,
    request: &Revision3StoryDraftRemovalRequestV1,
) -> Result<(), Failure> {
    if request.expected_head != *head {
        return Err(head_conflict());
    }
    if request.expected_project_id != project.project_id
        || request.expected_revision != project.revision
    {
        return Err(Failure::new(
            "AUTHORING_REVISION3_STORY_DRAFT_REMOVE_PROJECT_CONFLICT",
            "the removal request project differs from the exact published project",
        ));
    }
    if request.expected_target != project.target {
        return Err(Failure::new(
            "AUTHORING_REVISION3_STORY_DRAFT_REMOVE_TARGET_CONFLICT",
            "the removal request target differs from the exact published project target",
        ));
    }
    Ok(())
}

fn verify_outcome_binding(
    basis_head: &WorkingHead,
    basis: &ProjectRevision3,
    request: &Revision3StoryDraftRemovalRequestV1,
    outcome: &Revision3StoryDraftRemovalOutcomeV1,
) -> Result<(), Failure> {
    let expected_revision = basis
        .revision
        .checked_add(1)
        .ok_or_else(|| revision_limit("the project revision cannot be incremented"))?;
    let mut expected = basis.clone();
    expected.revision = expected_revision;
    let removed_draft = expected.entities.remove(&request.draft_id);
    let removed_module = expected.entities.remove(&request.script_module_id);
    if removed_draft.is_none()
        || removed_module.is_none()
        || outcome.basis_head != *basis_head
        || outcome.project != expected
        || outcome.draft_id != request.draft_id
        || outcome.draft_kind != request.draft_kind
        || outcome.draft_revision != request.expected_draft_revision
        || outcome.script_module_id != request.script_module_id
        || outcome.script_module_revision != request.expected_script_module_revision
    {
        return Err(invariant());
    }
    let canonical = outcome
        .project
        .to_canonical_json()
        .map_err(|_| invariant())?;
    if canonical != outcome.canonical_project_json {
        return Err(invariant());
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

fn checkpoint_head_json(head_bytes: &[u8]) -> Result<String, Failure> {
    if head_bytes.is_empty() || head_bytes.len() > MAX_HEAD_JSON_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_STORY_DRAFT_REMOVE_RESPONSE_LIMIT",
            "prepared working-store head exceeds its response limit",
        ));
    }
    let value = std::str::from_utf8(head_bytes).map_err(|_| invariant())?;
    let parsed: WorkingHead = serde_json::from_str(value).map_err(|_| invariant())?;
    let canonical = canonical_head_json(&parsed)?;
    if canonical != value {
        return Err(invariant());
    }
    Ok(canonical)
}

fn canonical_head_json(head: &WorkingHead) -> Result<String, Failure> {
    require_signed_serializable(head)?;
    let value = serde_json::to_string(head).map_err(|_| invariant())?;
    if value.is_empty() || value.len() > MAX_HEAD_JSON_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_STORY_DRAFT_REMOVE_RESPONSE_LIMIT",
            "working-store head exceeds its response limit",
        ));
    }
    Ok(value)
}

fn require_signed_serializable(value: &impl Serialize) -> Result<(), Failure> {
    let value = serde_json::to_value(value).map_err(|_| invariant())?;
    require_signed_json_value(&value)
}

fn require_signed_json_value(value: &Value) -> Result<(), Failure> {
    match value {
        Value::Number(number) if number.as_u64().is_some_and(|value| value > i64::MAX as u64) => {
            Err(Failure::new(
                "AUTHORING_REVISION3_STORY_DRAFT_REMOVE_SIGNED_WIRE_LIMIT",
                "a story Draft removal wire integer exceeds the signed 64-bit range",
            ))
        }
        Value::Array(values) => values.iter().try_for_each(require_signed_json_value),
        Value::Object(values) => values.values().try_for_each(require_signed_json_value),
        _ => Ok(()),
    }
}

fn enforce_response_budget(response: &Value) -> Result<(), Failure> {
    let encoded = serde_json::to_vec(response).map_err(|_| invariant())?;
    if encoded.len() > MAX_RESPONSE_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_STORY_DRAFT_REMOVE_RESPONSE_LIMIT",
            "story Draft removal response exceeds its bounded transport budget",
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
        "AUTHORING_REVISION3_STORY_DRAFT_REMOVE_REQUEST_INVALID",
        "request must contain one exact duplicate-free command and exactly current_project_json, root, and story_draft_removal_request_json",
    )
}

fn head_conflict() -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_STORY_DRAFT_REMOVE_HEAD_CONFLICT",
        "the published revision-3 head changed or differs from the removal request",
    )
}

fn revision_limit(message: &'static str) -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_STORY_DRAFT_REMOVE_REVISION_LIMIT",
        message,
    )
}

fn invariant() -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_STORY_DRAFT_REMOVE_INVARIANT",
        "the prepared story Draft removal failed its exact invariant",
    )
}

fn map_request_error(error: impl std::fmt::Display) -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_STORY_DRAFT_REMOVE_REQUEST_INVALID",
        format!("the exact story Draft removal request is invalid: {error}"),
    )
}

fn map_transaction_error(error: Revision3StoryDraftRemovalErrorV1) -> Failure {
    match error {
        Revision3StoryDraftRemovalErrorV1::InvalidProject(error) => Failure::new(
            "AUTHORING_REVISION3_STORY_DRAFT_REMOVE_PROJECT_INVALID",
            format!("the exact current project is invalid: {error}"),
        ),
        Revision3StoryDraftRemovalErrorV1::InvalidRequest(error) => map_request_error(error),
        Revision3StoryDraftRemovalErrorV1::ContentIndex(error) => Failure::new(
            "AUTHORING_REVISION3_STORY_DRAFT_REMOVE_PROJECT_INVALID",
            format!("the exact project content index is invalid: {error}"),
        ),
        Revision3StoryDraftRemovalErrorV1::ReopenCandidate(_)
        | Revision3StoryDraftRemovalErrorV1::CanonicalReopenMismatch
        | Revision3StoryDraftRemovalErrorV1::CandidatePreservationMismatch => invariant(),
    }
}

fn map_transaction_conflict(error: Revision3StoryDraftRemovalConflictV1) -> Failure {
    let code = match &error {
        Revision3StoryDraftRemovalConflictV1::CurrentHeadMismatch => {
            "AUTHORING_REVISION3_STORY_DRAFT_REMOVE_HEAD_CONFLICT"
        }
        Revision3StoryDraftRemovalConflictV1::ProjectIdentityMismatch { .. }
        | Revision3StoryDraftRemovalConflictV1::ProjectRevisionConflict { .. } => {
            "AUTHORING_REVISION3_STORY_DRAFT_REMOVE_PROJECT_CONFLICT"
        }
        Revision3StoryDraftRemovalConflictV1::ProjectTargetMismatch => {
            "AUTHORING_REVISION3_STORY_DRAFT_REMOVE_TARGET_CONFLICT"
        }
        Revision3StoryDraftRemovalConflictV1::ProjectRevisionOverflow => {
            "AUTHORING_REVISION3_STORY_DRAFT_REMOVE_REVISION_LIMIT"
        }
        Revision3StoryDraftRemovalConflictV1::ZeroDraftId
        | Revision3StoryDraftRemovalConflictV1::ZeroScriptModuleId
        | Revision3StoryDraftRemovalConflictV1::IdentityCollision => {
            "AUTHORING_REVISION3_STORY_DRAFT_REMOVE_REQUEST_REJECTED"
        }
        Revision3StoryDraftRemovalConflictV1::MissingDraftEntity { .. }
        | Revision3StoryDraftRemovalConflictV1::DraftKindMismatch { .. }
        | Revision3StoryDraftRemovalConflictV1::DraftRevisionConflict { .. } => {
            "AUTHORING_REVISION3_STORY_DRAFT_REMOVE_DRAFT_CONFLICT"
        }
        Revision3StoryDraftRemovalConflictV1::DraftModuleBindingMismatch { .. }
        | Revision3StoryDraftRemovalConflictV1::InvalidScriptModuleEntity { .. }
        | Revision3StoryDraftRemovalConflictV1::ScriptModuleRevisionConflict { .. } => {
            "AUTHORING_REVISION3_STORY_DRAFT_REMOVE_MODULE_CONFLICT"
        }
        Revision3StoryDraftRemovalConflictV1::PayloadOriginGeneratorMismatch { .. } => {
            "AUTHORING_REVISION3_STORY_DRAFT_REMOVE_PROJECT_INVALID"
        }
        Revision3StoryDraftRemovalConflictV1::OwnershipConflict { .. } => {
            "AUTHORING_REVISION3_STORY_DRAFT_REMOVE_OWNERSHIP_CONFLICT"
        }
        Revision3StoryDraftRemovalConflictV1::DraftReferenced { .. } => {
            "AUTHORING_REVISION3_STORY_DRAFT_REMOVE_DRAFT_REFERENCED"
        }
        Revision3StoryDraftRemovalConflictV1::ModuleReferenced { .. } => {
            "AUTHORING_REVISION3_STORY_DRAFT_REMOVE_MODULE_REFERENCED"
        }
        Revision3StoryDraftRemovalConflictV1::ReferenceLimit { .. } => {
            "AUTHORING_REVISION3_STORY_DRAFT_REMOVE_REFERENCE_LIMIT"
        }
        Revision3StoryDraftRemovalConflictV1::CandidateTooLarge { .. } => {
            "AUTHORING_REVISION3_STORY_DRAFT_REMOVE_PROJECT_LIMIT"
        }
        Revision3StoryDraftRemovalConflictV1::CandidateNotPersistable { .. } => {
            "AUTHORING_REVISION3_STORY_DRAFT_REMOVE_PROJECT_INVALID"
        }
    };
    Failure::new(code, error.to_string())
}

fn map_store_error(error: WorkingStoreError) -> Failure {
    let code = match error {
        WorkingStoreError::InvalidLimits(_) => {
            "AUTHORING_REVISION3_STORY_DRAFT_REMOVE_STORE_LIMITS_INVALID"
        }
        WorkingStoreError::MissingRoot(_) => {
            "AUTHORING_REVISION3_STORY_DRAFT_REMOVE_STORE_ROOT_MISSING"
        }
        WorkingStoreError::UnsafePath { .. } => {
            "AUTHORING_REVISION3_STORY_DRAFT_REMOVE_STORE_PATH_UNSAFE"
        }
        WorkingStoreError::LimitExceeded { .. } => {
            "AUTHORING_REVISION3_STORY_DRAFT_REMOVE_STORE_LIMIT"
        }
        WorkingStoreError::HeadConflict { .. } => {
            "AUTHORING_REVISION3_STORY_DRAFT_REMOVE_HEAD_CONFLICT"
        }
        WorkingStoreError::MissingHead(_) => "AUTHORING_REVISION3_STORY_DRAFT_REMOVE_HEAD_MISSING",
        WorkingStoreError::MissingObject(_) => {
            "AUTHORING_REVISION3_STORY_DRAFT_REMOVE_STORE_OBJECT_MISSING"
        }
        WorkingStoreError::SealMismatch { .. } => {
            "AUTHORING_REVISION3_STORY_DRAFT_REMOVE_STORE_SEAL_MISMATCH"
        }
        WorkingStoreError::Collision { .. } => {
            "AUTHORING_REVISION3_STORY_DRAFT_REMOVE_STORE_COLLISION"
        }
        WorkingStoreError::InvalidJson { .. } | WorkingStoreError::NonCanonicalJson { .. } => {
            "AUTHORING_REVISION3_STORY_DRAFT_REMOVE_STORE_JSON_INVALID"
        }
        WorkingStoreError::Invariant(_)
        | WorkingStoreError::InvalidOgg(_)
        | WorkingStoreError::OggMetadataMismatch { .. } => {
            "AUTHORING_REVISION3_STORY_DRAFT_REMOVE_STORE_INVARIANT"
        }
        WorkingStoreError::StagingCleanup { .. } | WorkingStoreError::Io(_) => {
            "AUTHORING_REVISION3_STORY_DRAFT_REMOVE_STORE_IO"
        }
    };
    Failure::new(
        code,
        "the revision-3 story Draft removal Store operation failed",
    )
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

    use gore_authoring::model_revision3::{
        Entity, EntityKind, EntityPayload, LocalizationEntry, NpcDraft, NpcDraftInput,
        NpcParentClassInput, OriginRef,
    };
    use gore_authoring::{
        AssetStoreIndex, ContentSeal, FormatV2, GameGenerationAnchor, ProjectId, ProjectMeta,
        Revision3TypedRef, SchemaRevisionV3, Sha256Digest, LOGICAL_NPC_CLONE_GENERATOR_ID,
        LOGICAL_NPC_CLONE_GENERATOR_VERSION,
    };
    use tempfile::TempDir;

    use super::*;

    const DRAFT_REVISION: u64 = 3;
    const MODULE_REVISION: u64 = 5;

    struct PublishedFixture {
        _temp: TempDir,
        root: String,
        project: ProjectRevision3,
        project_json: String,
        head: WorkingHead,
        draft_id: EntityId,
        module_id: EntityId,
        preserved_id: EntityId,
    }

    fn digest(value: u8) -> Sha256Digest {
        Sha256Digest::from_bytes([value; 32])
    }

    fn seal(value: u8, byte_len: u64) -> ContentSeal {
        ContentSeal {
            byte_len,
            sha256: digest(value),
        }
    }

    fn entity_id(value: u8) -> EntityId {
        EntityId::from_bytes([value; 16])
    }

    fn npc_parent(project: &ProjectRevision3, runtime_class: &str) -> NpcParentClassInput {
        NpcParentClassInput {
            generation: project.target.clone(),
            source_seal: seal(0xb0, 4),
            catalog_layer: "fixture.npcs.v1".to_owned(),
            canonical_selector: runtime_class.to_owned(),
            runtime_class: runtime_class.to_owned(),
        }
    }

    fn project_with_removable_npc(
        extra_draft_backlink: bool,
    ) -> (ProjectRevision3, EntityId, EntityId, EntityId) {
        let draft_id = entity_id(0x10);
        let module_id = entity_id(0x11);
        let preserved_id = entity_id(0x40);
        let mut project = ProjectRevision3 {
            format: FormatV2,
            schema_revision: SchemaRevisionV3,
            project_id: ProjectId::from_bytes([0x01; 16]),
            revision: 7,
            meta: ProjectMeta {
                name: "Removal FFI fixture".to_owned(),
                version: "1.0.0".to_owned(),
                author: "tests".to_owned(),
            },
            target: GameGenerationAnchor {
                executable: seal(0xa0, 123),
            },
            authoring_locales: BTreeSet::from(["en".parse().unwrap()]),
            entities: BTreeMap::new(),
            asset_store: AssetStoreIndex::default(),
        };
        let owner = Revision3TypedRef::new(project.project_id, draft_id, EntityKind::NpcDraft);
        let draft = NpcDraft {
            generator_id: LOGICAL_NPC_CLONE_GENERATOR_ID.to_owned(),
            generator_version: LOGICAL_NPC_CLONE_GENERATOR_VERSION,
            input: NpcDraftInput {
                target: project.target.clone(),
                module_namespace: "PROJECT.NPCS.REMOVABLE".to_owned(),
                unique_name: "GORE_REMOVABLE_NPC".to_owned(),
                parent_character_definition: npc_parent(&project, "UCharacterDefinition_Asghan"),
                parent_ai_agent_config: npc_parent(&project, "UAIAgentConfig_Asghan"),
                parent_spawn_definition: npc_parent(&project, "USpawnAIAgentDefinition_Asghan"),
            },
            script_module: Revision3TypedRef::new(
                project.project_id,
                module_id,
                EntityKind::ScriptModule,
            ),
        };
        let module = draft.regenerate_script_module(owner.clone()).unwrap();
        project.entities.insert(
            draft_id,
            Entity {
                id: draft_id,
                display_name: "Removable NPC".to_owned(),
                origin: OriginRef::New {
                    authored_runtime_id: "GORE_REMOVABLE_NPC".to_owned(),
                },
                revision: DRAFT_REVISION,
                payload: EntityPayload::NpcDraft(draft),
            },
        );
        project.entities.insert(
            module_id,
            Entity {
                id: module_id,
                display_name: "Removable NPC module".to_owned(),
                origin: OriginRef::Generated {
                    generator_id: LOGICAL_NPC_CLONE_GENERATOR_ID.to_owned(),
                    generator_version: LOGICAL_NPC_CLONE_GENERATOR_VERSION,
                    owner,
                },
                revision: MODULE_REVISION,
                payload: EntityPayload::ScriptModule(module),
            },
        );
        project.entities.insert(
            preserved_id,
            Entity {
                id: preserved_id,
                display_name: "Preserved localization".to_owned(),
                origin: if extra_draft_backlink {
                    OriginRef::Generated {
                        generator_id: "fixture.extra-owner".to_owned(),
                        generator_version: 1,
                        owner: Revision3TypedRef::new(
                            project.project_id,
                            draft_id,
                            EntityKind::NpcDraft,
                        ),
                    }
                } else {
                    OriginRef::New {
                        authored_runtime_id: "LOC_PRESERVED".to_owned(),
                    }
                },
                revision: 11,
                payload: EntityPayload::LocalizationEntry(LocalizationEntry {
                    loc_id: "LOC_PRESERVED".to_owned(),
                    texts: BTreeMap::from([("en".parse().unwrap(), "Preserved".to_owned())]),
                }),
            },
        );
        (project, draft_id, module_id, preserved_id)
    }

    fn publish(extra_draft_backlink: bool) -> PublishedFixture {
        let temp = TempDir::new().unwrap();
        let root = temp.path().to_string_lossy().into_owned();
        let store = WorkingProjectStore::at(temp.path(), ffi_store_limits()).unwrap();
        let (project, draft_id, module_id, preserved_id) =
            project_with_removable_npc(extra_draft_backlink);
        let project_json = project.to_canonical_json().unwrap();
        let prepared = store.prepare_revision3_checkpoint(None, &project).unwrap();
        fs::write(temp.path().join("gore-project.json"), &prepared.head_bytes).unwrap();
        PublishedFixture {
            _temp: temp,
            root,
            project,
            project_json,
            head: prepared.head,
            draft_id,
            module_id,
            preserved_id,
        }
    }

    fn request(fixture: &PublishedFixture) -> Revision3StoryDraftRemovalRequestV1 {
        Revision3StoryDraftRemovalRequestV1 {
            expected_head: fixture.head.clone(),
            expected_project_id: fixture.project.project_id,
            expected_revision: fixture.project.revision,
            expected_target: fixture.project.target.clone(),
            draft_id: fixture.draft_id,
            draft_kind: Revision3StoryDraftRemovalKindV1::NpcDraft,
            expected_draft_revision: DRAFT_REVISION,
            script_module_id: fixture.module_id,
            expected_script_module_revision: MODULE_REVISION,
        }
    }

    fn raw(fixture: &PublishedFixture) -> String {
        serde_json::to_string(&ExactWireRequest {
            command: COMMAND.to_owned(),
            payload: PrepareStoryDraftRemovalWirePayload {
                current_project_json: fixture.project_json.clone(),
                root: fixture.root.clone(),
                story_draft_removal_request_json: request(fixture).to_canonical_json().unwrap(),
            },
        })
        .unwrap()
    }

    fn error_code(value: &Value) -> &str {
        value
            .pointer("/error/code")
            .and_then(Value::as_str)
            .expect("error response has a code")
    }

    fn publish_rival(
        root: &str,
        basis_head: &WorkingHead,
        basis: &ProjectRevision3,
        preserved_id: EntityId,
    ) {
        let store =
            WorkingProjectStore::open_existing(Path::new(root), ffi_store_limits()).unwrap();
        let mut rival = basis.clone();
        rival.revision += 1;
        rival.entities.get_mut(&preserved_id).unwrap().display_name = "Concurrent edit".to_owned();
        let prepared = store
            .prepare_revision3_checkpoint(Some(basis_head), &rival)
            .unwrap();
        fs::write(
            Path::new(root).join("gore-project.json"),
            prepared.head_bytes,
        )
        .unwrap();
    }

    #[test]
    fn prepare_success_is_exact_prepare_only_and_reopenable() {
        let fixture = publish(false);
        let response = prepare_revision3_story_draft_removal_v1_raw(&raw(&fixture));
        assert_eq!(response["ok"], true);
        assert_eq!(response["outcome"], "prepared_remove_unpublished");
        assert_eq!(
            response["removed"]["draft"]["id"],
            fixture.draft_id.to_string()
        );
        assert_eq!(
            response["removed"]["script_module"]["id"],
            fixture.module_id.to_string()
        );
        assert_eq!(response["build_status"], "blocked");
        assert_eq!(response["runtime_status"], "runtime_unqualified");
        assert_eq!(response["artifact_authority"], "not_granted");
        assert_eq!(response["publication_status"], "not_supported");

        let candidate =
            ProjectRevision3::from_json(response["project_json"].as_str().unwrap()).unwrap();
        assert_eq!(candidate.revision, fixture.project.revision + 1);
        assert!(!candidate.entities.contains_key(&fixture.draft_id));
        assert!(!candidate.entities.contains_key(&fixture.module_id));
        assert_eq!(
            candidate.entities.get(&fixture.preserved_id),
            fixture.project.entities.get(&fixture.preserved_id)
        );
        assert_eq!(candidate.asset_store, fixture.project.asset_store);

        let store =
            WorkingProjectStore::open_existing(Path::new(&fixture.root), ffi_store_limits())
                .unwrap();
        let current = store
            .open_current_revision3(AssetVerification::Full)
            .unwrap();
        assert_eq!(current.head, fixture.head);
        assert_eq!(current.project, fixture.project);
        let reopened = store
            .open_revision3_head_bytes(
                response["head_json"].as_str().unwrap().as_bytes(),
                AssetVerification::Full,
            )
            .unwrap();
        assert_eq!(reopened.project, candidate);
    }

    #[test]
    fn exact_wire_and_public_dispatch_reject_forged_authority_and_malformed_shapes() {
        let fixture = publish(false);
        let valid = raw(&fixture);
        let parsed: PrepareStoryDraftRemovalWirePayload = parse_exact_wire(&valid).unwrap();
        assert_eq!(parsed.root, fixture.root);

        let duplicate = valid.replacen("\"root\":", "\"root\":\"forged\",\"root\":", 1);
        let mut missing: Value = serde_json::from_str(&valid).unwrap();
        missing["payload"].as_object_mut().unwrap().remove("root");
        let cases = [
            valid.clone() + "\n",
            duplicate,
            missing.to_string(),
            serde_json::to_string(&json!({
                "command": COMMAND,
                "payload": {
                    "current_project_json": fixture.project_json,
                    "root": fixture.root,
                    "story_draft_removal_request_json": request(&fixture).to_canonical_json().unwrap(),
                    "publication_authority": "forged"
                }
            }))
            .unwrap(),
        ];
        for input in cases {
            let response = prepare_revision3_story_draft_removal_v1_raw(&input);
            assert_eq!(
                error_code(&response),
                "AUTHORING_REVISION3_STORY_DRAFT_REMOVE_REQUEST_INVALID"
            );
        }

        let forged = serde_json::to_string(&json!({
            "command": COMMAND,
            "payload": {
                "current_project_json": fixture.project_json,
                "root": fixture.root,
                "story_draft_removal_request_json": request(&fixture).to_canonical_json().unwrap(),
                "runtime_authority": true
            }
        }))
        .unwrap();
        let public: Value = serde_json::from_str(&crate::execute_json(&forged)).unwrap();
        assert_eq!(
            error_code(&public),
            "AUTHORING_REVISION3_STORY_DRAFT_REMOVE_REQUEST_INVALID"
        );
        assert_eq!(
            error_code(&prepare_revision3_story_draft_removal_v1_raw(
                &"x".repeat(MAX_WIRE_BYTES + 1)
            )),
            "AUTHORING_REVISION3_STORY_DRAFT_REMOVE_INPUT_LIMIT"
        );
    }

    #[test]
    fn semantic_backlink_is_reported_without_preparing_a_candidate() {
        let fixture = publish(true);
        let response = prepare_revision3_story_draft_removal_v1_raw(&raw(&fixture));
        assert_eq!(
            error_code(&response),
            "AUTHORING_REVISION3_STORY_DRAFT_REMOVE_DRAFT_REFERENCED"
        );
        let store =
            WorkingProjectStore::open_existing(Path::new(&fixture.root), ffi_store_limits())
                .unwrap();
        assert_eq!(store.current_head().unwrap(), Some(fixture.head));
    }

    #[test]
    fn fixed_head_drift_is_caught_after_checkpoint_and_at_final_response_gate() {
        let after = publish(false);
        let after_raw = raw(&after);
        let after_root = after.root.clone();
        let after_head = after.head.clone();
        let after_project = after.project.clone();
        let after_preserved = after.preserved_id;
        let error = prepare_revision3_story_draft_removal_v1_inner_with_test_seams(
            &after_raw,
            move || publish_rival(&after_root, &after_head, &after_project, after_preserved),
            || {},
        )
        .unwrap_err();
        assert_eq!(
            error.code,
            "AUTHORING_REVISION3_STORY_DRAFT_REMOVE_HEAD_CONFLICT"
        );

        let final_gate = publish(false);
        let final_raw = raw(&final_gate);
        let final_root = final_gate.root.clone();
        let final_head = final_gate.head.clone();
        let final_project = final_gate.project.clone();
        let final_preserved = final_gate.preserved_id;
        let error = prepare_revision3_story_draft_removal_v1_inner_with_test_seams(
            &final_raw,
            || {},
            move || publish_rival(&final_root, &final_head, &final_project, final_preserved),
        )
        .unwrap_err();
        assert_eq!(
            error.code,
            "AUTHORING_REVISION3_STORY_DRAFT_REMOVE_HEAD_CONFLICT"
        );
    }
}
