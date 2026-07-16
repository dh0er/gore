//! Prepare-only selection (or clearing) of one existing revision-3 VoiceTake.
//!
//! This route accepts no game, save, Ogg-source, deployment, build, or fixed-head publication
//! authority. It fully opens the exact current Store project and every referenced asset, binds a
//! canonical selection request to that basis, evaluates the pure transaction, and prepares an
//! immutable candidate checkpoint. The fixed `gore-project.json` head is checked in full after
//! candidate preparation and again after response construction, but is never replaced here.

use std::path::Path;

use gore_authoring::model_revision3::{EntityKind, EntityPayload};
use gore_authoring::{
    apply_revision3_voice_take_selection_transaction_v1,
    validate_revision3_voice_loc_id_basename_stem_v1, AssetVerification, EntityId,
    ProjectRevision3, Revision3VoiceTakeSelectionBuildStatusV1,
    Revision3VoiceTakeSelectionConflictV1, Revision3VoiceTakeSelectionErrorV1,
    Revision3VoiceTakeSelectionEvaluationV1, Revision3VoiceTakeSelectionRequestV1,
    Revision3VoiceTakeSelectionRuntimeStatusV1, WorkingHead, WorkingProjectStore,
    WorkingStoreError, WorkingStoreLimits, MAX_PROJECT_JSON_BYTES,
    MAX_REVISION3_VOICE_TAKE_SELECTION_REQUEST_JSON_BYTES_V1,
};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::err;

pub(super) const COMMAND: &str = "authoring_store_prepare_revision3_voice_take_selection_v1";

const MAX_PATH_BYTES: usize = 32 * 1024;
const MAX_HEAD_JSON_BYTES: usize = 64 * 1024;
const MAX_RESPONSE_BYTES: usize = 64 * 1024 * 1024;
const MAX_ERROR_MESSAGE_BYTES: usize = 4 * 1024;
// Both the project and selected VoiceSlot revisions increment exactly once. Studio transports
// signed 64-bit JSON integers on every supported host language.
const MAX_BASIS_REVISION: u64 = i64::MAX as u64 - 1;
// Canonical nested JSON strings need at most one extra escape byte per source byte. Store roots
// are arbitrary caller strings and retain the full six-byte JSON escape allowance.
const MAX_WIRE_BYTES: usize = MAX_PROJECT_JSON_BYTES * 2
    + MAX_REVISION3_VOICE_TAKE_SELECTION_REQUEST_JSON_BYTES_V1 * 2
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
struct PrepareVoiceSelectionWirePayload {
    current_project_json: String,
    root: String,
    voice_take_selection_request_json: String,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct BoundVoiceSelection {
    localization_id: EntityId,
    previous_selected_take_id: Option<EntityId>,
}

pub(super) fn prepare_revision3_voice_take_selection_v1_raw(input: &str) -> Value {
    prepare_revision3_voice_take_selection_v1_inner(input).unwrap_or_else(Failure::response)
}

fn prepare_revision3_voice_take_selection_v1_inner(input: &str) -> Result<Value, Failure> {
    prepare_revision3_voice_take_selection_v1_inner_with_test_seams(input, || {}, || {}, || {})
}

#[cfg(test)]
fn prepare_revision3_voice_take_selection_v1_inner_with_final_guard<F>(
    input: &str,
    final_guard: F,
) -> Result<Value, Failure>
where
    F: FnOnce(),
{
    prepare_revision3_voice_take_selection_v1_inner_with_test_seams(
        input,
        || {},
        || {},
        final_guard,
    )
}

#[cfg(test)]
fn prepare_revision3_voice_take_selection_v1_inner_with_post_prepare_guard<A>(
    input: &str,
    after_checkpoint: A,
) -> Result<Value, Failure>
where
    A: FnOnce(),
{
    prepare_revision3_voice_take_selection_v1_inner_with_test_seams(
        input,
        || {},
        after_checkpoint,
        || {},
    )
}

fn prepare_revision3_voice_take_selection_v1_inner_with_test_seams<B, A, F>(
    input: &str,
    before_checkpoint: B,
    after_checkpoint: A,
    final_guard: F,
) -> Result<Value, Failure>
where
    B: FnOnce(),
    A: FnOnce(),
    F: FnOnce(),
{
    let payload: PrepareVoiceSelectionWirePayload = parse_exact_wire(input)?;
    validate_payload(&payload)?;

    // Parse the small semantic request, reject duplicate/noncanonical JSON, and prove every
    // number fits the signed Studio wire before opening or even probing the Store root.
    let request =
        Revision3VoiceTakeSelectionRequestV1::from_json(&payload.voice_take_selection_request_json)
            .map_err(map_request_error)?;
    require_signed_serializable(&request)?;
    validate_request_shape(&request)?;

    let store = WorkingProjectStore::open_existing(Path::new(&payload.root), ffi_store_limits())
        .map_err(map_store_error)?;
    let basis = store
        .open_current_revision3(AssetVerification::Full)
        .map_err(map_store_error)?;
    validate_basis_revision(basis.project.revision)?;
    validate_bound_slot_revision(&basis.project, &request)?;
    require_signed_serializable(&basis.project)?;
    require_signed_serializable(&basis.head)?;

    let canonical_basis = basis.project.to_canonical_json().map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_VOICE_SELECTION_STORE_INVARIANT",
            "the exact current revision-3 project could not be serialized canonically",
        )
    })?;
    if canonical_basis.as_bytes() != payload.current_project_json.as_bytes() {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_SELECTION_PROJECT_CONFLICT",
            "current_project_json differs from the exact published revision-3 project",
        ));
    }

    let bound = bind_request_to_basis(&basis.head, &basis.project, &request)?;
    let outcome = match apply_revision3_voice_take_selection_transaction_v1(
        &basis.head,
        &canonical_basis,
        &payload.voice_take_selection_request_json,
    )
    .map_err(map_transaction_error)?
    {
        Revision3VoiceTakeSelectionEvaluationV1::Applied(outcome) => *outcome,
        Revision3VoiceTakeSelectionEvaluationV1::Rejected(rejection) => {
            return Err(map_transaction_conflict(rejection.conflict));
        }
    };

    require_signed_serializable(&outcome.project)?;
    verify_outcome_binding(&basis.head, &basis.project, &request, bound, &outcome)?;
    match outcome.build_status {
        Revision3VoiceTakeSelectionBuildStatusV1::Blocked => {}
    }
    match outcome.runtime_status {
        Revision3VoiceTakeSelectionRuntimeStatusV1::RuntimeUnqualified => {}
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
            "AUTHORING_REVISION3_VOICE_SELECTION_STORE_INVARIANT",
            "the prepared Voice selection checkpoint did not fully reopen exactly",
        ));
    }
    let reopened_json = reopened.project.to_canonical_json().map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_VOICE_SELECTION_STORE_INVARIANT",
            "the fully reopened Voice selection candidate could not be serialized",
        )
    })?;
    if reopened_json != outcome.canonical_project_json {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_SELECTION_STORE_INVARIANT",
            "the fully reopened Voice selection candidate changed canonical bytes",
        ));
    }

    // Immutable candidate preparation must not hide a concurrent fixed-head publisher.
    after_checkpoint();
    require_fixed_basis(&store, &basis.head, &basis.project)?;

    let basis_head_json = canonical_head_json(&basis.head)?;
    let candidate_head_json = String::from_utf8(prepared.head_bytes).map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_VOICE_SELECTION_STORE_INVARIANT",
            "the prepared Voice selection head is not UTF-8 JSON",
        )
    })?;
    if candidate_head_json.is_empty() || candidate_head_json.len() > MAX_HEAD_JSON_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_SELECTION_RESPONSE_LIMIT",
            "the prepared Voice selection head exceeds its bounded transport limit",
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
        "line_id": outcome.line_id.to_string(),
        "slot_id": outcome.slot_id.to_string(),
        "slot_revision": outcome.slot_revision,
        "locale": outcome.locale.to_string(),
        "loc_id": outcome.loc_id,
        "previous_selected_take_id": outcome
            .previous_selected_take_id
            .map(|id| id.to_string()),
        "selected_take_id": outcome.selected_take_id.map(|id| id.to_string()),
        "build_status": "blocked",
        "runtime_status": "runtime_unqualified",
        "publication_status": "not_supported",
    });
    enforce_response_budget(&response)?;

    // Test seam models an external publisher after response construction. Production supplies a
    // no-op. This final full current-open is authoritative and never restores the old head.
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
            "AUTHORING_REVISION3_VOICE_SELECTION_INPUT_LIMIT",
            format!(
                "revision-3 Voice selection request exceeds the {MAX_WIRE_BYTES}-byte wire limit"
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
            "AUTHORING_REVISION3_VOICE_SELECTION_INVARIANT",
            "the Voice selection outer request could not be serialized",
        )
    })?;
    if canonical.as_bytes() != input.as_bytes() {
        return Err(invalid_request());
    }
    Ok(request.payload)
}

