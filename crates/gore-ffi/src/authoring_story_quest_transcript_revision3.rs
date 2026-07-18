//! Native prepare-only orchestration for one exact revision-3 Quest transcript edit.
//!
//! The route accepts only canonical project/request transports and an existing Store root. It
//! fully reopens and preflights the fixed Quest basis, prepares an immutable candidate, fully
//! reopens that candidate, and rechecks the fixed basis before success. It has no game, save,
//! source publication, build, deployment, runtime, topic-registration, or fixed-head authority.

use std::path::Path;

use gore_authoring::{
    apply_revision3_quest_transcript_edit_transaction_v1, AssetVerification,
    Revision3EntityPayload, Revision3QuestCollisionSourceErrorV2,
    Revision3QuestTranscriptBuildStatusV1, Revision3QuestTranscriptEditConflictV1,
    Revision3QuestTranscriptEditErrorV1, Revision3QuestTranscriptEditEvaluationV1,
    Revision3QuestTranscriptEditOutcomeV1, Revision3QuestTranscriptEditRequestV1,
    Revision3QuestTranscriptModeV1, Revision3QuestTranscriptPublicationStatusV1,
    Revision3QuestTranscriptRuntimeStatusV1, Revision3QuestTranscriptTopicAuthorityV1, WorkingHead,
    WorkingProjectStore, WorkingStoreError, WorkingStoreLimits, MAX_PROJECT_JSON_BYTES,
    MAX_REVISION3_QUEST_TRANSCRIPT_REQUEST_JSON_BYTES_V1,
};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::err;

pub(super) const COMMAND: &str = "authoring_store_prepare_revision3_quest_transcript_v1";

const MAX_PATH_BYTES: usize = 32 * 1024;
const MAX_HEAD_JSON_BYTES: usize = 64 * 1024;
const MAX_RESPONSE_BYTES: usize = 64 * 1024 * 1024;
const MAX_ERROR_MESSAGE_BYTES: usize = 4 * 1024;
const MAX_BASIS_REVISION: u64 = i64::MAX as u64 - 1;
const MAX_WIRE_BYTES: usize = MAX_PROJECT_JSON_BYTES * 2
    + MAX_REVISION3_QUEST_TRANSCRIPT_REQUEST_JSON_BYTES_V1 * 2
    + MAX_PATH_BYTES * 6
    + 4 * 1024;

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ExactWireRequest<P> {
    command: String,
    payload: P,
}

/// Field order is part of the strict canonical outer transport.
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct PrepareQuestTranscriptWirePayload {
    current_project_json: String,
    quest_transcript_request_json: String,
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

pub(super) fn prepare_revision3_quest_transcript_v1_raw(input: &str) -> Value {
    prepare_revision3_quest_transcript_v1_inner(input).unwrap_or_else(Failure::response)
}

fn prepare_revision3_quest_transcript_v1_inner(input: &str) -> Result<Value, Failure> {
    prepare_revision3_quest_transcript_v1_inner_with_test_seams(input, || {}, || {})
}

#[cfg(test)]
fn prepare_revision3_quest_transcript_v1_inner_with_final_guard<F>(
    input: &str,
    final_guard: F,
) -> Result<Value, Failure>
where
    F: FnOnce(),
{
    prepare_revision3_quest_transcript_v1_inner_with_test_seams(input, || {}, final_guard)
}

fn prepare_revision3_quest_transcript_v1_inner_with_test_seams<B, F>(
    input: &str,
    before_checkpoint: B,
    final_guard: F,
) -> Result<Value, Failure>
where
    B: FnOnce(),
    F: FnOnce(),
{
    let payload: PrepareQuestTranscriptWirePayload = parse_exact_wire(input)?;
    validate_payload(&payload)?;
    // Reject the bounded semantic request before any potentially expensive Store reopen.
    let request =
        Revision3QuestTranscriptEditRequestV1::from_json(&payload.quest_transcript_request_json)
            .map_err(map_request_error)?;
    require_signed_serializable(&request)?;

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
            "AUTHORING_REVISION3_QUEST_TRANSCRIPT_STORE_INVARIANT",
            "the exact current revision-3 project could not be serialized canonically",
        )
    })?;
    if canonical_basis.as_bytes() != payload.current_project_json.as_bytes() {
        return Err(Failure::new(
            "AUTHORING_REVISION3_QUEST_TRANSCRIPT_PROJECT_CONFLICT",
            "current_project_json differs from the exact published revision-3 project",
        ));
    }
    bind_request_to_basis(&basis.head, &basis.project, &request)?;

    // Exact-current Quest source closure is native preflight evidence only. Transcript metadata
    // never enters generated source and this capsule is intentionally dropped without exposure.
    let source = store
        .prepare_current_revision3_quest_collision_source_v2(&basis.head)
        .map_err(map_current_source_error)?;
    if source.current_head() != &basis.head
        || source.project_id() != basis.project.project_id
        || source.project_revision() != basis.project.revision
        || source.target() != &basis.project.target
    {
        return Err(Failure::new(
            "AUTHORING_REVISION3_QUEST_TRANSCRIPT_STORE_INVARIANT",
            "the exact-current Quest source preflight changed its Store binding",
        ));
    }
    drop(source);

    let outcome = match apply_revision3_quest_transcript_edit_transaction_v1(
        &basis.head,
        &canonical_basis,
        &payload.quest_transcript_request_json,
    )
    .map_err(map_transaction_error)?
    {
        Revision3QuestTranscriptEditEvaluationV1::Applied(outcome) => *outcome,
        Revision3QuestTranscriptEditEvaluationV1::Rejected(rejection) => {
            return Err(map_transaction_conflict(rejection.conflict));
        }
    };
    require_signed_serializable(&outcome.project)?;
    verify_outcome_binding(&basis.head, &basis.project, &request, &outcome)?;
    match outcome.build_status {
        Revision3QuestTranscriptBuildStatusV1::Blocked => {}
    }
    match outcome.runtime_status {
        Revision3QuestTranscriptRuntimeStatusV1::RuntimeUnqualified => {}
    }
    match outcome.topic_authority {
        Revision3QuestTranscriptTopicAuthorityV1::NotGranted => {}
    }
    match outcome.publication_status {
        Revision3QuestTranscriptPublicationStatusV1::NotSupported => {}
    }

    before_checkpoint();
    let prepared = store
        .prepare_revision3_checkpoint(Some(&basis.head), &outcome.project)
        .map_err(map_store_error)?;
    let reopened = store
        .open_revision3_head_bytes(&prepared.head_bytes, AssetVerification::Full)
        .map_err(map_store_error)?;
    if reopened.head != prepared.head || reopened.project != outcome.project {
        return Err(Failure::new(
            "AUTHORING_REVISION3_QUEST_TRANSCRIPT_STORE_INVARIANT",
            "the prepared Quest transcript checkpoint did not fully reopen exactly",
        ));
    }
    let reopened_json = reopened.project.to_canonical_json().map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_QUEST_TRANSCRIPT_STORE_INVARIANT",
            "the fully reopened Quest transcript candidate could not be serialized",
        )
    })?;
    if reopened_json != outcome.canonical_project_json {
        return Err(Failure::new(
            "AUTHORING_REVISION3_QUEST_TRANSCRIPT_STORE_INVARIANT",
            "the fully reopened Quest transcript candidate changed canonical bytes",
        ));
    }

    final_guard();
    require_fixed_basis(&store, &basis.head, &basis.project)?;

    let basis_head_json = canonical_head_json(&basis.head)?;
    let candidate_head_json = String::from_utf8(prepared.head_bytes).map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_QUEST_TRANSCRIPT_STORE_INVARIANT",
            "the prepared Quest transcript head is not UTF-8 JSON",
        )
    })?;
    if candidate_head_json.is_empty() || candidate_head_json.len() > MAX_HEAD_JSON_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_QUEST_TRANSCRIPT_RESPONSE_LIMIT",
            "the prepared Quest transcript head exceeds its bounded transport limit",
        ));
    }
    require_signed_serializable(&prepared.head)?;

    let mode = match outcome.mode {
        Revision3QuestTranscriptModeV1::Replace => "replace",
        Revision3QuestTranscriptModeV1::CreateAndInsert => "create_and_insert",
    };
    let (created_line_id, created_localization_id, created_voice_slot_id, localization_action) =
        match &outcome.created {
            Some(created) => (
                Some(created.line_id.to_string()),
                Some(created.localization_id.to_string()),
                created.voice_slot_id.map(|id| id.to_string()),
                Some(match created.localization_action {
                    gore_authoring::Revision3DialogLocalizationActionV1::Created => "created",
                    gore_authoring::Revision3DialogLocalizationActionV1::ReusedExact => {
                        "reused_exact"
                    }
                }),
            ),
            None => (None, None, None, None),
        };
    let response = json!({
        "ok": true,
        "outcome": "prepared_unpublished",
        "basis_head_json": basis_head_json,
        "head_json": candidate_head_json,
        "project_json": outcome.canonical_project_json,
        "project_id": outcome.project.project_id.to_string(),
        "revision": outcome.project.revision,
        "quest_id": outcome.quest_id.to_string(),
        "quest_revision": outcome.quest_revision,
        "module_id": outcome.script_module_id.to_string(),
        "module_revision": outcome.script_module_revision,
        "mode": mode,
        "transcript_count": outcome.transcript_count,
        "created_line_id": created_line_id,
        "created_localization_id": created_localization_id,
        "created_voice_slot_id": created_voice_slot_id,
        "localization_action": localization_action,
        "build_status": "blocked",
        "runtime_status": "runtime_unqualified",
        "topic_authority": "not_granted",
        "publication_status": "not_supported",
    });
    enforce_response_budget(&response)?;

    // A fixed-head race after candidate preparation can leave only immutable CAS orphans.
    require_fixed_basis(&store, &basis.head, &basis.project)?;
    Ok(response)
}

