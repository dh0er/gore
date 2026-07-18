//! Native, prepare-only orchestration for one stable-slot-aware revision-3 Quest outline edit.
//!
//! The caller supplies only exact canonical project/request transports and one working Store.
//! Native code fully reopens the fixed head, binds every project/head/target/Quest/module/plan CAS
//! token, preflights the exact-current Quest closure, evaluates the filesystem-free transaction,
//! prepares immutable candidate objects, and returns them only after a full reopen and final
//! fixed-head check. This route accepts no game root and never compiles, builds, deploys, writes a
//! save, or publishes `gore-project.json`.

use std::path::Path;

use gore_authoring::{
    apply_revision3_quest_outline_edit_transaction_v2, revision3_quest_transition_plan_basis_v1,
    AssetVerification, EntityId, OpenedRevision3Checkpoint, Revision3EntityPayload,
    Revision3QuestCollisionSourceErrorV2, Revision3QuestOutlineEditBuildStatusV2,
    Revision3QuestOutlineEditConflictV2, Revision3QuestOutlineEditErrorV2,
    Revision3QuestOutlineEditEvaluationV2, Revision3QuestOutlineEditOutcomeV2,
    Revision3QuestOutlineEditPublicationStatusV2, Revision3QuestOutlineEditRequestV2,
    Revision3QuestOutlineEditRuntimeStatusV2, Revision3QuestTransitionPlanBasisErrorV1,
    Revision3QuestTransitionPlanBasisV1, WorkingHead, WorkingProjectStore, WorkingStoreError,
    WorkingStoreLimits, MAX_PROJECT_JSON_BYTES,
    MAX_REVISION3_QUEST_OUTLINE_EDIT_REQUEST_JSON_BYTES_V2,
    REVISION3_SEMANTIC_QUEST_GENERATOR_VERSION,
};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::err;

pub(super) const COMMAND: &str = "authoring_store_prepare_revision3_quest_outline_edit_v2";

const MAX_PATH_BYTES: usize = 32 * 1024;
const MAX_HEAD_JSON_BYTES: usize = 64 * 1024;
const MAX_RESPONSE_BYTES: usize = 64 * 1024 * 1024;
const MAX_ERROR_MESSAGE_BYTES: usize = 4 * 1024;
const MAX_BASIS_REVISION: u64 = i64::MAX as u64 - 1;
const MAX_OUTLINE_REQUEST_JSON_BYTES: usize = 32 * 1024;
const _: () = assert!(
    MAX_REVISION3_QUEST_OUTLINE_EDIT_REQUEST_JSON_BYTES_V2 == MAX_OUTLINE_REQUEST_JSON_BYTES
);
// Nested canonical JSON strings add at most one escape byte per source byte. The Store path is
// arbitrary caller text and retains the conservative six-byte JSON escape allowance.
const MAX_WIRE_BYTES: usize =
    MAX_PROJECT_JSON_BYTES * 2 + MAX_OUTLINE_REQUEST_JSON_BYTES * 2 + MAX_PATH_BYTES * 6 + 4 * 1024;

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ExactWireRequest<P> {
    command: String,
    payload: P,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct PrepareQuestOutlineV2WirePayload {
    current_project_json: String,
    quest_outline_request_json: String,
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

pub(super) fn prepare_revision3_quest_outline_edit_v2_raw(input: &str) -> Value {
    prepare_revision3_quest_outline_edit_v2_inner(input).unwrap_or_else(Failure::response)
}

fn prepare_revision3_quest_outline_edit_v2_inner(input: &str) -> Result<Value, Failure> {
    prepare_revision3_quest_outline_edit_v2_inner_with_test_seams(input, || {}, || {})
}

#[cfg(test)]
fn prepare_revision3_quest_outline_edit_v2_inner_with_final_guard<F>(
    input: &str,
    final_guard: F,
) -> Result<Value, Failure>
where
    F: FnOnce(),
{
    prepare_revision3_quest_outline_edit_v2_inner_with_test_seams(input, || {}, final_guard)
}

fn prepare_revision3_quest_outline_edit_v2_inner_with_test_seams<B, F>(
    input: &str,
    before_checkpoint: B,
    final_guard: F,
) -> Result<Value, Failure>
where
    B: FnOnce(),
    F: FnOnce(),
{
    let payload: PrepareQuestOutlineV2WirePayload = parse_exact_wire(input)?;
    validate_payload(&payload)?;
    // Parse and inspect the bounded nested transport before opening any caller-selected Store.
    let request =
        Revision3QuestOutlineEditRequestV2::from_json(&payload.quest_outline_request_json)
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
        store_invariant("the exact current revision-3 project could not be serialized canonically")
    })?;
    if canonical_basis.as_bytes() != payload.current_project_json.as_bytes() {
        return Err(Failure::new(
            "AUTHORING_REVISION3_QUEST_OUTLINE_V2_PROJECT_CONFLICT",
            "current_project_json differs from the exact published revision-3 project",
        ));
    }

    let bound = bind_request_to_basis(&basis.head, &basis.project, &request)?;

    // This opaque capsule is exact-current closure and full referenced-asset preflight evidence.
    // It is dropped here and never exposed or converted into reusable collision authority.
    let source = store
        .prepare_current_revision3_quest_collision_source_v2(&basis.head)
        .map_err(map_current_source_error)?;
    if source.current_head() != &basis.head
        || source.project_id() != basis.project.project_id
        || source.project_revision() != basis.project.revision
        || source.target() != &basis.project.target
    {
        return Err(store_invariant(
            "the exact-current Quest source preflight changed its Store binding",
        ));
    }
    drop(source);

    let outcome = match apply_revision3_quest_outline_edit_transaction_v2(
        &basis.head,
        &payload.current_project_json,
        &payload.quest_outline_request_json,
    )
    .map_err(map_transaction_error)?
    {
        Revision3QuestOutlineEditEvaluationV2::Applied(outcome) => *outcome,
        Revision3QuestOutlineEditEvaluationV2::Rejected(rejection) => {
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
        return Err(store_invariant(
            "the prepared Quest outline-v2 checkpoint did not fully reopen exactly",
        ));
    }
    let reopened_json = reopened.project.to_canonical_json().map_err(|_| {
        store_invariant("the fully reopened Quest outline-v2 candidate could not be serialized")
    })?;
    if reopened_json != outcome.canonical_project_json {
        return Err(store_invariant(
            "the fully reopened Quest outline-v2 candidate changed canonical bytes",
        ));
    }
    verify_reopened_outline(&reopened.project, &request, &bound, &outcome)?;

    // A concurrent publisher may leave immutable candidate objects only; it can never make this
    // route report success against a changed fixed head.
    final_guard();
    require_fixed_basis(&store, &basis.head, &basis.project)?;

    let basis_head_json = canonical_head_json(&basis.head)?;
    let candidate_head_json = String::from_utf8(prepared.head_bytes)
        .map_err(|_| store_invariant("the prepared Quest outline-v2 head is not UTF-8 JSON"))?;
    if candidate_head_json.is_empty() || candidate_head_json.len() > MAX_HEAD_JSON_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_QUEST_OUTLINE_V2_RESPONSE_LIMIT",
            "the prepared Quest outline-v2 head exceeds its bounded transport limit",
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
            "AUTHORING_REVISION3_QUEST_OUTLINE_V2_INPUT_LIMIT",
            format!(
                "revision-3 Quest outline-v2 request exceeds the {MAX_WIRE_BYTES}-byte wire limit"
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
            "AUTHORING_REVISION3_QUEST_OUTLINE_V2_INVARIANT",
            "the Quest outline-v2 outer request could not be serialized",
        )
    })?;
    if canonical != input {
        return Err(invalid_request());
    }
    Ok(request.payload)
}

