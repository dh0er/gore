//! Native prepare-only orchestration for one exact revision-3 NPC greeting edit.
//!
//! The route accepts only canonical project/request transports and an existing Store root. It
//! fully reopens and preflights the fixed NPC basis, prepares an immutable candidate, fully
//! reopens that candidate, and rechecks the fixed basis before success. It has no game, save,
//! source publication, build, deployment, runtime, topic-registration, or fixed-head authority.

use std::path::Path;

use gore_authoring::{
    apply_revision3_npc_greeting_edit_transaction_v1, AssetVerification, Revision3EntityPayload,
    Revision3NpcGreetingBuildStatusV1, Revision3NpcGreetingEditConflictV1,
    Revision3NpcGreetingEditErrorV1, Revision3NpcGreetingEditEvaluationV1,
    Revision3NpcGreetingEditOutcomeV1, Revision3NpcGreetingEditRequestV1,
    Revision3NpcGreetingModeV1, Revision3NpcGreetingPublicationStatusV1,
    Revision3NpcGreetingRuntimeStatusV1, Revision3NpcGreetingTopicAuthorityV1, WorkingHead,
    WorkingProjectStore, WorkingStoreError, WorkingStoreLimits, MAX_PROJECT_JSON_BYTES,
    MAX_REVISION3_NPC_GREETING_REQUEST_JSON_BYTES_V1,
};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::err;

pub(super) const COMMAND: &str = "authoring_store_prepare_revision3_npc_greeting_v1";