fn verify_outcome_binding(
    basis_head: &WorkingHead,
    basis: &gore_authoring::ProjectRevision3,
    request: &Revision3QuestTranscriptEditRequestV1,
    outcome: &Revision3QuestTranscriptEditOutcomeV1,
) -> Result<(), Failure> {
    let basis_quest_entity = basis.entities.get(&request.quest_id).ok_or_else(|| {
        Failure::new(
            "AUTHORING_REVISION3_QUEST_TRANSCRIPT_INVARIANT",
            "the bound Quest disappeared during transaction verification",
        )
    })?;
    let Revision3EntityPayload::QuestDraft(basis_quest) = &basis_quest_entity.payload else {
        return Err(Failure::new(
            "AUTHORING_REVISION3_QUEST_TRANSCRIPT_INVARIANT",
            "the bound Quest changed kind during transaction verification",
        ));
    };
    let basis_module = basis
        .entities
        .get(&basis_quest.script_module.id)
        .ok_or_else(|| {
            Failure::new(
                "AUTHORING_REVISION3_QUEST_TRANSCRIPT_INVARIANT",
                "the bound Quest module disappeared during transaction verification",
            )
        })?;
    let outcome_quest_entity =
        outcome
            .project
            .entities
            .get(&request.quest_id)
            .ok_or_else(|| {
                Failure::new(
                    "AUTHORING_REVISION3_QUEST_TRANSCRIPT_INVARIANT",
                    "the edited Quest disappeared during transaction verification",
                )
            })?;
    let Revision3EntityPayload::QuestDraft(outcome_quest) = &outcome_quest_entity.payload else {
        return Err(Failure::new(
            "AUTHORING_REVISION3_QUEST_TRANSCRIPT_INVARIANT",
            "the edited Quest changed kind during transaction verification",
        ));
    };
    if outcome.basis_head != *basis_head
        || outcome.quest_id != request.quest_id
        || outcome.project.project_id != basis.project_id
        || outcome.project.target != basis.target
        || outcome.project.revision != basis.revision + 1
        || outcome.quest_revision != basis_quest_entity.revision + 1
        || outcome_quest_entity.revision != outcome.quest_revision
        || outcome.script_module_id != basis_quest.script_module.id
        || outcome.script_module_revision != basis_module.revision
        || outcome.mode != request.intent.mode()
        || outcome.transcript_count != outcome_quest.transcript.len() as u64
        || outcome.project.entities.get(&basis_quest.script_module.id) != Some(basis_module)
    {
        return Err(Failure::new(
            "AUTHORING_REVISION3_QUEST_TRANSCRIPT_INVARIANT",
            "the Quest transcript transaction changed its exact project/entity binding",
        ));
    }
    match (&request.intent, &outcome.created) {
        (gore_authoring::Revision3QuestTranscriptIntentV1::Replace { .. }, None) => {}
        (
            gore_authoring::Revision3QuestTranscriptIntentV1::CreateAndInsert { line, .. },
            Some(created),
        ) if created.line_id == line.line_id
            && created.localization_id == line.localization.localization_id()
            && created.voice_slot_id == line.voice_slot.as_ref().map(|slot| slot.slot_id) => {}
        _ => {
            return Err(Failure::new(
                "AUTHORING_REVISION3_QUEST_TRANSCRIPT_INVARIANT",
                "the Quest transcript transaction returned inconsistent creation metadata",
            ));
        }
    }
    Ok(())
}

