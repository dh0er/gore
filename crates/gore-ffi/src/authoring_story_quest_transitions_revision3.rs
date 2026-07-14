//! Native, prepare-only orchestration for one exact revision-3 Quest transition-plan edit.
//!
//! The caller supplies only exact canonical project/request transports and one working Store.
//! Native code fully reopens the fixed head, binds every project/head/target/Quest/plan CAS token,
//! verifies the exact-current Quest closure, evaluates the filesystem-free transaction, prepares
//! immutable candidate objects, and returns them only after a full reopen and final fixed-head
//! check. This route accepts no game root and never builds, deploys, writes a save, or publishes
//! `gore-project.json`.

use std::path::Path;

use gore_authoring::{
    apply_revision3_quest_transition_plan_transaction_v1, revision3_quest_transition_plan_basis_v1,
    AssetVerification, EntityId, OpenedRevision3Checkpoint, Revision3EntityPayload,
    Revision3QuestTransitionPlanBasisErrorV1, Revision3QuestTransitionPlanBasisV1,
    Revision3QuestTransitionPlanEditBuildStatusV1, Revision3QuestTransitionPlanEditConflictV1,
    Revision3QuestTransitionPlanEditErrorV1, Revision3QuestTransitionPlanEditEvaluationV1,
    Revision3QuestTransitionPlanEditOutcomeV1, Revision3QuestTransitionPlanEditPublicationStatusV1,
    Revision3QuestTransitionPlanEditRequestV1, Revision3QuestTransitionPlanEditRuntimeStatusV1,
    WorkingHead, WorkingProjectStore, WorkingStoreError, WorkingStoreLimits,
    MAX_PROJECT_JSON_BYTES, MAX_REVISION3_QUEST_TRANSITION_PLAN_EDIT_REQUEST_JSON_BYTES_V1,
    REVISION3_SEMANTIC_QUEST_GENERATOR_VERSION,
};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::err;

pub(super) const COMMAND: &str = "authoring_store_prepare_revision3_quest_transitions_edit_v1";

const MAX_PATH_BYTES: usize = 32 * 1024;
const MAX_HEAD_JSON_BYTES: usize = 64 * 1024;
const MAX_RESPONSE_BYTES: usize = 64 * 1024 * 1024;
const MAX_ERROR_MESSAGE_BYTES: usize = 4 * 1024;
const MAX_BASIS_REVISION: u64 = i64::MAX as u64 - 1;
const MAX_TRANSITIONS_REQUEST_JSON_BYTES: usize = 512 * 1024;
const _: () = assert!(
    MAX_REVISION3_QUEST_TRANSITION_PLAN_EDIT_REQUEST_JSON_BYTES_V1
        == MAX_TRANSITIONS_REQUEST_JSON_BYTES
);
// Nested canonical JSON strings can add at most one escape byte per source byte. The Store path
// is arbitrary caller text and retains the conservative six-byte JSON escape allowance.
const MAX_WIRE_BYTES: usize = MAX_PROJECT_JSON_BYTES * 2
    + MAX_TRANSITIONS_REQUEST_JSON_BYTES * 2
    + MAX_PATH_BYTES * 6
    + 4 * 1024;

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ExactWireRequest<P> {
    command: String,
    payload: P,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct PrepareQuestTransitionsWirePayload {
    current_project_json: String,
    quest_transitions_request_json: String,
    root: String,
}

#[derive(Debug)]
struct BoundQuestBasis {
    module_id: EntityId,
    module_revision: u64,
    transition: Revision3QuestTransitionPlanBasisV1,
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

pub(super) fn prepare_revision3_quest_transitions_edit_v1_raw(input: &str) -> Value {
    prepare_revision3_quest_transitions_edit_v1_inner(input).unwrap_or_else(Failure::response)
}

fn prepare_revision3_quest_transitions_edit_v1_inner(input: &str) -> Result<Value, Failure> {
    prepare_revision3_quest_transitions_edit_v1_inner_with_test_seams(input, || {}, || {})
}

#[cfg(test)]
fn prepare_revision3_quest_transitions_edit_v1_inner_with_final_guard<F>(
    input: &str,
    final_guard: F,
) -> Result<Value, Failure>
where
    F: FnOnce(),
{
    prepare_revision3_quest_transitions_edit_v1_inner_with_test_seams(input, || {}, final_guard)
}

fn prepare_revision3_quest_transitions_edit_v1_inner_with_test_seams<B, F>(
    input: &str,
    before_checkpoint: B,
    final_guard: F,
) -> Result<Value, Failure>
where
    B: FnOnce(),
    F: FnOnce(),
{
    let payload: PrepareQuestTransitionsWirePayload = parse_exact_wire(input)?;
    validate_payload(&payload)?;
    // Parse and inspect the bounded nested transport before opening any caller-selected Store.
    let request = Revision3QuestTransitionPlanEditRequestV1::from_json(
        &payload.quest_transitions_request_json,
    )
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
            "AUTHORING_REVISION3_QUEST_TRANSITIONS_STORE_INVARIANT",
            "the exact current revision-3 project could not be serialized canonically",
        )
    })?;
    if canonical_basis.as_bytes() != payload.current_project_json.as_bytes() {
        return Err(Failure::new(
            "AUTHORING_REVISION3_QUEST_TRANSITIONS_PROJECT_CONFLICT",
            "current_project_json differs from the exact published revision-3 project",
        ));
    }

    let bound = bind_request_to_basis(&basis.head, &basis.project, &request)?;

    // The transaction itself proves the exact owned module through deterministic regeneration
    // with the retained collision evidence. The full Store open above has already verified every
    // referenced asset, so this route neither accepts nor rebuilds separate collision authority.

    let outcome = match apply_revision3_quest_transition_plan_transaction_v1(
        &basis.head,
        &payload.current_project_json,
        &payload.quest_transitions_request_json,
    )
    .map_err(map_transaction_error)?
    {
        Revision3QuestTransitionPlanEditEvaluationV1::Applied(outcome) => *outcome,
        Revision3QuestTransitionPlanEditEvaluationV1::Rejected(rejection) => {
            return Err(map_transaction_conflict(rejection.conflict));
        }
    };
    require_signed_serializable(&outcome.project)?;
    validate_transaction_binding(&basis, &request, &bound, &outcome)?;

    before_checkpoint();
    let prepared = store
        .prepare_revision3_checkpoint(Some(&basis.head), &outcome.project)
        .map_err(map_store_error)?;
    let reopened = store
        .open_revision3_head_bytes(&prepared.head_bytes, AssetVerification::Full)
        .map_err(map_store_error)?;
    if reopened.head != prepared.head || reopened.project != outcome.project {
        return Err(Failure::new(
            "AUTHORING_REVISION3_QUEST_TRANSITIONS_STORE_INVARIANT",
            "the prepared Quest transition checkpoint did not fully reopen exactly",
        ));
    }
    let reopened_json = reopened.project.to_canonical_json().map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_QUEST_TRANSITIONS_STORE_INVARIANT",
            "the fully reopened Quest transition candidate could not be serialized",
        )
    })?;
    if reopened_json != outcome.canonical_project_json {
        return Err(Failure::new(
            "AUTHORING_REVISION3_QUEST_TRANSITIONS_STORE_INVARIANT",
            "the fully reopened Quest transition candidate changed canonical bytes",
        ));
    }
    verify_reopened_transition(&reopened.project, &request, &outcome)?;

    // A concurrent publisher may leave immutable candidate objects only; it can never make this
    // route report success against a changed fixed head.
    final_guard();
    require_fixed_basis(&store, &basis.head, &basis.project)?;

    let basis_head_json = canonical_head_json(&basis.head)?;
    let candidate_head_json = String::from_utf8(prepared.head_bytes).map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_QUEST_TRANSITIONS_STORE_INVARIANT",
            "the prepared Quest transition head is not UTF-8 JSON",
        )
    })?;
    if candidate_head_json.is_empty() || candidate_head_json.len() > MAX_HEAD_JSON_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_QUEST_TRANSITIONS_RESPONSE_LIMIT",
            "the prepared Quest transition head exceeds its bounded transport limit",
        ));
    }
    require_signed_serializable(&prepared.head)?;
    let response = json!({
        "ok": true,
        "outcome": "prepared_unpublished",
        "basis_head_json": basis_head_json,
        "head_json": candidate_head_json,
        "project_json": outcome.canonical_project_json,
        "project_id": outcome.project.project_id.to_string(),
        "revision": outcome.project.revision,
        "quest_id": outcome.quest_id.to_string(),
        "module_id": outcome.script_module_id.to_string(),
        "quest_revision": outcome.quest_revision,
        "module_revision": outcome.script_module_revision,
        "previous_generator_version": outcome.previous_generator_version,
        "upgraded_from_legacy": outcome.upgraded_from_legacy,
        "transition_plan_seal": outcome.transition_plan_seal,
        "build_status": "blocked",
        "runtime_status": "runtime_unqualified",
        "publication_status": "not_supported",
    });
    enforce_response_budget(&response)?;

    // Reopen the authoritative fixed head once more after constructing the complete response.
    require_fixed_basis(&store, &basis.head, &basis.project)?;
    Ok(response)
}