const MAX_PATH_BYTES: usize = 32 * 1024;
const MAX_HEAD_JSON_BYTES: usize = 64 * 1024;
const MAX_RESPONSE_BYTES: usize = 64 * 1024 * 1024;
const MAX_ERROR_MESSAGE_BYTES: usize = 4 * 1024;
const MAX_BASIS_REVISION: u64 = i64::MAX as u64 - 1;
const MAX_WIRE_BYTES: usize = MAX_PROJECT_JSON_BYTES * 2
    + MAX_REVISION3_NPC_GREETING_REQUEST_JSON_BYTES_V1 * 2
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
struct PrepareNpcGreetingWirePayload {
    current_project_json: String,
    npc_greeting_request_json: String,
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

pub(super) fn prepare_revision3_npc_greeting_v1_raw(input: &str) -> Value {
    prepare_revision3_npc_greeting_v1_inner(input).unwrap_or_else(Failure::response)
}

fn prepare_revision3_npc_greeting_v1_inner(input: &str) -> Result<Value, Failure> {
    prepare_revision3_npc_greeting_v1_inner_with_test_seams(input, || {}, || {})
}

#[cfg(test)]
fn prepare_revision3_npc_greeting_v1_inner_with_final_guard<F>(
    input: &str,
    final_guard: F,
) -> Result<Value, Failure>
where
    F: FnOnce(),
{
    prepare_revision3_npc_greeting_v1_inner_with_test_seams(input, || {}, final_guard)
}

fn prepare_revision3_npc_greeting_v1_inner_with_test_seams<B, F>(
    input: &str,
    before_checkpoint: B,
    final_guard: F,
) -> Result<Value, Failure>
where
    B: FnOnce(),
    F: FnOnce(),
{
    let payload: PrepareNpcGreetingWirePayload = parse_exact_wire(input)?;
    validate_payload(&payload)?;
    // Reject the bounded semantic request before any potentially expensive Store reopen.
    let request = Revision3NpcGreetingEditRequestV1::from_json(&payload.npc_greeting_request_json)
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
            "AUTHORING_REVISION3_NPC_GREETING_STORE_INVARIANT",
            "the exact current revision-3 project could not be serialized canonically",
        )
    })?;
    if canonical_basis.as_bytes() != payload.current_project_json.as_bytes() {
        return Err(Failure::new(
            "AUTHORING_REVISION3_NPC_GREETING_PROJECT_CONFLICT",
            "current_project_json differs from the exact published revision-3 project",
        ));
    }
    bind_request_to_basis(&basis.head, &basis.project, &request)?;

    let outcome = match apply_revision3_npc_greeting_edit_transaction_v1(
        &basis.head,
        &canonical_basis,
        &payload.npc_greeting_request_json,
    )
    .map_err(map_transaction_error)?
    {
        Revision3NpcGreetingEditEvaluationV1::Applied(outcome) => *outcome,
        Revision3NpcGreetingEditEvaluationV1::Rejected(rejection) => {
            return Err(map_transaction_conflict(rejection.conflict));
        }
    };
    require_signed_serializable(&outcome.project)?;
    verify_outcome_binding(&basis.head, &basis.project, &request, &outcome)?;
    match outcome.build_status {
        Revision3NpcGreetingBuildStatusV1::Blocked => {}
    }
    match outcome.runtime_status {
        Revision3NpcGreetingRuntimeStatusV1::RuntimeUnqualified => {}
    }
    match outcome.topic_authority {
        Revision3NpcGreetingTopicAuthorityV1::NotGranted => {}
    }
    match outcome.publication_status {
        Revision3NpcGreetingPublicationStatusV1::NotSupported => {}
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
            "AUTHORING_REVISION3_NPC_GREETING_STORE_INVARIANT",
            "the prepared NPC greeting checkpoint did not fully reopen exactly",
        ));
    }
    let reopened_json = reopened.project.to_canonical_json().map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_NPC_GREETING_STORE_INVARIANT",
            "the fully reopened NPC greeting candidate could not be serialized",
        )
    })?;
    if reopened_json != outcome.canonical_project_json {
        return Err(Failure::new(
            "AUTHORING_REVISION3_NPC_GREETING_STORE_INVARIANT",
            "the fully reopened NPC greeting candidate changed canonical bytes",
        ));
    }

    final_guard();
    require_fixed_basis(&store, &basis.head, &basis.project)?;

    let basis_head_json = canonical_head_json(&basis.head)?;
    let candidate_head_json = String::from_utf8(prepared.head_bytes).map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_NPC_GREETING_STORE_INVARIANT",
            "the prepared NPC greeting head is not UTF-8 JSON",
        )
    })?;
    if candidate_head_json.is_empty() || candidate_head_json.len() > MAX_HEAD_JSON_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_NPC_GREETING_RESPONSE_LIMIT",
            "the prepared NPC greeting head exceeds its bounded transport limit",
        ));
    }
    require_signed_serializable(&prepared.head)?;

    let mode = match outcome.mode {
        Revision3NpcGreetingModeV1::Replace => "replace",
        Revision3NpcGreetingModeV1::CreateAndInsert => "create_and_insert",
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
        "npc_id": outcome.npc_id.to_string(),
        "npc_revision": outcome.npc_revision,
        "module_id": outcome.script_module_id.to_string(),
        "module_revision": outcome.script_module_revision,
        "mode": mode,
        "greeting_count": outcome.greeting_count,
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
    request: &Revision3NpcGreetingEditRequestV1,
    outcome: &Revision3NpcGreetingEditOutcomeV1,
) -> Result<(), Failure> {
    let basis_npc_entity = basis.entities.get(&request.npc_id).ok_or_else(|| {
        Failure::new(
            "AUTHORING_REVISION3_NPC_GREETING_INVARIANT",
            "the bound NPC disappeared during transaction verification",
        )
    })?;
    let Revision3EntityPayload::NpcDraft(basis_npc) = &basis_npc_entity.payload else {
        return Err(Failure::new(
            "AUTHORING_REVISION3_NPC_GREETING_INVARIANT",
            "the bound NPC changed kind during transaction verification",
        ));
    };
    let basis_module = basis
        .entities
        .get(&basis_npc.script_module.id)
        .ok_or_else(|| {
            Failure::new(
                "AUTHORING_REVISION3_NPC_GREETING_INVARIANT",
                "the bound NPC module disappeared during transaction verification",
            )
        })?;
    let outcome_npc_entity = outcome
        .project
        .entities
        .get(&request.npc_id)
        .ok_or_else(|| {
            Failure::new(
                "AUTHORING_REVISION3_NPC_GREETING_INVARIANT",
                "the edited NPC disappeared during transaction verification",
            )
        })?;
    let Revision3EntityPayload::NpcDraft(outcome_npc) = &outcome_npc_entity.payload else {
        return Err(Failure::new(
            "AUTHORING_REVISION3_NPC_GREETING_INVARIANT",
            "the edited NPC changed kind during transaction verification",
        ));
    };
    if outcome.basis_head != *basis_head
        || outcome.npc_id != request.npc_id
        || outcome.project.project_id != basis.project_id
        || outcome.project.target != basis.target
        || outcome.project.revision != basis.revision + 1
        || outcome.npc_revision != basis_npc_entity.revision + 1
        || outcome_npc_entity.revision != outcome.npc_revision
        || outcome.script_module_id != basis_npc.script_module.id
        || outcome.script_module_revision != basis_module.revision
        || outcome.mode != request.intent.mode()
        || outcome.greeting_count != outcome_npc.greetings.len() as u64
        || outcome.project.entities.get(&basis_npc.script_module.id) != Some(basis_module)
    {
        return Err(Failure::new(
            "AUTHORING_REVISION3_NPC_GREETING_INVARIANT",
            "the NPC greeting transaction changed its exact project/entity binding",
        ));
    }
    match (&request.intent, &outcome.created) {
        (gore_authoring::Revision3NpcGreetingIntentV1::Replace { .. }, None) => {}
        (
            gore_authoring::Revision3NpcGreetingIntentV1::CreateAndInsert { line, .. },
            Some(created),
        ) if created.line_id == line.line_id
            && created.localization_id == line.localization.localization_id()
            && created.voice_slot_id == line.voice_slot.as_ref().map(|slot| slot.slot_id) => {}
        _ => {
            return Err(Failure::new(
                "AUTHORING_REVISION3_NPC_GREETING_INVARIANT",
                "the NPC greeting transaction returned inconsistent creation metadata",
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
            "AUTHORING_REVISION3_NPC_GREETING_INPUT_LIMIT",
            format!("NPC greeting request exceeds the {MAX_WIRE_BYTES}-byte wire limit"),
        ));
    }
    let request: ExactWireRequest<P> =
        serde_json::from_str(input).map_err(|_| invalid_request())?;
    if request.command != COMMAND {
        return Err(invalid_request());
    }
    let canonical = serde_json::to_string(&request).map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_NPC_GREETING_INVARIANT",
            "the NPC greeting outer request could not be serialized",
        )
    })?;
    if canonical != input {
        return Err(invalid_request());
    }
    Ok(request.payload)
}