fn parse_exact_wire<P>(input: &str) -> Result<P, Failure>
where
    P: DeserializeOwned + Serialize,
{
    if input.len() > MAX_WIRE_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_QUEST_TRANSCRIPT_INPUT_LIMIT",
            format!("Quest transcript request exceeds the {MAX_WIRE_BYTES}-byte wire limit"),
        ));
    }
    let request: ExactWireRequest<P> =
        serde_json::from_str(input).map_err(|_| invalid_request())?;
    if request.command != COMMAND {
        return Err(invalid_request());
    }
    let canonical = serde_json::to_string(&request).map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_QUEST_TRANSCRIPT_INVARIANT",
            "the Quest transcript outer request could not be serialized",
        )
    })?;
    if canonical != input {
        return Err(invalid_request());
    }
    Ok(request.payload)
}

fn validate_payload(payload: &PrepareQuestTranscriptWirePayload) -> Result<(), Failure> {
    if payload.root.is_empty()
        || payload.root.len() > MAX_PATH_BYTES
        || payload.root.contains('\0')
        || payload.current_project_json.is_empty()
        || payload.quest_transcript_request_json.is_empty()
    {
        return Err(invalid_request());
    }
    if payload.current_project_json.len() > MAX_PROJECT_JSON_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_QUEST_TRANSCRIPT_PROJECT_LIMIT",
            format!("current_project_json exceeds the {MAX_PROJECT_JSON_BYTES}-byte limit"),
        ));
    }
    if payload.quest_transcript_request_json.len()
        > MAX_REVISION3_QUEST_TRANSCRIPT_REQUEST_JSON_BYTES_V1
    {
        return Err(Failure::new(
            "AUTHORING_REVISION3_QUEST_TRANSCRIPT_REQUEST_LIMIT",
            format!(
                "quest_transcript_request_json exceeds the {MAX_REVISION3_QUEST_TRANSCRIPT_REQUEST_JSON_BYTES_V1}-byte limit"
            ),
        ));
    }
    Ok(())
}

fn bind_request_to_basis(
    head: &WorkingHead,
    project: &gore_authoring::ProjectRevision3,
    request: &Revision3QuestTranscriptEditRequestV1,
) -> Result<(), Failure> {
    if request.expected_head != *head {
        return Err(head_conflict());
    }
    if request.expected_project_id != project.project_id
        || request.expected_revision != project.revision
    {
        return Err(Failure::new(
            "AUTHORING_REVISION3_QUEST_TRANSCRIPT_PROJECT_CONFLICT",
            "the Quest transcript request project differs from the exact published project",
        ));
    }
    if request.expected_target != project.target {
        return Err(Failure::new(
            "AUTHORING_REVISION3_QUEST_TRANSCRIPT_TARGET_CONFLICT",
            "the Quest transcript request target differs from the exact published target",
        ));
    }
    let entity = project.entities.get(&request.quest_id).ok_or_else(|| {
        quest_conflict("the requested Quest does not exist in the exact published project")
    })?;
    let Revision3EntityPayload::QuestDraft(quest) = &entity.payload else {
        return Err(quest_conflict(
            "the requested entity is not a Quest in the exact published project",
        ));
    };
    if entity.revision != request.expected_quest_revision {
        return Err(quest_conflict(
            "the requested Quest revision differs from the exact published entity revision",
        ));
    }
    if quest.script_module.project_id != project.project_id {
        return Err(Failure::new(
            "AUTHORING_REVISION3_QUEST_TRANSCRIPT_PROJECT_INVALID",
            "the exact Quest has a foreign owned module binding",
        ));
    }
    Ok(())
}

fn require_fixed_basis(
    store: &WorkingProjectStore,
    expected_head: &WorkingHead,
    expected_project: &gore_authoring::ProjectRevision3,
) -> Result<(), Failure> {
    let current = store
        .open_current_revision3(AssetVerification::Full)
        .map_err(map_store_error)?;
    if &current.head != expected_head || &current.project != expected_project {
        return Err(head_conflict());
    }
    Ok(())
}

fn validate_basis_revision(revision: u64) -> Result<(), Failure> {
    if revision > MAX_BASIS_REVISION {
        return Err(Failure::new(
            "AUTHORING_REVISION3_QUEST_TRANSCRIPT_REVISION_LIMIT",
            format!("the published basis revision exceeds {MAX_BASIS_REVISION}"),
        ));
    }
    Ok(())
}

fn require_signed_serializable(value: &impl Serialize) -> Result<(), Failure> {
    let value = serde_json::to_value(value).map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_QUEST_TRANSCRIPT_INVARIANT",
            "a Quest transcript wire value could not be inspected",
        )
    })?;
    require_signed_json_value(&value)
}

fn require_signed_json_value(value: &Value) -> Result<(), Failure> {
    match value {
        Value::Number(number) if number.as_u64().is_some_and(|value| value > i64::MAX as u64) => {
            Err(Failure::new(
                "AUTHORING_REVISION3_QUEST_TRANSCRIPT_SIGNED_WIRE_LIMIT",
                "a Quest transcript wire integer exceeds signed 64-bit transport",
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
            "AUTHORING_REVISION3_QUEST_TRANSCRIPT_INVARIANT",
            "the Quest transcript basis head could not be serialized",
        )
    })?;
    if value.is_empty() || value.len() > MAX_HEAD_JSON_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_QUEST_TRANSCRIPT_RESPONSE_LIMIT",
            "the Quest transcript basis head exceeds its bounded transport limit",
        ));
    }
    Ok(value)
}

fn enforce_response_budget(response: &Value) -> Result<(), Failure> {
    let encoded = serde_json::to_vec(response).map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_QUEST_TRANSCRIPT_INVARIANT",
            "the Quest transcript response could not be serialized",
        )
    })?;
    if encoded.len() > MAX_RESPONSE_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_QUEST_TRANSCRIPT_RESPONSE_LIMIT",
            "the Quest transcript response exceeds its bounded transport budget",
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
        "AUTHORING_REVISION3_QUEST_TRANSCRIPT_REQUEST_INVALID",
        "request must contain one exact duplicate-free command and exactly current_project_json, quest_transcript_request_json, and root",
    )
}