fn parse_exact_wire<P>(input: &str) -> Result<P, Failure>
where
    P: DeserializeOwned + Serialize,
{
    if input.len() > MAX_WIRE_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_QUEST_TRANSITIONS_INPUT_LIMIT",
            format!(
                "revision-3 Quest transitions request exceeds the {MAX_WIRE_BYTES}-byte wire limit"
            ),
        ));
    }
    let request: ExactWireRequest<P> =
        serde_json::from_str(input).map_err(|_| invalid_request())?;
    if request.command != COMMAND {
        return Err(invalid_request());
    }
    let canonical = serde_json::to_string(&request).map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_QUEST_TRANSITIONS_INVARIANT",
            "the Quest transitions outer request could not be serialized",
        )
    })?;
    if canonical != input {
        return Err(invalid_request());
    }
    Ok(request.payload)
}

fn validate_payload(payload: &PrepareQuestTransitionsWirePayload) -> Result<(), Failure> {
    validate_path(&payload.root)?;
    if payload.current_project_json.is_empty() {
        return Err(invalid_request());
    }
    if payload.current_project_json.len() > MAX_PROJECT_JSON_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_QUEST_TRANSITIONS_PROJECT_LIMIT",
            format!("current_project_json exceeds the {MAX_PROJECT_JSON_BYTES}-byte limit"),
        ));
    }
    if payload.quest_transitions_request_json.is_empty() {
        return Err(invalid_request());
    }
    if payload.quest_transitions_request_json.len() > MAX_TRANSITIONS_REQUEST_JSON_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_QUEST_TRANSITIONS_REQUEST_LIMIT",
            format!(
                "quest_transitions_request_json exceeds the {MAX_TRANSITIONS_REQUEST_JSON_BYTES}-byte limit"
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

fn bind_request_to_basis(
    head: &WorkingHead,
    project: &gore_authoring::ProjectRevision3,
    request: &Revision3QuestTransitionPlanEditRequestV1,
) -> Result<BoundQuestBasis, Failure> {
    if request.expected_head != *head {
        return Err(head_conflict());
    }
    if request.expected_project_id != project.project_id
        || request.expected_revision != project.revision
    {
        return Err(Failure::new(
            "AUTHORING_REVISION3_QUEST_TRANSITIONS_PROJECT_CONFLICT",
            "the Quest transitions request project differs from the exact published project",
        ));
    }
    if request.expected_target != project.target {
        return Err(Failure::new(
            "AUTHORING_REVISION3_QUEST_TRANSITIONS_TARGET_CONFLICT",
            "the Quest transitions request target differs from the exact published project target",
        ));
    }
    let Some(entity) = project.entities.get(&request.quest_id) else {
        return Err(quest_conflict(
            "the requested Quest does not exist in the exact published project",
        ));
    };
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
    validate_entity_revision(entity.revision)?;
    if quest.script_module.project_id != project.project_id {
        return Err(project_invalid(
            "the exact Quest has a foreign owned module binding",
        ));
    }
    let module_id = quest.script_module.id;
    let Some(module) = project.entities.get(&module_id) else {
        return Err(project_invalid(
            "the exact Quest owned ScriptModule is missing",
        ));
    };
    if !matches!(module.payload, Revision3EntityPayload::ScriptModule(_)) {
        return Err(project_invalid(
            "the exact Quest owned entity is not a ScriptModule",
        ));
    }
    validate_entity_revision(module.revision)?;
    let transition =
        revision3_quest_transition_plan_basis_v1(quest).map_err(map_transition_basis_error)?;
    if request.expected_transition_plan_seal != transition.seal {
        return Err(Failure::new(
            "AUTHORING_REVISION3_QUEST_TRANSITIONS_TRANSITION_PLAN_CONFLICT",
            "the expected transition-plan seal differs from the exact effective Quest plan",
        ));
    }
    Ok(BoundQuestBasis {
        module_id,
        module_revision: module.revision,
        transition,
    })
}

fn validate_transaction_binding(
    basis: &OpenedRevision3Checkpoint,
    request: &Revision3QuestTransitionPlanEditRequestV1,
    bound: &BoundQuestBasis,
    outcome: &Revision3QuestTransitionPlanEditOutcomeV1,
) -> Result<(), Failure> {
    if outcome.basis_head != basis.head
        || outcome.quest_id != request.quest_id
        || outcome.project.project_id != basis.project.project_id
        || outcome.project.target != basis.project.target
        || outcome.project.revision != basis.project.revision + 1
        || outcome.script_module_id != bound.module_id
        || outcome.quest_revision != request.expected_quest_revision + 1
        || outcome.script_module_revision != bound.module_revision + 1
        || outcome.previous_generator_version != bound.transition.generator_version
        || outcome.upgraded_from_legacy != bound.transition.legacy_synthetic
    {
        return Err(Failure::new(
            "AUTHORING_REVISION3_QUEST_TRANSITIONS_INVARIANT",
            "the Quest transitions transaction changed its exact project/request binding",
        ));
    }
    match outcome.build_status {
        Revision3QuestTransitionPlanEditBuildStatusV1::Blocked => {}
    }
    match outcome.runtime_status {
        Revision3QuestTransitionPlanEditRuntimeStatusV1::RuntimeUnqualified => {}
    }
    match outcome.publication_status {
        Revision3QuestTransitionPlanEditPublicationStatusV1::NotSupported => {}
    }
    Ok(())
}

fn verify_reopened_transition(
    project: &gore_authoring::ProjectRevision3,
    request: &Revision3QuestTransitionPlanEditRequestV1,
    outcome: &Revision3QuestTransitionPlanEditOutcomeV1,
) -> Result<(), Failure> {
    let Some(entity) = project.entities.get(&request.quest_id) else {
        return Err(store_invariant(
            "the fully reopened Quest transition candidate lost its Quest",
        ));
    };
    let Revision3EntityPayload::QuestDraft(quest) = &entity.payload else {
        return Err(store_invariant(
            "the fully reopened Quest transition candidate changed Quest kind",
        ));
    };
    let transition =
        revision3_quest_transition_plan_basis_v1(quest).map_err(map_transition_basis_error)?;
    if entity.revision != outcome.quest_revision
        || quest.generator_version != REVISION3_SEMANTIC_QUEST_GENERATOR_VERSION
        || transition.legacy_synthetic
        || transition.plan != request.transition_plan
        || transition.seal != outcome.transition_plan_seal
    {
        return Err(store_invariant(
            "the fully reopened Quest transition plan differs from the prepared outcome",
        ));
    }
    let Some(module) = project.entities.get(&outcome.script_module_id) else {
        return Err(store_invariant(
            "the fully reopened Quest transition candidate lost its ScriptModule",
        ));
    };
    if module.revision != outcome.script_module_revision
        || !matches!(module.payload, Revision3EntityPayload::ScriptModule(_))
    {
        return Err(store_invariant(
            "the fully reopened Quest transition ScriptModule differs from the prepared outcome",
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
        return Err(revision_limit());
    }
    Ok(())
}

fn validate_entity_revision(revision: u64) -> Result<(), Failure> {
    if revision > MAX_BASIS_REVISION {
        return Err(revision_limit());
    }
    Ok(())
}

fn require_signed_serializable(value: &impl Serialize) -> Result<(), Failure> {
    let value = serde_json::to_value(value).map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_QUEST_TRANSITIONS_INVARIANT",
            "a Quest transitions wire value could not be inspected",
        )
    })?;
    require_signed_json_value(&value)
}