fn validate_payload(payload: &PrepareVoiceSelectionWirePayload) -> Result<(), Failure> {
    validate_path(&payload.root)?;
    if payload.current_project_json.is_empty() {
        return Err(invalid_request());
    }
    if payload.current_project_json.len() > MAX_PROJECT_JSON_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_SELECTION_PROJECT_LIMIT",
            format!("current_project_json exceeds the {MAX_PROJECT_JSON_BYTES}-byte limit"),
        ));
    }
    if payload.voice_take_selection_request_json.is_empty() {
        return Err(invalid_request());
    }
    if payload.voice_take_selection_request_json.len()
        > MAX_REVISION3_VOICE_TAKE_SELECTION_REQUEST_JSON_BYTES_V1
    {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_SELECTION_REQUEST_LIMIT",
            format!(
                "voice_take_selection_request_json exceeds the {MAX_REVISION3_VOICE_TAKE_SELECTION_REQUEST_JSON_BYTES_V1}-byte limit"
            ),
        ));
    }
    Ok(())
}

fn validate_request_shape(request: &Revision3VoiceTakeSelectionRequestV1) -> Result<(), Failure> {
    if is_zero_entity_id(request.line_id)
        || is_zero_entity_id(request.slot_id)
        || request.line_id == request.slot_id
    {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_SELECTION_REQUEST_REJECTED",
            "DialogLine and VoiceSlot identities must be non-zero and distinct",
        ));
    }
    for take in [request.expected_selected_take_id, request.selected_take_id]
        .into_iter()
        .flatten()
    {
        if is_zero_entity_id(take) || take == request.line_id || take == request.slot_id {
            return Err(Failure::new(
                "AUTHORING_REVISION3_VOICE_SELECTION_TAKE_CONFLICT",
                "a VoiceTake identity is zero or collides with the bound line or slot",
            ));
        }
    }
    if validate_revision3_voice_loc_id_basename_stem_v1(&request.expected_loc_id).is_err() {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_SELECTION_REQUEST_REJECTED",
            "expected_loc_id is not one bounded portable Voice basename stem",
        ));
    }
    Ok(())
}

fn validate_bound_slot_revision(
    project: &ProjectRevision3,
    request: &Revision3VoiceTakeSelectionRequestV1,
) -> Result<(), Failure> {
    let Some(line_entity) = project.entities.get(&request.line_id) else {
        return Ok(());
    };
    let EntityPayload::DialogLine(line) = &line_entity.payload else {
        return Ok(());
    };
    let Some(slot_reference) = line.voice_slots.get(&request.locale) else {
        return Ok(());
    };
    if slot_reference.project_id != project.project_id
        || slot_reference.expected_kind != EntityKind::VoiceSlot
        || slot_reference.id != request.slot_id
    {
        return Ok(());
    }
    let Some(slot_entity) = project.entities.get(&request.slot_id) else {
        return Ok(());
    };
    let EntityPayload::VoiceSlot(slot) = &slot_entity.payload else {
        return Ok(());
    };
    if slot.locale != request.locale || slot_entity.revision != request.expected_slot_revision {
        return Ok(());
    }
    if slot_entity.revision > MAX_BASIS_REVISION {
        return Err(revision_limit(
            "the published VoiceSlot revision cannot be incremented on the signed wire",
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

fn is_zero_entity_id(id: EntityId) -> bool {
    id.as_bytes().iter().all(|byte| *byte == 0)
}

fn bind_request_to_basis(
    head: &WorkingHead,
    project: &ProjectRevision3,
    request: &Revision3VoiceTakeSelectionRequestV1,
) -> Result<BoundVoiceSelection, Failure> {
    if request.expected_head != *head {
        return Err(head_conflict());
    }
    if request.expected_project_id != project.project_id
        || request.expected_revision != project.revision
    {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_SELECTION_PROJECT_CONFLICT",
            "the Voice selection request project differs from the exact published project",
        ));
    }
    if request.expected_target != project.target {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_SELECTION_TARGET_CONFLICT",
            "the Voice selection request target differs from the exact published project target",
        ));
    }

    let line_entity = project.entities.get(&request.line_id).ok_or_else(|| {
        line_conflict("the requested DialogLine is missing from the exact published project")
    })?;
    let EntityPayload::DialogLine(line) = &line_entity.payload else {
        return Err(line_conflict(
            "the requested entity is not a DialogLine in the exact published project",
        ));
    };
    if line.localization.project_id != project.project_id
        || line.localization.expected_kind != EntityKind::LocalizationEntry
    {
        return Err(project_invalid(
            "the exact DialogLine has an invalid LocalizationEntry reference",
        ));
    }
    let localization_entity = project
        .entities
        .get(&line.localization.id)
        .ok_or_else(|| project_invalid("the exact DialogLine LocalizationEntry is missing"))?;
    let EntityPayload::LocalizationEntry(localization) = &localization_entity.payload else {
        return Err(project_invalid(
            "the exact DialogLine localization reference has the wrong kind",
        ));
    };
    if localization.loc_id != request.expected_loc_id {
        return Err(line_conflict(
            "expected_loc_id differs from the exact DialogLine LocalizationEntry",
        ));
    }

    let slot_ref = line
        .voice_slots
        .get(&request.locale)
        .ok_or_else(|| slot_conflict("the requested DialogLine locale has no VoiceSlot"))?;
    if slot_ref.project_id != project.project_id
        || slot_ref.expected_kind != EntityKind::VoiceSlot
        || slot_ref.id != request.slot_id
    {
        return Err(slot_conflict(
            "the requested DialogLine locale is linked to a different VoiceSlot",
        ));
    }
    let slot_entity = project.entities.get(&request.slot_id).ok_or_else(|| {
        slot_conflict("the requested VoiceSlot is missing from the exact published project")
    })?;
    let EntityPayload::VoiceSlot(slot) = &slot_entity.payload else {
        return Err(slot_conflict(
            "the requested entity is not a VoiceSlot in the exact published project",
        ));
    };
    if slot_entity.revision != request.expected_slot_revision {
        return Err(slot_conflict(
            "the requested VoiceSlot revision differs from the exact published entity revision",
        ));
    }
    if slot.locale != request.locale {
        return Err(slot_conflict(
            "the requested VoiceSlot locale differs from the DialogLine locale",
        ));
    }
    let previous_selected_take_id = match &slot.selected {
        Some(selected)
            if selected.project_id == project.project_id
                && selected.expected_kind == EntityKind::VoiceTake =>
        {
            Some(selected.id)
        }
        Some(_) => {
            return Err(project_invalid(
                "the exact VoiceSlot selected take is not an exact-project VoiceTake reference",
            ));
        }
        None => None,
    };
    if previous_selected_take_id != request.expected_selected_take_id {
        return Err(selection_conflict(
            "the requested current selection differs from the exact published VoiceSlot",
        ));
    }
    Ok(BoundVoiceSelection {
        localization_id: line.localization.id,
        previous_selected_take_id,
    })
}