fn head_conflict() -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_QUEST_TRANSCRIPT_HEAD_CONFLICT",
        "the published revision-3 head changed or differs from the Quest transcript request",
    )
}

fn quest_conflict(message: &'static str) -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_QUEST_TRANSCRIPT_QUEST_CONFLICT",
        message,
    )
}

fn map_request_error(error: impl std::fmt::Display) -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_QUEST_TRANSCRIPT_REQUEST_INVALID",
        format!("the exact Quest transcript request is invalid: {error}"),
    )
}

fn map_transaction_error(error: Revision3QuestTranscriptEditErrorV1) -> Failure {
    match error {
        Revision3QuestTranscriptEditErrorV1::InvalidProject(_) => Failure::new(
            "AUTHORING_REVISION3_QUEST_TRANSCRIPT_PROJECT_INVALID",
            "the exact current revision-3 project is not a valid Quest transcript basis",
        ),
        Revision3QuestTranscriptEditErrorV1::InvalidRequest(error) => map_request_error(error),
        Revision3QuestTranscriptEditErrorV1::InvalidEmbeddedDialogRequest(error) => Failure::new(
            "AUTHORING_REVISION3_QUEST_TRANSCRIPT_DIALOG_CONFLICT",
            format!("the embedded dialog-line request is invalid: {error}"),
        ),
        Revision3QuestTranscriptEditErrorV1::DialogLineTransaction(_) => Failure::new(
            "AUTHORING_REVISION3_QUEST_TRANSCRIPT_DIALOG_CONFLICT",
            "the embedded dialog-line transaction failed",
        ),
        Revision3QuestTranscriptEditErrorV1::ReopenCandidate(_)
        | Revision3QuestTranscriptEditErrorV1::CanonicalReopenMismatch
        | Revision3QuestTranscriptEditErrorV1::CandidatePreservationMismatch => Failure::new(
            "AUTHORING_REVISION3_QUEST_TRANSCRIPT_INVARIANT",
            "the Quest transcript candidate failed exact canonical closure",
        ),
    }
}

fn map_transaction_conflict(error: Revision3QuestTranscriptEditConflictV1) -> Failure {
    let code = match &error {
        Revision3QuestTranscriptEditConflictV1::CurrentHeadMismatch => {
            "AUTHORING_REVISION3_QUEST_TRANSCRIPT_HEAD_CONFLICT"
        }
        Revision3QuestTranscriptEditConflictV1::ProjectIdentityMismatch { .. }
        | Revision3QuestTranscriptEditConflictV1::ProjectRevisionConflict { .. } => {
            "AUTHORING_REVISION3_QUEST_TRANSCRIPT_PROJECT_CONFLICT"
        }
        Revision3QuestTranscriptEditConflictV1::ProjectTargetMismatch => {
            "AUTHORING_REVISION3_QUEST_TRANSCRIPT_TARGET_CONFLICT"
        }
        Revision3QuestTranscriptEditConflictV1::ZeroQuestId
        | Revision3QuestTranscriptEditConflictV1::InvalidQuestEntity { .. }
        | Revision3QuestTranscriptEditConflictV1::QuestRevisionConflict { .. } => {
            "AUTHORING_REVISION3_QUEST_TRANSCRIPT_QUEST_CONFLICT"
        }
        Revision3QuestTranscriptEditConflictV1::ProjectRevisionOverflow
        | Revision3QuestTranscriptEditConflictV1::QuestRevisionOverflow { .. } => {
            "AUTHORING_REVISION3_QUEST_TRANSCRIPT_REVISION_LIMIT"
        }
        Revision3QuestTranscriptEditConflictV1::NoChanges => {
            "AUTHORING_REVISION3_QUEST_TRANSCRIPT_NO_CHANGES"
        }
        Revision3QuestTranscriptEditConflictV1::CandidateTooLarge { .. } => {
            "AUTHORING_REVISION3_QUEST_TRANSCRIPT_PROJECT_LIMIT"
        }
        Revision3QuestTranscriptEditConflictV1::InvalidQuestClosure { .. }
        | Revision3QuestTranscriptEditConflictV1::OwnedModuleDrift { .. }
        | Revision3QuestTranscriptEditConflictV1::CandidateNotPersistable { .. } => {
            "AUTHORING_REVISION3_QUEST_TRANSCRIPT_PROJECT_INVALID"
        }
        Revision3QuestTranscriptEditConflictV1::DialogLineRejected { .. } => {
            "AUTHORING_REVISION3_QUEST_TRANSCRIPT_DIALOG_CONFLICT"
        }
        Revision3QuestTranscriptEditConflictV1::DialogRequestBasisMismatch => {
            "AUTHORING_REVISION3_QUEST_TRANSCRIPT_DIALOG_CONFLICT"
        }
        Revision3QuestTranscriptEditConflictV1::InsertIndexOutOfBounds { .. } => {
            "AUTHORING_REVISION3_QUEST_TRANSCRIPT_INDEX_CONFLICT"
        }
        Revision3QuestTranscriptEditConflictV1::TooManyBindings { .. }
        | Revision3QuestTranscriptEditConflictV1::InvalidLineReference { .. }
        | Revision3QuestTranscriptEditConflictV1::DuplicateLine { .. }
        | Revision3QuestTranscriptEditConflictV1::InactiveObjectiveSlot { .. } => {
            "AUTHORING_REVISION3_QUEST_TRANSCRIPT_BINDING_CONFLICT"
        }
    };
    Failure::new(code, error.to_string())
}