fn require_signed_json_value(value: &Value) -> Result<(), Failure> {
    match value {
        Value::Number(number) if number.as_u64().is_some_and(|value| value > i64::MAX as u64) => {
            Err(Failure::new(
                "AUTHORING_REVISION3_QUEST_TRANSITIONS_SIGNED_WIRE_LIMIT",
                "a Quest transitions wire integer exceeds the signed 64-bit transport range",
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
            "AUTHORING_REVISION3_QUEST_TRANSITIONS_INVARIANT",
            "the Quest transitions basis head could not be serialized",
        )
    })?;
    if value.is_empty() || value.len() > MAX_HEAD_JSON_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_QUEST_TRANSITIONS_RESPONSE_LIMIT",
            "the Quest transitions basis head exceeds its bounded transport limit",
        ));
    }
    Ok(value)
}

fn enforce_response_budget(response: &Value) -> Result<(), Failure> {
    let encoded = serde_json::to_vec(response).map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_QUEST_TRANSITIONS_INVARIANT",
            "the Quest transitions response could not be serialized",
        )
    })?;
    if encoded.len() > MAX_RESPONSE_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_QUEST_TRANSITIONS_RESPONSE_LIMIT",
            "the Quest transitions response exceeds its bounded transport budget",
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
        "AUTHORING_REVISION3_QUEST_TRANSITIONS_REQUEST_INVALID",
        "request must contain one exact duplicate-free command and exactly current_project_json, quest_transitions_request_json, and root",
    )
}

fn head_conflict() -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_QUEST_TRANSITIONS_HEAD_CONFLICT",
        "the published revision-3 head changed or differs from the Quest transitions request",
    )
}

fn quest_conflict(message: &'static str) -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_QUEST_TRANSITIONS_QUEST_CONFLICT",
        message,
    )
}

fn project_invalid(message: &'static str) -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_QUEST_TRANSITIONS_PROJECT_INVALID",
        message,
    )
}

fn store_invariant(message: &'static str) -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_QUEST_TRANSITIONS_STORE_INVARIANT",
        message,
    )
}