fn validate_payload(payload: &PrepareNpcGreetingWirePayload) -> Result<(), Failure> {
    if payload.root.is_empty()
        || payload.root.len() > MAX_PATH_BYTES
        || payload.root.contains('\0')
        || payload.current_project_json.is_empty()
        || payload.npc_greeting_request_json.is_empty()
    {
        return Err(invalid_request());
    }
    if payload.current_project_json.len() > MAX_PROJECT_JSON_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_NPC_GREETING_PROJECT_LIMIT",
            format!("current_project_json exceeds the {MAX_PROJECT_JSON_BYTES}-byte limit"),
        ));
    }
    if payload.npc_greeting_request_json.len() > MAX_REVISION3_NPC_GREETING_REQUEST_JSON_BYTES_V1 {
        return Err(Failure::new(
            "AUTHORING_REVISION3_NPC_GREETING_REQUEST_LIMIT",
            format!(
                "npc_greeting_request_json exceeds the {MAX_REVISION3_NPC_GREETING_REQUEST_JSON_BYTES_V1}-byte limit"
            ),
        ));
    }
    Ok(())
}

fn bind_request_to_basis(
    head: &WorkingHead,
    project: &gore_authoring::ProjectRevision3,
    request: &Revision3NpcGreetingEditRequestV1,
) -> Result<(), Failure> {
    if request.expected_head != *head {
        return Err(head_conflict());
    }
    if request.expected_project_id != project.project_id
        || request.expected_revision != project.revision
    {
        return Err(Failure::new(
            "AUTHORING_REVISION3_NPC_GREETING_PROJECT_CONFLICT",
            "the NPC greeting request project differs from the exact published project",
        ));
    }
    if request.expected_target != project.target {
        return Err(Failure::new(
            "AUTHORING_REVISION3_NPC_GREETING_TARGET_CONFLICT",
            "the NPC greeting request target differs from the exact published target",
        ));
    }
    let entity = project.entities.get(&request.npc_id).ok_or_else(|| {
        npc_conflict("the requested NPC does not exist in the exact published project")
    })?;
    let Revision3EntityPayload::NpcDraft(npc) = &entity.payload else {
        return Err(npc_conflict(
            "the requested entity is not an NPC in the exact published project",
        ));
    };
    if entity.revision != request.expected_npc_revision {
        return Err(npc_conflict(
            "the requested NPC revision differs from the exact published entity revision",
        ));
    }
    if npc.script_module.project_id != project.project_id {
        return Err(Failure::new(
            "AUTHORING_REVISION3_NPC_GREETING_PROJECT_INVALID",
            "the exact NPC has a foreign owned module binding",
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
            "AUTHORING_REVISION3_NPC_GREETING_REVISION_LIMIT",
            format!("the published basis revision exceeds {MAX_BASIS_REVISION}"),
        ));
    }
    Ok(())
}

fn require_signed_serializable(value: &impl Serialize) -> Result<(), Failure> {
    let value = serde_json::to_value(value).map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_NPC_GREETING_INVARIANT",
            "an NPC greeting wire value could not be inspected",
        )
    })?;
    require_signed_json_value(&value)
}