fn map_current_source_error(error: Revision3QuestCollisionSourceErrorV2) -> Failure {
    match error {
        Revision3QuestCollisionSourceErrorV2::Store(error) => map_store_error(error),
        Revision3QuestCollisionSourceErrorV2::CurrentSnapshotDrift => head_conflict(),
        Revision3QuestCollisionSourceErrorV2::Limit { .. }
        | Revision3QuestCollisionSourceErrorV2::TooManyPriorQuests { .. } => Failure::new(
            "AUTHORING_REVISION3_QUEST_TRANSCRIPT_STORE_LIMIT",
            "the exact-current Quest closure exceeds its bounded resource limit",
        ),
        Revision3QuestCollisionSourceErrorV2::InvalidCurrentProject { .. }
        | Revision3QuestCollisionSourceErrorV2::SharedQuestModule { .. }
        | Revision3QuestCollisionSourceErrorV2::ResidualQuestState { .. }
        | Revision3QuestCollisionSourceErrorV2::HistoricalQuestArtifactCrossReference { .. }
        | Revision3QuestCollisionSourceErrorV2::QuestOwnerDrift { .. }
        | Revision3QuestCollisionSourceErrorV2::ModuleOriginDrift { .. }
        | Revision3QuestCollisionSourceErrorV2::ForeignGenerator { .. }
        | Revision3QuestCollisionSourceErrorV2::SourceHashMismatch { .. }
        | Revision3QuestCollisionSourceErrorV2::InputFingerprintMismatch { .. }
        | Revision3QuestCollisionSourceErrorV2::PersistedModuleDrift { .. }
        | Revision3QuestCollisionSourceErrorV2::DuplicateRuntimeId { .. }
        | Revision3QuestCollisionSourceErrorV2::PriorIdentityCollision { .. }
        | Revision3QuestCollisionSourceErrorV2::NonQuestBasisInvalid { .. } => Failure::new(
            "AUTHORING_REVISION3_QUEST_TRANSCRIPT_PROJECT_INVALID",
            "the exact current revision-3 project is not a valid Quest transcript basis",
        ),
    }
}