fn validate_payload(payload: &PrepareQuestOutlineV2WirePayload) -> Result<(), Failure> {
    validate_path(&payload.root)?;
    if payload.current_project_json.is_empty() {
        return Err(invalid_request());
    }
    if payload.current_project_json.len() > MAX_PROJECT_JSON_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_QUEST_OUTLINE_V2_PROJECT_LIMIT",
            format!("current_project_json exceeds the {MAX_PROJECT_JSON_BYTES}-byte limit"),
        ));
    }
    if payload.quest_outline_request_json.is_empty() {
        return Err(invalid_request());
    }
    if payload.quest_outline_request_json.len() > MAX_OUTLINE_REQUEST_JSON_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_QUEST_OUTLINE_V2_REQUEST_LIMIT",
            format!(
                "quest_outline_request_json exceeds the {MAX_OUTLINE_REQUEST_JSON_BYTES}-byte limit"
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
    request: &Revision3QuestOutlineEditRequestV2,
) -> Result<BoundQuestBasis, Failure> {
    if request.expected_head != *head {
        return Err(head_conflict());
    }
    if request.expected_project_id != project.project_id
        || request.expected_revision != project.revision
    {
        return Err(Failure::new(
            "AUTHORING_REVISION3_QUEST_OUTLINE_V2_PROJECT_CONFLICT",
            "the Quest outline-v2 request project differs from the exact published project",
        ));
    }
    if request.expected_target != project.target {
        return Err(Failure::new(
            "AUTHORING_REVISION3_QUEST_OUTLINE_V2_TARGET_CONFLICT",
            "the Quest outline-v2 request target differs from the exact published project target",
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
    if request.expected_script_module_id != module_id {
        return Err(module_conflict(
            "the expected ScriptModule ID differs from the exact Quest-owned module",
        ));
    }
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
    if request.expected_script_module_revision != module.revision {
        return Err(module_conflict(
            "the expected ScriptModule revision differs from the exact published module",
        ));
    }
    validate_entity_revision(module.revision)?;
    let transition =
        revision3_quest_transition_plan_basis_v1(quest).map_err(map_transition_basis_error)?;
    if transition.legacy_synthetic
        || transition.generator_version != REVISION3_SEMANTIC_QUEST_GENERATOR_VERSION
    {
        return Err(Failure::new(
            "AUTHORING_REVISION3_QUEST_OUTLINE_V2_REQUIRES_SEMANTIC_QUEST",
            "Quest outline v2 requires an exact retained generator-v4 transition plan",
        ));
    }
    if request.expected_transition_plan_seal != transition.seal {
        return Err(plan_conflict(
            "the expected transition-plan seal differs from the exact retained Quest plan",
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
    request: &Revision3QuestOutlineEditRequestV2,
    bound: &BoundQuestBasis,
    outcome: &Revision3QuestOutlineEditOutcomeV2,
) -> Result<(), Failure> {
    if outcome.basis_head != basis.head
        || outcome.quest_id != request.quest_id
        || outcome.project.project_id != basis.project.project_id
        || outcome.project.target != basis.project.target
        || outcome.project.revision != basis.project.revision + 1
        || outcome.script_module_id != bound.module_id
        || outcome.quest_revision != request.expected_quest_revision + 1
        || outcome.script_module_revision != bound.module_revision + 1
    {
        return Err(Failure::new(
            "AUTHORING_REVISION3_QUEST_OUTLINE_V2_INVARIANT",
            "the Quest outline-v2 transaction changed its exact project/request binding",
        ));
    }
    match outcome.build_status {
        Revision3QuestOutlineEditBuildStatusV2::Blocked => {}
    }
    match outcome.runtime_status {
        Revision3QuestOutlineEditRuntimeStatusV2::RuntimeUnqualified => {}
    }
    match outcome.publication_status {
        Revision3QuestOutlineEditPublicationStatusV2::NotSupported => {}
    }
    Ok(())
}

fn verify_reopened_outline(
    project: &gore_authoring::ProjectRevision3,
    request: &Revision3QuestOutlineEditRequestV2,
    bound: &BoundQuestBasis,
    outcome: &Revision3QuestOutlineEditOutcomeV2,
) -> Result<(), Failure> {
    let Some(entity) = project.entities.get(&request.quest_id) else {
        return Err(store_invariant(
            "the fully reopened Quest outline-v2 candidate lost its Quest",
        ));
    };
    let Revision3EntityPayload::QuestDraft(quest) = &entity.payload else {
        return Err(store_invariant(
            "the fully reopened Quest outline-v2 candidate changed Quest kind",
        ));
    };
    let transition =
        revision3_quest_transition_plan_basis_v1(quest).map_err(map_transition_basis_error)?;
    let requested_order = request
        .objectives
        .iter()
        .map(|objective| objective.slot)
        .collect::<Vec<_>>();
    let requested_titles = request
        .objectives
        .iter()
        .map(|objective| objective.title.as_str())
        .collect::<Vec<_>>();
    let reopened_titles = std::iter::once(quest.input.objective_title.as_str())
        .chain(
            quest
                .input
                .additional_objective_titles
                .iter()
                .map(String::as_str),
        )
        .collect::<Vec<_>>();
    if entity.revision != outcome.quest_revision
        || entity.display_name != request.display_name
        || quest.input.title != request.quest_title
        || transition.legacy_synthetic
        || transition.plan.objective_order != requested_order
        || reopened_titles != requested_titles
        || transition.plan.objective_slots != bound.transition.plan.objective_slots
        || transition.plan.next_slot_ordinal != bound.transition.plan.next_slot_ordinal
        || transition.plan.transitions != bound.transition.plan.transitions
        || transition.seal != outcome.transition_plan_seal
    {
        return Err(store_invariant(
            "the fully reopened Quest outline-v2 state differs from the prepared outcome",
        ));
    }
    let Some(module) = project.entities.get(&outcome.script_module_id) else {
        return Err(store_invariant(
            "the fully reopened Quest outline-v2 candidate lost its ScriptModule",
        ));
    };
    if module.revision != outcome.script_module_revision
        || outcome.script_module_id != request.expected_script_module_id
        || !matches!(module.payload, Revision3EntityPayload::ScriptModule(_))
    {
        return Err(store_invariant(
            "the fully reopened Quest outline-v2 ScriptModule differs from the prepared outcome",
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
            "AUTHORING_REVISION3_QUEST_OUTLINE_V2_INVARIANT",
            "a Quest outline-v2 wire value could not be inspected",
        )
    })?;
    require_signed_json_value(&value)
}

fn require_signed_json_value(value: &Value) -> Result<(), Failure> {
    match value {
        Value::Number(number) if number.as_u64().is_some_and(|value| value > i64::MAX as u64) => {
            Err(Failure::new(
                "AUTHORING_REVISION3_QUEST_OUTLINE_V2_SIGNED_WIRE_LIMIT",
                "a Quest outline-v2 wire integer exceeds the signed 64-bit transport range",
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
            "AUTHORING_REVISION3_QUEST_OUTLINE_V2_INVARIANT",
            "the Quest outline-v2 basis head could not be serialized",
        )
    })?;
    if value.is_empty() || value.len() > MAX_HEAD_JSON_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_QUEST_OUTLINE_V2_RESPONSE_LIMIT",
            "the Quest outline-v2 basis head exceeds its bounded transport limit",
        ));
    }
    Ok(value)
}

fn enforce_response_budget(response: &Value) -> Result<(), Failure> {
    let encoded = serde_json::to_vec(response).map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_QUEST_OUTLINE_V2_INVARIANT",
            "the Quest outline-v2 response could not be serialized",
        )
    })?;
    if encoded.len() > MAX_RESPONSE_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_QUEST_OUTLINE_V2_RESPONSE_LIMIT",
            "the Quest outline-v2 response exceeds its bounded transport budget",
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
        "AUTHORING_REVISION3_QUEST_OUTLINE_V2_REQUEST_INVALID",
        "request must contain one exact duplicate-free command and exactly current_project_json, quest_outline_request_json, and root",
    )
}