fn verify_outcome_binding(
    basis_head: &WorkingHead,
    basis: &ProjectRevision3,
    request: &Revision3VoiceTakeSelectionRequestV1,
    bound: BoundVoiceSelection,
    outcome: &gore_authoring::Revision3VoiceTakeSelectionOutcomeV1,
) -> Result<(), Failure> {
    let expected_revision = basis
        .revision
        .checked_add(1)
        .ok_or_else(|| revision_limit("the project revision cannot be incremented"))?;
    let expected_slot_revision = request
        .expected_slot_revision
        .checked_add(1)
        .ok_or_else(|| revision_limit("the VoiceSlot revision cannot be incremented"))?;
    if outcome.basis_head != *basis_head
        || outcome.line_id != request.line_id
        || outcome.localization_id != bound.localization_id
        || outcome.slot_id != request.slot_id
        || outcome.slot_revision != expected_slot_revision
        || outcome.locale != request.locale
        || outcome.loc_id != request.expected_loc_id
        || outcome.previous_selected_take_id != bound.previous_selected_take_id
        || outcome.selected_take_id != request.selected_take_id
        || outcome.project.project_id != basis.project_id
        || outcome.project.target != basis.target
        || outcome.project.revision != expected_revision
    {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_SELECTION_INVARIANT",
            "the Voice selection transaction changed its exact project/request binding",
        ));
    }

    // Close the FFI boundary independently: the only permitted semantic delta is one project
    // revision, one VoiceSlot revision, and that slot's selected exact candidate reference.
    let mut expected = basis.clone();
    expected.revision = expected_revision;
    let expected_slot_entity = expected.entities.get_mut(&request.slot_id).ok_or_else(|| {
        Failure::new(
            "AUTHORING_REVISION3_VOICE_SELECTION_INVARIANT",
            "the bound VoiceSlot disappeared while closing the candidate delta",
        )
    })?;
    expected_slot_entity.revision = expected_slot_revision;
    let EntityPayload::VoiceSlot(expected_slot) = &mut expected_slot_entity.payload else {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_SELECTION_INVARIANT",
            "the bound VoiceSlot changed kind while closing the candidate delta",
        ));
    };
    expected_slot.selected = match request.selected_take_id {
        Some(id) => Some(
            expected_slot
                .candidates
                .iter()
                .find(|candidate| candidate.id == id)
                .cloned()
                .ok_or_else(|| {
                    Failure::new(
                        "AUTHORING_REVISION3_VOICE_SELECTION_INVARIANT",
                        "an applied selected take is absent from the exact candidate list",
                    )
                })?,
        ),
        None => None,
    };
    let expected_json = expected.to_canonical_json().map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_VOICE_SELECTION_INVARIANT",
            "the independently reconstructed Voice selection candidate is invalid",
        )
    })?;
    if outcome.project != expected || outcome.canonical_project_json != expected_json {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_SELECTION_INVARIANT",
            "the Voice selection transaction changed content outside the selected VoiceSlot",
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
    if &current.head != expected_head || &current.project != expected_project {
        return Err(head_conflict());
    }
    Ok(())
}

fn validate_basis_revision(revision: u64) -> Result<(), Failure> {
    if revision > MAX_BASIS_REVISION {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_SELECTION_REVISION_LIMIT",
            format!("the published basis revision exceeds {MAX_BASIS_REVISION}"),
        ));
    }
    Ok(())
}

fn require_signed_serializable(value: &impl Serialize) -> Result<(), Failure> {
    let value = serde_json::to_value(value).map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_VOICE_SELECTION_INVARIANT",
            "a Voice selection wire value could not be inspected",
        )
    })?;
    require_signed_json_value(&value)
}

fn require_signed_json_value(value: &Value) -> Result<(), Failure> {
    match value {
        Value::Number(number) if number.as_u64().is_some_and(|value| value > i64::MAX as u64) => {
            Err(Failure::new(
                "AUTHORING_REVISION3_VOICE_SELECTION_SIGNED_WIRE_LIMIT",
                "a Voice selection wire integer exceeds the signed 64-bit transport range",
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
            "AUTHORING_REVISION3_VOICE_SELECTION_INVARIANT",
            "the Voice selection basis head could not be serialized",
        )
    })?;
    if value.is_empty() || value.len() > MAX_HEAD_JSON_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_SELECTION_RESPONSE_LIMIT",
            "the Voice selection basis head exceeds its bounded transport limit",
        ));
    }
    Ok(value)
}

fn enforce_response_budget(response: &Value) -> Result<(), Failure> {
    let encoded = serde_json::to_vec(response).map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_VOICE_SELECTION_INVARIANT",
            "the Voice selection response could not be serialized",
        )
    })?;
    if encoded.len() > MAX_RESPONSE_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_SELECTION_RESPONSE_LIMIT",
            "the Voice selection response exceeds its bounded transport budget",
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
        "AUTHORING_REVISION3_VOICE_SELECTION_REQUEST_INVALID",
        "request must contain one exact duplicate-free command and exactly current_project_json, root, and voice_take_selection_request_json",
    )
}

fn head_conflict() -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_VOICE_SELECTION_HEAD_CONFLICT",
        "the published revision-3 head changed or differs from the Voice selection request",
    )
}

fn line_conflict(message: &'static str) -> Failure {
    Failure::new("AUTHORING_REVISION3_VOICE_SELECTION_LINE_CONFLICT", message)
}

fn slot_conflict(message: &'static str) -> Failure {
    Failure::new("AUTHORING_REVISION3_VOICE_SELECTION_SLOT_CONFLICT", message)
}

fn selection_conflict(message: &'static str) -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_VOICE_SELECTION_SELECTION_CONFLICT",
        message,
    )
}

fn project_invalid(message: &'static str) -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_VOICE_SELECTION_PROJECT_INVALID",
        message,
    )
}

fn revision_limit(message: &'static str) -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_VOICE_SELECTION_REVISION_LIMIT",
        message,
    )
}

fn map_request_error(_error: impl std::fmt::Display) -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_VOICE_SELECTION_REQUEST_INVALID",
        "the exact canonical Voice selection request is invalid",
    )
}