fn map_store_error(error: WorkingStoreError) -> Failure {
    let code = match error {
        WorkingStoreError::InvalidLimits(_) => {
            "AUTHORING_REVISION3_QUEST_TRANSCRIPT_STORE_LIMITS_INVALID"
        }
        WorkingStoreError::MissingRoot(_) => {
            "AUTHORING_REVISION3_QUEST_TRANSCRIPT_STORE_ROOT_MISSING"
        }
        WorkingStoreError::UnsafePath { .. } => {
            "AUTHORING_REVISION3_QUEST_TRANSCRIPT_STORE_PATH_UNSAFE"
        }
        WorkingStoreError::LimitExceeded { .. } => {
            "AUTHORING_REVISION3_QUEST_TRANSCRIPT_STORE_LIMIT"
        }
        WorkingStoreError::HeadConflict { .. } => {
            "AUTHORING_REVISION3_QUEST_TRANSCRIPT_HEAD_CONFLICT"
        }
        WorkingStoreError::MissingHead(_) => "AUTHORING_REVISION3_QUEST_TRANSCRIPT_HEAD_MISSING",
        WorkingStoreError::MissingObject(_) => {
            "AUTHORING_REVISION3_QUEST_TRANSCRIPT_STORE_OBJECT_MISSING"
        }
        WorkingStoreError::SealMismatch { .. } => {
            "AUTHORING_REVISION3_QUEST_TRANSCRIPT_STORE_SEAL_MISMATCH"
        }
        WorkingStoreError::Collision { .. } => {
            "AUTHORING_REVISION3_QUEST_TRANSCRIPT_STORE_COLLISION"
        }
        WorkingStoreError::InvalidJson { .. } | WorkingStoreError::NonCanonicalJson { .. } => {
            "AUTHORING_REVISION3_QUEST_TRANSCRIPT_STORE_JSON_INVALID"
        }
        WorkingStoreError::Invariant(_)
        | WorkingStoreError::InvalidOgg(_)
        | WorkingStoreError::OggMetadataMismatch { .. } => {
            "AUTHORING_REVISION3_QUEST_TRANSCRIPT_STORE_INVARIANT"
        }
        WorkingStoreError::StagingCleanup { .. } | WorkingStoreError::Io(_) => {
            "AUTHORING_REVISION3_QUEST_TRANSCRIPT_STORE_IO"
        }
    };
    Failure::new(
        code,
        "the revision-3 Quest transcript Store operation failed",
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

    use gore_authoring::model_revision3::{DialogLine, LocalizationEntry};
    use gore_authoring::{
        regenerate_revision3_quest_module, AssetStoreIndex, ContentSeal, EntityId, FormatV2,
        GameGenerationAnchor, ProjectId, ProjectMeta, ProjectRevision3, QuestCollisionArtifactRef,
        QuestCollisionCatalogInput, Revision3DialogEmptyVoiceSlotIntentV1,
        Revision3DialogLineInsertRequestV1, Revision3DialogLocalizationIntentV1, Revision3Entity,
        Revision3EntityKind, Revision3OriginRef, Revision3QuestDraft, Revision3QuestDraftInput,
        Revision3QuestGiverInput, Revision3QuestParentInput, Revision3QuestTranscriptBindingV1,
        Revision3QuestTranscriptIntentV1, Revision3TypedRef, SchemaRevisionV3, Sha256Digest,
        QUEST_COLLISION_CATALOG_LAYER_V2, REVISION3_QUEST_GENERATOR_ID,
        REVISION3_QUEST_GENERATOR_VERSION,
    };
    use tempfile::TempDir;

    use super::*;

    const QUEST_ID: u8 = 0x41;
    const MODULE_ID: u8 = 0x42;
    const LINE_ID: u8 = 0x43;
    const LOCALIZATION_ID: u8 = 0x44;

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

    fn seal(value: u8, byte_len: u64) -> ContentSeal {
        ContentSeal {
            byte_len,
            sha256: Sha256Digest::from_bytes([value; 32]),
        }
    }

    fn target() -> GameGenerationAnchor {
        GameGenerationAnchor {
            executable: seal(0x10, 170_000_000),
        }
    }

    fn empty_project(revision: u64) -> ProjectRevision3 {
        ProjectRevision3 {
            format: FormatV2,
            schema_revision: SchemaRevisionV3,
            project_id: ProjectId::from_bytes([0x20; 16]),
            revision,
            meta: ProjectMeta {
                name: "Quest transcript FFI fixture".to_owned(),
                version: "1.0.0".to_owned(),
                author: "tests".to_owned(),
            },
            target: target(),
            authoring_locales: BTreeSet::new(),
            entities: BTreeMap::new(),
            asset_store: AssetStoreIndex::default(),
        }
    }

    fn published_store() -> PublishedStore {
        let temp = TempDir::new().unwrap();
        let store = WorkingProjectStore::at(temp.path(), ffi_store_limits()).unwrap();
        let base = empty_project(0);
        let base_checkpoint = store.prepare_revision3_checkpoint(None, &base).unwrap();
        fs::write(
            temp.path().join("gore-project.json"),
            &base_checkpoint.head_bytes,
        )
        .unwrap();

        let artifact_bytes = br#"{"format":"fixture-collision-v1"}"#;
        let imported = store
            .import_quest_collision_artifact_v2(artifact_bytes, &base_checkpoint.head)
            .unwrap();
        let mut project = empty_project(1);
        project
            .asset_store
            .assets
            .insert(imported.artifact.sha256, imported.asset_meta);
        let quest_id = entity_id(QUEST_ID);
        let module_id = entity_id(MODULE_ID);
        let quest = Revision3QuestDraft {
            generator_id: REVISION3_QUEST_GENERATOR_ID.to_owned(),
            generator_version: REVISION3_QUEST_GENERATOR_VERSION,
            input: Revision3QuestDraftInput {
                target: target(),
                quest_id,
                module_namespace: "GoreMods.Quests.TranscriptFixture".to_owned(),
                technical_id: "GORE_TRANSCRIPT_FIXTURE".to_owned(),
                text_helper: "GoreTranscriptFixtureText".to_owned(),
                parent_quest: Revision3QuestParentInput {
                    generation: target(),
                    source_seal: seal(0x34, 1234),
                    catalog_layer: "base-game.g1r.quests".to_owned(),
                    canonical_selector: "CatalogQuest_Parent".to_owned(),
                    runtime_class: "UQuest_Parent".to_owned(),
                },
                giver: Revision3QuestGiverInput {
                    generation: target(),
                    source_seal: seal(0x35, 2345),
                    catalog_layer: "base-game.g1r.characters".to_owned(),
                    canonical_selector: "CatalogCharacter_Asghan".to_owned(),
                    runtime_unique_name: "OM_GRD_Asghan_263".to_owned(),
                },
                title: "Transcript fixture".to_owned(),
                description: "Authoring metadata only".to_owned(),
                objective_title: "Talk".to_owned(),
                additional_objective_titles: Vec::new(),
                transition_plan: Box::new(
                    gore_authoring::QuestTransitionPlanV1::default_for_objectives(1).unwrap(),
                ),
                collision_catalog: QuestCollisionArtifactRef {
                    generation: target(),
                    catalog_layer: QUEST_COLLISION_CATALOG_LAYER_V2.to_owned(),
                    artifact: imported.artifact,
                    source_seal: seal(0x33, artifact_bytes.len() as u64),
                    basis_snapshot: base_checkpoint.head.snapshot.clone(),
                },
            },
            script_module: Revision3TypedRef::new(
                project.project_id,
                module_id,
                Revision3EntityKind::ScriptModule,
            ),
            transcript: Vec::new(),
        };
        let collision = QuestCollisionCatalogInput {
            generation: quest.input.collision_catalog.generation.clone(),
            source_seal: quest.input.collision_catalog.source_seal.clone(),
            catalog_layer: quest.input.collision_catalog.catalog_layer.clone(),
            modules: BTreeSet::new(),
            relative_paths: BTreeSet::new(),
            symbols: BTreeSet::new(),
        };
        let module = regenerate_revision3_quest_module(&quest, collision).unwrap();
        project.entities.insert(
            quest_id,
            Revision3Entity {
                id: quest_id,
                display_name: "Transcript fixture Quest".to_owned(),
                origin: Revision3OriginRef::New {
                    authored_runtime_id: quest.input.technical_id.clone(),
                },
                revision: 3,
                payload: Revision3EntityPayload::QuestDraft(quest),
            },
        );
        project.entities.insert(
            module_id,
            Revision3Entity {
                id: module_id,
                display_name: "Transcript fixture module".to_owned(),
                origin: Revision3OriginRef::Generated {
                    generator_id: REVISION3_QUEST_GENERATOR_ID.to_owned(),
                    generator_version: REVISION3_QUEST_GENERATOR_VERSION,
                    owner: Revision3TypedRef::new(
                        project.project_id,
                        quest_id,
                        Revision3EntityKind::QuestDraft,
                    ),
                },
                revision: 5,
                payload: Revision3EntityPayload::ScriptModule(module),
            },
        );
        project.entities.insert(
            entity_id(LOCALIZATION_ID),
            Revision3Entity {
                id: entity_id(LOCALIZATION_ID),
                display_name: "Existing transcript localization".to_owned(),
                origin: Revision3OriginRef::New {
                    authored_runtime_id: "GORE_TRANSCRIPT_EXISTING_TEXT_ENTITY".to_owned(),
                },
                revision: 2,
                payload: Revision3EntityPayload::LocalizationEntry(LocalizationEntry {
                    loc_id: "GORE_TRANSCRIPT_EXISTING_TEXT".to_owned(),
                    texts: BTreeMap::new(),
                }),
            },
        );
        project.entities.insert(
            entity_id(LINE_ID),
            Revision3Entity {
                id: entity_id(LINE_ID),
                display_name: "Existing transcript line".to_owned(),
                origin: Revision3OriginRef::New {
                    authored_runtime_id: "GORE_TRANSCRIPT_EXISTING_LINE".to_owned(),
                },
                revision: 4,
                payload: Revision3EntityPayload::DialogLine(DialogLine {
                    localization: Revision3TypedRef::new(
                        project.project_id,
                        entity_id(LOCALIZATION_ID),
                        Revision3EntityKind::LocalizationEntry,
                    ),
                    speaker_hint: Some("Asghan".to_owned()),
                    voice_slots: BTreeMap::new(),
                }),
            },
        );
        let project_json = project.to_canonical_json().unwrap();
        let published = store
            .prepare_revision3_checkpoint(Some(&base_checkpoint.head), &project)
            .unwrap();
        fs::write(temp.path().join("gore-project.json"), &published.head_bytes).unwrap();
        PublishedStore {
            temp,
            project,
            project_json,
            head: published.head,
            fixed_head_bytes: published.head_bytes,
        }
    }

    fn request(
        store: &PublishedStore,
        intent: Revision3QuestTranscriptIntentV1,
    ) -> Revision3QuestTranscriptEditRequestV1 {
        Revision3QuestTranscriptEditRequestV1 {
            expected_head: store.head.clone(),
            expected_project_id: store.project.project_id,
            expected_revision: store.project.revision,
            expected_target: store.project.target.clone(),
            quest_id: entity_id(QUEST_ID),
            expected_quest_revision: store.project.entities[&entity_id(QUEST_ID)].revision,
            intent,
        }
    }

    fn replace_request(store: &PublishedStore) -> Revision3QuestTranscriptEditRequestV1 {
        request(
            store,
            Revision3QuestTranscriptIntentV1::Replace {
                bindings: vec![Revision3QuestTranscriptBindingV1 {
                    line: Revision3TypedRef::new(
                        store.project.project_id,
                        entity_id(LINE_ID),
                        Revision3EntityKind::DialogLine,
                    ),
                    objective_slot: None,
                }],
            },
        )
    }

    fn create_request(store: &PublishedStore) -> Revision3QuestTranscriptEditRequestV1 {
        request(
            store,
            Revision3QuestTranscriptIntentV1::CreateAndInsert {
                index: 0,
                objective_slot: None,
                line: Revision3DialogLineInsertRequestV1 {
                    expected_head: store.head.clone(),
                    expected_project_id: store.project.project_id,
                    expected_revision: store.project.revision,
                    expected_target: store.project.target.clone(),
                    line_id: entity_id(0x51),
                    line_display_name: "Created transcript line".to_owned(),
                    line_authored_identity: "GORE_TRANSCRIPT_CREATED_LINE".to_owned(),
                    speaker_hint: Some("Asghan".to_owned()),
                    localization: Revision3DialogLocalizationIntentV1::Create {
                        localization_id: entity_id(0x52),
                        display_name: "Created transcript text".to_owned(),
                        loc_id: "GORE_TRANSCRIPT_CREATED_TEXT".to_owned(),
                        texts: BTreeMap::from([(
                            "de".parse().unwrap(),
                            "Eine neue Quest-Zeile.".to_owned(),
                        )]),
                    },
                    voice_slot: Some(Revision3DialogEmptyVoiceSlotIntentV1 {
                        slot_id: entity_id(0x53),
                        locale: "de".parse().unwrap(),
                        display_name: "Created transcript German voice".to_owned(),
                    }),
                },
            },
        )
    }

    fn raw_request(
        store: &PublishedStore,
        request: &Revision3QuestTranscriptEditRequestV1,
    ) -> String {
        serde_json::to_string(&ExactWireRequest {
            command: COMMAND.to_owned(),
            payload: PrepareQuestTranscriptWirePayload {
                current_project_json: store.project_json.clone(),
                quest_transcript_request_json: request.to_canonical_json().unwrap(),
                root: store.temp.path().to_string_lossy().into_owned(),
            },
        })
        .unwrap()
    }

    fn fixed_head(store: &PublishedStore) -> Vec<u8> {
        fs::read(store.temp.path().join("gore-project.json")).unwrap()
    }

    fn reopened_candidate(store: &PublishedStore, response: &Value) -> ProjectRevision3 {
        let opened = WorkingProjectStore::open_existing(store.temp.path(), ffi_store_limits())
            .unwrap()
            .open_revision3_head_bytes(
                response["head_json"].as_str().unwrap().as_bytes(),
                AssetVerification::Full,
            )
            .unwrap();
        assert_eq!(
            opened.project.to_canonical_json().unwrap(),
            response["project_json"].as_str().unwrap()
        );
        opened.project
    }

    fn wire() -> String {
        serde_json::to_string(&ExactWireRequest {
            command: COMMAND.to_owned(),
            payload: PrepareQuestTranscriptWirePayload {
                current_project_json: "{}".to_owned(),
                quest_transcript_request_json: "{}".to_owned(),
                root: "C:/store".to_owned(),
            },
        })
        .unwrap()
    }

    #[test]
    fn outer_wire_is_exact_closed_and_has_no_game_root() {
        let canonical = wire();
        let payload: PrepareQuestTranscriptWirePayload = parse_exact_wire(&canonical).unwrap();
        assert_eq!(payload.root, "C:/store");
        assert_eq!(
            parse_exact_wire::<PrepareQuestTranscriptWirePayload>(&(canonical + "\n"))
                .unwrap_err()
                .code,
            "AUTHORING_REVISION3_QUEST_TRANSCRIPT_REQUEST_INVALID"
        );
        let unknown = format!(
            "{{\"command\":\"{COMMAND}\",\"payload\":{{\"current_project_json\":\"{{}}\",\"quest_transcript_request_json\":\"{{}}\",\"root\":\"C:/store\",\"game_root\":\"C:/game\"}}}}"
        );
        assert_eq!(
            parse_exact_wire::<PrepareQuestTranscriptWirePayload>(&unknown)
                .unwrap_err()
                .code,
            "AUTHORING_REVISION3_QUEST_TRANSCRIPT_REQUEST_INVALID"
        );
        let duplicate = format!(
            "{{\"command\":\"{COMMAND}\",\"payload\":{{\"current_project_json\":\"{{}}\",\"current_project_json\":\"{{}}\",\"quest_transcript_request_json\":\"{{}}\",\"root\":\"C:/store\"}}}}"
        );
        assert_eq!(
            parse_exact_wire::<PrepareQuestTranscriptWirePayload>(&duplicate)
                .unwrap_err()
                .code,
            "AUTHORING_REVISION3_QUEST_TRANSCRIPT_REQUEST_INVALID"
        );
    }

    #[test]
    fn final_fixed_basis_guard_maps_external_change_to_head_conflict() {
        // Compile-time exercise of the race seam; Store-backed behavior is shared with the
        // fully tested prepare-only Quest routes.
        let _ = prepare_revision3_quest_transcript_v1_inner_with_final_guard::<fn()>
            as fn(&str, fn()) -> Result<Value, Failure>;
        assert_eq!(
            head_conflict().code,
            "AUTHORING_REVISION3_QUEST_TRANSCRIPT_HEAD_CONFLICT"
        );
    }

    #[test]
    fn replace_prepares_fully_reopened_candidate_without_publishing_fixed_head() {
        let store = published_store();
        let basis_module = store.project.entities[&entity_id(MODULE_ID)].clone();
        let response = prepare_revision3_quest_transcript_v1_raw(&raw_request(
            &store,
            &replace_request(&store),
        ));
        assert_eq!(response["ok"], true);
        let actual_keys = response
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        let expected_keys = BTreeSet::from([
            "basis_head_json",
            "build_status",
            "created_line_id",
            "created_localization_id",
            "created_voice_slot_id",
            "head_json",
            "localization_action",
            "mode",
            "module_id",
            "module_revision",
            "ok",
            "outcome",
            "project_id",
            "project_json",
            "publication_status",
            "quest_id",
            "quest_revision",
            "revision",
            "runtime_status",
            "topic_authority",
            "transcript_count",
        ]);
        assert_eq!(actual_keys, expected_keys);
        assert_eq!(response["mode"], "replace");
        assert_eq!(response["revision"], store.project.revision + 1);
        assert_eq!(
            response["quest_revision"],
            store.project.entities[&entity_id(QUEST_ID)].revision + 1
        );
        assert_eq!(
            response["module_revision"],
            store.project.entities[&entity_id(MODULE_ID)].revision
        );
        assert_eq!(response["transcript_count"], 1);
        assert!(response["created_line_id"].is_null());
        assert!(response["created_localization_id"].is_null());
        assert!(response["created_voice_slot_id"].is_null());
        assert!(response["localization_action"].is_null());
        assert_eq!(fixed_head(&store), store.fixed_head_bytes);

        let candidate = reopened_candidate(&store, &response);
        let Revision3EntityPayload::QuestDraft(quest) =
            &candidate.entities[&entity_id(QUEST_ID)].payload
        else {
            panic!("candidate Quest kind")
        };
        assert_eq!(quest.transcript.len(), 1);
        assert_eq!(quest.transcript[0].line.id, entity_id(LINE_ID));
        assert_eq!(candidate.entities[&entity_id(MODULE_ID)], basis_module);
        assert_eq!(fixed_head(&store), store.fixed_head_bytes);
    }

    #[test]
    fn create_and_insert_is_one_revision_and_reopens_all_created_entities() {
        let store = published_store();
        let basis_module = store.project.entities[&entity_id(MODULE_ID)].clone();
        let response = prepare_revision3_quest_transcript_v1_raw(&raw_request(
            &store,
            &create_request(&store),
        ));
        assert_eq!(response["ok"], true);
        assert_eq!(response["mode"], "create_and_insert");
        assert_eq!(response["revision"], store.project.revision + 1);
        assert_eq!(
            response["quest_revision"],
            store.project.entities[&entity_id(QUEST_ID)].revision + 1
        );
        assert_eq!(
            response["module_revision"],
            store.project.entities[&entity_id(MODULE_ID)].revision
        );
        assert_eq!(response["transcript_count"], 1);
        assert_eq!(response["created_line_id"], entity_id(0x51).to_string());
        assert_eq!(
            response["created_localization_id"],
            entity_id(0x52).to_string()
        );
        assert_eq!(
            response["created_voice_slot_id"],
            entity_id(0x53).to_string()
        );
        assert_eq!(response["localization_action"], "created");
        assert_eq!(fixed_head(&store), store.fixed_head_bytes);

        let candidate = reopened_candidate(&store, &response);
        let Revision3EntityPayload::QuestDraft(quest) =
            &candidate.entities[&entity_id(QUEST_ID)].payload
        else {
            panic!("candidate Quest kind")
        };
        assert_eq!(quest.transcript.len(), 1);
        assert_eq!(quest.transcript[0].line.id, entity_id(0x51));
        assert!(candidate.entities.contains_key(&entity_id(0x51)));
        assert!(candidate.entities.contains_key(&entity_id(0x52)));
        assert!(candidate.entities.contains_key(&entity_id(0x53)));
        assert_eq!(candidate.entities[&entity_id(MODULE_ID)], basis_module);
        assert_eq!(candidate.entities.len(), store.project.entities.len() + 3);
    }

    #[test]
    fn semantic_rejection_keeps_fixed_basis_exact_and_codes_match_dart_contract() {
        let store = published_store();
        let mut invalid = replace_request(&store);
        let Revision3QuestTranscriptIntentV1::Replace { bindings } = &mut invalid.intent else {
            unreachable!()
        };
        bindings.push(bindings[0].clone());
        let response = prepare_revision3_quest_transcript_v1_raw(&raw_request(&store, &invalid));
        assert_eq!(response["ok"], false);
        assert_eq!(
            response["error"]["code"],
            "AUTHORING_REVISION3_QUEST_TRANSCRIPT_BINDING_CONFLICT"
        );
        assert_eq!(fixed_head(&store), store.fixed_head_bytes);
        let current = WorkingProjectStore::open_existing(store.temp.path(), ffi_store_limits())
            .unwrap()
            .open_current_revision3(AssetVerification::Full)
            .unwrap();
        assert_eq!(current.head, store.head);
        assert_eq!(current.project, store.project);

        assert_eq!(
            map_transaction_conflict(
                Revision3QuestTranscriptEditConflictV1::InsertIndexOutOfBounds { index: 2, len: 1 }
            )
            .code,
            "AUTHORING_REVISION3_QUEST_TRANSCRIPT_INDEX_CONFLICT"
        );
        assert_eq!(
            map_transaction_conflict(
                Revision3QuestTranscriptEditConflictV1::DialogRequestBasisMismatch
            )
            .code,
            "AUTHORING_REVISION3_QUEST_TRANSCRIPT_DIALOG_CONFLICT"
        );
        assert_eq!(
            map_transaction_conflict(Revision3QuestTranscriptEditConflictV1::ZeroQuestId).code,
            "AUTHORING_REVISION3_QUEST_TRANSCRIPT_QUEST_CONFLICT"
        );
    }

    #[test]
    fn final_guard_rejects_fixed_head_race_after_candidate_preparation() {
        let store = published_store();
        let input = raw_request(&store, &replace_request(&store));
        let mut rival = store.project.clone();
        rival.revision += 1;
        rival.meta.name = "Concurrent publisher".to_owned();
        let result = prepare_revision3_quest_transcript_v1_inner_with_final_guard(&input, || {
            let working =
                WorkingProjectStore::open_existing(store.temp.path(), ffi_store_limits()).unwrap();
            let checkpoint = working
                .prepare_revision3_checkpoint(Some(&store.head), &rival)
                .unwrap();
            fs::write(
                store.temp.path().join("gore-project.json"),
                checkpoint.head_bytes,
            )
            .unwrap();
        });
        assert_eq!(
            result.unwrap_err().code,
            "AUTHORING_REVISION3_QUEST_TRANSCRIPT_HEAD_CONFLICT"
        );
    }

    #[test]
    fn malformed_semantic_request_is_rejected_before_store_open() {
        let input = serde_json::to_string(&ExactWireRequest {
            command: COMMAND.to_owned(),
            payload: PrepareQuestTranscriptWirePayload {
                current_project_json: "{}".to_owned(),
                quest_transcript_request_json: "{}".to_owned(),
                root: "C:/definitely/missing/quest-transcript-store".to_owned(),
            },
        })
        .unwrap();
        assert_eq!(
            prepare_revision3_quest_transcript_v1_inner(&input)
                .unwrap_err()
                .code,
            "AUTHORING_REVISION3_QUEST_TRANSCRIPT_REQUEST_INVALID"
        );
    }
}