fn head_conflict() -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_QUEST_OUTLINE_V2_HEAD_CONFLICT",
        "the published revision-3 head changed or differs from the Quest outline-v2 request",
    )
}

fn quest_conflict(message: &'static str) -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_QUEST_OUTLINE_V2_QUEST_CONFLICT",
        message,
    )
}

fn module_conflict(message: &'static str) -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_QUEST_OUTLINE_V2_MODULE_CONFLICT",
        message,
    )
}

fn plan_conflict(message: &'static str) -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_QUEST_OUTLINE_V2_PLAN_CONFLICT",
        message,
    )
}

fn project_invalid(message: &'static str) -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_QUEST_OUTLINE_V2_PROJECT_INVALID",
        message,
    )
}

fn store_invariant(message: &'static str) -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_QUEST_OUTLINE_V2_STORE_INVARIANT",
        message,
    )
}

fn revision_limit() -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_QUEST_OUTLINE_V2_REVISION_LIMIT",
        format!("a Quest outline-v2 basis revision exceeds {MAX_BASIS_REVISION}"),
    )
}

fn map_request_error(error: impl std::fmt::Display) -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_QUEST_OUTLINE_V2_REQUEST_INVALID",
        format!("the exact Quest outline-v2 request is invalid: {error}"),
    )
}

fn map_transition_basis_error(_error: Revision3QuestTransitionPlanBasisErrorV1) -> Failure {
    project_invalid("the exact current Quest has an invalid transition-plan basis")
}

fn map_transaction_error(error: Revision3QuestOutlineEditErrorV2) -> Failure {
    match error {
        Revision3QuestOutlineEditErrorV2::InvalidProject(_) => project_invalid(
            "the exact current revision-3 project is not a valid Quest-outline-v2 basis",
        ),
        Revision3QuestOutlineEditErrorV2::InvalidRequest(error) => map_request_error(error),
        Revision3QuestOutlineEditErrorV2::ReopenCandidate(_)
        | Revision3QuestOutlineEditErrorV2::CanonicalReopenMismatch => Failure::new(
            "AUTHORING_REVISION3_QUEST_OUTLINE_V2_INVARIANT",
            "the Quest outline-v2 candidate failed exact canonical reopen",
        ),
    }
}