fn map_transaction_error(error: Revision3VoiceTakeSelectionErrorV1) -> Failure {
    match error {
        Revision3VoiceTakeSelectionErrorV1::InvalidProject(_) => project_invalid(
            "the exact current revision-3 project is not a valid Voice selection basis",
        ),
        Revision3VoiceTakeSelectionErrorV1::InvalidRequest(error) => map_request_error(error),
        Revision3VoiceTakeSelectionErrorV1::ReopenCandidate(_)
        | Revision3VoiceTakeSelectionErrorV1::CanonicalReopenMismatch => Failure::new(
            "AUTHORING_REVISION3_VOICE_SELECTION_INVARIANT",
            "the Voice selection candidate failed exact canonical reopen",
        ),
    }
}

fn map_transaction_conflict(error: Revision3VoiceTakeSelectionConflictV1) -> Failure {
    let code = match &error {
        Revision3VoiceTakeSelectionConflictV1::CurrentHeadMismatch => {
            "AUTHORING_REVISION3_VOICE_SELECTION_HEAD_CONFLICT"
        }
        Revision3VoiceTakeSelectionConflictV1::ProjectIdentityMismatch { .. }
        | Revision3VoiceTakeSelectionConflictV1::ProjectRevisionConflict { .. } => {
            "AUTHORING_REVISION3_VOICE_SELECTION_PROJECT_CONFLICT"
        }
        Revision3VoiceTakeSelectionConflictV1::ProjectTargetMismatch => {
            "AUTHORING_REVISION3_VOICE_SELECTION_TARGET_CONFLICT"
        }
        Revision3VoiceTakeSelectionConflictV1::ProjectRevisionOverflow
        | Revision3VoiceTakeSelectionConflictV1::VoiceSlotRevisionOverflow { .. } => {
            "AUTHORING_REVISION3_VOICE_SELECTION_REVISION_LIMIT"
        }
        Revision3VoiceTakeSelectionConflictV1::InvalidEntityIdentity
        | Revision3VoiceTakeSelectionConflictV1::InvalidExpectedLocId => {
            "AUTHORING_REVISION3_VOICE_SELECTION_REQUEST_REJECTED"
        }
        Revision3VoiceTakeSelectionConflictV1::InvalidDialogLine { .. }
        | Revision3VoiceTakeSelectionConflictV1::InvalidLocalizationReference { .. }
        | Revision3VoiceTakeSelectionConflictV1::LocalizationIdentityMismatch { .. } => {
            "AUTHORING_REVISION3_VOICE_SELECTION_LINE_CONFLICT"
        }
        Revision3VoiceTakeSelectionConflictV1::VoiceSlotIdentityMismatch { .. }
        | Revision3VoiceTakeSelectionConflictV1::InvalidVoiceSlot { .. }
        | Revision3VoiceTakeSelectionConflictV1::VoiceSlotRevisionConflict { .. } => {
            "AUTHORING_REVISION3_VOICE_SELECTION_SLOT_CONFLICT"
        }
        Revision3VoiceTakeSelectionConflictV1::CurrentSelectionMismatch { .. } => {
            "AUTHORING_REVISION3_VOICE_SELECTION_SELECTION_CONFLICT"
        }
        Revision3VoiceTakeSelectionConflictV1::InvalidTakeIdentity { .. }
        | Revision3VoiceTakeSelectionConflictV1::SelectedTakeNotCandidate { .. }
        | Revision3VoiceTakeSelectionConflictV1::InvalidSelectedTake { .. }
        | Revision3VoiceTakeSelectionConflictV1::SelectedTakeLocaleMismatch { .. } => {
            "AUTHORING_REVISION3_VOICE_SELECTION_TAKE_CONFLICT"
        }
        Revision3VoiceTakeSelectionConflictV1::SelectedTakeNotApproved { .. } => {
            "AUTHORING_REVISION3_VOICE_SELECTION_TAKE_NOT_APPROVED"
        }
        Revision3VoiceTakeSelectionConflictV1::NoChanges => {
            "AUTHORING_REVISION3_VOICE_SELECTION_NO_CHANGES"
        }
        // Correctable project-size pressure is explicitly distinct from integrity failures.
        Revision3VoiceTakeSelectionConflictV1::CandidateTooLarge { .. } => {
            "AUTHORING_REVISION3_VOICE_SELECTION_PROJECT_LIMIT"
        }
        Revision3VoiceTakeSelectionConflictV1::CandidateNotPersistable { .. } => {
            "AUTHORING_REVISION3_VOICE_SELECTION_PROJECT_INVALID"
        }
    };
    Failure::new(code, error.to_string())
}