fn require_signed_json_value(value: &Value) -> Result<(), Failure> {
    match value {
        Value::Number(number) if number.as_u64().is_some_and(|value| value > i64::MAX as u64) => {
            Err(Failure::new(
                "AUTHORING_REVISION3_NPC_GREETING_SIGNED_WIRE_LIMIT",
                "an NPC greeting wire integer exceeds signed 64-bit transport",
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
            "AUTHORING_REVISION3_NPC_GREETING_INVARIANT",
            "the NPC greeting basis head could not be serialized",
        )
    })?;
    if value.is_empty() || value.len() > MAX_HEAD_JSON_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_NPC_GREETING_RESPONSE_LIMIT",
            "the NPC greeting basis head exceeds its bounded transport limit",
        ));
    }
    Ok(value)
}

fn enforce_response_budget(response: &Value) -> Result<(), Failure> {
    let encoded = serde_json::to_vec(response).map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_NPC_GREETING_INVARIANT",
            "the NPC greeting response could not be serialized",
        )
    })?;
    if encoded.len() > MAX_RESPONSE_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_NPC_GREETING_RESPONSE_LIMIT",
            "the NPC greeting response exceeds its bounded transport budget",
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
        "AUTHORING_REVISION3_NPC_GREETING_REQUEST_INVALID",
        "request must contain one exact duplicate-free command and exactly current_project_json, npc_greeting_request_json, and root",
    )
}

fn head_conflict() -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_NPC_GREETING_HEAD_CONFLICT",
        "the published revision-3 head changed or differs from the NPC greeting request",
    )
}

fn npc_conflict(message: &'static str) -> Failure {
    Failure::new("AUTHORING_REVISION3_NPC_GREETING_NPC_CONFLICT", message)
}

fn map_request_error(error: impl std::fmt::Display) -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_NPC_GREETING_REQUEST_INVALID",
        format!("the exact NPC greeting request is invalid: {error}"),
    )
}

fn map_transaction_error(error: Revision3NpcGreetingEditErrorV1) -> Failure {
    match error {
        Revision3NpcGreetingEditErrorV1::InvalidProject(_) => Failure::new(
            "AUTHORING_REVISION3_NPC_GREETING_PROJECT_INVALID",
            "the exact current revision-3 project is not a valid NPC greeting basis",
        ),
        Revision3NpcGreetingEditErrorV1::InvalidRequest(error) => map_request_error(error),
        Revision3NpcGreetingEditErrorV1::InvalidEmbeddedDialogRequest(error) => Failure::new(
            "AUTHORING_REVISION3_NPC_GREETING_DIALOG_CONFLICT",
            format!("the embedded dialog-line request is invalid: {error}"),
        ),
        Revision3NpcGreetingEditErrorV1::DialogLineTransaction(_) => Failure::new(
            "AUTHORING_REVISION3_NPC_GREETING_DIALOG_CONFLICT",
            "the embedded dialog-line transaction failed",
        ),
        Revision3NpcGreetingEditErrorV1::ReopenCandidate(_)
        | Revision3NpcGreetingEditErrorV1::CanonicalReopenMismatch
        | Revision3NpcGreetingEditErrorV1::CandidatePreservationMismatch => Failure::new(
            "AUTHORING_REVISION3_NPC_GREETING_INVARIANT",
            "the NPC greeting candidate failed exact canonical closure",
        ),
    }
}

fn map_transaction_conflict(error: Revision3NpcGreetingEditConflictV1) -> Failure {
    let code = match &error {
        Revision3NpcGreetingEditConflictV1::CurrentHeadMismatch => {
            "AUTHORING_REVISION3_NPC_GREETING_HEAD_CONFLICT"
        }
        Revision3NpcGreetingEditConflictV1::ProjectIdentityMismatch { .. }
        | Revision3NpcGreetingEditConflictV1::ProjectRevisionConflict { .. } => {
            "AUTHORING_REVISION3_NPC_GREETING_PROJECT_CONFLICT"
        }
        Revision3NpcGreetingEditConflictV1::ProjectTargetMismatch => {
            "AUTHORING_REVISION3_NPC_GREETING_TARGET_CONFLICT"
        }
        Revision3NpcGreetingEditConflictV1::ZeroNpcId
        | Revision3NpcGreetingEditConflictV1::InvalidNpcEntity { .. }
        | Revision3NpcGreetingEditConflictV1::NpcRevisionConflict { .. } => {
            "AUTHORING_REVISION3_NPC_GREETING_NPC_CONFLICT"
        }
        Revision3NpcGreetingEditConflictV1::ProjectRevisionOverflow
        | Revision3NpcGreetingEditConflictV1::NpcRevisionOverflow { .. } => {
            "AUTHORING_REVISION3_NPC_GREETING_REVISION_LIMIT"
        }
        Revision3NpcGreetingEditConflictV1::NoChanges => {
            "AUTHORING_REVISION3_NPC_GREETING_NO_CHANGES"
        }
        Revision3NpcGreetingEditConflictV1::CandidateTooLarge { .. } => {
            "AUTHORING_REVISION3_NPC_GREETING_PROJECT_LIMIT"
        }
        Revision3NpcGreetingEditConflictV1::InvalidNpcClosure { .. }
        | Revision3NpcGreetingEditConflictV1::OwnedModuleDrift { .. }
        | Revision3NpcGreetingEditConflictV1::CandidateNotPersistable { .. } => {
            "AUTHORING_REVISION3_NPC_GREETING_PROJECT_INVALID"
        }
        Revision3NpcGreetingEditConflictV1::DialogLineRejected { .. } => {
            "AUTHORING_REVISION3_NPC_GREETING_DIALOG_CONFLICT"
        }
        Revision3NpcGreetingEditConflictV1::DialogRequestBasisMismatch => {
            "AUTHORING_REVISION3_NPC_GREETING_DIALOG_CONFLICT"
        }
        Revision3NpcGreetingEditConflictV1::InsertIndexOutOfBounds { .. } => {
            "AUTHORING_REVISION3_NPC_GREETING_INDEX_CONFLICT"
        }
        Revision3NpcGreetingEditConflictV1::TooManyBindings { .. }
        | Revision3NpcGreetingEditConflictV1::InvalidLineReference { .. }
        | Revision3NpcGreetingEditConflictV1::DuplicateLine { .. } => {
            "AUTHORING_REVISION3_NPC_GREETING_BINDING_CONFLICT"
        }
    };
    Failure::new(code, error.to_string())
}