fn revision_limit() -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_QUEST_TRANSITIONS_REVISION_LIMIT",
        format!("a Quest transition basis revision exceeds {MAX_BASIS_REVISION}"),
    )
}

fn map_request_error(error: impl std::fmt::Display) -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_QUEST_TRANSITIONS_REQUEST_INVALID",
        format!("the exact Quest transitions request is invalid: {error}"),
    )
}

fn map_transition_basis_error(_error: Revision3QuestTransitionPlanBasisErrorV1) -> Failure {
    project_invalid("the exact current Quest has an invalid transition-plan basis")
}

fn map_transaction_error(error: Revision3QuestTransitionPlanEditErrorV1) -> Failure {
    match error {
        Revision3QuestTransitionPlanEditErrorV1::InvalidProject(_) => project_invalid(
            "the exact current revision-3 project is not a valid Quest-transition basis",
        ),
        Revision3QuestTransitionPlanEditErrorV1::InvalidRequest(error) => map_request_error(error),
        Revision3QuestTransitionPlanEditErrorV1::ReopenCandidate(_)
        | Revision3QuestTransitionPlanEditErrorV1::CanonicalReopenMismatch => Failure::new(
            "AUTHORING_REVISION3_QUEST_TRANSITIONS_INVARIANT",
            "the Quest transitions candidate failed exact canonical reopen",
        ),
    }
}

fn map_transaction_conflict(error: Revision3QuestTransitionPlanEditConflictV1) -> Failure {
    let code = match &error {
        Revision3QuestTransitionPlanEditConflictV1::CurrentHeadMismatch => {
            "AUTHORING_REVISION3_QUEST_TRANSITIONS_HEAD_CONFLICT"
        }
        Revision3QuestTransitionPlanEditConflictV1::ProjectIdentityMismatch { .. }
        | Revision3QuestTransitionPlanEditConflictV1::ProjectRevisionConflict { .. } => {
            "AUTHORING_REVISION3_QUEST_TRANSITIONS_PROJECT_CONFLICT"
        }
        Revision3QuestTransitionPlanEditConflictV1::ProjectTargetMismatch => {
            "AUTHORING_REVISION3_QUEST_TRANSITIONS_TARGET_CONFLICT"
        }
        Revision3QuestTransitionPlanEditConflictV1::InvalidQuestEntity { .. }
        | Revision3QuestTransitionPlanEditConflictV1::QuestRevisionConflict { .. } => {
            "AUTHORING_REVISION3_QUEST_TRANSITIONS_QUEST_CONFLICT"
        }
        Revision3QuestTransitionPlanEditConflictV1::TransitionPlanSealConflict { .. } => {
            "AUTHORING_REVISION3_QUEST_TRANSITIONS_TRANSITION_PLAN_CONFLICT"
        }
        Revision3QuestTransitionPlanEditConflictV1::ObjectiveSlotsChanged
        | Revision3QuestTransitionPlanEditConflictV1::NextSlotOrdinalRegression { .. } => {
            "AUTHORING_REVISION3_QUEST_TRANSITIONS_TRANSITION_PLAN_CONFLICT"
        }
        Revision3QuestTransitionPlanEditConflictV1::NoChanges => {
            "AUTHORING_REVISION3_QUEST_TRANSITIONS_NO_CHANGES"
        }
        Revision3QuestTransitionPlanEditConflictV1::ProjectRevisionOverflow
        | Revision3QuestTransitionPlanEditConflictV1::QuestRevisionOverflow { .. }
        | Revision3QuestTransitionPlanEditConflictV1::ScriptModuleRevisionOverflow { .. } => {
            "AUTHORING_REVISION3_QUEST_TRANSITIONS_REVISION_LIMIT"
        }
        Revision3QuestTransitionPlanEditConflictV1::ZeroQuestId
        | Revision3QuestTransitionPlanEditConflictV1::InvalidTransitionPlan { .. } => {
            "AUTHORING_REVISION3_QUEST_TRANSITIONS_REQUEST_REJECTED"
        }
        Revision3QuestTransitionPlanEditConflictV1::CandidateTooLarge { .. } => {
            "AUTHORING_REVISION3_QUEST_TRANSITIONS_PROJECT_LIMIT"
        }
        Revision3QuestTransitionPlanEditConflictV1::InvalidQuestClosure { .. }
        | Revision3QuestTransitionPlanEditConflictV1::OwnedModuleDrift { .. }
        | Revision3QuestTransitionPlanEditConflictV1::TechnicalIdentityChanged
        | Revision3QuestTransitionPlanEditConflictV1::CandidateNotPersistable { .. } => {
            "AUTHORING_REVISION3_QUEST_TRANSITIONS_PROJECT_INVALID"
        }
    };
    Failure::new(code, error.to_string())
}