fn map_store_error(error: WorkingStoreError) -> Failure {
    let code = match error {
        WorkingStoreError::InvalidLimits(_) => {
            "AUTHORING_REVISION3_VOICE_SELECTION_STORE_LIMITS_INVALID"
        }
        WorkingStoreError::MissingRoot(_) => {
            "AUTHORING_REVISION3_VOICE_SELECTION_STORE_ROOT_MISSING"
        }
        WorkingStoreError::UnsafePath { .. } => {
            "AUTHORING_REVISION3_VOICE_SELECTION_STORE_PATH_UNSAFE"
        }
        WorkingStoreError::LimitExceeded { .. } => {
            "AUTHORING_REVISION3_VOICE_SELECTION_STORE_LIMIT"
        }
        WorkingStoreError::HeadConflict { .. } => {
            "AUTHORING_REVISION3_VOICE_SELECTION_HEAD_CONFLICT"
        }
        WorkingStoreError::MissingHead(_) => "AUTHORING_REVISION3_VOICE_SELECTION_HEAD_MISSING",
        WorkingStoreError::MissingObject(_) => {
            "AUTHORING_REVISION3_VOICE_SELECTION_STORE_OBJECT_MISSING"
        }
        WorkingStoreError::SealMismatch { .. } => {
            "AUTHORING_REVISION3_VOICE_SELECTION_STORE_SEAL_MISMATCH"
        }
        WorkingStoreError::Collision { .. } => {
            "AUTHORING_REVISION3_VOICE_SELECTION_STORE_COLLISION"
        }
        WorkingStoreError::InvalidJson { .. } | WorkingStoreError::NonCanonicalJson { .. } => {
            "AUTHORING_REVISION3_VOICE_SELECTION_STORE_JSON_INVALID"
        }
        WorkingStoreError::Invariant(_)
        | WorkingStoreError::InvalidOgg(_)
        | WorkingStoreError::OggMetadataMismatch { .. } => {
            "AUTHORING_REVISION3_VOICE_SELECTION_STORE_INVARIANT"
        }
        WorkingStoreError::StagingCleanup { .. } | WorkingStoreError::Io(_) => {
            "AUTHORING_REVISION3_VOICE_SELECTION_STORE_IO"
        }
    };
    Failure::new(
        code,
        "the revision-3 Voice selection Store operation failed",
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
    use std::path::{Path, PathBuf};

    use gore_authoring::model_revision3::{
        DialogLine, Entity, EntityKind, EntityPayload, LocalizationEntry,
        OggCodec as Revision3OggCodec, OggMetadata as Revision3OggMetadata, OriginRef,
        SchemaRevisionV3, TypedRef, VoiceSlot, VoiceTake, VoiceTakeStatus, VoiceTargetResolution,
    };
    use gore_authoring::{
        AssetMeta, AssetStoreIndex, ContentSeal, FormatV2, GameGenerationAnchor, LocaleCode,
        ProjectId, ProjectMeta, Sha256Digest,
    };
    use tempfile::TempDir;

    use super::*;

    const LOC_ID: &str = "GRD_263_ASGHAN_OPEN_INFO_06_02";
    const LOCALIZATION_ID_BYTE: u8 = 0x22;
    const LINE_ID_BYTE: u8 = 0x23;
    const SLOT_ID_BYTE: u8 = 0x24;
    const TAKE_A_ID_BYTE: u8 = 0x25;
    const TAKE_B_ID_BYTE: u8 = 0x26;
    const TAKE_RECORDED_ID_BYTE: u8 = 0x27;

    struct PublishedStore {
        temp: TempDir,
        project: ProjectRevision3,
        project_json: String,
        head: WorkingHead,
        fixed_head_bytes: Vec<u8>,
        asset_path: PathBuf,
        asset_bytes: Vec<u8>,
    }

    fn id(tag: u8) -> EntityId {
        EntityId::from_bytes([tag; 16])
    }

    fn project_id() -> ProjectId {
        ProjectId::from_bytes([0x31; 16])
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

    fn imported_origin(tag: u8) -> OriginRef {
        OriginRef::Imported {
            importer: "voice-selection-ffi-tests".to_owned(),
            source_seal: ContentSeal {
                byte_len: 10,
                sha256: Sha256Digest::from_bytes([tag; 32]),
            },
            external_identity: None,
        }
    }

    fn empty_project(revision: u64) -> ProjectRevision3 {
        ProjectRevision3 {
            format: FormatV2,
            schema_revision: SchemaRevisionV3,
            project_id: project_id(),
            revision,
            meta: ProjectMeta {
                name: "Voice selection FFI fixture".to_owned(),
                version: "1.0.0".to_owned(),
                author: "tests".to_owned(),
            },
            target: target(),
            authoring_locales: BTreeSet::from([locale()]),
            entities: BTreeMap::new(),
            asset_store: AssetStoreIndex::default(),
        }
    }

    fn entity(tag: u8, revision: u64, display_name: &str, payload: EntityPayload) -> Entity {
        let id = id(tag);
        Entity {
            id,
            display_name: display_name.to_owned(),
            origin: imported_origin(tag),
            revision,
            payload,
        }
    }

    fn take_entity(
        tag: u8,
        logical_suffix: &str,
        status: VoiceTakeStatus,
        imported: &gore_authoring::ImportedOgg,
    ) -> Entity {
        let mut asset = imported.asset.clone();
        asset.logical_name = format!("{LOC_ID}_{logical_suffix}.ogg");
        entity(
            tag,
            1,
            &format!("Asghan {logical_suffix}"),
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
                status,
            }),
        )
    }

    fn voice_project(revision: u64, imported: &gore_authoring::ImportedOgg) -> ProjectRevision3 {
        let localization_id = id(LOCALIZATION_ID_BYTE);
        let line_id = id(LINE_ID_BYTE);
        let slot_id = id(SLOT_ID_BYTE);
        let take_a = id(TAKE_A_ID_BYTE);
        let take_b = id(TAKE_B_ID_BYTE);
        let take_recorded = id(TAKE_RECORDED_ID_BYTE);
        let take_ref = |take| TypedRef::new(project_id(), take, EntityKind::VoiceTake);

        let mut project = empty_project(revision);
        project.asset_store.assets.insert(
            imported.asset.sha256,
            AssetMeta {
                byte_len: imported.asset.byte_len,
                media_type: "audio/ogg".to_owned(),
            },
        );
        project.entities = BTreeMap::from([
            (
                localization_id,
                entity(
                    LOCALIZATION_ID_BYTE,
                    4,
                    "Asghan line text",
                    EntityPayload::LocalizationEntry(LocalizationEntry {
                        loc_id: LOC_ID.to_owned(),
                        texts: BTreeMap::new(),
                    }),
                ),
            ),
            (
                line_id,
                entity(
                    LINE_ID_BYTE,
                    2,
                    "Asghan greeting",
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
                    SLOT_ID_BYTE,
                    3,
                    "Asghan German voice slot",
                    EntityPayload::VoiceSlot(VoiceSlot {
                        locale: locale(),
                        target_resolution: VoiceTargetResolution::Unresolved,
                        candidates: vec![
                            take_ref(take_a),
                            take_ref(take_b),
                            take_ref(take_recorded),
                        ],
                        selected: Some(take_ref(take_a)),
                    }),
                ),
            ),
            (
                take_a,
                take_entity(
                    TAKE_A_ID_BYTE,
                    "take_a",
                    VoiceTakeStatus::Approved,
                    imported,
                ),
            ),
            (
                take_b,
                take_entity(
                    TAKE_B_ID_BYTE,
                    "take_b",
                    VoiceTakeStatus::Approved,
                    imported,
                ),
            ),
            (
                take_recorded,
                take_entity(
                    TAKE_RECORDED_ID_BYTE,
                    "take_recorded",
                    VoiceTakeStatus::Recorded,
                    imported,
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

    fn published_store() -> PublishedStore {
        let temp = TempDir::new().unwrap();
        let store = WorkingProjectStore::at(temp.path(), ffi_store_limits()).unwrap();
        let empty = empty_project(0);
        let basis = store.prepare_revision3_checkpoint(None, &empty).unwrap();
        fs::write(temp.path().join("gore-project.json"), &basis.head_bytes).unwrap();

        let source_temp = TempDir::new().unwrap();
        let source = source_temp.path().join("asghan-selection-fixture.ogg");
        fs::write(
            &source,
            include_bytes!("../../gore-vo/testdata/tiny-vorbis.ogg"),
        )
        .unwrap();
        let imported = store
            .import_ogg(&source, "asghan-selection-fixture.ogg", Some(&basis.head))
            .unwrap();
        let asset_path = asset_path(temp.path(), imported.asset.sha256);
        let asset_bytes = fs::read(&asset_path).unwrap();

        let project = voice_project(1, &imported);
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
            asset_path,
            asset_bytes,
        }
    }

    fn selection_request(
        store: &PublishedStore,
        selected_take_id: Option<EntityId>,
    ) -> Revision3VoiceTakeSelectionRequestV1 {
        Revision3VoiceTakeSelectionRequestV1 {
            expected_head: store.head.clone(),
            expected_project_id: store.project.project_id,
            expected_revision: store.project.revision,
            expected_target: store.project.target.clone(),
            line_id: id(LINE_ID_BYTE),
            slot_id: id(SLOT_ID_BYTE),
            expected_slot_revision: 3,
            locale: locale(),
            expected_loc_id: LOC_ID.to_owned(),
            expected_selected_take_id: Some(id(TAKE_A_ID_BYTE)),
            selected_take_id,
        }
    }

    fn raw_request(payload: Value) -> String {
        serde_json::to_string(&json!({"command": COMMAND, "payload": payload})).unwrap()
    }

    fn wire(root: &Path, project_json: &str, request_json: &str) -> String {
        raw_request(json!({
            "current_project_json": project_json,
            "root": root,
            "voice_take_selection_request_json": request_json,
        }))
    }

    fn call(store: &PublishedStore, request: &Revision3VoiceTakeSelectionRequestV1) -> Value {
        prepare_revision3_voice_take_selection_v1_raw(&wire(
            store.temp.path(),
            &store.project_json,
            &request.to_canonical_json().unwrap(),
        ))
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

    fn rival_head(store: &PublishedStore, name: &str) -> Vec<u8> {
        let working =
            WorkingProjectStore::open_existing(store.temp.path(), ffi_store_limits()).unwrap();
        let mut rival = store.project.clone();
        rival.revision += 1;
        rival.meta.name = name.to_owned();
        working
            .prepare_revision3_checkpoint(Some(&store.head), &rival)
            .unwrap()
            .head_bytes
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
    fn exact_wire_rejects_duplicates_unknown_authority_types_and_order() {
        let valid = raw_request(json!({
            "current_project_json": "{}",
            "root": "C:/missing",
            "voice_take_selection_request_json": "{}",
        }));
        let parsed: PrepareVoiceSelectionWirePayload = parse_exact_wire(&valid).unwrap();
        assert_eq!(parsed.current_project_json, "{}");

        let cases = [
            format!(
                "{{\"command\":\"{COMMAND}\",\"command\":\"{COMMAND}\",\"payload\":{{\"current_project_json\":\"{{}}\",\"root\":\"r\",\"voice_take_selection_request_json\":\"{{}}\"}}}}"
            ),
            format!(
                "{{\"command\":\"{COMMAND}\",\"payload\":{{\"current_project_json\":\"{{}}\",\"root\":\"r\",\"root\":\"forged\",\"voice_take_selection_request_json\":\"{{}}\"}}}}"
            ),
            raw_request(json!({
                "current_project_json": "{}", "root": "r",
                "voice_take_selection_request_json": "{}", "game_root": "forged"
            })),
            raw_request(json!({
                "root": "r", "voice_take_selection_request_json": "{}"
            })),
            raw_request(json!({
                "current_project_json": {}, "root": "r",
                "voice_take_selection_request_json": "{}"
            })),
            format!(" {valid}"),
            format!(
                "{{\"payload\":{{\"current_project_json\":\"{{}}\",\"root\":\"C:/missing\",\"voice_take_selection_request_json\":\"{{}}\"}},\"command\":\"{COMMAND}\"}}"
            ),
            format!(
                "{{\"command\":\"{COMMAND}\",\"payload\":{{\"root\":\"C:/missing\",\"current_project_json\":\"{{}}\",\"voice_take_selection_request_json\":\"{{}}\"}}}}"
            ),
        ];
        for input in cases {
            assert_eq!(
                prepare_revision3_voice_take_selection_v1_raw(&input)["error"]["code"],
                "AUTHORING_REVISION3_VOICE_SELECTION_REQUEST_INVALID",
                "{input}"
            );
        }
    }

    #[test]
    fn public_dispatch_preserves_raw_duplicate_and_noncanonical_rejection() {
        let duplicate = format!(
            "{{\"command\":\"{COMMAND}\",\"payload\":{{\"current_project_json\":\"{{}}\",\"root\":\"r\",\"root\":\"forged\",\"voice_take_selection_request_json\":\"{{}}\"}}}}"
        );
        let response: Value = serde_json::from_str(&crate::execute_json(&duplicate)).unwrap();
        assert_eq!(
            response["error"]["code"],
            "AUTHORING_REVISION3_VOICE_SELECTION_REQUEST_INVALID"
        );
        let spaced = format!(
            "{{ \"command\":\"{COMMAND}\",\"payload\":{{\"current_project_json\":\"{{}}\",\"root\":\"r\",\"voice_take_selection_request_json\":\"{{}}\"}}}}"
        );
        let response: Value = serde_json::from_str(&crate::execute_json(&spaced)).unwrap();
        assert_eq!(
            response["error"]["code"],
            "AUTHORING_REVISION3_VOICE_SELECTION_REQUEST_INVALID"
        );
    }

    #[test]
    fn transport_caps_and_signed_values_reject_before_store_mutation() {
        let valid_shape = || {
            json!({
                "current_project_json": "{}",
                "root": "C:/missing",
                "voice_take_selection_request_json": "{}",
            })
        };
        let mut payload = valid_shape();
        payload["root"] = Value::String("x".repeat(MAX_PATH_BYTES + 1));
        assert_eq!(
            prepare_revision3_voice_take_selection_v1_raw(&raw_request(payload))["error"]["code"],
            "AUTHORING_REVISION3_VOICE_SELECTION_REQUEST_INVALID"
        );
        let mut payload = valid_shape();
        payload["current_project_json"] = Value::String("x".repeat(MAX_PROJECT_JSON_BYTES + 1));
        assert_eq!(
            prepare_revision3_voice_take_selection_v1_raw(&raw_request(payload))["error"]["code"],
            "AUTHORING_REVISION3_VOICE_SELECTION_PROJECT_LIMIT"
        );
        let mut payload = valid_shape();
        payload["voice_take_selection_request_json"] =
            Value::String("x".repeat(MAX_REVISION3_VOICE_TAKE_SELECTION_REQUEST_JSON_BYTES_V1 + 1));
        assert_eq!(
            prepare_revision3_voice_take_selection_v1_raw(&raw_request(payload))["error"]["code"],
            "AUTHORING_REVISION3_VOICE_SELECTION_REQUEST_LIMIT"
        );
        assert_eq!(
            prepare_revision3_voice_take_selection_v1_raw(&" ".repeat(MAX_WIRE_BYTES + 1))["error"]
                ["code"],
            "AUTHORING_REVISION3_VOICE_SELECTION_INPUT_LIMIT"
        );

        let store = published_store();
        let before = snapshot_regular_files(store.temp.path());
        let mut request = selection_request(&store, Some(id(TAKE_B_ID_BYTE)));
        request.expected_revision = u64::MAX;
        assert_eq!(
            call(&store, &request)["error"]["code"],
            "AUTHORING_REVISION3_VOICE_SELECTION_SIGNED_WIRE_LIMIT"
        );
        assert_eq!(snapshot_regular_files(store.temp.path()), before);
    }

    #[test]
    fn malformed_nested_request_is_rejected_before_store_probe_or_write() {
        let missing_parent = TempDir::new().unwrap();
        let missing_root = missing_parent.path().join("missing-store");
        let response =
            prepare_revision3_voice_take_selection_v1_raw(&wire(&missing_root, "{}", "{}"));
        assert_eq!(
            response["error"]["code"],
            "AUTHORING_REVISION3_VOICE_SELECTION_REQUEST_INVALID"
        );
        assert!(!missing_root.exists());

        let store = published_store();
        let before = snapshot_regular_files(store.temp.path());
        let canonical = selection_request(&store, Some(id(TAKE_B_ID_BYTE)))
            .to_canonical_json()
            .unwrap();
        for nested in [
            "{}".to_owned(),
            format!(" {canonical}"),
            r#"{"C:/secret/voice-selection":true}"#.to_owned(),
            canonical.replacen(
                "\"expected_revision\":1",
                "\"expected_revision\":1,\"expected_revision\":1",
                1,
            ),
        ] {
            let response = prepare_revision3_voice_take_selection_v1_raw(&wire(
                store.temp.path(),
                &store.project_json,
                &nested,
            ));
            assert_eq!(
                response["error"]["code"],
                "AUTHORING_REVISION3_VOICE_SELECTION_REQUEST_INVALID"
            );
            assert!(!response.to_string().contains("secret"));
        }
        assert_eq!(snapshot_regular_files(store.temp.path()), before);
    }

    #[test]
    fn approved_selection_prepares_exact_fully_reopened_delta_without_publication() {
        let store = published_store();
        let response = call(&store, &selection_request(&store, Some(id(TAKE_B_ID_BYTE))));
        assert_eq!(response["ok"], true, "{response}");
        assert_eq!(response["outcome"], "prepared_unpublished");
        assert_eq!(response["project_id"], store.project.project_id.to_string());
        assert_eq!(response["revision"], 2);
        assert_eq!(response["line_id"], id(LINE_ID_BYTE).to_string());
        assert_eq!(response["slot_id"], id(SLOT_ID_BYTE).to_string());
        assert_eq!(response["slot_revision"], 4);
        assert_eq!(response["locale"], "de");
        assert_eq!(response["loc_id"], LOC_ID);
        assert_eq!(
            response["previous_selected_take_id"],
            id(TAKE_A_ID_BYTE).to_string()
        );
        assert_eq!(response["selected_take_id"], id(TAKE_B_ID_BYTE).to_string());
        assert_eq!(response["build_status"], "blocked");
        assert_eq!(response["runtime_status"], "runtime_unqualified");
        assert_eq!(response["publication_status"], "not_supported");
        assert_eq!(
            response["basis_head_json"],
            serde_json::to_string(&store.head).unwrap()
        );
        assert_eq!(fixed_head(&store), store.fixed_head_bytes);
        assert_eq!(fs::read(&store.asset_path).unwrap(), store.asset_bytes);

        let keys = response
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        assert_eq!(
            keys,
            BTreeSet::from([
                "basis_head_json",
                "build_status",
                "head_json",
                "line_id",
                "loc_id",
                "locale",
                "ok",
                "outcome",
                "previous_selected_take_id",
                "project_id",
                "project_json",
                "publication_status",
                "revision",
                "runtime_status",
                "selected_take_id",
                "slot_id",
                "slot_revision",
            ])
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
        let mut expected = store.project.clone();
        expected.revision = 2;
        let slot_entity = expected.entities.get_mut(&id(SLOT_ID_BYTE)).unwrap();
        slot_entity.revision = 4;
        let EntityPayload::VoiceSlot(slot) = &mut slot_entity.payload else {
            panic!("expected VoiceSlot")
        };
        slot.selected = Some(TypedRef::new(
            project_id(),
            id(TAKE_B_ID_BYTE),
            EntityKind::VoiceTake,
        ));
        assert_eq!(reopened.project, expected);

        let encoded = response.to_string();
        assert!(!encoded.contains(store.temp.path().to_string_lossy().as_ref()));
        assert!(!encoded.contains("game_root"));
        assert!(!encoded.contains("\"source\":"));
        assert!(!encoded.contains("deploy"));
    }

    #[test]
    fn clear_selection_returns_exact_nulls_and_reopens_with_no_selected_take() {
        let store = published_store();
        let response = call(&store, &selection_request(&store, None));
        assert_eq!(response["ok"], true, "{response}");
        assert_eq!(
            response["previous_selected_take_id"],
            id(TAKE_A_ID_BYTE).to_string()
        );
        assert!(response["selected_take_id"].is_null());
        assert_eq!(response["slot_revision"], 4);
        assert_eq!(fixed_head(&store), store.fixed_head_bytes);

        let reopened = WorkingProjectStore::open_existing(store.temp.path(), ffi_store_limits())
            .unwrap()
            .open_revision3_head_bytes(
                response["head_json"].as_str().unwrap().as_bytes(),
                AssetVerification::Full,
            )
            .unwrap();
        let EntityPayload::VoiceSlot(slot) = &reopened.project.entities[&id(SLOT_ID_BYTE)].payload
        else {
            panic!("expected VoiceSlot")
        };
        assert_eq!(reopened.project.revision, 2);
        assert_eq!(reopened.project.entities[&id(SLOT_ID_BYTE)].revision, 4);
        assert!(slot.selected.is_none());
        assert_eq!(fs::read(&store.asset_path).unwrap(), store.asset_bytes);
    }

    #[test]
    fn all_stale_bindings_unapproved_take_and_noop_write_no_candidate_objects() {
        let store = published_store();
        let before = snapshot_regular_files(store.temp.path());
        let base = selection_request(&store, Some(id(TAKE_B_ID_BYTE)));

        let response = prepare_revision3_voice_take_selection_v1_raw(&wire(
            store.temp.path(),
            "not-the-exact-current-project",
            &base.to_canonical_json().unwrap(),
        ));
        assert_eq!(
            response["error"]["code"],
            "AUTHORING_REVISION3_VOICE_SELECTION_PROJECT_CONFLICT"
        );

        let mut cases = Vec::new();
        let mut stale_head = base.clone();
        stale_head.expected_head.snapshot.byte_len += 1;
        cases.push((
            stale_head,
            "AUTHORING_REVISION3_VOICE_SELECTION_HEAD_CONFLICT",
        ));
        let mut foreign_project = base.clone();
        foreign_project.expected_project_id = ProjectId::from_bytes([0x91; 16]);
        cases.push((
            foreign_project,
            "AUTHORING_REVISION3_VOICE_SELECTION_PROJECT_CONFLICT",
        ));
        let mut stale_project = base.clone();
        stale_project.expected_revision = 0;
        cases.push((
            stale_project,
            "AUTHORING_REVISION3_VOICE_SELECTION_PROJECT_CONFLICT",
        ));
        let mut wrong_target = base.clone();
        wrong_target.expected_target.executable.sha256 = Sha256Digest::from_bytes([0x92; 32]);
        cases.push((
            wrong_target,
            "AUTHORING_REVISION3_VOICE_SELECTION_TARGET_CONFLICT",
        ));
        let mut wrong_line = base.clone();
        wrong_line.line_id = id(0x93);
        cases.push((
            wrong_line,
            "AUTHORING_REVISION3_VOICE_SELECTION_LINE_CONFLICT",
        ));
        let mut wrong_slot = base.clone();
        wrong_slot.slot_id = id(0x94);
        cases.push((
            wrong_slot,
            "AUTHORING_REVISION3_VOICE_SELECTION_SLOT_CONFLICT",
        ));
        let mut stale_slot = base.clone();
        stale_slot.expected_slot_revision = 2;
        cases.push((
            stale_slot,
            "AUTHORING_REVISION3_VOICE_SELECTION_SLOT_CONFLICT",
        ));
        let mut wrong_locale = base.clone();
        wrong_locale.locale = "en".parse().unwrap();
        cases.push((
            wrong_locale,
            "AUTHORING_REVISION3_VOICE_SELECTION_SLOT_CONFLICT",
        ));
        let mut wrong_loc_id = base.clone();
        wrong_loc_id.expected_loc_id = "GRD_OTHER_LINE".to_owned();
        cases.push((
            wrong_loc_id,
            "AUTHORING_REVISION3_VOICE_SELECTION_LINE_CONFLICT",
        ));
        let mut wrong_selection = base.clone();
        wrong_selection.expected_selected_take_id = Some(id(TAKE_B_ID_BYTE));
        cases.push((
            wrong_selection,
            "AUTHORING_REVISION3_VOICE_SELECTION_SELECTION_CONFLICT",
        ));
        let mut zero_line = base.clone();
        zero_line.line_id = EntityId::from_bytes([0; 16]);
        cases.push((
            zero_line,
            "AUTHORING_REVISION3_VOICE_SELECTION_REQUEST_REJECTED",
        ));
        let mut invalid_loc_id = base.clone();
        invalid_loc_id.expected_loc_id = "C:/secret/voice".to_owned();
        cases.push((
            invalid_loc_id,
            "AUTHORING_REVISION3_VOICE_SELECTION_REQUEST_REJECTED",
        ));
        let mut colliding_take = base.clone();
        colliding_take.expected_selected_take_id = Some(id(SLOT_ID_BYTE));
        cases.push((
            colliding_take,
            "AUTHORING_REVISION3_VOICE_SELECTION_TAKE_CONFLICT",
        ));
        let mut noop = base.clone();
        noop.selected_take_id = Some(id(TAKE_A_ID_BYTE));
        cases.push((noop, "AUTHORING_REVISION3_VOICE_SELECTION_NO_CHANGES"));
        let mut non_candidate = base.clone();
        non_candidate.selected_take_id = Some(id(0x95));
        cases.push((
            non_candidate,
            "AUTHORING_REVISION3_VOICE_SELECTION_TAKE_CONFLICT",
        ));
        let mut recorded = base;
        recorded.selected_take_id = Some(id(TAKE_RECORDED_ID_BYTE));
        cases.push((
            recorded,
            "AUTHORING_REVISION3_VOICE_SELECTION_TAKE_NOT_APPROVED",
        ));

        for (request, expected_code) in cases {
            let response = call(&store, &request);
            assert_eq!(response["error"]["code"], expected_code, "{response}");
            assert!(!response.to_string().contains("secret"));
            assert_eq!(snapshot_regular_files(store.temp.path()), before);
        }
        assert_eq!(fixed_head(&store), store.fixed_head_bytes);
    }

    #[test]
    fn maximum_signed_slot_revision_returns_precise_limit_before_candidate_write() {
        let mut store = published_store();
        let working =
            WorkingProjectStore::open_existing(store.temp.path(), ffi_store_limits()).unwrap();
        let mut project = store.project.clone();
        project.revision += 1;
        project
            .entities
            .get_mut(&id(SLOT_ID_BYTE))
            .unwrap()
            .revision = MAX_BASIS_REVISION + 1;
        let published = working
            .prepare_revision3_checkpoint(Some(&store.head), &project)
            .unwrap();
        fs::write(
            store.temp.path().join("gore-project.json"),
            &published.head_bytes,
        )
        .unwrap();
        store.project_json = project.to_canonical_json().unwrap();
        store.project = project;
        store.head = published.head;
        store.fixed_head_bytes = published.head_bytes;

        let before = snapshot_regular_files(store.temp.path());
        let mut request = selection_request(&store, Some(id(TAKE_B_ID_BYTE)));
        request.expected_slot_revision = MAX_BASIS_REVISION + 1;
        let response = call(&store, &request);
        assert_eq!(
            response["error"]["code"],
            "AUTHORING_REVISION3_VOICE_SELECTION_REVISION_LIMIT"
        );
        assert_eq!(snapshot_regular_files(store.temp.path()), before);
    }

    #[test]
    fn checkpoint_failure_is_path_free_and_never_publishes_candidate() {
        let store = published_store();
        let request = selection_request(&store, Some(id(TAKE_B_ID_BYTE)));
        let wire = wire(
            store.temp.path(),
            &store.project_json,
            &request.to_canonical_json().unwrap(),
        );
        let response = prepare_revision3_voice_take_selection_v1_inner_with_test_seams(
            &wire,
            || fs::remove_file(&store.asset_path).unwrap(),
            || {},
            || {},
        )
        .unwrap_err()
        .response();
        assert_eq!(
            response["error"]["code"],
            "AUTHORING_REVISION3_VOICE_SELECTION_STORE_OBJECT_MISSING"
        );
        assert!(!response
            .to_string()
            .contains(store.temp.path().to_string_lossy().as_ref()));
        assert_eq!(fixed_head(&store), store.fixed_head_bytes);
    }

    #[test]
    fn post_prepare_head_guard_preserves_external_publisher() {
        let store = published_store();
        let rival = rival_head(&store, "External publisher before first guard");
        let request = selection_request(&store, Some(id(TAKE_B_ID_BYTE)));
        let wire = wire(
            store.temp.path(),
            &store.project_json,
            &request.to_canonical_json().unwrap(),
        );
        let response =
            prepare_revision3_voice_take_selection_v1_inner_with_post_prepare_guard(&wire, || {
                fs::write(store.temp.path().join("gore-project.json"), &rival).unwrap();
            })
            .unwrap_err()
            .response();
        assert_eq!(
            response["error"]["code"],
            "AUTHORING_REVISION3_VOICE_SELECTION_HEAD_CONFLICT"
        );
        assert_eq!(fixed_head(&store), rival);
    }

    #[test]
    fn final_response_head_guard_preserves_late_external_publisher() {
        let store = published_store();
        let rival = rival_head(&store, "External publisher after response construction");
        let request = selection_request(&store, Some(id(TAKE_B_ID_BYTE)));
        let wire = wire(
            store.temp.path(),
            &store.project_json,
            &request.to_canonical_json().unwrap(),
        );
        let response =
            prepare_revision3_voice_take_selection_v1_inner_with_final_guard(&wire, || {
                fs::write(store.temp.path().join("gore-project.json"), &rival).unwrap();
            })
            .unwrap_err()
            .response();
        assert_eq!(
            response["error"]["code"],
            "AUTHORING_REVISION3_VOICE_SELECTION_HEAD_CONFLICT"
        );
        assert_eq!(fixed_head(&store), rival);
    }

    #[test]
    fn error_taxonomy_separates_retryable_limits_from_integrity_and_hides_paths() {
        assert_eq!(
            map_transaction_conflict(Revision3VoiceTakeSelectionConflictV1::NoChanges).code,
            "AUTHORING_REVISION3_VOICE_SELECTION_NO_CHANGES"
        );
        assert_eq!(
            map_transaction_conflict(Revision3VoiceTakeSelectionConflictV1::CandidateTooLarge {
                actual: MAX_PROJECT_JSON_BYTES + 1,
                limit: MAX_PROJECT_JSON_BYTES,
            })
            .code,
            "AUTHORING_REVISION3_VOICE_SELECTION_PROJECT_LIMIT"
        );
        assert_eq!(
            map_transaction_conflict(
                Revision3VoiceTakeSelectionConflictV1::CandidateNotPersistable {
                    reason: "fixture invariant".to_owned(),
                }
            )
            .code,
            "AUTHORING_REVISION3_VOICE_SELECTION_PROJECT_INVALID"
        );
        let unsafe_path = map_store_error(WorkingStoreError::UnsafePath {
            path: PathBuf::from("C:/secret/voice-selection-store"),
            reason: "fixture".to_owned(),
        });
        assert_eq!(
            unsafe_path.code,
            "AUTHORING_REVISION3_VOICE_SELECTION_STORE_PATH_UNSAFE"
        );
        assert!(!unsafe_path.message.contains("secret"));
    }

    #[test]
    fn linked_store_root_is_rejected_without_following_it() {
        let store = published_store();
        let parent = TempDir::new().unwrap();
        let alias = parent.path().join("alias");
        if !make_test_dir_link(store.temp.path(), &alias) {
            return;
        }
        let request = selection_request(&store, Some(id(TAKE_B_ID_BYTE)));
        let response = prepare_revision3_voice_take_selection_v1_raw(&wire(
            &alias,
            &store.project_json,
            &request.to_canonical_json().unwrap(),
        ));
        assert_eq!(
            response["error"]["code"],
            "AUTHORING_REVISION3_VOICE_SELECTION_STORE_PATH_UNSAFE"
        );
        assert_eq!(fixed_head(&store), store.fixed_head_bytes);
        #[cfg(unix)]
        fs::remove_file(alias).unwrap();
        #[cfg(windows)]
        fs::remove_dir(alias).unwrap();
    }
}