fn map_store_error(error: WorkingStoreError) -> Failure {
    let code = match error {
        WorkingStoreError::InvalidLimits(_) => {
            "AUTHORING_REVISION3_NPC_GREETING_STORE_LIMITS_INVALID"
        }
        WorkingStoreError::MissingRoot(_) => "AUTHORING_REVISION3_NPC_GREETING_STORE_ROOT_MISSING",
        WorkingStoreError::UnsafePath { .. } => {
            "AUTHORING_REVISION3_NPC_GREETING_STORE_PATH_UNSAFE"
        }
        WorkingStoreError::LimitExceeded { .. } => "AUTHORING_REVISION3_NPC_GREETING_STORE_LIMIT",
        WorkingStoreError::HeadConflict { .. } => "AUTHORING_REVISION3_NPC_GREETING_HEAD_CONFLICT",
        WorkingStoreError::MissingHead(_) => "AUTHORING_REVISION3_NPC_GREETING_HEAD_MISSING",
        WorkingStoreError::MissingObject(_) => {
            "AUTHORING_REVISION3_NPC_GREETING_STORE_OBJECT_MISSING"
        }
        WorkingStoreError::SealMismatch { .. } => {
            "AUTHORING_REVISION3_NPC_GREETING_STORE_SEAL_MISMATCH"
        }
        WorkingStoreError::Collision { .. } => "AUTHORING_REVISION3_NPC_GREETING_STORE_COLLISION",
        WorkingStoreError::InvalidJson { .. } | WorkingStoreError::NonCanonicalJson { .. } => {
            "AUTHORING_REVISION3_NPC_GREETING_STORE_JSON_INVALID"
        }
        WorkingStoreError::Invariant(_)
        | WorkingStoreError::InvalidOgg(_)
        | WorkingStoreError::OggMetadataMismatch { .. } => {
            "AUTHORING_REVISION3_NPC_GREETING_STORE_INVARIANT"
        }
        WorkingStoreError::StagingCleanup { .. } | WorkingStoreError::Io(_) => {
            "AUTHORING_REVISION3_NPC_GREETING_STORE_IO"
        }
    };
    Failure::new(code, "the revision-3 NPC greeting Store operation failed")
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
        AssetStoreIndex, ContentSeal, EntityId, FormatV2, GameGenerationAnchor,
        NpcParentClassInput, ProjectId, ProjectMeta, ProjectRevision3,
        Revision3DialogEmptyVoiceSlotIntentV1, Revision3DialogLineInsertRequestV1,
        Revision3DialogLocalizationIntentV1, Revision3Entity, Revision3EntityKind,
        Revision3EntityPayload, Revision3NpcDraft, Revision3NpcDraftInput,
        Revision3NpcGreetingBindingV1, Revision3NpcGreetingEditRequestV1,
        Revision3NpcGreetingIntentV1, Revision3OriginRef, Revision3TypedRef, SchemaRevisionV3,
        Sha256Digest, LOGICAL_NPC_CLONE_GENERATOR_ID, LOGICAL_NPC_CLONE_GENERATOR_VERSION,
    };
    use tempfile::TempDir;

    use super::*;

    const NPC_ID: u8 = 0x61;
    const MODULE_ID: u8 = 0x62;
    const LINE_ID: u8 = 0x63;
    const LOCALIZATION_ID: u8 = 0x64;

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

    fn parent(value: u8, runtime_class: &str) -> NpcParentClassInput {
        NpcParentClassInput {
            generation: target(),
            source_seal: seal(value, 4_096),
            catalog_layer: "base-game.g1r.npc-parents.v1".to_owned(),
            canonical_selector: runtime_class.to_owned(),
            runtime_class: runtime_class.to_owned(),
        }
    }

    fn npc_project(revision: u64) -> ProjectRevision3 {
        let project_id = ProjectId::from_bytes([0x60; 16]);
        let npc_id = entity_id(NPC_ID);
        let module_id = entity_id(MODULE_ID);
        let owner = Revision3TypedRef::new(project_id, npc_id, Revision3EntityKind::NpcDraft);
        let draft = Revision3NpcDraft {
            generator_id: LOGICAL_NPC_CLONE_GENERATOR_ID.to_owned(),
            generator_version: LOGICAL_NPC_CLONE_GENERATOR_VERSION,
            input: Revision3NpcDraftInput {
                target: target(),
                module_namespace: "GoreMods.Npcs.GreetingFixture".to_owned(),
                unique_name: "GORE_GREETING_FIXTURE".to_owned(),
                parent_character_definition: parent(
                    2,
                    "UCharacterDefinition_Human_OM_GRD_Asghan_263",
                ),
                parent_ai_agent_config: parent(3, "UAIAgentConfig_Human_OM_GRD_Asghan_263"),
                parent_spawn_definition: parent(4, "USpawnAIAgentDefinition_OM_GRD_Asghan_263"),
            },
            script_module: Revision3TypedRef::new(
                project_id,
                module_id,
                Revision3EntityKind::ScriptModule,
            ),
            greetings: Vec::new(),
        };
        let module = draft
            .regenerate_script_module(owner.clone())
            .expect("valid exact NPC fixture");
        ProjectRevision3 {
            format: FormatV2,
            schema_revision: SchemaRevisionV3,
            project_id,
            revision,
            meta: ProjectMeta {
                name: "NPC greeting FFI fixture".to_owned(),
                version: "1.0.0".to_owned(),
                author: "tests".to_owned(),
            },
            target: target(),
            authoring_locales: BTreeSet::new(),
            entities: BTreeMap::from([
                (
                    npc_id,
                    Revision3Entity {
                        id: npc_id,
                        display_name: "Greeting Guard".to_owned(),
                        origin: Revision3OriginRef::New {
                            authored_runtime_id: draft.input.unique_name.clone(),
                        },
                        revision: 3,
                        payload: Revision3EntityPayload::NpcDraft(draft),
                    },
                ),
                (
                    module_id,
                    Revision3Entity {
                        id: module_id,
                        display_name: "Greeting Guard source".to_owned(),
                        origin: Revision3OriginRef::Generated {
                            generator_id: LOGICAL_NPC_CLONE_GENERATOR_ID.to_owned(),
                            generator_version: LOGICAL_NPC_CLONE_GENERATOR_VERSION,
                            owner,
                        },
                        revision: 5,
                        payload: Revision3EntityPayload::ScriptModule(module),
                    },
                ),
                (
                    entity_id(LOCALIZATION_ID),
                    Revision3Entity {
                        id: entity_id(LOCALIZATION_ID),
                        display_name: "Existing greeting localization".to_owned(),
                        origin: Revision3OriginRef::New {
                            authored_runtime_id: "GORE_GREETING_EXISTING_TEXT_ENTITY".to_owned(),
                        },
                        revision: 2,
                        payload: Revision3EntityPayload::LocalizationEntry(LocalizationEntry {
                            loc_id: "GORE_GREETING_EXISTING_TEXT".to_owned(),
                            texts: BTreeMap::new(),
                        }),
                    },
                ),
                (
                    entity_id(LINE_ID),
                    Revision3Entity {
                        id: entity_id(LINE_ID),
                        display_name: "Existing greeting line".to_owned(),
                        origin: Revision3OriginRef::New {
                            authored_runtime_id: "GORE_GREETING_EXISTING_LINE".to_owned(),
                        },
                        revision: 4,
                        payload: Revision3EntityPayload::DialogLine(DialogLine {
                            localization: Revision3TypedRef::new(
                                project_id,
                                entity_id(LOCALIZATION_ID),
                                Revision3EntityKind::LocalizationEntry,
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

    fn published_store() -> PublishedStore {
        let temp = TempDir::new().unwrap();
        let store = WorkingProjectStore::at(temp.path(), ffi_store_limits()).unwrap();
        let project = npc_project(7);
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

    fn request(
        store: &PublishedStore,
        intent: Revision3NpcGreetingIntentV1,
    ) -> Revision3NpcGreetingEditRequestV1 {
        Revision3NpcGreetingEditRequestV1 {
            expected_head: store.head.clone(),
            expected_project_id: store.project.project_id,
            expected_revision: store.project.revision,
            expected_target: store.project.target.clone(),
            npc_id: entity_id(NPC_ID),
            expected_npc_revision: store.project.entities[&entity_id(NPC_ID)].revision,
            intent,
        }
    }

    fn replace_request(store: &PublishedStore) -> Revision3NpcGreetingEditRequestV1 {
        request(
            store,
            Revision3NpcGreetingIntentV1::Replace {
                bindings: vec![Revision3NpcGreetingBindingV1 {
                    line: Revision3TypedRef::new(
                        store.project.project_id,
                        entity_id(LINE_ID),
                        Revision3EntityKind::DialogLine,
                    ),
                }],
            },
        )
    }

    fn create_request(store: &PublishedStore) -> Revision3NpcGreetingEditRequestV1 {
        request(
            store,
            Revision3NpcGreetingIntentV1::CreateAndInsert {
                index: 0,
                line: Revision3DialogLineInsertRequestV1 {
                    expected_head: store.head.clone(),
                    expected_project_id: store.project.project_id,
                    expected_revision: store.project.revision,
                    expected_target: store.project.target.clone(),
                    line_id: entity_id(0x71),
                    line_display_name: "Created greeting line".to_owned(),
                    line_authored_identity: "GORE_GREETING_CREATED_LINE".to_owned(),
                    speaker_hint: Some("Asghan".to_owned()),
                    localization: Revision3DialogLocalizationIntentV1::Create {
                        localization_id: entity_id(0x72),
                        display_name: "Created greeting text".to_owned(),
                        loc_id: "GORE_GREETING_CREATED_TEXT".to_owned(),
                        texts: BTreeMap::from([(
                            "de".parse().unwrap(),
                            "Willkommen im Alten Lager.".to_owned(),
                        )]),
                    },
                    voice_slot: Some(Revision3DialogEmptyVoiceSlotIntentV1 {
                        slot_id: entity_id(0x73),
                        locale: "de".parse().unwrap(),
                        display_name: "Created greeting German voice".to_owned(),
                    }),
                },
            },
        )
    }

    fn raw_request(store: &PublishedStore, request: &Revision3NpcGreetingEditRequestV1) -> String {
        serde_json::to_string(&ExactWireRequest {
            command: COMMAND.to_owned(),
            payload: PrepareNpcGreetingWirePayload {
                current_project_json: store.project_json.clone(),
                npc_greeting_request_json: request.to_canonical_json().unwrap(),
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

    #[test]
    fn outer_wire_is_exact_closed_project_only_and_publicly_dispatched() {
        let canonical = serde_json::to_string(&ExactWireRequest {
            command: COMMAND.to_owned(),
            payload: PrepareNpcGreetingWirePayload {
                current_project_json: "{}".to_owned(),
                npc_greeting_request_json: "{}".to_owned(),
                root: "C:/store".to_owned(),
            },
        })
        .unwrap();
        assert_eq!(
            parse_exact_wire::<PrepareNpcGreetingWirePayload>(&(canonical.clone() + "\n"))
                .unwrap_err()
                .code,
            "AUTHORING_REVISION3_NPC_GREETING_REQUEST_INVALID"
        );
        let unknown = format!(
            "{{\"command\":\"{COMMAND}\",\"payload\":{{\"current_project_json\":\"{{}}\",\"npc_greeting_request_json\":\"{{}}\",\"root\":\"C:/store\",\"game_root\":\"C:/game\"}}}}"
        );
        let public: Value = serde_json::from_str(&crate::execute_json(&unknown)).unwrap();
        assert_eq!(
            public["error"]["code"],
            "AUTHORING_REVISION3_NPC_GREETING_REQUEST_INVALID"
        );
        assert!(!canonical.contains("game_root"));
        assert!(!canonical.contains("install"));
        assert!(!canonical.contains("save"));
    }

    #[test]
    fn replace_prepares_reopened_candidate_without_source_or_head_mutation() {
        let store = published_store();
        let basis_module = store.project.entities[&entity_id(MODULE_ID)].clone();
        let response =
            prepare_revision3_npc_greeting_v1_raw(&raw_request(&store, &replace_request(&store)));
        assert_eq!(response["ok"], true, "{response}");
        let actual_keys = response
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        assert_eq!(
            actual_keys,
            BTreeSet::from([
                "basis_head_json",
                "build_status",
                "created_line_id",
                "created_localization_id",
                "created_voice_slot_id",
                "greeting_count",
                "head_json",
                "localization_action",
                "mode",
                "module_id",
                "module_revision",
                "npc_id",
                "npc_revision",
                "ok",
                "outcome",
                "project_id",
                "project_json",
                "publication_status",
                "revision",
                "runtime_status",
                "topic_authority",
            ])
        );
        assert_eq!(response["outcome"], "prepared_unpublished");
        assert_eq!(response["mode"], "replace");
        assert_eq!(response["greeting_count"], 1);
        assert_eq!(response["build_status"], "blocked");
        assert_eq!(response["runtime_status"], "runtime_unqualified");
        assert_eq!(response["topic_authority"], "not_granted");
        assert_eq!(response["publication_status"], "not_supported");
        assert_eq!(fixed_head(&store), store.fixed_head_bytes);

        let candidate = reopened_candidate(&store, &response);
        let Revision3EntityPayload::NpcDraft(npc) = &candidate.entities[&entity_id(NPC_ID)].payload
        else {
            panic!("candidate NPC kind")
        };
        assert_eq!(npc.greetings.len(), 1);
        assert_eq!(npc.greetings[0].line.id, entity_id(LINE_ID));
        assert_eq!(candidate.entities[&entity_id(MODULE_ID)], basis_module);
        assert_eq!(fixed_head(&store), store.fixed_head_bytes);
    }

    #[test]
    fn create_and_insert_is_atomic_and_reopens_every_created_entity() {
        let store = published_store();
        let basis_module = store.project.entities[&entity_id(MODULE_ID)].clone();
        let response =
            prepare_revision3_npc_greeting_v1_raw(&raw_request(&store, &create_request(&store)));
        assert_eq!(response["ok"], true, "{response}");
        assert_eq!(response["mode"], "create_and_insert");
        assert_eq!(response["greeting_count"], 1);
        assert_eq!(response["created_line_id"], entity_id(0x71).to_string());
        assert_eq!(
            response["created_localization_id"],
            entity_id(0x72).to_string()
        );
        assert_eq!(
            response["created_voice_slot_id"],
            entity_id(0x73).to_string()
        );
        assert_eq!(response["localization_action"], "created");
        assert_eq!(fixed_head(&store), store.fixed_head_bytes);

        let candidate = reopened_candidate(&store, &response);
        let Revision3EntityPayload::NpcDraft(npc) = &candidate.entities[&entity_id(NPC_ID)].payload
        else {
            panic!("candidate NPC kind")
        };
        assert_eq!(npc.greetings[0].line.id, entity_id(0x71));
        assert!(candidate.entities.contains_key(&entity_id(0x71)));
        assert!(candidate.entities.contains_key(&entity_id(0x72)));
        assert!(candidate.entities.contains_key(&entity_id(0x73)));
        assert_eq!(candidate.entities[&entity_id(MODULE_ID)], basis_module);
        assert_eq!(candidate.entities.len(), store.project.entities.len() + 3);
    }

    #[test]
    fn rejection_and_fixed_head_race_never_publish_the_candidate() {
        let store = published_store();
        let mut duplicate = replace_request(&store);
        let Revision3NpcGreetingIntentV1::Replace { bindings } = &mut duplicate.intent else {
            unreachable!()
        };
        bindings.push(bindings[0].clone());
        let response = prepare_revision3_npc_greeting_v1_raw(&raw_request(&store, &duplicate));
        assert_eq!(
            response["error"]["code"],
            "AUTHORING_REVISION3_NPC_GREETING_BINDING_CONFLICT"
        );
        assert_eq!(fixed_head(&store), store.fixed_head_bytes);

        let input = raw_request(&store, &replace_request(&store));
        let mut rival = store.project.clone();
        rival.revision += 1;
        rival.meta.name = "Concurrent publisher".to_owned();
        let result = prepare_revision3_npc_greeting_v1_inner_with_final_guard(&input, || {
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
            "AUTHORING_REVISION3_NPC_GREETING_HEAD_CONFLICT"
        );
    }

    #[test]
    fn malformed_nested_request_is_rejected_before_store_open() {
        let temp = TempDir::new().unwrap();
        let missing = temp.path().join("missing-store");
        let input = serde_json::to_string(&ExactWireRequest {
            command: COMMAND.to_owned(),
            payload: PrepareNpcGreetingWirePayload {
                current_project_json: "{}".to_owned(),
                npc_greeting_request_json: "{}".to_owned(),
                root: missing.to_string_lossy().into_owned(),
            },
        })
        .unwrap();
        assert_eq!(
            prepare_revision3_npc_greeting_v1_inner(&input)
                .unwrap_err()
                .code,
            "AUTHORING_REVISION3_NPC_GREETING_REQUEST_INVALID"
        );
        assert!(!missing.exists());
    }
}