fn map_transaction_conflict(error: Revision3QuestOutlineEditConflictV2) -> Failure {
    let code = match &error {
        Revision3QuestOutlineEditConflictV2::CurrentHeadMismatch => {
            "AUTHORING_REVISION3_QUEST_OUTLINE_V2_HEAD_CONFLICT"
        }
        Revision3QuestOutlineEditConflictV2::ProjectIdentityMismatch { .. }
        | Revision3QuestOutlineEditConflictV2::ProjectRevisionConflict { .. } => {
            "AUTHORING_REVISION3_QUEST_OUTLINE_V2_PROJECT_CONFLICT"
        }
        Revision3QuestOutlineEditConflictV2::ProjectTargetMismatch => {
            "AUTHORING_REVISION3_QUEST_OUTLINE_V2_TARGET_CONFLICT"
        }
        Revision3QuestOutlineEditConflictV2::InvalidQuestEntity { .. }
        | Revision3QuestOutlineEditConflictV2::QuestRevisionConflict { .. } => {
            "AUTHORING_REVISION3_QUEST_OUTLINE_V2_QUEST_CONFLICT"
        }
        Revision3QuestOutlineEditConflictV2::ZeroExpectedScriptModuleId
        | Revision3QuestOutlineEditConflictV2::ScriptModuleIdentityConflict { .. }
        | Revision3QuestOutlineEditConflictV2::ScriptModuleRevisionConflict { .. } => {
            "AUTHORING_REVISION3_QUEST_OUTLINE_V2_MODULE_CONFLICT"
        }
        Revision3QuestOutlineEditConflictV2::TransitionPlanSealConflict { .. } => {
            "AUTHORING_REVISION3_QUEST_OUTLINE_V2_PLAN_CONFLICT"
        }
        Revision3QuestOutlineEditConflictV2::InvalidObjectiveCount { .. }
        | Revision3QuestOutlineEditConflictV2::ObjectiveCountChange { .. }
        | Revision3QuestOutlineEditConflictV2::DuplicateObjectiveSlot { .. }
        | Revision3QuestOutlineEditConflictV2::ForeignObjectiveSlot { .. }
        | Revision3QuestOutlineEditConflictV2::MissingObjectiveSlot { .. } => {
            "AUTHORING_REVISION3_QUEST_OUTLINE_V2_SLOT_CONFLICT"
        }
        Revision3QuestOutlineEditConflictV2::SemanticQuestRequired { .. } => {
            "AUTHORING_REVISION3_QUEST_OUTLINE_V2_REQUIRES_SEMANTIC_QUEST"
        }
        Revision3QuestOutlineEditConflictV2::NoChanges => {
            "AUTHORING_REVISION3_QUEST_OUTLINE_V2_NO_CHANGES"
        }
        Revision3QuestOutlineEditConflictV2::ProjectRevisionOverflow
        | Revision3QuestOutlineEditConflictV2::QuestRevisionOverflow { .. }
        | Revision3QuestOutlineEditConflictV2::ScriptModuleRevisionOverflow { .. } => {
            "AUTHORING_REVISION3_QUEST_OUTLINE_V2_REVISION_LIMIT"
        }
        Revision3QuestOutlineEditConflictV2::ZeroQuestId
        | Revision3QuestOutlineEditConflictV2::InvalidDisplayName
        | Revision3QuestOutlineEditConflictV2::InvalidObjectiveTitles { .. }
        | Revision3QuestOutlineEditConflictV2::InvalidOutlineText { .. } => {
            "AUTHORING_REVISION3_QUEST_OUTLINE_V2_REQUEST_REJECTED"
        }
        Revision3QuestOutlineEditConflictV2::CandidateTooLarge { .. } => {
            "AUTHORING_REVISION3_QUEST_OUTLINE_V2_PROJECT_LIMIT"
        }
        Revision3QuestOutlineEditConflictV2::InvalidQuestClosure { .. }
        | Revision3QuestOutlineEditConflictV2::OwnedModuleDrift { .. }
        | Revision3QuestOutlineEditConflictV2::TechnicalIdentityChanged
        | Revision3QuestOutlineEditConflictV2::CandidateNotPersistable { .. } => {
            "AUTHORING_REVISION3_QUEST_OUTLINE_V2_PROJECT_INVALID"
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
            "AUTHORING_REVISION3_QUEST_OUTLINE_V2_STORE_LIMIT",
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
        | Revision3QuestCollisionSourceErrorV2::NonQuestBasisInvalid { .. } => project_invalid(
            "the exact current revision-3 project is not a valid Quest-outline-v2 basis",
        ),
    }
}

fn map_store_error(error: WorkingStoreError) -> Failure {
    let code = match error {
        WorkingStoreError::InvalidLimits(_) => {
            "AUTHORING_REVISION3_QUEST_OUTLINE_V2_STORE_LIMITS_INVALID"
        }
        WorkingStoreError::MissingRoot(_) => {
            "AUTHORING_REVISION3_QUEST_OUTLINE_V2_STORE_ROOT_MISSING"
        }
        WorkingStoreError::UnsafePath { .. } => {
            "AUTHORING_REVISION3_QUEST_OUTLINE_V2_STORE_PATH_UNSAFE"
        }
        WorkingStoreError::LimitExceeded { .. } => {
            "AUTHORING_REVISION3_QUEST_OUTLINE_V2_STORE_LIMIT"
        }
        WorkingStoreError::HeadConflict { .. } => {
            "AUTHORING_REVISION3_QUEST_OUTLINE_V2_HEAD_CONFLICT"
        }
        WorkingStoreError::MissingHead(_) => "AUTHORING_REVISION3_QUEST_OUTLINE_V2_HEAD_MISSING",
        WorkingStoreError::MissingObject(_) => {
            "AUTHORING_REVISION3_QUEST_OUTLINE_V2_STORE_OBJECT_MISSING"
        }
        WorkingStoreError::SealMismatch { .. } => {
            "AUTHORING_REVISION3_QUEST_OUTLINE_V2_STORE_SEAL_MISMATCH"
        }
        WorkingStoreError::Collision { .. } => {
            "AUTHORING_REVISION3_QUEST_OUTLINE_V2_STORE_COLLISION"
        }
        WorkingStoreError::InvalidJson { .. } | WorkingStoreError::NonCanonicalJson { .. } => {
            "AUTHORING_REVISION3_QUEST_OUTLINE_V2_STORE_JSON_INVALID"
        }
        WorkingStoreError::Invariant(_)
        | WorkingStoreError::InvalidOgg(_)
        | WorkingStoreError::OggMetadataMismatch { .. } => {
            "AUTHORING_REVISION3_QUEST_OUTLINE_V2_STORE_INVARIANT"
        }
        WorkingStoreError::StagingCleanup { .. } | WorkingStoreError::Io(_) => {
            "AUTHORING_REVISION3_QUEST_OUTLINE_V2_STORE_IO"
        }
    };
    Failure::new(
        code,
        "the revision-3 Quest outline-v2 Store operation failed",
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
        QuestTransitionConditionAtomV1, QuestTransitionConditionGroupV1, QuestTransitionEdgeV1,
        QuestTransitionEffectKindV1, QuestTransitionEffectV1, QuestTransitionNodeV1,
        QuestTransitionPlanV1, QuestTransitionPredicateV1, QuestTransitionStateTestV1,
        Revision3Entity, Revision3EntityKind, Revision3EntityPayload, Revision3OriginRef,
        Revision3QuestDraft, Revision3QuestDraftInput, Revision3QuestGiverInput,
        Revision3QuestOutlineObjectiveEditV2, Revision3QuestParentInput, Revision3TypedRef,
        SchemaRevisionV3, Sha256Digest, QUEST_COLLISION_CATALOG_LAYER,
        REVISION3_QUEST_GENERATOR_ID, REVISION3_SEMANTIC_QUEST_GENERATOR_VERSION,
    };
    use tempfile::TempDir;

    use super::*;

    const QUEST_ID_BYTE: u8 = 0x41;
    const MODULE_ID_BYTE: u8 = 0x42;

    struct PublishedStore {
        temp: TempDir,
        project: ProjectRevision3,
        project_json: String,
        head: WorkingHead,
        fixed_head_bytes: Vec<u8>,
        artifact_path: PathBuf,
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
                name: "Quest outline-v2 FFI fixture".to_owned(),
                version: "1.0.0".to_owned(),
                author: "tests".to_owned(),
            },
            target: target(),
            authoring_locales: BTreeSet::new(),
            entities: BTreeMap::new(),
            asset_store: AssetStoreIndex::default(),
        }
    }

    fn asset_path(root: &Path, digest: Sha256Digest) -> PathBuf {
        let hex = digest.to_string();
        root.join("assets")
            .join("sha256")
            .join(&hex[..2])
            .join(&hex[2..])
    }

    fn semantic_plan() -> QuestTransitionPlanV1 {
        let mut plan = QuestTransitionPlanV1::legacy_seed(3).unwrap();
        let root_start = plan
            .transitions
            .iter_mut()
            .find(|transition| {
                transition.node == QuestTransitionNodeV1::Root
                    && transition.edge == QuestTransitionEdgeV1::Start
            })
            .unwrap();
        root_start.effects.push(QuestTransitionEffectV1 {
            target: QuestTransitionNodeV1::Objective { slot: 2 },
            effect: QuestTransitionEffectKindV1::Start,
        });
        let objective_one_availability = plan
            .transitions
            .iter_mut()
            .find(|transition| {
                transition.node == QuestTransitionNodeV1::Objective { slot: 1 }
                    && transition.edge == QuestTransitionEdgeV1::Availability
            })
            .unwrap();
        objective_one_availability.external_allowed = false;
        objective_one_availability.predicate = Some(QuestTransitionPredicateV1 {
            any_of: vec![QuestTransitionConditionGroupV1 {
                all_of: vec![QuestTransitionConditionAtomV1 {
                    node: QuestTransitionNodeV1::Root,
                    test: QuestTransitionStateTestV1::Running,
                    negated: false,
                }],
            }],
        });
        gore_authoring::validate_draft_quest_transition_plan_v1(&plan, 3).unwrap();
        plan
    }

    fn collision_input(quest: &Revision3QuestDraft) -> QuestCollisionCatalogInput {
        QuestCollisionCatalogInput {
            generation: quest.input.collision_catalog.generation.clone(),
            source_seal: quest.input.collision_catalog.source_seal.clone(),
            catalog_layer: quest.input.collision_catalog.catalog_layer.clone(),
            modules: BTreeSet::new(),
            relative_paths: BTreeSet::new(),
            symbols: BTreeSet::new(),
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
        let artifact_path = asset_path(temp.path(), imported.artifact.sha256);

        let mut project = empty_project(1);
        project
            .asset_store
            .assets
            .insert(imported.artifact.sha256, imported.asset_meta.clone());
        let quest_id = entity_id(QUEST_ID_BYTE);
        let module_id = entity_id(MODULE_ID_BYTE);
        let quest = Revision3QuestDraft {
            generator_id: REVISION3_QUEST_GENERATOR_ID.to_owned(),
            generator_version: REVISION3_SEMANTIC_QUEST_GENERATOR_VERSION,
            input: Revision3QuestDraftInput {
                target: target(),
                quest_id,
                module_namespace: "GoreMods.Quests.OutlineV2Fixture".to_owned(),
                technical_id: "GORE_OUTLINE_V2_FIXTURE".to_owned(),
                text_helper: "GoreOutlineV2FixtureText".to_owned(),
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
                title: "Original title".to_owned(),
                description: "Description remains immutable in outline edit v2.".to_owned(),
                objective_title: "Enter the arena".to_owned(),
                additional_objective_titles: vec![
                    "Defeat the guard".to_owned(),
                    "Report to Asghan".to_owned(),
                ],
                transition_plan: Some(Box::new(semantic_plan())),
                collision_catalog: QuestCollisionArtifactRef {
                    generation: target(),
                    catalog_layer: QUEST_COLLISION_CATALOG_LAYER.to_owned(),
                    artifact: imported.artifact,
                    source_seal: seal(0x33, artifact_bytes.len() as u64),
                    basis_snapshot: basis.head.snapshot.clone(),
                },
            },
            script_module: Revision3TypedRef::new(
                project.project_id,
                module_id,
                Revision3EntityKind::ScriptModule,
            ),
            transcript: Vec::new(),
        };
        let module = regenerate_revision3_quest_module_v2(&quest, collision_input(&quest)).unwrap();
        let owner = Revision3TypedRef::new(
            project.project_id,
            quest_id,
            Revision3EntityKind::QuestDraft,
        );
        project.entities.insert(
            quest_id,
            Revision3Entity {
                id: quest_id,
                display_name: "Original fixture Quest".to_owned(),
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
                display_name: "Outline-v2 fixture generated module".to_owned(),
                origin: Revision3OriginRef::Generated {
                    generator_id: REVISION3_QUEST_GENERATOR_ID.to_owned(),
                    generator_version: REVISION3_SEMANTIC_QUEST_GENERATOR_VERSION,
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
            artifact_path,
        }
    }

    fn quest(project: &ProjectRevision3) -> &Revision3QuestDraft {
        let Revision3EntityPayload::QuestDraft(quest) =
            &project.entities[&entity_id(QUEST_ID_BYTE)].payload
        else {
            panic!("expected Quest")
        };
        quest
    }

    fn current_objectives(project: &ProjectRevision3) -> Vec<Revision3QuestOutlineObjectiveEditV2> {
        let quest = quest(project);
        let plan = quest.input.transition_plan.as_deref().unwrap();
        let titles = std::iter::once(quest.input.objective_title.as_str()).chain(
            quest
                .input
                .additional_objective_titles
                .iter()
                .map(String::as_str),
        );
        plan.objective_order
            .iter()
            .copied()
            .zip(titles)
            .map(|(slot, title)| Revision3QuestOutlineObjectiveEditV2 {
                slot,
                title: title.to_owned(),
            })
            .collect()
    }

    fn base_request(store: &PublishedStore) -> Revision3QuestOutlineEditRequestV2 {
        let quest_id = entity_id(QUEST_ID_BYTE);
        Revision3QuestOutlineEditRequestV2 {
            expected_head: store.head.clone(),
            expected_project_id: store.project.project_id,
            expected_revision: store.project.revision,
            expected_target: store.project.target.clone(),
            quest_id,
            expected_quest_revision: store.project.entities[&quest_id].revision,
            expected_script_module_id: entity_id(MODULE_ID_BYTE),
            expected_script_module_revision: store.project.entities[&entity_id(MODULE_ID_BYTE)]
                .revision,
            expected_transition_plan_seal: revision3_quest_transition_plan_basis_v1(quest(
                &store.project,
            ))
            .unwrap()
            .seal,
            display_name: store.project.entities[&quest_id].display_name.clone(),
            quest_title: quest(&store.project).input.title.clone(),
            objectives: current_objectives(&store.project),
        }
    }

    fn edit_request(store: &PublishedStore) -> Revision3QuestOutlineEditRequestV2 {
        let base = base_request(store);
        Revision3QuestOutlineEditRequestV2 {
            display_name: "Edited semantic fixture Quest".to_owned(),
            quest_title: "A clearer semantic title".to_owned(),
            objectives: vec![
                Revision3QuestOutlineObjectiveEditV2 {
                    slot: 3,
                    title: "Report your victory".to_owned(),
                },
                Revision3QuestOutlineObjectiveEditV2 {
                    slot: 1,
                    title: "Enter Asghan's arena".to_owned(),
                },
                Revision3QuestOutlineObjectiveEditV2 {
                    slot: 2,
                    title: "Defeat the arena guard".to_owned(),
                },
            ],
            ..base
        }
    }

    fn raw_request(payload: Value) -> String {
        serde_json::to_string(&json!({"command": COMMAND, "payload": payload})).unwrap()
    }

    fn wire(store: &PublishedStore, request: &Revision3QuestOutlineEditRequestV2) -> String {
        raw_request(json!({
            "current_project_json": store.project_json,
            "quest_outline_request_json": request.to_canonical_json().unwrap(),
            "root": store.temp.path(),
        }))
    }

    fn call(store: &PublishedStore, request: &Revision3QuestOutlineEditRequestV2) -> Value {
        prepare_revision3_quest_outline_edit_v2_raw(&wire(store, request))
    }

    fn fixed_head(store: &PublishedStore) -> Vec<u8> {
        fs::read(store.temp.path().join("gore-project.json")).unwrap()
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
    fn exact_wire_and_public_dispatch_reject_duplicates_unknowns_and_noncanonical_spelling() {
        let valid = raw_request(json!({
            "current_project_json": "{}",
            "quest_outline_request_json": "{}",
            "root": "C:/missing",
        }));
        let parsed: PrepareQuestOutlineV2WirePayload = parse_exact_wire(&valid).unwrap();
        assert_eq!(parsed.current_project_json, "{}");

        let cases = [
            format!(
                "{{\"command\":\"{COMMAND}\",\"command\":\"{COMMAND}\",\"payload\":{{\"current_project_json\":\"{{}}\",\"quest_outline_request_json\":\"{{}}\",\"root\":\"r\"}}}}"
            ),
            format!(
                "{{\"command\":\"{COMMAND}\",\"payload\":{{\"current_project_json\":\"{{}}\",\"quest_outline_request_json\":\"{{}}\",\"root\":\"r\",\"root\":\"forged\"}}}}"
            ),
            raw_request(json!({
                "current_project_json": "{}", "quest_outline_request_json": "{}",
                "root": "r", "authority": "forged"
            })),
            raw_request(json!({
                "quest_outline_request_json": "{}", "root": "r"
            })),
            raw_request(json!({
                "current_project_json": {}, "quest_outline_request_json": "{}", "root": "r"
            })),
            format!(" {valid}"),
            format!(
                "{{\"payload\":{{\"current_project_json\":\"{{}}\",\"quest_outline_request_json\":\"{{}}\",\"root\":\"C:/missing\"}},\"command\":\"{COMMAND}\"}}"
            ),
        ];
        for input in cases {
            assert_eq!(
                prepare_revision3_quest_outline_edit_v2_raw(&input)["error"]["code"],
                "AUTHORING_REVISION3_QUEST_OUTLINE_V2_REQUEST_INVALID",
                "{input}"
            );
        }

        let duplicate = format!(
            "{{\"command\":\"{COMMAND}\",\"payload\":{{\"current_project_json\":\"{{}}\",\"quest_outline_request_json\":\"{{}}\",\"root\":\"r\",\"root\":\"forged\"}}}}"
        );
        let response: Value = serde_json::from_str(&crate::execute_json(&duplicate)).unwrap();
        assert_eq!(
            response["error"]["code"],
            "AUTHORING_REVISION3_QUEST_OUTLINE_V2_REQUEST_INVALID"
        );
    }

    #[test]
    fn transports_and_signed_wire_values_are_bounded_before_store_mutation() {
        let valid_shape = || {
            json!({
                "current_project_json": "{}",
                "quest_outline_request_json": "{}",
                "root": "C:/missing",
            })
        };
        let mut payload = valid_shape();
        payload["root"] = Value::String("x".repeat(MAX_PATH_BYTES + 1));
        assert_eq!(
            prepare_revision3_quest_outline_edit_v2_raw(&raw_request(payload))["error"]["code"],
            "AUTHORING_REVISION3_QUEST_OUTLINE_V2_REQUEST_INVALID"
        );
        let mut payload = valid_shape();
        payload["current_project_json"] = Value::String("x".repeat(MAX_PROJECT_JSON_BYTES + 1));
        assert_eq!(
            prepare_revision3_quest_outline_edit_v2_raw(&raw_request(payload))["error"]["code"],
            "AUTHORING_REVISION3_QUEST_OUTLINE_V2_PROJECT_LIMIT"
        );
        let mut payload = valid_shape();
        payload["quest_outline_request_json"] =
            Value::String("x".repeat(MAX_OUTLINE_REQUEST_JSON_BYTES + 1));
        assert_eq!(
            prepare_revision3_quest_outline_edit_v2_raw(&raw_request(payload))["error"]["code"],
            "AUTHORING_REVISION3_QUEST_OUTLINE_V2_REQUEST_LIMIT"
        );
        assert_eq!(
            prepare_revision3_quest_outline_edit_v2_raw(&" ".repeat(MAX_WIRE_BYTES + 1))["error"]
                ["code"],
            "AUTHORING_REVISION3_QUEST_OUTLINE_V2_INPUT_LIMIT"
        );

        let store = published_store();
        let mut request = edit_request(&store);
        request.expected_revision = u64::MAX;
        assert_eq!(
            call(&store, &request)["error"]["code"],
            "AUTHORING_REVISION3_QUEST_OUTLINE_V2_SIGNED_WIRE_LIMIT"
        );
        assert_eq!(fixed_head(&store), store.fixed_head_bytes);
    }

    #[test]
    fn semantic_reorder_fully_reopens_with_stable_slots_and_never_publishes_fixed_head() {
        let store = published_store();
        let before_plan = quest(&store.project)
            .input
            .transition_plan
            .as_deref()
            .unwrap()
            .clone();
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
        let reopened_quest = quest(&reopened.project);
        let reopened_plan = reopened_quest.input.transition_plan.as_deref().unwrap();
        assert_eq!(reopened_plan.objective_order, vec![3, 1, 2]);
        assert_eq!(reopened_plan.objective_slots, before_plan.objective_slots);
        assert_eq!(
            reopened_plan.next_slot_ordinal,
            before_plan.next_slot_ordinal
        );
        assert_eq!(reopened_plan.transitions, before_plan.transitions);
        assert_eq!(reopened_quest.input.objective_title, "Report your victory");
        assert_eq!(
            reopened_quest.input.additional_objective_titles,
            vec!["Enter Asghan's arena", "Defeat the arena guard"]
        );
        assert_eq!(current_objectives(&reopened.project), request.objectives);
        assert_eq!(
            response["transition_plan_seal"],
            serde_json::to_value(
                revision3_quest_transition_plan_basis_v1(reopened_quest)
                    .unwrap()
                    .seal
            )
            .unwrap()
        );
        assert_eq!(
            revision3_quest_transition_plan_seal_v1(reopened_plan).unwrap(),
            revision3_quest_transition_plan_basis_v1(reopened_quest)
                .unwrap()
                .seal
        );

        let encoded = response.to_string();
        assert!(!encoded.contains(store.temp.path().to_string_lossy().as_ref()));
        assert!(!encoded.contains("game_root"));
        assert!(!encoded.contains("compiler"));
        assert!(!encoded.contains("deploy"));
        assert_ne!(response["publication_status"], "published");
    }

    #[test]
    fn stale_project_head_quest_module_plan_slots_and_noop_are_precisely_mapped_without_writes() {
        let store = published_store();
        let before = snapshot_regular_files(store.temp.path());
        let base = edit_request(&store);

        let response = prepare_revision3_quest_outline_edit_v2_raw(&raw_request(json!({
            "current_project_json": store.project_json.replacen("\"revision\":1", "\"revision\":0", 1),
            "quest_outline_request_json": base.to_canonical_json().unwrap(),
            "root": store.temp.path(),
        })));
        assert_eq!(
            response["error"]["code"],
            "AUTHORING_REVISION3_QUEST_OUTLINE_V2_PROJECT_CONFLICT"
        );

        let mut candidate = base.clone();
        candidate.expected_head.snapshot.byte_len += 1;
        assert_eq!(
            call(&store, &candidate)["error"]["code"],
            "AUTHORING_REVISION3_QUEST_OUTLINE_V2_HEAD_CONFLICT"
        );
        candidate = base.clone();
        candidate.expected_revision += 1;
        assert_eq!(
            call(&store, &candidate)["error"]["code"],
            "AUTHORING_REVISION3_QUEST_OUTLINE_V2_PROJECT_CONFLICT"
        );
        candidate = base.clone();
        candidate.expected_target.executable.byte_len += 1;
        assert_eq!(
            call(&store, &candidate)["error"]["code"],
            "AUTHORING_REVISION3_QUEST_OUTLINE_V2_TARGET_CONFLICT"
        );
        candidate = base.clone();
        candidate.expected_quest_revision += 1;
        assert_eq!(
            call(&store, &candidate)["error"]["code"],
            "AUTHORING_REVISION3_QUEST_OUTLINE_V2_QUEST_CONFLICT"
        );
        candidate = base.clone();
        candidate.expected_script_module_id = entity_id(0x70);
        assert_eq!(
            call(&store, &candidate)["error"]["code"],
            "AUTHORING_REVISION3_QUEST_OUTLINE_V2_MODULE_CONFLICT"
        );
        candidate = base.clone();
        candidate.expected_script_module_revision += 1;
        assert_eq!(
            call(&store, &candidate)["error"]["code"],
            "AUTHORING_REVISION3_QUEST_OUTLINE_V2_MODULE_CONFLICT"
        );
        candidate = base.clone();
        candidate.expected_transition_plan_seal.byte_len += 1;
        assert_eq!(
            call(&store, &candidate)["error"]["code"],
            "AUTHORING_REVISION3_QUEST_OUTLINE_V2_PLAN_CONFLICT"
        );
        candidate = base.clone();
        candidate.objectives[1].slot = candidate.objectives[0].slot;
        assert_eq!(
            call(&store, &candidate)["error"]["code"],
            "AUTHORING_REVISION3_QUEST_OUTLINE_V2_SLOT_CONFLICT"
        );
        candidate = base.clone();
        candidate.objectives[2].slot = 99;
        assert_eq!(
            call(&store, &candidate)["error"]["code"],
            "AUTHORING_REVISION3_QUEST_OUTLINE_V2_SLOT_CONFLICT"
        );
        candidate = base.clone();
        candidate.objectives.pop();
        assert_eq!(
            call(&store, &candidate)["error"]["code"],
            "AUTHORING_REVISION3_QUEST_OUTLINE_V2_SLOT_CONFLICT"
        );
        assert_eq!(
            call(&store, &base_request(&store))["error"]["code"],
            "AUTHORING_REVISION3_QUEST_OUTLINE_V2_NO_CHANGES"
        );
        assert_eq!(fixed_head(&store), store.fixed_head_bytes);
        assert_eq!(snapshot_regular_files(store.temp.path()), before);
    }

    #[test]
    fn malformed_or_noncanonical_nested_request_is_rejected_before_store_access_or_mutation() {
        let store = published_store();
        let before = snapshot_regular_files(store.temp.path());
        let canonical = edit_request(&store).to_canonical_json().unwrap();
        for nested in [
            "{}".to_owned(),
            format!(" {canonical}"),
            canonical.replacen(
                "\"expected_revision\":1",
                "\"expected_revision\":1,\"expected_revision\":1",
                1,
            ),
            canonical.replacen("\"slot\":3", "\"slot\":3,\"forged\":true", 1),
        ] {
            let response = prepare_revision3_quest_outline_edit_v2_raw(&raw_request(json!({
                "current_project_json": store.project_json,
                "quest_outline_request_json": nested,
                "root": store.temp.path(),
            })));
            assert_eq!(
                response["error"]["code"],
                "AUTHORING_REVISION3_QUEST_OUTLINE_V2_REQUEST_INVALID"
            );
        }
        assert_eq!(fixed_head(&store), store.fixed_head_bytes);
        assert_eq!(snapshot_regular_files(store.temp.path()), before);

        let temp = TempDir::new().unwrap();
        let missing_root = temp.path().join("missing-store");
        let response = prepare_revision3_quest_outline_edit_v2_raw(&raw_request(json!({
            "current_project_json": "{}",
            "quest_outline_request_json": "{}",
            "root": missing_root,
        })));
        assert_eq!(
            response["error"]["code"],
            "AUTHORING_REVISION3_QUEST_OUTLINE_V2_REQUEST_INVALID"
        );
    }

    #[test]
    fn checkpoint_failure_is_path_free_and_never_publishes_candidate() {
        let store = published_store();
        let request = edit_request(&store);
        let response = prepare_revision3_quest_outline_edit_v2_inner_with_test_seams(
            &wire(&store, &request),
            || fs::remove_file(&store.artifact_path).unwrap(),
            || {},
        )
        .unwrap_err()
        .response();
        assert_eq!(
            response["error"]["code"],
            "AUTHORING_REVISION3_QUEST_OUTLINE_V2_STORE_OBJECT_MISSING"
        );
        assert!(!response
            .to_string()
            .contains(store.temp.path().to_string_lossy().as_ref()));
        assert_eq!(fixed_head(&store), store.fixed_head_bytes);
    }

    #[test]
    fn late_external_head_race_returns_conflict_and_preserves_racing_head() {
        let store = published_store();
        let working =
            WorkingProjectStore::open_existing(store.temp.path(), ffi_store_limits()).unwrap();
        let mut rival = store.project.clone();
        rival.revision += 1;
        rival.meta.name = "External publisher won".to_owned();
        let rival = working
            .prepare_revision3_checkpoint(Some(&store.head), &rival)
            .unwrap();
        let request = edit_request(&store);
        let response = prepare_revision3_quest_outline_edit_v2_inner_with_final_guard(
            &wire(&store, &request),
            || {
                fs::write(
                    store.temp.path().join("gore-project.json"),
                    &rival.head_bytes,
                )
                .unwrap();
            },
        )
        .unwrap_err()
        .response();
        assert_eq!(
            response["error"]["code"],
            "AUTHORING_REVISION3_QUEST_OUTLINE_V2_HEAD_CONFLICT"
        );
        assert_eq!(fixed_head(&store), rival.head_bytes);
    }

    #[test]
    fn conflict_mapping_distinguishes_plan_module_slots_and_integrity_failures() {
        assert_eq!(
            map_transaction_conflict(
                Revision3QuestOutlineEditConflictV2::TransitionPlanSealConflict {
                    expected: seal(1, 1),
                    actual: seal(2, 2),
                }
            )
            .code,
            "AUTHORING_REVISION3_QUEST_OUTLINE_V2_PLAN_CONFLICT"
        );
        assert_eq!(
            map_transaction_conflict(
                Revision3QuestOutlineEditConflictV2::ScriptModuleRevisionConflict {
                    expected: 1,
                    actual: 2,
                }
            )
            .code,
            "AUTHORING_REVISION3_QUEST_OUTLINE_V2_MODULE_CONFLICT"
        );
        assert_eq!(
            map_transaction_conflict(
                Revision3QuestOutlineEditConflictV2::DuplicateObjectiveSlot { slot: 1 }
            )
            .code,
            "AUTHORING_REVISION3_QUEST_OUTLINE_V2_SLOT_CONFLICT"
        );
        assert_eq!(
            map_transaction_conflict(Revision3QuestOutlineEditConflictV2::OwnedModuleDrift {
                quest: entity_id(QUEST_ID_BYTE),
                module: entity_id(MODULE_ID_BYTE),
            })
            .code,
            "AUTHORING_REVISION3_QUEST_OUTLINE_V2_PROJECT_INVALID"
        );
        assert_eq!(
            map_store_error(WorkingStoreError::UnsafePath {
                path: PathBuf::from("C:/secret/store"),
                reason: "fixture".to_owned(),
            })
            .code,
            "AUTHORING_REVISION3_QUEST_OUTLINE_V2_STORE_PATH_UNSAFE"
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
        let response = prepare_revision3_quest_outline_edit_v2_raw(&raw_request(json!({
            "current_project_json": store.project_json,
            "quest_outline_request_json": request.to_canonical_json().unwrap(),
            "root": alias,
        })));
        assert_eq!(
            response["error"]["code"],
            "AUTHORING_REVISION3_QUEST_OUTLINE_V2_STORE_PATH_UNSAFE"
        );
        assert_eq!(fixed_head(&store), store.fixed_head_bytes);
        #[cfg(unix)]
        fs::remove_file(alias).unwrap();
        #[cfg(windows)]
        fs::remove_dir(alias).unwrap();
    }
}