fn map_store_error(error: WorkingStoreError) -> Failure {
    let code = match error {
        WorkingStoreError::InvalidLimits(_) => {
            "AUTHORING_REVISION3_QUEST_TRANSITIONS_STORE_LIMITS_INVALID"
        }
        WorkingStoreError::MissingRoot(_) => {
            "AUTHORING_REVISION3_QUEST_TRANSITIONS_STORE_ROOT_MISSING"
        }
        WorkingStoreError::UnsafePath { .. } => {
            "AUTHORING_REVISION3_QUEST_TRANSITIONS_STORE_PATH_UNSAFE"
        }
        WorkingStoreError::LimitExceeded { .. } => {
            "AUTHORING_REVISION3_QUEST_TRANSITIONS_STORE_LIMIT"
        }
        WorkingStoreError::HeadConflict { .. } => {
            "AUTHORING_REVISION3_QUEST_TRANSITIONS_HEAD_CONFLICT"
        }
        WorkingStoreError::MissingHead(_) => "AUTHORING_REVISION3_QUEST_TRANSITIONS_HEAD_MISSING",
        WorkingStoreError::MissingObject(_) => {
            "AUTHORING_REVISION3_QUEST_TRANSITIONS_STORE_OBJECT_MISSING"
        }
        WorkingStoreError::SealMismatch { .. } => {
            "AUTHORING_REVISION3_QUEST_TRANSITIONS_STORE_SEAL_MISMATCH"
        }
        WorkingStoreError::Collision { .. } => {
            "AUTHORING_REVISION3_QUEST_TRANSITIONS_STORE_COLLISION"
        }
        WorkingStoreError::InvalidJson { .. } | WorkingStoreError::NonCanonicalJson { .. } => {
            "AUTHORING_REVISION3_QUEST_TRANSITIONS_STORE_JSON_INVALID"
        }
        WorkingStoreError::Invariant(_)
        | WorkingStoreError::InvalidOgg(_)
        | WorkingStoreError::OggMetadataMismatch { .. } => {
            "AUTHORING_REVISION3_QUEST_TRANSITIONS_STORE_INVARIANT"
        }
        WorkingStoreError::StagingCleanup { .. } | WorkingStoreError::Io(_) => {
            "AUTHORING_REVISION3_QUEST_TRANSITIONS_STORE_IO"
        }
    };
    Failure::new(
        code,
        "the revision-3 Quest transitions Store operation failed",
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

    use gore_authoring::{
        regenerate_revision3_quest_module_v2, revision3_quest_transition_plan_seal_v1,
        AssetStoreIndex, ContentSeal, FormatV2, GameGenerationAnchor, ProjectId, ProjectMeta,
        ProjectRevision3, QuestCollisionArtifactRef, QuestCollisionCatalogInput,
        QuestTransitionConditionAtomV1, QuestTransitionConditionGroupV1, QuestTransitionNodeV1,
        QuestTransitionPredicateV1, QuestTransitionStateTestV1, Revision3Entity,
        Revision3EntityKind, Revision3OriginRef, Revision3QuestDraft, Revision3QuestDraftInput,
        Revision3QuestGiverInput, Revision3QuestParentInput, Revision3TypedRef, SchemaRevisionV3,
        Sha256Digest, QUEST_COLLISION_CATALOG_LAYER, REVISION3_QUEST_GENERATOR_ID,
        REVISION3_QUEST_GENERATOR_VERSION,
    };
    use tempfile::TempDir;

    use super::*;

    const QUEST_ID_BYTE: u8 = 0x71;
    const MODULE_ID_BYTE: u8 = 0x72;

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
                name: "Quest transitions FFI fixture".to_owned(),
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
        let basis_project = empty_project(0);
        let basis = store
            .prepare_revision3_checkpoint(None, &basis_project)
            .unwrap();
        fs::write(temp.path().join("gore-project.json"), &basis.head_bytes).unwrap();

        let artifact_bytes = br#"{"format":"fixture-collision-v1"}"#;
        let imported = store
            .import_quest_collision_artifact_v1(artifact_bytes, Some(&basis.head))
            .unwrap();

        let mut project = empty_project(1);
        project
            .asset_store
            .assets
            .insert(imported.artifact.sha256, imported.asset_meta);
        let quest_id = entity_id(QUEST_ID_BYTE);
        let module_id = entity_id(MODULE_ID_BYTE);
        let collision_ref = QuestCollisionArtifactRef {
            generation: target(),
            catalog_layer: QUEST_COLLISION_CATALOG_LAYER.to_owned(),
            artifact: imported.artifact,
            source_seal: seal(0x33, artifact_bytes.len() as u64),
            basis_snapshot: basis.head.snapshot.clone(),
        };
        let quest = Revision3QuestDraft {
            generator_id: REVISION3_QUEST_GENERATOR_ID.to_owned(),
            generator_version: REVISION3_QUEST_GENERATOR_VERSION,
            input: Revision3QuestDraftInput {
                target: target(),
                quest_id,
                module_namespace: "GoreMods.Quests.TransitionsFixture".to_owned(),
                technical_id: "GORE_TRANSITIONS_FIXTURE".to_owned(),
                text_helper: "GoreTransitionsFixtureText".to_owned(),
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
                title: "Transition fixture".to_owned(),
                description: "Transition fixture description".to_owned(),
                objective_title: "Transition fixture objective".to_owned(),
                additional_objective_titles: Vec::new(),
                transition_plan: None,
                collision_catalog: collision_ref,
            },
            script_module: Revision3TypedRef::new(
                project.project_id,
                module_id,
                Revision3EntityKind::ScriptModule,
            ),
        };
        let collision = QuestCollisionCatalogInput {
            generation: quest.input.collision_catalog.generation.clone(),
            source_seal: quest.input.collision_catalog.source_seal.clone(),
            catalog_layer: quest.input.collision_catalog.catalog_layer.clone(),
            modules: BTreeSet::new(),
            relative_paths: BTreeSet::new(),
            symbols: BTreeSet::new(),
        };
        let module = regenerate_revision3_quest_module_v2(&quest, collision).unwrap();
        let owner = Revision3TypedRef::new(
            project.project_id,
            quest_id,
            Revision3EntityKind::QuestDraft,
        );
        project.entities.insert(
            quest_id,
            Revision3Entity {
                id: quest_id,
                display_name: "Transition fixture Quest".to_owned(),
                origin: Revision3OriginRef::New {
                    authored_runtime_id: quest.input.technical_id.clone(),
                },
                revision: 0,
                payload: Revision3EntityPayload::QuestDraft(quest),
            },
        );
        project.entities.insert(
            module_id,
            Revision3Entity {
                id: module_id,
                display_name: "Transition fixture generated module".to_owned(),
                origin: Revision3OriginRef::Generated {
                    generator_id: REVISION3_QUEST_GENERATOR_ID.to_owned(),
                    generator_version: REVISION3_QUEST_GENERATOR_VERSION,
                    owner,
                },
                revision: 0,
                payload: Revision3EntityPayload::ScriptModule(module),
            },
        );
        let project_json = project.to_canonical_json().unwrap();
        let published = store
            .prepare_revision3_checkpoint(Some(&basis.head), &project)
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

    fn edit_request(store: &PublishedStore) -> Revision3QuestTransitionPlanEditRequestV1 {
        edit_request_for(&store.project, &store.head)
    }

    fn edit_request_for(
        project: &ProjectRevision3,
        head: &WorkingHead,
    ) -> Revision3QuestTransitionPlanEditRequestV1 {
        let entity = &project.entities[&entity_id(QUEST_ID_BYTE)];
        let Revision3EntityPayload::QuestDraft(quest) = &entity.payload else {
            panic!("expected Quest")
        };
        let basis = revision3_quest_transition_plan_basis_v1(quest).unwrap();
        let mut transition_plan = basis.plan;
        if let Some(predicate) = &mut transition_plan.transitions[0].predicate {
            predicate.any_of[0].all_of[0].negated = !predicate.any_of[0].all_of[0].negated;
        } else {
            transition_plan.transitions[0].predicate = Some(QuestTransitionPredicateV1 {
                any_of: vec![QuestTransitionConditionGroupV1 {
                    all_of: vec![QuestTransitionConditionAtomV1 {
                        node: QuestTransitionNodeV1::Objective { slot: 1 },
                        test: QuestTransitionStateTestV1::Started,
                        negated: false,
                    }],
                }],
            });
        }
        Revision3QuestTransitionPlanEditRequestV1 {
            expected_head: head.clone(),
            expected_project_id: project.project_id,
            expected_revision: project.revision,
            expected_target: project.target.clone(),
            quest_id: entity_id(QUEST_ID_BYTE),
            expected_quest_revision: entity.revision,
            expected_transition_plan_seal: basis.seal,
            transition_plan,
        }
    }

    fn raw_request(payload: Value) -> String {
        json!({"command": COMMAND, "payload": payload}).to_string()
    }

    fn call(store: &PublishedStore, request: &Revision3QuestTransitionPlanEditRequestV1) -> Value {
        prepare_revision3_quest_transitions_edit_v1_raw(&raw_request(json!({
            "current_project_json": store.project_json,
            "quest_transitions_request_json": request.to_canonical_json().unwrap(),
            "root": store.temp.path(),
        })))
    }

    fn fixed_head(store: &PublishedStore) -> Vec<u8> {
        fs::read(store.temp.path().join("gore-project.json")).unwrap()
    }

    fn snapshot_regular_files(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
        fn visit(root: &Path, path: &Path, output: &mut BTreeMap<PathBuf, Vec<u8>>) {
            for entry in fs::read_dir(path).unwrap() {
                let entry = entry.unwrap();
                let path = entry.path();
                let metadata = fs::symlink_metadata(&path).unwrap();
                if metadata.file_type().is_dir() {
                    visit(root, &path, output);
                } else if metadata.file_type().is_file() {
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
    fn exact_wire_and_public_dispatch_reject_ambiguous_authority() {
        let valid = raw_request(json!({
            "current_project_json": "{}",
            "quest_transitions_request_json": "{}",
            "root": "C:/missing",
        }));
        let parsed: PrepareQuestTransitionsWirePayload = parse_exact_wire(&valid).unwrap();
        assert_eq!(parsed.current_project_json, "{}");

        let cases = [
            format!(
                "{{\"command\":\"{COMMAND}\",\"command\":\"{COMMAND}\",\"payload\":{{\"current_project_json\":\"{{}}\",\"quest_transitions_request_json\":\"{{}}\",\"root\":\"r\"}}}}"
            ),
            format!(
                "{{\"command\":\"{COMMAND}\",\"payload\":{{\"current_project_json\":\"{{}}\",\"quest_transitions_request_json\":\"{{}}\",\"root\":\"r\",\"root\":\"forged\"}}}}"
            ),
            raw_request(json!({
                "current_project_json": "{}", "quest_transitions_request_json": "{}",
                "root": "r", "game_root": "forged"
            })),
            raw_request(json!({
                "quest_transitions_request_json": "{}", "root": "r"
            })),
            raw_request(json!({
                "current_project_json": {}, "quest_transitions_request_json": "{}", "root": "r"
            })),
            format!(" {valid}"),
        ];
        for input in cases {
            assert_eq!(
                prepare_revision3_quest_transitions_edit_v1_raw(&input)["error"]["code"],
                "AUTHORING_REVISION3_QUEST_TRANSITIONS_REQUEST_INVALID",
                "{input}"
            );
            let response: Value = serde_json::from_str(&crate::execute_json(&input)).unwrap();
            assert_eq!(
                response["error"]["code"],
                "AUTHORING_REVISION3_QUEST_TRANSITIONS_REQUEST_INVALID"
            );
        }
    }

    #[test]
    fn nested_request_is_exactly_512_kib_bounded_before_store_open() {
        assert_eq!(MAX_TRANSITIONS_REQUEST_JSON_BYTES, 512 * 1024);
        let valid_shape = || {
            json!({
                "current_project_json": "{}",
                "quest_transitions_request_json": "{}",
                "root": "C:/missing",
            })
        };
        let mut payload = valid_shape();
        payload["root"] = Value::String("x".repeat(MAX_PATH_BYTES + 1));
        assert_eq!(
            prepare_revision3_quest_transitions_edit_v1_raw(&raw_request(payload))["error"]["code"],
            "AUTHORING_REVISION3_QUEST_TRANSITIONS_REQUEST_INVALID"
        );
        let mut payload = valid_shape();
        payload["current_project_json"] = Value::String("x".repeat(MAX_PROJECT_JSON_BYTES + 1));
        assert_eq!(
            prepare_revision3_quest_transitions_edit_v1_raw(&raw_request(payload))["error"]["code"],
            "AUTHORING_REVISION3_QUEST_TRANSITIONS_PROJECT_LIMIT"
        );
        let mut payload = valid_shape();
        payload["quest_transitions_request_json"] =
            Value::String("x".repeat(MAX_TRANSITIONS_REQUEST_JSON_BYTES + 1));
        assert_eq!(
            prepare_revision3_quest_transitions_edit_v1_raw(&raw_request(payload))["error"]["code"],
            "AUTHORING_REVISION3_QUEST_TRANSITIONS_REQUEST_LIMIT"
        );
        assert_eq!(
            prepare_revision3_quest_transitions_edit_v1_raw(&" ".repeat(MAX_WIRE_BYTES + 1))
                ["error"]["code"],
            "AUTHORING_REVISION3_QUEST_TRANSITIONS_INPUT_LIMIT"
        );

        let store = published_store();
        let mut request = edit_request(&store);
        request.expected_revision = u64::MAX;
        assert_eq!(
            call(&store, &request)["error"]["code"],
            "AUTHORING_REVISION3_QUEST_TRANSITIONS_SIGNED_WIRE_LIMIT"
        );
        assert_eq!(fixed_head(&store), store.fixed_head_bytes);
    }

    #[test]
    fn legacy_happy_path_fully_reopens_v4_candidate_without_publishing() {
        let store = published_store();
        let request = edit_request(&store);
        let response = call(&store, &request);
        assert_eq!(response["ok"], true, "{response}");
        assert_eq!(response["outcome"], "prepared_unpublished");
        assert_eq!(response["project_id"], store.project.project_id.to_string());
        assert_eq!(response["revision"], 2);
        assert_eq!(response["quest_id"], entity_id(QUEST_ID_BYTE).to_string());
        assert_eq!(response["module_id"], entity_id(MODULE_ID_BYTE).to_string());
        assert_eq!(response["quest_revision"], 1);
        assert_eq!(response["module_revision"], 1);
        assert_eq!(response["previous_generator_version"], 2);
        assert_eq!(response["upgraded_from_legacy"], true);
        assert_eq!(response["build_status"], "blocked");
        assert_eq!(response["runtime_status"], "runtime_unqualified");
        assert_eq!(response["publication_status"], "not_supported");
        assert_eq!(fixed_head(&store), store.fixed_head_bytes);
        assert_eq!(
            response["basis_head_json"],
            serde_json::to_string(&store.head).unwrap()
        );

        let reopened = WorkingProjectStore::open_existing(store.temp.path(), ffi_store_limits())
            .unwrap()
            .open_revision3_head_bytes(
                response["head_json"].as_str().unwrap().as_bytes(),
                AssetVerification::Full,
            )
            .unwrap();
        assert_eq!(
            reopened.project.to_canonical_json().unwrap(),
            response["project_json"]
        );
        let quest = &reopened.project.entities[&entity_id(QUEST_ID_BYTE)];
        assert_eq!(quest.revision, 1);
        let Revision3EntityPayload::QuestDraft(quest) = &quest.payload else {
            panic!("expected Quest")
        };
        assert_eq!(
            quest.generator_version,
            REVISION3_SEMANTIC_QUEST_GENERATOR_VERSION
        );
        assert_eq!(
            quest.input.transition_plan.as_deref(),
            Some(&request.transition_plan)
        );
        let reopened_basis = revision3_quest_transition_plan_basis_v1(quest).unwrap();
        assert!(!reopened_basis.legacy_synthetic);
        assert_eq!(
            response["transition_plan_seal"],
            serde_json::to_value(&reopened_basis.seal).unwrap()
        );
        assert_eq!(
            reopened_basis.seal,
            revision3_quest_transition_plan_seal_v1(&request.transition_plan).unwrap()
        );

        let encoded = response.to_string();
        assert!(!encoded.contains(store.temp.path().to_string_lossy().as_ref()));
        assert!(!encoded.contains("game_root"));
        assert!(!encoded.contains("deploy"));
        assert_ne!(response["publication_status"], "published");
    }

    #[test]
    fn semantic_v4_edit_reports_retained_plan_and_rejects_noop_without_writes() {
        let store = published_store();
        let first = call(&store, &edit_request(&store));
        assert_eq!(first["ok"], true, "{first}");
        let first_head_bytes = first["head_json"].as_str().unwrap().as_bytes().to_vec();
        fs::write(
            store.temp.path().join("gore-project.json"),
            &first_head_bytes,
        )
        .unwrap();

        let working =
            WorkingProjectStore::open_existing(store.temp.path(), ffi_store_limits()).unwrap();
        let current = working
            .open_current_revision3(AssetVerification::Full)
            .unwrap();
        let entity = &current.project.entities[&entity_id(QUEST_ID_BYTE)];
        let Revision3EntityPayload::QuestDraft(quest) = &entity.payload else {
            panic!("expected Quest")
        };
        let current_basis = revision3_quest_transition_plan_basis_v1(quest).unwrap();
        assert!(!current_basis.legacy_synthetic);
        let noop = Revision3QuestTransitionPlanEditRequestV1 {
            expected_head: current.head.clone(),
            expected_project_id: current.project.project_id,
            expected_revision: current.project.revision,
            expected_target: current.project.target.clone(),
            quest_id: entity_id(QUEST_ID_BYTE),
            expected_quest_revision: entity.revision,
            expected_transition_plan_seal: current_basis.seal,
            transition_plan: current_basis.plan,
        };
        let before_noop = snapshot_regular_files(store.temp.path());
        let noop_response = prepare_revision3_quest_transitions_edit_v1_raw(&raw_request(json!({
            "current_project_json": current.project.to_canonical_json().unwrap(),
            "quest_transitions_request_json": noop.to_canonical_json().unwrap(),
            "root": store.temp.path(),
        })));
        assert_eq!(
            noop_response["error"]["code"], "AUTHORING_REVISION3_QUEST_TRANSITIONS_NO_CHANGES",
            "{noop_response}"
        );
        assert_eq!(snapshot_regular_files(store.temp.path()), before_noop);

        let request = edit_request_for(&current.project, &current.head);
        let response = prepare_revision3_quest_transitions_edit_v1_raw(&raw_request(json!({
            "current_project_json": current.project.to_canonical_json().unwrap(),
            "quest_transitions_request_json": request.to_canonical_json().unwrap(),
            "root": store.temp.path(),
        })));
        assert_eq!(response["ok"], true, "{response}");
        assert_eq!(response["revision"], 3);
        assert_eq!(response["quest_revision"], 2);
        assert_eq!(response["module_revision"], 2);
        assert_eq!(
            response["previous_generator_version"],
            REVISION3_SEMANTIC_QUEST_GENERATOR_VERSION
        );
        assert_eq!(response["upgraded_from_legacy"], false);
        assert_eq!(fixed_head(&store), first_head_bytes);
    }

    #[test]
    fn stale_project_head_target_quest_and_plan_cas_write_nothing() {
        let store = published_store();
        let before = snapshot_regular_files(store.temp.path());
        let base = edit_request(&store);

        let response = prepare_revision3_quest_transitions_edit_v1_raw(&raw_request(json!({
            "current_project_json": store.project_json.replacen("\"revision\":1", "\"revision\":0", 1),
            "quest_transitions_request_json": base.to_canonical_json().unwrap(),
            "root": store.temp.path(),
        })));
        assert_eq!(
            response["error"]["code"],
            "AUTHORING_REVISION3_QUEST_TRANSITIONS_PROJECT_CONFLICT"
        );

        let mut stale_head = base.clone();
        stale_head.expected_head.snapshot.byte_len += 1;
        assert_eq!(
            call(&store, &stale_head)["error"]["code"],
            "AUTHORING_REVISION3_QUEST_TRANSITIONS_HEAD_CONFLICT"
        );
        let mut wrong_target = base.clone();
        wrong_target.expected_target.executable.sha256 = Sha256Digest::from_bytes([0xee; 32]);
        assert_eq!(
            call(&store, &wrong_target)["error"]["code"],
            "AUTHORING_REVISION3_QUEST_TRANSITIONS_TARGET_CONFLICT"
        );
        let mut stale_quest = base.clone();
        stale_quest.expected_quest_revision = 1;
        assert_eq!(
            call(&store, &stale_quest)["error"]["code"],
            "AUTHORING_REVISION3_QUEST_TRANSITIONS_QUEST_CONFLICT"
        );
        let mut stale_plan = base;
        stale_plan.expected_transition_plan_seal.byte_len += 1;
        assert_eq!(
            call(&store, &stale_plan)["error"]["code"],
            "AUTHORING_REVISION3_QUEST_TRANSITIONS_TRANSITION_PLAN_CONFLICT"
        );
        assert_eq!(fixed_head(&store), store.fixed_head_bytes);
        assert_eq!(snapshot_regular_files(store.temp.path()), before);
    }

    #[test]
    fn malformed_nested_request_is_rejected_before_opening_the_store() {
        let temp = tempfile::tempdir().unwrap();
        let missing_root = temp.path().join("missing-store");
        for nested in [
            "{}".to_owned(),
            " {\"expected_head\":null}".to_owned(),
            "{\"quest_id\":1,\"quest_id\":2}".to_owned(),
        ] {
            let response = prepare_revision3_quest_transitions_edit_v1_raw(&raw_request(json!({
                "current_project_json": "{}",
                "quest_transitions_request_json": nested,
                "root": missing_root,
            })));
            assert_eq!(
                response["error"]["code"],
                "AUTHORING_REVISION3_QUEST_TRANSITIONS_REQUEST_INVALID"
            );
        }
        assert!(!missing_root.exists());
    }

    #[test]
    fn late_external_head_race_fails_closed_and_preserves_racing_head() {
        let store = published_store();
        let working =
            WorkingProjectStore::open_existing(store.temp.path(), ffi_store_limits()).unwrap();
        let mut rival = store.project.clone();
        rival.meta.name = "External publisher won".to_owned();
        let rival = working
            .prepare_revision3_checkpoint(Some(&store.head), &rival)
            .unwrap();
        let request = edit_request(&store);
        let wire = raw_request(json!({
            "current_project_json": store.project_json,
            "quest_transitions_request_json": request.to_canonical_json().unwrap(),
            "root": store.temp.path(),
        }));
        let response =
            prepare_revision3_quest_transitions_edit_v1_inner_with_final_guard(&wire, || {
                fs::write(
                    store.temp.path().join("gore-project.json"),
                    &rival.head_bytes,
                )
                .unwrap();
            })
            .unwrap_err()
            .response();
        assert_eq!(
            response["error"]["code"],
            "AUTHORING_REVISION3_QUEST_TRANSITIONS_HEAD_CONFLICT"
        );
        assert_eq!(fixed_head(&store), rival.head_bytes);
    }

    #[test]
    fn shape_regressions_map_to_transition_plan_conflict() {
        let store = published_store();
        let before = snapshot_regular_files(store.temp.path());
        let mut changed_slots = edit_request(&store);
        changed_slots.transition_plan.objective_slots.push(2);
        assert_eq!(
            call(&store, &changed_slots)["error"]["code"],
            "AUTHORING_REVISION3_QUEST_TRANSITIONS_TRANSITION_PLAN_CONFLICT"
        );
        let mut ordinal_regression = edit_request(&store);
        ordinal_regression.transition_plan.next_slot_ordinal = 1;
        assert_eq!(
            call(&store, &ordinal_regression)["error"]["code"],
            "AUTHORING_REVISION3_QUEST_TRANSITIONS_TRANSITION_PLAN_CONFLICT"
        );
        assert_eq!(fixed_head(&store), store.fixed_head_bytes);
        assert_eq!(snapshot_regular_files(store.temp.path()), before);

        assert_eq!(
            map_transaction_conflict(
                Revision3QuestTransitionPlanEditConflictV1::ObjectiveSlotsChanged
            )
            .code,
            "AUTHORING_REVISION3_QUEST_TRANSITIONS_TRANSITION_PLAN_CONFLICT"
        );
        assert_eq!(
            map_transaction_conflict(
                Revision3QuestTransitionPlanEditConflictV1::NextSlotOrdinalRegression {
                    current: 3,
                    requested: 2,
                }
            )
            .code,
            "AUTHORING_REVISION3_QUEST_TRANSITIONS_TRANSITION_PLAN_CONFLICT"
        );
        assert_eq!(
            map_transaction_conflict(Revision3QuestTransitionPlanEditConflictV1::NoChanges).code,
            "AUTHORING_REVISION3_QUEST_TRANSITIONS_NO_CHANGES"
        );
    }

    #[test]
    fn linked_store_root_is_rejected_without_following_it() {
        let store = published_store();
        let parent = TempDir::new().unwrap();
        let alias = parent.path().join("alias");
        if !make_test_dir_link(store.temp.path(), &alias) {
            return;
        }
        let request = edit_request(&store);
        let response = prepare_revision3_quest_transitions_edit_v1_raw(&raw_request(json!({
            "current_project_json": store.project_json,
            "quest_transitions_request_json": request.to_canonical_json().unwrap(),
            "root": alias,
        })));
        assert_eq!(
            response["error"]["code"],
            "AUTHORING_REVISION3_QUEST_TRANSITIONS_STORE_PATH_UNSAFE"
        );
        assert_eq!(fixed_head(&store), store.fixed_head_bytes);
        #[cfg(unix)]
        fs::remove_file(alias).unwrap();
        #[cfg(windows)]
        fs::remove_dir(alias).unwrap();
    }
}
